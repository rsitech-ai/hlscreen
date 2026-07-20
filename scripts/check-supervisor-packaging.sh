#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/hlscreen-supervisor.XXXXXX")"
service_pid=""
service_stdout=""
service_stderr=""

cleanup() {
  if [[ -n "$service_pid" ]] && kill -0 "$service_pid" 2>/dev/null; then
    kill -TERM "$service_pid" 2>/dev/null || true
    for _ in {1..50}; do
      kill -0 "$service_pid" 2>/dev/null || break
      sleep 0.1
    done
    if kill -0 "$service_pid" 2>/dev/null; then
      kill -KILL "$service_pid" 2>/dev/null || true
    fi
    wait "$service_pid" 2>/dev/null || true
  fi
  service_pid=""
  if [[ -n "$evidence_dir" && -d "$evidence_dir" ]]; then
    rm -rf -- "$evidence_dir"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_service_evidence() {
  if [[ -n "$service_stdout" && -s "$service_stdout" ]]; then
    echo "captured service stdout:" >&2
    cat "$service_stdout" >&2
  fi
  if [[ -n "$service_stderr" && -s "$service_stderr" ]]; then
    echo "captured service stderr:" >&2
    cat "$service_stderr" >&2
  fi
}

fail() {
  echo "supervisor packaging check failed: $1" >&2
  show_service_evidence
  exit 1
}

require_text() {
  local needle="$1"
  local path="$2"
  if ! grep -q -- "$needle" "$path"; then
    fail "missing required text in $path: $needle"
  fi
}

systemd="deploy/systemd/hlscreen-live.service"
launchd="deploy/launchd/com.hlscreen.live.plist.template"

for path in \
  "$systemd" \
  "$launchd" \
  "docs/deployment.md" \
  "scripts/run-supervised-soak.sh" \
  "scripts/validate-soak-report.py"; do
  if [[ ! -s "$path" ]]; then
    fail "missing supervisor packaging file: $path"
  fi
done

require_text "--all-symbols" scripts/run-supervised-soak.sh
require_text "--backfill-gaps" scripts/run-supervised-soak.sh
require_text "--verify-parity" scripts/run-supervised-soak.sh
require_text "max_rss_growth_bytes" scripts/validate-soak-report.py
require_text "unrepaired_gaps" scripts/validate-soak-report.py

require_text "hls server --live" "$systemd"
require_text "--all-symbols" "$systemd"
require_text "--duration-secs 86400" "$systemd"
require_text "--bind 127.0.0.1:8787" "$systemd"
require_text "Restart=always" "$systemd"
require_text "NoNewPrivileges=true" "$systemd"
require_text "ReadWritePaths=%h/.local/state/hlscreen" "$systemd"

require_text "com.hlscreen.live" "$launchd"
require_text "<string>server</string>" "$launchd"
require_text "<string>--live</string>" "$launchd"
require_text "<string>--all-symbols</string>" "$launchd"
require_text "<string>127.0.0.1:8787</string>" "$launchd"
require_text "<key>KeepAlive</key>" "$launchd"

unsafe_evidence="$evidence_dir/unsafe.txt"
if grep -RInE "wallet|private[-_ ]?key|place order|cancel order|withdraw|API_SECRET|SECRET_KEY" \
  deploy/systemd deploy/launchd >"$unsafe_evidence"; then
  echo "supervisor templates contain unsafe trading/private-data wording at:" >&2
  cut -d: -f1-2 "$unsafe_evidence" >&2
  exit 1
fi

cargo build -p hls-cli --bin hls >/dev/null

port="$(python3 - <<'PY'
import socket

with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
)"

wait_for_health() {
  local address="$1"
  for _ in {1..60}; do
    if python3 - "$address" <<'PY' >/dev/null 2>&1
import json
import sys
import urllib.request

with urllib.request.urlopen(sys.argv[1], timeout=0.2) as response:
    payload = json.load(response)
if payload.get("read_only") is not True:
    raise SystemExit(1)
PY
    then
      return 0
    fi
    if ! kill -0 "$service_pid" 2>/dev/null; then
      return 1
    fi
    sleep 0.1
  done
  return 1
}

wait_for_exit() {
  local pid="$1"
  for _ in {1..50}; do
    local state
    state="$(ps -o stat= -p "$pid" 2>/dev/null | tr -d '[:space:]')"
    if [[ -z "$state" || "$state" == Z* ]]; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

start_service() {
  local phase="$1"
  service_stdout="$evidence_dir/$phase.out"
  service_stderr="$evidence_dir/$phase.err"
  target/debug/hls server \
    --bind "127.0.0.1:$port" \
    >"$service_stdout" \
    2>"$service_stderr" &
  service_pid=$!
  if ! wait_for_health "http://127.0.0.1:$port/health"; then
    fail "$phase did not become healthy within the bounded readiness wait"
  fi
}

stop_service_cleanly() {
  local phase="$1"
  local stopped_pid="$service_pid"
  kill -TERM "$stopped_pid" || fail "$phase could not receive SIGTERM"
  if ! wait_for_exit "$stopped_pid"; then
    fail "$phase did not exit within the bounded shutdown wait"
  fi
  local status=0
  wait "$stopped_pid" || status=$?
  service_pid=""
  if (( status != 0 )); then
    fail "$phase exited with status $status after SIGTERM"
  fi
  if kill -0 "$stopped_pid" 2>/dev/null; then
    fail "$phase process remains after wait"
  fi
  require_text "stopped cleanly after SIGTERM" "$service_stderr"
}

start_service "first-start"
stop_service_cleanly "first-start"

python3 - "$port" <<'PY' || fail "listener port was not released after SIGTERM"
import socket
import sys

with socket.socket() as listener:
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", int(sys.argv[1])))
PY

start_service "same-port-restart"
stop_service_cleanly "same-port-restart"

echo "supervisor_packaging=passed"

#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

require_text() {
  local needle="$1"
  local path="$2"
  if ! grep -q -- "$needle" "$path"; then
    echo "missing required text in $path: $needle" >&2
    exit 1
  fi
}

systemd="deploy/systemd/hlscreen-live.service"
launchd="deploy/launchd/com.hlscreen.live.plist.template"

for path in "$systemd" "$launchd" "docs/deployment.md"; do
  if [[ ! -s "$path" ]]; then
    echo "missing supervisor packaging file: $path" >&2
    exit 1
  fi
done

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

if grep -RInE "wallet|private[-_ ]?key|place order|cancel order|withdraw|API_SECRET|SECRET_KEY" \
  deploy/systemd deploy/launchd \
  >/tmp/hlscreen-supervisor-unsafe.txt; then
  echo "supervisor templates contain unsafe trading/private-data wording:" >&2
  cat /tmp/hlscreen-supervisor-unsafe.txt >&2
  exit 1
fi

cargo build -p hls-cli --bin hls >/dev/null

target/debug/hls server \
  --live \
  --symbols @107 \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --bind 127.0.0.1:0 \
  >/tmp/hlscreen-supervisor-smoke.out \
  2>/tmp/hlscreen-supervisor-smoke.err

require_text "server_live_run=complete" /tmp/hlscreen-supervisor-smoke.out
require_text "rows=1" /tmp/hlscreen-supervisor-smoke.out
require_text "hls live server listening" /tmp/hlscreen-supervisor-smoke.err

echo "supervisor_packaging=passed"

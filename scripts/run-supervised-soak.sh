#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
duration_secs=900
sample_interval_secs=30
minimum_free_bytes=$((2 * 1024 * 1024 * 1024))
max_rss_growth_bytes=$((256 * 1024 * 1024))
data_dir="$repo_root/.hls-soak"
binary="$repo_root/target/release/hls"
binary_was_supplied=0
run_id="soak-$(date -u +%Y%m%dT%H%M%SZ)"
report_path=""

usage() {
  cat <<'EOF'
Usage: scripts/run-supervised-soak.sh [options]

Runs a bounded, public all-symbol live recording and validates its replay.

Options:
  --duration-secs N          Live capture duration (default: 900)
  --sample-interval-secs N   Resource sample interval (default: 30)
  --data-dir PATH            Recording/evidence directory
  --run-id ID                Recording run ID
  --report PATH              Final report path
  --binary PATH              hls binary (default: target/release/hls)
  --minimum-free-bytes N     Disk preflight threshold (default: 2 GiB)
  --max-rss-growth-bytes N   Validation threshold (default: 256 MiB)
EOF
}

while (($#)); do
  case "$1" in
    --duration-secs) duration_secs="${2:?missing duration}"; shift 2 ;;
    --sample-interval-secs) sample_interval_secs="${2:?missing interval}"; shift 2 ;;
    --data-dir) data_dir="${2:?missing data directory}"; shift 2 ;;
    --run-id) run_id="${2:?missing run ID}"; shift 2 ;;
    --report) report_path="${2:?missing report path}"; shift 2 ;;
    --binary) binary="${2:?missing binary path}"; binary_was_supplied=1; shift 2 ;;
    --minimum-free-bytes) minimum_free_bytes="${2:?missing byte count}"; shift 2 ;;
    --max-rss-growth-bytes) max_rss_growth_bytes="${2:?missing byte count}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for value_name in duration_secs sample_interval_secs minimum_free_bytes max_rss_growth_bytes; do
  value="${!value_name}"
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    echo "$value_name must be a non-negative integer" >&2
    exit 2
  fi
done
if ((duration_secs < 2 || sample_interval_secs < 1 || sample_interval_secs >= duration_secs)); then
  echo "duration must be at least 2 seconds and exceed the sample interval" >&2
  exit 2
fi
if [[ ! "$run_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]]; then
  echo "run ID contains unsupported characters" >&2
  exit 2
fi

initial_head="$(git -C "$repo_root" rev-parse HEAD)"
if [[ -n "$(git -C "$repo_root" status --porcelain --untracked-files=all)" ]]; then
  echo "soak evidence requires a clean source tree" >&2
  exit 1
fi
runtime_source_sha256="$(python3 "$repo_root/scripts/runtime-source-sha256.py" "$repo_root")"

assert_source_unchanged() {
  local current_head current_runtime_hash
  current_head="$(git -C "$repo_root" rev-parse HEAD)"
  current_runtime_hash="$(python3 "$repo_root/scripts/runtime-source-sha256.py" "$repo_root")"
  if [[ "$current_head" != "$initial_head" \
    || "$current_runtime_hash" != "$runtime_source_sha256" \
    || -n "$(git -C "$repo_root" status --porcelain --untracked-files=all)" ]]; then
    echo "source tree or HEAD changed during soak evidence collection" >&2
    exit 1
  fi
}

mkdir -p "$data_dir"
evidence_dir="$data_dir/soak-reports/$run_id"
mkdir -p "$evidence_dir"
report_path="${report_path:-$evidence_dir/report.json}"
mkdir -p "$(dirname "$report_path")"

available_kib="$(df -Pk "$data_dir" | awk 'NR == 2 {print $4}')"
if [[ ! "$available_kib" =~ ^[0-9]+$ ]]; then
  echo "could not determine free disk space for $data_dir" >&2
  exit 1
fi
if ((available_kib * 1024 < minimum_free_bytes)); then
  echo "disk preflight failed: available bytes below $minimum_free_bytes" >&2
  exit 1
fi

if ((binary_was_supplied == 0)); then
  (cd "$repo_root" && cargo build --release --locked -p hls-cli --bin hls) || exit 1
elif [[ ! -x "$binary" ]]; then
  echo "supplied binary is not executable: $binary" >&2
  exit 1
fi

assert_source_unchanged
commit="$initial_head"
binary_sha256="$(shasum -a 256 "$binary" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
host_triple="$(rustc -vV | awk '/^host:/ {print $2}')"
live_out="$evidence_dir/live.stdout.log"
live_err="$evidence_dir/live.stderr.log"
samples_tsv="$evidence_dir/resources.tsv"
replay_first_out="$evidence_dir/replay-first.stdout.log"
replay_first_err="$evidence_dir/replay-first.stderr.log"
replay_second_out="$evidence_dir/replay-second.stdout.log"
replay_second_err="$evidence_dir/replay-second.stderr.log"
: >"$samples_tsv"

command=(
  "$binary" live --all-symbols --duration-secs "$duration_secs"
  --refresh-secs 30 --record --raw --normalized --backfill-gaps
  --run-id "$run_id" --data-dir "$data_dir" --color never
)

started_epoch="$(date -u +%s)"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
"${command[@]}" >"$live_out" 2>"$live_err" &
child_pid=$!
signal_received=0

forward_signal() {
  signal_received=1
  kill -TERM "$child_pid" 2>/dev/null || true
}
trap forward_signal INT TERM HUP

sample_process() {
  local elapsed cpu rss_kib data_kib
  elapsed=$(($(date -u +%s) - started_epoch))
  read -r cpu rss_kib < <(ps -o %cpu= -o rss= -p "$child_pid" | awk 'NF >= 2 {print $1, $2}')
  cpu="${cpu:-0}"
  rss_kib="${rss_kib:-0}"
  data_kib="$(du -sk "$data_dir" 2>/dev/null | awk '{print $1}')"
  printf '%s\t%s\t%s\t%s\n' "$elapsed" "$cpu" "$((rss_kib * 1024))" "$((data_kib * 1024))" >>"$samples_tsv"
}

sample_process
next_sample=$sample_interval_secs
while kill -0 "$child_pid" 2>/dev/null; do
  sleep 1 &
  sleep_pid=$!
  wait "$sleep_pid" 2>/dev/null || true
  kill -0 "$child_pid" 2>/dev/null || break
  elapsed_now=$(($(date -u +%s) - started_epoch))
  if ((elapsed_now >= next_sample)); then
    sample_process
    next_sample=$((next_sample + sample_interval_secs))
  fi
done

wait "$child_pid"
live_exit=$?
ended_epoch="$(date -u +%s)"
ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
actual_duration=$((ended_epoch - started_epoch))
assert_source_unchanged

last_elapsed="$(tail -n 1 "$samples_tsv" | cut -f1)"
if [[ "$last_elapsed" != "$actual_duration" ]]; then
  sample_process
fi

replay_first_exit=1
replay_second_exit=1
if ((live_exit == 0)); then
  "$binary" replay --data-dir "$data_dir" --run-id "$run_id" --verify-parity \
    >"$replay_first_out" 2>"$replay_first_err"
  replay_first_exit=$?
  if ((replay_first_exit == 0)); then
    "$binary" replay --data-dir "$data_dir" --run-id "$run_id" --verify-parity \
      >"$replay_second_out" 2>"$replay_second_err"
    replay_second_exit=$?
  fi
fi
assert_source_unchanged

tmp_report="$report_path.tmp.$$"
python3 - "$tmp_report" "$samples_tsv" "$live_out" "$started_at" "$ended_at" \
  "$actual_duration" "$live_exit" "$commit" "$run_id" "$max_rss_growth_bytes" \
  "$replay_first_out" "$replay_second_out" "$replay_first_exit" "$replay_second_exit" \
  "$binary_sha256" "$runtime_source_sha256" "$rustc_version" "$cargo_version" "$host_triple" \
  "${command[@]}" <<'PY'
import json
import re
import sys
from pathlib import Path

(
    output_path, samples_path, live_path, started_at, ended_at, duration,
    live_exit, commit, run_id, max_rss_growth, replay_first_path,
    replay_second_path, replay_first_exit, replay_second_exit,
    binary_sha256, runtime_source_sha256, rustc_version, cargo_version,
    host_triple, *command
) = sys.argv[1:]

def read(path):
    candidate = Path(path)
    return candidate.read_text(encoding="utf-8") if candidate.exists() else ""

def value(text, key, default=0):
    matches = re.findall(rf"(?m)^{re.escape(key)}=(\d+)$", text)
    return int(matches[-1]) if matches else default

def status(text):
    matches = re.findall(r"(?m)^replay_parity=([a-z_]+)$", text)
    return matches[-1] if matches else "failed"

samples = []
for line in read(samples_path).splitlines():
    elapsed, cpu, rss, data = line.split("\t")
    sample = {
        "elapsed_secs": int(elapsed), "cpu_percent": float(cpu),
        "rss_bytes": int(rss), "data_bytes": int(data),
    }
    if samples and sample["elapsed_secs"] == samples[-1]["elapsed_secs"]:
        samples[-1] = sample
    else:
        samples.append(sample)

live = read(live_path)
first = read(replay_first_path)
second = read(replay_second_path)
data_gaps = value(live, "data_gaps")
report = {
    "schema_version": 2,
    "run_id": run_id,
    "commit": commit,
    "git_dirty": False,
    "binary_sha256": binary_sha256,
    "runtime_source_sha256": runtime_source_sha256,
    "build": {
        "rustc": rustc_version,
        "cargo": cargo_version,
        "host": host_triple,
    },
    "command": command,
    "started_at": started_at,
    "ended_at": ended_at,
    "duration_secs": int(duration),
    "exit_code": int(live_exit),
    "clean_shutdown": int(live_exit) == 0 and "clean_shutdown=true" in live,
    "samples": samples,
    "summary": {
        "symbols": value(live, "symbols"),
        "subscriptions": value(live, "subscriptions"),
        "ws_messages": value(live, "ws_messages"),
        "market_events": value(live, "market_events"),
        "reconnects": value(live, "reconnects"),
        "data_gaps": data_gaps,
        "unrepaired_gaps": data_gaps,
        "parser_drops": value(live, "parser_drops"),
        "backfill_requests_failed": value(live, "requests_failed"),
    },
    "replay": {
        "first_status": status(first) if int(replay_first_exit) == 0 else "failed",
        "second_status": status(second) if int(replay_second_exit) == 0 else "failed",
        "drift": value(second, "confidence_drift"),
        "missing": value(second, "confidence_missing"),
        "extra": value(second, "confidence_extra"),
    },
    "limits": {"max_rss_growth_bytes": int(max_rss_growth)},
}
Path(output_path).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
PY
generator_exit=$?
if ((generator_exit != 0)); then
  rm -f "$tmp_report"
  exit "$generator_exit"
fi
mv "$tmp_report" "$report_path"
assert_source_unchanged

minimum_duration="$duration_secs"
python3 "$repo_root/scripts/validate-soak-report.py" "$report_path" \
  --minimum-duration-secs "$minimum_duration" \
  --binary "$binary"
validation_exit=$?
echo "soak_report_path=$report_path"
if ((signal_received != 0)); then
  echo "soak interrupted by operator signal" >&2
fi
exit "$validation_exit"

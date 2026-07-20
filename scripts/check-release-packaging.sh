#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$repo_root/target"

HLS_REPO_ROOT="$repo_root" cargo test -p hls-cli --test release_packaging --locked

python3 "$repo_root/scripts/harden-generated-release-workflow.py" \
  --check \
  --workflow "$repo_root/.github/workflows/release.yml"

python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-valid.json" \
  --minimum-duration-secs 900
python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/docs/evidence/soak/sota-allpairs-20260713-15m.json" \
  --minimum-duration-secs 900
evidence_runtime_source_sha256="$(python3 -c \
  'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["runtime_source_sha256"])' \
  "$repo_root/docs/evidence/soak/sota-allpairs-20260713-15m.json")"
current_runtime_source_sha256="$(python3 "$repo_root/scripts/runtime-source-sha256.py" "$repo_root")"
if [[ "$evidence_runtime_source_sha256" != "$current_runtime_source_sha256" ]]; then
  echo "soak evidence runtime_source_sha256 does not match the reviewed runtime source" >&2
  exit 1
fi
if python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-invalid.json" \
  --minimum-duration-secs 900; then
  echo "invalid soak fixture unexpectedly passed" >&2
  exit 1
fi
if python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-invalid-command.json" \
  --minimum-duration-secs 900; then
  echo "non-live soak command fixture unexpectedly passed" >&2
  exit 1
fi

"$repo_root/scripts/check-supervisor-packaging.sh"
"$repo_root/scripts/check-public-readiness.sh"
"$repo_root/scripts/local-release-artifact-smoke.sh"

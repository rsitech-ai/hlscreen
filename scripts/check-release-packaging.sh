#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$repo_root/target"

HLS_REPO_ROOT="$repo_root" cargo test -p hls-cli --test release_packaging --locked

python3 "$repo_root/scripts/harden-generated-release-workflow.py" \
  --check \
  --workflow "$repo_root/.github/workflows/release.yml"

python3 "$repo_root/scripts/test-soak-validation.py"
python3 "$repo_root/scripts/test-dist-local-artifact.py"
python3 "$repo_root/scripts/test-mfop5-smoke.py"

python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-valid.json" \
  --minimum-duration-secs 900
python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/docs/evidence/soak/shared-feed-20261019-15m.json" \
  --minimum-duration-secs 900
# This committed receipt is historical evidence for its recorded macOS binary,
# not a soak qualification for today's platform/build. Exact --binary and
# --runtime-source-root bindings are exercised by test-soak-validation.py;
# candidate qualification uses both explicitly (see docs/deployment.md).
echo "soak_evidence_scope=historical_report_only current_candidate_qualified=false"
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

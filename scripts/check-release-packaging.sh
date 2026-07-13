#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$repo_root/target"

HLS_REPO_ROOT="$repo_root" rustc \
  --edition=2024 \
  --test "$repo_root/tests/integration/release_packaging.rs" \
  -o "$repo_root/target/release_packaging_test"

HLS_REPO_ROOT="$repo_root" "$repo_root/target/release_packaging_test"

python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-valid.json" \
  --minimum-duration-secs 900
if python3 "$repo_root/scripts/validate-soak-report.py" \
  "$repo_root/tests/fixtures/operations/soak-report-invalid.json" \
  --minimum-duration-secs 900; then
  echo "invalid soak fixture unexpectedly passed" >&2
  exit 1
fi

"$repo_root/scripts/check-public-readiness.sh"
"$repo_root/scripts/local-release-artifact-smoke.sh"

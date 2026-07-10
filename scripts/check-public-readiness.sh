#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_files=(
  "LICENSE"
  "README.md"
  "CONTRIBUTING.md"
  "CODE_OF_CONDUCT.md"
  "SECURITY.md"
  "SUPPORT.md"
  "CHANGELOG.md"
  "docs/RELEASING.md"
  "docs/ROADMAP.md"
  "docs/OPEN_SOURCE_CHECKLIST.md"
  "docs/PRIVACY.md"
  "docs/THREAT_MODEL.md"
  "docs/architecture.md"
  "docs/deployment.md"
  "docs/production-readiness.md"
  "docs/assets/screenshots/live-screen.svg"
  ".github/workflows/ci.yml"
  ".github/workflows/release.yml"
  ".github/pull_request_template.md"
  ".github/ISSUE_TEMPLATE/bug_report.yml"
  ".github/ISSUE_TEMPLATE/feature_request.yml"
)

for path in "${required_files[@]}"; do
  if [[ ! -s "$path" ]]; then
    echo "missing required public-readiness file: $path" >&2
    exit 1
  fi
done

require_text() {
  local needle="$1"
  local path="$2"
  if ! grep -q "$needle" "$path"; then
    echo "missing required text in $path: $needle" >&2
    exit 1
  fi
}

require_text "Release tag created" docs/OPEN_SOURCE_CHECKLIST.md
require_text "Release binaries/checksums published" docs/OPEN_SOURCE_CHECKLIST.md
require_text "Draft/local proof only" docs/ROADMAP.md
require_text "no reviewed \`v\*\` release artifact publication" docs/ROADMAP.md
require_text "Release Artifact Status" docs/RELEASING.md
require_text "not a published release" docs/RELEASING.md
require_text "does not currently provide a production daemon" docs/deployment.md

if grep -RInE "wallet_enabled[[:space:]]*=[[:space:]]*true|trading_enabled[[:space:]]*=[[:space:]]*true|guaranteed profit|profit guaranteed|place orders for you|private_key[[:space:]]*=" \
  README.md docs .github \
  | grep -v "Search for" \
  >/tmp/hlscreen-public-readiness-findings.txt; then
  echo "public-readiness wording scan found possible unsafe claims:" >&2
  cat /tmp/hlscreen-public-readiness-findings.txt >&2
  exit 1
fi

echo "public_readiness=passed"

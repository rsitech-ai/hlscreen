#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
scan_dir="$(mktemp -d "${TMPDIR:-/tmp}/hlscreen-public-readiness.XXXXXX")"
trap 'rm -rf "$scan_dir"' EXIT

required_files=(
  "LICENSE"
  "README.md"
  "CONTRIBUTING.md"
  "CODE_OF_CONDUCT.md"
  "SECURITY.md"
  "SUPPORT.md"
  "CHANGELOG.md"
  "deny.toml"
  "docs/RELEASING.md"
  "docs/ROADMAP.md"
  "docs/OPEN_SOURCE_CHECKLIST.md"
  "docs/PRIVACY.md"
  "docs/THREAT_MODEL.md"
  "docs/architecture.md"
  "docs/deployment.md"
  "docs/production-readiness.md"
  "docs/assets/screenshots/live-screen.svg"
  "scripts/harden-generated-release-workflow.py"
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

set +e
grep -RInE "wallet_enabled[[:space:]]*=[[:space:]]*true|trading_enabled[[:space:]]*=[[:space:]]*true|guaranteed profit|profit guaranteed|place orders for you|private_key[[:space:]]*=" \
  README.md docs .github >"$scan_dir/unsafe-wording-raw.txt"
wording_status=$?
set -e
if (( wording_status > 1 )); then
  echo "public-readiness wording scan failed" >&2
  exit 1
fi
if grep -v "Search for" "$scan_dir/unsafe-wording-raw.txt" \
  >"$scan_dir/unsafe-wording.txt"; then
  echo "public-readiness wording scan found possible unsafe claims:" >&2
  cat "$scan_dir/unsafe-wording.txt" >&2
  exit 1
fi

# Build path literals in pieces so this scanner does not flag its own source.
private_path_pattern="/""Users/[^/$<{[:space:]]+|/private""/tmp/hlscreen"
set +e
git grep -n -E -e "$private_path_pattern" >"$scan_dir/private-paths-raw.txt"
private_path_status=$?
set -e
if (( private_path_status > 1 )); then
  echo "public-readiness filesystem path scan failed" >&2
  exit 1
fi
if grep -v '/Users/YOUR_USER' "$scan_dir/private-paths-raw.txt" \
  >"$scan_dir/private-paths.txt"; then
  echo "public-readiness scan found developer-specific filesystem paths:" >&2
  cat "$scan_dir/private-paths.txt" >&2
  exit 1
fi

credential_pattern='-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}'
set +e
git grep -n -E -e "$credential_pattern" >"$scan_dir/credentials.txt"
credential_status=$?
set -e
if (( credential_status > 1 )); then
  echo "public-readiness credential scan failed" >&2
  exit 1
fi
if (( credential_status == 0 )); then
  echo "public-readiness scan found a possible committed credential:" >&2
  exit 1
fi

if ! git log -p --all --no-ext-diff --no-textconv \
  >"$scan_dir/history.patch"; then
  echo "public-readiness scan could not inspect Git history" >&2
  exit 1
fi
set +e
grep -E -e "$credential_pattern" "$scan_dir/history.patch" \
  >"$scan_dir/history-credentials.txt"
history_credential_status=$?
set -e
if (( history_credential_status > 1 )); then
  echo "public-readiness Git history credential scan failed" >&2
  exit 1
fi
if (( history_credential_status == 0 )); then
  echo "public-readiness scan found a possible credential in Git history" >&2
  exit 1
fi

echo "public_readiness=passed"

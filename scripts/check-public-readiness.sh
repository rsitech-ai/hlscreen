#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
scan_dir="$(mktemp -d "${TMPDIR:-/tmp}/hlscreen-public-readiness.XXXXXX")"
trap 'rm -rf "$scan_dir"' EXIT

required_files=(
  "LICENSE"
  "THIRD_PARTY_LICENSES.txt"
  "THIRD_PARTY_NOTICES.md"
  "third_party/spec-kit/LICENSE"
  "third_party/notices/manifest.json"
  "README.md"
  "CONTRIBUTING.md"
  "CODE_OF_CONDUCT.md"
  "SECURITY.md"
  "SUPPORT.md"
  "CHANGELOG.md"
  "deny.toml"
  "docs/RELEASING.md"
  "docs/DEVELOPMENT_TOOLING.md"
  "docs/OPEN_SOURCE_AUDIT.md"
  "docs/ROADMAP.md"
  "docs/OPEN_SOURCE_CHECKLIST.md"
  "docs/PRIVACY.md"
  "docs/THREAT_MODEL.md"
  "docs/architecture.md"
  "docs/deployment.md"
  "docs/production-readiness.md"
  "docs/assets/screenshots/live-screen.svg"
  "docs/evidence/soak/sota-allpairs-20260713-15m.json"
  "tests/fixtures/README.md"
  "scripts/check.sh"
  "scripts/check-history-secrets.sh"
  "scripts/check-public-surface.sh"
  "scripts/test-public-surface-gate.py"
  "scripts/summarize-git-identities.py"
  "scripts/summarize-git-history-privacy.py"
  "scripts/test-history-privacy.py"
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
  if ! grep -q -- "$needle" "$path"; then
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
require_text "independent open-source project" README.md
require_text "No Contributor License Agreement (CLA) is required" CONTRIBUTING.md
require_text "Synthetic and minimized fixtures" tests/fixtures/README.md
require_text "developer-only" docs/DEVELOPMENT_TOOLING.md
require_text "GitHub billing/spending" docs/OPEN_SOURCE_AUDIT.md
require_text "Private vulnerability reporting" docs/OPEN_SOURCE_CHECKLIST.md
require_text "Branch decision:" docs/OPEN_SOURCE_AUDIT.md
require_text "PR decision:" docs/OPEN_SOURCE_AUDIT.md
require_text "RETIRE_BEFORE_PUBLIC" docs/OPEN_SOURCE_AUDIT.md
require_text "CLOSE_BEFORE_PUBLIC" docs/OPEN_SOURCE_AUDIT.md
require_text "INTEGRATED_IN_CLOSEOUT_CLOSE_BEFORE_PUBLIC" docs/OPEN_SOURCE_AUDIT.md
require_text "Owner confirmation: Packages inventory checked in GitHub UI." docs/OPEN_SOURCE_AUDIT.md
require_text "Git commit-author metadata exposure accepted" docs/OPEN_SOURCE_AUDIT.md
require_text "Historical developer-path and non-public email" docs/OPEN_SOURCE_AUDIT.md
require_text "DELETE_NON_CANDIDATE_RUNS_BEFORE_PUBLIC" docs/OPEN_SOURCE_AUDIT.md

public_text_paths=(
  README.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md SUPPORT.md CHANGELOG.md
  .github/ISSUE_TEMPLATE docs/README.md docs/DEVELOPMENT_TOOLING.md
  docs/OPEN_SOURCE_CHECKLIST.md docs/PRIVACY.md docs/RELEASING.md docs/ROADMAP.md
  docs/THREAT_MODEL.md docs/architecture.md docs/data-format.md docs/deployment.md
  docs/production-readiness.md tests/fixtures/README.md
)

placeholder_pattern='(^|[^[:alnum:]_])(TODO|TBD|FIXME|CHANGEME)([^[:alnum:]_]|$)|YOUR_(USER|NAME|EMAIL|ORG)|example\.(com|org)'
set +e
grep -RInE "$placeholder_pattern" "${public_text_paths[@]}" \
  >"$scan_dir/placeholders.txt"
placeholder_status=$?
set -e
if (( placeholder_status > 1 )); then
  echo "public-readiness placeholder scan failed" >&2
  exit 1
fi
if (( placeholder_status == 0 )); then
  echo "public-readiness scan found unresolved placeholder wording at:" >&2
  cut -d: -f1-2 "$scan_dir/placeholders.txt" >&2
  exit 1
fi

obsolete_contact_pattern='contact (the )?maintainer privately through GitHub|report (it )?privately through GitHub( Issues)?([.,]|$)|email the maintainer privately'
set +e
grep -RInEi "$obsolete_contact_pattern" SECURITY.md CODE_OF_CONDUCT.md SUPPORT.md \
  .github/ISSUE_TEMPLATE >"$scan_dir/obsolete-contact.txt"
obsolete_contact_status=$?
set -e
if (( obsolete_contact_status > 1 )); then
  echo "public-readiness contact-route scan failed" >&2
  exit 1
fi
if (( obsolete_contact_status == 0 )); then
  echo "public-readiness scan found an obsolete private-contact route at:" >&2
  cut -d: -f1-2 "$scan_dir/obsolete-contact.txt" >&2
  exit 1
fi

if git show-ref --verify --quiet refs/tags/v0.1.0; then
  :
elif grep -qE '^## \[?0\.1\.0\]? - [0-9]{4}-[0-9]{2}-[0-9]{2}' CHANGELOG.md; then
  echo "CHANGELOG.md dates 0.1.0 before refs/tags/v0.1.0 exists" >&2
  exit 1
fi

unsafe_wording_pattern="wallet_enabled[[:space:]]*=[[:space:]]*true|trading_enabled[[:space:]]*=[[:space:]]*true|guaranteed profit|profit guaranteed|place orders for you|private_key[[:space:]]*="
set +e
grep -RInE "$unsafe_wording_pattern" \
  README.md docs .github >"$scan_dir/unsafe-wording-raw.txt"
wording_status=$?
set -e
if (( wording_status > 1 )); then
  echo "public-readiness wording scan failed" >&2
  exit 1
fi
if grep -v "Search for" "$scan_dir/unsafe-wording-raw.txt" \
  >"$scan_dir/unsafe-wording.txt"; then
  echo "public-readiness wording scan found possible unsafe claims at:" >&2
  cut -d: -f1-2 "$scan_dir/unsafe-wording.txt" >&2
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
  echo "public-readiness scan found developer-specific filesystem paths at:" >&2
  cut -d: -f1-2 "$scan_dir/private-paths.txt" >&2
  exit 1
fi

credential_pattern='-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}'
set +e
git grep -l -E -e "$credential_pattern" >"$scan_dir/credentials.txt"
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

# Full-history content is intentionally not materialized here: a crash could
# leave an unredacted patch in temporary storage. The required history gate is
# scripts/check-history-secrets.sh, which pins gitleaks, scans all fetched refs,
# forces 100% redaction, and streams the same reachable history through a
# metadata-only privacy summarizer without emitting matched content.
require_text "--redact=100" scripts/check-history-secrets.sh
require_text "--log-opts=\"--all\"" scripts/check-history-secrets.sh
require_text "summarize-git-history-privacy.py" scripts/check-history-secrets.sh
require_text "history_privacy_metadata=" scripts/summarize-git-history-privacy.py

echo "public_readiness=passed"

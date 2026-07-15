#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

expected_version="8.30.1"
scan_dir="$(mktemp -d "${TMPDIR:-/tmp}/hlscreen-history-secrets.XXXXXX")"
scan_id="$(basename "$scan_dir" | tr -cd 'A-Za-z0-9_-')"
scan_namespace="refs/scan/public-readiness/$scan_id"
report="$scan_dir/gitleaks.json"

cleanup() {
  while IFS= read -r ref; do
    git update-ref -d "$ref"
  done < <(git for-each-ref --format='%(refname)' "$scan_namespace")
  rm -rf "$scan_dir"
}
trap cleanup EXIT

if ! command -v gitleaks >/dev/null 2>&1; then
  echo "gitleaks is required at exactly $expected_version" >&2
  exit 1
fi
actual_version="$(gitleaks version 2>/dev/null)"
if [[ "$actual_version" != "$expected_version" ]]; then
  echo "gitleaks is required at exactly $expected_version; found a different version" >&2
  exit 1
fi

# Fetch every hosted branch and pull-request head without checking out or executing
# any fetched content. These temporary refs make `--all` cover the hosted surface.
git fetch --quiet --force --prune origin \
  "+refs/heads/*:$scan_namespace/heads/*" \
  "+refs/pull/*/head:$scan_namespace/pulls/*" >/dev/null

remote_head_count="$(git for-each-ref --format='%(refname)' "$scan_namespace/heads" | wc -l | tr -d ' ')"
pull_head_count="$(git for-each-ref --format='%(refname)' "$scan_namespace/pulls" | wc -l | tr -d ' ')"
ref_count="$((remote_head_count + pull_head_count))"
commit_count="$(git rev-list --all --count)"
identity_summary="$(
  git log --all --format='%ae%x09%ce' \
    | python3 scripts/summarize-git-identities.py
)"

set +e
gitleaks git --redact=100 --no-banner --report-format=json \
  --report-path="$report" --log-opts="--all"
scan_status=$?
set -e

if (( scan_status == 0 )); then
  echo "history_secret_scan=passed tool=gitleaks version=$expected_version refs=$ref_count remote_heads=$remote_head_count pull_heads=$pull_head_count commits=$commit_count $identity_summary"
  exit 0
fi
if (( scan_status != 1 )); then
  echo "history secret scan failed to complete with gitleaks $expected_version" >&2
  exit 1
fi

# Gitleaks reports contain redacted findings in the temporary file. Emit only
# bounded metadata; never echo a matched fragment or a secret field.
python3 - "$report" "$scan_namespace" <<'PY'
import hashlib
import json
import pathlib
import re
import subprocess
import sys

report = pathlib.Path(sys.argv[1])
namespace = sys.argv[2]
try:
    findings = json.loads(report.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    print("history secret scan produced an unreadable metadata report", file=sys.stderr)
    raise SystemExit(2)

def clean(value: object, pattern: str, fallback: str) -> str:
    text = str(value or "")
    return text if re.fullmatch(pattern, text) else fallback

print(f"history_secret_scan=failed findings={len(findings)}", file=sys.stderr)
for finding in findings[:100]:
    detector = clean(finding.get("RuleID"), r"[A-Za-z0-9_.:-]{1,100}", "unknown")
    commit = clean(finding.get("Commit"), r"[0-9a-fA-F]{7,64}", "unknown")
    path_hash = hashlib.sha256(str(finding.get("File") or "").encode()).hexdigest()[:16]
    line = clean(finding.get("StartLine"), r"[0-9]{1,10}", "unknown")
    ref_count = 0
    if commit != "unknown":
        result = subprocess.run(
            ["git", "for-each-ref", "--contains", commit, "--format=%(refname)", namespace],
            check=False,
            capture_output=True,
            text=True,
        )
        ref_count = sum(
            1
            for ref in result.stdout.splitlines()
            if re.fullmatch(r"refs/scan/public-readiness/[A-Za-z0-9_./+-]{1,300}", ref)
        )
    print(
        f"detector={detector} ref_count={ref_count} commit={commit} "
        f"path_sha256={path_hash} line={line}",
        file=sys.stderr,
    )
if len(findings) > 100:
    print(f"findings_metadata_truncated={len(findings) - 100}", file=sys.stderr)
raise SystemExit(1)
PY

#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: scripts/check-public-surface.sh [private-candidate|public] <expected-sha>" >&2
}

if (( $# != 2 )); then
  usage
  exit 2
fi

mode="$1"
expected_sha="$2"
case "$mode" in
  private-candidate|public) ;;
  *)
    usage
    exit 2
    ;;
esac
if [[ ! "$expected_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "expected-sha must be a lowercase 40-character Git object ID" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
repo="${HLS_GITHUB_REPOSITORY:-s1korrrr/hlscreen}"
audit="docs/OPEN_SOURCE_AUDIT.md"
gh_bin="${HLS_GH_BIN:-gh}"

if [[ "$(git rev-parse HEAD)" != "$expected_sha" ]]; then
  echo "surface gate must run at the exact expected_sha" >&2
  exit 1
fi
if ! git diff --quiet || ! git diff --cached --quiet \
  || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
  echo "surface gate requires a clean candidate worktree" >&2
  exit 1
fi
if [[ "$(git rev-parse origin/main)" != "$(git rev-parse refs/remotes/origin/main)" ]] \
  || ! git merge-base --is-ancestor origin/main "$expected_sha"; then
  echo "candidate is not based on origin/main" >&2
  exit 1
fi
origin_url="$(git remote get-url origin)"
origin_repo="$(python3 - "$origin_url" <<'PY'
import re
import sys
from urllib.parse import urlparse

url = sys.argv[1]
if re.match(r"^[^@]+@github\.com:", url):
    path = url.split(":", 1)[1]
else:
    parsed = urlparse(url)
    path = parsed.path.lstrip("/") if parsed.hostname == "github.com" else ""
print(path.removesuffix(".git").strip("/"))
PY
)"
if [[ "$(printf '%s' "$origin_repo" | tr '[:upper:]' '[:lower:]')" \
  != "$(printf '%s' "$repo" | tr '[:upper:]' '[:lower:]')" ]]; then
  echo "origin does not match HLS_GITHUB_REPOSITORY" >&2
  exit 1
fi
if [[ ! -s "$audit" ]]; then
  echo "missing $audit" >&2
  exit 1
fi
if ! command -v "$gh_bin" >/dev/null 2>&1 \
  || ! "$gh_bin" auth status >/dev/null 2>&1; then
  echo "authenticated GitHub CLI is required" >&2
  exit 1
fi

# All hosted reads below use `gh api`; the script contains no mutation method.
HLS_GH_BIN="$gh_bin" python3 - "$mode" "$expected_sha" "$repo" "$audit" <<'PY'
from __future__ import annotations

import json
import io
import os
import re
import subprocess
import sys
import zipfile
from pathlib import Path
from urllib.parse import quote


mode, expected_sha, repo, audit_path = sys.argv[1:]
owner = repo.split("/", 1)[0]
audit = Path(audit_path).read_text(encoding="utf-8")
failures: list[str] = []
gh_bin = os.environ["HLS_GH_BIN"]


def api(
    endpoint: str,
    *,
    optional: bool = False,
    paginate: bool = False,
) -> object | None:
    command = [gh_bin, "api", endpoint]
    if paginate:
        command.extend(["--paginate", "--slurp"])
    result = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        if optional:
            return None
        failures.append(f"GitHub API read failed: {endpoint.split('?', 1)[0]}")
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        failures.append(f"GitHub API returned invalid JSON: {endpoint.split('?', 1)[0]}")
        return None


def list_result(endpoint: str, *, optional: bool = False) -> list[dict[str, object]]:
    value = api(endpoint, optional=optional, paginate=True)
    if value is None and optional:
        return []
    if not isinstance(value, list):
        failures.append(f"GitHub API did not return paginated list data: {endpoint.split('?', 1)[0]}")
        return []
    pages = value if all(isinstance(page, list) for page in value) else [value]
    return [
        item
        for page in pages
        if isinstance(page, list)
        for item in page
        if isinstance(item, dict)
    ]


def object_items(endpoint: str, key: str) -> tuple[list[dict[str, object]], int | None]:
    value = api(endpoint, paginate=True)
    if not isinstance(value, list):
        failures.append(f"GitHub API did not return paginated object data: {endpoint.split('?', 1)[0]}")
        return [], None
    pages = value if all(isinstance(page, dict) for page in value) else []
    if not pages:
        failures.append(f"GitHub API returned malformed paginated object data: {endpoint.split('?', 1)[0]}")
        return [], None
    items = [
        item
        for page in pages
        for item in (page.get(key) or [])
        if isinstance(item, dict)
    ]
    totals = [page.get("total_count") for page in pages if isinstance(page.get("total_count"), int)]
    expected_total = totals[0] if totals else None
    if expected_total is not None and len(items) != expected_total:
        failures.append(f"paginated inventory count mismatch: {endpoint.split('?', 1)[0]}")
    return items, expected_total


def status_read(endpoint: str) -> bool:
    result = subprocess.run(
        [gh_bin, "api", endpoint],
        check=False,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def api_zip(endpoint: str) -> bytes | None:
    result = subprocess.run(
        [gh_bin, "api", endpoint],
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        if b"HTTP 404" in result.stderr:
            return None
        failures.append(f"GitHub API binary read failed: {endpoint}")
        return None
    return result.stdout


metadata = api(f"repos/{repo}")
if not isinstance(metadata, dict):
    metadata = {}
expected_visibility = "private" if mode == "private-candidate" else "public"
if metadata.get("visibility") != expected_visibility:
    failures.append(f"repository visibility is not {expected_visibility}")
if metadata.get("default_branch") != "main":
    failures.append("default_branch is not main")
if metadata.get("has_pages") is not False:
    failures.append("Pages is enabled or could not be proven disabled")

branches = list_result(f"repos/{repo}/branches?per_page=100")
branch_names = sorted(str(branch.get("name", "")) for branch in branches)
main_branches = [branch for branch in branches if branch.get("name") == "main"]
hosted_main_sha = ""
if len(main_branches) == 1 and isinstance(main_branches[0].get("commit"), dict):
    hosted_main_sha = str(main_branches[0]["commit"].get("sha", ""))
local_origin_main = subprocess.run(
    ["git", "rev-parse", "refs/remotes/origin/main"],
    check=False,
    capture_output=True,
    text=True,
).stdout.strip()
if not re.fullmatch(r"[0-9a-f]{40}", hosted_main_sha) or hosted_main_sha != local_origin_main:
    failures.append("local origin/main does not match the hosted main SHA")
tags = list_result(f"repos/{repo}/tags?per_page=100")
for branch in branch_names:
    decision = rf"^- Branch decision: `{re.escape(branch)}` — (?:RETIRE_BEFORE_PUBLIC|MERGE_BEFORE_PUBLIC)\.$"
    if branch != "main" and not re.search(decision, audit, flags=re.MULTILINE):
        failures.append("a non-main branch lacks a final recorded surface decision")
if mode == "public" and branch_names != ["main"]:
    failures.append("public surface still has non-main branches")
if mode == "public" and hosted_main_sha != expected_sha:
    failures.append("public main does not point to expected_sha")
if tags:
    failures.append("candidate surface has tags before the first reviewed release")

open_pulls = list_result(f"repos/{repo}/pulls?state=open&per_page=100")
human_pulls = []
for pull in open_pulls:
    login = str((pull.get("user") or {}).get("login", "")) if isinstance(pull.get("user"), dict) else ""
    number = pull.get("number")
    if login != "dependabot[bot]":
        head = pull.get("head")
        base = pull.get("base")
        is_candidate_pull = (
            mode == "private-candidate"
            and isinstance(head, dict)
            and head.get("sha") == expected_sha
            and isinstance(base, dict)
            and base.get("ref") == "main"
        )
        if not is_candidate_pull:
            human_pulls.append(number)
    else:
        decision = (
            rf"^- PR decision: `#{number}` — "
            rf"(?:CLOSE_BEFORE_PUBLIC|INTEGRATED_IN_CLOSEOUT_CLOSE_BEFORE_PUBLIC)\.$"
        )
        if not re.search(decision, audit, flags=re.MULTILINE):
            failures.append("an open dependency PR lacks a final recorded surface decision")
if human_pulls:
    failures.append(f"open human pull requests remain: count={len(human_pulls)}")
if mode == "public" and open_pulls:
    failures.append("public surface still has open pull requests")
if mode == "private-candidate":
    candidate_branches = [
        branch for branch in branches
        if branch.get("name") != "main"
        and isinstance(branch.get("commit"), dict)
        and branch["commit"].get("sha") == expected_sha
    ]
    candidate_pulls = [
        pull for pull in open_pulls
        if isinstance(pull.get("head"), dict)
        and pull["head"].get("sha") == expected_sha
    ]
    if not candidate_branches and not candidate_pulls:
        failures.append("private candidate SHA is not hosted on a branch or open pull request")

all_runs, _ = object_items(f"repos/{repo}/actions/runs?per_page=100", "workflow_runs")
stale_runs = [run for run in all_runs if run.get("head_sha") != expected_sha]
if stale_runs:
    failures.append(
        "historical Actions runs/logs outside expected_sha remain: "
        f"count={len(stale_runs)}"
    )
runs, _ = object_items(f"repos/{repo}/actions/runs?head_sha={expected_sha}&per_page=100", "workflow_runs")
ci_runs = [run for run in runs if isinstance(run, dict) and run.get("name") == "CI"]
successful_ci = [
    run for run in ci_runs
    if run.get("head_sha") == expected_sha
    and run.get("status") == "completed"
    and run.get("conclusion") == "success"
    and (
        (mode == "public" and run.get("event") == "push" and run.get("head_branch") == "main")
        or (mode == "private-candidate" and run.get("event") in ("push", "pull_request"))
    )
]
if not successful_ci:
    failures.append("no successful hosted CI run exists at expected_sha")
else:
    run_id = successful_ci[0].get("id")
    jobs, _ = object_items(f"repos/{repo}/actions/runs/{run_id}/jobs?filter=all&per_page=100", "jobs")
    expected_ci_conclusions = {
        "GitHub Actions security": "success",
        "RustSec advisory scan": "success",
        "Dependency license and source policy": "success",
        "PTY TUI (ubuntu-24.04)": "success",
        "PTY TUI (macos-15)": "success",
        "Rust workspace": "success",
        "Scheduled public API contract smoke": "skipped",
    }
    jobs_by_name = {
        str(job.get("name")): job
        for job in jobs
        if isinstance(job, dict)
    }
    if (
        set(jobs_by_name) != set(expected_ci_conclusions)
        or len(jobs) != len(expected_ci_conclusions)
    ):
        failures.append("hosted CI job inventory is not exact at expected_sha")
    bad_required_jobs = [
        name for name, conclusion in sorted(expected_ci_conclusions.items())
        if name not in jobs_by_name
        or jobs_by_name[name].get("conclusion") != conclusion
        or (conclusion == "success" and not jobs_by_name[name].get("steps"))
    ]
    if bad_required_jobs:
        failures.append(
            "required hosted CI jobs did not all execute successfully at expected_sha: "
            f"count={len(bad_required_jobs)}"
        )

release_runs = [run for run in runs if isinstance(run, dict) and run.get("name") == "Release"]
successful_release = [
    run for run in release_runs
    if run.get("head_sha") == expected_sha
    and run.get("status") == "completed"
    and run.get("conclusion") == "success"
    and run.get("event") == "pull_request"
]
release_run_id: int | None = None
if not successful_release:
    failures.append("no successful hosted Release run exists at expected_sha")
else:
    release_run = max(successful_release, key=lambda run: int(run.get("id") or 0))
    candidate_release_run_id = release_run.get("id")
    if isinstance(candidate_release_run_id, int):
        release_run_id = candidate_release_run_id
    release_jobs, _ = object_items(
        f"repos/{repo}/actions/runs/{release_run_id}/jobs?filter=all&per_page=100",
        "jobs",
    )
    expected_release_conclusions = {
        "Plan release": "success",
        "build-local-artifacts (aarch64-apple-darwin)": "success",
        "build-local-artifacts (x86_64-apple-darwin)": "success",
        "build-local-artifacts (x86_64-pc-windows-msvc)": "success",
        "build-local-artifacts (x86_64-unknown-linux-gnu)": "success",
        "Build global artifacts": "success",
        "Publish tag artifacts": "skipped",
        "Confirm release announcement": "skipped",
    }
    release_jobs_by_name = {
        str(job.get("name")): job
        for job in release_jobs
        if isinstance(job, dict)
    }
    if (
        set(release_jobs_by_name) != set(expected_release_conclusions)
        or len(release_jobs) != len(expected_release_conclusions)
    ):
        failures.append("hosted Release job inventory is not exact at expected_sha")
    bad_release_jobs = [
        name
        for name, conclusion in expected_release_conclusions.items()
        if name not in release_jobs_by_name
        or release_jobs_by_name[name].get("conclusion") != conclusion
        or (
            conclusion == "success"
            and not release_jobs_by_name[name].get("steps")
        )
    ]
    if bad_release_jobs:
        failures.append(
            "required hosted Release jobs did not all execute successfully at expected_sha: "
            f"count={len(bad_release_jobs)}"
        )

collaborators = list_result(f"repos/{repo}/collaborators?affiliation=all&per_page=100")
if len(collaborators) != 1 or collaborators[0].get("login") != owner \
        or collaborators[0].get("role_name") != "admin":
    failures.append("collaborator inventory differs from the sole expected owner/admin")
if list_result(f"repos/{repo}/hooks?per_page=100"):
    failures.append("webhooks are configured")
if list_result(f"repos/{repo}/keys?per_page=100"):
    failures.append("deploy keys are configured")

for label, endpoint, key in [
    ("Actions secrets", f"repos/{repo}/actions/secrets?per_page=100", "secrets"),
    ("Actions variables", f"repos/{repo}/actions/variables?per_page=100", "variables"),
    ("Dependabot secrets", f"repos/{repo}/dependabot/secrets?per_page=100", "secrets"),
    ("Codespaces secrets", f"repos/{repo}/codespaces/secrets?per_page=100", "secrets"),
    ("environments", f"repos/{repo}/environments?per_page=100", "environments"),
]:
    inventory, total = object_items(endpoint, key)
    if inventory or total != 0:
        failures.append(f"{label} inventory is non-empty or unreadable")

if list_result(f"repos/{repo}/deployments?per_page=100"):
    failures.append("deployments exist")
if list_result(f"repos/{repo}/releases?per_page=100"):
    failures.append("releases exist before the first reviewed release")
artifacts, _ = object_items(f"repos/{repo}/actions/artifacts?per_page=100", "artifacts")
active_artifacts = [artifact for artifact in artifacts if isinstance(artifact, dict) and not artifact.get("expired")]
expected_candidate_artifacts = {
    "cargo-dist-cache",
    "artifacts-plan-dist-manifest",
    "artifacts-build-local-aarch64-apple-darwin",
    "artifacts-build-local-x86_64-apple-darwin",
    "artifacts-build-local-x86_64-pc-windows-msvc",
    "artifacts-build-local-x86_64-unknown-linux-gnu",
    "artifacts-build-global",
}
if mode == "private-candidate":
    candidate_artifact_names = {
        str(artifact.get("name"))
        for artifact in active_artifacts
        if isinstance(artifact.get("workflow_run"), dict)
        and artifact["workflow_run"].get("id") == release_run_id
        and artifact["workflow_run"].get("head_sha") == expected_sha
    }
    if (
        candidate_artifact_names != expected_candidate_artifacts
        or len(active_artifacts) != len(expected_candidate_artifacts)
    ):
        failures.append("candidate Release artifact inventory is not exact")
elif active_artifacts:
    failures.append(f"unexpired Actions artifacts remain: count={len(active_artifacts)}")

associated_packages = 0
for package_type in ("container", "npm", "maven", "rubygems", "nuget"):
    packages = list_result(
        f"users/{owner}/packages?package_type={quote(package_type)}&per_page=100",
        optional=True,
    )
    if not packages and api(
        f"users/{owner}/packages?package_type={quote(package_type)}&per_page=1",
        optional=True,
    ) is None:
        continue
    associated_packages += sum(
        1 for package in packages
        if isinstance(package.get("repository"), dict)
        and package["repository"].get("full_name") == repo
    )
if associated_packages:
    failures.append(f"repository-associated packages exist: count={associated_packages}")
if "- [x] Owner confirmation: Packages inventory checked in GitHub UI." not in audit:
    failures.append("Packages inventory needs owner UI confirmation")

for marker in [
    "- [x] Owner confirmation: Private advisory drafts checked",
    "- [x] Owner confirmation: info@rsitech.ai monitoring checked",
    "- [x] Owner confirmation: Git commit-author metadata exposure accepted",
    "- [x] Owner confirmation: Historical developer-path and non-public email",
]:
    if marker not in audit:
        failures.append(marker.removeprefix("- [x] ") + " is incomplete")

# Scan hosted text in memory. Emit only resource kind and numeric identifier,
# never body text, matches, emails, tokens, secret names, or variable values.
text_hits: list[tuple[str, object]] = []
developer_home_pattern = r"/" r"Users/[^\s`]+"
credential = re.compile(
    r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY|"
    r"gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|"
    r"sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|"
    + developer_home_pattern
    + r"|/private" r"/tmp/hlscreen|"
    + r"(?<![A-Za-z0-9._%+-])[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?![A-Za-z0-9])"
)

def scan(kind: str, identifier: object, *values: object) -> None:
    if any(credential.search(str(value or "")) for value in values):
        text_hits.append((kind, identifier))

all_pulls = list_result(f"repos/{repo}/pulls?state=all&per_page=100")
for pull in all_pulls:
    number = pull.get("number")
    scan("pull", number, pull.get("title"), pull.get("body"))
    reviews = list_result(f"repos/{repo}/pulls/{number}/reviews?per_page=100")
    for review in reviews:
        scan("review", review.get("id"), review.get("body"))
for comment in list_result(f"repos/{repo}/issues/comments?per_page=100"):
    scan("issue-comment", comment.get("id"), comment.get("body"))
for comment in list_result(f"repos/{repo}/pulls/comments?per_page=100"):
    scan("review-comment", comment.get("id"), comment.get("body"))
for issue in list_result(f"repos/{repo}/issues?state=all&per_page=100"):
    scan("issue", issue.get("number"), issue.get("title"), issue.get("body"))
for comment in list_result(f"repos/{repo}/comments?per_page=100"):
    scan("commit-comment", comment.get("id"), comment.get("body"))
discussions = list_result(
    f"repos/{repo}/discussions?per_page=100",
    optional=mode != "public",
)
for discussion in discussions:
    number = discussion.get("number")
    scan("discussion", number, discussion.get("title"), discussion.get("body"))
    discussion_comments = list_result(
        f"repos/{repo}/discussions/{number}/comments?per_page=100"
    )
    for comment in discussion_comments:
        child_count = comment.get("child_comment_count")
        scan("discussion-comment", comment.get("id"), comment.get("body"))
        if isinstance(child_count, int) and child_count > 0:
            for reply in list_result(
                f"repos/{repo}/discussions/{number}/comments/{comment.get('id')}/replies?per_page=100"
            ):
                scan("discussion-reply", reply.get("id"), reply.get("body"))
if text_hits:
    for kind, identifier in text_hits[:20]:
        print(f"hosted_text_hit kind={kind} id={identifier}", file=sys.stderr)
    failures.append(f"hosted text privacy scan found suspicious resources: count={len(text_hits)}")

# Historical runs must be removed before this point. Scan only retained
# exact-candidate logs in memory; do not write log archives or matched text.
log_hits: list[object] = []
for run in all_runs:
    run_id = run.get("id")
    if run.get("head_sha") != expected_sha or not isinstance(run_id, int):
        continue
    payload = api_zip(f"repos/{repo}/actions/runs/{run_id}/logs")
    if payload is None:
        continue
    try:
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:
            members = [member for member in archive.infolist() if not member.is_dir()]
            if sum(member.file_size for member in members) > 50_000_000:
                failures.append(f"candidate Actions log archive is unexpectedly large: run_id={run_id}")
                continue
            for member in members:
                text = archive.read(member).decode("utf-8", errors="replace")
                text = re.sub(
                    r"/" r"Users/(?:runner|runneradmin)/",
                    "/ci-runner/",
                    text,
                )
                if credential.search(text):
                    log_hits.append(run_id)
                    break
    except (OSError, zipfile.BadZipFile, RuntimeError):
        failures.append(f"candidate Actions log archive is unreadable: run_id={run_id}")
if log_hits:
    for run_id in log_hits[:20]:
        print(f"actions_log_hit run_id={run_id}", file=sys.stderr)
    failures.append(f"candidate Actions logs contain suspicious content: count={len(log_hits)}")

actions_permissions = api(f"repos/{repo}/actions/permissions")
workflow_permissions = api(f"repos/{repo}/actions/permissions/workflow")
if not isinstance(workflow_permissions, dict) \
        or workflow_permissions.get("default_workflow_permissions") != "read" \
        or workflow_permissions.get("can_approve_pull_request_reviews") is not False:
    failures.append("default GITHUB_TOKEN permissions are not read-only/fail-closed")

if mode == "public":
    expected_required_checks = {
        "GitHub Actions security",
        "RustSec advisory scan",
        "Dependency license and source policy",
        "PTY TUI (ubuntu-24.04)",
        "PTY TUI (macos-15)",
        "Rust workspace",
    }

    def required_check_names(values: object) -> set[str]:
        if not isinstance(values, list):
            return set()
        names: set[str] = set()
        for value in values:
            if isinstance(value, str):
                names.add(value)
            elif isinstance(value, dict) and isinstance(value.get("context"), str):
                names.add(value["context"])
        return names

    if metadata.get("has_discussions") is not True:
        failures.append("Discussions Q&A is not enabled")
    if (
        "- [x] Owner confirmation: Discussions and its answerable Q&A category are enabled"
    ) not in audit:
        failures.append("Discussions Q&A owner confirmation is incomplete")
    if (
        "- [x] Owner confirmation: private vulnerability reporting enabled before public launch"
    ) not in audit:
        failures.append("Private vulnerability reporting owner confirmation is incomplete")
    selected_actions = api(f"repos/{repo}/actions/permissions/selected-actions")
    expected_action_patterns = {
        "actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10",
        "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
        "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
        "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
        "actions/attest@a1948c3f048ba23858d222213b7c278aabede763",
        "astral-sh/setup-uv@11f9893b081a58869d3b5fccaea48c9e9e46f990",
    }
    if (
        not isinstance(actions_permissions, dict)
        or actions_permissions.get("enabled") is not True
        or actions_permissions.get("allowed_actions") != "selected"
        or actions_permissions.get("sha_pinning_required") is not True
        or not isinstance(selected_actions, dict)
        or selected_actions.get("github_owned_allowed") is not False
        or selected_actions.get("verified_allowed") is not False
        or set(selected_actions.get("patterns_allowed") or []) != expected_action_patterns
    ):
        failures.append("Actions policy is not the exact audited SHA allowlist")
    rulesets = list_result(
        f"repos/{repo}/rulesets?includes_parents=true&per_page=100",
        optional=True,
    )
    active_main_ruleset = False
    for summary in rulesets:
        ruleset_id = summary.get("id")
        detail = api(f"repos/{repo}/rulesets/{ruleset_id}")
        if not isinstance(detail, dict) or detail.get("enforcement") != "active":
            continue
        conditions = detail.get("conditions") or {}
        ref_name = conditions.get("ref_name") if isinstance(conditions, dict) else None
        include = ref_name.get("include", []) if isinstance(ref_name, dict) else []
        exclude = ref_name.get("exclude", []) if isinstance(ref_name, dict) else []
        targets_main = "~DEFAULT_BRANCH" in include or "refs/heads/main" in include
        rules = detail.get("rules") or []
        rule_types = {
            str(rule.get("type"))
            for rule in rules
            if isinstance(rule, dict)
        }
        pull_rules = [
            rule for rule in rules
            if isinstance(rule, dict) and rule.get("type") == "pull_request"
        ]
        status_rules = [
            rule for rule in rules
            if isinstance(rule, dict) and rule.get("type") == "required_status_checks"
        ]
        has_pull_contract = bool(pull_rules) and all(
            isinstance(rule.get("parameters"), dict)
            and isinstance(rule["parameters"].get("required_approving_review_count"), int)
            and rule["parameters"].get("required_review_thread_resolution") is True
            for rule in pull_rules
        )
        has_status_contract = bool(status_rules) and all(
            isinstance(rule.get("parameters"), dict)
            and expected_required_checks == required_check_names(
                rule["parameters"].get("required_status_checks")
            )
            and rule["parameters"].get("strict_required_status_checks_policy") is True
            for rule in status_rules
        )
        if (
            targets_main
            and not exclude
            and not detail.get("bypass_actors")
            and has_pull_contract
            and has_status_contract
            and {"deletion", "non_fast_forward"}.issubset(rule_types)
        ):
            active_main_ruleset = True
            break
    protection = api(f"repos/{repo}/branches/main/protection", optional=True)
    if isinstance(protection, dict):
        reviews = protection.get("required_pull_request_reviews")
        statuses = protection.get("required_status_checks")
        status_checks = []
        if isinstance(statuses, dict):
            status_checks = statuses.get("checks") or statuses.get("contexts") or []
        protection_ok = (
            isinstance(reviews, dict)
            and isinstance(reviews.get("required_approving_review_count"), int)
            and isinstance(statuses, dict)
            and statuses.get("strict") is True
            and expected_required_checks == required_check_names(status_checks)
            and isinstance(protection.get("enforce_admins"), dict)
            and protection["enforce_admins"].get("enabled") is True
            and isinstance(protection.get("required_conversation_resolution"), dict)
            and protection["required_conversation_resolution"].get("enabled") is True
            and isinstance(protection.get("allow_force_pushes"), dict)
            and protection["allow_force_pushes"].get("enabled") is False
            and isinstance(protection.get("allow_deletions"), dict)
            and protection["allow_deletions"].get("enabled") is False
        )
    else:
        protection_ok = False
    if not (active_main_ruleset or protection_ok):
        failures.append(
            "main policy does not enforce PRs, the exact strict CI checks, "
            "conversations, admin inclusion, and ref safety"
        )
    security = metadata.get("security_and_analysis")
    required_security = (
        "dependency_graph",
        "dependabot_security_updates",
        "secret_scanning",
        "secret_scanning_push_protection",
    )
    if not isinstance(security, dict) or any(
        not isinstance(security.get(name), dict)
        or security[name].get("status") != "enabled"
        for name in required_security
    ):
        failures.append("required security_and_analysis features are not enabled")
    default_setup = api(f"repos/{repo}/code-scanning/default-setup")
    if not isinstance(default_setup, dict) or default_setup.get("state") != "configured":
        failures.append("code scanning default setup is not configured")
    if not status_read(f"repos/{repo}/vulnerability-alerts"):
        failures.append("dependency graph and vulnerability alerts are not enabled")
    dependabot_alerts = api(
        f"repos/{repo}/dependabot/alerts?state=open&per_page=1",
        optional=True,
    )
    if not isinstance(dependabot_alerts, list):
        failures.append("Dependabot alerts endpoint is unavailable")
    private_reporting = api(f"repos/{repo}/private-vulnerability-reporting")
    if not isinstance(private_reporting, dict) or private_reporting.get("enabled") is not True:
        failures.append("Private vulnerability reporting is not enabled")

print(
    "public_surface_summary="
    f"mode:{mode} branches:{len(branches)} tags:{len(tags)} "
    f"open_prs:{len(open_pulls)} artifacts:{len(active_artifacts)} "
    f"runs:{len(all_runs)} collaborators:{len(collaborators)} "
    f"text_hits:{len(text_hits)} log_hits:{len(log_hits)}"
)
if failures:
    for failure in dict.fromkeys(failures):
        print(f"public_surface_blocker={failure}", file=sys.stderr)
    raise SystemExit(1)
print("public_surface=passed")
PY

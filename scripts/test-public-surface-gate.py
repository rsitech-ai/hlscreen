#!/usr/bin/env python3
"""Exercise the hosted-surface gate against deterministic mocked API payloads."""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import tempfile
import time
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
SURFACE_GATE = REPO_ROOT / "scripts/check-public-surface.sh"
CASE_COUNT = 0

FAKE_GH = r'''#!/usr/bin/env python3
import io
import json
import os
import sys
import time
import zipfile

scenario = os.environ.get("HLS_SURFACE_SCENARIO", "private_ok")
expected_sha = os.environ["HLS_EXPECTED_SHA"]
main_sha = os.environ["HLS_MAIN_SHA"]
args = sys.argv[1:]
if args == ["auth", "status"]:
    if scenario == "private_auth_timeout":
        time.sleep(2)
    raise SystemExit(0)
if len(args) < 2 or args[0] != "api":
    raise SystemExit(2)
endpoint = args[1]
slurp = "--slurp" in args
public = scenario.startswith("public")

if scenario == "private_timeout" and endpoint.startswith(
    "repos/s1korrrr/hlscreen/branches?"
):
    time.sleep(2)

def emit(value):
    print(json.dumps(value))

def direct_list(items):
    emit([items] if slurp else items)

if endpoint == "repos/s1korrrr/hlscreen":
    security = {}
    if public:
        security = {
            "dependency_graph": {"status": "enabled"},
            "dependabot_security_updates": {"status": "enabled"},
            "secret_scanning": {"status": "enabled"},
            "secret_scanning_push_protection": {"status": "enabled"},
        }
        if scenario == "public_security_disabled":
            security.pop("dependency_graph")
    emit({
        "visibility": "public" if public else "private",
        "default_branch": "main",
        "has_pages": False,
        "has_discussions": public,
        "security_and_analysis": security,
    })
elif endpoint.startswith("repos/s1korrrr/hlscreen/branches?"):
    hosted_main_sha = "0" * 40 if scenario == "private_stale_main" else main_sha
    main = {"name": "main", "commit": {"sha": hosted_main_sha}}
    if scenario == "private_second_page_branch":
        emit([[main], [{"name": "rogue-second-page", "commit": {"sha": "1" * 40}}]])
    elif public:
        direct_list([main])
    else:
        direct_list([
            main,
            {"name": "feat/andrzej_oss_full_closeout", "commit": {"sha": expected_sha}},
        ])
elif endpoint.startswith("repos/s1korrrr/hlscreen/tags?"):
    direct_list([])
elif endpoint.startswith("repos/s1korrrr/hlscreen/pulls?state=open"):
    pulls = [] if public else [{
        "number": 47,
        "user": {"login": "s1korrrr"},
        "head": {"sha": expected_sha},
        "base": {"ref": "main"},
    }]
    if scenario == "private_missing_pr_decision":
        pulls.append({"number": 99, "user": {"login": "dependabot[bot]"}})
    if scenario == "private_extra_human_pr":
        pulls.append({
            "number": 48,
            "user": {"login": "contributor"},
            "head": {"sha": "9" * 40},
            "base": {"ref": "main"},
        })
    direct_list(pulls)
elif endpoint.startswith("repos/s1korrrr/hlscreen/actions/runs?"):
    candidate_ci_run = {
        "id": 7,
        "name": "CI",
        "head_sha": expected_sha,
        "status": "completed",
        "conclusion": "success",
        "event": "push" if public else "pull_request",
        "head_branch": "main" if public else "feat/andrzej_oss_full_closeout",
    }
    candidate_release_run = {
        "id": 8,
        "name": "Release",
        "head_sha": expected_sha,
        "status": "completed",
        "conclusion": "failure" if scenario == "private_release_failure" else "success",
        "event": "pull_request",
        "head_branch": "feat/andrzej_oss_full_closeout",
    }
    candidate_runs = [candidate_ci_run]
    if scenario != "private_missing_release":
        candidate_runs.append(candidate_release_run)
    if "head_sha=" in endpoint:
        emit([{"total_count": len(candidate_runs), "workflow_runs": candidate_runs}])
    elif scenario == "private_second_page_run":
        emit([
            {"total_count": 3, "workflow_runs": candidate_runs},
            {"total_count": 3, "workflow_runs": [{
                "id": 99,
                "name": "CI",
                "head_sha": "2" * 40,
                "status": "completed",
                "conclusion": "success",
            }]},
        ])
    else:
        emit([{"total_count": len(candidate_runs), "workflow_runs": candidate_runs}])
elif endpoint.startswith("repos/s1korrrr/hlscreen/actions/runs/7/jobs?"):
    names = [
        "GitHub Actions security",
        "RustSec advisory scan",
        "Dependency license and source policy",
        "PTY TUI (ubuntu-24.04)",
        "PTY TUI (macos-15)",
        "Rust workspace",
    ]
    jobs = [
        {
            "id": index,
            "name": name,
            "conclusion": "success",
            "steps": [{"name": "test", "conclusion": "success"}],
        }
        for index, name in enumerate(names, start=8)
    ]
    jobs.append({
        "id": 98,
        "name": "Scheduled public API contract smoke",
        "conclusion": "skipped",
        "steps": [],
    })
    if scenario == "private_prestep_failure":
        jobs[-2]["conclusion"] = "failure"
        jobs[-2]["steps"] = []
    if scenario == "private_extra_ci_job":
        jobs.append({
            "id": 99,
            "name": "Unexpected CI job",
            "conclusion": "success",
            "steps": [{"name": "test", "conclusion": "success"}],
        })
    emit([{"total_count": len(jobs), "jobs": jobs}])
elif endpoint.startswith("repos/s1korrrr/hlscreen/actions/runs/8/jobs?"):
    conclusions = {
        "Plan release": "success",
        "build-local-artifacts (aarch64-apple-darwin)": "success",
        "build-local-artifacts (x86_64-apple-darwin)": "success",
        "build-local-artifacts (x86_64-pc-windows-msvc)": "success",
        "build-local-artifacts (x86_64-unknown-linux-gnu)": "success",
        "Build global artifacts": "success",
        "Publish tag artifacts": "skipped",
        "Confirm release announcement": "skipped",
    }
    if scenario == "private_release_job_failure":
        conclusions["Build global artifacts"] = "failure"
    jobs = [
        {
            "id": index,
            "name": name,
            "conclusion": conclusion,
            "steps": [] if conclusion == "skipped" else [{"name": "test", "conclusion": conclusion}],
        }
        for index, (name, conclusion) in enumerate(conclusions.items(), start=20)
    ]
    emit([{"total_count": len(jobs), "jobs": jobs}])
elif endpoint in {
    "repos/s1korrrr/hlscreen/actions/runs/7/logs",
    "repos/s1korrrr/hlscreen/actions/runs/8/logs",
}:
    payload = io.BytesIO()
    with zipfile.ZipFile(payload, "w") as archive:
        content = "build ok\n"
        if scenario == "private_sensitive_log":
            content = "masked fixture " + "gh" + "p_" + ("A" * 20) + "\n"
        archive.writestr("job.txt", content)
    sys.stdout.buffer.write(payload.getvalue())
elif endpoint.startswith("repos/s1korrrr/hlscreen/collaborators?"):
    direct_list([{"login": "s1korrrr", "role_name": "admin"}])
elif endpoint.startswith((
    "repos/s1korrrr/hlscreen/hooks?",
    "repos/s1korrrr/hlscreen/keys?",
    "repos/s1korrrr/hlscreen/deployments?",
    "repos/s1korrrr/hlscreen/releases?",
    "repos/s1korrrr/hlscreen/pulls?state=all",
    "repos/s1korrrr/hlscreen/issues/comments?",
    "repos/s1korrrr/hlscreen/pulls/comments?",
    "repos/s1korrrr/hlscreen/issues?",
    "repos/s1korrrr/hlscreen/comments?",
    "repos/s1korrrr/hlscreen/discussions?",
)):
    direct_list([])
elif "/reviews?" in endpoint:
    direct_list([])
elif endpoint.startswith((
    "repos/s1korrrr/hlscreen/actions/secrets?",
    "repos/s1korrrr/hlscreen/dependabot/secrets?",
    "repos/s1korrrr/hlscreen/codespaces/secrets?",
)):
    emit([{"total_count": 0, "secrets": []}])
elif endpoint.startswith("repos/s1korrrr/hlscreen/actions/variables?"):
    emit([{"total_count": 0, "variables": []}])
elif endpoint.startswith("repos/s1korrrr/hlscreen/environments?"):
    emit([{"total_count": 0, "environments": []}])
elif endpoint.startswith("repos/s1korrrr/hlscreen/actions/artifacts?"):
    expected_names = [
        "cargo-dist-cache",
        "artifacts-plan-dist-manifest",
        "artifacts-build-local-aarch64-apple-darwin",
        "artifacts-build-local-x86_64-apple-darwin",
        "artifacts-build-local-x86_64-pc-windows-msvc",
        "artifacts-build-local-x86_64-unknown-linux-gnu",
        "artifacts-build-global",
    ]
    artifacts = [
        {
            "id": index,
            "name": name,
            "expired": False,
            "workflow_run": {"id": 8, "head_sha": expected_sha},
        }
        for index, name in enumerate(expected_names, start=100)
    ]
    if public:
        artifacts = []
    if scenario == "private_bad_release_artifacts":
        artifacts.pop()
    if scenario == "private_second_page_artifact":
        emit([
            {"total_count": len(artifacts) + 1, "artifacts": artifacts},
            {"total_count": len(artifacts) + 1, "artifacts": [{
                "id": 12,
                "name": "rogue-artifact",
                "expired": False,
                "workflow_run": {"id": 8, "head_sha": expected_sha},
            }]},
        ])
    else:
        emit([{"total_count": len(artifacts), "artifacts": artifacts}])
elif endpoint.startswith("users/s1korrrr/packages?"):
    direct_list([])
elif endpoint == "repos/s1korrrr/hlscreen/actions/permissions":
    emit({"enabled": True, "allowed_actions": "selected", "sha_pinning_required": True})
elif endpoint == "repos/s1korrrr/hlscreen/actions/permissions/selected-actions":
    patterns = [
        "actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10",
        "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
        "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
        "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
        "actions/attest@a1948c3f048ba23858d222213b7c278aabede763",
        "astral-sh/setup-uv@11f9893b081a58869d3b5fccaea48c9e9e46f990",
    ]
    if scenario == "public_incomplete_actions":
        patterns.remove("actions/attest@a1948c3f048ba23858d222213b7c278aabede763")
    emit({
        "github_owned_allowed": False,
        "verified_allowed": False,
        "patterns_allowed": patterns,
    })
elif endpoint == "repos/s1korrrr/hlscreen/actions/permissions/workflow":
    emit({
        "default_workflow_permissions": "read",
        "can_approve_pull_request_reviews": False,
    })
elif endpoint.startswith("repos/s1korrrr/hlscreen/rulesets?"):
    direct_list([] if scenario == "public_classic_only" else [{"id": 1}])
elif endpoint == "repos/s1korrrr/hlscreen/rulesets/1":
    required_checks = [
        {"context": "GitHub Actions security"},
        {"context": "RustSec advisory scan"},
        {"context": "Dependency license and source policy"},
        {"context": "PTY TUI (ubuntu-24.04)"},
        {"context": "PTY TUI (macos-15)"},
        {"context": "Rust workspace"},
    ]
    if scenario in {"public_bad_policy", "public_wrong_checks"}:
        required_checks = [{"context": "obsolete-check"}]
    elif scenario == "public_extra_check":
        required_checks.append({"context": "obsolete-check"})
    emit({
        "id": 1,
        "enforcement": "active",
        "bypass_actors": [],
        "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
        "rules": [
            {"type": "deletion"},
            {"type": "non_fast_forward"},
            {"type": "pull_request", "parameters": {
                "required_approving_review_count": 0,
                "required_review_thread_resolution": True,
            }},
            {"type": "required_status_checks", "parameters": {
                "required_status_checks": required_checks,
                "strict_required_status_checks_policy": True,
            }},
        ],
    })
elif endpoint == "repos/s1korrrr/hlscreen/branches/main/protection":
    if scenario == "public_ruleset_only":
        raise SystemExit(1)
    valid = scenario != "public_bad_policy"
    checks = [
        {"context": "GitHub Actions security"},
        {"context": "RustSec advisory scan"},
        {"context": "Dependency license and source policy"},
        {"context": "PTY TUI (ubuntu-24.04)"},
        {"context": "PTY TUI (macos-15)"},
        {"context": "Rust workspace"},
    ]
    if scenario == "public_wrong_checks":
        checks = [{"context": "obsolete-check"}]
    elif scenario == "public_extra_check":
        checks.append({"context": "obsolete-check"})
    emit({
        "required_pull_request_reviews": {"required_approving_review_count": 0},
        "required_status_checks": {"strict": valid, "checks": checks},
        "enforce_admins": {"enabled": True},
        "required_conversation_resolution": {"enabled": True},
        "allow_force_pushes": {"enabled": False},
        "allow_deletions": {"enabled": False},
    })
elif endpoint == "repos/s1korrrr/hlscreen/code-scanning/default-setup":
    emit({"state": "configured"})
elif endpoint == "repos/s1korrrr/hlscreen/vulnerability-alerts":
    raise SystemExit(0)
elif endpoint.startswith("repos/s1korrrr/hlscreen/dependabot/alerts?"):
    emit([])
elif endpoint == "repos/s1korrrr/hlscreen/private-vulnerability-reporting":
    emit({"enabled": scenario != "public_pvr_disabled"})
else:
    print(f"unexpected fake endpoint: {endpoint}", file=sys.stderr)
    raise SystemExit(3)
'''


def run_case(
    scenario: str,
    mode: str,
    *,
    expected_success: bool,
    packages_confirmed: bool = True,
    origin_matches: bool = True,
    expected_error: str = "",
    expected_absent: str = "",
    env_overrides: dict[str, str] | None = None,
) -> None:
    global CASE_COUNT
    with tempfile.TemporaryDirectory(prefix="hlscreen-surface-test.") as temp:
        root = Path(temp)
        (root / "scripts").mkdir()
        (root / "docs").mkdir()
        shutil.copy2(SURFACE_GATE, root / "scripts/check-public-surface.sh")
        package_marker = (
            "- [x] Owner confirmation: Packages inventory checked in GitHub UI."
            if packages_confirmed
            else "- [ ] Owner confirmation: Packages inventory checked in GitHub UI."
        )
        (root / "docs/OPEN_SOURCE_AUDIT.md").write_text(
            "\n".join(
                [
                    "# Mock audit",
                    package_marker,
                    "- [x] Owner confirmation: Private advisory drafts checked",
                    "- [x] Owner confirmation: info@rsitech.ai monitoring checked",
                    "- [x] Owner confirmation: Git commit-author metadata exposure accepted",
                    "- [x] Owner confirmation: Historical developer-path and non-public email content exposure accepted",
                    "- [x] Owner confirmation: Discussions and its answerable Q&A category are enabled.",
                    "- [x] Owner confirmation: private vulnerability reporting enabled before public launch.",
                    "- Branch decision: `feat/andrzej_oss_full_closeout` — MERGE_BEFORE_PUBLIC.",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        fake_gh = root / "fake-gh"
        fake_gh.write_text(FAKE_GH, encoding="utf-8")
        fake_gh.chmod(fake_gh.stat().st_mode | stat.S_IXUSR)

        subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.email", "surface@example.invalid"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.name", "Surface Gate Test"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(["git", "commit", "-qm", "base fixture"], cwd=root, check=True)
        base_sha = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True
        ).strip()
        (root / "candidate.txt").write_text("candidate\n", encoding="utf-8")
        subprocess.run(["git", "add", "candidate.txt"], cwd=root, check=True)
        subprocess.run(["git", "commit", "-qm", "candidate fixture"], cwd=root, check=True)
        origin = (
            "https://github.com/s1korrrr/hlscreen.git"
            if origin_matches
            else "https://github.com/s1korrrr/not-hlscreen.git"
        )
        subprocess.run(["git", "remote", "add", "origin", origin], cwd=root, check=True)
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        main_sha = sha if mode == "public" else base_sha
        subprocess.run(
            ["git", "update-ref", "refs/remotes/origin/main", main_sha],
            cwd=root,
            check=True,
        )
        env = os.environ.copy()
        env.update(
            {
                "HLS_GH_BIN": str(fake_gh),
                "HLS_EXPECTED_SHA": sha,
                "HLS_MAIN_SHA": main_sha,
                "HLS_SURFACE_SCENARIO": scenario,
            }
        )
        env.update(env_overrides or {})
        result = subprocess.run(
            ["bash", "scripts/check-public-surface.sh", mode, sha],
            cwd=root,
            env=env,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if (result.returncode == 0) != expected_success:
            raise AssertionError(
                f"scenario {scenario} returned {result.returncode}\n"
                f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )
        if expected_error and expected_error not in result.stderr:
            raise AssertionError(
                f"scenario {scenario} omitted {expected_error!r}\n"
                f"stderr:\n{result.stderr}"
            )
        if expected_absent and expected_absent in result.stderr:
            raise AssertionError(
                f"scenario {scenario} unexpectedly emitted {expected_absent!r}\n"
                f"stderr:\n{result.stderr}"
            )
        if scenario == "private_sensitive_log":
            fixture_token = "gh" + "p_" + ("A" * 20)
            if fixture_token in result.stdout or fixture_token in result.stderr:
                raise AssertionError("surface gate emitted matched Actions log content")
        CASE_COUNT += 1


def main() -> None:
    run_case("private_ok", "private-candidate", expected_success=True)
    run_case(
        "private_auth_timeout",
        "private-candidate",
        expected_success=False,
        expected_error="GitHub CLI authentication check timed out",
        expected_absent="Traceback",
        env_overrides={"HLS_GH_READ_TIMEOUT_SECS": "1"},
    )
    run_case(
        "private_timeout",
        "private-candidate",
        expected_success=False,
        expected_error="GitHub API read timed out: repos/s1korrrr/hlscreen/branches",
        expected_absent="Traceback",
        env_overrides={"HLS_GH_READ_TIMEOUT_SECS": "1"},
    )
    run_case(
        "private_ok",
        "private-candidate",
        expected_success=False,
        expected_error="HLS_GH_READ_TIMEOUT_SECS must be an integer from 1 through 600",
        expected_absent="Traceback",
        env_overrides={"HLS_GH_READ_TIMEOUT_SECS": "0"},
    )
    for name, maximum in (
        ("HLS_GH_READ_TIMEOUT_SECS", 600),
        ("HLS_LOCAL_GIT_TIMEOUT_SECS", 60),
    ):
        expected_error = f"{name} must be an integer from 1 through {maximum}"
        run_case(
            "private_ok",
            "private-candidate",
            expected_success=False,
            expected_error=expected_error,
            expected_absent="Traceback",
            env_overrides={name: str(maximum + 1)},
        )
        run_case(
            "private_ok",
            "private-candidate",
            expected_success=False,
            expected_error=expected_error,
            expected_absent="Traceback",
            env_overrides={name: "9" * 5_000},
        )
    run_case(
        "private_ok",
        "private-candidate",
        expected_success=False,
        packages_confirmed=False,
        expected_error="Packages inventory needs owner UI confirmation",
    )
    run_case(
        "private_ok",
        "private-candidate",
        expected_success=False,
        origin_matches=False,
        expected_error="origin does not match HLS_GITHUB_REPOSITORY",
    )
    run_case(
        "private_second_page_branch",
        "private-candidate",
        expected_success=False,
        expected_error="final recorded surface decision",
    )
    run_case(
        "private_stale_main",
        "private-candidate",
        expected_success=False,
        expected_error="local origin/main does not match the hosted main SHA",
    )
    run_case(
        "private_missing_pr_decision",
        "private-candidate",
        expected_success=False,
        expected_error="final recorded surface decision",
    )
    run_case(
        "private_extra_human_pr",
        "private-candidate",
        expected_success=False,
        expected_error="open human pull requests remain: count=1",
    )
    run_case(
        "private_second_page_artifact",
        "private-candidate",
        expected_success=False,
        expected_error="candidate Release artifact inventory is not exact",
    )
    run_case(
        "private_second_page_run",
        "private-candidate",
        expected_success=False,
        expected_error="historical Actions runs/logs outside expected_sha remain",
    )
    run_case(
        "private_sensitive_log",
        "private-candidate",
        expected_success=False,
        expected_error="candidate Actions logs contain suspicious content",
    )
    run_case(
        "private_prestep_failure",
        "private-candidate",
        expected_success=False,
        expected_error="required hosted CI jobs did not all execute successfully",
    )
    run_case(
        "private_extra_ci_job",
        "private-candidate",
        expected_success=False,
        expected_error="hosted CI job inventory is not exact",
    )
    run_case(
        "private_missing_release",
        "private-candidate",
        expected_success=False,
        expected_error="no successful hosted Release run exists at expected_sha",
    )
    run_case(
        "private_release_failure",
        "private-candidate",
        expected_success=False,
        expected_error="no successful hosted Release run exists at expected_sha",
    )
    run_case(
        "private_release_job_failure",
        "private-candidate",
        expected_success=False,
        expected_error="required hosted Release jobs did not all execute successfully",
    )
    run_case(
        "private_bad_release_artifacts",
        "private-candidate",
        expected_success=False,
        expected_error="candidate Release artifact inventory is not exact",
    )
    run_case("public_ok", "public", expected_success=True)
    run_case("public_ruleset_only", "public", expected_success=True)
    run_case("public_classic_only", "public", expected_success=True)
    run_case(
        "public_bad_policy",
        "public",
        expected_success=False,
        expected_error="main policy does not enforce",
    )
    run_case(
        "public_wrong_checks",
        "public",
        expected_success=False,
        expected_error="main policy does not enforce",
    )
    run_case(
        "public_extra_check",
        "public",
        expected_success=False,
        expected_error="main policy does not enforce",
    )
    run_case(
        "public_security_disabled",
        "public",
        expected_success=False,
        expected_error="required security_and_analysis features are not enabled",
    )
    run_case(
        "public_incomplete_actions",
        "public",
        expected_success=False,
        expected_error="Actions policy is not the exact audited SHA allowlist",
    )
    run_case(
        "public_pvr_disabled",
        "public",
        expected_success=False,
        expected_error="Private vulnerability reporting is not enabled",
    )
    print(f"public_surface_mock_tests=passed cases={CASE_COUNT}")


if __name__ == "__main__":
    main()

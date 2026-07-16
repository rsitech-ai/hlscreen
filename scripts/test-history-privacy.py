#!/usr/bin/env python3
"""Verify that the history privacy summarizer emits aggregate metadata only."""

from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SUMMARIZER = ROOT / "scripts/summarize-git-history-privacy.py"


def main() -> None:
    fixture = (
        "commit 1111111111111111111111111111111111111111\n"
        "subject without private data\n"
        "diff --git a/" "Users/header/private b/example\n"
        "--- a/" "Users/header/private\n"
        "+++ b/" "Users/header/private\n"
        "@@ -1,3 +1,3 @@\n"
        " unchanged = /" "Users/context/private\n"
        "+local = /" "Users/demo/private/project\n"
        "+++private = third@" "private.test\n"
        "---private = /" "Users/removed/private\n"
        "+scratch = /private" "/tmp/hlscreen-private/output\n"
        "+contact = info@rsitech.ai\n"
        "+fixture = test@example.invalid\n"
        "+private = person@" "private.test\n"
        "commit 2222222222222222222222222222222222222222\n"
        "+local = /" "Users/runner/work/project\n"
        "+private = second@" "private.test\n"
    )
    result = subprocess.run(
        ["python3", str(SUMMARIZER)],
        input=fixture,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise AssertionError(result.stderr)
    expected = (
        "history_privacy_metadata="
        "commits:2 developer_home_occurrences:2 developer_home_commits:1 "
        "private_tmp_occurrences:1 private_tmp_commits:1 "
        "non_public_email_occurrences:3 non_public_email_commits:2"
    )
    if result.stdout.strip() != expected:
        raise AssertionError(f"unexpected summary: {result.stdout!r}")
    for private_value in (
        "/" + "Users/demo/private/project",
        "/private" + "/tmp/hlscreen-private/output",
        "person@" + "private.test",
        "second@" + "private.test",
        "third@" + "private.test",
    ):
        if private_value in result.stdout or private_value in result.stderr:
            raise AssertionError("summarizer emitted private fixture content")
    print("history_privacy_mock_tests=passed cases=1")


if __name__ == "__main__":
    main()

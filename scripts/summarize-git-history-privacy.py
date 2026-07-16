#!/usr/bin/env python3
"""Summarize privacy-sensitive Git history without emitting matched content."""

from __future__ import annotations

import re
import sys


COMMIT = re.compile(r"^commit ([0-9a-f]{40})$")
DEVELOPER_HOME = re.compile(
    r"/" r"Users/(?!runner(?:admin)?(?:/|\b))[^\s`'\"<>]+"
)
PRIVATE_TMP = re.compile(r"/private" r"/tmp/hlscreen[^\s`'\"<>]*")
EMAIL = re.compile(
    r"(?<![A-Za-z0-9._%+-])"
    r"[A-Za-z0-9._%+-]+@([A-Za-z0-9.-]+\.[A-Za-z]{2,})"
    r"(?![A-Za-z0-9])"
)
PUBLIC_EMAIL_DOMAINS = {
    "example.com",
    "example.invalid",
    "example.org",
    "rsitech.ai",
    "users.noreply.github.com",
}


def main() -> None:
    commits: set[str] = set()
    developer_commits: set[str] = set()
    private_tmp_commits: set[str] = set()
    email_commits: set[str] = set()
    developer_occurrences = 0
    private_tmp_occurrences = 0
    non_public_email_occurrences = 0
    current_commit = ""
    in_patch = False
    in_hunk = False

    for line in sys.stdin:
        commit = COMMIT.fullmatch(line.rstrip("\n"))
        if commit:
            current_commit = commit.group(1)
            commits.add(current_commit)
            in_patch = False
            in_hunk = False
            continue
        if not current_commit:
            continue
        if line.startswith("diff --git "):
            in_patch = True
            in_hunk = False
            continue
        if in_patch:
            if line.startswith("@@"):
                in_hunk = True
                continue
            if not in_hunk or not line.startswith(("+", "-")):
                continue

        developer_count = len(DEVELOPER_HOME.findall(line))
        if developer_count:
            developer_occurrences += developer_count
            developer_commits.add(current_commit)

        private_tmp_count = len(PRIVATE_TMP.findall(line))
        if private_tmp_count:
            private_tmp_occurrences += private_tmp_count
            private_tmp_commits.add(current_commit)

        private_email_count = sum(
            1
            for match in EMAIL.finditer(line)
            if match.group(1).lower() not in PUBLIC_EMAIL_DOMAINS
        )
        if private_email_count:
            non_public_email_occurrences += private_email_count
            email_commits.add(current_commit)

    print(
        "history_privacy_metadata="
        f"commits:{len(commits)} "
        f"developer_home_occurrences:{developer_occurrences} "
        f"developer_home_commits:{len(developer_commits)} "
        f"private_tmp_occurrences:{private_tmp_occurrences} "
        f"private_tmp_commits:{len(private_tmp_commits)} "
        f"non_public_email_occurrences:{non_public_email_occurrences} "
        f"non_public_email_commits:{len(email_commits)}"
    )


if __name__ == "__main__":
    main()

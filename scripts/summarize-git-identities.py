#!/usr/bin/env python3
"""Emit aggregate commit-identity counts without printing mailbox values."""

from __future__ import annotations

import sys


def is_noreply(mailbox: str) -> bool:
    normalized = mailbox.strip().lower()
    return normalized == "noreply@github.com" or normalized.endswith(
        "@users.noreply.github.com"
    )


rows: list[tuple[str, str]] = []
for raw_line in sys.stdin:
    fields = raw_line.rstrip("\n").split("\t", 1)
    if len(fields) == 2:
        rows.append((fields[0].strip().lower(), fields[1].strip().lower()))

authors = [author for author, _ in rows if author]
committers = [committer for _, committer in rows if committer]
all_mailboxes = authors + committers
non_noreply = [mailbox for mailbox in all_mailboxes if not is_noreply(mailbox)]

print(
    "identity_metadata="
    f"commits:{len(rows)} "
    f"author_non_noreply_occurrences:{sum(not is_noreply(value) for value in authors)} "
    f"committer_non_noreply_occurrences:{sum(not is_noreply(value) for value in committers)} "
    f"unique_mailboxes:{len(set(all_mailboxes))} "
    f"unique_non_noreply_mailboxes:{len(set(non_noreply))}"
)

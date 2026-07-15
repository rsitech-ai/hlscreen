#!/usr/bin/env python3
"""Fail closed when locked dependencies add or change packaged NOTICE files."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "third_party/notices/manifest.json"
DOCUMENT_PATH = REPO_ROOT / "THIRD_PARTY_NOTICES.md"


def fail(message: str) -> None:
    print(f"third-party notice check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1:
        fail("unsupported notice manifest schema")

    metadata_output = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--all-features",
            "--format-version",
            "1",
        ],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(metadata_output.stdout)

    detected: dict[tuple[str, str, str], Path] = {}
    for package in metadata["packages"]:
        if package.get("source") is None:
            continue
        package_root = Path(package["manifest_path"]).parent
        for candidate in package_root.rglob("*"):
            if candidate.is_file() and candidate.name.upper().startswith("NOTICE"):
                source = candidate.relative_to(package_root).as_posix()
                key = (package["name"], package["version"], source)
                detected[key] = candidate

    expected: dict[tuple[str, str, str], Path] = {}
    for entry in manifest.get("notices", []):
        key = (entry["package"], entry["version"], entry["source"])
        preserved = (REPO_ROOT / entry["preserved"]).resolve()
        try:
            preserved.relative_to(REPO_ROOT)
        except ValueError:
            fail(f"preserved notice escapes repository: {entry['preserved']}")
        if key in expected:
            fail(f"duplicate notice manifest entry: {key}")
        expected[key] = preserved

    untracked = sorted(set(detected) - set(expected))
    if untracked:
        fail(f"untracked packaged NOTICE files: {untracked}")
    stale = sorted(set(expected) - set(detected))
    if stale:
        fail(f"manifest entries without packaged NOTICE files: {stale}")

    document = DOCUMENT_PATH.read_bytes()
    for key, packaged_path in sorted(detected.items()):
        preserved_path = expected[key]
        if not preserved_path.is_file():
            fail(f"missing preserved notice: {preserved_path.relative_to(REPO_ROOT)}")
        packaged = packaged_path.read_bytes()
        preserved = preserved_path.read_bytes()
        if preserved != packaged:
            fail(f"{preserved_path.relative_to(REPO_ROOT)} does not match its packaged source")
        if preserved.rstrip(b"\n") not in document:
            fail(f"{preserved_path.relative_to(REPO_ROOT)} is not embedded verbatim in THIRD_PARTY_NOTICES.md")

    print(f"third_party_notices=verified count={len(detected)}")


if __name__ == "__main__":
    main()

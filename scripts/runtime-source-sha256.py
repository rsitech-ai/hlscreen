#!/usr/bin/env python3
"""Hash the tracked source inputs that define the hls runtime binary."""

from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path


RUNTIME_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    "crates",
    "config",
)


def main() -> int:
    repo_root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    command = ["git", "-C", str(repo_root), "ls-files", "-z", "--", *RUNTIME_PATHS]
    result = subprocess.run(command, check=True, capture_output=True)
    paths = sorted(path for path in result.stdout.split(b"\0") if path)
    if not paths:
        raise SystemExit("runtime source inventory is empty")

    digest = hashlib.sha256()
    for raw_path in paths:
        path = repo_root / raw_path.decode("utf-8", errors="strict")
        data = path.read_bytes()
        executable = b"1" if path.stat().st_mode & 0o111 else b"0"
        digest.update(len(raw_path).to_bytes(8, "big"))
        digest.update(raw_path)
        digest.update(executable)
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    print(digest.hexdigest())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Fail-closed validation for one native cargo-dist target artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath


SUPPORTED_TARGETS = {
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
}
REQUIRED_FILES = {
    "LICENSE",
    "THIRD_PARTY_LICENSES.txt",
    "THIRD_PARTY_NOTICES.md",
    "README.md",
    "CHANGELOG.md",
}
MAX_ARCHIVE_BYTES = 256 * 1024 * 1024
MAX_MEMBER_BYTES = 128 * 1024 * 1024


def safe_member(name: str) -> PurePosixPath:
    normalized = name.replace("\\", "/")
    path = PurePosixPath(normalized)
    if path.is_absolute() or not path.parts or ".." in path.parts:
        raise ValueError(f"unsafe archive member: {name!r}")
    return path


def load_manifest(path: Path) -> None:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("dist manifest root is not an object")


def verify_checksum(archive: Path) -> None:
    checksum_path = Path(f"{archive}.sha256")
    if not checksum_path.is_file():
        raise ValueError(f"missing checksum: {checksum_path.name}")
    fields = checksum_path.read_text(encoding="utf-8").split()
    if not fields or not re.fullmatch(r"[0-9a-fA-F]{64}", fields[0]):
        raise ValueError(f"invalid checksum file: {checksum_path.name}")
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if digest.lower() != fields[0].lower():
        raise ValueError(f"checksum mismatch: {archive.name}")


def inspect_tar(archive: Path) -> tuple[set[str], str]:
    names: set[str] = set()
    binary = ""
    with tarfile.open(archive, mode="r:*") as bundle:
        for member in bundle.getmembers():
            path = safe_member(member.name)
            if member.issym() or member.islnk() or member.isdev():
                raise ValueError(f"unsupported archive member type: {member.name!r}")
            if not member.isdir() and not member.isfile():
                raise ValueError(f"unsupported archive member type: {member.name!r}")
            if member.size > MAX_MEMBER_BYTES:
                raise ValueError(f"oversized archive member: {member.name!r}")
            if member.isfile():
                names.add(path.name)
                if path.name == "hls":
                    binary = member.name
    return names, binary


def inspect_zip(archive: Path) -> tuple[set[str], str]:
    names: set[str] = set()
    binary = ""
    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            path = safe_member(member.filename)
            unix_mode = member.external_attr >> 16
            if stat.S_ISLNK(unix_mode):
                raise ValueError(f"symlink archive member: {member.filename!r}")
            if member.file_size > MAX_MEMBER_BYTES:
                raise ValueError(f"oversized archive member: {member.filename!r}")
            if not member.is_dir():
                names.add(path.name)
                if path.name == "hls.exe":
                    binary = member.filename
    return names, binary


def inspect_archive(archive: Path, target: str) -> tuple[set[str], str]:
    if archive.stat().st_size > MAX_ARCHIVE_BYTES:
        raise ValueError(f"archive exceeds {MAX_ARCHIVE_BYTES} bytes: {archive.name}")
    if target.endswith("windows-msvc"):
        return inspect_zip(archive)
    return inspect_tar(archive)


def extract_verified(archive: Path, target: str, destination: Path) -> None:
    if target.endswith("windows-msvc"):
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(destination)
    else:
        with tarfile.open(archive, mode="r:*") as bundle:
            bundle.extractall(destination)


def run_checked(command: list[str], *, timeout: int) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path("dist-manifest.json"))
    parser.add_argument("--target", required=True)
    args = parser.parse_args()

    if args.target not in SUPPORTED_TARGETS:
        raise SystemExit(f"unsupported target: {args.target}")
    load_manifest(args.manifest)

    suffix = ".zip" if args.target.endswith("windows-msvc") else ".tar.xz"
    archives = sorted(
        path
        for path in args.artifact_dir.glob(f"*{args.target}*{suffix}")
        if path.is_file()
    )
    if len(archives) != 1:
        raise SystemExit(
            f"expected one {args.target} archive, found {len(archives)}"
        )
    archive = archives[0]
    verify_checksum(archive)
    names, binary_member = inspect_archive(archive, args.target)
    missing = sorted(REQUIRED_FILES - names)
    if missing:
        raise SystemExit(f"archive is missing required files: {', '.join(missing)}")
    if not binary_member:
        raise SystemExit("archive is missing the hls executable")

    repo_root = Path(__file__).resolve().parent.parent
    fixture = repo_root / "tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    with tempfile.TemporaryDirectory(prefix="hls-dist-verify.") as temp:
        root = Path(temp)
        extract_verified(archive, args.target, root)
        binary = root / Path(binary_member.replace("/", os.sep))
        if os.name != "nt":
            binary.chmod(binary.stat().st_mode | stat.S_IXUSR)
        run_checked([str(binary), "--help"], timeout=30)
        run_checked(
            [str(binary), "doctor", "--data-dir", str(root / "doctor")],
            timeout=30,
        )
        run_checked(
            [
                str(binary),
                "live",
                "--symbols",
                "@107",
                "--fixture-file",
                str(fixture),
                "--once",
                "--data-dir",
                str(root / "live"),
            ],
            timeout=30,
        )
        audit = run_checked(["cargo", "audit", "bin", str(binary)], timeout=300)
        audit_output = audit.stdout + audit.stderr
        if "Found 'cargo auditable' data" not in audit_output:
            raise SystemExit("binary does not expose complete cargo-auditable metadata")

    print(f"dist_local_artifact_validation=passed target={args.target}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"dist artifact validation failed: {error}") from None

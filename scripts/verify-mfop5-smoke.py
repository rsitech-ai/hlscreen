#!/usr/bin/env python3
"""Finite portable synthetic-fixture consumption by one actual hls executable.

This verifies structural consumer compatibility, not native producer execution
on the artifact's platform, source freshness, or strategy availability.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
import threading
import time
from pathlib import Path, PurePosixPath

REQUIRED_FILES = {"README.md", "feed/events.mfsf", "feed/committed.json"}
MAX_FILE_BYTES = 2 * 1024 * 1024
MAX_OUTPUT_BYTES = 1024 * 1024
DEFAULT_FIXTURE = (
    Path(__file__).resolve().parent.parent
    / "tests/fixtures/hyperliquid/mfop5-synthetic-v1"
)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def invalid_constant(value):
    raise ValueError(f"nonfinite JSON constant: {value}")


def finite_float(value):
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError(f"nonfinite JSON number: {value}")
    return parsed


def parse_json(text):
    return json.loads(
        text,
        object_pairs_hook=unique_object,
        parse_constant=invalid_constant,
        parse_float=finite_float,
    )


def exact_equal(actual, expected):
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(
            exact_equal(actual[key], value) for key, value in expected.items()
        )
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(
            exact_equal(a, e) for a, e in zip(actual, expected)
        )
    return actual == expected


def file_sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def fingerprint(path):
    stat = path.stat()
    return {
        "device": stat.st_dev,
        "inode": stat.st_ino,
        "bytes": stat.st_size,
        "mode": stat.st_mode,
        "mtime_ns": stat.st_mtime_ns,
        "ctime_ns": stat.st_ctime_ns,
        "sha256": file_sha256(path),
    }


def load_fixture(root):
    root = root.resolve(strict=True)
    manifest_path = root / "manifest.json"
    if manifest_path.is_symlink() or manifest_path.stat().st_size > MAX_OUTPUT_BYTES:
        raise ValueError("invalid fixture manifest")
    manifest = parse_json(manifest_path.read_text(encoding="utf-8"))
    if (
        not isinstance(manifest, dict)
        or type(manifest.get("schema_version")) is not int
        or manifest.get("schema_version") != 1
        or manifest.get("synthetic") is not True
        or type(manifest.get("source_profile")) is not int
        or manifest.get("source_profile") != 5
        or type(manifest.get("owner_history_schema")) is not int
        or manifest.get("owner_history_schema") != 4
        or manifest.get("portable_native_authentication") is not False
    ):
        raise ValueError("unsupported synthetic fixture scope")
    files = manifest.get("files")
    if not isinstance(files, list) or len(files) != len(REQUIRED_FILES):
        raise ValueError("unexpected fixture file inventory")
    snapshots = {}
    for row in files:
        if not isinstance(row, dict):
            raise ValueError("fixture inventory row must be an object")
        name = row.get("path")
        if name not in REQUIRED_FILES or name in snapshots:
            raise ValueError("unexpected or duplicate fixture path")
        relative = PurePosixPath(name)
        path = root
        for part in relative.parts:
            path = path / part
            if path.is_symlink():
                raise ValueError("fixture symlinks are unsupported")
        if not path.is_file() or path.stat().st_size > MAX_FILE_BYTES:
            raise ValueError("fixture file bound")
        actual = fingerprint(path)
        if (
            type(row.get("bytes")) is not int
            or actual["bytes"] != row["bytes"]
            or actual["sha256"] != row.get("sha256")
        ):
            raise ValueError(f"fixture hash/length mismatch: {name}")
        snapshots[name] = actual
    if set(snapshots) != REQUIRED_FILES:
        raise ValueError("fixture inventory incomplete")
    if (root / "feed/events.mfsf").read_bytes()[:8] != b"MFOP1\0\0\0":
        raise ValueError("fixture is not an MFOP1 projection")
    expected = manifest.get("expected_summary")
    if (
        not isinstance(expected, dict)
        or expected.get("sealed") is not True
        or expected.get("availability_time_ns", "missing") is not None
        or not exact_equal(expected.get("events"), {"trade": 2})
        or type(manifest.get("expected_market_events")) is not int
        or manifest.get("expected_market_events") != 2
    ):
        raise ValueError("unexpected finite fixture expectations")
    snapshots["manifest.json"] = fingerprint(manifest_path)
    return root, manifest, snapshots


def run_child(argv, *, timeout=30):
    # Keep one aggregate byte budget while draining both pipes. Reader threads
    # handle bytes only; UTF8 decoding/failure occurs in this calling thread.
    process = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    lock = threading.Lock()
    errors = []

    def stop():
        try:
            process.kill()
        except ProcessLookupError:
            pass

    def drain(name, stream):
        try:
            while True:
                chunk = stream.read(16384)
                if not chunk:
                    break
                with lock:
                    if sum(map(len, buffers.values())) + len(chunk) > MAX_OUTPUT_BYTES:
                        if not errors:
                            errors.append("fixture CLI output bound")
                        stop()
                        break
                    buffers[name].extend(chunk)
        except OSError as error:
            with lock:
                if not errors:
                    errors.append(str(error))
            stop()
        finally:
            stream.close()

    threads = [
        threading.Thread(target=drain, args=(name, stream), daemon=True)
        for name, stream in [("stdout", process.stdout), ("stderr", process.stderr)]
    ]
    for thread in threads:
        thread.start()
    expired = None
    try:
        code = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        expired = error
        stop()
        code = process.wait(timeout=1)
    until = time.monotonic() + 1
    for thread in threads:
        thread.join(timeout=max(0, until - time.monotonic()))
    if any(thread.is_alive() for thread in threads):
        raise ValueError("fixture child output drain incomplete")
    if expired is not None:
        raise expired
    if errors:
        raise ValueError(errors[0])
    stdout = bytes(buffers["stdout"]).decode("utf-8")
    stderr = bytes(buffers["stderr"]).decode("utf-8")
    if code:
        raise ValueError(f"fixture CLI exited {code}: {stderr[-8192:]}")
    return stdout, stderr


def singleton(lines, prefix):
    values = [line[len(prefix) :] for line in lines if line.startswith(prefix)]
    if len(values) != 1:
        raise ValueError(f"expected exactly one {prefix} line")
    return values[0]


def verify(binary, fixture, data_dir, runner=run_child):
    binary = binary.resolve(strict=True)
    if data_dir.exists() or data_dir.is_symlink():
        raise ValueError("smoke data directory must be new")
    root, manifest, before = load_fixture(fixture)
    binary_before = fingerprint(binary)
    argv = [
        str(binary),
        "live",
        "--shared-feed",
        str(root / "feed"),
        "--once",
        "--data-dir",
        str(data_dir),
    ]
    stdout, stderr = runner(argv)
    lines = stdout.splitlines()
    actual = parse_json(singleton(lines, "shared_feed_summary="))
    if not exact_equal(actual, manifest["expected_summary"]):
        raise ValueError(f"MFOP5 summary mismatch: {actual!r}")
    cursor = parse_json(singleton(lines, "shared_feed_cursor="))
    count = singleton(lines, "market_events=")
    if (
        not exact_equal(cursor, actual["cursor"])
        or not re.fullmatch(r"[0-9]+", count)
        or int(count) != manifest["expected_market_events"]
    ):
        raise ValueError("MFOP5 cursor/event count mismatch")
    for name, snapshot in before.items():
        if fingerprint(root / name) != snapshot:
            raise ValueError(f"fixture changed during consumption: {name}")
    if fingerprint(binary) != binary_before:
        raise ValueError("executable file changed during consumption")
    return {
        "schema_version": 1,
        "scope": "portable synthetic MFOP5 structural consumer smoke",
        "binary_sha256": binary_before["sha256"],
        "fixture_manifest_sha256": before["manifest.json"]["sha256"],
        "argv": argv,
        "exit_code": 0,
        "market_events": int(count),
        "summary": actual,
        "fixture_bytes_and_metadata_unchanged": True,
        "native_source_authentication_on_this_platform": False,
        "availability_time_ns": None,
        "loaded_image_attestation": False,
        "stderr_sha256": hashlib.sha256(stderr.encode("utf-8")).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    if args.receipt and (args.receipt.exists() or args.receipt.is_symlink()):
        raise ValueError("receipt path must be new")
    report = verify(args.binary, args.fixture, args.data_dir)
    if args.receipt:
        with args.receipt.open("x", encoding="utf-8") as output:
            json.dump(report, output, indent=2)
            output.write("\n")
    print("mfop5_smoke=" + json.dumps(report, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit(f"MFOP5 smoke failed: {error}") from None

#!/usr/bin/env python3
"""Offline regression checks for historical reports and exact candidate binding."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parent.parent
VALIDATOR = REPO / "scripts/validate-soak-report.py"
SOURCE_HASHER = REPO / "scripts/runtime-source-sha256.py"


class SoakValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="hls-soak-validation-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.runtime = self.root / "Cargo.toml"
        self.runtime.write_text("# synthetic runtime input\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.root), "add", "Cargo.toml"], check=True)
        self.binary = self.root / "hls"
        self.binary.write_bytes(b"synthetic candidate binary\n")
        self.report = json.loads(
            (REPO / "tests/fixtures/operations/soak-report-valid.json").read_text()
        )
        self.report["binary_sha256"] = hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.report["runtime_source_sha256"] = subprocess.check_output(
            [sys.executable, str(SOURCE_HASHER), str(self.root)], text=True
        ).strip()
        self.report_path = self.root / "report.json"
        self.report_path.write_text(json.dumps(self.report), encoding="utf-8")

    def run_validator(self, *args: str, report: Path | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(VALIDATOR), str(report or self.report_path), *args],
            text=True, capture_output=True, timeout=10,
        )

    def test_exact_candidate_binary_and_source_pass(self) -> None:
        result = self.run_validator(
            "--binary", str(self.binary), "--runtime-source-root", str(self.root)
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("scope=binary_and_source_bound", result.stdout)

    def test_changed_binary_fails_even_with_matching_sources(self) -> None:
        self.binary.write_bytes(b"different platform or build\n")
        result = self.run_validator(
            "--binary", str(self.binary), "--runtime-source-root", str(self.root)
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("binary_sha256 does not match --binary", result.stderr)

    def test_changed_runtime_fails_even_with_original_binary(self) -> None:
        self.runtime.write_text("# changed dependency or source\n", encoding="utf-8")
        result = self.run_validator(
            "--binary", str(self.binary), "--runtime-source-root", str(self.root)
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("runtime_source_sha256 does not match --runtime-source-root", result.stderr)

    def test_documentation_change_preserves_runtime_binding(self) -> None:
        (self.root / "README.md").write_text("documentation only\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.root), "add", "README.md"], check=True)
        result = self.run_validator(
            "--binary", str(self.binary), "--runtime-source-root", str(self.root)
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_binary_fails(self) -> None:
        result = self.run_validator("--binary", str(self.root / "missing"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("could not hash --binary", result.stderr)

    def test_source_binding_requires_binary(self) -> None:
        result = self.run_validator("--runtime-source-root", str(self.root))
        self.assertEqual(result.returncode, 2)
        self.assertIn("--runtime-source-root requires --binary", result.stderr)

    def test_retained_binary_only_does_not_claim_current_source_binding(self) -> None:
        self.runtime.write_text("# changed current runtime\n", encoding="utf-8")
        result = self.run_validator("--binary", str(self.binary))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("scope=binary_bound", result.stdout)
        self.assertNotIn("scope=binary_and_source_bound", result.stdout)

    def test_historical_reports_stay_immutable_and_report_only(self) -> None:
        expected = {
            "shared-feed-20261019-15m.json": "ee3e5a00fcd4145832025453e1d7776e36724520705ae74eabb10c8aa1d4f8e7",
            "sota-allpairs-20260720-15m.json": "685b705edd76997a67f9d7c2e6e22a47d04f691fcad7baaf321b1169e2650656",
        }
        for name, digest in expected.items():
            with self.subTest(report=name):
                path = REPO / "docs/evidence/soak" / name
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), digest)
                result = self.run_validator(report=path)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("scope=report_only", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)

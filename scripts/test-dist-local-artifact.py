#!/usr/bin/env python3
"""Exercise real child output and failure handling in the artifact validator."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    "dist_validator", Path(__file__).with_name("verify-dist-local-artifact.py")
)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


class ChildOutputTests(unittest.TestCase):
    def child(self, source: str, *, timeout: int = 10):
        return VALIDATOR.run_checked([sys.executable, "-c", source], timeout=timeout)

    def test_utf8_output_is_preserved_under_windows_legacy_locale(self):
        # U+258F includes 0x8f in UTF-8, the undefined cp1252 byte observed
        # in the hosted Windows reader-thread failure. Both streams matter.
        expected = "artifact \u258f \u2022 \u221e\n"
        payload = expected.encode("utf-8")
        with mock.patch("locale.getpreferredencoding", return_value="cp1252"), mock.patch(
            "locale.getencoding", return_value="cp1252", create=True
        ), mock.patch("subprocess._text_encoding", return_value="cp1252", create=True):
            result = self.child(
                f"import os; os.write(1, {payload!r}); os.write(2, {payload!r})"
            )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, expected)
        self.assertEqual(result.stderr, expected)

    def test_invalid_utf8_on_either_stream_fails_in_the_calling_thread(self):
        for descriptor in (1, 2):
            with self.subTest(descriptor=descriptor):
                with self.assertRaises(UnicodeDecodeError):
                    self.child(f"import os; os.write({descriptor}, b'\\xff')")

    def test_nonzero_child_remains_failed_with_captured_evidence(self):
        with self.assertRaises(subprocess.CalledProcessError) as error:
            self.child("import os; os.write(2, b'audit rejected'); raise SystemExit(7)")
        self.assertEqual(error.exception.returncode, 7)
        self.assertEqual(error.exception.stderr, b"audit rejected")
        self.assertIn("audit rejected", VALIDATOR.failure_message(error.exception))

    def test_failure_diagnostic_bounds_and_escapes_invalid_child_bytes(self):
        error = subprocess.CalledProcessError(
            7, ["synthetic-child"], output=b"x" * 10000,
            stderr=b"discarded-prefix" + b"y" * 8190 + b"\xff\n",
        )
        message = VALIDATOR.failure_message(error)
        evidence = json.loads(message.split(" captured_child=", 1)[1])
        self.assertEqual(evidence["stdout"], "x" * 8192)
        self.assertEqual(evidence["stderr"], "y" * 8190 + "\\xff\n")
        self.assertTrue(evidence["stdout_truncated"])
        self.assertTrue(evidence["stderr_truncated"])
        self.assertNotIn("discarded-prefix", message)
        self.assertNotIn("\n", message)
        self.assertEqual(error.returncode, 7)

    def test_timeout_remains_failed(self):
        with self.assertRaises(subprocess.TimeoutExpired):
            self.child("import time; time.sleep(10)", timeout=1)


if __name__ == "__main__":
    unittest.main()

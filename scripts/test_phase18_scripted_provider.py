#!/usr/bin/env python3
"""Tests for the Phase 18 scripted provider fixture (task 18.10.1).

Runs the fixture as a real subprocess with canned stdin and asserts exact
deterministic behavior: fixed response bytes, EOF termination, typed
rejections for malformed requests, and the bounded request cap. This is the
smoke-addendum gate for task 18.10.1 (``python scripts/test_phase18_scripted_provider.py``).
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("phase18-scripted-provider.py")
EXPECTED_LINE = (
    '{"content":"scripted-provider: acknowledged","schema":"phase18-scripted-provider/1"}'
)


def run_provider(stdin_text: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT)],
        input=stdin_text,
        capture_output=True,
        text=True,
        timeout=30,
    )


class ScriptedProviderTest(unittest.TestCase):
    def test_serves_one_fixed_response_per_request(self) -> None:
        result = run_provider('{"prompt": "first"}\n{"prompt": "second"}\n')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            EXPECTED_LINE + "\n" + EXPECTED_LINE + "\n",
        )
        self.assertEqual(result.stderr, "")

    def test_is_deterministic_across_runs(self) -> None:
        for _ in range(3):
            result = run_provider('{"prompt": "same"}\n')
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, EXPECTED_LINE + "\n")

    def test_blank_lines_are_ignored_and_eof_terminates_cleanly(self) -> None:
        result = run_provider('\n\n{"prompt": "x"}\n\n')
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, EXPECTED_LINE + "\n")

    def test_malformed_requests_are_rejected_with_typed_errors(self) -> None:
        for bad in ("not json", '{"wrong": 1}', '{"prompt": 3}', '[]'):
            with self.subTest(bad=bad):
                result = run_provider(bad + "\n")
                self.assertEqual(result.returncode, 2)
                self.assertIn("scripted-provider:", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_request_cap_is_bounded(self) -> None:
        # MAX_REQUESTS is pinned in the script; exceeding it terminates.
        cap = 8
        stdin_text = "".join(
            json.dumps({"prompt": f"p{i}"}) + "\n" for i in range(cap + 1)
        )
        result = run_provider(stdin_text)
        self.assertEqual(result.returncode, 3)
        self.assertIn("request cap", result.stderr)
        self.assertEqual(result.stdout.count("\n"), cap)

    def test_no_network_or_credential_surface(self) -> None:
        source = SCRIPT.read_text(encoding="utf-8")
        for forbidden in ("socket", "urllib", "requests", "http", "getenv", "environ"):
            self.assertNotIn(forbidden, source)

    def test_writes_no_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run(
                [sys.executable, str(SCRIPT)],
                input='{"prompt": "x"}\n',
                capture_output=True,
                text=True,
                cwd=tmp,
                timeout=30,
            )
            self.assertEqual(result.returncode, 0)
            self.assertEqual(list(Path(tmp).iterdir()), [])


if __name__ == "__main__":
    unittest.main()

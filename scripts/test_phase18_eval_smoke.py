#!/usr/bin/env python3
"""Tests for the Phase 18 assembled-run smoke wrappers (task 18.12).

Executes the platform wrapper for ``scripts/phase18-eval-smoke`` and
asserts the preserved evidence: exact command, stdout report bytes, stderr,
exit codes, receipt bytes, sealed manifests, the content-addressed bundle
identity stability across two identical runs, and the artifact audit.
This is the smoke-addendum gate for task 18.12
(``python scripts/test_phase18_eval_smoke.py``).
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SH_WRAPPER = Path(__file__).with_name("phase18-eval-smoke.sh")
PS1_WRAPPER = Path(__file__).with_name("phase18-eval-smoke.ps1")


def run_wrapper(out: Path) -> Path:
    if sys.platform == "win32":
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(PS1_WRAPPER),
             "-Out", str(out)],
            capture_output=True,
            text=True,
            cwd=REPO_ROOT,
        )
    else:
        completed = subprocess.run(
            ["sh", str(SH_WRAPPER), "--out", str(out)],
            capture_output=True,
            text=True,
            cwd=REPO_ROOT,
        )
    assert completed.returncode == 0, f"wrapper failed: {completed.stderr}"
    return Path(completed.stdout.strip().splitlines()[-1])


class Phase18EvalSmokeWrapper(unittest.TestCase):
    def test_wrapper_preserves_commands_streams_codes_and_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = run_wrapper(Path(tmp))

            for behavior, expected_code in [("happy", 0), ("verifier-failure", 1)]:
                case = out / behavior
                command = (case / "command.txt").read_text().strip()
                self.assertIn("opi-eval", command)
                self.assertIn("--behavior", command)
                code = int((case / "exit_code").read_text().strip())
                self.assertEqual(code, expected_code, behavior)
                report = json.loads((case / "stdout.json").read_text())
                self.assertEqual(report["schema"], "phase18-run-report/1")
                self.assertEqual(report["outcome"], "completed" if code == 0 else "incomplete")
                # The preserved stderr exists as a file even when empty.
                self.assertTrue((case / "stderr.txt").exists(), behavior)

            happy = out / "happy"
            trials = sorted((happy / "root" / "trials").iterdir())
            self.assertEqual(len(trials), 2)
            for trial in trials:
                receipt_bytes = (trial / "receipt.json").read_bytes()
                receipt = json.loads(receipt_bytes)
                self.assertEqual(receipt["status"], "sealed")
                self.assertEqual(len(receipt["bundle_identity"]), 64)
                # The bundle is sealed and its manifest covers the receipt's
                # identity.
                manifest = json.loads((trial / "bundle" / "manifest.json").read_text())
                self.assertEqual(manifest["identity"], receipt["bundle_identity"])
                # The authority ledger is sealed evidence.
                artifacts = trial / "bundle" / "artifacts"
                self.assertTrue((artifacts / "native" / "authority-ledger.json").exists())
                self.assertTrue((artifacts / "native" / "agent-stdout.log").exists())

            # The audit records the preserved identities and receipt
            # digests, and proves the report transition stayed refused for
            # the verifier failure.
            audit = json.loads((out / "audit.json").read_text())
            self.assertEqual(audit["schema"], "phase18-eval-smoke-audit/1")
            identities = {
                row["trial"]: row["bundle_identity"]
                for row in audit["happy"]["bundle_identities"]
            }
            self.assertEqual(len(identities), 2)
            for trial in trials:
                row = next(
                    row for row in audit["happy"]["bundle_identities"]
                    if row["trial"] == trial.name
                )
                receipt = json.loads((trial / "receipt.json").read_bytes())
                self.assertEqual(row["bundle_identity"], receipt["bundle_identity"])
                self.assertTrue(row["sealed"])
                observed = hashlib.sha256(
                    (trial / "receipt.json").read_bytes()
                ).hexdigest()
                self.assertEqual(row["receipt_sha256"], observed)
            self.assertFalse(audit["verifier_failure"]["receipt_written"])

    def test_bundle_identity_is_content_addressed_across_runs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            first = run_wrapper(Path(tmp) / "a")
            second = run_wrapper(Path(tmp) / "b")
            a = json.loads((first / "audit.json").read_text())
            b = json.loads((second / "audit.json").read_text())
            ids_a = {row["trial"]: row["bundle_identity"] for row in a["happy"]["bundle_identities"]}
            ids_b = {row["trial"]: row["bundle_identity"] for row in b["happy"]["bundle_identities"]}
            self.assertEqual(ids_a, ids_b, "identical content must seal to one identity")


if __name__ == "__main__":
    unittest.main()

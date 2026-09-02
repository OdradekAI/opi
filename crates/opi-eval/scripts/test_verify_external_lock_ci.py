#!/usr/bin/env python3
"""Contract tests for the opi-eval materialization CI verifier.

The verifier pins the committed lock-materialization workflow: its path, its
bytes, every producer script it invokes, every action it uses, and its
manual-authorization contract. These tests exercise the rejection matrix
(missing context, workflow path mismatch, workflow drift, unhashed invoked
producer, mutable action, candidate/workflow contract mismatch) against
hermetic temporary workspaces, and the acceptance path against the real
committed repository files.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify-external-lock-ci.py")
REPO_ROOT = Path(__file__).resolve().parents[3]

WORKFLOW_PATH = ".github/workflows/opi-eval-external-lock-materialization.yml"
MATERIALIZER = "crates/opi-eval/scripts/materialize-external-locks.sh"
CI_VERIFIER = "crates/opi-eval/scripts/verify-external-lock-ci.py"
STATIC_LOCK = "crates/opi-eval/external-locks/static/linux-x86_64.json"

ACTION_COMMIT = "a" * 40
OTHER_ACTION_COMMIT = "b" * 40
WORKFLOW_SHA = "c" * 64
MATERIALIZER_SHA = "d" * 64
CI_VERIFIER_SHA = "e" * 64


def lf_sha256(data: bytes) -> str:
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def valid_workflow_text(trigger: str = "workflow_dispatch") -> str:
    return (
        "name: opi-eval external-lock materialization\n"
        f"on:\n  {trigger}:\n    inputs:\n      candidate_sha:\n        required: true\n"
        "permissions:\n  contents: read\n"
        "concurrency:\n  group: opi-eval-lock-materialization\n"
        "  cancel-in-progress: false\n"
        "jobs:\n  materialize:\n    runs-on: ubuntu-24.04\n    steps:\n"
        f"      - uses: actions/checkout@{ACTION_COMMIT} # v4.2.2\n"
        f"      - run: python3 {CI_VERIFIER} --workflow {WORKFLOW_PATH} --script {MATERIALIZER}\n"
        f"      - run: bash {MATERIALIZER} --candidate-commit \"${{{{ inputs.candidate_sha }}}}\"\n"
    )


def valid_static_lock(
    workflow_sha: str = WORKFLOW_SHA,
    workflow_path: str = WORKFLOW_PATH,
    materializer_sha: str = MATERIALIZER_SHA,
    verifier_sha: str = CI_VERIFIER_SHA,
    schema: str = "opi-eval-external-lock/static/1",
) -> dict:
    return {
        "schema": schema,
        "lock_id": "opi-eval-linux-x86_64",
        "platform": "linux-x86_64",
        "authority": {
            "trigger": "workflow_dispatch",
            "admission": "digest",
            "workflow": {"path": workflow_path, "sha256": workflow_sha},
            "producers": [
                {"path": MATERIALIZER, "role": "materializer", "sha256": materializer_sha},
                {"path": CI_VERIFIER, "role": "verifier", "sha256": verifier_sha},
            ],
            "actions": [
                {"name": "actions/checkout", "version": "v4.2.2", "commit": ACTION_COMMIT}
            ],
        },
    }


class Workspace:
    """A hermetic repository-shaped workspace for one verifier run."""

    def __init__(
        self,
        workflow_text: str | None = None,
        static_lock: dict | None = None,
        extra_files: dict[str, bytes] | None = None,
        workflow_name: str = WORKFLOW_PATH,
    ) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="opi-eval-ci-verify-"))
        workflow_text = (
            valid_workflow_text() if workflow_text is None else workflow_text
        )
        static_lock = valid_static_lock() if static_lock is None else static_lock
        for name, body in {
            MATERIALIZER: b"#!/usr/bin/env bash\nset -euo pipefail\n",
            CI_VERIFIER: b"#!/usr/bin/env python3\n",
            **(extra_files or {}),
        }.items():
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
        # Bind the recorded digests to the actual bytes unless a test overrode
        # them for a drift case.
        workflow_path = self.root / workflow_name
        workflow_path.parent.mkdir(parents=True, exist_ok=True)
        workflow_path.write_text(workflow_text, encoding="utf-8", newline="\n")
        if static_lock["authority"]["workflow"]["sha256"] == WORKFLOW_SHA:
            static_lock["authority"]["workflow"]["sha256"] = lf_sha256(
                workflow_path.read_bytes()
            )
        for producer in static_lock["authority"]["producers"]:
            placeholder = {
                MATERIALIZER: MATERIALIZER_SHA,
                CI_VERIFIER: CI_VERIFIER_SHA,
            }.get(producer["path"])
            if producer["sha256"] == placeholder:
                producer["sha256"] = lf_sha256(
                    (self.root / producer["path"]).read_bytes()
                )
        lock_path = self.root / STATIC_LOCK
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        lock_path.write_text(
            json.dumps(static_lock, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
    def ensure_file(self, name: str, body: bytes) -> None:
        target = self.root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)

    def run(self, *extra_args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--workflow",
                str(self.root / WORKFLOW_PATH),
                "--script",
                str(self.root / MATERIALIZER),
                *extra_args,
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )


def seeded_workspace() -> Workspace:
    return Workspace()


class RejectsMissingContext(unittest.TestCase):
    def test_missing_workflow_argument_rejects(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--script", str(REPO_ROOT / MATERIALIZER)],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)

    def test_missing_script_argument_rejects(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--workflow", str(REPO_ROOT / WORKFLOW_PATH)],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)


class RejectsWorkflowPathMismatch(unittest.TestCase):
    def test_workflow_at_unpinned_path_rejects(self) -> None:
        ws = Workspace(workflow_name=".github/workflows/other-workflow.yml")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--workflow",
                str(ws.root / ".github/workflows/other-workflow.yml"),
                "--script",
                str(ws.root / MATERIALIZER),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("path", (result.stdout + result.stderr).lower())


class RejectsWorkflowDrift(unittest.TestCase):
    def test_workflow_bytes_drift_rejects(self) -> None:
        ws = Workspace()
        workflow = ws.root / WORKFLOW_PATH
        workflow.write_text(
            workflow.read_text(encoding="utf-8") + "# drifted\n",
            encoding="utf-8",
            newline="\n",
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("drift", (result.stdout + result.stderr).lower())


class RejectsUnhashedInvokedProducer(unittest.TestCase):
    def test_unpinned_script_argument_rejects(self) -> None:
        ws = Workspace()
        rogue = ws.root / "crates/opi-eval/scripts/rogue-producer.sh"
        rogue.write_bytes(b"#!/usr/bin/env bash\n")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--workflow",
                str(ws.root / WORKFLOW_PATH),
                "--script",
                str(rogue),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())

    def test_producer_bytes_drift_rejects(self) -> None:
        ws = seeded_workspace()
        (ws.root / MATERIALIZER).write_bytes(b"#!/usr/bin/env bash\n# tampered\n")
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())

    def test_workflow_invoking_unpinned_script_rejects(self) -> None:
        ws = Workspace(
            extra_files={"scripts/forgotten-helper.sh": b"#!/usr/bin/env bash\n"}
        )
        workflow = ws.root / WORKFLOW_PATH
        workflow.write_text(
            workflow.read_text(encoding="utf-8")
            + "      - run: bash scripts/forgotten-helper.sh\n",
            encoding="utf-8",
            newline="\n",
        )
        # Re-bind the recorded workflow digest to the tampered bytes; the
        # unpinned invocation must still reject.
        lock_path = ws.root / STATIC_LOCK
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        lock["authority"]["workflow"]["sha256"] = lf_sha256(workflow.read_bytes())
        lock_path.write_text(
            json.dumps(lock, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())


class RejectsMutableAction(unittest.TestCase):
    def test_tag_pinned_action_rejects(self) -> None:
        ws = Workspace(workflow_text=valid_workflow().replace(ACTION_COMMIT, "v4.2.2"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("action", (result.stdout + result.stderr).lower())

    def test_unrecorded_action_commit_rejects(self) -> None:
        ws = Workspace(
            workflow_text=valid_workflow().replace(ACTION_COMMIT, OTHER_ACTION_COMMIT)
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("action", (result.stdout + result.stderr).lower())


def valid_workflow() -> str:
    return valid_workflow_text()


class RejectsContractMismatch(unittest.TestCase):
    def make(self, workflow_text: str) -> Workspace:
        ws = Workspace(workflow_text=workflow_text)
        return ws

    def test_push_trigger_rejects(self) -> None:
        ws = self.make(valid_workflow_text(trigger="push"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("trigger", (result.stdout + result.stderr).lower())

    def test_missing_materializer_invocation_rejects(self) -> None:
        text = valid_workflow().replace(
            f'      - run: bash {MATERIALIZER} --candidate-commit "${{{{ inputs.candidate_sha }}}}"\n',
            "",
        )
        ws = self.make(text)
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("materializer", (result.stdout + result.stderr).lower())

    def test_missing_read_permissions_rejects(self) -> None:
        text = valid_workflow().replace("permissions:\n  contents: read\n", "")
        ws = self.make(text)
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("permission", (result.stdout + result.stderr).lower())

    def test_wrong_static_lock_schema_rejects(self) -> None:
        ws = Workspace(static_lock=valid_static_lock(schema="other/1"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("schema", (result.stdout + result.stderr).lower())


class AcceptsCommittedContract(unittest.TestCase):
    def test_seeded_workspace_accepts(self) -> None:
        ws = seeded_workspace()
        result = ws.run()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_real_repository_files_accept(self) -> None:
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(REPO_ROOT),
                "--workflow",
                str(REPO_ROOT / WORKFLOW_PATH),
                "--script",
                str(REPO_ROOT / MATERIALIZER),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()

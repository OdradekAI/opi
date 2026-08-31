#!/usr/bin/env python3
"""Hermetic tests for scripts/verify-phase18-ci.py (task 18.16).

The CI verifier owns two contracts:

* static mode — the repository CI workflow gains ONLY the minimal
  Phase 18 attestation producer: every pre-existing merge-ref integration
  job survives, the new job is exactly one three-platform attestation
  producer that records the pull-request head separately from the
  merge-ref checkout, and no job ever substitutes head-only checkout
  semantics;
* receipt mode — a downloaded attestation receipt carries the complete
  identity contract (run, event, merge commit, separate PR head, workflow
  bytes digest, runner, required-job set).

Usage:
    python scripts/test_verify_phase18_ci.py
"""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify-phase18-ci.py")
SPEC = importlib.util.spec_from_file_location("verify_phase18_ci", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)

CHECKOUT_PIN = "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683"
UPLOAD_PIN = "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02"

PREEXISTING_JOBS = (
    "docs_contract",
    "fmt",
    "clippy",
    "test",
    "execution_acceptance",
    "phase17_acceptance",
    "doctest",
    "doc",
    "sandbox_package",
    "target_check",
)


def workflow_text(*, extra_job: str = "", drop_job: str | None = None,
                  head_only: str = "", pr_head_binding: str = (
                      "          PR_HEAD: ${{ github.event.pull_request.head.sha }}\n"),
                  receipt_body: str | None = None) -> str:
    jobs = [f"  {name}:\n    runs-on: ubuntu-latest\n" for name in PREEXISTING_JOBS]
    if drop_job is not None:
        jobs = [job for job in jobs if not job.startswith(f"  {drop_job}:")]
    if receipt_body is None:
        receipt_body = (
            '              "schema": "phase18-ci-attestation/1",\n'
            '              "run_id": int(os.environ["GITHUB_RUN_ID"]),\n'
            '              "run_attempt": int(os.environ["GITHUB_RUN_ATTEMPT"]),\n'
            '              "event": os.environ["GITHUB_EVENT_NAME"],\n'
            '              "ref": os.environ["GITHUB_REF"],\n'
            '              "merge_commit": os.environ["GITHUB_SHA"],\n'
            '              "pull_request_head": pr_head,\n'
            '              "workflow_sha256": hashlib.sha256(workflow_bytes).hexdigest(),\n'
            '              "runner_os": os.environ["RUNNER_OS"],\n'
            '              "required_jobs": REQUIRED_JOBS,\n'
            '              "attestation_commit": os.environ["GITHUB_SHA"],\n'
        )
    attestation = (
        "  phase18_attestation:\n"
        "    name: Phase 18 attestation (${{ matrix.os }})\n"
        "    strategy:\n"
        "      fail-fast: false\n"
        "      matrix:\n"
        "        os: [ubuntu-latest, windows-latest, macos-latest]\n"
        "    runs-on: ${{ matrix.os }}\n"
        "    steps:\n"
        f"      - uses: {CHECKOUT_PIN}\n"
        "      - name: Record attestation receipt\n"
        "        shell: bash\n"
        "        env:\n"
        f"{pr_head_binding}"
        "        run: |\n"
        "          set -euo pipefail\n"
        '          PY="$(command -v python3 || command -v python)"\n'
        '          "$PY" - "$PR_HEAD" <<\'PYEOF\'\n'
        "          import hashlib, json, os, sys\n"
        "          pr_head = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None\n"
        '          workflow_path = ".github/workflows/ci.yml"\n'
        '          workflow_bytes = open(workflow_path, "rb").read()\n'
        "          receipt = {\n"
        f"{receipt_body}"
        "          }\n"
        '          with open("phase18-attestation-receipt.json", "w", encoding="utf-8") as handle:\n'
        "              json.dump(receipt, handle, indent=2, sort_keys=True)\n"
        "          PYEOF\n"
        f"      - uses: {UPLOAD_PIN}\n"
        "        with:\n"
        "          name: phase18-attestation-${{ matrix.os }}\n"
        "          path: phase18-attestation-receipt.json\n"
        "          if-no-files-found: error\n"
    )
    return (
        "name: CI\n\n"
        "on:\n"
        "  push:\n"
        "    branches: [main]\n"
        "  pull_request:\n"
        "    branches: [main]\n\n"
        "permissions:\n"
        "  contents: read\n\n"
        "jobs:\n"
        + "".join(jobs)
        + attestation
        + extra_job
        + head_only
    )


def receipt_value(**overrides) -> dict:
    value = {
        "schema": "phase18-ci-attestation/1",
        "run_id": 1234567890,
        "run_attempt": 1,
        "event": "pull_request",
        "ref": "refs/pull/42/merge",
        "merge_commit": "a" * 40,
        "pull_request_head": "b" * 40,
        "workflow_path": ".github/workflows/ci.yml",
        "workflow_sha256": "c" * 64,
        "runner_os": "Linux",
        "required_jobs": list(PREEXISTING_JOBS) + ["phase18_attestation"],
        "attestation_commit": "a" * 40,
    }
    value.update(overrides)
    return value


class StaticWorkflowContract(unittest.TestCase):
    def run_static(self, text: str) -> list[str]:
        return verifier.verify_workflow_static(text)

    def test_repository_workflow_passes(self) -> None:
        repo = Path(__file__).resolve().parents[1]
        text = (repo / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertEqual([], self.run_static(text))

    def test_synthetic_workflow_passes(self) -> None:
        self.assertEqual([], self.run_static(workflow_text()))

    def test_removing_one_preexisting_job_is_rejected(self) -> None:
        findings = self.run_static(workflow_text(drop_job="doctest"))
        self.assertTrue(
            any("pre-existing integration job disappeared" in row for row in findings),
            findings,
        )

    def test_a_second_added_job_is_rejected(self) -> None:
        findings = self.run_static(
            workflow_text(extra_job="  phase18_extra:\n    runs-on: ubuntu-latest\n")
        )
        self.assertTrue(
            any("unexpected additional job" in row for row in findings), findings
        )

    def test_missing_attestation_job_is_rejected(self) -> None:
        text = workflow_text().replace("  phase18_attestation:\n", "  other_name:\n", 1)
        findings = self.run_static(text)
        self.assertTrue(
            any("attestation producer job is missing" in row for row in findings),
            findings,
        )

    def test_head_only_checkout_is_rejected(self) -> None:
        findings = self.run_static(
            workflow_text(
                head_only=(
                    "      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n"
                    "        with:\n"
                    "          ref: ${{ github.event.pull_request.head.sha }}\n"
                )
            )
        )
        self.assertTrue(
            any("head-only checkout" in row for row in findings), findings
        )

    def test_unpinned_action_is_rejected(self) -> None:
        findings = self.run_static(
            workflow_text().replace(CHECKOUT_PIN, "actions/checkout@v4", 1)
        )
        self.assertTrue(any("not pinned by a full commit" in row for row in findings),
                        findings)

    def test_missing_separate_pr_head_recording_is_rejected(self) -> None:
        findings = self.run_static(workflow_text(pr_head_binding=""))
        self.assertTrue(
            any("pull-request head is not recorded separately" in row
                for row in findings),
            findings,
        )

    def test_missing_receipt_identity_is_rejected(self) -> None:
        findings = self.run_static(
            workflow_text(
                receipt_body='              "schema": "phase18-ci-attestation/1",\n'
            )
        )
        self.assertTrue(
            any("receipt identity contract" in row for row in findings), findings
        )


class ReceiptContract(unittest.TestCase):
    def run_receipt(self, value: dict) -> list[str]:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "receipt.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            return verifier.verify_receipt_file(path)

    def test_valid_pull_request_receipt_passes(self) -> None:
        self.assertEqual([], self.run_receipt(receipt_value()))

    def test_valid_push_receipt_has_no_pr_head(self) -> None:
        self.assertEqual(
            [],
            self.run_receipt(
                receipt_value(event="push", ref="refs/heads/main",
                              pull_request_head=None)
            ),
        )

    def test_push_receipt_with_pr_head_is_rejected(self) -> None:
        findings = self.run_receipt(receipt_value(event="push"))
        self.assertTrue(any("push event must not carry" in row for row in findings),
                        findings)

    def test_pull_request_without_separate_head_is_rejected(self) -> None:
        findings = self.run_receipt(
            receipt_value(pull_request_head=None)
        )
        self.assertTrue(
            any("pull-request head is missing" in row for row in findings), findings
        )

    def test_pr_head_equal_to_merge_commit_is_rejected(self) -> None:
        findings = self.run_receipt(
            receipt_value(pull_request_head="a" * 40)
        )
        self.assertTrue(
            any("head-only" in row for row in findings), findings
        )

    def test_unknown_schema_is_rejected(self) -> None:
        findings = self.run_receipt(receipt_value(schema="other/1"))
        self.assertTrue(any("unexpected schema" in row for row in findings), findings)

    def test_workflow_digest_must_be_sha256_hex(self) -> None:
        findings = self.run_receipt(receipt_value(workflow_sha256="short"))
        self.assertTrue(
            any("workflow digest" in row for row in findings), findings
        )

    def test_required_jobs_must_match_the_contract(self) -> None:
        findings = self.run_receipt(
            receipt_value(required_jobs=["docs_contract"])
        )
        self.assertTrue(
            any("required-job set" in row for row in findings), findings
        )

    def test_attestation_commit_must_equal_merge_commit(self) -> None:
        findings = self.run_receipt(receipt_value(attestation_commit="d" * 40))
        self.assertTrue(
            any("attestation commit" in row for row in findings), findings
        )


def terminal_inputs(**overrides) -> dict:
    """A completed three-platform green run with its downloaded inner
    receipt, shaped like the real GitHub run/jobs/artifact metadata."""
    head = "0" * 40
    merge = "8" * 40
    run = {
        "id": 1,
        "status": "completed",
        "conclusion": "success",
        "event": "pull_request",
        "head_sha": head,
        "run_attempt": 1,
        "updated_at": "2026-08-30T10:08:32Z",
    }
    jobs = {"jobs": [
        {"name": display + " (ubuntu-latest)", "conclusion": "success",
         "head_sha": head}
        for display in (
            "docs_contract", "fmt", "clippy", "test",
            "execution_acceptance", "Phase 17 acceptance", "doctest", "doc",
            "opi-sandbox package", "Target check", "Phase 18 attestation",
        )
    ]}
    artifacts = {"artifacts": [
        {"id": 7 + index,
         "name": f"phase18-attestation-{os_name}-latest",
         "expired": False, "digest": "sha256:" + "a" * 64,
         "size_in_bytes": 10}
        for index, os_name in enumerate(
            ("ubuntu", "windows", "macos"))
    ]}
    inner = {
        "schema": "phase18-ci-attestation/1",
        "run_id": 1,
        "run_attempt": 1,
        "event": "pull_request",
        "merge_commit": merge,
        "pull_request_head": head,
        "workflow_path": ".github/workflows/ci.yml",
        "workflow_sha256": None,  # filled by the caller fixture
        "runner_os": "Linux",
        "required_jobs": list(verifier.REQUIRED_JOBS),
        "attestation_commit": merge,
        "_artifact_id": 7,
        "_artifact_sha256": "sha256:" + "a" * 64,
    }
    values = {"run": run, "jobs": jobs, "artifacts": artifacts,
              "inner": inner}
    values.update(overrides)
    return values


class TerminalMode(unittest.TestCase):
    """The post-terminal verifier binds or rejects, never self-claims."""

    def run_terminal(self, values: dict, head: str = "0" * 40) -> list[str]:
        """Run verify_terminal with the workflow bytes staged in a temp
        repo; on success the bound receipt must be status=verified."""
        import copy
        import hashlib

        with tempfile.TemporaryDirectory() as repo:
            workflow = Path(repo) / ".github" / "workflows" / "ci.yml"
            workflow.parent.mkdir(parents=True)
            workflow.write_bytes(b"workflow-bytes")
            values["inner"]["workflow_sha256"] = hashlib.sha256(
                b"workflow-bytes").hexdigest()
            findings, receipt = verifier.verify_terminal(
                head, values["run"], values["jobs"], values["artifacts"],
                copy.deepcopy(values["inner"]), Path(repo),
            )
            if not findings:
                self.assertIsNotNone(receipt)
                self.assertEqual(receipt["status"], "verified")
            return findings

    def test_green_run_binds(self) -> None:
        self.assertEqual([], self.run_terminal(terminal_inputs()))

    def test_failed_job_is_rejected(self) -> None:
        values = terminal_inputs()
        values["jobs"]["jobs"][0]["conclusion"] = "failure"
        self.assertTrue(any("not every job succeeded" in row
                            for row in self.run_terminal(values)))

    def test_skipped_job_is_rejected(self) -> None:
        values = terminal_inputs()
        values["jobs"]["jobs"][0]["conclusion"] = "skipped"
        self.assertTrue(any("not every job succeeded" in row
                            for row in self.run_terminal(values)))

    def test_head_mismatch_is_rejected(self) -> None:
        self.assertTrue(any("run head" in row
                            for row in self.run_terminal(
                                terminal_inputs(), head="1" * 40)))

    def test_expired_artifact_is_rejected(self) -> None:
        values = terminal_inputs()
        values["artifacts"]["artifacts"][0]["expired"] = True
        self.assertTrue(any("expired" in row
                            for row in self.run_terminal(values)))

    def test_digest_drift_is_rejected(self) -> None:
        values = terminal_inputs()
        values["inner"]["_artifact_sha256"] = "sha256:" + "0" * 64
        self.assertTrue(any("downloaded artifact bytes" in row
                            for row in self.run_terminal(values)))

    def test_fork_only_push_event_is_rejected(self) -> None:
        values = terminal_inputs()
        values["run"]["event"] = "push"
        self.assertTrue(any("pull_request event" in row
                            for row in self.run_terminal(values)))

    def test_head_only_merge_identity_is_rejected(self) -> None:
        values = terminal_inputs()
        values["inner"]["merge_commit"] = "0" * 40
        values["inner"]["attestation_commit"] = "0" * 40
        self.assertTrue(any("head-only semantics" in row
                            for row in self.run_terminal(values)))


if __name__ == "__main__":
    unittest.main(verbosity=2)

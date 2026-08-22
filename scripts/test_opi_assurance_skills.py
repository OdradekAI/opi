from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
VALIDATOR = (
    ROOT
    / ".agents"
    / "skills"
    / "_shared"
    / "scripts"
    / "validate_assurance_artifact.py"
)
LINEAGE = (
    ROOT
    / ".agents"
    / "skills"
    / "_shared"
    / "scripts"
    / "compare_finding_lineage.py"
)
HEAD = "136c380f0c5eea541190cc1a0f5c1d62f983b4e8"


def run_script(script: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(script), *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def finding_record() -> dict[str, object]:
    return {
        "id": "P17-CODEX-SPEC-001",
        "source_kind": "audit",
        "source_path": (
            "docs/snapshots/phase17/"
            "audit.codex.136c380.run1.md"
        ),
        "source_model": "codex",
        "observed_at": HEAD,
        "independence": "fresh-context-same-family",
        "axis": "spec",
        "severity": "Major",
        "title": "Durable session binding is absent",
        "claim": "New sessions do not persist the required runtime binding.",
        "evidence": [
            {
                "location": "crates/opi-agent/src/session.rs:42",
                "detail": "The serialized header has no runtime binding.",
            }
        ],
        "criterion_source": "docs/opi-spec.md#INV-007",
        "reproduction": ["cargo test -p opi-agent --test session_contract"],
        "confidence": "high",
        "status": "unverified",
    }


def disposition_record(
    *,
    closure_key: str = "session.runtime-binding",
    family_key: str = "session.durability",
    lineage_kind: str = "new",
    prior_disposition: str = "none",
) -> dict[str, object]:
    return {
        "source": {
            "source_path": (
                "docs/snapshots/phase17/"
                "audit.codex.136c380.run1.md"
            ),
            "id": "P17-CODEX-SPEC-001",
        },
        "verified_at": HEAD,
        "verification_status": "Confirmed",
        "final_severity": "Major",
        "final_severity_rationale": "The mandatory durable binding is absent.",
        "closure_key": closure_key,
        "family_key": family_key,
        "lineage": {
            "kind": lineage_kind,
            "prior_occurrences": [],
            "prior_disposition": prior_disposition,
        },
        "decision": "D6",
        "closure_batch": "B2",
        "change_kind": "behavioral",
        "red_before": {
            "command": "cargo test -p opi-agent --test session_contract binding",
            "expected": "FAIL because the binding is absent",
            "observed": "FAIL: binding was None",
        },
        "green_after": {
            "command": "cargo test -p opi-agent --test session_contract binding",
            "expected": "PASS",
            "observed": "PASS",
        },
    }


class AssuranceArtifactValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_jsonl(self, name: str, records: list[dict[str, object]]) -> Path:
        path = self.root / name
        path.write_text(
            "".join(json.dumps(record) + "\n" for record in records),
            encoding="utf-8",
        )
        return path

    def test_valid_immutable_finding_artifact_passes(self) -> None:
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [finding_record()],
        )
        (self.root / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("findings: PASS", result.stdout)

    def test_empty_immutable_finding_artifact_represents_no_findings(self) -> None:
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [],
        )
        (self.root / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertEqual(0, result.returncode, result.stderr)

    def test_finding_sidecar_requires_immutable_report_sibling(self) -> None:
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [finding_record()],
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("missing immutable report sibling", result.stderr)

    def test_mutable_finding_artifact_name_is_rejected(self) -> None:
        path = self.write_jsonl("audit.codex.findings.jsonl", [finding_record()])

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("immutable audit findings filename", result.stderr)

    def test_finding_requires_observed_at(self) -> None:
        record = finding_record()
        del record["observed_at"]
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [record],
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("missing fields: observed_at", result.stderr)

    def test_finding_requires_nonempty_claim_and_structured_evidence(self) -> None:
        record = finding_record()
        record["claim"] = ""
        record["evidence"] = [{"location": "", "detail": ""}]
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [record],
        )
        (self.root / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("claim must be non-empty", result.stderr)
        self.assertIn("evidence entries require non-empty location and detail", result.stderr)

    def test_finding_source_path_must_be_repo_relative_phase_snapshot(self) -> None:
        record = finding_record()
        record["source_path"] = "audit.codex.136c380.run1.md"
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [record],
        )
        (self.root / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("repo-relative Phase snapshot report", result.stderr)

    def test_finding_source_path_phase_must_match_physical_sibling(self) -> None:
        record = finding_record()
        record["source_path"] = (
            "docs/snapshots/phase999/audit.codex.136c380.run1.md"
        )
        phase_dir = self.root / "docs" / "snapshots" / "phase17"
        phase_dir.mkdir(parents=True)
        path = phase_dir / "audit.codex.136c380.run1.findings.jsonl"
        path.write_text(json.dumps(record) + "\n", encoding="utf-8")
        (phase_dir / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source_path Phase must match artifact directory", result.stderr)

    def test_audit_findings_sidecar_cannot_claim_eval_source_kind(self) -> None:
        record = finding_record()
        record["source_kind"] = "eval"
        path = self.write_jsonl(
            "audit.codex.136c380.run1.findings.jsonl",
            [record],
        )
        (self.root / "audit.codex.136c380.run1.md").write_text(
            "# Audit\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "findings", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("audit findings source_kind must be audit", result.stderr)

    def test_valid_plan_disposition_artifact_passes(self) -> None:
        record = disposition_record()
        record["green_after"]["observed"] = "not-run"
        path = self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [record],
        )
        (self.root / "remediation.136c380.r4.plan.md").write_text(
            "# Plan\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("dispositions: PASS", result.stdout)

    def test_valid_result_disposition_artifact_passes(self) -> None:
        plan_record = disposition_record()
        plan_record["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_record],
        )
        (self.root / "remediation.136c380.r4.plan.md").write_text(
            "# Plan\n",
            encoding="utf-8",
        )
        record = disposition_record()
        record["remediation_status"] = "Closed"
        path = self.write_jsonl(
            "remediation.136c380.r4.result.dispositions.jsonl",
            [record],
        )
        (self.root / "remediation.136c380.r4.result.md").write_text(
            "# Result\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertEqual(0, result.returncode, result.stderr)

    def test_result_disposition_requires_remediation_status(self) -> None:
        plan_record = disposition_record()
        plan_record["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_record],
        )
        (self.root / "remediation.136c380.r4.plan.md").write_text(
            "# Plan\n",
            encoding="utf-8",
        )
        path = self.write_jsonl(
            "remediation.136c380.r4.result.dispositions.jsonl",
            [disposition_record()],
        )
        (self.root / "remediation.136c380.r4.result.md").write_text(
            "# Result\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("result disposition requires remediation_status", result.stderr)

    def test_behavioral_disposition_requires_red_before(self) -> None:
        record = disposition_record()
        del record["red_before"]
        path = self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [record],
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("behavioral disposition requires red_before", result.stderr)

    def test_one_closure_batch_cannot_hide_multiple_closure_keys(self) -> None:
        first = disposition_record()
        first["green_after"]["observed"] = "not-run"
        second = disposition_record(closure_key="session.required-entry")
        second["source"]["id"] = "P17-CODEX-SPEC-002"
        second["green_after"]["observed"] = "not-run"
        path = self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [first, second],
        )
        (self.root / "remediation.136c380.r4.plan.md").write_text(
            "# Plan\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("closure batch B2 has multiple closure keys", result.stderr)

    def test_result_disposition_cannot_drift_from_plan_stage(self) -> None:
        plan_record = disposition_record()
        plan_record["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_record],
        )
        (self.root / "remediation.136c380.r4.plan.md").write_text(
            "# Plan\n",
            encoding="utf-8",
        )
        result_record = disposition_record()
        result_record["final_severity"] = "Minor"
        result_record["remediation_status"] = "Closed"
        path = self.write_jsonl(
            "remediation.136c380.r4.result.dispositions.jsonl",
            [result_record],
        )
        (self.root / "remediation.136c380.r4.result.md").write_text(
            "# Result\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "dispositions", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("drifts from plan-stage disposition", result.stderr)

    def test_ready_plan_passes_when_decisions_and_closure_proofs_are_complete(
        self,
    ) -> None:
        plan_disposition = disposition_record()
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source: docs/snapshots/phase17/audit.codex.136c380.run1.md "
            "P17-CODEX-SPEC-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: a reconstructed branch returns the exact binding\n"
            "- **Red-before**: `cargo test binding` -> FAIL: binding absent\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("plan: PASS", result.stdout)

    def test_ready_plan_requires_observed_red_before(self) -> None:
        plan_disposition = disposition_record()
        plan_disposition["red_before"]["observed"] = "not-run"
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source: docs/snapshots/phase17/audit.codex.136c380.run1.md "
            "P17-CODEX-SPEC-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: the binding survives reconstruction\n"
            "- **Red-before**: pending\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("READY-FOR-APPLY requires observed red-before", result.stderr)

    def test_ready_plan_requires_nonempty_closure_proof_fields(self) -> None:
        plan_disposition = disposition_record()
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source: docs/snapshots/phase17/audit.codex.136c380.run1.md "
            "P17-CODEX-SPEC-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: the binding survives reconstruction\n"
            "- **Red-before**:\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("empty Red-before", result.stderr)

    def test_ready_plan_requires_concrete_green_after_command(self) -> None:
        plan_disposition = disposition_record()
        plan_disposition["green_after"]["command"] = ""
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source: docs/snapshots/phase17/audit.codex.136c380.run1.md "
            "P17-CODEX-SPEC-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: the binding survives reconstruction\n"
            "- **Red-before**: `cargo test binding` -> FAIL\n"
            "- **Green-after**: pending command\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("READY-FOR-APPLY requires concrete green-after", result.stderr)

    def test_ready_plan_must_cover_every_source_disposition(self) -> None:
        first = disposition_record()
        first["green_after"]["observed"] = "not-run"
        second = disposition_record()
        second["source"]["id"] = "P17-CODEX-SPEC-002"
        second["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [first, second],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source: docs/snapshots/phase17/audit.codex.136c380.run1.md "
            "P17-CODEX-SPEC-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: the binding survives reconstruction\n"
            "- **Red-before**: `cargo test binding` -> FAIL\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("missing source disposition P17-CODEX-SPEC-002", result.stderr)

    def test_ready_plan_cannot_carry_pending_decisions(self) -> None:
        plan_disposition = disposition_record()
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: READY-FOR-APPLY\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: D6\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: the binding survives reconstruction\n"
            "- **Red-before**: `cargo test binding` -> FAIL\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("READY-FOR-APPLY requires no unresolved decisions", result.stderr)

    def test_draft_unresolved_plan_can_stop_before_fix_design(self) -> None:
        plan_disposition = disposition_record()
        plan_disposition["green_after"]["observed"] = "not-run"
        self.write_jsonl(
            "remediation.136c380.r4.plan.dispositions.jsonl",
            [plan_disposition],
        )
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: DRAFT-UNRESOLVED\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: D6\n\n"
            "## Unresolved decisions\n\n"
            "| ID | Required decision | Authority needed |\n"
            "|---|---|---|\n"
            "| D6 | Choose public policy | registered product source |\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertEqual(0, result.returncode, result.stderr)

    def test_plan_requires_its_exact_disposition_sibling(self) -> None:
        path = self.root / "remediation.136c380.r4.plan.md"
        path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            "**Status**: DRAFT-UNRESOLVED\n"
            f"**Verification target**: committed `{HEAD}`\n"
            "**Disposition artifact**: `remediation.136c380.r4.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: D6\n",
            encoding="utf-8",
        )

        result = run_script(VALIDATOR, "plan", str(path))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("missing exact plan disposition sibling", result.stderr)


class FindingLineageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_jsonl(self, name: str, records: list[dict[str, object]]) -> Path:
        path = self.root / name
        path.write_text(
            "".join(json.dumps(record) + "\n" for record in records),
            encoding="utf-8",
        )
        return path

    def compare(
        self,
        current: list[dict[str, object]],
        history: list[dict[str, object]],
    ) -> list[dict[str, object]]:
        current_path = self.write_jsonl("current.jsonl", current)
        history_path = self.write_jsonl("history.jsonl", history)
        result = run_script(
            LINEAGE,
            "--current",
            str(current_path),
            "--history",
            str(history_path),
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return [json.loads(line) for line in result.stdout.splitlines() if line]

    def test_exact_closed_issue_is_recurrent_same_defect(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Closed"
        current = {
            "closure_key": "session.runtime-binding",
            "family_key": "session.durability",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual("recurrent-same-defect", compared[0]["lineage"]["kind"])

    def test_exact_deferred_issue_is_carried_forward(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Deferred by registered source"
        current = {
            "closure_key": "session.runtime-binding",
            "family_key": "session.durability",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual(
            "carried-forward-deferred",
            compared[0]["lineage"]["kind"],
        )

    def test_exact_info_no_action_issue_is_recurrent_not_deferred(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Info/No action"
        current = {
            "closure_key": "session.runtime-binding",
            "family_key": "session.durability",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual("recurrent-same-defect", compared[0]["lineage"]["kind"])

    def test_mixed_closed_and_deferred_history_is_recurrent(self) -> None:
        closed = disposition_record()
        closed["remediation_status"] = "Closed"
        deferred = disposition_record()
        deferred["source"]["id"] = "P17-CODEX-SPEC-000"
        deferred["remediation_status"] = "Deferred by registered source"
        current = {
            "closure_key": "session.runtime-binding",
            "family_key": "session.durability",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [closed, deferred])

        self.assertEqual("recurrent-same-defect", compared[0]["lineage"]["kind"])
        self.assertIn("Closed", compared[0]["lineage"]["prior_disposition"])
        self.assertIn(
            "Deferred by registered source",
            compared[0]["lineage"]["prior_disposition"],
        )

    def test_known_passing_change_can_be_classified_as_regression(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Closed"
        current = {
            "closure_key": "session.required-entry",
            "family_key": "session.durability",
            "regression_of": "session.runtime-binding",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual("regression", compared[0]["lineage"]["kind"])

    def test_deferred_history_cannot_prove_regression(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Deferred by registered source"
        current = {
            "closure_key": "session.required-entry",
            "family_key": "session.durability",
            "regression_of": "session.runtime-binding",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual(
            "recurrent-adjacent-path",
            compared[0]["lineage"]["kind"],
        )

    def test_same_family_different_closure_is_adjacent_path(self) -> None:
        history = disposition_record()
        history["remediation_status"] = "Closed"
        current = {
            "closure_key": "session.required-entry",
            "family_key": "session.durability",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [history])

        self.assertEqual(
            "recurrent-adjacent-path",
            compared[0]["lineage"]["kind"],
        )

    def test_unseen_family_is_new(self) -> None:
        current = {
            "closure_key": "queue.observable-full",
            "family_key": "agent.control-queue",
            "verified_at": HEAD,
        }

        compared = self.compare([current], [])

        self.assertEqual("new", compared[0]["lineage"]["kind"])

    def test_first_run_can_omit_history_argument(self) -> None:
        current_path = self.write_jsonl(
            "current-without-history.jsonl",
            [
                {
                    "closure_key": "queue.observable-full",
                    "family_key": "agent.control-queue",
                    "verified_at": HEAD,
                }
            ],
        )

        result = run_script(LINEAGE, "--current", str(current_path))

        self.assertEqual(0, result.returncode, result.stderr)
        compared = json.loads(result.stdout)
        self.assertEqual("new", compared["lineage"]["kind"])

    def test_history_can_span_multiple_result_disposition_artifacts(self) -> None:
        first = disposition_record(
            closure_key="session.other",
            family_key="session.other-family",
        )
        first["remediation_status"] = "Closed"
        second = disposition_record()
        second["remediation_status"] = "Closed"
        current_path = self.write_jsonl(
            "current-multiple.jsonl",
            [
                {
                    "closure_key": "session.required-entry",
                    "family_key": "session.durability",
                    "verified_at": HEAD,
                }
            ],
        )
        first_path = self.write_jsonl("history-one.jsonl", [first])
        second_path = self.write_jsonl("history-two.jsonl", [second])

        result = run_script(
            LINEAGE,
            "--current",
            str(current_path),
            "--history",
            str(first_path),
            str(second_path),
        )

        self.assertEqual(0, result.returncode, result.stderr)
        compared = json.loads(result.stdout)
        self.assertEqual("recurrent-adjacent-path", compared["lineage"]["kind"])


if __name__ == "__main__":
    unittest.main()

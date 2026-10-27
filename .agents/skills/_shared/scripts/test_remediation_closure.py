from __future__ import annotations

import hashlib
import json
import unittest

from test_opi_assurance_skills import (
    AssuranceFixture,
    disposition_record,
    finding_record,
    requirement_record,
    run_validator,
    write_jsonl,
)


class RemediationClosureTests(AssuranceFixture):
    def setUp(self) -> None:
        super().setUp()
        self.write_audit_set(
            requirements=[requirement_record(state="not-met", finding_ids=["P17-AUD-001"])],
            findings=[finding_record()],
        )
        self.meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(encoding="utf-8")
        )

    def record(self, **changes):
        record = disposition_record(self.meta["findings_sha256"])
        record["remaining_obligations"] = []
        record.update(changes)
        return record

    def no_action(self, status="Refuted", obligations=None):
        record = self.record(
            verification_status=status,
            decision="no-action:retain-current-behavior",
            closure_batch=None,
            change_kind="metadata",
            changed_paths=[],
            remaining_obligations=obligations or [],
            red_before_not_applicable="This review records a disposition and authorizes no behavior edit.",
        )
        record.pop("red_before")
        record["green_after"] = {
            "command": "inspect current consumer with explicit and omitted options",
            "expected": "Record the observed boundary and remaining obligation",
            "observed": "not-run",
        }
        return record

    def plan(self, record=None, status="READY-FOR-APPLY", dependencies="[]"):
        plan, sidecar = self.write_plan(disposition=record or self.record(), status=status)
        text = plan.read_text(encoding="utf-8")
        # Keep compatibility with the shared fixture while it gains batch headers.
        if "### Batch B1:" not in text:
            text = text.replace("#### Fix B1.1:", f"### Batch B1: Binding\n\n**Dependencies**: {dependencies}\n\n#### Fix B1.1:")
        else:
            text = text.replace("**Dependencies**: []", f"**Dependencies**: {dependencies}")
        if record is not None and record["closure_batch"] is None:
            text = text.split("### Batch B1:")[0]
        plan.write_text(text, encoding="utf-8")
        return plan, sidecar

    def result(self, plan, sidecar, outcome="APPLIED", status="Closed"):
        result, records_path = self.write_result(plan, sidecar)
        text = result.read_text(encoding="utf-8")
        if "**Outcome**:" not in text:
            text += f"**Outcome**: {outcome}\n"
        else:
            text = text.replace("**Outcome**: APPLIED", f"**Outcome**: {outcome}")
        result.write_text(text, encoding="utf-8")
        records = [json.loads(line) for line in records_path.read_text(encoding="utf-8").splitlines()]
        for record in records:
            record["remediation_status"] = status
        write_jsonl(records_path, records)
        return result, records_path

    def test_refuted_claim_with_remaining_obligation_cannot_be_ready(self):
        plan = self.two_batch_plan(([], ["B1"]))
        sidecar = plan.with_name("remediation.plan.dispositions.jsonl")
        records = [json.loads(line) for line in sidecar.read_text(encoding="utf-8").splitlines()]
        refuted = self.no_action(obligations=["Explicit forcing precedence needs source adjudication."])
        refuted["source"]["id"] = "P17-AUD-002"
        write_jsonl(sidecar, [records[0], refuted])
        plan.write_text(plan.read_text(encoding="utf-8").split("### Batch B2:")[0], encoding="utf-8")
        result = run_validator("plan", plan)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("remaining obligations", result.stderr)

    def test_refuted_and_cannot_confirm_cannot_authorize_a_fix(self):
        for status in ("Refuted", "Cannot confirm"):
            with self.subTest(status=status):
                plan, _ = self.plan(self.record(verification_status=status))
                result = run_validator("plan", plan)
                self.assertNotEqual(0, result.returncode)
                self.assertIn("fix requires Confirmed or Partially confirmed", result.stderr)

    def test_remaining_obligations_are_required_and_typed(self):
        for obligations in (None, "none", [""]):
            with self.subTest(obligations=obligations):
                record = self.record(remaining_obligations=obligations)
                if obligations is None:
                    record.pop("remaining_obligations")
                plan, _ = self.plan(record)
                result = run_validator("plan", plan)
                self.assertNotEqual(0, result.returncode)
                self.assertIn("remaining_obligations", result.stderr)

    def test_verification_commit_must_match_remediation_head(self):
        plan, _ = self.plan(self.record(verified_at="a" * 40))
        result = run_validator("plan", plan)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("verified_at must match Remediation head", result.stderr)

    def test_no_change_can_finish_refutation_or_evidence_review_without_fix(self):
        for verification, remaining in (("Refuted", []), ("Refuted", ["Ordering applicability requires source adjudication."]), ("Cannot confirm", ["Required native proof is unavailable."])):
            with self.subTest(verification=verification, remaining=remaining):
                plan, sidecar = self.plan(self.no_action(verification, remaining), "DRAFT-UNRESOLVED")
                result, _ = self.result(plan, sidecar, "NO-CHANGE", verification)
                checked = run_validator("result", result)
                self.assertEqual(0, checked.returncode, checked.stderr)
                digest = hashlib.sha256(plan.read_bytes() + b"\0" + sidecar.read_bytes()).hexdigest()
                applied = run_validator("apply", plan, "--plan-sha256", digest)
                self.assertNotEqual(0, applied.returncode)
                self.assertIn("apply requires READY-FOR-APPLY", applied.stderr)

    def test_no_action_result_cannot_swap_refuted_and_unconfirmed(self):
        for verification, result_status in (("Cannot confirm", "Refuted"), ("Refuted", "Cannot confirm")):
            with self.subTest(verification=verification):
                plan, sidecar = self.plan(self.no_action(verification), "DRAFT-UNRESOLVED")
                result, _ = self.result(plan, sidecar, "NO-CHANGE", result_status)
                checked = run_validator("result", result)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("result evidence status must match verification_status", checked.stderr)

    def test_no_change_cannot_hide_a_planned_edit(self):
        plan, sidecar = self.plan(self.record(), "DRAFT-UNRESOLVED")
        result, _ = self.result(plan, sidecar, "NO-CHANGE")
        checked = run_validator("result", result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("NO-CHANGE requires only no-action dispositions", checked.stderr)

    def test_no_change_accepts_an_empty_current_finding_union(self):
        self.write_audit_set()
        plan, sidecar = self.plan(self.no_action(), "DRAFT-UNRESOLVED")
        write_jsonl(sidecar, [])
        result, _ = self.result(plan, sidecar, "NO-CHANGE", "Refuted")
        checked = run_validator("result", result)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_applied_result_cannot_complete_an_unresolved_draft(self):
        plan, sidecar = self.plan(self.record(), "DRAFT-UNRESOLVED")
        result, _ = self.result(plan, sidecar)
        checked = run_validator("result", result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("APPLIED requires READY-FOR-APPLY", checked.stderr)

    def test_no_action_cannot_claim_closed_or_change_paths(self):
        plan, sidecar = self.plan(self.no_action(), "DRAFT-UNRESOLVED")
        result, _ = self.result(plan, sidecar, "NO-CHANGE", "Closed")
        checked = run_validator("result", result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("no-action cannot claim Closed", checked.stderr)
        record = self.no_action()
        record["changed_paths"] = ["crates/opi-agent/src/session.rs"]
        plan, _ = self.plan(record, "DRAFT-UNRESOLVED")
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("no-action requires metadata, no batch and no changed paths", checked.stderr)

    def test_result_cannot_discard_remaining_obligations(self):
        plan, sidecar = self.plan(self.no_action(obligations=["Source decision pending."]), "DRAFT-UNRESOLVED")
        result, result_sidecar = self.result(plan, sidecar, "NO-CHANGE", "Refuted")
        record = json.loads(result_sidecar.read_text(encoding="utf-8"))
        record["remaining_obligations"] = []
        write_jsonl(result_sidecar, [record])
        checked = run_validator("result", result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("drifts from plan-stage disposition", checked.stderr)

    def test_batch_rejects_unknown_self_and_malformed_dependencies(self):
        for dependency in ('["B9"]', '["B1"]', '"none"', '["B9", "B9"]'):
            with self.subTest(dependency=dependency):
                plan, _ = self.plan(dependencies=dependency)
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("dependenc", checked.stderr)

    def two_batch_plan(self, dependencies):
        run_id = self.meta["audit_run_id"]
        self.write_audit_set(
            requirements=[requirement_record(state="not-met", finding_ids=["P17-AUD-001", "P17-AUD-002"])],
            findings=[finding_record(), finding_record(finding_id="P17-AUD-002")],
        )
        self.meta = json.loads((self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(encoding="utf-8"))
        first = self.record()
        second = self.record(closure_batch="B2", closure_key="session.other-binding")
        second["source"]["id"] = "P17-AUD-002"
        plan, sidecar = self.plan(first, dependencies=json.dumps(dependencies[0]))
        write_jsonl(sidecar, [first, second])
        text = plan.read_text(encoding="utf-8")
        text += f"\nSource finding: {run_id}/P17-AUD-002\n### Batch B2: Second\n\n**Dependencies**: {json.dumps(dependencies[1])}\n\n#### Fix B2.1: Second binding\n- **Closure predicate**: second binding retained\n- **Red-before**: FAIL\n- **Green-after**: PASS\n"
        plan.write_text(text, encoding="utf-8")
        return plan

    def test_dependencies_require_acyclic_topological_order(self):
        for dependencies in ((["B2"], ["B1"]), (["B2"], [])):
            with self.subTest(dependencies=dependencies):
                plan = self.two_batch_plan(dependencies)
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("dependenc", checked.stderr)
        plan = self.two_batch_plan(([], ["B1"]))
        checked = run_validator("plan", plan)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_batch_headers_must_exactly_cover_dispositions(self):
        for replacement in ("### Batch B9:", "### Other B1:"):
            with self.subTest(replacement=replacement):
                plan, _ = self.plan()
                plan.write_text(plan.read_text(encoding="utf-8").replace("### Batch B1:", replacement), encoding="utf-8")
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("batch", checked.stderr)


if __name__ == "__main__":
    unittest.main()

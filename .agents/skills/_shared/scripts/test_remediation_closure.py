from __future__ import annotations

import hashlib
import json
import unittest

from test_opi_assurance_skills import (
    AssuranceFixture,
    disposition_record,
    finding_record,
    finding_source_tuple,
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

    def compound_plan(self):
        record = self.record(closure_batches=["B1", "B2"])
        record["closure_key"] = "session.binding-and-reopen"
        record["changed_paths"].append("crates/opi-agent/src/reopen.rs")
        record["red_before"] = {
            "command": "python check_binding_and_reopen.py",
            "expected": "FAIL if either binding persistence or reopen loses the binding",
            "observed": "FAIL: binding persistence and reopen both lose the binding",
        }
        record["green_after"] = {
            "command": "python check_binding_and_reopen.py",
            "expected": "PASS only when both binding persistence and reopen retain the binding",
            "observed": "not-run",
        }
        plan, sidecar = self.write_plan(disposition=record)
        text = plan.read_text(encoding="utf-8")
        text += (
            "\n### Batch B2: Reopen persisted binding\n\n"
            '**Dependencies**: ["B1"]\n\n'
            "#### Fix B2.1: Restore the persisted binding on reopen\n\n"
            f"- **Finding source(s)**: {finding_source_tuple(record)}\n"
            "- **Closure predicate**: reopening retains the persisted binding\n"
            "- **Red-before**: `cargo test reopen` -> FAIL\n"
            "- **Green-after**: `cargo test reopen` -> PASS\n"
        )
        plan.write_text(text, encoding="utf-8")
        return plan, sidecar

    def test_one_source_can_require_two_distinct_dependent_batches(self):
        plan, sidecar = self.compound_plan()
        checked = run_validator("plan", plan)
        self.assertEqual(0, checked.returncode, checked.stderr)
        result, _ = self.result(plan, sidecar)
        checked = run_validator("result", result)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_compound_batch_requires_its_own_finding_source_field(self):
        plan, sidecar = self.compound_plan()
        source = finding_source_tuple(json.loads(sidecar.read_text(encoding="utf-8")))
        first, second = plan.read_text(encoding="utf-8").split("### Batch B2:", 1)
        second = second.replace(f"- **Finding source(s)**: {source}\n", "")
        # The exact tuple elsewhere in the report must not satisfy B2.
        text = first + "### Batch B2:" + second + f"\n## Summary\n{source}\n"
        plan.write_text(text, encoding="utf-8")
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("batch B2", checked.stderr)
        self.assertIn("Finding source(s)", checked.stderr)

    def test_batch_source_tuple_must_match_exact_run_digest_and_id(self):
        for position, replacement in ((0, "other-run"), (1, "0" * 64), (2, "P17-AUD-002")):
            with self.subTest(position=position):
                plan, sidecar = self.compound_plan()
                source = finding_source_tuple(json.loads(sidecar.read_text(encoding="utf-8")))
                parts = source.split(" / ")
                parts[position] = replacement
                first, second = plan.read_text(encoding="utf-8").split("### Batch B2:", 1)
                second = second.replace(source, " / ".join(parts))
                plan.write_text(first + "### Batch B2:" + second, encoding="utf-8")
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("batch B2 has unassigned Finding source(s)", checked.stderr)

    def test_known_source_assigned_to_another_batch_is_rejected(self):
        plan = self.two_batch_plan(([], ["B1"]))
        sidecar = plan.with_name("remediation.plan.dispositions.jsonl")
        records = [json.loads(line) for line in sidecar.read_text(encoding="utf-8").splitlines()]
        text = plan.read_text(encoding="utf-8").replace(
            finding_source_tuple(records[1]), finding_source_tuple(records[0])
        )
        plan.write_text(text, encoding="utf-8")
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("batch B2 has unassigned Finding source(s)", checked.stderr)
        self.assertIn("batch B2 is missing assigned Finding source(s)", checked.stderr)

    def test_finding_source_field_requires_one_canonical_tuple_list(self):
        for value in ("", "P17-AUD-001", "run / digest / id / extra", "run / digest / id;", "`run / digest / id", "run / digest / `id", "duplicate-field"):
            with self.subTest(value=value):
                plan, sidecar = self.compound_plan()
                source = finding_source_tuple(json.loads(sidecar.read_text(encoding="utf-8")))
                if value == "duplicate-field":
                    value = source + "\n- **Finding source(s)**: " + source
                text = plan.read_text(encoding="utf-8").replace(source, value)
                plan.write_text(text, encoding="utf-8")
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("Finding source(s)", checked.stderr)

    def test_finding_source_tuple_accepts_optional_code_formatting(self):
        for quoted_tokens in (False, True):
            with self.subTest(quoted_tokens=quoted_tokens):
                plan, sidecar = self.compound_plan()
                source = finding_source_tuple(json.loads(sidecar.read_text(encoding="utf-8")))
                formatted = " / ".join(f"`{part}`" for part in source.split(" / ")) if quoted_tokens else f"`{source}`"
                plan.write_text(plan.read_text(encoding="utf-8").replace(source, formatted), encoding="utf-8")
                checked = run_validator("plan", plan)
                self.assertEqual(0, checked.returncode, checked.stderr)

    def test_compound_source_can_share_each_batch_with_singleton_sources(self):
        ids = [f"P17-AUD-00{number}" for number in (1, 2, 3)]
        self.write_audit_set(
            requirements=[requirement_record(state="not-met", finding_ids=ids)],
            findings=[finding_record(finding_id=finding_id) for finding_id in ids],
        )
        self.meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(encoding="utf-8")
        )
        plan, sidecar = self.compound_plan()
        compound = json.loads(sidecar.read_text(encoding="utf-8"))
        binding = self.record()
        binding["source"]["id"] = ids[1]
        reopen = self.record(closure_batches=["B2"], closure_key="session.reopen")
        reopen["source"]["id"] = ids[2]
        write_jsonl(sidecar, [compound, binding, reopen])
        text = plan.read_text(encoding="utf-8")
        field = f"- **Finding source(s)**: {finding_source_tuple(compound)}"
        text = text.replace(field, field + "; " + finding_source_tuple(binding), 1)
        first, second = text.split("### Batch B2:", 1)
        second = second.replace(field, field + "; " + finding_source_tuple(reopen), 1)
        text = first + "### Batch B2:" + second
        text += f"\nSource findings: {ids[1]}, {ids[2]}\n"
        plan.write_text(text, encoding="utf-8")
        checked = run_validator("plan", plan)
        self.assertEqual(0, checked.returncode, checked.stderr)
        result, _ = self.result(plan, sidecar)
        checked = run_validator("result", result)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_closure_batches_are_required_unique_nonempty_strings(self):
        for batches in (None, "B1", [""], [" "], [1], [["B1"]], ["B1", "B1"]):
            with self.subTest(batches=batches):
                record = self.record(closure_batches=batches)
                if batches is None:
                    record.pop("closure_batches")
                plan, _ = self.write_plan(disposition=record)
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("closure_batches", checked.stderr)

    def test_legacy_scalar_and_unknown_disposition_fields_fail_closed(self):
        for field in ("closure_batch", "unexpected"):
            for include_array in (False, True):
                with self.subTest(field=field, include_array=include_array):
                    record = self.record()
                    record[field] = "B1"
                    if not include_array:
                        record.pop("closure_batches")
                    plan, _ = self.write_plan(disposition=record)
                    checked = run_validator("plan", plan)
                    self.assertNotEqual(0, checked.returncode)
                    self.assertIn("unexpected fields: " + field, checked.stderr)

    def test_fix_requires_batches_and_no_action_requires_empty_batches(self):
        plan, _ = self.write_plan(disposition=self.record(closure_batches=[]))
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("fix requires closure_batches", checked.stderr)
        record = self.no_action()
        record["closure_batches"] = ["B1"]
        plan, _ = self.plan(record, "DRAFT-UNRESOLVED")
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("no-action requires metadata, no batch and no changed paths", checked.stderr)

    def test_compound_batches_reject_unknown_or_unrepresented_headers(self):
        for batches in (["B1", "B9"], ["B1"]):
            with self.subTest(batches=batches):
                plan, sidecar = self.compound_plan()
                record = json.loads(sidecar.read_text(encoding="utf-8"))
                record["closure_batches"] = batches
                write_jsonl(sidecar, [record])
                checked = run_validator("plan", plan)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("batch headers must exactly cover", checked.stderr)

    def test_compound_batch_list_follows_declared_execution_order(self):
        plan, sidecar = self.compound_plan()
        record = json.loads(sidecar.read_text(encoding="utf-8"))
        record["closure_batches"].reverse()
        write_jsonl(sidecar, [record])
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("closure_batches must follow declared batch order", checked.stderr)

    def test_compound_plan_cannot_be_ready_with_remaining_obligations(self):
        plan, sidecar = self.compound_plan()
        record = json.loads(sidecar.read_text(encoding="utf-8"))
        record["remaining_obligations"] = ["Reopen applicability still needs source proof."]
        write_jsonl(sidecar, [record])
        checked = run_validator("plan", plan)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("remaining obligations", checked.stderr)

    def test_compound_result_preserves_entire_ordered_batch_list(self):
        for batches in (["B1"], ["B2", "B1"], ["B1", "B2", "B3"]):
            with self.subTest(batches=batches):
                plan, sidecar = self.compound_plan()
                result, result_sidecar = self.result(plan, sidecar)
                record = json.loads(result_sidecar.read_text(encoding="utf-8"))
                record["closure_batches"] = batches
                write_jsonl(result_sidecar, [record])
                checked = run_validator("result", result)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("drifts from plan-stage disposition", checked.stderr)

    def test_compound_result_requires_full_conjunction_pass_for_every_change_kind(self):
        for kind in ("behavioral", "test-only", "documentation", "metadata"):
            with self.subTest(kind=kind):
                plan, sidecar = self.compound_plan()
                record = json.loads(sidecar.read_text(encoding="utf-8"))
                record["change_kind"] = kind
                write_jsonl(sidecar, [record])
                result, result_sidecar = self.result(plan, sidecar)
                record = json.loads(result_sidecar.read_text(encoding="utf-8"))
                record["green_after"]["observed"] = "FAIL: B1 passed but B2 still loses binding; conjunction failed"
                write_jsonl(result_sidecar, [record])
                checked = run_validator("result", result)
                self.assertNotEqual(0, checked.returncode)
                self.assertIn("Closed requires observed PASS green-after", checked.stderr)

    def test_compound_evidence_requires_command_expected_and_observed(self):
        for evidence in ("red_before", "green_after"):
            for field in ("command", "expected", "observed"):
                with self.subTest(evidence=evidence, field=field):
                    plan, sidecar = self.compound_plan()
                    record = json.loads(sidecar.read_text(encoding="utf-8"))
                    record[evidence].pop(field)
                    write_jsonl(sidecar, [record])
                    checked = run_validator("plan", plan)
                    self.assertNotEqual(0, checked.returncode)
                    self.assertIn(evidence + " must contain command, expected, observed", checked.stderr)

    def no_action(self, status="Refuted", obligations=None):
        record = self.record(
            verification_status=status,
            decision="no-action:retain-current-behavior",
            closure_batches=[],
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
        if record is not None and record["closure_batches"] == []:
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
        second = self.record(closure_batches=["B2"], closure_key="session.other-binding")
        second["source"]["id"] = "P17-AUD-002"
        plan, sidecar = self.plan(first, dependencies=json.dumps(dependencies[0]))
        write_jsonl(sidecar, [first, second])
        text = plan.read_text(encoding="utf-8")
        text += f"\nSource finding: {run_id}/P17-AUD-002\n### Batch B2: Second\n\n**Dependencies**: {json.dumps(dependencies[1])}\n\n#### Fix B2.1: Second binding\n- **Finding source(s)**: {finding_source_tuple(second)}\n- **Closure predicate**: second binding retained\n- **Red-before**: FAIL\n- **Green-after**: PASS\n"
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

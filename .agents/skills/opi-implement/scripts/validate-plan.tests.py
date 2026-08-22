from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate-plan.py")


def valid_task(*, trigger: str = "none") -> dict[str, object]:
    reuse_reason = "searched=workspace; reused=none; gap=validator"
    ceiling_reason = (
        "ceiling=validate admitted evidence; "
        "revisit_when=ledger schema changes; "
        f"simplification_trigger={trigger}"
    )
    if trigger != "none":
        reuse_reason += (
            "; production_consumers=opi-implement plan"
            "; nonproduction_consumers=tests/docs"
        )
        ceiling_reason += "; net_deletion=one validator; residual_glue=none"

    source = "docs/opi-spec.md#Example"
    return {
        "id": "18.1",
        "inference_notes": [
            {
                "field": "reuse_search",
                "reason": reuse_reason,
                "source": source,
            },
            {
                "field": "placement",
                "reason": (
                    "target=assurance; existing_home=opi-implement; "
                    "cannot_fit_fully=not-applicable"
                ),
                "source": source,
            },
            {
                "field": "surface_necessity",
                "reason": (
                    "public_api=none; config=none; state=none; "
                    "dependency_edge=none"
                ),
                "source": source,
            },
            {
                "field": "simplification_ceiling",
                "reason": ceiling_reason,
                "source": source,
            },
        ],
    }


def shared_task(
    task_id: str,
    *,
    role: str,
    owner_task: str = "18.1",
    consumer_tasks: str = "18.2",
    trigger: str = "multiple-consumers",
    depends_on: list[str] | None = None,
    call_sites: list[str] | None = None,
) -> dict[str, object]:
    task = valid_task()
    task.update(
        {
            "id": task_id,
            "depends_on": depends_on if depends_on is not None else [],
            "evaluator_required": True,
            "production_call_sites": (
                call_sites if call_sites is not None else [f"production::{task_id}"]
            ),
            "acceptance_scenarios": [
                {"id": "P18-A01", "verification": ["shared_contract_test"]}
            ],
            "verification": {
                "behavioral_tests": ["tests/shared_contract.rs"],
                "library_gates": ["shared_contract_test"],
            },
        }
    )
    notes = task["inference_notes"]
    assert isinstance(notes, list)
    notes.append(
        {
            "field": "shared_decision",
            "reason": (
                "decision_id=route-owner; "
                f"role={role}; owner_task={owner_task}; "
                "module=opi_ai::ProviderCollection; interface=resolve_model; "
                "representation=ResolvedProviderRoute; "
                f"consumer_tasks={consumer_tasks}; criterion_ids=P18-A01; "
                "legacy_paths=none; closure_test=tests/shared_contract.rs; "
                f"trigger={trigger}"
            ),
            "source": "docs/opi-spec.md#Example",
        }
    )
    return task


def complete_shared_graph() -> list[dict[str, object]]:
    return [
        shared_task("18.1", role="owner"),
        shared_task("18.2", role="consumer", depends_on=["18.1"]),
    ]


class ValidatePlanTests(unittest.TestCase):
    def run_validator(self, ledger: dict[str, object]) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "draft.json"
            path.write_text(json.dumps(ledger), encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(VALIDATOR), str(path)],
                capture_output=True,
                check=False,
                text=True,
            )

    def test_rejects_task_without_simplification_ceiling_note(self) -> None:
        result = self.run_validator(
            {
                "schema_version": 2,
                "tasks": [{"id": "18.1", "inference_notes": []}],
            }
        )

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] requires exactly one simplification_ceiling note",
            result.stderr,
        )

    def test_rejects_task_without_reuse_search_note(self) -> None:
        task = valid_task()
        task["inference_notes"] = [
            note
            for note in task["inference_notes"]
            if note["field"] != "reuse_search"
        ]
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] requires exactly one reuse_search note",
            result.stderr,
        )

    def test_accepts_complete_ordinary_task(self) -> None:
        result = self.run_validator(
            {"schema_version": 2, "tasks": [valid_task()]}
        )

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("plan validation ok tasks=1", result.stdout)

    def test_ignores_archived_legacy_task(self) -> None:
        result = self.run_validator(
            {
                "schema_version": 2,
                "tasks": [
                    {"id": "old", "status": "archived"},
                    valid_task(),
                ],
            }
        )

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("plan validation ok tasks=2", result.stdout)

    def test_rejects_live_v1_plan(self) -> None:
        result = self.run_validator(
            {"schema_version": 1, "tasks": [valid_task()]}
        )

        self.assertEqual(1, result.returncode)
        self.assertIn("plan ledger schema_version must be 2", result.stderr)

    def test_rejects_note_without_source(self) -> None:
        task = valid_task()
        task["inference_notes"][0]["source"] = ""
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] reuse_search requires non-empty source",
            result.stderr,
        )

    def test_rejects_missing_base_reason_clause(self) -> None:
        task = valid_task()
        task["inference_notes"][0]["reason"] = "searched=workspace; reused=none"
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn("task[0] reuse_search requires gap=", result.stderr)

    def test_rejects_unknown_placement_target(self) -> None:
        task = valid_task()
        task["inference_notes"][1]["reason"] = (
            "target=somewhere; existing_home=none; cannot_fit_fully=reason"
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] placement target 'somewhere' is not recognized",
            result.stderr,
        )

    def test_rejects_unknown_simplification_trigger(self) -> None:
        task = valid_task()
        task["inference_notes"][3]["reason"] = (
            "ceiling=known; revisit_when=observable; "
            "simplification_trigger=unknown"
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] simplification_trigger 'unknown' is not recognized",
            result.stderr,
        )

    def test_rejects_duplicate_simplification_trigger_clause(self) -> None:
        task = valid_task()
        task["inference_notes"][3]["reason"] = (
            "ceiling=known; revisit_when=observable; "
            "simplification_trigger=none; simplification_trigger=delete"
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] simplification_ceiling requires exactly one "
            "simplification_trigger=",
            result.stderr,
        )

    def test_rejects_duplicate_clause_with_empty_second_value(self) -> None:
        task = valid_task()
        task["inference_notes"][3]["reason"] = (
            "ceiling=known; revisit_when=observable; "
            "simplification_trigger=none; simplification_trigger="
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] simplification_ceiling requires exactly one "
            "simplification_trigger=",
            result.stderr,
        )

    def test_triggered_simplification_requires_consumer_evidence(self) -> None:
        task = valid_task(trigger="delete")
        task["inference_notes"][0]["reason"] = (
            "searched=workspace; reused=none; gap=validator; "
            "nonproduction_consumers=tests/docs"
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] simplification_trigger=delete requires production_consumers=",
            result.stderr,
        )

    def test_triggered_simplification_requires_deletion_evidence(self) -> None:
        task = valid_task(trigger="delete")
        task["inference_notes"][3]["reason"] = (
            "ceiling=known; revisit_when=observable; "
            "simplification_trigger=delete; residual_glue=none"
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task[0] simplification_trigger=delete requires net_deletion=",
            result.stderr,
        )

    def test_accepts_complete_triggered_simplification(self) -> None:
        result = self.run_validator(
            {"schema_version": 2, "tasks": [valid_task(trigger="delete")]}
        )

        self.assertEqual(0, result.returncode, result.stderr)

    def test_accepts_complete_shared_decision_graph(self) -> None:
        result = self.run_validator(
            {"schema_version": 2, "tasks": complete_shared_graph()}
        )

        self.assertEqual(0, result.returncode, result.stderr)

    def test_rejects_multiple_shared_decision_owners(self) -> None:
        tasks = complete_shared_graph()
        note = tasks[1]["inference_notes"][-1]
        note["reason"] = note["reason"].replace("role=consumer", "role=owner")
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner requires exactly one owner", result.stderr
        )

    def test_rejects_consumer_without_owner_dependency(self) -> None:
        tasks = complete_shared_graph()
        tasks[1]["depends_on"] = []
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task 18.2 must depend transitively on owner 18.1", result.stderr
        )

    def test_rejects_shared_task_without_evaluator(self) -> None:
        tasks = complete_shared_graph()
        tasks[1]["evaluator_required"] = False
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "task 18.2 shared decision requires evaluator_required=true",
            result.stderr,
        )

    def test_rejects_expand_contract_without_legacy_path(self) -> None:
        tasks = complete_shared_graph()
        for task in tasks:
            note = task["inference_notes"][-1]
            note["reason"] = note["reason"].replace(
                "trigger=multiple-consumers", "trigger=expand-contract"
            )
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner expand-contract requires "
            "legacy_paths other than none",
            result.stderr,
        )

    def test_accepts_intrinsic_owner_without_consumer(self) -> None:
        task = shared_task(
            "18.1",
            role="owner",
            consumer_tasks="none",
            trigger="intrinsic-state",
        )
        result = self.run_validator({"schema_version": 2, "tasks": [task]})

        self.assertEqual(0, result.returncode, result.stderr)

    def test_rejects_disagreeing_shared_interface(self) -> None:
        tasks = complete_shared_graph()
        note = tasks[1]["inference_notes"][-1]
        note["reason"] = note["reason"].replace(
            "interface=resolve_model", "interface=normalize_locally"
        )
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn("decision route-owner disagrees on interface", result.stderr)

    def test_rejects_missing_declared_criterion(self) -> None:
        tasks = complete_shared_graph()
        for task in tasks:
            note = task["inference_notes"][-1]
            note["reason"] = note["reason"].replace(
                "criterion_ids=P18-A01", "criterion_ids=P18-MISSING"
            )
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner criterion P18-MISSING has no scenario",
            result.stderr,
        )

    def test_rejects_missing_owner_closure_test(self) -> None:
        tasks = complete_shared_graph()
        for task in tasks:
            note = task["inference_notes"][-1]
            note["reason"] = note["reason"].replace(
                "closure_test=tests/shared_contract.rs",
                "closure_test=tests/missing.rs",
            )
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner owner 18.1 lacks closure_test tests/missing.rs",
            result.stderr,
        )

    def test_rejects_missing_consumer_note(self) -> None:
        result = self.run_validator(
            {"schema_version": 2, "tasks": [complete_shared_graph()[0]]}
        )

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner consumer notes do not match", result.stderr
        )

    def test_rejects_multiple_consumers_without_two_production_participants(
        self,
    ) -> None:
        tasks = complete_shared_graph()
        tasks[1]["production_call_sites"] = []
        result = self.run_validator({"schema_version": 2, "tasks": tasks})

        self.assertEqual(1, result.returncode)
        self.assertIn(
            "decision route-owner requires two production participants",
            result.stderr,
        )


if __name__ == "__main__":
    unittest.main()

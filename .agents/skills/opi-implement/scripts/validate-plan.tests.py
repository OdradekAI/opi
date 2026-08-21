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


if __name__ == "__main__":
    unittest.main()

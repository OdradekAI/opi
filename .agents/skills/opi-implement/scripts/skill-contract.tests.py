from __future__ import annotations

import unittest
from pathlib import Path


SKILL_ROOT = Path(__file__).resolve().parents[1]
SHARED = SKILL_ROOT.parent / "_shared" / "references" / (
    "shared-decision-and-test-stewardship.md"
)


class SkillContractTests(unittest.TestCase):
    def test_both_skills_use_the_shared_test_contract(self) -> None:
        implement = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")
        slim = (SKILL_ROOT.parent / "opi-slim-tests" / "SKILL.md").read_text(
            encoding="utf-8"
        )
        pointer = "shared-decision-and-test-stewardship.md"
        self.assertIn(pointer, implement)
        self.assertIn(pointer, slim)

    def test_shared_contract_defines_plan_and_runtime_outputs(self) -> None:
        contract = SHARED.read_text(encoding="utf-8")
        for token in (
            "field=shared_decision",
            "decision_id=",
            "role=owner|consumer",
            "criterion_ids=",
            "test_disposition",
            "replace-don't-layer",
            "slim_candidate",
        ):
            self.assertIn(token, contract)

    def test_user_invoked_description_is_human_facing(self) -> None:
        skill = (SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")
        description = next(
            line for line in skill.splitlines() if line.startswith("description:")
        )
        self.assertNotIn("Triggers on", description)
        self.assertNotIn("Use when", description)


if __name__ == "__main__":
    unittest.main()

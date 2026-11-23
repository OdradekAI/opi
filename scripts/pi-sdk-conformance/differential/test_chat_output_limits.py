from __future__ import annotations

import copy
import json
import unittest

from common import FIXTURE_DIR, REPO_ROOT
from replay_chat_output_limits import FIXTURE, compare


class ChatOutputLimitComparison(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads((REPO_ROOT / FIXTURE).read_bytes())
        path = FIXTURE_DIR / "phase5-chat-output-limits.json"
        cls.record = json.loads(path.read_bytes())

    def setUp(self) -> None:
        self.pi, self.opi = (copy.deepcopy(self.record[side]["first"]["trace"]) for side in ("pi", "opi"))

    def row(self, trace: list, case: str, entry: str) -> dict:
        return next(row for row in trace if row["case"] == case and row["entry"] == entry)

    def test_frozen_entries_match(self) -> None:
        result = compare(self.fixture, self.pi, self.opi)
        self.assertEqual(result, self.record["comparison"])
        self.assertEqual(result["matched_pairs"], 42)

    def test_cross_entry_comparison_fails(self) -> None:
        self.opi[0] = self.opi[1]
        with self.assertRaisesRegex(ValueError, "matched entries differ"):
            compare(self.fixture, self.pi, self.opi)

    def test_both_sides_wrong_cap_is_rejected(self) -> None:
        for trace in (self.pi, self.opi):
            row = self.row(trace, "explicit_near_context", "native")
            row["request"]["body"][row["field"]] = 3618
        with self.assertRaisesRegex(ValueError, "wire request differs"):
            compare(self.fixture, self.pi, self.opi)

    def test_direct_call_must_bypass_the_resolver(self) -> None:
        for trace in (self.pi, self.opi):
            self.row(trace, "omitted", "direct")["resolver_calls"] = 1
        with self.assertRaisesRegex(ValueError, "resolver calls differ"):
            compare(self.fixture, self.pi, self.opi)

    def test_positive_floor_keeps_length_result(self) -> None:
        for trace in (self.pi, self.opi):
            self.row(trace, "context_floor", "simple")["result"]["raw_stop"] = "stop"
        with self.assertRaisesRegex(ValueError, "terminal result differs"):
            compare(self.fixture, self.pi, self.opi)


if __name__ == "__main__":
    unittest.main()

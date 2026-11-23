from __future__ import annotations

import copy
import json
import unittest

from common import FIXTURE_DIR
from replay_partial_aliases import compare


class PartialAliasesComparison(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        path = FIXTURE_DIR / "phase5-partial-aliases.json"
        cls.record = json.loads(path.read_bytes())

    def setUp(self) -> None:
        self.pi = copy.deepcopy(self.record["pi_runs"][0]["trace"])
        self.rust = copy.deepcopy(self.record["opi_runs"][0]["trace"])

    def test_frozen_observations_keep_differences(self) -> None:
        self.assertEqual(compare(self.pi, self.rust), self.record["comparison"])
        self.assertFalse(self.record["comparison"]["strict_alignment"])

    def test_loss_of_faux_block_alias_fails(self) -> None:
        self.pi["faux"]["baseline"]["after_terminal"]["latest"]["content"][0]["ref"] = -1
        with self.assertRaisesRegex(ValueError, "shares a started block"):
            compare(self.pi, self.rust)

    def test_loss_of_arguments_replacement_fails(self) -> None:
        row = self.pi["openai"]["baseline"]["after_terminal"]
        row["tool_delta"]["content"][1]["args"]["ref"] = row["saved_args"]["ref"]
        with self.assertRaisesRegex(ValueError, "replacement arguments"):
            compare(self.pi, self.rust)

    def test_lost_or_reordered_events_fail(self) -> None:
        events = self.rust["openai"]["baseline"]["events"]
        events[2], events[3] = events[3], events[2]
        with self.assertRaisesRegex(ValueError, "events differ"):
            compare(self.pi, self.rust)

    def test_lost_preview_difference_fails(self) -> None:
        event = next(e for e in self.rust["faux"]["baseline"]["events"] if e["kind"] == "tool_call_delta")
        event["message"]["content"][1]["args"]["value"] = {}
        with self.assertRaisesRegex(ValueError, "display projection"):
            compare(self.pi, self.rust)

    def test_lost_metadata_timing_difference_fails(self) -> None:
        self.pi["openai"]["baseline"]["after_usage"]["latest"]["input_tokens"] = 0
        with self.assertRaisesRegex(ValueError, "metadata-only read"):
            compare(self.pi, self.rust)


if __name__ == "__main__":
    unittest.main()

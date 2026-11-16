from __future__ import annotations

import copy
import json
import unittest

from replay_chat_interleaving import ITERATION, REPO_ROOT, compare


class ChatInterleavingComparison(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        path = REPO_ROOT / ITERATION / "evidence/phase5-chat-interleaving.json"
        cls.record = json.loads(path.read_bytes())

    def setUp(self) -> None:
        self.pi, self.before, self.after = (
            copy.deepcopy(self.record[side][0]["trace"])
            for side in ("pi_runs", "opi_before_runs", "opi_after_runs")
        )

    def test_frozen_pair_retains_finish_timing_and_preview_differences(self) -> None:
        result = compare(self.pi, self.before, self.after)
        self.assertEqual(result, self.record["comparison"])
        self.assertFalse(result["strict_alignment"])

    def test_regression_to_a_split_text_block_fails(self) -> None:
        self.after["resumed_text"] = self.before["resumed_text"]
        with self.assertRaisesRegex(ValueError, "text delta indexes"):
            compare(self.pi, self.before, self.after)

    def test_whitespace_loss_in_continuation_fails(self) -> None:
        self.after["resumed_whitespace"]["chat_requests"][1]["messages"][1]["content"] = "ABCD"
        with self.assertRaisesRegex(ValueError, "Chat request bodies differ"):
            compare(self.pi, self.before, self.after)

    def test_ignoring_held_finish_timing_fails(self) -> None:
        self.after["resumed_text"]["checkpoints"][6] = self.pi["chat"]["resumed_text"]["checkpoints"][6]
        with self.assertRaisesRegex(ValueError, "fixed Opi closes at real finish"):
            compare(self.pi, self.before, self.after)

    def test_preview_difference_must_remain_visible(self) -> None:
        trace = self.after["resumed_text"]
        event = next(event for event in trace["events"] if event["kind"] == "toolcall_delta")
        event["message"]["content"][1]["arguments"] = {}
        with self.assertRaisesRegex(ValueError, "incomplete argument previews"):
            compare(self.pi, self.before, self.after)

    def test_durable_replays_the_final_blocks_in_order(self) -> None:
        self.pi["durable"]["opi_after"]["replayed"].reverse()
        with self.assertRaisesRegex(ValueError, "Durable stored/replayed blocks"):
            compare(self.pi, self.before, self.after)


if __name__ == "__main__":
    unittest.main()

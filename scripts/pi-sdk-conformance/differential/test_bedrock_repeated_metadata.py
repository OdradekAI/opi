import json
import unittest
from common import FIXTURE_DIR


class BedrockRepeatedMetadataDifferentialTests(unittest.TestCase):
    receipt = FIXTURE_DIR / "phase7-bedrock-pending-differential-v3.json"

    def test_three_stable_paired_runs(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(len(evidence["pi_runs"]), 3)
        self.assertEqual(len(evidence["rust_runs"]), 3)
        self.assertTrue(all(run["trace"] == evidence["pi_runs"][0]["trace"] for run in evidence["pi_runs"]))
        self.assertTrue(all(run["trace"] == evidence["rust_runs"][0]["trace"] for run in evidence["rust_runs"]))

    def test_repeated_metadata_is_final_and_single_terminal(self):
        trace = json.loads(self.receipt.read_text(encoding="utf-8"))["pi_runs"][0]["trace"]
        self.assertEqual(trace["usage"], {"input": 12, "output": 7, "cache_read": 3})
        self.assertEqual(trace["stop"], "stop")
        self.assertEqual(trace["raw_stop"], "end_turn")
        self.assertEqual(trace["done_count"], 1)
        self.assertEqual(trace["events"], ["start", "text_start", "text_delta", "text_end", "done"])


if __name__ == "__main__":
    unittest.main()

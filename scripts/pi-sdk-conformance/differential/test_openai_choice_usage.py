import json
import unittest
from pathlib import Path


class OpenAiChoiceUsageDifferentialTests(unittest.TestCase):
    receipt = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-openai-choice-differential.json")

    def test_three_stable_paired_runs(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(len(evidence["pi_runs"]), 3)
        self.assertEqual(len(evidence["rust_runs"]), 3)
        self.assertTrue(all(run["trace"] == evidence["pi_runs"][0]["trace"] for run in evidence["pi_runs"]))
        self.assertTrue(all(run["trace"] == evidence["rust_runs"][0]["trace"] for run in evidence["rust_runs"]))
        self.assertEqual(evidence["pi_runs"][0]["trace"], evidence["rust_runs"][0]["trace"])

    def test_choice_usage_projection(self):
        trace = json.loads(self.receipt.read_text(encoding="utf-8"))["pi_runs"][0]["trace"]
        self.assertEqual(trace["usage"], {"input": 25, "output": 20, "cache_read": 75, "cache_write": 0, "total": 120})
        self.assertEqual(trace["stop"], "stop")
        self.assertEqual(trace["requests"], 1)


if __name__ == "__main__":
    unittest.main()

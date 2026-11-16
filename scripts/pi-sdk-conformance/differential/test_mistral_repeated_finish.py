import json
import unittest
from pathlib import Path


class MistralRepeatedFinishDifferentialTests(unittest.TestCase):
    evidence = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-mistral-repeated-finish-differential.json")

    def test_frozen_pi_and_rust_traces_pass(self):
        value = json.loads(self.evidence.read_text(encoding="utf-8"))
        self.assertEqual(value["status"], "PASS")
        self.assertEqual(value["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(len(value["pi_runs"]), 3)
        self.assertEqual(len(value["rust_runs"]), 3)


if __name__ == "__main__":
    unittest.main()

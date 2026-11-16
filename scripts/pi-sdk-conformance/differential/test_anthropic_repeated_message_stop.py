import json
import unittest
from pathlib import Path


class AnthropicRepeatedMessageStopDifferentialTests(unittest.TestCase):
    receipt = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-anthropic-repeated-message-stop-differential.json")

    def test_frozen_pi_and_rust_traces_match(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(len(evidence["pi_runs"]), 3)
        self.assertEqual(len(evidence["rust_runs"]), 3)


if __name__ == "__main__":
    unittest.main()

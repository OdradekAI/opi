import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
RECEIPT = ROOT / "docs/iterations/opi-ai-1-5-alignment/evidence/phase7-openai-chat-response-observer-differential.json"


class OpenAiChatResponseObserverReceiptTests(unittest.TestCase):
    def test_canonical_receipt_is_stable_and_matches_contract(self):
        receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
        self.assertEqual(receipt["status"], "pass")
        self.assertEqual(receipt["pi_revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(receipt["repetitions"], 3)
        self.assertTrue(receipt["normalized_match"])
        self.assertEqual(len(receipt["pi_runs"]), 3)
        self.assertEqual(len(receipt["rust_runs"]), 3)
        for row in receipt["pi_runs"] + receipt["rust_runs"]:
            self.assertEqual(row["observer_statuses"], [200, 200])
            self.assertEqual(row["first"], {"stop": "error", "event_types": ["error"]})
            self.assertEqual(row["second"]["stop"], "stop")
            self.assertEqual(row["second"]["usage"], {"input": 2, "output": 1, "total": 3})
            self.assertEqual(row["requests"], 2)


if __name__ == "__main__":
    unittest.main()

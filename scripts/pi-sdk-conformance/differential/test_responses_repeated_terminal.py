import json
import unittest
from pathlib import Path


class ResponsesRepeatedTerminalDifferentialTests(unittest.TestCase):
    receipt = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-responses-repeated-terminal-differential.json")

    def test_frozen_pi_and_rust_traces_match(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual(len(evidence["pi_runs"]), 3)
        self.assertEqual(len(evidence["rust_runs"]), 3)
        self.assertEqual(evidence["differences"], [
            "Pi exposes provider_events from every source item; Rust's direct fixture helper exposes normalized event_types only.",
        ])


if __name__ == "__main__":
    unittest.main()

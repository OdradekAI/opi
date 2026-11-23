import json
import unittest
from common import FIXTURE_DIR


class PiMessagesTerminalDifferentialTests(unittest.TestCase):
    receipt = FIXTURE_DIR / "phase7-pi-messages-terminal-differential.json"

    def test_frozen_pair_passes(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["revision"], "f9a26365568127461c47a07a37dcb42c140a3ac5")
        self.assertEqual({row["route"] for row in evidence["pi_runs"][0]["trace"]["cases"]}, {"simple", "direct"})
        pi = evidence["pi_runs"][0]["trace"]
        rust = evidence["rust_runs"][0]["trace"]
        for pi_row, rust_row in zip(pi["cases"], rust["cases"], strict=True):
            for key in ("route", "stop", "response_model", "usage", "done_count", "event_types", "requests"):
                self.assertEqual(pi_row[key], rust_row[key], key)


if __name__ == "__main__":
    unittest.main()

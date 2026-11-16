import json
import re
import unittest
from pathlib import Path


class AnthropicRepeatedMessageStopPackageTests(unittest.TestCase):
    receipt = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-anthropic-repeated-message-stop-package.json")
    root = Path("C:/Temp/opi-anthropic-repeated-message-stop-package-20261115-v1")

    def test_actual_archive_profiles_pass(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["verdict"], "PASS")
        self.assertEqual({profile["name"] for profile in evidence["profiles"]}, {"builtins-no-tracing", "defaults"})
        self.assertEqual(set(evidence["executed_sources"]), {"tests/anthropic_repeated_message_stop.rs", "tests/data/anthropic-repeated-message-stop.json"})
        for profile in ("builtins-no-tracing", "defaults"):
            output = (self.root / f"{profile}-tests.stdout").read_text(encoding="utf-8")
            self.assertRegex(output, re.compile(r"test result: ok\. [1-9][0-9]* passed; 0 failed;"))


if __name__ == "__main__":
    unittest.main()

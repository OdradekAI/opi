import json
import re
import unittest
from pathlib import Path


class OpenAiChoiceUsagePackageTests(unittest.TestCase):
    receipt = Path(__file__).with_name("evidence") / "phase7-openai-choice-package.json"
    root = receipt.with_suffix("")

    def test_actual_archive_profiles_pass(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["verdict"], "PASS")
        self.assertEqual({profile["name"] for profile in evidence["profiles"]}, {"builtins-no-tracing", "defaults"})
        self.assertEqual(set(evidence["executed_sources"]), {"tests/openai_choice_usage.rs", "tests/data/openai-choice-usage.json"})
        for profile in ("builtins-no-tracing", "defaults"):
            output = (self.root / f"{profile}-tests.stdout").read_text(encoding="utf-8")
            self.assertRegex(output, r"test result: ok\. 3 passed; 0 failed;")


if __name__ == "__main__":
    unittest.main()

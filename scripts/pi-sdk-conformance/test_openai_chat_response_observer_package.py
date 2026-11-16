import json
import re
import unittest
from pathlib import Path


RECEIPT = Path("docs/iterations/opi-ai-1-5-alignment/evidence/phase7-openai-chat-response-observer-package.json")
ROOT = Path("C:/Temp/opi-openai-chat-response-observer-package-20261115-v2")


class OpenAiChatResponseObserverPackageTests(unittest.TestCase):
    def test_canonical_archive_consumer_passes_both_profiles(self):
        receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
        self.assertEqual(receipt["verdict"], "PASS")
        self.assertEqual({row["name"] for row in receipt["profiles"]}, {"builtins-no-tracing", "defaults"})
        self.assertEqual(len(receipt["executed_sources"]), 2)
        for profile in ("builtins-no-tracing", "defaults"):
            output = (ROOT / f"{profile}-tests.stdout").read_text(encoding="utf-8")
            self.assertRegex(output, re.compile(r"test result: ok\. 1 passed; 0 failed;"))


if __name__ == "__main__":
    unittest.main()

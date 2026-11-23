import json
import unittest
from pathlib import Path


class NestedAnthropicPackageTests(unittest.TestCase):
    receipt = Path(__file__).with_name("evidence") / "phase6-anthropic-nested-package.json"
    root = receipt.with_suffix("")

    def test_actual_archive_profiles_and_tests_pass(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        self.assertEqual(evidence["verdict"], "PASS")
        self.assertEqual(len(evidence["profiles"]), 2)
        self.assertEqual(set(evidence["executed_sources"]), {
            "tests/anthropic_usage_updates.rs",
            "tests/anthropic_nested_usage.rs",
            "tests/data/anthropic-usage-updates.json",
        })
        for profile in ("builtins-no-tracing", "defaults"):
            updates = (self.root / f"{profile}-updates-tests.stdout").read_text(encoding="utf-8")
            nested = (self.root / f"{profile}-nested-tests.stdout").read_text(encoding="utf-8")
            self.assertRegex(updates, r"test result: ok\. 5 passed; 0 failed;")
            self.assertRegex(nested, r"test result: ok\. 1 passed; 0 failed;")

    def test_companion_tracing_features_are_profile_bound(self):
        evidence = json.loads(self.receipt.read_text(encoding="utf-8"))
        profiles = {profile["name"]: profile for profile in evidence["profiles"]}
        self.assertNotIn("tracing", profiles["builtins-no-tracing"]["telemetry_features"])
        self.assertIn("tracing", profiles["defaults"]["telemetry_features"])


if __name__ == "__main__":
    unittest.main()

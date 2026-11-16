import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from compare_anthropic_nested_usage import compare


class NestedAnthropicUsageTests(unittest.TestCase):
    def test_frozen_nested_trace_matches(self):
        root = Path("C:/Temp/opi-anthropic-nested-20261114")
        result = compare(root / "pi.stdout", root / "rust.stdout")
        self.assertEqual(result["status"], "PASS_WITH_LOW_LEVEL_COST_SCOPE")

    def test_cost_scope_is_explicit(self):
        root = Path("C:/Temp/opi-anthropic-nested-20261114")
        result = compare(root / "pi.stdout", root / "rust.stdout")
        self.assertEqual(result["pi_cost_total"], 0.00079)
        self.assertEqual(result["rust_low_level_cost_total"], 0.0)
        self.assertEqual(len(result["differences"]), 2)


if __name__ == "__main__":
    unittest.main()

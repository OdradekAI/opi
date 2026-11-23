import json
from pathlib import Path
import subprocess
import sys
import unittest

import check_structure_gate as gate


class StructureGateTests(unittest.TestCase):
    def test_summary_hash_is_canonical(self):
        self.assertEqual(gate.summary_hash({"b": 1, "a": 2}),
                         gate.summary_hash({"a": 2, "b": 1}))
        self.assertNotEqual(gate.summary_hash({"a": 2}), gate.summary_hash({"a": 3}))

    def test_changed_selection_reports_unmapped_path(self):
        rows = gate.test_inventory.load_inventory()
        selection = gate.changed_selection(rows, ["crates/opi-ai/tests/missing.rs"])
        self.assertEqual(selection["scope"], "broader checks required")
        self.assertTrue(selection["expansion"])

    def test_cli_negative_path_returns_nonzero(self):
        result = subprocess.run(
            [sys.executable, str(Path(gate.__file__).resolve()), "--strict-selection",
             "crates/opi-ai/tests/missing.rs"],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertTrue(report["violations"])
        self.assertTrue(any("broader checks" in item for item in report["violations"]))


if __name__ == "__main__":
    unittest.main()

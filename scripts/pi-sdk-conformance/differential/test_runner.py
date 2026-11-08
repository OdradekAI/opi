import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from compare_lifecycle import SCENARIOS
from run_lifecycle_differential import verdict


class LifecycleRunnerTests(unittest.TestCase):
    def cases(self, status="difference"):
        return [{"scenario": name, "comparison": {"status": status}} for name in SCENARIOS]

    def test_failures_cannot_be_hidden_by_blocked_materials(self):
        cases = self.cases()
        cases[0]["comparison"]["status"] = "fail"
        self.assertEqual(verdict(cases, ["lock drift"]), ("FAIL", 1))

    def test_blockers_and_unapproved_differences_are_not_pass(self):
        self.assertEqual(verdict(self.cases(), ["lock drift"]), ("BLOCKED_WITH_RECORDED_OBSERVATIONS", 2))
        self.assertEqual(verdict(self.cases(), []), ("DIFFERENCES_REQUIRE_DECISION", 0))

    def test_missing_and_duplicate_cases_fail(self):
        self.assertEqual(verdict([], []), ("INCOMPLETE_CASES", 1))
        cases = self.cases()
        cases[1]["scenario"] = cases[0]["scenario"]
        self.assertEqual(verdict(cases, []), ("INCOMPLETE_CASES", 1))

    def test_cli_refuses_to_overwrite_evidence_before_running_drivers(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "existing.json"
            output.write_bytes(b"original")
            result = subprocess.run([
                sys.executable, "-B", str(Path(__file__).with_name("run_lifecycle_differential.py")),
                "--repo", directory, "--materials", str(Path(directory) / "missing.json"),
                "--output", str(output),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn("refusing to overwrite", result.stderr)
            self.assertEqual(output.read_bytes(), b"original")


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from common import build_rust_driver, run_rust_trace
from compare_lifecycle import compare_traces
from test_lifecycle_differential import upstream_trace


class RustLifecycleDriverTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build = build_rust_driver()

    def test_three_scenarios_observe_barriers_and_public_results(self):
        for scenario in ("lazy-no-consumption", "lazy-stop-consumption", "lazy-setup-failure"):
            with self.subTest(scenario=scenario):
                trace = run_rust_trace(scenario, self.build)["trace"]
                comparison = compare_traces(upstream_trace(scenario), trace, scenario)
                self.assertEqual(comparison["status"], "difference", comparison)
                self.assertEqual(comparison["shared_observations"], "pass")
                if scenario != "lazy-setup-failure":
                    self.assertTrue(trace["shared"]["source_completed"])
                    self.assertEqual(trace["shared"]["terminal_before_release"], 0)
                    self.assertTrue(trace["runtimeDetails"]["source_dropped"])
                    self.assertTrue(trace["runtimeDetails"]["result_observed"])

    def test_unknown_scenario_is_a_nonzero_subprocess_failure(self):
        with self.assertRaises(subprocess.CalledProcessError) as raised:
            run_rust_trace("unknown", self.build)
        self.assertEqual(raised.exception.returncode, 2)
        self.assertIn(b"unknown lifecycle scenario", raised.exception.stderr)


if __name__ == "__main__":
    unittest.main()

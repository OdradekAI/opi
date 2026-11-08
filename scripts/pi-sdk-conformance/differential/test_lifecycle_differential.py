from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from common import REPO_ROOT
from compare_lifecycle import compare_traces


HISTORICAL = json.loads((REPO_ROOT / "docs/iterations/opi-ai-1-5-alignment/evidence/phase5-lifecycle-differential.json").read_text(encoding="utf-8"))


def upstream_trace(scenario: str) -> dict:
    return copy.deepcopy(next(case["upstream_trace"] for case in HISTORICAL["cases"] if case["scenario"] == scenario))


def rust_trace(scenario: str) -> dict:
    if scenario == "lazy-setup-failure":
        return {
            "schemaVersion": 2, "runtime": "rust", "scenario": scenario,
            "shared": {"terminal_kind": "error", "terminal_count": 1, "result_matches_event": True},
            "runtimeDetails": {"cause": "ModelNotFound", "failure_input": "unregistered provider record"},
        }
    before = {"setup_calls": 1, "source_started": True, "terminal_count": 0, "source_eof": False, "source_dropped": False}
    after = {**before, "terminal_count": 1, "source_eof": True, "source_dropped": True}
    steps = [
        {"name": "constructed", "observed": {**before, "setup_calls": 0, "source_started": False}},
        {"name": "source-at-gate", "observed": before.copy()},
    ]
    if scenario == "lazy-stop-consumption":
        steps.append({"name": "consumer-stopped-after-start", "observed": before.copy()})
    steps += [
        {"name": "before-source-release", "observed": before.copy()},
        {"name": "source-released", "observed": after.copy()},
        {"name": "result-observed", "stop": "stop"},
    ]
    return {
        "schemaVersion": 2, "runtime": "rust", "scenario": scenario, "steps": steps,
        "shared": {
            "setup_calls": 1, "source_started": True, "terminal_before_release": 0,
            "source_terminal_count": 1, "source_completed": True,
            "consumer_polled": scenario == "lazy-stop-consumption", "terminal_kind": "stop",
        },
        "runtimeDetails": {"consumer_handle_dropped": False, "source_dropped": True, "result_observed": True},
    }


class LifecycleComparatorTests(unittest.TestCase):
    def test_shared_success_is_not_strict_alignment_or_approved_difference(self):
        for scenario in ("lazy-no-consumption", "lazy-stop-consumption", "lazy-setup-failure"):
            with self.subTest(scenario=scenario):
                result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
                self.assertEqual(result["status"], "difference")
                self.assertEqual(result["shared_observations"], "pass")
                self.assertFalse(result["strict_alignment"])
                self.assertEqual(result["decision_status"], "not-approved-by-this-probe")

    def test_unexpected_terminal_mismatch_fails(self):
        scenario = "lazy-stop-consumption"
        rust = rust_trace(scenario)
        rust["shared"]["terminal_kind"] = "error"
        rust["steps"][-1]["stop"] = "error"
        result = compare_traces(upstream_trace(scenario), rust, scenario)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["mismatches"][0]["field"], "terminal_kind")

    def test_missing_or_wrong_type_compared_fields_fail_closed(self):
        scenario = "lazy-no-consumption"
        for key in rust_trace(scenario)["shared"]:
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                del rust["shared"][key]
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["shared"]["source_terminal_count"] = True
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_early_completion_and_unobserved_cleanup_are_rejected(self):
        scenario = "lazy-stop-consumption"
        for field, value in (("terminal_count", 1), ("source_eof", True), ("source_dropped", True)):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-3]["observed"][field] = value
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["runtimeDetails"]["source_dropped"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_missing_stop_checkpoint_or_result_observation_fails(self):
        scenario = "lazy-stop-consumption"
        rust = rust_trace(scenario)
        del rust["steps"][2]
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["runtimeDetails"]["result_observed"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_wrong_identity_and_unknown_scenarios_fail(self):
        scenario = "lazy-no-consumption"
        rust = rust_trace(scenario)
        rust["schemaVersion"] = 1
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        self.assertEqual(compare_traces({}, {}, "unknown")["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["sourceAssertions"] = "failed"
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_setup_error_details_are_retained_not_normalized_away(self):
        scenario = "lazy-setup-failure"
        rust = rust_trace(scenario)
        result = compare_traces(upstream_trace(scenario), rust, scenario)
        self.assertEqual(result["difference"]["rust"], rust["runtimeDetails"])
        rust["shared"]["terminal_count"] = 2
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")


if __name__ == "__main__":
    unittest.main()

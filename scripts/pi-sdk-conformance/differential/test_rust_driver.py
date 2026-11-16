from __future__ import annotations

import os
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from common import build_rust_driver, file_hash, run_rust_trace
from compare_lifecycle import (
    DEFERRED_AUTH_SETUP_FAILURE, DEFERRED_HANDLE_EXPIRY, DEFERRED_HANDLE_ROUTE_MISMATCH,
    DEFERRED_LOCAL_CANCEL_AFTER_DONE, DEFERRED_LOCAL_CANCEL_BEFORE_DONE,
    DEFERRED_POSITIVE_WAIT, DEFERRED_READY_BEFORE_EOF, DEFERRED_REMOTE_CANCEL, DEFERRED_RESULT_BEFORE_EOF,
    DEFERRED_UNKNOWN_JOB_ID,
    DEFERRED_STREAM_SOURCE_CLEANUP, DEFERRED_WAIT0, EOF_CHECKPOINTS,
    FIRST_CHUNK_DURATION_ERROR, FIRST_CHUNK_DURATION_FAILURE,
    FIRST_CHUNK_WRITE_ERROR, FIRST_CHUNK_WRITE_FAILURE, HISTOGRAM_CREATION_ERROR,
    REQUEST_DURATION_ERROR, REQUEST_DURATION_FAILURE,
    HISTOGRAM_CREATION_FAILURE, REQUEST_SPAN_START_ERROR, REQUEST_SPAN_START_FAILURE, RESULT_ATTRIBUTE_WRITE_ERROR,
    RESULT_ATTRIBUTE_WRITE_FAILURE, RESULT_BEFORE_EOF, SCENARIOS, SETUP_CASES,
    TERMINAL_AND_SPAN_BEFORE_EOF, compare_traces,
)
from test_lifecycle_differential import upstream_trace


class RustLifecycleDriverTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        executable = os.environ.get("OPI_LIFECYCLE_DRIVER")
        if executable is None:
            cls.build = build_rust_driver()
        else:
            cls.build = {"executable": executable, "executable_sha256": file_hash(Path(executable))}

    def test_scenarios_observe_barriers_and_public_results(self):
        for scenario in SCENARIOS:
            with self.subTest(scenario=scenario):
                trace = run_rust_trace(scenario, self.build)["trace"]
                comparison = compare_traces(upstream_trace(scenario), trace, scenario)
                self.assertEqual(comparison["shared_observations"], "pass")
                if scenario in SETUP_CASES:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["rust"]["result_delivery"], "typed-err")
                elif scenario == DEFERRED_WAIT0:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-poll-telemetry-at-result")
                    for step in trace["steps"]:
                        self.assertEqual(step["settled_at_result"], [True])
                        self.assertEqual(step["metric_names_at_result"], [row["name"] for row in step["measurements"]])
                elif scenario in (DEFERRED_RESULT_BEFORE_EOF, DEFERRED_READY_BEFORE_EOF):
                    held, released = trace["steps"][1:]
                    if scenario == DEFERRED_READY_BEFORE_EOF:
                        self.assertEqual(comparison["status"], "pass", comparison)
                        self.assertEqual(held["result"], released["result"])
                    else:
                        self.assertEqual(comparison["status"], "difference", comparison)
                        self.assertEqual(comparison["difference"]["rust"], {"result_state": "pending", "result": None})
                    self.assertIsNone(trace["steps"][0]["result"])
                    self.assertFalse(held["observed"]["source_dropped"])
                    self.assertFalse(held["spans"][0]["settled"])
                    self.assertTrue(released["observed"]["source_dropped"])
                    self.assertTrue(released["spans"][0]["settled"])
                elif scenario == DEFERRED_LOCAL_CANCEL_AFTER_DONE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-local-cancel-after-done")
                    cancelled = trace["steps"][2]
                    refetched = trace["steps"][-1]
                    self.assertTrue(cancelled["backend_cancelled"])
                    self.assertTrue(cancelled["observed"]["source_dropped"])
                    self.assertFalse(cancelled["observed"]["source_eof"])
                    self.assertTrue(cancelled["spans"][0]["settled"])
                    self.assertEqual(refetched["remote_cancel_calls"], 0)
                    self.assertEqual(refetched["result"], cancelled["result"])
                elif scenario == DEFERRED_LOCAL_CANCEL_BEFORE_DONE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-local-cancel-before-done")
                    held, cancelled, released, refetched = trace["steps"]
                    self.assertIsNone(held["result"])
                    self.assertEqual(cancelled["typed_cause"], "Cancelled")
                    self.assertTrue(cancelled["observed"]["source_dropped"])
                    self.assertEqual(cancelled["observed"]["terminal_count"], 0)
                    self.assertFalse(released["observed"]["source_eof"])
                    self.assertTrue(cancelled["spans"][0]["settled"])
                    self.assertEqual(cancelled["result"]["stop"], "aborted")
                    self.assertEqual(refetched["remote_cancel_calls"], 0)
                    self.assertEqual(refetched["result"]["stop"], "stop")
                elif scenario == DEFERRED_REMOTE_CANCEL:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-refetch-after-remote-cancel")
                    pending, cancelled, refetched = trace["steps"]
                    handle = pending["result"]["deferred"]
                    self.assertEqual(cancelled["cancelled_handles"], [handle])
                    self.assertEqual(cancelled["dispatches"], [])
                    self.assertEqual(cancelled["spans"], [])
                    self.assertEqual(refetched["result"]["provider"], handle["provider"])
                    self.assertEqual(refetched["result"]["model"], handle["modelId"])
                    self.assertEqual(refetched["typed_cause"], "Config")
                elif scenario == DEFERRED_AUTH_SETUP_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-auth-setup-failure")
                    gate, failed = trace["steps"]
                    self.assertEqual(gate["result_state"], "pending")
                    self.assertEqual(gate["auth_calls"], 1)
                    self.assertEqual(gate["measurements"], [])
                    self.assertEqual(failed["dispatches"], [])
                    self.assertEqual(failed["typed_cause"], "AuthFailed")
                    self.assertEqual(failed["spans"][0]["events"], ["pi.ai.first_chunk"])
                elif scenario == DEFERRED_POSITIVE_WAIT:
                    self.assertEqual(comparison["status"], "pass", comparison)
                    waiting, deferred, waiting_for_ready, ready = trace["steps"]
                    handle = deferred["result"]["deferred"]
                    self.assertEqual(handle["pollAfterMs"], 25)
                    for step in trace["steps"]:
                        self.assertEqual(step["dispatches"], [{"handle": handle, "wait_ms": 50}])
                    for step in (waiting, waiting_for_ready):
                        self.assertEqual(step["result_state"], "pending")
                        self.assertEqual(step["measurements"], [])
                    self.assertEqual(deferred["result"]["stop"], "deferred")
                    self.assertEqual(ready["result"]["stop"], "stop")
                elif scenario == DEFERRED_HANDLE_EXPIRY:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-handle-expiry")
                    before, expired, without_clock = trace["steps"]
                    handle = before["request_handle"]
                    self.assertEqual([step["request_handle"] for step in trace["steps"]], [handle] * 3)
                    self.assertEqual([step["sdk_now_ms"] for step in trace["steps"]],
                                     [handle["expiresAt"] - 1, handle["expiresAt"], None])
                    self.assertEqual([step["auth_calls"] for step in trace["steps"]], [1, 0, 1])
                    self.assertEqual([step["fetch_count"] for step in trace["steps"]], [1, 0, 1])
                    self.assertEqual(expired["result_delivery"], "typed-err")
                    self.assertEqual(expired["typed_cause"], "Config")
                    self.assertEqual(expired["dispatches"], [])
                    self.assertEqual(before["result"], without_clock["result"])
                elif scenario == DEFERRED_HANDLE_ROUTE_MISMATCH:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-handle-route-mismatch")
                    steps = trace["steps"]
                    self.assertEqual([step["auth_calls"] for step in steps], [0, 0, 0, 1])
                    self.assertEqual([step["fetch_count"] for step in steps], [0, 0, 0, 1])
                    self.assertEqual([step["handle_validation"] for step in steps],
                                     ["Unsupported", "RouteMismatch", "Unsupported", "ok"])
                    self.assertEqual([step["typed_cause"] for step in steps], ["Config", "Config", "Config", None])
                    self.assertEqual([step["result"]["stop"] for step in steps], ["error", "error", "error", "stop"])
                elif scenario == DEFERRED_UNKNOWN_JOB_ID:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-unknown-job-id")
                    failed, ready = trace["steps"]
                    self.assertEqual(failed["request_handle"], ready["request_handle"] | {"id": "deferred-missing"})
                    for step in (failed, ready):
                        self.assertEqual((step["handle_validation"], step["auth_calls"], step["fetch_count"]), ("ok", 1, 1))
                        self.assertEqual(step["dispatches"], [{"handle": step["request_handle"], "wait_ms": 0}])
                    self.assertEqual((failed["result"]["stop"], failed["typed_cause"]), ("error", "Config"))
                    self.assertEqual(ready["result"]["stop"], "stop")
                elif scenario == DEFERRED_STREAM_SOURCE_CLEANUP:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    self.assertEqual(comparison["difference"]["id"], "deferred-stream-source-cleanup")
                    steps = {step["name"]: step for step in trace["steps"]}
                    public_eof = steps["public-eof-observed"]
                    self.assertTrue(public_eof["spans"][0]["settled"])
                    self.assertTrue(public_eof["observed"]["source_dropped"])
                    self.assertTrue(public_eof["observed"]["source_eof"])
                    self.assertEqual(steps["next-eof-gate-held"]["delivery"], {"kind": "pending"})
                    self.assertFalse(steps["next-eof-gate-held"]["spans"][0]["settled"])
                    self.assertTrue(steps["source-released"]["observed"]["source_eof"])
                    self.assertEqual(steps["started-done-observed"]["delivery"], steps["done-observed"]["delivery"])
                    self.assertEqual(steps["started-next-eof-gate-held"]["delivery"], {"kind": "pending"})
                    for name in ("started-done-observed", "started-next-eof-gate-held",
                                 "consumer-dropped-eof-gate-held", "cached-terminal-eof-gate-held"):
                        self.assertTrue(steps[name]["eof_gate_reached"])
                        self.assertFalse(steps[name]["observed"]["source_eof"])
                        self.assertFalse(steps[name]["spans"][0]["settled"])
                        self.assertFalse(steps[name]["observed"]["source_dropped"])
                    for name in ("started-public-eof-observed", "consumer-dropped-source-released",
                                 "cached-terminal-public-eof-observed"):
                        self.assertTrue(steps[name]["observed"]["source_eof"])
                        self.assertTrue(steps[name]["observed"]["source_dropped"])
                        self.assertTrue(steps[name]["spans"][0]["settled"])
                    for name in ("started-result-observed", "cached-terminal-result-observed"):
                        self.assertEqual(steps[name]["delivery"], steps["source-released"]["delivery"])
                    for prefix, tail_polled in (("stop-after-done", False), ("stop-after-pending-tail", True)):
                        stopped = steps[f"{prefix}-consumer-stopped"]
                        self.assertEqual(stopped["eof_gate_reached"], tail_polled)
                        self.assertFalse(stopped["observed"]["source_eof"])
                        self.assertTrue(stopped["observed"]["source_dropped"])
                        self.assertEqual(stopped["spans"], public_eof["spans"])
                        self.assertEqual(stopped["measurements"], public_eof["measurements"])
                        self.assertEqual(stopped["poll_finish_source_dropped"], [True])
                        self.assertEqual(stopped["remote_cancel_calls"], 0)
                        self.assertEqual(steps[f"{prefix}-after-eof-release"], stopped | {
                            "name": f"{prefix}-after-eof-release",
                        })
                    full_stop = steps["stop-after-public-eof-consumer-stopped"]
                    self.assertEqual(full_stop["observed"], public_eof["observed"])
                    self.assertEqual(full_stop["spans"], public_eof["spans"])
                    self.assertEqual(full_stop["poll_finish_source_dropped"], [True])
                    cancelled = steps["cancel-after-done-public-eof-observed"]
                    self.assertTrue(cancelled["backend_cancelled"])
                    self.assertTrue(cancelled["observed"]["source_dropped"])
                    self.assertFalse(cancelled["observed"]["source_eof"])
                    self.assertTrue(cancelled["spans"][0]["settled"])
                    self.assertEqual(steps["cancel-after-done-source-released"]["delivery"], steps["source-released"]["delivery"])
                elif scenario == REQUEST_SPAN_START_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    state = comparison["difference"]["rust"]
                    self.assertEqual(state["fault_delivery"], "typed-err")
                    self.assertEqual(state["next_state_at_terminal_gate"], "pending")
                    self.assertEqual(state["terminal_kind"], "done")
                    self.assertEqual(state["source_release"], "eof")
                    self.assertTrue(state["source_eof"])
                    self.assertEqual(state["host_errors"], [{
                        "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                        "fields": {"message": "AI telemetry observer failed to start", "error": REQUEST_SPAN_START_ERROR},
                    }])
                    self.assertEqual(trace["steps"][-1]["span_finish_calls"], 0)
                    self.assertFalse(trace["steps"][-1]["span_backend_dropped"])
                    self.assertTrue(trace["runtimeDetails"]["source_dropped"])
                elif scenario == HISTOGRAM_CREATION_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    state = comparison["difference"]["rust"]
                    self.assertEqual(state["call_delivery"], "returned-handle")
                    self.assertEqual(state["dispatch_calls"], 1)
                    self.assertEqual(state["public_terminal"]["kind"], "done")
                    self.assertEqual(state["result_delivery"], "ok")
                    self.assertTrue(state["source_eof"])
                    self.assertTrue(state["source_dropped"])
                    self.assertIsNone(state["request_span_at_fault"])
                    self.assertIsNone(state["request_span_at_call_boundary"])
                    self.assertIsNone(state["request_span_at_end"])
                    self.assertEqual(state["span_finish_calls_at_call_boundary"], 0)
                    self.assertEqual(state["span_finish_calls"], 0)
                    self.assertFalse(state["span_backend_dropped_at_call_boundary"])
                    self.assertFalse(state["span_backend_dropped"])
                    self.assertEqual(state["host_errors"], [{
                        "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                        "fields": {"message": "AI telemetry observer failed to start", "error": HISTOGRAM_CREATION_ERROR},
                    }])
                elif scenario == FIRST_CHUNK_WRITE_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    steps = {step["name"]: step for step in trace["steps"]}
                    held = steps["terminal-gate-held"]
                    self.assertEqual(held["next_state"], "pending")
                    self.assertTrue(held["request_spans"][0]["settled"])
                    self.assertIsNone(held["request_spans"][0]["stop"])
                    self.assertFalse(held["observed"]["source_dropped"])
                    completed = steps["public-stream-completed"]
                    self.assertEqual(completed["terminal"]["kind"], "done")
                    self.assertTrue(completed["observed"]["source_eof"])
                    self.assertTrue(completed["observed"]["source_dropped"])
                    self.assertTrue(steps["result-observed"]["matches_terminal"])
                    fault = steps["first-chunk-write-failure-observed"]
                    self.assertEqual(fault["span_finish_calls"], 1)
                    self.assertTrue(fault["span_backend_dropped"])
                    self.assertEqual(fault["host_errors"], [{
                        "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                        "fields": {"message": "AI telemetry observer failed while recording an event", "error": FIRST_CHUNK_WRITE_ERROR},
                    }])
                elif scenario == FIRST_CHUNK_DURATION_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    state = comparison["difference"]["rust"]
                    self.assertEqual(state["next_state_at_source_exit"], "pending")
                    self.assertEqual(state["source_release"], "eof")
                    self.assertTrue(trace["runtimeDetails"]["source_dropped"])
                    self.assertTrue(state["request_span_at_source_exit"]["settled"])
                    self.assertEqual(state["request_span_at_end"]["events"], ["pi.ai.first_chunk"])
                    self.assertIsNone(state["request_span_at_end"]["stop"])
                    self.assertEqual(state["duration_measurements"], [])
                    self.assertEqual(trace["steps"][-1]["span_finish_calls"], 1)
                    self.assertTrue(trace["steps"][-1]["span_backend_dropped"])
                    self.assertEqual(state["host_errors"], [{
                        "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                        "fields": {"message": "AI telemetry observer failed while recording an event", "error": FIRST_CHUNK_DURATION_ERROR},
                    }])
                elif scenario == REQUEST_DURATION_FAILURE:
                    self.assertEqual(comparison["status"], "difference", comparison)
                    state = comparison["difference"]["rust"]
                    fault = trace["steps"][-1]
                    attempt, = fault["record_attempts"]
                    self.assertTrue(attempt["observed"]["source_dropped"])
                    self.assertEqual(attempt["request_spans"][0]["stop"], "stop")
                    self.assertEqual(fault["metric_names"], ["pi.ai.first_chunk.duration"])
                    self.assertEqual(fault["span_finish_calls"], 1)
                    self.assertTrue(fault["span_backend_dropped"])
                    self.assertTrue(state["span_settled"])
                    self.assertEqual(fault["request_spans"][0]["stop"], "stop")
                    self.assertEqual(state["host_errors"], [{
                        "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                        "fields": {"message": "AI telemetry observer failed to finish", "error": REQUEST_DURATION_ERROR},
                    }])
                else:
                    steps = {step["name"]: step for step in trace["steps"]}
                    if scenario == RESULT_BEFORE_EOF:
                        self.assertEqual(comparison["status"], "difference", comparison)
                        checkpoint = steps["result-before-source-eof"]
                        self.assertEqual(checkpoint["result_state"], "pending")
                        self.assertIsNone(checkpoint["result"])
                        self.assertEqual(checkpoint["observed"]["terminal_count"], 1)
                        self.assertFalse(checkpoint["observed"]["source_eof"])
                    elif scenario == TERMINAL_AND_SPAN_BEFORE_EOF:
                        self.assertEqual(comparison["status"], "difference", comparison)
                        held = steps[EOF_CHECKPOINTS[scenario]]
                        self.assertEqual(held["next_state"], "pending")
                        self.assertFalse(held["request_spans"][0]["settled"])
                        completed = steps["public-stream-completed"]
                        self.assertEqual(completed["terminal"]["kind"], "done")
                        self.assertTrue(completed["outer_eof"])
                        self.assertTrue(completed["request_spans"][0]["settled"])
                    elif scenario == RESULT_ATTRIBUTE_WRITE_FAILURE:
                        self.assertEqual(comparison["status"], "difference", comparison)
                        fault = steps["result-attribute-write-failure-observed"]
                        self.assertEqual(len(fault["write_attempts"]), 1)
                        self.assertTrue(fault["write_attempts"][0]["observed"]["source_dropped"])
                        self.assertTrue(fault["request_spans"][0]["settled"])
                        self.assertIsNone(fault["request_spans"][0]["stop"])
                        self.assertEqual(fault["span_finish_calls"], 1)
                        self.assertTrue(fault["span_backend_dropped"])
                        self.assertEqual(fault["host_errors"], [{
                            "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                            "fields": {"message": "AI telemetry observer failed to finish", "error": RESULT_ATTRIBUTE_WRITE_ERROR},
                        }])
                    else:
                        self.assertEqual(comparison["status"], "pass", comparison)
                    self.assertTrue(steps["source-released"]["observed"]["source_eof"])
                    self.assertEqual(steps["before-source-release"]["observed"]["terminal_count"], 0)
                    self.assertTrue(trace["runtimeDetails"]["source_dropped"])
                    self.assertTrue(trace["runtimeDetails"]["result_observed"])

    def test_unknown_scenario_is_a_nonzero_subprocess_failure(self):
        with self.assertRaises(subprocess.CalledProcessError) as raised:
            run_rust_trace("unknown", self.build)
        self.assertEqual(raised.exception.returncode, 2)
        self.assertIn(b"unknown lifecycle scenario", raised.exception.stderr)


if __name__ == "__main__":
    unittest.main()

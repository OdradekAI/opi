from __future__ import annotations

import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from common import FIXTURE_DIR
from compare_lifecycle import (
    DEFERRED_AUTH_SETUP_FAILURE, DEFERRED_HANDLE_EXPIRY, DEFERRED_HANDLE_ROUTE_MISMATCH,
    DEFERRED_LOCAL_CANCEL_AFTER_DONE, DEFERRED_LOCAL_CANCEL_BEFORE_DONE,
    DEFERRED_POSITIVE_WAIT, DEFERRED_READY_BEFORE_EOF, DEFERRED_REMOTE_CANCEL, DEFERRED_RESULT_BEFORE_EOF,
    DEFERRED_UNKNOWN_JOB_ID,
    DEFERRED_STREAM_SOURCE_CLEANUP, DEFERRED_WAIT0, EOF_CHECKPOINTS,
    FIRST_CHUNK_DURATION_ERROR, FIRST_CHUNK_DURATION_FAILURE,
    FIRST_CHUNK_WRITE_ERROR, FIRST_CHUNK_WRITE_FAILURE, HISTOGRAM_CREATION_ERROR,
    HISTOGRAM_CREATION_FAILURE, MISSING_SPAN_EVENT_ERROR, REQUEST_INSTRUMENTS,
    REQUEST_DURATION_ERROR, REQUEST_DURATION_FAILURE,
    REQUEST_SPAN_START_ERROR, REQUEST_SPAN_START_FAILURE, RESULT_ATTRIBUTE_WRITE_ERROR,
    RESULT_ATTRIBUTE_WRITE_FAILURE, RESULT_BEFORE_EOF, SCENARIOS, SETUP_CASES,
    TERMINAL_AND_SPAN_BEFORE_EOF, compare_traces,
)


HISTORICAL = json.loads((FIXTURE_DIR / "phase5-lifecycle-differential.json").read_text(encoding="utf-8"))
DEFERRED_FIRST_CHUNK_REPLAY = "phase5-deferred-auth-setup-failure-differential.json"


def frozen_case(file: str, scenario: str) -> dict:
    report = json.loads((FIXTURE_DIR / file).read_bytes())
    return next(case for case in report["cases"] if case["scenario"] == scenario)


FROZEN_REPLAYS = {
    FIRST_CHUNK_DURATION_FAILURE: frozen_case(
        "phase5-first-chunk-duration-record-failure-differential.json", FIRST_CHUNK_DURATION_FAILURE,
    ),
    REQUEST_DURATION_FAILURE: frozen_case(
        "phase5-request-duration-record-failure-differential.json", REQUEST_DURATION_FAILURE,
    ),
    DEFERRED_WAIT0: frozen_case("phase5-deferred-wait0-differential.json", DEFERRED_WAIT0),
    DEFERRED_RESULT_BEFORE_EOF: frozen_case(
        "phase5-deferred-result-before-source-eof-differential.json", DEFERRED_RESULT_BEFORE_EOF,
    ),
    DEFERRED_READY_BEFORE_EOF: frozen_case(
        "phase5-deferred-fetch-ready-differential.json", DEFERRED_READY_BEFORE_EOF,
    ),
    DEFERRED_LOCAL_CANCEL_AFTER_DONE: frozen_case(
        "phase5-deferred-local-cancel-after-done-differential.json", DEFERRED_LOCAL_CANCEL_AFTER_DONE,
    ),
    DEFERRED_LOCAL_CANCEL_BEFORE_DONE: frozen_case(
        "phase5-deferred-local-cancel-before-done-differential.json", DEFERRED_LOCAL_CANCEL_BEFORE_DONE,
    ),
    DEFERRED_REMOTE_CANCEL: frozen_case(DEFERRED_FIRST_CHUNK_REPLAY, DEFERRED_REMOTE_CANCEL),
    DEFERRED_AUTH_SETUP_FAILURE: frozen_case(DEFERRED_FIRST_CHUNK_REPLAY, DEFERRED_AUTH_SETUP_FAILURE),
    DEFERRED_POSITIVE_WAIT: frozen_case("phase5-deferred-positive-wait-differential.json", DEFERRED_POSITIVE_WAIT),
    DEFERRED_HANDLE_EXPIRY: frozen_case("phase5-deferred-handle-expiry-differential.json", DEFERRED_HANDLE_EXPIRY),
    DEFERRED_HANDLE_ROUTE_MISMATCH: frozen_case(
        "phase5-deferred-lookup-diagnostic-differential.json", DEFERRED_HANDLE_ROUTE_MISMATCH,
    ),
    DEFERRED_UNKNOWN_JOB_ID: frozen_case("phase5-deferred-unknown-job-id-differential.json", DEFERRED_UNKNOWN_JOB_ID),
    DEFERRED_STREAM_SOURCE_CLEANUP: frozen_case(
        "phase5-deferred-started-done-differential.json", DEFERRED_STREAM_SOURCE_CLEANUP,
    ),
}


def setup_steps(scenario: str, *, upstream: bool) -> list[dict]:
    auth = scenario == "models-auth-setup-failure"
    phase = "authentication" if auth else "route-lookup"
    provider = "lifecycle-differential" if auth else "missing-provider"
    diagnostic = "controlled-auth-setup-failure" if auth else "Unknown provider: missing-provider"
    key = "step" if upstream else "name"
    steps = [
        {key: "constructed", "phase": phase, "provider": provider, "model": "mock-model",
         "auth_calls": int(upstream and auth), "dispatch_calls": 0},
    ]
    if auth:
        steps.append({key: "auth-at-gate", "auth_calls": 1, "dispatch_calls": 0})
    steps += [
        {key: "failure-event", "delivery": "error-event" if upstream else "stream-item-err",
         "stop": "error", "error_message": diagnostic,
         "message_provider": provider, "message_model": "mock-model"},
        {key: "event-eof", "done": True, "auth_calls": int(auth), "dispatch_calls": 0},
        {key: "result-observed", "delivery": "fulfilled-message" if upstream else "typed-err",
         "stop": "error", "matches_event": True},
    ]
    if not upstream:
        steps[-3]["cause"] = "AuthFailed" if auth else "Config"
    return steps


def upstream_trace(scenario: str) -> dict:
    if scenario in FROZEN_REPLAYS:
        return deepcopy(FROZEN_REPLAYS[scenario]["upstream_runs"][0]["trace"])
    if scenario in SETUP_CASES:
        return {"id": scenario, "sourceAssertions": "passed", "trace": setup_steps(scenario, upstream=True)}
    return {"id": scenario, "sourceAssertions": "passed", "trace": completion_steps(scenario, upstream=True)}


def completion_steps(scenario: str, *, upstream: bool) -> list[dict]:
    key = "step" if upstream else "name"
    before = {"dispatch_calls": 1, "source_started": True, "terminal_count": 0, "source_eof": False}
    initial = {**before, "dispatch_calls": 0, "source_started": False}
    after = {**before, "terminal_count": 1, "source_eof": True}
    if upstream:
        for state in (initial, before, after):
            state.update(source_iterators=state["dispatch_calls"], source_return_calls=0)
    else:
        for state in (initial, before, after):
            state["source_dropped"] = state["source_eof"]
    result = {"stop": "stop", "provider": "lifecycle-differential", "model": "mock-model"}
    if scenario == HISTOGRAM_CREATION_FAILURE:
        identity = {"provider": result["provider"], "model": result["model"]}
        span = {"name": "pi.ai.request", **identity, "settled": False, "stop": None}
        boundary = {
            key: "call-boundary", **identity, "delivery": "synchronous-throw" if upstream else "returned-handle",
            "error_message": HISTOGRAM_CREATION_ERROR if upstream else None,
            "instrument_attempt_count": 1, "observed": initial.copy(),
            "request_spans": [] if upstream else [span.copy()],
        }
        fault = {
            key: "histogram-creation-failure-observed",
            "fault_delivery": "synchronous-throw" if upstream else "typed-err", "fault_marker": HISTOGRAM_CREATION_ERROR,
            "instrument_attempts": [{
                "name": "pi.ai.request.duration", "kind": "histogram", "unit": "ms",
                "observed": initial.copy(), "request_spans": [] if upstream else [span.copy()],
            }],
            "observed": initial.copy() if upstream else after,
            "request_spans": [] if upstream else [span.copy()], "host_errors": [],
        }
        if upstream:
            return [boundary, fault]
        boundary.update(span_finish_calls=0, span_backend_dropped=True)
        fault.update(span_finish_calls=0, span_backend_dropped=True, host_errors=[{
            "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
            "fields": {"message": "AI telemetry observer failed to start", "error": HISTOGRAM_CREATION_ERROR},
        }])
        return [
            boundary,
            {key: "source-at-gate", "observed": before.copy()},
            {key: "before-source-release", "observed": before, "public_start": "start"},
            {key: "public-stream-completed", "terminal": {"kind": "done", **result}, "outer_eof": True},
            {key: "result-observed", **result, "delivery": "ok", "matches_terminal": True},
            fault,
        ]
    if scenario == REQUEST_SPAN_START_FAILURE:
        held = before.copy()
        if upstream:
            held.update(source_return_calls=1, source_return_completions=1, source_finally_calls=1, source_result_calls=0)
            after = held.copy()
            result["stop"] = "error"
        terminal = {"kind": "error" if upstream else "done", **result}
        if upstream:
            terminal["error_message"] = MISSING_SPAN_EVENT_ERROR
        fault = {
            key: "request-span-start-failure-observed", "fault_marker": REQUEST_SPAN_START_ERROR,
            "fault_delivery": "rejected-promise" if upstream else "typed-err", "request_spans": [],
            "span_start_attempts": [{
                "name": "pi.ai.request", "provider": result["provider"], "model": result["model"],
                "created_instruments": [item.copy() for item in REQUEST_INSTRUMENTS],
                "observed": initial.copy(), "request_spans": [],
            }],
        }
        if upstream:
            fault["host_errors"] = [
                {"channel": "unhandledRejection", "message": f"Error: {REQUEST_SPAN_START_ERROR}",
                 "error_name": "Error", "injected_start_rejection": True},
                {"channel": "unhandledRejection",
                 "message": "TypeError: Cannot read properties of undefined (reading 'setAttributes')",
                 "error_name": "TypeError", "injected_start_rejection": False},
            ]
        else:
            fault.update(span_finish_calls=0, span_backend_dropped=False, host_errors=[{
                "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                "fields": {"message": "AI telemetry observer failed to start", "error": REQUEST_SPAN_START_ERROR},
            }])
        return [
            {key: "constructed", "call_delivery": "returned-handle", "span_start_attempt_count": 1,
             "observed": initial, "request_spans": []},
            {key: "terminal-gate-held", "observed": held, "public_start": "start", "request_spans": [],
             "next_state": "ready" if upstream else "pending", "terminal": terminal.copy() if upstream else None},
            {key: "public-stream-completed", "terminal": terminal, "outer_eof": True, "observed": after},
            {key: "result-observed", **result, "matches_terminal": True,
             "delivery": "fulfilled-message" if upstream else "ok"},
            fault,
        ]
    if scenario == FIRST_CHUNK_WRITE_FAILURE:
        terminal = {"kind": "done", **result}
        span = {"name": "pi.ai.request", "provider": result["provider"], "model": result["model"],
                "settled": True, "stop": None}
        held = before.copy()
        if upstream:
            before.update(source_return_completions=0, source_finally_calls=0, source_result_calls=0)
            held = {**before, "source_return_calls": 1, "source_return_completions": 1, "source_finally_calls": 1}
            after = held.copy()
            result["stop"] = "error"
            terminal = {"kind": "error", **result, "error_message": FIRST_CHUNK_WRITE_ERROR}
            span["stop"] = "error"
        fault = {
            key: "first-chunk-write-failure-observed",
            "fault_delivery": "synchronous-throw" if upstream else "typed-err",
            "fault_marker": FIRST_CHUNK_WRITE_ERROR,
            "write_attempts": [{"event": "pi.ai.first_chunk", "observed": before}],
            "request_spans": [span.copy()], "host_errors": [],
        }
        if not upstream:
            fault.update(span_finish_calls=1, span_backend_dropped=True, host_errors=[{
                "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                "fields": {"message": "AI telemetry observer failed while recording an event", "error": FIRST_CHUNK_WRITE_ERROR},
            }])
        return [
            {key: "constructed", "observed": initial, "request_spans": [{**span, "settled": False, "stop": None}]},
            {key: "terminal-gate-held", "observed": held, "public_start": "start", "request_spans": [span.copy()],
             "next_state": "ready" if upstream else "pending", "terminal": terminal.copy() if upstream else None},
            {key: "public-stream-completed", "terminal": terminal, "outer_eof": True, "observed": after},
            {key: "result-observed", **result, "matches_terminal": True,
             "delivery": "fulfilled-message" if upstream else "ok"},
            fault,
        ]
    steps = [
        {key: "constructed", "observed": initial},
        {key: "source-at-gate", "observed": before.copy()},
    ]
    if scenario == "models-stop-consumption":
        steps.append({key: "consumer-stopped-after-start", "observed": before.copy()})
    steps += [
        {key: "before-source-release", "observed": before},
        {key: "source-released", "observed": after},
        {key: "result-observed", **result},
    ]
    if upstream:
        steps[-1]["matches_source"] = True
    if scenario == RESULT_BEFORE_EOF:
        steps[-3]["result_state"] = "pending"
        steps.insert(-2, {
            key: "result-before-source-eof",
            "observed": {**before, "terminal_count": 1},
            "result_state": "ready" if upstream else "pending",
            "result": result if upstream else None,
        })
    elif scenario in (TERMINAL_AND_SPAN_BEFORE_EOF, RESULT_ATTRIBUTE_WRITE_FAILURE):
        span = {"name": "pi.ai.request", "provider": result["provider"], "model": result["model"],
                "settled": False, "stop": None}
        terminal = {"kind": "done", **result}
        steps[-3].update(public_start="start", request_spans=[span.copy()])
        steps[-1]["matches_terminal"] = True
        completed = {key: "public-stream-completed", "terminal": terminal, "outer_eof": True}
        if scenario == TERMINAL_AND_SPAN_BEFORE_EOF:
            steps[-3]["next_state"] = "pending"
            steps.insert(-2, {
                key: EOF_CHECKPOINTS[scenario], "observed": {**before, "terminal_count": 1},
                "next_state": "ready" if upstream else "pending", "terminal": terminal.copy() if upstream else None,
                "request_spans": [span.copy()],
            })
            completed["request_spans"] = [{**span, "settled": True, "stop": result["stop"]}]
        steps.insert(-1, completed)
        if scenario == RESULT_ATTRIBUTE_WRITE_FAILURE:
            fault = {
                key: "result-attribute-write-failure-observed",
                "fault_delivery": "synchronous-throw" if upstream else "typed-err",
                "fault_marker": RESULT_ATTRIBUTE_WRITE_ERROR,
                "write_attempts": [{"observed": after.copy(), "stop": result["stop"]}],
                "request_spans": [{**span, "settled": not upstream}], "host_errors": [],
            }
            if not upstream:
                fault.update(span_finish_calls=1, span_backend_dropped=True, host_errors=[{
                    "channel": "tracing", "level": "WARN", "target": "opi_ai::telemetry",
                    "fields": {"message": "AI telemetry observer failed to finish", "error": RESULT_ATTRIBUTE_WRITE_ERROR},
                }])
            steps.append(fault)
    return steps


def rust_trace(scenario: str) -> dict:
    if scenario in FROZEN_REPLAYS:
        return deepcopy(FROZEN_REPLAYS[scenario]["rust_run"]["trace"])
    if scenario in SETUP_CASES:
        return {
            "schemaVersion": 2, "runtime": "rust", "scenario": scenario,
            "steps": setup_steps(scenario, upstream=False),
        }
    return {
        "schemaVersion": 2, "runtime": "rust", "scenario": scenario,
        "steps": completion_steps(scenario, upstream=False),
        "runtimeDetails": {"consumer_handle_dropped": False, "source_dropped": True, "result_observed": True},
    }


class LifecycleComparatorTests(unittest.TestCase):
    def test_shared_success_is_not_strict_alignment_or_approved_difference(self):
        for scenario in SCENARIOS:
            with self.subTest(scenario=scenario):
                result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
                self.assertEqual(result["shared_observations"], "pass")
                self.assertFalse(result["strict_alignment"])
                if (scenario in SETUP_CASES or scenario in EOF_CHECKPOINTS
                        or scenario in (RESULT_ATTRIBUTE_WRITE_FAILURE, FIRST_CHUNK_WRITE_FAILURE,
                                        HISTOGRAM_CREATION_FAILURE, REQUEST_SPAN_START_FAILURE,
                                        FIRST_CHUNK_DURATION_FAILURE, REQUEST_DURATION_FAILURE,
                                        DEFERRED_WAIT0, DEFERRED_RESULT_BEFORE_EOF, DEFERRED_LOCAL_CANCEL_AFTER_DONE,
                                        DEFERRED_LOCAL_CANCEL_BEFORE_DONE, DEFERRED_REMOTE_CANCEL,
                                        DEFERRED_AUTH_SETUP_FAILURE, DEFERRED_HANDLE_EXPIRY,
                                        DEFERRED_HANDLE_ROUTE_MISMATCH, DEFERRED_UNKNOWN_JOB_ID,
                                        DEFERRED_STREAM_SOURCE_CLEANUP)):
                    self.assertEqual(result["status"], "difference")
                    self.assertEqual(result["decision_status"], "not-approved-by-this-probe")
                else:
                    self.assertEqual(result["status"], "pass")
                    field = {
                        DEFERRED_POSITIVE_WAIT: "held_result_states",
                        DEFERRED_READY_BEFORE_EOF: "result_while_eof_held",
                    }.get(scenario, "dispatch_calls_at_construction")
                    self.assertIn(field, result["compared_fields"])
                    self.assertNotIn("difference", result)

    def test_result_availability_is_compared_while_eof_is_held(self):
        upstream = upstream_trace(RESULT_BEFORE_EOF)
        rust = rust_trace(RESULT_BEFORE_EOF)
        result = compare_traces(upstream, rust, RESULT_BEFORE_EOF)
        self.assertEqual(result["difference"], {
            "id": "result-before-source-eof",
            "upstream": {"result_state": "ready"},
            "rust": {"result_state": "pending"},
        })
        self.assertIn("terminal_count_at_eof_gate", result["compared_fields"])
        self.assertIn("source_eof_at_eof_gate", result["compared_fields"])
        upstream["trace"][-3].update(result_state="pending", result=None)
        result = compare_traces(upstream, rust, RESULT_BEFORE_EOF)
        self.assertEqual(result["status"], "pass")
        self.assertNotIn("difference", result)

    def test_result_timing_rejects_missing_or_invalid_eof_gate(self):
        upstream = upstream_trace(RESULT_BEFORE_EOF)
        for key, value in (("terminal_count", 0), ("source_eof", True), ("source_dropped", True)):
            with self.subTest(key=key):
                rust = rust_trace(RESULT_BEFORE_EOF)
                rust["steps"][-3]["observed"][key] = value
                self.assertEqual(compare_traces(upstream, rust, RESULT_BEFORE_EOF)["status"], "fail")
        rust = rust_trace(RESULT_BEFORE_EOF)
        del rust["steps"][-3]
        self.assertEqual(compare_traces(upstream, rust, RESULT_BEFORE_EOF)["status"], "fail")

    def test_result_timing_rejects_premature_or_inconsistent_outcomes(self):
        rust = rust_trace(RESULT_BEFORE_EOF)
        upstream = upstream_trace(RESULT_BEFORE_EOF)
        upstream["trace"][-4]["result_state"] = "ready"
        self.assertEqual(compare_traces(upstream, rust, RESULT_BEFORE_EOF)["status"], "fail")
        upstream = upstream_trace(RESULT_BEFORE_EOF)
        upstream["trace"][-3]["result"]["model"] = "wrong-model"
        self.assertEqual(compare_traces(upstream, rust, RESULT_BEFORE_EOF)["status"], "fail")
        rust["steps"][-3]["result"] = {"stop": "stop"}
        self.assertEqual(compare_traces(upstream_trace(RESULT_BEFORE_EOF), rust, RESULT_BEFORE_EOF)["status"], "fail")

    def test_public_terminal_and_span_timing_are_observed_independently(self):
        scenario = TERMINAL_AND_SPAN_BEFORE_EOF
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        result = compare_traces(upstream, rust, scenario)
        self.assertEqual(result["eof_checkpoint"], {
            "upstream": {"terminal_state": "ready", "span_settled": False},
            "rust": {"terminal_state": "pending", "span_settled": False},
        })
        self.assertEqual(result["difference"]["id"], EOF_CHECKPOINTS[scenario])
        self.assertIn("request_span_settled_after_source_eof", result["compared_fields"])
        upstream_at_gate = next(step for step in upstream["trace"] if step["step"] == EOF_CHECKPOINTS[scenario])
        upstream_at_gate.update(next_state="pending", terminal=None)
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "pass")
        upstream_at_gate["request_spans"][0].update(settled=True, stop="stop")
        result = compare_traces(upstream, rust, scenario)
        self.assertEqual(result["status"], "difference")
        self.assertTrue(result["difference"]["upstream"]["span_settled"])
        self.assertFalse(result["difference"]["rust"]["span_settled"])

    def test_terminal_timing_rejects_a_false_gate_or_incomplete_public_stream(self):
        scenario = TERMINAL_AND_SPAN_BEFORE_EOF
        upstream = upstream_trace(scenario)
        for name, field, value in (
            ("before-source-release", "next_state", "ready"),
            (EOF_CHECKPOINTS[scenario], "terminal", {"kind": "done"}),
            ("public-stream-completed", "outer_eof", False),
            ("result-observed", "matches_terminal", False),
        ):
            with self.subTest(checkpoint=name, field=field):
                rust = rust_trace(scenario)
                next(step for step in rust["steps"] if step["name"] == name)[field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        held = next(step for step in rust["steps"] if step["name"] == EOF_CHECKPOINTS[scenario])
        held["observed"]["source_eof"] = True
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_span_timing_requires_one_matching_span_with_observed_settlement(self):
        scenario = TERMINAL_AND_SPAN_BEFORE_EOF
        rust = rust_trace(scenario)
        for name, field, value in (
            ("before-source-release", "settled", True),
            (EOF_CHECKPOINTS[scenario], "model", "wrong-model"),
            (EOF_CHECKPOINTS[scenario], "settled", 0),
            ("public-stream-completed", "settled", False),
            ("public-stream-completed", "stop", "error"),
        ):
            with self.subTest(checkpoint=name, field=field):
                upstream = upstream_trace(scenario)
                next(step for step in upstream["trace"] if step["step"] == name)["request_spans"][0][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        held = next(step for step in upstream["trace"] if step["step"] == EOF_CHECKPOINTS[scenario])
        held["request_spans"].append(held["request_spans"][0].copy())
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_result_attribute_fault_preserves_host_diagnostics_and_span_state(self):
        scenario = RESULT_ATTRIBUTE_WRITE_FAILURE
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        result = compare_traces(upstream, rust, scenario)
        self.assertEqual(result["difference"], {
            "id": "result-attribute-write-failure",
            "upstream": {"span_settled": False, "span_stop": None, "host_errors": []},
            "rust": {"span_settled": True, "span_stop": None, "host_errors": rust["steps"][-1]["host_errors"]},
        })
        self.assertIn("source_eof_at_failed_write", result["compared_fields"])
        rust["steps"][-1]["host_errors"] = []
        result = compare_traces(upstream, rust, scenario)
        self.assertEqual(result["status"], "difference")
        self.assertFalse(result["difference"]["upstream"]["span_settled"])
        self.assertTrue(result["difference"]["rust"]["span_settled"])
        upstream["trace"][-1]["request_spans"][0]["settled"] = True
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "pass")

    def test_result_attribute_fault_rejects_wrong_fault_or_phase(self):
        scenario = RESULT_ATTRIBUTE_WRITE_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("fault_marker", "wrong-fault"), ("fault_delivery", "synchronous-throw"), ("write_attempts", [])):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        attempts = rust["steps"][-1]["write_attempts"]
        attempts.append(attempts[0].copy())
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        for field in ("source_eof", "source_dropped"):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1]["write_attempts"][0]["observed"][field] = False
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_result_attribute_fault_requires_business_span_and_host_observations(self):
        scenario = RESULT_ATTRIBUTE_WRITE_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("request_spans", []), ("span_finish_calls", 0), ("span_backend_dropped", None),
                             ("host_errors", [{"channel": "unhandledRejection", "message": "wrong-channel"}])):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-2]["matches_terminal"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_first_chunk_fault_preserves_business_source_span_and_diagnostic_differences(self):
        scenario = FIRST_CHUNK_WRITE_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["status"], "difference")
        self.assertIn("source_terminal_count_at_failed_write", result["compared_fields"])
        difference = result["difference"]
        self.assertEqual(difference["id"], "first-chunk-event-write-failure")
        keys = ("next_state_at_terminal_gate", "terminal_kind", "result_stop", "source_release",
                "source_eof", "span_settled_at_terminal_gate", "span_stop")
        self.assertEqual(tuple(difference["upstream"][key] for key in keys),
                         ("ready", "error", "error", "iterator-return", False, True, "error"))
        self.assertEqual(tuple(difference["rust"][key] for key in keys),
                         ("pending", "done", "stop", "eof", True, True, None))
        self.assertEqual(difference["upstream"]["public_error_message"], FIRST_CHUNK_WRITE_ERROR)
        self.assertEqual(difference["upstream"]["host_errors"], [])
        self.assertEqual(difference["rust"]["host_errors"][0]["fields"]["error"], FIRST_CHUNK_WRITE_ERROR)

    def test_first_chunk_fault_requires_actual_return_cleanup_and_rust_eof(self):
        scenario = FIRST_CHUNK_WRITE_FAILURE
        rust = rust_trace(scenario)
        for field, value in (("source_return_completions", 0), ("source_finally_calls", 0), ("source_result_calls", 1),
                             ("source_eof", True), ("terminal_count", 1)):
            with self.subTest(field=field):
                upstream = upstream_trace(scenario)
                upstream["trace"][1]["observed"][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust["steps"][2]["observed"]["source_dropped"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_first_chunk_fault_rejects_wrong_event_phase_and_unmatched_public_terminal(self):
        scenario = FIRST_CHUNK_WRITE_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("terminal_count", 1), ("source_eof", True), ("source_dropped", True)):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1]["write_attempts"][0]["observed"][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["write_attempts"][0]["event"] = "pi.ai.block"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        upstream["trace"][1]["terminal"]["error_message"] = "different-error"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        rust["steps"][3]["matches_terminal"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_histogram_creation_fault_preserves_call_dispatch_span_and_diagnostic_differences(self):
        scenario = HISTOGRAM_CREATION_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["status"], "difference")
        self.assertIn("dispatch_calls_at_fault", result["compared_fields"])
        difference = result["difference"]
        self.assertEqual(difference["id"], "request-duration-histogram-creation-failure")
        pi, rust = difference["upstream"], difference["rust"]
        self.assertEqual(pi["call_delivery"], "synchronous-throw")
        self.assertEqual(pi["public_error_message"], HISTOGRAM_CREATION_ERROR)
        self.assertEqual(pi["dispatch_calls"], 0)
        self.assertIsNone(pi["public_terminal"])
        self.assertIsNone(pi["request_span_at_fault"])
        self.assertIsNone(pi["request_span_at_end"])
        self.assertEqual(rust["call_delivery"], "returned-handle")
        self.assertEqual(rust["dispatch_calls"], 1)
        self.assertEqual(rust["public_terminal"]["kind"], "done")
        self.assertFalse(rust["request_span_at_fault"]["settled"])
        self.assertFalse(rust["request_span_at_end"]["settled"])
        self.assertEqual(rust["span_finish_calls"], 0)
        self.assertTrue(rust["span_backend_dropped_at_call_boundary"])
        self.assertEqual(rust["host_errors"][0]["fields"]["error"], HISTOGRAM_CREATION_ERROR)

    def test_histogram_creation_fault_rejects_wrong_instrument_phase_and_call_boundary(self):
        scenario = HISTOGRAM_CREATION_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("name", "pi.ai.first_chunk.duration"), ("kind", "counter"), ("unit", "s")):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1]["instrument_attempts"][0][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["instrument_attempts"][0]["observed"]["dispatch_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][0]["instrument_attempt_count"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream["trace"][0]["delivery"] = "returned-handle"
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_histogram_creation_fault_requires_span_state_and_successful_source_release(self):
        scenario = HISTOGRAM_CREATION_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("request_spans", []), ("span_finish_calls", 1), ("span_backend_dropped", 1)):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["observed"]["source_dropped"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-2]["matches_terminal"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_request_span_start_fault_retains_business_outcomes_and_both_pi_rejections(self):
        scenario = REQUEST_SPAN_START_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["status"], "difference")
        self.assertIn("created_instruments", result["compared_fields"])
        self.assertIn("call_delivery", result["compared_fields"])
        self.assertIn("request_span_count", result["compared_fields"])
        difference = result["difference"]
        self.assertEqual(difference["id"], "request-span-start-failure")
        pi, rust = difference["upstream"], difference["rust"]
        self.assertEqual((pi["fault_delivery"], pi["terminal_kind"], pi["source_release"]),
                         ("rejected-promise", "error", "iterator-return"))
        self.assertEqual(pi["public_error_message"], MISSING_SPAN_EVENT_ERROR)
        self.assertEqual([error["injected_start_rejection"] for error in pi["host_errors"]], [True, False])
        self.assertIn("setAttributes", pi["host_errors"][1]["message"])
        self.assertEqual((rust["fault_delivery"], rust["terminal_kind"], rust["source_release"]),
                         ("typed-err", "done", "eof"))
        self.assertEqual(rust["host_errors"][0]["fields"]["error"], REQUEST_SPAN_START_ERROR)

    def test_request_span_start_fault_requires_four_created_instruments_and_no_span(self):
        scenario = REQUEST_SPAN_START_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("created_instruments", REQUEST_INSTRUMENTS[:-1]), ("name", "pi.ai.poll"),
                             ("request_spans", [{"name": "pi.ai.request", "settled": False}])):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1]["span_start_attempts"][0][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["span_start_attempts"][0]["observed"]["dispatch_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][0]["span_start_attempt_count"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_request_span_start_fault_rejects_false_source_release_and_rejection_identity(self):
        scenario = REQUEST_SPAN_START_FAILURE
        rust = rust_trace(scenario)
        upstream = upstream_trace(scenario)
        upstream["trace"][1]["observed"]["source_return_completions"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["trace"][-1]["host_errors"][1]["injected_start_rejection"] = True
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["trace"][-2]["matches_terminal"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_duration_record_failure_keeps_public_success_and_retains_telemetry_outcomes(self):
        scenario = FIRST_CHUNK_DURATION_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["shared_observations"], "pass")
        self.assertIn("public_terminal", result["compared_fields"])
        self.assertIn("public_terminal_matches_result", result["compared_fields"])
        difference = result["difference"]
        self.assertEqual(difference["id"], "first-chunk-duration-record-failure")
        pi, rust = difference["upstream"], difference["rust"]
        self.assertEqual((pi["source_release"], pi["source_eof"], pi["next_state_at_source_exit"]),
                         ("iterator-return", False, "ready"))
        self.assertEqual((rust["source_release"], rust["source_eof"], rust["next_state_at_source_exit"]),
                         ("eof", True, "pending"))
        self.assertTrue(pi["request_span_at_end"]["settled"])
        self.assertTrue(rust["request_span_at_end"]["settled"])
        self.assertEqual(pi["request_span_at_end"]["stop"], "error")
        self.assertEqual(pi["request_span_at_end"]["status"]["error"]["message"], FIRST_CHUNK_DURATION_ERROR)
        self.assertIsNone(rust["request_span_at_end"]["stop"])
        self.assertEqual(rust["request_span_at_end"]["status"], {"status": "ok"})
        self.assertEqual(pi["duration_measurements"], [{"name": "pi.ai.request.duration", "stop": "error"}])
        self.assertEqual(rust["duration_measurements"], [])
        self.assertEqual(pi["host_errors"], [])
        self.assertEqual(rust["host_errors"][0]["fields"]["error"], FIRST_CHUNK_DURATION_ERROR)

    def test_duration_record_failure_requires_real_cleanup_and_a_record_fault_after_done(self):
        scenario = FIRST_CHUNK_DURATION_FAILURE
        upstream = upstream_trace(scenario)
        for field, value in (("name", "pi.ai.request.duration"), ("unit", "s"),
                             ("request_spans", [])):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-1]["record_attempts"][0][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["record_attempts"][0]["observed"]["terminal_count"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["record_attempts"][0]["request_spans"][0]["events"] = []
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][2]["eof_gate_reached"] = 1
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-2]["matches_terminal"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream["trace"][2]["observed"]["source_return_completions"] = 0
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_request_duration_fault_retains_open_span_and_public_success(self):
        scenario = REQUEST_DURATION_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["shared_observations"], "pass")
        for field in ("source_eof_at_failed_record", "result_attributes_written", "recorded_metrics",
                      "public_terminal_matches_result"):
            self.assertIn(field, result["compared_fields"])
        difference = result["difference"]
        self.assertEqual(difference["id"], "request-duration-record-failure")
        pi, rust = difference["upstream"], difference["rust"]
        self.assertFalse(pi["span_settled"])
        self.assertFalse(rust["span_settled"])
        self.assertEqual(pi["host_errors"], [])
        self.assertEqual(rust["host_errors"][0]["fields"]["error"], REQUEST_DURATION_ERROR)

    def test_request_duration_fault_requires_final_attributes_real_eof_and_skipped_metrics(self):
        scenario = REQUEST_DURATION_FAILURE
        upstream = upstream_trace(scenario)
        for key, value in (("name", "pi.ai.first_chunk.duration"), ("record_stop", "error"), ("usage_input", None)):
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                rust["steps"][-1]["record_attempts"][0][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        for key in ("source_eof", "source_dropped"):
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                rust["steps"][-1]["record_attempts"][0]["observed"][key] = False
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["metric_names"].append("pi.ai.tokens")
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream["trace"][-1]["record_attempts"][0]["observed"]["source_result_calls"] = 0
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_request_duration_fault_reports_span_settlement_when_finish_was_observed(self):
        scenario = REQUEST_DURATION_FAILURE
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        fault = rust["steps"][-1]
        fault["request_spans"][0]["settled"] = True
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        fault["span_finish_calls"] = 1
        result = compare_traces(upstream, rust, scenario)
        self.assertEqual(result["shared_observations"], "pass")
        self.assertTrue(result["difference"]["rust"]["span_settled"])

    def test_deferred_wait0_preserves_telemetry_timing_as_a_difference(self):
        upstream, rust = upstream_trace(DEFERRED_WAIT0), rust_trace(DEFERRED_WAIT0)
        comparison = compare_traces(upstream, rust, DEFERRED_WAIT0)
        difference = comparison["difference"]
        self.assertEqual(difference["id"], "deferred-poll-telemetry-at-result")
        self.assertEqual(comparison["compared_fields"], ["dispatches", "result", "spans", "measurements"])
        for name, rust_count in (("pending", 2), ("ready", 7)):
            with self.subTest(name=name):
                self.assertEqual(difference["upstream"][name], {
                    "span_settled": False, "metric_names": ["pi.ai.first_chunk.duration"],
                })
                self.assertTrue(difference["rust"][name]["span_settled"])
                self.assertEqual(len(difference["rust"][name]["metric_names"]), rust_count)

    def test_deferred_wait0_rejects_repeat_dispatch_or_changed_handle_and_wait(self):
        upstream = upstream_trace(DEFERRED_WAIT0)
        for field, value in (("wait_ms", 25), ("wait_ms", False), ("handle", {"id": "other"})):
            with self.subTest(field=field, value=value):
                rust = rust_trace(DEFERRED_WAIT0)
                rust["steps"][1]["dispatches"][0][field] = value
                self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")
        rust = rust_trace(DEFERRED_WAIT0)
        rust["steps"][0]["dispatches"] *= 2
        self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")
        rust = rust_trace(DEFERRED_WAIT0)
        rust["steps"][0]["result"]["deferred"] = None
        self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")

    def test_deferred_wait0_rejects_unscoped_or_wrongly_timed_measurements(self):
        upstream = upstream_trace(DEFERRED_WAIT0)
        rust = rust_trace(DEFERRED_WAIT0)
        rust["steps"][0]["measurements"].append(rust["steps"][1]["measurements"][-1])
        self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")
        rust = rust_trace(DEFERRED_WAIT0)
        rust["steps"][1]["measurements"][1]["context"] = "other"
        self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")
        rust = rust_trace(DEFERRED_WAIT0)
        rust["steps"][1]["metric_names_at_result"] = ["pi.ai.first_chunk.duration"]
        self.assertEqual(compare_traces(upstream, rust, DEFERRED_WAIT0)["status"], "fail")

    def test_deferred_eof_gate_compares_result_readiness_and_poll_state(self):
        scenario = DEFERRED_RESULT_BEFORE_EOF
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        comparison = compare_traces(upstream, rust, scenario)
        self.assertEqual(comparison["shared_observations"], "pass")
        self.assertIn("spans_while_eof_held", comparison["compared_fields"])
        self.assertIn("measurements_while_eof_held", comparison["compared_fields"])
        self.assertEqual(comparison["difference"], {
            "id": "deferred-result-before-source-eof",
            "upstream": {"result_state": "ready", "result": upstream["trace"][-1]["result"]},
            "rust": {"result_state": "pending", "result": None},
        })
        rust["steps"][1]["result"] = rust["steps"][-1]["result"]
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "pass")

    def test_deferred_eof_gate_requires_the_actual_terminal_and_source_cleanup(self):
        scenario = DEFERRED_RESULT_BEFORE_EOF
        upstream = upstream_trace(scenario)
        for index, key, value in ((1, "terminal_count", 0), (1, "source_eof", True),
                                  (1, "source_dropped", True), (2, "source_dropped", False)):
            with self.subTest(key=key, value=value):
                rust = rust_trace(scenario)
                rust["steps"][index]["observed"][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream["trace"][1]["observed"]["source_result_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_deferred_readiness_pair_requires_both_results_at_the_held_eof(self):
        scenario = DEFERRED_READY_BEFORE_EOF
        for upstream_pending in (True, False):
            with self.subTest(upstream_pending=upstream_pending):
                upstream, rust = upstream_trace(scenario), rust_trace(scenario)
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "pass")
                steps = upstream["trace"] if upstream_pending else rust["steps"]
                steps[1]["result"] = None
                comparison = compare_traces(upstream, rust, scenario)
                self.assertEqual(comparison["status"], "fail")
                self.assertIn("both readiness results", comparison["invalid_trace"])

    def test_deferred_eof_gate_rejects_early_settlement_or_a_different_result(self):
        scenario = DEFERRED_RESULT_BEFORE_EOF
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        rust["steps"][1]["spans"][0]["settled"] = True
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream["trace"][1]["result"]["content"][0]["text"] = "other"
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_deferred_local_cancel_compares_cleanup_and_preserves_refetch(self):
        scenario = DEFERRED_LOCAL_CANCEL_AFTER_DONE
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["shared_observations"], "pass")
        self.assertIn("remote_cancel_calls", comparison["compared_fields"])
        self.assertIn("refetch_result", comparison["compared_fields"])
        self.assertEqual(comparison["difference"], {
            "id": "deferred-local-cancel-after-done",
            "upstream": {"result_before_cancel": "ready", "source_released_at_cancel_checkpoint": False,
                         "poll_settled_at_cancel_checkpoint": False, "natural_source_eof_at_cleanup": True},
            "rust": {"result_before_cancel": "pending", "source_released_at_cancel_checkpoint": True,
                     "poll_settled_at_cancel_checkpoint": True, "natural_source_eof_at_cleanup": False},
        })

    def test_deferred_local_cancel_requires_propagation_drop_and_settlement(self):
        scenario = DEFERRED_LOCAL_CANCEL_AFTER_DONE
        upstream = upstream_trace(scenario)
        rust = rust_trace(scenario)
        rust["steps"][2]["backend_cancelled"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        for field, value in (("source_dropped", False), ("source_eof", True)):
            with self.subTest(field=field, value=value):
                rust = rust_trace(scenario)
                rust["steps"][2]["observed"][field] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][2]["spans"][0]["settled"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_local_cancel_rejects_remote_cancel_or_changed_ready_result(self):
        scenario = DEFERRED_LOCAL_CANCEL_AFTER_DONE
        upstream = upstream_trace(scenario)
        rust = rust_trace(scenario)
        rust["steps"][-1]["remote_cancel_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][2]["result"]["stop"] = "aborted"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["dispatches"][0]["handle"]["id"] = "other"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_cancel_before_done_compares_abort_payload_and_refetch(self):
        scenario = DEFERRED_LOCAL_CANCEL_BEFORE_DONE
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["shared_observations"], "pass")
        self.assertIn("source_done_count", comparison["compared_fields"])
        self.assertIn("refetch_result", comparison["compared_fields"])
        self.assertEqual(comparison["difference"], {
            "id": "deferred-local-cancel-before-done",
            "upstream": {"result_delivery": "fulfilled-message", "typed_cause": None,
                         "error_message": "Request was aborted", "poll_settled_at_result": False,
                         "natural_source_eof_at_cleanup": True},
            "rust": {"result_delivery": "typed-err", "typed_cause": "Cancelled",
                     "error_message": "cancelled", "poll_settled_at_result": True,
                     "natural_source_eof_at_cleanup": False},
        })

    def test_deferred_cancel_before_done_requires_the_abort_terminal_and_typed_cause(self):
        scenario = DEFERRED_LOCAL_CANCEL_BEFORE_DONE
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        upstream["trace"][1]["observed"]["source_aborted_terminals"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        for key, value in (("typed_cause", "Timeout"), ("backend_cancelled", False)):
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                rust["steps"][1][key] = value
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][1]["observed"]["terminal_count"] = 1
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_deferred_cancel_before_done_checks_source_release_poll_and_refetch(self):
        scenario = DEFERRED_LOCAL_CANCEL_BEFORE_DONE
        upstream, rust = upstream_trace(scenario), rust_trace(scenario)
        upstream["trace"][2]["observed"]["source_result_calls"] = 0
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        rust["steps"][1]["observed"]["source_dropped"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][1]["spans"][0]["settled"] = False
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["remote_cancel_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["result"]["content"][0]["text"] = "other"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_remote_cancel_compares_refetch_and_first_chunk_with_different_failure_delivery(self):
        scenario = DEFERRED_REMOTE_CANCEL
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["shared_observations"], "pass")
        self.assertIn("cancelled_handles", result["compared_fields"])
        self.assertIn("refetch_result", result["compared_fields"])
        self.assertIn("refetch_span", result["compared_fields"])
        self.assertIn("refetch_measurements", result["compared_fields"])
        diff = result["difference"]
        self.assertEqual(diff["id"], "deferred-refetch-after-remote-cancel")
        self.assertEqual(diff["upstream"]["result_delivery"], "fulfilled-message")
        self.assertEqual(diff["rust"]["result_delivery"], "typed-err")
        self.assertEqual(diff["rust"]["typed_cause"], "Config")

    def test_remote_cancel_rejects_the_pre_repair_refetch_failure_identity(self):
        scenario = DEFERRED_REMOTE_CANCEL
        baseline = frozen_case("phase5-deferred-remote-cancel-before-identity-repair-differential.json", scenario)
        rust = rust_trace(scenario)
        rust["steps"][-1]["result"] = baseline["rust_run"]["trace"]["steps"][-1]["result"]
        result = compare_traces(upstream_trace(scenario), rust, scenario)
        self.assertEqual(result, baseline["comparison"])
        self.assertEqual(result["status"], "fail")
        mismatch, = result["mismatches"]
        self.assertEqual(mismatch["field"], "refetch_result")
        self.assertEqual((mismatch["rust"]["provider"], mismatch["rust"]["model"]), ("", ""))

    def test_remote_cancel_requires_one_cancel_without_an_sdk_span_and_the_same_refetch_handle(self):
        scenario = DEFERRED_REMOTE_CANCEL
        upstream = upstream_trace(scenario)
        original = rust_trace(scenario)["steps"][0]
        for key, value in (("fetch_count", 2), ("telemetry_forwarded", False),
                           ("cancelled_handles", []), ("spans", original["spans"])):
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                rust["steps"][1][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-1]["dispatches"][0]["handle"]["id"] = "other"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_auth_failure_compares_gate_and_first_chunk_with_typed_auth_failure(self):
        scenario = DEFERRED_AUTH_SETUP_FAILURE
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(result["shared_observations"], "pass")
        self.assertTrue({"auth_at_gate", "dispatches", "fetch_result", "fetch_span", "fetch_measurements"}
                        <= set(result["compared_fields"]))
        diff = result["difference"]
        self.assertEqual(diff["id"], "deferred-auth-setup-failure")
        self.assertEqual(diff["upstream"]["result_delivery"], "fulfilled-message")
        self.assertEqual(diff["rust"]["result_delivery"], "typed-err")
        self.assertEqual(diff["rust"]["typed_cause"], "AuthFailed")

    def test_deferred_auth_failure_requires_the_gate_and_no_backend_calls(self):
        scenario = DEFERRED_AUTH_SETUP_FAILURE
        upstream = upstream_trace(scenario)
        for step, key, value in ((0, "auth_calls", 0), (0, "wait_ms", 25), (0, "result_state", "fulfilled"),
                                 (1, "dispatches", [{}]), (1, "remote_cancel_calls", 1), (1, "typed_cause", "Config")):
            with self.subTest(step=step, key=key):
                rust = rust_trace(scenario)
                rust["steps"][step][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_failures_require_first_chunk_event_and_duration(self):
        baselines = {
            DEFERRED_REMOTE_CANCEL: "phase5-deferred-remote-cancel-then-fetch-differential.json",
            DEFERRED_AUTH_SETUP_FAILURE: "phase5-deferred-auth-setup-failure-baseline-differential.json",
        }
        for scenario, file in baselines.items():
            upstream = upstream_trace(scenario)
            with self.subTest(scenario=scenario):
                baseline = frozen_case(file, scenario)
                comparison = compare_traces(upstream, baseline["rust_run"]["trace"], scenario)
                self.assertEqual(comparison["status"], "fail")
                self.assertIn("expected one settled pi.ai.poll span", comparison["invalid_trace"])
                rust = rust_trace(scenario)
                del rust["steps"][-1]["measurements"][0]
                comparison = compare_traces(upstream, rust, scenario)
                self.assertEqual(comparison["status"], "fail")
                self.assertIn("unexpected deferred poll duration", comparison["invalid_trace"])

    def test_deferred_positive_wait_requires_one_dispatch_of_the_same_handle_with_exact_wait(self):
        scenario = DEFERRED_POSITIVE_WAIT
        upstream = upstream_trace(scenario)
        for index, wait in ((0, 0), (1, 25), (2, True), (3, 51)):
            with self.subTest(step=index, wait=wait):
                rust = rust_trace(scenario)
                rust["steps"][index]["dispatches"][0]["wait_ms"] = wait
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][1]["dispatches"].append(deepcopy(rust["steps"][1]["dispatches"][0]))
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][2]["dispatches"][0]["handle"]["id"] = "other"
        self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_positive_wait_requires_pending_at_both_gates_and_the_provider_outcomes(self):
        scenario = DEFERRED_POSITIVE_WAIT
        upstream = upstream_trace(scenario)
        ready = rust_trace(scenario)["steps"][-1]
        for index, key, value in ((0, "result_state", "fulfilled"), (1, "result", ready["result"]),
                                  (2, "measurements", ready["measurements"]), (2, "spans", ready["spans"]),
                                  (3, "remote_cancel_calls", 1)):
            with self.subTest(step=index, key=key):
                rust = rust_trace(scenario)
                rust["steps"][index][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_handle_expiry_preserves_the_pre_dispatch_difference(self):
        scenario = DEFERRED_HANDLE_EXPIRY
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        difference = comparison["difference"]
        self.assertEqual(difference["id"], "deferred-handle-expiry")
        pi, rust = difference["upstream"], difference["rust"]
        self.assertEqual((pi["auth_calls"], pi["fetch_count"], pi["result"]["stop"]), (1, 1, "stop"))
        self.assertEqual((rust["auth_calls"], rust["fetch_count"], rust["result"]["stop"]), (0, 0, "error"))
        self.assertEqual(rust["typed_cause"], "Config")

    def test_deferred_handle_expiry_requires_the_clock_and_no_side_effects_on_rejection(self):
        scenario = DEFERRED_HANDLE_EXPIRY
        upstream = upstream_trace(scenario)
        before, expired, _ = rust_trace(scenario)["steps"]
        for index, key, value in (
            (0, "sdk_now_ms", expired["sdk_now_ms"]), (1, "sdk_now_ms", None),
            (2, "sdk_now_ms", expired["sdk_now_ms"]),
            (2, "request_handle", before["request_handle"] | {"expiresAt": 1_000}),
            (1, "auth_calls", 1), (1, "fetch_count", 1), (1, "dispatches", before["dispatches"]),
            (1, "remote_cancel_calls", 1), (1, "result", before["result"]),
            (1, "typed_cause", "AuthFailed"), (1, "measurements", expired["measurements"][1:]),
        ):
            with self.subTest(step=index, key=key):
                rust = rust_trace(scenario)
                rust["steps"][index][key] = value
                self.assertEqual(compare_traces(upstream, rust, scenario)["status"], "fail")

    def test_deferred_route_mismatch_preserves_rejection_stage_and_route(self):
        scenario = DEFERRED_HANDLE_ROUTE_MISMATCH
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        diff = comparison["difference"]
        self.assertEqual(diff["id"], "deferred-handle-route-mismatch")
        for name, expected in (("provider-mismatch", "Unsupported"), ("model-mismatch", "RouteMismatch"),
                               ("api-mismatch", "Unsupported")):
            with self.subTest(name=name):
                pi, rust = diff["upstream"][name], diff["rust"][name]
                self.assertEqual((pi["auth_calls"], pi["fetch_count"], pi["result_delivery"]), (1, 1, "fulfilled-message"))
                self.assertEqual((rust["auth_calls"], rust["fetch_count"], rust["result_delivery"]), (0, 0, "typed-err"))
                self.assertEqual((rust["handle_validation"], rust["typed_cause"]), (expected, "Config"))
                self.assertNotEqual(pi["poll_route"], rust["poll_route"])

    def test_deferred_route_mismatch_requires_exact_handle_dispatch_and_original_refetch(self):
        scenario = DEFERRED_HANDLE_ROUTE_MISMATCH
        pi = upstream_trace(scenario)
        rust = rust_trace(scenario)
        original = rust["steps"][-1]
        for side, index, key, value in (
            ("pi", 0, "auth_calls", 0), ("pi", 0, "dispatches", original["dispatches"]),
            ("rust", 0, "auth_calls", 1), ("rust", 1, "handle_validation", "Unsupported"),
            ("rust", 2, "fetch_count", 1), ("rust", 2, "dispatches", original["dispatches"]),
            ("rust", 2, "request_handle", original["request_handle"] | {"api": "other-api", "id": "unknown"}),
            ("rust", 3, "result", rust["steps"][0]["result"]),
        ):
            with self.subTest(side=side, step=index, key=key):
                bad_pi, bad_rust = deepcopy(pi), deepcopy(rust)
                steps = bad_pi["trace"] if side == "pi" else bad_rust["steps"]
                steps[index][key] = value
                self.assertEqual(compare_traces(bad_pi, bad_rust, scenario)["status"], "fail")
        bad_rust = deepcopy(rust)
        bad_rust["steps"][2]["spans"][0]["attributes"]["api"] = original["request_handle"]["api"]
        self.assertEqual(compare_traces(pi, bad_rust, scenario)["status"], "fail")
        rust["steps"].pop()
        self.assertEqual(compare_traces(pi, rust, scenario)["status"], "fail")

    def test_deferred_route_mismatch_rejects_provider_only_api_diagnostic(self):
        scenario = DEFERRED_HANDLE_ROUTE_MISMATCH
        baseline = frozen_case("phase5-deferred-handle-route-mismatch-differential.json", scenario)
        rust = rust_trace(scenario)
        rust["steps"][2] = baseline["rust_run"]["trace"]["steps"][2]
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario), {
            "status": "fail",
            "invalid_trace": "api-mismatch: unexpected validation, fetch delivery, cause or diagnostic",
        })

    def test_deferred_unknown_job_id_preserves_backend_failure_delivery(self):
        scenario = DEFERRED_UNKNOWN_JOB_ID
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        self.assertEqual(comparison["difference"], {
            "id": "deferred-unknown-job-id",
            "upstream": {"result_delivery": "fulfilled-message", "typed_cause": None,
                         "error_message": "Unknown faux deferred response: deferred-missing"},
            "rust": {"result_delivery": "typed-err", "typed_cause": "Config",
                     "error_message": "invalid provider configuration: Unknown scripted deferred response: deferred-missing"},
        })

    def test_deferred_unknown_job_id_requires_exact_dispatch_and_original_refetch(self):
        scenario = DEFERRED_UNKNOWN_JOB_ID
        pi = upstream_trace(scenario)
        failed, ready = rust_trace(scenario)["steps"]
        for index, key, value in (
            (0, "handle_validation", "Unsupported"), (0, "auth_calls", 0), (0, "fetch_count", 0),
            (0, "dispatches", ready["dispatches"]), (0, "remote_cancel_calls", 1),
            (0, "result", failed["result"] | {"provider": ""}),
            (0, "measurements", failed["measurements"][1:]), (1, "result", failed["result"]),
        ):
            with self.subTest(step=index, key=key):
                rust = rust_trace(scenario)
                rust["steps"][index][key] = value
                self.assertEqual(compare_traces(pi, rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"].pop()
        self.assertEqual(compare_traces(pi, rust, scenario)["status"], "fail")

    def test_deferred_stream_source_cleanup_preserves_each_completion_boundary(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        difference = comparison["difference"]
        self.assertEqual(difference["id"], "deferred-stream-source-cleanup")
        self.assertEqual(difference["upstream"]["public-eof-observed"], {
            "eof_gate_reached": True, "source_eof": False, "poll_settled": False, "source_finally_calls": 0,
        })
        rust = difference["rust"]
        self.assertEqual(rust["public-eof-observed"], rust["source-released"])
        held = {"eof_gate_reached": True, "source_eof": False, "poll_settled": False, "source_dropped": False}
        self.assertEqual(rust["next-eof-gate-held"], held)
        for name in ("started-done-observed", "started-next-eof-gate-held",
                     "consumer-dropped-eof-gate-held", "cached-terminal-eof-gate-held"):
            self.assertEqual(rust[name], held)
        for name in ("started-public-eof-observed", "started-result-observed",
                     "consumer-dropped-source-released", "cached-terminal-result-observed"):
            self.assertEqual(rust[name], rust["source-released"])

    def test_direct_deferred_consumer_stop_exposes_source_and_poll_lifetimes(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        pi, rust = (comparison["difference"][side] for side in ("upstream", "rust"))
        for stop_after in ("done", "public-eof"):
            prefix = f"stop-after-{stop_after}"
            self.assertEqual(pi[f"{prefix}-consumer-stopped"], {
                "eof_gate_reached": True, "source_eof": False, "poll_settled": False, "source_finally_calls": 0,
            })
            self.assertEqual(pi[f"{prefix}-after-eof-release"], pi["source-released"])
        stopped = {
            "eof_gate_reached": False, "source_eof": False, "source_dropped": True, "poll_settled": False,
        }
        self.assertEqual(rust["stop-after-done-consumer-stopped"], stopped)
        self.assertEqual(rust["stop-after-done-after-eof-release"], stopped)
        self.assertEqual(rust["stop-after-public-eof-next-eof-gate-held"], rust["next-eof-gate-held"])
        self.assertEqual(rust["stop-after-public-eof-consumer-stopped"], rust["source-released"])

    def test_direct_deferred_drop_requires_one_finish_after_release(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        pi = upstream_trace(scenario)
        case = frozen_case("phase5-deferred-direct-drop-poll-differential.json", scenario)
        rust = case["rust_run"]["trace"]
        comparison = compare_traces(pi, rust, scenario)
        self.assertEqual(comparison, case["comparison"])
        self.assertEqual(comparison["shared_observations"], "pass")
        self.assertEqual(comparison["status"], "difference")

        stopped = "stop-after-pending-tail-consumer-stopped"
        full_stop = "stop-after-public-eof-consumer-stopped"
        held = "stop-after-pending-tail-next-eof-gate-held"
        steps = {row["name"]: row for row in rust["steps"]}
        for name, update in (
            (stopped, {"poll_finish_source_dropped": []}),
            (stopped, {"poll_finish_source_dropped": [False]}),
            (stopped, {"remote_cancel_calls": 1}),
            (stopped, {"measurements": steps[stopped]["measurements"] + steps[stopped]["measurements"][1:]}),
            (full_stop, {"poll_finish_source_dropped": [True, True]}),
            (held, {"poll_finish_source_dropped": [True]}),
        ):
            with self.subTest(step=name, update=update):
                changed = rust | {"steps": [row | update if row["name"] == name else row for row in rust["steps"]]}
                self.assertEqual(compare_traces(pi, changed, scenario)["status"], "fail")

    def test_direct_deferred_cancellation_ends_the_eof_wait_with_the_cached_result(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        pi, rust = upstream_trace(scenario), rust_trace(scenario)
        comparison = compare_traces(pi, rust, scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        state = comparison["difference"]["rust"]
        self.assertEqual(state["cancel-after-done-next-eof-gate-held"], state["next-eof-gate-held"] | {
            "local_cancelled": False,
        })
        self.assertEqual(state["cancel-after-done-public-eof-observed"], {
            "eof_gate_reached": True, "source_eof": False, "source_dropped": True, "poll_settled": True,
            "local_cancelled": True,
        })
        steps = {step["name"]: step for step in rust["steps"]}
        self.assertEqual(steps["cancel-after-done-source-released"]["delivery"], steps["source-released"]["delivery"])

    def test_deferred_stream_source_cleanup_requires_tail_and_settlement_evidence(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        pi, rust = upstream_trace(scenario), rust_trace(scenario)
        pi_steps = {step["step"]: step for step in pi["trace"]}
        rust_steps = {step["name"]: step for step in rust["steps"]}
        for side, name, update in (
            ("pi", "public-eof-observed", {
                key: pi_steps["source-released"][key] for key in ("spans", "measurements")
            }),
            ("rust", "public-eof-observed", {"eof_gate_reached": False}),
            ("rust", "next-eof-gate-held", {"delivery": rust_steps["public-eof-observed"]["delivery"]}),
            ("rust", "next-eof-gate-held", {
                key: rust_steps["source-released"][key] for key in ("spans", "measurements")
            }),
            ("rust", "started-next-eof-gate-held", {"delivery": rust_steps["started-public-eof-observed"]["delivery"]}),
            ("rust", "started-done-observed", {
                key: rust_steps["source-released"][key] for key in ("observed", "spans", "measurements")
            }),
            ("rust", "started-result-observed", {"remote_cancel_calls": 1}),
            ("rust", "consumer-dropped-source-released", {"observed": rust_steps["consumer-dropped-eof-gate-held"]["observed"]}),
            ("rust", "consumer-dropped-source-released", {"auth_calls": 2}),
            ("rust", "cached-terminal-eof-gate-held", {
                key: rust_steps["source-released"][key] for key in ("spans", "measurements")
            }),
            ("rust", "cached-terminal-public-eof-observed", {"delivery": rust_steps["started-done-observed"]["delivery"]}),
            ("pi", "stop-after-done-after-eof-release", {
                "observed": pi_steps["stop-after-done-after-eof-release"]["observed"] | {"source_result_calls": 0}
            }),
            ("pi", "stop-after-public-eof-consumer-stopped", {"auth_calls": 2}),
            ("rust", "stop-after-done-consumer-stopped", {
                key: rust_steps["source-released"][key] for key in ("spans", "measurements")
            }),
            ("rust", "stop-after-public-eof-consumer-stopped", {
                "observed": rust_steps["stop-after-public-eof-consumer-stopped"]["observed"] | {"source_eof": False}
            }),
            ("rust", "cancel-after-done-public-eof-observed", {"backend_cancelled": False}),
            ("rust", "cancel-after-done-public-eof-observed", {
                "observed": rust_steps["cancel-after-done-public-eof-observed"]["observed"] | {"source_eof": True}
            }),
        ):
            with self.subTest(side=side, step=name):
                bad_pi, bad_rust = deepcopy(pi), deepcopy(rust)
                rows, key = (bad_pi["trace"], "step") if side == "pi" else (bad_rust["steps"], "name")
                next(step for step in rows if step[key] == name).update(update)
                self.assertEqual(compare_traces(bad_pi, bad_rust, scenario)["status"], "fail")

    def test_deferred_tail_resumption_preserves_the_held_source(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        comparison = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "difference", comparison)
        self.assertEqual(comparison["shared_observations"], "pass")
        state = comparison["difference"]["rust"]
        held = state["next-eof-gate-held"] | {"local_cancelled": False}
        released = state["source-released"] | {"local_cancelled": False}
        for resume in ("result", "started", "detached"):
            with self.subTest(resume=resume):
                self.assertEqual(state[f"resume-{resume}-next-dropped-eof-gate-held"], held)
                self.assertEqual(state[f"resume-{resume}-resumed-eof-gate-held"], held)
                self.assertEqual(state[f"resume-{resume}-source-released"], released)
        self.assertEqual(state["resume-started-public-eof-observed"], released)

    def test_deferred_tail_resumption_requires_repoll_and_completion_evidence(self):
        scenario = DEFERRED_STREAM_SOURCE_CLEANUP
        pi, rust = upstream_trace(scenario), rust_trace(scenario)
        steps = {step["name"]: step for step in rust["steps"]}
        detached_repoll = "resume-detached-resumed-eof-gate-held"
        without_repoll = rust | {"steps": [step for step in rust["steps"] if step["name"] != detached_repoll]}
        self.assertEqual(compare_traces(pi, without_repoll, scenario)["status"], "fail")
        for name, update in (
            ("resume-result-resumed-eof-gate-held", {
                key: steps["source-released"][key] for key in ("spans", "measurements")
            }),
            ("resume-started-public-eof-observed", {"delivery": steps["done-observed"]["delivery"]}),
            ("resume-result-source-released", {"dispatches": steps["source-released"]["dispatches"] * 2}),
            ("resume-started-source-released", {"backend_cancelled": True}),
            ("resume-detached-source-released", {
                "observed": steps["resume-detached-source-released"]["observed"] | {"source_dropped": False}
            }),
        ):
            with self.subTest(step=name):
                changed = rust | {"steps": [step | update if step["name"] == name else step for step in rust["steps"]]}
                self.assertEqual(compare_traces(pi, changed, scenario)["status"], "fail")

    def test_unexpected_terminal_mismatch_fails(self):
        scenario = "models-stop-consumption"
        rust = rust_trace(scenario)
        rust["steps"][-1]["stop"] = "error"
        result = compare_traces(upstream_trace(scenario), rust, scenario)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["mismatches"][0]["field"], "terminal_kind")

    def test_missing_or_wrong_type_compared_fields_fail_closed(self):
        scenario = "models-no-consumption"
        for key in rust_trace(scenario)["steps"][-2]["observed"]:
            with self.subTest(key=key):
                rust = rust_trace(scenario)
                del rust["steps"][-2]["observed"][key]
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-2]["observed"]["terminal_count"] = True
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_early_completion_and_unobserved_cleanup_are_rejected(self):
        scenario = "models-stop-consumption"
        for field, value in (("terminal_count", 1), ("source_eof", True), ("source_dropped", True)):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][-3]["observed"][field] = value
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["steps"][-2]["observed"]["source_eof"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["runtimeDetails"]["source_dropped"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_missing_stop_checkpoint_or_result_observation_fails(self):
        scenario = "models-stop-consumption"
        rust = rust_trace(scenario)
        del rust["steps"][2]
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        rust = rust_trace(scenario)
        rust["runtimeDetails"]["result_observed"] = False
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["trace"][-1]["matches_source"] = False
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_wrong_identity_and_unknown_scenarios_fail(self):
        scenario = "models-no-consumption"
        rust = rust_trace(scenario)
        rust["schemaVersion"] = 1
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        self.assertEqual(compare_traces({}, {}, "unknown")["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["sourceAssertions"] = "failed"
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_completion_rejects_wrong_dispatch_stage_or_source_return(self):
        scenario = "models-stop-consumption"
        for name, field, value in (
            ("constructed", "dispatch_calls", 1),
            ("source-at-gate", "dispatch_calls", 0),
            ("source-released", "dispatch_calls", 2),
        ):
            with self.subTest(name=name, field=field):
                upstream = upstream_trace(scenario)
                next(step for step in upstream["trace"] if step["step"] == name)["observed"][field] = value
                self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")
                rust = rust_trace(scenario)
                next(step for step in rust["steps"] if step["name"] == name)["observed"][field] = value
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["trace"][2]["observed"]["source_return_calls"] = 1
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_raw_lazy_completion_cannot_supply_public_dispatch_observations(self):
        scenario = "models-no-consumption"
        raw = next(case["upstream_trace"] for case in HISTORICAL["cases"] if case["scenario"] == "lazy-no-consumption")
        comparison = compare_traces({**raw, "id": scenario}, rust_trace(scenario), scenario)
        self.assertEqual(comparison["status"], "fail")
        self.assertIn("public completion checkpoints", comparison["invalid_trace"])

    def test_setup_error_details_are_retained_not_normalized_away(self):
        scenario = "models-auth-setup-failure"
        result = compare_traces(upstream_trace(scenario), rust_trace(scenario), scenario)
        difference = result["difference"]
        self.assertEqual(difference["phase"], "authentication")
        self.assertEqual(difference["upstream"]["auth_calls_at_construction"], 1)
        self.assertEqual(difference["rust"]["auth_calls_at_construction"], 0)
        self.assertEqual(difference["upstream"]["result_delivery"], "fulfilled-message")
        self.assertEqual(difference["rust"]["result_delivery"], "typed-err")
        self.assertIn("message_provider", result["compared_fields"])
        self.assertIn("message_model", result["compared_fields"])
        self.assertEqual(difference["rust"]["typed_cause"], "AuthFailed")

    def test_setup_rejects_the_wrong_fault_dispatch_or_terminal(self):
        scenario = "models-auth-setup-failure"
        for name, field, value in (
            ("constructed", "phase", "route-lookup"),
            ("auth-at-gate", "auth_calls", 0),
            ("failure-event", "error_message", "Unknown provider: missing-provider"),
            ("failure-event", "cause", "Config"),
            ("event-eof", "dispatch_calls", 1),
            ("event-eof", "done", False),
            ("result-observed", "matches_event", False),
            ("result-observed", "delivery", "fulfilled-message"),
        ):
            with self.subTest(name=name, field=field):
                rust = rust_trace(scenario)
                next(step for step in rust["steps"] if step["name"] == name)[field] = value
                self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")

    def test_setup_requires_typed_rust_cause_and_pi_payload_identity(self):
        scenario = "models-missing-provider"
        rust = rust_trace(scenario)
        del rust["steps"][1]["cause"]
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        upstream = upstream_trace(scenario)
        upstream["trace"][1]["message_provider"] = "unrelated-provider"
        self.assertEqual(compare_traces(upstream, rust_trace(scenario), scenario)["status"], "fail")

    def test_setup_rejects_rust_payload_identity_drift(self):
        scenario = "models-missing-provider"
        for field, expected, actual in (
            ("message_provider", "missing-provider", ""),
            ("message_model", "mock-model", "unrelated-model"),
        ):
            with self.subTest(field=field):
                rust = rust_trace(scenario)
                rust["steps"][1][field] = actual
                result = compare_traces(upstream_trace(scenario), rust, scenario)
                self.assertEqual(result["status"], "fail")
                self.assertEqual(result["mismatches"], [
                    {"field": field, "upstream": expected, "rust": actual},
                ])

    def test_setup_requires_the_gate_and_preserves_raw_lazy_probe_as_separate_evidence(self):
        scenario = "models-auth-setup-failure"
        rust = rust_trace(scenario)
        del rust["steps"][1]
        self.assertEqual(compare_traces(upstream_trace(scenario), rust, scenario)["status"], "fail")
        raw_lazy = next(case["upstream_trace"] for case in HISTORICAL["cases"] if case["scenario"] == "lazy-setup-failure")
        scenario = "models-missing-provider"
        self.assertEqual(compare_traces(raw_lazy, rust_trace(scenario), scenario)["status"], "fail")


if __name__ == "__main__":
    unittest.main()

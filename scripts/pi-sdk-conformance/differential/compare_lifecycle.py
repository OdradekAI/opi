from __future__ import annotations

from typing import Any


SETUP_CASES = {
    "models-missing-provider": ("route-lookup", "missing-provider", "unknown provider: missing-provider", 0),
    "models-auth-setup-failure": ("authentication", "lifecycle-differential", "controlled-auth-setup-failure", 1),
}
RESULT_BEFORE_EOF = "models-result-before-source-eof"
TERMINAL_AND_SPAN_BEFORE_EOF = "models-terminal-and-span-before-source-eof"
RESULT_ATTRIBUTE_WRITE_FAILURE = "models-result-attribute-write-failure"
RESULT_ATTRIBUTE_WRITE_ERROR = "controlled-result-attribute-write-failure"
FIRST_CHUNK_WRITE_FAILURE = "models-first-chunk-event-write-failure"
FIRST_CHUNK_WRITE_ERROR = "controlled-first-chunk-event-write-failure"
HISTOGRAM_CREATION_FAILURE = "models-request-duration-histogram-creation-failure"
HISTOGRAM_CREATION_ERROR = "controlled-request-duration-histogram-creation-failure"
REQUEST_SPAN_START_FAILURE = "models-request-span-start-failure"
REQUEST_SPAN_START_ERROR = "controlled-request-span-start-failure"
FIRST_CHUNK_DURATION_FAILURE = "models-first-chunk-duration-record-failure"
FIRST_CHUNK_DURATION_ERROR = "controlled-first-chunk-duration-record-failure"
REQUEST_DURATION_FAILURE = "models-request-duration-record-failure"
REQUEST_DURATION_ERROR = "controlled-request-duration-record-failure"
DEFERRED_WAIT0 = "models-deferred-wait0-pending-then-ready"
DEFERRED_RESULT_BEFORE_EOF = "models-deferred-result-before-source-eof"
DEFERRED_READY_BEFORE_EOF = "models-deferred-ready-before-source-eof"
DEFERRED_LOCAL_CANCEL_AFTER_DONE = "models-deferred-local-cancel-after-done"
DEFERRED_LOCAL_CANCEL_BEFORE_DONE = "models-deferred-local-cancel-before-done"
DEFERRED_REMOTE_CANCEL = "models-deferred-remote-cancel-then-fetch"
DEFERRED_AUTH_SETUP_FAILURE = "models-deferred-auth-setup-failure"
DEFERRED_POSITIVE_WAIT = "models-deferred-positive-wait"
DEFERRED_HANDLE_EXPIRY = "models-deferred-handle-expiry"
DEFERRED_HANDLE_ROUTE_MISMATCH = "models-deferred-handle-route-mismatch"
DEFERRED_UNKNOWN_JOB_ID = "models-deferred-unknown-job-id"
DEFERRED_STREAM_SOURCE_CLEANUP = "models-deferred-stream-source-cleanup"
MISSING_SPAN_EVENT_ERROR = "Cannot read properties of undefined (reading 'addEvent')"
REQUEST_INSTRUMENTS = [
    {"name": "pi.ai.request.duration", "kind": "histogram", "unit": "ms"},
    {"name": "pi.ai.first_chunk.duration", "kind": "histogram", "unit": "ms"},
    {"name": "pi.ai.tokens", "kind": "counter", "unit": "token"},
    {"name": "pi.ai.cost", "kind": "counter", "unit": "USD"},
]
TERMINAL_GATE_STEPS = ["constructed", "terminal-gate-held", "public-stream-completed", "result-observed"]
EOF_CHECKPOINTS = {
    RESULT_BEFORE_EOF: "result-before-source-eof",
    TERMINAL_AND_SPAN_BEFORE_EOF: "terminal-and-span-before-source-eof",
}
RESULT_FAULT_CHECKPOINTS = {
    RESULT_ATTRIBUTE_WRITE_FAILURE: "result-attribute-write-failure-observed",
    REQUEST_DURATION_FAILURE: "request-duration-record-failure-observed",
}
SCENARIOS = (
    "models-no-consumption", "models-stop-consumption", *EOF_CHECKPOINTS,
    RESULT_ATTRIBUTE_WRITE_FAILURE, FIRST_CHUNK_WRITE_FAILURE, HISTOGRAM_CREATION_FAILURE,
    REQUEST_SPAN_START_FAILURE, FIRST_CHUNK_DURATION_FAILURE, REQUEST_DURATION_FAILURE, *SETUP_CASES, DEFERRED_WAIT0,
    DEFERRED_RESULT_BEFORE_EOF, DEFERRED_READY_BEFORE_EOF, DEFERRED_LOCAL_CANCEL_AFTER_DONE, DEFERRED_LOCAL_CANCEL_BEFORE_DONE,
    DEFERRED_REMOTE_CANCEL, DEFERRED_AUTH_SETUP_FAILURE, DEFERRED_POSITIVE_WAIT, DEFERRED_HANDLE_EXPIRY,
    DEFERRED_HANDLE_ROUTE_MISMATCH, DEFERRED_UNKNOWN_JOB_ID, DEFERRED_STREAM_SOURCE_CLEANUP,
)


def indexed_steps(rows: list[dict], name_key: str) -> dict[str, dict]:
    steps = {row[name_key]: row for row in rows}
    if len(steps) != len(rows):
        raise ValueError("duplicate lifecycle checkpoints")
    return steps


def validate_source(
    name: str, observed: dict, *, started: bool, terminal_emitted: bool, eof: bool, upstream: bool, returned: bool = False,
    dropped: bool | None = None,
) -> None:
    expected = {
        "dispatch_calls": int(started), "source_started": started,
        "terminal_count": int(terminal_emitted), "source_eof": eof,
    }
    if upstream:
        expected.update({"source_iterators": int(started), "source_return_calls": int(returned)})
    else:
        expected["source_dropped"] = eof if dropped is None else dropped
    for key, value in expected.items():
        if type(observed[key]) is not type(value) or observed[key] != value:
            raise ValueError(f"{name}: unexpected {key}, expected {value!r}, got {observed[key]!r}")


def normalize_completion(
    steps: dict[str, dict], scenario: str, *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    required = ["constructed", "source-at-gate"]
    stopped_consuming = scenario == "models-stop-consumption"
    result_fault = RESULT_FAULT_CHECKPOINTS.get(scenario)
    observe_terminal = scenario == TERMINAL_AND_SPAN_BEFORE_EOF or result_fault is not None
    if stopped_consuming:
        required.append("consumer-stopped-after-start")
    required.append("before-source-release")
    checkpoint_name = EOF_CHECKPOINTS.get(scenario)
    if checkpoint_name:
        required.append(checkpoint_name)
    required.append("source-released")
    source_steps = tuple(required)
    if observe_terminal:
        required.append("public-stream-completed")
    required.append("result-observed")
    if result_fault:
        required.append(result_fault)
    if list(steps) != required:
        raise ValueError("missing or reordered public completion checkpoints")
    for name in source_steps:
        released = name == "source-released"
        validate_source(name, steps[name]["observed"], started=name != "constructed",
                        terminal_emitted=released or name == checkpoint_name, eof=released, upstream=upstream)

    before = steps["before-source-release"]["observed"]
    after = steps["source-released"]["observed"]
    result = steps["result-observed"]
    if upstream and (
        result["matches_source"] is not True
        or (result["stop"], result["provider"], result["model"]) != ("stop", "lifecycle-differential", "mock-model")
    ):
        raise ValueError("Pi result does not identify the controlled completion")
    common = {
        "dispatch_calls_at_construction": steps["constructed"]["observed"]["dispatch_calls"],
        "dispatch_calls": after["dispatch_calls"], "source_started": before["source_started"],
        "terminal_before_release": before["terminal_count"], "source_terminal_count": after["terminal_count"],
        "source_completed": after["source_eof"], "consumer_polled": stopped_consuming or observe_terminal,
        "terminal_kind": result["stop"], "message_provider": result["provider"], "message_model": result["model"],
    }
    if scenario == RESULT_BEFORE_EOF:
        if steps["before-source-release"]["result_state"] != "pending":
            raise ValueError("public result was not pending before source terminal emission")
        checkpoint = steps["result-before-source-eof"]
        state = checkpoint["result_state"]
        early_result = checkpoint["result"]
        if state == "ready":
            if early_result != {key: result[key] for key in ("stop", "provider", "model")}:
                raise ValueError("result before EOF disagrees with the final result")
        elif state != "pending" or early_result is not None:
            raise ValueError("invalid result observation at the EOF gate")
        common.update({
            "result_pending_before_terminal": True,
            "terminal_count_at_eof_gate": checkpoint["observed"]["terminal_count"],
            "source_eof_at_eof_gate": checkpoint["observed"]["source_eof"],
        })
        return common, {"result_state": state}
    if scenario == RESULT_ATTRIBUTE_WRITE_FAILURE:
        observed, state = normalize_result_attribute_failure(steps, upstream=upstream)
        return common | observed, state
    if scenario == REQUEST_DURATION_FAILURE:
        observed, state = normalize_request_duration_failure(steps, upstream=upstream)
        return common | observed, state
    if observe_terminal:
        observed, state = normalize_terminal_and_span(steps)
        return common | observed, state
    return common, None


def public_terminal(steps: dict[str, dict], *, require_start: bool = True) -> dict:
    completed, result = steps["public-stream-completed"], steps["result-observed"]
    if require_start and steps["before-source-release"]["public_start"] != "start":
        raise ValueError("public Start must be observed before the source terminal")
    terminal = {"kind": "done", **{key: result[key] for key in ("stop", "provider", "model")}}
    if completed["terminal"] != terminal or completed["outer_eof"] is not True or result["matches_terminal"] is not True:
        raise ValueError("public Done, stream EOF and the matching result must be observed")
    return terminal


def request_span(step: dict, result: dict) -> dict:
    spans = step["request_spans"]
    if not isinstance(spans, list) or len(spans) != 1:
        raise ValueError("each checkpoint must observe exactly one request span")
    span = spans[0]
    if (span["name"], span["provider"], span["model"]) != ("pi.ai.request", result["provider"], result["model"]):
        raise ValueError("request span name or call identity mismatch")
    if type(span["settled"]) is not bool:
        raise ValueError("request span settlement must be observed as a boolean")
    return span


def open_request_span(step: dict, result: dict) -> None:
    span = request_span(step, result)
    if span["settled"] or span["stop"] is not None:
        raise ValueError("request span must be open before the source terminal")


def normalize_terminal_and_span(steps: dict[str, dict]) -> tuple[dict[str, Any], dict[str, Any]]:
    terminal = public_terminal(steps)
    before = steps["before-source-release"]
    checkpoint = steps[EOF_CHECKPOINTS[TERMINAL_AND_SPAN_BEFORE_EOF]]
    completed, result = steps["public-stream-completed"], steps["result-observed"]
    if before["next_state"] != "pending":
        raise ValueError("the next public item must be pending after Start and before the source terminal")
    state, early_terminal = checkpoint["next_state"], checkpoint["terminal"]
    if state == "ready":
        if early_terminal != terminal:
            raise ValueError("public Done at the EOF gate disagrees with the final terminal")
    elif state != "pending" or early_terminal is not None:
        raise ValueError("invalid public next observation at the EOF gate")

    open_request_span(before, result)
    span_at_eof_gate = request_span(checkpoint, result)
    span_after = request_span(completed, result)
    if not span_after["settled"] or span_after["stop"] != result["stop"]:
        raise ValueError("request span must be settled with the final stop after source EOF")
    if span_at_eof_gate["settled"] and span_at_eof_gate["stop"] != result["stop"]:
        raise ValueError("request span settled at the EOF gate with the wrong stop")
    return {
        "public_start_observed": True, "public_terminal_matches_result": True, "public_stream_eof": True,
        "terminal_count_at_eof_gate": checkpoint["observed"]["terminal_count"],
        "source_eof_at_eof_gate": checkpoint["observed"]["source_eof"],
        "request_span_open_before_terminal": True, "request_span_settled_after_source_eof": True,
        "request_span_stop": span_after["stop"],
    }, {"terminal_state": state, "span_settled": span_at_eof_gate["settled"]}


def normalize_result_attribute_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    public_terminal(steps)
    result = steps["result-observed"]
    open_request_span(steps["before-source-release"], result)
    fault = steps["result-attribute-write-failure-observed"]
    delivery = "synchronous-throw" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (delivery, RESULT_ATTRIBUTE_WRITE_ERROR):
        raise ValueError("missing the controlled result attribute write failure")
    attempts = fault["write_attempts"]
    if not isinstance(attempts, list) or len(attempts) != 1 or attempts[0]["stop"] != result["stop"]:
        raise ValueError("the matching result attributes must be written once")
    validate_source("result attribute write", attempts[0]["observed"], started=True,
                    terminal_emitted=True, eof=True, upstream=upstream)
    span = request_span(fault, result)
    if span["stop"] not in (None, result["stop"]):
        raise ValueError("unexpected request span stop after the write failure")
    if not upstream:
        finish_calls = fault["span_finish_calls"]
        if type(finish_calls) is not int or finish_calls != int(span["settled"]):
            raise ValueError("Rust span settlement disagrees with the observed finish calls")
        if type(fault["span_backend_dropped"]) is not bool:
            raise ValueError("Rust span backend Drop was not sampled")

    host_errors = observed_host_errors(fault["host_errors"], upstream=upstream)
    return {
        "public_start_observed": True, "public_terminal_matches_result": True, "public_stream_eof": True,
        "source_eof_at_failed_write": attempts[0]["observed"]["source_eof"],
        "result_attribute_write_attempts": len(attempts), "request_span_open_before_terminal": True,
    }, {"span_settled": span["settled"], "span_stop": span["stop"], "host_errors": host_errors}


def observed_host_errors(host_errors: Any, *, upstream: bool) -> list[dict]:
    if not isinstance(host_errors, list):
        raise ValueError("the host error channel must be observed")
    for error in host_errors:
        if upstream:
            if error["channel"] != "unhandledRejection" or not isinstance(error["message"], str):
                raise ValueError("invalid Pi host rejection observation")
        elif (error["channel"] != "tracing" or error["target"] != "opi_ai::telemetry"
              or error["level"] not in ("WARN", "ERROR") or not isinstance(error["fields"], dict)):
            raise ValueError("invalid Rust host tracing observation")
    return host_errors


def validate_pi_cleanup(
    name: str, observed: dict, *, returned: bool, eof: bool, result_read: bool | None = None,
) -> None:
    expected = {
        "source_return_completions": int(returned),
        "source_finally_calls": int(returned or eof),
        "source_result_calls": int(eof if result_read is None else result_read),
    }
    if any(type(observed[key]) is not int or observed[key] != value for key, value in expected.items()):
        raise ValueError(f"{name}: source iterator return, finally or result observation is inconsistent")


def normalize_request_duration_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    terminal = public_terminal(steps)
    result = steps["result-observed"]
    open_request_span(steps["before-source-release"], result)
    fault = steps[RESULT_FAULT_CHECKPOINTS[REQUEST_DURATION_FAILURE]]
    delivery = "synchronous-throw" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (delivery, REQUEST_DURATION_ERROR):
        raise ValueError("missing the controlled request duration recording failure")
    attempts = fault["record_attempts"]
    if not isinstance(attempts, list) or len(attempts) != 1:
        raise ValueError("the request duration histogram must fail during recording once")
    attempt = attempts[0]
    if (attempt["name"], attempt["kind"], attempt["unit"], attempt["record_stop"]) != (
        "pi.ai.request.duration", "histogram", "ms", terminal["stop"],
    ):
        raise ValueError("wrong failed histogram definition or result attributes")
    validate_source("request duration record", attempt["observed"], started=True,
                    terminal_emitted=True, eof=True, upstream=upstream)
    if upstream:
        validate_pi_cleanup("request duration record", attempt["observed"], returned=False, eof=True)
    at_fault, after = (request_span(row, result) for row in (attempt, fault))
    usage_input = attempt["usage_input"]
    if at_fault["settled"] or type(usage_input) not in (int, float) or not 0 <= usage_input < float("inf"):
        raise ValueError("final usage must already be recorded on the open request span")
    for span in (at_fault, after):
        if (span["stop"], span["events"], span["status"]) != (terminal["stop"], ["pi.ai.first_chunk"], {"status": "ok"}):
            raise ValueError("the request span must retain its successful result attributes and first chunk event")
    metrics = fault["metric_names"]
    if metrics != ["pi.ai.first_chunk.duration"]:
        raise ValueError("record the first chunk duration and no measurements at or after the failed write")
    if not upstream and (type(fault["span_finish_calls"]) is not int
                         or fault["span_finish_calls"] != int(after["settled"])
                         or fault["span_backend_dropped"] is not True):
        raise ValueError("observe Rust span settlement, finish count and wrapper Drop")
    return {
        "public_start_observed": True, "public_terminal_matches_result": True, "public_stream_eof": True,
        "failed_histogram": attempt["name"], "record_attempts": len(attempts),
        "source_eof_at_failed_record": attempt["observed"]["source_eof"],
        "request_span_open_at_failed_record": not at_fault["settled"], "result_attributes_written": True,
        "request_span_stop": after["stop"], "recorded_metrics": metrics,
    }, {
        "fault_delivery": delivery, "span_settled": after["settled"],
        "host_errors": observed_host_errors(fault["host_errors"], upstream=upstream),
    }


def normalize_gated_terminal(
    steps: dict[str, dict], *, upstream: bool, error_message: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    initial, held = steps["constructed"], steps["terminal-gate-held"]
    completed, result = steps["public-stream-completed"], steps["result-observed"]
    validate_source("constructed", initial["observed"], started=False, terminal_emitted=False, eof=False, upstream=upstream)
    returned = upstream and held["observed"]["source_return_calls"] == 1
    source_checks = (
        ("terminal gate held", held["observed"], returned, False),
        ("public stream completed", completed["observed"], returned, not returned),
    )
    for name, observed, iterator_returned, eof in source_checks:
        validate_source(name, observed, started=True, terminal_emitted=eof, eof=eof,
                        upstream=upstream, returned=iterator_returned)
        if upstream:
            validate_pi_cleanup(name, observed, returned=iterator_returned, eof=eof)

    terminal = completed["terminal"]
    outcome = terminal["kind"]
    expected_outcome = "error" if returned else "done"
    expected_stop = "error" if returned else "stop"
    if (outcome, terminal["stop"], result["stop"]) != (expected_outcome, expected_stop, expected_stop):
        raise ValueError("public terminal and result do not match the observed source exit")
    if terminal.get("error_message") != (error_message if returned else None):
        raise ValueError("public failure must identify the observed event fault")
    if result["delivery"] != ("fulfilled-message" if upstream else "ok") or result["matches_terminal"] is not True:
        raise ValueError("the public result channel must carry the observed terminal")
    identity = ("lifecycle-differential", "mock-model")
    if (result["provider"], result["model"]) != identity or (terminal["provider"], terminal["model"]) != identity:
        raise ValueError("public terminal or result changed the request identity")
    if held["public_start"] != "start" or completed["outer_eof"] is not True:
        raise ValueError("observe public Start, one terminal and outer EOF")
    state, early = held["next_state"], held["terminal"]
    if state == "ready":
        if outcome != "error" or early != terminal:
            raise ValueError("only the matching public error can be ready before the source terminal")
    elif state != "pending" or early is not None:
        raise ValueError("invalid public next observation at the held terminal gate")

    return {
        "public_start_observed": True, "public_terminal_matches_result": True, "public_stream_eof": True,
        "message_provider": result["provider"], "message_model": result["model"],
    }, {
        "next_state_at_terminal_gate": state, "terminal_kind": outcome, "result_stop": result["stop"],
        "result_delivery": "success" if outcome == "done" else result["delivery"],
        "public_error_message": terminal.get("error_message"),
        "source_release": "iterator-return" if returned else "eof", "source_released_at_terminal_gate": returned,
        "source_terminal_count": completed["observed"]["terminal_count"], "source_eof": completed["observed"]["source_eof"],
    }


def normalize_first_chunk_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if list(steps) != [*TERMINAL_GATE_STEPS, "first-chunk-write-failure-observed"]:
        raise ValueError("missing or reordered first chunk failure checkpoints")
    initial, held, result = steps["constructed"], steps["terminal-gate-held"], steps["result-observed"]
    fault = steps["first-chunk-write-failure-observed"]
    delivery = "synchronous-throw" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (delivery, FIRST_CHUNK_WRITE_ERROR):
        raise ValueError("missing the controlled first chunk write failure")
    attempts = fault["write_attempts"]
    if not isinstance(attempts, list) or len(attempts) != 1 or attempts[0]["event"] != "pi.ai.first_chunk":
        raise ValueError("the first chunk event must be written once")
    at_write = attempts[0]["observed"]
    validate_source("first chunk write", at_write, started=True, terminal_emitted=False, eof=False, upstream=upstream)
    if upstream:
        validate_pi_cleanup("first chunk write", at_write, returned=False, eof=False)
    public, outcome = normalize_gated_terminal(steps, upstream=upstream, error_message=FIRST_CHUNK_WRITE_ERROR)
    open_request_span(initial, result)
    span_held, span = request_span(held, result), request_span(fault, result)
    if any(item["stop"] not in (None, result["stop"]) for item in (span_held, span)):
        raise ValueError("request span stop does not match the public outcome")
    if not upstream and (type(fault["span_finish_calls"]) is not int
                         or fault["span_finish_calls"] != int(span["settled"])
                         or fault["span_backend_dropped"] is not True):
        raise ValueError("Rust span finish and wrapper Drop must be observed")
    return {
        "dispatch_calls": attempts[0]["observed"]["dispatch_calls"], "failed_event": attempts[0]["event"],
        "event_write_attempts": len(attempts), "source_terminal_count_at_failed_write": attempts[0]["observed"]["terminal_count"],
        "source_eof_at_failed_write": attempts[0]["observed"]["source_eof"], "request_span_open_at_construction": True,
    } | public, outcome | {
        "span_settled_at_terminal_gate": span_held["settled"], "span_stop_at_terminal_gate": span_held["stop"],
        "span_settled": span["settled"], "span_stop": span["stop"],
        "host_errors": observed_host_errors(fault["host_errors"], upstream=upstream),
    }


def normalize_histogram_creation_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    required = ["call-boundary"]
    if not upstream:
        required += ["source-at-gate", "before-source-release", "public-stream-completed", "result-observed"]
    required.append("histogram-creation-failure-observed")
    if list(steps) != required:
        raise ValueError("missing or reordered histogram creation failure checkpoints")

    boundary, fault = steps[required[0]], steps[required[-1]]
    identity = boundary["provider"], boundary["model"]
    delivery = "synchronous-throw" if upstream else "returned-handle"
    error_message = HISTOGRAM_CREATION_ERROR if upstream else None
    if identity != ("lifecycle-differential", "mock-model") or (
        boundary["delivery"], boundary["error_message"]
    ) != (delivery, error_message):
        raise ValueError("wrong request identity or public call boundary")
    fault_delivery = "synchronous-throw" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (fault_delivery, HISTOGRAM_CREATION_ERROR):
        raise ValueError("missing the controlled histogram creation failure")
    attempts = fault["instrument_attempts"]
    count = boundary["instrument_attempt_count"]
    if not isinstance(attempts, list) or len(attempts) != 1 or type(count) is not int or count != 1:
        raise ValueError("the histogram creation must fail once during the call")
    attempt = attempts[0]
    if (attempt["name"], attempt["kind"], attempt["unit"]) != ("pi.ai.request.duration", "histogram", "ms"):
        raise ValueError("wrong failed instrument definition")
    for name, row in (("histogram creation", attempt), ("public call boundary", boundary)):
        validate_source(name, row["observed"], started=False, terminal_emitted=False, eof=False, upstream=upstream)
    validate_source("final startup failure checkpoint", fault["observed"], started=not upstream,
                    terminal_emitted=not upstream, eof=not upstream, upstream=upstream)

    terminal, result_delivery = None, None
    if not upstream:
        for name in ("source-at-gate", "before-source-release"):
            validate_source(name, steps[name]["observed"], started=True, terminal_emitted=False, eof=False, upstream=False)
        terminal = public_terminal(steps)
        result = steps["result-observed"]
        if (result["provider"], result["model"]) != identity or (result["delivery"], result["stop"]) != ("ok", "stop"):
            raise ValueError("Rust must deliver the matching successful provider result")
        result_delivery = result["delivery"]

    at_fault, at_boundary, after = [
        None if row["request_spans"] == [] else request_span(row, boundary)
        for row in (attempt, boundary, fault)
    ]
    if at_fault is not None and (at_fault["settled"] or at_fault["stop"] is not None):
        raise ValueError("a request span present at the factory fault must still be open")
    if (at_fault is not None and at_boundary is None) or (at_boundary is not None and after is None):
        raise ValueError("a recorded request span disappeared")
    details = {
        "call_delivery": delivery, "public_error_message": error_message,
        "dispatch_calls": fault["observed"]["dispatch_calls"], "source_eof": fault["observed"]["source_eof"],
        "public_terminal": terminal, "result_delivery": result_delivery,
        "request_span_at_fault": at_fault, "request_span_at_call_boundary": at_boundary, "request_span_at_end": after,
        "host_errors": observed_host_errors(fault["host_errors"], upstream=upstream),
    }
    if not upstream:
        for row, span in ((boundary, at_boundary), (fault, after)):
            finish_calls, dropped = row["span_finish_calls"], row["span_backend_dropped"]
            if (type(finish_calls) is not int or finish_calls != int(span is not None and span["settled"])
                    or type(dropped) is not bool or (span is None and dropped)):
                raise ValueError("invalid Rust span finish or wrapper Drop observation")
        details.update(
            span_finish_calls_at_call_boundary=boundary["span_finish_calls"],
            span_backend_dropped_at_call_boundary=boundary["span_backend_dropped"],
            span_finish_calls=fault["span_finish_calls"], span_backend_dropped=fault["span_backend_dropped"],
            source_dropped=fault["observed"]["source_dropped"],
        )
    return {
        "instrument_name": attempt["name"], "instrument_kind": attempt["kind"], "instrument_unit": attempt["unit"],
        "instrument_creation_attempts": len(attempts),
        "dispatch_calls_at_fault": attempt["observed"]["dispatch_calls"],
        "dispatch_calls_at_call_boundary": boundary["observed"]["dispatch_calls"],
        "requested_provider": boundary["provider"], "requested_model": boundary["model"],
    }, details


def normalize_request_span_start_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if list(steps) != [*TERMINAL_GATE_STEPS, "request-span-start-failure-observed"]:
        raise ValueError("missing or reordered request span start failure checkpoints")
    initial, held = steps["constructed"], steps["terminal-gate-held"]
    fault = steps["request-span-start-failure-observed"]
    delivery = "rejected-promise" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (delivery, REQUEST_SPAN_START_ERROR):
        raise ValueError("missing the controlled request span start failure")
    attempts = fault["span_start_attempts"]
    count = initial["span_start_attempt_count"]
    if (not isinstance(attempts, list) or len(attempts) != 1 or type(count) is not int or count != 1
            or initial["call_delivery"] != "returned-handle"):
        raise ValueError("request span start must fail once before the handle returns")
    attempt = attempts[0]
    if (attempt["name"], attempt["provider"], attempt["model"]) != ("pi.ai.request", "lifecycle-differential", "mock-model"):
        raise ValueError("span start fault must identify the selected request")
    if attempt["created_instruments"] != REQUEST_INSTRUMENTS:
        raise ValueError("all four instruments must be created before the span start attempt")
    validate_source("request span start", attempt["observed"], started=False, terminal_emitted=False, eof=False, upstream=upstream)
    if any(row["request_spans"] != [] for row in (attempt, initial, held, fault)):
        raise ValueError("failed request span start must leave zero request spans")
    if not upstream and (type(fault["span_finish_calls"]) is not int or fault["span_finish_calls"] != 0
                         or fault["span_backend_dropped"] is not False):
        raise ValueError("failed Rust span start must leave no wrapper or finish call")

    public, outcome = normalize_gated_terminal(steps, upstream=upstream, error_message=MISSING_SPAN_EVENT_ERROR)
    host_errors = observed_host_errors(fault["host_errors"], upstream=upstream)
    if upstream:
        for error in host_errors:
            matched, name = error["injected_start_rejection"], error["error_name"]
            if type(matched) is not bool or (name is not None and not isinstance(name, str)):
                raise ValueError("Pi host rejection identity and error name must be observed")
            if matched and (name, error["message"]) != ("Error", f"Error: {REQUEST_SPAN_START_ERROR}"):
                raise ValueError("Pi host rejection marked as the injected promise must match its error")
    return {
        "span_name": attempt["name"], "created_instruments": attempt["created_instruments"],
        "span_start_attempts": len(attempts), "call_delivery": initial["call_delivery"],
        "dispatch_calls_at_fault": attempt["observed"]["dispatch_calls"],
        "dispatch_calls_at_construction": initial["observed"]["dispatch_calls"],
        "dispatch_calls": held["observed"]["dispatch_calls"], "request_span_count": len(fault["request_spans"]),
    } | public, outcome | {"fault_delivery": delivery, "host_errors": host_errors}


def normalize_first_chunk_duration_failure(
    steps: dict[str, dict], *, upstream: bool,
) -> tuple[dict[str, Any], dict[str, Any]]:
    fault_step = "first-chunk-duration-record-failure-observed"
    if list(steps) != [*TERMINAL_GATE_STEPS[:2], "source-exit-checkpoint", *TERMINAL_GATE_STEPS[2:], fault_step]:
        raise ValueError("missing or reordered first chunk duration failure checkpoints")
    initial, held, checkpoint, completed, result, fault = steps.values()
    terminal = public_terminal(steps, require_start=False)
    delivery = "fulfilled-message" if upstream else "ok"
    if (result["stop"], result["provider"], result["model"], result["delivery"]) != (
        "stop", "lifecycle-differential", "mock-model", delivery,
    ):
        raise ValueError("observe the original successful terminal and matching result")

    fault_delivery = "synchronous-throw" if upstream else "typed-err"
    if (fault["fault_delivery"], fault["fault_marker"]) != (fault_delivery, FIRST_CHUNK_DURATION_ERROR):
        raise ValueError("missing the controlled first chunk duration recording failure")
    attempts = fault["record_attempts"]
    if not isinstance(attempts, list) or len(attempts) != 1:
        raise ValueError("the first chunk duration must fail during recording once")
    attempt = attempts[0]
    metric = "pi.ai.first_chunk.duration"
    if (attempt["name"], attempt["kind"], attempt["unit"]) != (metric, "histogram", "ms"):
        raise ValueError("wrong failed histogram definition")

    returned = upstream and checkpoint["observed"]["source_return_calls"] == 1
    for name, row, started, emitted, eof, did_return in (
        ("constructed", initial, False, False, False, False),
        ("terminal gate", held, True, False, False, False),
        ("failed duration record", attempt, True, True, False, False),
        ("source exit checkpoint", checkpoint, True, True, False, returned),
        ("public stream completed", completed, True, True, not returned, returned),
    ):
        observed = row["observed"]
        validate_source(name, observed, started=started, terminal_emitted=emitted, eof=eof,
                        upstream=upstream, returned=did_return)
        if upstream:
            validate_pi_cleanup(name, observed, returned=did_return, eof=eof)
    if held["next_state"] != "pending" or checkpoint["eof_gate_reached"] is not (not returned):
        raise ValueError("observe the terminal gate and either iterator return or the held EOF gate")
    state = checkpoint["next_state"]
    if state not in ("pending", "ready") or checkpoint["terminal"] != (terminal if state == "ready" else None):
        raise ValueError("the first public item must be pending or match the final Done")

    first_chunk_events = ["pi.ai.first_chunk"]
    for row, events in ((initial, []), (held, []), (attempt, first_chunk_events)):
        open_request_span(row, result)
        span = row["request_spans"][0]
        if span["events"] != events or span["status"] != {"status": "ok"}:
            raise ValueError("the first chunk event must be written before its histogram record fails")
    span_at_exit, span = (request_span(row, result) for row in (checkpoint, fault))
    for observed_span in (span_at_exit, span):
        status = {"status": "ok"}
        if observed_span["stop"] == "error":
            status = {"status": "error", "error": {"name": "provider_error", "message": FIRST_CHUNK_DURATION_ERROR}}
        if (observed_span["events"] != first_chunk_events or observed_span["stop"] not in (None, "stop", "error")
                or observed_span["status"] != status):
            raise ValueError("invalid request span observations after the duration record failure")
    durations = fault["duration_measurements"]
    if durations not in ([], [{"name": "pi.ai.request.duration", "stop": span["stop"]}]):
        raise ValueError("duration measurements must exclude the failed record and match the request span stop")
    if not upstream and (type(fault["span_finish_calls"]) is not int
                         or fault["span_finish_calls"] != int(span["settled"])
                         or type(fault["span_backend_dropped"]) is not bool):
        raise ValueError("observe Rust span finish calls and wrapper Drop")
    return {
        "dispatch_calls_at_construction": initial["observed"]["dispatch_calls"],
        "dispatch_calls": attempt["observed"]["dispatch_calls"],
        "failed_histogram": metric, "record_attempts": len(attempts),
        "source_terminal_count_at_fault": attempt["observed"]["terminal_count"],
        "source_eof_at_fault": attempt["observed"]["source_eof"],
        "first_chunk_event_recorded": True, "public_terminal": terminal,
        "public_terminal_matches_result": result["matches_terminal"], "public_stream_eof": completed["outer_eof"],
    }, {
        "fault_delivery": fault_delivery, "next_state_at_source_exit": state,
        "source_release": "iterator-return" if returned else "eof", "source_eof": completed["observed"]["source_eof"],
        "request_span_at_source_exit": span_at_exit, "request_span_at_end": span,
        "duration_measurements": durations, "host_errors": observed_host_errors(fault["host_errors"], upstream=upstream),
    }


def normalize_setup(
    steps: dict[str, dict], scenario: str, *, event_delivery: str, result_delivery: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    phase, provider, marker, expected_auth_calls = SETUP_CASES[scenario]
    required = ["constructed", "failure-event", "event-eof", "result-observed"]
    if expected_auth_calls:
        required.insert(1, "auth-at-gate")
    if list(steps) != required:
        raise ValueError("missing or reordered public setup checkpoints")
    constructed = steps["constructed"]
    if (constructed["phase"], constructed["provider"], constructed["model"]) != (phase, provider, "mock-model"):
        raise ValueError("setup trace does not identify the selected fault and request")

    eof = steps["event-eof"]
    counter_steps = [constructed, eof]
    if expected_auth_calls:
        gate = steps["auth-at-gate"]
        if type(gate["auth_calls"]) is not int or gate["auth_calls"] != expected_auth_calls:
            raise ValueError("auth did not reach the pre-consumption gate exactly once")
        counter_steps.append(gate)
    if any(type(row["dispatch_calls"]) is not int or row["dispatch_calls"] != 0 for row in counter_steps):
        raise ValueError("provider dispatched despite setup failure")
    initial_auth_calls = constructed["auth_calls"]
    if type(initial_auth_calls) is not int or not 0 <= initial_auth_calls <= expected_auth_calls:
        raise ValueError("invalid auth count at construction")
    if type(eof["auth_calls"]) is not int or eof["auth_calls"] != expected_auth_calls or eof["done"] is not True:
        raise ValueError("setup must finish once and the error event must be followed by EOF")

    event, result = steps["failure-event"], steps["result-observed"]
    if event["delivery"] != event_delivery or result["delivery"] != result_delivery:
        raise ValueError("setup trace does not describe the observed event and result channels")
    if event["stop"] != "error" or result["stop"] != event["stop"] or result["matches_event"] is not True:
        raise ValueError("the setup result must agree with its single error terminal")
    error_message = event["error_message"]
    if not isinstance(error_message, str) or marker not in error_message.casefold():
        raise ValueError("error did not come from the selected fault")
    message_provider, message_model = event["message_provider"], event["message_model"]
    if not isinstance(message_provider, str) or not isinstance(message_model, str):
        raise ValueError("setup message identity was not observed")
    if event_delivery == "error-event":
        if (message_provider, message_model) != (provider, constructed["model"]):
            raise ValueError("Pi setup error does not identify the requested provider and model")
    elif event["cause"] != ("AuthFailed" if expected_auth_calls else "Config"):
        raise ValueError("Rust setup error has the wrong typed cause")
    return {
        "failure_phase": phase, "requested_provider": provider, "requested_model": constructed["model"],
        "message_provider": message_provider, "message_model": message_model,
        "auth_calls": eof["auth_calls"], "dispatch_calls": eof["dispatch_calls"], "terminal_kind": result["stop"],
        "terminal_count": 1, "result_matches_event": result["matches_event"],
    }, {
        "auth_calls_at_construction": initial_auth_calls, "event_delivery": event["delivery"],
        "result_delivery": result["delivery"], "error_message": error_message,
        "typed_cause": event.get("cause"),
    }


def validate_deferred_dispatches(name: str, step: dict, expected: list[dict]) -> None:
    dispatches = step["dispatches"]
    if dispatches != expected or type(dispatches[0]["wait_ms"]) is not int:
        raise ValueError(f"{name}: expected one dispatch with the original handle and exact wait")


DEFERRED_ROUTE = {"provider": "lifecycle-differential", "model": "mock-model"}
DEFERRED_API = "scripted-deferred"
DEFERRED_HANDLE = {"provider": DEFERRED_ROUTE["provider"], "modelId": DEFERRED_ROUTE["model"],
                   "api": DEFERRED_API, "id": "deferred-1", "pollAfterMs": 25}


def expected_deferred_result(stop: str) -> dict:
    reported_usage = stop in ("stop", "aborted")
    tokens = dict(zip(("input", "output", "cache_read", "cache_write"),
                      (2, 3, 4, 5) if reported_usage else (0, 0, 0, 0)))
    return {
        "stop": stop, **DEFERRED_ROUTE,
        "content": [{"type": "text", "text": "ready"}] if stop == "stop" else [],
        "deferred": DEFERRED_HANDLE if stop == "deferred" else None,
        "usage": {**tokens, "cost": 0.25 if reported_usage else 0},
    }


def expected_faux_deferred_ready() -> dict:
    return expected_deferred_result("stop") | {
        "usage": {"input": 0, "output": 2, "cache_read": 0, "cache_write": 0, "cost": 0},
    }


def validate_deferred_poll(name: str, step: dict, *, stop: str, wait_ms: int = 0) -> None:
    validate_deferred_dispatches(name, step, [{"handle": DEFERRED_HANDLE, "wait_ms": wait_ms}])
    result = expected_deferred_result(stop)
    if step["result"] != result:
        raise ValueError(f"{name}: unexpected public deferred fetch result")
    validate_deferred_poll_telemetry(name, step, stop, result["usage"], status={"status": "ok"})


def validate_deferred_poll_telemetry(
    name: str, step: dict, stop: str, usage: dict, *, status: dict,
    events: tuple[str, ...] = ("pi.ai.first_chunk",),
    route: dict[str, str] | None = None, api: str = DEFERRED_API,
) -> None:
    route = DEFERRED_ROUTE if route is None else route
    tokens = usage.copy()
    cost = tokens.pop("cost")
    attributes = {**route, "api": api, "stop": stop, "cost.total": cost}
    attributes.update({f"usage.{kind}": value for kind, value in tokens.items()})
    span = {"name": "pi.ai.poll", "attributes": attributes, "status": status,
            "parent_is_caller": True, "settled": True, "events": list(events)}
    if step["spans"] != [span]:
        raise ValueError(f"{name}: expected one settled pi.ai.poll span with the observed result")
    measurements = [
        {"name": "pi.ai.first_chunk.duration", "kind": "histogram", "unit": "ms",
         "attributes": route, "value": None, "context": "poll"},
        {"name": "pi.ai.request.duration", "kind": "histogram", "unit": "ms",
         "attributes": {**route, "stop": stop}, "value": None, "context": "poll"},
    ]
    if stop != "deferred":
        measurements.extend(
            {"name": "pi.ai.tokens", "kind": "counter", "unit": "token",
             "attributes": {**route, "kind": kind}, "value": value, "context": "none"}
            for kind, value in tokens.items()
        )
        measurements.append({"name": "pi.ai.cost", "kind": "counter", "unit": "USD",
                             "attributes": route, "value": cost, "context": "none"})
    if step["measurements"] != measurements:
        raise ValueError(f"{name}: unexpected deferred poll duration, token or cost measurements")


def normalize_deferred_wait0(steps: dict[str, dict]) -> tuple[dict, dict]:
    if list(steps) != ["pending", "ready"]:
        raise ValueError("expected two ordered deferred fetch checkpoints")
    telemetry_at_result = {}
    for name, step in steps.items():
        validate_deferred_poll(name, step, stop="deferred" if name == "pending" else "stop")
        states = step["settled_at_result"]
        if len(states) != 1 or type(states[0]) is not bool:
            raise ValueError(f"{name}: missing poll span settlement observation at public result")
        metric_names = [measurement["name"] for measurement in step["measurements"]]
        early_names = step["metric_names_at_result"]
        if (not early_names or early_names != metric_names[:len(early_names)]
                or (states[0] and early_names != metric_names)):
            raise ValueError(f"{name}: inconsistent poll measurements at public result")
        telemetry_at_result[name] = {"span_settled": states[0], "metric_names": early_names}
    observations = {key: [step[key] for step in steps.values()]
                    for key in ("dispatches", "result", "spans", "measurements")}
    return observations, telemetry_at_result


def validate_open_deferred_poll(name: str, step: dict, final: dict) -> None:
    open_span = final["spans"][0] | {
        "settled": False,
        "attributes": {key: final["spans"][0]["attributes"][key] for key in ("provider", "model", "api")},
    }
    if step["spans"] != [open_span] or step["measurements"] != final["measurements"][:1]:
        raise ValueError(f"{name}: expected an open poll span and only the first chunk measurement")


def normalize_deferred_result_before_eof(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["terminal-gate-held", "eof-gate-held", "source-released"]:
        raise ValueError("expected ordered deferred terminal, EOF and release checkpoints")
    before, held, final = steps.values()
    validate_deferred_poll("source-released", final, stop="stop")
    for name, step in steps.items():
        released = name == "source-released"
        observed = step["observed"]
        validate_source(name, observed, started=True, terminal_emitted=name != "terminal-gate-held",
                        eof=released, upstream=upstream)
        if upstream:
            validate_pi_cleanup(name, observed, returned=False, eof=released)
        validate_deferred_dispatches(name, step, final["dispatches"])
        if not released:
            validate_open_deferred_poll(name, step, final)
    if before["result"] is not None:
        raise ValueError("deferred result must be pending before the terminal")
    early = held["result"]
    if early is not None and early != final["result"]:
        raise ValueError("deferred result while EOF is held disagrees with the released result")
    return {
        "dispatches": final["dispatches"],
        "result_pending_before_terminal": True,
        "terminal_count_at_eof_gate": held["observed"]["terminal_count"],
        "source_eof_at_gate": held["observed"]["source_eof"],
        "source_completed": final["observed"]["source_eof"],
        "spans_while_eof_held": held["spans"],
        "measurements_while_eof_held": held["measurements"],
        "result": final["result"], "spans": final["spans"], "measurements": final["measurements"],
    }, {"result_state": "pending" if early is None else "ready", "result": early}


def validate_deferred_cancellation(name: str, step: dict, *, upstream: bool, cancelled: bool | None) -> None:
    remote_cancels = step["remote_cancel_calls"]
    if type(remote_cancels) is not int or remote_cancels != 0:
        raise ValueError(f"{name}: local cancellation must leave the remote cancel entry unused")
    if cancelled is None:
        return
    if step["local_cancelled"] is not cancelled or step["backend_cancelled"] is not cancelled:
        raise ValueError(f"{name}: unexpected caller or backend cancellation state")
    if upstream and step["signal_forwarded"] is not True:
        raise ValueError(f"{name}: Pi provider must receive the caller's signal")


def normalize_deferred_local_cancel_after_done(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["terminal-gate-held", "eof-gate-held", "local-cancel-observed",
                       "source-released", "handle-refetched"]:
        raise ValueError("expected ordered deferred terminal, EOF, local cancel, release and refetch checkpoints")
    before, held, cancelled, final, refetched = steps.values()
    validate_deferred_poll("source-released", final, stop="stop")
    for name, step in steps.items():
        validate_deferred_dispatches(name, step, final["dispatches"])
        local_cancelled = name in ("local-cancel-observed", "source-released")
        validate_deferred_cancellation(name, step, upstream=upstream,
                                       cancelled=None if name == "handle-refetched" else local_cancelled)
        if name == "handle-refetched":
            continue
        observed = step["observed"]
        eof = upstream and name == "source-released"
        validate_source(name, observed, started=True, terminal_emitted=name != "terminal-gate-held",
                        eof=eof, upstream=upstream, dropped=local_cancelled)
        if upstream:
            validate_pi_cleanup(name, observed, returned=False, eof=eof)
        if not local_cancelled or (upstream and name == "local-cancel-observed"):
            validate_open_deferred_poll(name, step, final)
    if not upstream:
        validate_deferred_poll("local-cancel-observed", cancelled, stop="stop")

    result = final["result"]
    if before["result"] is not None or held["result"] not in (None, result):
        raise ValueError("unexpected public result before local cancellation")
    if cancelled["result"] != result or refetched["result"] != result:
        raise ValueError("local cancellation and explicit refetch must preserve the ready result")
    observed = cancelled["observed"]
    source_released = observed["source_finally_calls"] == 1 if upstream else observed["source_dropped"]
    return {
        "dispatches": final["dispatches"], "remote_cancel_calls": final["remote_cancel_calls"],
        "result": result, "spans": final["spans"], "measurements": final["measurements"],
        "refetch_dispatches": refetched["dispatches"], "refetch_result": refetched["result"],
    }, {
        "result_before_cancel": "pending" if held["result"] is None else "ready",
        "source_released_at_cancel_checkpoint": source_released,
        "poll_settled_at_cancel_checkpoint": cancelled["spans"][0]["settled"],
        "natural_source_eof_at_cleanup": final["observed"]["source_eof"],
    }


def normalize_deferred_local_cancel_before_done(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["terminal-gate-held", "local-cancel-observed", "source-released", "handle-refetched"]:
        raise ValueError("expected ordered deferred terminal gate, local cancel, release and refetch checkpoints")
    before, cancelled, final, refetched = steps.values()
    validate_deferred_poll("source-released", final, stop="aborted")
    validate_open_deferred_poll("terminal-gate-held", before, final)
    if upstream:
        validate_open_deferred_poll("local-cancel-observed", cancelled, final)
    else:
        validate_deferred_poll("local-cancel-observed", cancelled, stop="aborted")
    if before["result"] is not None:
        raise ValueError("deferred result must be pending before cancellation")
    if refetched["result"] != expected_deferred_result("stop"):
        raise ValueError("explicit refetch must return the ready result after local cancellation")

    delivery, cause, message = (("fulfilled-message", None, "Request was aborted") if upstream
                                else ("typed-err", "Cancelled", "cancelled"))
    for name, step in steps.items():
        validate_deferred_dispatches(name, step, final["dispatches"])
        local_cancelled = name != "terminal-gate-held"
        validate_deferred_cancellation(name, step, upstream=upstream,
                                       cancelled=None if name == "handle-refetched" else local_cancelled)
        if name == "handle-refetched":
            continue
        observed = step["observed"]
        eof = upstream and local_cancelled
        validate_source(name, observed, started=True, terminal_emitted=False, eof=eof, upstream=upstream,
                        dropped=local_cancelled)
        if upstream:
            validate_pi_cleanup(name, observed, returned=False, eof=eof, result_read=name == "source-released")
            aborts = observed["source_aborted_terminals"]
            if type(aborts) is not int or aborts != int(local_cancelled):
                raise ValueError(f"{name}: expected one Pi provider abort terminal only after cancellation")
        if local_cancelled:
            if (step["result_delivery"], step.get("typed_cause"), step["error_message"]) != (delivery, cause, message):
                raise ValueError(f"{name}: unexpected public cancellation channel, cause or diagnostic")
            if step["result"] != final["result"]:
                raise ValueError(f"{name}: public cancellation payload changed during cleanup")

    return {
        "dispatches": final["dispatches"], "remote_cancel_calls": final["remote_cancel_calls"],
        "result_pending_before_cancel": before["result"] is None,
        "source_done_count": final["observed"]["terminal_count"],
        "result": final["result"], "spans": final["spans"], "measurements": final["measurements"],
        "refetch_dispatches": refetched["dispatches"], "refetch_result": refetched["result"],
    }, {
        "result_delivery": cancelled["result_delivery"], "typed_cause": cancelled.get("typed_cause"),
        "error_message": cancelled["error_message"], "poll_settled_at_result": cancelled["spans"][0]["settled"],
        "natural_source_eof_at_cleanup": final["observed"]["source_eof"],
    }


def normalize_deferred_remote_cancel(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["pending", "cancelled", "refetch-failed"]:
        raise ValueError("expected pending fetch, explicit remote cancel and failed refetch")
    pending, cancelled, refetched = steps.values()
    validate_deferred_poll("pending", pending, stop="deferred")
    validate_deferred_dispatches("refetch-failed", refetched, pending["dispatches"])
    for (name, step), expected_count in zip(steps.items(), (1, 1, 2)):
        if type(step["fetch_count"]) is not int or step["fetch_count"] != expected_count:
            raise ValueError(f"{name}: unexpected backend fetch count")
        if step["cancelled_handles"] != ([] if name == "pending" else [DEFERRED_HANDLE]):
            raise ValueError(f"{name}: expected one explicit cancellation of the original handle")
    if cancelled["result_delivery"] != "ok" or cancelled["telemetry_forwarded"] is not True:
        raise ValueError("remote cancel must succeed and forward the caller telemetry")
    if cancelled["dispatches"] != [] or cancelled["spans"] != [] or cancelled["measurements"] != []:
        raise ValueError("remote cancel must not create a fetch or SDK AI telemetry")

    result = refetched["result"]
    identity = {key: result[key] for key in DEFERRED_ROUTE}
    if any(not isinstance(value, str) for value in identity.values()) or (upstream and identity != DEFERRED_ROUTE):
        raise ValueError("refetch failure did not identify the requested route")
    expected_result = expected_deferred_result("error") | identity
    if result != expected_result:
        raise ValueError("refetch must deliver an error with no content, handle or usage")
    delivery, cause = ("fulfilled-message", None) if upstream else ("typed-err", "Config")
    diagnostic = ("Faux deferred response was cancelled: deferred-1" if upstream else
                  "invalid provider configuration: Scripted deferred response was cancelled: deferred-1")
    if (refetched["result_delivery"], refetched.get("typed_cause"), refetched["error_message"]) != (
        delivery, cause, diagnostic,
    ):
        raise ValueError("unexpected public refetch failure channel or diagnostic")
    validate_deferred_poll_telemetry(
        "refetch-failed", refetched, "error", result["usage"],
        status={"status": "error", "error": {"name": "provider_error", "message": diagnostic}},
    )
    span, = refetched["spans"]
    return {
        "pending_result": pending["result"], "pending_dispatches": pending["dispatches"],
        "cancelled_handles": cancelled["cancelled_handles"], "cancel_telemetry_forwarded": cancelled["telemetry_forwarded"],
        "refetch_dispatches": refetched["dispatches"], "refetch_result": result,
        "refetch_span": {key: span[key] for key in ("name", "attributes", "parent_is_caller", "settled", "events")}
        | {"status": span["status"]["status"]},
        "refetch_measurements": refetched["measurements"],
    }, {
        "result_delivery": refetched["result_delivery"], "typed_cause": refetched.get("typed_cause"),
        "error_message": refetched["error_message"],
    }


def validate_deferred_poll_before_delivery(name: str, step: dict) -> None:
    open_poll = {"name": "pi.ai.poll", "attributes": DEFERRED_ROUTE | {"api": DEFERRED_API},
                 "status": {"status": "ok"}, "parent_is_caller": True, "settled": False, "events": []}
    if step["spans"] != [open_poll] or step["measurements"] != []:
        raise ValueError(f"{name}: expected one open poll span and no measurements before delivery")


def normalize_deferred_auth_setup_failure(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["auth-at-gate", "fetch-failed"]:
        raise ValueError("expected a held deferred auth gate and failed fetch")
    gate, failed = steps.values()
    for name, step in steps.items():
        if type(step["auth_calls"]) is not int or step["auth_calls"] != 1:
            raise ValueError(f"{name}: expected exactly one deferred auth resolution")
        if step["dispatches"] != [] or type(step["remote_cancel_calls"]) is not int or step["remote_cancel_calls"] != 0:
            raise ValueError(f"{name}: deferred auth failure must not dispatch or cancel the job")
    if (gate["handle"] != DEFERRED_HANDLE or type(gate["wait_ms"]) is not int or gate["wait_ms"] != 0
            or gate["result_state"] != "pending"):
        raise ValueError("expected one pending wait0 fetch of the original handle at the auth gate")
    validate_deferred_poll_before_delivery("auth-at-gate", gate)

    result = expected_deferred_result("error")
    if failed["result"] != result:
        raise ValueError("deferred auth failure must preserve route identity with no content, handle or usage")
    delivery, cause = ("fulfilled-message", None) if upstream else ("typed-err", "AuthFailed")
    diagnostic = ("API key auth failed for provider lifecycle-differential: controlled-auth-setup-failure"
                  if upstream else "authentication failed: controlled-auth-setup-failure")
    if (failed["result_delivery"], failed.get("typed_cause"), failed["error_message"]) != (delivery, cause, diagnostic):
        raise ValueError("unexpected public deferred auth failure channel or diagnostic")
    validate_deferred_poll_telemetry(
        "fetch-failed", failed, "error", result["usage"],
        status={"status": "error", "error": {"name": "provider_error", "message": diagnostic}},
    )
    span, = failed["spans"]
    return {
        "auth_at_gate": {key: gate[key] for key in ("handle", "wait_ms", "result_state", "spans", "measurements")},
        "auth_calls": [step["auth_calls"] for step in steps.values()],
        "dispatches": [step["dispatches"] for step in steps.values()],
        "remote_cancel_calls": [step["remote_cancel_calls"] for step in steps.values()],
        "fetch_result": failed["result"],
        "fetch_span": {key: span[key] for key in ("name", "attributes", "parent_is_caller", "settled", "events")}
        | {"status": span["status"]["status"]},
        "fetch_measurements": failed["measurements"],
    }, {
        "result_delivery": failed["result_delivery"], "typed_cause": failed.get("typed_cause"),
        "error_message": failed["error_message"],
    }


def normalize_deferred_positive_wait(steps: dict[str, dict]) -> dict:
    held_names = ("deferred-outcome-held", "ready-outcome-held")
    if list(steps) != [held_names[0], "deferred", held_names[1], "ready"]:
        raise ValueError("expected a held provider outcome and result for each of two explicit positive-wait fetches")
    for name, step in steps.items():
        if type(step["remote_cancel_calls"]) is not int or step["remote_cancel_calls"] != 0:
            raise ValueError(f"{name}: the fetch must leave the remote cancel operation unused")
    for held_name, outcome, stop in zip(held_names, ("deferred", "ready"), ("deferred", "stop")):
        held = steps[held_name]
        if held["result_state"] != "pending" or held["result"] is not None:
            raise ValueError(f"{held_name}: the public fetch must stay pending until provider release")
        validate_deferred_poll(outcome, steps[outcome], stop=stop, wait_ms=50)
        validate_deferred_dispatches(held_name, held, steps[outcome]["dispatches"])
        validate_deferred_poll_before_delivery(held_name, held)
    return {key: [step[key] for step in steps.values()]
            for key in ("dispatches", "result", "spans", "measurements", "remote_cancel_calls")} | {
                "held_result_states": [steps[name]["result_state"] for name in held_names],
            }


def normalize_deferred_handle_expiry(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    if list(steps) != ["before-expiry", "at-expiry", "without-sdk-clock"]:
        raise ValueError("expected a fetch just before expiry, at expiry and without an SDK clock")
    expiry = 1_700_000_000_000
    handle = DEFERRED_HANDLE | {"expiresAt": expiry}
    sdk_clock_readings = (None, None, None) if upstream else (expiry - 1, expiry, None)
    ready = expected_faux_deferred_ready()
    for (name, step), clock in zip(steps.items(), sdk_clock_readings):
        supplied_time = step["sdk_now_ms"]
        if (step["request_handle"] != handle or supplied_time != clock
                or (clock is not None and type(supplied_time) is not int)):
            raise ValueError(f"{name}: expected the same expiry metadata and the selected SDK clock")
        rejected = not upstream and name == "at-expiry"
        calls = int(not rejected)
        for key, count in (("auth_calls", calls), ("fetch_count", calls), ("remote_cancel_calls", 0)):
            if type(step[key]) is not int or step[key] != count:
                raise ValueError(f"{name}: unexpected {key} at the expiry boundary")
        if rejected:
            if step["dispatches"] != []:
                raise ValueError(f"{name}: expiry must reject before dispatch")
            result = expected_deferred_result("error")
            diagnostic = f"invalid provider configuration: deferred handle 'deferred-1' expired at {expiry} (now {expiry})"
            delivery, cause = "typed-err", "Config"
            status = {"status": "error", "error": {"name": "provider_error", "message": diagnostic}}
            events = ("pi.ai.first_chunk",)
        else:
            validate_deferred_dispatches(name, step, [{"handle": handle, "wait_ms": 0}])
            result = ready
            delivery, cause, diagnostic = ("fulfilled-message" if upstream else "ok"), None, None
            status = {"status": "ok"}
            events = ("pi.ai.first_chunk", "pi.ai.block")
        if (step["result_delivery"], step.get("typed_cause"), step["error_message"]) != (delivery, cause, diagnostic):
            raise ValueError(f"{name}: unexpected result channel, cause or expiry diagnostic")
        if step["result"] != result:
            raise ValueError(f"{name}: unexpected public result at the expiry boundary")
        validate_deferred_poll_telemetry(name, step, result["stop"], result["usage"], status=status, events=events)

    observed_fields = ("auth_calls", "fetch_count", "dispatches", "result", "spans", "measurements")
    controls = {name: {key: steps[name][key] for key in observed_fields}
                for name in ("before-expiry", "without-sdk-clock")}
    boundary = steps["at-expiry"]
    return {
        "request_handle": handle, "controls": controls,
        "remote_cancel_calls": [step["remote_cancel_calls"] for step in steps.values()],
    }, {key: boundary[key] for key in (*observed_fields, "sdk_now_ms", "result_delivery", "error_message")} | {
        "typed_cause": boundary.get("typed_cause"),
    }


def normalize_deferred_handle_route_mismatch(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    changes = {
        "provider-mismatch": {"provider": "other-provider"},
        "model-mismatch": {"modelId": "other-model"},
        "api-mismatch": {"api": "other-api"},
        "original-handle-refetch": {},
    }
    if list(steps) != list(changes):
        raise ValueError("expected separate provider, model and API mismatches followed by the original handle")
    errors, differences = {}, {}
    for name, change in changes.items():
        step = steps[name]
        handle = DEFERRED_HANDLE | change
        if step["request_handle"] != handle or step["sdk_now_ms"] is not None:
            raise ValueError(f"{name}: expected a single handle route change with the original job ID and no SDK clock")
        valid = not change
        reaches_backend = upstream or valid
        calls = int(reaches_backend)
        for key, count in (("auth_calls", calls), ("fetch_count", calls), ("remote_cancel_calls", 0)):
            if type(step[key]) is not int or step[key] != count:
                raise ValueError(f"{name}: unexpected {key} for the submitted handle")
        if reaches_backend:
            validate_deferred_dispatches(name, step, [{"handle": handle, "wait_ms": 0}])
            delivery = "fulfilled-message" if upstream else "ok"
            cause = None
        else:
            if step["dispatches"] != []:
                raise ValueError(f"{name}: the SDK must reject the route before dispatch")
            delivery, cause = "typed-err", "Config"

        route = DEFERRED_ROUTE if reaches_backend else {"provider": handle["provider"], "model": handle["modelId"]}
        api = DEFERRED_API if reaches_backend else handle["api"]
        if valid:
            result = expected_faux_deferred_ready()
            validation = None if upstream else "ok"
            diagnostic = None
        else:
            result = expected_deferred_result("error") | route
            if upstream:
                validation = None
                diagnostic = "Unknown faux deferred response: deferred-1"
            elif name == "model-mismatch":
                validation = "RouteMismatch"
                diagnostic = ("invalid provider configuration: deferred handle route mismatch: "
                              "handle lifecycle-differential/other-model/scripted-deferred does not match route "
                              "lifecycle-differential/mock-model/scripted-deferred")
            else:
                validation = "Unsupported"
                diagnostic = ("invalid provider configuration: no deferred backend is registered for "
                              f"provider '{handle['provider']}' and API '{handle['api']}'")
        if (step.get("handle_validation"), step["result_delivery"], step.get("typed_cause"), step["error_message"]) != (
            validation, delivery, cause, diagnostic,
        ):
            raise ValueError(f"{name}: unexpected validation, fetch delivery, cause or diagnostic")
        if step["result"] != result:
            raise ValueError(f"{name}: unexpected result or route identity")
        status = {"status": "ok"} if valid else {
            "status": "error", "error": {"name": "provider_error", "message": diagnostic},
        }
        events = ("pi.ai.first_chunk", "pi.ai.block") if valid else ("pi.ai.first_chunk",)
        validate_deferred_poll_telemetry(name, step, result["stop"], result["usage"],
                                         status=status, events=events, route=route, api=api)
        if not valid:
            span, = step["spans"]
            errors[name] = {
                "request_handle": step["request_handle"],
                "result": {key: step["result"][key] for key in ("stop", "content", "deferred", "usage")},
                "poll_events": span["events"], "metric_names": [row["name"] for row in step["measurements"]],
            }
            differences[name] = {
                **{key: step[key] for key in ("auth_calls", "fetch_count", "dispatches", "result_delivery", "error_message")},
                "handle_validation": step.get("handle_validation"), "typed_cause": step.get("typed_cause"),
                "message_route": {key: step["result"][key] for key in DEFERRED_ROUTE},
                "poll_route": {key: span["attributes"][key] for key in ("provider", "model", "api")},
            }
    ready = steps["original-handle-refetch"]
    return {
        "failed_calls": errors,
        "original_handle_refetch": {key: ready[key] for key in (
            "request_handle", "auth_calls", "fetch_count", "dispatches", "result", "spans", "measurements",
        )},
        "remote_cancel_calls": [step["remote_cancel_calls"] for step in steps.values()],
    }, differences


def normalize_deferred_unknown_job_id(steps: dict[str, dict], *, upstream: bool) -> tuple[dict, dict]:
    unknown_id = "deferred-missing"
    requests = {
        "unknown-job-id": DEFERRED_HANDLE | {"id": unknown_id},
        "original-handle-refetch": DEFERRED_HANDLE,
    }
    if list(steps) != list(requests):
        raise ValueError("expected one fetch of an unknown job ID followed by a fetch of the original handle")
    observed = {}
    for name, handle in requests.items():
        step = steps[name]
        if step["request_handle"] != handle or step["sdk_now_ms"] is not None:
            raise ValueError(f"{name}: expected the selected job ID on the original route with no SDK clock")
        if step.get("handle_validation") != (None if upstream else "ok"):
            raise ValueError(f"{name}: the submitted handle must pass local route validation")
        for key, count in (("auth_calls", 1), ("fetch_count", 1), ("remote_cancel_calls", 0)):
            if type(step[key]) is not int or step[key] != count:
                raise ValueError(f"{name}: unexpected {key} for the submitted handle")
        validate_deferred_dispatches(name, step, [{"handle": handle, "wait_ms": 0}])

        if name == "unknown-job-id":
            result = expected_deferred_result("error")
            delivery, cause = ("fulfilled-message", None) if upstream else ("typed-err", "Config")
            diagnostic = (f"Unknown faux deferred response: {unknown_id}" if upstream else
                          f"invalid provider configuration: Unknown scripted deferred response: {unknown_id}")
            status = {"status": "error", "error": {"name": "provider_error", "message": diagnostic}}
            events = ("pi.ai.first_chunk",)
        else:
            result = expected_faux_deferred_ready()
            delivery, cause, diagnostic = ("fulfilled-message" if upstream else "ok"), None, None
            status = {"status": "ok"}
            events = ("pi.ai.first_chunk", "pi.ai.block")
        if (step["result_delivery"], step.get("typed_cause"), step["error_message"]) != (delivery, cause, diagnostic):
            raise ValueError(f"{name}: unexpected result channel, backend cause or diagnostic")
        if step["result"] != result:
            raise ValueError(f"{name}: unexpected result or route identity")
        validate_deferred_poll_telemetry(name, step, result["stop"], result["usage"], status=status, events=events)

        span, = step["spans"]
        shared_status = status if diagnostic is None else {
            "status": status["status"], "error": {"name": status["error"]["name"]},
        }
        observed[name] = {key: step[key] for key in (
            "request_handle", "auth_calls", "fetch_count", "dispatches", "remote_cancel_calls", "result", "measurements",
        )} | {"span": span | {"status": shared_status}}
    failure = steps["unknown-job-id"]
    return observed, {
        "result_delivery": failure["result_delivery"], "typed_cause": failure.get("typed_cause"),
        "error_message": failure["error_message"],
    }


def normalize_deferred_stream_source_cleanup(
    steps: dict[str, dict], *, upstream: bool, observes_drop_finish: bool = False,
) -> tuple[dict, dict]:
    # Each row specifies delivery, EOF gate entry, natural EOF, source Drop and poll settlement.
    checkpoints = [
        ("terminal-gate-held", "start", False, False, False, False),
        ("done-observed", "done", upstream, False, False, False),
    ]
    if upstream:
        checkpoints.append(("public-eof-observed", "eof", True, False, False, False))
    else:
        checkpoints.extend([
            ("next-eof-gate-held", "pending", True, False, False, False),
            ("public-eof-observed", "eof", True, True, True, True),
        ])
    checkpoints.append(("source-released", "result", True, True, True, True))
    drop_settled = observes_drop_finish and not upstream
    if not upstream:
        checkpoints.extend([
            ("started-terminal-gate-held", "start", False, False, False, False),
            ("started-done-observed", "done", True, False, False, False),
            ("started-next-eof-gate-held", "pending", True, False, False, False),
            ("started-public-eof-observed", "eof", True, True, True, True),
            ("started-result-observed", "result", True, True, True, True),
            ("consumer-dropped-eof-gate-held", "detached", True, False, False, False),
            ("consumer-dropped-source-released", "detached", True, True, True, True),
            ("before-start-done-observed", "done", False, False, False, False),
            ("cached-terminal-eof-gate-held", "pending", True, False, False, False),
            ("cached-terminal-public-eof-observed", "eof", True, True, True, True),
            ("cached-terminal-result-observed", "result", True, True, True, True),
        ])
    stops = ("done", "pending-tail", "public-eof") if drop_settled else ("done", "public-eof")
    for stop_after in stops:
        prefix = f"stop-after-{stop_after}"
        checkpoints.extend([
            (f"{prefix}-terminal-gate-held", "start", False, False, False, False),
            (f"{prefix}-done-observed", "done", upstream, False, False, False),
        ])
        tail_polled = stop_after == "pending-tail"
        if tail_polled:
            checkpoints.append((f"{prefix}-next-eof-gate-held", "pending", True, False, False, False))
        if stop_after == "public-eof" and not upstream:
            checkpoints.extend([
                (f"{prefix}-next-eof-gate-held", "pending", True, False, False, False),
                (f"{prefix}-public-eof-observed", "eof", True, True, True, True),
                (f"{prefix}-consumer-stopped", "stopped", True, True, True, True),
            ])
            continue
        if stop_after == "public-eof":
            checkpoints.append((f"{prefix}-public-eof-observed", "eof", True, False, False, False))
        gate_reached = upstream or tail_polled
        checkpoints.extend([
            (f"{prefix}-consumer-stopped", "stopped", gate_reached, False, not upstream, drop_settled),
            (f"{prefix}-after-eof-release", "stopped", gate_reached, upstream, True, upstream or drop_settled),
        ])
    if not upstream:
        checkpoints.extend([
            ("cancel-after-done-terminal-gate-held", "start", False, False, False, False),
            ("cancel-after-done-done-observed", "done", False, False, False, False),
            ("cancel-after-done-next-eof-gate-held", "pending", True, False, False, False),
            ("cancel-after-done-public-eof-observed", "eof", True, False, True, True),
            ("cancel-after-done-source-released", "result", True, False, True, True),
        ])
        for resume in ("result", "started", "detached"):
            prefix = f"resume-{resume}"
            detached = resume == "detached"
            checkpoints.extend([
                (f"{prefix}-terminal-gate-held", "start", False, False, False, False),
                (f"{prefix}-done-observed", "done", False, False, False, False),
                (f"{prefix}-next-dropped-eof-gate-held", "pending", True, False, False, False),
                (f"{prefix}-resumed-eof-gate-held", "detached" if detached else "pending", True, False, False, False),
            ])
            if resume == "started":
                checkpoints.append((f"{prefix}-public-eof-observed", "eof", True, True, True, True))
            checkpoints.append((f"{prefix}-source-released", "detached" if detached else "result", True, True, True, True))
    if list(steps) != [name for name, *_ in checkpoints]:
        raise ValueError("expected ordered deferred stream and source cleanup checkpoints")

    ready = expected_deferred_result("stop")
    final = steps["source-released"]
    validate_deferred_poll_telemetry("source-released", final, "stop", ready["usage"], status={"status": "ok"})
    states = {}
    for name, kind, gate_reached, eof, dropped, settled in checkpoints:
        step = steps[name]
        delivery = {"kind": kind}
        if kind in ("done", "result"):
            delivery["message"] = ready
        if kind == "done":
            delivery["reason"] = "stop"
        if step["delivery"] != delivery:
            raise ValueError(f"{name}: unexpected public deferred stream delivery")
        if step["eof_gate_reached"] is not gate_reached:
            raise ValueError(f"{name}: unexpected source EOF gate entry")
        validate_deferred_dispatches(name, step, [{"handle": DEFERRED_HANDLE, "wait_ms": 0}])
        if not upstream or name.startswith("stop-after-"):
            if type(step["auth_calls"]) is not int or step["auth_calls"] != 1:
                raise ValueError(f"{name}: expected exactly one deferred auth resolution")
        cancelled = None
        if name.startswith("cancel-after-done-"):
            cancelled = kind in ("eof", "result")
        elif name.startswith("resume-"):
            cancelled = False
        validate_deferred_cancellation(name, step, upstream=upstream, cancelled=cancelled)
        observed = step["observed"]
        validate_source(name, observed, started=True, terminal_emitted=kind != "start", eof=eof,
                        upstream=upstream, dropped=dropped)
        if upstream:
            validate_pi_cleanup(name, observed, returned=False, eof=eof)
        if drop_settled and name.startswith("stop-after-"):
            finishes = step["poll_finish_source_dropped"]
            valid = (isinstance(finishes, list) and len(finishes) == int(settled)
                     and all(source_dropped is True for source_dropped in finishes))
            if not valid:
                raise ValueError(f"{name}: expected exactly one poll finish after source release at settlement")
        if settled:
            if (step["spans"], step["measurements"]) != (final["spans"], final["measurements"]):
                raise ValueError(f"{name}: expected the complete settled poll record")
        else:
            validate_open_deferred_poll(name, step, final)
        states[name] = {
            "eof_gate_reached": step["eof_gate_reached"],
            "source_eof": observed["source_eof"],
            "poll_settled": step["spans"][0]["settled"],
        } | ({"source_finally_calls": observed["source_finally_calls"]} if upstream
             else {"source_dropped": observed["source_dropped"]})
        if cancelled is not None:
            states[name]["local_cancelled"] = step["local_cancelled"]
    # Compare the eager Pi entry with the started Rust entry.
    public_delivery_names = (
        ("terminal-gate-held", "done-observed", "public-eof-observed", "source-released")
        if upstream else
        ("started-terminal-gate-held", "started-done-observed", "started-public-eof-observed", "started-result-observed")
    )
    return {
        "dispatches": final["dispatches"],
        "public_deliveries": [steps[name]["delivery"] for name in public_delivery_names],
        "spans": final["spans"], "measurements": final["measurements"],
    }, states


def compare_traces(upstream: dict[str, Any], rust: dict[str, Any], scenario: str) -> dict[str, Any]:
    if scenario not in SCENARIOS:
        return {"status": "fail", "invalid_trace": f"unknown scenario: {scenario}"}
    try:
        if upstream["id"] != scenario or upstream["sourceAssertions"] != "passed":
            raise ValueError("upstream scenario/assertion identity mismatch")
        schemas = (2, 3) if scenario == DEFERRED_STREAM_SOURCE_CLEANUP else (2,)
        if rust["schemaVersion"] not in schemas or rust["runtime"] != "rust" or rust["scenario"] != scenario:
            raise ValueError("Rust trace identity/schema mismatch")
        upstream_steps = indexed_steps(upstream["trace"], "step")
        rust_steps = indexed_steps(rust["steps"], "name")
        if scenario in SETUP_CASES:
            expected, upstream_details = normalize_setup(
                upstream_steps, scenario, event_delivery="error-event", result_delivery="fulfilled-message",
            )
            actual, rust_details = normalize_setup(
                rust_steps, scenario, event_delivery="stream-item-err", result_delivery="typed-err",
            )
            difference = {
                "id": "setup-error-channel", "phase": expected["failure_phase"],
                "upstream": upstream_details, "rust": rust_details,
            }
        elif scenario == DEFERRED_WAIT0:
            expected, upstream_state = normalize_deferred_wait0(upstream_steps)
            actual, rust_state = normalize_deferred_wait0(rust_steps)
            difference = None if upstream_state == rust_state else {
                "id": "deferred-poll-telemetry-at-result",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario in (DEFERRED_RESULT_BEFORE_EOF, DEFERRED_READY_BEFORE_EOF):
            expected, upstream_state = normalize_deferred_result_before_eof(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_result_before_eof(rust_steps, upstream=False)
            if scenario == DEFERRED_READY_BEFORE_EOF:
                if upstream_state["result_state"] != "ready" or rust_state["result_state"] != "ready":
                    raise ValueError("both readiness results must be available while source EOF is held")
                expected["result_while_eof_held"] = upstream_state["result"]
                actual["result_while_eof_held"] = rust_state["result"]
            difference = None if upstream_state == rust_state else {
                "id": "deferred-result-before-source-eof",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario == DEFERRED_LOCAL_CANCEL_AFTER_DONE:
            expected, upstream_state = normalize_deferred_local_cancel_after_done(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_local_cancel_after_done(rust_steps, upstream=False)
            difference = None if upstream_state == rust_state else {
                "id": "deferred-local-cancel-after-done",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario == DEFERRED_LOCAL_CANCEL_BEFORE_DONE:
            expected, upstream_state = normalize_deferred_local_cancel_before_done(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_local_cancel_before_done(rust_steps, upstream=False)
            difference = {
                "id": "deferred-local-cancel-before-done",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario == DEFERRED_REMOTE_CANCEL:
            expected, upstream_state = normalize_deferred_remote_cancel(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_remote_cancel(rust_steps, upstream=False)
            difference = {
                "id": "deferred-refetch-after-remote-cancel",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario == DEFERRED_AUTH_SETUP_FAILURE:
            expected, upstream_state = normalize_deferred_auth_setup_failure(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_auth_setup_failure(rust_steps, upstream=False)
            difference = {
                "id": "deferred-auth-setup-failure",
                "upstream": upstream_state, "rust": rust_state,
            }
        elif scenario == DEFERRED_POSITIVE_WAIT:
            expected = normalize_deferred_positive_wait(upstream_steps)
            actual = normalize_deferred_positive_wait(rust_steps)
            difference = None
        elif scenario == DEFERRED_HANDLE_EXPIRY:
            expected, upstream_state = normalize_deferred_handle_expiry(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_handle_expiry(rust_steps, upstream=False)
            difference = {"id": "deferred-handle-expiry", "upstream": upstream_state, "rust": rust_state}
        elif scenario == DEFERRED_HANDLE_ROUTE_MISMATCH:
            expected, upstream_state = normalize_deferred_handle_route_mismatch(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_handle_route_mismatch(rust_steps, upstream=False)
            difference = {"id": "deferred-handle-route-mismatch", "upstream": upstream_state, "rust": rust_state}
        elif scenario == DEFERRED_UNKNOWN_JOB_ID:
            expected, upstream_state = normalize_deferred_unknown_job_id(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_unknown_job_id(rust_steps, upstream=False)
            difference = {"id": "deferred-unknown-job-id", "upstream": upstream_state, "rust": rust_state}
        elif scenario == DEFERRED_STREAM_SOURCE_CLEANUP:
            expected, upstream_state = normalize_deferred_stream_source_cleanup(upstream_steps, upstream=True)
            actual, rust_state = normalize_deferred_stream_source_cleanup(
                rust_steps, upstream=False, observes_drop_finish=rust["schemaVersion"] == 3,
            )
            difference = {"id": "deferred-stream-source-cleanup", "upstream": upstream_state, "rust": rust_state}
        else:
            details = rust["runtimeDetails"]
            if details["consumer_handle_dropped"] is not False or details["result_observed"] is not True:
                raise ValueError("comparison requires a retained handle and an observed public result")
            if details["source_dropped"] is not True:
                raise ValueError("Rust source cleanup was not observed")
            if scenario == FIRST_CHUNK_WRITE_FAILURE:
                expected, upstream_state = normalize_first_chunk_failure(upstream_steps, upstream=True)
                actual, rust_state = normalize_first_chunk_failure(rust_steps, upstream=False)
                difference_id = "first-chunk-event-write-failure"
            elif scenario == HISTOGRAM_CREATION_FAILURE:
                expected, upstream_state = normalize_histogram_creation_failure(upstream_steps, upstream=True)
                actual, rust_state = normalize_histogram_creation_failure(rust_steps, upstream=False)
                difference_id = "request-duration-histogram-creation-failure"
            elif scenario == REQUEST_SPAN_START_FAILURE:
                expected, upstream_state = normalize_request_span_start_failure(upstream_steps, upstream=True)
                actual, rust_state = normalize_request_span_start_failure(rust_steps, upstream=False)
                difference_id = "request-span-start-failure"
            elif scenario == FIRST_CHUNK_DURATION_FAILURE:
                expected, upstream_state = normalize_first_chunk_duration_failure(upstream_steps, upstream=True)
                actual, rust_state = normalize_first_chunk_duration_failure(rust_steps, upstream=False)
                difference_id = "first-chunk-duration-record-failure"
            else:
                expected, upstream_state = normalize_completion(upstream_steps, scenario, upstream=True)
                actual, rust_state = normalize_completion(rust_steps, scenario, upstream=False)
                fault_step = RESULT_FAULT_CHECKPOINTS.get(scenario)
                difference_id = fault_step.removesuffix("-observed") if fault_step else EOF_CHECKPOINTS.get(scenario)
            difference = None
            if upstream_state != rust_state:
                difference = {
                    "id": difference_id, "upstream": upstream_state, "rust": rust_state,
                }
        keys = tuple(expected)
        mismatches = [
            {"field": key, "upstream": expected[key], "rust": actual[key]}
            for key in keys
            if type(expected[key]) is not type(actual[key]) or expected[key] != actual[key]
        ]
        if mismatches:
            return {"status": "fail", "mismatches": mismatches}
        comparison = {
            "status": "pass",
            "shared_observations": "pass",
            "compared_fields": list(keys),
            "mismatches": [],
            "strict_alignment": False,
        }
        if scenario == TERMINAL_AND_SPAN_BEFORE_EOF:
            comparison["eof_checkpoint"] = {"upstream": upstream_state, "rust": rust_state}
        if difference is not None:
            comparison.update({
                "status": "difference", "difference": difference, "decision_status": "not-approved-by-this-probe",
            })
        return comparison
    except (KeyError, TypeError, ValueError, IndexError) as error:
        return {"status": "fail", "invalid_trace": str(error)}

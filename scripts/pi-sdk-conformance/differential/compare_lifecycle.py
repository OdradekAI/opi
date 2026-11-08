from __future__ import annotations

from typing import Any


SCENARIOS = {
    "lazy-no-consumption": "lazy-no-consumption",
    "lazy-stop-consumption": "lazy-stop-consumption",
    "lazy-setup-failure": "lazy-setup-failure-phases",
}
COMPLETION_KEYS = (
    "setup_calls",
    "source_started",
    "terminal_before_release",
    "source_terminal_count",
    "source_completed",
    "consumer_polled",
    "terminal_kind",
)


def normalize_upstream(raw: dict[str, Any], scenario: str) -> dict[str, Any]:
    if raw["id"] != SCENARIOS[scenario] or raw["sourceAssertions"] != "passed":
        raise ValueError("upstream scenario/assertion identity mismatch")
    steps = {step["step"]: step for step in raw["trace"]}
    if len(steps) != len(raw["trace"]):
        raise ValueError("duplicate upstream checkpoints")
    if scenario == "lazy-setup-failure":
        setup = steps["setup-failures"]
        if setup["delivered"] != ["error"]:
            raise ValueError("setup rejection must deliver exactly one error")
        return {"terminal_kind": setup["rejectedStop"], "terminal_count": len(setup["delivered"])}

    gate = steps["source-at-gate-without-outer-poll"]["counters"]
    finished = steps["source-completed-after-gate"]
    consumer_polled = "outer-iterator-return" in steps
    if consumer_polled != (scenario == "lazy-stop-consumption"):
        raise ValueError("consumer checkpoint does not match scenario")
    before = steps["outer-iterator-return"]["counters"] if consumer_polled else gate
    if before["sourceReturn"] != 0 or finished["counters"]["sourceReturn"] != 0:
        raise ValueError("consumer return unexpectedly closed upstream source")
    if not finished["sameResult"]:
        raise ValueError("upstream did not retain the source result")
    return {
        "setup_calls": steps["immediately-after-call"]["counters"]["setup"],
        "source_started": gate["iterators"] == 1,
        "terminal_before_release": before["sourceTerminal"],
        "source_terminal_count": finished["counters"]["sourceTerminal"],
        "source_completed": finished["counters"]["sourceTerminal"] == 1,
        "consumer_polled": consumer_polled,
        "terminal_kind": finished["resultStop"],
    }


def normalize_rust(raw: dict[str, Any], scenario: str) -> dict[str, Any]:
    if raw["schemaVersion"] != 2 or raw["runtime"] != "rust" or raw["scenario"] != scenario:
        raise ValueError("Rust trace identity/schema mismatch")
    if scenario != "lazy-setup-failure":
        details = raw["runtimeDetails"]
        if details["consumer_handle_dropped"] or not details["result_observed"]:
            raise ValueError("comparison requires a retained handle and an observed public result")
        if not details["source_dropped"]:
            raise ValueError("Rust source cleanup was not observed")
        steps = {step["name"]: step for step in raw["steps"]}
        required = ["constructed", "source-at-gate", "before-source-release", "source-released", "result-observed"]
        if scenario == "lazy-stop-consumption":
            required.insert(2, "consumer-stopped-after-start")
        if list(steps) != required or len(steps) != len(raw["steps"]):
            raise ValueError("missing, duplicate or reordered Rust checkpoints")
        before = steps["before-source-release"]["observed"]
        if before["terminal_count"] != 0 or before["source_eof"] or before["source_dropped"]:
            raise ValueError("source already terminated before consumer-stop barrier")
        after = steps["source-released"]["observed"]
        if not after["source_dropped"]:
            raise ValueError("source release was not observed")
        if after["source_eof"] != raw["shared"]["source_completed"]:
            raise ValueError("summary disagrees with observed source EOF")
        if steps["result-observed"]["stop"] != raw["shared"]["terminal_kind"]:
            raise ValueError("summary disagrees with observed public result")
    elif not raw["shared"]["result_matches_event"]:
        raise ValueError("Rust final failure differs from its single event")
    return raw["shared"]


def compare_traces(upstream: dict[str, Any], rust: dict[str, Any], scenario: str) -> dict[str, Any]:
    if scenario not in SCENARIOS:
        return {"status": "fail", "invalid_trace": f"unknown scenario: {scenario}"}
    try:
        expected = normalize_upstream(upstream, scenario)
        actual = normalize_rust(rust, scenario)
        keys = ("terminal_kind", "terminal_count") if scenario == "lazy-setup-failure" else COMPLETION_KEYS
        mismatches = [
            {"field": key, "upstream": expected[key], "rust": actual[key]}
            for key in keys
            if type(expected[key]) is not type(actual[key]) or expected[key] != actual[key]
        ]
        if mismatches:
            return {"status": "fail", "mismatches": mismatches}
        if scenario == "lazy-setup-failure":
            difference = {
                "id": "setup-error-channel",
                "upstream_synchronous_throw": upstream["trace"][0]["synchronousError"],
                "upstream_rejected_message": upstream["trace"][0]["rejectedMessage"],
                "rust": rust["runtimeDetails"],
                "scope": "Rejected setup vs unregistered-route setup: only single-error termination is compared, not identical failure injection or error payload equivalence.",
            }
        else:
            difference = {
                "id": "setup-start-timing",
                "upstream_setup_calls_at_construction": upstream["trace"][0]["counters"]["setup"],
                "rust_setup_calls_at_construction": rust["steps"][0]["observed"]["setup_calls"],
            }
        return {
            "status": "difference",
            "shared_observations": "pass",
            "compared_fields": list(keys),
            "mismatches": [],
            "difference": difference,
            "decision_status": "not-approved-by-this-probe",
            "strict_alignment": False,
        }
    except (KeyError, TypeError, ValueError, IndexError) as error:
        return {"status": "fail", "invalid_trace": str(error)}

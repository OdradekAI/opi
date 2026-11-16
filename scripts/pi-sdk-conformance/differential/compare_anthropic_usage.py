"""Check accounting facts independently of either adapter; retain ownership differences."""
from collections import Counter
from math import isclose

ROUTES = ("simple", "prepared-observed", "prepared-raw", "direct")
ENDINGS = ("success", "wire-error", "callback-error")


def require(ok, message):
    if not ok:
        raise ValueError(message)


def key(row):
    return row["id"], row["ending"], row["route"]


def projection(trace):
    """Timings remain in raw evidence; only duration values vary between runs."""
    import copy
    trace = copy.deepcopy(trace)
    for row in trace["cases"]:
        for metric in row["metrics"]:
            if metric["name"].endswith(".duration"):
                metric["value"] = "measured"
    return trace


def compare(fixtures, pi, rust):
    expected_pi = {(c["id"], e, r) for c in fixtures for e in ENDINGS for r in ("simple", "direct")}
    expected_rust = {(c["id"], e, r) for c in fixtures for e in ENDINGS for r in ROUTES}
    require(Counter(map(key, pi["cases"])) == Counter(expected_pi), "Pi case coverage")
    require(Counter(map(key, rust["cases"])) == Counter(expected_rust), "Rust case coverage")
    fixture_map = {c["id"]: c for c in fixtures}
    differences = []
    for runtime, trace in (("pi", pi), ("rust", rust)):
        for row in trace["cases"]:
            c = fixture_map[row["id"]]
            label = (runtime, *key(row))
            counts = c["expected"]
            cost = sum(n * (i + 1) / 1e6 for i, n in enumerate(counts))
            observed = row["route"] in ("simple", "prepared-observed")
            fault = c.get("observer_fault", False) and observed
            lost = runtime == "pi" and fault
            result = row["result"]
            stop = "error" if lost or row["ending"] != "success" else "stop"
            require(result["usage"] == ([0]*4 if lost else counts), f"usage {label}")
            require(isclose(result["cost"], 0 if lost else cost, abs_tol=1e-12, rel_tol=0), f"cost {label}")
            require(result["text"] == ("" if lost else "AB"), f"text {label}")
            require(result["stop"] == stop and row["delivery"] == ("error" if stop == "error" else "ok"), f"outcome {label}")
            require(result["response_id"] == (None if lost else "usage-response"), f"response identity {label}")
            require(result["raw_stop"] == (None if lost else "end_turn"), f"raw stop {label}")
            require(row["requests"] == 1, f"dispatch count {label}")
            events = row["events"]
            terminal = "error" if stop == "error" else "done"
            require([e["kind"] for e in events] == (["error"] if lost else ["text_delta","text_delta","text_end",terminal]), f"business event order {label}")
            require(events[-1]["message"] == result, f"terminal event/result {label}")
            if not lost:
                require([e["delta"] for e in events[:2]] == ["A","B"], f"text deltas {label}")
                for index, event in enumerate(events[:-1]):
                    message = event["message"]
                    require(message["text"] == ("A" if index == 0 else "AB"), f"event text {label}")
                    # The first text callback barrier fixes its read before B.
                    # Later Pi reads can already see silent changes through the
                    # shared object; Rust carries immutable event-time values.
                    initial = message["usage"] == [11,7,5,3] and message["stop"] == "pending" and message["raw_stop"] is None and isclose(message["cost"],52e-6,abs_tol=1e-12,rel_tol=0)
                    updated = message["usage"] == counts and isclose(message["cost"],cost,abs_tol=1e-12,rel_tol=0) and message["raw_stop"] == "end_turn" and message["stop"] in ("stop",stop)
                    earlier_stop = c.get("after_stop",False) and message["usage"] == [11,7,5,3] and isclose(message["cost"],52e-6,abs_tol=1e-12,rel_tol=0) and message["raw_stop"] == "end_turn" and message["stop"] == "stop"
                    require(initial or (runtime == "pi" and index > 0 and (updated or earlier_stop)), f"event accounting {label}/{index}")
            held = row["held"]
            if lost:
                require(held is None, f"Pi fault must precede text {label}")
                require(row["source_result"]["text"] == "AB" and row["source_result"]["usage"] == counts, f"source completion {label}")
                differences.append({"case": list(key(row)), "contract": "#48 observer failure isolation", "pi": result})
            elif runtime == "pi":
                require(held == result, f"Pi shared partial {label}")
            else:
                require(held["text"] == "A" and held["usage"] == [11,7,5,3] and held["stop"] == "pending", f"Rust owned partial {label}")
                require(isclose(held["cost"], 52e-6, abs_tol=1e-12, rel_tol=0), f"start cost {label}")
            spans, metrics = row["spans"], row["metrics"]
            require(len(spans) == int(observed), f"observer ownership {label}")
            if not observed:
                require(not metrics, f"raw route metrics {label}")
                continue
            span = spans[0]
            require(span["name"] == "pi.ai.request" and span["ended"], f"observer settlement {label}")
            if fault and runtime == "rust":
                require(not metrics and "usage.input" not in span["attributes"] and not span["events"], f"disabled observer {label}")
                continue
            attrs = span["attributes"]
            require(attrs["stop"] == stop, f"span stop {label}")
            expected_counts = [0]*4 if lost else counts
            for k, n in zip(("input","output","cache_read","cache_write"),expected_counts):
                require(attrs[f"usage.{k}"] == n, f"span usage {label}/{k}")
                selected = [m for m in metrics if m["name"] == "pi.ai.tokens" and m["attributes"]["kind"] == k]
                require(len(selected) == 1 and selected[0]["value"] == n, f"token metric {label}/{k}")
            expected_cost = 0 if lost else cost
            require(isclose(attrs["cost.total"],expected_cost,abs_tol=1e-12,rel_tol=0), f"span cost {label}")
            amounts = [m["value"] for m in metrics if m["name"] == "pi.ai.cost"]
            require(len(amounts) == 1 and isclose(amounts[0],expected_cost,abs_tol=1e-12,rel_tol=0), f"cost metric {label}")
            expected_names = {"pi.ai.tokens":4,"pi.ai.cost":1,"pi.ai.request.duration":1}
            if not lost: expected_names["pi.ai.first_chunk.duration"] = 1
            require(Counter(m["name"] for m in metrics) == expected_names, f"metric multiplicity {label}")
            require([e["name"] for e in span["events"]] == ([] if lost else ["pi.ai.first_chunk","pi.ai.block"]), f"block observation {label}")
            if not lost:
                require(span["events"][1]["attributes"] == {"type":"text","index":0,"chars":2}, f"block attributes {label}")
    return {"status":"PASS_WITH_RETAINED_DIFFERENCES", "rust_routes":len(rust["cases"]),
            "pi_routes":len(pi["cases"]),"observer_fault_differences":differences,
            "partial_ownership":"Pi shared reference versus Rust event-time owned snapshot (#103/public contract)",
            "strict_alignment":False}

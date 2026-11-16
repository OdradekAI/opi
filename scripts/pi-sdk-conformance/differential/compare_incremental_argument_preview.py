"""Fail-closed trace comparisons; execution/material provenance is a separate gate."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import struct
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def json_equal(left, right):
    """JSON numeric equality, with booleans separate and non-finite inputs rejected."""
    if type(left) in (int, float) and type(right) in (int, float):
        return math.isfinite(left) and math.isfinite(right) and left == right
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(json_equal(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(json_equal(a, b) for a, b in zip(left, right))
    return left == right


def equal(left, right, where):
    require(json_equal(left, right), where)


def compare_numeric(rust, pi, callers):
    """Exact finite controls and observed real caller values; not policy equivalence."""
    controls = [
        ("1" + "0" * 64, "4d384f03e93ff9f5"), ("1e64", "4d384f03e93ff9f5"),
        ("-1e64", "cd384f03e93ff9f5"), ("1.2345678901234567", "3ff3c0ca428c59fb"),
        ("2.2250738585072014e-308", "0010000000000000"),
        ("2.4703282292062327e-324", "0000000000000000"),
        ("2.4703282292062328e-324", "0000000000000001"),
        ("1.7976931348623157e308", "7fefffffffffffff"),
        ("-1e-400", "8000000000000000"), ("18446744073709551616", "43f0000000000000"),
    ]
    require(len(rust["final_numbers"]) == len(pi["final_numbers"]) == len(controls), "numeric inventory")
    for (literal, bits), a, b in zip(controls, rust["final_numbers"], pi["final_numbers"], strict=True):
        require(a["raw"] == b["raw"] == '{"n":' + literal + '}', "numeric raw input")
        require(a["bits"] == b["bits"] == b["rust_serialization_bits"] == bits, "numeric bit mismatch")
        value = json.loads(a["serialized"], parse_int=float, parse_float=float)
        require(set(value) == {"n"} and type(value["n"]) is float
                and struct.pack(">d", value["n"]).hex() == bits, "numeric serialization")

    expected = "4d384f03e93ff9f5"
    def stages(rows):
        require(len(rows) == 4 and {r["stage"] for r in rows} == {"hook", "authorize", "execute", "event"},
                "Rust caller stage inventory")
        require(all(r["bits"] == expected for r in rows), "Rust caller bits")
    stages(callers["agent_final_number"])
    stages(callers["bounded_agent_final_number"])
    session = callers["session_final_number"]
    stages(session["next_execution"])
    require(type(session["restored_calls"]) is int and session["restored_calls"] == 2, "Rust restored calls")
    equal(json.loads(session["persisted"]), {"n": 1e64, "password": "[REDACTED]"}, "Rust persisted value/redaction")
    require([(r["stage"], r["id"]) for r in pi["agent"]]
            == [(stage, call) for call in ("initial", "after-resume") for stage in ("hook", "execute")],
            "Pi caller stages")
    require(all(r["bits"] == expected for r in pi["agent"]), "Pi caller bits")
    require(pi["session"]["restored_bits"] == expected
            and type(pi["session"]["executions_before_resume"]) is int
            and type(pi["session"]["executions_after_next_call"]) is int
            and pi["session"]["executions_before_resume"] == 1
            and pi["session"]["executions_after_next_call"] == 2, "Pi session execution boundary")
    return {"status": "FINAL_NUMERIC_DECODING_VERIFIED", "finite_controls": len(controls),
            "numeric_alignment": True, "rust_stages_per_call": 4,
            "scope": "Observed numeric values through actual dispatch and disk recovery; no policy/whole-Pi-Agent equivalence"}


# The exact eighteen remaining stream differences in the verified #110 receipt.
CHAT_DIFFERENCES = {
    "missing-members": [1, 2, 4], "split-key": [0, 1, 2, 3, 4],
    "empty-key": [0, 1], "nested": [1, 4], "fraction-exponent": [0, 1],
    "escape": [2, 3], "space-and-control": [], "invalid-escape": [],
    "duplicate-key": [1, 2], "array-literals": [],
}


def compare_chat(fixture, pi, old, current):
    """old is #110's repaired Rust trace, not its pre-literal-repair trace."""
    require(isinstance(fixture, list), "chat fixture must be a case list")
    names = [case["name"] for case in fixture]
    equal(names, list(CHAT_DIFFERENCES), "chat fixture inventory changed")
    for trace in (pi, old, current):
        equal([case["name"] for case in trace["cases"]], names, "chat trace inventory")
    closed = 0
    checkpoints = 0
    cases = []
    for test, upstream, before, after in zip(fixture, pi["cases"], old["cases"], current["cases"], strict=True):
        name = test["name"]
        pieces = test["pieces"]
        require(pieces and all(isinstance(piece, str) for piece in pieces), f"{name}: pieces")
        for trace in (upstream, before, after):
            require(len(trace["previews"]) == len(pieces), f"{name}: missing checkpoint")
            equal(trace["final"], test["final"], f"{name}: final")
        require(len(upstream["utility"]) == len(pieces), f"{name}: utility inventory")
        equal(upstream["request"], before["request"], f"{name}: old request")
        equal(upstream["request"], after["request"], f"{name}: new request")
        durable = upstream["durable"]
        equal(durable["pi"], durable["opi"], f"{name}: Durable projection")
        require(len(durable["pi"]["progress"]) == len(pieces), f"{name}: Durable checkpoints")
        equal(durable["pi"]["final"], test["final"], f"{name}: Durable final")
        prefix = ""
        differences = []
        for index, piece in enumerate(pieces):
            prefix += piece
            a, b = before["previews"][index], after["previews"][index]
            require(a["raw"] == b["raw"] == prefix, f"{name}/{index}: raw")
            equal(b["legacy_preview"], a["preview"], f"{name}/{index}: legacy helper changed")
            equal(b["preview"], upstream["previews"][index], f"{name}/{index}: incremental preview")
            if not json_equal(a["preview"], upstream["previews"][index]):
                differences.append(index)
            checkpoints += 1
        equal(differences, CHAT_DIFFERENCES[name], f"{name}: #110 difference inventory")
        closed += len(differences)
        cases.append({"name": name, "checkpoints": len(pieces), "closed": differences})
    require(checkpoints == 65 and closed == 18, "expected exactly 65 checkpoints and 18 closures")
    return {"status": "CHAT_INCREMENTAL_PREVIEW_VERIFIED", "strict_alignment": True,
            "checkpoints": checkpoints, "closed_checkpoints": closed, "cases": cases}


def lone_surrogate(value):
    if isinstance(value, str):
        index = 0
        while index < len(value):
            code = ord(value[index])
            if 0xD800 <= code <= 0xDBFF:
                if index + 1 == len(value) or not 0xDC00 <= ord(value[index + 1]) <= 0xDFFF:
                    return True
                index += 1
            elif 0xDC00 <= code <= 0xDFFF:
                return True
            index += 1
        return False
    if isinstance(value, dict):
        return any(lone_surrogate(k) or lone_surrogate(v) for k, v in value.items())
    if isinstance(value, (list, tuple)):
        return any(lone_surrogate(v) for v in value)
    return False


def raw_lone_surrogate(raw):
    # Keep all key/value pairs: an earlier duplicate may contain an unpaired unit.
    try:
        parsed = json.loads(raw, object_pairs_hook=lambda pairs: pairs,
                            parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))
    except (ValueError, RecursionError):
        return False
    return lone_surrogate(parsed)


def key_order(value, path=()):
    rows = []
    if isinstance(value, dict):
        rows.append({"path": list(path), "keys": list(value)})
        for key, child in value.items():
            rows.extend(key_order(child, (*path, key)))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            rows.extend(key_order(child, (*path, index)))
    return rows


def negative_zeros(value, path=()):
    if type(value) in (int, float):
        return [list(path)] if value == 0 and math.copysign(1, value) < 0 else []
    if isinstance(value, dict):
        return [p for key, child in value.items() for p in negative_zeros(child, (*path, key))]
    if isinstance(value, list):
        return [p for index, child in enumerate(value) for p in negative_zeros(child, (*path, index))]
    return []


def at_path(value, path):
    require(isinstance(path, list), "metadata path must be a list")
    for part in path:
        if isinstance(value, dict):
            require(isinstance(part, str) and part in value, "metadata object path")
        elif isinstance(value, list):
            require(type(part) is int and 0 <= part < len(value), "metadata array path")
        else:
            raise ValueError("metadata path enters scalar")
        value = value[part]
    return value


def validate_metadata(row, where):
    preview = row["preview"]
    equal(row["key_order"], key_order(preview), where + ": key order does not describe serialized preview")
    for record in row["key_order"]:
        require(set(record) == {"path", "keys"} and isinstance(at_path(preview, record["path"]), dict),
                where + ": key-order object path")
    paths = row["negative_zero_paths"]
    require(isinstance(paths, list), where + ": negative zero metadata")
    seen = set()
    for path in paths:
        encoded = json.dumps(path, ensure_ascii=True)
        require(encoded not in seen, where + ": duplicate negative zero path")
        seen.add(encoded)
        value = at_path(preview, path)
        require(type(value) in (int, float) and value == 0, where + ": negative zero is not a zero")
    rows = row["non_finite"]
    require(isinstance(rows, list), where + ": non-finite metadata")
    seen.clear()
    for record in rows:
        require(set(record) == {"path", "kind"}, where + ": non-finite fields")
        require(record["kind"] in ("Infinity", "-Infinity"), where + ": non-finite kind")
        encoded = json.dumps(record["path"], ensure_ascii=True)
        require(encoded not in seen, where + ": duplicate non-finite path")
        seen.add(encoded)
        require(at_path(preview, record["path"]) is None, where + ": serialized non-finite must be null")


# Exact operation controls prevent a fixture flag from exempting arbitrary drift.
NON_FINITE_CONTROLS = {'non-finite': [{'op': 'append', 'text': '{"n":1e'},
                {'op': 'append', 'text': '400,"m":-1e400,"z":1e-400}'},
                {'op': 'finish'}],
 'non-finite-array': [{'op': 'append', 'text': '{"a":[7'},
                      {'op': 'append', 'text': ',1e'},
                      {'op': 'append', 'text': '400'},
                      {'op': 'append', 'text': ',false]}'},
                      {'op': 'finish'},
                      {'op': 'replace', 'text': '{"a":[7,8,false]}'},
                      {'op': 'finish'}],
 'non-finite-duplicate': [{'op': 'append', 'text': '{"n":2,'},
                          {'op': 'append', 'text': '"n":1e400,"other":true,'},
                          {'op': 'snapshot'},
                          {'op': 'append', 'text': '"n":f'},
                          {'op': 'append', 'text': 'alse}'},
                          {'op': 'finish'}],
 'non-finite-isolation': [{'op': 'append', 'text': '{"ok":1}', 'tool': 0},
                          {'op': 'append', 'text': '{"n":-1e400}', 'tool': 1},
                          {'op': 'replace', 'text': '{"n":1e400}', 'tool': 0},
                          {'op': 'replace', 'text': '{"ok":2}', 'tool': 1},
                          {'op': 'snapshot', 'tool': 0},
                          {'op': 'replace', 'text': '{"ok":3}', 'tool': 0}]}
NON_FINITE_PREVIEWS = {'non-finite': [{'n': 1}, {'n': math.inf, 'm': -math.inf, 'z': 0}, {'n': math.inf, 'm': -math.inf, 'z': 0}],
 'non-finite-array': [{'a': [7]},
                      {'a': [7, 1]},
                      {'a': [7, math.inf]},
                      {'a': [7, math.inf, False]},
                      {'a': [7, math.inf, False]},
                      {'a': [7, 8, False]},
                      {'a': [7, 8, False]}],
 'non-finite-duplicate': [{'n': 2},
                          {'n': math.inf, 'other': True},
                          {'n': math.inf, 'other': True},
                          {'n': False, 'other': True},
                          {'n': False, 'other': True},
                          {'n': False, 'other': True}],
 'non-finite-isolation': [{'ok': 1}, {'n': -math.inf}, {'n': math.inf}, {'ok': 2}, {'n': math.inf}, {'ok': 3}]}
DEPTH_RAW = '{"a":' * 129 + "0" + "}" * 129
LONG_SCALAR_RAW = '{"n":1' + "0" * 64 + "}"


def depth(value):
    if isinstance(value, dict):
        return 1 + max((depth(child) for child in value.values()), default=0)
    if isinstance(value, list):
        return 1 + max((depth(child) for child in value), default=0)
    return 0


def finite_projection(value, path=()):
    """Serialize a known reference value without hiding overflow path or sign."""
    if type(value) is float and not math.isfinite(value):
        return None, [{"path": list(path), "kind": "Infinity" if value > 0 else "-Infinity"}]
    if isinstance(value, dict):
        projected, rows = {}, []
        for key, child in value.items():
            projected[key], child_rows = finite_projection(child, (*path, key))
            rows.extend(child_rows)
        return projected, rows
    if isinstance(value, list):
        projected, rows = [], []
        for index, child in enumerate(value):
            item, child_rows = finite_projection(child, (*path, index))
            projected.append(item)
            rows.extend(child_rows)
        return projected, rows
    return value, []


def non_finite_control(test, row, index):
    name = test["name"]
    equal(test["operations"], NON_FINITE_CONTROLS[name], "non-finite fixture changed")
    value, rows = finite_projection(NON_FINITE_PREVIEWS[name][index])
    equal(row["preview"], value, "non-finite Pi standard projection")
    equal(row["non_finite"], rows, "non-finite metadata mismatch")


def raw_non_finite(raw):
    try:
        # All duplicate values matter to strict Rust final parsing.
        parsed = json.loads(raw, object_pairs_hook=lambda pairs: [v for _, v in pairs],
                            parse_int=float, parse_float=float, parse_constant=reject_constant)
    except (ValueError, RecursionError):
        return False
    def contains(value):
        return (type(value) is float and not math.isfinite(value)) or (
            isinstance(value, list) and any(contains(v) for v in value))
    return contains(parsed)


def compare_final(test, a, b, where, differences):
    for final in (a, b):
        require({"valid", "raw", "value"} <= set(final)
                and set(final) <= {"valid", "raw", "value", "non_finite", "negative_zero_paths"},
                where + ": final fields")
        require(type(final["valid"]) is bool and isinstance(final["raw"], str), where + ": final types")
        if not final["valid"]:
            require(final["value"] is None and final.get("non_finite", []) == [], where + ": invalid final value")
    require(a["raw"] == b["raw"], where + ": final raw differs")
    raw = a["raw"]
    nf = a.get("non_finite", [])
    equal(b.get("non_finite", []), [], where + ": Rust final non-finite")
    if nf or raw_non_finite(raw):
        require(test["name"] in NON_FINITE_CONTROLS, where + ": unexpected non-finite final")
        equal(test["operations"], NON_FINITE_CONTROLS[test["name"]], where + ": non-finite final fixture")
        require(raw_non_finite(raw), where + ": non-finite final raw")
        value, rows = finite_projection(json.loads(raw, parse_int=float, parse_float=float))
        equal(nf, rows, where + ": non-finite final metadata")
        equal(a["value"], value, where + ": non-finite final projection")
        require(a["valid"] and not b["valid"], where + ": non-finite final validity")
        differences.append({"at": where, "kind": "non_finite_final_rejected"})
        return
    if a["valid"] != b["valid"]:
        require(a["valid"] and not b["valid"], where + ": unexpected final validity direction")
        if raw_lone_surrogate(raw):
            # Parse all duplicate members for the guard, then last-key-wins for value.
            equal(a["value"], json.loads(raw), where + ": Unicode final value")
            kind = "unpaired_surrogate_final_rejected"
        else:
            require(test["name"] == "depth-boundary" and raw == DEPTH_RAW,
                    where + ": unexpected final validity difference")
            parsed = json.loads(raw)
            require(depth(parsed) > 128, where + ": depth guard not exercised")
            equal(a["value"], parsed, where + ": deep final value")
            kind = "depth_limit_final_rejected"
        differences.append({"at": where, "kind": kind})
        return
    if test["name"] == "long-scalar":
        require(raw == LONG_SCALAR_RAW and a["valid"] and b["valid"],
                where + ": long-scalar final input/validity")
        equal(a["value"], {"n": 1e64}, where + ": Pi long scalar")
    equal(a["value"], b["value"], where + ": final numeric/value decoding")


def compare_interface(fixture, pi, rust):
    require(isinstance(fixture, dict), "interface fixture must be an object")
    tests = fixture["cases"]
    names = [case["name"] for case in tests]
    require(names and len(names) == len(set(names)), "empty or duplicate interface cases")
    for trace in (pi, rust):
        equal([case["name"] for case in trace["cases"]], names, "interface case inventory")
    differences = []
    checkpoints = 0
    for test, upstream, local in zip(tests, pi["cases"], rust["cases"], strict=True):
        name = test["name"]
        actions = test["operations"]
        require(actions, name + ": empty operations")
        if name == "depth-boundary":
            equal(actions, [{"op": "append", "text": DEPTH_RAW}, {"op": "finish"}], "depth fixture changed")
        for trace in (upstream, local):
            require(len(trace["rows"]) == len(actions), name + ": missing operation")
            equal(trace["retained"], [row["preview"] for row in trace["rows"]], name + ": retained snapshot drift")
        states = {}
        for index, (action, a, b) in enumerate(zip(actions, upstream["rows"], local["rows"], strict=True)):
            where = f"{name}/{index}"
            op = action["op"]
            tool = action.get("tool", 0)
            require(type(tool) is int and tool >= 0, where + ": tool identity")
            require(op in ("append", "replace", "snapshot", "finish"), where + ": operation")
            require(set(action) <= {"op", "tool", "text"}, where + ": action fields")
            state = states.setdefault(tool, {"raw": "", "preview": {}, "negative_zero_paths": []})
            raw, suffix = state["raw"], None
            if op in ("append", "replace"):
                text = action["text"]
                require(isinstance(text, str), where + ": operation text")
                if op == "append":
                    raw += text
                else:
                    suffix = text[len(raw):] if text.startswith(raw) else None
                    if suffix is None:
                        state["preview"], state["negative_zero_paths"] = {}, []
                    raw = text
            else:
                require("text" not in action, where + ": unexpected operation text")
            for row in (a, b):
                expected_fields = {"op", "tool", "raw", "preview", "suffix", "key_order", "non_finite", "negative_zero_paths"}
                if op == "finish":
                    expected_fields.add("final")
                require(set(row) == expected_fields, where + ": row fields")
                require(type(row["tool"]) is int, where + ": tool must be an integer")
                equal([row["op"], row["tool"], row["raw"], row["suffix"]], [op, tool, raw, suffix], where + ": operation/raw/suffix")
                validate_metadata(row, where)
            equal(b["non_finite"], [], where + ": Rust non-finite metadata")
            equal(b["negative_zero_paths"], negative_zeros(b["preview"]), where + ": Rust signed-zero metadata")
            projected = a["preview"]
            zeros = a["negative_zero_paths"]
            if name in NON_FINITE_CONTROLS:
                non_finite_control(test, a, index)
                if a["non_finite"]:
                    projected, zeros = state["preview"], state["negative_zero_paths"]
                    differences.append({"at": where, "kind": "non_finite_previous_snapshot"})
            else:
                equal(a["non_finite"], [], where + ": unexpected non-finite preview")
            if name == "depth-boundary":
                require(raw == DEPTH_RAW and depth(json.loads(raw)) > 128, where + ": depth input")
                equal(a["preview"], json.loads(raw), where + ": Pi deep preview")
                projected, zeros = {}, []
                differences.append({"at": where, "kind": "depth_limit_preview_retained"})
            elif lone_surrogate(a["preview"]):
                projected, zeros = state["preview"], state["negative_zero_paths"]
                differences.append({"at": where, "kind": "unpaired_surrogate_previous_snapshot"})
            equal(b["preview"], projected, where + ": preview")
            equal(b["key_order"], key_order(projected), where + ": preview key order")
            equal(b["negative_zero_paths"], zeros, where + ": negative zero paths")
            require(not lone_surrogate(b["preview"]), where + ": Rust preview contains an unpaired surrogate")
            if op == "finish":
                if name in NON_FINITE_CONTROLS or name == "long-scalar":
                    require(a["final"]["raw"] == b["final"]["raw"] == raw,
                            where + ": controlled final raw differs from accumulated input")
                compare_final(test, a["final"], b["final"], where, differences)
            state.update(raw=raw, preview=copy.deepcopy(projected), negative_zero_paths=copy.deepcopy(zeros))
            checkpoints += 1
    require(fixture["large_array_items"] == 1200, "large-array fixture must exercise 1200 items")
    for trace in (pi, rust):
        large = trace["large_array"]
        require(set(large) == {"lengths", "credit", "final_length"}, "large-array fields")
        require(len(large["lengths"]) == 1200 and large["final_length"] == 1200, "large-array inventory")
        require(all(type(value) is int and 0 <= value <= index + 1 for index, value in enumerate(large["lengths"])), "large-array lengths")
        credit = large["credit"]
        require(set(credit) == {"received", "published", "repeated", "identical_replace", "empty_append"}, "copy-credit fields")
        require(all(type(value) is int for value in credit.values()), "copy-credit types")
        require(0 < credit["published"] < credit["received"] <= 1200
                and credit["published"] == credit["repeated"] == credit["identical_replace"]
                and credit["published"] <= credit["empty_append"] <= credit["received"], "copy-credit exhaustion")
        require(large["lengths"][credit["received"] - 1] == credit["published"], "copy-credit checkpoint")
    equal(pi["large_array"], rust["large_array"], "large-array length/credit difference")
    return {"status": "INTERFACE_VERIFIED_WITH_RECORDED_REPRESENTATION_BOUNDARIES" if differences else "INTERFACE_VERIFIED",
            "strict_alignment": not differences, "cases": len(tests), "checkpoints": checkpoints,
            "differences": differences, "large_array_items": 1200,
            "cost_scope": "cost_observations are not timing/performance acceptance and are not compared"}


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reject_constant(value):
    raise ValueError("non-JSON number in trace: " + value)


def load_trace(path):
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate object key in trace")
            result[key] = value
        return result

    def finite_float(text):
        value = float(text)
        require(math.isfinite(value), "non-finite JSON number in trace")
        return value

    return json.loads(path.read_text(encoding="utf-8"), parse_constant=reject_constant,
                      parse_float=finite_float, object_pairs_hook=unique_object)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("fixture", "pi", "rust", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        require(not args.output.exists(), "output already exists")
        paths = [args.fixture.resolve(), args.pi.resolve(), args.rust.resolve()]
        require(len(set(paths)) == len(paths) and args.output.resolve() not in paths, "input/output path alias")
        inputs = {str(path): file_hash(path) for path in paths}
        result = compare_interface(*(load_trace(path) for path in paths))
        require(all(file_hash(Path(path)) == digest for path, digest in inputs.items()), "input drift")
        with args.output.open("x", encoding="utf-8", newline="\n") as output:
            json.dump({"scope": "Trace comparison only; material and execution provenance are separate.",
                       "inputs": inputs, "comparison": result}, output, ensure_ascii=True, indent=2, allow_nan=False)
            output.write("\n")
        print(result["status"])
        return 0
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError, RecursionError) as error:
        print("rejected: " + str(error))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

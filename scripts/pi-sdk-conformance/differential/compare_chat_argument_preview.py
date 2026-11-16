"""Compare event-time previews without treating an observed difference as approval."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import file_hash


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


EXPECTED_REPAIRS = {"array-literals": [4, 7]}


def json_equal(left, right) -> bool:
    """Compare JSON values while keeping booleans distinct from numbers."""
    if type(left) in (int, float) and type(right) in (int, float):
        return left == right
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(json_equal(left[key], right[key]) for key in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(json_equal(a, b) for a, b in zip(left, right))
    return left == right


def compare(fixture: list[dict], pi: dict, before: dict, after: dict,
            expected_repairs: dict[str, list[int]] = EXPECTED_REPAIRS) -> dict:
    names = [case["name"] for case in fixture]
    require(bool(names) and len(set(names)) == len(names), "empty or duplicate fixture cases")
    require(expected_repairs.keys() <= set(names), "missing expected repair case")
    for trace in (pi, before, after):
        require([case["name"] for case in trace["cases"]] == names, "case inventory mismatch")
    cases = []
    repaired = 0
    for test, upstream, old, new in zip(fixture, pi["cases"], before["cases"], after["cases"], strict=True):
        count = len(test["pieces"])
        require(count > 0, "empty piece inventory")
        for trace in (upstream, old, new):
            require(len(trace["previews"]) == count, "missing preview checkpoint")
            require(json_equal(trace["final"], test["final"]), "final arguments mismatch")
        require(len(upstream["utility"]) == count, "missing utility checkpoint")
        require(json_equal(upstream["request"], old["request"]) and json_equal(old["request"], new["request"]), "request mismatch")
        durable = upstream["durable"]
        require(json_equal(durable["pi"], durable["opi"]), "Durable commit mismatch")
        require(len(durable["pi"]["progress"]) == count, "missing Durable commit")
        require(json_equal(durable["pi"]["final"], test["final"]), "Durable final mismatch")
        prefix = ""
        differences = []
        fixes = []
        utility_differences = []
        for index, piece in enumerate(test["pieces"]):
            prefix += piece
            a, b = old["previews"][index], new["previews"][index]
            require(a["raw"] == b["raw"] == prefix, "raw argument loss")
            if not json_equal(a["preview"], b["preview"]):
                require(any(prefix.endswith(word) for word in ("true", "false", "null")), "unrelated preview change")
                require(json_equal(b["preview"], upstream["previews"][index]) and
                        json_equal(b["preview"], upstream["utility"][index]), "literal repair mismatch")
                fixes.append(index)
                repaired += 1
            if not json_equal(b["preview"], upstream["previews"][index]):
                differences.append(index)
            if not json_equal(b["preview"], upstream["utility"][index]):
                utility_differences.append(index)
        require(fixes == expected_repairs.get(test["name"], []), "expected literal repair missing or unexpected change")
        cases.append({"name": test["name"], "checkpoints": count, "repaired": fixes,
                      "stream_differences": differences, "utility_differences": utility_differences})
    require(repaired > 0, "no repaired literal observed")
    return {"status": "LITERAL_REPAIR_VERIFIED_WITH_REMAINING_DIFFERENCES",
            "strict_alignment": False, "repaired_checkpoints": repaired, "cases": cases}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("fixture", "pi", "before", "after", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        require(not args.output.exists(), "output already exists")
        paths = [args.fixture, args.pi, args.before, args.after]
        inputs = {str(path.resolve()): file_hash(path) for path in paths}
        result = compare(*(json.loads(path.read_bytes()) for path in paths))
        require(all(file_hash(Path(path)) == digest for path, digest in inputs.items()), "input drift")
        with args.output.open("x", encoding="utf-8", newline="\n") as output:
            json.dump({"scope": "Trace comparison only; material and execution provenance are separate.",
                       "inputs": inputs, "comparison": result}, output, indent=2)
            output.write("\n")
        print(result["status"])
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"rejected: {error}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

"""Compare the frozen Pi and Rust nested Anthropic usage traces."""
import argparse
import hashlib
import json
from pathlib import Path

EXPECTED_USAGE = [100, 20, 0, 200, 75, 6]


def trace(path):
    rows = [
        json.loads(line)
        for line in Path(path).read_text(encoding="utf-8-sig").splitlines()
        if line.startswith('{"cases":')
    ]
    if len(rows) != 1:
        raise ValueError(f"expected one trace row in {path}")
    return rows[0]


def compare(pi_path, rust_path):
    pi = trace(pi_path)
    rust = trace(rust_path)
    expected_pi = {(route, ending) for route in ("simple", "direct") for ending in ("success", "wire-error")}
    if {(row["route"], row["ending"]) for row in pi["cases"]} != expected_pi:
        raise ValueError("Pi nested case coverage")
    if {row["ending"] for row in rust["cases"]} != {"success", "wire-error"}:
        raise ValueError("Rust nested case coverage")
    for row in pi["cases"]:
        result = row["result"]
        if result["usage"] != EXPECTED_USAGE or row["source_result"] != result:
            raise ValueError(f"Pi nested usage/result mismatch: {row}")
        if result["raw_stop"] != "end_turn":
            raise ValueError(f"Pi nested raw stop mismatch: {row}")
        if result["stop"] != ("stop" if row["ending"] == "success" else "error"):
            raise ValueError(f"Pi nested terminal mismatch: {row}")
        if abs(result["cost"]["total"] - 0.00079) > 1e-15:
            raise ValueError(f"Pi nested cost mismatch: {row}")
    for row in rust["cases"]:
        result = row["result"]
        if result["usage"] != EXPECTED_USAGE or result["raw_stop"] != "end_turn":
            raise ValueError(f"Rust nested usage mismatch: {row}")
        if result["stop"] != "stop" or result["cost"] != 0.0:
            raise ValueError(f"Rust nested low-level terminal/cost mismatch: {row}")
    return {
        "status": "PASS_WITH_LOW_LEVEL_COST_SCOPE",
        "pi_cases": len(pi["cases"]),
        "rust_cases": len(rust["cases"]),
        "expected_nested_usage": EXPECTED_USAGE,
        "pi_cost_total": 0.00079,
        "rust_low_level_cost_total": 0.0,
        "differences": [
            {
                "field": "stop",
                "ending": "wire-error",
                "rust": "stop",
                "pi": "error",
                "scope": "Raw AnthropicMapper retains an accepted stop on a later wire Error; canonical ChatStream/Agent failure remains error.",
            },
            {
                "field": "cost",
                "scope": "The low-level mapper test has no ModelInfo/catalog; actual Agent route/session regression verifies catalog cost separately.",
                "rust": 0.0,
                "pi": 0.00079,
            },
        ],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pi", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite evidence")
    result = compare(args.pi, args.rust)
    result["inputs"] = {
        str(path): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in (args.pi, args.rust)
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result))


if __name__ == "__main__":
    main()

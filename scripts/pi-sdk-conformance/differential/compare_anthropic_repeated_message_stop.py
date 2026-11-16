"""Pair frozen Pi Anthropic message_stop tails with Opi."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

PI_REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
PI_PROBE = Path("docs/iterations/opi-ai-1-5-alignment/tools/observe_anthropic_repeated_message_stop.mjs")
PI_SOURCES = ("packages/ai/src/api/anthropic-messages.ts",)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def run(command: list[str], cwd: Path) -> dict:
    result = subprocess.run(command, cwd=cwd, capture_output=True, check=True, timeout=60)
    return {"command": command, "exit_code": result.returncode, "stdout": result.stdout.decode(), "stderr": result.stderr.decode()}


def trace(text: str) -> dict:
    rows = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
    require(len(rows) == 1, f"expected one JSON trace row, got {len(rows)}")
    return rows[0]


def normalized(value: dict) -> dict:
    return {
        "case": value["case"],
        "cases": [
            {key: row[key] for key in ("route", "stop", "raw_stop", "usage", "done_count", "event_types", "requests")}
            for row in value["cases"]
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--snapshot", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--rust-executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    repo, snapshot, fixture, rust = (p.resolve() for p in (args.repo, args.snapshot, args.fixture, args.rust_executable))
    fixture_value = json.loads(fixture.read_text(encoding="utf-8"))
    for source in PI_SOURCES:
        require((snapshot / source).is_file(), f"missing frozen Pi source: {source}")
    inputs = {
        str(path): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in (repo / PI_PROBE, fixture, Path(__file__), rust, *(snapshot / source for source in PI_SOURCES))
    }
    pi_command = ["node", "--conditions=source", str(repo / PI_PROBE), str(snapshot), str(fixture)]
    rust_command = [str(rust), "--exact", "repeated_message_stop_usage_is_deferred_until_eof", "--nocapture"]
    pi_runs = [run(pi_command, repo) for _ in range(3)]
    rust_runs = [run(rust_command, repo) for _ in range(3)]
    for row in pi_runs + rust_runs:
        row["trace"] = trace(row["stdout"])
    require(all(row["trace"] == pi_runs[0]["trace"] for row in pi_runs), "Pi trace is unstable")
    require(all(row["trace"] == rust_runs[0]["trace"] for row in rust_runs), "Rust trace is unstable")
    pi, rust_trace = pi_runs[0]["trace"], rust_runs[0]["trace"]
    for label, value in (("Pi", pi), ("Rust", rust_trace)):
        require(value["case"] == fixture_value["id"], f"{label} case identity")
        require({row["route"] for row in value["cases"]} == {"simple", "direct"}, f"{label} route coverage")
        for row in value["cases"]:
            require(row["stop"] == fixture_value["expected"]["stop"], f"{label} stop")
            require(row["raw_stop"] == fixture_value["expected"]["raw_stop"], f"{label} raw stop")
            require(row["usage"] == {"input": 10, "output": 7}, f"{label} usage")
            require(row["done_count"] == 1 and row["event_types"] == ["start", "done"], f"{label} terminal sequence")
            require(row["requests"] == 1, f"{label} request count")
    for row in pi["cases"]:
        require(row["provider_events"] == [event["type"] for event in fixture_value["events"]], "Pi raw observer order")
    require(normalized(pi) == normalized(rust_trace), "frozen Pi and Rust normalized traces differ")
    result = {
        "schemaVersion": 1,
        "status": "PASS",
        "revision": PI_REVISION,
        "scope": "Anthropic message_stop repeated usage and tail observer ordering on simple and direct routes",
        "inputs": inputs,
        "expected": fixture_value["expected"],
        "pi_runs": pi_runs,
        "rust_runs": rust_runs,
        "differences": ["Pi exposes provider_events from every source item; Rust's direct fixture helper exposes normalized event_types only."],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": result["status"], "runs": 3}))


if __name__ == "__main__":
    main()

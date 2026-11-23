"""Pair frozen Pi Bedrock repeated-metadata handling with Opi."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from common import DIFFERENTIAL_PATH

PI_REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
PI_PROBE = DIFFERENTIAL_PATH / "tools/observe_bedrock_repeated_metadata.mjs"
PI_LOADER = DIFFERENTIAL_PATH / "tools/bedrock_mock_loader.mjs"
PI_MOCK = DIFFERENTIAL_PATH / "tools/bedrock_mock_runtime.mjs"
PI_SOURCES = (
    "packages/ai/src/api/bedrock-converse-stream.ts",
    "packages/ai/src/utils/event-stream.ts",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_one_json_line(text: str) -> dict:
    rows = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
    require(len(rows) == 1, f"expected one JSON trace row, got {len(rows)}")
    return rows[0]


def run(command: list[str], cwd: Path) -> dict:
    result = subprocess.run(command, cwd=cwd, capture_output=True, check=True, timeout=120)
    return {
        "command": command,
        "exit_code": result.returncode,
        "stdout": result.stdout.decode("utf-8"),
        "stderr": result.stderr.decode("utf-8"),
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

    repo = args.repo.resolve()
    snapshot = args.snapshot.resolve()
    fixture = args.fixture.resolve()
    rust_executable = args.rust_executable.resolve()
    fixture_value = json.loads(fixture.read_text(encoding="utf-8"))
    expected = fixture_value["expected"]
    for name in PI_SOURCES:
        require((snapshot / name).is_file(), f"missing frozen Pi source: {name}")
    inputs = {
        str(path): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in (
            repo / PI_PROBE,
            repo / PI_LOADER,
            repo / PI_MOCK,
            fixture,
            Path(__file__),
            rust_executable,
            *(snapshot / name for name in PI_SOURCES),
        )
    }

    pi_command = [
        "node",
        "--experimental-loader",
        (repo / PI_LOADER).as_uri(),
        str(repo / PI_PROBE),
        str(snapshot),
        str(fixture),
    ]
    rust_command = [
        str(rust_executable),
        "--exact",
        "repeated_metadata_updates_pending_done_before_eof",
        "--nocapture",
    ]
    pi_runs = [run(pi_command, repo) for _ in range(3)]
    rust_runs = [run(rust_command, repo) for _ in range(3)]
    for row in pi_runs:
        row["trace"] = read_one_json_line(row["stdout"])
    for row in rust_runs:
        row["trace"] = read_one_json_line(row["stdout"])
    for row in pi_runs:
        row["trace"].pop("provider_events", None)

    require(all(row["trace"] == pi_runs[0]["trace"] for row in pi_runs), "Pi trace is unstable")
    require(all(row["trace"] == rust_runs[0]["trace"] for row in rust_runs), "Rust trace is unstable")
    pi = pi_runs[0]["trace"]
    rust = rust_runs[0]["trace"]
    for label, trace in (("Pi", pi), ("Rust", rust)):
        require(trace["case"] == fixture_value["id"], f"{label} case identity")
        require(trace["stop"] == expected["stop"], f"{label} normalized stop")
        require(trace["raw_stop"] == expected["raw_stop"], f"{label} raw stop")
        require(trace["usage"]["input"] == expected["input_tokens"], f"{label} input usage")
        require(trace["usage"]["output"] == expected["output_tokens"], f"{label} output usage")
        require(trace["usage"]["cache_read"] == expected["cache_read_tokens"], f"{label} cache usage")
        require(trace["done_count"] == expected["done_count"], f"{label} done count")
        require(trace["events"] == ["start", "text_start", "text_delta", "text_end", "done"], f"{label} event order")
    require(pi == rust, "frozen Pi and Rust traces differ")
    result = {
        "schemaVersion": 1,
        "status": "PASS",
        "revision": PI_REVISION,
        "scope": "Bedrock messageStop pending terminal, repeated metadata authority, and provider-event ordering.",
        "inputs": inputs,
        "expected": expected,
        "pi_runs": pi_runs,
        "rust_runs": rust_runs,
        "differences": [],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": result["status"], "runs": 3}))


if __name__ == "__main__":
    main()

"""Pair the frozen Pi OpenAI-compatible choice-usage fallback with Opi."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from common import DIFFERENTIAL_PATH

PI_REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
PI_PROBE = DIFFERENTIAL_PATH / "tools/observe_openai_choice_usage.mjs"
PI_SOURCES = (
    "packages/ai/src/api/openai-completions.ts",
    "packages/ai/src/models.ts",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_one_json_line(text: str, prefix: str = "{") -> dict:
    rows = [json.loads(line) for line in text.splitlines() if line.startswith(prefix)]
    require(len(rows) == 1, f"expected one JSON trace row, got {len(rows)}")
    return rows[0]


def run(command: list[str], cwd: Path) -> dict:
    result = subprocess.run(command, cwd=cwd, capture_output=True, check=True, timeout=60)
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
        for path in (repo / PI_PROBE, fixture, Path(__file__), rust_executable, *(snapshot / name for name in PI_SOURCES))
    }

    pi_command = ["node", str(repo / PI_PROBE), str(snapshot), str(fixture)]
    rust_command = [
        str(rust_executable),
        "--exact",
        "choice_level_usage_is_used_when_chunk_usage_is_absent",
        "--nocapture",
    ]
    pi_runs = [run(pi_command, repo) for _ in range(3)]
    rust_runs = [run(rust_command, repo) for _ in range(3)]
    for row in pi_runs:
        row["trace"] = read_one_json_line(row["stdout"])
    for row in rust_runs:
        row["trace"] = read_one_json_line(row["stdout"])

    require(all(row["trace"] == pi_runs[0]["trace"] for row in pi_runs), "Pi trace is unstable")
    require(all(row["trace"] == rust_runs[0]["trace"] for row in rust_runs), "Rust trace is unstable")
    pi = pi_runs[0]["trace"]
    rust = rust_runs[0]["trace"]
    for label, trace in (("Pi", pi), ("Rust", rust)):
        require(trace["case"] == fixture_value["id"], f"{label} case identity")
        require(trace["stop"] == "stop" and trace["raw_stop"] == "stop", f"{label} terminal")
        require(trace["usage"]["input"] == expected["input_tokens"], f"{label} input usage")
        require(trace["usage"]["cache_read"] == expected["cache_read_tokens"], f"{label} cache usage")
        require(trace["usage"]["output"] == expected["output_tokens"], f"{label} output usage")
        require(trace["usage"]["total"] == expected["total_tokens"], f"{label} total usage")
        require(trace["requests"] == 1, f"{label} request count")
    require(pi == rust, "frozen Pi and Rust traces differ")
    result = {
        "schemaVersion": 1,
        "status": "PASS",
        "revision": PI_REVISION,
        "scope": "OpenAI-compatible Chat choice-level usage fallback; top-level usage precedence is covered by the Rust adapter test.",
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

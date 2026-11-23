"""Pair the OpenAI Chat response-observer release trace with frozen Pi 1.5.0."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from common import DIFFERENTIAL_PATH, REPO_ROOT


PI_REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
DEFAULT_SNAPSHOT = REPO_ROOT / ".pi" / "pi-1.5.0"
FIXTURE = REPO_ROOT / "crates/opi-ai/tests/data/openai-chat-response-observer.json"
PROBE = REPO_ROOT / DIFFERENTIAL_PATH / "tools/observe_openai_chat_response_observer.mjs"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_json(command: list[str], cwd: Path) -> dict:
    result = subprocess.run(command, cwd=cwd, check=True, text=True, capture_output=True)
    lines = [line.strip() for line in result.stdout.splitlines() if line.strip().startswith("{")]
    return json.loads(lines[-1])


def normalize(row: dict) -> dict:
    return {
        "observer_statuses": row["observer_statuses"],
        "first": {"stop": row["first"]["stop"], "event_types": row["first"]["event_types"]},
        "second": {"stop": row["second"]["stop"], "usage": row["second"]["usage"]},
        "requests": row["requests"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--snapshot", type=Path, default=DEFAULT_SNAPSHOT)
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    pi_runs = []
    rust_runs = []
    for _ in range(args.repetitions):
        pi_runs.append(run_json(["node", str(PROBE), str(args.snapshot), str(FIXTURE)], REPO_ROOT))
        rust_runs.append(run_json([str(args.rust_exe), "--nocapture"], REPO_ROOT))
    pi_normalized = [normalize(row) for row in pi_runs]
    rust_normalized = [normalize(row) for row in rust_runs]
    assert all(row == pi_normalized[0] for row in pi_normalized), pi_normalized
    assert all(row == rust_normalized[0] for row in rust_normalized), rust_normalized
    assert pi_normalized[0] == rust_normalized[0], (pi_normalized[0], rust_normalized[0])
    expected = {
        "observer_statuses": fixture["expected"]["observer_statuses"],
        "first": {"stop": fixture["expected"]["first_stop"], "event_types": fixture["expected"]["first_event_types"]},
        "second": {"stop": fixture["expected"]["second_stop"], "usage": fixture["expected"]["second_usage"]},
        "requests": fixture["expected"]["requests"],
    }
    assert pi_normalized[0] == expected, (pi_normalized[0], expected)
    source = args.snapshot / "packages" / "ai" / "src" / "api" / "openai-completions.ts"
    receipt = {
        "status": "pass",
        "case": fixture["id"],
        "pi_revision": PI_REVISION,
        "pi_source": str(source.relative_to(REPO_ROOT)),
        "pi_source_sha256": sha256(source),
        "fixture_sha256": sha256(FIXTURE),
        "probe_sha256": sha256(PROBE),
        "rust_executable": str(args.rust_exe),
        "rust_executable_sha256": sha256(args.rust_exe),
        "repetitions": args.repetitions,
        "pi_runs": pi_runs,
        "rust_runs": rust_runs,
        "normalized_match": True,
        "approved_differences": [
            "Pi emits an error AssistantMessage event and resolves result(); Rust exposes a typed CallbackPhase CallFailure.",
            "Pi's event stream includes no Rust-normalized partial message on the callback-failure path.",
        ],
    }
    with args.output.open("x", encoding="utf-8") as output:
        output.write(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"status": "pass", "output": str(args.output), "repetitions": args.repetitions}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

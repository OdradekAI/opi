"""Compare Chat output limits through matched SDK entries on frozen materials."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from common import REPO_ROOT, file_hash, run_json, runtime_environment
from replay_partial_aliases import PI_REVISION
from run_lifecycle_differential import ITERATION, material_tools


FIXTURE = Path("crates/opi-ai/tests/data/openai-chat-output-limits.json")
RUST_PROBE = Path("crates/opi-ai/tests/openai_chat_output_limits.rs")
PI_PROBE = ITERATION / "tools/observe_chat_output_limits.mjs"
PI_SOURCES = (
    "packages/ai/package.json",
    "packages/ai/src/models.ts",
    "packages/ai/src/api/openai-completions.ts",
    "packages/ai/src/api/simple-options.ts",
    "packages/ai/src/utils/transcript.ts",
    "packages/ai/src/utils/estimate.ts",
    "packages/ai/test/openai-completions-empty-tools.test.ts",
)
ENTRIES = ("native", "simple", "direct")


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise ValueError(detail)


def check_build(path: Path, materials: Path) -> dict:
    build = json.loads(path.read_bytes())
    require(build["exit_code"] == 0, "Rust build did not succeed")
    require(build["materials_sha256"] == file_hash(materials), "Rust build materials mismatch")
    for name in (RUST_PROBE, FIXTURE):
        require(build["inputs"][name.as_posix()] == file_hash(REPO_ROOT / name), f"changed Rust input: {name}")
    exe = Path(build["executable"])
    require(file_hash(exe) == build["executable_sha256"], "Rust executable differs from its build receipt")
    artifacts = (json.loads(line) for line in build["stdout"].splitlines() if line.startswith("{"))
    executables = [
        row["executable"] for row in artifacts
        if row.get("reason") == "compiler-artifact" and row["target"]["name"] == RUST_PROBE.stem
        and row["executable"]
    ]
    require(executables == [build["cargo_executable"]], "Rust test executable missing from Cargo output")
    require(exe.name == Path(executables[0]).name, "Rust test executable name differs")
    return {key: build[key] for key in ("command", "executable", "executable_sha256", "inputs")} | {
        "receipt": str(path), "receipt_sha256": file_hash(path), "build_stderr": build["stderr"],
    }


def run_rust(exe: str) -> dict:
    command = [exe, "--exact", "observe_chat_output_limits", "--nocapture"]
    run = subprocess.run(command, cwd=REPO_ROOT, env=runtime_environment(), check=True, capture_output=True, timeout=30)
    stdout = run.stdout.decode("utf-8")
    [trace] = [json.loads(line) for line in stdout.splitlines() if line.startswith("[")]
    return {
        "command": command, "exit_code": run.returncode, "stdout": stdout,
        "stdout_sha256": hashlib.sha256(run.stdout).hexdigest(), "stderr": run.stderr.decode("utf-8"),
        "trace": trace,
    }


def compare(fixture: dict, pi: list, opi: list) -> dict:
    cases = [(field, case, entry) for field in fixture["fields"] for case in fixture["cases"] for entry in ENTRIES]
    require(len(pi) == len(opi) == len(cases), "request pair count differs")
    limits: dict[str, dict] = {}
    for (field, case, entry), upstream, rust in zip(cases, pi, opi, strict=True):
        label = f"{field}/{case['id']}/{entry}"
        require(upstream == rust, f"matched entries differ: {label}")
        require((upstream["field"], upstream["case"], upstream["entry"]) == (field, case["id"], entry),
                f"request identity differs: {label}")
        require(upstream["resolver_calls"] == (0 if entry == "direct" else 1), f"resolver calls differ: {label}")

        messages = [{"role": "user", "content": fixture["input"]}]
        if case["anchor"] is not None:
            messages[:0] = [
                {"role": "user", "content": fixture["history"][0]},
                {"role": "assistant", "content": fixture["history"][1]},
            ]
        text_units = len(fixture["input"].encode("utf-16-le")) // 2
        estimate = (case["anchor"] or 0) + (2 * text_units + 6) // 7
        require(upstream["estimate"] == estimate, f"context estimate differs: {label}")
        limit = case["simple_limit"] if entry == "simple" else case["max_tokens"] or None
        body = {
            "model": fixture["model"]["id"], "messages": messages, "stream": True,
            "store": False, "stream_options": {"include_usage": True},
        }
        if limit is not None:
            body[field] = limit
        require(upstream["request"] == {
            "method": "POST", "path": "/v1/chat/completions",
            "authorization": "Bearer fixture-direct-key" if entry == "direct" else "Bearer fixture-route-key",
            "body": body,
        }, f"wire request differs: {label}")
        stop = "length" if limit == 1 else "stop"
        output_tokens = 1 if stop == "length" else 2
        require(upstream["result"] == {
            "text": "o" if stop == "length" else "ok", "stop": stop, "raw_stop": stop,
            "response_id": "chat-limit", "input_tokens": 10, "output_tokens": output_tokens,
            "total_tokens": 10 + output_tokens, "error": None,
        }, f"terminal result differs: {label}")
        limits.setdefault(field, {}).setdefault(case["id"], {})[entry] = limit
    return {"matched_pairs": len(cases), "output_limits": limits}


def keep_repeated_runs(runs: list[dict], name: str) -> dict:
    require(all(row["trace"] == runs[0]["trace"] for row in runs), f"nondeterministic {name} observations")
    return {
        "first": runs[0],
        "repeated": [
            {key: row[key] for key in ("command", "exit_code", "stdout_sha256", "stderr")} for row in runs[1:]
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("materials", "rust-build", "pi-repo", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")

    repo = REPO_ROOT
    freezer = material_tools(repo)
    materials = args.materials.resolve()
    record = json.loads(materials.read_bytes())
    snapshot = Path(record["snapshot"])
    build = check_build(args.rust_build.resolve(), materials)
    sources = {}
    for name in PI_SOURCES:
        pinned = subprocess.check_output(["git", "-C", str(args.pi_repo), "show", f"{PI_REVISION}:{name}"])
        require((snapshot / name).read_bytes() == pinned, f"pinned Pi source differs: {name}")
        sources[name] = file_hash(snapshot / name)
    inputs = [
        materials, repo / PI_PROBE, repo / RUST_PROBE, repo / FIXTURE, Path(__file__).resolve(),
        Path(build["receipt"]), Path(build["executable"]), *(snapshot / name for name in PI_SOURCES),
    ]
    hashes = {str(path): file_hash(path) for path in inputs}

    def verify() -> None:
        require(all(file_hash(Path(path)) == digest for path, digest in hashes.items()), "replay input drift")
        require(not record["blocked_gates"], "frozen materials blocked")
        errors = freezer.verify_evidence(repo, record)
        require(not errors, f"current materials drift: {errors}")

    verify()
    command = ["node", str(repo / PI_PROBE), str(snapshot), str(repo / FIXTURE)]
    pi = keep_repeated_runs([run_json(command, repo) for _ in range(3)], "Pi")
    opi = keep_repeated_runs([run_rust(build["executable"]) for _ in range(3)], "Opi")
    comparison = compare(json.loads((repo / FIXTURE).read_bytes()), pi["first"]["trace"], opi["first"]["trace"])
    verify()
    freezer.write_new_json(args.output, {
        "schemaVersion": 1, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "scope": "OpenAI Chat output limits for native, simple and injected-auth calls with thinking disabled",
        "snapshot": str(snapshot), "upstream_revision": PI_REVISION, "pi_sources": sources,
        "inputs": hashes, "build": build, "comparison": comparison, "pi": pi, "opi": opi,
    })
    print(f"Verified {comparison['matched_pairs']} matched Chat request pairs in {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

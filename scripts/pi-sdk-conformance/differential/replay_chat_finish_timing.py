"""Verify repaired Chat terminal boundaries against frozen Pi; retain cancellation differences."""
from __future__ import annotations

import argparse
import copy
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory

from common import REPO_ROOT, file_hash, run_json, runtime_environment
from replay_partial_aliases import PI_REVISION
from run_lifecycle_differential import ITERATION, material_tools

FIXTURE = Path("crates/opi-ai/tests/data/openai-chat-finish-timing.json")
RUST_PROBE = Path("crates/opi-ai/tests/openai_chat_finish_timing.rs")
PI_PROBE = ITERATION / "tools/observe_chat_finish_timing.mjs"
PI_SOURCES = (
    "packages/ai/src/models.ts", "packages/ai/src/api/openai-completions.ts",
    "packages/ai/src/utils/event-stream.ts", "packages/durable/src/harness/generation.ts",
    "packages/durable/src/harness/partial-changes.ts",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def message_view(message: dict) -> dict:
    result = copy.deepcopy(message)
    for block in result["content"]:
        block.pop("raw", None)  # Rust retains the original argument JSON too.
    return result


def compare(pi: list, opi: list, fixture: dict) -> dict:
    cases = fixture["cases"]
    require(len(pi) == len(opi) == len(cases), "case count differs")
    summaries = []
    for case, up, rust in zip(cases, pi, opi, strict=True):
        name, usage, tail = (case[key] for key in ("id", "usage", "tail"))
        require(up["case"] == rust["case"] == name, "case identity differs")
        require(up["request"] == rust["request"], f"paired request differs: {name}")
        prefix = ["start", "text_start", "text_delta", "toolcall_start", "toolcall_delta"]
        for checkpoint in ("after_finish", "after_usage"):
            for side, row in (("Pi", up), ("Opi", rust)):
                value = row["checkpoints"][checkpoint]
                require([e["kind"] for e in value["events"]] == prefix
                        and value["result"] is None and not value["eof"] and not value["peer_closed"],
                        f"{side} ended before sentinel: {name}/{checkpoint}")
        settled = tail != "eof"
        for side, row in (("Pi", up), ("Opi", rust)):
            value = row["checkpoints"]["held_http_eof"]
            require(value["eof"] == settled and value["peer_closed"] == settled
                    and (value["result"] is not None) == settled, f"{side} held EOF timing: {name}")
        stop = "error" if tail == "error" else "aborted" if tail == "cancel" else "tool_use"
        expected = {
            "content": [{"type":"text","text":"ready"}, {"type":"toolCall","id":"call_finish","name":"record","arguments":{"a":"ok","n":2}}],
            "stop":stop, "raw_stop":"tool_calls", "response_id":"chat-finish",
            "input_tokens":0 if usage == "none" else 7, "output_tokens":0 if usage == "none" else 3,
        }
        for side, row in (("Pi", up), ("Opi", rust)):
            require(message_view(row["final"]["result"]) == expected, f"{side} final result: {name}")
            require(row["final"]["eof"], f"{side} final EOF: {name}")
            held = row["checkpoints"]["held_http_eof"]
            if settled:
                require(message_view(held["result"]) == expected, f"{side} held terminal result: {name}")
                require(held["events"] == row["final"]["events"], f"{side} events changed after terminal: {name}")
            else:
                require([e["kind"] for e in held["events"]] == prefix, f"{side} blocks ended before natural EOF: {name}")
            # Pi's transport abort unwinds the iterator before closing blocks.
            # Opi retains cancellation priority and never synthesizes block ends.
            closes = tail != "error" and (side == "Pi" or tail != "cancel")
            kinds = prefix + (["text_end", "toolcall_end"] if closes else []) + ["error" if tail in ("error", "cancel") else "done"]
            require([e["kind"] for e in row["final"]["events"]] == kinds, f"{side} terminal event sequence: {name}")
        summaries.append({"case":name,"result_equal":True,"terminal_timing_equal":True,
                          "response_released_at_terminal":True,"block_end_sequence_equal":tail != "cancel"})
    return {"matched_request_pairs":len(cases), "cases":summaries,
            "terminal_contract_verified":True, "strict_alignment":False,
            "decision":"repaired-with-explicit-cancellation-difference",
            "differences":["Pi closes blocks during abort unwind; Opi cancellation preempts buffered events without synthetic ends"],
            "scope":"checkpoint timing, event kinds/order, terminal result and response release; event-time partial aliasing is outside this comparison"}


def check_build(path: Path, materials: Path) -> dict:
    build = json.loads(path.read_bytes())
    require(build["exit_code"] == 0, "Rust probe build failed")
    require(build["materials_sha256"] == file_hash(materials), "build materials differ")
    for name, digest in build["inputs"].items():
        require(file_hash(REPO_ROOT / name) == digest, f"build input drift: {name}")
    require(file_hash(Path(build["executable"])) == build["executable_sha256"], "executable drift")
    require(file_hash(Path(build["log"])) == build["log_sha256"], "build log drift")
    rows = [json.loads(line) for line in Path(build["log"]).read_text(encoding="utf-8").splitlines() if line.startswith("{")]
    require(any(row.get("reason") == "compiler-artifact" and row.get("executable") == build["cargo_executable"]
                and row["target"]["name"] == RUST_PROBE.stem for row in rows), "missing Cargo executable receipt")
    return build


def run_rust(executable: str) -> dict:
    command = [executable, "--exact", "observe_chat_finish_timing", "--nocapture"]
    result = subprocess.run(command, cwd=REPO_ROOT, env=runtime_environment(), capture_output=True, check=True, timeout=30)
    stdout = result.stdout.decode("utf-8")
    [trace] = [json.loads(line) for line in stdout.splitlines() if line.startswith("[{")]
    return {"command":command,"exit_code":result.returncode,"stdout":stdout,"stderr":result.stderr.decode("utf-8"),"trace":trace}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("materials", "rust-build", "pi-repo", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    freezer = material_tools(REPO_ROOT)
    record = json.loads(args.materials.read_bytes())
    require(not record["blocked_gates"], "materials blocked")
    snapshot = Path(record["snapshot"])
    build = check_build(args.rust_build, args.materials)
    for name in PI_SOURCES:
        pinned = subprocess.check_output(["git","-C",str(args.pi_repo),"show",f"{PI_REVISION}:{name}"])
        require((snapshot / name).read_bytes() == pinned, f"pinned Pi source differs: {name}")
    paths = [REPO_ROOT / p for p in (PI_PROBE, RUST_PROBE, FIXTURE)] + [Path(__file__), args.materials, args.rust_build,
        Path(build["executable"]), Path(build["log"]), *(snapshot / name for name in PI_SOURCES),
        snapshot / "packages/ai/node_modules/openai/core/streaming.mjs"]
    inputs = {str(path.resolve()):file_hash(path) for path in paths}
    def verify() -> None:
        require(all(file_hash(Path(path)) == digest for path,digest in inputs.items()), "replay input drift")
        require(not freezer.verify_evidence(REPO_ROOT, record), "frozen materials drift")
    verify()
    pi_command = ["node",str(REPO_ROOT / PI_PROBE),str(snapshot),str(REPO_ROOT / FIXTURE)]
    pi_runs = [run_json(pi_command, REPO_ROOT) for _ in range(3)]
    opi_runs = [run_rust(build["executable"]) for _ in range(3)]
    for name,runs in (("Pi",pi_runs),("Opi",opi_runs)):
        require(all(row["trace"] == runs[0]["trace"] for row in runs), f"nondeterministic {name} trace")
    comparison = compare(pi_runs[0]["trace"], opi_runs[0]["trace"], json.loads((REPO_ROOT / FIXTURE).read_bytes()))
    verify()
    freezer.write_new_json(args.output, {"schemaVersion":1,"generatedAtUtc":datetime.now(timezone.utc).isoformat(),
        "scope":"Chat terminal repair: finish/usage/sentinel/HTTP EOF; explicit abort block-end difference",
        "upstream_revision":PI_REVISION,"snapshot":str(snapshot),"inputs":inputs,"build":build,
        "comparison":comparison,"pi_runs":pi_runs,"opi_runs":opi_runs})
    print(json.dumps(comparison))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

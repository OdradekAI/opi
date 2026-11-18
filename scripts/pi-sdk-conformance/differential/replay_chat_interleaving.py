"""Compare sealed Chat interleaving builds against one frozen Pi candidate."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory

from common import REPO_ROOT, file_hash, run_json, runtime_environment
from replay_partial_aliases import PI_REVISION
from run_lifecycle_differential import ITERATION, material_tools


FIXTURE = Path("crates/opi-ai/tests/data/openai-chat-interleaved.sse")
RUST_PROBE = Path("crates/opi-ai/tests/openai_chat_interleaved.rs")
MAPPER = Path("crates/opi-ai/src/api/openai.rs")
RUST_TREE = MAPPER.parent.as_posix()
PI_PROBE = ITERATION / "tools/observe_chat_interleaving.mjs"
PI_SOURCES = (
    "packages/ai/src/models.ts",
    "packages/ai/src/api/openai-completions.ts",
    "packages/ai/src/api/anthropic-messages.ts",
    "packages/ai/src/utils/streaming-json.ts",
    "packages/ai/src/utils/event-stream.ts",
    "packages/durable/src/harness/generation.ts",
    "packages/durable/src/harness/partial-changes.ts",
    "packages/durable/src/harness/json.ts",
    "packages/durable/src/timers.ts",
    "packages/durable/test/chat-support.ts",
    "packages/durable/test/harness-support.ts",
    "packages/durable/test/session-support.ts",
)


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise ValueError(detail)


def same_materials(before: dict, after: dict) -> None:
    for key in ("repository", "snapshot", "runtime", "baseline", "inputs", "catalog", "source_comparison", "inspection"):
        require(before[key] == after[key], f"changed material: {key}")
    require(before["trees"].keys() == after["trees"].keys(), "changed tree inventory")
    for key, tree in before["trees"].items():
        if key != RUST_TREE:
            require(tree == after["trees"][key], f"changed tree: {key}")
            continue
        old, new = [{row["path"]: row for row in record["trees"][key]["files"]} for record in (before, after)]
        changed = {name for name in old.keys() | new.keys() if old.get(name) != new.get(name)}
        require(changed == {MAPPER.name}, f"unexpected Rust source drift: {changed}")


def check_build(path: Path, manifest: Path, record: dict) -> dict:
    build = json.loads(path.read_bytes())
    require(build["exit_code"] == 0, "Rust build did not succeed")
    require(build["materials_sha256"] == file_hash(manifest), "Rust build materials mismatch")
    for name in (RUST_PROBE, FIXTURE):
        require(build["inputs"][name.as_posix()] == file_hash(REPO_ROOT / name), f"changed Rust input: {name}")
    [mapper] = [row for row in record["trees"][RUST_TREE]["files"] if row["path"] == MAPPER.name]
    require(build["inputs"][MAPPER.as_posix()] == mapper["sha256"], "Rust build mapper mismatch")
    exe = Path(build["executable"])
    require(file_hash(exe) == build["executable_sha256"], "Rust executable differs from its build receipt")
    artifacts = (json.loads(line) for line in build["stdout"].splitlines() if line.startswith("{"))
    executables = [
        row["executable"] for row in artifacts
        if row.get("reason") == "compiler-artifact" and row["target"]["name"] == "openai_chat_interleaved"
        and row["executable"]
    ]
    require(executables == [build["cargo_executable"]], "Rust test executable missing from Cargo output")
    require(exe.name == Path(executables[0]).name, "Rust test executable name differs")
    return {
        key: build[key] for key in ("command", "exit_code", "executable", "executable_sha256", "cargo_executable", "inputs")
    } | {"receipt": str(path), "receipt_sha256": file_hash(path), "build_stderr": build["stderr"]}


def run_rust(exe: str) -> dict:
    command = [exe, "--exact", "observe_interleaved_chat", "--nocapture"]
    run = subprocess.run(
        command, cwd=REPO_ROOT, env=runtime_environment(), check=True, capture_output=True, timeout=30,
    )
    stdout = run.stdout.decode("utf-8")
    [trace] = [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]
    return {
        "command": command, "exit_code": run.returncode, "stdout": stdout,
        "stdout_sha256": hashlib.sha256(run.stdout).hexdigest(),
        "stderr": run.stderr.decode("utf-8"), "trace": trace,
    }


def compare(pi: dict, before: dict, after: dict) -> dict:
    result = {}
    arguments = {"a": "BCDEF", "n": 2}
    raw_arguments = '{"a":"BCDEF","n":2}'
    for name, suffix in (("resumed_text", "😀EF"), ("resumed_whitespace", " \n")):
        up, old, new = (row[name] for row in (pi["chat"], before, after))
        text = "ABCD" + suffix
        tool = {"type": "toolCall", "id": "call_interleaved", "name": "record", "arguments": arguments}
        blocks = [{"type": "text", "text": text}, tool]
        for trace, indices, ends in (
            (up, [0, 0], [(0, text)]),
            (old, [0, 2], [(0, "ABCD"), (2, suffix)]),
            (new, [0, 0], [(0, text)]),
        ):
            events = trace["events"]
            require([e["index"] for e in events if e["kind"] == "text_delta"] == indices, "text delta indexes")
            require([(e["index"], e["content"]) for e in events if e["kind"] == "text_end"] == ends, "text ends")
            require(
                [(e["kind"], e["delta"]) for e in events if e["delta"] is not None] == [
                    ("text_delta", "ABCD"), ("toolcall_delta", '{"a"'), ("text_delta", suffix),
                    ("toolcall_delta", ':"BCDEF","n"'), ("toolcall_delta", ":2}"),
                ],
                "raw delta order or value changed",
            )
            require(
                trace["result"]["content"][1]["arguments"] == arguments
                and trace["result"]["stop"] == "tool_use"
                and trace["result"]["raw_stop"] == "tool_calls"
                and trace["result"]["response_id"] == "chatcmpl-interleaved"
                and (trace["result"]["input_tokens"], trace["result"]["output_tokens"]) == (7, 3),
                "changed tool or terminal facts",
            )
        old_blocks = [{"type": "text", "text": "ABCD"}, tool, {"type": "text", "text": suffix}]
        for trace, expected in ((up, blocks), (old, old_blocks), (new, blocks)):
            require(
                [{key: value for key, value in block.items() if key != "raw"} for block in trace["result"]["content"]]
                == expected, "terminal content order or value",
            )
        fields = ("kind", "index", "delta", "content")
        require(
            [[event[key] for key in fields] for event in up["events"]]
            == [[event[key] for key in fields] for event in new["events"]],
            "fixed public event fields differ",
        )
        require(up["checkpoints"] == [1, 3, 5, 6, 7, 8, 8, 11], "Pi closes after the provider iterator")
        require(old["checkpoints"] == [1, 3, 6, 8, 9, 10, 12, 13], "old Opi closes at tool start")
        require(new["checkpoints"] == [1, 3, 5, 6, 7, 8, 10, 11], "fixed Opi closes at real finish")
        previews = [
            [e["message"]["content"][1]["arguments"] for e in trace["events"] if e["kind"] == "toolcall_delta"]
            for trace in (up, old, new)
        ]
        require(previews == [
            [{}, {"a": "BCDEF"}, arguments],
            [{"a": None}, {"a": "BCDEF", "n": None}, arguments],
            [{"a": None}, {"a": "BCDEF", "n": None}, arguments],
        ], "incomplete argument previews changed")
        require(
            old["result"]["content"][1]["raw"] == new["result"]["content"][1]["raw"] == raw_arguments,
            "final Opi argument text changed",
        )
        require(up["chat_requests"] == new["chat_requests"], "fixed Chat request bodies differ")
        expected_requests = json.loads(json.dumps(up["chat_requests"]))
        if name == "resumed_whitespace":
            expected_requests[1]["messages"][1]["content"] = "ABCD"
        require(old["chat_requests"] == expected_requests, "old Chat request diff escaped the whitespace suffix")
        wire_tool = {"type": "tool_use", "id": tool["id"], "name": tool["name"], "input": arguments}
        require(
            up["anthropic_replay"] == new["anthropic_replay"] == [blocks[0], wire_tool],
            "fixed Anthropic assistant body differs",
        )
        require(
            old["anthropic_replay"] == [old_blocks[0], wire_tool, old_blocks[2]],
            "old Anthropic block sequence changed",
        )
        result[name] = {
            "after_repair_event_fields_equal": True,
            "after_repair_chat_bodies_equal": True,
            "after_repair_anthropic_assistant_equal": True,
            "before_repair_chat_bodies_equal": old["chat_requests"] == up["chat_requests"],
            "text_delta_indexes": {"pi": [0, 0], "opi_before": [0, 2], "opi_after": [0, 0]},
            "events_at_held_usage": {key: trace["checkpoints"][6] for key, trace in (
                ("pi", up), ("opi_before", old), ("opi_after", new),
            )},
            "argument_previews": dict(zip(("pi", "opi_before", "opi_after"), previews, strict=True)),
        }

    consumers = pi["durable"]
    new_progress = [
        [{"type": "text", "text": "ABCD"}, {"type": "toolCall", "id": "call_interleaved", "name": "record", "arguments": {}}],
        [{"type": "text", "text": "ABCD😀EF"}, {"type": "toolCall", "id": "call_interleaved", "name": "record", "arguments": {}}],
        [{"type": "text", "text": "ABCD😀EF"}, {"type": "toolCall", "id": "call_interleaved", "name": "record", "arguments": {"a": "BCDEF"}}],
    ]
    old_progress = [
        new_progress[0],
        [new_progress[0][0], new_progress[1][1], {"type": "text", "text": "😀EF"}],
        [new_progress[0][0], new_progress[2][1], {"type": "text", "text": "😀EF"}],
    ]
    for route, trace in (("pi", pi["chat"]["resumed_text"]), ("opi_before", before["resumed_text"]), ("opi_after", after["resumed_text"])):
        consumer = consumers[route]
        require(consumer["progress"] == (old_progress if route == "opi_before" else new_progress), f"Durable progress: {route}")
        expected = [{key: value for key, value in b.items() if key != "raw"} for b in trace["result"]["content"]]
        require(consumer["stored"] == consumer["replayed"] == expected, f"Durable stored/replayed blocks: {route}")
        require(
            consumer["settlement"] == "done" and consumer["dispatches"] == 2 and consumer["executed"] == [arguments]
            and consumer["tool_result"] == "ok" and consumer["answer"] == "settled",
            f"Durable tool or settlement: {route}",
        )
    return {
        **result, "durable": {"after_repair_equal": consumers["pi"] == consumers["opi_after"], "tool_and_settlement_equal": True},
        "remaining_differences": ["OpenAI Chat finish_reason versus provider iterator end", "incomplete tool argument preview"],
        "strict_alignment": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("before-materials", "after-materials", "before-build", "after-build", "pi-repo", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    repo = REPO_ROOT
    freezer = material_tools(repo)
    manifests = [args.before_materials.resolve(), args.after_materials.resolve()]
    before, after = [json.loads(path.read_bytes()) for path in manifests]
    snapshot = Path(after["snapshot"])
    same_materials(before, after)
    require(not before["blocked_gates"] and not after["blocked_gates"], "frozen materials blocked")
    builds = [check_build(path.resolve(), manifest, record) for path, manifest, record in zip(
        (args.before_build, args.after_build), manifests, (before, after), strict=True,
    )]

    source_hashes = {}
    for name in PI_SOURCES:
        pinned = subprocess.check_output(["git", "-C", str(args.pi_repo), "show", f"{PI_REVISION}:{name}"])
        require((snapshot / name).read_bytes() == pinned, f"pinned Pi source differs: {name}")
        source_hashes[name] = file_hash(snapshot / name)
    inputs = [
        *manifests, repo / PI_PROBE, repo / RUST_PROBE, repo / FIXTURE, Path(__file__).resolve(),
        *(Path(row[key]) for row in builds for key in ("receipt", "executable")),
        *(snapshot / name for name in PI_SOURCES),
    ]
    hashes = {str(path): file_hash(path) for path in inputs}

    def verify() -> None:
        require(all(file_hash(Path(path)) == digest for path, digest in hashes.items()), "replay input drift")
        errors = freezer.verify_evidence(repo, after)
        require(not errors, f"current materials drift: {errors}")

    verify()
    old_runs, new_runs = [[run_rust(row["executable"]) for _ in range(3)] for row in builds]
    with TemporaryDirectory(prefix="opi-chat-interleaving-") as temp:
        traces = [Path(temp) / f"{name}.json" for name in ("before", "after")]
        for path, runs in zip(traces, (old_runs, new_runs), strict=True):
            path.write_text(json.dumps(runs[0]["trace"]), encoding="utf-8")
        command = ["node", "--conditions=source", str(repo / PI_PROBE), str(snapshot), str(repo / FIXTURE), *map(str, traces)]
        pi_runs = [run_json(command, repo) for _ in range(3)]
    for name, runs in (("Pi", pi_runs), ("Opi before", old_runs), ("Opi after", new_runs)):
        require(all(row["trace"] == runs[0]["trace"] for row in runs), f"nondeterministic {name} observations")
    comparison = compare(pi_runs[0]["trace"], old_runs[0]["trace"], new_runs[0]["trace"])
    verify()
    freezer.write_new_json(args.output, {
        "schemaVersion": 1, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "scope": "OpenAI Chat block interleaving before and after repair, next-turn payloads, and Durable consumer projection",
        "snapshot": str(snapshot), "upstream_revision": PI_REVISION, "pi_sources": source_hashes, "inputs": hashes,
        "before_build": builds[0], "after_build": builds[1], "comparison": comparison,
        "pi_runs": pi_runs, "opi_before_runs": old_runs, "opi_after_runs": new_runs,
    })
    print(f"Chat interleaving traces verified in {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

"""Replay retained partials on the bound Pi build and a sealed Rust test."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from common import REPO_ROOT, file_hash, run_json, runtime_environment
from run_lifecycle_differential import ITERATION, material_tools


PI_REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
PI_DRIVER = ITERATION / "tools/observe_partial_aliases.mjs"
RUST_DRIVER = Path("crates/opi-ai/tests/partial_aliases.rs")
FIXTURE = Path("crates/opi-ai/tests/data/partial-aliases.sse")
PI_SOURCES = (
    "packages/ai/src/types.ts",
    "packages/ai/src/models.ts",
    "packages/ai/src/api/lazy.ts",
    "packages/ai/src/api/openai-completions.ts",
    "packages/ai/src/providers/faux.ts",
    "packages/ai/src/utils/event-stream.ts",
    "packages/ai/src/utils/streaming-json.ts",
    "packages/agent/src/agent-loop.ts",
    "packages/agent/src/agent.ts",
    "packages/coding-agent/src/modes/interactive/interactive-mode.ts",
    "packages/coding-agent/src/modes/interactive/components/assistant-message.ts",
    "packages/coding-agent/src/modes/json-event.ts",
    "packages/durable/src/harness/generation.ts",
    "packages/durable/src/harness/partial-changes.ts",
    "packages/durable/test/chat-support.ts",
    "packages/durable/test/session-support.ts",
)


def source_bindings(snapshot: Path, pi_repo: Path) -> dict:
    rows = {}
    for name in PI_SOURCES:
        git_bytes = subprocess.check_output(["git", "-C", str(pi_repo), "show", f"{PI_REVISION}:{name}"])
        path = snapshot / name
        if path.read_bytes() != git_bytes:
            raise ValueError(f"pinned Pi source differs: {name}")
        rows[name] = file_hash(path)
    return rows


def rust_receipt(executable: Path, repo: Path) -> dict:
    command = [str(executable), "--exact", "partial_aliases", "--nocapture"]
    run = subprocess.run(command, cwd=repo, env=runtime_environment(), check=True, capture_output=True, timeout=30)
    # libtest surrounds the single trace with its own stdout summary.
    stdout = run.stdout.decode("utf-8")
    [line] = [line for line in stdout.splitlines() if line.startswith("{")]
    return {
        "command": command, "exit_code": run.returncode,
        "stdout": stdout, "stderr": run.stderr.decode("utf-8"),
        "stdout_sha256": hashlib.sha256(run.stdout).hexdigest(), "trace": json.loads(line),
    }


def event_fields(row: dict) -> tuple:
    return row["kind"], row["index"], row["delta"], row["content"]


def without_text_end(fields: list) -> list:
    return [entry for entry in fields if entry[0] != "text_end"]


def final_fields(row: dict) -> dict:
    content = [
        {"text": block["text"]} if block["kind"] == "text"
        else {"id": block["id"], "name": block["name"], "arguments": block["args"]["value"]}
        for block in row["content"]
    ]
    return {
        "stop": row["stop"], "response_id": row["response_id"],
        "input_tokens": row["input_tokens"], "content": content,
    }


def compare(pi: dict, rust: dict) -> dict:
    text = "ABCD😀EF"
    arguments = {"a": "BCDEF", "n": 2}
    final_content = [{"text": text}, {"id": "call_alias", "name": "record", "arguments": arguments}]
    previews = [{}, {"a": "BC"}, {"a": "BCDEF"}, {"a": "BCDEF"}, arguments]
    expected_delta = ["ABCD", "😀EF", '{"a"', ':"BC', 'DEF"', ',"n"', ":2}"]
    results = {}
    for route in ("openai", "faux"):
        upstream, opi = pi[route]["baseline"], rust[route]["baseline"]
        pi_mutated, rust_mutated = pi[route]["mutated"], rust[route]["mutated"]
        u_events, r_events = upstream["events"], opi["events"]
        u_delayed, r_delayed = upstream["after_terminal"], opi["after_terminal"]
        u_tool = [e for e in u_events if e["kind"] == "tool_call_delta"]
        r_tool = [e for e in r_events if e["kind"] == "tool_call_delta"]
        u_args = [e["message"]["content"][1]["args"] for e in u_tool]
        r_args = [e["message"]["content"][1]["args"] for e in r_tool]
        u_event_fields = [event_fields(e) for e in u_events]
        r_event_fields = [event_fields(e) for e in r_events]

        if [fields[2] for fields in u_event_fields if fields[2] is not None] != expected_delta:
            raise ValueError(f"changed Pi fixture deltas: {route}")
        # OpenAI closes text at different points. The exact raw event order is
        # preserved below and all other event fields must still agree.
        if without_text_end(u_event_fields) != without_text_end(r_event_fields):
            raise ValueError(f"events differ beyond recorded text_end timing: {route}")
        u_end = [(i, fields) for i, fields in enumerate(u_event_fields) if fields[0] == "text_end"]
        r_end = [(i, fields) for i, fields in enumerate(r_event_fields) if fields[0] == "text_end"]
        if u_end != [(10 if route == "openai" else 4, ("text_end", 0, None, text))]:
            raise ValueError(f"changed Pi text_end: {route}")
        if r_end != [(4, ("text_end", 0, None, text))]:
            raise ValueError(f"changed Opi text_end: {route}")

        final = final_fields(upstream["result"])
        if final["content"] != final_content or final["stop"] != "tool_use" or final != final_fields(opi["result"]):
            raise ValueError(f"baseline terminal differs: {route}")
        if not all(t["requests"] == 1 for t in (upstream, opi, pi_mutated, rust_mutated)):
            raise ValueError(f"extra dispatch: {route}")

        if u_delayed["first_text"]["content"][0]["text"] != text or r_delayed["first_text"]["content"][0]["text"] != "ABCD":
            raise ValueError(f"missing delayed text divergence: {route}")
        if u_delayed["saved_args"] != u_args[1] or r_delayed["saved_args"] != r_args[1]:
            raise ValueError(f"retained arguments changed: {route}")
        if u_args[2]["ref"] != u_args[3]["ref"] or u_args[1]["ref"] == u_args[2]["ref"]:
            raise ValueError(f"changed Pi arguments identity: {route}")
        u_tool_block = u_delayed["tool_delta"]["content"][1]
        if u_tool_block["ref"] != u_tool[1]["message"]["content"][1]["ref"] or u_tool_block["args"]["ref"] == u_args[1]["ref"]:
            raise ValueError(f"retained Pi tool block must point to the replacement arguments: {route}")
        if [a["value"] for a in u_args] != previews:
            raise ValueError(f"changed Pi tool previews: {route}")
        projected = [a["value"] for a in r_args]
        if projected != [{"a": None}, previews[1], previews[2], {"a": "BCDEF", "n": None}, arguments]:
            raise ValueError(f"changed Opi display projection: {route}")
        if [a["raw"] for a in r_args] != ["".join(expected_delta[2:index]) for index in range(3, 8)]:
            raise ValueError(f"changed Opi raw arguments: {route}")

        u_start, u_text, u_latest = (u_delayed[key] for key in ("start", "first_text", "latest"))
        if r_delayed["start"]["content"] or r_delayed["start"]["stop"] != "pending":
            raise ValueError(f"changed Opi start snapshot: {route}")
        if final_fields(rust_mutated["result"]) != final:
            raise ValueError(f"mutation changed Opi final result: {route}")
        pi_write = pi_mutated["write"]
        if pi_write["after"]["first_text"]["content"][0]["text"] != text + "|consumer|":
            raise ValueError(f"missing Pi nested write propagation: {route}")
        if pi_mutated["after_terminal"]["saved_args"]["value"] != {"a": "BC", "consumer": True}:
            raise ValueError(f"missing Pi local arguments write: {route}")
        pi_mutated_final = final_fields(pi_mutated["result"])
        if route == "openai":
            if not u_start["ref"] == u_text["ref"] == u_latest["ref"] == upstream["result"]["ref"]:
                raise ValueError("OpenAI no longer shares the response object")
            if not u_start["content_ref"] == u_text["content_ref"] == u_latest["content_ref"]:
                raise ValueError("OpenAI no longer shares its content array")
            if upstream["after_usage"]["latest"]["input_tokens"] != 7 or opi["after_usage"]["latest"]["input_tokens"] != 0:
                raise ValueError("metadata-only read changed")
            if pi_mutated_final != {
                **final, "response_id": "consumer-id", "content": [{"text": text + "|consumer|"}, final_content[1]],
            }:
                raise ValueError("changed OpenAI mutation propagation")
            if not upstream["request"] == opi["request"] == pi_mutated["request"] == rust_mutated["request"]:
                raise ValueError("OpenAI requests differ")
            if not all(t["raw_frames"] == 11 for t in (upstream, opi, pi_mutated, rust_mutated)):
                raise ValueError("OpenAI raw frame count differs")
        else:
            if len({u_start["ref"], u_text["ref"], u_latest["ref"]}) != 3 or u_start["content"]:
                raise ValueError("changed Faux top-level or start alias")
            if len({u_start["content_ref"], u_text["content_ref"], u_latest["content_ref"]}) != 3:
                raise ValueError("Faux no longer replaces its array on append")
            if u_text["content"][0]["ref"] != u_latest["content"][0]["ref"]:
                raise ValueError("Faux no longer shares a started block")
            if pi_mutated_final != final or pi_mutated["source_after"] != ["changed script", {"a": "changed script", "n": 2}]:
                raise ValueError("changed Faux script isolation")
        results[route] = {
            "status": "difference", "baseline_final": final,
            "pi_text_end_offset": u_end[0][0], "opi_text_end_offset": r_end[0][0],
            "pi_preview": previews, "opi_display_projection": projected,
            "pi_mutation_reaches_terminal": pi_mutated_final != final,
        }

    durable = pi["durable"]
    if durable["shared"]["progress"] != {"text": "ABCD", "input_tokens": 7} or durable["owned"]["progress"] != {
        "text": "ABCD", "input_tokens": 0,
    }:
        raise ValueError("Durable preview diverges from the held metadata control")
    if durable["shared"]["final"] != durable["owned"]["final"] or any(
        row["settlement"] != "done" or row["calls"] != 1 for row in durable.values()
    ):
        raise ValueError("Durable final or settlement changed")
    return {**results, "durable": durable, "strict_alignment": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--materials", type=Path, required=True)
    parser.add_argument("--pi-repo", type=Path, required=True)
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument("--rust-exe-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")

    repo = REPO_ROOT
    freezer = material_tools(repo)
    materials = args.materials.resolve()
    material_digest = file_hash(materials)
    record = json.loads(materials.read_bytes())
    snapshot = Path(record["snapshot"])
    executable = args.rust_exe.resolve()
    inputs = [repo / PI_DRIVER, repo / RUST_DRIVER, repo / FIXTURE, Path(__file__).resolve(), executable]
    bindings = {str(path): file_hash(path) for path in inputs}
    if bindings[str(executable)] != args.rust_exe_sha256:
        parser.error("Rust executable differs from the build receipt")
    pi_sources = source_bindings(snapshot, args.pi_repo.resolve())

    def check_materials() -> None:
        problems = record["blocked_gates"] + freezer.verify_evidence(repo, record)
        if problems:
            raise ValueError(f"frozen materials: {problems}")
        if file_hash(materials) != material_digest:
            raise ValueError("material manifest changed")
        if any(file_hash(Path(path)) != digest for path, digest in bindings.items()):
            raise ValueError("probe inputs changed")
        if any(file_hash(snapshot / name) != digest for name, digest in pi_sources.items()):
            raise ValueError("bound Pi source changed")

    check_materials()
    pi_command = ["node", "--conditions=source", str(repo / PI_DRIVER), "--snapshot", str(snapshot), str(repo / FIXTURE)]
    pi_runs = [run_json(pi_command, repo) for _ in range(3)]
    rust_runs = [rust_receipt(executable, repo) for _ in range(3)]
    for side, runs in (("Pi", pi_runs), ("Opi", rust_runs)):
        if any(run["trace"] != runs[0]["trace"] for run in runs[1:]):
            raise ValueError(f"nondeterministic {side} observations")
    comparison = compare(pi_runs[0]["trace"], rust_runs[0]["trace"])
    check_materials()
    freezer.write_new_json(args.output, {
        "schemaVersion": 1, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "scope": "OpenAI Chat and Faux partial retention, and a controlled Durable ownership projection",
        "materials": {"path": str(materials), "sha256": material_digest, "snapshot": str(snapshot)},
        "upstream_revision": PI_REVISION,
        "pi_sources": pi_sources, "inputs": bindings,
        "comparison": comparison, "pi_runs": pi_runs, "opi_runs": rust_runs,
    })
    print(f"partial traces verified, retained differences recorded in {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

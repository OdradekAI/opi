"""Validate real Agent and projected frozen Durable consumers of Chat timing."""
from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from common import DIFFERENTIAL_PATH, REPO_ROOT, file_hash, run_json, runtime_environment
from replay_chat_finish_timing import require
from run_lifecycle_differential import material_tools

PROBE = DIFFERENTIAL_PATH / "tools/observe_chat_finish_consumers.mjs"
RUST_PROBE = Path("crates/opi-agent/tests/agent_chat_finish_timing.rs")
PI_SOURCES = ("packages/durable/src/harness/generation.ts", "packages/durable/src/harness/partial-changes.ts",
              "packages/durable/src/harness/json.ts", "packages/durable/src/timers.ts",
              "packages/durable/test/chat-support.ts", "packages/durable/test/harness-support.ts",
              "packages/durable/test/session-support.ts")


def compare(durable: dict, agent: list) -> dict:
    rows = durable["observations"]
    require(len(rows) == 6 and len(agent) == 3, "consumer case count")
    for case in ("trailer", "error_after_usage", "cancel_after_usage"):
        for side in ("pi", "opi"):
            [row] = [r for r in rows if r["side"] == side and r["case"] == case]
            before = row["after_usage"]
            require(before["completion"] is None and before["executed"] == [], "early Durable tool or settlement")
            require(before["stored_assistants"] == [], "early assistant persistence")
            success = case == "trailer"
            require(row["final"]["outcome"]["status"] == ("done" if success else "unanswered"), "Durable failure classification")
            require(row["final"]["dispatches"] == (2 if success else 1), "Durable dispatch count")
            require(row["final"]["executed"] == ([{"a":"ok","n":2}] if success else []), "Durable tool execution")
            stored = row["final"]["stored_assistants"]
            require(len(stored) == (2 if success else 1), "Durable stored assistants")
            require(stored[0]["stop"] == ("toolUse" if success else "error" if case.startswith("error") else "aborted"), "Durable stored terminal")
        pair = [r["final"] for r in rows if r["case"] == case]
        require(pair[0] == pair[1], "Durable paired final state differs")
    require([r["case"] for r in agent] == ["done","error","cancel"], "Agent case identities")
    for row in agent:
        require(not row["before_tail"]["agent_finished"] and row["before_tail"]["tool_calls"] == 0, "Agent finished before tail")
        ends = [e for e in row["before_tail"]["events"] if e["data"]["event"] == "message_end"]
        require(not ends, "Agent MessageEnd before tail")
        success = row["case"] == "done"
        require(row["final"]["ok"] == success and row["final"]["tool_calls"] == (1 if success else 0), "Agent terminal tool execution")
        if not success:
            require(row["final"]["error"]["kind"] == ("provider" if row["case"] == "error" else "cancelled"), "Agent failure classification")
    return {"durable_projection_pairs":3,"real_agent_cases":3,"decision":"terminal-repair-verified",
            "early_tools_and_persistence":False,"terminal_consumer_alignment":True}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("paired", "materials", "agent-build", "pi-repo", "output"):
        parser.add_argument("--"+name, type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    pair = json.loads(args.paired.read_bytes())
    snapshot = Path(pair["snapshot"])
    build = json.loads(args.agent_build.read_bytes())
    require(build["exit_code"] == 0, "Agent build failed")
    for name, digest in build["inputs"].items():
        require(file_hash(REPO_ROOT / name) == digest, f"Agent build input drift: {name}")
    require(file_hash(args.materials) == pair["build"]["materials_sha256"], "paired materials differ")
    materials = json.loads(args.materials.read_bytes())
    freezer = material_tools(REPO_ROOT)
    require(file_hash(Path(build["log"])) == build["log_sha256"], "Agent log drift")
    rows = [json.loads(line) for line in Path(build["log"]).read_text(encoding="utf-8").splitlines() if line.startswith("{")]
    require(any(row.get("reason") == "compiler-artifact" and row.get("executable") == build["cargo_executable"]
                and row["target"]["name"] == RUST_PROBE.stem for row in rows), "Agent executable receipt")
    for source in PI_SOURCES:
        pinned = subprocess.check_output(["git","-C",str(args.pi_repo),"show",f"{pair['upstream_revision']}:{source}"])
        require((snapshot / source).read_bytes() == pinned, f"Pi source drift: {source}")
    inputs = {**pair["inputs"], **{str(REPO_ROOT / name):digest for name,digest in build["inputs"].items()},
              str(Path(build["executable"])):build["executable_sha256"]}
    paths = [args.paired,args.materials,args.agent_build,Path(__file__),REPO_ROOT / PROBE,REPO_ROOT / RUST_PROBE,
             *(snapshot / name for name in PI_SOURCES)]
    inputs.update({str(path.resolve()):file_hash(path) for path in paths})
    def verify() -> None:
        require(all(file_hash(Path(path)) == digest for path,digest in inputs.items()), "consumer input drift")
        require(not freezer.verify_evidence(REPO_ROOT, materials), "consumer materials drift")
    verify()
    durable_command = ["node","--conditions=source",str(REPO_ROOT / PROBE),str(snapshot),str(args.paired.resolve())]
    durable_runs = [run_json(durable_command,REPO_ROOT) for _ in range(3)]
    agent_runs = []
    for _ in range(3):
        command = [build["executable"],"--exact","observe_agent_chat_finish_timing","--nocapture"]
        result = subprocess.run(command,cwd=REPO_ROOT,env=runtime_environment(),check=True,capture_output=True,timeout=30)
        stdout = result.stdout.decode("utf-8")
        trace = [json.loads(line.removeprefix("agent_chat_finish_timing ")) for line in stdout.splitlines() if line.startswith("agent_chat_finish_timing ")]
        agent_runs.append({"command":command,"exit_code":result.returncode,"stdout":stdout,"stderr":result.stderr.decode("utf-8"),"trace":trace})
    for runs in (durable_runs,agent_runs):
        # Late TCP write acceptance is OS-dependent and is retained separately.
        def semantic(run: dict) -> object:
            trace = run["trace"]
            return [{key:value for key,value in row.items() if key != "peer"} for row in trace] if isinstance(trace,list) else trace
        require(all(semantic(run) == semantic(runs[0]) for run in runs), "nondeterministic consumer semantics")
    comparison = compare(durable_runs[0]["trace"],agent_runs[0]["trace"])
    verify()
    material_tools(REPO_ROOT).write_new_json(args.output,{"schemaVersion":1,"generatedAtUtc":datetime.now(timezone.utc).isoformat(),
        "upstream_revision":pair["upstream_revision"],"inputs":inputs,"agent_build":build,
        "comparison":comparison,"durable_runs":durable_runs,"agent_runs":agent_runs})
    print(json.dumps(comparison))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

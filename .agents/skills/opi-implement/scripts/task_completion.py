#!/usr/bin/env python3
"""Read-only task source, review-input and completion identity checks.

Capture source before verification, seal raw proof afterward, and recheck at
worktree/index/commit/checkpoint boundaries. Receipts bind bytes, not truth of
assertions, actual execution provenance, or permission to commit.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys

sys.dont_write_bytecode = True
SHA = re.compile(r"[0-9a-f]{64}\Z")
COMMIT = re.compile(r"[0-9a-f]{40}\Z")
LEDGER = ".opi-impl-state.json"


class CompletionError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise CompletionError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def git(repo, *args, data=None):
    result = subprocess.run(["git", "-C", str(repo), *args], input=data,
                            capture_output=True, env=dict(os.environ, GIT_OPTIONAL_LOCKS="0"))
    require(result.returncode == 0, "Git inspection failed: " + result.stderr.decode("utf-8", errors="replace").strip())
    return result.stdout


def path_name(value):
    require(isinstance(value, str) and bool(value), "missing repository path")
    path = PurePosixPath(value)
    require(not path.is_absolute() and not re.match(r"^[A-Za-z]:", value)
            and ".." not in path.parts and "\\" not in value and path.as_posix() == value
            and path.parts[0].casefold() != ".git", "invalid repository path: " + value)
    return value


def names(raw):
    return {path_name(item.decode("utf-8")) for item in raw.split(b"\0") if item}


def entries(repo, revision=None):
    output = git(repo, "ls-tree", "-r", "-z", "--full-tree", revision) if revision else git(repo, "ls-files", "--stage", "-z")
    values = {}
    for item in output.split(b"\0"):
        if not item:
            continue
        header, raw_name = item.split(b"\t", 1)
        first, second, third = header.decode("ascii").split()
        mode, blob = (first, third) if revision else (first, second)
        require((second == "blob" if revision else third == "0") and mode in {"100644", "100755", "120000"},
                "unmerged index or unsupported Git mode")
        name = path_name(raw_name.decode("utf-8"))
        if name != LEDGER:
            values[name] = {"mode": mode, "blob": blob}
    return values


def state(repo, name, index):
    path = repo / path_name(name)
    for parent in path.parents:
        if parent == repo:
            break
        require(not parent.is_symlink(), "symlink parent is outside source proof")
    require(path.parent.resolve().is_relative_to(repo), "source path escapes repository")
    try:
        info = path.lstat()
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode):
        content = os.fsencode(os.readlink(path))
        mode = "120000"
    else:
        require(stat.S_ISREG(info.st_mode), "source path is not a regular file")
        content = path.read_bytes()
        mode = index.get(name, {}).get("mode", "100644")
        if os.name != "nt":
            mode = "100755" if info.st_mode & stat.S_IXUSR else "100644"
    return {"sha256": digest(content), "mode": mode}


def snapshot(repo):
    head = git(repo, "rev-parse", "HEAD").decode().strip()
    tree, index = entries(repo, head), entries(repo)
    untracked = names(git(repo, "ls-files", "--others", "--exclude-standard", "-z")) - {LEDGER}
    working = {}
    for name in sorted(set(tree) | set(index) | untracked):
        value = state(repo, name, index)
        if value is not None:
            working[name] = value
    dirty = names(git(repo, "diff", "--name-only", "--no-renames", "-z")) | names(git(repo, "diff", "--cached", "--name-only", "--no-renames", "-z")) | untracked
    return {"head": head, "tree": tree, "index": index, "working": working,
            "dirty_paths": sorted(dirty - {LEDGER})}


def stable_snapshot(repo):
    result = snapshot(repo)
    require(result == snapshot(repo), "source changed during capture")
    return result


def changed(before, after):
    return {name for name in set(before) | set(after) if before.get(name) != after.get(name)}


def owned(task, name):
    return any(fnmatch.fnmatchcase(name, pattern) for pattern in task["task_owned_paths"])


def read_json(path):
    return json.loads(Path(path).read_bytes())


def read_receipt(path, expected, kind):
    require(isinstance(expected, str) and SHA.fullmatch(expected), "invalid receipt SHA-256")
    raw = Path(path).read_bytes()
    require(digest(raw) == expected, "receipt digest mismatch")
    value = json.loads(raw)
    require(value.get("schema") == "opi-task-" + kind + "/1", "wrong receipt schema")
    return value


def write_receipt(repo, path, value):
    path = Path(path).absolute()
    require(not path.resolve().is_relative_to(repo), "receipt must be outside repository")
    raw = encoded(value)
    with path.open("xb") as stream:
        stream.write(raw)
    return digest(raw)


def validate_task(task):
    require(isinstance(task, dict) and isinstance(task.get("id"), str) and bool(task["id"]), "invalid task id")
    require(isinstance(task.get("definition_of_done"), str) and bool(task["definition_of_done"]), "missing task DoD")
    require(isinstance(task.get("start_commit"), str) and COMMIT.fullmatch(task["start_commit"]), "start_commit must be a full commit SHA")
    require(isinstance(task.get("evaluator_required"), bool), "evaluator_required must be explicit")
    require(isinstance(task.get("task_owned_paths"), list) and bool(task["task_owned_paths"]), "missing task-owned paths")
    for pattern in task["task_owned_paths"]:
        path_name(pattern)
    require(not owned(task, LEDGER), "task cannot own the canonical ledger")


def source_inputs(baseline, current):
    task = baseline["task"]
    sources = {name: current["working"].get(name) for name in baseline["sources"]}
    for name, identity in baseline["sources"].items():
        if sources[name] == identity:
            continue
        # Admission owns the meaning of this frozen declaration. A C.1a path
        # append cannot turn normative input into an authorized source edit.
        required = {"docs/opi-spec.md", "docs/opi-spec.zh.md"} if name in {
            "docs/opi-spec.md", "docs/opi-spec.zh.md"} else {name}
        require(task.get("tier") == "documentation"
                and all(path in task["task_owned_paths"] and path in task["definition_of_done"] for path in required),
                "registered source change is outside admitted documentation scope: " + name)
        require(sources[name] is not None and sources[name]["mode"] in {"100644", "100755"},
                "registered source must remain a regular file: " + name)
    return sources


def verify_sources(repo, sources):
    for name, identity in sources.items():
        require(state(repo, name, entries(repo)) == identity, "registered source changed: " + name)


def baseline(args):
    repo = Path(git(Path.cwd(), "rev-parse", "--show-toplevel").decode().strip()).resolve()
    task = read_json(args.task)
    validate_task(task)
    current = stable_snapshot(repo)
    if args.mode == "worktree":
        require(current["head"] == task["start_commit"], "baseline HEAD differs from start_commit")
    else:
        parent_check(repo, current["head"], task["start_commit"])
    require(not any(owned(task, name) for name in current["dirty_paths"]), "task overlaps carried-in work")
    source_paths = {path_name(args.source_design_path), *(path_name(p) for p in args.source)}
    sources = {name: current["working"].get(name) for name in source_paths}
    require(all(sources.values()), "registered source is absent")
    value = {"schema": "opi-task-baseline/1", "repository": str(repo), "task": task, "mode": args.mode,
             "source_design_path": args.source_design_path, "sources": sources, "snapshot": current}
    if args.mode == "commit":
        origin = entries(repo, task["start_commit"])
        delta = changed(origin, current["tree"])
        require(all(owned(task, name) for name in delta), "manual commit includes unowned paths")
        require(git(repo, "diff-tree", "--no-commit-id", "--name-only", "-r", current["head"], "--", LEDGER).strip() == b"", "manual task commit contains ledger")
        original_working = dict(current["working"])
        for name in delta:
            if name in origin:
                require(origin[name]["mode"] != "120000", "changed symlink is outside completion proof")
                original_working[name] = {"mode": origin[name]["mode"], "sha256": digest(git(repo, "cat-file", "blob", origin[name]["blob"]))}
            else:
                original_working.pop(name, None)
        value["origin_working"] = original_working
        # A recovery receipt must not silently promote the manual commit's
        # edited requirements to its original admission authority.
        value["sources"] = {name: original_working.get(name) for name in source_paths}
        require(all(value["sources"].values()), "registered source is absent from start_commit")
    return write_receipt(repo, args.output, value)


def check_carried(baseline, current):
    old = baseline["snapshot"]
    for name in old["dirty_paths"]:
        require(old["working"].get(name) == current["working"].get(name)
                and old["index"].get(name) == current["index"].get(name), "carried-in work changed: " + name)


def task_delta(baseline, current):
    delta = changed(baseline.get("origin_working", baseline["snapshot"]["working"]), current["working"])
    require(all(owned(baseline["task"], name) for name in delta), "unowned source change")
    for name in delta:
        require(all(s["working"].get(name, {}).get("mode") != "120000" for s in (baseline["snapshot"], current)),
                "changed symlink is outside task source proof")
    return sorted(delta)


def parent_check(repo, head, start):
    require(git(repo, "rev-list", "--parents", "-n", "1", head).decode().split()[1:] == [start],
            "task commit must have start_commit as its sole parent")


def current_task(original, task):
    validate_task(task)
    runtime = {"status", "iteration_count", "max_iterations", "verified_at_commit", "evidence", "blocker", "session_notes"}
    append_fields = {"task_owned_paths", "production_call_sites", "inference_notes"}
    for key in (set(original) | set(task)) - runtime - append_fields - {"acceptance_scenarios"}:
        require(original.get(key) == task.get(key), "task contract changed: " + key)
    for key in append_fields:
        before, after = original.get(key, []), task.get(key, [])
        require(isinstance(after, list) and after[:len(before)] == before, "task field is not append-only: " + key)
    old_notes, new_notes = original.get("session_notes", []), task.get("session_notes", [])
    require(isinstance(new_notes, list) and new_notes[:len(old_notes)] == old_notes, "session notes are not append-only")
    added = task["task_owned_paths"][len(original["task_owned_paths"]):]
    notes = task.get("inference_notes", [])[len(original.get("inference_notes", [])):]
    require(len([n for n in notes if isinstance(n, dict) and n.get("field") == "task_owned_paths" and n.get("reason")]) >= len(added),
            "ownership append lacks inference note")
    for note in notes:
        require(isinstance(note, dict) and note.get("field") == "task_owned_paths" and isinstance(note.get("reason"), str) and bool(note["reason"]),
                "unexpected task inference change")
    strip_status = lambda items: [{k: v for k, v in item.items() if k != "status"} for item in items]
    require(strip_status(original.get("acceptance_scenarios", [])) == strip_status(task.get("acceptance_scenarios", [])), "acceptance scenario contract changed")
    return task


def capture(args):
    base = read_receipt(args.path, args.sha256, "baseline")
    repo = Path(base["repository"])
    task = current_task(base["task"], read_json(args.task)) if args.task else base["task"]
    require(not any(owned(task, name) for name in base["snapshot"]["dirty_paths"]), "expanded task overlaps carried-in work")
    current = stable_snapshot(repo)
    sources = source_inputs(base, current)
    base = dict(base, task=task)
    check_carried(base, current)
    if args.mode == "worktree":
        require(current["head"] == base["task"]["start_commit"], "worktree HEAD differs from start_commit")
    else:
        parent_check(repo, current["head"], base["task"]["start_commit"])
        require(not any(owned(base["task"], name) for name in current["dirty_paths"]), "manual commit has dirty task paths")
    delta = task_delta(base, current)
    require(not changed(base["snapshot"]["index"], current["index"]) - set(delta),
            "index change is absent from the working candidate; reconcile staged and working content before capture")
    require(bool(delta), "candidate has no task source changes")
    return write_receipt(repo, args.output, {
        "schema": "opi-task-candidate/1", "repository": str(repo), "mode": args.mode,
        "baseline_path": str(args.path.resolve()), "baseline_sha256": args.sha256, "task": task,
        "files": delta, "snapshot": current, "sources": sources})


def load_candidate(path, sha):
    candidate = read_receipt(path, sha, "candidate")
    base = read_receipt(candidate["baseline_path"], candidate["baseline_sha256"], "baseline")
    require(candidate["repository"] == base["repository"], "candidate repository mismatch")
    require(candidate["sources"] == source_inputs(base, candidate["snapshot"]), "candidate registered source identity mismatch")
    base = dict(base, task=current_task(base["task"], candidate["task"]))
    return candidate, base, Path(base["repository"])


def footers_check(task, footers):
    require(isinstance(footers, dict) and all(isinstance(k, str) and isinstance(v, str) and v
            and "\n" not in v and "\r" not in v for k, v in footers.items()), "invalid evidence footers")
    required = {"Opi-Task", "Opi-DoD-SHA256", "Opi-Verification", "Opi-Evaluator"}
    if task.get("acceptance_scenarios"):
        required.add("Opi-Acceptance")
    require(set(footers) == required, "missing or unexpected Opi footer")
    require(footers["Opi-Task"] == task["id"], "Opi-Task mismatch")
    require(footers["Opi-DoD-SHA256"] == digest(task["definition_of_done"].encode("utf-8")), "DoD digest mismatch")
    require(footers["Opi-Evaluator"] == ("passed" if task["evaluator_required"] else "not-required"), "evaluator footer mismatch")


def raw_proofs(verification):
    checks = verification.get("checks")
    require(isinstance(checks, list) and bool(checks), "verification requires raw checks")
    identities = {}
    for check in checks:
        require(isinstance(check, dict) and isinstance(check.get("command"), str) and bool(check["command"]), "check command is missing")
        require(type(check.get("exit_code")) is int and check["exit_code"] == 0, "check did not exit successfully")
        require(check.get("kind") in {"gate", "behavioral"}, "check kind must be gate or behavioral")
        paths = [check.get("stdout"), check.get("stderr")]
        rust = check.get("rust")
        cargo_command = re.search(r"(?:^|[\\/\s\"'])cargo(?:\.exe)?(?=[\s\"'])", check["command"], re.IGNORECASE)
        observed_cargo = re.search(r"(?:^|[\\/\s\"'])opi-cargo-cache\.py[\"']?\s+[\"']?run(?=[\s\"'])", check["command"], re.IGNORECASE)
        if check["kind"] == "behavioral" and (cargo_command or observed_cargo) and re.search(r"\btest\b", check["command"]):
            require(isinstance(rust, dict), "behavioral cargo test requires Rust execution proof")
        if rust is not None:
            require(isinstance(rust, dict), "invalid Rust execution proof")
            require(all(type(rust.get(key)) is int and rust[key] == 0 for key in ("list_exit_code", "ignored_list_exit_code")),
                    "Rust inventory processes did not exit successfully")
            require(rust.get("run_output") == check["stdout"], "Rust execution log must be the captured command stdout")
            module_path = Path(__file__).parents[2] / "opi-audit" / "scripts" / "check_test_execution.py"
            spec = importlib.util.spec_from_file_location("opi_test_execution", module_path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            logs = [Path(rust[key]) for key in ("list_output", "ignored_list_output", "run_output")]
            try:
                result = module.check_execution(*(module.read_log(p)[0] for p in logs),
                    test_filter=rust.get("filter"), exact=rust.get("exact", False),
                    selection=rust.get("selection", "normal"), expected_count=rust.get("expected_count"))
            except module.LogError as error:
                raise CompletionError(str(error)) from error
            require(result["status"] == "PASS", "Rust behavioral execution proof failed")
            paths.extend(str(p) for p in logs)
        for value in paths:
            require(isinstance(value, str) and Path(value).is_absolute(), "raw log path must be absolute")
            path = Path(value)
            require(path.is_file() and not path.is_symlink(), "raw proof is not a regular file")
            identities[str(path.resolve())] = digest(path.read_bytes())
    return identities


def review(args):
    candidate, base, repo = load_candidate(args.path, args.sha256)
    verify_sources(repo, candidate["sources"])
    require(stable_snapshot(repo) == candidate["snapshot"], "source changed after verification input capture")
    verification, footers = read_json(args.evidence), read_json(args.footers)
    footers_check(base["task"], footers)
    proof = raw_proofs(verification)
    current = candidate["snapshot"]
    files = [{"path": name, "before": base.get("origin_working", base["snapshot"]["working"]).get(name),
              "after": current["working"].get(name), "index": current["index"].get(name)} for name in candidate["files"]]
    value = {"schema": "opi-task-review/1", "repository": str(repo), "task": base["task"],
             "task_id": base["task"]["id"], "task_sha256": digest(encoded(base["task"])),
             "start_commit": base["task"]["start_commit"], "review_head": current["head"],
             "mode": candidate["mode"], "candidate_path": str(args.path.resolve()), "candidate_sha256": args.sha256,
             "baseline_path": candidate["baseline_path"], "baseline_sha256": candidate["baseline_sha256"],
             "source_design_path": base["source_design_path"], "source_sha256": candidate["sources"][base["source_design_path"]]["sha256"],
             "baseline_sources": base["sources"], "sources": candidate["sources"],
             "dod_sha256": footers["Opi-DoD-SHA256"], "files": files,
             "verification": verification, "raw_proofs": proof, "proposed_footers": footers}
    require(stable_snapshot(repo) == current and raw_proofs(verification) == proof, "source or proof changed while sealing review")
    return write_receipt(repo, args.output, value)


def expected_blob(repo, name, value):
    if value is None:
        return None
    attrs = git(repo, "check-attr", "-z", "filter", "--", name).split(b"\0")
    require(len(attrs) >= 3 and attrs[2] in {b"unspecified", b"unset"}, "external Git filter is outside completion proof")
    content = (repo / name).read_bytes()
    require(digest(content) == value["sha256"], "source changed before Git comparison")
    return {"mode": value["mode"], "blob": git(repo, "hash-object", "--path", name, "--stdin", data=content).decode().strip()}


def actual_footers(repo, commit):
    message = git(repo, "show", "-s", "--format=%B", commit)
    trailers = git(repo, "interpret-trailers", "--parse", data=message).decode("utf-8")
    values = {}
    for line in trailers.splitlines():
        key, separator, value = line.partition(":")
        if separator and key.startswith("Opi-"):
            require(key not in values, "duplicate Opi footer")
            values[key] = value.strip()
    return values


def mirror(evidence):
    aliases = {"task": "Opi-Task", "task_id": "Opi-Task", "dod_sha256": "Opi-DoD-SHA256",
               "verification": "Opi-Verification", "evaluator": "Opi-Evaluator", "acceptance": "Opi-Acceptance"}
    require(isinstance(evidence, dict), "missing ledger evidence mirror")
    result = {}
    for key, value in evidence.items():
        canonical = aliases.get(key, key)
        require(canonical not in result, "duplicate ledger evidence alias")
        result[canonical] = value
    return result


def exec_finding(finding, task_id):
    keys = {"lens", "task_id", "criterion_id", "decision_id", "field", "problem", "severity", "suggested_fix", "source_citation", "confidence"}
    require(isinstance(finding, dict) and set(finding) == keys, "malformed Workflow finding")
    require(finding["task_id"] == task_id and finding["criterion_id"] is None,
            "Workflow finding belongs to another scope")
    require(finding["decision_id"] is None or isinstance(finding["decision_id"], str),
            "invalid Workflow decision identity")
    require(all(isinstance(finding[key], str) for key in
                ("lens", "field", "problem", "suggested_fix", "source_citation")), "invalid Workflow finding field")
    require(finding["severity"] in {"high", "medium", "low"}
            and finding["confidence"] in {"high", "medium", "low"}
            and re.search(r"[§#]", finding["source_citation"]), "invalid Workflow finding classification")


def verify(args):
    reviewed = read_receipt(args.path, args.sha256, "review")
    candidate, base, repo = load_candidate(reviewed["candidate_path"], reviewed["candidate_sha256"])
    require(reviewed["task"] == base["task"] and reviewed["baseline_sha256"] == candidate["baseline_sha256"], "review task identity mismatch")
    require(reviewed["baseline_sources"] == base["sources"] and reviewed["sources"] == candidate["sources"]
            and reviewed["source_design_path"] == base["source_design_path"]
            and reviewed["source_sha256"] == candidate["sources"][base["source_design_path"]]["sha256"],
            "review registered source identity mismatch")
    verify_sources(repo, candidate["sources"])
    require(raw_proofs(reviewed["verification"]) == reviewed["raw_proofs"], "raw proof changed after review")
    task = base["task"]
    footers_check(task, reviewed["proposed_footers"])
    if task["evaluator_required"]:
        require(args.workflow_result is not None, "required Workflow result is missing")
        result = read_json(args.workflow_result)
        require(set(result) == {"wf_ref", "review_sha256", "candidate_sha256", "reviewed_input", "must_fix", "flagged_for_human", "rejected", "report"},
                "Workflow result is not the complete exec result")
        require(isinstance(result.get("wf_ref"), str) and bool(result["wf_ref"].strip()), "required Workflow run identity is missing")
        require(encoded(result["reviewed_input"]) == encoded(reviewed), "Workflow inspected different candidate content")
        require(result.get("review_sha256") == args.sha256 and result.get("candidate_sha256") == reviewed["candidate_sha256"], "Workflow result has wrong review identity")
        require(result.get("must_fix") == [] and isinstance(result.get("rejected"), list)
                and isinstance(result.get("flagged_for_human"), list), "Workflow result is incomplete or has must-fix findings")
        require(not any(item.get("reason") == "verify-agent-error" for item in result["rejected"]), "Workflow verifier did not complete")
        for finding in result["flagged_for_human"]:
            exec_finding(finding, task["id"])
        for rejection in result["rejected"]:
            require(isinstance(rejection, dict) and set(rejection) == {"finding", "reason"}
                    and isinstance(rejection["reason"], str) and bool(rejection["reason"].strip()),
                    "malformed Workflow rejection")
            exec_finding(rejection["finding"], task["id"])
        report = result["report"]
        require(isinstance(report, dict) and set(report) == {"summary", "must_fix", "flagged_for_human", "rejected"}
                and isinstance(report["summary"], str)
                and all(report[key] == result[key] for key in ("must_fix", "flagged_for_human", "rejected")),
                "Workflow report differs from its completed result")
    current = stable_snapshot(repo)
    check_carried(base, current)
    require(current["working"] == candidate["snapshot"]["working"], "source differs from reviewed candidate")
    require(task_delta(base, current) == candidate["files"], "candidate file coverage differs")
    if args.stage == "worktree":
        require(current == candidate["snapshot"], "worktree/index differs from reviewed candidate")
    elif args.stage == "index":
        require(candidate["mode"] == "worktree" and current["head"] == task["start_commit"], "index proof requires precommit candidate")
        target = current["index"]
        require(changed(entries(repo, task["start_commit"]), target) == set(candidate["files"]), "index contains unrelated paths or omits candidate files")
        for name in candidate["files"]:
            require(target.get(name) == expected_blob(repo, name, current["working"].get(name)), "index differs from candidate content")
        require(git(repo, "diff", "--cached", "--name-only", "--", LEDGER).strip() == b"", "canonical ledger staged with implementation")
    else:
        parent_check(repo, current["head"], task["start_commit"])
        if candidate["mode"] == "commit":
            require(current["head"] == candidate["snapshot"]["head"], "manual candidate commit changed")
        target = current["tree"]
        require(changed(entries(repo, task["start_commit"]), target) == set(candidate["files"]), "commit file coverage differs from candidate")
        require(not any(owned(task, name) for name in current["dirty_paths"]), "task paths remain dirty after commit")
        require(not changed(target, current["index"]) - set(base["snapshot"]["dirty_paths"]), "unrelated index changes after task commit")
        require(git(repo, "diff-tree", "--no-commit-id", "--name-only", "-r", current["head"], "--", LEDGER).strip() == b"", "implementation commit contains ledger")
        for name in candidate["files"]:
            require(target.get(name) == expected_blob(repo, name, current["working"].get(name)), "commit differs from reviewed candidate")
        require(actual_footers(repo, current["head"]) == reviewed["proposed_footers"], "actual Git footers differ from reviewed evidence")
        if args.stage == "checkpoint":
            require(args.ledger is not None, "checkpoint requires proposed ledger")
            ledger = read_json(args.ledger)
            require(ledger.get("schema_version") == 2, "checkpoint requires ledger schema version 2")
            records = [t for t in ledger.get("tasks", []) if t.get("id") == task["id"]]
            require(len(records) == 1, "checkpoint task identity missing or duplicated")
            record = records[0]
            current_task(task, record)
            for key in ("task_owned_paths", "production_call_sites", "inference_notes"):
                require(record.get(key, []) == task.get(key, []), "checkpoint cannot extend task contract: " + key)
            require(record.get("status") == "passing" and record.get("verified_at_commit") == current["head"], "checkpoint completion identity mismatch")
            require(mirror(record.get("evidence")) == reviewed["proposed_footers"], "checkpoint evidence mirror differs from actual Git footers")
    require(stable_snapshot(repo) == current, "source changed during completion verification")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="command", required=True)
    first = subs.add_parser("baseline")
    first.add_argument("--task", type=Path, required=True)
    first.add_argument("--mode", choices=("worktree", "commit"), default="worktree")
    first.add_argument("--source-design-path", required=True)
    first.add_argument("--source", action="append", default=[])
    first.add_argument("--output", type=Path, required=True)
    for name in ("capture", "review", "verify"):
        command = subs.add_parser(name)
        command.add_argument("path", type=Path)
        command.add_argument("--sha256", required=True)
        if name != "verify":
            command.add_argument("--output", type=Path, required=True)
        if name == "capture":
            command.add_argument("--mode", choices=("worktree", "commit"), required=True)
            command.add_argument("--task", type=Path)
        elif name == "review":
            command.add_argument("--evidence", type=Path, required=True)
            command.add_argument("--footers", type=Path, required=True)
        else:
            command.add_argument("--stage", choices=("worktree", "index", "commit", "checkpoint"), required=True)
            command.add_argument("--workflow-result", type=Path)
            command.add_argument("--ledger", type=Path)
    args = parser.parse_args()
    try:
        sha = globals()[args.command](args)
        print(args.command + ": PASS" + (" sha256=" + sha if sha else ""))
        return 0
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print("FAIL: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

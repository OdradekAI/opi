#!/usr/bin/env python3
"""Bind remediation source inputs to an actual diff, index, or local commit.

Receipts attest file identity only. Capture ``tested`` before verification and
verify afterward; commands, assertions, ignored/external inputs, and authorization
remain the calling workflow's responsibility. Git is read-only in this helper.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any

sys.dont_write_bytecode = True

import assurance_set
import validate_assurance_artifact as validator


SCHEMA = "opi-remediation-source-proof/1"


class ProofError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ProofError(message)


def git(repository: Path, *args: str, data: bytes | None = None) -> bytes:
    environment = dict(os.environ, GIT_OPTIONAL_LOCKS="0", GIT_GRAFT_FILE=os.devnull)
    result = subprocess.run(
        ["git", "--no-replace-objects", "-C", str(repository), *args], input=data,
        capture_output=True, check=False, env=environment,
    )
    require(result.returncode == 0, "Git inspection failed: " + result.stderr.decode("utf-8", errors="replace").strip())
    return result.stdout


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def repo_path(value: str) -> str:
    require(isinstance(value, str) and bool(value), "invalid repository path")
    path = PurePosixPath(value)
    require(bool(path.parts) and not path.is_absolute() and re.match(r"^[A-Za-z]:", value) is None and ".." not in path.parts and "\\" not in value and path.as_posix() == value, "invalid repository path: " + value)
    require(path.parts[0].casefold() != ".git", "Git metadata is not a source path")
    return value


def paths(data: bytes) -> set[str]:
    return {repo_path(item.decode("utf-8")) for item in data.split(b"\0") if item}


def index_entries(repository: Path) -> dict[str, dict[str, str]]:
    entries = {}
    for item in git(repository, "ls-files", "--stage", "-z").split(b"\0"):
        if not item:
            continue
        header, name = item.split(b"\t", 1)
        mode, blob, stage = header.decode("ascii").split()
        require(stage == "0", "unmerged index cannot be captured")
        require(mode in {"100644", "100755", "120000"}, "unsupported index mode: " + mode)
        entries[repo_path(name.decode("utf-8"))] = {"mode": mode, "blob": blob}
    return entries


def tree_entries(repository: Path, revision: str) -> dict[str, dict[str, str]]:
    entries = {}
    for item in git(repository, "ls-tree", "-r", "-z", "--full-tree", revision).split(b"\0"):
        if not item:
            continue
        header, name = item.split(b"\t", 1)
        mode, kind, blob = header.decode("ascii").split()
        require(kind == "blob" and mode in {"100644", "100755", "120000"}, "unsupported committed file mode: " + mode)
        entries[repo_path(name.decode("utf-8"))] = {"mode": mode, "blob": blob}
    return entries


def file_state(repository: Path, name: str, index: dict[str, dict[str, str]], filemode: bool) -> dict[str, str] | None:
    path = repository / repo_path(name)
    # Inspect link bytes, never a linked target or a file below a linked parent.
    for parent in path.parents:
        if parent == repository:
            break
        require(not parent.is_symlink(), "source path has a symlink parent: " + name)
    require(path.parent.resolve().is_relative_to(repository), "source path escapes repository: " + name)
    try:
        info = path.lstat()
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode):
        content = os.fsencode(os.readlink(path))
        mode = "120000"
    else:
        require(stat.S_ISREG(info.st_mode), "source is not a regular file: " + name)
        content = path.read_bytes()
        original_mode = index.get(name, {}).get("mode", "100644")
        mode = original_mode if not filemode or original_mode == "120000" else "100755" if info.st_mode & stat.S_IXUSR else "100644"
    return {"sha256": digest(content), "mode": mode}


def snapshot(repository: Path, exempt: set[str]) -> dict[str, Any]:
    head = git(repository, "rev-parse", "HEAD").decode().strip()
    index = index_entries(repository)
    tree = tree_entries(repository, head)
    untracked = paths(git(repository, "ls-files", "--others", "--exclude-standard", "-z"))
    filemode = git(repository, "config", "--type=bool", "--get", "core.filemode").strip() == b"true"
    names = (set(index) | set(tree) | untracked) - exempt
    working = {}
    for name in sorted(names):
        value = file_state(repository, name, index, filemode)
        if value is not None:
            working[name] = value
    dirty = (
        paths(git(repository, "diff", "--name-only", "--no-renames", "-z"))
        | paths(git(repository, "diff", "--cached", "--name-only", "--no-renames", "-z"))
        | untracked
    ) - exempt
    return {
        "head": head,
        "working": working,
        "index": {name: value for name, value in index.items() if name not in exempt},
        "tree": {name: value for name, value in tree.items() if name not in exempt},
        "dirty_paths": sorted(dirty),
    }


def stable_snapshot(repository: Path, exempt: set[str]) -> dict[str, Any]:
    first = snapshot(repository, exempt)
    require(first == snapshot(repository, exempt), "source changed while capturing receipt")
    return first


def check_capture_identity(repository: Path, exempt: set[str], state: dict[str, Any]) -> None:
    require(git(repository, "rev-parse", "HEAD").decode().strip() == state["head"], "HEAD changed during capture")
    current_index = {name: value for name, value in index_entries(repository).items() if name not in exempt}
    require(current_index == state["index"], "index changed during capture")


def read_receipt(path: Path, expected: str, kind: str) -> dict[str, Any]:
    require(validator.SHA256_RE.fullmatch(expected) is not None, "invalid receipt digest")
    data = path.read_bytes()
    require(digest(data) == expected, "receipt digest mismatch")
    value = json.loads(data)
    require(isinstance(value, dict) and value.get("schema") == SCHEMA and value.get("kind") == kind, "wrong receipt schema or kind")
    return value


def write_receipt(repository: Path, path: Path, value: dict[str, Any]) -> str:
    require(not path.absolute().is_relative_to(repository) and not path.resolve().is_relative_to(repository), "receipt output must be outside the repository")
    data = (json.dumps(value, sort_keys=True, indent=2) + "\n").encode("utf-8")
    with path.open("xb") as handle:
        handle.write(data)
    return digest(data)


def fixed_paths(repository: Path, plan: Path) -> set[str]:
    return {(plan.parent / name).relative_to(repository).as_posix() for name in validator.REMEDIATION_FILES}


def frozen_paths(repository: Path, plan: Path) -> set[str]:
    index = json.loads(plan.with_name("audit.index.json").read_bytes())
    names = {"audit.index.json"}
    for member in index["members"]:
        names.update(member["artifact_stem"] + suffix for suffix in (".meta.json", ".requirements.jsonl", ".findings.jsonl", ".md"))
    return {(plan.parent / name).relative_to(repository).as_posix() for name in names}


def artifact_states(repository: Path, names: set[str]) -> dict[str, Any]:
    index = index_entries(repository)
    filemode = git(repository, "config", "--type=bool", "--get", "core.filemode").strip() == b"true"
    result = {name: file_state(repository, name, index, filemode) for name in names}
    require(all(value is not None for value in result.values()), "required assurance artifact is missing")
    return result


def check_errors(errors: list[str]) -> None:
    require(not errors, "; ".join(errors))


def validate_binding(baseline: dict[str, Any], *, admission: bool) -> tuple[Path, Path, set[str]]:
    repository = Path(baseline["repository"]).resolve()
    plan = repository / repo_path(baseline["plan_path"])
    require(assurance_set.repository_root(plan.parent) == repository, "receipt repository does not match plan")
    require(validator.plan_digest(plan, plan.with_name("remediation.plan.dispositions.jsonl")) == baseline["plan_sha256"], "plan bytes changed since baseline")
    require(digest(plan.with_name("audit.index.json").read_bytes()) == baseline["audit_index_sha256"], "audit index changed since baseline")
    check_errors(validator.validate_apply(plan, baseline["plan_sha256"]) if admission else validator.validate_plan(plan))
    frozen = frozen_paths(repository, plan)
    require(artifact_states(repository, frozen) == baseline["frozen_artifacts"], "frozen indexed assurance artifacts changed since baseline")
    return repository, plan, fixed_paths(repository, plan) | frozen


def check_carried(baseline: dict[str, Any], current: dict[str, Any]) -> None:
    before = baseline["snapshot"]
    for name in before["dirty_paths"]:
        require(before["working"].get(name) == current["working"].get(name) and before["index"].get(name) == current["index"].get(name), "carried-in work changed: " + name)


def changed_paths(before: dict[str, Any], after: dict[str, Any]) -> set[str]:
    return {name for name in set(before) | set(after) if before.get(name) != after.get(name)}


def check_index_ownership(baseline: dict[str, Any], current: dict[str, Any], owned: set[str]) -> None:
    changed = changed_paths(baseline["snapshot"]["index"], current["index"])
    unowned = changed - owned - set(baseline["snapshot"]["dirty_paths"])
    require(not unowned, "unowned index changes: " + ", ".join(sorted(unowned)))


def attributed_paths(dispositions: Path) -> set[str]:
    return {
        repo_path(name)
        for line in dispositions.read_text(encoding="utf-8").splitlines() if line
        for name in json.loads(line)["changed_paths"]
    }


def receipt_link(repository: Path, path: Path, sha256: str) -> dict[str, str]:
    resolved = path.resolve()
    require(not resolved.is_relative_to(repository), "linked receipt must be outside the repository")
    return {"path": str(resolved), "sha256": sha256}


def linked_receipt(repository: Path, link: dict[str, str], kind: str) -> dict[str, Any]:
    require(isinstance(link, dict) and set(link) == {"path", "sha256"}, "invalid receipt link")
    path = Path(link["path"])
    require(path.is_absolute(), "linked receipt must have an absolute path")
    receipt_link(repository, path, link["sha256"])
    return read_receipt(path, link["sha256"], kind)


def read_tested(path: Path, sha256: str, baseline_sha256: str) -> dict[str, Any]:
    tested = read_receipt(path, sha256, "tested")
    require(tested.get("baseline_sha256") == baseline_sha256, "tested receipt belongs to another baseline")
    return tested


def commit_parent(repository: Path, commit: str, expected: str) -> None:
    require(validator.FULL_SHA_RE.fullmatch(commit) is not None, "invalid commit SHA")
    header = git(repository, "cat-file", "commit", commit).split(b"\n\n", 1)[0]
    parents = [line[7:] for line in header.splitlines() if line.startswith(b"parent ")]
    require(parents == [expected.encode("ascii")], "commit parent does not match expected source proof HEAD")


def checkpoint_delta(parent: dict[str, Any], target: dict[str, Any], planned: set[str]) -> None:
    changed = changed_paths(parent, target)
    unexpected = changed - planned
    require(not unexpected, "unrelated staged or committed checkpoint paths: " + ", ".join(sorted(unexpected)))
    for name in changed & planned:
        require(all(tree.get(name, {}).get("mode") != "120000" for tree in (parent, target)), "changed symlink is outside source proof: " + name)


def checkpoint_head(
    repository: Path, plan: Path, exempt: set[str], baseline: dict[str, Any],
    baseline_sha256: str, link: dict[str, str] | None,
) -> str:
    """Verify the full linear chain back to the original execution HEAD."""
    original = baseline["snapshot"]["head"]
    if link is None:
        return original
    planned = attributed_paths(plan.with_name("remediation.plan.dispositions.jsonl"))
    require(not planned.intersection(exempt), "fixed or frozen artifacts cannot be planned source paths")
    tip = None
    expected_commit = None
    seen: set[str] = set()
    while link is not None:
        receipt = linked_receipt(repository, link, "checkpoint-commit")
        require(receipt.get("baseline_sha256") == baseline_sha256, "checkpoint receipt belongs to another baseline")
        commit = receipt["commit"]
        require(commit not in seen, "checkpoint chain repeats a commit")
        require(expected_commit is None or commit == expected_commit, "checkpoint predecessor does not match commit parent")
        seen.add(commit)
        if tip is None:
            tip = commit
        candidate = linked_receipt(repository, receipt["candidate"], "checkpoint-index")
        require(candidate.get("baseline_sha256") == baseline_sha256, "checkpoint candidate belongs to another baseline")
        tested = linked_receipt(repository, candidate["tested"], "tested")
        require(tested.get("baseline_sha256") == baseline_sha256, "checkpoint tested receipt belongs to another baseline")
        parent = tested["snapshot"]["head"]
        commit_parent(repository, commit, parent)
        target = tree_entries(repository, commit)
        require(target == candidate["index"], "checkpoint commit tree differs from its verified index")
        checkpoint_delta(tree_entries(repository, parent), target, planned)
        expected_commit = parent
        link = tested.get("checkpoint")
    require(expected_commit == original, "checkpoint chain does not reach baseline execution HEAD")
    assert tip is not None
    return tip


def capture_baseline(args: argparse.Namespace) -> str:
    plan = args.path.resolve()
    require(validator.SHA256_RE.fullmatch(args.plan_sha256) is not None, "invalid plan_sha256")
    check_errors(validator.validate_apply(plan, args.plan_sha256))
    repository = assurance_set.repository_root(plan.parent)
    index_digest = digest(plan.with_name("audit.index.json").read_bytes())
    frozen = artifact_states(repository, frozen_paths(repository, plan))
    exempt = fixed_paths(repository, plan) | set(frozen)
    state = stable_snapshot(repository, exempt)
    owned = attributed_paths(plan.with_name("remediation.plan.dispositions.jsonl"))
    require(not owned.intersection(state["dirty_paths"]), "planned paths overlap carried-in work")
    value = {
        "schema": SCHEMA, "kind": "baseline", "repository": str(repository),
        "plan_path": plan.relative_to(repository).as_posix(),
        "plan_sha256": args.plan_sha256,
        "audit_index_sha256": index_digest,
        "frozen_artifacts": frozen,
        "remediation_head": validator.unquote_code(validator.header_value(plan.read_text(encoding="utf-8"), "Remediation head")),
        "snapshot": state,
    }
    validate_binding(value, admission=True)
    check_capture_identity(repository, exempt, state)
    return write_receipt(repository, args.output, value)


def capture_tested(args: argparse.Namespace) -> str:
    baseline = read_receipt(args.path, args.baseline_sha256, "baseline")
    repository, plan, exempt = validate_binding(baseline, admission=args.checkpoint is None)
    link = receipt_link(repository, args.checkpoint, args.checkpoint_sha256) if args.checkpoint else None
    expected_head = checkpoint_head(repository, plan, exempt, baseline, args.baseline_sha256, link)
    current = stable_snapshot(repository, exempt)
    require(current["head"] == expected_head, "HEAD changed since baseline or last verified checkpoint")
    check_carried(baseline, current)
    planned = attributed_paths(plan.with_name("remediation.plan.dispositions.jsonl"))
    actual = changed_paths(baseline["snapshot"]["working"], current["working"])
    check_index_ownership(baseline, current, planned | actual)
    value = {"schema": SCHEMA, "kind": "tested", "baseline_sha256": args.baseline_sha256, "snapshot": current}
    if link is not None:
        value["checkpoint"] = link
    validate_binding(baseline, admission=link is None)
    check_capture_identity(repository, exempt, current)
    return write_receipt(repository, args.output, value)


def expected_entry(repository: Path, name: str, state: dict[str, str] | None) -> dict[str, str] | None:
    if state is None:
        require(not (repository / name).exists() and not (repository / name).is_symlink(), "path appeared after tested input: " + name)
        return None
    path = repository / name
    current_index = index_entries(repository)
    filemode = git(repository, "config", "--type=bool", "--get", "core.filemode").strip() == b"true"
    require(file_state(repository, name, current_index, filemode) == state, "file differs from tested or frozen input: " + name)
    if state["mode"] == "120000":
        content = os.fsencode(os.readlink(path)) if path.is_symlink() else path.read_bytes()
        require(digest(content) == state["sha256"], "file changed after tested input check: " + name)
        blob = git(repository, "hash-object", "--no-filters", "--stdin", data=content).decode().strip()
    else:
        # External clean filters may execute arbitrary tools; fail closed rather
        # than running them merely to inspect a candidate index.
        attributes = git(repository, "check-attr", "-z", "filter", "--", name).split(b"\0")
        require(len(attributes) >= 3 and attributes[2] in {b"unspecified", b"unset"}, "external Git filter is outside source proof: " + name)
        content = path.read_bytes()
        require(digest(content) == state["sha256"], "file changed after tested input check: " + name)
        blob = git(repository, "hash-object", "--path", name, "--stdin", data=content).decode().strip()
    return {"mode": state["mode"], "blob": blob}


def verify_checkpoint(args: argparse.Namespace) -> str:
    baseline = read_receipt(args.path, args.baseline_sha256, "baseline")
    tested = read_tested(args.tested, args.tested_sha256, args.baseline_sha256)
    predecessor = tested.get("checkpoint")
    initial_admission = args.stage == "index" and predecessor is None
    repository, plan, exempt = validate_binding(baseline, admission=initial_admission)
    parent = checkpoint_head(repository, plan, exempt, baseline, args.baseline_sha256, predecessor)
    require(tested["snapshot"]["head"] == parent, "tested receipt does not match checkpoint parent")
    current = stable_snapshot(repository, exempt)
    check_carried(baseline, current)
    require(current["working"] == tested["snapshot"]["working"], "working tree differs from tested input")
    planned = attributed_paths(plan.with_name("remediation.plan.dispositions.jsonl"))
    require(not planned.intersection(exempt), "fixed or frozen artifacts cannot be planned source paths")
    actual = changed_paths(baseline["snapshot"]["working"], current["working"])
    require(not actual - planned, "unplanned checkpoint source changes: " + ", ".join(sorted(actual - planned)))
    target = index_entries(repository)
    tested_link = receipt_link(repository, args.tested, args.tested_sha256)
    if args.stage == "index":
        require(current["head"] == parent, "HEAD changed since baseline or last verified checkpoint")
        value = {
            "schema": SCHEMA, "kind": "checkpoint-index", "baseline_sha256": args.baseline_sha256,
            "tested": tested_link, "index": target,
        }
    else:
        candidate_link = receipt_link(repository, args.candidate, args.candidate_sha256)
        candidate = linked_receipt(repository, candidate_link, "checkpoint-index")
        require(candidate.get("baseline_sha256") == args.baseline_sha256, "checkpoint candidate belongs to another baseline")
        require(candidate.get("tested") == tested_link, "checkpoint candidate belongs to another tested receipt")
        require(args.commit == current["head"], "checkpoint commit proof requires the current HEAD")
        commit_parent(repository, args.commit, parent)
        require(tree_entries(repository, args.commit) == candidate["index"] == target, "checkpoint commit tree or current index differs from the verified index")
        value = {
            "schema": SCHEMA, "kind": "checkpoint-commit", "baseline_sha256": args.baseline_sha256,
            "commit": args.commit, "candidate": candidate_link,
        }
    checkpoint_delta(tree_entries(repository, parent), target, planned)
    for name in sorted(planned):
        state = tested["snapshot"]["working"].get(name)
        require(target.get(name) == expected_entry(repository, name, state), "checkpoint index differs from tested source: " + name)
    validate_binding(baseline, admission=initial_admission)
    require(stable_snapshot(repository, exempt) == current and index_entries(repository) == target, "source or index changed during checkpoint verification")
    return write_receipt(repository, args.output, value)


def verify(args: argparse.Namespace) -> None:
    baseline = read_receipt(args.baseline, args.baseline_sha256, "baseline")
    tested = read_tested(args.tested, args.tested_sha256, args.baseline_sha256)
    predecessor = tested.get("checkpoint")
    initial_admission = args.stage != "commit" and predecessor is None
    repository, plan, exempt = validate_binding(baseline, admission=initial_admission)
    parent = checkpoint_head(repository, plan, exempt, baseline, args.baseline_sha256, predecessor)
    require(tested["snapshot"]["head"] == parent, "tested receipt does not match final verification parent")
    result = args.path.resolve()
    require(result == plan.with_name("remediation.result.md"), "result is not the fixed sibling of the bound plan")
    artifacts = artifact_states(repository, exempt)
    check_errors(validator.validate_result(result))
    require(validator.header_value(result.read_text(encoding="utf-8"), "Outcome") == "APPLIED", "source materialization proof requires APPLIED result")
    owned = attributed_paths(result.with_name("remediation.result.dispositions.jsonl"))
    require(not owned.intersection(exempt), "fixed remediation outputs cannot be attributed as source repairs")
    require(not owned.intersection(baseline["snapshot"]["dirty_paths"]), "result paths overlap carried-in work")
    actual = changed_paths(baseline["snapshot"]["working"], tested["snapshot"]["working"])
    require(not actual - owned, "unattributed actual source changes: " + ", ".join(sorted(actual - owned)))
    require(not owned - actual, "recorded source paths not changed: " + ", ".join(sorted(owned - actual)))
    for name in actual:
        require(all(state["working"].get(name, {}).get("mode") != "120000" for state in (baseline["snapshot"], tested["snapshot"])), "changed symlink is outside source proof: " + name)
    current = stable_snapshot(repository, exempt)
    check_carried(baseline, current)
    check_index_ownership(baseline, current, owned)
    require(current["working"] == tested["snapshot"]["working"], "working tree differs from tested input")
    if args.stage != "commit":
        require(current["head"] == parent, "HEAD changed since baseline or last verified checkpoint")
    target = None
    if args.stage == "index":
        target = index_entries(repository)
        changed = changed_paths(tree_entries(repository, current["head"]), target)
        require(not changed - owned - exempt, "unrelated staged paths: " + ", ".join(sorted(changed - owned - exempt)))
    elif args.stage == "commit":
        require(args.commit == current["head"], "commit proof requires the current HEAD")
        commit_parent(repository, args.commit, parent)
        target = tree_entries(repository, args.commit)
        changed = changed_paths(tree_entries(repository, parent), target)
        require(not changed - owned - exempt, "commit includes unrelated paths: " + ", ".join(sorted(changed - owned - exempt)))
        for name in set(target) | set(current["index"]):
            if name not in exempt and name not in baseline["snapshot"]["dirty_paths"]:
                require(target.get(name) == current["index"].get(name), "index changed after commit: " + name)
    if target is not None:
        cumulative = changed_paths(baseline["snapshot"]["tree"], target)
        require(not owned - cumulative, args.stage + " omits repaired paths")
        require(not cumulative - owned - exempt, args.stage + " includes unattributed checkpoint paths")
        for name in sorted(owned | exempt):
            state = artifacts[name] if name in exempt else tested["snapshot"]["working"].get(name)
            require(target.get(name) == expected_entry(repository, name, state), args.stage + " differs from tested source or fixed artifacts: " + name)
    # The phase lock coordinates assurance tools, not editors. Detect drift over
    # this observation window without claiming to lock the user's filesystem.
    validate_binding(baseline, admission=initial_admission)
    require(artifact_states(repository, exempt) == artifacts, "fixed or frozen assurance artifacts changed during verification")
    final = stable_snapshot(repository, exempt)
    require(final == current, "source or index changed during tested input verification")
    if args.stage == "index":
        require(index_entries(repository) == target, "index changed during verification")
    elif args.stage == "commit":
        require(git(repository, "rev-parse", "HEAD").decode().strip() == args.commit and tree_entries(repository, args.commit) == target, "commit changed during verification")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    baseline_parser = subparsers.add_parser("baseline")
    baseline_parser.add_argument("path", type=Path)
    baseline_parser.add_argument("--plan-sha256", required=True)
    baseline_parser.add_argument("--output", type=Path, required=True)
    tested_parser = subparsers.add_parser("tested")
    tested_parser.add_argument("path", type=Path)
    tested_parser.add_argument("--baseline-sha256", required=True)
    tested_parser.add_argument("--output", type=Path, required=True)
    tested_parser.add_argument("--checkpoint", type=Path)
    tested_parser.add_argument("--checkpoint-sha256")
    checkpoint_parser = subparsers.add_parser("checkpoint", help="prove a CI candidate index or its committed tree")
    checkpoint_parser.add_argument("path", type=Path, help="original baseline receipt")
    checkpoint_parser.add_argument("--baseline-sha256", required=True)
    checkpoint_parser.add_argument("--tested", type=Path, required=True)
    checkpoint_parser.add_argument("--tested-sha256", required=True)
    checkpoint_parser.add_argument("--stage", choices=("index", "commit"), required=True)
    checkpoint_parser.add_argument("--candidate", type=Path)
    checkpoint_parser.add_argument("--candidate-sha256")
    checkpoint_parser.add_argument("--commit")
    checkpoint_parser.add_argument("--output", type=Path, required=True)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("path", type=Path)
    verify_parser.add_argument("--baseline", type=Path, required=True)
    verify_parser.add_argument("--baseline-sha256", required=True)
    verify_parser.add_argument("--tested", type=Path, required=True)
    verify_parser.add_argument("--tested-sha256", required=True)
    verify_parser.add_argument("--stage", choices=("worktree", "index", "commit"), default="worktree")
    verify_parser.add_argument("--commit")
    args = parser.parse_args()
    if args.command == "tested" and bool(args.checkpoint) != bool(args.checkpoint_sha256):
        parser.error("--checkpoint and --checkpoint-sha256 are required together")
    if args.command in {"verify", "checkpoint"}:
        if (args.stage == "commit") != bool(args.commit):
            parser.error("--commit is required only for --stage commit")
        if args.commit and validator.FULL_SHA_RE.fullmatch(args.commit) is None:
            parser.error("--commit must be a full lowercase commit SHA")
    if args.command == "checkpoint":
        if args.stage == "commit" and not (args.candidate and args.candidate_sha256):
            parser.error("--stage commit requires --candidate and --candidate-sha256")
        if args.stage == "index" and (args.candidate or args.candidate_sha256):
            parser.error("--candidate and --candidate-sha256 are only for --stage commit")
    try:
        if args.command == "baseline":
            plan = args.path.resolve()
        else:
            receipt = read_receipt(args.baseline if args.command == "verify" else args.path, args.baseline_sha256, "baseline")
            plan = Path(receipt["repository"]) / repo_path(receipt["plan_path"])
        repository = assurance_set.repository_root(plan.parent)
        phase_directory = plan.parent.parent
        phase = assurance_set.parse_phase(phase_directory)
        with assurance_set.AssuranceLock(repository, phase):
            pending = assurance_set.transaction_base(repository)
            require(not any(pending.glob(f"phase{phase}-*")), "pending assurance transaction; recover through the owning validator first")
            if args.command == "baseline":
                proof_digest = capture_baseline(args)
            elif args.command == "tested":
                proof_digest = capture_tested(args)
            elif args.command == "checkpoint":
                proof_digest = verify_checkpoint(args)
            else:
                verify(args)
                proof_digest = None
        suffix = " sha256=" + proof_digest if proof_digest else ""
        print(args.command + ": PASS" + suffix)
        return 0
    except (ProofError, assurance_set.AssuranceSetError, OSError, ValueError, KeyError, TypeError) as exc:
        print("FAIL: " + str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

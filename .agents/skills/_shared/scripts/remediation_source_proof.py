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
    environment = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    result = subprocess.run(
        ["git", "-C", str(repository), *args], input=data,
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


def capture_baseline(args: argparse.Namespace) -> str:
    plan = args.path.resolve()
    require(validator.SHA256_RE.fullmatch(args.plan_sha256) is not None, "invalid plan_sha256")
    check_errors(validator.validate_apply(plan, args.plan_sha256))
    repository = assurance_set.repository_root(plan.parent)
    index_digest = digest(plan.with_name("audit.index.json").read_bytes())
    frozen = artifact_states(repository, frozen_paths(repository, plan))
    exempt = fixed_paths(repository, plan) | set(frozen)
    state = stable_snapshot(repository, exempt)
    records = [json.loads(line) for line in plan.with_name("remediation.plan.dispositions.jsonl").read_text(encoding="utf-8").splitlines() if line]
    owned = {repo_path(name) for record in records for name in record["changed_paths"]}
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
    repository, plan, exempt = validate_binding(baseline, admission=True)
    current = stable_snapshot(repository, exempt)
    require(current["head"] == baseline["snapshot"]["head"], "HEAD changed since baseline capture")
    check_carried(baseline, current)
    planned = {
        repo_path(name)
        for line in plan.with_name("remediation.plan.dispositions.jsonl").read_text(encoding="utf-8").splitlines() if line
        for name in json.loads(line)["changed_paths"]
    }
    actual = changed_paths(baseline["snapshot"]["working"], current["working"])
    check_index_ownership(baseline, current, planned | actual)
    value = {"schema": SCHEMA, "kind": "tested", "baseline_sha256": args.baseline_sha256, "snapshot": current}
    validate_binding(baseline, admission=True)
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


def verify(args: argparse.Namespace) -> None:
    baseline = read_receipt(args.baseline, args.baseline_sha256, "baseline")
    tested = read_receipt(args.tested, args.tested_sha256, "tested")
    require(tested.get("baseline_sha256") == args.baseline_sha256, "tested receipt belongs to another baseline")
    repository, plan, exempt = validate_binding(baseline, admission=args.stage != "commit")
    result = args.path.resolve()
    require(result == plan.with_name("remediation.result.md"), "result is not the fixed sibling of the bound plan")
    artifacts = artifact_states(repository, exempt)
    check_errors(validator.validate_result(result))
    require(validator.header_value(result.read_text(encoding="utf-8"), "Outcome") == "APPLIED", "source materialization proof requires APPLIED result")
    records = [json.loads(line) for line in result.with_name("remediation.result.dispositions.jsonl").read_text(encoding="utf-8").splitlines() if line]
    owned = {repo_path(name) for record in records for name in record["changed_paths"]}
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
        require(current["head"] == baseline["snapshot"]["head"], "HEAD changed since baseline capture")
    target = None
    if args.stage == "index":
        target = index_entries(repository)
        changed = changed_paths(tree_entries(repository, current["head"]), target)
        require(not changed - owned - exempt, "unrelated staged paths: " + ", ".join(sorted(changed - owned - exempt)))
    elif args.stage == "commit":
        require(args.commit == current["head"], "commit proof requires the current HEAD")
        parents = git(repository, "rev-list", "--parents", "-n", "1", args.commit).decode().split()[1:]
        require(parents == [baseline["snapshot"]["head"]], "commit parent does not match baseline execution HEAD")
        target = tree_entries(repository, args.commit)
        changed = changed_paths(tree_entries(repository, parents[0]), target)
        require(not changed - owned - exempt, "commit includes unrelated paths: " + ", ".join(sorted(changed - owned - exempt)))
        for name in set(target) | set(current["index"]):
            if name not in exempt and name not in baseline["snapshot"]["dirty_paths"]:
                require(target.get(name) == current["index"].get(name), "index changed after commit: " + name)
    if target is not None:
        require(not owned - changed, args.stage + " omits repaired paths")
        for name in sorted(owned | exempt):
            state = artifacts[name] if name in exempt else tested["snapshot"]["working"].get(name)
            require(target.get(name) == expected_entry(repository, name, state), args.stage + " differs from tested source or fixed artifacts: " + name)
    # The phase lock coordinates assurance tools, not editors. Detect drift over
    # this observation window without claiming to lock the user's filesystem.
    validate_binding(baseline, admission=args.stage != "commit")
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
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("path", type=Path)
    verify_parser.add_argument("--baseline", type=Path, required=True)
    verify_parser.add_argument("--baseline-sha256", required=True)
    verify_parser.add_argument("--tested", type=Path, required=True)
    verify_parser.add_argument("--tested-sha256", required=True)
    verify_parser.add_argument("--stage", choices=("worktree", "index", "commit"), default="worktree")
    verify_parser.add_argument("--commit")
    args = parser.parse_args()
    if args.command == "verify":
        if (args.stage == "commit") != bool(args.commit):
            parser.error("--commit is required only for --stage commit")
        if args.commit and validator.FULL_SHA_RE.fullmatch(args.commit) is None:
            parser.error("--commit must be a full lowercase commit SHA")
    try:
        if args.command == "baseline":
            plan = args.path.resolve()
        else:
            receipt = read_receipt(args.path if args.command == "tested" else args.baseline, args.baseline_sha256, "baseline")
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

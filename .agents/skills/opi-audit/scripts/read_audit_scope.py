"""Project committed audit scope without exposing ledger execution conclusions."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import PurePosixPath


ROOT_STATE = ".opi-impl-state.json"
SPEC = "docs/opi-spec.md"
SHA256 = re.compile(r"[0-9a-f]{64}")
TASK_STRINGS = (
    "id", "title", "crate", "parent_spec_row", "definition_of_done",
    "definition_source", "tier",
)
TASK_LISTS = ("depends_on", "production_call_sites", "task_owned_paths")
NOTE_FIELDS = {
    "definition_of_done", "forbidden_scope", "deferred", "split", "residual",
    "reuse_search", "placement", "surface_necessity", "simplification_ceiling",
    "shared_decision", "source_outcome", "test_seam", "task_owned_paths",
    "verification", "verification_platform", "artifact_audit_scope",
}


class ScopeError(Exception):
    """A diagnostic containing only fixed labels, never ledger values."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ScopeError(message)


def git(*args: str) -> bytes:
    result = subprocess.run(["git", *args], capture_output=True, check=False)
    require(result.returncode == 0, "cannot resolve committed audit input")
    return result.stdout


def object_value(value: object) -> dict:
    require(isinstance(value, dict), "expected a ledger object")
    return value


def list_value(value: object) -> list:
    require(isinstance(value, list), "expected a ledger list")
    return value


def fields(value: object, strings: tuple, lists: tuple = ()) -> dict:
    """Select typed leaves; a nested object cannot bypass the allowlist."""
    value = object_value(value)
    result = {}
    for key in strings:
        if key in value:
            require(value[key] is None or isinstance(value[key], str), "invalid scope text field")
            result[key] = value[key]
    for key in lists:
        if key in value:
            items = list_value(value[key])
            require(all(isinstance(item, str) for item in items), "invalid scope string list")
            result[key] = items
    return result


def source_path(value: object) -> str:
    require(isinstance(value, str) and bool(value), "invalid registered source path")
    path = PurePosixPath(value)
    require(
        not path.is_absolute() and path.as_posix() == value
        and not any(part in {"..", "."} for part in path.parts)
        and not any(char in value for char in "\\:\r\n\t")
        and value.startswith("docs/") and value.endswith(".md")
        and "snapshots" not in path.parts,
        "invalid registered specification path",
    )
    return value


def registration(state: dict) -> tuple[list, dict]:
    version = state.get("schema_version")
    require(type(version) is int and version in {1, 2}, "unsupported ledger schema")
    if version == 1:
        paths = [source_path(state.get("spec_path"))]
        hashes = {paths[0]: state.get("spec_sha256")}
    else:
        paths = [source_path(path) for path in list_value(state.get("spec_files"))]
        hashes = object_value(state.get("spec_files_sha256"))
    require(SPEC in paths and len(paths) == len(set(paths)), "invalid registered specification set")
    selected = {}
    for path in paths:
        digest = hashes.get(path)
        require(isinstance(digest, str) and SHA256.fullmatch(digest) is not None, "invalid stored source hash")
        selected[path] = digest
    return paths, selected


def project_task(task: dict, phase: int) -> dict:
    result = fields(task, TASK_STRINGS, TASK_LISTS)
    require(
        isinstance(result.get("id"), str)
        and re.fullmatch(rf"{phase}\.\d+(?:\.\d+)?", result["id"]) is not None
        and all(isinstance(result.get(key), str) and result[key].strip() for key in ("title", "definition_of_done")),
        "missing or malformed task requirement",
    )
    result["phase"] = phase
    if "substrate_only" in task:
        require(type(task["substrate_only"]) is bool, "invalid substrate flag")
        result["substrate_only"] = task["substrate_only"]
    if "verification" in task:
        result["verification"] = fields(
            task["verification"], ("smoke_addendum",),
            ("library_gates", "behavioral_tests", "snapshot_tests"),
        )
    if "acceptance_scenarios" in task:
        scenarios = []
        for scenario in list_value(task["acceptance_scenarios"]):
            selected = fields(
                scenario, ("id", "source", "scenario"), ("verification", "production_call_sites"),
            )
            require(
                all(isinstance(selected.get(key), str) and selected[key].strip() for key in ("id", "source", "scenario")),
                "missing or malformed acceptance scenario",
            )
            scenarios.append(selected)
        result["acceptance_scenarios"] = scenarios
    if "inference_notes" in task:
        notes = []
        for note in list_value(task["inference_notes"]):
            note = object_value(note)
            if isinstance(note.get("field"), str) and note["field"] in NOTE_FIELDS:
                notes.append(fields(note, ("field", "reason", "source")))
        result["inference_notes"] = notes
    return result


def project_state(state: dict, phase: int) -> dict:
    paths, hashes = registration(state)
    result = {
        "schema_version": state["schema_version"],
        "spec_files": paths, "spec_files_sha256": hashes,
        "tasks": [],
    }
    for key in ("current_phase", "archived_phase"):
        if key in state:
            require(type(state[key]) is int, "invalid ledger phase")
            result[key] = state[key]
    for task in list_value(state.get("tasks")):
        task = object_value(task)
        require(type(task.get("phase")) is int, "missing task phase")
        if task["phase"] == phase:
            result["tasks"].append(project_task(task, phase))
    return result


def read_scope(phase: int, revision: str) -> dict:
    require(phase > 0, "phase must be positive")
    head = git("rev-parse", "--verify", "--end-of-options", revision + "^{commit}").decode("ascii").strip()
    git("merge-base", "--is-ancestor", head, "HEAD")
    raw_sources = {}

    def read(path: str) -> bytes:
        if path not in raw_sources:
            raw_sources[path] = git("show", f"{head}:{path}")
        return raw_sources[path]

    def ledger(path: str) -> dict:
        return object_value(json.loads(read(path).decode("utf-8")))

    root = ledger(ROOT_STATE)
    exit_entry = object_value(object_value(root.get("phase_exit")).get(str(phase)))
    snapshot = f"docs/snapshots/phase{phase}/opi-impl-state.json"
    require(exit_entry.get("snapshot_path") == snapshot, "missing or invalid Phase snapshot registration")
    phase_state = ledger(snapshot)
    require(
        phase_state.get("current_phase", phase_state.get("archived_phase")) == phase,
        "snapshot belongs to a different Phase",
    )
    root_projection = project_state(root, phase)
    root_projection["phase_exit"] = {str(phase): {"snapshot_path": snapshot}}
    phase_projection = project_state(phase_state, phase)
    require(bool(phase_projection["tasks"]), "snapshot has no target Phase tasks")
    specs = list(dict.fromkeys([SPEC, *root_projection["spec_files"], *phase_projection["spec_files"]]))
    baseline = [
        {"path": path, "sha256": hashlib.sha256(read(path)).hexdigest()}
        for path in [ROOT_STATE, snapshot, *specs]
    ]
    mismatches = []
    for ledger_path, projection in [(ROOT_STATE, root_projection), (snapshot, phase_projection)]:
        for path, stored in projection["spec_files_sha256"].items():
            data = read(path)
            # v2 ledger registrations normalize CRLF; audit baselines never do.
            if projection["schema_version"] == 2:
                data = data.replace(b"\r\n", b"\n")
            actual = hashlib.sha256(data).hexdigest()
            if stored != actual:
                mismatches.append({
                    "ledger": ledger_path, "path": path,
                    "stored_sha256": stored, "actual_sha256": actual,
                })
    return {
        "audit_head": head, "phase": phase,
        "baseline_sources": baseline, "stored_hash_mismatches": mismatches,
        "root_state": root_projection, "phase_state": phase_projection,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", type=int, required=True)
    parser.add_argument("--head", default="HEAD", help="committed ancestor to seal (default: HEAD)")
    args = parser.parse_args()
    try:
        scope = read_scope(args.phase, args.head)
    except ScopeError as error:
        print(f"AUDIT-INCOMPLETE: {error}", file=sys.stderr)
        return 1
    except (OSError, ValueError):
        # Parser and OS exception text may contain raw input or history values.
        print("AUDIT-INCOMPLETE: unreadable committed audit input", file=sys.stderr)
        return 1
    print(json.dumps(scope, indent=2, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())

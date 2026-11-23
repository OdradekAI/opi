#!/usr/bin/env python3
"""Record supplied acceptance materials without installing, fetching or overwriting evidence."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DIFFERENTIAL_PATH = Path("scripts/pi-sdk-conformance/differential")
SNAPSHOT = Path(".pi/pi-1.5.0")
MATERIAL_SCHEMA_VERSION = 3
INPUT_PATHS = (
    SNAPSHOT / "package.json", SNAPSHOT / "package-lock.json",
    SNAPSHOT / "node_modules/.package-lock.json", SNAPSHOT / "nix/model-catalog.json",
    SNAPSHOT / "packages/ai/package.json",
    SNAPSHOT / "packages/ai/src/utils/event-stream.ts", SNAPSHOT / "packages/ai/src/api/lazy.ts",
    SNAPSHOT / "packages/ai/scripts/model-data.ts",
    Path("Cargo.toml"), Path("Cargo.lock"),
    Path("crates/opi-ai/Cargo.toml"), Path("crates/opi-telemetry/Cargo.toml"),
    Path("crates/opi-ai/examples/lifecycle_differential.rs"),
    DIFFERENTIAL_PATH / "tools/observe_upstream.mjs", DIFFERENTIAL_PATH / "tools/freeze_acceptance_materials.py",
    DIFFERENTIAL_PATH / "tools/inspect_acceptance_materials.mjs",
    Path("scripts/pi-sdk-conformance/differential/common.py"),
    Path("scripts/pi-sdk-conformance/differential/compare_lifecycle.py"),
    Path("scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py"),
)
TREE_PATHS = (
    *(SNAPSHOT / f"packages/{name}/dist" for name in ("ai", "schema", "telemetry")),
    SNAPSHOT / "packages/ai/src/providers/data",
    Path("crates/opi-ai/src"), Path("crates/opi-telemetry/src"),
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_record(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"status": "missing"}
    if not path.is_file():
        return {"status": "wrong-kind"}
    try:
        data = path.read_bytes()
        return {"status": "present", "bytes": len(data), "sha256": sha256_bytes(data)}
    except OSError as error:
        return {"status": "unreadable", "error": str(error)}


def tree_record(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"status": "missing"}
    if not path.is_dir():
        return {"status": "wrong-kind"}
    rows = []
    try:
        for child in sorted(path.rglob("*")):
            if child.is_symlink() or (hasattr(child, "is_junction") and child.is_junction()):
                return {"status": "unsupported-link", "path": str(child)}
            if child.is_file():
                item = file_record(child)
                if item["status"] != "present":
                    return {"status": "unreadable", "path": str(child)}
                rows.append({"path": child.relative_to(path).as_posix(), "bytes": item["bytes"], "sha256": item["sha256"]})
        rows.sort(key=lambda row: row["path"])
        manifest = "".join(f"{row['sha256']}  {row['path']}\n" for row in rows).encode("utf-8")
        return {"status": "present" if rows else "empty", "file_count": len(rows), "sha256": sha256_bytes(manifest), "files": rows}
    except OSError as error:
        return {"status": "unreadable", "error": str(error)}


def read_revision(path: Path) -> str:
    revision = json.loads(path.read_text(encoding="utf-8"))["revision"]
    if not isinstance(revision, str) or len(revision) != 71 or not revision.startswith("sha256-"):
        raise ValueError("invalid catalog revision")
    if any(char not in "0123456789abcdef" for char in revision[7:]):
        raise ValueError("invalid catalog revision")
    return revision


def verify_catalog(path: Path, revision: str) -> dict[str, Any]:
    record = file_record(path)
    if record["status"] != "present":
        return record
    if "sha256-" + record["sha256"] != revision:
        return {**record, "status": "hash-mismatch", "revision": revision}
    try:
        catalog = json.loads(path.read_bytes())
        if not isinstance(catalog, dict) or not catalog or not all(isinstance(value, list) for value in catalog.values()):
            raise ValueError("catalog must contain provider model arrays")
    except (ValueError, UnicodeError) as error:
        return {**record, "status": "invalid-json", "error": str(error)}
    return {**record, "status": "verified", "revision": revision, "path": str(path.resolve())}


def source_comparison(snapshot: Path, baseline: dict) -> dict:
    rows = baseline["upstream"]["manifest"]["files"]
    changes = []
    for row in rows:
        current = file_record(snapshot / row["path"])
        if current.get("sha256") != row["sha256"]:
            changes.append({"path": row["path"], "expected_sha256": row["sha256"], "current": current})
    return {"checked_files": len(rows), "frozen_manifest_sha256": baseline["upstream"]["manifest"]["sha256"], "changes": changes}


def command_result(command: list[str], root: Path, *, isolated: bool = False) -> dict:
    environment = None
    if isolated:
        allowed = ("PATH", "SystemRoot", "WINDIR", "TEMP", "TMP", "TMPDIR")
        environment = {**{key: os.environ[key] for key in allowed if key in os.environ}, "TZ": "UTC"}
    try:
        result = subprocess.run(command, cwd=root, env=environment, capture_output=True, text=True, encoding="utf-8", timeout=120)
        return {"command": command, "status": "ok" if result.returncode == 0 else "error", "exit_code": result.returncode,
                "stdout": result.stdout, "stderr": result.stderr}
    except (OSError, subprocess.SubprocessError) as error:
        return {"command": command, "status": "error", "error": str(error)}


def inspection_blockers(inspection: dict) -> list[str]:
    blocked = []
    dependencies = inspection.get("dependencies", [])
    if not dependencies or any(row.get("status") != "matched-frozen-direct-record" for row in dependencies):
        blocked.append("installed direct dependency identity")
    dist = inspection.get("dist", [])
    if {row.get("package") for row in dist} != {"ai", "schema", "telemetry"} or any(row.get("status") != "complete-source-maps-match" for row in dist):
        blocked.append("dist completeness/source maps")
    catalog = inspection.get("catalog", {})
    if catalog.get("status") != "pinned-model-values-match":
        blocked.append("generated catalog differs from pinned response")
    if catalog.get("internal_validation") != {"source": "pass", "dist": "pass"}:
        blocked.append("generated data internal validity")
    if catalog.get("source_dist_byte_mismatches"):
        blocked.append("source/dist model data disagree")
    return blocked


def material_paths(root: Path, snapshot: Path, paths: tuple[Path, ...]) -> dict[str, Path]:
    inventory = {}
    for path in paths:
        actual = snapshot / path.relative_to(SNAPSHOT) if path.is_relative_to(SNAPSHOT) else root / path
        key = actual.relative_to(root) if actual.is_relative_to(root) else actual
        inventory[key.as_posix()] = actual
    return inventory


def collect_evidence(
    root: Path, catalog_input: Path, baseline_path: Path, *, snapshot: Path | None = None,
) -> tuple[dict, int]:
    root = root.resolve()
    snapshot = (snapshot if snapshot is not None else root / SNAPSHOT).resolve()
    inputs = {key: file_record(path) for key, path in material_paths(root, snapshot, INPUT_PATHS).items()}
    trees = {key: tree_record(path) for key, path in material_paths(root, snapshot, TREE_PATHS).items()}
    blocked = [f"input: {path}" for path, record in inputs.items() if record["status"] != "present"]
    blocked += [f"tree: {path}" for path, record in trees.items() if record["status"] != "present"]
    code = 0
    try:
        revision = read_revision(snapshot / "nix/model-catalog.json")
        catalog = verify_catalog(catalog_input, revision)
    except (OSError, ValueError, KeyError, TypeError) as error:
        catalog = {"status": "invalid-pin", "error": str(error)}
    if catalog["status"] != "verified":
        blocked.append("catalog revision")
        code = 2
    baseline_record = file_record(baseline_path)
    try:
        baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
        source = source_comparison(snapshot, baseline)
        if source["changes"]:
            blocked.append("frozen upstream source/lock provenance drift")
    except (OSError, ValueError, KeyError, TypeError) as error:
        source = {"error": str(error)}
        blocked.append("frozen upstream source baseline")
    inspection = None
    probe = None
    if catalog["status"] == "verified" and "error" not in source:
        probe = command_result([
            "node", "--experimental-import-meta-resolve", str(root / DIFFERENTIAL_PATH / "tools/inspect_acceptance_materials.mjs"),
            str(snapshot), str(baseline_path.resolve()), str(catalog_input.resolve()),
        ], root, isolated=True)
        try:
            if probe["status"] != "ok":
                raise ValueError("material inspector failed")
            inspection = json.loads(probe["stdout"])
            blocked.extend(inspection_blockers(inspection))
        except (ValueError, KeyError, TypeError) as error:
            blocked.append("material inspector")
            inspection = {"error": str(error)}
    commands = {name: command_result(command, root) for name, command in {
        "git_head": ["git", "rev-parse", "HEAD"], "git_status": ["git", "status", "--short"],
        "node": ["node", "--version"], "rustc": ["rustc", "--version"], "cargo": ["cargo", "--version"],
    }.items()}
    if any(row["status"] != "ok" for row in commands.values()):
        blocked.append("runtime/toolchain identity")
    evidence = {
        "schemaVersion": MATERIAL_SCHEMA_VERSION, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "repository": str(root), "snapshot": str(snapshot),
        "scope": "Read-only current-material inventory; not reproducible-build or package-consumer acceptance.",
        "runtime": {"platform": platform.platform(), "system": platform.system(), "machine": platform.machine(), "python": platform.python_version()},
        "baseline": {"path": str(baseline_path.resolve()), **baseline_record},
        "inputs": inputs, "trees": trees, "catalog": catalog, "source_comparison": source,
        "inspection_command": probe, "inspection": inspection, "commands": commands,
        "blocked_gates": sorted(set(blocked)),
        "not_executed": ["original full-lock reconstruction", "reproducible dist build", "upstream npm package consumer"],
    }
    return evidence, code


def verify_evidence(root: Path, evidence: dict) -> list[str]:
    """Reject drift before replay; original blocked gates remain blocked."""
    if evidence.get("schemaVersion") != MATERIAL_SCHEMA_VERSION:
        return ["unsupported material evidence schema"]
    root = root.resolve()
    if evidence.get("repository") != str(root):
        return ["repository path differs from material evidence"]
    snapshot_path = evidence.get("snapshot")
    if not isinstance(snapshot_path, str) or not Path(snapshot_path).is_absolute():
        return ["material evidence requires an absolute upstream snapshot path"]
    snapshot = Path(snapshot_path)
    if snapshot != snapshot.resolve():
        return ["upstream snapshot path changed after freezing"]
    inputs = material_paths(root, snapshot, INPUT_PATHS)
    trees = material_paths(root, snapshot, TREE_PATHS)
    if set(evidence.get("inputs", {})) != inputs.keys():
        return ["material input inventory is incomplete"]
    if set(evidence.get("trees", {})) != trees.keys():
        return ["material tree inventory is incomplete"]
    errors = []
    for key, path in inputs.items():
        if file_record(path) != evidence["inputs"][key]:
            errors.append(f"input drift: {key}")
    for key, path in trees.items():
        if tree_record(path) != evidence["trees"][key]:
            errors.append(f"tree drift: {key}")
    catalog = evidence["catalog"]
    if catalog.get("status") != "verified" or verify_catalog(Path(catalog["path"]), catalog["revision"]) != catalog:
        errors.append("catalog evidence drift")
    baseline = evidence["baseline"]
    if file_record(Path(baseline["path"])) != {key: value for key, value in baseline.items() if key != "path"}:
        errors.append("baseline evidence drift")
    if errors:
        return errors
    frozen = json.loads(Path(baseline["path"]).read_text(encoding="utf-8"))
    if source_comparison(snapshot, frozen) != evidence["source_comparison"]:
        errors.append("upstream source comparison drift")
    if evidence.get("inspection_command") is not None:
        # Re-resolve from the AI importer so a newly shadowing/nested dependency
        # cannot pass merely because previously recorded files still exist.
        probe = command_result([
            "node", "--experimental-import-meta-resolve", str(root / DIFFERENTIAL_PATH / "tools/inspect_acceptance_materials.mjs"),
            str(snapshot), baseline["path"], catalog["path"],
        ], root, isolated=True)
        try:
            if probe["status"] != "ok" or json.loads(probe["stdout"]) != evidence["inspection"]:
                errors.append("installed/generated material inspection drift")
        except (KeyError, ValueError):
            errors.append("material reinspection failed")
    return errors


def write_new_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, ensure_ascii=False)
        output.write("\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--snapshot", type=Path, help="upstream tree to bind (default: .pi/pi-1.5.0 under --repo)")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--catalog-input", type=Path)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args(argv)
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    catalog = args.catalog_input or args.repo / DIFFERENTIAL_PATH / "fixtures/phase5-model-catalog.json"
    baseline = args.baseline or args.repo / DIFFERENTIAL_PATH / "fixtures/phase1-baseline.json"
    evidence, code = collect_evidence(args.repo, catalog, baseline, snapshot=args.snapshot)
    write_new_json(args.output, evidence)
    print(json.dumps({"status": "recorded", "output": str(args.output), "blocked_gates": evidence["blocked_gates"], "exit_code": code}))
    return code


if __name__ == "__main__":
    raise SystemExit(main())

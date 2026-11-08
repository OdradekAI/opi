"""Restore pinned catalog data in a NEW upstream copy; never install or edit the supplied snapshot.

Copies only baseline-listed source files. Installed dependencies are shared via
explicit directory links; workspace schema/telemetry links target the new copy.
Only the offline hydrator, validators and compiler are run, with a sanitized
environment. Partial candidates and failure evidence are retained for diagnosis.
This does not establish full-lock provenance, npm consumption or SDK alignment.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
from datetime import datetime, timezone

from freeze_acceptance_materials import (
    ITERATION, SNAPSHOT, command_result, file_record, inspection_blockers,
    read_revision, source_comparison, tree_record, verify_catalog, write_new_json,
)

PACKAGES = ("telemetry", "schema", "ai")


def checked_source(snapshot: Path, name: str) -> Path:
    relative = PurePosixPath(name)
    if "\\" in name or relative.is_absolute() or relative.as_posix() != name or any(part in ("..", ".") or ":" in part for part in relative.parts):
        raise ValueError(f"unsafe baseline path: {name}")
    source = snapshot / name
    if not source.resolve().is_relative_to(snapshot.resolve()) or file_record(source)["status"] != "present":
        raise ValueError(f"baseline path is missing, linked or outside snapshot: {name}")
    return source


def check_destinations(snapshot: Path, candidate: Path, output: Path) -> None:
    snapshot = snapshot.resolve()
    for destination in (candidate, output):
        if os.path.lexists(destination):
            raise FileExistsError(f"refusing to overwrite: {destination}")
        resolved = destination.resolve()
        if resolved.is_relative_to(snapshot) or snapshot.is_relative_to(resolved):
            raise ValueError(f"destination overlaps supplied snapshot: {destination}")
    if output.resolve().is_relative_to(candidate.resolve()) or candidate.resolve().is_relative_to(output.resolve()):
        raise ValueError("candidate and evidence paths must be separate")


def copy_sources(snapshot: Path, candidate: Path, baseline: dict) -> int:
    rows = baseline["upstream"]["manifest"]["files"]
    paths = [checked_source(snapshot, row["path"]) for row in rows]
    if len({path.relative_to(snapshot).as_posix() for path in paths}) != len(paths):
        raise ValueError("duplicate baseline paths")
    comparison = source_comparison(snapshot, baseline)
    if any(row["path"] != "package-lock.json" for row in comparison["changes"]):
        raise ValueError("unexpected source drift; only the recorded full-lock drift is allowed")
    candidate.mkdir(parents=True, exist_ok=False)
    for source in paths:
        destination = candidate / source.relative_to(snapshot)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        if file_record(destination) != file_record(source):
            raise ValueError(f"copy identity mismatch: {source}")
    return len(paths)


def link_directory(source: Path, destination: Path) -> None:
    if os.name == "nt":
        # CPython's native junction primitive needs no symlink privilege and
        # never interprets shell metacharacters in caller-selected paths.
        from _winapi import CreateJunction
        CreateJunction(str(source), str(destination))
    else:
        destination.symlink_to(source, target_is_directory=True)


def share_dependencies(snapshot: Path, candidate: Path) -> list[dict]:
    links = []
    replacements = {f"@earendil-works/pi-{name}": candidate / "packages" / name for name in ("schema", "telemetry")}
    roots = [Path("node_modules"), *(Path("packages") / name / "node_modules" for name in PACKAGES)]
    for relative in roots:
        installed = snapshot / relative
        if not installed.is_dir():
            if relative == Path("node_modules"):
                raise ValueError("supplied root node_modules is missing")
            continue
        destination = candidate / relative
        destination.mkdir(parents=True, exist_ok=False)
        for entry in sorted(installed.iterdir()):
            if entry.name.startswith(".") or not entry.is_dir():
                continue  # no npm lock/cache/.bin mutation or shim execution
            entries = sorted(entry.iterdir()) if entry.name.startswith("@") else [entry]
            for package in entries:
                if not package.is_dir():
                    continue
                name = package.relative_to(installed).as_posix()
                target = replacements.get(name, package.resolve())
                link = destination / name
                link.parent.mkdir(parents=True, exist_ok=True)
                link_directory(target, link)
                links.append({"path": link.relative_to(candidate).as_posix(), "target": str(target)})
    return links


def supplied_state(snapshot: Path, baseline: dict) -> dict:
    trees = {}
    for relative in [Path("packages/ai/src/providers/data"), *(Path("packages") / name / "dist" for name in PACKAGES)]:
        record = tree_record(snapshot / relative)
        trees[relative.as_posix()] = {key: value for key, value in record.items() if key != "files"}
    return {"source": source_comparison(snapshot, baseline), "generated_trees": trees}


def restore(root: Path, snapshot: Path, candidate: Path, catalog_path: Path, baseline_path: Path, output: Path) -> tuple[dict, int]:
    check_destinations(snapshot, candidate, output)
    baseline_record = file_record(baseline_path)
    baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    revision = read_revision(snapshot / "nix/model-catalog.json")
    catalog = verify_catalog(catalog_path, revision)
    if catalog["status"] != "verified":
        raise ValueError(f"pinned catalog rejected: {catalog['status']}")
    before = supplied_state(snapshot, baseline)
    commands = []
    record = {
        "schemaVersion": 1, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "scope": "Isolated offline pinned-catalog restoration, not a replacement baseline or full acceptance.",
        "snapshot": str(snapshot), "candidate": str(candidate),
        "baseline": {"path": str(baseline_path), **baseline_record}, "catalog": catalog,
        "supplied_before": before, "commands": commands,
        "supplied_unchanged_scope": {
            "before_after_state": ["baseline-listed source files", "AI generated data", "AI/schema/telemetry dist trees"],
            "success_reinspection": "Direct-dependency manifest/entry identities and generated-material inspection",
            "excluded": ["all files in shared node_modules", "OS-level write protection of shared dependency links"],
        },
        "tool_inputs": {path.name: file_record(path) for path in [
            Path(__file__), Path(__file__).with_name("freeze_acceptance_materials.py"),
            Path(__file__).with_name("inspect_acceptance_materials.mjs"),
            Path(__file__).with_name("verify_catalog_runtime.mjs"),
        ]},
        "not_proven": ["original full-lock identity", "all transitive installed dependency identities",
                       "reproducible clean installation/build", "upstream npm package consumer",
                       "full TS/Rust parity", "core+Faux minimal features"],
    }

    def run(label: str, args: list[str], cwd: Path) -> dict:
        result = command_result(args, cwd, isolated=True)
        commands.append({"label": label, "cwd": str(cwd), **result})
        if result["status"] != "ok":
            raise ValueError(f"command failed: {label}")
        return result

    inspector = root / ITERATION / "tools/inspect_acceptance_materials.mjs"
    inspector_command = ["node", "--experimental-import-meta-resolve", str(inspector)]
    try:
        original = json.loads(run("inspect-supplied", [*inspector_command, str(snapshot), str(baseline_path), str(catalog_path)], root)["stdout"])
        record["copied_source_files"] = copy_sources(snapshot, candidate, baseline)
        record["dependency_links"] = share_dependencies(snapshot, candidate)
        compiler = snapshot / "node_modules/typescript/bin/tsc"
        compiler_manifest = snapshot / "node_modules/typescript/package.json"
        version = json.loads(compiler_manifest.read_text(encoding="utf-8"))["version"]
        declared = json.loads((snapshot / "package.json").read_text(encoding="utf-8"))["devDependencies"]["typescript"]
        record["compiler"] = {"entry": str(compiler), **file_record(compiler),
                              "manifest": file_record(compiler_manifest), "version": version, "declared": declared}
        if version != declared:
            raise ValueError("installed TypeScript version differs from frozen declaration")
        run("typescript-version", ["node", str(compiler), "--version"], candidate)
        ai = candidate / "packages/ai"
        run("hydrate-pinned", ["node", str(ai / "scripts/hydrate-model-catalog.ts"), str(catalog_path)], candidate)
        run("validate-data", ["node", str(ai / "scripts/check-model-data.ts")], candidate)
        for package in PACKAGES:
            run(f"compile-{package}", ["node", str(compiler), "-p", str(candidate / "packages" / package / "tsconfig.build.json")], candidate)
        # Match build:offline's data asset step, including the hidden manifest.
        run("copy-model-assets", ["node", str(candidate / "scripts/package-files.mjs"), "copy", "src/providers/data", "dist/providers"], ai)
        inspection = json.loads(run("inspect-candidate", [*inspector_command, str(candidate), str(baseline_path), str(catalog_path)], root)["stdout"])
        record["inspection"] = inspection
        blockers = inspection_blockers(inspection)
        if blockers:
            raise ValueError(f"restored candidate validation failed: {blockers}")
        record["runtime_catalog"] = json.loads(run("runtime-catalog", ["node", str(Path(__file__).with_name("verify_catalog_runtime.mjs")), str(candidate), str(catalog_path)], root)["stdout"])
        if record["runtime_catalog"]["status"] != "pass":
            raise ValueError("runtime model catalog did not match")
        record["candidate_source"] = source_comparison(candidate, baseline)
        if record["candidate_source"] != before["source"]:
            raise ValueError("candidate source changed beyond generated assets")
        record["candidate_trees"] = {f"packages/{package}/{directory}": tree_record(candidate / "packages" / package / directory)
                                     for package, directory in [(name, "dist") for name in PACKAGES] + [("ai", "src/providers/data")]}
        record["blocked_gates"] = ["frozen upstream source/lock provenance drift"] if before["source"]["changes"] else []
        record["status"] = "catalog-restored-full-provenance-not-established"
        after_inspection = json.loads(run("reinspect-supplied", [*inspector_command, str(snapshot), str(baseline_path), str(catalog_path)], root)["stdout"])
        if after_inspection != original:
            raise ValueError("supplied material inspection changed during restoration")
        code = 0
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        record.update(status="failed", error=str(error))
        code = 1
    after = supplied_state(snapshot, baseline)
    record["supplied_after"] = after
    record["supplied_unchanged"] = before == after
    if before != after or file_record(baseline_path) != baseline_record or verify_catalog(catalog_path, revision) != catalog:
        record.update(status="failed", error="source/baseline/catalog identity changed during restoration")
        code = 1
    write_new_json(output, record)
    return record, code


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    root = args.repo.resolve()
    try:
        evidence, code = restore(root, root / SNAPSHOT, args.candidate.absolute(),
                                 root / ITERATION / "evidence/phase5-model-catalog.json",
                                 root / ITERATION / "evidence/phase1-baseline.json", args.output.absolute())
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"{error}\n")
    print(json.dumps({"status": evidence["status"], "candidate": evidence["candidate"],
                      "output": str(args.output), "error": evidence.get("error"),
                      "blocked_gates": evidence.get("blocked_gates", [])}))
    return code


if __name__ == "__main__":
    raise SystemExit(main())

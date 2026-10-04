"""Existing native CLI orchestration; operator artifacts never replace native seals."""
from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import sys
import tempfile
import tomllib
import uuid

from operator_config import BENCHMARKS, PRIVATE_ENV, MAX_LOGICAL_TRIALS, dumps_toml, experiment, load, resolve
from operator_evidence import (choose_baseline, compare_snapshots, indexed_snapshots, logical_slots,
                               read_json, retry_trials, semantic_identity, unverified_slots)
from operator_process import (BatchDeadline, Commands, OperatorError, check_pin, clean_environment,
                              digest, pin, sha, utc_now, write_bytes, write_json)

REPO = Path(__file__).resolve().parents[4]
EVAL = REPO / "crates/opi-eval"
INDEX = REPO / "docs/eval/evidence-roots.jsonl"


def platform_check():
    if sys.platform != "linux" or platform.machine() != "x86_64":
        raise OperatorError("native preparation/execution requires Linux x86_64 (use WSL2) and Docker")


def external(path, repo=REPO):
    target = Path(path).resolve()
    if target == repo or repo in target.parents:
        raise OperatorError("evaluation outputs must live outside the source checkout")
    if target == Path(target.anchor):
        raise OperatorError("filesystem root cannot be an evaluation output")
    return target


def source_snapshot(repo, commands):
    files = commands.run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], repo).stdout.split("\0")
    included = []
    for name in sorted(set(files)):
        if not name or not (name.startswith(("crates/", ".cargo/")) or name in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain"}):
            continue
        path = repo / name
        if path.is_symlink():
            resolved = path.resolve()
            if repo.resolve() not in resolved.parents or not resolved.is_file():
                raise OperatorError("build source link must select a regular file inside this checkout")
            included.append({"path": name, "link": os.readlink(path), "target": str(resolved.relative_to(repo)), "target_sha256": sha(resolved)})
        elif path.is_file():
            included.append({"path": name, "sha256": sha(path), "executable": bool(path.stat().st_mode & stat.S_IXUSR)})
        else:
            included.append({"path": name, "missing": True})
    return {"head": commands.run(["git", "rev-parse", "HEAD"], repo).stdout.strip(),
            "files": included, "manifest_sha256": digest(included),
            "dirty": commands.run(["git", "status", "--porcelain", "--", "Cargo.toml", "Cargo.lock", "crates", ".cargo", "rust-toolchain.toml"], repo).stdout.splitlines()}


def copy_file(source, target):
    target = Path(target)
    if target.exists() or target.is_symlink():
        raise OperatorError("frozen file destination already exists")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    if sha(source) != sha(target):
        raise OperatorError("file changed while freezing it")
    return target


def freeze_source_file(source, target, expected, commands):
    if not source.is_file() or source.is_symlink():
        raise OperatorError("source inputs changed while freezing them; prepare a fresh candidate")
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise OperatorError("source inputs changed while freezing them; prepare a fresh candidate")
    if any(secret.encode() in data for secret in commands.secrets):
        raise OperatorError("credential found in build source; cannot preserve exportable source inputs")
    write_bytes(target, data, mode=stat.S_IMODE(source.stat().st_mode))


def cargo_build(commands, repo, package, binary, env, target=None):
    argv = ["cargo", "build", "--locked", "--release", "-p", package, "--bin", binary,
            "--message-format=json-render-diagnostics"]
    if target:
        argv += ["--target", target]
    result = commands.run(argv, repo, env=env)
    executables = []
    for line in result.stdout.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == binary and message.get("executable"):
            executables.append(Path(message["executable"]).resolve())
    if len(set(executables)) != 1 or not executables[0].is_file():
        raise OperatorError("locked build did not report one actual compiler executable")
    return executables[0]


def build(commands, root, product, companion=None, repo=REPO):
    """Build current bytes using the existing shared Cargo cache; freeze every executable."""
    before = source_snapshot(repo, commands)
    write_json(root / "builds/source-before.json", before)
    archive = root / "builds/source"
    # Copy regular files first, so a link which sorts before its target does not
    # create that target twice. Links always select bytes inside this archive.
    for entry in before["files"]:
        if "sha256" in entry:
            freeze_source_file(repo / entry["path"], archive / entry["path"], entry["sha256"], commands)
    for entry in before["files"]:
        if "link" not in entry:
            continue
        source = repo / entry["path"]
        resolved = source.resolve()
        if not source.is_symlink() or os.readlink(source) != entry["link"] or repo.resolve() not in resolved.parents or resolved.relative_to(repo.resolve()) != Path(entry["target"]):
            raise OperatorError("source link changed while freezing it; prepare a fresh candidate")
        frozen_target = archive / entry["target"]
        if frozen_target.exists():
            if frozen_target.is_symlink() or sha(frozen_target) != entry["target_sha256"]:
                raise OperatorError("source link target changed while freezing it; prepare a fresh candidate")
        else:
            freeze_source_file(resolved, frozen_target, entry["target_sha256"], commands)
        target = archive / entry["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.symlink_to(os.path.relpath(frozen_target, target.parent))
    env = clean_environment()
    cache_helper = repo / "scripts/opi-cargo-cache.py"
    leased = False
    if not env.get("CARGO_TARGET_DIR"):
        result = commands.run([sys.executable, cache_helper, "resolve", "--workspace", repo], repo)
        env["CARGO_TARGET_DIR"] = result.stdout.strip()
        commands.run([sys.executable, cache_helper, "lease", "start", "--target", env["CARGO_TARGET_DIR"], "--pid", os.getpid()], repo)
        leased = True
    env["CARGO_INCREMENTAL"] = "1"
    try:
        toolchain = commands.run(["rustc", "-Vv"], repo, env=env).stdout
        target = "x86_64-unknown-linux-musl"
        # Static binaries survive heterogeneous benchmark task images. Missing musl
        # tooling is a named build failure; this helper never installs toolchains.
        actual = Path(companion).resolve() if companion else cargo_build(commands, repo, "opi-eval", "opi-eval", env, target)
        frozen = copy_file(actual, root / "builds/companion")
        commands.run([frozen, "--help"], root, env=env)
        opi = None
        if product == "opi":
            built = cargo_build(commands, repo, "opi-coding-agent", "opi", env, target)
            opi = copy_file(built, root / "cache/agents/opi/opi")
        helper = actual if not companion else cargo_build(commands, repo, "opi-eval", "opi-eval", env, target)
        copy_file(helper, root / "cache/tools/native-capture-helper")
        write_json(root / "cache/tools/native-capture-helper.json", {
            "schema": "opi-eval-capture-helper-pin/1", "target": target,
            "protocol": "opi-eval-native-capture-helper/1", "cargo_lock_sha256": sha(repo / "Cargo.lock"), "sha256": sha(helper)})
        after = source_snapshot(repo, commands)
        write_json(root / "builds/source-after.json", after)
        if before != after:
            raise OperatorError("source inputs changed during build; prepare a fresh candidate")
        identity = {"schema": "opi-eval-operator-build/1", "source": pin(root / "builds/source-before.json"),
                    "source_manifest_sha256": before["manifest_sha256"], "head": before["head"],
                    "cargo_lock_sha256": sha(repo / "Cargo.lock"), "rustc": toolchain,
                    "target": target, "profile": "release", "features": "workspace-defaults",
                    "companion": pin(frozen), "companion_origin": "supplied-executable" if companion else "current-working-tree-build",
                    "opi": pin(opi) if opi else None}
        write_json(root / "builds/identity.json", identity)
        return identity
    finally:
        if leased:
            commands.run([sys.executable, cache_helper, "lease", "end", "--target", env["CARGO_TARGET_DIR"], "--pid", os.getpid()], repo)


def stage_cache(profile, root, product, repo=REPO):
    """Physical copies; runners get a fresh .venv at the final immutable path."""
    destination = root / "cache"
    destination.mkdir()
    source = external(profile["storage"]["cache"], repo)
    lock_digest = sha(repo / "crates/opi-eval/external-locks/static/linux-x86_64.json")
    # Cache seeds are reusable material copies, not run/progress records. New
    # runs never use them in place: copy bytes and reinstall the runner .venv.
    seeds = [path for path in (source / "seeds").glob("*") if path.is_dir()
             and (path / ".opi-eval-material-seed.json").is_file()
             and read_json(path / ".opi-eval-material-seed.json").get("static_lock_sha256") == lock_digest]
    if product == "pi":
        pi_seeds = [path for path in seeds if (path / "agents/pi/build-identity.json").is_file()]
        seeds = pi_seeds or seeds
    if seeds:
        source = max(seeds, key=lambda path: path.name)
    if source.is_dir():
        for entry in source.iterdir():
            if entry.name in {"pins", "packages", "profiles", "home", "logs", "results", "seeds", "inventory.json", ".opi-eval-material-seed.json"}:
                continue
            if entry.name == "agents":
                if product == "pi" and (entry / "pi").exists():
                    package = entry / "pi"
                    if package.is_dir():
                        shutil.copytree(package, destination / "agents/pi", symlinks=True)
                    else:
                        copy_file(package, destination / "agents/pi")
                continue
            if entry.name == "tools":
                (destination / "tools").mkdir(exist_ok=True)
                for tool in entry.iterdir():
                    if tool.name.startswith("native-capture-helper"):
                        continue
                    if tool.is_file():
                        copy_file(tool, destination / "tools" / tool.name)
                continue
            if entry.is_dir():
                shutil.copytree(entry, destination / entry.name, symlinks=entry.name != "uv-cache",
                                ignore=shutil.ignore_patterns(".venv", "__pycache__", "*.pyc"))
            elif entry.is_file():
                copy_file(entry, destination / entry.name)
    # Refuse cached links escaping their copied generation. Inert in-corpus links
    # retain their relative text for the native corpus validator.
    for path in destination.rglob("*"):
        if path.is_symlink() and destination.resolve() not in path.resolve().parents:
            raise OperatorError("cache contains an external symlink; seed portable material bytes")
    eval_root = repo / "crates/opi-eval"
    copies = [(eval_root / "external-locks/static/linux-x86_64.json", "pins/static/linux-x86_64.json")]
    lock = read_json(copies[0][0])
    copies.extend((repo / entry["path"], "pins/scripts/" + Path(entry["path"]).name)
                  for entry in lock["authority"]["producers"])
    copies.append((eval_root / "scripts/scripted-provider.py", "pins/scripts/scripted-provider.py"))
    copies.extend((path, "pins/profiles/benchmarks/" + path.name) for path in (eval_root / "profiles/benchmarks").glob("*.toml"))
    copies.extend((path, "pins/integrations/" + family + "/" + path.name)
                  for family in ("harbor", "pier") for path in (eval_root / "integrations" / family).glob("*.py"))
    seen = set()
    for source_file, relative in copies:
        if relative not in seen:
            copy_file(source_file, destination / relative)
            seen.add(relative)
    return lock


def publish_cache_seed(profile, root, repo):
    """Publish fresh reusable bytes without changing any existing cache generation."""
    seeds = external(profile["storage"]["cache"], repo) / "seeds"
    seeds.mkdir(parents=True, exist_ok=True)
    target = seeds / (utc_now().replace(":", "-") + "-" + uuid.uuid4().hex)
    shutil.copytree(root / "cache", target, symlinks=True,
                    ignore=shutil.ignore_patterns(".venv", "__pycache__", "*.pyc", "packages", "profiles", "home", "inventory.json"))
    write_json(target / ".opi-eval-material-seed.json", {"schema": "opi-eval-material-seed/1",
                "static_lock_sha256": sha(repo / "crates/opi-eval/external-locks/static/linux-x86_64.json")})


def verify_pi_package(package, lock):
    """A warm binary hash alone cannot establish a pi version or source pin."""
    package = Path(package)
    identity = read_json(package / "build-identity.json")
    metadata = read_json(package / "package.json")
    selected = next(row for row in lock["subjects"] if row["id"] == "pi")
    blobs = {row["path"]: row for row in selected["blobs"]}
    node = next(row for row in lock["tools"] if row["id"] == "node")
    launcher = b'#!/bin/sh\nagent_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$agent_root/node" "$agent_root/dist/bundle/cli.js" "$@"\n'
    if (metadata.get("version") != selected["model_data_archive"]["version"]
            or sha(package / "package.json") != blobs["packages/coding-agent/package.json"]["sha256"]
            or identity.get("checkout_commit") != selected["commit"]
            or identity.get("checkout_dirty") is not False
            or identity.get("package_lock_sha256") != blobs["package-lock.json"]["sha256"]
            or identity.get("shrinkwrap_sha256") != blobs["packages/coding-agent/npm-shrinkwrap.json"]["sha256"]
            or identity.get("node_version") != "v" + node["version"]
            or identity.get("node_sha256") != sha(package / "node")
            or identity.get("bundle_sha256") != sha(package / "dist/bundle/cli.js")
            or (package / "pi").read_bytes() != launcher):
        raise OperatorError("cached pi package version/source/build differs from admitted static lock")
    return pin(package / "build-identity.json")


def frozen_plan(commands, companion, config, material, out, root):
    result = commands.run([companion, "validate", "--config", config, "--native-material", material], root)
    matches = re.findall(r"native_integrity=([0-9a-f]{64})", result.stdout)
    if len(matches) != 1:
        raise OperatorError("native validation did not derive one integrity identity")
    document = load(config)
    document["benchmark"]["integrity_digest"] = matches[0]
    # This is the sole unfrozen derivation edit, before any plan exists.
    config.write_text(dumps_toml(document), encoding="utf-8")
    commands.run([companion, "validate", "--config", config, "--native-material", material], root)
    commands.run([companion, "plan", "--config", config, "--native-material", material, "--out", out], root)
    plan = read_json(out)
    if plan.get("schema") != "opi-eval-run-plan/2" or not plan.get("material") or not plan.get("trials"):
        raise OperatorError("native plan has no exact material-bound trial expansion")
    if plan["experiment"]["source"] != config.read_text(encoding="utf-8"):
        raise OperatorError("native plan experiment source drift")
    return plan


def verify_report(commands, companion, run, destination, root, canaries=None):
    destination.mkdir(parents=True, exist_ok=False)
    commands.run([companion, "verify", "--root", run], root)
    for name in ("report-a.json", "report-b.json"):
        argv = [companion, "report", "--root", run, "--out", destination / name]
        if canaries:
            argv += ["--canaries", canaries]
        commands.run(argv, root)
    a, b = destination / "report-a.json", destination / "report-b.json"
    if a.read_bytes() != b.read_bytes():
        raise OperatorError("offline native reports are not byte-identical")
    report = read_json(a)
    if report.get("schema") != "opi-eval-normalized-report/2" or report.get("outcome") != "published":
        raise OperatorError("native report is not verified and publishable")
    return {"root": str(run), "report_pin": pin(a), "report": report}


def validate_run_binding(commands, companion, attempt, original, material, destination):
    """Bind the derivative's config/task labels to the native sealed contract."""
    document = load(check_pin(attempt["config"]))
    expected = copy.deepcopy(original)
    if attempt.get("number", 0) > 0:
        expected.pop("selection", None)
        expected["experiment_id"] = document["experiment_id"]
        expected["trials"] = document["trials"]
        original_slots = {(row["subject"], row["task"], row["group"]) for row in attempt["original_trials"]}
        retry_slots = {(row["subject"], row["task"], row["group"]) for row in document["trials"]}
        if not retry_slots or not retry_slots <= original_slots or len(retry_slots) != len(document["trials"]):
            raise OperatorError("retry changed logical scope or duplicated a repetition")
    if document != expected:
        raise OperatorError("native attempt changed frozen model, controls, task conditions or identity")
    summary = commands.run([companion, "validate", "--config", attempt["config"]["path"], "--native-material", material], destination)
    manifests = re.findall(r"\bdigest=([0-9a-f]{64})", summary.stdout)
    integrity = re.findall(r"native_integrity=([0-9a-f]{64})", summary.stdout)
    control = read_json(Path(attempt["root"]) / "run-control.json")
    contract = control.get("experiment_contract", {})
    if (len(manifests) != 1 or len(integrity) != 1
            or control.get("schema") != "opi-eval-run-control/2"
            or contract.get("manifest_digest") != manifests[0]
            or attempt["report"].get("manifest_digest") != manifests[0]
            or attempt["report"].get("integrity_digest") != integrity[0]
            or document["benchmark"].get("integrity_digest") != integrity[0]):
        raise OperatorError("native run contract/report is not bound to the frozen experiment and material")
    declared = document.get("trials", attempt["original_trials"])
    if sorted(control.get("planned_trials", []), key=lambda row: row["id"]) != sorted(declared, key=lambda row: row["id"]):
        raise OperatorError("native run planned trial attribution differs from frozen operation")
    report_trials = attempt["report"].get("trials", [])
    by_id = {row["id"]: row for row in declared}
    if len({row["id"] for row in report_trials}) != len(report_trials):
        raise OperatorError("duplicate native report trial identity")
    for row in report_trials:
        if row["id"] not in by_id or any(row.get(key) != by_id[row["id"]][key] for key in ("task", "subject", "group")):
            raise OperatorError("native report trial attribution differs from declared task/repetition")


def verify_snapshot(snapshot, commands, destination):
    """Rebuild native originals, then recompute the derivative's matching facts."""
    if snapshot.get("schema") != "opi-eval-baseline-snapshot/1":
        raise OperatorError("unsupported snapshot schema")
    companion = check_pin(snapshot["companion"])
    plan_path, material_path = check_pin(snapshot["plan"]), check_pin(snapshot["material"])
    effective_path = check_pin(snapshot["effective"])
    plan, material, profile = read_json(plan_path), read_json(material_path), read_json(effective_path)
    commands.excluded_env.update({profile["credential"].get("reference", PRIVATE_ENV), PRIVATE_ENV})
    original = tomllib.loads(plan["experiment"]["source"])
    actual_reports = []
    for number, attempt in enumerate(snapshot["runs"]):
        verified = verify_report(commands, companion, Path(attempt["root"]), destination / f"attempt-{number}", destination)
        if verified["report_pin"]["sha256"] != attempt["report"]["sha256"]:
            raise OperatorError("native report differs from frozen snapshot input")
        # Keep original report pins in the recomputed slots.
        verified["report_pin"] = attempt["report"]
        verified.update({"config": attempt["config"], "number": number, "original_trials": plan["trials"]})
        validate_run_binding(commands, companion, verified, original, material_path, destination)
        if number == 0 and verified["report"].get("manifest_digest") != plan["experiment"]["manifest_digest"]:
            raise OperatorError("snapshot plan source differs from sealed original experiment")
        actual_reports.append(verified)
    semantic = semantic_identity(profile, snapshot["bench"], material, plan)
    slots = unverified_slots(logical_slots(plan, actual_reports), snapshot["attempts"])
    document = tomllib.loads(plan["experiment"]["source"])
    subjects = document["subjects"]
    if len(subjects) != 1 or subjects[0]["product"] != snapshot["product"] or subjects[0]["version"] != snapshot["version"]:
        raise OperatorError("snapshot subject version/product differs from frozen native experiment")
    check_pin(snapshot["source"])
    check_pin(snapshot["build"])
    builds = read_json(snapshot["build"]["path"])
    if builds["companion"] != snapshot["companion"]:
        raise OperatorError("snapshot Companion differs from frozen build identity")
    lock = read_json(check_pin(material["static_lock"]))
    pi_pin = next(row for row in lock["subjects"] if row["id"] == "pi")
    if snapshot["pi_version"] != pi_pin["model_data_archive"]["version"]:
        raise OperatorError("snapshot pi version differs from retained static lock")
    if snapshot["product"] == "pi":
        agent = material["agents"]["pi"]
        if verify_pi_package(Path(agent["runtime_package"]["path"]), lock) != snapshot["source"]:
            raise OperatorError("snapshot pi build provenance differs")
    else:
        source = read_json(snapshot["source"]["path"])
        if builds["source"] != snapshot["source"] or source["manifest_sha256"] != builds["source_manifest_sha256"] or builds["opi"]["sha256"] != material["agents"]["opi"]["executable"]["sha256"]:
            raise OperatorError("snapshot Opi source/build differs from retained material")
    if semantic != snapshot["semantic"] or slots != snapshot["slots"] or snapshot["complete"] != (bool(slots) and all(slot["valid"] for slot in slots)):
        raise OperatorError("snapshot derivative differs from verified original identities/results")
    return snapshot


def discover(profile, target, commands, root, index=INDEX):
    if profile["baseline"]["lookup"] == "none":
        return None
    # Stable selection based on identities/time, followed by original verification;
    # unavailable/drifted originals reject that baseline, never fall back to best score.
    snapshot = choose_baseline(indexed_snapshots(index), target)
    if snapshot:
        try:
            verify_snapshot(snapshot, commands, root / "checks/baseline")
        except (OperatorError, OSError, KeyError, ValueError):
            write_json(root / "checks/baseline-rejected.json", {"schema": "opi-eval-baseline-rejection/1",
                        "snapshot": snapshot["self"], "reason": "original verification or identity binding failed",
                        "selection": "latest eligible snapshot; no score-based fallback"})
            return None
    return snapshot


def prepare(profile_path, bench, action, *, explicit=(), companion=None, repo=REPO, index=INDEX):
    platform_check()
    raw = load(profile_path)
    profile = resolve(raw, explicit)
    if profile["benchmarks"].get(bench, {}).get("seeds"):
        raise OperatorError("native clients do not project selection seeds; nonempty seeds are a preparation gap")
    workspace = external(profile["storage"]["workspace"], repo)
    workspace.mkdir(parents=True, exist_ok=True)
    product = "pi" if action == "baseline" else "opi"
    identity = action + "-" + bench + "-" + uuid.uuid4().hex
    root = workspace / identity
    root.mkdir(mode=0o700)
    commands = Commands(root / "commands", secrets=[raw["model"].get("api_key")],
                        excluded_env=[profile["credential"].get("reference", PRIVATE_ENV)])
    try:
        write_json(root / "inputs/effective.json", profile)
        lock = stage_cache(profile, root, product, repo)
        pi_pin = next(row for row in lock["subjects"] if row["id"] == "pi")
        pi_version = pi_pin["model_data_archive"]["version"]
        if profile["baseline"]["pi_version"] != pi_version:
            raise OperatorError("requested pi version is not admitted by the current static lock")
        builds = build(commands, root, product, companion, repo)
        version = pi_version if product == "pi" else load(repo / "Cargo.toml")["workspace"]["package"]["version"]
        document = experiment(profile, bench, product, version, identity, root / "cache", root)
        config = root / "inputs/experiment.toml"
        write_bytes(config, dumps_toml(document).encode())
        executable = builds["companion"]["path"]
        commands.run([executable, "doctor", "--config", config], root)
        commands.run([executable, "prepare", "--config", config, "--out", root / "material"], root)
        material_path = root / "material/material.json"
        subject_source = verify_pi_package(root / "cache/agents/pi", lock) if product == "pi" else builds["source"]
        task_count = len(read_json(material_path)["benchmarks"][BENCHMARKS[bench][2]]["tasks"])
        logical_count = task_count * profile["benchmarks"][bench]["repetitions"]
        if logical_count < 1 or logical_count > MAX_LOGICAL_TRIALS:
            raise OperatorError("operator plan exceeds its 10000 logical-trial admission bound")
        document["execution"]["max_trials"] = logical_count
        config.write_text(dumps_toml(document), encoding="utf-8")
        commands.run([sys.executable, repo / "crates/opi-eval/scripts/native-control-proof.py",
                      "--companion", executable, "--config", config, "--material", material_path,
                      "--out", root / "checks/control-proof"], root)
        plan_path = root / "inputs/plan.json"
        plan = frozen_plan(commands, executable, config, material_path, plan_path, root)
        if len(plan["trials"]) != logical_count:
            raise OperatorError("native trial expansion differs from bounded selected material inventory")
        publish_cache_seed(profile, root, repo)
        target = {"product": product, "pi_version": pi_version, "version": version,
                  "semantic": semantic_identity(profile, bench, read_json(material_path), plan),
                  "companion": builds["companion"]}
        baseline = discover(profile, target, commands, root, index) if product == "opi" else None
        comparison_input = {"schema": "opi-eval-comparison-input/1", "candidate_plan": pin(plan_path),
                            "baseline": baseline.get("self") if baseline else None,
                            "status": "selected" if baseline else ("baseline_rejected" if (root / "checks/baseline-rejected.json").is_file() else "baseline_missing")}
        write_json(root / "inputs/comparison-input.json", comparison_input)
        prepared = {"schema": "opi-eval-prepared-operation/1", "id": identity, "action": action, "bench": bench,
                    "product": product, "version": version, "pi_version": pi_version,
                    "profile_path": str(Path(profile_path).resolve()), "effective": pin(root / "inputs/effective.json"),
                    "config": pin(config), "plan": pin(plan_path), "material": pin(material_path),
                    "companion": builds["companion"], "source": subject_source, "build": pin(root / "builds/identity.json"),
                    "semantic": target["semantic"], "comparison_input": pin(root / "inputs/comparison-input.json"),
                    "trial_count": len(plan["trials"]), "maximum_attempted_count": len(plan["trials"]) * (1 + profile["execution"]["trial_retries"]),
                    "budget_seconds": profile["execution"]["batch_timeout_seconds"], "created_at": utc_now()}
        write_json(root / "prepared.json", prepared)
        write_json(root / "handoff.json", {"schema": "opi-eval-handoff/1", "status": "prepared",
                    "operation": str(root), "prepared": pin(root / "prepared.json"), "comparison_input": pin(root / "inputs/comparison-input.json"),
                    "baseline": comparison_input["baseline"], "baseline_status": comparison_input["status"],
                    "trial_count": prepared["trial_count"], "maximum_attempted_count": prepared["maximum_attempted_count"],
                    "budget_seconds": prepared["budget_seconds"], "source": subject_source,
                    "execute_command": [sys.executable, str(Path(__file__).with_name("evaluate.py")), action, "--prepared", str(root), "--execute"]})
        return root
    except Exception:
        write_json(root / "preparation-failure.json", {"status": "preparation_failed", "at": utc_now(),
                    "commands": str(root / "commands"), "live_dispatch": False})
        write_json(root / "handoff.json", {"schema": "opi-eval-handoff/1", "status": "preparation_failed",
                    "operation": str(root), "commands": str(root / "commands"), "gap": "inspect preserved command records"})
        raise OperatorError(f"preparation failed; preserved operation: {root}; handoff: {root / 'handoff.json'}") from None


def resolve_live_credential(profile, raw):
    reference = profile["credential"]
    if reference["kind"] == "private-profile":
        value = raw["model"].get("api_key")
        variable = PRIVATE_ENV
    elif reference["kind"] == "credential_env":
        variable = reference["reference"]
        value = os.environ.get(variable)
    else:
        variable = None
        try:
            value = Path(reference["reference"]).read_text(encoding="utf-8").strip()
        except OSError as exc:
            raise OperatorError("cannot read selected credential file") from exc
    if not isinstance(value, str) or not value or any(ord(char) < 32 for char in value):
        raise OperatorError("selected credential is unavailable or invalid")
    return variable, value


def run_prepared(root, action, *, repo=REPO, index=INDEX):
    platform_check()
    root = external(root, repo)
    prepared = read_json(root / "prepared.json")
    if prepared.get("schema") != "opi-eval-prepared-operation/1" or prepared["action"] != action:
        raise OperatorError("prepared operation action/schema mismatch")
    for key in ("config", "plan", "material", "companion", "effective", "source", "build", "comparison_input"):
        check_pin(prepared[key])
    write_json(root / "execution-authorization.json", {"schema": "opi-eval-authorization/1", "prepared": pin(root / "prepared.json"),
                "source": "explicit --execute", "at": utc_now(), "maximum_attempted_count": prepared["maximum_attempted_count"]})
    profile = read_json(prepared["effective"]["path"])
    raw = load(prepared["profile_path"])
    # Configuration changes require fresh preparation; a key may rotate in the private profile.
    if resolve(raw, profile["overrides"]) != profile:
        raise OperatorError("effective profile changed after preparation")
    variable, secret = resolve_live_credential(profile, raw)
    commands = Commands(root / "commands", secrets=[secret], excluded_env=[variable] if variable else [])
    companion = prepared["companion"]["path"]
    plan = read_json(prepared["plan"]["path"])
    material = prepared["material"]["path"]
    document = load(prepared["config"]["path"])
    if (document != tomllib.loads(plan["experiment"]["source"]) or len(plan["trials"]) != prepared["trial_count"]
            or prepared["maximum_attempted_count"] != len(plan["trials"]) * (1 + profile["execution"]["trial_retries"])
            or prepared["budget_seconds"] != profile["execution"]["batch_timeout_seconds"]
            or prepared["semantic"] != semantic_identity(profile, prepared["bench"], read_json(material), plan)):
        raise OperatorError("prepared metadata differs from its frozen config, scope or budget")
    comparison_input = read_json(prepared["comparison_input"]["path"])
    baseline = None
    if comparison_input["baseline"]:
        snapshot_path = check_pin(comparison_input["baseline"])
        baseline = read_json(snapshot_path)
        baseline["self"] = comparison_input["baseline"]
        verify_snapshot(baseline, commands, root / "checks/baseline-before-run")
    # Re-admit cached material and exact-client proof before any credential dispatch.
    commands.run([companion, "validate", "--config", prepared["config"]["path"], "--native-material", material], root)
    env = clean_environment()
    if variable:
        env[variable] = secret
    reports = []
    attempt_records = []
    deadline = BatchDeadline(profile["execution"]["batch_timeout_seconds"])
    write_json(root / "batch.json", {"started_at": utc_now(), "seconds": profile["execution"]["batch_timeout_seconds"],
                "clock": "monotonic", "retries": profile["execution"]["trial_retries"], "client_retries": "client-owned; separately recorded when measured"})
    # Canary is invocation-owned and deleted even on failure, never under exportable evidence.
    descriptor, canary_name = tempfile.mkstemp(prefix="opi-eval-canary-", text=True)
    canary = Path(canary_name)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        stream.write(secret + "\n")
    stopped = None
    original_ids = {(trial["subject"], trial["task"], trial["group"]): trial["id"] for trial in plan["trials"]}
    try:
        pending = plan["trials"]
        for number in range(1 + profile["execution"]["trial_retries"]):
            if deadline.remaining() <= 0:
                stopped = "batch_deadline"
                break
            run = root / "runs" / f"attempt-{number}"
            attempt_dir = root / "attempts" / str(number)
            attempt_dir.mkdir(parents=True)
            if number == 0:
                attempt_config = prepared["config"]
                argv = [companion, "run", "--plan", prepared["plan"]["path"], "--root", run, "--canaries", canary]
            else:
                retry = copy.deepcopy(document)
                retry.pop("selection", None)
                retry["experiment_id"] = prepared["id"] + f"-retry-{number}"
                retry["trials"] = pending
                retry_config = attempt_dir / "experiment.toml"
                write_bytes(retry_config, dumps_toml(retry).encode())
                attempt_config = pin(retry_config)
                commands.run([companion, "validate", "--config", retry_config, "--native-material", material], root)
                # The native plan command requires selection and cannot express sparse
                # repetition slots. Freeze exact explicit trials instead of replaying scope.
                write_json(attempt_dir / "plan.json", {"schema": "opi-eval-explicit-attempt/1", "config": pin(retry_config),
                            "material": prepared["material"], "trials": pending,
                            "replaces": [{"original": original_ids[(trial["subject"], trial["task"], trial["group"])], "replacement": trial["id"]} for trial in pending]})
                argv = [companion, "run", "--config", retry_config, "--native-material", material, "--root", run, "--canaries", canary]
            record = {"attempt": number, "root": str(run), "trials": pending, "remaining_before_dispatch": deadline.remaining()}
            write_json(attempt_dir / "dispatch.json", record)
            attempt_records.append({**record, "config": attempt_config, "verification": "pending"})
            result = commands.run(argv, root, env=env, deadline=deadline, require=False, allow_credentials=True)
            attempt_records[-1]["exit_code"] = result.returncode
            verified = verify_report(commands, companion, run, root / "checks" / f"attempt-{number}", root, canary)
            verified.update({"config": attempt_config, "number": number, "original_trials": plan["trials"]})
            validate_run_binding(commands, companion, verified, document, material, root)
            reports.append(verified)
            attempt_records[-1].update({"report": verified["report_pin"], "verification": "verified"})
            if getattr(result, "operator_interrupted", False) or getattr(result, "deadline_expired", False):
                stopped = "operator_interrupted" if result.operator_interrupted else "batch_deadline"
                break
            pending = retry_trials(pending, verified["report"], number + 1)
            if not pending:
                break
    except OperatorError as exc:
        stopped = str(exc)
        write_json(root / "execution-failure.json", {"status": "stopped", "reason": stopped, "at": utc_now()})
    finally:
        canary.unlink(missing_ok=True)
    slots = unverified_slots(logical_slots(plan, reports), attempt_records)
    snapshot = {"schema": "opi-eval-baseline-snapshot/1", "id": prepared["id"], "product": prepared["product"],
                "bench": prepared["bench"], "version": prepared["version"], "pi_version": prepared["pi_version"],
                "completed_at": utc_now(), "complete": bool(slots) and all(slot["valid"] for slot in slots),
                "status": "stopped" if stopped else ("complete" if all(slot["valid"] for slot in slots) else "incomplete"),
                "stopped_reason": stopped, "source": prepared["source"], "companion": prepared["companion"],
                "build": prepared["build"],
                "effective": prepared["effective"], "plan": prepared["plan"], "material": prepared["material"],
                "semantic": prepared["semantic"], "slots": slots,
                "runs": [{"root": report["root"], "report": report["report_pin"], "config": report["config"]} for report in reports],
                "attempts": attempt_records, "metrics": {"efficiency": {"state": "unknown", "reason": "native telemetry incomplete"}}}
    snapshot_path = root / "snapshot.json"
    write_json(snapshot_path, snapshot)
    snapshot["self"] = pin(snapshot_path)
    comparison = compare_snapshots(snapshot, baseline) if prepared["product"] == "opi" else None
    if comparison:
        if comparison_input["status"] == "baseline_rejected":
            comparison["status"] = "baseline_rejected"
        write_json(root / "comparison.json", comparison)
        write_json(root / "comparison-manifest.json", {"schema": "opi-eval-comparison-manifest/1",
                    "candidate": pin(snapshot_path), "baseline": comparison_input["baseline"],
                    "comparison": pin(root / "comparison.json"), "frozen_input": prepared["comparison_input"]})
    handoff = {"schema": "opi-eval-handoff/1", "status": snapshot["status"], "coverage": {
                "planned": len(slots), "valid": sum(slot["valid"] for slot in slots), "verified_attempts": sum(len(slot["attempts"]) for slot in slots),
                "dispatched_attempt_runs": len(attempt_records), "unverified_attempt_runs": sum(attempt["verification"] != "verified" for attempt in attempt_records)},
                "operation": str(root), "snapshot": pin(snapshot_path), "comparison": pin(root / "comparison.json") if comparison else None,
                "source": prepared["source"], "effective": prepared["effective"],
                "baseline": comparison_input["baseline"], "baseline_status": comparison["status"] if comparison else "baseline_created",
                "checks": str(root / "checks"), "attempts": str(root / "attempts"), "gap": stopped}
    write_json(root / "result-handoff.json", handoff)
    # Existing index is discovery only, not a progress ledger. Include incomplete
    # baselines honestly; only complete verified snapshots are reusable.
    entry = {"schema": "opi-eval-evidence-roots/1", "kind": "pi-baseline-snapshot" if prepared["product"] == "pi" else "opi-candidate-snapshot", "path": str(root),
             "snapshot": pin(snapshot_path), "handoff": pin(root / "result-handoff.json"),
             "reports": [report["report_pin"] for report in reports], "status": snapshot["status"], "registered": utc_now(), "families": [prepared["bench"]]}
    with Path(index).open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(entry, sort_keys=True) + "\n")
    return handoff


def compare(candidate_path, baseline_path, out, *, index=INDEX):
    out = external(out)
    candidate = read_json(candidate_path)
    candidate["self"] = pin(candidate_path)
    if candidate.get("product") != "opi":
        raise OperatorError("offline compare requires an Opi candidate snapshot")
    baseline = None
    if baseline_path:
        baseline = read_json(baseline_path)
        baseline["self"] = pin(baseline_path)
    else:
        profile = read_json(candidate["effective"]["path"])
        if profile["baseline"]["lookup"] != "none":
            baseline = choose_baseline(indexed_snapshots(index), candidate)
    protected = []
    for snapshot in (candidate, baseline):
        if snapshot is None:
            continue
        protected += [Path(snapshot["self"]["path"]).resolve().parent,
                      Path(snapshot["plan"]["path"]).resolve().parent.parent,
                      Path(snapshot["material"]["path"]).resolve().parent.parent]
        protected += [Path(run["root"]).resolve() for run in snapshot.get("runs", [])]
    if any(out == root or root in out.parents or out in root.parents for root in protected):
        raise OperatorError("comparison output overlaps protected original evidence")
    out.mkdir(mode=0o700, parents=True)
    commands = Commands(out / "commands")
    verify_snapshot(candidate, commands, out / "checks/candidate")
    if baseline:
        if baseline_path:
            verify_snapshot(baseline, commands, out / "checks/baseline")
        else:
            try:
                verify_snapshot(baseline, commands, out / "checks/baseline")
            except (OperatorError, OSError, KeyError, ValueError):
                write_json(out / "checks/baseline-rejected.json", {"snapshot": baseline["self"], "reason": "original verification or identity binding failed"})
                baseline = None
    write_json(out / "comparison-input.json", {"schema": "opi-eval-comparison-input/1", "candidate": candidate["self"],
                "baseline": baseline.get("self") if baseline else None})
    result = compare_snapshots(candidate, baseline)
    if (out / "checks/baseline-rejected.json").is_file():
        result["status"] = "baseline_rejected"
    # Explicit incompatible Companion identities cannot silently become comparable.
    if baseline and baseline["companion"]["sha256"] != candidate["companion"]["sha256"]:
        result["status"] = "incomparable"
        result["reason"] = "Companion changed; grading-path equivalence evidence required"
        result["paired_reward_delta"] = None
    write_json(out / "comparison.json", result)
    write_json(out / "comparison-manifest.json", {"schema": "opi-eval-comparison-manifest/1", "candidate": candidate["self"],
                "baseline": baseline.get("self") if baseline else None, "comparison": pin(out / "comparison.json")})
    write_json(out / "handoff.json", {"schema": "opi-eval-handoff/1", "status": result["status"],
                "comparison": pin(out / "comparison.json"), "inputs": pin(out / "comparison-input.json"), "checks": str(out / "checks")})
    return result

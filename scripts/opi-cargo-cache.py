#!/usr/bin/env python3
"""Observe Cargo targets and review supersession or incremental eviction plans."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import opi_cargo_cache_support as lifecycle
from datetime import UTC, datetime, timedelta
from pathlib import Path


MARKER = ".opi-cargo-cache.json"
LEASE_PREFIX = ".opi-cargo-lease-"
GIB = 1024**3


def now() -> datetime:
    return datetime.now(UTC)


def cache_root(override: str | None = None) -> Path:
    if override:
        root = Path(override)
    elif os.environ.get("OPI_CARGO_CACHE_ROOT"):
        root = Path(os.environ["OPI_CARGO_CACHE_ROOT"])
    elif os.name == "nt" and os.environ.get("LOCALAPPDATA"):
        root = Path(os.environ["LOCALAPPDATA"]) / "opi" / "cargo-targets"
    else:
        base = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
        root = base / "opi" / "cargo-targets"
    return lifecycle.safe_path(root)


def workspace_root(override: str | None = None) -> Path:
    if override:
        return Path(override).expanduser().resolve()
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        return Path(result.stdout.strip()).resolve()
    except (OSError, subprocess.CalledProcessError):
        return Path(__file__).resolve().parent.parent


def toolchain_identity() -> str:
    try:
        result = subprocess.run(
            ["rustc", "-Vv"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        return result.stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise SystemExit(f"cannot resolve rustc toolchain identity: {exc}") from exc


def atomic_json(path: Path, value: dict) -> None:
    lifecycle.atomic(path, value)


def resolved_target(root: Path, workspace: Path, toolchain: str) -> Path:
    digest = hashlib.sha256(f"{workspace}\n{toolchain}".encode()).hexdigest()[:16]
    stem = re.sub(r"[^A-Za-z0-9._-]+", "-", workspace.name).strip("-") or "workspace"
    target = lifecycle.safe_path(root / f"{stem}-{digest}")
    if target.parent != root:
        raise SystemExit(f"resolved target escaped cache root: {target}")
    return target


def cmd_resolve(args: argparse.Namespace) -> int:
    lifecycle.safe_path(args.root or os.environ.get("OPI_CARGO_CACHE_ROOT") or cache_root())
    root = cache_root(args.root)
    workspace = workspace_root(args.workspace)
    toolchain = toolchain_identity()
    root.mkdir(parents=True, exist_ok=True)
    target = resolved_target(root, workspace, toolchain)
    target.mkdir(parents=False, exist_ok=True)
    lifecycle.safe_path(target)
    marker = lifecycle.safe_path(target / MARKER)
    if marker.exists():
        metadata = json.loads(marker.read_text(encoding="utf-8"))
        if (not isinstance(metadata, dict) or metadata.get("schema") != 1
                or metadata.get("workspace") != str(workspace) or metadata.get("toolchain") != toolchain
                or not isinstance(metadata.get("last_used"), str)):
            raise ValueError("existing cache marker has invalid ownership")
    atomic_json(
        target / MARKER,
        {
            "schema": 1,
            "workspace": str(workspace),
            "toolchain": toolchain,
            "last_used": now().isoformat(),
        },
    )
    print(target)
    return 0


def cmd_lease(args: argparse.Namespace) -> int:
    target = lifecycle.safe_path(args.target)
    lifecycle.lease(target, args.pid, args.state == "start", writer=True)
    if args.state == "end":
        with lifecycle.guard(target):
            marker = lifecycle.safe_path(target / MARKER)
            if marker.exists():
                metadata = json.loads(marker.read_text(encoding="utf-8"))
                if not isinstance(metadata, dict) or metadata.get("schema") != 1 or not all(isinstance(metadata.get(k), str) for k in ("workspace", "toolchain", "last_used")):
                    raise ValueError("cannot refresh invalid cache marker")
                metadata["last_used"] = now().isoformat()
                atomic_json(marker, metadata)
    return 0


def directory_size(path: Path) -> int:
    total = 0
    for base, directories, files in os.walk(path, followlinks=False):
        safe_directories = []
        for name in directories:
            try:
                lifecycle.safe_path(Path(base) / name)
                safe_directories.append(name)
            except ValueError:
                pass
        directories[:] = safe_directories
        for name in files:
            try:
                total += lifecycle.safe_path(Path(base) / name).stat().st_size
            except (OSError, ValueError):
                pass
    return total


def entries(root: Path, quick: bool = False, selected: Path | None = None) -> list[dict]:
    if not root.is_dir():
        return []
    output: list[dict] = []
    for path in ([selected] if selected is not None else root.iterdir()):
        if not path.is_dir() or path.is_symlink() or path.parent.resolve() != root:
            continue
        marker = path / MARKER
        metadata = {}
        last_used = datetime.fromtimestamp(path.stat().st_mtime, UTC)
        try:
            lifecycle.safe_path(path)
            if marker.is_file():
                lifecycle.safe_path(marker)
                metadata = json.loads(marker.read_text(encoding="utf-8"))
                if not isinstance(metadata, dict) or metadata.get("schema") != 1 or not all(isinstance(metadata.get(k), str) for k in ("workspace", "toolchain", "last_used")):
                    raise ValueError("invalid marker")
                last_used = datetime.fromisoformat(metadata["last_used"])
                if last_used.tzinfo is None:
                    raise ValueError("marker timestamp lacks timezone")
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
            metadata = {}
        observation_status = "unobserved"
        observed = set()
        candidates = 0
        try:
            if (path / lifecycle.INVENTORY).exists():
                inventory = lifecycle.load(path)
                observation_status = "observed"
                observed.update(lifecycle.protected_paths(inventory))
                observed.update(inventory["candidates"])
                candidates = len(inventory["candidates"])
                for artifacts in inventory["scopes"].values():
                    for files in artifacts.values():
                        observed.update(files)
        except (OSError, ValueError, KeyError, TypeError):
            observation_status = "invalid"
        output.append(
            {
                "path": path.resolve(),
                "workspace": metadata.get("workspace", "?"),
                "last_used": last_used.astimezone(UTC),
                "size": None if quick else directory_size(path),
                "active": lifecycle.blockers(path),
                "marked": bool(metadata),
                "observation_status": observation_status,
                "recorded_files": len(observed),
                "candidate_files": candidates,
                "unknown_files_protected": True,
                "disk_free_bytes": shutil.disk_usage(path).free,
            }
        )
    return output


def print_entries(items: list[dict]) -> None:
    if not items:
        print("no Opi Cargo cache directories")
        return
    for item in sorted(items, key=lambda value: value["last_used"]):
        active = ",".join(map(str, item["active"])) or "-"
        size = "       ?" if item["size"] is None else f"{item['size'] / GIB:8.2f}"
        ownership = "marked" if item["marked"] else "unmarked"
        print(
            f"{size} GiB  {ownership}  "
            f"last={item['last_used'].date()}  active={active}  {item['path']}"
        )
        print(f"  free={item['disk_free_bytes'] / GIB:.2f} GiB  observation={item['observation_status']}  recorded={item['recorded_files']}  candidates={item['candidate_files']}  unknown files protected")
    if all(item["size"] is not None for item in items):
        print(f"total: {sum(item['size'] for item in items) / GIB:.2f} GiB")


def cmd_status(args: argparse.Namespace) -> int:
    if args.target:
        target = lifecycle.safe_path(args.target)
        items = entries(target.parent, args.quick, target)
    else:
        items = entries(cache_root(args.root), args.quick)
    if args.json:
        print(json.dumps(items, default=lambda value: value.isoformat() if isinstance(value, datetime) else str(value), sort_keys=True))
    else:
        print_entries(items)
    return 0


def cmd_prune(args: argparse.Namespace) -> int:
    if args.execute:
        raise ValueError("whole-cache prune execution is disabled; use plan and digest-bound apply --quiescent")
    root = cache_root(args.root)
    items = [item for item in entries(root) if item["marked"]]
    total = sum(item["size"] for item in items)
    threshold = now() - timedelta(days=args.older_than_days)
    planned: list[dict] = []
    remaining = total
    for item in sorted(items, key=lambda value: value["last_used"]):
        if remaining <= args.max_gib * GIB:
            break
        if item["active"] or item["last_used"] > threshold:
            continue
        planned.append(item)
        remaining -= item["size"]

    if not planned:
        print("nothing eligible for pruning")
        print_entries(items)
        return 0

    action = "DELETE" if args.execute else "WOULD DELETE"
    for item in planned:
        print(f"{action}: {item['size'] / GIB:.2f} GiB  {item['path']}")
    print(f"projected total: {remaining / GIB:.2f} GiB")
    if not args.execute:
        print("legacy whole-cache estimate only; deletion requires an observed-file plan and apply --quiescent")
        return 0

    return 0


def invocation_identity(arguments: list[str]) -> tuple[str, str]:
    workspace = workspace_root()
    configs = []
    for ancestor in (Path.cwd(), *Path.cwd().parents):
        for name in ("config", "config.toml"):
            config = ancestor / ".cargo" / name
            if config.is_file():
                configs.append([str(config), hashlib.sha256(config.read_bytes()).hexdigest()])
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    for name in ("config", "config.toml"):
        config = cargo_home / name
        if config.is_file():
            configs.append([str(config), hashlib.sha256(config.read_bytes()).hexdigest()])
    # Persist hashes only: neither command-line config values nor environment
    # values (which can contain tokens) are written to observation receipts.
    environment = {k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"}
    effective = {k: v for k, v in environment.items() if k.startswith(("RUST", "CARGO_BUILD_", "CARGO_TARGET_", "CARGO_PROFILE_"))
                 or k in ("CC", "CXX", "AR", "CFLAGS", "CXXFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_ENCODED_RUSTDOCFLAGS")}
    scope = lifecycle.digest([lifecycle.host(), str(workspace), str(Path.cwd()), toolchain_identity(), arguments, configs, effective])
    lock = workspace / "Cargo.lock"
    revision = lifecycle.digest([hashlib.sha256(lock.read_bytes() if lock.is_file() else b"").hexdigest(), environment])
    return scope, revision


def source_snapshot(target: Path) -> str:
    """Hash local Cargo/source inputs without consulting workflow assurance.

    Includes manifests, Rust/native sources, .cargo config and files in common
    source/test directories. Build scripts can consult inputs outside this set;
    this check only refuses concurrent drift in these observable local inputs.
    """
    root = Path.cwd()
    try:
        result = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True)
    except OSError:
        result = None
    if result is not None and result.returncode == 0:
        root = Path(result.stdout.strip())
        files = subprocess.run(["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"], capture_output=True, check=True).stdout
        paths = [root / os.fsdecode(name) for name in files.split(b"\0") if name]
    else:
        paths = []
        for base, directories, filenames in os.walk(root, followlinks=False):
            directories[:] = [name for name in directories if name not in (".git", ".repo", "target", "node_modules", ".venv")]
            paths.extend(Path(base) / name for name in filenames)
    hasher = hashlib.sha256()
    for path in sorted(set(paths)):
        if path.is_relative_to(target):
            continue
        relative = path.relative_to(root)
        parts = relative.parts
        if (path.name.startswith(".opi-impl-state") or any(part in (".agents", ".claude", ".repo") for part in parts)
                or (len(parts) >= 2 and parts[:2] == ("docs", "snapshots"))):
            continue
        included = (path.name in ("Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain", "rust-toolchain.toml")
                    or path.suffix in (".rs", ".c", ".h", ".cpp", ".cc", ".cxx", ".hpp", ".s", ".S")
                    or any(part in ("src", "tests", "examples", "benches") for part in parts)
                    or (".cargo" in parts and path.name in ("config", "config.toml")))
        if not included:
            continue
        hasher.update(os.fsencode(str(relative)))
        if path.is_symlink():
            hasher.update(os.fsencode(os.readlink(path)))
        elif path.is_file():
            with path.open("rb") as stream:
                for block in iter(lambda: stream.read(1024 * 1024), b""):
                    hasher.update(block)
        else:
            hasher.update(b"missing")
    return hasher.hexdigest()


def cmd_run(args: argparse.Namespace) -> int:
    arguments = args.cargo_args
    if arguments and arguments[0] == "--":
        arguments = arguments[1:]
    if not arguments or arguments[0] not in ("build", "check", "clippy", "test"):
        raise ValueError("run supports Cargo build/check/clippy/test arguments")
    cargo_options = arguments[:arguments.index("--")] if "--" in arguments else arguments
    if any(arg.split("=", 1)[0] in ("--target-dir", "--message-format") for arg in cargo_options):
        raise ValueError("run owns --target-dir and --message-format")
    target = lifecycle.safe_path(args.target)
    scope, revision = invocation_identity(arguments)
    snapshot = source_snapshot(target)
    with lifecycle.guard(target):
        lifecycle.load(target)
        lifecycle.key()
    lifecycle.lease(target, os.getpid(), True, writer=True)
    observed = {}
    observation_gap = None
    build_success = None
    phase_finished = False
    code = 1
    child = None
    interrupted = False
    try:
        command = ["cargo", arguments[0], "--message-format=json-render-diagnostics", "--target-dir", str(target), *arguments[1:]]
        child = subprocess.Popen(command, stdout=subprocess.PIPE)
        if child.poll() is None:
            try:
                lifecycle.lease(target, child.pid, True, writer=True)
            except ValueError:
                if child.poll() is None:
                    raise
        assert child.stdout is not None
        for line in child.stdout:
            sys.stdout.buffer.write(line)
            sys.stdout.buffer.flush()
            try:
                message = json.loads(line)
                if not isinstance(message, dict):
                    continue
                if phase_finished:
                    continue
                if message.get("reason") == "build-finished":
                    build_success = message.get("success") is True
                    phase_finished = True
                item = lifecycle.observe(target, message)
                if item:
                    observed.setdefault(item[0], {}).update(item[1])
                    observation_gap = observation_gap or item[2]
            except (ValueError, TypeError):
                continue
        code = child.wait()
    except KeyboardInterrupt:
        interrupted = True
        if child is not None:
            child.terminate()
            child.wait()
        code = 130
    except (OSError, ValueError) as exc:
        if child is None:
            raise
        interrupted = True
        print(f"Cargo cache observation interrupted: {exc}", file=sys.stderr)
        if child.poll() is None:
            child.terminate()
        code = child.wait()
    finally:
        observation_error = False
        try:
            if child is not None:
                if child.poll() is None:
                    child.terminate()
                child.wait()
            try:
                gap = observation_gap if source_snapshot(target) == snapshot else "local inputs changed during Cargo"
            except (OSError, ValueError, subprocess.SubprocessError):
                gap = "cannot verify local inputs after Cargo"
            if interrupted:
                gap = "interrupted writer; descendants require operator inspection"
            lifecycle.record_observed(target, scope, lifecycle.digest([revision, snapshot]), observed, build_success, code, gap=gap)
        except (OSError, ValueError, TypeError, KeyError) as exc:
            observation_error = True
            print(f"Cargo cache observation not saved: {exc}; writer lease retained", file=sys.stderr)
        if not interrupted and not observation_error:
            try:
                if child is not None:
                    lifecycle.lease(target, child.pid, False)
                lifecycle.lease(target, os.getpid(), False)
            except (OSError, ValueError) as exc:
                print(f"Cargo cache lease release failed: {exc}; inspect remaining writer leases", file=sys.stderr)
        else:
            print(f"Cargo cache unresolved writer lease retained for PID {os.getpid()}; inspect descendants before explicit lease end", file=sys.stderr)
    return code


def cmd_plan(args: argparse.Namespace) -> int:
    target = lifecycle.safe_path(args.target)
    plan = lifecycle.make_plan(target)
    output = lifecycle.safe_path(args.output)
    if output.is_relative_to(target):
        raise ValueError("write reviewed plans outside the target directory")
    atomic_json(output, plan)
    print(hashlib.sha256(output.read_bytes()).hexdigest())
    return 0


def eviction_output_protection(output: Path, protection: Path) -> str:
    """Keep review output outside the evidence exclusions it is about to bind."""
    raw = protection.read_bytes()
    value = json.loads(raw)
    if not isinstance(value, dict) or any(not isinstance(value.get(k), list) for k in ("files", "trees", "bindings")):
        raise ValueError("invalid protection manifest")
    protected_files = [*value["files"], *(item["path"] for item in value["bindings"])]
    if any(output == lifecycle.safe_path(path) for path in protected_files):
        raise ValueError("plan output is a protected evidence file")
    if any(output.is_relative_to(lifecycle.safe_path(tree)) for tree in value["trees"]):
        raise ValueError("plan output must stay outside protected evidence trees")
    return hashlib.sha256(raw).hexdigest()


def cmd_plan_eviction(args: argparse.Namespace) -> int:
    import opi_cargo_cache_eviction as eviction

    target = lifecycle.safe_path(args.target)
    output = lifecycle.safe_path(args.output)
    if output.is_relative_to(target):
        raise ValueError("write reviewed plans outside the target directory")
    if output.exists():
        raise ValueError("plan output already exists; choose a new review artifact path")
    protection = lifecycle.safe_path(args.protection)
    if output == protection:
        raise ValueError("plan output must not overwrite its protection manifest")
    protection_digest = eviction_output_protection(output, protection)
    plan = eviction.make_plan(target, workspace_root(args.workspace), args.package, protection)
    if eviction_output_protection(output, protection) != protection_digest:
        raise ValueError("protection manifest changed while planning")
    # Planning can be lengthy: another task may create this path after admission.
    # Exclusive creation preserves that artifact; an interrupted JSON write is
    # unusable until a complete reviewed digest has been produced.
    with lifecycle.safe_path(output).open("x", encoding="utf-8") as stream:
        json.dump(plan, stream, sort_keys=True, indent=2)
        stream.write("\n")
    print(hashlib.sha256(output.read_bytes()).hexdigest())
    print(json.dumps(plan["summary"], sort_keys=True))
    return 0


def cmd_apply(args: argparse.Namespace) -> int:
    if not args.quiescent:
        raise ValueError("apply requires --quiescent assertion for unmanaged writers")
    raw = lifecycle.safe_path(args.plan).read_bytes()
    if hashlib.sha256(raw).hexdigest() != args.sha256:
        raise ValueError("reviewed plan SHA256 does not match")
    plan = json.loads(raw)
    if isinstance(plan, dict) and plan.get("kind") == "incremental-eviction":
        import opi_cargo_cache_eviction as eviction
        count = eviction.apply_plan(plan)
        print(f"removed {count} reviewed incremental payload files")
    else:
        count = lifecycle.apply_plan(plan)
        print(f"removed {count} observed superseded files")
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="command", required=True)

    resolve = sub.add_parser("resolve", help="create/reuse and print this worktree's cache")
    resolve.add_argument("--root")
    resolve.add_argument("--workspace")
    resolve.set_defaults(func=cmd_resolve)

    lease = sub.add_parser("lease", help="mark a cache active/inactive for safe pruning")
    lease.add_argument("state", choices=("start", "end"))
    lease.add_argument("--target", required=True)
    lease.add_argument("--pid", type=int, required=True)
    lease.set_defaults(func=cmd_lease)

    status = sub.add_parser("status", help="report marked/unmarked targets, activity and observation coverage")
    status.add_argument("--root")
    status.add_argument("--target")
    status.add_argument("--quick", action="store_true")
    status.add_argument("--json", action="store_true")
    status.set_defaults(func=cmd_status)

    prune = sub.add_parser("prune", help="legacy whole-cache estimate only; --execute is refused")
    prune.add_argument("--root")
    prune.add_argument("--max-gib", type=float, default=20.0)
    prune.add_argument("--older-than-days", type=float, default=14.0)
    prune.add_argument("--execute", action="store_true")
    prune.set_defaults(func=cmd_prune)
    run = sub.add_parser("run", help="observe a leased Cargo build without deleting files")
    run.add_argument("--target", required=True)
    run.add_argument("cargo_args", nargs=argparse.REMAINDER)
    run.set_defaults(func=cmd_run)
    plan = sub.add_parser("plan", help="write an exact observed-file maintenance plan; print SHA256")
    plan.add_argument("--target", required=True)
    plan.add_argument("--output", required=True)
    plan.set_defaults(func=cmd_plan)
    eviction = sub.add_parser("plan-eviction", help="review rebuildable incremental payload eviction; never delete")
    eviction.add_argument("--target", required=True)
    eviction.add_argument("--workspace", required=True)
    eviction.add_argument("--package", action="append", required=True)
    eviction.add_argument("--protection", required=True)
    eviction.add_argument("--output", required=True)
    eviction.set_defaults(func=cmd_plan_eviction)
    apply = sub.add_parser("apply", help="apply only an unchanged reviewed plan")
    apply.add_argument("--plan", required=True)
    apply.add_argument("--sha256", required=True)
    apply.add_argument("--quiescent", action="store_true")
    apply.set_defaults(func=cmd_apply)
    return root


if __name__ == "__main__":
    arguments = parser().parse_args()
    try:
        raise SystemExit(arguments.func(arguments))
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError) as exc:
        print(f"Cargo cache: {exc}", file=sys.stderr)
        raise SystemExit(2) from None

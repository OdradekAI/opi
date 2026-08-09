#!/usr/bin/env python3
"""Resolve, lease, inspect, and explicitly prune Opi Cargo target caches."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import uuid
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
    return root.expanduser().resolve()


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
    temp = path.with_name(f"{path.name}.tmp-{os.getpid()}-{uuid.uuid4().hex}")
    temp.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temp, path)


def resolved_target(root: Path, workspace: Path, toolchain: str) -> Path:
    digest = hashlib.sha256(f"{workspace}\n{toolchain}".encode()).hexdigest()[:16]
    stem = re.sub(r"[^A-Za-z0-9._-]+", "-", workspace.name).strip("-") or "workspace"
    target = (root / f"{stem}-{digest}").resolve()
    if target.parent != root:
        raise SystemExit(f"resolved target escaped cache root: {target}")
    return target


def cmd_resolve(args: argparse.Namespace) -> int:
    root = cache_root(args.root)
    workspace = workspace_root(args.workspace)
    toolchain = toolchain_identity()
    root.mkdir(parents=True, exist_ok=True)
    target = resolved_target(root, workspace, toolchain)
    target.mkdir(parents=False, exist_ok=True)
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


def pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    if os.name == "nt":
        process = ctypes.windll.kernel32.OpenProcess(0x1000, False, pid)
        if not process:
            return False
        ctypes.windll.kernel32.CloseHandle(process)
        return True
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def validate_marked_target(target_arg: str) -> tuple[Path, Path]:
    target = Path(target_arg).expanduser().resolve()
    marker = target / MARKER
    if not marker.is_file():
        raise SystemExit(f"refusing unmarked Cargo cache: {target}")
    root = target.parent.resolve()
    if target.parent != root or target.is_symlink():
        raise SystemExit(f"refusing unsafe Cargo cache path: {target}")
    return target, marker


def cmd_lease(args: argparse.Namespace) -> int:
    target, marker = validate_marked_target(args.target)
    lease = target / f"{LEASE_PREFIX}{args.pid}.json"
    if args.state == "start":
        atomic_json(lease, {"pid": args.pid, "started_at": now().isoformat()})
    else:
        lease.unlink(missing_ok=True)
        try:
            metadata = json.loads(marker.read_text(encoding="utf-8"))
            metadata["last_used"] = now().isoformat()
            atomic_json(marker, metadata)
        except (OSError, ValueError, TypeError, json.JSONDecodeError) as exc:
            raise SystemExit(f"cannot refresh cache marker: {exc}") from exc
    return 0


def directory_size(path: Path) -> int:
    total = 0
    for base, _, files in os.walk(path, followlinks=False):
        for name in files:
            try:
                total += (Path(base) / name).stat().st_size
            except OSError:
                pass
    return total


def active_pids(path: Path) -> list[int]:
    active: list[int] = []
    for lease in path.glob(f"{LEASE_PREFIX}*.json"):
        try:
            pid = int(lease.stem.removeprefix(LEASE_PREFIX))
        except ValueError:
            continue
        if pid_alive(pid):
            active.append(pid)
    return sorted(active)


def entries(root: Path) -> list[dict]:
    if not root.is_dir():
        return []
    output: list[dict] = []
    for path in root.iterdir():
        if not path.is_dir() or path.is_symlink() or path.parent.resolve() != root:
            continue
        marker = path / MARKER
        if not marker.is_file():
            continue
        try:
            metadata = json.loads(marker.read_text(encoding="utf-8"))
            last_used = datetime.fromisoformat(metadata["last_used"])
            if last_used.tzinfo is None:
                last_used = last_used.replace(tzinfo=UTC)
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
            continue
        output.append(
            {
                "path": path.resolve(),
                "workspace": metadata.get("workspace", "?"),
                "last_used": last_used.astimezone(UTC),
                "size": directory_size(path),
                "active": active_pids(path),
            }
        )
    return output


def print_entries(items: list[dict]) -> None:
    if not items:
        print("no marked Opi Cargo caches")
        return
    for item in sorted(items, key=lambda value: value["last_used"]):
        active = ",".join(map(str, item["active"])) or "-"
        print(
            f"{item['size'] / GIB:8.2f} GiB  "
            f"last={item['last_used'].date()}  active={active}  {item['path']}"
        )
    print(f"total: {sum(item['size'] for item in items) / GIB:.2f} GiB")


def cmd_status(args: argparse.Namespace) -> int:
    print_entries(entries(cache_root(args.root)))
    return 0


def cmd_prune(args: argparse.Namespace) -> int:
    root = cache_root(args.root)
    items = entries(root)
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
        print("dry run; pass --execute to remove only the listed marked caches")
        return 0

    for item in planned:
        path = item["path"].resolve()
        if path.parent != root or path.is_symlink() or not (path / MARKER).is_file():
            raise SystemExit(f"safety check changed before delete: {path}")
        if active_pids(path):
            raise SystemExit(f"cache became active before delete: {path}")
        shutil.rmtree(path)
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

    status = sub.add_parser("status", help="report marked cache size, age, and activity")
    status.add_argument("--root")
    status.set_defaults(func=cmd_status)

    prune = sub.add_parser("prune", help="prune old inactive marked caches; dry-run by default")
    prune.add_argument("--root")
    prune.add_argument("--max-gib", type=float, default=20.0)
    prune.add_argument("--older-than-days", type=float, default=14.0)
    prune.add_argument("--execute", action="store_true")
    prune.set_defaults(func=cmd_prune)
    return root


if __name__ == "__main__":
    arguments = parser().parse_args()
    raise SystemExit(arguments.func(arguments))

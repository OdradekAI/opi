"""Reviewed eviction of verified incremental payloads, independent of freshness.

The caller must require a reviewed plan digest and unmanaged-writer quiescence.
Planning is read-only. Execution preserves directories, session locks, metadata
and every file that is not a verified member of a complete selected payload set.
"""
from __future__ import annotations

from contextlib import ExitStack, contextmanager, nullcontext
from concurrent.futures import ThreadPoolExecutor, wait
from contextvars import ContextVar
import ctypes
from datetime import datetime
import hashlib
from functools import wraps
import json
import os
from pathlib import Path
import platform
import re
import stat
import subprocess

import opi_cargo_cache_support as support

SESSION = re.compile(r"s-[0-9a-z]+-[0-9a-z]+-[0-9a-z]+\Z")
UNIT = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)-([0-9a-z]+)\Z")
BIN_NAMES = {"dep-graph.bin", "query-cache.bin", "work-products.bin"}
PLAN_KEYS = {"schema", "kind", "host", "target", "workspace", "packages", "target_identity",
             "workspace_identity", "metadata", "ownership", "protection", "incremental_roots",
             "units", "sessions", "files", "summary", "skipped", "leases"}
_POSIX_LOCK_STREAMS = {}
_HEADER_POOL = ContextVar("incremental_eviction_header_pool", default=None)


def _with_header_pool(operation):
    @wraps(operation)
    def run(*args, **kwargs):
        if os.name != "nt" or _HEADER_POOL.get() is not None:
            return operation(*args, **kwargs)
        pool = ThreadPoolExecutor(max_workers=8)
        token = _HEADER_POOL.set(pool)
        try:
            return operation(*args, **kwargs)
        finally:
            try:
                pool.shutdown(wait=True, cancel_futures=True)
            finally:
                _HEADER_POOL.reset(token)
    return run


def _payload_infos(paths):
    pool = _HEADER_POOL.get()
    if pool is None:
        return [(path, file_info(path, header=True)) for path in paths]
    result = []
    for offset in range(0, len(paths), 256):
        batch = paths[offset:offset + 256]
        futures = []
        try:
            for path in batch:
                futures.append(pool.submit(file_info, path, header=True))
            wait(futures)
            result.extend((path, future.result()) for path, future in zip(batch, futures))
        finally:
            # A failed result, submission or interrupt cannot release a native
            # session lock while another worker still holds a payload handle.
            for future in futures:
                future.cancel()
            while any(not future.done() for future in futures):
                try:
                    wait(futures)
                except KeyboardInterrupt:
                    continue
    return result


def sha256(path):
    hasher = hashlib.sha256()
    with support.safe_path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def equivalent(value):
    """Compare only drive-letter /mnt paths and native lexical absolute paths."""
    text = str(value).replace("\\", "/").rstrip("/")
    match = re.fullmatch(r"/mnt/([a-zA-Z])/(.*)", text)
    if match:
        text = match[1] + ":/" + match[2]
    if re.match(r"^[a-zA-Z]:/", text):
        return text.casefold()
    return text


def cargo_metadata(workspace):
    result = subprocess.run(
        ["cargo", "metadata", "--offline", "--locked", "--all-features", "--format-version", "1",
         "--manifest-path", str(workspace / "Cargo.toml")],
        cwd=workspace, check=True, capture_output=True, text=True, encoding="utf-8",
    )
    return json.loads(result.stdout)


def metadata_scope(workspace, packages):
    if (not isinstance(packages, list) or not packages
            or any(not isinstance(p, str) or not p for p in packages)
            or len(set(packages)) != len(packages)):
        raise ValueError("select distinct nonempty workspace package names")
    metadata = cargo_metadata(workspace)
    if equivalent(metadata["workspace_root"]) != equivalent(workspace):
        raise ValueError("metadata workspace does not match selected workspace")
    members = set(metadata["workspace_members"])
    records = {p["id"]: p for p in metadata["packages"]}
    names = {p["name"] for identity, p in records.items() if identity in members}
    if not set(packages).issubset(names):
        raise ValueError("selected package is not a current workspace member")
    selected = {identity for identity, p in records.items() if identity in members and p["name"] in packages}
    nodes = {node["id"]: node for node in (metadata.get("resolve") or {}).get("nodes", [])}
    if set(nodes) != set(records) or not members.issubset(records):
        raise ValueError("incomplete dependency ownership: missing resolved package")
    # All workspace features and target platforms participate in the current
    # resolved graph. Target names delimit that scope; they cannot establish
    # historical ownership for removed or inactive dependencies.
    for node in nodes.values():
        dependencies = set(node["dependencies"])
        if dependencies != {edge["pkg"] for edge in node["deps"]} or not dependencies.issubset(records):
            raise ValueError("incomplete dependency ownership: invalid resolved edge")
    owners = {}
    binding = []
    for identity, package in sorted(records.items()):
        targets = []
        for target in package["targets"]:
            stem = target["name"].replace("-", "_")
            owners.setdefault(stem, set()).add(identity)
            targets.append({k: target[k] for k in ("name", "kind", "src_path")})
        binding.append({"id": package["id"], "name": package["name"], "manifest_path": package["manifest_path"], "targets": targets})
    admitted = {stem for stem, pkgs in owners.items() if pkgs.issubset(selected) and stem != "build_script_build"}
    result = {"sha256": support.digest(binding), "configured_target": metadata["target_directory"],
              "admitted_stems": sorted(admitted)}
    return result


def ownership(target, workspace, metadata):
    marker = support.safe_path(target / ".opi-cargo-cache.json")
    if marker.exists():
        value = json.loads(marker.read_text(encoding="utf-8"))
        if (not isinstance(value, dict) or value.get("schema") != 1
                or not all(isinstance(value.get(k), str) and value[k] for k in ("workspace", "toolchain", "last_used"))
                or equivalent(value["workspace"]) != equivalent(workspace)
                or datetime.fromisoformat(value["last_used"]).tzinfo is None):
            raise ValueError("invalid or unrelated persistent target marker")
        return {"kind": "marker", "path": str(marker), "sha256": sha256(marker)}
    if equivalent(metadata["configured_target"]) != equivalent(target):
        raise ValueError("target is neither marked for workspace nor currently configured")
    return {"kind": "configured-target", "path": str(target)}


def protection_manifest(path, target):
    path = support.safe_path(path)
    if path.is_relative_to(target):
        raise ValueError("protection manifest must be outside target")
    raw = path.read_bytes()
    value = json.loads(raw)
    if (not isinstance(value, dict) or set(value) != {"schema", "files", "trees", "bindings"}
            or value["schema"] != 1 or any(not isinstance(value[k], list) for k in ("files", "trees", "bindings"))):
        raise ValueError("invalid protection manifest")
    paths = {}
    for key in ("files", "trees"):
        paths[key] = set()
        for item in value[key]:
            if not isinstance(item, str) or not Path(item).is_absolute():
                raise ValueError("protection paths must be absolute")
            protected_path = support.safe_path(item)
            if protected_path.is_relative_to(target) or (key == "trees" and target.is_relative_to(protected_path)):
                paths[key].add(protected_path)
    bindings = []
    for item in value["bindings"]:
        if (not isinstance(item, dict) or set(item) != {"path", "sha256"}
                or not isinstance(item["path"], str) or not Path(item["path"]).is_absolute()
                or not isinstance(item["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", item["sha256"])):
            raise ValueError("invalid protection evidence binding")
        bound = support.safe_path(item["path"])
        if sha256(bound) != item["sha256"]:
            raise ValueError("protection evidence binding changed")
        bindings.append({"path": str(bound), "sha256": item["sha256"]})
        if bound.is_relative_to(target):
            paths["files"].add(bound)
    return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest(), "bindings": bindings}, paths


def protected(path, protections):
    return path in protections["files"] or path in protections["trees"] or any(parent in protections["trees"] for parent in path.parents)


if os.name == "nt":
    from ctypes import wintypes

    class StandardInfo(ctypes.Structure):
        _fields_ = [("allocation", ctypes.c_longlong), ("size", ctypes.c_longlong),
                    ("links", wintypes.DWORD), ("delete_pending", ctypes.c_ubyte), ("directory", ctypes.c_ubyte)]

    class BasicInfo(ctypes.Structure):
        _fields_ = [("creation", ctypes.c_longlong), ("access", ctypes.c_longlong),
                    ("write", ctypes.c_longlong), ("change", ctypes.c_longlong), ("attributes", wintypes.DWORD)]

    class HandleInfo(ctypes.Structure):
        _fields_ = [("attributes", wintypes.DWORD), ("creation", wintypes.FILETIME),
                    ("access", wintypes.FILETIME), ("write", wintypes.FILETIME),
                    ("volume", wintypes.DWORD), ("size_high", wintypes.DWORD), ("size_low", wintypes.DWORD),
                    ("links", wintypes.DWORD), ("index_high", wintypes.DWORD), ("index_low", wintypes.DWORD)]

    class Overlapped(ctypes.Structure):
        _fields_ = [("internal", ctypes.c_size_t), ("internal_high", ctypes.c_size_t),
                    ("offset", wintypes.DWORD), ("offset_high", wintypes.DWORD), ("event", wintypes.HANDLE)]

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p,
                                  wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.GetFileInformationByHandle.argtypes = [wintypes.HANDLE, ctypes.POINTER(HandleInfo)]
    kernel.GetFileInformationByHandleEx.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
    kernel.LockFileEx.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD,
                                 wintypes.DWORD, ctypes.POINTER(Overlapped)]
    kernel.UnlockFileEx.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.DWORD,
                                   wintypes.DWORD, ctypes.POINTER(Overlapped)]


def windows_info(handle, regular=False):
    basic, standard, info = BasicInfo(), StandardInfo(), HandleInfo()
    if (not kernel.GetFileInformationByHandle(handle, ctypes.byref(info))
            or not kernel.GetFileInformationByHandleEx(handle, 0, ctypes.byref(basic), ctypes.sizeof(basic))
            or not kernel.GetFileInformationByHandleEx(handle, 1, ctypes.byref(standard), ctypes.sizeof(standard))):
        raise ctypes.WinError(ctypes.get_last_error())
    if info.attributes & 0x400 or standard.delete_pending:
        raise ValueError("reparse point or pending deletion refused")
    if regular and (standard.directory or info.attributes & 0x10):
        raise ValueError("payload or lock is not a regular file")
    return {"id": [info.volume, info.index_high << 32 | info.index_low], "size": standard.size,
            "allocated": standard.allocation, "links": standard.links, "mtime": basic.write,
            "ctime": basic.change, "birth": basic.creation}


def posix_info(info):
    if not stat.S_ISREG(info.st_mode):
        raise ValueError("payload or lock is not a regular file")
    return {"id": [info.st_dev, info.st_ino], "size": info.st_size,
            "allocated": info.st_blocks * 512, "links": info.st_nlink,
            "mtime": info.st_mtime_ns, "ctime": info.st_ctime_ns}


def file_info(path, header=False):
    path = support.safe_path(path)
    if not path.is_file():
        raise ValueError("payload or lock must be an existing regular file")
    if os.name == "nt" and not header:
        handle = kernel.CreateFileW(str(path), 0x80, 7, None, 3, 0x00200000, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            info = windows_info(handle, regular=True)
            if windows_info(handle, regular=True) != info:
                raise ValueError("file changed during observation")
            return info
        finally:
            kernel.CloseHandle(handle)
    held = _POSIX_LOCK_STREAMS.get(path)
    with (nullcontext(held) if held is not None else path.open("rb")) as stream:
        if os.name == "nt":
            import msvcrt
            info = windows_info(msvcrt.get_osfhandle(stream.fileno()), regular=True)
        else:
            info = posix_info(os.fstat(stream.fileno()))
            if held is not None and posix_info(path.stat()) != info:
                raise ValueError("held session lock path changed")
        data = stream.read(262) if header else b""
        after = windows_info(msvcrt.get_osfhandle(stream.fileno()), regular=True) if os.name == "nt" else posix_info(os.fstat(stream.fileno()))
        if info != after:
            raise ValueError("file changed during observation")
    if header:
        info["header"] = payload_header(path.name, data, info["size"])
    return info


def directory_identity(path):
    path = support.safe_path(path)
    if not path.is_dir():
        raise ValueError("expected existing directory")
    if os.name == "nt":
        handle = kernel.CreateFileW(str(path), 0x80, 7, None, 3, 0x02200000, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            return windows_info(handle)["id"]
        finally:
            kernel.CloseHandle(handle)
    info = path.stat()
    return [info.st_dev, info.st_ino]


def payload_header(name, data, size):
    if name.endswith(".bin"):
        if name not in BIN_NAMES or len(data) < 8 or data[:6] != b"RSIC\0\0":
            raise ValueError("unknown incremental binary header")
        length = data[6]
        version = data[7:7 + length]
        if (not length or len(version) != length or size <= 7 + length
                or not re.fullmatch(rb"[0-9]+\.[0-9]+\.[0-9]+[^\x00-\x1f\x7f]*", version)):
            raise ValueError("invalid Rust incremental version header")
        return data[:7 + length].hex()
    if not name.endswith(".o") or size < 20:
        raise ValueError("unknown compiler payload")
    if data.startswith(b"\x7fELF") and len(data) >= 64 and data[4] in (1, 2) and data[5] in (1, 2):
        endian = "little" if data[5] == 1 else "big"
        header_size = 52 if data[4] == 1 else 64
        offset = 40 if data[4] == 1 else 52
        if (data[6] == 1 and int.from_bytes(data[16:18], endian) == 1
                and int.from_bytes(data[18:20], endian) > 0 and int.from_bytes(data[20:24], endian) == 1
                and int.from_bytes(data[offset:offset + 2], endian) == header_size and size >= header_size):
            return data[:header_size].hex()
    if data[:4] in (b"\xfe\xed\xfa\xce", b"\xce\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe"):
        endian = "little" if data[:4] in (b"\xce\xfa\xed\xfe", b"\xcf\xfa\xed\xfe") else "big"
        header_size = 32 if data[:4] in (b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe") else 28
        if int.from_bytes(data[12:16], endian) == 1 and size >= header_size:
            return data[:header_size].hex()
    if data[:4] in (b"BC\xc0\xde", b"\xde\xc0\x17\x0b"):
        return data[:20].hex()
    machines = (b"\x4c\x01", b"\x64\x86", b"\x64\xaa", b"\xc4\x01")
    if (data[:2] in machines and int.from_bytes(data[2:4], "little") > 0
            and data[16:18] == b"\0\0" and not int.from_bytes(data[18:20], "little") & 0x0002
            and size >= 20 + 40 * int.from_bytes(data[2:4], "little")):
        return data[:20].hex()
    if (len(data) >= 56 and data[:6] == b"\0\0\xff\xff\x02\0" and data[6:8] in machines
            and data[12:28] == bytes.fromhex("c7a1bad1eebaa94baf20faf66aa4dcb8")
            and int.from_bytes(data[44:48], "little") > 0
            and size >= 56 + 40 * int.from_bytes(data[44:48], "little")):
        return data[:56].hex()
    raise ValueError("unrecognized object format")


@contextmanager
def session_lock(path):
    """Lock an existing rustc lock; never create it or write lock-file bytes."""
    path = support.safe_path(path)
    if path in _POSIX_LOCK_STREAMS:
        raise ValueError("session lock already held")
    before = file_info(path)
    if before["links"] != 1:
        raise ValueError("session lock must not have hardlink aliases")
    if os.name == "nt":
        handle = kernel.CreateFileW(str(path), 0xC0000000, 7, None, 3, 0x00200000, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ValueError("cannot open native session lock")
        state = Overlapped()
        acquired = False
        try:
            if windows_info(handle) != before or not kernel.LockFileEx(handle, 3, 0, 0xFFFFFFFF, 0xFFFFFFFF, ctypes.byref(state)):
                raise ValueError("native session lock is busy or changed")
            acquired = True
            yield
        finally:
            if acquired:
                kernel.UnlockFileEx(handle, 0, 0xFFFFFFFF, 0xFFFFFFFF, ctypes.byref(state))
            kernel.CloseHandle(handle)
    else:
        import fcntl
        with path.open("r+b") as stream:
            if posix_info(os.fstat(stream.fileno())) != before:
                raise ValueError("session lock changed")
            operation = fcntl.flock if platform.system() == "Linux" else fcntl.lockf
            try:
                operation(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as exc:
                raise ValueError("native session lock is busy") from exc
            try:
                if platform.system() != "Linux":
                    # POSIX record locks are released when *any* descriptor for
                    # this inode is closed. Reuse this handle during validation.
                    _POSIX_LOCK_STREAMS[path] = stream
                yield
            finally:
                _POSIX_LOCK_STREAMS.pop(path, None)
                operation(stream.fileno(), fcntl.LOCK_UN)


def member_digest(path, unit=False):
    members = []
    for child in sorted(path.iterdir()):
        support.safe_path(child)
        if child.is_dir():
            if not unit:
                raise ValueError("unknown nested session layout")
            members.append([child.name, "directory", directory_identity(child)])
        elif child.is_file():
            members.append([child.name, "file", file_info(child)])
        else:
            raise ValueError("unknown directory member")
    return support.digest(members)


def incremental_roots(target):
    roots = []
    for first in sorted(target.iterdir()):
        if first.name.startswith(".") or not first.is_dir():
            continue
        support.safe_path(first)
        direct = first / "incremental"
        if direct.is_dir():
            roots.append(support.safe_path(direct))
        else:
            for profile in sorted(first.iterdir()):
                if profile.name.startswith(".") or not profile.is_dir():
                    continue
                indirect = profile / "incremental"
                if indirect.is_dir():
                    roots.append(support.safe_path(indirect))
    return roots


def relative_path(target, value):
    if not isinstance(value, str) or "\\" in value:
        raise ValueError("invalid relative plan path")
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or path.as_posix() != value:
        raise ValueError("plan path escaped target")
    result = support.safe_path(target / path)
    if not result.is_relative_to(target):
        raise ValueError("plan path escaped target")
    return result


def session_files(session, protections):
    paths = []
    if protected(session, protections):
        raise ValueError("explicitly protected session")
    binaries = set()
    versions = set()
    for path in sorted(session.iterdir()):
        support.safe_path(path)
        if not path.is_file():
            raise ValueError("unknown nested session layout")
        if protected(path, protections):
            raise ValueError("explicitly protected session member")
        if path.suffix in (".bin", ".o"):
            paths.append(path)
        elif path.name != "metadata.rmeta":
            raise ValueError("unknown session content")
        else:
            file_info(path)
    files = _payload_infos(paths)
    for path, info in files:
        if path.suffix == ".bin":
            binaries.add(path.name)
            versions.add(info["header"])
    if not {"dep-graph.bin", "work-products.bin"}.issubset(binaries) or len(versions) != 1:
        raise ValueError("incomplete or mixed incremental binary set")
    return files


def space_summary(files, sessions):
    objects = {}
    logical = allocated = 0
    for item in files:
        info = item["identity"]
        logical += info["size"]
        allocated += info["allocated"]
        key = tuple(info["id"])
        if key not in objects:
            objects[key] = [info["allocated"], info["links"], 0]
        if objects[key][:2] != [info["allocated"], info["links"]]:
            raise ValueError("inconsistent hardlink identity")
        objects[key][2] += 1
    if any(count > links for _, links, count in objects.values()):
        raise ValueError("invalid hardlink count")
    return {"files": len(files), "sessions": sessions, "logical_bytes": logical,
            "allocated_bytes": allocated, "unique_allocated_bytes": sum(x[0] for x in objects.values()),
            "reclaimable_bytes": sum(size for size, links, count in objects.values() if count == links)}


@_with_header_pool
def make_plan(target: Path, workspace: Path, packages: list[str], protection: Path) -> dict:
    target, workspace = support.safe_path(target), support.safe_path(workspace)
    target_id, workspace_id = directory_identity(target), directory_identity(workspace)
    scope = metadata_scope(workspace, packages)
    owner = ownership(target, workspace, scope)
    manifest, protections = protection_manifest(protection, target)
    plan = {"schema": 1, "kind": "incremental-eviction", "host": support.host(),
            "target": str(target), "workspace": str(workspace), "packages": sorted(packages),
            "target_identity": target_id, "workspace_identity": workspace_id,
            "metadata": scope, "ownership": owner, "protection": manifest,
            "incremental_roots": [], "units": [], "sessions": [], "files": [], "skipped": [],
            "leases": support.blockers(target)}
    for incremental in incremental_roots(target):
        plan["incremental_roots"].append({"path": incremental.relative_to(target).as_posix(), "identity": directory_identity(incremental)})
        for unit in sorted(incremental.iterdir()):
            match = UNIT.fullmatch(unit.name)
            if not match or match[1] not in scope["admitted_stems"] or not unit.is_dir():
                plan["skipped"].append({"path": unit.relative_to(target).as_posix(), "reason": "outside unambiguous selected workspace target scope"})
                continue
            try:
                support.safe_path(unit)
                members = sorted(unit.iterdir())
                if protected(unit, protections) or any(p.name.endswith("-working") for p in members):
                    raise ValueError("protected or working unit")
                snapshot = {"path": unit.relative_to(target).as_posix(), "identity": directory_identity(unit), "members": member_digest(unit, unit=True)}
                unit_index = len(plan["units"])
                plan["units"].append(snapshot)
            except (OSError, ValueError) as exc:
                plan["skipped"].append({"path": unit.relative_to(target).as_posix(), "reason": str(exc)})
                continue
            for session in members:
                if not session.is_dir():
                    continue
                try:
                    if not SESSION.fullmatch(session.name) or session.name.endswith("-working"):
                        raise ValueError("unknown or working session name")
                    lock = support.safe_path(unit / (session.name.rsplit("-", 1)[0] + ".lock"))
                    if protected(lock, protections):
                        raise ValueError("protected session lock")
                    with session_lock(lock):
                        before = member_digest(session)
                        payloads = session_files(session, protections)
                        if member_digest(session) != before:
                            raise ValueError("session changed while planning")
                        offset = len(plan["files"])
                        record = {"path": session.relative_to(target).as_posix(), "identity": directory_identity(session),
                                  "unit": unit_index, "lock": lock.relative_to(target).as_posix(), "lock_identity": file_info(lock),
                                  "members": before, "file_start": offset, "file_count": len(payloads)}
                        plan["files"].extend({"path": p.relative_to(target).as_posix(), "identity": info} for p, info in payloads)
                        plan["sessions"].append(record)
                except (OSError, ValueError) as exc:
                    plan["skipped"].append({"path": session.relative_to(target).as_posix(), "reason": str(exc)})
            if member_digest(unit, unit=True) != snapshot["members"]:
                raise ValueError("unit changed while planning")
    plan["summary"] = space_summary(plan["files"], len(plan["sessions"]))
    return plan


@_with_header_pool
def apply_plan(plan) -> int:
    if (not isinstance(plan, dict) or set(plan) != PLAN_KEYS or plan["schema"] != 1
            or plan["kind"] != "incremental-eviction" or plan["host"] != support.host()):
        raise ValueError("invalid or foreign incremental eviction plan")
    if (any(not isinstance(plan[k], list) for k in ("units", "sessions", "files", "incremental_roots", "skipped", "leases"))
            or any(not isinstance(plan[k], str) or not Path(plan[k]).is_absolute() for k in ("target", "workspace"))
            or not isinstance(plan["protection"], dict) or set(plan["protection"]) != {"path", "sha256", "bindings"}):
        raise ValueError("invalid incremental eviction plan structure")
    for item in plan["incremental_roots"]:
        if not isinstance(item, dict) or set(item) != {"path", "identity"}:
            raise ValueError("invalid incremental root record")
    for item in plan["units"]:
        if not isinstance(item, dict) or set(item) != {"path", "identity", "members"}:
            raise ValueError("invalid incremental unit record")
    for item in plan["sessions"]:
        if (not isinstance(item, dict) or set(item) != {"path", "identity", "unit", "lock", "lock_identity", "members", "file_start", "file_count"}
                or type(item["unit"]) is not int or not 0 <= item["unit"] < len(plan["units"])
                or type(item["file_start"]) is not int or item["file_start"] < 0):
            raise ValueError("invalid incremental session record")
    target, workspace = support.safe_path(plan["target"]), support.safe_path(plan["workspace"])
    with support.guard(target), ExitStack() as locks:
        if directory_identity(target) != plan["target_identity"] or directory_identity(workspace) != plan["workspace_identity"]:
            raise ValueError("target or workspace directory was replaced")
        if support.blockers(target):
            raise ValueError("live or unresolved leases block eviction")
        scope = metadata_scope(workspace, plan["packages"])
        if scope != plan["metadata"] or ownership(target, workspace, scope) != plan["ownership"]:
            raise ValueError("workspace scope or target ownership changed")
        manifest, protections = protection_manifest(plan["protection"]["path"], target)
        if manifest != plan["protection"]:
            raise ValueError("protection manifest or evidence changed")
        actual_roots = incremental_roots(target)
        expected_roots = [{"path": p.relative_to(target).as_posix(), "identity": directory_identity(p)} for p in actual_roots]
        if expected_roots != plan["incremental_roots"]:
            raise ValueError("incremental root set changed")
        for root in plan["incremental_roots"]:
            if directory_identity(relative_path(target, root["path"])) != root["identity"]:
                raise ValueError("incremental root replaced")
        seen = set()
        for unit in plan["units"]:
            path = relative_path(target, unit["path"])
            match = UNIT.fullmatch(path.name)
            if (not match or match[1] not in scope["admitted_stems"] or path.parent not in actual_roots
                    or directory_identity(path) != unit["identity"] or member_digest(path, unit=True) != unit["members"]
                    or any(p.name.endswith("-working") for p in path.iterdir())):
                raise ValueError("unit scope, membership or working state changed")
        offset = 0
        for session in plan["sessions"]:
            path = relative_path(target, session["path"])
            unit = plan["units"][session["unit"]]
            if (not SESSION.fullmatch(path.name) or path.name.endswith("-working")
                    or path.parent != relative_path(target, unit["path"]) or path in seen
                    or directory_identity(path) != session["identity"]
                    or session["file_start"] != offset or type(session["file_count"]) is not int or session["file_count"] <= 0):
                raise ValueError("invalid or replaced session selection")
            seen.add(path)
            lock = relative_path(target, session["lock"])
            if lock != path.parent / (path.name.rsplit("-", 1)[0] + ".lock"):
                raise ValueError("session lock path mismatch")
            locks.enter_context(session_lock(lock))
            if file_info(lock) != session["lock_identity"] or member_digest(path) != session["members"]:
                raise ValueError("session lock or membership changed")
            payloads = session_files(path, protections)
            actual = [{"path": p.relative_to(target).as_posix(), "identity": info} for p, info in payloads]
            expected = plan["files"][offset:offset + session["file_count"]]
            if actual != expected:
                raise ValueError("complete payload identities or membership changed")
            offset += session["file_count"]
        if offset != len(plan["files"]) or space_summary(plan["files"], len(plan["sessions"])) != plan["summary"]:
            raise ValueError("invalid flat payload set or capacity summary")
        # Recheck all selected unit/session membership after every lock is held.
        # Unmanaged writers remain covered by the caller's quiescence assertion.
        for unit in plan["units"]:
            if member_digest(relative_path(target, unit["path"]), unit=True) != unit["members"]:
                raise ValueError("unit changed during preflight")
        for session in plan["sessions"]:
            if member_digest(relative_path(target, session["path"])) != session["members"]:
                raise ValueError("session changed during preflight")
        final_scope = metadata_scope(workspace, plan["packages"])
        if (final_scope != plan["metadata"] or directory_identity(target) != plan["target_identity"]
                or directory_identity(workspace) != plan["workspace_identity"]
                or ownership(target, workspace, final_scope) != plan["ownership"]):
            raise ValueError("workspace scope, identity or target ownership changed during preflight")
        if protection_manifest(plan["protection"]["path"], target)[0] != plan["protection"]:
            raise ValueError("protection changed during preflight")
        for item in plan["files"]:
            relative_path(target, item["path"]).unlink()
        return len(plan["files"])

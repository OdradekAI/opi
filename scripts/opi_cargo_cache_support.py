"""Conservative observed-file maintenance; no directory is a deletion candidate."""
from __future__ import annotations

from contextlib import contextmanager
import ctypes
import hashlib
import hmac
import json
import os
from pathlib import Path
import platform
import stat
import subprocess
import time
import uuid

INVENTORY = ".opi-cargo-inventory.json"
LEASE_PREFIX = ".opi-cargo-lease-"


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def digest(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def safe_path(value):
    """Check every lexical ancestor before resolving, including Windows junctions."""
    path = Path(os.path.abspath(Path(value).expanduser()))
    for part in [*reversed(path.parents), path]:
        try:
            info = part.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"linked path refused: {part}")
    return path


def atomic(path, value):
    safe_path(path)
    temp = path.with_name(path.name + ".tmp-" + uuid.uuid4().hex)
    try:
        with temp.open("x", encoding="utf-8") as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write("\n")
        os.replace(temp, path)
    finally:
        temp.unlink(missing_ok=True)


@contextmanager
def guard(target):
    target = safe_path(target)
    target.mkdir(parents=True, exist_ok=True)
    # mkdir is one common filesystem primitive across Windows and WSL; flock
    # and Windows byte-range locks do not coordinate with one another.
    path = safe_path(target / ".opi-cargo-guard-v1")
    token = uuid.uuid4().hex
    deadline = time.monotonic() + 30
    while True:
        try:
            safe_path(path).mkdir()
            break
        except FileExistsError:
            if time.monotonic() >= deadline:
                raise ValueError("cache coordination guard is busy or stale; inspect writers before manually removing it")
            time.sleep(0.05)
    owner = path / "owner.json"
    try:
        atomic(owner, {"token": token, "host": host(), "pid": os.getpid()})
        yield
    finally:
        safe_path(owner)
        value = json.loads(owner.read_text(encoding="utf-8"))
        if value.get("token") != token:
            raise ValueError("coordination guard ownership changed")
        owner.unlink()
        path.rmdir()


def host():
    # WSL distro and operating-system identity distinguish namespaces with equal PIDs.
    return digest([platform.node(), platform.system(), platform.release(),
                   os.environ.get("WSL_DISTRO_NAME"), os.environ.get("USERNAME", os.environ.get("USER"))])


def process_identity(pid):
    if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 0:
        return None
    if os.name == "nt":
        from ctypes import wintypes
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4)]
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        handle = kernel.OpenProcess(0x1000, False, pid)
        if not handle:
            # Access denied is unresolved, not proof that the process is dead.
            return "unknown" if ctypes.get_last_error() == 5 else None
        try:
            times = [wintypes.FILETIME() for _ in range(4)]
            if not kernel.GetProcessTimes(handle, *(ctypes.byref(item) for item in times)):
                return "unknown"
            return str(times[0].dwHighDateTime << 32 | times[0].dwLowDateTime)
        finally:
            kernel.CloseHandle(handle)
    if platform.system() != "Linux":
        try:
            result = subprocess.run(["ps", "-p", str(pid), "-o", "lstart="], capture_output=True, text=True,
                                    env={**os.environ, "LC_ALL": "C"})
            value = result.stdout.strip()
            if result.returncode == 0 and value:
                return value
            # ps exit 1 with no output means no matching process. Tool failures
            # and permission errors remain unresolved, so they block deletion.
            if result.returncode == 1 and not value and not result.stderr.strip():
                return None
            return "unknown"
        except OSError:
            return "unknown"
    try:
        content = Path(f"/proc/{pid}/stat").read_text()
        start = content[content.rindex(")") + 2:].split()[19]
        boot = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
        return boot + ":" + start
    except FileNotFoundError:
        return None
    except (OSError, ValueError, IndexError):
        return "unknown"


def lease(target, pid, start, writer=False):
    if pid <= 0:
        raise ValueError("lease PID must be positive")
    target = safe_path(target)
    with guard(target):
        path = safe_path(target / f"{LEASE_PREFIX}{host()[:16]}-{pid}.json")
        if start:
            identity = process_identity(pid)
            if identity in (None, "unknown"):
                raise ValueError("cannot verify lease process identity")
            value = {"schema": 2, "host": host(), "pid": pid, "identity": identity}
            if writer:
                value["writer"] = True
            atomic(path, value)
        else:
            path.unlink(missing_ok=True)


def blockers(target):
    blocked = []
    for path in target.glob(LEASE_PREFIX + "*.json"):
        try:
            safe_path(path)
            value = json.loads(path.read_text(encoding="utf-8"))
            if (not isinstance(value, dict) or set(value) not in ({"schema", "host", "pid", "identity"}, {"schema", "host", "pid", "identity", "writer"})
                    or value.get("schema") != 2 or value.get("host") != host()
                    or type(value.get("pid")) is not int or value["pid"] <= 0
                    or not isinstance(value.get("identity"), str) or not value["identity"]):
                blocked.append(path.name)
                continue
            identity = process_identity(value.get("pid"))
            if identity == "unknown" or identity == value["identity"] or value.get("writer") is not None:
                blocked.append(path.name)
        except (OSError, ValueError, TypeError):
            blocked.append(path.name)
    return blocked


def key():
    # The key is outside targets: copying/forging a target-local receipt cannot
    # authorize deletion. This is integrity protection, not a hostile-user sandbox.
    if os.environ.get("OPI_CARGO_CACHE_ROOT"):
        root = Path(os.environ["OPI_CARGO_CACHE_ROOT"])
    elif os.name == "nt" and os.environ.get("LOCALAPPDATA"):
        root = Path(os.environ["LOCALAPPDATA"]) / "opi" / "cargo-targets"
    else:
        root = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "opi" / "cargo-targets"
    root = safe_path(root)
    root.mkdir(parents=True, exist_ok=True)
    path = safe_path(root / ".opi-cargo-observer-key")
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError:
        pass
    else:
        with os.fdopen(fd, "wb") as stream:
            stream.write(os.urandom(32))
    value = path.read_bytes()
    if len(value) != 32 or path.stat().st_nlink != 1:
        raise ValueError("invalid observer ownership key")
    return value


def empty(target):
    return {"schema": 1, "host": host(), "target": str(target), "scopes": {},
            "candidates": {}, "protected": {}, "generation": 0}


def load(target):
    path = safe_path(target / INVENTORY)
    if not path.exists():
        return empty(target)
    envelope = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(envelope, dict) or set(envelope) != {"payload", "signature"}:
        raise ValueError("invalid inventory envelope")
    value = envelope["payload"]
    expected = hmac.new(key(), encoded(value), hashlib.sha256).hexdigest()
    if not isinstance(envelope["signature"], str) or not hmac.compare_digest(expected, envelope["signature"]):
        raise ValueError("inventory integrity check failed")
    if (not isinstance(value, dict) or value.get("schema") != 1 or value.get("host") != host()
            or value.get("target") != str(target)
            or any(not isinstance(value.get(k), dict) for k in ("scopes", "candidates", "protected"))
            or type(value.get("generation")) is not int):
        raise ValueError("invalid or foreign inventory ownership")
    return value


def save(target, value):
    value["generation"] += 1
    atomic(target / INVENTORY, {"payload": value, "signature": hmac.new(key(), encoded(value), hashlib.sha256).hexdigest()})


def output_relative(relative):
    if not isinstance(relative, str) or "\\" in relative:
        raise ValueError("invalid output path")
    path = Path(relative)
    if path.is_absolute() or any(p in ("..", ".") for p in path.parts):
        raise ValueError("output escaped target")
    # Only Cargo's file outputs directly in deps are admitted. Public binaries,
    # build-script data, incremental, fingerprints and frozen copies stay unknown.
    if len(path.parts) < 3 or path.parts[-2] != "deps" or any(p.startswith(".") for p in path.parts):
        raise ValueError("output is outside admitted Cargo deps files")
    return path


def output_path(target, relative):
    result = safe_path(target / output_relative(relative))
    if not result.is_relative_to(target):
        raise ValueError("output escaped target")
    return result


def identity(path, content=True):
    before = path.stat()
    if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
        raise ValueError("output is not an exclusive regular file")
    stamp = lambda s: [s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns]
    if not content:
        return {"stat": stamp(before)}
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    after = path.stat()
    if stamp(before) != stamp(after):
        raise ValueError("output changed while reading")
    return {"stat": stamp(after), "sha256": hasher.hexdigest()}


def observe(target, message):
    """Return observed files and a sticky gap; unknown identities are pins only."""
    if message.get("reason") != "compiler-artifact":
        return None
    package = message.get("package_id")
    if isinstance(package, str) and (package.startswith("path+") or package.startswith("file:")):
        package = package.split("#", 1)[0]
    artifact_key = digest([package, *[message.get(k) for k in ("target", "profile", "features")]])
    files = {}
    filenames = message.get("filenames")
    complete = isinstance(filenames, list)
    for filename in filenames if complete else []:
        try:
            absolute = Path(os.path.abspath(Path(filename).expanduser()))
        except (OSError, ValueError, TypeError):
            complete = False
            continue
        try:
            relative = absolute.relative_to(target).as_posix()
            output_relative(relative)
        except ValueError:
            # Public binaries and other out-of-scope outputs are not evidence
            # gaps. Check filesystem safety only after lexical admission.
            continue
        try:
            files[relative] = identity(output_path(target, relative), content=False)
        except (OSError, ValueError, TypeError):
            complete = False
            # Cargo still reported this path. Retain it even if it was already
            # a candidate; missing identity cannot establish supersession.
            files.setdefault(relative, None)
    return artifact_key, files, None if complete else "cannot inspect all admitted Cargo outputs"


def record(target, scope, revision, messages, build_success, command_exit):
    """Commit only positively observed artifact keys; missing keys stay retained."""
    observed = {}
    gap = None
    for message in messages:
        item = observe(target, message)
        if item:
            observed.setdefault(item[0], {}).update(item[1])
            gap = gap or item[2]
    record_observed(target, scope, revision, observed, build_success, command_exit, gap=gap)


def record_observed(target, scope, revision, observed, build_success, command_exit, gap=None):
    with guard(target):
        state = load(target)
        state["last_run"] = {"scope": scope, "revision": revision, "build_success": build_success, "command_exit": command_exit, "observation_gap": gap}
        if build_success is True and command_exit == 0 and gap is None:
            previous = state["scopes"].setdefault(scope, {})
            for artifact, files in observed.items():
                for path, stamp in previous.get(artifact, {}).items():
                    if path not in files:
                        state["candidates"][path] = stamp
                previous[artifact] = files
        else:
            failed = state["protected"].setdefault(scope, {})
            for artifact, files in observed.items():
                failed.setdefault(artifact, {}).update(files)
        # A path referenced by any scope or a failed observation is never eligible.
        protected = protected_paths(state)
        for artifacts in state["scopes"].values():
            for files in artifacts.values():
                protected.update(files)
        for path in protected:
            state["candidates"].pop(path, None)
        save(target, state)


def protected_paths(state):
    return {path for artifacts in state["protected"].values() for files in artifacts.values() for path in files}


def make_plan(target):
    target = safe_path(target)
    with guard(target):
        state = load(target)
        files = []
        for relative, stamp in sorted(state["candidates"].items()):
            path = output_path(target, relative)
            try:
                current = identity(path, content=False)
            except FileNotFoundError:
                continue
            if current == stamp:
                files.append({"path": relative, "identity": identity(path)})
        return {"schema": 1, "host": host(), "target": str(target), "inventory": digest(state), "files": files}


def apply_plan(plan):
    if (not isinstance(plan, dict) or set(plan) != {"schema", "host", "target", "inventory", "files"}
            or plan["schema"] != 1 or plan["host"] != host() or not isinstance(plan["files"], list)):
        raise ValueError("invalid or foreign plan")
    target = safe_path(plan["target"])
    with guard(target):
        state = load(target)
        if digest(state) != plan["inventory"]:
            raise ValueError("inventory changed since plan")
        if blockers(target):
            raise ValueError("live or unresolved Cargo leases block maintenance")
        paths = []
        seen = set()
        for item in plan["files"]:
            if not isinstance(item, dict) or set(item) != {"path", "identity"}:
                raise ValueError("invalid plan file")
            path = output_path(target, item["path"])
            if (not isinstance(item["identity"], dict) or set(item["identity"]) != {"stat", "sha256"}
                    or item["path"] in seen or state["candidates"].get(item["path"]) != {"stat": item["identity"]["stat"]}):
                raise ValueError("file is not a recorded candidate")
            seen.add(item["path"])
            if identity(path) != item["identity"]:
                raise ValueError("candidate changed since observation")
            paths.append((path, item))
        # Validate the entire batch before the first unlink. Cooperating writers
        # cannot acquire a lease until all deletion and state updates finish.
        for path, item in paths:
            if identity(output_path(target, item["path"])) != item["identity"]:
                raise ValueError("candidate changed before deletion")
            path.unlink()
            del state["candidates"][item["path"]]
        save(target, state)
        return len(paths)

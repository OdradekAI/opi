"""Bounded subprocess execution and exclusive operator artifacts."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import threading
from datetime import datetime, timezone


class OperatorError(Exception):
    """A refusal with a credential-free diagnostic."""


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                     ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def write_bytes(path, data, mode=0o600):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    except FileExistsError as exc:
        raise OperatorError("artifact already exists; choose a fresh destination") from exc
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)


def write_json(path, value):
    write_bytes(path, (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False,
                                allow_nan=False) + "\n").encode())


def pin(path):
    return {"path": str(Path(path).resolve()), "sha256": sha(path)}


def check_pin(value):
    path = Path(value["path"])
    if not path.is_file() or sha(path) != value["sha256"]:
        raise OperatorError("frozen artifact identity drift")
    return path


def clean_environment():
    # Build/preparation may inherit operational tools, never provider credentials.
    return {key: value for key, value in os.environ.items()
            if not any(part in key.upper() for part in ("API_KEY", "API_TOKEN", "AUTH_TOKEN", "SECRET"))
            and not key.upper().endswith(("_KEY", "_TOKEN", "_PASSWORD", "_CREDENTIAL"))}


class BatchDeadline:
    """One monotonic interval, shared by every attempt and native grader."""
    def __init__(self, seconds, clock=time.monotonic):
        self.clock = clock
        self.start = clock()
        self.end = self.start + seconds

    def remaining(self):
        return max(0, self.end - self.clock())


class Commands:
    def __init__(self, root, secrets=(), excluded_env=(), capture_cap=64 * 1024 * 1024):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.number = len(list(self.root.glob("*.json")))
        self.secrets = tuple(value for value in secrets if value)
        self.excluded_env = set(excluded_env)
        self.capture_cap = capture_cap

    def run(self, argv, cwd, *, env=None, deadline=None, require=True, allow_credentials=False):
        argv = [str(value) for value in argv]
        if any(secret in item for secret in self.secrets for item in argv):
            raise OperatorError("credential in command argument refused")
        if deadline is not None and deadline.remaining() <= 0:
            raise OperatorError("batch deadline exhausted before dispatch")
        self.number += 1
        stem = f"{self.number:04d}-{Path(argv[0]).stem}"
        started = utc_now()
        wall_start = time.monotonic()
        timed_out = False
        interrupted = False
        environment = env if env is not None else {key: value for key, value in clean_environment().items() if key not in self.excluded_env}
        if not allow_credentials:
            environment = {key: value for key, value in environment.items() if key not in self.excluded_env}
        process = subprocess.Popen(argv, cwd=cwd, env=environment,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   )
        overflow = threading.Event()
        streams = [bytearray(), bytearray()]
        def consume(stream, buffer):
            while True:
                chunk = stream.read(65536)
                if not chunk:
                    break
                room = self.capture_cap - len(buffer)
                buffer.extend(chunk[:max(0, room)])
                if len(chunk) > room and not overflow.is_set():
                    overflow.set()
                    try:
                        process.send_signal(signal.SIGINT)
                    except (ProcessLookupError, ValueError):
                        pass
            stream.close()
        readers = [threading.Thread(target=consume, args=(stream, buffer), daemon=True)
                   for stream, buffer in zip((process.stdout, process.stderr), streams)]
        for reader in readers:
            reader.start()
        try:
            while process.poll() is None:
                if overflow.is_set():
                    process.wait(timeout=30)
                    break
                if deadline is not None and deadline.remaining() <= 0:
                    raise subprocess.TimeoutExpired(argv, 0)
                try:
                    process.wait(timeout=min(0.2, deadline.remaining()) if deadline else 0.2)
                except subprocess.TimeoutExpired:
                    continue
        except (subprocess.TimeoutExpired, KeyboardInterrupt) as interruption:
            timed_out = isinstance(interruption, subprocess.TimeoutExpired)
            interrupted = isinstance(interruption, KeyboardInterrupt)
            process.send_signal(signal.SIGINT)
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired as exc:
                write_json(self.root / f"{stem}.json", {
                    "argv": argv, "cwd": str(cwd), "started_at": started,
                    "deadline_expired": timed_out, "operator_interrupted": interrupted, "cleanup": "SIGINT-sent-completion-unknown",
                    "pid": process.pid})
                raise OperatorError("native SIGINT cleanup did not settle; stop and inspect owned resources") from exc
        for reader in readers:
            reader.join(timeout=5)
        stream_unknown = any(reader.is_alive() for reader in readers)
        stdout, stderr = (bytes(buffer).decode("utf-8", errors="replace") for buffer in streams)
        leak = any(secret in stdout or secret in stderr for secret in self.secrets)
        safe_out, safe_err = stdout, stderr
        for secret in self.secrets:
            safe_out = safe_out.replace(secret, "[credential-redacted]")
            safe_err = safe_err.replace(secret, "[credential-redacted]")
        write_bytes(self.root / f"{stem}.stdout", safe_out.encode())
        write_bytes(self.root / f"{stem}.stderr", safe_err.encode())
        write_json(self.root / f"{stem}.json", {
            "argv": argv, "cwd": str(cwd), "started_at": started, "finished_at": utc_now(),
            "elapsed_seconds": time.monotonic() - wall_start, "exit_code": process.returncode,
            "deadline_expired": timed_out, "privacy_failure": leak, "capture_cap_bytes": self.capture_cap,
            "capture_overflow": overflow.is_set(), "operator_interrupted": interrupted,
            "stream_cleanup": "unknown" if stream_unknown else "closed"})
        if leak:
            raise OperatorError("credential leak in subprocess output; affected execution stopped")
        if overflow.is_set():
            raise OperatorError("bounded subprocess capture overflow; affected execution stopped")
        if stream_unknown:
            raise OperatorError("subprocess stream did not close; inspect owned process descendants")
        if interrupted and require:
            raise OperatorError("operator interrupted; native SIGINT cleanup settled")
        if require and process.returncode != 0:
            raise OperatorError(f"{Path(argv[0]).name} {argv[1] if len(argv) > 1 else ''} failed; inspect command records")
        result = subprocess.CompletedProcess(argv, process.returncode, stdout, stderr)
        result.operator_interrupted = interrupted
        result.deadline_expired = timed_out
        return result

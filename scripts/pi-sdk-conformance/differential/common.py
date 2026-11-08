from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]


def file_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def runtime_environment() -> dict[str, str]:
    # Controlled drivers never inherit provider credentials or proxy settings.
    names = ("PATH", "SystemRoot", "WINDIR", "TEMP", "TMP", "TMPDIR")
    return {**{name: os.environ[name] for name in names if name in os.environ}, "TZ": "UTC"}


def run_json(command: list[str], repo: Path, timeout: int = 30) -> dict:
    completed = subprocess.run(
        command, cwd=repo, check=True, capture_output=True,
        env=runtime_environment(), timeout=timeout,
    )
    # Reject noise/multiple records rather than silently taking the last line.
    trace = json.loads(completed.stdout)
    return {
        "command": command,
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8"),
        "stderr": completed.stderr.decode("utf-8"),
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "trace": trace,
    }


def build_rust_driver(repo: Path = REPO_ROOT) -> dict:
    cache_script = repo / "scripts/opi-cargo-cache.py"
    configured = os.environ.get("CARGO_TARGET_DIR")
    target = configured or subprocess.check_output(
        [sys.executable, str(cache_script), "resolve", "--workspace", str(repo)],
        cwd=repo, text=True,
    ).strip()
    status_command = [sys.executable, str(cache_script), "status", "--target", target, "--quick", "--json"]
    status = subprocess.run(status_command, cwd=repo, check=True, capture_output=True, text=True)
    command = [
        sys.executable, str(cache_script), "run", "--target", target, "--",
        "build", "-p", "opi-ai", "--example", "lifecycle_differential", "--offline", "-j4",
    ]
    # The cache wrapper owns writer leases. Do not kill it on a short driver
    # timeout: only execution of the already-built probe has a watchdog.
    completed = subprocess.run(command, cwd=repo, capture_output=True, text=True, encoding="utf-8")
    if completed.returncode:
        raise subprocess.CalledProcessError(completed.returncode, command, completed.stdout, completed.stderr)
    artifacts = [json.loads(line) for line in completed.stdout.splitlines() if line.startswith("{")]
    executables = [
        row["executable"] for row in artifacts
        if row.get("reason") == "compiler-artifact"
        and row.get("target", {}).get("name") == "lifecycle_differential"
        and row.get("executable")
    ]
    if len(executables) != 1:
        raise ValueError(f"expected one Cargo-reported driver executable, got {executables}")
    executable = Path(executables[0])
    return {
        "command": command,
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "cache_status": {"command": status_command, "stdout": status.stdout},
        "executable": str(executable),
        "executable_sha256": file_hash(executable),
    }


def run_rust_trace(scenario: str, build: dict, repo: Path = REPO_ROOT) -> dict:
    executable = Path(build["executable"])
    if file_hash(executable) != build["executable_sha256"]:
        raise ValueError("Rust driver changed after its recorded build")
    return run_json([str(executable), scenario], repo)

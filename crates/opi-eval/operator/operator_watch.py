"""Read-only supervision of one running evaluation.

`watch` observes the operator's own records — never evidence bytes — so a
caller can wait event-driven instead of sleeping a fixed interval: it wakes
on terminal states (result published, operator crashed or gone), on an
abnormal stall (no settled-trial progress while no task container runs), on
the frozen batch budget expiring, or on its own maximum wait. Evaluation
authority stays entirely with the executing operator and the Companion.

Exit codes (stable contract for callers and scripts):
  0 completed            result-handoff.json exists for the evaluation
  1 not-running          no live execution and no result (never started or
                         already consumed by maintenance)
  2 operator-crashed     the evaluation lease names a dead operator process
  3 stall-suspected      no receipt progress for --stall seconds while no
                         Docker container runs (long single trials keep
                         containers alive and never trip this)
  4 budget-expired       a family batch clock reached its frozen budget
  5 watch-error          unusable evaluation records
124 max-reached          still healthy after --max seconds
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import time

from operator_evidence import read_json
from operator_flow import external
from operator_process import OperatorError, utc_now

WATCH_SCHEMA = "opi-eval-watch-status/1"


def _settled(family: Path) -> int:
    attempts = family / "runs"
    if not attempts.is_dir():
        return 0
    return sum(1 for _ in attempts.glob("attempt-*/trials/*/receipt.json"))


def _batch_deadline(family: Path):
    """The family's frozen batch clock, or None when dispatch has not started."""
    batch = family / "batch.json"
    if not batch.is_file():
        return None
    try:
        document = read_json(batch)
    except (OperatorError, OSError, ValueError):
        return None
    started, seconds = document.get("started_at"), document.get("seconds")
    if not isinstance(started, str) or not isinstance(seconds, (int, float)):
        return None
    try:
        from datetime import datetime, timezone

        begin = datetime.fromisoformat(started)
        if begin.tzinfo is None:
            return None
        return begin.timestamp() + float(seconds)
    except ValueError:
        return None


def _containers() -> int | None:
    """Running task-container count, or None when the engine is unobservable."""
    try:
        result = subprocess.run(["docker", "ps", "--quiet"], capture_output=True, text=True, timeout=15)
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    return len([line for line in result.stdout.splitlines() if line.strip()])


def _lease_pid(store_leases: Path, identity: str) -> int | None:
    lease = store_leases / (identity + ".json")
    if not lease.is_file():
        return None
    try:
        return int(read_json(lease).get("pid"))
    except (OperatorError, OSError, ValueError, TypeError):
        return None


def _alive(pid: int | None) -> bool:
    if not isinstance(pid, int) or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def evaluate_state(evaluation: Path, leases: Path, containers: int | None) -> tuple[int, str, dict]:
    """One supervision snapshot: (exit code, reason, machine status)."""
    record = evaluation / "record.json"
    if not record.is_file():
        return 5, "watch-error", {"reason_detail": "evaluation record is missing"}
    try:
        identity = read_json(record).get("id") or evaluation.name
    except (OperatorError, OSError, ValueError):
        return 5, "watch-error", {"reason_detail": "evaluation record is unreadable"}
    settled = {family.name: _settled(family) for family in _family_roots(evaluation)}
    status = {
        "schema": WATCH_SCHEMA,
        "evaluation": str(evaluation),
        "id": identity,
        "families": settled,
        "settled": sum(settled.values()),
        "containers": containers,
        "observed_at": utc_now(),
    }
    if (evaluation / "result-handoff.json").is_file():
        return 0, "completed", status
    pid = _lease_pid(leases, identity)
    if pid is None:
        return 1, "not-running", status
    if not _alive(pid):
        return 2, "operator-crashed", status
    now = time.time()
    for family in _family_roots(evaluation):
        deadline = _batch_deadline(family)
        if deadline is not None and now >= deadline:
            status["budget_family"] = family.name
            return 4, "budget-expired", status
    return 124, "running", status


def _family_roots(evaluation: Path):
    families = evaluation / "families"
    if not families.is_dir():
        return []
    return sorted(path for path in families.iterdir() if path.is_dir())


def watch(evaluation: Path, workspace: Path, *, maximum: float, poll: float, stall: float,
          container_counter=_containers) -> dict:
    """Poll until a terminal state, stall, budget expiry or the maximum wait.

    Returns the final status document (also describing the wake reason).
    """
    evaluation = external(evaluation)
    workspace = external(workspace)
    if maximum <= 0 or poll <= 0 or stall < poll:
        raise OperatorError("watch bounds must be positive and stall must cover one poll")
    started = time.monotonic()
    last_settled, last_change = None, started
    while True:
        containers = container_counter()
        code, reason, status = evaluate_state(evaluation, workspace / "leases", containers)
        if last_settled is not None and status["settled"] != last_settled:
            last_change = time.monotonic()
        last_settled = status["settled"]
        idle = time.monotonic() - last_change
        status.update({"reason": reason, "exit_code": code,
                       "watch_elapsed_seconds": round(time.monotonic() - started, 3),
                       "stall_seconds": round(idle, 3)})
        terminal = code != 124
        stalled = (reason == "running" and containers == 0 and idle >= stall)
        if terminal or stalled:
            if stalled:
                status.update({"reason": "stall-suspected", "exit_code": 3})
            return status
        if time.monotonic() - started >= maximum:
            status.update({"reason": "max-reached", "exit_code": 124})
            return status
        time.sleep(poll)

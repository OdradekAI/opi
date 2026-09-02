#!/usr/bin/env python3
"""Static verifier for the committed Phase 18 lock-materialization contract.

Checks that the manually authorized materialization workflow, the producer
scripts it invokes, and every GitHub action it uses are pinned by the
committed static external lock, and that the workflow declares only the
`workflow_dispatch` trigger with read-only contents permission. This is a
static contract check only: it never executes the materialization workflow
and never touches the network.

Exit codes: 0 accepted, 2 rejected (including missing context).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

STATIC_LOCK_SCHEMA = "phase18-external-lock/static/1"
LOCK_ID = "phase18-linux-x86_64"
PLATFORM = "linux-x86_64"
DEFAULT_STATIC_LOCK = "crates/opi-eval/external-locks/static/linux-x86_64.json"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
USES_RE = re.compile(r"^\s*(?:-\s+)?uses:\s*(\S+)", re.MULTILINE)
SCRIPT_REF_RE = re.compile(r"(?:[A-Za-z0-9_.-]+/)*scripts/[A-Za-z0-9_.\-/]+")


class Rejection(Exception):
    """One fail-closed contract violation."""


def lf_sha256(data: bytes) -> str:
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def load_static_lock(path: Path) -> dict:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise Rejection(f"cannot read static lock {path}: {error}") from error
    try:
        lock = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Rejection(f"static lock {path} is not valid UTF-8 JSON: {error}") from error
    schema = lock.get("schema")
    if schema != STATIC_LOCK_SCHEMA:
        raise Rejection(f"unsupported static lock schema: {schema!r}")
    if lock.get("lock_id") != LOCK_ID or lock.get("platform") != PLATFORM:
        raise Rejection(
            f"static lock identity mismatch: {lock.get('lock_id')!r}/{lock.get('platform')!r}"
        )
    return lock


def top_level_block(text: str, key: str) -> list[str]:
    """Direct child lines of a column-0 `key:` block in the workflow grammar."""
    lines = text.splitlines()
    children: list[str] = []
    index = 0
    while index < len(lines and lines) and lines[index].rstrip() != f"{key}:":
        index += 1
    if index >= len(lines):
        return children
    index += 1
    child_indent: int | None = None
    for line in lines[index:]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip())
        if indent == 0:
            break
        if child_indent is None or indent <= child_indent:
            child_indent = indent
            children.append(line.strip())
    return children


def parse_triggers(text: str) -> set[str]:
    triggers: set[str] = set()
    for child in top_level_block(text, "on"):
        if child.startswith("- "):
            triggers.add(child[2:].strip())
        elif ":" in child:
            triggers.add(child.split(":", 1)[0].strip())
    return triggers


def verify(
    root: Path, workflow_arg: Path, script_arg: Path, static_lock_path: Path
) -> tuple[int, str]:
    lock = load_static_lock(static_lock_path)
    authority = lock.get("authority")
    if not isinstance(authority, dict):
        raise Rejection("static lock has no authority object")
    workflow_pin = authority.get("workflow")
    producers = authority.get("producers")
    actions = authority.get("actions")
    if not isinstance(workflow_pin, dict) or not isinstance(producers, list) or not isinstance(actions, list):
        raise Rejection("static lock authority is malformed")

    def relative(path: Path) -> str:
        try:
            return path.resolve().relative_to(root.resolve()).as_posix()
        except ValueError as error:
            raise Rejection(f"path {path} is outside the repository root {root}") from error

    workflow_rel = relative(workflow_arg)
    if workflow_rel != workflow_pin.get("path"):
        raise Rejection(
            f"workflow path mismatch: verifier ran against {workflow_rel}, "
            f"static lock pins {workflow_pin.get('path')!r}"
        )
    workflow_bytes = workflow_arg.read_bytes()
    workflow_text = workflow_bytes.decode("utf-8")
    recorded_workflow_sha = workflow_pin.get("sha256")
    actual_workflow_sha = lf_sha256(workflow_bytes)
    if actual_workflow_sha != recorded_workflow_sha:
        raise Rejection(
            f"workflow drift: {workflow_rel} hashes to {actual_workflow_sha}, "
            f"static lock pins {recorded_workflow_sha}"
        )

    producer_paths: dict[str, str] = {}
    for producer in producers:
        path = producer.get("path")
        recorded = producer.get("sha256")
        role = producer.get("role")
        if not isinstance(path, str) or not isinstance(recorded, str) or not isinstance(role, str):
            raise Rejection(f"malformed producer pin: {producer!r}")
        producer_file = root / path
        if not producer_file.is_file():
            raise Rejection(f"pinned producer file is missing: {path}")
        actual = lf_sha256(producer_file.read_bytes())
        if actual != recorded:
            raise Rejection(
                f"producer bytes drifted for {path}: {actual} != pinned {recorded}"
            )
        producer_paths[path] = role

    script_rel = relative(script_arg)
    if script_rel not in producer_paths:
        raise Rejection(
            f"invoked producer {script_rel} is not pinned by the static lock"
        )

    materializers = {p for p, role in producer_paths.items() if role == "materializer"}
    if not materializers:
        raise Rejection("no materializer producer is pinned")
    invoked = any(
        re.search(rf"(?:^|[\s;|])(?:bash|sh)\s+{re.escape(name)}(?:\s|$)", line)
        for name in materializers
        for line in workflow_text.splitlines()
    )
    if not invoked:
        raise Rejection(
            "candidate/workflow contract mismatch: workflow never invokes the "
            f"materializer {sorted(materializers)} through a run step"
        )

    for match in USES_RE.finditer(workflow_text):
        ref = match.group(1)
        name, _, pin = ref.partition("@")
        if not HEX40.match(pin):
            raise Rejection(f"mutable action reference: {ref} (must pin a full commit)")
        recorded = next(
            (a for a in actions if isinstance(a, dict) and a.get("name") == name),
            None,
        )
        if not isinstance(recorded, dict) or recorded.get("commit") != pin:
            raise Rejection(
                f"action {name}@{pin} is not recorded by the static lock"
            )

    for match in SCRIPT_REF_RE.finditer(workflow_text):
        referenced = match.group(0).rstrip("\"'")
        if referenced not in producer_paths:
            raise Rejection(
                f"workflow invokes unhashed producer script: {referenced}"
            )

    triggers = parse_triggers(workflow_text)
    if triggers != {"workflow_dispatch"}:
        raise Rejection(
            f"candidate/workflow contract mismatch on triggers: {sorted(triggers)}"
        )
    permissions = top_level_block(workflow_text, "permissions")
    if not any(child.replace(" ", "") == "contents:read" for child in permissions):
        raise Rejection(
            "candidate/workflow contract mismatch on permissions: "
            "contents: read is required"
        )

    return (
        len(producer_paths),
        f"phase18-materialization-ci: PASS workflow={workflow_rel} "
        f"producers={len(producer_paths)} actions={len(actions)} "
        f"triggers=workflow_dispatch",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="repository root (default: cwd)")
    parser.add_argument("--workflow", required=True, help="materialization workflow path")
    parser.add_argument("--script", required=True, help="invoked producer script path")
    parser.add_argument(
        "--static-lock",
        default=None,
        help=f"static lock path (default: <root>/{DEFAULT_STATIC_LOCK})",
    )
    args = parser.parse_args()
    root = Path(args.root)
    static_lock = (
        root / DEFAULT_STATIC_LOCK if args.static_lock is None else Path(args.static_lock)
    )
    try:
        _, summary = verify(root, Path(args.workflow), Path(args.script), static_lock)
    except Rejection as rejection:
        print(f"FAIL: {rejection}")
        return 2
    except OSError as error:
        print(f"FAIL: {error}")
        return 2
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())

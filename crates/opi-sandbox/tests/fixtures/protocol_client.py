#!/usr/bin/env python3
"""Product-neutral ``command-execution-jsonl-v1`` host client for opi-sandbox.

Drives the REAL ``opi-sandbox backend --stdio`` binary and is OS-aware:

- ``initialize -> ready`` (negotiation; the command is not disclosed until ready
  validates) on every platform;
- on a SUPPORTED platform (Linux, task 16.13): ``execute -> accepted ->
  started{supervised, restricted} -> ... -> completed`` — the backend runs a
  confined target end-to-end against a real workspace (the DoD's
  ``backend --stdio`` positive sentinel);
- on an UNSUPPORTED platform (Windows; macOS until 16.14.1):
  ``execute -> accepted -> failed{unavailable, handshake}`` — the Phase 16.12
  pre-start refusal (the target never runs).

Stdlib only; imports no opi module. Exit 0 = the backend behaved as the spec
requires; non-zero = an assertion failure (the calling Rust test reports
stdout/stderr).

Usage: protocol_client.py <path-to-opi-sandbox-binary>
"""

import json
import platform
import subprocess
import sys
import tempfile

WIRE = "command-execution-jsonl-v1"
# The platform is supported (Landlock + seccomp) on Linux as of task 16.13;
# macOS lands in 16.14.1; Windows publishes no confinement artifact in Phase 16.
SUPPORTED = platform.system() == "Linux"


def _send(proc, frame):
    proc.stdin.write(json.dumps(frame) + "\n")
    proc.stdin.flush()


def _read_frame(proc):
    line = proc.stdout.readline()
    if not line:
        return None
    return json.loads(line)


def _negotiate(proc, rid):
    """initialize -> ready; returns the ready payload or exits 1."""
    _send(
        proc,
        {
            "type": "initialize",
            "payload": {
                "request_id": rid,
                "deadline_ms": 30000,
                "adapter_config": {},
                "supported_protocols": [WIRE],
            },
        },
    )
    ready = _read_frame(proc)
    if ready is None:
        sys.stderr.write("no ready frame; stderr=" + (proc.stderr.read() or "") + "\n")
        sys.exit(1)
    assert ready["type"] == "ready", ready
    rp = ready["payload"]
    assert rp["request_id"] == rid, rp
    assert rp["selected_protocol"] == WIRE, rp
    assert rp["implementation_version"], rp
    assert rp["target"], rp
    return rp


def _execute_confined(proc, rid, workspace):
    """SUPPORTED path: execute -> started{supervised, restricted} -> completed
    with no failed frame. Returns 0 on success, 1 on assertion failure."""
    _send(
        proc,
        {
            "type": "execute",
            "payload": {
                "request_id": rid,
                "program": "sh",
                "args": ["-c", "echo hi"],
                "workspace": workspace,
                "cwd": workspace,
                "timeout_ms": 10000,
                "env_inherit": "inherit",
                "env_additions": {},
            },
        },
    )
    started = None
    failed = None
    completed = None
    for _ in range(16):
        frame = _read_frame(proc)
        if frame is None:
            break
        ftype = frame["type"]
        if ftype == "started":
            started = frame["payload"]
        elif ftype == "failed":
            failed = frame["payload"]
            break
        elif ftype == "completed":
            completed = frame["payload"]
            break
        # accepted / stdout / stderr / diagnostic are skipped.
    if failed is not None:
        sys.stderr.write(
            "supported backend rejected the execute: " + json.dumps(failed) + "\n"
        )
        return 1
    if started is None:
        sys.stderr.write(
            "no started frame on supported backend; stderr=" + (proc.stderr.read() or "") + "\n"
        )
        return 1
    assert started["guarantee"] == "supervised", started
    assert started["policy"] == "restricted", started
    if completed is None:
        sys.stderr.write("no completed frame on supported backend\n")
        return 1
    return 0


def _execute_refused(proc, rid):
    """UNSUPPORTED path: execute -> failed{unavailable, handshake} (16.12
    pre-start refusal). Returns 0 on success, 1 on assertion failure."""
    _send(
        proc,
        {
            "type": "execute",
            "payload": {
                "request_id": rid,
                "program": "sh",
                "args": ["-c", "echo hi"],
                "workspace": "/ws",
                "cwd": "/ws",
                "timeout_ms": 10000,
                "env_inherit": "inherit",
                "env_additions": {},
            },
        },
    )
    failed = None
    for _ in range(8):
        frame = _read_frame(proc)
        if frame is None:
            break
        if frame["type"] == "failed":
            failed = frame["payload"]
            break
    if failed is None:
        sys.stderr.write(
            "no failed frame; stderr=" + (proc.stderr.read() or "") + "\n"
        )
        return 1
    assert failed["code"] == "unavailable", failed
    assert failed["phase"] == "handshake", failed
    return 0


def main():
    if len(sys.argv) != 2:
        sys.stderr.write("usage: protocol_client.py <path-to-opi-sandbox-binary>\n")
        return 2
    binary = sys.argv[1]
    # A real workspace is required on a supported platform so the Landlock fs
    # ruleset can build (PathFd on the workspace) and the target actually runs.
    workspace = tempfile.mkdtemp(prefix="opi-backend-ws-") if SUPPORTED else "/ws"
    proc = subprocess.Popen(
        [binary, "backend", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    rid = "py-host-1"

    _negotiate(proc, rid)

    if SUPPORTED:
        rc = _execute_confined(proc, rid, workspace)
    else:
        rc = _execute_refused(proc, rid)
    if rc != 0:
        return rc

    # After the terminal frame the backend exits 0; close stdin and reap.
    proc.stdin.close()
    try:
        wait_rc = proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        sys.stderr.write("backend did not exit after the terminal frame\n")
        return 1
    assert wait_rc == 0, wait_rc
    return 0


if __name__ == "__main__":
    sys.exit(main())

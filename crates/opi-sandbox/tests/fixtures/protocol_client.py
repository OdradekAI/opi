#!/usr/bin/env python3
"""Product-neutral ``command-execution-jsonl-v1`` host client for opi-sandbox.

Drives the REAL ``opi-sandbox backend --stdio`` binary through protocol
negotiation (initialize -> ready) and the Phase 16.12 pre-start refusal
(execute -> accepted -> failed{unavailable, handshake}), then closes stdin and
asserts a clean backend exit (0). The target program is never run: the platform
gate refuses before target start on every platform in 16.12.

Stdlib only; imports no opi module. Exit code 0 = the backend behaved as the
spec requires; non-zero = an assertion failure (the calling Rust test reports
stdout/stderr).

Usage: protocol_client.py <path-to-opi-sandbox-binary>
"""

import json
import subprocess
import sys

WIRE = "command-execution-jsonl-v1"


def _send(proc, frame):
    proc.stdin.write(json.dumps(frame) + "\n")
    proc.stdin.flush()


def main():
    if len(sys.argv) != 2:
        sys.stderr.write("usage: protocol_client.py <path-to-opi-sandbox-binary>\n")
        return 2
    binary = sys.argv[1]
    proc = subprocess.Popen(
        [binary, "backend", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    rid = "py-host-1"

    # initialize -> ready (negotiation; the command is not disclosed until ready
    # validates).
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
    ready_line = proc.stdout.readline()
    if not ready_line:
        sys.stderr.write("no ready frame; stderr=" + (proc.stderr.read() or "") + "\n")
        return 1
    ready = json.loads(ready_line)
    assert ready["type"] == "ready", ready
    rp = ready["payload"]
    assert rp["request_id"] == rid, rp
    assert rp["selected_protocol"] == WIRE, rp
    assert rp["implementation_version"], rp
    assert rp["target"], rp

    # execute -> accepted -> failed{unavailable, handshake} (16.12: the platform
    # gate refuses before target start on every platform).
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
        line = proc.stdout.readline()
        if not line:
            break
        frame = json.loads(line)
        if frame["type"] == "failed":
            failed = frame["payload"]
            break
        # `accepted` (and any other non-terminal frame) is skipped.
    if failed is None:
        sys.stderr.write("no failed frame; stderr=" + (proc.stderr.read() or "") + "\n")
        return 1
    assert failed["code"] == "unavailable", failed
    assert failed["phase"] == "handshake", failed

    # After the terminal frame the backend exits 0; close stdin and reap.
    proc.stdin.close()
    try:
        rc = proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        sys.stderr.write("backend did not exit after the terminal frame\n")
        return 1
    assert rc == 0, rc
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Product-neutral command-execution-jsonl-v1 client for opi-sandbox.

The client imports no Opi code. On Linux and macOS it drives four fresh
``opi-sandbox backend --stdio`` processes against an explicit target and proves
binary stdout/stderr plus normal, nonzero, signal, and timeout outcomes. On an
unsupported platform it proves the negotiated pre-start refusal.

Usage: protocol_client.py <opi-sandbox-binary> [archive-sha256] [expected-target]
"""

import base64
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile

WIRE = "command-execution-jsonl-v1"
SUPPORTED = platform.system() in ("Linux", "Darwin")
ESCAPE = "\ue000"


def _native(value):
    """Encode a native string into the protocol's byte-preserving form."""
    if platform.system() == "Windows":
        raw = value.encode("utf-16-le", errors="surrogatepass")
    else:
        raw = os.fsencode(value)
    return "".join(ESCAPE + chr(byte) for byte in raw)


def _send(proc, frame):
    proc.stdin.write(json.dumps(frame) + "\n")
    proc.stdin.flush()


def _read_frame(proc):
    line = proc.stdout.readline()
    if not line:
        return None
    return json.loads(line)


def _negotiate(proc, rid):
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
        raise AssertionError("no ready frame; stderr=" + (proc.stderr.read() or ""))
    assert ready["type"] == "ready", ready
    payload = ready["payload"]
    assert payload["request_id"] == rid, payload
    assert payload["selected_protocol"] == WIRE, payload
    assert payload["implementation"] == "opi-sandbox", payload
    assert payload["implementation_version"], payload
    assert payload["target"], payload
    if len(sys.argv) > 3:
        assert payload["target"] == sys.argv[3], payload


def _execute(binary, workspace, rid, mode, timeout_ms, expected):
    proc = subprocess.Popen(
        [binary, "backend", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    try:
        _negotiate(proc, rid)
        _send(
            proc,
            {
                "type": "execute",
                "payload": {
                    "request_id": rid,
                    "program": _native("/bin/sh"),
                    "args": [
                        _native(os.path.join(workspace, "target.sh")),
                        _native(mode),
                        _native("arg one"),
                        _native("--literal"),
                    ],
                    "workspace": _native(workspace),
                    "cwd": _native(workspace),
                    "timeout_ms": timeout_ms,
                    "env_inherit": "inherit",
                    "env_additions": {},
                },
            },
        )

        accepted = False
        started = None
        completed = None
        failure = None
        stdout = bytearray()
        stderr = bytearray()
        for _ in range(64):
            frame = _read_frame(proc)
            if frame is None:
                break
            kind = frame["type"]
            payload = frame["payload"]
            assert payload["request_id"] == rid, payload
            if kind == "accepted":
                accepted = True
            elif kind == "started":
                started = payload
            elif kind == "stdout":
                stdout.extend(base64.b64decode(payload["data"], validate=True))
            elif kind == "stderr":
                stderr.extend(base64.b64decode(payload["data"], validate=True))
            elif kind == "completed":
                completed = payload
                break
            elif kind == "failed":
                failure = payload
                break

        assert failure is None, failure
        assert accepted, "missing accepted frame"
        assert started is not None, "missing started frame"
        assert started["guarantee"] == "restricted", started
        assert started["policy"] == "restricted", started
        assert completed is not None, "missing completed frame"
        assert bytes(stdout) == expected.get("stdout", b""), bytes(stdout)
        assert bytes(stderr) == expected.get("stderr", b""), bytes(stderr)
        for key in ("exit", "signal", "timed_out", "cancelled"):
            assert completed[key] == expected[key], (key, completed, expected)
        assert completed["cleanup"] == "confirmed", completed

        proc.stdin.close()
        rc = proc.wait(timeout=10)
        assert rc == 0, (rc, proc.stderr.read())
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def _execute_refused(binary):
    proc = subprocess.Popen(
        [binary, "backend", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    rid = "unsupported-1"
    try:
        _negotiate(proc, rid)
        _send(
            proc,
            {
                "type": "execute",
                "payload": {
                    "request_id": rid,
                    "program": _native("cmd"),
                    "args": [_native("/C"), _native("exit 0")],
                    "workspace": _native("C:\\ws"),
                    "cwd": _native("C:\\ws"),
                    "timeout_ms": 10000,
                    "env_inherit": "inherit",
                    "env_additions": {},
                },
            },
        )
        accepted = _read_frame(proc)
        failed = _read_frame(proc)
        assert accepted["type"] == "accepted", accepted
        assert failed["type"] == "failed", failed
        assert failed["payload"]["code"] == "unavailable", failed
        assert failed["payload"]["phase"] == "handshake", failed
        proc.stdin.close()
        assert proc.wait(timeout=10) == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def _write_target(workspace):
    target = os.path.join(workspace, "target.sh")
    with open(target, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "#!/bin/sh\n"
            "mode=$1; shift\n"
            "[ \"$#\" -eq 2 ] && [ \"$1\" = 'arg one' ] && "
            "[ \"$2\" = '--literal' ] || exit 96\n"
            "case \"$mode\" in\n"
            "  output) printf '\\001\\377'; printf '\\002\\376' >&2 ;;\n"
            "  nonzero) exit 37 ;;\n"
            "  signal) kill -TERM $$; sleep 5 ;;\n"
            "  timeout) sleep 5 ;;\n"
            "  *) exit 97 ;;\n"
            "esac\n"
        )
    os.chmod(target, 0o755)


def main():
    if len(sys.argv) not in (2, 3, 4):
        sys.stderr.write(
            "usage: protocol_client.py <opi-sandbox-binary> "
            "[archive-sha256] [expected-target]\n"
        )
        return 2
    binary = os.path.abspath(sys.argv[1])
    archive_sha256 = sys.argv[2] if len(sys.argv) >= 3 else None

    if not SUPPORTED:
        _execute_refused(binary)
        return 0

    workspace = tempfile.mkdtemp(prefix="opi-backend-ws-")
    try:
        _write_target(workspace)
        base = {"signal": None, "timed_out": False, "cancelled": False}
        _execute(
            binary,
            workspace,
            "output-1",
            "output",
            10000,
            {**base, "exit": 0, "stdout": b"\x01\xff", "stderr": b"\x02\xfe"},
        )
        _execute(
            binary,
            workspace,
            "nonzero-1",
            "nonzero",
            10000,
            {**base, "exit": 37},
        )
        _execute(
            binary,
            workspace,
            "signal-1",
            "signal",
            10000,
            {**base, "exit": None, "signal": 15},
        )
        _execute(
            binary,
            workspace,
            "timeout-1",
            "timeout",
            150,
            {
                "exit": None,
                "signal": None,
                "timed_out": True,
                "cancelled": False,
            },
        )
    finally:
        shutil.rmtree(workspace)

    marker = "opi-sandbox-backend-smoke: OK"
    if archive_sha256:
        marker += " archive_sha256=" + archive_sha256
    print(marker)
    return 0


if __name__ == "__main__":
    sys.exit(main())

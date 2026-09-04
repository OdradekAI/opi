#!/usr/bin/env python3
"""opi-eval deterministic local provider fixture.

A scripted model endpoint with two deterministic modes. Fixture mode
Hermetic mode reads one JSON request line per turn from stdin and writes
one fixed JSON response line to stdout, exiting at EOF or after the bounded
request cap; it makes no network connection. Listener mode (native mode)
binds exactly one declared endpoint (``--listen <host:port>`` plus a
required ``--request-log`` path) and serves deterministic OpenAI-compatible
Chat Completions responses: a fixed streaming tool-call turn, then a final
assistant turn once a tool result appears, with every served request
recorded as a normalized digest line. Neither mode reads credentials,
ambient variables, or user/project resources (EVAL-AGT-006); no oracle or
reference-solution material appears in any response.

Request lines must be objects with exactly a ``prompt`` string field; any
other shape terminates with exit code 2 and a one-line diagnostic on stderr.
Responses are deterministic single objects with exactly a ``schema`` and a
``content`` field.
"""

from __future__ import annotations

import hashlib
import http.server
import json
import sys
import threading

SCHEMA = "opi-eval-scripted-provider/1"
LOG_SCHEMA = "opi-eval-scripted-provider-log/1"
RESPONSE_SCRIPT = "scripted-turn-plan/1"
RESPONSE_CONTENT = "scripted-provider: acknowledged"
MAX_REQUESTS = 8
MAX_LISTENER_REQUESTS = 64
COMPLETION_ID = "chatcmpl-scripted-eval"
CREATED = 1750000000
FINAL_CONTENT = "scripted-provider: integration turn complete"
TOOL_NAME = "bash"
TOOL_ARGUMENTS = (
    "{\"command\": \"printf 'scripted-integration-result\\\\n' > answer.txt\"}"
)


def canonical_digest(raw: bytes) -> str:
    """SHA-256 over the canonical JSON form; raw bytes when not JSON."""
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return hashlib.sha256(raw).hexdigest()
    canonical = json.dumps(parsed, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def wants_final_turn(body: dict) -> bool:
    """The turn plan keys on the conversation shape, never the prompt
    text: once a tool result is present the next turn is the final
    assistant answer."""
    messages = body.get("messages")
    if not isinstance(messages, list):
        return False
    return any(
        isinstance(message, dict) and message.get("role") == "tool"
        for message in messages
    )


def tool_call_json() -> dict:
    return {
        "id": "call-scripted-0001",
        "type": "function",
        "function": {"name": TOOL_NAME, "arguments": TOOL_ARGUMENTS},
    }


def stream_chunks(final: bool) -> list[bytes]:
    """Deterministic SSE chunks: role delta, payload delta, finish, DONE."""
    first = {"role": "assistant"}
    if final:
        payload = {"content": FINAL_CONTENT}
        finish = "stop"
    else:
        payload = {"tool_calls": [dict(tool_call_json(), index=0)]}
        finish = "tool_calls"
    chunks = []
    for delta, finish_reason in ((first, None), (payload, None), ({}, finish)):
        chunk = {
            "id": COMPLETION_ID,
            "object": "chat.completion.chunk",
            "created": CREATED,
            "model": "scripted/eval",
            "choices": [
                {"index": 0, "delta": delta, "finish_reason": finish_reason}
            ],
        }
        chunks.append(
            b"data: "
            + json.dumps(chunk, sort_keys=True, separators=(",", ":")).encode()
            + b"\n\n"
        )
    chunks.append(b"data: [DONE]\n\n")
    return chunks


def completion_json(final: bool) -> dict:
    message = (
        {"role": "assistant", "content": FINAL_CONTENT}
        if final
        else {"role": "assistant", "content": None,
              "tool_calls": [tool_call_json()]}
    )
    return {
        "id": COMPLETION_ID,
        "object": "chat.completion",
        "created": CREATED,
        "model": "scripted/eval",
        "choices": [
            {"index": 0, "message": message,
             "finish_reason": "stop" if final else "tool_calls"}
        ],
    }


def main() -> int:
    argv = sys.argv[1:]
    if argv and argv[0] == "--listen":
        return serve_listener(argv[1:])
    if argv:
        print(
            "scripted-provider: unknown arguments (fixture mode reads "
            "stdin; listener mode is --listen <host:port> "
            "--request-log <path>)",
            file=sys.stderr,
        )
        return 2
    return serve_stdin()


def serve_listener(rest: list[str]) -> int:
    well_formed = (
        len(rest) == 3 and rest[1] == "--request-log" and bool(rest[2])
    )
    if not well_formed:
        print(
            "scripted-provider: --listen requires <host:port> and "
            "--request-log <path>",
            file=sys.stderr,
        )
        return 2
    endpoint = rest[0]
    log_path = rest[2] if len(rest) == 3 and rest[1] == "--request-log" else ""
    if ":" not in endpoint or not log_path:
        print(
            "scripted-provider: --listen requires <host:port> and "
            "--request-log <path>",
            file=sys.stderr,
        )
        return 2
    host, _, port_text = endpoint.rpartition(":")
    try:
        port = int(port_text)
    except ValueError:
        port = -1
    if not host or not 0 <= port <= 65535:
        print("scripted-provider: invalid listen endpoint", file=sys.stderr)
        return 2

    lock = threading.Lock()
    state = {"served": 0}

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, format, *args):  # noqa: A002 - stdlib signature
            return

        def _record(self, method: str, path: str, raw: bytes, status: int):
            line = json.dumps(
                {
                    "schema": LOG_SCHEMA,
                    "seq": state["served"],
                    "method": method,
                    "path": path,
                    "request_sha256": canonical_digest(raw),
                    "response_script": RESPONSE_SCRIPT,
                    "status": status,
                },
                sort_keys=True,
                separators=(",", ":"),
            )
            with lock:
                with open(log_path, "a", encoding="utf-8") as log:
                    log.write(line + "\n")

        def _reply(self, status: int, content_type: str, body: bytes):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            self.do_POST()

        def do_POST(self):
            raw = b""
            length = self.headers.get("Content-Length")
            if length is not None:
                try:
                    raw = self.rfile.read(int(length))
                except ValueError:
                    raw = b""
            admitted = self.path in ("/chat/completions",
                                     "/v1/chat/completions")
            if self.command != "POST" or not admitted:
                self._record(self.command, self.path, raw, 404)
                self._reply(404, "application/json", json.dumps(
                    {"error": {"message": "scripted-provider: undeclared "
                                          "path", "type": "invalid_request"}},
                    sort_keys=True).encode())
                return
            with lock:
                state["served"] += 1
                served = state["served"]
            if served > MAX_LISTENER_REQUESTS:
                self._record("POST", self.path, raw, 429)
                self._reply(429, "application/json", json.dumps(
                    {"error": {"message": f"scripted-provider: request cap "
                                          f"{MAX_LISTENER_REQUESTS} exceeded",
                               "type": "rate_limit"}},
                    sort_keys=True).encode())
                return
            try:
                body = json.loads(raw.decode("utf-8"))
                if not isinstance(body, dict):
                    raise ValueError("not an object")
            except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
                self._record("POST", self.path, raw, 400)
                self._reply(400, "application/json", json.dumps(
                    {"error": {"message": "scripted-provider: request body "
                                          "is not a JSON object",
                               "type": "invalid_request"}},
                    sort_keys=True).encode())
                return
            final = wants_final_turn(body)
            if body.get("stream") is True:
                payload = b"".join(stream_chunks(final))
                self._record("POST", self.path, raw, 200)
                self._reply(200, "text/event-stream", payload)
            else:
                payload = json.dumps(completion_json(final), sort_keys=True,
                                     separators=(",", ":")).encode()
                self._record("POST", self.path, raw, 200)
                self._reply(200, "application/json", payload)

    server = http.server.ThreadingHTTPServer((host, port), Handler)
    server.daemon_threads = True
    bound_host, bound_port = server.server_address[:2]
    print(f"listening {bound_host}:{bound_port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


def serve_stdin() -> int:
    served = 0
    for line in sys.stdin:  # fixture mode: one line in, one line out
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            print("scripted-provider: request line is not JSON", file=sys.stderr)
            return 2
        if (
            not isinstance(request, dict)
            or set(request) != {"prompt"}
            or not isinstance(request["prompt"], str)
        ):
            print(
                "scripted-provider: request must be {\"prompt\": <string>}",
                file=sys.stderr,
            )
            return 2
        served += 1
        if served > MAX_REQUESTS:
            print(
                f"scripted-provider: request cap {MAX_REQUESTS} exceeded",
                file=sys.stderr,
            )
            return 3
        sys.stdout.write(
            json.dumps(
                {"schema": SCHEMA, "content": RESPONSE_CONTENT},
                separators=(",", ":"),
                sort_keys=True,
            )
            + "\n"
        )
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())

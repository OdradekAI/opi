#!/usr/bin/env python3
"""opi-eval deterministic local provider fixture.

A scripted model endpoint with two deterministic modes. Fixture mode
Hermetic mode reads one JSON request line per turn from stdin and writes
one fixed JSON response line to stdout, exiting at EOF or after the bounded
request cap; it makes no network connection. Listener mode (native mode)
binds exactly one declared endpoint (``--listen <host:port>`` plus a
required ``--request-log`` path) and serves deterministic responses on the
three admitted protocol paths (OpenAI Chat Completions, Anthropic Messages,
OpenAI Responses): a fixed streaming tool-call turn, then a final assistant
turn once a tool result appears, with every served request recorded as a
normalized digest line. Neither mode reads credentials, ambient variables,
or user/project resources (EVAL-AGT-006); no oracle or reference-solution
material appears in any response.

Terminal failure shapes are scripted by the request model id: a model whose
final path segment is ``auth-fail`` answers 401, ``rate-limit`` answers 429
on every request, ``retry`` answers 429 once and then serves the normal
turn (one intermediate HTTP error never becomes the terminal result), and
``protocol-error`` answers HTTP 200 with a body no protocol can parse.
Every other model serves the deterministic tool loop.

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
ANTHROPIC_MESSAGE_ID = "msg_scripted_eval"
RESPONSES_ID = "resp_scripted_eval"
TOOL_USE_ID = "toolu_scripted_0001"
CALL_ID = "call-scripted-0001"
CREATED = 1750000000
FINAL_CONTENT = "scripted-provider: integration turn complete"
TOOL_NAME = "bash"
TOOL_ARGUMENTS = (
    "{\"command\": \"printf 'scripted-integration-result\\\\n' > answer.txt\"}"
)
# Model-keyed terminal failure scripts (P19-D02).
SCRIPT_AUTH_FAIL = "auth-fail"
SCRIPT_RATE_LIMIT = "rate-limit"
SCRIPT_RETRY = "retry"
SCRIPT_PROTOCOL_ERROR = "protocol-error"
SCRIPTS = (SCRIPT_AUTH_FAIL, SCRIPT_RATE_LIMIT, SCRIPT_RETRY,
           SCRIPT_PROTOCOL_ERROR)


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
    assistant answer. Tool results appear as OpenAI ``tool`` messages,
    Anthropic user-message ``tool_result`` content blocks, or OpenAI
    Responses ``function_call_output`` input items."""
    messages = body.get("messages")
    if isinstance(messages, list):
        for message in messages:
            if not isinstance(message, dict):
                continue
            if message.get("role") == "tool":
                return True
            content = message.get("content")
            if (message.get("role") == "user" and isinstance(content, list)
                    and any(isinstance(block, dict)
                            and block.get("type") == "tool_result"
                            for block in content)):
                return True
    items = body.get("input")
    if isinstance(items, list):
        return any(isinstance(item, dict)
                   and item.get("type") == "function_call_output"
                   for item in items)
    return False


def scripted_failure(model) -> str | None:
    """The terminal failure script a request model selects, if any."""
    if not isinstance(model, str):
        return None
    for separator in ("/", ":", " "):
        model = model.rsplit(separator, 1)[-1]
    return model if model in SCRIPTS else None


class RetryState:
    """Per-listener retry-script bookkeeping: the first request of a
    retry-scripted model fails once, every later one succeeds."""

    def __init__(self) -> None:
        self.seen: set[str] = set()
        self.lock = threading.Lock()

    def consume(self, model: str) -> bool:
        """True when this request must fail with the retryable 429."""
        with self.lock:
            if model in self.seen:
                return False
            self.seen.add(model)
            return True


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



def anthropic_content(final: bool) -> list[dict]:
    """One deterministic content block: the bash tool call, or the final
    assistant text."""
    if final:
        return [{"type": "text", "text": FINAL_CONTENT}]
    return [{
        "type": "tool_use",
        "id": TOOL_USE_ID,
        "name": TOOL_NAME,
        "input": json.loads(TOOL_ARGUMENTS),
    }]


def anthropic_message(final: bool) -> dict:
    return {
        "id": ANTHROPIC_MESSAGE_ID,
        "type": "message",
        "role": "assistant",
        "model": "scripted/eval",
        "content": anthropic_content(final),
        "stop_reason": "end_turn" if final else "tool_use",
        "usage": {"input_tokens": 12, "output_tokens": 34},
    }


def anthropic_events(final: bool) -> list[bytes]:
    """Deterministic Anthropic SSE event sequence."""
    events = [
        {"type": "message_start", "message": anthropic_message(final)},
        {"type": "content_block_start", "index": 0,
         "content_block": anthropic_content(final)[0]},
    ]
    block = anthropic_content(final)[0]
    if final:
        events.append({"type": "content_block_delta", "index": 0,
                       "delta": {"type": "text_delta",
                                 "text": FINAL_CONTENT}})
    else:
        events.append({"type": "content_block_delta", "index": 0,
                       "delta": {"type": "input_json_delta",
                                 "partial_json": TOOL_ARGUMENTS}})
    events.append({"type": "content_block_stop", "index": 0})
    events.append({"type": "message_delta",
                   "delta": {"stop_reason":
                             "end_turn" if final else "tool_use",
                             "stop_sequence": None},
                   "usage": {"output_tokens": 34}})
    events.append({"type": "message_stop"})
    payload = []
    for event in events:
        payload.append(
            b"event: "
            + event["type"].encode()
            + b"\ndata: "
            + json.dumps(event, sort_keys=True,
                         separators=(",", ":")).encode() + b"\n\n"
        )
    # The Anthropic wire ends with message_stop; it has no [DONE]
    # sentinel (parsing one would be a stream error for the client).
    return payload


def responses_output(final: bool) -> list[dict]:
    if final:
        return [{
            "type": "message",
            "id": "msg_scripted_final",
            "status": "completed",
            "role": "assistant",
            "content": [{"type": "output_text", "text": FINAL_CONTENT,
                         "annotations": []}],
        }]
    return [{
        "type": "function_call",
        "id": "fc_scripted_0001",
        "call_id": CALL_ID,
        "name": TOOL_NAME,
        "arguments": TOOL_ARGUMENTS,
        "status": "completed",
    }]


def responses_object(final: bool) -> dict:
    return {
        "id": RESPONSES_ID,
        "object": "response",
        "created_at": CREATED,
        "status": "completed",
        "model": "scripted/eval",
        "output": responses_output(final),
        "usage": {"input_tokens": 12, "output_tokens": 34,
                  "total_tokens": 46},
    }


def responses_events(final: bool) -> list[bytes]:
    """Deterministic OpenAI Responses SSE event sequence."""
    items = responses_output(final)
    # The real Responses API creates the response empty and fills its
    # output through the stream; the created frame carries `output: []`.
    events = [{"type": "response.created", "response": responses_object(
        final) | {"output": []}}]
    events.append({"type": "response.output_item.added",
                   "output_index": 0, "item": items[0]})
    if final:
        events.append({"type": "response.output_text.delta",
                       "item_id": items[0]["id"],
                       "output_index": 0, "content_index": 0,
                       "delta": FINAL_CONTENT})
    else:
        events.append({"type": "response.function_call_arguments.delta",
                       "item_id": items[0]["id"], "output_index": 0,
                       "delta": TOOL_ARGUMENTS})
        events.append({"type": "response.function_call_arguments.done",
                       "item_id": items[0]["id"], "output_index": 0,
                       "arguments": TOOL_ARGUMENTS})
    events.append({"type": "response.output_item.done",
                   "output_index": 0, "item": items[0]})
    events.append({"type": "response.completed",
                   "response": responses_object(final)})
    payload = []
    for event in events:
        payload.append(
            b"data: "
            + json.dumps(event, sort_keys=True,
                         separators=(",", ":")).encode() + b"\n\n"
        )
    payload.append(b"data: [DONE]\n\n")
    return payload


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
    retry_state = RetryState()

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, format, *args):  # noqa: A002 - stdlib signature
            return

        def _record(self, method: str, path: str, raw: bytes, status: int,
                    model=None):
            line = json.dumps(
                {
                    "schema": LOG_SCHEMA,
                    "seq": state["served"],
                    "method": method,
                    "path": path,
                    "model": model,
                    "auth": self.auth_kind(),
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

        def auth_kind(self) -> str | None:
            if self.headers.get("Authorization", "").startswith("Bearer "):
                return "bearer"
            if self.headers.get("x-api-key"):
                return "x-api-key"
            return None

        def _error_body(self, kind: str, message: str) -> bytes:
            return json.dumps(
                {"error": {"message": message, "type": kind}},
                sort_keys=True,
            ).encode()

        def _reply(self, status: int, content_type: str, body: bytes,
                   extra_headers: dict[str, str] | None = None):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            for key, value in (extra_headers or {}).items():
                self.send_header(key, value)
            self.end_headers()
            self.wfile.write(body)

        def _admit(self, body: dict) -> tuple[str, bytes] | None:
            """Terminal failure scripts (model-keyed, P19-D02)."""
            script = scripted_failure(body.get("model"))
            if script == SCRIPT_AUTH_FAIL:
                payload = self._error_body(
                    "authentication_error",
                    "scripted-provider: invalid credentials")
                return "401", payload
            if script == SCRIPT_RATE_LIMIT:
                payload = self._error_body(
                    "rate_limit",
                    "scripted-provider: rate limit exhausted")
                return "429", payload
            if script == SCRIPT_RETRY and retry_state.consume(
                    str(body.get("model"))):
                payload = self._error_body(
                    "rate_limit",
                    "scripted-provider: retryable rate limit")
                return ("429-retry", payload)
            if script == SCRIPT_PROTOCOL_ERROR:
                return ("200", b"scripted-provider: not-json{")
            return None

        def do_POST(self):
            raw = b""
            length = self.headers.get("Content-Length")
            if length is not None:
                try:
                    raw = self.rfile.read(int(length))
                except ValueError:
                    raw = b""
            # Both pinned clients resolve the same connection root with
            # their own path conventions: OpenAI SDKs post to
            # ``/chat/completions`` and ``/responses`` on a ``/v1``-bearing
            # base URL, Anthropic SDKs and Opi append the full
            # ``/v1/...`` path to a root base URL.
            admitted = self.path in ("/chat/completions",
                                     "/v1/chat/completions",
                                     "/v1/messages",
                                     "/v1/responses",
                                     "/responses")
            if self.command != "POST" or not admitted:
                self._record(self.command, self.path, raw, 404)
                self._reply(404, "application/json", self._error_body(
                    "invalid_request",
                    "scripted-provider: undeclared path"))
                return
            with lock:
                state["served"] += 1
                served = state["served"]
            if served > MAX_LISTENER_REQUESTS:
                self._record("POST", self.path, raw, 429)
                self._reply(429, "application/json", self._error_body(
                    "rate_limit",
                    f"scripted-provider: request cap "
                    f"{MAX_LISTENER_REQUESTS} exceeded"))
                return
            try:
                body = json.loads(raw.decode("utf-8"))
                if not isinstance(body, dict):
                    raise ValueError("not an object")
            except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
                self._record("POST", self.path, raw, 400)
                self._reply(400, "application/json", self._error_body(
                    "invalid_request",
                    "scripted-provider: request body is not a JSON object"))
                return
            scripted = self._admit(body)
            if scripted is not None:
                token, payload = scripted
                status = 200 if token == "200" else int(token.split("-")[0])
                headers = (
                    {"Retry-After": "1"} if token == "429-retry" else None)
                self._record("POST", self.path, raw, status,
                             model=body.get("model"))
                self._reply(status, "application/json", payload, headers)
                return
            final = wants_final_turn(body)
            if self.path == "/v1/messages":
                payload = b"".join(anthropic_events(final)) if (
                    body.get("stream") is True) else json.dumps(
                    anthropic_message(final), sort_keys=True,
                    separators=(",", ":")).encode()
                self._record("POST", self.path, raw, 200,
                             model=body.get("model"))
                self._reply(200, "text/event-stream" if body.get("stream")
                            is True else "application/json", payload)
                return
            if self.path in ("/v1/responses", "/responses"):
                payload = b"".join(responses_events(final)) if (
                    body.get("stream") is True) else json.dumps(
                    responses_object(final), sort_keys=True,
                    separators=(",", ":")).encode()
                self._record("POST", self.path, raw, 200,
                             model=body.get("model"))
                self._reply(200, "text/event-stream" if body.get("stream")
                            is True else "application/json", payload)
                return
            if self.path not in ("/chat/completions", "/v1/chat/completions"):
                self._record("POST", self.path, raw, 404, model=body.get("model"))
                self._reply(404, "application/json", self._error_body(
                    "invalid_request",
                    "scripted-provider: undeclared path"))
                return
            if body.get("stream") is True:
                payload = b"".join(stream_chunks(final))
                self._record("POST", self.path, raw, 200,
                             model=body.get("model"))
                self._reply(200, "text/event-stream", payload)
            else:
                payload = json.dumps(completion_json(final), sort_keys=True,
                                     separators=(",", ":")).encode()
                self._record("POST", self.path, raw, 200,
                             model=body.get("model"))
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

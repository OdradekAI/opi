#!/usr/bin/env python3
"""Tests for the opi-eval scripted provider fixture.

Runs the fixture as a real subprocess and asserts exact deterministic
behavior. The stdin/stdout fixture mode (hermetic-conformance) keeps its byte-identical
contract: fixed response lines, EOF termination, typed rejections for
malformed requests, and the bounded request cap. The listener mode
(native-material) binds exactly one declared endpoint and serves deterministic
OpenAI-compatible Chat Completions responses with a normalized request log.
This is the smoke-addendum gate (``python
crates/opi-eval/scripts/test_scripted_provider.py``).
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("scripted-provider.py")
EXPECTED_LINE = (
    '{"content":"scripted-provider: acknowledged","schema":"opi-eval-scripted-provider/1"}'
)


def run_provider(stdin_text: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT)],
        input=stdin_text,
        capture_output=True,
        text=True,
        timeout=30,
    )


class ScriptedProviderTest(unittest.TestCase):
    def test_serves_one_fixed_response_per_request(self) -> None:
        result = run_provider('{"prompt": "first"}\n{"prompt": "second"}\n')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            EXPECTED_LINE + "\n" + EXPECTED_LINE + "\n",
        )
        self.assertEqual(result.stderr, "")

    def test_is_deterministic_across_runs(self) -> None:
        for _ in range(3):
            result = run_provider('{"prompt": "same"}\n')
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, EXPECTED_LINE + "\n")

    def test_blank_lines_are_ignored_and_eof_terminates_cleanly(self) -> None:
        result = run_provider('\n\n{"prompt": "x"}\n\n')
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, EXPECTED_LINE + "\n")

    def test_malformed_requests_are_rejected_with_typed_errors(self) -> None:
        for bad in ("not json", '{"wrong": 1}', '{"prompt": 3}', '[]'):
            with self.subTest(bad=bad):
                result = run_provider(bad + "\n")
                self.assertEqual(result.returncode, 2)
                self.assertIn("scripted-provider:", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_request_cap_is_bounded(self) -> None:
        # MAX_REQUESTS is pinned in the script; exceeding it terminates.
        cap = 8
        stdin_text = "".join(
            json.dumps({"prompt": f"p{i}"}) + "\n" for i in range(cap + 1)
        )
        result = run_provider(stdin_text)
        self.assertEqual(result.returncode, 3)
        self.assertIn("request cap", result.stderr)
        self.assertEqual(result.stdout.count("\n"), cap)

    def test_no_credential_or_third_party_surface(self) -> None:
        # Credentials and third-party clients stay forbidden in every mode;
        # the network surface exists only as the explicit listener mode.
        source = SCRIPT.read_text(encoding="utf-8")
        for forbidden in ("getenv", "environ", "requests", "OPENAI_API_KEY",
                          "ANTHROPIC_API_KEY"):
            self.assertNotIn(forbidden, source)

    def test_stdin_mode_has_no_listener(self) -> None:
        # Without --listen the process must not bind any socket: the
        # fixture mode stays a pure stdin/stdout pipe.
        result = run_provider('{"prompt": "x"}\n')
        self.assertEqual(result.returncode, 0)
        self.assertNotIn("listening", result.stdout)

    def test_writes_no_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run(
                [sys.executable, str(SCRIPT)],
                input='{"prompt": "x"}\n',
                capture_output=True,
                text=True,
                cwd=tmp,
                timeout=30,
            )
            self.assertEqual(result.returncode, 0)
            self.assertEqual(list(Path(tmp).iterdir()), [])


LISTENER_TURN_ONE_PREFIX = "data: "
LISTENER_DONE = "data: [DONE]\n\n"


class ScriptedProviderListenerTest(unittest.TestCase):
    """Listener mode (native mode): one declared endpoint, deterministic
    OpenAI-compatible Chat Completions serving, normalized request log."""

    @staticmethod
    def stop_listener(process: subprocess.Popen[str]) -> None:
        if process.poll() is None:
            process.kill()
        process.communicate(timeout=10)

    @staticmethod
    def start_listener(request_log: Path):
        import socket

        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        process = subprocess.Popen(
            [sys.executable, str(SCRIPT),
             "--listen", f"127.0.0.1:{port}",
             "--request-log", str(request_log)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        readiness = process.stdout.readline().strip()
        if not readiness.startswith("listening "):
            ScriptedProviderListenerTest.stop_listener(process)
            raise AssertionError(f"no readiness line: {readiness!r}")
        host, ready_port = readiness.split(" ", 1)[1].rsplit(":", 1)
        return process, f"http://{host}:{ready_port}"

    def post(self, base: str, body: dict, path: str = "/v1/chat/completions"):
        import urllib.request

        request = urllib.request.Request(
            base + path,
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=10) as response:
                return response.status, response.read().decode("utf-8")
        except urllib.error.HTTPError as error:
            return error.code, error.read().decode("utf-8")

    @staticmethod
    def chat_body(stream: bool) -> dict:
        return {
            "model": "scripted/eval",
            "stream": stream,
            "messages": [
                {"role": "user", "content": "solve the pinned task"}
            ],
        }

    def test_streaming_tool_call_turn_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                status_a, body_a = self.post(base, self.chat_body(stream=True))
                status_b, body_b = self.post(base, self.chat_body(stream=True))
            finally:
                self.stop_listener(process)
        self.assertEqual(status_a, 200)
        self.assertEqual(body_a, body_b)
        self.assertTrue(body_a.endswith(LISTENER_DONE))
        self.assertIn("tool_calls", body_a)
        self.assertIn("chat.completion.chunk", body_a)
        chunks = [
            json.loads(line.removeprefix(LISTENER_TURN_ONE_PREFIX))
            for line in body_a.splitlines()
            if line.startswith(LISTENER_TURN_ONE_PREFIX)
            and line != "data: [DONE]"
        ]
        choices = [chunk["choices"][0] for chunk in chunks]
        self.assertEqual(len(choices), 3)
        self.assertEqual(choices[0]["delta"], {"role": "assistant"})
        self.assertIsNone(choices[0]["finish_reason"])
        self.assertIn("tool_calls", choices[1]["delta"])
        self.assertIsNone(choices[1]["finish_reason"])
        tool_call = choices[1]["delta"]["tool_calls"][0]
        arguments = json.loads(tool_call["function"]["arguments"])
        self.assertEqual(
            arguments,
            {"command": "printf 'scripted-integration-result\\n' > answer.txt"},
        )
        self.assertEqual(choices[2]["delta"], {})
        self.assertEqual(choices[2]["finish_reason"], "tool_calls")

    def test_non_stream_turn_is_deterministic_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                status, body = self.post(base, self.chat_body(stream=False))
            finally:
                self.stop_listener(process)
        self.assertEqual(status, 200)
        decoded = json.loads(body)
        self.assertEqual(decoded["object"], "chat.completion")
        self.assertIn("tool_calls", decoded["choices"][0]["message"])

    def test_request_log_records_normalized_digests_only(self) -> None:
        secret_prompt = "PROMPT-CANARY-does-not-belong-in-logs"
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                body = self.chat_body(stream=False)
                body["messages"][0]["content"] = secret_prompt
                self.post(base, body)
                self.post(base, body)
            finally:
                self.stop_listener(process)
            log_text = log.read_text(encoding="utf-8")
        lines = [json.loads(line) for line in log_text.splitlines() if line]
        self.assertEqual(len(lines), 2)
        self.assertEqual(lines[0]["request_sha256"], lines[1]["request_sha256"])
        self.assertNotIn(secret_prompt, log_text)
        self.assertEqual(lines[0]["schema"], "opi-eval-scripted-provider-log/1")
        self.assertEqual(lines[0]["response_script"], "scripted-turn-plan/1")

    def test_request_log_retains_observed_output_limits(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                for field in ("max_tokens", "max_completion_tokens", "max_output_tokens"):
                    body = self.chat_body(stream=False) | {field: 16384}
                    self.post(base, body)
            finally:
                self.stop_listener(process)
            rows = [json.loads(line) for line in log.read_text().splitlines()]
        self.assertEqual([r.get("output_limits") for r in rows], [
            {"max_tokens": 16384}, {"max_completion_tokens": 16384},
            {"max_output_tokens": 16384},
        ])

    def test_undeclared_paths_and_methods_are_rejected(self) -> None:
        import urllib.request

        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                status, _ = self.post(base, self.chat_body(stream=False),
                                      path="/v1/other")
                request = urllib.request.Request(base + "/health", method="GET")
                try:
                    with urllib.request.urlopen(request, timeout=10) as response:
                        get_status = response.status
                except urllib.error.HTTPError as error:
                    get_status = error.code
            finally:
                self.stop_listener(process)
        self.assertEqual(status, 404)
        self.assertEqual(get_status, 404)

    def test_listener_request_cap_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                statuses = []
                for _ in range(66):
                    statuses.append(self.post(base, self.chat_body(stream=False))[0])
            finally:
                self.stop_listener(process)
        self.assertEqual(statuses[:64], [200] * 64)
        self.assertEqual(set(statuses[64:]), {429})

    def test_listener_requires_a_request_log_path(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--listen", "127.0.0.1:48199"],
            capture_output=True, text=True, timeout=10,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("scripted-provider:", result.stderr)

    def test_unknown_arguments_are_rejected_in_both_modes(self) -> None:
        for argv in (["--surprise"], ["--listen", "127.0.0.1:48199",
                                      "--request-log", "x", "--extra"]):
            with self.subTest(argv=argv):
                result = subprocess.run(
                    [sys.executable, str(SCRIPT), *argv],
                    capture_output=True, text=True, timeout=10,
                )
                self.assertEqual(result.returncode, 2)
                self.assertIn("scripted-provider:", result.stderr)




class ScriptedProviderProtocolMatrixTest(unittest.TestCase):
    """P19-D02: the listener serves all three admitted protocols with the
    same deterministic tool loop, model-keyed terminal failure scripts,
    and authentication evidence in the request log."""

    @staticmethod
    def stop(process):
        if process.poll() is None:
            process.kill()
        process.communicate(timeout=10)

    @staticmethod
    def start(request_log: Path):
        import socket

        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        process = subprocess.Popen(
            [sys.executable, str(SCRIPT),
             "--listen", f"127.0.0.1:{port}",
             "--request-log", str(request_log)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        readiness = process.stdout.readline().strip()
        if not readiness.startswith("listening "):
            ScriptedProviderProtocolMatrixTest.stop(process)
            raise AssertionError(f"no readiness line: {readiness!r}")
        host, ready_port = readiness.split(" ", 1)[1].rsplit(":", 1)
        return process, f"http://{host}:{ready_port}"

    def post(self, base, body, path, headers=None):
        import urllib.request

        request = urllib.request.Request(
            base + path, data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json", **(headers or {})},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=10) as response:
                return response.status, response.read().decode("utf-8"), dict(response.headers)
        except urllib.error.HTTPError as error:
            return error.code, error.read().decode("utf-8"), dict(error.headers)

    @staticmethod
    def anthropic_body(stream: bool, tool_result: bool) -> dict:
        content = (
            [{"type": "tool_result", "tool_use_id": "toolu_scripted_0001",
              "content": "scripted-integration-result"}]
            if tool_result
            else "solve the pinned task"
        )
        return {
            "model": "scripted/eval",
            "max_tokens": 512,
            "stream": stream,
            "messages": [{"role": "user", "content": content}],
        }

    @staticmethod
    def responses_body(stream: bool, tool_result: bool) -> dict:
        items = (
            [{"type": "function_call", "call_id": "call-scripted-0001",
              "name": "bash", "arguments": "{}"},
             {"type": "function_call_output", "call_id": "call-scripted-0001",
              "output": "scripted-integration-result"}]
            if tool_result
            else [{"role": "user",
                   "content": [{"type": "input_text",
                                "text": "solve the pinned task"}]}]
        )
        return {"model": "scripted/eval", "stream": stream, "input": items}

    def test_anthropic_stream_tool_loop_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                first = self.post(
                    base, self.anthropic_body(True, False), "/v1/messages")
                repeat = self.post(
                    base, self.anthropic_body(True, False), "/v1/messages")
                final = self.post(
                    base, self.anthropic_body(True, True), "/v1/messages")
            finally:
                self.stop(process)
        status, body, _ = first
        self.assertEqual(status, 200)
        self.assertEqual(body, repeat[1])
        events = [
            json.loads(line.removeprefix("data: "))
            for line in body.splitlines()
            if line.startswith("data: ") and line != "data: [DONE]"
        ]
        types = [event["type"] for event in events]
        self.assertEqual(types[0], "message_start")
        self.assertIn("content_block_start", types)
        self.assertIn("content_block_delta", types)
        self.assertIn("message_delta", types)
        self.assertEqual(types[-1], "message_stop")
        tool_use = [
            event for event in events
            if event["type"] == "content_block_start"
            and event["content_block"]["type"] == "tool_use"
        ]
        self.assertEqual(len(tool_use), 1)
        self.assertEqual(tool_use[0]["content_block"]["name"], "bash")
        arguments = json.loads(tool_use[0]["content_block"]["input"]["command_extra"]
                               ) if False else tool_use[0]["content_block"]["input"]
        self.assertEqual(
            arguments,
            {"command": "printf 'scripted-integration-result\\n' > answer.txt"},
        )
        stop = [event for event in events if event["type"] == "message_delta"]
        self.assertEqual(stop[0]["delta"]["stop_reason"], "tool_use")
        # The tool-result turn ends the loop deterministically.
        self.assertEqual(final[0], 200)
        self.assertIn('"stop_reason":"end_turn"', final[1].replace(" ", ""))

    def test_anthropic_non_stream_shapes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                first = self.post(
                    base, self.anthropic_body(False, False), "/v1/messages")
                final = self.post(
                    base, self.anthropic_body(False, True), "/v1/messages")
            finally:
                self.stop(process)
        self.assertEqual(first[0], 200)
        message = json.loads(first[1])
        self.assertEqual(message["stop_reason"], "tool_use")
        self.assertEqual(
            message["content"][0],
            {"type": "tool_use", "id": "toolu_scripted_0001", "name": "bash",
             "input": {"command":
                       "printf 'scripted-integration-result\\n' > answer.txt"}},
        )
        self.assertEqual(final[0], 200)
        self.assertEqual(json.loads(final[1])["stop_reason"], "end_turn")

    def test_anthropic_beta_query_serves_the_same_tool_loop(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start(log)
            try:
                for stream in (False, True):
                    for tool_result in (False, True):
                        with self.subTest(stream=stream, tool_result=tool_result):
                            body = self.anthropic_body(stream, tool_result)
                            stable = self.post(base, body, "/v1/messages")
                            beta = self.post(base, body, "/v1/messages?beta=true")
                            self.assertEqual(beta[0], 200)
                            self.assertEqual(beta[1], stable[1])
                            self.assertEqual(beta[2]["Content-Type"],
                                             stable[2]["Content-Type"])
            finally:
                self.stop(process)
            lines = [json.loads(line) for line in log.read_text().splitlines()]
        self.assertEqual([line["path"] for line in lines],
                         ["/v1/messages", "/v1/messages?beta=true"] * 4)

    def test_anthropic_beta_query_does_not_admit_unknown_routes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                for path in ("/v1/messages?beta=false", "/v1/messages?other=true",
                             "/v1/messages?beta=true&other=true",
                             "/v1/v1/messages?beta=true", "/v1/other?beta=true",
                             "/v1/chat/completions?beta=true"):
                    with self.subTest(path=path):
                        status, _, _ = self.post(
                            base, self.anthropic_body(False, False), path)
                        self.assertEqual(status, 404)
            finally:
                self.stop(process)

    def test_openai_responses_stream_tool_loop_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                first = self.post(
                    base, self.responses_body(True, False), "/v1/responses")
                repeat = self.post(
                    base, self.responses_body(True, False), "/v1/responses")
                final = self.post(
                    base, self.responses_body(True, True), "/v1/responses")
            finally:
                self.stop(process)
        status, body, _ = first
        self.assertEqual(status, 200)
        self.assertEqual(body, repeat[1])
        events = [
            json.loads(line.removeprefix("data: "))
            for line in body.splitlines()
            if line.startswith("data: ") and line != "data: [DONE]"
        ]
        types = [event["type"] for event in events]
        self.assertIn("response.created", types)
        self.assertIn("response.output_item.added", types)
        self.assertIn("response.function_call_arguments.delta", types)
        self.assertIn("response.output_item.done", types)
        self.assertEqual(types[-1], "response.completed")
        call = [
            event for event in events
            if event["type"] == "response.output_item.added"
            and event["item"]["type"] == "function_call"
        ]
        self.assertEqual(call[0]["item"]["name"], "bash")
        done = [
            event for event in events
            if event["type"] == "response.output_item.done"
            and event["item"]["type"] == "function_call"
        ]
        self.assertEqual(
            json.loads(done[0]["item"]["arguments"]),
            {"command": "printf 'scripted-integration-result\\n' > answer.txt"},
        )
        self.assertEqual(final[0], 200)
        self.assertIn("response.completed", final[1])

    def test_openai_responses_non_stream_shapes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                first = self.post(
                    base, self.responses_body(False, False), "/v1/responses")
                final = self.post(
                    base, self.responses_body(False, True), "/v1/responses")
            finally:
                self.stop(process)
        self.assertEqual(first[0], 200)
        response = json.loads(first[1])
        self.assertEqual(response["status"], "completed")
        self.assertEqual(response["output"][0]["type"], "function_call")
        self.assertEqual(response["output"][0]["name"], "bash")
        self.assertEqual(final[0], 200)
        self.assertEqual(json.loads(final[1])["status"], "completed")

    def test_model_keyed_failure_scripts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            process, base = self.start(Path(tmp) / "requests.jsonl")
            try:
                auth = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/auth-fail"}, "/v1/messages")
                limit = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/rate-limit"}, "/v1/messages")
                limit_again = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/rate-limit"}, "/v1/messages")
                retry_first = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/retry"}, "/v1/messages")
                retry_second = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/retry"}, "/v1/messages")
                broken = self.post(
                    base, self.anthropic_body(False, False)
                    | {"model": "scripted/protocol-error"}, "/v1/messages")
            finally:
                self.stop(process)
        # Terminal authentication failure: 401, typed error body.
        self.assertEqual(auth[0], 401)
        self.assertIn("authentication", auth[1])
        # Exhausted rate limiting: 429 on every request, never a success.
        self.assertEqual(limit[0], 429)
        self.assertEqual(limit_again[0], 429)
        self.assertIn("rate_limit", limit[1])
        # Retryable 429: exactly one failure, then the deterministic
        # tool-loop turn succeeds.
        self.assertEqual(retry_first[0], 429)
        self.assertIn("Retry-After", retry_first[2])
        self.assertEqual(retry_second[0], 200)
        # Native protocol error: HTTP 200 with a body the protocol cannot
        # parse (the failure belongs to the client's typed observations).
        self.assertEqual(broken[0], 200)
        with self.assertRaises(json.JSONDecodeError):
            json.loads(broken[1])

    def test_request_log_records_auth_and_model_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start(log)
            try:
                self.post(base, self.anthropic_body(False, False), "/v1/messages",
                          headers={"x-api-key": "dummy-scripted-credential"})
                self.post(base, self.chat_body(False),
                          "/v1/chat/completions",
                          headers={"Authorization": "Bearer dummy"})
            finally:
                self.stop(process)
            log_text = log.read_text()
            lines = [json.loads(line) for line in log_text.splitlines()]
        self.assertEqual(len(lines), 2)
        by_path = {line["path"]: line for line in lines}
        self.assertEqual(by_path["/v1/messages"]["auth"], "x-api-key")
        self.assertEqual(by_path["/v1/messages"]["model"], "scripted/eval")
        self.assertEqual(by_path["/v1/chat/completions"]["auth"], "bearer")
        # The credential value itself never enters the log.
        self.assertNotIn("dummy-scripted-credential", log_text)

    @staticmethod
    def chat_body(stream: bool) -> dict:
        return {
            "model": "scripted/eval",
            "stream": stream,
            "messages": [{"role": "user", "content": "solve the pinned task"}],
        }


if __name__ == "__main__":
    unittest.main()

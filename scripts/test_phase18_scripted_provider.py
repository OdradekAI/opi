#!/usr/bin/env python3
"""Tests for the Phase 18 scripted provider fixture (tasks 18.10.1, 18.14.1).

Runs the fixture as a real subprocess and asserts exact deterministic
behavior. The stdin/stdout fixture mode (18.10.1) keeps its byte-identical
contract: fixed response lines, EOF termination, typed rejections for
malformed requests, and the bounded request cap. The listener mode
(18.14.1) binds exactly one declared endpoint and serves deterministic
OpenAI-compatible Chat Completions responses with a normalized request log.
This is the smoke-addendum gate (``python
scripts/test_phase18_scripted_provider.py``).
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("phase18-scripted-provider.py")
EXPECTED_LINE = (
    '{"content":"scripted-provider: acknowledged","schema":"phase18-scripted-provider/1"}'
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
    """Listener mode (task 18.14.1): one declared endpoint, deterministic
    OpenAI-compatible Chat Completions serving, normalized request log."""

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
            process.kill()
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
            "model": "scripted/phase18",
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
                process.kill()
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
        self.assertEqual(choices[2]["delta"], {})
        self.assertEqual(choices[2]["finish_reason"], "tool_calls")

    def test_non_stream_turn_is_deterministic_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "requests.jsonl"
            process, base = self.start_listener(log)
            try:
                status, body = self.post(base, self.chat_body(stream=False))
            finally:
                process.kill()
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
                process.kill()
            log_text = log.read_text(encoding="utf-8")
        lines = [json.loads(line) for line in log_text.splitlines() if line]
        self.assertEqual(len(lines), 2)
        self.assertEqual(lines[0]["request_sha256"], lines[1]["request_sha256"])
        self.assertNotIn(secret_prompt, log_text)
        self.assertEqual(lines[0]["schema"], "phase18-scripted-provider-log/1")
        self.assertEqual(lines[0]["response_script"], "scripted-turn-plan/1")

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
                process.kill()
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
                process.kill()
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


if __name__ == "__main__":
    unittest.main()

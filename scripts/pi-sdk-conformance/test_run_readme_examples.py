"""Offline subprocess admission tests for same-source packaged onboarding proof."""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("run_readme_examples.py")

# A separate interpreter exercises the same report/source admission called by
# the runner. It never starts Cargo, an SDK process or a provider call.
PROBE = """
import importlib.util
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(sys.argv[1]).parent))
spec = importlib.util.spec_from_file_location('readme_proof', sys.argv[1])
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
if sys.argv[2] == 'execution':
    runner.parse_execution(Path(sys.argv[3]).read_text(encoding='utf-8'))
elif sys.argv[2] == 'sources':
    runner.check_packaged_sources(Path(sys.argv[3]), Path(sys.argv[4]))
elif sys.argv[2] == 'environment':
    original = json.loads(Path(sys.argv[3]).read_text(encoding='utf-8'))
    controlled, evidence = runner.execution_environment(original)
    assert original == json.loads(Path(sys.argv[3]).read_text(encoding='utf-8'))
    print(json.dumps({'controlled': controlled, 'evidence': evidence}))
else:
    raise AssertionError('unknown test probe')
"""


def passing_report() -> dict:
    idle = {"source_reads": 0, "store_reads": 0, "requests": 0}
    checks = []
    for name in ("quick_start", "quick_start_stream", "tool_loop"):
        request = {"method": "POST", "target": "/v1/messages",
                   "headers": {"x-api-key": "readme-synthetic-key"},
                   "body": {"model": "claude-sonnet-4-5", "messages": []}}
        requests = [copy.deepcopy(request)]
        if name == "tool_loop":
            second = copy.deepcopy(request)
            second["body"]["messages"] = [
                {"role": "assistant", "content": [{"type": "tool_use", "id": "weather-1",
                  "name": "get_weather", "input": {"city": "Tokyo"}}]},
                {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "weather-1",
                  "content": "22 C, clear"}]},
            ]
            requests.append(second)
        checks.append({"name": name, "passed": True, "setup": dict(idle), "unpolled": dict(idle),
                       "observed": {"source_reads": 3, "store_reads": 1, "requests": len(requests)},
                       "final_text": "Tokyo: 22 C, clear." if name == "tool_loop" else "Hello world",
                       "continuation": name == "tool_loop", "requests": requests})
    return {"schema": "opi-readme-execution/1", "verdict": "PASS", "failed": 0,
            "total": 3, "checks": checks}


class ReadmeExecutionAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="opi-readme-runner-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def probe(self, mode: str, *arguments: Path) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-c", PROBE, str(SCRIPT), mode,
                               *(str(argument) for argument in arguments)],
                              cwd=self.root, env=dict(os.environ), capture_output=True,
                              text=True, encoding="utf-8", errors="replace", timeout=30)

    def execution(self, report: dict | None, prefix: str = "OPI_README_PROOF="):
        path = self.root / "execution.stdout"
        stdout = "example business output\n"
        if report is not None:
            stdout += prefix + json.dumps(report) + "\n"
        path.write_text(stdout, encoding="utf-8")
        return self.probe("execution", path)

    def assert_rejected(self, result: subprocess.CompletedProcess, reason: str):
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(reason, result.stderr)

    def test_complete_three_flow_report_is_accepted(self):
        result = self.execution(passing_report())
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_compilation_without_execution_report_is_rejected(self):
        self.assert_rejected(self.execution(None), "exactly one executed onboarding report")

    def test_missing_named_flow_is_rejected(self):
        report = passing_report()
        report["checks"].pop()
        report["total"] = 2
        self.assert_rejected(self.execution(report), "exactly the three named onboarding flows")

    def test_missing_tool_second_request_is_rejected(self):
        report = passing_report()
        case = report["checks"][2]
        case["requests"].pop()
        case["observed"]["requests"] = 1
        self.assert_rejected(self.execution(report), "lacks actual auth, sends and expected final outcome")

    def test_claimed_continuation_without_tool_result_is_rejected(self):
        report = passing_report()
        report["checks"][2]["requests"][1]["body"]["messages"].pop()
        self.assert_rejected(self.execution(report), "second request lost the admitted call or tool result")

    def test_source_access_before_poll_is_rejected(self):
        report = passing_report()
        report["checks"][0]["unpolled"]["source_reads"] = 1
        self.assert_rejected(self.execution(report), "zero-I/O setup and actual unpolled-call proof")

    def test_success_without_expected_business_output_is_rejected(self):
        report = passing_report()
        report["checks"][1]["final_text"] = ""
        self.assert_rejected(self.execution(report), "expected final outcome")

    def test_execution_proxy_isolation_preserves_host_and_records_no_proxy_values(self):
        original = {"HTTP_PROXY": "http://user:secret@proxy.invalid", "https_proxy": "https://secret.invalid",
                    "aLl_PrOxY": "socks5://secret.invalid", "No_PrOxY": "private.internal", "SAFE": "kept"}
        path = self.root / "environment.json"
        path.write_text(json.dumps(original), encoding="utf-8")
        result = self.probe("environment", path)
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertEqual(output["controlled"], {"SAFE": "kept", "NO_PROXY": "localhost,127.0.0.1,::1",
                                                "no_proxy": "localhost,127.0.0.1,::1"})
        self.assertEqual(output["evidence"]["removed_proxy_variable_names"], sorted(set(original) - {"SAFE"}))
        self.assertNotIn("secret", json.dumps(output["evidence"]))
        self.assertNotIn("private.internal", json.dumps(output["evidence"]))
        self.assertEqual(json.loads(path.read_text(encoding="utf-8")), original)

    def source_fixture(self) -> tuple[Path, Path]:
        repo = self.root / "repo"
        package = self.root / "package"
        (repo / "crates/opi-ai/examples").mkdir(parents=True)
        (package / "examples").mkdir(parents=True)
        examples = []
        for name in ("quick_start.rs", "quick_start_stream.rs", "tool_loop.rs"):
            source = f"// exact packaged {name}\nfn main() {{}}\n"
            examples.append(source)
            (repo / "crates/opi-ai/examples" / name).write_text(source, encoding="utf-8")
            (package / "examples" / name).write_text(source, encoding="utf-8")
        for name in ("README.md", "README.zh.md"):
            (package / name).write_text("\n".join(examples), encoding="utf-8")
        return repo, package

    def test_exact_sources_and_both_embeds_are_accepted(self):
        repo, package = self.source_fixture()
        result = self.probe("sources", repo, package)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_mismatched_packaged_source_is_rejected(self):
        repo, package = self.source_fixture()
        (package / "examples/tool_loop.rs").write_text("fn main() {}\n", encoding="utf-8")
        self.assert_rejected(self.probe("sources", repo, package),
                             "packaged example tool_loop.rs differs from the workspace source")

    def test_missing_localized_embed_is_rejected(self):
        repo, package = self.source_fixture()
        (package / "README.zh.md").write_text("missing examples\n", encoding="utf-8")
        self.assert_rejected(self.probe("sources", repo, package), "README.zh.md does not embed")

    def test_cli_requires_explicit_offline_marker(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "--scratch", str(self.root / "scratch"),
                                 "--target-dir", str(self.root / "target")], cwd=self.root,
                                env=dict(os.environ), capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=30)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("--offline is required", result.stderr)


if __name__ == "__main__":
    unittest.main()

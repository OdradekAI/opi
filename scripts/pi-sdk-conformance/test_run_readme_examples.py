"""Offline subprocess admission tests for same-source packaged onboarding proof."""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib
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
        checks.append({"name": name, "passed": True, "setup": dict(idle), "before_consumption": {"source_reads": 3, "store_reads": 1, "requests": 1},
                       "observed": {"source_reads": 3, "store_reads": 1, "requests": len(requests)},
                       "final_text": "Tokyo: 22 C, clear." if name == "tool_loop" else "Hello world",
                       "continuation": name == "tool_loop", "requests": requests})
    return {"schema": "opi-readme-execution/2", "verdict": "PASS", "failed": 0,
            "total": 3, "checks": checks}


class ReadmeExecutionAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="opi-readme-runner-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_consumer_template_declares_both_opi_ai_feature_profiles(self):
        manifest = (SCRIPT.parent / "consumer-readme" / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('opi-ai = { path = "@PACKAGE@", default-features = false, features = ["builtins"] }', manifest)
        self.assertIn('[features]\ncaller-default-opi-ai = ["opi-ai/default"]', manifest)

    def test_packaged_onboarding_sources_use_consumer_commands(self):
        examples = Path(__file__).parents[2] / "crates" / "opi-ai" / "examples"
        for name in ("quick_start", "quick_start_stream", "tool_loop"):
            source = (examples / f"{name}.rs").read_text(encoding="utf-8")
            self.assertNotIn("-p opi-ai", source)
            self.assertIn(f"cargo check --example {name}", source)
            self.assertIn(f"cargo run --example {name}", source)

    def test_readmes_publish_standalone_consumer_manifest(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for name in ("README.md", "README.zh.md"):
            source = (root / name).read_text(encoding="utf-8")
            manifest = re.search(r"^```toml\n(\[dependencies\].*?)\n```$", source, re.M | re.S)
            self.assertIsNotNone(manifest, name)
            deps = tomllib.loads(manifest[1])["dependencies"]
            self.assertEqual(deps["opi-ai"], {"path": "../opi/crates/opi-ai"})
            self.assertEqual(set(deps), {"opi-ai", "tokio"})
            self.assertIn("macros", deps["tokio"]["features"])
            self.assertIn("rt-multi-thread", deps["tokio"]["features"])
            for extra in ('futures-util = "0.3"', 'serde_json = "1"', 'tokio-util = "0.7"'):
                self.assertIn(extra, source, name)

    def test_readmes_cover_runnable_sdk_workflows(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for name in ("README.md", "README.zh.md"):
            source = (root / name).read_text(encoding="utf-8")
            rust = "\n".join(re.findall(r"^```rust\n(.+?)\n```$", source, re.M | re.S))
            for call in ("register_builtin(", ".complete_simple(", ".stream_simple(",
                         ".get_all_models(", "ToolDef {", ".generate_images(", ".classify("):
                self.assertIn(call, rust, f"{name} omits a Rust example using {call}")

    def test_public_readmes_do_not_expose_private_implementation_sections(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        forbidden = (
            "Private protocol ownership",
            "Compression and maintenance boundaries",
            "Private Schema Dialect Engine",
            "Private Upstream Crypto Profile",
            "Scripted Multi-Operation MockProvider Support",
            "私有协议所有权",
            "压缩与维护边界",
            "私有 Schema 方言引擎",
            "私有上游 Crypto Profile",
            "MockProvider 多操作脚本测试支撑",
        )
        links = {
            "README.md": (
                "docs/transport.md",
                "docs/utilities.md",
                "docs/providers.md",
                "docs/testing.md",
            ),
            "README.zh.md": (
                "docs/transport.zh.md",
                "docs/utilities.zh.md",
                "docs/providers.zh.md",
                "docs/testing.zh.md",
            ),
        }
        for name, required_links in links.items():
            source = (root / name).read_text(encoding="utf-8")
            for heading in forbidden:
                self.assertNotIn(heading, source, f"{name} exposes private section: {heading}")
            for link in required_links:
                self.assertIn(link, source, f"{name} omits topic reference: {link}")

    def test_readmes_keep_license_after_reference_appendices(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for name, heading in (("README.md", "## License"), ("README.zh.md", "## 许可证")):
            source = (root / name).read_text(encoding="utf-8")
            self.assertGreater(source.rfind(heading), source.rfind("### "),
                               f"{name} puts reference appendices under the license heading")

    def test_readmes_put_installation_before_capability_context(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for name, installation, next_section in (
            ("README.md", "## Installation", "## Quick start"),
            ("README.zh.md", "## 安装", "## 快速开始"),
        ):
            source = (root / name).read_text(encoding="utf-8")
            self.assertLess(source.index(installation), source.index(next_section),
                            f"{name} puts onboarding after the first capability")

    def test_readmes_cover_consumer_feature_profiles(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for name in ("README.md", "README.zh.md"):
            source = (root / name).read_text(encoding="utf-8")
            for features in ('default-features = false, features = ["builtins"]',
                             'default-features = false, features = ["catalog-bundled"]',
                             '`default-features = false`'):
                self.assertIn(features, source, name)

    def test_readmes_split_cost_pricing_into_package_local_topic_references(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        english = (root / "README.md").read_text(encoding="utf-8")
        chinese = (root / "README.zh.md").read_text(encoding="utf-8")
        english_topic = (root / "docs" / "cost-pricing.md").read_text(encoding="utf-8")
        chinese_topic = (root / "docs" / "cost-pricing.zh.md").read_text(encoding="utf-8")
        self.assertIn("](docs/cost-pricing.md)", english)
        self.assertIn("](docs/cost-pricing.zh.md)", chinese)
        for fragment in ("calculate_cost_tiered", "total_from_provider", "PricingTier::input_tokens_above"):
            self.assertIn(fragment, english_topic)
            self.assertIn(fragment, chinese_topic)

    def test_readmes_link_provider_auth_reference(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        for suffix in ("", ".zh"):
            readme = (root / f"README{suffix}.md").read_text(encoding="utf-8")
            self.assertIn(f"](docs/provider-auth{suffix}.md)", readme)

    def test_public_opi_ai_manuals_omit_internal_alignment_labels(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai"
        forbidden = re.compile(
            r"(?:\b(?:AIR|RDF|DEL|ONB)-[A-Z0-9]+(?:/[A-Z0-9]+)*\b|"
            r"\bP20(?:-[A-Z0-9]+)+\b|\bV-\d+\b|"
            r"\b(?:task|Task)\s+20(?:\.\d+)+(?:[–-]\d+)?\b|"
            r"任务\s*20(?:\.\d+)+(?:[–-]\d+)?|Phase\s+20|phase20|"
            r"\.repo/pi-|\b20\.\d+(?:[–-]\d+)?\b)",
            re.IGNORECASE,
        )
        for path in (
            root / "README.md",
            root / "README.zh.md",
            root / "docs" / "cost-pricing.md",
            root / "docs" / "cost-pricing.zh.md",
            root / "docs" / "provider-auth.md",
            root / "docs" / "provider-auth.zh.md",
            root / "docs" / "transport.md",
            root / "docs" / "transport.zh.md",
            root / "docs" / "utilities.md",
            root / "docs" / "utilities.zh.md",
            root / "docs" / "providers.md",
            root / "docs" / "providers.zh.md",
            root / "docs" / "testing.md",
            root / "docs" / "testing.zh.md",
            root / "examples" / "quick_start.rs",
            root / "examples" / "quick_start_stream.rs",
            root / "examples" / "tool_loop.rs",
        ):
            source = path.read_text(encoding="utf-8")
            match = forbidden.search(source)
            self.assertIsNone(
                match,
                f"{path.name} exposes an internal alignment label: {match.group(0)!r}" if match else "",
            )

    def test_public_topic_references_expose_current_contracts(self):
        root = Path(__file__).parents[2] / "crates" / "opi-ai" / "docs"
        required = {
            "provider-auth": (
                "check_auth", "LoginPresenter", "Catalog::builtin()", "Models::refresh", "allow_network",
            ),
            "transport": (
                "proxy_from_env", "TlsOptions", "cleanup_session_resources",
                "fetch_deferred_ready", "cancel_deferred",
            ),
            "utilities": (
                "validate_tool_call", "TranscriptContext", "ReplayProjection::Strict", "Request::callbacks",
            ),
            "providers": (
                "Models::register_chat_api", "Models::classifier_model", "ClassifierAnswer", "Models::image_model",
            ),
            "testing": ("test_support::MockProvider", "single_route_collection", "](../tests/README"),
        }
        for topic, fragments in required.items():
            for suffix in ("", ".zh"):
                path = root / f"{topic}{suffix}.md"
                source = path.read_text(encoding="utf-8")
                for fragment in fragments:
                    self.assertIn(fragment, source, f"{path.name} omits public entry point: {fragment}")

    def test_telemetry_readmes_describe_current_ai_integration_boundary(self):
        root = Path(__file__).parents[2] / "crates" / "opi-telemetry"
        english = (root / "README.md").read_text(encoding="utf-8")
        chinese = (root / "README.zh.md").read_text(encoding="utf-8")
        for fragment in (
            "ordinary `Models` chat installs an AI-owned",
            "`pi.ai.request`",
            "deferred fetch installs `pi.ai.poll`",
            "Raw `ChatStream` and prepared raw attempts remain manual",
            "AI schema, token, and cost rules remain in `opi-ai`",
        ):
            self.assertIn(fragment, english, f"opi-telemetry README omits AI boundary: {fragment}")
        for fragment in (
            "普通 `Models` chat",
            "会安装由 AI 拥有的 `pi.ai.request`",
            "deferred fetch 会安装 `pi.ai.poll`",
            "raw `ChatStream` 与 prepared raw attempt 仍是手动边界",
            "AI schema、token 和 cost 规则仍由 `opi-ai` 拥有",
        ):
            self.assertIn(fragment, chinese, f"opi-telemetry README.zh.md omits AI boundary: {fragment}")

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

    def test_source_access_during_setup_is_rejected(self):
        report = passing_report()
        report["checks"][0]["setup"]["source_reads"] = 1
        self.assert_rejected(self.execution(report), "zero-I/O setup")

    def test_missing_execution_before_consumption_is_rejected(self):
        report = passing_report()
        report["checks"][0]["before_consumption"]["requests"] = 0
        self.assert_rejected(self.execution(report), "execution before consumption")

    def test_legacy_lazy_evidence_is_rejected(self):
        report = passing_report()
        report["schema"] = "opi-readme-execution/1"
        self.assert_rejected(self.execution(report), "exactly the three named onboarding flows")

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
        links = []
        for name in ("quick_start.rs", "quick_start_stream.rs", "tool_loop.rs"):
            source = f"// exact packaged {name}\nfn main() {{}}\n"
            links.append(f"[{name}](examples/{name})")
            (repo / "crates/opi-ai/examples" / name).write_text(source, encoding="utf-8")
            (package / "examples" / name).write_text(source, encoding="utf-8")
        for name in ("README.md", "README.zh.md"):
            (package / name).write_text("\n".join(links) + "\n```rust\nfn main() {}\n```\n", encoding="utf-8")
        return repo, package

    def test_exact_sources_and_matching_handbooks_are_accepted(self):
        repo, package = self.source_fixture()
        result = self.probe("sources", repo, package)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_mismatched_packaged_source_is_rejected(self):
        repo, package = self.source_fixture()
        (package / "examples/tool_loop.rs").write_text("fn main() {}\n", encoding="utf-8")
        self.assert_rejected(self.probe("sources", repo, package),
                             "packaged example tool_loop.rs differs from the workspace source")

    def test_missing_localized_example_link_is_rejected(self):
        repo, package = self.source_fixture()
        (package / "README.zh.md").write_text("missing examples\n", encoding="utf-8")
        self.assert_rejected(self.probe("sources", repo, package), "README.zh.md does not link")

    def test_different_localized_rust_is_rejected(self):
        repo, package = self.source_fixture()
        path = package / "README.zh.md"
        path.write_text(path.read_text(encoding="utf-8").replace("main()", "different()"),
                        encoding="utf-8")
        self.assert_rejected(self.probe("sources", repo, package), "Rust examples differ")

    def test_cli_requires_explicit_offline_marker(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "--scratch", str(self.root / "scratch"),
                                 "--target-dir", str(self.root / "target")], cwd=self.root,
                                env=dict(os.environ), capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=30)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("--offline is required", result.stderr)

    def test_new_evidence_output_cannot_overwrite_history_or_start_work(self):
        existing = self.root / "historical-evidence.json"
        existing.write_bytes(b"historical evidence")
        scratch = self.root / "not-created"
        result = subprocess.run([sys.executable, str(SCRIPT), "--offline", "--scratch", str(scratch),
                                 "--target-dir", str(self.root / "missing-target"),
                                 "--evidence-output", str(existing)], cwd=self.root,
                                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("--evidence-output already exists", result.stderr)
        self.assertEqual(existing.read_bytes(), b"historical evidence")
        self.assertFalse(scratch.exists())


if __name__ == "__main__":
    unittest.main()

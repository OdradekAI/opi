import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import check_architecture_boundaries as checker


def cli_fixture(repo):
    harness = repo / "scripts/pi-sdk-conformance"
    harness.mkdir(parents=True)
    for name in ("check_architecture_boundaries.py", "profile_api_edges.py", "rust_source.py",
                 "lower_layer_edges.py"):
        shutil.copyfile(Path(checker.__file__).with_name(name), harness / name)
    (harness / "lower_layer_baseline.json").write_text("[]", encoding="utf-8")
    src = repo / "crates/opi-ai/src"
    (src / "providers").mkdir(parents=True)
    (src / "api").mkdir()
    for path in ("api/contract.rs", "provider.rs", "stream.rs", "execution.rs", "models.rs"):
        (src / path).touch()
    for name in checker.PROFILE_FILES:
        (src / "providers" / name).touch()
    return harness / "check_architecture_boundaries.py", src


class ArchitectureBoundaryTests(unittest.TestCase):
    def test_code_lines_ignores_rustdoc_and_line_comments(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.rs"
            path.write_text(
                "//! crate::api::contract\n// crate::api::contract\nuse crate::api::contract;\n",
                encoding="utf-8",
            )
            self.assertEqual(checker.code_lines(path), [(3, "use crate::api::contract;")])

    def test_report_exposes_measurable_current_edges(self):
        report = checker.architecture_report()
        self.assertIn("counts", report)
        self.assertIn("violations", {"violations": checker.violations(report)})
        self.assertEqual(report["counts"]["api_provider_edges"], 0)
        self.assertGreaterEqual(report["counts"]["api_route_context_edges"], 1)
        self.assertEqual(report["counts"]["api_contract_provider_edges"], 0)
        lower = report["lower_layer_dependencies"]
        self.assertTrue(lower["reviewed_contracts"])
        self.assertTrue(lower["known_exceptions"])
        self.assertTrue(lower["excluded_tests"])
        self.assertFalse(lower["new_dependencies"])
        self.assertFalse(lower["unused_baseline"])
        self.assertFalse(lower["provider_copilot_policy"])
        self.assertFalse(report["stream_execution_edges"]["stream_imports_execution"])
        self.assertTrue(report["stream_execution_edges"]["stream_reexports_execution"])
        self.assertTrue(report["stream_execution_edges"]["execution_imports_stream"])
        self.assertEqual(report["counts"]["profile_native_state"], 0)
        self.assertGreaterEqual(report["counts"]["models_owner_modules"], 1)
        self.assertEqual(report["counts"]["models_owner_module_files"], 4)
        self.assertEqual(report["counts"]["models_owner_definitions_in_facade"], 0)
        self.assertEqual(report["counts"]["models_prepared_execution_owner"], 1)
        self.assertEqual(report["counts"]["models_prepared_execution_owner_files"], 1)
        self.assertEqual(report["counts"]["models_registration_records"], 5)
        self.assertEqual(report["counts"]["models_registration_record_files"], 1)
        self.assertEqual(report["counts"]["models_registration_records_in_facade"], 0)
        self.assertEqual(report["counts"]["models_chat_call_orchestration"], 8)
        self.assertEqual(report["counts"]["models_chat_call_files"], 1)
        self.assertEqual(report["counts"]["models_chat_call_orchestration_in_facade"], 0)
        self.assertLess(report["models_lines"], 4000)

    def test_enforce_is_green_after_current_edges_are_closed(self):
        report = checker.architecture_report()
        self.assertFalse(checker.violations(report))

    def test_enforce_reports_unapproved_profile_import_at_its_source(self):
        with tempfile.TemporaryDirectory() as directory:
            script, src = cli_fixture(Path(directory))
            (src / "providers/gemini.rs").write_text(
                "//! See crate::api::openai_completions.\nuse crate::{api::openai_completions::OpenAiCompletionsOptions};\n",
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(script), "--enforce", "--json"],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["violations"], [
                "provider profiles have unapproved API dependencies (see --json for sites)",
            ])
            self.assertEqual(report["counts"]["unapproved_profile_api_edges"], 1)
            edge, = report["profile_api_edges"]
            self.assertEqual(edge["file"], "crates/opi-ai/src/providers/gemini.rs")
            self.assertEqual(edge["line"], 2)
            self.assertEqual(edge["path"], "crate::api::openai_completions::OpenAiCompletionsOptions")

    def test_enforce_rejects_new_lower_edges_but_excludes_test_modules(self):
        with tempfile.TemporaryDirectory() as directory:
            script, src = cli_fixture(Path(directory))
            (src / "lib.rs").write_text("pub use models::ChatModelRecord;", encoding="utf-8")
            (src / "stream.rs").write_text(
                "//! crate::api::Ignored\n"
                "#[cfg(test)] mod probes;\n"
                "pub use crate::ChatModelRecord as Exposed;\n",
                encoding="utf-8",
            )
            (src / "stream").mkdir()
            (src / "stream/probes.rs").write_text(
                "use crate::api::Fixture;", encoding="utf-8",
            )
            (src / "provider").mkdir()
            (src / "provider.rs").write_text("mod routes;", encoding="utf-8")
            (src / "provider/routes.rs").write_text(
                "mod inner {\n"
                "    use crate::{providers::{openai as native}};\n"
                "    type Other = super::super::super::api::OpenAiOptions;\n"
                "}\n",
                encoding="utf-8",
            )
            result = subprocess.run([sys.executable, str(script), "--enforce", "--json"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["violations"], [
                "lower owners have new guarded dependencies (see --json for sites)",
            ])
            lower = report["lower_layer_dependencies"]
            self.assertEqual(
                [(edge["file"].split("/src/")[1], edge["line"], edge["kind"], edge["path"])
                 for edge in lower["new_dependencies"]],
                [
                    ("provider/routes.rs", 2, "use", "crate::providers::openai"),
                    ("provider/routes.rs", 3, "qualified", "crate::api::OpenAiOptions"),
                    ("stream.rs", 3, "pub use", "crate::models::ChatModelRecord"),
                ],
            )
            self.assertEqual([edge["path"] for edge in lower["excluded_tests"]], ["crate::api::Fixture"])

    def test_lower_baseline_caps_each_symbol_and_reports_removed_sites(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            script, src = cli_fixture(repo)
            (script.parent / "lower_layer_baseline.json").write_text(json.dumps([{
                "file": "stream.rs", "feature": "minimum", "role": "exception",
                "reason": "Projection facts captured at setup",
                "sites": [["qualified", "crate::transcript::ProjectionFacts", 1],
                          ["use", "crate::transcript::Removed", 1]],
            }]), encoding="utf-8")
            (src / "stream.rs").write_text(
                "type A = crate::transcript::ProjectionFacts;\n"
                "type B = crate::transcript::ProjectionFacts;\n"
                "#[cfg(all(test, feature = \"wire\"))] use crate::api::TestOnly;\n"
                "#[cfg(any(test, feature = \"wire\"))] use crate::api::AlsoProduction;\n",
                encoding="utf-8",
            )
            lower = checker.lower_layer_report(repo)
            self.assertEqual([edge["line"] for edge in lower["known_exceptions"]], [1])
            self.assertEqual([edge["line"] for edge in lower["new_dependencies"]], [2, 4])
            self.assertEqual([edge["line"] for edge in lower["excluded_tests"]], [3])
            [unused] = lower["unused_baseline"]
            self.assertEqual(unused["path"], "crate::transcript::Removed")
            self.assertEqual(unused["unused_sites"], 1)

    def test_enforce_rejects_copilot_policy_in_the_lower_provider(self):
        with tempfile.TemporaryDirectory() as directory:
            script, src = cli_fixture(Path(directory))
            (src / "provider").mkdir()
            (src / "provider.rs").write_text(
                "const GITHUB_COPILOT_MANAGED_HEADERS: &[&str] = &[];\n"
                "fn github_copilot_route_headers() {}\n"
                "mod routes;\n"
                "#[cfg(test)] fn github_copilot_initiator() {}\n",
                encoding="utf-8",
            )
            (src / "provider/routes.rs").write_text(
                "#[cfg(feature = \"provider-openai-chat\")]\n"
                "fn github_copilot_initiator() {}\n",
                encoding="utf-8",
            )
            result = subprocess.run([sys.executable, str(script), "--enforce", "--json"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["violations"], [
                "lower provider owns Copilot header policy (see --json for sites)",
            ])
            sites = report["lower_layer_dependencies"]["provider_copilot_policy"]
            self.assertCountEqual(
                [(site["file"].split("/src/")[1], site["line"], site["symbol"]) for site in sites],
                [
                    ("provider.rs", 1, "GITHUB_COPILOT_MANAGED_HEADERS"),
                    ("provider.rs", 2, "github_copilot_route_headers"),
                    ("provider/routes.rs", 2, "github_copilot_initiator"),
                ],
            )


if __name__ == "__main__":
    unittest.main()

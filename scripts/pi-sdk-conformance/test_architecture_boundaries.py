import tempfile
import unittest
from pathlib import Path

import check_architecture_boundaries as checker


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
        self.assertFalse(report["stream_execution_edges"]["stream_imports_execution"])
        self.assertTrue(report["stream_execution_edges"]["stream_reexports_execution"])
        self.assertTrue(report["stream_execution_edges"]["execution_imports_stream"])
        self.assertEqual(report["counts"]["profile_native_state"], 0)
        self.assertGreaterEqual(report["counts"]["models_owner_modules"], 1)
        self.assertEqual(report["counts"]["models_prepared_execution_owner"], 1)
        self.assertGreater(report["models_lines"], 5000)

    def test_enforce_is_red_until_current_edges_are_closed(self):
        report = checker.architecture_report()
        self.assertTrue(checker.violations(report))


if __name__ == "__main__":
    unittest.main()

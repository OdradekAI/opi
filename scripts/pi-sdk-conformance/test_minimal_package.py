"""Adversarial admission checks for the external minimum dependency graph."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import run_minimal_package as runner


class MinimumGraphTests(unittest.TestCase):
    def test_negative_api_probes_require_the_expected_diagnostic(self):
        def output(value):
            return json.dumps({"reason": "compiler-message", "message": value})
        for code, detail, symbols in (
            ("E0599", "no associated function or constant named `builtin` found for struct `Catalog`",
             ("builtin", "Catalog")),
            ("E0432", "unresolved import `opi_ai::test_support::force_sequence_exhausted`",
             ("opi_ai::test_support::force_sequence_exhausted",)),
        ):
            message = {"level": "error", "code": {"code": code}, "message": detail}
            with self.subTest(code=code):
                runner.check_api_absence(output(message), code, *symbols)
                for unrelated in (output(dict(message, code={"code": "E0433"})),
                                  output(dict(message, message="cannot find `unrelated`")), ""):
                    with self.assertRaisesRegex(ValueError, "different reason"):
                        runner.check_api_absence(unrelated, code, *symbols)

    def graph(self, tracing=False):
        package = Path("candidate/opi-ai").resolve()
        metadata = {"packages": [
            {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None,
             "manifest_path": str(package / "Cargo.toml")},
            {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
        ], "resolve": {"root": "root", "nodes": [
            {"id": "sdk", "features": ["telemetry-tracing"] if tracing else []},
            {"id": "telemetry", "features": ["tracing"] if tracing else []},
        ]}}
        return metadata, package

    @patch.object(runner.shared, "candidate_local_id", return_value="telemetry")
    def test_independent_tracing_is_admitted(self, _):
        for tracing in (False, True):
            metadata, package = self.graph(tracing)
            runner.check_graph(metadata, package, tracing)

    @patch.object(runner.shared, "candidate_local_id", return_value="telemetry")
    def test_each_backend_and_product_dependency_is_rejected(self, _):
        original, package = self.graph()
        for name in runner.FORBIDDEN:
            metadata = copy.deepcopy(original)
            metadata["packages"].append({"id": name, "name": name, "version": "1", "source": "registry+fixture"})
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "forbidden backends"):
                runner.check_graph(metadata, package)

    def test_default_and_builtin_feature_leaks_are_rejected(self):
        for feature in ("default", "builtins", "telemetry-tracing"):
            metadata, package = self.graph()
            metadata["resolve"]["nodes"][0]["features"].append(feature)
            with self.subTest(feature=feature), self.assertRaisesRegex(ValueError, "feature union"):
                runner.check_graph(metadata, package)

    def test_workspace_sdk_substitution_is_rejected(self):
        metadata, package = self.graph()
        metadata["packages"][0]["manifest_path"] = str(Path("workspace/Cargo.toml").resolve())
        with self.assertRaisesRegex(ValueError, "extracted SDK"):
            runner.check_graph(metadata, package)

    @patch.object(runner.shared, "candidate_local_id", return_value="telemetry")
    def test_unreviewed_local_dependency_is_rejected(self, _):
        metadata, package = self.graph()
        metadata["packages"].append({"id": "local", "name": "other", "source": None})
        with self.assertRaisesRegex(ValueError, "local dependency"):
            runner.check_graph(metadata, package)

    @patch.object(runner.shared, "candidate_local_id", return_value="telemetry")
    def test_disconnected_or_unwanted_carrier_is_rejected(self, _):
        for tracing in (False, True):
            metadata, package = self.graph(tracing)
            metadata["resolve"]["nodes"][1]["features"] = [] if tracing else ["tracing"]
            with self.subTest(tracing=tracing), self.assertRaisesRegex(ValueError, "companion telemetry"):
                runner.check_graph(metadata, package, tracing)


if __name__ == "__main__":
    unittest.main()

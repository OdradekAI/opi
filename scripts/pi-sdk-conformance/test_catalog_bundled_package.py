from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import run_catalog_bundled_package as runner


class CatalogBundledRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        return {
            "packages": [
                {"id": "sdk", "name": "opi-ai", "version": "0.8.3",
                 "source": None, "manifest_path": str(package / "Cargo.toml")},
                {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3",
                 "source": None, "manifest_path": str(package.parent / "opi-telemetry/Cargo.toml")},
                {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
            ],
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": ["catalog-bundled"]},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_catalog_feature_is_admitted(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], ["catalog-bundled"])

    def test_network_backend_or_feature_union_is_rejected(self):
        metadata, package = self.metadata()
        metadata["resolve"]["nodes"][0]["features"] = ["builtins", "catalog-bundled"]
        with self.assertRaisesRegex(ValueError, "feature union"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "reqwest", "name": "reqwest", "version": "1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden backend"):
            runner.check_graph(metadata, package)

    def test_cli_refuses_existing_receipt_before_work(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "existing.json"
            output.write_text("{}\n", encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(Path(runner.__file__).resolve()),
                 "--scratch", str(Path(directory) / "scratch"),
                 "--target-dir", str(Path(directory) / "target"),
                 "--evidence-output", str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("evidence already exists", result.stderr)
            self.assertFalse((Path(directory) / "scratch").exists())

    def test_archived_manifest_must_match_workspace_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory)
            resources = Path("resources/catalog/builtin")
            shutil.copytree(runner.REPO / "crates/opi-ai" / resources, package / resources)
            summary = runner.packaged_bundle_summary(package)
            self.assertTrue(summary.startswith("catalog=sha256-"))
            self.assertIn(" classifier=", summary)

            manifest = package / resources / "manifest.json"
            manifest.write_bytes(manifest.read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "differs from workspace: manifest.json"):
                runner.packaged_bundle_summary(package)


if __name__ == "__main__":
    unittest.main()

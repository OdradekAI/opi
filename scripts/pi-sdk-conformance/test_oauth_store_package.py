from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import run_oauth_store_package as runner


class OAuthStoreRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        return {
            "packages": [
                {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None, "manifest_path": str(package / "Cargo.toml")},
                {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3", "source": None, "manifest_path": str(package.parent / "opi-telemetry/Cargo.toml")},
                {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
                {"id": "reqwest", "name": "reqwest", "version": "0.12", "source": "registry+fixture"},
                {"id": "rustls", "name": "rustls", "version": "0.23", "source": "registry+fixture"},
                {"id": "native-certs", "name": "rustls-native-certs", "version": "0.8", "source": "registry+fixture"},
                {"id": "aws-lc", "name": "aws-lc-rs", "version": "1", "source": "registry+fixture"},
            ],
            "resolve": {"root": "root", "nodes": [{"id": "sdk", "features": ["http-transport", "oauth-store"]}, {"id": "telemetry", "features": []}]},
        }, package

    def test_oauth_store_feature_is_admitted(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], ["http-transport", "oauth-store"])
        self.assertTrue(result["catalog_absent"])

    def test_protocol_or_catalog_owner_is_rejected(self):
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "catalog", "name": "opi-catalog", "version": "0.1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden protocol owner"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "hmac", "name": "hmac", "version": "0.12", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden protocol owner"):
            runner.check_graph(metadata, package)

    def test_cli_refuses_existing_receipt_before_work(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "existing.json"
            output.write_text("{}\n", encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(Path(runner.__file__).resolve()), "--scratch", str(Path(directory) / "scratch"), "--target-dir", str(Path(directory) / "target"), "--evidence-output", str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("evidence already exists", result.stderr)
            self.assertFalse((Path(directory) / "scratch").exists())


if __name__ == "__main__":
    unittest.main()

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import run_tls_backend_package as runner


class TlsBackendRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        return {
            "packages": [
                {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None,
                 "manifest_path": str(package / "Cargo.toml")},
                {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3", "source": None,
                 "manifest_path": str(package.parent / "opi-telemetry/Cargo.toml")},
                {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
                {"id": "rustls", "name": "rustls", "version": "0.23", "source": "registry+fixture"},
                {"id": "native-certs", "name": "rustls-native-certs", "version": "0.8", "source": "registry+fixture"},
                {"id": "aws-lc", "name": "aws-lc-rs", "version": "1", "source": "registry+fixture"},
            ],
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": ["tls-backend"]},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_tls_feature_is_admitted_without_http(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], ["tls-backend"])
        self.assertEqual(result["backend_present"], ["aws-lc-rs", "rustls", "rustls-native-certs"])
        self.assertTrue(result["http_absent"])

    def test_http_or_protocol_owner_is_rejected(self):
        metadata, package = self.metadata()
        metadata["resolve"]["nodes"][0]["features"] = ["tls-backend", "http-transport"]
        with self.assertRaisesRegex(ValueError, "feature union"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "reqwest", "name": "reqwest", "version": "1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden owner"):
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


if __name__ == "__main__":
    unittest.main()

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import run_websocket_compression_package as runner


class WebSocketCompressionRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        packages = [
            {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None,
             "manifest_path": str(package / "Cargo.toml")},
            {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3", "source": None,
             "manifest_path": str(package.parent / "opi-telemetry/Cargo.toml")},
            {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
        ]
        for package_id, name in [
            ("reqwest", "reqwest"), ("rustls", "rustls"),
            ("native-certs", "rustls-native-certs"), ("aws-lc", "aws-lc-rs"),
            ("libz", "libz-rs-sys"), ("flate2", "flate2"),
        ]:
            packages.append({"id": package_id, "name": name, "version": "1", "source": "registry+fixture"})
        return {
            "packages": packages,
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": ["http-transport", "tls-backend", "websocket-compression"]},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_websocket_feature_requires_http_tls_and_compression(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], ["http-transport", "tls-backend", "websocket-compression"])
        self.assertEqual(result["compression_present"], [
            "aws-lc-rs", "flate2", "libz-rs-sys", "reqwest", "rustls", "rustls-native-certs",
        ])

    def test_provider_owner_is_rejected(self):
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "hmac", "name": "hmac", "version": "1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden provider owner"):
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

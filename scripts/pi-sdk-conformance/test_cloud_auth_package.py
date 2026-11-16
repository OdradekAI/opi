from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import run_cloud_auth_package as runner


class CloudAuthRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        names = [
            ("sdk", "opi-ai", None), ("telemetry", "opi-telemetry", None),
            ("root", "consumer", None),
            ("reqwest", "reqwest", "registry+fixture"),
            ("rustls", "rustls", "registry+fixture"),
            ("native-certs", "rustls-native-certs", "registry+fixture"),
            ("aws-lc", "aws-lc-rs", "registry+fixture"),
            ("hmac", "hmac", "registry+fixture"),
            ("smithy-query", "aws-smithy-query", "registry+fixture"),
            ("smithy-xml", "aws-smithy-xml", "registry+fixture"),
            ("xmlparser", "xmlparser", "registry+fixture"),
            ("p256", "p256", "registry+fixture"),
            ("sha1", "sha1", "registry+fixture"),
        ]
        packages = []
        for ident, name, source in names:
            item = {"id": ident, "name": name, "version": "0.8.3", "source": source}
            if name == "opi-ai": item["manifest_path"] = str(package / "Cargo.toml")
            if name == "opi-telemetry": item["manifest_path"] = str(package.parent / "opi-telemetry/Cargo.toml")
            packages.append(item)
        return {
            "packages": packages,
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": ["cloud-auth", "http-transport", "tls-backend"]},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_cloud_auth_feature_is_admitted(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], ["cloud-auth", "http-transport", "tls-backend"])
        self.assertEqual(result["cloud_auth_present"], [
            "aws-lc-rs", "aws-smithy-query", "aws-smithy-xml", "hmac",
            "p256", "reqwest", "rustls", "rustls-native-certs", "sha1", "xmlparser",
        ])

    def test_provider_or_missing_cloud_owner_is_rejected(self):
        metadata, package = self.metadata()
        metadata["resolve"]["nodes"][0]["features"] = ["cloud-auth", "http-transport", "tls-backend", "protocol-adapters"]
        with self.assertRaisesRegex(ValueError, "feature union"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "protocol", "name": "protocol-adapters", "version": "0.1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden provider"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"] = [entry for entry in metadata["packages"] if entry["name"] != "p256"]
        with self.assertRaisesRegex(ValueError, "lost required owner"):
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

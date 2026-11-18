from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import run_protocol_adapters_package as runner


class ProtocolAdaptersRunnerTests(unittest.TestCase):
    def metadata(self):
        package = Path("candidate/opi-ai").resolve()
        return {
            "packages": [
                {"id": "sdk", "name": "opi-ai", "version": "0.8.3",
                 "source": None, "manifest_path": str(package / "Cargo.toml")},
                {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3",
                 "source": None, "manifest_path": str(package.parent / "opi-telemetry/Cargo.toml")},
                {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
                {"id": "reqwest", "name": "reqwest", "version": "0.12", "source": "registry+fixture"},
                {"id": "rustls", "name": "rustls", "version": "0.23", "source": "registry+fixture"},
                {"id": "native-certs", "name": "rustls-native-certs", "version": "0.8", "source": "registry+fixture"},
                {"id": "aws-lc", "name": "aws-lc-rs", "version": "1", "source": "registry+fixture"},
                {"id": "hmac", "name": "hmac", "version": "0.12", "source": "registry+fixture"},
                {"id": "smithy-query", "name": "aws-smithy-query", "version": "0.62", "source": "registry+fixture"},
                {"id": "smithy-xml", "name": "aws-smithy-xml", "version": "0.62", "source": "registry+fixture"},
                {"id": "xmlparser", "name": "xmlparser", "version": "0.13", "source": "registry+fixture"},
                {"id": "p256", "name": "p256", "version": "0.13", "source": "registry+fixture"},
                {"id": "sha1", "name": "sha1", "version": "0.10", "source": "registry+fixture"},
                {"id": "libz", "name": "libz-rs-sys", "version": "0.5", "source": "registry+fixture"},
                {"id": "flate2", "name": "flate2", "version": "1", "source": "registry+fixture"},
            ],
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": [
                    "cloud-auth", "http-transport", "protocol-adapters",
                    "provider-anthropic", "provider-azure-openai", "provider-bedrock",
                    "provider-gemini", "provider-mistral", "provider-openai-chat",
                    "provider-openai-codex-responses", "provider-openai-responses",
                    "provider-openai-responses-shared", "provider-openrouter",
                    "provider-openrouter-usage", "provider-pi-messages",
                    "provider-protocol-common", "provider-vertex", "tls-backend",
                    "websocket-compression",
                ]},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_protocol_feature_is_admitted(self):
        metadata, package = self.metadata()
        result = runner.check_graph(metadata, package)
        self.assertEqual(result["features"], [
            "cloud-auth", "http-transport", "protocol-adapters",
            "provider-anthropic", "provider-azure-openai", "provider-bedrock",
            "provider-gemini", "provider-mistral", "provider-openai-chat",
            "provider-openai-codex-responses", "provider-openai-responses",
            "provider-openai-responses-shared", "provider-openrouter",
            "provider-openrouter-usage", "provider-pi-messages",
            "provider-protocol-common", "provider-vertex", "tls-backend",
            "websocket-compression",
        ])
        self.assertEqual(result["protocol_present"], ["aws-lc-rs", "aws-smithy-query", "aws-smithy-xml", "flate2", "hmac", "libz-rs-sys", "p256", "reqwest", "rustls", "rustls-native-certs", "sha1", "xmlparser"])

    def test_catalog_or_missing_protocol_owner_is_rejected(self):
        metadata, package = self.metadata()
        metadata["resolve"]["nodes"][0]["features"] = [
            "builtins", "cloud-auth", "protocol-adapters", "tls-backend", "websocket-compression",
        ]
        with self.assertRaisesRegex(ValueError, "feature union"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"].append({"id": "catalog", "name": "opi-catalog", "version": "0.1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden catalog owner"):
            runner.check_graph(metadata, package)
        metadata, package = self.metadata()
        metadata["packages"] = [entry for entry in metadata["packages"] if entry["name"] != "hmac"]
        with self.assertRaisesRegex(ValueError, "lost required adapter owner"):
            runner.check_graph(metadata, package)

    def test_canonical_shared_layout_rejects_root_and_requires_moved_decoder(self):
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory) / "candidate" / "opi-ai"
            (package / "src" / "api" / "shared").mkdir(parents=True)
            (package / "src" / "api" / "shared" / "text_decode.rs").write_text("", encoding="utf-8")
            self.assertEqual(
                runner.check_canonical_shared_layout(package),
                {"root_text_decode_absent": True, "canonical_text_decode_present": True},
            )
            (package / "src" / "text_decode.rs").write_text("", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "deleted root text_decode"):
                runner.check_canonical_shared_layout(package)

    def test_canonical_endpoint_layout_rejects_root_and_requires_moved_helper(self):
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory) / "candidate" / "opi-ai"
            (package / "src" / "api" / "shared").mkdir(parents=True)
            (package / "src" / "api" / "shared" / "endpoint.rs").write_text("", encoding="utf-8")
            self.assertEqual(
                runner.check_canonical_endpoint_layout(package),
                {"root_endpoint_absent": True, "canonical_endpoint_present": True},
            )
            (package / "src" / "endpoint.rs").write_text("", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "deleted root endpoint"):
                runner.check_canonical_endpoint_layout(package)

    def test_provider_profiles_do_not_retain_wire_options_or_api_dependency(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["anthropic_profile_has_no_native_options"])
        self.assertTrue(result["anthropic_profile_has_no_api_dependency"])
        self.assertTrue(result["azure_openai_profile_has_no_native_options"])
        self.assertTrue(result["azure_openai_profile_has_no_api_dependency"])

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

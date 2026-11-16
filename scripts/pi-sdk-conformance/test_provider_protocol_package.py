from pathlib import Path
import unittest

import run_provider_protocol_package as runner


class ProviderProtocolRunnerTests(unittest.TestCase):
    def metadata(self, features, names):
        package = Path("candidate/opi-ai").resolve()
        packages = [
            {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None,
             "manifest_path": str(package / "Cargo.toml")},
            {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3", "source": None},
            {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
        ]
        packages.extend({"id": name, "name": name, "version": "1.0.0", "source": "registry+fixture"}
                        for name in names)
        return {
            "packages": packages,
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": sorted(features)},
                {"id": "telemetry", "features": []},
            ]},
        }, package

    def test_each_owner_requires_its_exact_feature_union(self):
        for owner, config in runner.OWNERS.items():
            with self.subTest(owner=owner):
                names = {"reqwest", "rustls", "rustls-native-certs", "aws-lc-rs"}
                if owner in {"provider-bedrock", "provider-vertex"}:
                    names |= runner.CLOUD_PACKAGES
                if owner == "provider-openai-codex-responses":
                    names |= runner.WS_PACKAGES
                metadata, package = self.metadata(config["features"], names)
                result = runner.check_graph(metadata, package, owner)
                self.assertEqual(result["features"], sorted(config["features"]))

    def test_wrong_owner_graph_is_rejected(self):
        metadata, package = self.metadata(
            runner.OWNERS["provider-anthropic"]["features"] | {"provider-openrouter"},
            {"reqwest", "rustls", "rustls-native-certs", "aws-lc-rs"},
        )
        with self.assertRaisesRegex(ValueError, "feature union"):
            runner.check_graph(metadata, package, "provider-anthropic")


if __name__ == "__main__":
    unittest.main()

import unittest
from pathlib import Path
import tomllib

import run_shared_owner_pairs as runner


class SharedOwnerPairTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = tomllib.loads(Path("crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))

    def test_pair_set_is_exact(self):
        self.assertEqual(list(runner.PAIRS), [
            "cloud-auth+catalog-bundled",
            "websocket-compression+oauth-store",
        ])
        self.assertEqual(len(runner.PAIRS), 2)

    def test_feature_closures_keep_transport_owners(self):
        cloud = runner.feature_closure(self.manifest, {"cloud-auth", "catalog-bundled"})
        self.assertEqual(cloud, {"cloud-auth", "catalog-bundled", "http-transport", "tls-backend"})
        websocket = runner.feature_closure(self.manifest, {"websocket-compression", "oauth-store"})
        self.assertEqual(websocket, {"websocket-compression", "oauth-store", "http-transport", "tls-backend"})

    def test_pair_graph_rejects_missing_and_forbidden_packages(self):
        definition = runner.PAIRS["cloud-auth+catalog-bundled"]
        package = Path("candidate/opi-ai").resolve()
        metadata = {
            "packages": [
                {"id": "sdk", "name": "opi-ai", "version": "0.8.3", "source": None,
                 "manifest_path": str(package / "Cargo.toml")},
                {"id": "telemetry", "name": "opi-telemetry", "version": "0.8.3", "source": None},
                {"id": "root", "name": "consumer", "version": "0.0.0", "source": None},
            ],
            "resolve": {"root": "root", "nodes": [
                {"id": "sdk", "features": ["cloud-auth", "catalog-bundled", "http-transport", "tls-backend"]},
                {"id": "telemetry", "features": []},
            ]},
        }
        for index, name in enumerate(sorted(definition["required_packages"]), start=1):
            metadata["packages"].append({"id": f"pkg{index}", "name": name, "version": "1", "source": "registry+fixture"})
        result = runner.check_package_graph(metadata, package, "cloud-auth+catalog-bundled", definition, self.manifest)
        self.assertIn("cloud-auth", result["features"])
        metadata["packages"].append({"id": "ws", "name": "flate2", "version": "1", "source": "registry+fixture"})
        with self.assertRaisesRegex(ValueError, "forbidden"):
            runner.check_package_graph(metadata, package, "cloud-auth+catalog-bundled", definition, self.manifest)


if __name__ == "__main__":
    unittest.main()

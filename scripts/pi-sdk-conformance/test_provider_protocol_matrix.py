import unittest
from pathlib import Path
import tomllib

import run_provider_protocol_matrix as matrix


class ProviderProtocolMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = tomllib.loads(
            Path("crates/opi-ai/Cargo.toml").read_text(encoding="utf-8")
        )

    def test_pair_matrix_is_exhaustive_and_deterministic(self):
        pairs = matrix.provider_pairs()
        self.assertEqual(len(pairs), 55)
        self.assertEqual(len({tuple(pair) for pair in pairs}), 55)
        self.assertTrue(all(left < right for left, right in pairs))
        self.assertEqual(len(matrix.profiles()), 57)

    def test_provider_closure_keeps_transitive_owners(self):
        vertex = matrix.feature_closure(self.manifest, {"provider-vertex"})
        self.assertTrue({"provider-vertex", "provider-gemini", "cloud-auth", "http-transport", "tls-backend"}.issubset(vertex))
        codex = matrix.feature_closure(self.manifest, {"provider-openai-codex-responses"})
        self.assertTrue({"provider-openai-codex-responses", "websocket-compression", "http-transport", "tls-backend"}.issubset(codex))

    def test_high_order_profile_contains_support_owners(self):
        features = set(matrix.HIGH_ORDER["all-provider-owners-plus-support"])
        self.assertTrue(set(matrix.PROVIDERS).issubset(features))
        self.assertTrue({"catalog-bundled", "oauth-store", "telemetry-tracing"}.issubset(features))


if __name__ == "__main__":
    unittest.main()

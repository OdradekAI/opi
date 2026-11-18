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

    def test_migrated_owners_use_canonical_namespaces_only(self):
        anthropic_source = runner.OWNERS["provider-anthropic"]["source"]
        openrouter_source = runner.OWNERS["provider-openrouter"]["source"]
        azure_source = runner.OWNERS["provider-azure-openai"]["source"]
        gemini_source = runner.OWNERS["provider-gemini"]["source"]
        mistral_source = runner.OWNERS["provider-mistral"]["source"]
        openai_source = runner.OWNERS["provider-openai-chat"]["source"]
        codex_source = runner.OWNERS["provider-openai-codex-responses"]["source"]
        openai_responses_source = runner.OWNERS["provider-openai-responses"]["source"]
        pi_messages_source = runner.OWNERS["provider-pi-messages"]["source"]
        vertex_source = runner.OWNERS["provider-vertex"]["source"]
        bedrock_source = runner.OWNERS["provider-bedrock"]["source"]
        self.assertNotIn("opi_ai::anthropic::", anthropic_source)
        self.assertNotIn("opi_ai::openrouter::", openrouter_source)
        self.assertNotIn("opi_ai::azure_openai::", azure_source)
        self.assertNotIn("opi_ai::gemini::", gemini_source)
        self.assertNotIn("opi_ai::mistral::", mistral_source)
        self.assertNotIn("opi_ai::pi_messages::", pi_messages_source)
        self.assertNotIn("opi_ai::openai_chat::", openai_source)
        self.assertNotIn("opi_ai::openai_codex_responses::", codex_source)
        self.assertNotIn("opi_ai::openai_responses::", openai_responses_source)
        self.assertNotIn("opi_ai::vertex::", vertex_source)
        self.assertNotIn("opi_ai::bedrock::", bedrock_source)
        self.assertIn("opi_ai::api::anthropic::", anthropic_source)
        self.assertIn("opi_ai::providers::anthropic::", anthropic_source)
        self.assertIn("opi_ai::providers::openrouter::", openrouter_source)
        self.assertIn("opi_ai::providers::azure_openai::", azure_source)
        self.assertIn("opi_ai::api::azure_openai::", azure_source)
        self.assertIn("opi_ai::api::gemini::", gemini_source)
        self.assertIn("opi_ai::providers::gemini::", gemini_source)
        self.assertIn("opi_ai::providers::mistral::", mistral_source)
        self.assertIn("opi_ai::api::mistral::", mistral_source)
        self.assertIn("opi_ai::api::openai::", openai_source)
        self.assertIn("opi_ai::providers::openai::", openai_source)
        self.assertIn("opi_ai::api::openai_codex_responses::", codex_source)
        self.assertIn("opi_ai::providers::openai_codex::", codex_source)
        self.assertIn("opi_ai::api::openai_responses::", openai_responses_source)
        self.assertIn("opi_ai::providers::openai_responses::", openai_responses_source)
        self.assertIn("opi_ai::api::pi_messages::", pi_messages_source)
        self.assertIn("opi_ai::providers::pi_messages::", pi_messages_source)
        self.assertIn("opi_ai::providers::vertex::", vertex_source)
        self.assertIn("opi_ai::api::vertex::", vertex_source)
        self.assertIn("opi_ai::api::bedrock::", bedrock_source)
        self.assertIn("opi_ai::providers::bedrock::", bedrock_source)
        self.assertIn("OpenRouterUsageAccounting", openrouter_source)
        self.assertIn("OpenRouterCostDetails", openrouter_source)

    def test_vertex_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/vertex.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/vertex.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/vertex.rs").exists())

    def test_pi_messages_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/pi_messages.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/pi_messages.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/pi_messages.rs").exists())

    def test_openai_responses_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openai_responses.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/openai_responses.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/openai_responses.rs").exists())

    def test_openai_responses_shared_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openai_responses_shared.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/openai_responses.rs").exists())

    def test_openai_responses_shared_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/openai.rs",
            "crates/opi-ai/src/api/openai_responses.rs",
            "crates/opi-ai/src/api/openai_codex_responses.rs",
            "crates/opi-ai/src/transport/codex.rs",
            "crates/opi-ai/src/api/azure_openai.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertIn("crate::api::shared::openai_responses", source, relative)
            self.assertNotIn("crate::openai_responses_shared", source, relative)

    def test_openrouter_usage_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openrouter_usage.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/openrouter_usage.rs").exists())

    def test_openrouter_usage_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/anthropic.rs",
            "crates/opi-ai/src/api/openai.rs",
            "crates/opi-ai/src/providers/openrouter.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertIn("crate::api::shared::openrouter_usage", source, relative)
            self.assertNotIn("crate::openrouter_usage", source, relative)

    def test_bedrock_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/bedrock.rs").exists())
        self.assertFalse((runner.REPO / "crates/opi-ai/src/bedrock").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/bedrock.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/bedrock/event_stream.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/bedrock.rs").exists())

    def test_bedrock_callers_use_canonical_namespaces(self):
        for relative in (
            "crates/opi-ai/src/api/bedrock.rs",
            "crates/opi-ai/src/providers/bedrock.rs",
            "crates/opi-ai/src/providers/collection.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("crate::bedrock", source, relative)
        for relative in (
            "crates/opi-ai/src/api/bedrock.rs",
            "crates/opi-ai/src/providers/bedrock.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("crate::bedrock::sigv4", source, relative)

    def test_mapped_provider_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/api_mapped.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/mapped.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("pub mod api_mapped", lib)
        self.assertNotIn("pub use api_mapped", lib)

    def test_mapped_provider_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/providers/collection.rs",
            "crates/opi-coding-agent/src/provider_factory.rs",
            "crates/opi-ai/tests/api_mapped_provider.rs",
            "crates/opi-ai/tests/provider_collection.rs",
            "crates/opi-coding-agent/tests/extension_mapped_catalog.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("crate::api_mapped", source, relative)
            self.assertNotIn("opi_ai::api_mapped", source, relative)
            self.assertNotIn("opi_ai::ApiMappedProvider", source, relative)
        source = (runner.REPO / "crates/opi-ai/src/providers/collection.rs").read_text(encoding="utf-8")
        self.assertIn("crate::providers::mapped::ApiMappedProvider", source)

    def test_provider_collection_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/provider_collection.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/collection.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("pub mod provider_collection", lib)

    def test_provider_collection_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-coding-agent/src/provider_factory.rs",
            "crates/opi-ai/tests/provider_collection.rs",
            "crates/opi-ai/examples/quick_start.rs",
            "crates/opi-ai/examples/quick_start_stream.rs",
            "crates/opi-ai/examples/tool_loop.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("opi_ai::provider_collection", source, relative)
            self.assertIn("opi_ai::providers::collection", source, relative)
        source = (runner.REPO / "crates/opi-ai/src/providers/collection.rs").read_text(encoding="utf-8")
        self.assertNotIn("crate::provider_collection", source)

    def test_provider_headers_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/provider_headers.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/headers.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("pub mod provider_headers", lib)
        self.assertNotIn("pub use provider_headers", lib)

    def test_provider_headers_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/shared/headers.rs",
            "crates/opi-ai/src/api/openai_responses.rs",
            "crates/opi-ai/src/providers/openai.rs",
            "crates/opi-ai/src/providers/collection.rs",
            "crates/opi-coding-agent/src/provider_factory.rs",
            "crates/opi-ai/tests/public_utilities.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("provider_headers::", source, relative)
        source = (runner.REPO / "crates/opi-ai/src/api.rs").read_text(encoding="utf-8")
        self.assertIn("pub mod shared", source)
        source = (runner.REPO / "crates/opi-ai/src/api/shared.rs").read_text(encoding="utf-8")
        self.assertIn("pub mod headers", source)

    def test_provider_env_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/provider_env.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/env.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("pub mod provider_env", lib)

    def test_provider_env_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/pi_messages.rs",
            "crates/opi-ai/src/api/shared/env.rs",
            "crates/opi-ai/tests/public_utilities.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertNotIn("provider_env::", source, relative)
        source = (runner.REPO / "crates/opi-ai/src/api/shared.rs").read_text(encoding="utf-8")
        self.assertIn("pub mod env", source)

    def test_text_decode_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/text_decode.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/text_decode.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("mod text_decode", lib)

    def test_operation_contract_root_module_is_deleted(self):
        result = runner.check_canonical_operation_contract_layout(
            runner.REPO / "crates/opi-ai"
        )
        self.assertTrue(result["root_deferred_absent"])
        self.assertTrue(result["canonical_operation_contract_present"])
        self.assertTrue(result["models_chat_contract_absent"])

    def test_operation_contract_callers_use_canonical_namespaces(self):
        source_root = runner.REPO / "crates"
        stale = []
        for path in source_root.rglob("*.rs"):
            source = path.read_text(encoding="utf-8")
            if "crate::deferred::" in source or "opi_ai::deferred::" in source:
                stale.append(path.relative_to(runner.REPO).as_posix())
        self.assertEqual(stale, [])

    def test_openai_profile_does_not_store_wire_options(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["openai_profile_has_no_native_options"])

    def test_openai_responses_profile_does_not_store_wire_options(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["openai_responses_profile_has_no_native_options"])

    def test_openai_codex_profile_does_not_store_wire_options_or_depend_on_api(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["openai_codex_profile_has_no_native_options"])
        self.assertTrue(result["openai_codex_profile_has_no_api_dependency"])

    def test_anthropic_profile_does_not_store_wire_options_or_depend_on_api(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["anthropic_profile_has_no_native_options"])
        self.assertTrue(result["anthropic_profile_has_no_api_dependency"])

    def test_azure_openai_profile_does_not_store_wire_options_or_depend_on_api(self):
        result = runner.check_openai_profile_layout(runner.REPO / "crates/opi-ai")
        self.assertTrue(result["azure_openai_profile_has_no_native_options"])
        self.assertTrue(result["azure_openai_profile_has_no_api_dependency"])

    def test_anthropic_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/anthropic.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/anthropic.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/anthropic.rs").exists())

    def test_azure_openai_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/azure_openai.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/azure_openai.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/azure_openai.rs").exists())

    def test_text_decode_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/anthropic.rs",
            "crates/opi-ai/src/api/azure_openai.rs",
            "crates/opi-ai/src/api/gemini.rs",
            "crates/opi-ai/src/api/mistral.rs",
            "crates/opi-ai/src/api/openai.rs",
            "crates/opi-ai/src/api/openai_codex_responses.rs",
            "crates/opi-ai/src/api/openai_responses.rs",
            "crates/opi-ai/src/api/pi_messages.rs",
            "crates/opi-ai/src/api/vertex.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertIn("crate::api::shared::text_decode", source, relative)
            self.assertNotIn("crate::text_decode", source, relative)
        shared = (runner.REPO / "crates/opi-ai/src/api/shared.rs").read_text(encoding="utf-8")
        self.assertIn("pub(crate) mod text_decode", shared)

    def test_endpoint_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/endpoint.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/shared/endpoint.rs").exists())
        lib = (runner.REPO / "crates/opi-ai/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("mod endpoint", lib)

    def test_endpoint_callers_use_canonical_namespace(self):
        for relative in (
            "crates/opi-ai/src/api/mistral.rs",
            "crates/opi-ai/src/api/openai.rs",
            "crates/opi-ai/src/api/openai_codex_responses.rs",
            "crates/opi-ai/src/transport/codex.rs",
            "crates/opi-ai/src/api/openai_responses.rs",
        ):
            source = (runner.REPO / relative).read_text(encoding="utf-8")
            self.assertIn("crate::api::shared::endpoint::join_endpoint", source, relative)
            self.assertNotIn("crate::endpoint", source, relative)
        shared = (runner.REPO / "crates/opi-ai/src/api/shared.rs").read_text(encoding="utf-8")
        self.assertIn("pub(crate) mod endpoint", shared)

    def test_openai_chat_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openai_chat.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/openai.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/openai.rs").exists())

    def test_openai_codex_responses_root_module_is_deleted(self):
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openai_codex_responses.rs").exists())
        self.assertFalse((runner.REPO / "crates/opi-ai/src/openai_codex_responses").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/openai_codex_responses.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/api/openai_codex_responses/native.rs").exists())
        self.assertFalse((runner.REPO / "crates/opi-ai/src/api/openai_codex_responses/ws_transport.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/transport.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/transport/codex.rs").exists())
        self.assertTrue((runner.REPO / "crates/opi-ai/src/providers/openai_codex.rs").exists())


if __name__ == "__main__":
    unittest.main()

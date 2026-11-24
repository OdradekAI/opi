from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import check_test_inventory as inventory
import check_support_inventory as support_inventory
import select_tests as selector


class SelectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = inventory.load_inventory()
        cls.support_rows = support_inventory.resolve_inventory(rows=cls.rows)

    def plan(self, *paths, targets=()):
        return selector.make_plan(self.rows, ["crates/opi-ai/" + p for p in paths], targets)

    def test_each_classifier_includes_its_wire_and_shared_dispatch(self):
        for module, count in (
            ("openai_decisions", 5),
            ("cloudflare_workers_ai_system_one", 3),
            ("llama_cpp_classify", 2),
        ):
            with self.subTest(module=module):
                plan = self.plan("src/api/" + module + ".rs")
                rows = {r["target"]: r["args"] for r in plan["tests"]}
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(len(rows), count)
                args = rows["classifier_contract"]
                self.assertIn(module + "::", args)
                self.assertTrue(set(selector.CROSS_CLASSIFIER).issubset(args))
                self.assertFalse(any(query.startswith("models_contract::") for query in args))
                self.assertIn("builtins", args)
                self.assertIn("classifier_and_image_apis_answer_typed_through_one_instance",
                              rows["external_sdk"])
                self.assertIn("archive@" + selector.ARCHIVE, plan["runners"])

    def test_system_one_and_shared_test_source_override_narrow_filters(self):
        plan = self.plan("src/api/openai_decisions.rs", "src/api/system_one.rs")
        tests = {r["target"]: r for r in plan["tests"]}
        self.assertNotIn("--", tests["classifier_contract"]["args"])
        self.assertEqual(len(tests["classifier_contract"]["reasons"]), 2)
        self.assertEqual(tests["telemetry_operations"]["args"][-2:],
                         ["--", "classifier_calls_freeze_context"])
        self.assertIn("builtin_openrouter_classifier_sends_images_before_state",
                      tests["provider_collection"]["args"])
        explicit = self.plan("src/api/llama_cpp_classify.rs", "tests/classifier_contract.rs")
        command = next(r for r in explicit["tests"] if r["target"] == "classifier_contract")
        self.assertNotIn("--", command["args"])

    def test_qualified_filter_requires_the_declared_classifier_module(self):
        path = "src/api/openai_decisions.rs"
        with patch.dict(selector.PRODUCTION, {
            path: {"classifier_contract": ("renamed_module::decisions_",)},
        }):
            with self.assertRaisesRegex(ValueError, "stale test filter classifier_contract: renamed_module"):
                self.plan(path)

    def test_nested_protocol_topic_selects_only_its_module(self):
        for source, module in (
            ("tests/anthropic_fixtures/sse.rs", "sse::"),
            ("tests/anthropic_fixtures/constrained_sampling.rs", "constrained_sampling::"),
            ("tests/anthropic_fixtures/adapter.rs", "adapter::"),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]}, {"anthropic_fixtures"})
                args = plan["tests"][0]["args"]
                self.assertIn("--", args)
                self.assertIn(module, args)

    def test_shared_nested_fixture_keeps_the_whole_protocol_target(self):
        plan = self.plan("tests/anthropic_fixtures/fixtures.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

    def test_external_sdk_topics_keep_their_filters_and_archive_owner(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "external_sdk"]
        runner = "archive@scripts/pi-sdk-conformance/run_readme_examples.py"
        for source, query in (
            ("tests/external_sdk/chat_matrix.rs", "chat_matrix::"),
            ("tests/external_sdk/classifier_and_images.rs", "classifier_and_images::"),
            ("tests/external_sdk/models_contract.rs", "models_contract::"),
            ("tests/external_sdk/fixtures.rs", None),
            ("tests/external_sdk.rs", None),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])
        for source, query in (
            ("src/api/openai_completions.rs", "assembled_matrix_drives"),
            ("src/api/system_one.rs", "classifier_and_image_apis_answer_typed_through_one_instance"),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertIn(runner, plan["runners"])
                self.assertIn("archive@" + selector.ARCHIVE, plan["runners"])
                self.assertEqual([row["args"] for row in plan["tests"]
                                  if row["target"] == "external_sdk"], [command + ["--", query]])

    def test_codex_protocol_topics_select_only_their_module(self):
        for source, module in (
            ("tests/openai_codex_responses/wire.rs", "wire::"),
            ("tests/openai_codex_responses/constrained_sampling.rs", "constrained_sampling::"),
            ("tests/openai_codex_responses/ws_path.rs", "ws_path::"),
            ("tests/openai_codex_responses/ws_continuation.rs", "ws_continuation::"),
            ("tests/openai_codex_responses/ws_branches.rs", "ws_branches::"),
            ("tests/openai_codex_responses/ws_cancellation.rs", "ws_cancellation::"),
            ("tests/openai_codex_responses/ws_resources.rs", "ws_resources::"),
            ("tests/openai_codex_responses/ws_protocol_regressions.rs", "ws_protocol_regressions::"),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]}, {"openai_codex_responses"})
                args = plan["tests"][0]["args"]
                self.assertIn("--", args)
                self.assertIn(module, args)

    def test_codex_shared_deflate_helper_keeps_the_whole_protocol_target(self):
        plan = self.plan("tests/openai_codex_responses/deflate_audit.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

    def test_provider_collection_topics_select_only_their_module(self):
        for module in (
            "auth_resolution", "azure", "builtin_auth", "builtin_chat", "builtin_operations",
            "builtin_registration", "catalog_store_refresh_20_33", "dispatch",
            "helper_retry_hints", "opencode", "phase17", "registry_preflight_and_refresh",
            "selected_owner_management",
        ):
            with self.subTest(module=module):
                plan = self.plan(f"tests/provider_collection/{module}.rs")
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]},
                                 {"provider_collection"})
                self.assertEqual(plan["tests"][0]["args"][-2:], ["--", f"{module}::"])

    def test_shared_auth_and_collection_fixtures_select_the_whole_target(self):
        for target in ("auth_contracts", "provider_collection"):
            with self.subTest(target=target):
                plan = self.plan(f"tests/{target}/fixtures.rs")
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual([row["target"] for row in plan["tests"]], [target])
                self.assertNotIn("--", plan["tests"][0]["args"])

    def test_catalog_operation_topics_and_shared_inputs_select_the_expected_scope(self):
        for source, tail in (
            ("catalog_operations/publication.rs", ["--", "publication::"]),
            ("catalog_operations/dispatch.rs", ["--", "dispatch::"]),
            ("catalog_operations/pricing.rs", ["--", "pricing::"]),
            ("catalog_operations/fixtures.rs", []),
            ("catalog_operations.rs", []),
        ):
            with self.subTest(source=source):
                plan = self.plan(f"tests/{source}")
                self.assertEqual(plan["scope"], "focused")
                [command] = plan["tests"]
                self.assertEqual(command["target"], "catalog_operations")
                index = command["args"].index("--test")
                self.assertEqual(command["args"][index + 2:], tail)
                self.assertIn(
                    "archive@scripts/pi-sdk-conformance/run_catalog_bundled_package.py",
                    plan["runners"],
                )

    def test_bedrock_fixture_topics_select_only_their_module(self):
        for source, module in (
            ("tests/bedrock_fixtures/builtin_routes.rs", "builtin_routes::"),
            ("tests/bedrock_fixtures/constrained_guard.rs", "constrained_guard::"),
            ("tests/bedrock_fixtures/converse_adapter/projection.rs", "converse_adapter::projection::"),
            ("tests/bedrock_fixtures/converse_adapter/stream.rs", "converse_adapter::stream::"),
            ("tests/bedrock_fixtures/converse_adapter/replay.rs", "converse_adapter::replay::"),
            ("tests/bedrock_fixtures/converse_adapter/lifecycle.rs", "converse_adapter::lifecycle::"),
            ("tests/bedrock_fixtures/wire.rs", "wire::"),
            ("tests/bedrock_fixtures/lifecycle.rs", "lifecycle::"),
            ("tests/bedrock_fixtures/aws_credentials/chain.rs", "lifecycle::aws_credentials::chain::"),
            ("tests/bedrock_fixtures/aws_credentials/role_sources.rs", "lifecycle::aws_credentials::role_sources::"),
            ("tests/bedrock_fixtures/aws_credentials/metadata.rs", "lifecycle::aws_credentials::metadata::"),
            ("tests/bedrock_fixtures/aws_credentials/lifecycle.rs", "lifecycle::aws_credentials::lifecycle::"),
            ("tests/bedrock_fixtures/aws_sso/discovery.rs", "lifecycle::aws_sso::discovery::"),
            ("tests/bedrock_fixtures/aws_sso/expiry.rs", "lifecycle::aws_sso::expiry::"),
            ("tests/bedrock_fixtures/aws_sso/login.rs", "lifecycle::aws_sso::login::"),
            ("tests/bedrock_fixtures/aws_sso/chain.rs", "lifecycle::aws_sso::chain::"),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]},
                                 {"bedrock_fixtures"})
                args = plan["tests"][0]["args"]
                self.assertIn("--", args)
                self.assertIn(module, args)

    def test_bedrock_shared_fixture_keeps_the_whole_target(self):
        plan = self.plan("tests/bedrock_fixtures/fixtures.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/converse_adapter.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/converse_adapter/support.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/aws_credentials.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/aws_credentials/support.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/aws_sso.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

        plan = self.plan("tests/bedrock_fixtures/aws_sso/support.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

    def test_openai_chat_fixture_topics_select_only_their_module(self):
        for source, module in (
            ("tests/openai_chat_fixtures/wire.rs", "wire::"),
            ("tests/openai_chat_fixtures/lifecycle.rs", "lifecycle::"),
            ("tests/openai_chat_fixtures/constrained_guard.rs", "constrained_guard::"),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]},
                                 {"openai_chat_fixtures"})
                args = plan["tests"][0]["args"]
                self.assertIn("--", args)
                self.assertIn(module, args)

    def test_openai_chat_shared_fixture_keeps_the_whole_target(self):
        plan = self.plan("tests/openai_chat_fixtures/fixtures.rs")
        self.assertEqual(plan["scope"], "focused")
        args = plan["tests"][0]["args"]
        self.assertNotIn("--", args)

    def test_http_acceptance_topics_select_their_filters_and_archive_family(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "http_acceptance"]
        for source, query in (
            ("tests/http_acceptance/response_parsing.rs", "response_parsing::"),
            ("tests/http_acceptance/redirects_and_disconnects.rs", "redirects_and_disconnects::"),
            ("tests/http_acceptance/https_proxy.rs", "https_proxy::"),
            ("tests/http_acceptance/fixtures.rs", None),
            ("tests/http_acceptance.rs", None),
        ):
            with self.subTest(source=source):
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [f"{selector.PACKAGE_SLICE} --family http"])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_chat_capture_replay_selects_each_stage_and_nested_projection_topic(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "chat_capture_replay"]
        runner = "archive@scripts/pi-sdk-conformance/run_readme_examples.py"
        for source, query in (
            ("/producer_capture.rs", "producer_capture::"),
            ("/persistence_restore.rs", "persistence_restore::"),
            ("/target_projection.rs", "target_projection::"),
            ("/target_projection/replay_boundaries.rs", "target_projection::replay_boundaries::"),
            ("/target_projection/persisted_producers.rs", "target_projection::persisted_producers::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(source=source):
                plan = self.plan("tests/chat_capture_replay" + source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_replay_contract_topics_select_independently_with_readme_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "replay_contract"]
        runner = "archive@scripts/pi-sdk-conformance/run_readme_examples.py"
        for suffix, query in (
            ("/association.rs", "association::"),
            ("/execution_effects.rs", "execution_effects::"),
            ("/malformed_arguments.rs", "malformed_arguments::"),
            ("/wire_ids.rs", "wire_ids::"),
            ("/content_projection.rs", "content_projection::"),
            ("/object_wire.rs", "object_wire::"),
            ("/responses_grammar.rs", "grammar_replay_tests::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/replay_contract" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_model_wire_metadata_topics_select_independently_with_catalog_archive(self):
        target = "model_wire_metadata"
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", target]
        runner = "archive@scripts/pi-sdk-conformance/run_catalog_bundled_package.py"
        for suffix, query in (
            ("/model_contract.rs", "model_contract::"),
            ("/thinking_admission.rs", "thinking_admission::"),
            ("/sampling_parameters.rs", "sampling_parameters::"),
            ("/sampling_wire.rs", "sampling_wire::"),
            ("/pricing_usage.rs", "pricing_usage::"),
            ("/anthropic_complete_metadata.rs", "anthropic_complete_metadata::"),
            ("/anthropic_response_cost.rs", "anthropic_response_cost::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/" + target + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_retry_backoff_topics_select_independently_with_http_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "retry_backoff"]
        for topic in ("response_headers", "backoff_policy", "acquisition", "smithy", None):
            with self.subTest(topic=topic):
                source = f"tests/retry_backoff/{topic}.rs" if topic else "tests/retry_backoff.rs"
                plan = self.plan(source)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [f"{selector.PACKAGE_SLICE} --family http"])
                filters = ["--", f"{topic}::"] if topic else []
                self.assertEqual([row["args"] for row in plan["tests"]], [command + filters])

    def test_whole_call_retry_topics_select_independently_with_http_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "whole_call_retry"]
        for suffix, query in (
            ("/classification.rs", "classification::"),
            ("/call_and_callbacks.rs", "call_and_callbacks::"),
            ("/backoff_and_cancellation.rs", "backoff_and_cancellation::"),
            ("/call_and_callbacks/frozen_target_wire.rs", "call_and_callbacks::frozen_target_wire::"),
            ("/call_and_callbacks/quota_transport.rs", "call_and_callbacks::quota_transport::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/whole_call_retry" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [f"{selector.PACKAGE_SLICE} --family http"])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_provider_lifecycle_topics_select_independently_with_readme_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "provider_lifecycle"]
        runner = "archive@scripts/pi-sdk-conformance/run_readme_examples.py"
        for suffix, query in (
            ("/request_and_errors.rs", "request_and_errors::"),
            ("/sse_decoding.rs", "sse_decoding::"),
            ("/cancellation_and_release.rs", "cancellation_and_release::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/provider_lifecycle" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_deferred_contract_topics_select_independently_with_minimal_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--test", "deferred_contract"]
        runner = "archive@scripts/pi-sdk-conformance/run_minimal_package.py"
        for suffix, query in (
            ("/handle_validation.rs", "handle_validation::"),
            ("/submission_and_fetch.rs", "submission_and_fetch::"),
            ("/cancellation.rs", "cancellation::"),
            ("/reopen.rs", "reopen::"),
            ("/telemetry.rs", "telemetry::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/deferred_contract" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_public_utility_topics_select_independently_with_protocol_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--features", "builtins",
                   "--test", "public_utilities"]
        runner = "archive@" + selector.ARCHIVE
        for topic in ("rendering", "tool_arguments", "context_overflow", "provider_diagnostics",
                      "identity", "provider_configuration", "cancellation", "grammar_property_names", None):
            with self.subTest(topic=topic):
                suffix = f"/{topic}.rs" if topic else ".rs"
                plan = self.plan("tests/public_utilities" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                filters = ["--", f"{topic}::"] if topic else []
                self.assertEqual([row["args"] for row in plan["tests"]], [command + filters])

    def test_schema_engine_topics_select_independently_with_schema_archive(self):
        command = [*selector.CARGO_TEST, "--no-default-features", "--test", "schema_engine"]
        runner = "archive@scripts/pi-sdk-conformance/run_early_package_proof.py"
        for topic in ("corpus", "value_keywords", "container_keywords", "applicators",
                      "reference_resolution", "admission", "stack_safety", "fixtures", None):
            with self.subTest(topic=topic):
                suffix = f"/{topic}.rs" if topic else ".rs"
                plan = self.plan("tests/schema_engine" + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                filters = ["--", f"{topic}::"] if topic and topic != "fixtures" else []
                self.assertEqual([row["args"] for row in plan["tests"]], [command + filters])

    def test_custom_registration_topics_select_independently_with_minimal_archive(self):
        target = "custom_provider_registration"
        command = [*selector.CARGO_TEST, "--no-default-features", "--test", target]
        runner = "archive@scripts/pi-sdk-conformance/run_minimal_package.py"
        for suffix, query in (
            ("/registry_providers.rs", "registry_providers::"),
            ("/registry_models.rs", "registry_models::"),
            ("/chat_registration.rs", "chat_registration::"),
            ("/chat_dispatch.rs", "chat_dispatch::"),
            ("/chat_lifecycle.rs", "chat_lifecycle::"),
            ("/chat_admission.rs", "chat_admission::"),
            ("/fixtures.rs", None),
            (".rs", None),
        ):
            with self.subTest(suffix=suffix):
                plan = self.plan("tests/" + target + suffix)
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual(plan["runners"], [runner])
                self.assertEqual([row["args"] for row in plan["tests"]],
                                 [command + (["--", query] if query else [])])

    def test_shared_helper_consumers_and_private_ws_test(self):
        expected = {
            "pending_auth": {"classifier_contract", "deferred_contract", "openrouter_fixtures"},
            "catalog_store": {"catalog_lifecycle", "node4_preparation", "provider_collection",
                              "radius_builtin_catalog", "registry"},
            "tls_fixtures": {"bedrock_fixtures", "oauth_copilot", "oauth_regressions", "http_acceptance", "tls_contract", "ws_upgrade"},
            "ws_peer": {"codex_native_options", "service_tier_cost", "ws_upgrade", "lib"},
        }
        for helper, targets in expected.items():
            with self.subTest(helper=helper):
                plan = self.plan(f"tests/support/{helper}.rs")
                self.assertEqual({r["target"] for r in plan["tests"]}, targets)
                if helper in {"pending_auth", "catalog_store", "tls_fixtures"}:
                    self.assertEqual(plan["scope"], "focused")
                    self.assertEqual(
                        [row["target"] for row in plan["tests"] if "--" in row["args"]],
                        [],
                    )
                if helper == "pending_auth":
                    self.assertIn("archive@scripts/pi-sdk-conformance/run_minimal_package.py",
                                  plan["runners"])
                if helper == "tls_fixtures":
                    self.assertEqual(
                        [name for name in plan["runners"] if "package_slice.py" in name],
                        [
                            "archive@scripts/pi-sdk-conformance/package_slice.py --family crypto",
                            "archive@scripts/pi-sdk-conformance/package_slice.py --family http",
                            "archive@scripts/pi-sdk-conformance/package_slice.py --family ws",
                        ],
                    )
                if helper == "ws_peer":
                    self.assertEqual([name for name in plan["runners"] if "package_slice.py" in name],
                                     ["archive@scripts/pi-sdk-conformance/package_slice.py --family ws"])
                    command = next(r["args"] for r in plan["tests"] if r["target"] == "lib")
                    self.assertIn("websocket-compression", command)
                    self.assertIn("ws::tests::upgrade::", command)

    def test_support_inventory_matches_selector_consumers(self):
        for row in self.support_rows:
            with self.subTest(helper=row["helper"]):
                plan = self.plan("tests/support/" + row["helper"])
                targets = {entry["target"] for entry in plan["tests"]}
                expected = set(support_inventory.parse_list(row["consumers"]))
                if row["helper"] == "ws_peer.rs":
                    expected.add("lib")
                self.assertEqual(targets, expected)

    def test_support_inventory_metadata_selects_its_checker(self):
        plan = self.plan("tests/support/OWNERS.tsv")
        self.assertEqual(plan["scope"], "focused")
        self.assertEqual(plan["tests"], [])
        self.assertEqual(plan["runners"],
                         ["inventory@scripts/pi-sdk-conformance/check_support_inventory.py"])

    def test_target_inventory_selects_its_checker(self):
        plan = self.plan("tests/targets.tsv")
        self.assertEqual(plan["scope"], "focused")
        self.assertEqual(plan["tests"], [])
        self.assertEqual(plan["runners"],
                         ["inventory@scripts/pi-sdk-conformance/check_test_inventory.py"])

    def test_provider_profile_sources_select_one_owner_each(self):
        profiles = {
            "anthropic": "provider_anthropic_profile",
            "azure_openai": "provider_azure_openai_profile",
            "bedrock": "provider_bedrock_profile",
            "gemini": "provider_gemini_profile",
            "mistral": "provider_mistral_profile",
            "openai_chat": "provider_openai_chat_profile",
            "openai_codex_responses": "provider_openai_codex_responses_profile",
            "openai_responses": "provider_openai_responses_profile",
            "openrouter": "provider_openrouter_profile",
            "pi_messages": "provider_pi_messages_profile",
            "vertex": "provider_vertex_profile",
        }
        for source, target in profiles.items():
            with self.subTest(source=source):
                plan = self.plan(f"tests/provider_protocol_profile/{source}.rs")
                self.assertEqual(plan["scope"], "focused")
                self.assertEqual({row["target"] for row in plan["tests"]}, {target})
                args = plan["tests"][0]["args"]
                self.assertIn("--no-default-features", args)
                self.assertEqual(args[args.index("--features") + 1],
                                 target.removesuffix("_profile").replace("_", "-"))

    def test_profile_rules_validate_filters_and_include_cross_layer_callers(self):
        for name in ("mapped", "openai", "openai_responses"):
            with self.subTest(name=name):
                plan = self.plan(f"src/providers/{name}.rs")
                targets = {row["target"] for row in plan["tests"]}
                self.assertTrue({"api_mapped_provider", "provider_collection"}.issubset(targets))

    def test_gateway_and_completions_include_their_runtime_and_isolated_owners(self):
        for path, required in (
            ("src/providers/pi_messages.rs",
             {"radius_builtin_catalog", "provider_pi_messages_profile", "external_sdk"}),
            ("src/api/openai_completions.rs",
             {"openai_chat_fixtures", "provider_openai_chat_profile", "external_sdk"}),
        ):
            with self.subTest(path=path):
                plan = self.plan(path)
                self.assertEqual(plan["scope"], "focused")
                self.assertTrue(required.issubset({row["target"] for row in plan["tests"]}))

    def test_unknown_and_feature_paths_do_not_hide_a_broad_check_requirement(self):
        for path in ("Cargo.toml", "src/providers/collection.rs", "src/transcript.rs", "tests/missing.rs"):
            with self.subTest(path=path):
                plan = self.plan("src/api/llama_cpp_classify.rs", path)
                self.assertEqual(plan["scope"], "broader checks required")
                self.assertTrue(any(path in reason for reason in plan["expansion"]))
                self.assertTrue(plan["tests"])

    def test_removed_helper_records_incomplete_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / "crates/opi-ai/tests"
            root.mkdir(parents=True)
            (root / "contract.rs").write_text('#[path = "support/retired.rs"]\nmod old;', encoding="utf-8")
            rows = [{"target": "contract", "source": "tests/contract.rs", "required-features": "-",
                     "cases": "contract", "runners": "-"}]
            plan = selector.make_plan(rows, ["crates/opi-ai/tests/support/retired.rs"], [], repo=repo)
            self.assertTrue(plan["expansion"])
            self.assertEqual([r["target"] for r in plan["tests"]], ["contract"])

    def test_nested_module_consumers_exclude_comments_and_literal_examples(self):
        with tempfile.TemporaryDirectory() as directory:
            crate = Path(directory)
            files = {
                "tests/root.rs": '''
                    // #[path = "ghost.rs"] mod example;
                    const EXAMPLE: &str = r#"mod phantom; include!("ghost.rs");"#;
                    mod ordinary { mod nested; }
                    include!(r#"shared.rs"#);
                ''',
                "tests/ordinary/nested/mod.rs": '#[path = "detail.rs"] mod implementation;',
                "tests/ordinary/nested/detail.rs": "",
                "tests/shared.rs": "",
            }
            for name, content in files.items():
                path = crate / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            resolved = selector.test_inputs(crate, "tests/root.rs")
            self.assertEqual({p.relative_to(crate).as_posix() for p in resolved}, files.keys())

    def test_cli_rejects_unknown_target(self):
        result = subprocess.run([sys.executable, selector.__file__, "--target", "no-such-target"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("unknown inventory target: no-such-target", result.stderr)


if __name__ == "__main__":
    unittest.main()

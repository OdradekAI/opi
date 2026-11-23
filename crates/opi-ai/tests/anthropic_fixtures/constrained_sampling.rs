//! Anthropic strict wire and constrained sampling fixtures.

use super::text_fixture;
use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::StopReason;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// --- task 20.11: constrained sampling substrate (V-34/AIR-040) ---

mod constrained_sampling_tests {
    use opi_ai::constrained_sampling::{
        AnthropicStrictKeywordAdmission, ConstrainedSampling, GrammarInputBuffer,
        NoRejectedKeywords, StrictKeywordAdmission, StrictPreference,
        append_grammar_tool_input_delta, make_strict_json_schema, normalize_wire_arguments_nulls,
        resolve_grammar_constrained_sampling, resolve_json_schema_strict_sampling,
    };
    use opi_ai::message::ToolDef;
    use serde_json::{Value, json};

    fn object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "required_str": {"type": "string"},
                "optional_num": {"type": "number"},
                "nullable_opt": {"type": ["string", "null"]}
            },
            "required": ["required_str"]
        })
    }

    fn strict_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "tool".into(),
            description: "d".into(),
            input_schema: object_schema(),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    #[test]
    fn strict_conversion_clones_and_forces_required_nullable() {
        let original = object_schema();
        let converted = make_strict_json_schema(&original, &AnthropicStrictKeywordAdmission)
            .expect("plain object schema converts");
        assert_eq!(
            original,
            object_schema(),
            "the trusted schema is never mutated"
        );
        assert_eq!(converted["additionalProperties"], json!(false));
        assert_eq!(
            converted["required"],
            json!(["required_str", "optional_num", "nullable_opt"]),
            "every property becomes required"
        );
        assert_eq!(
            converted["properties"]["optional_num"],
            json!({"anyOf": [{"type": "number"}, {"type": "null"}]}),
            "optional non-nullable properties are wrapped nullable"
        );
        assert_eq!(
            converted["properties"]["nullable_opt"],
            json!({"type": ["string", "null"]}),
            "already-nullable properties are not double-wrapped"
        );
        assert_eq!(
            converted["properties"]["required_str"],
            json!({"type": "string"}),
            "required properties are unchanged"
        );
    }

    #[test]
    fn strict_conversion_rejects_unsupported_shapes() {
        for schema in [
            json!({"type": "object", "$ref": "#/x"}),
            json!({"type": "object", "oneOf": [{"type": "object"}]}),
            json!({"type": "array", "items": [{"type": "string"}]}),
            json!(true),
            json!({"properties": {"a": {"type": "string"}}}),
            json!({"type": "object", "additionalProperties": true, "properties": {}}),
            json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["b"]}),
            json!({"type": "object", "anyOf": [{"type": "object"}]}),
            json!({"type": "object", "properties": {"a": {"anyOf": []}}}),
            json!({"type": "string"}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_err(),
                "schema {schema} must be rejected for strict wire projection"
            );
        }
    }

    #[test]
    fn strict_required_shape_is_validated_with_or_without_properties() {
        for properties in [None, Some(json!({"path": {"type": "string"}}))] {
            for required in [
                Value::Null,
                json!("path"),
                json!(1),
                json!(true),
                json!({"path": true}),
                json!([1]),
                json!(["path", false]),
                json!(["unknown"]),
            ] {
                let mut schema = json!({"type": "object", "required": required});
                if let Some(properties) = &properties {
                    schema["properties"] = properties.clone();
                }
                let original = schema.clone();
                for admission in [
                    &AnthropicStrictKeywordAdmission as &dyn StrictKeywordAdmission,
                    &NoRejectedKeywords,
                ] {
                    let error = make_strict_json_schema(&schema, admission)
                        .expect_err("present required must be a string array of known properties");
                    assert!(error.to_string().contains("required"));
                    assert_eq!(
                        schema, original,
                        "failed conversion must preserve the input"
                    );
                }
            }
        }
        for schema in [
            json!({"type": "object"}),
            json!({"type": "object", "required": []}),
            json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
            json!({"type": "object", "properties": {"child": {"type": "object", "required": []}}}),
            // The pinned source keeps object-only keywords inert on scalar nodes.
            json!({"type": "object", "properties": {"scalar": {"type": "string", "required": 1}}}),
        ] {
            let original = schema.clone();
            assert!(make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_ok());
            assert_eq!(schema, original);
        }
        let nested = json!({"type": "object", "properties": {"child": {"type": "object", "required": null}}});
        assert!(make_strict_json_schema(&nested, &AnthropicStrictKeywordAdmission).is_err());
    }

    #[test]
    fn anthropic_keyword_admission_rejects_numeric_and_unknown_formats() {
        // The Anthropic strict subset rejects these keywords with any value.
        for schema in [
            json!({"type": "object", "properties": {"a": {"type": "integer", "minimum": 1}}}),
            json!({"type": "object", "properties": {"a": {"type": "array", "maxItems": 2}}}),
            json!({"type": "object", "properties": {"a": {"type": "string", "format": "semver"}}}),
            json!({"type": "object", "properties": {"a": {"type": "array", "minItems": 2}}}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_err(),
                "schema {schema} hits an Anthropic-rejected keyword"
            );
        }
        // minItems 0/1 and the documented string formats stay admissible.
        for schema in [
            json!({"type": "object", "properties": {"a": {"type": "array", "items": {"type": "string"}, "minItems": 0}}}),
            json!({"type": "object", "properties": {"a": {"type": "string", "format": "uuid"}}}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_ok(),
                "schema {schema} is inside the Anthropic strict subset"
            );
        }
    }

    #[test]
    fn prefer_falls_back_non_strict_require_fails_typed() {
        // Wire without strict support: prefer projects non-strict.
        assert_eq!(
            resolve_json_schema_strict_sampling(
                &strict_tool(StrictPreference::Prefer),
                false,
                &AnthropicStrictKeywordAdmission
            )
            .expect("prefer never fails on unsupported wires"),
            None
        );
        // ...but require fails before dispatch.
        let require_unsupported = resolve_json_schema_strict_sampling(
            &strict_tool(StrictPreference::Require),
            false,
            &AnthropicStrictKeywordAdmission,
        );
        assert!(require_unsupported.is_err(), "require must fail typed");

        // Wire with strict support: both prefer and require resolve strict.
        for preference in [StrictPreference::Prefer, StrictPreference::Require] {
            assert_eq!(
                resolve_json_schema_strict_sampling(
                    &strict_tool(preference),
                    true,
                    &AnthropicStrictKeywordAdmission
                )
                .unwrap(),
                Some(true)
            );
        }
        // A schema the wire cannot convert: prefer falls back, require fails.
        let mut unsupported = strict_tool(StrictPreference::Prefer);
        unsupported.input_schema =
            json!({"type": "object", "properties": {"a": {"type": "integer", "minimum": 1}}});
        assert_eq!(
            resolve_json_schema_strict_sampling(
                &unsupported,
                true,
                &AnthropicStrictKeywordAdmission
            )
            .unwrap(),
            None
        );
        unsupported.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        });
        assert!(
            resolve_json_schema_strict_sampling(
                &unsupported,
                true,
                &AnthropicStrictKeywordAdmission
            )
            .is_err()
        );
        // A tool without constrained sampling is never strict.
        let mut plain = strict_tool(StrictPreference::Prefer);
        plain.constrained_sampling = None;
        assert_eq!(
            resolve_json_schema_strict_sampling(&plain, true, &AnthropicStrictKeywordAdmission)
                .unwrap(),
            None
        );
    }

    fn grammar_tool(variants: serde_json::Value) -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "d".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: variants
                    .get("lark")
                    .and_then(|v| v.as_str().map(String::from)),
                openai_regex: variants
                    .get("regex")
                    .and_then(|v| v.as_str().map(String::from)),
            }),
        }
    }

    #[test]
    fn grammar_resolution_prefers_lark_over_regex() {
        let both = grammar_tool(json!({"lark": "start: WORD", "regex": "^a+$"}));
        let grammar = resolve_grammar_constrained_sampling(&both, true)
            .expect("valid grammar tool resolves")
            .expect("supported target yields a grammar");
        assert_eq!(grammar.format, "lark");
        assert_eq!(grammar.definition, "start: WORD");
        assert_eq!(grammar.input_property, "input");

        let regex_only = grammar_tool(json!({"regex": "^a+$"}));
        let grammar = resolve_grammar_constrained_sampling(&regex_only, true)
            .unwrap()
            .expect("regex-only grammar resolves");
        assert_eq!(grammar.format, "regex");
    }

    #[test]
    fn grammar_unsupported_target_falls_back_to_plain_json() {
        let both = grammar_tool(json!({"lark": "start: WORD"}));
        // Unsupported target: plain-JSON projection, no failure.
        assert!(
            resolve_grammar_constrained_sampling(&both, false)
                .expect("unsupported target never fails")
                .is_none()
        );
    }

    #[test]
    fn grammar_supported_target_without_variant_or_schema_fails() {
        for tool in [
            grammar_tool(json!({})),
            grammar_tool(json!({"lark": "   "})),
        ] {
            assert!(
                resolve_grammar_constrained_sampling(&tool, true).is_err(),
                "supported target with no valid variant fails before dispatch"
            );
        }
        // Schema shape violations fail with details.
        let mut bad_schema = grammar_tool(json!({"lark": "start: A"}));
        bad_schema.input_schema = json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a", "b"]});
        assert!(resolve_grammar_constrained_sampling(&bad_schema, true).is_err());
        bad_schema.input_schema =
            json!({"type": "object", "properties": {"a": {"type": "number"}}, "required": ["a"]});
        assert!(resolve_grammar_constrained_sampling(&bad_schema, true).is_err());
    }

    #[test]
    fn grammar_input_buffer_wraps_increments_and_closes() {
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        let first = append_grammar_tool_input_delta(&mut buffer, "hel", false)
            .expect("first delta appends");
        assert_eq!(first, Some(r#"{"input":"hel"#.into()));
        let next = append_grammar_tool_input_delta(&mut buffer, "hello", false)
            .expect("monotonic delta appends");
        assert_eq!(next, Some(r#"lo"#.into()));
        // Empty non-closing delta produces nothing.
        assert_eq!(
            append_grammar_tool_input_delta(&mut buffer, "hello", false).unwrap(),
            None
        );
        let closed = append_grammar_tool_input_delta(&mut buffer, "hello!", true)
            .expect("close wraps the object");
        assert_eq!(closed, Some(r#"!""#.to_string() + "}"));

        // Non-monotonic input fails typed.
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        append_grammar_tool_input_delta(&mut buffer, "abc", false).unwrap();
        assert!(append_grammar_tool_input_delta(&mut buffer, "abd", false).is_err());
        // Changed-after-close fails typed.
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        append_grammar_tool_input_delta(&mut buffer, "abc", true).unwrap();
        assert!(append_grammar_tool_input_delta(&mut buffer, "abcd", true).is_err());
    }

    #[test]
    fn wire_null_arguments_normalize_per_original_schema() {
        let original = object_schema();
        let mut arguments = json!({
            "required_str": "x",
            "optional_num": null,
            "nullable_opt": null
        });
        normalize_wire_arguments_nulls(&original, &mut arguments);
        assert_eq!(
            arguments,
            json!({"required_str": "x", "nullable_opt": null}),
            "nulls for originally optional non-nullable properties are dropped; \
             explicitly nullable positions keep their null"
        );
    }
}

// --- task 20.11: Anthropic strict wire projection (V-34/AIR-040) ---

mod anthropic_strict_wire_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::ToolDef;
    use opi_ai::model_info::{
        AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
    };
    use opi_ai::transcript::Context;
    use std::sync::Arc;

    fn strict_tool(name: &str, strict: StrictPreference, schema: serde_json::Value) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: "d".into(),
            input_schema: schema,
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn plain_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "limit": {"type": "number"}
            },
            "required": ["path"]
        })
    }

    fn strict_capable_provider(base: Option<String>, strict_tools: bool) -> AnthropicProvider {
        let model = ModelInfo::new(
            "strict-model",
            "Strict Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100),
        )
        .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: false,
            force_adaptive_thinking: false,
            supports_temperature: true,
            supports_strict_tools: strict_tools,
            supports_mid_convo_system_messages: false,
            supports_mid_convo_tool_changes: false,
            ..Default::default()
        }))
        .unwrap();
        AnthropicProvider::for_route(
            "test".into(),
            vec![model],
            base,
            Default::default(),
            Arc::new(opi_ai::http::HttpClient::new()),
            false,
        )
    }

    fn request_with_tools(tools: Vec<ToolDef>) -> Request {
        Request {
            projection: Default::default(),
            transport: Default::default(),
            model: "test:strict-model".into(),
            context: opi_ai::transcript::normalize_context(Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 0,
                })],
                tools,
            })
            .expect("valid context declarations"),
            max_tokens: Some(64),
            temperature: None,
            sampling_params: Default::default(),
            thinking: ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

    fn tool_def_in_body(
        provider: &AnthropicProvider,
        request: &Request,
        index: usize,
    ) -> serde_json::Value {
        let body = provider.build_request_body(request).expect("body builds");
        body["tools"][index].clone()
    }

    #[test]
    fn strict_prefer_and_require_project_converted_wire_schema() {
        let provider = strict_capable_provider(None, true);
        let prefer_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "prefer-tool",
                StrictPreference::Prefer,
                plain_schema(),
            )]),
            0,
        );
        assert_eq!(prefer_tool["strict"], serde_json::json!(true));
        assert_eq!(
            prefer_tool["input_schema"]["additionalProperties"],
            serde_json::json!(false)
        );
        assert_eq!(
            prefer_tool["input_schema"]["required"],
            serde_json::json!(["path", "limit"])
        );
        assert_eq!(
            prefer_tool["input_schema"]["properties"]["limit"],
            serde_json::json!({"anyOf": [{"type": "number"}, {"type": "null"}]})
        );

        let require_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "require-tool",
                StrictPreference::Require,
                plain_schema(),
            )]),
            0,
        );
        assert_eq!(require_tool["strict"], serde_json::json!(true));

        let plain_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![ToolDef {
                name: "plain-tool".into(),
                description: "d".into(),
                input_schema: plain_schema(),
                constrained_sampling: None,
            }]),
            0,
        );
        assert!(
            plain_tool.get("strict").is_none(),
            "tools without constrained sampling never carry strict"
        );
        assert_eq!(
            plain_tool["input_schema"],
            plain_schema(),
            "the trusted schema projects unchanged for non-strict tools"
        );
    }

    #[test]
    fn strict_prefer_unconvertible_schema_stays_non_strict_on_the_wire() {
        let provider = strict_capable_provider(None, true);
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string", "minLength": 1}}
        });
        let _ = unconvertible;
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"n": {"type": "integer", "minimum": 1}}
        });
        let tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "prefer-tool",
                StrictPreference::Prefer,
                unconvertible.clone(),
            )]),
            0,
        );
        assert!(tool.get("strict").is_none());
        assert_eq!(tool["input_schema"], unconvertible);
    }

    #[test]
    fn malformed_required_prefer_preserves_raw_schema_on_the_wire() {
        let provider = strict_capable_provider(None, true);
        for required in [
            serde_json::Value::Null,
            serde_json::json!("path"),
            serde_json::json!(1),
            serde_json::json!(false),
            serde_json::json!({"path": true}),
            serde_json::json!([1]),
            serde_json::json!(["unknown"]),
        ] {
            for properties in [None, Some(serde_json::json!({"path": {"type": "string"}}))] {
                let mut schema = serde_json::json!({"type": "object", "required": required});
                if let Some(properties) = properties {
                    schema["properties"] = properties;
                }
                let request = request_with_tools(vec![strict_tool(
                    "prefer",
                    StrictPreference::Prefer,
                    schema.clone(),
                )]);
                let canonical = request.context.clone();
                let body = provider.build_request_body(&request).unwrap();
                assert!(body["tools"][0].get("strict").is_none());
                assert_eq!(body["tools"][0]["input_schema"], schema);
                assert_eq!(request.context, canonical);
            }
        }
    }

    #[tokio::test]
    async fn required_preauth_checks_native_history_but_excludes_collapsed_declarations() {
        use opi_ai::api::contract::SimpleOptions;
        use opi_ai::auth::{AuthResolver, ResolvedAuth};
        use opi_ai::models::{ChatModelRecord, Models};
        use opi_ai::transcript::{SystemMessage, ToolReference};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingAuth(Arc<AtomicUsize>);
        impl AuthResolver for CountingAuth {
            fn resolve(&self) -> opi_ai::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(opi_ai::test_support::resolved_auth()) })
            }
        }
        for inline in [false, true] {
            for native in [true, false] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(text_fixture()),
                    )
                    .mount(&server)
                    .await;
                let mut provider = strict_capable_provider(Some(server.uri()), true);
                let mut model = provider.models()[0].clone();
                model.capabilities = ModelCapabilities::new(100_000, 1_000);
                let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
                    unreachable!()
                };
                compat.supports_mid_convo_system_messages = native;
                compat.supports_mid_convo_tool_changes = native;
                provider.replace_model_catalog(vec![model.clone()]).unwrap();
                let malformed = strict_tool(
                    "retired",
                    StrictPreference::Require,
                    serde_json::json!({"type": "object", "required": null}),
                );
                let initial = if inline {
                    strict_tool("seed", StrictPreference::Require, plain_schema())
                } else {
                    malformed.clone()
                };
                let mut request = request_with_tools(vec![initial]);
                if inline {
                    request
                        .context
                        .append_system_change(SystemMessage {
                            tools_added: vec![malformed],
                            ..Default::default()
                        })
                        .unwrap();
                }
                request
                    .context
                    .append_system_change(SystemMessage {
                        tools_removed: vec![ToolReference {
                            name: "retired".into(),
                        }],
                        ..Default::default()
                    })
                    .unwrap();
                let canonical = request.context.clone();
                let direct = provider.build_request_body(&request);
                if native {
                    assert!(direct.unwrap_err().to_string().contains("required"));
                } else {
                    assert!(
                        direct.is_ok(),
                        "collapse must exclude the retired declaration"
                    );
                }
                let count = Arc::new(AtomicUsize::new(0));
                let mut models = Models::new();
                models
                    .register_route(
                        Box::new(provider),
                        Arc::new(CountingAuth(count.clone())),
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                let result = models
                    .complete_simple(
                        ChatModelRecord::new("test", model),
                        request.clone(),
                        SimpleOptions::default(),
                    )
                    .await;
                if native {
                    let failure =
                        result.expect_err("native history still transmits the declaration");
                    assert!(failure.cause().to_string().contains("required"));
                    assert_eq!(count.load(Ordering::SeqCst), 0);
                    assert!(server.received_requests().await.unwrap().is_empty());
                } else {
                    assert_eq!(result.unwrap().stop_reason, StopReason::Stop);
                    assert_eq!(count.load(Ordering::SeqCst), 1);
                    assert_eq!(server.received_requests().await.unwrap().len(), 1);
                }
                assert_eq!(request.context, canonical);
            }
        }
    }

    #[test]
    fn strict_require_fails_typed_before_any_dispatch_when_unconvertible() {
        let provider = strict_capable_provider(None, true);
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"n": {"type": "integer", "minimum": 1}}
        });
        let error = provider
            .build_request_body(&request_with_tools(vec![strict_tool(
                "require-tool",
                StrictPreference::Require,
                unconvertible,
            )]))
            .expect_err("require with an unconvertible schema must fail");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
    }

    #[tokio::test]
    async fn strict_require_fails_before_http_on_strict_unsupported_wire() {
        let server = MockServer::start().await;
        let provider = strict_capable_provider(Some(server.uri()), false);
        let error = provider
            .stream_prepared(
                request_with_tools(vec![strict_tool(
                    "require-tool",
                    StrictPreference::Require,
                    plain_schema(),
                )]),
                opi_ai::test_support::resolved_auth(),
            )
            .next()
            .await
            .expect("result")
            .expect_err("require on a strict-unsupported wire fails");
        let ProviderError::StreamFailure(failure) = &error else {
            panic!("accepted projection retains its failure carrier: {error:?}")
        };
        assert!(failure.message().projection.is_some());
        let cause = failure.cause();
        assert!(
            matches!(cause, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "zero HTTP dispatch for an unmet require"
        );
    }
}

// --- task 20.11: the single source-qualified Anthropic strict downgrade
// variant (V-34/V-36/AIR-042/P20-S11) ---

mod anthropic_strict_variant_tests {
    use super::*;
    use opi_ai::auth::{AuthProvenanceSource, StaticAuthResolver};
    use opi_ai::callbacks::RequestCallbacks;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::http::HttpClientBuilder;
    use opi_ai::message::ToolDef;
    use opi_ai::model_info::{
        AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
    };
    use opi_ai::models::Models;
    use opi_ai::retry::AcquisitionPolicy;
    use opi_ai::stream::ChatStream;
    use opi_ai::transcript::Context;
    use secrecy::SecretString;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn strict_tool(name: &str, strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn anthropic_sse_ok() -> String {
        text_fixture().to_owned()
    }

    async fn strict_variant_setup(
        server: &MockServer,
        first_status: u16,
        first_body: String,
    ) -> Models {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(first_status).set_body_string(first_body))
            .up_to_n_times(1)
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
            .mount(server)
            .await;
        strict_models(server.uri())
    }

    fn strict_models(base_url: String) -> Models {
        strict_models_with_inline(base_url, false)
    }

    fn strict_models_with_inline(base_url: String, inline: bool) -> Models {
        let model = ModelInfo::new(
            "strict-model",
            "Strict Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100),
        )
        .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: false,
            force_adaptive_thinking: false,
            supports_temperature: true,
            supports_strict_tools: true,
            supports_mid_convo_system_messages: inline,
            supports_mid_convo_tool_changes: inline,
            ..Default::default()
        }))
        .unwrap();
        let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
        let provider = AnthropicProvider::for_route(
            "test".into(),
            vec![model],
            Some(base_url),
            Default::default(),
            client,
            false,
        );
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    SecretString::from("test-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata {
                    openai_compatible: false,
                    profile: None,
                },
            )
            .expect("register route");
        models
    }

    fn variant_request(tools: Vec<ToolDef>, callbacks: RequestCallbacks) -> Request {
        Request {
            projection: Default::default(),
            transport: Default::default(),
            model: "test:strict-model".into(),
            context: opi_ai::transcript::normalize_context(Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 0,
                })],
                tools,
            })
            .expect("valid context declarations"),
            max_tokens: Some(64),
            temperature: None,
            sampling_params: Default::default(),
            thinking: ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks,
        }
    }

    fn strict_limits_400() -> String {
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"Too many strict tools (65). Reduce the number of strict tools or convert some to non-strict."}}"#.into()
    }

    async fn wait_retry(admission: opi_ai::retry::RetryAdmission) {
        tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;
    }

    async fn ordinary_strict_variant_contract(direct: bool) {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let payload_calls = Arc::new(AtomicUsize::new(0));
        let payload_probe = payload_calls.clone();
        let header_calls = Arc::new(AtomicUsize::new(0));
        let header_probe = header_calls.clone();
        let responses = Arc::new(std::sync::Mutex::new(Vec::new()));
        let response_probe = responses.clone();
        let request = variant_request(
            vec![
                strict_tool("prefer-tool", StrictPreference::Prefer),
                strict_tool("require-tool", StrictPreference::Require),
            ],
            RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_| {
                    payload_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }))
                .with_header_transform(Arc::new(move |headers| {
                    header_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(headers.to_vec())
                }))
                .with_on_response(Arc::new(move |status, _| {
                    response_probe.lock().unwrap().push(status);
                    Ok(())
                })),
        );
        let message = if direct {
            let (provider, _) = models.resolve("test:strict-model").unwrap();
            ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_auth()),
            )
            .result()
            .await
        } else {
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(
                        "test",
                        models.get_model("test", "strict-model").unwrap(),
                    ),
                    request,
                    Default::default(),
                )
                .await
        }
        .expect("ordinary entry drives its single admitted strict variant");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
        let first: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        let second: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        assert_eq!(first["tools"][0]["strict"], true);
        assert!(second["tools"][0].get("strict").is_none());
        assert_eq!(second["tools"][1]["strict"], true);
        assert_eq!(first["model"], second["model"]);
        assert_eq!(sent[0].headers["x-api-key"], sent[1].headers["x-api-key"]);
        assert_eq!(payload_calls.load(Ordering::SeqCst), 2);
        assert_eq!(header_calls.load(Ordering::SeqCst), 1);
        assert_eq!(*responses.lock().unwrap(), vec![200]);
        assert_eq!(
            message
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.kind == "anthropic_strict_tools_fallback")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn ordinary_models_drives_the_single_strict_variant() {
        ordinary_strict_variant_contract(false).await;
    }

    #[tokio::test]
    async fn inline_preferred_tools_participate_in_the_single_strict_variant() {
        for initial_preferred in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
                .up_to_n_times(1)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
                .mount(&server)
                .await;
            let models = strict_models_with_inline(server.uri(), true);
            let mut initial = strict_tool("initial", StrictPreference::Prefer);
            if !initial_preferred {
                initial.constrained_sampling = None;
            }
            let payload_calls = Arc::new(AtomicUsize::new(0));
            let observed = payload_calls.clone();
            let mut request = variant_request(
                vec![],
                RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                })),
            );
            request.context = opi_ai::transcript::normalize_context(Context {
                messages: vec![
                    Message::System(opi_ai::transcript::SystemMessage {
                        tools_added: vec![initial],
                        ..Default::default()
                    }),
                    Message::User(UserMessage {
                        content: vec![InputContent::Text { text: "go".into() }],
                        timestamp_ms: 0,
                    }),
                    Message::System(opi_ai::transcript::SystemMessage {
                        tools_added: vec![
                            ToolDef {
                                description: "inline replacement".into(),
                                ..strict_tool("initial", StrictPreference::Prefer)
                            },
                            strict_tool("later-require", StrictPreference::Require),
                        ],
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            })
            .unwrap();
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(
                        "test",
                        models.get_model("test", "strict-model").unwrap(),
                    ),
                    request,
                    Default::default(),
                )
                .await
                .expect("inline Prefer admits the strict variant");
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 2);
            assert_eq!(payload_calls.load(Ordering::SeqCst), 2);
            for (index, request) in sent.iter().enumerate() {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let definitions: Vec<_> = body["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|message| message["content"].as_array().into_iter().flatten())
                    .filter(|block| block["type"] == "tool_addition")
                    .map(|block| &block["tool"]["definition"])
                    .collect();
                assert_eq!(definitions.len(), 2);
                assert_eq!(definitions[0]["description"], "inline replacement");
                assert_eq!(definitions[0].get("strict").is_some(), index == 0);
                assert_eq!(definitions[1]["strict"], true);
            }
        }
    }

    #[tokio::test]
    async fn ordinary_direct_adapter_drives_the_single_strict_variant() {
        ordinary_strict_variant_contract(true).await;
    }

    #[tokio::test]
    async fn successful_headers_seal_acquisition_before_a_body_timeout() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n")
                .unwrap();
            socket.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(250));
        });
        let models = strict_models(format!("http://127.0.0.1:{port}"));
        let mut request = variant_request(vec![], RequestCallbacks::default());
        request.timeout = Some(std::time::Duration::from_millis(50));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                request,
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .unwrap();
        let error = ChatStream::new(call.start_attempt().unwrap())
            .result()
            .await
            .expect_err("successful headers precede the body timeout");
        assert!(matches!(error.cause(), ProviderError::Timeout));
        assert!(call.retry_admission().is_none(), "no post-handoff resend");
        assert_eq!(
            call.acquisition_seal(),
            Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
        );
        peer.join().unwrap();
    }

    #[tokio::test]
    async fn strict_marker_400_rebuilds_one_variant_with_prefer_removed() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;

        let payload_calls = Arc::new(AtomicUsize::new(0));
        let payload_probe = Arc::clone(&payload_calls);
        let transform_calls = Arc::new(AtomicUsize::new(0));
        let transform_probe = Arc::clone(&transform_calls);
        let response_statuses = Arc::new(std::sync::Mutex::new(Vec::<u16>::new()));
        let response_probe = Arc::clone(&response_statuses);

        let request = variant_request(
            vec![
                strict_tool("prefer-tool", StrictPreference::Prefer),
                strict_tool("require-tool", StrictPreference::Require),
            ],
            RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_body| {
                    payload_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }))
                .with_header_transform(Arc::new(move |merged| {
                    transform_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(merged.to_vec())
                }))
                .with_on_response(Arc::new(move |status, _headers| {
                    response_probe.lock().unwrap().push(status);
                    Ok(())
                })),
        );

        let mut policy = AcquisitionPolicy::with_max_retries(0);
        policy.jitter_seed = Some(0);
        let call = models
            .prepare_call_with_policy("test:strict-model", request, policy)
            .await
            .expect("prepare call");

        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("the strict-limit 400 fails the first variant");
        assert!(
            matches!(first.cause(), ProviderError::StrictToolLimits { .. }),
            "source-qualified strict-limit 400 is typed, got {:?}",
            first.cause()
        );
        // Not an ordinary transport retry: no admission, but one variant.
        assert!(call.retry_admission().is_none());
        let strict_admission = call
            .strict_variant_admission()
            .expect("exactly one downgrade variant is admitted");
        assert_eq!(strict_admission.reason, "strict_tool_limits_exceeded");

        let message = ChatStream::new(call.start_attempt().expect("variant dispatch"))
            .result()
            .await
            .expect("variant completes");
        // The variant admission is consumed.
        assert!(call.strict_variant_admission().is_none());

        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent.len(),
            2,
            "N=0 still allows two sends (one per variant)"
        );
        let first_body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        let second_body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        assert_eq!(first_body["tools"][0]["strict"], serde_json::json!(true));
        assert_eq!(first_body["tools"][1]["strict"], serde_json::json!(true));
        assert!(
            second_body["tools"][0].get("strict").is_none(),
            "the rebuilt variant removes only the prefer-strict flag"
        );
        assert_eq!(
            second_body["tools"][1]["strict"],
            serde_json::json!(true),
            "require stays strict in the rebuilt variant"
        );

        assert_eq!(
            payload_calls.load(Ordering::SeqCst),
            2,
            "onPayload re-runs exactly once on the new variant"
        );
        assert_eq!(
            transform_calls.load(Ordering::SeqCst),
            1,
            "the header transform does not re-run on the strict branch"
        );
        assert_eq!(
            *response_statuses.lock().unwrap(),
            vec![200],
            "onResponse fires only on the successfully acquired response"
        );
        assert_eq!(
            message
                .diagnostics
                .iter()
                .filter(|d| d.kind == "anthropic_strict_tools_fallback")
                .count(),
            1,
            "exactly one downgrade diagnostic is recorded on the message"
        );
    }

    #[tokio::test]
    async fn non_marker_400_does_not_branch() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(
            &server,
            400,
            r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens is required"}}"#.into(),
        )
        .await;
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("plain 400 fails");
        assert!(!matches!(
            first.cause(),
            ProviderError::StrictToolLimits { .. }
        ));
        assert!(call.retry_admission().is_none());
        assert!(
            call.strict_variant_admission().is_none(),
            "other 400s never branch"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn marker_400_without_prefer_strict_tools_does_not_branch() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("require-tool", StrictPreference::Require)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("marker 400 without prefer-strict tools fails");
        assert!(
            call.strict_variant_admission().is_none(),
            "the downgrade is qualified by an actually-dispatched prefer-strict tool"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn second_variant_marker_400_surfaces_without_a_third_dispatch() {
        let server = MockServer::start().await;
        // Both sends answer with the marker 400.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
            .mount(&server)
            .await;
        let models = strict_models(server.uri());
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("first variant fails");
        assert!(call.strict_variant_admission().is_some());
        ChatStream::new(call.start_attempt().expect("variant dispatch"))
            .result()
            .await
            .expect_err("the rebuilt variant surfaces the second 400");
        assert!(
            call.strict_variant_admission().is_none(),
            "at most one downgrade variant exists"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn ordinary_retry_budget_is_per_variant_with_total_bound() {
        let server = MockServer::start().await;
        // Variant 1: 429 (retryable), then the marker 400.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        // Variant 2: 429, then success.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
            .mount(&server)
            .await;
        let models = strict_models(server.uri());

        let mut policy = AcquisitionPolicy::with_max_retries(1);
        policy.jitter_seed = Some(0);
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                policy,
            )
            .await
            .expect("prepare call");

        ChatStream::new(call.start_attempt().expect("v1 send 1"))
            .result()
            .await
            .expect_err("429");
        wait_retry(call.retry_admission().expect("ordinary retry")).await;
        ChatStream::new(call.start_attempt().expect("v1 send 2"))
            .result()
            .await
            .expect_err("marker 400");
        assert!(call.retry_admission().is_none(), "per-variant budget spent");
        assert!(call.strict_variant_admission().is_some());
        ChatStream::new(call.start_attempt().expect("v2 send 1"))
            .result()
            .await
            .expect_err("429 again");
        wait_retry(call.retry_admission().expect("fresh per-variant budget")).await;
        ChatStream::new(call.start_attempt().expect("v2 send 2"))
            .result()
            .await
            .expect("variant 2 completes within its own 1+N budget");

        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent.len(),
            4,
            "total bound 2x(1+N) with N=1: two per variant, no more"
        );
    }

    #[tokio::test]
    async fn callback_phase_failure_never_branches() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|_body| {
            Err(opi_ai::callbacks::CallbackError::new("boom"))
        }));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    callbacks,
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("callback failure fails the call");
        assert!(matches!(first.cause(), ProviderError::CallbackPhase(_)));
        assert!(call.retry_admission().is_none());
        assert!(
            call.strict_variant_admission().is_none(),
            "callback errors stop dispatch"
        );
        let sent = server.received_requests().await.unwrap();
        assert!(sent.is_empty(), "zero HTTP dispatch for a callback failure");
    }

    #[tokio::test]
    async fn post_payload_wire_without_prefer_strict_does_not_admit_downgrade() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
            let mut body = body.clone();
            for tool in body["tools"].as_array_mut().unwrap() {
                tool.as_object_mut().unwrap().remove("strict");
            }
            Ok(Some(body))
        }));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    callbacks,
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .unwrap();
        ChatStream::new(call.start_attempt().unwrap())
            .result()
            .await
            .expect_err("source marker without actual strict wire stays final");
        assert!(call.strict_variant_admission().is_none());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Task 20.39: complete anthropic-messages chat adapter — native options,
// thinking states, inline-tools chronology, error-turn relocation, and the
// public direct adapter. Sources: the Anthropic Messages wire contract,
// transform-messages.ts, compat-schema.ts (annex rows 37/50/165/176/244).
// ---------------------------------------------------------------------------

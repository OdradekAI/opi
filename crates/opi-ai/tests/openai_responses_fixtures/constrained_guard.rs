//! OpenAI Responses constrained-sampling admission and projection assertions.

use super::fixtures::*;
use super::*;

mod responses_constrained_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::providers::openai_responses::ResponsesConfig;

    fn grammar_tool(lark: Option<&str>, regex: Option<&str>) -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "grammar constrained".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: lark.map(String::from),
                openai_regex: regex.map(String::from),
            }),
        }
    }

    fn json_schema_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "structured".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}, "limit": {"type": "number"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn request_with(tools: Vec<ToolDef>) -> Request {
        let mut request = responses_tool_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools,
        })
        .expect("valid context declarations");
        request
    }

    #[tokio::test]
    async fn models_complete_record_enables_explicit_grammar_support() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string(responses_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let mut model = o3_responses_model();
        if let opi_ai::model_info::WireCompat::OpenAiResponses(compat) = &mut model.compat {
            compat.source.supports_grammar_tools = Some(true);
        } else {
            panic!("Responses compat");
        }
        // The registered default denies grammar; the complete selected record
        // opts in for this call, without changing the registered adapter.
        let (models, handle) =
            registered_responses_models(server.uri(), o3_responses_model()).await;
        let mut request = request_with(vec![grammar_tool(Some("start: WORD"), None)]);
        request.model = "openai-responses:o3".into();
        models
            .complete(
                ChatModelRecord::new("openai-responses", model),
                request,
                handle.options(OpenAiResponsesOptions::default()),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["tools"][0]["type"], "custom");
        assert_eq!(body["tools"][0]["format"]["syntax"], "lark");
    }

    #[test]
    fn grammar_tool_projects_custom_tool_preferring_lark() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let body = provider
            .build_request_body(&request_with(vec![grammar_tool(
                Some("start: WORD"),
                Some("^a+$"),
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("custom"));
        assert_eq!(
            body["tools"][0]["format"],
            serde_json::json!({"type": "grammar", "syntax": "lark", "definition": "start: WORD"}),
            "lark is preferred over regex when both variants are provided"
        );
    }

    #[test]
    fn grammar_unsupported_target_projects_plain_function_with_recorded_loss() {
        let provider = responses_provider();
        let body = provider
            .build_request_body(&request_with(vec![grammar_tool(Some("start: WORD"), None)]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
        assert!(
            body["tools"][0].get("format").is_none(),
            "no grammar format is sent to an unsupported target"
        );
    }

    #[test]
    fn grammar_supported_target_without_variant_fails_before_dispatch() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let error = provider
            .build_request_body(&request_with(vec![grammar_tool(None, Some("   "))]))
            .unwrap_err();
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
    }

    #[test]
    fn strict_json_schema_resolves_per_tool_over_the_wire_default() {
        // With no strict support declared, Prefer stays non-strict and
        // Require fails before dispatch.
        let provider = OpenAiResponsesProvider::new_with_config(None, ResponsesConfig::default());
        let body = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Prefer,
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
        assert!(
            body["tools"][0].get("strict").is_none(),
            "no strict flag without strict support"
        );
        let error = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Require,
            )]))
            .unwrap_err();
        assert!(matches!(error, ProviderError::Config(_)));

        // With strict support: prefer resolves strict with the converted
        // schema (required-all + additionalProperties false).
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let body = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Prefer,
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["strict"], serde_json::json!(true));
        assert_eq!(
            body["tools"][0]["parameters"]["additionalProperties"],
            serde_json::json!(false)
        );
        assert_eq!(
            body["tools"][0]["parameters"]["properties"]["path"],
            serde_json::json!({"type": "string"}),
            "required properties stay unchanged"
        );
        assert_eq!(
            body["tools"][0]["parameters"]["properties"]["limit"],
            serde_json::json!({"anyOf": [{"type": "number"}, {"type": "null"}]}),
            "optional properties wrap nullable"
        );
    }

    #[test]
    fn malformed_required_is_typed_and_preserves_the_trusted_schema() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
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
                let mut tool = json_schema_tool(StrictPreference::Require);
                tool.input_schema = serde_json::json!({"type": "object", "required": required});
                if let Some(properties) = properties {
                    tool.input_schema["properties"] = properties;
                }
                let request = request_with(vec![tool]);
                let canonical = request.context.clone();
                let error = provider.build_request_body(&request).unwrap_err();
                assert_eq!(request.context, canonical);
                let ProviderError::Config(summary) = error else {
                    panic!("required admission must fail typed: {error:?}")
                };
                assert!(summary.as_str().contains("required"));
            }
        }
    }

    #[tokio::test]
    async fn malformed_required_fails_before_native_authentication_and_dispatch() {
        use opi_ai::api::openai_responses::OpenAiResponsesChatApi;
        use opi_ai::model_info::{
            ModelCapabilities, ModelInfo, OpenAiResponsesCompat, OpenAiResponsesSourceCompat,
            WireApi, WireCompat,
        };
        use opi_ai::models::{ChatModelRecord, Models};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        struct CountingAuth(Arc<AtomicUsize>);
        impl opi_ai::auth::AuthResolver for CountingAuth {
            fn resolve(
                &self,
            ) -> opi_ai::BoxAuthFuture<'_, Result<opi_ai::auth::ResolvedAuth, ProviderError>>
            {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(opi_ai::test_support::resolved_bearer_auth()) })
            }
        }
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(responses_sse_ok()),
            )
            .mount(&server)
            .await;
        let model = ModelInfo::new(
            "required-probe",
            "Required probe",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(100_000, 4_096),
        )
        .with_base_url(server.uri())
        .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            source: OpenAiResponsesSourceCompat {
                supports_strict_mode: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }))
        .unwrap();
        let api = OpenAiResponsesChatApi::new(
            Arc::new(OpenAiResponsesProvider::new(Some(server.uri()))),
            model.clone(),
        )
        .unwrap();
        let mut models = Models::new();
        let count = Arc::new(AtomicUsize::new(0));
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(CountingAuth(count.clone())),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("openai-responses", model);
        models
            .complete(
                record.clone(),
                request_with(vec![json_schema_tool(StrictPreference::Require)]),
                handle.options(Default::default()),
            )
            .await
            .expect("valid strict control must authenticate and dispatch");
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        for required in [
            serde_json::Value::Null,
            serde_json::json!("path"),
            serde_json::json!(1),
        ] {
            for properties in [None, Some(serde_json::json!({"path": {"type": "string"}}))] {
                let mut tool = json_schema_tool(StrictPreference::Require);
                tool.input_schema = serde_json::json!({"type": "object", "required": required});
                if let Some(properties) = properties {
                    tool.input_schema["properties"] = properties;
                }
                let failure = models
                    .complete(
                        record.clone(),
                        request_with(vec![tool]),
                        handle.options(Default::default()),
                    )
                    .await
                    .expect_err("malformed required must be rejected before auth");
                let cause = match failure.cause() {
                    ProviderError::StreamFailure(frame) => frame.cause(),
                    cause => cause,
                };
                let ProviderError::Config(summary) = cause else {
                    panic!("typed required failure: {cause:?}")
                };
                assert!(summary.as_str().contains("required"));
                assert_eq!(count.load(Ordering::SeqCst), 1);
                assert_eq!(server.received_requests().await.unwrap().len(), 1);
            }
        }
    }

    fn custom_tool_call_sse() -> String {
        [
            r#"data: {"type":"response.output_item.added","output_index":1,"item":{"type":"custom_tool_call","id":"ctc_1","call_id":"call_1","name":"grammar-tool","input":""}}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.delta","output_index":1,"delta":"hel"}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.delta","output_index":1,"delta":"lo"}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.done","output_index":1,"input":"hello"}"#,
            "",
            r#"data: {"type":"response.output_item.done","output_index":1,"item":{"type":"custom_tool_call","id":"ctc_1","call_id":"call_1","name":"grammar-tool","input":"hello"}}"#,
            "",
            r#"data: {"type":"response.completed","response":{"id":"resp_1","model":"gpt-4o","usage":{"input_tokens":3,"output_tokens":2}}}"#,
            "",
            "",
        ]
        .join("\n")
    }

    #[tokio::test]
    async fn custom_tool_call_input_wraps_incrementally_and_finalizes_raw() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(custom_tool_call_sse()),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let events: Vec<_> = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|event| event.expect("valid custom input deltas must not emit a failure"))
            .collect();

        // The streaming deltas carry the wrapped JSON object text.
        let deltas: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::ToolCallDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            deltas.join(""),
            r#"{"input":"hello"}"#,
            "the raw wire input wraps into the incremental JSON object text"
        );
        // The final tool call keeps the raw input under its property.
        let final_call = events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
                _ => None,
            })
            .expect("custom tool call finalizes");
        assert_eq!(final_call.name, "grammar-tool");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&final_call.arguments).unwrap(),
            serde_json::json!({"input": "hello"})
        );
        assert!(final_call.arguments_error.is_none());
    }

    #[tokio::test]
    async fn custom_tool_initial_input_is_included_in_the_first_wrapped_delta() {
        let server = MockServer::start().await;
        let sse = custom_tool_call_sse()
            .replace(r#""input":"""#, r#""input":"he""#)
            .replace(r#""delta":"hel""#, r#""delta":"l""#);
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let events: Vec<_> = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|event| event.expect("initial input plus wire increments is valid"))
            .collect();
        let delta = events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::ToolCallDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(delta, r#"{"input":"hello"}"#);
    }

    #[tokio::test]
    async fn custom_tool_terminal_item_preserves_input_without_input_done() {
        for added in [true, false] {
            let server = MockServer::start().await;
            let mut wire = Vec::new();
            if added {
                wire.push(serde_json::json!({
                    "type": "response.output_item.added", "output_index": 0,
                    "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                        "call_id": "call_terminal", "name": "grammar-tool", "input": ""}
                }));
            }
            wire.push(serde_json::json!({
                "type": "response.output_item.done", "output_index": 0,
                "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                    "call_id": "call_terminal", "name": "grammar-tool", "input": "hello"}
            }));
            wire.push(serde_json::json!({
                "type": "response.completed", "response": {"id": "resp_terminal",
                    "model": "gpt-4o", "output": []}
            }));
            let sse = wire
                .into_iter()
                .map(|value| format!("data: {value}\n\n"))
                .collect::<String>();
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(sse),
                )
                .mount(&server)
                .await;
            let provider = OpenAiResponsesProvider::new_with_config(
                Some(server.uri()),
                ResponsesConfig {
                    source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                        supports_grammar_tools: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let message = opi_ai::stream::ChatStream::new(provider.stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            ))
            .result()
            .await
            .expect("terminal custom input is a complete valid call");
            let tool = message
                .content
                .iter()
                .find_map(|content| match content {
                    opi_ai::message::AssistantContent::ToolCall { tool_call } => Some(tool_call),
                    _ => None,
                })
                .expect("terminal item creates and finishes its tool slot");
            assert_eq!(tool.id, "call_terminal|ctc_terminal");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&tool.arguments).unwrap(),
                serde_json::json!({"input": "hello"}),
                "terminal input is authoritative whether the added event was present"
            );
            assert!(tool.arguments_error.is_none());
        }
    }

    #[tokio::test]
    async fn custom_tool_call_closed_input_violation_is_observable() {
        let server = MockServer::start().await;
        // The done event's input contradicts the accumulated deltas.
        let broken = custom_tool_call_sse().replace(
            r#""response.custom_tool_call_input.done","output_index":1,"input":"hello""#,
            r#""response.custom_tool_call_input.done","output_index":1,"input":"hexlo""#,
        );
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(broken),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        use futures_util::StreamExt;
        let results = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await;
        let failure_index = results.iter().position(Result::is_err).expect("failure");
        assert!(
            results[failure_index + 1..].is_empty(),
            "a grammar failure seals the mapper instead of later finalizing successfully"
        );
        let failure = results
            .into_iter()
            .find_map(Result::err)
            .expect("the non-monotonic close fails typed");
        let is_typed_stream_failure = match &failure {
            ProviderError::StreamError(_) => true,
            ProviderError::StreamFailure(boxed) => {
                matches!(boxed.cause(), ProviderError::StreamError(_))
            }
            _ => false,
        };
        assert!(
            is_typed_stream_failure,
            "closed-input violation surfaces as a typed stream failure, got {failure:?}"
        );
    }

    #[tokio::test]
    async fn custom_tool_completed_input_cannot_rewrite_the_streamed_prefix() {
        let server = MockServer::start().await;
        let wire = [
            serde_json::json!({
                "type": "response.output_item.added", "output_index": 0,
                "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                    "call_id": "call_terminal", "name": "grammar-tool", "input": ""}
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta", "output_index": 0,
                "delta": "hel"
            }),
            serde_json::json!({
                "type": "response.completed", "response": {"id": "resp_terminal",
                    "model": "gpt-4o", "output": [{"type": "custom_tool_call",
                        "id": "ctc_terminal", "call_id": "call_terminal",
                        "name": "grammar-tool", "input": "hexlo"}]}
            }),
        ];
        let sse = wire
            .into_iter()
            .map(|value| format!("data: {value}\n\n"))
            .collect::<String>();
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let results = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await;
        assert!(results.last().is_some_and(Result::is_err));
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        assert!(
            !results
                .iter()
                .any(|result| matches!(result, Ok(AssistantStreamEvent::Done { .. })))
        );
    }
}

//! Azure OpenAI Responses options, projection and replay assertions.

use super::*;

#[tokio::test]
async fn azure_responses_native_options_reach_the_wire_without_service_tier() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::High);
    value.reasoning_summary = Some(ResponsesReasoningSummary::Detailed);
    value.tool_choice = Some(ResponsesToolChoice::Required);
    let options = handle.options(value);

    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect("native azure responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "detailed"})
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(body["tool_choice"], "required");
    assert!(
        body.get("service_tier").is_none(),
        "azure responses omits service_tier (source omission; no tier multiplier)"
    );
    assert_eq!(body["model"], "my-gpt4o");
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_uses_the_complete_per_call_record() {
    let factory_server = wiremock::MockServer::start().await;
    let selected_server = wiremock::MockServer::start().await;
    // Accept either server so a stale factory record produces an assertion
    // failure rather than a connection error.
    for server in [&factory_server, &selected_server] {
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_string(azure_responses_sse_text()),
            )
            .mount(server)
            .await;
    }
    let registered = azure_responses_record("my-gpt4o")
        .with_base_url(format!("{}/factory-api", factory_server.uri()));
    let provider = std::sync::Arc::new(
        AzureOpenAIProvider::for_records(
            Some(format!("{}/factory-api", factory_server.uri())),
            vec![registered.clone()],
            None,
        )
        .unwrap(),
    );
    let api = AzureOpenAIResponsesChatApi::new(provider, registered).unwrap();
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("offline-azure-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let selected = azure_responses_record("my-gpt4o")
        .with_base_url(format!("{}/custom-api", selected_server.uri()))
        .with_sampling_params(
            [("temperature".into(), serde_json::json!(0.37))]
                .into_iter()
                .collect(),
        );
    let mut options = AzureOpenAIOptions::default();
    options.azure_deployment_name = Some("selected-deployment".into());
    let result = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", selected),
            azure_responses_request("my-gpt4o", None),
            handle.options(options),
        )
        .await
        .unwrap();
    assert_eq!(result.provider, "azure");
    assert!(factory_server.received_requests().await.unwrap().is_empty());
    let sent = selected_server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].url.path(), "/custom-api/responses");
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["model"], "selected-deployment");
    assert_eq!(body["temperature"], serde_json::json!(0.37));
}

#[tokio::test]
async fn azure_responses_simple_entry_maps_neutral_thinking_and_clamps() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let models = {
        let (models, _handle) =
            registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o"))
                .await;
        models
    };
    let message = models
        .complete_simple(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", Some(ThinkingLevel::High)),
            opi_ai::api::contract::SimpleOptions::default(),
        )
        .await
        .expect("simple azure responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"]["effort"], "high",
        "simple entry carries the neutral thinking intent"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_disabled_thinking_sends_off_projection() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "none"}),
        "reasoning-capable azure deployments send the off mapping"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_endpoint_overrides_in_native_options_reach_the_url() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.azure_base_url = Some(format!("{}/openai/v1", server.uri()));
    value.azure_api_version = Some("v1".into());
    value.azure_deployment_name = Some("my-gpt4o".into());
    let options = handle.options(value);

    models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect("endpoint overrides reach the transport");
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_replays_signed_reasoning_for_same_identity_turns() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            }),
            opi_ai::message::Message::Assistant(azure_history_assistant(
                vec![
                    opi_ai::message::AssistantContent::Thinking {
                        thinking: "let me think".into(),
                        signature: Some(
                            serde_json::json!({
                                "type": "reasoning",
                                "id": "rs_1",
                                "encrypted_content": "ENC",
                            })
                            .to_string(),
                        ),
                    },
                    opi_ai::message::AssistantContent::Text {
                        text: "Earlier".into(),
                        signature: None,
                    },
                ],
                Some(WireApi::AzureOpenAiResponses),
                "azure",
            )),
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    let input = body["input"].as_array().expect("input list");
    assert!(
        input.iter().any(
            |item| item.get("type") == Some(&serde_json::json!("reasoning"))
                && item.get("encrypted_content") == Some(&serde_json::json!("ENC"))
        ),
        "same-identity signed reasoning replays into input: {input:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_foreign_identity_history_never_replays_reasoning() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            }),
            opi_ai::message::Message::Assistant(azure_history_assistant(
                vec![
                    opi_ai::message::AssistantContent::Thinking {
                        thinking: "foreign thought".into(),
                        signature: Some(
                            serde_json::json!({
                                "type": "reasoning",
                                "id": "rs_x",
                                "encrypted_content": "FOREIGN",
                            })
                            .to_string(),
                        ),
                    },
                    opi_ai::message::AssistantContent::Text {
                        text: "Earlier".into(),
                        signature: None,
                    },
                ],
                Some(WireApi::OpenAiResponses),
                "openai-responses",
            )),
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    let input = body["input"].as_array().expect("input list");
    assert!(
        !input
            .iter()
            .any(|item| item.get("type") == Some(&serde_json::json!("reasoning"))),
        "foreign-identity reasoning never replays: {input:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_native_unsupported_effort_fails_typed_before_auth() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_string(azure_responses_sse_text())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_source_defaults_and_ordinary_only_controls_use_actual_wire() {
    use opi_ai::message::{Message, ToolDef};
    use opi_ai::model_info::{
        OpenAiResponsesCompat, OpenAiResponsesSourceCompat, SessionAffinityFormat, WireCompat,
    };
    for flag in [None, Some(false), Some(true)] {
        let server = MockServer::start().await;
        mount_azure_responses_mock(&server, azure_responses_sse_text()).await;
        let source = OpenAiResponsesSourceCompat {
            supports_developer_role: flag,
            supports_strict_mode: flag,
            supports_mid_convo_system_messages: Some(true),
            supports_max_output_tokens: Some(false),
            supports_long_cache_retention: Some(true),
            supports_explicit_prompt_cache_mode: Some(true),
            supports_mid_convo_effort: Some(true),
            session_affinity_format: Some(SessionAffinityFormat::OpenRouter),
            ..Default::default()
        };
        let model = azure_responses_record("my-gpt4o")
            .with_compat(WireCompat::AzureOpenAiResponses(OpenAiResponsesCompat {
                source,
                ..Default::default()
            }))
            .unwrap();
        let (models, handle) = registered_azure_responses_models(server.uri(), model.clone()).await;
        let mut request = azure_responses_request("my-gpt4o", None);
        let mut context = opi_ai::transcript::Context {
            system_prompt: Some("leading instruction".into()),
            messages: request.context.messages().to_vec(),
            tools: vec![ToolDef {
                name: "read".into(),
                description: "read".into(),
                input_schema: serde_json::json!({"type":"object"}),
                constrained_sampling: None,
            }],
        };
        context
            .messages
            .push(Message::System(opi_ai::transcript::SystemMessage {
                content: "later instruction".into(),
                sections: vec![],
                tools_added: vec![],
                tools_removed: vec![],
                timestamp_ms: 1,
            }));
        request.context = opi_ai::transcript::normalize_context(context).unwrap();
        request.max_tokens = Some(7);
        request.session_id = Some("azure-own-session".into());
        request.cache_retention = CacheRetention::Long;
        let mut options = AzureOpenAIOptions::default();
        options.reasoning_effort = Some(ThinkingLevel::High);
        let message = models
            .complete(
                opi_ai::models::ChatModelRecord::new("azure", model),
                request,
                handle.options(options),
            )
            .await
            .unwrap();
        assert!(message.projection.is_some());
        let body = sent_body(&server).await;
        assert_eq!(body["input"][0]["content"], "leading instruction");
        assert_eq!(
            body["input"][0]["role"],
            if flag == Some(false) {
                "system"
            } else {
                "developer"
            }
        );
        assert!(body.get("instructions").is_none());
        let later = body["input"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["content"] == "later instruction")
            .unwrap();
        assert_eq!(
            later["role"],
            if flag == Some(false) {
                "system"
            } else {
                "developer"
            }
        );
        assert_eq!(
            body["tools"][0].get("strict"),
            (flag != Some(false)).then_some(&serde_json::Value::Bool(false))
        );
        assert_eq!(
            body["max_output_tokens"], 16,
            "Azure ignores the ordinary Responses max capability"
        );
        assert!(body.get("prompt_cache_options").is_none());
        assert!(body.get("prompt_cache_retention").is_none());
        assert!(
            !body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "configuration_update")
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        assert!(
            !sent[0].headers.contains_key("x-session-id"),
            "ordinary affinity format stays inert on Azure"
        );
        server.verify().await;
    }
}

//! Bedrock persisted-history and opaque-block replay cases.

use super::*;

fn opaque_replay_stream_42() -> Vec<u8> {
    build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"PRIVATE-first"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"signature":"sig-first-"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"signature":"one"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"redactedContent":"AP8="}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"redactedContent":"gEE="}},"contentBlockIndex":1}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":1}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":2}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"answer-left"},"contentBlockIndex":2}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":2}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":3}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"PRIVATE-second","signature":"sig-second"}},"contentBlockIndex":3}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":3}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":4}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"signature":"sig-empty"}},"contentBlockIndex":4}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":4}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":5}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"answer-right"},"contentBlockIndex":5}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":5}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":3,"outputTokens":4}}"#,
        ),
    ])
}

fn persisted_request_42(model: &ModelInfo, assistant: &AssistantMessage) -> Request {
    let mut request = req_42(Some("prefix one"), "next");
    request.model = model.id.clone();
    request.cache_retention = CacheRetention::Disabled;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("prefix one".into()),
        tools: vec![],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "first".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::Assistant(assistant.clone()),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "next".into(),
                }],
                timestamp_ms: 0,
            }),
        ],
    })
    .unwrap();
    request
}

fn replay_options_42() -> BedrockOptions {
    let mut options = native();
    options.reasoning = Some(ThinkingLevel::High);
    options
}

async fn persisted_capture_42() -> (
    MockServer,
    Models,
    opi_ai::ChatApiHandle<BedrockOptions>,
    ModelInfo,
    AssistantMessage,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            opaque_replay_stream_42(),
            "application/vnd.amazon.eventstream",
        ))
        .mount(&server)
        .await;
    let model = model_42("amazon.replay-profile-v1", "Claude Sonnet 5").with_base_url(server.uri());
    let provider = Arc::new(
        BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()))
            .with_models(vec![model.clone()]),
    );
    let api = BedrockConverseStreamChatApi::new(provider, model.clone()).unwrap();
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(StaticAwsResolver42 {
                credentials: test_credentials(),
            }),
            AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
        )
        .unwrap();
    let mut request = req_42(Some("prefix one"), "first");
    request.model = model.id.clone();
    request.cache_retention = CacheRetention::Disabled;
    let captured = models
        .complete(
            ChatModelRecord::new("bedrock", model.clone()),
            request,
            handle.options(replay_options_42()),
        )
        .await
        .unwrap();
    assert_eq!(captured.provider, "bedrock");
    assert_eq!(captured.model, model.id);
    assert_eq!(captured.wire, Some(WireApi::BedrockConverseStream));
    assert_eq!(captured.raw_stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(captured.content.len(), 6);
    assert!(
        matches!(&captured.content[0], AssistantContent::Thinking { thinking, signature }
            if thinking == "PRIVATE-first" && signature.as_deref() == Some("sig-first-one"))
    );
    assert!(
        matches!(&captured.content[1], AssistantContent::RedactedThinking { data }
            if data == "AP+AQQ==")
    );
    assert!(
        matches!(&captured.content[4], AssistantContent::Thinking { thinking, signature }
            if thinking.is_empty() && signature.as_deref() == Some("sig-empty"))
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("assistant.json");
    let encoded = serde_json::to_vec(&captured).unwrap();
    std::fs::write(&path, &encoded).unwrap();
    let reloaded: AssistantMessage = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(serde_json::to_vec(&reloaded).unwrap(), encoded);
    (server, models, handle, model, reloaded)
}

fn replayed_assistant_42(body: &serde_json::Value) -> &serde_json::Value {
    body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap()
}

#[tokio::test]
async fn persisted_opaque_blocks_keep_bytes_order_and_source_blank_projection() {
    let (server, models, handle, model, captured) = persisted_capture_42().await;
    let canonical = serde_json::to_vec(&captured).unwrap();
    models
        .complete(
            ChatModelRecord::new("bedrock", model.clone()),
            persisted_request_42(&model, &captured),
            handle.options(replay_options_42()),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[1].url.path(),
        "/model/amazon.replay-profile-v1/converse-stream"
    );
    assert!(
        sent[1].headers["authorization"]
            .to_str()
            .unwrap()
            .starts_with("AWS4-HMAC-SHA256 ")
    );
    let replay: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(
        replayed_assistant_42(&replay)["content"],
        serde_json::json!([
            {"reasoningContent":{"reasoningText":{"text":"PRIVATE-first","signature":"sig-first-one"}}},
            {"reasoningContent":{"redactedContent":"AP+AQQ=="}},
            {"text":"answer-left"},
            {"reasoningContent":{"reasoningText":{"text":"PRIVATE-second","signature":"sig-second"}}},
            {"text":"answer-right"},
        ])
    );
    // Converse drops blank reasoning text even when canonical storage retained its signature.
    assert!(!replay.to_string().contains("sig-empty"));
    assert_eq!(serde_json::to_vec(&captured).unwrap(), canonical);
}

#[tokio::test]
async fn persisted_history_follows_selected_signature_capability_and_binding_policy() {
    let (server, models, handle, model, captured) = persisted_capture_42().await;
    let canonical = serde_json::to_vec(&captured).unwrap();
    let mut non_claude = model.clone();
    non_claude.display_name = "Nova profile".into();
    let mut no_new_thinking = model.clone();
    no_new_thinking.capabilities.supports_thinking = false;
    for selected in [&non_claude, &no_new_thinking, &model] {
        let mut request = persisted_request_42(selected, &captured);
        if selected == &model {
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: Some("prefix two".into()),
                tools: vec![],
                messages: request
                    .context
                    .messages()
                    .iter()
                    .filter(|message| !matches!(message, Message::System(_)))
                    .cloned()
                    .collect(),
            })
            .unwrap();
        }
        models
            .complete(
                ChatModelRecord::new("bedrock", selected.clone()),
                request,
                handle.options(replay_options_42()),
            )
            .await
            .unwrap();
    }
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 4);
    let bodies: Vec<serde_json::Value> = sent
        .iter()
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    let non_claude = &replayed_assistant_42(&bodies[1])["content"];
    assert_eq!(
        non_claude[0],
        serde_json::json!({"reasoningContent":{"reasoningText":{"text":"PRIVATE-first"}}})
    );
    assert_eq!(
        non_claude[3],
        serde_json::json!({"reasoningContent":{"reasoningText":{"text":"PRIVATE-second"}}})
    );
    assert!(!non_claude.to_string().contains("signature"));
    assert!(bodies[2].get("additionalModelRequestFields").is_none());
    assert_eq!(
        replayed_assistant_42(&bodies[2])["content"][0]["reasoningContent"]["reasoningText"]["signature"],
        "sig-first-one"
    );
    assert_eq!(bodies[3]["system"][0]["text"], "prefix two");
    assert_eq!(
        replayed_assistant_42(&bodies[3])["content"][0]["reasoningContent"]["reasoningText"]["signature"],
        "sig-first-one"
    );
    assert_eq!(
        bodies[3]["additionalModelRequestFields"]["thinking"]["block_binding"]["prefix_mismatch_behavior"],
        "drop_block"
    );
    assert_eq!(
        bodies[3]["additionalModelRequestFields"]["anthropic_beta"],
        serde_json::json!(["thinking-binding-controls-2026-08-01"])
    );
    assert_eq!(serde_json::to_vec(&captured).unwrap(), canonical);
}

#[tokio::test]
async fn persisted_invalid_unknown_and_changed_model_history_project_on_real_requests() {
    let (server, models, handle, model, captured) = persisted_capture_42().await;
    let canonical = serde_json::to_vec(&captured).unwrap();
    let mut invalid = captured.clone();
    let AssistantContent::RedactedThinking { data } = &mut invalid.content[1] else {
        panic!("opaque capture")
    };
    *data = "!!not-base64!!".into();
    let invalid_before = serde_json::to_vec(&invalid).unwrap();
    let mut unknown = captured.clone();
    unknown.wire = None;
    let unknown_before = serde_json::to_vec(&unknown).unwrap();
    let other = model_42("amazon.nova-lite-v1", "Nova Lite").with_base_url(server.uri());
    for (history, selected) in [(&invalid, &model), (&unknown, &model), (&captured, &other)] {
        models
            .complete(
                ChatModelRecord::new("bedrock", selected.clone()),
                persisted_request_42(selected, history),
                handle.options(replay_options_42()),
            )
            .await
            .unwrap();
    }
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 4);
    let bodies: Vec<serde_json::Value> = sent
        .iter()
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    assert_eq!(
        replayed_assistant_42(&bodies[1])["content"],
        serde_json::json!([
            {"reasoningContent":{"reasoningText":{"text":"PRIVATE-first","signature":"sig-first-one"}}},
            {"text":"answer-left"},
            {"reasoningContent":{"reasoningText":{"text":"PRIVATE-second","signature":"sig-second"}}},
            {"text":"answer-right"},
        ])
    );
    assert!(!bodies[1].to_string().contains("not-base64"));
    for body in [&bodies[2], &bodies[3]] {
        assert_eq!(
            replayed_assistant_42(body)["content"],
            serde_json::json!([
                {"text":"answer-left"}, {"text":"answer-right"},
            ])
        );
        assert!(!body.to_string().contains("PRIVATE-"));
        assert!(!body.to_string().contains("AP+AQQ=="));
    }
    assert_eq!(
        sent[3].url.path(),
        "/model/amazon.nova-lite-v1/converse-stream"
    );
    assert_eq!(serde_json::to_vec(&invalid).unwrap(), invalid_before);
    assert_eq!(serde_json::to_vec(&unknown).unwrap(), unknown_before);
    assert_eq!(serde_json::to_vec(&captured).unwrap(), canonical);
}

#[tokio::test]
async fn persisted_redacted_bytes_follow_source_forgiving_base64_acceptance() {
    let (server, models, handle, model, captured) = persisted_capture_42().await;
    let cases = [
        ("AP+AQQ", true),
        ("AP+AQR==", true),
        (" \tAP+A\r\nQQ==\u{000c}", true),
        ("AP+AQQ=", false),
        ("AA=", false),
        ("=AAA", false),
        ("AP-AQQ==", false),
        ("AP+A\u{000b}QQ==", false),
        ("AP+A\u{00a0}QQ==", false),
        ("", false),
        (" \t\r\n", false),
    ];
    for (encoded, _) in cases {
        let mut history = captured.clone();
        let AssistantContent::RedactedThinking { data } = &mut history.content[1] else {
            panic!("opaque capture")
        };
        *data = encoded.into();
        let canonical = serde_json::to_vec(&history).unwrap();
        models
            .complete(
                ChatModelRecord::new("bedrock", model.clone()),
                persisted_request_42(&model, &history),
                handle.options(replay_options_42()),
            )
            .await
            .unwrap();
        assert_eq!(serde_json::to_vec(&history).unwrap(), canonical);
    }
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), cases.len() + 1);
    let observed: Vec<_> = sent[1..]
        .iter()
        .map(|request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            replayed_assistant_42(&body)["content"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|block| block.pointer("/reasoningContent/redactedContent").cloned())
        })
        .collect();
    let expected: Vec<_> = cases
        .iter()
        .map(|(_, nonempty)| nonempty.then(|| serde_json::json!("AP+AQQ==")))
        .collect();
    assert_eq!(
        observed, expected,
        "persisted base64 follows atob acceptance, with empty opaque blocks omitted"
    );
}

#[tokio::test]
async fn persisted_bedrock_capture_drops_opaque_on_an_actual_api_only_route_change() {
    use opi_ai::api::openai_completions::{OpenAiCompletionsChatApi, OpenAiCompletionsOptions};
    use opi_ai::providers::openai::OpenAiChatProvider;
    let (_, mut models, _, model, captured) = persisted_capture_42().await;
    let canonical = serde_json::to_vec(&captured).unwrap();
    let remote = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_raw(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
            "text/event-stream",
        )).expect(1).mount(&remote).await;
    let target = ModelInfo::new(
        &model.id,
        "Chat target",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192).with_streaming(true),
    )
    .with_base_url(remote.uri());
    let api = OpenAiCompletionsChatApi::new(
        Arc::new(OpenAiChatProvider::for_route(
            Some(remote.uri()),
            "bedrock".into(),
            Default::default(),
            vec![target.clone()],
            Arc::new(HttpClient::new()),
        )),
        target.clone(),
    )
    .unwrap();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                "fixture-key".into(),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let result = models
        .complete(
            ChatModelRecord::new("bedrock", target.clone()),
            persisted_request_42(&target, &captured),
            handle.options(OpenAiCompletionsOptions::default()),
        )
        .await
        .unwrap();
    assert_eq!(result.provider, captured.provider);
    assert_eq!(result.model, captured.model);
    assert_eq!(result.wire, Some(WireApi::OpenAiCompletions));
    let sent = remote.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].url.path().ends_with("/chat/completions"));
    assert_eq!(sent[0].headers["authorization"], "Bearer fixture-key");
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    let assistant = replayed_assistant_42(&body);
    assert!(assistant.to_string().contains("answer-left"));
    assert!(assistant.to_string().contains("answer-right"));
    for hidden in [
        "PRIVATE-",
        "sig-first",
        "sig-second",
        "sig-empty",
        "AP+AQQ==",
    ] {
        assert!(
            !body.to_string().contains(hidden),
            "foreign API received {hidden}: {body}"
        );
    }
    assert_eq!(serde_json::to_vec(&captured).unwrap(), canonical);
}

//! Native Mistral Conversations wire assertions.

use super::*;

#[tokio::test]
async fn direct_auth_refusal_retains_accepted_projection_and_eof() {
    let server = MockServer::start().await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut auth = opi_ai::test_support::resolved_bearer_auth();
    auth.scheme = opi_ai::auth::AuthScheme::ApiKey;
    let request = req_native("go");
    let mut stream = provider.stream_prepared(request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("wrong auth scheme");
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier")
    };
    let ProviderError::Config(summary) = failure.cause() else {
        panic!("wrong underlying auth refusal: {:?}", failure.cause())
    };
    assert_eq!(
        summary.as_str(),
        "Mistral accepts only Bearer authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

// -- wire projection ----------------------------------------------------

#[tokio::test]
async fn tool_choice_wire_modes() {
    let cases = [
        (Some(MistralToolChoice::Auto), serde_json::json!("auto")),
        (Some(MistralToolChoice::None_), serde_json::json!("none")),
        (Some(MistralToolChoice::Any), serde_json::json!("any")),
        (
            Some(MistralToolChoice::Required),
            serde_json::json!("required"),
        ),
        (
            Some(MistralToolChoice::Named("read_file".into())),
            serde_json::json!({"type": "function", "function": {"name": "read_file"}}),
        ),
        (None, serde_json::Value::Null),
    ];
    for (choice, expected) in cases {
        let server = MockServer::start().await;
        mount_native(&server, sse_ok_native()).await;
        let options = MistralOptions::native(choice, None, None);
        let provider = provider_native(
            Some(&server.uri()),
            model_native(SMALL_NATIVE, false, false),
            options,
        );
        let events = run_native(&provider, req_native("go")).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. }))),
            "mode {expected} completes"
        );
        let body = body_native(&server).await;
        match expected {
            serde_json::Value::Null => assert!(
                body.get("tool_choice").is_none(),
                "omitted tool choice must stay off the wire: {body}"
            ),
            value => assert_eq!(body["tool_choice"], value),
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn native_reasoning_effort_and_prompt_mode_ride_the_wire() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let options = MistralOptions::native(
        None,
        Some(MistralReasoningEffort::Max),
        Some(MistralPromptMode::Reasoning),
    );
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        options,
    );
    run_native(&provider, req_native("go")).await;
    let body = body_native(&server).await;
    assert_eq!(body["reasoning_effort"], serde_json::json!("max"));
    assert_eq!(body["prompt_mode"], serde_json::json!("reasoning"));
}

#[tokio::test]
async fn simple_reasoning_uses_effort_map_on_mapped_models() {
    // A reasoning model whose level map carries explicit entries sends
    // `reasoning_effort` from the map; an unmapped level falls back to
    // pi's "high" and the off entry projects when reasoning is disabled.
    // Requested levels outside the map clamp to the nearest supported
    // level (pi clampThinkingLevel): Max clamps down to medium.
    let cases = [
        (ThinkingLevel::Medium, serde_json::json!("medium")),
        (ThinkingLevel::Max, serde_json::json!("medium")),
    ];
    for (level, expected) in cases {
        let server = MockServer::start().await;
        mount_native(&server, sse_ok_native()).await;
        let model = effort_model_native(
            "magistral-medium-latest",
            map_native(&[
                (ThinkingLevel::Low, "low"),
                (ThinkingLevel::Medium, "medium"),
            ]),
        );
        let provider =
            MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
        let mut request = req_native("go");
        request.model = "mistral:magistral-medium-latest".into();
        request.thinking = ThinkingConfig {
            enabled: true,
            level,
            budget_tokens: None,
        };
        let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
        let events: Vec<_> = stream.collect().await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. }))),
            "level {level:?} completes"
        );
        let body = body_native(&server).await;
        assert_eq!(body["reasoning_effort"], expected, "level {level:?}");
        assert!(
            body.get("prompt_mode").is_none(),
            "mapped models never send prompt_mode: {body}"
        );
    }
}

#[tokio::test]
async fn simple_reasoning_off_uses_effort_map_off_entry() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let model = effort_model_native(
        "magistral-medium-latest",
        map_native(&[(ThinkingLevel::Low, "low"), (ThinkingLevel::None, "low")]),
    );
    let provider = MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
    let mut request = req_native("go");
    request.model = "mistral:magistral-medium-latest".into();
    request.thinking = ThinkingConfig {
        enabled: false,
        level: ThinkingLevel::None,
        budget_tokens: None,
    };
    let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
    let events: Vec<_> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
    );
    let body = body_native(&server).await;
    assert_eq!(body["reasoning_effort"], serde_json::json!("low"));
}

#[tokio::test]
async fn simple_reasoning_without_map_uses_prompt_mode() {
    // A reasoning model without explicit map entries (opi's synthetic
    // identity default) rides pi's prompt_mode=reasoning branch instead.
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let model = model_native("magistral-small-latest", true, false);
    let provider = MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
    let mut request = req_native("go");
    request.model = "mistral:magistral-small-latest".into();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::Low,
        budget_tokens: None,
    };
    let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
    let events: Vec<_> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
    );
    let body = body_native(&server).await;
    assert_eq!(body["prompt_mode"], serde_json::json!("reasoning"));
    assert!(
        body.get("reasoning_effort").is_none(),
        "unmapped models never send reasoning_effort: {body}"
    );
}

#[tokio::test]
async fn simple_non_reasoning_models_send_neither_reasoning_member() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model_native(SMALL_NATIVE, false, false)]);
    let mut request = req_native("go");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
    let events: Vec<_> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
    );
    let body = body_native(&server).await;
    assert!(body.get("reasoning_effort").is_none(), "{body}");
    assert!(body.get("prompt_mode").is_none(), "{body}");
}

#[tokio::test]
async fn sampling_projects_temperature_and_max_tokens_only() {
    // pi's Mistral payload builder sets only temperature and maxTokens;
    // stop sequences, top_p and the penalty family stay off this wire.
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.temperature = Some(0.7);
    request.max_tokens = Some(512);
    request.stop_sequences = vec!["END".into()];
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    assert_eq!(body["temperature"], serde_json::json!(0.7));
    assert_eq!(body["max_tokens"], serde_json::json!(512));
    for absent in [
        "stop",
        "top_p",
        "presence_penalty",
        "frequency_penalty",
        "random_seed",
        "safe_prompt",
    ] {
        assert!(
            body.get(absent).is_none(),
            "member {absent} must stay off the wire: {body}"
        );
    }
    server.verify().await;
}

#[tokio::test]
async fn tools_unannotated_default_is_false() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "go".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
                "additionalProperties": false
            }),
            constrained_sampling: None,
        }],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    let function = &body["tools"][0]["function"];
    assert_eq!(function["name"], serde_json::json!("read_file"));
    assert_eq!(function["strict"], serde_json::json!(false));
    assert_eq!(
        function["parameters"]["properties"]["path"]["type"],
        serde_json::json!("string")
    );
}

#[tokio::test]
async fn tools_unannotated_schema_is_unconverted() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "go".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}}
            }),
            constrained_sampling: None,
        }],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    let function = &body["tools"][0]["function"];
    assert_eq!(function["name"], serde_json::json!("read_file"));
    assert!(
        function["parameters"].get("additionalProperties").is_none(),
        "unconstrained parameters must stay unconverted"
    );
    assert_eq!(
        function["parameters"],
        serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}}
        })
    );
}

#[tokio::test]
async fn tools_explicit_require_remains_strict() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "go".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}}
            }),
            constrained_sampling: Some(
                opi_ai::constrained_sampling::ConstrainedSampling::JsonSchema {
                    strict: opi_ai::constrained_sampling::StrictPreference::Require,
                },
            ),
        }],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    let function = &body["tools"][0]["function"];
    assert_eq!(function["name"], serde_json::json!("read_file"));
    assert_eq!(function["strict"], serde_json::json!(true));
    assert_eq!(function["parameters"]["additionalProperties"], false);
    assert_eq!(
        function["parameters"]["required"],
        serde_json::json!(["path"])
    );
    assert_eq!(
        function["parameters"]["properties"]["path"],
        serde_json::json!({"anyOf": [{"type": "string"}, {"type": "null"}]})
    );
}

// -- prompt caching affinity --------------------------------------------

#[tokio::test]
async fn prompt_cache_key_and_x_affinity_gate_together() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("x-affinity", "session-1"))
        .and(body_partial_json(
            serde_json::json!({"prompt_cache_key": "session-1"}),
        ))
        .respond_with(sse_ok_native())
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.session_id = Some("session-1".into());
    request.cache_retention = CacheRetention::Short;
    run_native(&provider, request).await;
    server.verify().await;
}

#[tokio::test]
async fn explicit_affinity_header_wins_and_disabled_retention_suppresses() {
    // Explicit x-affinity override is kept verbatim (the adapter does not
    // rewrite it); a Disabled retention drops both affinity signals.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("x-affinity", "pinned"))
        .respond_with(sse_ok_native())
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = req_native("go");
    request.session_id = Some("session-1".into());
    request.cache_retention = CacheRetention::Short;
    request.extra_headers = vec![("x-affinity".into(), "pinned".into())];
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    // pi: the explicit header only suppresses the auto affinity header;
    // prompt_cache_key follows the caching gate, not the header.
    assert_eq!(body["prompt_cache_key"], serde_json::json!("session-1"));
    server.verify().await;

    let server2 = MockServer::start().await;
    mount_native(&server2, sse_ok_native()).await;
    let provider2 = provider_native(
        Some(&server2.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request2 = req_native("go");
    request2.session_id = Some("session-1".into());
    request2.cache_retention = CacheRetention::Disabled;
    run_native(&provider2, request2).await;
    let body2 = body_native(&server2).await;
    assert!(body2.get("prompt_cache_key").is_none(), "{body2}");
}

#[tokio::test]
async fn sse_headers_and_trailing_slash_base_url() {
    // The API keeps the mistral-conversations identity but posts the
    // chat/completions endpoint regardless of a trailing-slash base URL.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("accept", "text/event-stream"))
        .and(header("content-type", "application/json"))
        .respond_with(sse_ok_native())
        .expect(1)
        .mount(&server)
        .await;
    let provider = MistralConversationsProvider::new(Some(format!("{}/", server.uri())))
        .with_models(vec![model_native(SMALL_NATIVE, false, false)]);
    run_native(&provider, req_native("go")).await;
    server.verify().await;
}

// -- SSE consumption -----------------------------------------------------

#[tokio::test]
async fn chunked_content_streams_thinking_then_text_blocks() {
    let sse = concat!(
        "data: {\"id\":\"chatcmpl-9\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"ponder\"}]},{\"type\":\"text\",\"text\":\"answer\"}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\n\n",
        "data: [DONE]\n\n",
    );
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let message = done_message_native(&events).await;
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-9"));
    let mut kinds = Vec::new();
    for content in &message.content {
        kinds.push(match content {
            AssistantContent::Thinking { .. } => "thinking",
            AssistantContent::Text { .. } => "text",
            other => panic!("unexpected content block: {other:?}"),
        });
    }
    assert_eq!(kinds, vec!["thinking", "text"]);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::ThinkingStart { .. })))
    );
}

#[tokio::test]
async fn empty_content_deltas_do_not_split_blocks() {
    // GLM-style empty deltas around thinking must not open new blocks
    // (a split replays as multiple thinking chunks, which Mistral
    // rejects on replay).
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"a\"}]}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"b\"}]}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let message = done_message_native(&events).await;
    let thinking = message
        .content
        .iter()
        .filter(|content| matches!(content, AssistantContent::Thinking { .. }))
        .count();
    assert_eq!(
        thinking, 1,
        "empty deltas must not split the thinking block"
    );
    match &message.content[0] {
        AssistantContent::Thinking { thinking, .. } => assert_eq!(thinking, "ab"),
        other => panic!("expected one thinking block, got {other:?}"),
    }
}

#[tokio::test]
async fn tool_calls_accumulate_by_index_and_derive_ids() {
    // The first chunk's id is the literal "null" placeholder: the id is
    // derived from the tool index; argument fragments join by index.
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"null\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let message = done_message_native(&events).await;
    assert_eq!(message.stop_reason, StopReason::ToolUse);
    match &message.content[0] {
        AssistantContent::ToolCall { tool_call } => {
            assert_eq!(tool_call.name, "read_file");
            assert!(
                tool_call.arguments.contains("\"path\":\"foo.rs\"")
                    || serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                        .map(|v| v["path"] == serde_json::json!("foo.rs"))
                        .unwrap_or(false),
                "arguments parse with path: {}",
                tool_call.arguments
            );
            assert_eq!(
                tool_call.id.len(),
                9,
                "derived id must be 9 chars: {}",
                tool_call.id
            );
            assert!(
                tool_call.id.chars().all(|c| c.is_ascii_alphanumeric()),
                "derived id must be alphanumeric: {}",
                tool_call.id
            );
        }
        other => panic!("expected tool call, got {other:?}"),
    }
}

#[tokio::test]
async fn trailing_literal_preview_does_not_validate_incomplete_final() {
    let frames = [
        serde_json::json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"abcdefghi","function":{"name":"lookup","arguments":"{\"flag\":true"}}]}}]}),
        serde_json::json!({"choices":[{"delta":{},"finish_reason":"tool_calls"}]}),
    ];
    let sse = frames
        .iter()
        .map(|frame| format!("data: {frame}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n";
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let deltas: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Ok(AssistantStreamEvent::ToolCallDelta { partial, .. }) => Some(partial),
            _ => None,
        })
        .collect();
    assert_eq!(deltas.len(), 1);
    let AssistantContent::ToolCall { tool_call } = &deltas[0].content[0] else {
        panic!("partial tool call")
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&tool_call.arguments).unwrap(),
        serde_json::json!({"flag":true})
    );
    let final_message = done_message_native(&events).await;
    let AssistantContent::ToolCall { tool_call } = &final_message.content[0] else {
        panic!("final tool call")
    };
    assert_eq!(tool_call.arguments, "{\"flag\":true");
    assert!(tool_call.arguments_error.is_some());
}

#[tokio::test]
async fn usage_splits_cached_prompt_tokens() {
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":20,\"prompt_tokens_details\":{\"cached_tokens\":30}}}\n\n",
        "data: [DONE]\n\n",
    );
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let message = done_message_native(&events).await;
    let usage = &message.usage;
    assert_eq!(usage.input_tokens, 70, "input = prompt - cached");
    assert_eq!(usage.output_tokens, 20);
    assert_eq!(usage.cache_read_tokens, 30);
    assert_eq!(
        usage.cache_write_tokens, 0,
        "mistral reports no cache writes"
    );
    assert_eq!(usage.total_tokens, Some(120), "total falls back to the sum");
}

#[tokio::test]
async fn stop_reason_map_covers_mistral_names() {
    let cases = [
        ("stop", StopReason::Stop),
        ("length", StopReason::Length),
        ("model_length", StopReason::Length),
    ];
    for (raw, expected) in cases {
        let sse = format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"x\"}}}}]}}\n\n\
             data: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{raw}\"}}]}}\n\n\
             data: [DONE]\n\n"
        );
        let server = MockServer::start().await;
        mount_native(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_native(
            Some(&server.uri()),
            model_native(SMALL_NATIVE, false, false),
            MistralOptions::default(),
        );
        let events = run_native(&provider, req_native("go")).await;
        let message = done_message_native(&events).await;
        assert_eq!(message.stop_reason, expected, "raw {raw}");
        assert_eq!(message.raw_stop_reason.as_deref(), Some(raw));
    }
}

#[tokio::test]
async fn in_stream_error_stop_is_typed_failure() {
    for (raw, needle) in [
        ("error", "Provider stopped with: error (server error)"),
        ("something_else", "Provider stopped with: something_else"),
    ] {
        let sse = format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"x\"}}}}]}}\n\n\
             data: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{raw}\"}}]}}\n\n\
             data: [DONE]\n\n"
        );
        let server = MockServer::start().await;
        mount_native(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_native(
            Some(&server.uri()),
            model_native(SMALL_NATIVE, false, false),
            MistralOptions::default(),
        );
        let events = run_native(&provider, req_native("go")).await;
        let error = first_error_native(&events).await;
        assert!(
            error.to_string().contains(needle),
            "stop {raw} must fail typed with {needle}, got {error}"
        );
    }
}

#[tokio::test]
async fn missing_finish_reason_fails_typed() {
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\ndata: [DONE]\n\n";
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let error = first_error_native(&events).await;
    assert!(
        error
            .to_string()
            .contains("Mistral stream ended without a finish reason"),
        "got {error}"
    );
}

#[tokio::test]
async fn invalid_streaming_event_fails_typed() {
    let sse = "data: {\"id\":\"x\"}\n\ndata: [DONE]\n\n";
    let server = MockServer::start().await;
    mount_native(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(sse)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let error = first_error_native(&events).await;
    assert!(
        error
            .to_string()
            .contains("Invalid Mistral streaming event"),
        "got {error}"
    );
}

// -- HTTP failures --------------------------------------------------------

#[tokio::test]
async fn http_errors_format_status_and_truncated_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"))
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events = run_native(&provider, req_native("go")).await;
    let error = first_error_native(&events).await;
    assert!(
        error
            .to_string()
            .contains("Mistral API error (500): internal boom"),
        "got {error}"
    );

    // 429 keeps its retryable identity (the body detail does not
    // displace the rate-limit family).
    let server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .expect(1)
        .mount(&server2)
        .await;
    let provider2 = provider_native(
        Some(&server2.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events2 = run_native(&provider2, req_native("go")).await;
    let error2 = first_error_native(&events2).await;
    assert!(
        matches!(error2, ProviderError::RateLimited { .. }),
        "got {error2}"
    );

    // Long bodies truncate at pi's 4000-char boundary; the crate-wide
    // sanitized-summary cap then bounds the observable text, so neither
    // the full body nor its tail survives into diagnostics.
    let long = "x".repeat(5000);
    let server3 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string(long))
        .expect(1)
        .mount(&server3)
        .await;
    let provider3 = provider_native(
        Some(&server3.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let events3 = run_native(&provider3, req_native("go")).await;
    let error3 = first_error_native(&events3).await;
    let text = error3.to_string();
    assert!(text.contains("Mistral API error (503)"), "got {text}");
    let xs = text.matches('x').count();
    assert!(
        xs < 5000,
        "the raw 5000-char body must not ride whole into diagnostics ({xs} x chars)"
    );
    assert!(
        text.chars().count() < 5000,
        "the sanitized summary caps the formatted error text"
    );
}

fn assert_direct_projection_prefix(
    message: &opi_ai::message::AssistantMessage,
    request: &opi_ai::provider::Request,
) {
    use sha2::Digest as _;
    let facts = message
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
}

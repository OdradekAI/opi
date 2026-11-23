//! Bedrock Converse request and message projection cases.

use super::*;

#[test]
fn native_tool_choice_projects_converse_modes() {
    for (choice, wire) in [
        (
            opi_ai::api::bedrock::BedrockToolChoice::Auto,
            r#"{"auto":{}}"#,
        ),
        (
            opi_ai::api::bedrock::BedrockToolChoice::Any,
            r#"{"any":{}}"#,
        ),
        (
            opi_ai::api::bedrock::BedrockToolChoice::Named("read".into()),
            r#"{"tool":{"name":"read"}}"#,
        ),
    ] {
        let mut options = native();
        options.tool_choice = Some(choice);
        let provider =
            BedrockProvider::new(None, Arc::new(HttpClient::new())).with_native_options(options);
        let body = body_42(&provider, &tool_call_request());
        assert_eq!(
            serde_json::to_string(&body["toolConfig"]["toolChoice"]).unwrap(),
            wire
        );
        // Tools ride the config alongside the choice.
        assert!(
            body["toolConfig"]["tools"][0]["toolSpec"]["name"]
                .as_str()
                .is_some_and(|name| name == "read")
        );
    }

    // Pi drops the whole toolConfig for `none` (tools are not sent).
    let mut options = native();
    options.tool_choice = Some(opi_ai::api::bedrock::BedrockToolChoice::None_);
    let provider =
        BedrockProvider::new(None, Arc::new(HttpClient::new())).with_native_options(options);
    let body = body_42(&provider, &tool_call_request());
    assert!(body.get("toolConfig").is_none());
}

#[test]
fn native_numeric_reasoning_uses_budget_table_and_interleaved_beta() {
    // Default table: high -> 16384 with the interleaved beta and the
    // summarized display default.
    let body = reasoning_body_42(
        "anthropic.claude-sonnet-4-20250514-v2",
        Some(ThinkingLevel::High),
        BTreeMap::new(),
    );
    assert_eq!(
        body["additionalModelRequestFields"],
        serde_json::json!({
            "thinking": {"type": "enabled", "budget_tokens": 16384, "display": "summarized"},
            "anthropic_beta": ["interleaved-thinking-2025-05-14"],
        })
    );

    // Custom budgets override the level budget; xhigh clamps to high on
    // budget-based models.
    let mut budgets = BTreeMap::new();
    budgets.insert(ThinkingLevel::High, 9_000);
    let body = reasoning_body_42(
        "anthropic.claude-sonnet-4-20250514-v2",
        Some(ThinkingLevel::XHigh),
        budgets,
    );
    assert_eq!(
        body["additionalModelRequestFields"]["thinking"]["budget_tokens"],
        serde_json::json!(9_000)
    );
}

#[test]
fn native_display_and_interleaved_options_shape_fields() {
    let mut options = native();
    options.reasoning = Some(ThinkingLevel::High);
    options.thinking_display = Some(opi_ai::api::bedrock::BedrockThinkingDisplay::Omitted);
    options.interleaved_thinking = Some(false);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")])
        .with_native_options(options);
    let body = body_42(&provider, &req_42(None, "go"));
    assert_eq!(
        body["additionalModelRequestFields"]["thinking"]["display"],
        serde_json::json!("omitted")
    );
    // Interleaved opt-out removes the beta entirely.
    assert!(
        body["additionalModelRequestFields"]
            .get("anthropic_beta")
            .is_none()
    );
}

#[test]
fn govcloud_ids_omit_display_and_block_binding() {
    // GovCloud model ids skip display and block binding (pi pins both
    // until the GovCloud Converse schema catches up).
    let body = reasoning_body_42(
        "us-gov.anthropic.claude-sonnet-4-x",
        Some(ThinkingLevel::High),
        BTreeMap::new(),
    );
    assert_eq!(
        body["additionalModelRequestFields"]["thinking"],
        serde_json::json!({"type": "enabled", "budget_tokens": 16384})
    );
    // The interleaved beta still rides numeric GovCloud requests (pi
    // applies only the display/binding omissions there).
    assert_eq!(
        body["additionalModelRequestFields"]["anthropic_beta"],
        serde_json::json!(["interleaved-thinking-2025-05-14"])
    );
}

#[test]
fn adaptive_models_take_adaptive_fields_and_native_xhigh() {
    let body = reasoning_body_42(
        "anthropic.claude-opus-4-7-20260101-v1",
        Some(ThinkingLevel::High),
        BTreeMap::new(),
    );
    assert_eq!(
        body["additionalModelRequestFields"],
        serde_json::json!({
            "thinking": {
                "type": "adaptive",
                "display": "summarized",
                "block_binding": {"prefix_mismatch_behavior": "drop_block"},
            },
            "output_config": {"effort": "high"},
            "anthropic_beta": ["thinking-binding-controls-2026-08-01"],
        })
    );

    // Native xhigh passes through on xhigh-capable models; thinking
    // budgets are ignored on the adaptive path.
    let mut budgets = BTreeMap::new();
    budgets.insert(ThinkingLevel::High, 9_000);
    let body = reasoning_body_42(
        "anthropic.claude-opus-4-7-20260101-v1",
        Some(ThinkingLevel::XHigh),
        budgets,
    );
    assert_eq!(
        body["additionalModelRequestFields"]["output_config"]["effort"],
        serde_json::json!("xhigh")
    );

    // A thinking-level map overrides the level->effort mapping.
    let mut model = model_42("anthropic.claude-opus-4-7-20260101-v1", "Opus");
    model.thinking_level_map = ThinkingLevelMap::reasoning_default().with_mapping(
        ThinkingLevel::High,
        opi_ai::model_info::ThinkingLevelMapping::Mapped("max".to_owned()),
    );
    let mut options = native();
    options.reasoning = Some(ThinkingLevel::High);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model])
        .with_native_options(options);
    let mut request = req_42(None, "go");
    request.model = "anthropic.claude-opus-4-7-20260101-v1".into();
    let body = body_42(&provider, &request);
    assert_eq!(
        body["additionalModelRequestFields"]["output_config"]["effort"],
        serde_json::json!("max")
    );
}

#[test]
fn gpt_oss_and_gpt_map_efforts() {
    // gpt-oss takes a flat reasoning_effort and clamps to high.
    let body = reasoning_body_42(
        "openai.gpt-oss-120b-1",
        Some(ThinkingLevel::Max),
        BTreeMap::new(),
    );
    assert_eq!(
        body["additionalModelRequestFields"],
        serde_json::json!({"reasoning_effort": "high"})
    );

    // GPT models take a nested reasoning.effort; minimal maps to low.
    let body = reasoning_body_42(
        "openai.gpt-5.1",
        Some(ThinkingLevel::Minimal),
        BTreeMap::new(),
    );
    assert_eq!(
        body["additionalModelRequestFields"],
        serde_json::json!({"reasoning": {"effort": "low"}})
    );
}

#[test]
fn native_omission_sends_no_reasoning_fields() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")])
        .with_native_options(native());
    let body = body_42(&provider, &req_42(None, "go"));
    assert!(body.get("additionalModelRequestFields").is_none());
}

#[test]
fn request_metadata_rides_verbatim() {
    let mut options = native();
    options.request_metadata = BTreeMap::from([
        ("team".to_owned(), "core".to_owned()),
        // Pi documents limits in comments only; the consumption path
        // forwards whatever arrives without local preflight.
        ("k".repeat(100).to_owned(), "v".to_owned()),
    ]);
    let provider =
        BedrockProvider::new(None, Arc::new(HttpClient::new())).with_native_options(options);
    let body = body_42(&provider, &req_42(None, "go"));
    assert_eq!(body["requestMetadata"]["team"], serde_json::json!("core"));
    assert_eq!(
        body["requestMetadata"][&"k".repeat(100)],
        serde_json::json!("v")
    );
}

#[test]
fn cache_points_system_and_last_user() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")]);
    let mut request = req_42(Some("You are helpful."), "go");
    request.cache_retention = CacheRetention::None;
    let body = body_42(&provider, &request);
    // Default retention keeps the pi "short" default: system text plus a
    // default cache point, and the last user message carries one.
    assert_eq!(
        body["system"],
        serde_json::json!([
            {"text": "You are helpful."},
            {"cachePoint": {"type": "default"}},
        ])
    );
    let messages = body["messages"].as_array().expect("messages");
    let last = messages.last().expect("last message");
    assert_eq!(
        last["content"].as_array().expect("content")[1],
        serde_json::json!({"cachePoint": {"type": "default"}})
    );

    // Long retention adds the one-hour TTL.
    let mut request = req_42(Some("You are helpful."), "go");
    request.cache_retention = CacheRetention::Long;
    let body = body_42(&provider, &request);
    assert_eq!(
        body["system"][1],
        serde_json::json!({"cachePoint": {"type": "default", "ttl": "1h"}})
    );

    // Disabled drops every cache point.
    let mut request = req_42(Some("You are helpful."), "go");
    request.cache_retention = CacheRetention::Disabled;
    let body = body_42(&provider, &request);
    assert!(
        body["system"]
            .as_array()
            .is_none_or(|blocks| blocks.len() == 1)
    );

    // Non-Claude models get no cache points at all.
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42("amazon.nova-lite-v1", "Nova Lite")]);
    let mut request = req_42(Some("You are helpful."), "go");
    request.cache_retention = CacheRetention::None;
    let body = body_42(&provider, &request);
    assert!(
        body["system"]
            .as_array()
            .is_none_or(|blocks| blocks.len() == 1)
    );
}

#[test]
fn empty_text_placeholder_and_blank_drop() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")]);
    let mut request = req_42(None, "   ");
    request.cache_retention = CacheRetention::Disabled;
    // Blank user text falls to the <empty> placeholder.
    let body = body_42(&provider, &request);
    assert_eq!(
        body["messages"][0]["content"],
        serde_json::json!([{"text": "<empty>"}])
    );

    // Blank assistant text is dropped, not placeholdered.
    let mut context_messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Text { text: "q".into() }],
        timestamp_ms: 0,
    })];
    context_messages.push(Message::Assistant(AssistantMessage {
        content: vec![
            AssistantContent::Text {
                text: "  ".into(),
                signature: None,
            },
            AssistantContent::Text {
                text: "real".into(),
                signature: None,
            },
        ],
        ..assistant_42()
    }));
    let mut request = req_42(None, "q");
    request.cache_retention = CacheRetention::Disabled;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: context_messages,
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = body_42(&provider, &request);
    assert_eq!(
        body["messages"][1]["content"],
        serde_json::json!([{"text": "real"}])
    );
}

#[test]
fn thinking_replay_projects_converse_shapes() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")]);
    let mut assistant = assistant_42();
    assistant.provider = "bedrock".into();
    assistant.content = vec![
        AssistantContent::Thinking {
            thinking: "step".into(),
            signature: Some("sig-1".into()),
        },
        AssistantContent::Thinking {
            thinking: "unsigned".into(),
            signature: None,
        },
        AssistantContent::RedactedThinking {
            data: "b3BhcXVl".into(),
        },
        AssistantContent::RedactedThinking {
            data: "!!not-base64!!".into(),
        },
    ];
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(assistant),
    ];
    let mut request = req_42(None, "q");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = body_42(&provider, &request);
    assert_eq!(
        body["messages"][1]["content"],
        serde_json::json!([
            {"reasoningContent": {"reasoningText": {"text": "step", "signature": "sig-1"}}},
            // Claude without a signature replays plain text.
            {"text": "unsigned"},
            // Redacted payload replays as opaque bytes; a malformed
            // payload drops the block instead of failing the request.
            {"reasoningContent": {"redactedContent": "b3BhcXVl"}},
        ])
    );

    // Non-Claude models never receive the signature member.
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42("amazon.nova-lite-v1", "Nova Lite")]);
    let mut assistant = assistant_42();
    // (routed below)
    assistant.provider = "bedrock".into();
    assistant.model = "amazon.nova-lite-v1".into();
    assistant.content = vec![AssistantContent::Thinking {
        thinking: "step".into(),
        signature: Some("sig-1".into()),
    }];
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(assistant),
    ];
    let mut request = req_42(None, "q");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid context declarations");
    request.model = "amazon.nova-lite-v1".into();
    let body = body_42(&provider, &request);
    assert_eq!(
        body["messages"][1]["content"],
        serde_json::json!([
            {"reasoningContent": {"reasoningText": {"text": "step"}}}
        ])
    );
}

#[test]
fn error_tool_result_images_split_after_turn_results() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")]);
    let pixel =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, vec![1, 2, 3, 4]);
    let tool_turn = vec![
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            content: vec![
                OutputContent::Text {
                    text: "boom".into(),
                },
                OutputContent::Image {
                    source: ImageSource::Base64 {
                        data: pixel.clone(),
                    },
                    media_type: MediaType::Png,
                },
            ],
            details: None,
            is_error: true,
            not_executed: None::<ToolNotExecutedReason>,
            timestamp_ms: 0,
            truncated: false,
        }),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "tool-2".into(),
            tool_name: "grep".into(),
            content: vec![OutputContent::Text { text: "ok".into() }],
            details: None,
            is_error: false,
            not_executed: None::<ToolNotExecutedReason>,
            timestamp_ms: 0,
            truncated: false,
        }),
    ];
    let mut messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Text { text: "q".into() }],
        timestamp_ms: 0,
    })];
    let mut assistant = assistant_42();
    assistant.content = vec![
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tool-1".into(),
                name: "read".into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        },
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "tool-2".into(),
                name: "grep".into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        },
    ];
    messages.push(Message::Assistant(assistant));
    messages.extend(tool_turn);
    let mut request = req_42(None, "q");
    request.cache_retention = CacheRetention::Disabled;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = body_42(&provider, &request);
    let messages = body["messages"].as_array().expect("messages");
    // The turn's tool results share one user message; the error images
    // follow all of them as a caption plus image blocks.
    let turn = messages.last().expect("tool turn");
    let content = turn["content"].as_array().expect("turn content");
    assert_eq!(content.len(), 4);
    assert_eq!(
        content[0]["toolResult"]["toolUseId"],
        serde_json::json!("tool-1")
    );
    assert_eq!(
        content[0]["toolResult"]["content"],
        serde_json::json!([
            {"text": "boom"},
            {"text": "(image attached after the tool results)"},
        ])
    );
    assert_eq!(
        content[1]["toolResult"]["status"],
        serde_json::json!("success")
    );
    assert_eq!(
        content[2],
        serde_json::json!({"text": "Images from the error result of tool call tool-1 (read):"})
    );
    assert_eq!(
        content[3],
        serde_json::json!({"image": {"format": "png", "source": {"bytes": pixel}}})
    );
}

#[test]
fn consecutive_tool_results_share_one_user_message() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(SONNET4, "Claude (Bedrock)")]);
    let mut messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Text { text: "q".into() }],
        timestamp_ms: 0,
    })];
    let mut assistant = assistant_42();
    assistant.content = vec![
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "t1".into(),
                name: "a".into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        },
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "t2".into(),
                name: "b".into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        },
    ];
    messages.push(Message::Assistant(assistant));
    for (id, name) in [("t1", "a"), ("t2", "b")] {
        messages.push(Message::ToolResult(ToolResultMessage {
            tool_call_id: id.into(),
            tool_name: name.into(),
            content: vec![OutputContent::Text { text: "r".into() }],
            details: None,
            is_error: false,
            not_executed: None::<ToolNotExecutedReason>,
            timestamp_ms: 0,
            truncated: false,
        }));
    }
    let mut request = req_42(None, "q");
    request.cache_retention = CacheRetention::Disabled;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = body_42(&provider, &request);
    let messages = body["messages"].as_array().expect("messages");
    let turn = messages.last().expect("tool turn");
    assert_eq!(turn["content"].as_array().expect("content").len(), 2);
}

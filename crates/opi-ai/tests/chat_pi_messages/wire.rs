//! Pi Messages endpoint, options and context projection assertions.

use super::*;

#[tokio::test]
async fn endpoint_is_messages_path_and_trailing_slashes_strip() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(format!("{}/", server.uri())))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
// wire body: exact whitelist, omissions, debug
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wire_body_is_exact_whitelist() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(
            Some(ThinkingLevel::High),
            Some(PiMessagesToolChoice::Function {
                name: "lookup".into(),
            }),
            false,
        ));
    let mut request = req_44("hi");
    request.temperature = Some(0.7);
    request.max_tokens = Some(1200);
    request.session_id = Some("sess-44".into());
    request.cache_retention = CacheRetention::Long;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let mut top: Vec<&str> = body
        .as_object()
        .expect("object body")
        .keys()
        .map(String::as_str)
        .collect();
    top.sort_unstable();
    assert_eq!(top, vec!["context", "model", "options"]);
    assert_eq!(body["model"], serde_json::json!(MODEL_44));

    let options = &body["options"];
    let mut keys: Vec<&str> = options
        .as_object()
        .expect("options object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "cacheRetention",
            "maxTokens",
            "reasoning",
            "sessionId",
            "temperature",
            "toolChoice"
        ],
        "the pi-messages whitelist is exactly these six members"
    );
    assert_eq!(options["temperature"], serde_json::json!(0.7));
    assert_eq!(options["maxTokens"], serde_json::json!(1200));
    assert_eq!(options["reasoning"], serde_json::json!("high"));
    assert_eq!(options["cacheRetention"], serde_json::json!("long"));
    assert_eq!(options["sessionId"], serde_json::json!("sess-44"));
    assert_eq!(
        options["toolChoice"],
        serde_json::json!({"type": "function", "function": {"name": "lookup"}})
    );
    server.verify().await;
}

#[tokio::test]
async fn omissions_stay_omitted_without_defaults() {
    // PI_CACHE_RETENTION is process-global; hold the env lock so the
    // legacy-env test cannot race this omission assertion.
    let _guard = env_lock_44().lock().await;
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    run_44(&provider, req_44("hi")).await;

    let body = body_44(&server).await;
    assert_eq!(
        body["options"],
        serde_json::json!({}),
        "no temperature/maxTokens/reasoning/cacheRetention/sessionId/toolChoice is invented"
    );
    server.verify().await;
}

#[tokio::test]
async fn debug_flag_adds_query_parameter() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(query_param("debug", "1"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(sse_ok_44())
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, true));
    let events = run_44(&provider, req_44("hi")).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

#[tokio::test]
async fn reasoning_wire_values_including_off() {
    for (level, wire) in [
        (ThinkingLevel::None, "off"),
        (ThinkingLevel::Minimal, "minimal"),
        (ThinkingLevel::Low, "low"),
        (ThinkingLevel::Medium, "medium"),
        (ThinkingLevel::High, "high"),
        (ThinkingLevel::XHigh, "xhigh"),
        (ThinkingLevel::Max, "max"),
    ] {
        let server = MockServer::start().await;
        mount_44(&server, sse_ok_44()).await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(Some(level), None, false));
        run_44(&provider, req_44("hi")).await;
        let body = body_44(&server).await;
        assert_eq!(body["options"]["reasoning"], serde_json::json!(wire));
        server.verify().await;
    }
}

#[tokio::test]
async fn tool_choice_wire_modes() {
    let cases = [
        (PiMessagesToolChoice::Auto, serde_json::json!("auto")),
        (PiMessagesToolChoice::None_, serde_json::json!("none")),
        (
            PiMessagesToolChoice::Required,
            serde_json::json!("required"),
        ),
        (
            PiMessagesToolChoice::Function {
                name: "search".into(),
            },
            serde_json::json!({"type": "function", "function": {"name": "search"}}),
        ),
    ];
    for (choice, wire) in cases {
        let server = MockServer::start().await;
        mount_44(&server, sse_ok_44()).await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, Some(choice), false));
        run_44(&provider, req_44("hi")).await;
        let body = body_44(&server).await;
        assert_eq!(body["options"]["toolChoice"], wire);
        server.verify().await;
    }
}

// ---------------------------------------------------------------------------
// cache retention resolution (explicit request value, legacy env opt-in)
// ---------------------------------------------------------------------------

#[test]
fn cache_retention_resolution_matrix() {
    // Unset keeps the backend default: only the legacy explicit "long"
    // opt-in maps without an explicit request value.
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, None),
        None
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("long")),
        Some("long")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("short")),
        None
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("none")),
        None
    );
    // Explicit request values win and map one-to-one onto pi's wire values.
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Disabled, Some("long")),
        Some("none")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Short, None),
        Some("short")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Long, None),
        Some("long")
    );
}

static ENV_LOCK_44: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
fn env_lock_44() -> &'static tokio::sync::Mutex<()> {
    ENV_LOCK_44.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[tokio::test]
async fn legacy_env_long_maps_when_request_unset() {
    let guard = env_lock_44().lock().await;
    unsafe { std::env::set_var("PI_CACHE_RETENTION", "long") };
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    run_44(&provider, req_44("hi")).await;
    unsafe { std::env::remove_var("PI_CACHE_RETENTION") };
    drop(guard);

    let body = body_44(&server).await;
    assert_eq!(body["options"]["cacheRetention"], serde_json::json!("long"));
    server.verify().await;
}

// ---------------------------------------------------------------------------
// canonical chronology projection (pi camelCase wire JSON)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn context_projects_pi_wire_json() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("Be brief.".into()),
        tools: vec![ToolDef {
            name: "lookup".into(),
            description: "Look things up.".into(),
            input_schema: serde_json::json!({"type": "object"}),
            constrained_sampling: None,
        }],
        messages: vec![
            Message::User(UserMessage {
                content: vec![
                    InputContent::Text { text: "see".into() },
                    InputContent::Image {
                        source: opi_ai::message::ImageSource::Base64 {
                            data: "aW1n".into(),
                        },
                        media_type: opi_ai::message::MediaType::Png,
                    },
                ],
                timestamp_ms: 1_000,
            }),
            serde_json::from_value::<opi_ai::message::Message>(serde_json::json!({
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Found it.", "signature": "sig-t"},
                    {"type": "thinking", "thinking": "hmm", "signature": "sig-h"},
                    {"type": "redacted_thinking", "data": "enc-1"},
                    {"type": "tool_call", "tool_call": {
                        "id": "call-1", "name": "lookup", "arguments": "{\"q\":\"x\"}"
                    }}
                ],
                "api": "Anthropic",
                "provider": "radius",
                "model": "claude-sonnet-4-5",
                "wire": "anthropic-messages",
                "response_id": "resp-9",
                "provider_thinking_level": "high",
                "usage": {
                    "input_tokens": 4, "output_tokens": 2, "cache_read_tokens": 1,
                    "total_tokens": 6
                },
                "stop_reason": "tool_use",
                "raw_stop_reason": "tool_use",
                "error_message": null,
                "timestamp_ms": 2_000
            }))
            .expect("assistant message"),
            serde_json::from_value::<opi_ai::message::Message>(serde_json::json!({
                "role": "tool_result",
                "tool_call_id": "call-1",
                "tool_name": "lookup",
                "content": [
                    {"type": "text", "text": "result"},
                    {"type": "image", "source": {"type": "base64", "data": "aW1n"},
                     "media_type": "image/png"}
                ],
                "details": {"rows": 3},
                "is_error": false,
                "not_executed": null,
                "truncated": false,
                "timestamp_ms": 3_000
            }))
            .expect("tool result message"),
        ],
    })
    .expect("valid context");
    let mut request = base_req_44();
    request.context = context;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let messages = body["context"]["messages"]
        .as_array()
        .expect("context messages");
    assert_eq!(messages.len(), 4);

    let system = &messages[0];
    assert_eq!(system["role"], serde_json::json!("system"));
    assert_eq!(system["content"], serde_json::json!("Be brief."));
    assert_eq!(
        system["toolsAdded"][0],
        serde_json::json!({
            "name": "lookup",
            "description": "Look things up.",
            "parameters": {"type": "object"}
        })
    );
    assert_eq!(system["timestamp"], serde_json::json!(0));

    let user = &messages[1];
    assert_eq!(user["role"], serde_json::json!("user"));
    assert_eq!(
        user["content"],
        serde_json::json!([
            {"type": "text", "text": "see"},
            {"type": "image", "data": "aW1n", "mimeType": "image/png"}
        ])
    );
    assert_eq!(user["timestamp"], serde_json::json!(1_000));

    let assistant = &messages[2];
    assert_eq!(assistant["role"], serde_json::json!("assistant"));
    assert_eq!(assistant["api"], serde_json::json!("anthropic-messages"));
    assert_eq!(assistant["provider"], serde_json::json!("radius"));
    assert_eq!(assistant["model"], serde_json::json!("claude-sonnet-4-5"));
    assert_eq!(assistant["responseId"], serde_json::json!("resp-9"));
    assert_eq!(
        assistant["providerThinkingLevel"],
        serde_json::json!("high")
    );
    assert_eq!(assistant["stopReason"], serde_json::json!("toolUse"));
    assert_eq!(assistant["rawStopReason"], serde_json::json!("tool_use"));
    assert_eq!(
        assistant["content"][0],
        serde_json::json!({"type": "text", "text": "Found it.", "textSignature": "sig-t"})
    );
    assert_eq!(
        assistant["content"][1],
        serde_json::json!({
            "type": "thinking", "thinking": "hmm", "thinkingSignature": "sig-h"
        })
    );
    assert_eq!(
        assistant["content"][2],
        serde_json::json!({
            "type": "thinking", "thinking": "", "thinkingSignature": "enc-1", "redacted": true
        })
    );
    assert_eq!(
        assistant["content"][3],
        serde_json::json!({
            "type": "toolCall", "id": "call-1", "name": "lookup", "arguments": {"q": "x"}
        })
    );
    assert_eq!(
        assistant["usage"],
        serde_json::json!({
            "input": 4, "output": 2, "cacheRead": 1, "cacheWrite": 0, "totalTokens": 6
        })
    );

    let tool_result = &messages[3];
    assert_eq!(tool_result["role"], serde_json::json!("toolResult"));
    assert_eq!(tool_result["toolCallId"], serde_json::json!("call-1"));
    assert_eq!(tool_result["toolName"], serde_json::json!("lookup"));
    assert_eq!(
        tool_result["content"],
        serde_json::json!([
            {"type": "text", "text": "result"},
            {"type": "image", "data": "aW1n", "mimeType": "image/png"}
        ])
    );
    assert_eq!(tool_result["details"], serde_json::json!({"rows": 3}));
    assert_eq!(tool_result["isError"], serde_json::json!(false));
    assert_eq!(tool_result["timestamp"], serde_json::json!(3_000));
    server.verify().await;
}

#[tokio::test]
async fn mid_conversation_system_messages_stay_native() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let mid_system: Message = serde_json::from_value(
        serde_json::json!({"role": "system", "content": "more", "timestamp_ms": 5}),
    )
    .expect("system message");
    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("base".into()),
        tools: vec![],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "one".into() }],
                timestamp_ms: 1,
            }),
            mid_system,
        ],
    })
    .expect("valid context");
    let mut request = base_req_44();
    request.context = context;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let messages = body["context"]["messages"]
        .as_array()
        .expect("context messages");
    assert_eq!(messages.len(), 3, "chronology stays native, no collapse");
    assert_eq!(messages[2]["role"], serde_json::json!("system"));
    assert_eq!(messages[2]["content"], serde_json::json!("more"));
    server.verify().await;
}

//! Native Mistral Conversations replay and partial-failure assertions.

use super::*;

#[tokio::test]
async fn mistral_producer_capture_reload_preserves_content_and_native_effort() {
    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"id\":\"capture-1\",\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"consider\"}]},{\"type\":\"text\",\"text\":\"answer\"}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"raw-call\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(4)
        .mount(&server)
        .await;
    let model = model_native(SMALL_NATIVE, true, false).with_base_url(server.uri());
    let api = MistralConversationsChatApi::new(
        MistralConversationsProvider::new(Some(server.uri())),
        model.clone(),
    )
    .unwrap();
    let mut models = opi_ai::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::AuthScheme::Bearer,
                "fixture-key".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let options = || {
        handle.options(MistralOptions::native(
            None,
            Some(MistralReasoningEffort::Medium),
            None,
        ))
    };
    let first = models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", model.clone()),
            req_native("first"),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(first.provider, "mistral");
    assert_eq!(first.wire, Some(WireApi::MistralConversations));
    assert_eq!(first.response_id.as_deref(), Some("capture-1"));
    assert_eq!(first.raw_stop_reason.as_deref(), Some("tool_calls"));
    let captured_call = first
        .content
        .iter()
        .find_map(|content| match content {
            AssistantContent::ToolCall { tool_call } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(captured_call.arguments, "{");
    let tool_result = Message::ToolResult(opi_ai::message::ToolResultMessage {
        tool_call_id: captured_call.id.clone(),
        tool_name: "lookup".into(),
        content: vec![OutputContent::Text {
            text: "invalid JSON was rejected".into(),
        }],
        details: None,
        is_error: true,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    });
    // The pinned Mistral response protocol does not supply an effort stamp.
    assert!(first.provider_thinking_level.is_none());
    let reload: opi_ai::message::AssistantMessage =
        serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    let canonical = serde_json::to_vec(&reload).unwrap();
    assert_eq!(
        serde_json::to_value(&reload).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    let mut request = req_native("second");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            Message::Assistant(reload.clone()),
            tool_result.clone(),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "continue".into(),
                }],
                timestamp_ms: 0,
            }),
        ],
    })
    .unwrap();
    models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    let mut changed_model = model.clone();
    changed_model.id = "different-mistral-model".into();
    let result = models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", changed_model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(result.model, changed_model.id);
    let mut unknown = reload.clone();
    unknown.wire = None;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            Message::Assistant(unknown.clone()),
            tool_result,
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "continue".into(),
                }],
                timestamp_ms: 0,
            }),
        ],
    })
    .unwrap();
    let unknown_canonical = serde_json::to_vec(&unknown).unwrap();
    models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", model),
            request,
            options(),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    for (index, request) in sent.iter().enumerate() {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["reasoning_effort"], "medium");
        assert_eq!(
            body["model"],
            if index == 2 {
                changed_model.id.as_str()
            } else {
                SMALL_NATIVE
            }
        );
        if index > 0 {
            let messages = body["messages"].as_array().unwrap();
            let assistant = messages
                .iter()
                .find(|message| message["role"] == "assistant")
                .unwrap();
            let call = &assistant["tool_calls"][0];
            assert_eq!(call["function"]["arguments"], "{");
            let tool = messages
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            assert_eq!(tool["tool_call_id"], call["id"]);
            assert_eq!(
                tool["content"],
                serde_json::json!([{"type":"text","text":"[tool error] invalid JSON was rejected"}])
            );
        }
    }
    let replay: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    let assistant = replay["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(
        assistant["content"],
        serde_json::json!([
            {"type":"thinking","thinking":[{"type":"text","text":"consider"}]},
            {"type":"text","text":"answer"},
        ])
    );
    for request in &sent[2..] {
        assert_eq!(request.url.path(), "/v1/chat/completions");
        assert_eq!(request.headers["authorization"], "Bearer fixture-key");
        let projected: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let assistant = projected["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(
            assistant["content"],
            serde_json::json!([{"type":"text","text":"answer"}]),
            "foreign or unknown-source thinking must not reach the native Mistral wire"
        );
    }
    assert_eq!(serde_json::to_vec(&reload).unwrap(), canonical);
    assert_eq!(serde_json::to_vec(&unknown).unwrap(), unknown_canonical);
}

pub(crate) async fn mistral_complete(
    base: &str,
    request: Request,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    let model = model_native(SMALL_NATIVE, false, false);
    let api = MistralConversationsChatApi::new(
        MistralConversationsProvider::new(Some(base.into())).with_models(vec![model.clone()]),
        model.clone(),
    )
    .unwrap();
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("fixture"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    models
        .complete(
            opi_ai::models::ChatModelRecord::new("mistral", model),
            request,
            handle.options(MistralOptions::native(None, None, None)),
        )
        .await
}

#[tokio::test]
async fn mistral_parser_failure_keeps_live_partial_and_usage() {
    for delta in [
        serde_json::json!({"content":"visible"}),
        serde_json::json!({"content":[{"type":"thinking","thinking":[{"text":"reason"}]}]}),
        serde_json::json!({"tool_calls":[{"index":0,"id":"abcdefghi","function":{"name":"lookup","arguments":"{\"q\":\"live\"}"}}]}),
        serde_json::json!({}),
    ] {
        let server = MockServer::start().await;
        let raw = serde_json::json!({"id":"partial-id","choices":[{"delta":delta}],"usage":{"prompt_tokens":11,"completion_tokens":3}});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {raw}\n\ndata: {{broken\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let failure = mistral_complete(&server.uri(), req_native("probe"))
            .await
            .expect_err("malformed stream");
        assert_eq!(failure.message().usage.input_tokens, 11);
        assert_eq!(failure.message().usage.output_tokens, 3);
        assert_eq!(failure.message().response_id.as_deref(), Some("partial-id"));
        if let Some(content) = delta.get("content") {
            if content.is_string() {
                assert!(
                    matches!(&failure.message().content[0],AssistantContent::Text{text,..} if text=="visible")
                );
            } else {
                assert!(
                    matches!(&failure.message().content[0],AssistantContent::Thinking{thinking,..} if thinking=="reason")
                );
            }
        }
        if delta.get("tool_calls").is_some() {
            assert!(
                matches!(&failure.message().content[0],AssistantContent::ToolCall{tool_call} if tool_call.arguments.contains("live"))
            );
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn mistral_direct_delta_contains_current_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_ok_native())
        .mount(&server)
        .await;
    let provider = MistralConversationsProvider::new(Some(server.uri()))
        .with_models(vec![model_native(SMALL_NATIVE, false, false)]);
    let events: Vec<_> = provider
        .stream_prepared(
            req_native("probe"),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    assert!(events.iter().any(|event|matches!(event,Ok(AssistantStreamEvent::TextDelta{delta,partial,..}) if delta=="Hi"&&matches!(&partial.content[0],AssistantContent::Text{text,..} if text=="Hi"))));
}

#[tokio::test]
async fn mistral_cache_fields_follow_source_order() {
    for (extra, expected) in [
        (serde_json::json!({"num_cached_tokens":30}), 30),
        (
            serde_json::json!({"numCachedTokens":40,"num_cached_tokens":30}),
            40,
        ),
        (
            serde_json::json!({"promptTokensDetails":{"cachedTokens":0},"num_cached_tokens":30}),
            0,
        ),
        (
            serde_json::json!({"prompt_tokens_details":{"cached_tokens":20},"num_cached_tokens":30}),
            20,
        ),
    ] {
        let server = MockServer::start().await;
        let mut usage = extra;
        usage["prompt_tokens"] = 100.into();
        usage["completion_tokens"] = 20.into();
        let raw = serde_json::json!({"choices":[{"delta":{"content":"ok"},"finish_reason":"stop"}],"usage":usage});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {raw}\n\ndata: [DONE]\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let message = mistral_complete(&server.uri(), req_native("probe"))
            .await
            .unwrap();
        assert_eq!(message.usage.cache_read_tokens, expected);
        assert_eq!(message.usage.input_tokens, 100 - expected);
    }
}

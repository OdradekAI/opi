//! Node 5 Pi Messages replay and producer regression assertions.

use super::*;

#[tokio::test]
async fn node5_models_pi_simple_bypasses_budget_and_effort_mapping_with_tool_choice() {
    for typed in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(sse_ok_44())
            .expect(2)
            .mount(&server)
            .await;
        let mut model = clamped_model_44();
        model.base_url = Some(server.uri());
        let provider = PiMessagesProvider::new(Some(server.uri())).with_models(vec![model.clone()]);
        let mut models = opi_ai::Models::new();
        let auth = std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
            opi_ai::auth::AuthScheme::Bearer,
            SecretString::from("fixture-key"),
        ));
        if typed {
            models
                .register_chat_api(
                    std::sync::Arc::new(PiMessagesChatApi::new(provider, model.clone()).unwrap()),
                    auth,
                    opi_ai::AuthProvenanceSource::Static,
                )
                .unwrap();
        } else {
            models
                .register_route(
                    Box::new(provider),
                    auth,
                    opi_ai::AuthProvenanceSource::Static,
                    Default::default(),
                )
                .unwrap();
        }
        for (max_tokens, choice) in [
            (None, opi_ai::SimpleToolChoice::Auto),
            (Some(999_999_999), opi_ai::SimpleToolChoice::None),
        ] {
            let mut request = req_44("go");
            request.max_tokens = max_tokens;
            request.thinking = ThinkingConfig {
                enabled: true,
                level: ThinkingLevel::XHigh,
                budget_tokens: None,
            };
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new("radius", model.clone()),
                    request,
                    opi_ai::SimpleOptions {
                        tool_choice: Some(choice),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
        let sent = server.received_requests().await.unwrap();
        for (index, choice) in ["auto", "none"].iter().enumerate() {
            let body: serde_json::Value = serde_json::from_slice(&sent[index].body).unwrap();
            assert_eq!(body["options"]["reasoning"], "xhigh");
            assert_eq!(body["options"]["toolChoice"], *choice);
            if index == 0 {
                assert!(body["options"].get("maxTokens").is_none());
            } else {
                assert_eq!(body["options"]["maxTokens"], 999_999_999_u64);
            }
        }
    }
}

#[tokio::test]
async fn node5_pi_terminal_error_preserves_final_accounting_and_context_envelope() {
    let server = MockServer::start().await;
    let body = node5_pi_sse(&[
        serde_json::json!({"type":"start"}),
        serde_json::json!({"type":"text_start","contentIndex":0}),
        serde_json::json!({"type":"text_delta","contentIndex":0,"delta":"partial"}),
        node5_pi_terminal("error"),
    ]);
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(body)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let failure = node5_pi_complete(&server.uri())
        .await
        .expect_err("error terminal");
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert_eq!(failure.message().usage.input_tokens, 13);
    assert_eq!(failure.message().usage.output_tokens, 7);
    assert_eq!(
        failure.message().response_id.as_deref(),
        Some("terminal-id")
    );
    assert!(
        matches!(&failure.message().content[0],opi_ai::message::AssistantContent::Text{text,..} if text=="partial")
    );
    let body = body_44(&server).await;
    assert!(body["context"].is_object());
    assert!(body["context"]["messages"].is_array());
    server.verify().await;
}

#[tokio::test]
async fn node5_pi_toolcall_end_overrides_delta_arguments() {
    for (delta, preview) in [
        (None, serde_json::json!({})),
        (Some("{\"q\":\"draft\"}"), serde_json::json!({"q":"draft"})),
        (Some("{\"flag\":true"), serde_json::json!({"flag":true})),
    ] {
        let server = MockServer::start().await;
        let mut events = vec![
            serde_json::json!({"type":"start"}),
            serde_json::json!({"type":"toolcall_start","contentIndex":0,"id":"call-1","toolName":"lookup"}),
        ];
        if let Some(delta) = delta {
            events
                .push(serde_json::json!({"type":"toolcall_delta","contentIndex":0,"delta":delta}));
        }
        events.extend([serde_json::json!({"type":"toolcall_end","contentIndex":0,"toolCall":{"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque"}}),node5_pi_terminal("done")]);
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(node5_pi_sse(&events))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider =
            PiMessagesProvider::new(Some(server.uri())).with_models(vec![model_44(MODEL_44)]);
        let events = run_44(&provider, req_44("probe")).await;
        let deltas: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Ok(AssistantStreamEvent::ToolCallDelta { partial, .. }) => Some(partial),
                _ => None,
            })
            .collect();
        assert_eq!(deltas.len(), usize::from(delta.is_some()));
        if let Some(partial) = deltas.first() {
            let opi_ai::message::AssistantContent::ToolCall { tool_call } = &partial.content[0]
            else {
                panic!("partial tool call")
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&tool_call.arguments).unwrap(),
                preview
            );
        }
        let message = done_message_44(&events).await;
        let opi_ai::message::AssistantContent::ToolCall { tool_call } = &message.content[0] else {
            panic!("tool call")
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&tool_call.arguments).unwrap(),
            serde_json::json!({"q":"final"})
        );
        assert_eq!(tool_call.signature.as_deref(), Some("opaque"));
    }
}

#[tokio::test]
async fn node5_pi_producer_capture_reload_preserves_authoritative_tools_and_effort() {
    let server = MockServer::start().await;
    let mut terminal = node5_pi_terminal("done");
    terminal["providerThinkingLevel"] = serde_json::json!("high");
    let body = node5_pi_sse(&[
        serde_json::json!({"type":"start"}),
        serde_json::json!({"type":"thinking_start","contentIndex":0}),
        serde_json::json!({"type":"thinking_end","contentIndex":0,"content":"","contentSignature":"pi-signed-empty"}),
        serde_json::json!({"type":"thinking_start","contentIndex":1}),
        serde_json::json!({"type":"thinking_end","contentIndex":1,"content":"","contentSignature":"pi-redacted","redacted":true}),
        serde_json::json!({"type":"thinking_start","contentIndex":2}),
        serde_json::json!({"type":"thinking_delta","contentIndex":2,"delta":"pi-private"}),
        serde_json::json!({"type":"thinking_end","contentIndex":2,"content":"pi-private","contentSignature":"pi-thinking-signature"}),
        serde_json::json!({"type":"text_start","contentIndex":3}),
        serde_json::json!({"type":"text_delta","contentIndex":3,"delta":"draft"}),
        serde_json::json!({"type":"text_end","contentIndex":3,"content":"pi-answer","contentSignature":"pi-text-signature"}),
        serde_json::json!({"type":"toolcall_start","contentIndex":4,"id":"call-1","toolName":"lookup"}),
        serde_json::json!({"type":"toolcall_delta","contentIndex":4,"delta":"{\"q\":\"draft\"}"}),
        serde_json::json!({"type":"toolcall_end","contentIndex":4,"toolCall":{"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque-signature"}}),
        terminal,
    ]);
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(3)
        .mount(&server)
        .await;
    let model = thinking_model_44().with_base_url(server.uri());
    let api =
        PiMessagesChatApi::new(PiMessagesProvider::new(Some(server.uri())), model.clone()).unwrap();
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
        handle.options(PiMessagesOptions::native(
            Some(ThinkingLevel::High),
            None,
            false,
        ))
    };
    let first = models
        .complete(
            opi_ai::ChatModelRecord::new("radius", model.clone()),
            req_44("first"),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(first.provider_thinking_level.as_deref(), Some("high"));
    assert_eq!(first.response_id.as_deref(), Some("terminal-id"));
    let encoded = serde_json::to_vec(&first).unwrap();
    let reload: opi_ai::message::AssistantMessage = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&reload).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    let mut request = req_44("second");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            Message::Assistant(reload.clone()),
            Message::ToolResult(opi_ai::message::ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "lookup".into(),
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "lookup-result".into(),
                }],
                details: None,
                is_error: false,
                not_executed: None,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
    })
    .unwrap();
    models
        .complete(
            opi_ai::ChatModelRecord::new("radius", model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    let mut changed_model = model;
    changed_model.id = "different-pi-model".into();
    let changed = models
        .complete(
            opi_ai::ChatModelRecord::new("radius", changed_model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(changed.model, changed_model.id);
    let sent = server.received_requests().await.unwrap();
    for (sent, target_model) in sent[1..].iter().zip([MODEL_44, changed_model.id.as_str()]) {
        assert_eq!(sent.url.path(), "/messages");
        assert_eq!(sent.headers["authorization"], "Bearer fixture-key");
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["model"], target_model);
        let replay = body["context"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(replay["provider"], "radius");
        assert_eq!(replay["api"], "pi-messages");
        assert_eq!(replay["model"], MODEL_44);
        assert_eq!(replay["providerThinkingLevel"], "high");
        assert_eq!(
            replay["content"],
            serde_json::json!([
                {"type":"thinking","thinking":"","thinkingSignature":"pi-signed-empty"},
                {"type":"thinking","thinking":"","thinkingSignature":"pi-redacted","redacted":true},
                {"type":"thinking","thinking":"pi-private","thinkingSignature":"pi-thinking-signature"},
                {"type":"text","text":"pi-answer","textSignature":"pi-text-signature"},
                {"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque-signature"}
            ])
        );
        assert_eq!(body["options"]["reasoning"], "high");
    }

    // The gateway carries canonical history. A native inference wire owns
    // the source-identity projection when it consumes that same history.
    let destination = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"target-answer\"},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                )),
        )
        .expect(1)
        .mount(&destination)
        .await;
    let target_model = opi_ai::provider::ModelInfo::new(
        "mistral-replay-target",
        "mistral-replay-target",
        WireApi::MistralConversations,
        ModelCapabilities::new(200_000, 16_384).with_streaming(true),
    )
    .with_base_url(destination.uri());
    let target = models
        .register_chat_api(
            std::sync::Arc::new(
                opi_ai::api::mistral::MistralConversationsChatApi::new(
                    opi_ai::providers::mistral::MistralConversationsProvider::new(Some(
                        destination.uri(),
                    )),
                    target_model.clone(),
                )
                .unwrap(),
            ),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::AuthScheme::Bearer,
                "destination-key".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let result = models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", target_model),
            request,
            target.options(opi_ai::api::mistral::MistralOptions::default()),
        )
        .await
        .unwrap();
    assert_eq!(result.provider, "mistral");
    assert_eq!(result.wire, Some(WireApi::MistralConversations));
    let projected = destination.received_requests().await.unwrap();
    assert_eq!(
        projected[0].headers["authorization"],
        "Bearer destination-key"
    );
    let projected: serde_json::Value = serde_json::from_slice(&projected[0].body).unwrap();
    let messages = projected["messages"].as_array().unwrap();
    let assistant = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(
        assistant["content"],
        serde_json::json!([{"type":"text","text":"pi-answer"}])
    );
    let call = &assistant["tool_calls"][0];
    assert_eq!(call["function"]["name"], "lookup");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(call["function"]["arguments"].as_str().unwrap())
            .unwrap(),
        serde_json::json!({"q":"final"})
    );
    let tool_result = messages
        .iter()
        .find(|message| message["role"] == "tool")
        .unwrap();
    assert_eq!(tool_result["tool_call_id"], call["id"]);
    assert_eq!(
        tool_result["content"],
        serde_json::json!([{"type":"text","text":"lookup-result"}])
    );
    for secret in [
        "pi-private",
        "pi-signed-empty",
        "pi-redacted",
        "pi-thinking-signature",
        "pi-text-signature",
        "opaque-signature",
    ] {
        assert!(!projected.to_string().contains(secret));
    }
    assert_eq!(serde_json::to_vec(&reload).unwrap(), encoded);
}

#[tokio::test]
async fn node5_pi_split_utf8_preserves_delta_and_final_text() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0; 1];
            socket.read_exact(&mut byte).await.unwrap();
            head.push(byte[0]);
        }
        let content_length = std::str::from_utf8(&head)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim().parse::<usize>().unwrap())
            .unwrap();
        let mut input = vec![0; content_length];
        socket.read_exact(&mut input).await.unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        let body = node5_pi_sse(&[
            serde_json::json!({"type":"start"}),
            serde_json::json!({"type":"text_start","contentIndex":0}),
            serde_json::json!({"type":"text_delta","contentIndex":0,"delta":"中"}),
            serde_json::json!({"type":"text_end","contentIndex":0,"content":"中"}),
            node5_pi_terminal("done"),
        ])
        .into_bytes();
        let mut bounds = vec![0];
        bounds.extend(
            body.iter()
                .enumerate()
                .filter_map(|(i, b)| (*b == 0xe4).then_some(i + 1)),
        );
        bounds.push(body.len());
        for range in bounds.windows(2) {
            let bytes = &body[range[0]..range[1]];
            socket
                .write_all(format!("{:x}\r\n", bytes.len()).as_bytes())
                .await
                .unwrap();
            socket.write_all(bytes).await.unwrap();
            socket.write_all(b"\r\n").await.unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
        socket.write_all(b"0\r\n\r\n").await.unwrap();
    });
    let provider = PiMessagesProvider::new(Some(format!("http://{address}")))
        .with_models(vec![model_44(MODEL_44)]);
    let events = run_44(&provider, req_44("probe")).await;
    let deltas: String = events
        .iter()
        .filter_map(|event| match event {
            Ok(AssistantStreamEvent::TextDelta { delta, .. }) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(deltas, "中");
    let message = done_message_44(&events).await;
    assert!(
        matches!(&message.content[0],opi_ai::message::AssistantContent::Text{text,..} if text=="中")
    );
    task.await.unwrap();
}

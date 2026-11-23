//! Native Mistral Conversations lifecycle and message projection assertions.

use super::*;

// -- headers-only timeout boundary ----------------------------------------

async fn spawn_raw_native(delay_headers: Duration, delay_body: Duration) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = Vec::new();
        let mut buf = [0u8; 2048];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .unwrap();
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        // Drain the request body before responding.
        let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let declared = String::from_utf8_lossy(&request[..header_end])
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        while request.len() < header_end + 4 + declared {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .unwrap();
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
        }

        tokio::time::sleep(delay_headers).await;
        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
        write_chunk(
            &mut socket,
            "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n",
        )
        .await
        .unwrap();
        tokio::time::sleep(delay_body).await;
        write_chunk(
            &mut socket,
            concat!(
                "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            ),
        )
        .await
        .unwrap();
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn header_wait_deadline_times_out_before_headers() {
    let uri = spawn_raw_native(Duration::from_millis(900), Duration::from_millis(50)).await;
    let provider = MistralConversationsProvider::new(Some(uri)).with_models(vec![model_native(
        SMALL_NATIVE,
        false,
        false,
    )]);
    let mut request = req_native("go");
    request.timeout = Some(Duration::from_millis(250));
    let started = Instant::now();
    let events = run_native(&provider, request).await;
    let error = first_error_native(&events).await;
    assert!(matches!(error, ProviderError::Timeout), "got {error}");
    assert!(
        started.elapsed() < Duration::from_millis(800),
        "header deadline must fire fast"
    );
}

#[tokio::test]
async fn stalled_body_after_headers_is_not_under_the_header_deadline() {
    // The default deadline covers only the wait for response headers: a
    // body stall past the header budget must still complete.
    let uri = spawn_raw_native(Duration::from_millis(50), Duration::from_millis(900)).await;
    let provider = MistralConversationsProvider::new(Some(uri)).with_models(vec![model_native(
        SMALL_NATIVE,
        false,
        false,
    )]);
    let mut request = req_native("go");
    request.timeout = Some(Duration::from_millis(250));
    let events = run_native(&provider, request).await;
    let message = done_message_native(&events).await;
    assert_eq!(message.stop_reason, StopReason::Stop);
}

// -- message conversion ----------------------------------------------------

#[tokio::test]
async fn message_conversion_projects_mistral_shapes() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, true),
        MistralOptions::default(),
    );
    let mut assistant = opi_ai::message::AssistantMessage {
        projection: None,
        content: vec![
            AssistantContent::Thinking {
                thinking: "step".into(),
                signature: None,
            },
            AssistantContent::Text {
                text: "  ".into(),
                signature: None,
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "call_abcdefgh1234567".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "x.rs"}).to_string(),
                    arguments_error: None,
                    signature: None,
                },
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "call_second_error_id".into(),
                    name: "read_file".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            },
        ],
        api: opi_ai::ApiKind::OpenAi,
        provider: "mistral".into(),
        model: SMALL_NATIVE.into(),
        response_model: None,
        wire: Some(WireApi::MistralConversations),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    };
    assistant.content[1] = AssistantContent::Text {
        text: "  ".into(),
        signature: None,
    };
    let mut request = base_req_native();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("Be brief.".into()),
        messages: vec![
            Message::User(UserMessage {
                content: vec![
                    InputContent::Text {
                        text: "look".into(),
                    },
                    InputContent::Image {
                        source: opi_ai::message::ImageSource::Bytes {
                            data: vec![0xff, 0xd8, 0xff],
                        },
                        media_type: MediaType::Jpeg,
                    },
                ],
                timestamp_ms: 0,
            }),
            Message::Assistant(assistant),
            Message::ToolResult(opi_ai::message::ToolResultMessage {
                not_executed: None,
                tool_call_id: "call_abcdefgh1234567".into(),
                tool_name: "read_file".into(),
                content: vec![
                    OutputContent::Text {
                        text: "  ok  ".into(),
                    },
                    OutputContent::Image {
                        source: opi_ai::message::ImageSource::Bytes {
                            data: vec![0xff, 0xd8, 0xff],
                        },
                        media_type: MediaType::Jpeg,
                    },
                ],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 0,
            }),
            Message::ToolResult(opi_ai::message::ToolResultMessage {
                not_executed: None,
                tool_call_id: "call_second_error_id".into(),
                tool_name: "read_file".into(),
                content: vec![],
                details: None,
                is_error: true,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(
        messages[0]["role"],
        serde_json::json!("system"),
        "the normalized leading system prompt stays first"
    );

    let user = &messages[1];
    assert_eq!(user["role"], serde_json::json!("user"));
    assert_eq!(user["content"][0]["type"], serde_json::json!("text"));
    assert_eq!(
        user["content"][1]["image_url"],
        serde_json::json!("data:image/jpeg;base64,/9j/")
    );

    let assistant_msg = &messages[2];
    assert_eq!(assistant_msg["role"], serde_json::json!("assistant"));
    assert_eq!(assistant_msg["prefix"], serde_json::json!(false));
    // Blank text drops; thinking replays as a chunked thinking entry.
    assert_eq!(
        assistant_msg["content"][0]["type"],
        serde_json::json!("thinking")
    );
    assert_eq!(
        assistant_msg["content"][0]["thinking"][0]["text"],
        serde_json::json!("step")
    );
    // Long tool-call ids normalize to 9 alnum chars, mirrored into the
    // tool result's tool_call_id so the pairing stays consistent.
    let wire_id = assistant_msg["tool_calls"][0]["id"]
        .as_str()
        .expect("tool call id");
    assert_eq!(wire_id.len(), 9, "id normalizes to 9 chars: {wire_id}");
    assert!(wire_id.chars().all(|c| c.is_ascii_alphanumeric()));
    assert_eq!(
        assistant_msg["tool_calls"][0]["function"]["arguments"],
        serde_json::json!("{\"path\":\"x.rs\"}")
    );

    let tool_ok = &messages[3];
    assert_eq!(tool_ok["role"], serde_json::json!("tool"));
    assert_eq!(tool_ok["tool_call_id"], serde_json::json!(wire_id));
    assert_eq!(tool_ok["name"], serde_json::json!("read_file"));
    assert_eq!(tool_ok["content"][0]["text"], serde_json::json!("ok"));
    assert_eq!(
        tool_ok["content"][1]["type"],
        serde_json::json!("image_url")
    );

    let tool_err = &messages[4];
    assert_eq!(
        tool_err["content"][0]["text"],
        serde_json::json!("[tool error] (no tool output)")
    );
}

#[tokio::test]
async fn non_image_models_get_placeholder_projections() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = base_req_native();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Image {
                source: opi_ai::message::ImageSource::Bytes {
                    data: vec![0xff, 0xd8, 0xff],
                },
                media_type: MediaType::Jpeg,
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    assert_eq!(
        body["messages"][0]["content"],
        serde_json::json!("(image omitted: model does not support images)")
    );
}

#[tokio::test]
async fn mid_conversation_system_messages_collapse() {
    let server = MockServer::start().await;
    mount_native(&server, sse_ok_native()).await;
    let provider = provider_native(
        Some(&server.uri()),
        model_native(SMALL_NATIVE, false, false),
        MistralOptions::default(),
    );
    let mut request = base_req_native();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("root system".into()),
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "one".into() }],
                timestamp_ms: 0,
            }),
            Message::System(opi_ai::transcript::SystemMessage {
                content: "mid system".into(),
                sections: vec![],
                tools_added: vec![],
                tools_removed: vec![],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "two".into() }],
                timestamp_ms: 0,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    run_native(&provider, request).await;
    let body = body_native(&server).await;
    let messages = body["messages"].as_array().expect("messages array");
    let roles: Vec<_> = messages
        .iter()
        .map(|message| message["role"].as_str().unwrap_or_default())
        .collect();
    // Mistral takes no mid-conversation system role: collapse replays
    // the merged current system state as the leading message and drops
    // the in-place update (pi resolveTranscript + opi collapse policy).
    assert_eq!(roles, vec!["system", "user", "user"]);
    assert_eq!(
        messages[0]["content"],
        serde_json::json!("root system\n\nmid system")
    );
    assert_eq!(messages[1]["content"], serde_json::json!("one"));
}

// -- lifecycle -------------------------------------------------------------

#[test]
fn wrong_wire_record_is_rejected_typed() {
    let provider = MistralConversationsProvider::new(None);
    let openai_model = opi_ai::provider::ModelInfo::new(
        SMALL_NATIVE,
        SMALL_NATIVE,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(32_000, 8_192),
    );
    let error = MistralConversationsChatApi::new(provider, openai_model)
        .expect_err("wrong wire must be rejected");
    assert!(matches!(error, ProviderError::Config(_)), "got {error}");
}

#[tokio::test]
async fn native_lifecycle_through_models_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer mistral-key"))
        .and(body_partial_json(serde_json::json!({
            "model": "magistral-medium-latest",
            "reasoning_effort": "medium"
        })))
        .respond_with(sse_ok_native())
        .expect(1)
        .mount(&server)
        .await;

    let model = effort_model_native(
        "magistral-medium-latest",
        map_native(&[
            (ThinkingLevel::Low, "low"),
            (ThinkingLevel::Medium, "medium"),
        ]),
    );
    let api = MistralConversationsChatApi::new(
        MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
        model,
    )
    .expect("valid mistral record");
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("mistral-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let mut request = req_native("go");
    request.model = "mistral:magistral-medium-latest".into();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::Medium,
        budget_tokens: None,
    };
    let options = MistralOptions::native(None, Some(MistralReasoningEffort::Medium), None);
    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new(
                "mistral",
                effort_model_native(
                    "magistral-medium-latest",
                    map_native(&[
                        (ThinkingLevel::Low, "low"),
                        (ThinkingLevel::Medium, "medium"),
                    ]),
                ),
            ),
            request,
            handle.options(options),
        )
        .await
        .expect("native lifecycle completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
    server.verify().await;
}

#[tokio::test]
async fn models_mistral_simple_tool_choice_reaches_wire() {
    for typed in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(sse_ok_native())
            .expect(2)
            .mount(&server)
            .await;
        let model = model_native(SMALL_NATIVE, false, false).with_base_url(server.uri());
        let provider =
            MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model.clone()]);
        let auth = std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
            opi_ai::AuthScheme::Bearer,
            "fixture-key".into(),
        ));
        let mut models = opi_ai::Models::new();
        if typed {
            models
                .register_chat_api(
                    std::sync::Arc::new(
                        MistralConversationsChatApi::new(provider, model.clone()).unwrap(),
                    ),
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
        for choice in [
            opi_ai::SimpleToolChoice::Auto,
            opi_ai::SimpleToolChoice::None,
        ] {
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new("mistral", model.clone()),
                    req_native("go"),
                    opi_ai::SimpleOptions {
                        tool_choice: Some(choice),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
        let sent = server.received_requests().await.unwrap();
        for (request, expected) in sent.iter().zip(["auto", "none"]) {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["tool_choice"], expected);
        }
    }
}

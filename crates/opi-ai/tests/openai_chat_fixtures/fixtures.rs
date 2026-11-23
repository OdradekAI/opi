//! Shared OpenAI Chat fixture and request-builder helpers.

use super::*;

pub(super) fn map_fixture(input: &str) -> Vec<AssistantStreamEvent> {
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Done | ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "openai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut stream_events: Vec<_> = events
        .into_iter()
        .flat_map(|e| mapper.process(e))
        .map(|item| item.expect("fixture maps to stream events"))
        .collect();
    stream_events.extend(
        mapper
            .finish(true)
            .into_iter()
            .map(|item| item.expect("fixture terminal")),
    );
    stream_events
}

#[allow(dead_code)]
pub(super) fn map_fixture_as(
    input: &str,
    api: opi_ai::ApiKind,
    provider: &str,
) -> Vec<AssistantStreamEvent> {
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Done | ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        api,
        provider,
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut stream_events: Vec<_> = events
        .into_iter()
        .flat_map(|e| mapper.process(e))
        .map(|item| item.expect("fixture maps to stream events"))
        .collect();
    stream_events.extend(
        mapper
            .finish(true)
            .into_iter()
            .map(|item| item.expect("fixture terminal")),
    );
    stream_events
}

pub(super) fn collect_valid_events(input: &str) -> Vec<OpenAiChatEvent> {
    parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Done | ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect()
}

pub(super) async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

pub(super) async fn write_chunk(
    socket: &mut tokio::net::TcpStream,
    body: &str,
) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

pub(super) async fn spawn_stalled_openai_chat_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled OpenAI Chat server");
    let addr = listener.local_addr().expect("stalled server addr");

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept stalled stream");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];

        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let request_text = String::from_utf8_lossy(&request);
        assert!(
            request_text.starts_with("POST /v1/chat/completions "),
            "unexpected request line: {request_text}"
        );

        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write response headers");
        write_chunk(&mut socket, stalled_openai_chat_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_openai_chat_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

pub(super) fn stalled_openai_chat_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

pub(super) fn stalled_openai_chat_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    )
}

pub(super) fn text_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":25,"completion_tokens":8,"total_tokens":33}}

data: [DONE]

"#
}

pub(super) fn tool_call_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"/tmp/test\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":50,"completion_tokens":30,"total_tokens":80}}

data: [DONE]

"#
}

pub(super) fn cache_tokens_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-cache","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-cache","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"prompt_tokens_details":{"cached_tokens":400}}}

data: [DONE]

"#
}

pub(super) fn reasoning_tokens_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-reason","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-reason","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":500,"total_tokens":600,"completion_tokens_details":{"reasoning_tokens":300}}}

data: [DONE]

"#
}

pub(super) fn reasoning_malformed_fixture() -> &'static str {
    // reasoning_tokens (800) > completion_tokens (500) is malformed.
    r#"data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":500,"total_tokens":600,"completion_tokens_details":{"reasoning_tokens":800}}}

data: [DONE]

"#
}

pub(super) fn missing_usage_fixture() -> &'static str {
    // Final chunk carries finish_reason but no `usage` field.
    r#"data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"ok"}}]}

data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

"#
}

pub(super) fn reported_zero_usage_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"ok"}}]}

data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}}

data: [DONE]

"#
}

pub(super) fn error_fixture() -> &'static str {
    r#"data: {"error":{"message":"Rate limit exceeded","type":"rate_limit_error","param":null,"code":"rate_limit_exceeded"}}

"#
}

pub(super) fn mixed_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Let me read that."},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"call_123","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"path\":\"src/main.rs\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":30,"completion_tokens":20,"total_tokens":50}}

data: [DONE]

"#
}

pub(super) fn make_test_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
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

pub(super) fn thinking_chat_model(id: &str, thinking: bool) -> ModelInfo {
    let mut model = ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(thinking),
    );
    if thinking {
        model = model.with_thinking_level_map(opi_ai::ThinkingLevelMap::reasoning_default());
    }
    model
}

pub(super) fn profile_provider(
    provider_id: &str,
    compat: opi_ai::providers::openai::CompatConfig,
    models: Vec<ModelInfo>,
) -> OpenAiChatProvider {
    OpenAiChatProvider::new_for_profile(
        format!("https://{provider_id}.example.test"),
        provider_id.into(),
        compat,
        vec![],
        models,
    )
}

pub(super) fn request_for(provider: &str, model: &str) -> Request {
    let mut request = make_test_request();
    request.model = format!("{provider}:{model}");
    request
}

pub(super) fn thinking_request(
    provider: &str,
    model: &str,
    level: Option<opi_ai::ThinkingLevel>,
) -> Request {
    let mut request = request_for(provider, model);
    request.thinking = ThinkingConfig {
        enabled: level.is_some(),
        budget_tokens: None,
        level: level.unwrap_or(opi_ai::ThinkingLevel::None),
    };
    request
}

pub(super) fn make_tool_request_for(provider: &str) -> Request {
    let mut request = make_tool_request();
    request.model = format!("{provider}:list_dir");
    // The tool fixture's model id is not in the catalog; register one that is.
    request.model = format!("{provider}:glm");
    request
}

pub(super) fn make_tool_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "list files".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "list_dir".into(),
                description: "list a directory".into(),
                input_schema: serde_json::json!({"type":"object","properties":{}}),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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

pub(super) fn tool_loop_context(
    tool_content: Vec<OutputContent>,
    is_error: bool,
    follow_up_user: bool,
) -> opi_ai::transcript::Context {
    let mut messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "run it".into(),
            }],
            timestamp_ms: 0,
        }),
        Message::Assistant(AssistantMessage {
            projection: None,
            content: vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "call_1".into(),
                    name: "list_dir".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            }],
            api: opi_ai::ApiKind::OpenAi,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            response_model: None,
            wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
            response_id: None,
            usage: Default::default(),
            stop_reason: opi_ai::stream::StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: Vec::new(),
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,

            deferred: None,
        }),
        Message::ToolResult(ToolResultMessage {
            not_executed: None,
            tool_call_id: "call_1".into(),
            tool_name: "list_dir".into(),
            content: tool_content,
            details: None,
            is_error,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    if follow_up_user {
        messages.push(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "continue".into(),
            }],
            timestamp_ms: 1,
        }));
    }
    opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    }
}

pub(super) fn tool_loop_request(
    tool_content: Vec<OutputContent>,
    is_error: bool,
    follow_up_user: bool,
) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(tool_loop_context(
            tool_content,
            is_error,
            follow_up_user,
        ))
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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

pub(super) fn image_tool_content() -> Vec<OutputContent> {
    vec![
        OutputContent::Text {
            text: "chart data".into(),
        },
        OutputContent::Image {
            source: ImageSource::Base64 {
                data: "aW1n".into(),
            },
            media_type: MediaType::Png,
        },
    ]
}

pub(super) fn make_tool_result_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "run it".into(),
                    }],
                    timestamp_ms: 0,
                }),
                Message::Assistant(AssistantMessage {
                    projection: None,
                    content: vec![AssistantContent::ToolCall {
                        tool_call: ToolCall {
                            id: "call_1".into(),
                            name: "list_dir".into(),
                            arguments: "{}".into(),
                            arguments_error: None,
                            signature: None,
                        },
                    }],
                    api: opi_ai::ApiKind::OpenAi,
                    provider: "openai".into(),
                    model: "gpt-4o".into(),
                    response_model: None,
                    wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                    response_id: None,
                    usage: Default::default(),
                    stop_reason: opi_ai::stream::StopReason::ToolUse,
                    error_message: None,
                    timestamp_ms: 0,
                    diagnostics: Vec::new(),
                    duration_ms: None,
                    raw_stop_reason: None,
                    provider_thinking_level: None,

                    deferred: None,
                }),
                Message::ToolResult(ToolResultMessage {
                    not_executed: None,
                    tool_call_id: "call_1".into(),
                    tool_name: "list_dir".into(),
                    content: vec![OutputContent::Text {
                        text: "a.rs".into(),
                    }],
                    details: None,
                    is_error: false,
                    truncated: false,
                    timestamp_ms: 0,
                }),
            ],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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

pub(super) fn multi_tool_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"read_file","arguments":""}},{"index":1,"id":"call_2","type":"function","function":{"name":"bash","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\"a.rs\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"cmd\":\"ls\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":60,"completion_tokens":40,"total_tokens":100}}

data: [DONE]

"#
}

pub(super) fn malformed_tool_args_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_bad","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{not-json"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}

data: [DONE]

"#
}

pub(super) fn chat_tool_fixture(arguments: &str) -> String {
    let escaped = arguments.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{\"tool_calls\":[{{\"index\":0,\"id\":\"call_a\",\"type\":\"function\",\"function\":{{\"name\":\"read_file\",\"arguments\":\"\"}}}}]}},\"finish_reason\":null}}]}}\n\n\
         data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{\"tool_calls\":[{{\"index\":0,\"function\":{{\"arguments\":\"{escaped}\"}}}}]}},\"finish_reason\":null}}]}}\n\n\
         : keepalive comment frames are ignored\n\n\
         data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}],\"usage\":{{\"prompt_tokens\":5,\"completion_tokens\":3}}}}\n\n\
         data: [DONE]\n\n"
    )
}

pub(super) async fn chat_final_tool_call(arguments: &str) -> AssistantStreamEvent {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_tool_fixture(arguments))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let request = opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    use opi_ai::provider::Provider as _;
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    futures_util::StreamExt::collect::<Vec<_>>(stream)
        .await
        .into_iter()
        .find_map(|result| match result.expect("valid chat tool stream") {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => {
                Some(AssistantStreamEvent::ToolCallEnd {
                    content_index: 0,
                    tool_call,
                    partial: opi_ai::test_support::base_assistant(),
                })
            }
            _ => None,
        })
        .expect("chat tool call terminal")
}

pub(super) fn tool_call_of(event: AssistantStreamEvent) -> opi_ai::message::ToolCall {
    match event {
        AssistantStreamEvent::ToolCallEnd { tool_call, .. } => tool_call,
        _ => panic!("expected ToolCallEnd"),
    }
}

pub(super) fn chat_callbacks_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Some(
            serde_json::json!({"top_p": 0.9})
                .as_object()
                .cloned()
                .unwrap(),
        ),
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

pub(super) fn chat_callbacks_sse() -> String {
    concat!(
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

pub(super) fn chat_sse_ok() -> String {
    concat!(
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

pub(super) fn chat_public_request(model: &str, thinking: Option<opi_ai::ThinkingLevel>) -> Request {
    let mut request = make_test_request();
    request.model = format!("openai:{model}");
    if let Some(level) = thinking {
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        };
    }
    request
}

pub(super) async fn registered_chat_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    Models,
    opi_ai::api::contract::ChatApiHandle<OpenAiCompletionsOptions>,
) {
    let provider = StdArc::new(
        OpenAiChatProvider::new_with_compat(Some(server_uri), Default::default())
            .with_model_overrides(Default::default()),
    );
    let api = OpenAiCompletionsChatApi::new(provider, model).expect("valid chat record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            StdArc::new(api),
            StdArc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-chat-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register chat api");
    (models, handle)
}

pub(super) fn o3_record() -> ChatModelRecord {
    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    ChatModelRecord::new("openai", model)
}

pub(super) async fn chat_duration_on_peer(
    body_delay: std::time::Duration,
    setup_delay: std::time::Duration,
) -> (opi_ai::message::AssistantMessage, std::time::Duration) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut bytes = [0; 4096];
        loop {
            let read = socket.read(&mut bytes).await.unwrap();
            assert!(read > 0, "request terminated before its full body");
            request.extend_from_slice(&bytes[..read]);
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if request.len() >= end + 4 + length {
                    assert!(
                        headers.starts_with("post /v1/chat/completions http/1.1"),
                        "{headers}"
                    );
                    assert!(headers.contains("\r\nauthorization: bearer test-key"));
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                    assert_eq!(body["model"], "gpt-4o");
                    break;
                }
            }
        }
        tokio::time::sleep(setup_delay).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        socket.flush().await.unwrap();
        tokio::time::sleep(body_delay).await;
        let body = chat_sse_ok();
        socket
            .write_all(format!("{:x}\r\n{}\r\n0\r\n\r\n", body.len(), body).as_bytes())
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    struct DelayedAuth {
        delay: std::time::Duration,
        calls: Arc<AtomicUsize>,
    }
    impl opi_ai::auth::AuthResolver for DelayedAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                tokio::time::sleep(self.delay).await;
                Ok(opi_ai::test_support::resolved_bearer_auth())
            })
        }
    }
    let provider = OpenAiChatProvider::new(Some(format!("http://{address}")));
    let model = provider
        .models()
        .iter()
        .find(|model| model.id == "gpt-4o")
        .unwrap()
        .clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(DelayedAuth {
                delay: setup_delay,
                calls: calls.clone(),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = chat_public_request("gpt-4o", None);
    let (terminal_tx, terminal_rx) = tokio::sync::oneshot::channel();
    let terminal_tx = std::sync::Mutex::new(Some(terminal_tx));
    request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_async_on_response(move |status, _| async move {
            assert_eq!(status, 200);
            tokio::time::sleep(setup_delay).await;
            Ok(())
        })
        .with_on_raw(Arc::new(move |event| {
            if event["choices"][0]["finish_reason"] == "stop" {
                terminal_tx
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .send(())
                    .unwrap();
            }
            Ok(())
        }));
    let started = std::time::Instant::now();
    let call = models.stream_simple(
        opi_ai::ChatModelRecord::new("openai", model),
        request,
        Default::default(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), terminal_rx)
        .await
        .expect("unconsumed execution receives the raw terminal")
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "execution authenticates without consumption"
    );
    // The source proceeds past the terminal independently while the consumer
    // waits. This interval must not inflate the recorded body duration.
    tokio::time::sleep(setup_delay).await;
    let message = call
        .result()
        .await
        .expect("controlled real adapter succeeds");
    let elapsed = started.elapsed();
    peer.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(message.usage.input_tokens, 10);
    assert_eq!(message.usage.output_tokens, 5);
    (message, elapsed)
}

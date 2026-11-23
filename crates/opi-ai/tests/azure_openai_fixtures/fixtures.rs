//! Shared Azure OpenAI Chat fixture builders and local peers.

use super::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn make_provider() -> AzureOpenAIProvider {
    AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap()
}

pub(crate) fn wire_adapter(provider: &AzureOpenAIProvider) -> AzureOpenAIWireAdapter {
    AzureOpenAIWireAdapter::new(provider.clone())
}

pub(crate) fn make_provider_with_deployments(deployments: Vec<&str>) -> AzureOpenAIProvider {
    AzureOpenAIProvider::from_config(
        Some("https://myresource.openai.azure.com".into()),
        deployments.into_iter().map(|s| s.into()).collect(),
        Some("2024-06-01".into()),
    )
    .unwrap()
}

pub(crate) fn text_request() -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "azure:my-gpt4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(256),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
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

pub(crate) fn tool_request() -> opi_ai::provider::Request {
    use opi_ai::message::{InputContent, ToolDef, UserMessage};
    use opi_ai::provider::Request;

    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "azure:my-gpt4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "What is the weather?".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "get_weather".into(),
                description: "Get weather".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    }
                }),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(256),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
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

/// OpenAI-compatible SSE fixture for a simple text response.
pub(crate) fn text_sse_fixture() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" there\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
        "data: [DONE]\n\n",
    )
}

/// SSE fixture with tool calls.
pub(crate) fn tool_call_sse_fixture() -> String {
    let c1 = serde_json::json!({
        "id": "chatcmpl-2", "object": "chat.completion.chunk", "created": 1,
        "model": "gpt-4o",
        "choices": [{"index": 0, "delta": {"role": "assistant", "content": null}, "finish_reason": null}]
    }).to_string();
    let c2 = serde_json::json!({
        "id": "chatcmpl-2", "object": "chat.completion.chunk", "created": 1,
        "model": "gpt-4o",
        "choices": [{"index": 0, "delta": {"tool_calls": [{"index": 0, "id": "call_abc", "type": "function", "function": {"name": "get_weather", "arguments": ""}}]}, "finish_reason": null}]
    }).to_string();
    let args_json = serde_json::json!({"city": "London"}).to_string();
    let c3 = serde_json::json!({
        "id": "chatcmpl-2", "object": "chat.completion.chunk", "created": 1,
        "model": "gpt-4o",
        "choices": [{"index": 0, "delta": {"tool_calls": [{"index": 0, "function": {"arguments": args_json}}]}, "finish_reason": null}]
    }).to_string();
    let c4 = serde_json::json!({
        "id": "chatcmpl-2", "object": "chat.completion.chunk", "created": 1,
        "model": "gpt-4o",
        "choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}],
        "usage": {"prompt_tokens": 20, "completion_tokens": 15, "total_tokens": 35}
    })
    .to_string();
    format!("data: {c1}\n\ndata: {c2}\n\ndata: {c3}\n\ndata: {c4}\n\ndata: [DONE]\n\n")
}

/// SSE fixture with an error response.
pub(crate) fn error_sse_fixture() -> &'static str {
    "data: {\"error\":{\"message\":\"Deployment not found\",\"type\":\"invalid_request_error\"}}\n\n"
}

pub(crate) async fn collect_events(
    stream: opi_ai::provider::EventStream,
) -> Vec<AssistantStreamEvent> {
    // Match the historical collector: failures terminate collection and are
    // not surfaced as events (error-specific tests use `collect_items`).
    collect_items(stream)
        .await
        .into_iter()
        .filter_map(|item| item.ok())
        .collect()
}

/// Collect raw stream items (events and typed failures) until the first
/// terminal event or failure.
pub(crate) async fn collect_items(
    stream: opi_ai::provider::EventStream,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    let mut events = Vec::new();
    let mut stream = std::pin::pin!(stream);
    while let Some(item) = stream.next().await {
        let is_terminal = match &item {
            Ok(event) => event.is_terminal(),
            Err(_) => true,
        };
        events.push(item);
        if is_terminal {
            break;
        }
    }
    events
}

#[derive(Clone, Copy)]
pub(crate) enum AzureStallPoint {
    BeforeHeaders,
    ResponseBody,
}

pub(crate) async fn spawn_stalled_azure_server(
    stall_point: AzureStallPoint,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Azure server");
    let addr = listener.local_addr().expect("stalled server addr");
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();

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
            request_text.starts_with(
                "POST /openai/deployments/my-gpt4o/chat/completions?api-version=2024-06-01 "
            ),
            "unexpected request line: {request_text}"
        );

        if !matches!(stall_point, AzureStallPoint::BeforeHeaders) {
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

pub(crate) async fn spawn_stalled_azure_error_body_server(
    status: u16,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Azure error server");
    let addr = listener.local_addr().expect("stalled error server addr");
    let headers_flushed = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_headers_flushed = headers_flushed.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Azure request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Azure request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let reason = match status {
            401 => "Unauthorized",
            403 => "Forbidden",
            _ => "Error",
        };
        let headers = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: 32\r\nConnection: close\r\n\r\n"
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, headers.as_bytes())
            .await
            .expect("write Azure error headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush Azure error headers");
        server_headers_flushed.notify_one();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    (format!("http://{addr}"), headers_flushed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

pub(crate) fn direct_error_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        other => other,
    }
}

pub(crate) fn assert_direct_projection_prefix(
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

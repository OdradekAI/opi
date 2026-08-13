//! Azure OpenAI provider fixture tests (task 3.2).
//!
//! Tests cover: text streaming, tool calls, usage, errors, secret redaction,
//! URL construction, and api-key auth header. All use deterministic SSE
//! fixture data — no live Azure calls.

use futures_util::StreamExt;
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::provider::{CacheRetention, Provider};
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_provider() -> AzureOpenAIProvider {
    AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap()
}

fn make_provider_with_deployments(deployments: Vec<&str>) -> AzureOpenAIProvider {
    AzureOpenAIProvider::from_config(
        Some("https://myresource.openai.azure.com".into()),
        deployments.into_iter().map(|s| s.into()).collect(),
        Some("2024-06-01".into()),
    )
    .unwrap()
}

fn text_request() -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        model: "azure:my-gpt4o".into(),
        system: Some("You are helpful.".into()),
        messages: vec![opi_ai::message::Message::User(
            opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            },
        )],
        tools: vec![],
        max_tokens: Some(256),
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn tool_request() -> opi_ai::provider::Request {
    use opi_ai::message::{InputContent, ToolDef, UserMessage};
    use opi_ai::provider::Request;

    Request {
        model: "azure:my-gpt4o".into(),
        system: None,
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
        }],
        max_tokens: Some(256),
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

/// OpenAI-compatible SSE fixture for a simple text response.
fn text_sse_fixture() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" there\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
        "data: [DONE]\n\n",
    )
}

/// SSE fixture with tool calls.
fn tool_call_sse_fixture() -> String {
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
fn error_sse_fixture() -> &'static str {
    "data: {\"error\":{\"message\":\"Deployment not found\",\"type\":\"invalid_request_error\"}}\n\n"
}

async fn collect_events(stream: opi_ai::provider::EventStream) -> Vec<AssistantStreamEvent> {
    let mut events = Vec::new();
    let mut stream = std::pin::pin!(stream);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = matches!(
                    event,
                    AssistantStreamEvent::Done { .. } | AssistantStreamEvent::Error { .. }
                );
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => {
                eprintln!("stream error: {e}");
                break;
            }
        }
    }
    events
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

async fn spawn_stalled_azure_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Azure server");
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
            request_text.starts_with(
                "POST /openai/deployments/my-gpt4o/chat/completions?api-version=2024-06-01 "
            ),
            "unexpected request line: {request_text}"
        );

        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write response headers");
        write_chunk(&mut socket, stalled_azure_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_azure_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

fn stalled_azure_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

fn stalled_azure_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}\n\n",
        "data: [DONE]\n\n",
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn provider_id_is_azure() {
    let provider = make_provider();
    assert_eq!(provider.id(), "azure");
}

#[test]
fn azure_url_construction() {
    let provider = make_provider();
    let url = provider.build_azure_url("my-gpt4o");
    assert_eq!(
        url,
        "https://myresource.openai.azure.com/openai/deployments/my-gpt4o/chat/completions?api-version=2024-06-01"
    );
}

#[test]
fn missing_endpoint_returns_error() {
    let result = AzureOpenAIProvider::new(None, "deploy1".into(), None);
    assert!(result.is_err(), "missing endpoint should return error");
    let err = result.unwrap_err();
    match err {
        opi_ai::provider::ProviderError::Config(msg) => {
            assert!(
                msg.contains("endpoint is required"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn models_from_config_deployments() {
    let provider = make_provider_with_deployments(vec!["my-gpt4o", "my-gpt4o-mini"]);
    let models = provider.models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "my-gpt4o");
    assert_eq!(models[1].id, "my-gpt4o-mini");
}

#[test]
fn selected_deployment_is_advertised_when_catalog_not_configured() {
    let provider = make_provider();
    assert_eq!(provider.models().len(), 1);
    assert_eq!(provider.models()[0].id, "my-gpt4o");
    assert_eq!(
        provider.models()[0].wire_api,
        opi_ai::WireApi::AzureOpenAiCompletions
    );
}

#[tokio::test]
async fn text_streaming_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = provider.stream_from_sse(text_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
    );

    // Check text content
    let text_deltas: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text_deltas.join(""), "Hi there");
}

#[tokio::test]
async fn tool_call_from_fixture() {
    let provider = make_provider();
    let request = tool_request();
    let stream = provider.stream_from_sse(&tool_call_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallDelta { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }));
    assert!(done.is_some());
    if let Some(AssistantStreamEvent::Done { reason, .. }) = done {
        assert_eq!(*reason, opi_ai::stream::StopReason::ToolUse);
    }
}

#[tokio::test]
async fn usage_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = provider.stream_from_sse(text_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }));
    assert!(done.is_some());
    if let Some(AssistantStreamEvent::Done { message, .. }) = done {
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
    }
}

#[tokio::test]
async fn error_from_fixture() {
    let provider = make_provider();
    let request = text_request();
    let stream = provider.stream_from_sse(error_sse_fixture(), request.cancel);
    let events = collect_events(stream).await;

    let error_event = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Error { .. }));
    assert!(error_event.is_some());
    if let Some(AssistantStreamEvent::Error { message, .. }) = error_event {
        let err = message.error_message.as_deref().unwrap_or("");
        assert!(
            err.contains("openai chat stream error"),
            "error_message must be the neutral literal, got: {err}"
        );
        assert!(
            !err.contains("Deployment not found"),
            "raw upstream error text must not leak into the public error_message: {err}"
        );
    }
}

#[test]
fn secret_redaction_in_debug() {
    // Phase 17.5: the api key moved out of AzureOpenAIProvider construction
    // into ResolvedAuth (passed via stream_prepared). The provider no longer
    // stores the credential, so it cannot leak through its Debug output.
    // ResolvedAuth's own Debug redaction is pinned in auth_contracts.rs.
    let provider = make_provider();
    let debug_str = format!("{provider:?}");
    assert!(!debug_str.contains("test-api-key-12345"));
}

#[test]
fn request_body_uses_deployment_name() {
    let provider = make_provider();
    let request = text_request();
    let body = provider.build_request_body(&request);
    assert_eq!(body["model"], "my-gpt4o");
}

// ---------------------------------------------------------------------------
// Production Provider::stream lifecycle through a real HTTP exchange
// ---------------------------------------------------------------------------
//
// The offline `stream_from_sse` tests above exercise the SSE parser/mapper but
// bypass the production HTTP transport (`Provider::stream` -> `stream_azure_http`),
// so they cannot prove the Azure deployment URL, `api-key` auth header, request
// body, or end-to-end draining through the adapter path. These wiremock tests
// cover that contract using local mock HTTP only.

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .and(query_param("api-version", "2024-06-01"))
        .and(header("api-key", "test-api-key-12345"))
        .and(body_partial_json(serde_json::json!({
            "model": "my-gpt4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 256
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();

    // The api-key header is supplied via ResolvedAuth (Phase 17.5); the
    // production transport derives the `api-key` header from `auth.secret`
    // in stream_prepared.
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-api-key-12345"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let events = collect_events(provider.stream_prepared(text_request(), auth)).await;

    // Lifecycle: Start -> TextDelta(s) -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    let text: String = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hi there", "should stream the full text content");
    let done = events
        .last()
        .expect("stream should produce a terminal event");
    match done {
        AssistantStreamEvent::Done { reason, .. } => {
            // OpenAI `stop` finish reason must map to the shared StopReason::Stop.
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // The matchers above (path + api-version query + api-key header + body
    // shape) ARE the request assertion; verify() confirms the production
    // request carried all of them.
    server.verify().await;
}

#[tokio::test]
async fn stream_http_error_maps_to_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"error":{"message":"invalid api key"}}"#),
        )
        .mount(&server)
        .await;

    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();

    let stream = provider.stream_prepared(text_request(), opi_ai::test_support::resolved_auth());
    let first = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .next()
        .expect("should produce an event");
    match first {
        Err(opi_ai::provider::ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("authentication failed"),
                "auth error should mention failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed from HTTP 401, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — Azure inherits the shared compat profile path
// ---------------------------------------------------------------------------

#[test]
fn azure_inherits_shared_compat_flags_via_with_compat() {
    // Azure is a first-class provider that routes request-body serialization
    // through the shared OpenAI Chat adapter. With a developer-role + strict-
    // tools compat applied via with_compat, its request body reflects both
    // flags through the shared serializer (DoD), not an Azure-specific one.
    use opi_ai::openai_chat::CompatConfig;

    let provider = make_provider().with_compat(CompatConfig {
        system_role_override: Some("developer".into()),
        strict_tool_schema: true,
        ..Default::default()
    });
    // text_request carries a system message -> developer role override applies.
    let body_text = provider.build_request_body(&text_request());
    let messages = body_text["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "Azure inherits developer-role override from the shared compat path"
    );
    // tool_request carries tools -> strict flag applies.
    let body_tool = provider.build_request_body(&tool_request());
    let tools = body_tool["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "Azure inherits strict-tool-schema from the shared compat path"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the Azure OpenAI adapter's HTTP
    // body-stream loop (azure_openai.rs `cancel.cancelled()` select arm).
    // Cancelling while the stream is open must terminate it before the delayed
    // terminal chunk arrives, proving the shared OpenAI-compatible adapter path
    // observes cancellation under Azure's deployment URL shape.
    let server = spawn_stalled_azure_server().await;

    let cancel = CancellationToken::new();
    let provider =
        AzureOpenAIProvider::new(Some(server), "my-gpt4o".into(), Some("2024-06-01".into()))
            .unwrap();
    let mut request = text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must close before the delayed terminal fixture completes");
    assert!(
        next.is_none(),
        "stream should end on cancellation before the terminal SSE chunk arrives"
    );
}

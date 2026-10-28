//! Azure OpenAI provider fixture tests (task 3.2).
//!
//! Tests cover: text streaming, tool calls, usage, errors, secret redaction,
//! URL construction, and api-key auth header. All use deterministic SSE
//! fixture data — no live Azure calls.

use futures_util::StreamExt;
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError};
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

fn tool_request() -> opi_ai::provider::Request {
    use opi_ai::message::{InputContent, ToolDef, UserMessage};
    use opi_ai::provider::Request;

    Request {
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
async fn collect_items(
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
enum AzureStallPoint {
    BeforeHeaders,
    ResponseBody,
}

async fn spawn_stalled_azure_server(
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

async fn spawn_stalled_azure_error_body_server(
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
            // Task 20.38: the message follows the pinned source resolver
            // wording (base URL required, config/env positions named).
            assert!(
                msg.contains("base URL is required"),
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
    let events: Vec<Result<AssistantStreamEvent, ProviderError>> = collect_items(stream).await;

    // Current producers stop the dual terminal format: the in-band error
    // event is a stream failure with a bounded typed cause.
    let failure = events.iter().find(|e| {
        e.as_ref()
            .is_err_and(|cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream)
    });
    let Some(Err(cause)) = failure else {
        panic!("expected a typed stream failure, got {events:?}");
    };
    let summary = cause.to_string();
    assert!(
        summary.contains("openai chat stream error"),
        "cause must be the neutral literal, got: {summary}"
    );
    assert!(
        !summary.contains("Deployment not found"),
        "raw upstream error text must not leak into the cause: {summary}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Error { .. })))
    );
}

#[tokio::test]
async fn malformed_frame_does_not_expose_upstream_content() {
    let canary = "azure-malformed-secret-canary";
    let sse = format!("data: {{not-json-{canary}}}\n\n");
    let events = make_provider()
        .stream_from_sse(&sse, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;
    let rendered = format!("{events:?}");
    assert!(events.iter().any(Result::is_err));
    assert!(
        !rendered.contains(canary),
        "malformed Azure frame leaked upstream content: {rendered}"
    );
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
    let body = provider.build_request_body(&request).unwrap();
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
            "max_completion_tokens": 256
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
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "azure-access-canary-with-no-known-token-shape",
        "azure-secret-canary-with-no-known-token-shape",
        "azure-session-canary-with-no-known-token-shape",
        "azure-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/openai/deployments/my-gpt4o/chat/completions"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;

        let provider = AzureOpenAIProvider::new(
            Some(server.uri()),
            "my-gpt4o".into(),
            Some("2024-06-01".into()),
        )
        .unwrap();

        let mut stream =
            provider.stream_prepared(text_request(), opi_ai::test_support::resolved_auth());
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(
            error,
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        let rendered = format!("{error} {error:?}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "Azure HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn request_enrichment_reaches_azure_http_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .and(header("api-key", "test-api-key-12345"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "azure"))
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
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-api-key-12345"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let mut request = text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "azure".into())];

    let events = collect_events(provider.stream_prepared(request, auth)).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_azure_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let mut request = text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Azure request timeout must resolve promptly")
        .expect("Azure timeout must produce a stream item");

    assert!(matches!(
        result,
        Err(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_azure_error_body_server(status).await;
        let provider =
            AzureOpenAIProvider::new(Some(server), "my-gpt4o".into(), Some("2024-06-01".into()))
                .unwrap();
        let mut stream =
            provider.stream_prepared(text_request(), opi_ai::test_support::resolved_auth());

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Azure auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Azure direct auth status must not wait for its body")
            .expect("Azure direct auth status must produce a stream item");

        let error = result.expect_err("Azure direct auth status must fail");
        assert!(matches!(
            error,
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        assert_eq!(
            error.to_string(),
            "authentication failed: provider rejected credentials"
        );
    }
}

#[tokio::test]
async fn request_header_cannot_override_azure_auth_routing() {
    let server = MockServer::start().await;
    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let mut request = text_request();
    request.extra_headers = vec![("api-key".into(), "override".into())];
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    assert!(matches!(
        stream.next().await,
        Some(Err(opi_ai::provider::ProviderError::RequestFailed(_)))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Azure request headers must fail before dispatch"
    );
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
        instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
        strict_tool_schema: true,
        ..Default::default()
    });
    // text_request carries a system message -> developer role override applies.
    let body_text = provider.build_request_body(&text_request()).unwrap();
    let messages = body_text["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "Azure inherits developer-role override from the shared compat path"
    );
    // tool_request carries tools -> strict flag applies.
    let body_tool = provider.build_request_body(&tool_request()).unwrap();
    let tools = body_tool["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "Azure inherits strict-tool-schema from the shared compat path"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

async fn assert_azure_cancelled(stall_point: AzureStallPoint) {
    let (server, stalled) = spawn_stalled_azure_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider =
        AzureOpenAIProvider::new(Some(server), "my-gpt4o".into(), Some("2024-06-01".into()))
            .unwrap();
    let mut request = text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Azure server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Azure cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Azure cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_azure_cancelled(AzureStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_azure_cancelled(AzureStallPoint::ResponseBody).await;
}

// ---------------------------------------------------------------------------
// Task 20.51: callbacks flow through the shared Responses wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn azure_async_callbacks_flow_through_shared_responses_wire() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = AzureOpenAIProvider::new(
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-api-key-12345"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let payload_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let payload_counter = payload_calls.clone();
    let mut request = text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(move |_body| {
            let payload_counter = payload_counter.clone();
            async move {
                tokio::task::yield_now().await;
                payload_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(None)
            }
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    let events = collect_events(provider.stream_prepared(request, auth)).await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    assert_eq!(payload_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let order = order.lock().unwrap().clone();
    assert_eq!(order.first().map(String::as_str), Some("response:200"));
    assert!(
        order.len() > 1,
        "raw wire events follow the response callback"
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Task 20.38: azure-openai-responses dual-wire adapter, deployment/resource/
// apiVersion mapping and the Azure native option inventory (no service tier).
// ---------------------------------------------------------------------------

use opi_ai::azure_openai::{
    AzureOpenAIOptions, AzureOpenAIResponsesChatApi, resolve_azure_api_version,
    resolve_azure_base_url, resolve_deployment_name,
};
use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};
use opi_ai::openai_responses::{ResponsesReasoningSummary, ResponsesToolChoice};

fn azure_responses_record(id: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        WireApi::AzureOpenAiResponses,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

fn azure_responses_provider(endpoint: String) -> std::sync::Arc<AzureOpenAIProvider> {
    std::sync::Arc::new(
        AzureOpenAIProvider::for_records(
            Some(endpoint),
            vec![azure_responses_record("my-gpt4o")],
            None,
        )
        .unwrap(),
    )
}

fn azure_responses_request(
    model: &str,
    thinking: Option<ThinkingLevel>,
) -> opi_ai::provider::Request {
    let mut request = text_request();
    request.model = format!("azure:{model}");
    request.thinking = match thinking {
        Some(level) => opi_ai::provider::ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        },
        None => Default::default(),
    };
    request
}

fn azure_responses_sse_text() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_az\",\"status\":\"in_progress\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_az\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}}\n\n",
    )
    .to_owned()
}

fn azure_responses_sse_tool() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_az\",\"status\":\"in_progress\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"\"}}\n\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"call_id\":\"call_1\",\"delta\":\"{\\\"city\\\":\\\"London\\\"}\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"London\\\"}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_az\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"London\\\"}\"}],\"usage\":{\"input_tokens\":20,\"output_tokens\":10}}}\n\n",
    )
    .to_owned()
}

fn azure_api_key_auth() -> opi_ai::auth::ResolvedAuth {
    opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("offline-azure-key"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    }
}

async fn mount_azure_responses_mock(server: &wiremock::MockServer, body: String) {
    use wiremock::matchers::{method, path, query_param};
    wiremock::Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/responses"))
        .and(query_param("api-version", "v1"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(server)
        .await;
}

async fn sent_body(server: &wiremock::MockServer) -> serde_json::Value {
    serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap()
}

#[test]
fn azure_endpoint_resolution_prefers_explicit_then_resource_then_model_base() {
    assert_eq!(
        resolve_azure_base_url(
            Some(" https://custom.example.com/ "),
            Some("myresource"),
            Some("https://model.example.com"),
        )
        .unwrap(),
        "https://custom.example.com"
    );
    assert_eq!(
        resolve_azure_base_url(None, Some("myresource"), None).unwrap(),
        "https://myresource.openai.azure.com"
    );
    assert_eq!(
        resolve_azure_base_url(None, None, Some("https://model.example.com/")).unwrap(),
        "https://model.example.com"
    );
    assert!(
        matches!(
            resolve_azure_base_url(None, None, None),
            Err(opi_ai::provider::ProviderError::Config(_))
        ),
        "missing endpoint resolves to a typed failure"
    );
}

#[test]
fn azure_endpoint_normalizes_azure_host_paths_to_resource_root() {
    for base in [
        "https://myresource.openai.azure.com/openai",
        "https://myresource.openai.azure.com/openai/v1",
        "https://myresource.openai.azure.com/openai/v1/responses",
        "https://myresource.cognitiveservices.azure.com/openai/v1/",
    ] {
        let expected = if base.contains("cognitiveservices") {
            "https://myresource.cognitiveservices.azure.com"
        } else {
            "https://myresource.openai.azure.com"
        };
        assert_eq!(
            resolve_azure_base_url(Some(base), None, None).unwrap(),
            expected,
            "azure host base {base} normalizes to the resource root"
        );
    }
    assert_eq!(
        resolve_azure_base_url(Some("https://gateway.example.com/azure"), None, None).unwrap(),
        "https://gateway.example.com/azure",
        "non-azure hosts keep their configured path"
    );
}

#[test]
fn azure_api_version_defaults_to_v1_and_accepts_override() {
    assert_eq!(resolve_azure_api_version(None), "v1");
    assert_eq!(resolve_azure_api_version(Some("2024-10-21")), "2024-10-21");
}

#[test]
fn azure_deployment_resolution_explicit_map_then_model_id() {
    assert_eq!(
        resolve_deployment_name(Some("prod-deploy"), Some("o3=o3-prod"), "o3"),
        "prod-deploy"
    );
    assert_eq!(
        resolve_deployment_name(None, Some("o3=o3-prod, gpt4o=gpt4o-prod"), "gpt4o"),
        "gpt4o-prod"
    );
    assert_eq!(
        resolve_deployment_name(None, Some("o3=o3-prod"), "o4-mini"),
        "o4-mini"
    );
    assert_eq!(resolve_deployment_name(None, None, "o3"), "o3");
}

#[test]
fn azure_responses_url_construction() {
    let provider = AzureOpenAIProvider::from_config(
        Some("https://myresource.openai.azure.com".into()),
        vec!["my-gpt4o".into()],
        Some("2024-10-21".into()),
    )
    .unwrap();
    assert_eq!(
        provider.build_azure_responses_url("my-gpt4o"),
        "https://myresource.openai.azure.com/openai/deployments/my-gpt4o/responses?api-version=2024-10-21"
    );
    // Default version follows the pinned source default v1.
    let provider = azure_responses_provider("https://myresource.openai.azure.com".into());
    assert!(
        provider
            .build_azure_responses_url("my-gpt4o")
            .ends_with("/openai/deployments/my-gpt4o/responses?api-version=v1")
    );
}

#[tokio::test]
async fn azure_responses_text_lifecycle_uses_deployment_url_and_api_key_auth() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(server.uri());
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    let events = collect_azure_events(&mut stream).await;

    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("terminal Done");
    assert_eq!(done.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(done.usage.input_tokens, 7);
    assert_eq!(done.usage.output_tokens, 3);
    assert!(
        done.duration_ms.is_some(),
        "terminal Done carries the measured duration"
    );

    let sent = &server.received_requests().await.unwrap()[0];
    assert!(
        sent.headers.get("api-key").is_some(),
        "azure responses uses the api-key header"
    );
    assert!(sent.headers.get("authorization").is_none());
    let body = serde_json::from_slice::<serde_json::Value>(&sent.body).unwrap();
    assert_eq!(body["model"], "my-gpt4o", "deployment name rides the body");
    assert_eq!(body["store"], false, "source semantics: store false");
    assert_eq!(body["stream"], true);
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_tool_call_lifecycle_reaches_the_wire() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_tool()).await;

    let provider = azure_responses_provider(server.uri());
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![opi_ai::message::Message::User(
            opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "What is the weather?".into(),
                }],
                timestamp_ms: 0,
            },
        )],
        tools: vec![opi_ai::message::ToolDef {
            name: "get_weather".into(),
            description: "Get weather".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"city": {"type": "string"}}
            }),
            constrained_sampling: None,
        }],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    let events = collect_azure_events(&mut stream).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::ToolCallStart { .. })),
        "tool call start observed: {events:?}"
    );
    let body = sent_body(&server).await;
    assert_eq!(
        body["tools"][0]["name"], "get_weather",
        "declared tools reach the azure responses body"
    );
    server.verify().await;
}

async fn collect_azure_events(
    stream: &mut opi_ai::provider::EventStream,
) -> Vec<AssistantStreamEvent> {
    use futures_util::StreamExt;
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                let terminal = event.is_terminal();
                events.push(event);
                if terminal {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

async fn registered_azure_responses_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    opi_ai::models::Models,
    opi_ai::models::ChatApiHandle<AzureOpenAIOptions>,
) {
    let provider = azure_responses_provider(server_uri);
    let api = AzureOpenAIResponsesChatApi::new(provider, model).expect("valid azure record");
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("offline-azure-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register azure responses api");
    (models, handle)
}

#[tokio::test]
async fn azure_responses_native_options_reach_the_wire_without_service_tier() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::High);
    value.reasoning_summary = Some(ResponsesReasoningSummary::Detailed);
    value.tool_choice = Some(ResponsesToolChoice::Required);
    let options = handle.options(value);

    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect("native azure responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "detailed"})
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(body["tool_choice"], "required");
    assert!(
        body.get("service_tier").is_none(),
        "azure responses omits service_tier (source omission; no tier multiplier)"
    );
    assert_eq!(body["model"], "my-gpt4o");
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_simple_entry_maps_neutral_thinking_and_clamps() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let models = {
        let (models, _handle) =
            registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o"))
                .await;
        models
    };
    let message = models
        .complete_simple(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", Some(ThinkingLevel::High)),
            opi_ai::models::SimpleOptions::default(),
        )
        .await
        .expect("simple azure responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"]["effort"], "high",
        "simple entry carries the neutral thinking intent"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_disabled_thinking_sends_off_projection() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(server.uri());
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "none"}),
        "reasoning-capable azure deployments send the off mapping"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_endpoint_overrides_in_native_options_reach_the_url() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.azure_base_url = Some(server.uri());
    value.azure_api_version = Some("v1".into());
    value.azure_deployment_name = Some("my-gpt4o".into());
    let options = handle.options(value);

    models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect("endpoint overrides reach the transport");
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_setup_failure_before_activation_is_typed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("denied"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = azure_responses_provider(server.uri());
    let mut stream = provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    );
    let items = collect_azure_items(&mut stream).await;
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Err(ProviderError::AuthFailed(_)))),
        "401 maps to typed AuthFailed: {items:?}"
    );
    server.verify().await;
}

async fn collect_azure_items(
    stream: &mut opi_ai::provider::EventStream,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    use futures_util::StreamExt;
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        let terminal = match &item {
            Ok(event) => event.is_terminal(),
            Err(_) => true,
        };
        items.push(item);
        if terminal {
            break;
        }
    }
    items
}

#[tokio::test]
async fn azure_responses_cancellation_during_body_is_typed_and_prompt() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let text = String::from_utf8_lossy(&request);
        assert!(
            text.starts_with("POST /openai/deployments/my-gpt4o/responses?api-version=v1 "),
            "unexpected request line: {text}"
        );
        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush");
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    let provider = azure_responses_provider(format!("http://{addr}"));
    let request = azure_responses_request("my-gpt4o", None);
    let cancel = request.cancel.clone();
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    stalled.notified().await;
    cancel.cancel();
    let items = collect_azure_items(&mut stream).await;
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Err(ProviderError::Cancelled))),
        "cancellation surfaces typed Cancelled: {items:?}"
    );
}

fn azure_history_assistant(
    content: Vec<opi_ai::message::AssistantContent>,
    wire: Option<WireApi>,
    provider: &str,
) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: provider.into(),
        model: "my-gpt4o".into(),
        response_model: None,
        wire,
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
    }
}

#[tokio::test]
async fn azure_responses_replays_signed_reasoning_for_same_identity_turns() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(server.uri());
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            }),
            opi_ai::message::Message::Assistant(azure_history_assistant(
                vec![
                    opi_ai::message::AssistantContent::Thinking {
                        thinking: "let me think".into(),
                        signature: Some(
                            serde_json::json!({
                                "type": "reasoning",
                                "id": "rs_1",
                                "encrypted_content": "ENC",
                            })
                            .to_string(),
                        ),
                    },
                    opi_ai::message::AssistantContent::Text {
                        text: "Earlier".into(),
                        signature: None,
                    },
                ],
                Some(WireApi::AzureOpenAiResponses),
                "azure",
            )),
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    let input = body["input"].as_array().expect("input list");
    assert!(
        input.iter().any(
            |item| item.get("type") == Some(&serde_json::json!("reasoning"))
                && item.get("encrypted_content") == Some(&serde_json::json!("ENC"))
        ),
        "same-identity signed reasoning replays into input: {input:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_foreign_identity_history_never_replays_reasoning() {
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(&server, azure_responses_sse_text()).await;

    let provider = azure_responses_provider(server.uri());
    let mut request = azure_responses_request("my-gpt4o", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            }),
            opi_ai::message::Message::Assistant(azure_history_assistant(
                vec![
                    opi_ai::message::AssistantContent::Thinking {
                        thinking: "foreign thought".into(),
                        signature: Some(
                            serde_json::json!({
                                "type": "reasoning",
                                "id": "rs_x",
                                "encrypted_content": "FOREIGN",
                            })
                            .to_string(),
                        ),
                    },
                    opi_ai::message::AssistantContent::Text {
                        text: "Earlier".into(),
                        signature: None,
                    },
                ],
                Some(WireApi::OpenAiResponses),
                "openai-responses",
            )),
            opi_ai::message::Message::User(opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let mut stream = provider.stream_prepared(request, azure_api_key_auth());
    collect_azure_events(&mut stream).await;

    let body = sent_body(&server).await;
    let input = body["input"].as_array().expect("input list");
    assert!(
        !input
            .iter()
            .any(|item| item.get("type") == Some(&serde_json::json!("reasoning"))),
        "foreign-identity reasoning never replays: {input:?}"
    );
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_direct_adapter_rejects_wrong_wire_records() {
    let provider = azure_responses_provider("https://unused.example.com".into());

    let chat_record = {
        let mut record = azure_responses_record("my-gpt4o");
        record.wire_api = WireApi::AzureOpenAiCompletions;
        record
    };
    let failure = AzureOpenAIResponsesChatApi::new(provider.clone(), chat_record)
        .expect_err("chat-wire records fail before authentication");
    assert!(
        matches!(failure, ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );

    let openai_record = {
        let mut record = azure_responses_record("my-gpt4o");
        record.wire_api = WireApi::OpenAiResponses;
        record
    };
    let failure = AzureOpenAIResponsesChatApi::new(provider, openai_record)
        .expect_err("plain openai-responses records fail before authentication");
    assert!(
        matches!(failure, ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );
}

#[tokio::test]
async fn azure_responses_native_unsupported_effort_fails_typed_before_auth() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_string(azure_responses_sse_text())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let (models, handle) =
        registered_azure_responses_models(server.uri(), azure_responses_record("my-gpt4o")).await;
    let mut value = AzureOpenAIOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", azure_responses_record("my-gpt4o")),
            azure_responses_request("my-gpt4o", None),
            options,
        )
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

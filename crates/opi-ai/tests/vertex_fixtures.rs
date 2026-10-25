//! Google Vertex AI provider fixture tests (task 3.3).
//!
//! Verifies: Vertex-specific URL formatting, OAuth token auth, secret redaction,
//! SSE event mapping (reuses Gemini wire format), model resolution, and
//! request body construction. No live Google Cloud calls.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::vertex::VertexProvider;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn vertex_provider() -> VertexProvider {
    VertexProvider::new("my-project".into(), "us-central1".into(), None)
}

async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn vertex_provider_id_is_vertex() {
    let provider = vertex_provider();
    assert_eq!(provider.id(), "vertex");
}

// ---------------------------------------------------------------------------
// URL construction
// ---------------------------------------------------------------------------

#[test]
fn vertex_url_contains_project_and_location() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.contains("my-project"),
        "URL should contain project: {url}"
    );
    assert!(
        url.contains("us-central1"),
        "URL should contain location: {url}"
    );
    assert!(
        url.contains("publishers/google/models/gemini-2.5-flash"),
        "URL should contain model in path: {url}"
    );
    assert!(
        url.contains("streamGenerateContent"),
        "URL should contain streamGenerateContent: {url}"
    );
}

#[test]
fn vertex_url_has_alt_sse_param() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.contains("alt=sse"),
        "URL should have alt=sse query param: {url}"
    );
}

#[test]
fn vertex_url_uses_aiplatform_domain() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.starts_with("https://us-central1-aiplatform.googleapis.com"),
        "URL should use Vertex AI domain: {url}"
    );
}

#[test]
fn vertex_url_with_custom_base() {
    let provider = VertexProvider::new(
        "proj".into(),
        "europe-west1".into(),
        Some("https://custom.vertex.proxy".into()),
    );
    let url = provider.build_vertex_url("gemini-2.5-pro");
    assert!(
        url.contains("europe-west1"),
        "custom base should still inject location: {url}"
    );
}

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

#[test]
fn vertex_access_token_not_in_debug() {
    // Phase 17.5: the access token moved out of VertexProvider construction
    // into ResolvedAuth (passed via stream_prepared). The provider no longer
    // stores the credential, so it cannot leak through its Debug output.
    // ResolvedAuth's own Debug redaction is pinned in auth_contracts.rs.
    let provider = VertexProvider::new("proj".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(
        !debug.contains("super-secret-oauth-token-12345"),
        "access token leaked in Debug: {debug}"
    );
}

#[test]
fn vertex_project_visible_in_debug() {
    let provider = vertex_provider();
    let debug = format!("{provider:?}");
    assert!(debug.contains("my-project"));
    assert!(debug.contains("us-central1"));
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[test]
fn vertex_default_models() {
    let provider = vertex_provider();
    let models = provider.models();
    assert!(!models.is_empty(), "Vertex should have default model list");
    assert!(
        models.iter().any(|m| m.id == "gemini-2.5-flash"),
        "should include gemini-2.5-flash"
    );
}

#[test]
fn vertex_custom_models_from_config() {
    let provider = VertexProvider::from_config(
        "proj".into(),
        "europe-west4".into(),
        vec!["my-custom-model".into(), "other-model".into()],
        None,
    );
    let models = provider.models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "my-custom-model");
    assert_eq!(models[1].id, "other-model");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn vertex_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(vertex_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("vertex:gemini-2.5-flash").unwrap();
    assert_eq!(provider.id(), "vertex");
    assert_eq!(model.id, "gemini-2.5-flash");
}

// ---------------------------------------------------------------------------
// SSE text streaming (reuses Gemini wire format)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_text_streaming_produces_start_delta_done() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Start { .. }))
        .count();
    let deltas = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .count();
    let dones = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .count();

    assert_eq!(starts, 1, "should have exactly one Start");
    assert_eq!(deltas, 1, "should have exactly one TextDelta");
    assert_eq!(dones, 1, "should have exactly one Done");
}

#[tokio::test]
async fn vertex_malformed_frame_does_not_expose_upstream_content() {
    let canary = "vertex-malformed-secret-canary";
    let sse = format!("data: {{not-json-{canary}}}\n\n");
    let events = vertex_provider()
        .stream_from_sse(&sse, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;
    let rendered = format!("{events:?}");
    assert!(events.iter().any(Result::is_err));
    assert!(
        !rendered.contains(canary),
        "malformed Vertex frame leaked upstream content: {rendered}"
    );
}

#[tokio::test]
async fn vertex_done_event_has_vertex_provider() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let done_provider = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done_provider, "vertex");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_tool_call_streaming_works() {
    let provider = vertex_provider();
    let sse = tool_call_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_starts, 1, "should have one ToolCallStart");
    assert_eq!(tool_ends, 1, "should have one ToolCallEnd");
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_error_event_routing() {
    let provider = vertex_provider();
    let sse = "data: {\"error\":{\"code\":404,\"message\":\"Model not found\",\"status\":\"NOT_FOUND\"}}\n\n";

    let items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;

    // Current producers stop the dual terminal format: the error event routes
    // to a typed stream failure, not an Error event.
    assert!(
        items.iter().any(|e| e.as_ref().is_err_and(
            |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
        )),
        "should have a typed stream failure"
    );
    assert!(
        !items
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Error { .. })))
    );
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_usage_in_done_event() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let usage = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(usage.input_tokens, 42);
    assert_eq!(usage.output_tokens, 13);
}

// ---------------------------------------------------------------------------
// Request body construction (delegates to Gemini format)
// ---------------------------------------------------------------------------

#[test]
fn vertex_request_body_uses_gemini_format() {
    let provider = vertex_provider();
    let request = Request {
        model: "vertex:gemini-2.5-flash".into(),
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
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("contents").is_some(),
        "should use Gemini contents format"
    );
    assert!(
        body.get("systemInstruction").is_some(),
        "should have systemInstruction"
    );
    assert!(
        body.get("model").is_none(),
        "model should NOT be in body (goes in URL)"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn text_sse_fixture() -> String {
    concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n",
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":42,\"candidatesTokenCount\":13,\"totalTokenCount\":55}}\n\n",
    ).into()
}

fn tool_call_sse_fixture() -> String {
    let data = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "read_file",
                        "args": {"path": "foo.rs"}
                    }
                }]
            },
            "index": 0
        }],
        "usageMetadata": {
            "promptTokenCount": 20,
            "candidatesTokenCount": 10,
            "totalTokenCount": 30
        }
    });
    format!("data: {data}\n\n")
}

// ---------------------------------------------------------------------------
// Production Provider::stream lifecycle through a real HTTP exchange
// ---------------------------------------------------------------------------
//
// The offline `stream_from_sse` tests above exercise the Gemini SSE parser and
// mapper but bypass the production HTTP transport (`Provider::stream` ->
// `stream_vertex_http`), so they cannot prove the Vertex AI URL, OAuth bearer
// auth header, Gemini request body, or end-to-end draining through the adapter
// path. These wiremock tests cover that contract using local mock HTTP only.

fn lifecycle_text_request() -> Request {
    Request {
        model: "vertex:gemini-2.5-flash".into(),
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
    }
}

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .and(header("authorization", "Bearer test-access-token"))
        .and(body_partial_json(serde_json::json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "systemInstruction": {"parts": [{"text": "You are helpful."}]},
            "generationConfig": {"maxOutputTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );

    // The OAuth bearer token is supplied via ResolvedAuth (Phase 17.5); the
    // production transport derives `Authorization: Bearer <secret>` from
    // `auth.secret` in stream_prepared.
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(lifecycle_text_request(), auth);
    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        let event = result.expect("matched Vertex request must stream successfully");
        assert!(!matches!(event, AssistantStreamEvent::Error { .. }));
        events.push(event);
    }

    // Lifecycle: Start -> TextDelta -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello"
        )),
        "should emit TextDelta carrying the streamed text"
    );
    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("stream should produce a terminal Done event");
    match done {
        AssistantStreamEvent::Done { reason, message } => {
            // Gemini `STOP` finish reason must map to the shared StopReason::Stop.
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            assert_eq!(message.provider, "vertex");
            assert_eq!(
                message.usage,
                opi_ai::stream::Usage::reported(42, 13, 0, 0, None, None)
            );
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // verify() confirms the OAuth bearer header, alt=sse query, and Gemini
    // body all matched the production request. Independently assert the Vertex
    // URL path carries project/location/model/resource so the structural path
    // is pinned regardless of colon encoding in `:streamGenerateContent`.
    server.verify().await;
    let received = server
        .received_requests()
        .await
        .expect("should have recorded the provider request");
    assert_eq!(received.len(), 1);
    let url = received[0].url.as_str();
    assert!(url.contains("projects/my-project"), "url: {url}");
    assert!(url.contains("locations/us-central1"), "url: {url}");
    assert!(
        url.contains("publishers/google/models/gemini-2.5-flash"),
        "url: {url}"
    );
    assert!(url.contains("streamGenerateContent"), "url: {url}");
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "vertex-access-canary-with-no-known-token-shape",
        "vertex-secret-canary-with-no-known-token-shape",
        "vertex-session-canary-with-no-known-token-shape",
        "vertex-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(query_param("alt", "sse"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;

        let provider = VertexProvider::new(
            "my-project".into(),
            "us-central1".into(),
            Some(server.uri()),
        );

        let mut stream = provider.stream_prepared(
            lifecycle_text_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        );
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
                "Vertex HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_vertex_error_body_server(status).await;
        let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
        let mut stream = provider.stream_prepared(
            lifecycle_text_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        );

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Vertex auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Vertex direct auth status must not wait for its body")
            .expect("Vertex direct auth status must produce a stream item");

        let error = result.expect_err("Vertex direct auth status must fail");
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
async fn oversized_embedded_auth_body_is_not_read_past_the_classification_cap() {
    let server = MockServer::start().await;
    let body = format!(
        r#"{{"error":{{"code":401}},"padding":"{}"}}"#,
        "x".repeat(64 * 1024)
    );
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .respond_with(ResponseTemplate::new(400).set_body_string(body))
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut stream = provider.stream_prepared(
        lifecycle_text_request(),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let error = stream
        .next()
        .await
        .expect("oversized Vertex error body must produce a stream item")
        .expect_err("oversized Vertex error body must fail");

    assert!(matches!(
        error,
        opi_ai::provider::ProviderError::ProviderSide(_)
    ));
}

#[tokio::test]
async fn request_enrichment_reaches_vertex_http_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer test-access-token"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "vertex"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "vertex".into())];

    let events = collect_stream(provider.stream_prepared(request, auth)).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_vertex_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Vertex request timeout must resolve promptly")
        .expect("Vertex timeout must produce a stream item");

    assert!(matches!(
        result,
        Err(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn stalled_embedded_auth_body_respects_a_stricter_request_timeout() {
    let (server, headers_flushed) = spawn_stalled_vertex_error_body_server(400).await;
    let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(50));
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        headers_flushed.notified(),
    )
    .await
    .expect("Vertex error headers must be flushed before the body stalls");

    let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
        .await
        .expect("Vertex error-body timeout must resolve promptly")
        .expect("Vertex error-body timeout must produce a stream item");

    assert!(matches!(
        result,
        Err(opi_ai::provider::ProviderError::Timeout)
    ));
}

async fn spawn_stalled_vertex_error_body_server(
    status: u16,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Vertex error server");
    let addr = listener.local_addr().expect("stalled error server addr");
    let headers_flushed = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_headers_flushed = headers_flushed.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Vertex request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Vertex request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let reason = match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            _ => "Error",
        };
        let headers = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: 32\r\nConnection: close\r\n\r\n"
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, headers.as_bytes())
            .await
            .expect("write Vertex error headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush Vertex error headers");
        server_headers_flushed.notify_one();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    (format!("http://{addr}"), headers_flushed)
}

#[derive(Clone, Copy)]
enum VertexStallPoint {
    BeforeHeaders,
    ResponseBody,
    ErrorBody,
}

async fn spawn_stalled_vertex_server(
    stall_point: VertexStallPoint,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Vertex server");
    let addr = listener.local_addr().expect("stalled Vertex server addr");
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Vertex request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Vertex request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if !matches!(stall_point, VertexStallPoint::BeforeHeaders) {
            let status = if matches!(stall_point, VertexStallPoint::ErrorBody) {
                "400 Bad Request"
            } else {
                "200 OK"
            };
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .expect("write Vertex response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Vertex response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

#[tokio::test]
async fn request_header_cannot_override_vertex_auth_routing() {
    let server = MockServer::start().await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("authorization".into(), "override".into())];
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

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
        "reserved Vertex request headers must fail before dispatch"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

async fn assert_vertex_cancelled(stall_point: VertexStallPoint) {
    let (server, stalled) = spawn_stalled_vertex_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
    let mut request = lifecycle_text_request();
    request.cancel = cancel.clone();
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Vertex server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Vertex cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Vertex cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::ResponseBody).await;
}

#[tokio::test]
async fn cancellation_during_error_body_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::ErrorBody).await;
}

// ---------------------------------------------------------------------------
// ADC native sources through the real Vertex adapter (task 20.21.1)
// ---------------------------------------------------------------------------

mod adc_vertex {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport};
    use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError, AuthResolver};
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::stream::AssistantStreamEvent;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Non-secret synthetic admission key (RDF-23 sealed corpus; copy of the
    /// `pub(crate)` crypto fixture so the integration test can sign through
    /// the public owner).
    const SERVICE_ACCOUNT_PEM: &str = r###"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgvXz9l8JQ6kC1
E5/ZBuDBqumYD03J6pq9twlaSZ4cDCSOhadPVn8y9yayRNxEDZ25eWObVr5SrQvJ
QnlnKmN4bIXKThvkFxzGWTX96ESZbBBg2Fbv3EjkaRaE+H8WBUtc/Hfn5av8CfKj
x9GOwSzW3VkBPW4LhBSCbXLB07f4hgQ6tMyTaejx5nNBzYtD8SsTW9LnpW4GE2GX
z+U1axSU8WFLfgUzVIXZMSrHY0rwOSwCcSicgbNGd8uIEIv0juOyakaRrPlIk5Ce
s+BYC2dT/SEDJd7VqH98qAw9KxOMmGw9rD7odHafXIB8VmQsTsm53N0KdrsufhJO
32YWIsWFAgMBAAECggEACR6ES+Z99kq6mztO3zMXcbgBUtM9ccuO9gfqGSv3iHZS
OvQPXajV2b2b/ahP9HFu+xHitNz7OmQJuOQ9+ycfEmWFF/ON/Ab+ptcdVndvWuKU
BvaHGh+90pg5WEJj2As6rxQTYRM5NQ5DFzJz1lzpxGfgOo8yYidfECuRmOMAJd0s
3k8FZ/y6nshQgfQ0Lw96aFGfCu8Ycu+3K7p8mh+wH1Bq5oVeXgExlcdO6DLXjtQO
vq0H52mBEHKTG7uiJRadPwoYL8gNhyMhg8hDMUX0w6cUIUchJkLygEaa6NElisMZ
jnSo+gcvHDl2LGQrknineuwgJT12SQ1mtnhltp//eQKBgQDQR5ssNInh/sMOxD8o
ZbuT1iRYMQYeNZqLZeJjstMtQHy3Ul1NH2REImStR1Y5dJRO3d1vRiGIZZx++SEp
44QXdfKin0IfW9w89+2KrZrxfk6eraWHPbYX6FRCob2cfm84RgI+P9YQ7jNLoiix
264MjWJ59Qoyl+ZxTFKMh6637QKBgQDFkYJQSeOApt2OUqtbozJ6NuiyzqpTJVDd
tx4/gGGwKtNm5c1FhYBF6n3WEq261l3drvDC6ucnoN2f7eItcS7Owv5Vylzx1oAM
8KPr59WM96phpnugAWBrOYa4UyJYUxn1eVyjx9l986IeaSIXdhXa32bVfd220Ouu
CC7Lv/1g+QKBgQCugA/mDW6oRcYvDQTNV868oeF5cjlp/gcpW4c9eImtlCXpgbxh
7TcYTqiC9Q2svZu0J/ZqUBuwJ1Hin9/FNLVq4osS7NN9an0vJ5mBYso6IlTEZUAV
+wIhLanK2a/++X89aopelSVIsgmQCKCHw0IjiiFDhKWylcmgiw1uaA9eUQKBgAaE
gFoJtfmeh5yUWZbSUW36tb6AVtobVwmZauQJZi+bmhHA7v6mIs8rpy8Suqx5OeuA
Q+0KNsn14pnODTIWk8a6ZA2MeXh47QyAO8gig8b8vKAwD8/9e6LN6QXXqEBlrWuN
r2Kvyxr+mJCMQ1WmzZ3E1O2iE1hemAfLuU97a+5xAoGBAKoGwyrOChBroJmIcMc7
RYpC88yq0FXkN4rXD1o/3n0CUzeeb1pad2wlt0efNCWRU5E2dM13aBxdQvg0kGNq
f1JdzC/2b2W3wiseeVHZn7MSsny0JpfbveYtQ9xbrmmRY2amtuMzS/jbnrIKQ+wv
7VCl8xqVmjOIpkF7O/B26nBb
-----END PRIVATE KEY-----"###;

    fn service_account_json() -> String {
        let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
        format!(
            r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}"}}"#
        )
    }

    fn authorized_user_json() -> String {
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","quota_project_id":"quota-proj"}"#.to_owned()
    }

    #[derive(Default)]
    struct MapContext {
        env: HashMap<String, String>,
    }

    impl MapContext {
        fn with_env(mut self, name: &str, value: &str) -> Self {
            self.env.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    /// Public-path resolver: the ADC owner resolves, provenance carries the
    /// selected source identity, and the Vertex adapter consumes the Bearer
    /// secret at its wire boundary.
    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(opi_ai::auth::adc::native::SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint),
        )
    }

    /// Models with the mock-backed provider as the streaming route and a
    /// default-catalog instance for listing. The route provider is the one
    /// that streams; it must carry the mock base URL.
    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    async fn drain_to_done(models: &Models) -> AssistantStreamEvent {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        let mut done = None;
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(event, AssistantStreamEvent::Done { .. }) {
                done = Some(event);
                break;
            }
        }
        done.expect("stream reaches Done")
    }

    #[tokio::test]
    async fn express_api_key_uses_key_header_and_projectless_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .and(header("x-goog-api-key", "express-key"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        // The static API-key resolver drives the Express route.
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(VertexProvider::express(Some(server.uri()))),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("express-key"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn adc_authorized_user_end_to_end_bearer_and_quota_header() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("adc.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("client_id=cid"))
            .and(body_string_contains("client_secret=csec"))
            .and(body_string_contains("refresh_token=rt1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"vtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer vtok"))
            .and(header("x-goog-user-project", "quota-proj"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", server.uri())),
                ),
            },
        );

        // Source identity is captured on the public resolution path.
        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::AuthorizedUser {
                    quota_project: Some("quota-proj".to_owned())
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "vtok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn adc_source_failure_fails_before_generation() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("bad.json");
        std::fs::write(&creds, r#"{"type":"authorized_user"}"#).unwrap();

        let token_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_request"}"#),
            )
            .mount(&token_server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", token_server.uri())),
                ),
            },
        );
        let error = models
            .get_auth("vertex", Default::default())
            .await
            .expect_err("a selected source error fails the resolution");
        assert!(matches!(
            error,
            opi_ai::models::ModelsError::AuthResolution { .. }
        ));
        // Fail before generation: zero requests to the model endpoint.
        let requests = generation.received_requests().await.unwrap();
        assert!(requests.is_empty(), "no generation send may happen");
    }

    #[tokio::test]
    async fn adc_service_account_loopback_exchange_shape_and_single_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("sa.json");
        std::fs::write(&creds, service_account_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"satok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer satok"))
            .respond_with(sse_ok())
            .expect(2)
            .mount(&server)
            .await;

        let endpoint = format!("{}/token", server.uri());
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(endpoint.clone()),
                ),
            },
        );

        drain_to_done(&models).await;
        drain_to_done(&models).await;

        // The JWT assertion: canonical grant type, iss/aud from the file and
        // the actually used endpoint, exp = iat + 3600.
        let requests = server.received_requests().await.unwrap();
        let token_post = requests
            .iter()
            .find(|r| r.url.path() == "/token")
            .expect("token exchange observed");
        let body = String::from_utf8(token_post.body.clone()).unwrap();
        let assertion = body
            .strip_prefix(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=",
            )
            .expect("jwt-bearer form");
        let parts: Vec<&str> = assertion.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header = b64url_decode(parts[0]);
        assert_eq!(header, b"{\"alg\":\"RS256\"}");
        let claims: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
        assert_eq!(claims["iss"], "sa@proj.iam.gserviceaccount.com");
        assert_eq!(claims["aud"], endpoint);
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            3600
        );

        // One exchange serves both logical calls within the eager window.
        let token_posts = requests.iter().filter(|r| r.url.path() == "/token").count();
        assert_eq!(token_posts, 1);
    }

    #[tokio::test]
    async fn token_503_then_success_retries_with_the_first_fixed_delay() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("creds.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"rt","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .respond_with(sse_ok())
            .mount(&server)
            .await;

        let started = Instant::now();
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", server.uri())),
                ),
            },
        );
        drain_to_done(&models).await;
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(100),
            "the first retry waits the fixed 100ms gaxios delay"
        );
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.iter().filter(|r| r.url.path() == "/token").count(),
            2
        );
    }

    #[tokio::test]
    async fn token_redirect_is_terminal_and_generation_is_never_contacted() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("creds.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let token_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/elsewhere"))
            .mount(&token_server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", token_server.uri())),
                ),
            },
        );
        assert!(
            models.get_auth("vertex", Default::default()).await.is_err(),
            "a redirect on a token exchange is terminal"
        );
        let requests = generation.received_requests().await.unwrap();
        assert!(requests.is_empty());
    }

    #[tokio::test]
    async fn direct_metadata_selected_explicitly_streams_bearer() {
        let metadata = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .and(query_param(
                "scopes",
                "https://www.googleapis.com/auth/cloud-platform",
            ))
            .and(header("Metadata-Flavor", "Google"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"mdtok","expires_in":3600}"#)
                    .insert_header("Metadata-Flavor", "Google"),
            )
            .expect(1)
            .mount(&metadata)
            .await;

        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer mdtok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let host = metadata.uri().trim_start_matches("http://").to_owned();
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::DirectMetadata {
                        residency: Some(false),
                    },
                    Arc::new(MapContext::default().with_env("GCE_METADATA_HOST", &host)),
                    None,
                ),
            },
        );
        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("metadata resolves")
            .expect("configured");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::DirectMetadata
            }
        );
        drain_to_done(&models).await;
    }

    fn b64url_decode(input: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = Vec::new();
        let mut buffer: u32 = 0;
        let mut bits = 0;
        for &byte in input.as_bytes() {
            if byte == b'=' {
                continue;
            }
            let value = ALPHABET
                .iter()
                .position(|&a| a == byte)
                .expect("valid base64url byte") as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    }
}

/// Task 20.21.2: external-account federation through the public path.
mod adc_external {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, AuthResolver, ExternalAccountSupplier,
    };
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const AUDIENCE: &str =
        "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o";
    const SUBJECT_TOKEN_TYPE: &str = "urn:ietf:params:oauth:token-type:jwt";

    struct MapContext {
        env: HashMap<String, String>,
        process_dir: tempfile::TempDir,
    }

    impl Default for MapContext {
        fn default() -> Self {
            Self {
                env: HashMap::new(),
                process_dir: tempfile::tempdir().unwrap(),
            }
        }
    }

    impl MapContext {
        fn with_env(mut self, name: &str, value: &str) -> Self {
            self.env.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl AuthContext for MapContext {
        fn process_snapshot(&self) -> Result<opi_ai::auth::AuthProcessSnapshot, AuthContextError> {
            Ok(opi_ai::test_support::portable_auth_process_snapshot(
                self.process_dir.path(),
            ))
        }
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint),
        )
    }

    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    fn external_json(credential_source: &str) -> String {
        format!(
            r#"{{"type":"external_account","audience":"{AUDIENCE}","subject_token_type":"{SUBJECT_TOKEN_TYPE}","credential_source":{credential_source}}}"#
        )
    }

    async fn drain_to_done(models: &Models) {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(
                event,
                opi_ai::stream::AssistantStreamEvent::Done { .. }
                    | opi_ai::stream::AssistantStreamEvent::Error { .. }
            ) {
                assert!(!matches!(
                    event,
                    opi_ai::stream::AssistantStreamEvent::Error { .. }
                ));
                break;
            }
        }
    }

    #[tokio::test]
    async fn external_file_supplier_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "k8s-projected-token").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"file":{}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            )),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Atoken-exchange",
            ))
            .and(body_string_contains("audience=%2F%2Fiam.googleapis.com"))
            .and(body_string_contains("subject_token=k8s-projected-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer fedtok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::File,
                    workforce_user_project: None,
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "fedtok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_sts_failure_sends_zero_generation() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "s").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"file":{}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            )),
        )
        .unwrap();

        let sts = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#),
            )
            .mount(&sts)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", sts.uri())),
                ),
            },
        );

        let error = models
            .get_auth("vertex", Default::default())
            .await
            .expect_err("failed supplier/STS fails the resolution");
        assert!(matches!(
            error,
            opi_ai::models::ModelsError::AuthResolution { .. }
        ));
        assert!(generation.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn external_executable_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let expires = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let response = format!(
            r#"{{"version":1,"success":true,"token_type":"{SUBJECT_TOKEN_TYPE}","id_token":"pluggable-token","expiration_time":{expires}}}"#
        );
        // Platform bodies stay plain strings; the executable command is a
        // direct (quote-aware) argv, never a shell re-parse.
        let unix = format!("printf '%s' '{}'", response);
        let windows = format!("Write-Host -NoNewline '{}'", response);
        let command =
            opi_ai::test_support::portable_executable_script(dir.path(), "helper", &unix, &windows);
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"executable":{{"command":"{}"}}}}"#,
                command.replace('\\', "\\\\").replace('"', "\\\"")
            )),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("subject_token=pluggable-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"exetok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer exetok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(
                        MapContext::default()
                            .with_env("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1"),
                    ),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::Executable,
                    workforce_user_project: None,
                }
            }
        );
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_aws_env_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(
                r#"{"environment_id":"aws1","regional_cred_verification_url":"https://sts.{region}.example.dev?Action=GetCallerIdentity&Version=2011-06-15"}"#,
            ),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("GetCallerIdentity"))
            .and(body_string_contains(urlencoded(AUDIENCE)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"awstok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer awstok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(
                        MapContext::default()
                            .with_env("AWS_REGION", "us-east-1")
                            .with_env("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE")
                            .with_env("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI")
                            .with_env("AWS_SESSION_TOKEN", "session-token"),
                    ),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::Aws,
                    workforce_user_project: None,
                }
            }
        );
        drain_to_done(&models).await;
    }

    fn urlencoded(value: &str) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("", value)
            .finish()
            .trim_start_matches('=')
            .to_owned()
    }
}

// ---------------------------------------------------------------------------
// Task 20.21.3 — impersonation, recursive composition, keyed cache (e2e)
// ---------------------------------------------------------------------------

mod adc_impersonation {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, AuthResolver, ExternalAccountSupplier,
    };
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TARGET: &str = "target@proj.iam.gserviceaccount.com";

    #[derive(Default)]
    struct MapContext(HashMap<String, String>);

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.0.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
        impersonation_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint)
            .with_impersonation_endpoint(impersonation_endpoint),
        )
    }

    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    async fn drain_to_done(models: &Models) {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(
                event,
                opi_ai::stream::AssistantStreamEvent::Done { .. }
                    | opi_ai::stream::AssistantStreamEvent::Error { .. }
            ) {
                assert!(!matches!(
                    event,
                    opi_ai::stream::AssistantStreamEvent::Error { .. }
                ));
                break;
            }
        }
    }

    fn impersonated_creds(path: &std::path::Path, source_json: &str, quota: Option<&str>) {
        let quota_field = quota
            .map(|q| format!(r#","quota_project_id":"{q}""#))
            .unwrap_or_default();
        std::fs::write(
            path,
            format!(
                r#"{{"type":"impersonated_service_account","source_credentials":{source_json},"service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"{quota_field}}}"#
            ),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn impersonated_authorized_user_streams_bearer_with_quota_and_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            Some("quota-proj"),
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .and(header("authorization", "Bearer sourcetok"))
            .and(body_string_contains(
                "\"https://www.googleapis.com/auth/cloud-platform\"",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer imptok"))
            .and(header("x-goog-user-project", "quota-proj"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::Impersonated {
                    source: Box::new(AdcCredentialSource::AuthorizedUser {
                        quota_project: None,
                    }),
                    target_principal: TARGET.to_owned(),
                    delegates: 0,
                    quota_project: Some("quota-proj".to_owned()),
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "imptok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn owner_caches_the_impersonated_chain_across_streams() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            None,
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            // Two streams, one acquisition chain: the keyed cache serves the
            // second call without a new source refresh or impersonation.
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .respond_with(sse_ok())
            .expect(2)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        drain_to_done(&models).await;
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_federation_then_impersonation_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "k8s-projected-token").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            format!(
                r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken","service_account_impersonation":{{"token_lifetime_seconds":2700}},"credential_source":{{"file":{}}}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            ),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("subject_token=k8s-projected-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .and(header("authorization", "Bearer fedtok"))
            .and(body_string_contains("\"lifetime\":\"2700s\""))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer imptok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::Impersonated {
                    source: Box::new(AdcCredentialSource::ExternalAccount {
                        supplier: ExternalAccountSupplier::File,
                        workforce_user_project: None,
                    }),
                    target_principal: TARGET.to_owned(),
                    delegates: 0,
                    quota_project: None,
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "imptok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn impersonation_failure_sends_zero_generation() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            None,
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_string(r#"{"error":{"status":"PERMISSION_DENIED"}}"#),
            )
            .mount(&server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models.get_auth("vertex", Default::default()).await;
        assert!(matches!(
            resolved,
            Err(opi_ai::models::ModelsError::AuthResolution { .. })
        ));
        assert!(generation.received_requests().await.unwrap().is_empty());
    }
}

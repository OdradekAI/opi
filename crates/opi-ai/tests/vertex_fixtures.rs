//! Google Vertex AI provider fixture tests (task 3.3).
//!
//! Verifies: Vertex-specific URL formatting, OAuth token auth, secret redaction,
//! SSE event mapping (reuses Gemini wire format), model resolution, and
//! request body construction. No live Google Cloud calls.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, EventStream, Provider, Request, ThinkingConfig};
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

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Error { .. })),
        "should have an Error event"
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
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let body = provider.build_request_body(&request);
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
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
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
    let events = collect_stream(provider.stream_prepared(lifecycle_text_request(), auth)).await;

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
            opi_ai::test_support::resolved_auth(),
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
            opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

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
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

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
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

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
            [Err(opi_ai::provider::ProviderError::Cancelled)]
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

//! Google Vertex AI provider fixture tests (task 3.3).
//!
//! Verifies: Vertex-specific URL formatting, OAuth token auth, secret redaction,
//! SSE event mapping (reuses Gemini wire format), model resolution, and
//! request body construction. No live Google Cloud calls.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{EventStream, Provider, Request, ThinkingConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::vertex::VertexProvider;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn vertex_provider() -> VertexProvider {
    VertexProvider::new(
        "test-access-token".into(),
        "my-project".into(),
        "us-central1".into(),
        None,
    )
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
        "token".into(),
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
    let provider = VertexProvider::new(
        "super-secret-oauth-token-12345".into(),
        "proj".into(),
        "us-central1".into(),
        None,
    );
    let debug = format!("{provider:?}");
    assert!(
        !debug.contains("super-secret-oauth-token-12345"),
        "access token leaked in Debug: {debug}"
    );
    assert!(debug.contains("***"));
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
        "token".into(),
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
    registry.register(Box::new(vertex_provider()));
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
        "test-access-token".into(),
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );

    let events = collect_stream(provider.stream(lifecycle_text_request())).await;

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
async fn stream_http_error_maps_to_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            "data: {\"error\":{\"code\":401,\"message\":\"invalid token\",\"status\":\"UNAUTHENTICATED\"}}\n\n",
        ))
        .mount(&server)
        .await;

    let provider = VertexProvider::new(
        "bad-token".into(),
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );

    let stream = provider.stream(lifecycle_text_request());
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
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
    // The CancellationToken is threaded into the Vertex adapter's HTTP
    // body-stream loop (vertex.rs `cancel.cancelled()` select arm). Cancelling
    // while the stream is open must drain promptly without hanging. This
    // wiremock fixture is fully buffered, so it does not prove cancellation
    // wins a race against delayed terminal SSE data.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = VertexProvider::new(
        "test-access-token".into(),
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut request = lifecycle_text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream(request);

    let _ = stream
        .next()
        .await
        .expect("stream should produce at least one event");
    cancel.cancel();

    let drain = async {
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) if event.is_terminal() => break,
                Err(_) => break,
                _ => {}
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(2), drain)
        .await
        .expect("stream must drain promptly after cancellation (no hang/panic)");
}

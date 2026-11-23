//! Vertex URL, auth, model, request and stream wire assertions.

use super::fixtures::*;
use super::*;

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
fn vertex_url_uses_the_published_multiregion_hosts() {
    for location in ["us", "eu"] {
        let provider = VertexProvider::new("p1".into(), location.into(), None);
        assert_eq!(
            provider.build_vertex_url("gemini-2.5-flash"),
            format!(
                "https://aiplatform.{location}.rep.googleapis.com/v1/projects/p1/locations/{location}/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
            )
        );
    }
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

#[tokio::test]
async fn standalone_vertex_rejects_the_retired_model_and_wrong_provider_before_http() {
    let server = MockServer::start().await;
    let express = VertexProvider::express(Some(server.uri()));
    let regional = VertexProvider::new("p1".into(), "us".into(), Some(server.uri()));
    for (provider, auth) in [
        (&express, opi_ai::test_support::resolved_auth()),
        (&regional, opi_ai::test_support::resolved_bearer_auth()),
    ] {
        assert!(
            provider
                .models()
                .iter()
                .all(|model| model.id != "gemini-3.1-flash-lite-image")
        );
        for (spec, rejected) in [
            (
                "vertex:gemini-3.1-flash-lite-image",
                "gemini-3.1-flash-lite-image",
            ),
            ("google:gemini-2.5-flash", "google:gemini-2.5-flash"),
        ] {
            let mut request = lifecycle_text_request();
            request.model = spec.into();
            assert!(matches!(
                provider.stream_prepared(request, auth.clone()).next().await,
                Some(Err(ProviderError::UnknownModel { provider_id, model_id }))
                    if provider_id == "vertex" && model_id == rejected
            ));
        }
    }

    let mut request = lifecycle_text_request();
    request.model = "vertex:gemini-2.5-flash".into();
    let failure = express
        .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(
        matches!(direct_cause(&failure), ProviderError::Config(message) if message.contains("project ID"))
    );
    assert!(server.received_requests().await.unwrap().is_empty());
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
async fn vertex_malformed_tail_after_finish_suppresses_deferred_done() {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":7}}\n\n",
        "data: {not-json}\n\n",
    );
    let events: Vec<_> = vertex_provider()
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;
    assert!(events.iter().any(Result::is_err));
    assert!(
        !events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
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
        projection: Default::default(),
        transport: Default::default(),
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
        telemetry_context: None,
        callbacks: Default::default(),
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

//! OpenRouter provider profile fixture tests (task 2.2).
//!
//! Verifies: model resolution, routing through OpenAI-compatible adapter,
//! request body construction, and SSE streaming diagnostics.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_chat::{CompatConfig, OpenAiChatProvider};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create an OpenRouter-configured provider.
fn openrouter_provider() -> OpenAiChatProvider {
    opi_ai::openrouter::openrouter_provider(None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

async fn spawn_stalled_openrouter_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled OpenRouter server");
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
        write_chunk(&mut socket, stalled_openrouter_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_openrouter_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

fn stalled_openrouter_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

fn stalled_openrouter_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    )
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn openrouter_provider_id_is_openrouter() {
    let provider = openrouter_provider();
    assert_eq!(provider.id(), "openrouter");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn openrouter_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let (provider, model) = registry
        .resolve("openrouter:anthropic/claude-sonnet-4")
        .unwrap();
    assert_eq!(provider.id(), "openrouter");
    assert_eq!(model.id, "anthropic/claude-sonnet-4");
}

#[test]
fn openrouter_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let ids = registry.provider_ids();
    assert!(ids.contains(&"openrouter"));
}

#[test]
fn openrouter_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let result = registry.resolve("openrouter:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body — prefix stripping
// ---------------------------------------------------------------------------

#[test]
fn openrouter_request_body_strips_provider_prefix() {
    let provider = openrouter_provider();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
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
    assert_eq!(body["model"], "openai/gpt-4o");
}

// ---------------------------------------------------------------------------
// SSE text streaming through OpenAI adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openrouter_text_streaming_produces_start_delta_done() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}],\"model\":\"anthropic/claude-sonnet-4\"}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi there\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

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
async fn openrouter_done_message_has_openrouter_provider() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done, "openrouter");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openrouter_tool_call_streaming_works() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}]}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}}]}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":20,\"completion_tokens\":10}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

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
async fn openrouter_error_event_routing() {
    let provider = openrouter_provider();
    let sse = "data: {\"error\":{\"message\":\"Model not found\"}}\n\n\
               data: [DONE]\n\n";
    let items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;

    // Current producers stop the dual terminal format: the error frame routes
    // to a typed stream failure, not an Error event.
    assert!(
        items.iter().any(|e| e.as_ref().is_err_and(
            |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
        ))
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
async fn openrouter_usage_in_done_event() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"test\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":42,\"completion_tokens\":13}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done.input_tokens, 42);
    assert_eq!(done.output_tokens, 13);
}

// ---------------------------------------------------------------------------
// Model list
// ---------------------------------------------------------------------------

#[test]
fn openrouter_has_model_list() {
    let provider = openrouter_provider();
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "OpenRouter should have at least one model"
    );
    assert!(
        models.iter().any(|m| m.id.contains('/')),
        "OpenRouter model IDs should use provider/model format"
    );
}

// ---------------------------------------------------------------------------
// Custom base URL
// ---------------------------------------------------------------------------

#[test]
fn openrouter_custom_base_url() {
    let provider = opi_ai::openrouter::openrouter_provider(Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "openrouter");
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi there\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n\
               data: [DONE]\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("HTTP-Referer", "https://github.com/OdradekAI/opi"))
        .and(header("X-Title", "opi"))
        .and(body_partial_json(serde_json::json!({
            "model": "openai/gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_completion_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::openrouter::openrouter_provider(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
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

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        match result.expect("matched text request must stream successfully") {
            AssistantStreamEvent::Done { reason, .. } => {
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                saw_done = true;
            }
            AssistantStreamEvent::Error { .. } => {
                panic!("text fixture must complete with Done");
            }
            _ => {}
        }
    }
    assert!(saw_done, "text fixture must produce a terminal Done event");

    // verify() confirms the production request carried the OpenRouter chat body
    // (prefix-stripped model + system/user messages + max_tokens), the Bearer
    // auth header, the HTTP-Referer + X-Title identification headers, and the
    // /v1/chat/completions path.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — OpenRouter inherits the shared compat profile path
// ---------------------------------------------------------------------------

#[test]
fn openrouter_profile_inherits_shared_compat_flags() {
    // OpenRouter is a config-driven OpenAI-compatible profile. Constructed with
    // the shared OpenAI Chat adapter under the "openrouter" identity + its
    // identification headers + a developer-role/strict-tools compat, its request
    // body reflects all compat flags. This proves the family inherits the shared
    // profile path (DoD) rather than carrying a parallel serializer.
    let provider = OpenAiChatProvider::new_for_profile(
        "https://openrouter.example.com".into(),
        "openrouter".into(),
        CompatConfig {
            instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
            strict_tool_schema: true,
            ..Default::default()
        },
        vec![
            ("HTTP-Referer".into(), "https://myapp.example.com".into()),
            ("X-Title".into(), "my-app".into()),
        ],
        vec![],
    );
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
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
    };
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "OpenRouter inherits developer-role override from the shared compat path"
    );
    let tools = body["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "OpenRouter inherits strict-tool-schema from the shared compat path"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the OpenRouter adapter's HTTP
    // body-stream loop (inherited from the shared OpenAI-compat path; see
    // openai_chat.rs `cancel.cancelled()` select arm). Cancelling while the
    // stream is open must terminate it before the delayed terminal chunk
    // arrives, proving the inherited adapter path observes cancellation.
    let server = spawn_stalled_openrouter_server().await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::openrouter::openrouter_provider(Some(server));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must surface cancellation before the delayed terminal fixture completes");
    match next {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled =>
        { /* typed cancellation, as contracted */ }
        other => panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Task 20.45: typed ImageApi public path + OpenRouter image adapter.
//
// Pins the pinned pi openrouter-images contract through the public
// `Models::generate_images` seam: nonstream Chat-wire request shape,
// first-choice ordered text/data-URL parsing, source timeout/retry/cancel
// semantics, typed accounting, and pre-dispatch rejections.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ImageApi;
use opi_ai::api::openrouter_images::{
    OpenRouterImagesApi, OpenRouterImagesOptions, default_openrouter_image_record,
};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::images::{
    ImageCostRates, ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesInput,
    ImagesOutput, ImagesStopReason,
};
use opi_ai::models::{ApiId, Models};
use opi_ai::telemetry::TelemetryContext;

fn image_cost_rates() -> ImageCostRates {
    ImageCostRates {
        input: 3.0,
        output: 15.0,
        cache_read: 0.3,
        cache_write: 3.75,
    }
}

fn image_record(base_url: String, output: Vec<ImageOutputModality>) -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new("openrouter-images"),
        base_url,
        output,
        cost: Some(image_cost_rates()),
    }
}

fn image_models(record: &ImageModelRecord) -> Models {
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register image api");
    models
}

#[test]
fn image_registration_preserves_arbitrary_exact_api_ids_without_auth() {
    for id in ["normal", "", " \t", "api/β\0id"] {
        let mut models = Models::new();
        let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
        let mut record = image_record("http://127.0.0.1:1".into(), vec![]);
        record.api = ApiId::new(id);
        let handle = models
            .register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .expect("ApiId has no nonempty or whitespace grammar");
        assert_eq!(handle.api_id().as_str(), id);
        assert_eq!(
            models
                .image_model("openrouter", &record.id)
                .unwrap()
                .api
                .as_str(),
            id
        );
        record.provider_id = " \t".into();
        assert!(matches!(
            models.register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record)),
                resolver.clone(),
                AuthProvenanceSource::Static,
            ),
            Err(opi_ai::registry::RegistrationError::EmptyProviderId)
        ));
        assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 0);
    }
}

fn images_call() -> ImagesCall {
    ImagesCall {
        timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    }
}

fn text_part_json(text: &str) -> serde_json::Value {
    serde_json::json!({ "type": "text", "text": text })
}

#[tokio::test]
async fn images_first_choice_orders_text_then_data_url_images() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-123",
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Done",
                        "images": [
                            { "image_url": "data:image/png;base64,QUJD" },
                            { "image_url": "https://example.com/not-a-data-url.png" },
                            { "image_url": { "url": "data:image/jpeg;base64,R0lG" } },
                            { "image_url": "data:broken-no-base64-marker" },
                            { "image_url": { } }
                        ]
                    },
                    "finish_reason": "stop"
                },
                {
                    "message": {
                        "content": "second choice",
                        "images": [{ "image_url": "data:image/png;base64,SRVDT05E" }]
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let context = ImagesContext {
        input: vec![ImagesInput::Text {
            text: "draw a cat".into(),
        }],
    };
    let result = models.generate_images(record, context, images_call()).await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(result.response_id.as_deref(), Some("gen-123"));
    assert_eq!(
        result.output,
        vec![
            ImagesOutput::Text {
                text: "Done".into()
            },
            ImagesOutput::Image {
                mime_type: "image/png".into(),
                data: "QUJD".into()
            },
            ImagesOutput::Image {
                mime_type: "image/jpeg".into(),
                data: "R0lG".into()
            },
        ]
    );
    assert!(result.error.is_none());
    assert!(result.error_message.is_none());

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json request body");
    assert_eq!(body["model"], "google/gemini-2.5-flash-image");
    assert_eq!(body["stream"], false);
    assert_eq!(body["modalities"], serde_json::json!(["image"]));
    assert_eq!(
        body["messages"],
        serde_json::json!([
            { "role": "user", "content": [text_part_json("draw a cat")] }
        ])
    );
    assert!(
        sent[0]
            .headers
            .get("authorization")
            .expect("bearer header")
            .to_str()
            .unwrap()
            .starts_with("Bearer ")
    );
}

#[tokio::test]
async fn images_text_only_and_empty_results_stay_success() {
    let text_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-text-only",
            "choices": [
                { "message": { "role": "assistant", "content": "text only", "images": [] } }
            ]
        })))
        .expect(1)
        .mount(&text_server)
        .await;
    let text_record = image_record(text_server.uri(), vec![ImageOutputModality::Image]);
    let text_models = image_models(&text_record);
    let text_only = text_models
        .generate_images(
            text_record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;
    assert_eq!(text_only.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        text_only.output,
        vec![ImagesOutput::Text {
            text: "text only".into()
        }]
    );

    let empty_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-empty",
            "choices": [
                { "message": { "role": "assistant", "content": "", "images": [] } }
            ]
        })))
        .expect(1)
        .mount(&empty_server)
        .await;
    let empty_record = image_record(empty_server.uri(), vec![ImageOutputModality::Image]);
    let empty_models = image_models(&empty_record);
    let empty = empty_models
        .generate_images(
            empty_record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;
    assert_eq!(empty.stop_reason, ImagesStopReason::Stop);
    assert!(empty.output.is_empty());
}

#[tokio::test]
async fn images_zero_choices_is_success() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-none",
            "choices": []
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert!(result.output.is_empty());
    assert_eq!(result.response_id.as_deref(), Some("gen-none"));
}

#[tokio::test]
async fn images_usage_splits_cache_buckets_and_costs_use_base_rates() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-usage",
            "choices": [
                { "message": { "role": "assistant", "content": "ok" } }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 40,
                "prompt_tokens_details": {
                    "cached_tokens": 30,
                    "cache_write_tokens": 10
                }
            }
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert!(usage.reported);
    // cacheRead = reportedCached - cacheWrite when cacheWrite > 0;
    // input excludes both cache buckets (pi parseUsage).
    assert_eq!(usage.input_tokens, 70);
    assert_eq!(usage.cache_read_tokens, 20);
    assert_eq!(usage.cache_write_tokens, 10);
    assert_eq!(usage.output_tokens, 40);
    assert_eq!(usage.total_tokens, Some(140));

    // Base-rate cost exception: the adapter prices with the record's base
    // per-million rates only (never tiers).
    let cost = result.cost.expect("cost computed");
    let eps = 1e-12;
    assert!((cost.input - 70.0 * 3.0 / 1_000_000.0).abs() < eps);
    assert!((cost.output - 40.0 * 15.0 / 1_000_000.0).abs() < eps);
    assert!((cost.cache_read - 20.0 * 0.3 / 1_000_000.0).abs() < eps);
    assert!((cost.cache_write - 10.0 * 3.75 / 1_000_000.0).abs() < eps);
    assert!(
        (cost.total - (cost.input + cost.output + cost.cache_read + cost.cache_write)).abs() < eps
    );
}

#[tokio::test]
async fn images_default_zero_extra_retries_fails_immediately() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": { "message": "Rate limited", "code": "rate_limited" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    let message = result.error_message.expect("error message");
    assert!(message.contains("429"), "message: {message}");
    assert!(message.contains("Rate limited"), "message: {message}");
    assert!(matches!(
        result.error.as_ref().expect("typed error"),
        ProviderError::RateLimited { .. }
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Manual server answering the first request with 429 + `retry-after-ms: 5`
/// and every later request with a successful generation, capturing the raw
/// request bodies for the frozen-payload assertion.
async fn spawn_retry_images_server() -> (String, std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind retry server");
    let addr = listener.local_addr().expect("retry server addr");
    let bodies = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = std::sync::Arc::clone(&bodies);
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut served = 0;
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buf = vec![0u8; 65536];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            let mut body = buf[..read].to_vec();
            // Keep only the JSON body after the blank line separator.
            if let Some(index) = body.windows(4).position(|window| window == b"\r\n\r\n") {
                body = body[index + 4..].to_vec();
            }
            captured.lock().expect("bodies").push(body);
            served += 1;
            let response = if served == 1 {
                "HTTP/1.1 429 Too Many Requests\r\nretry-after-ms: 5\r\ncontent-type: application/json\r\n\r\n{\"error\":{\"message\":\"Rate limited\"}}"
            } else {
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"id\":\"gen-after-retry\",\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"\",\"images\":[{\"image_url\":\"data:image/png;base64,QUJD\"}]}}]}"
            };
            socket.write_all(response.as_bytes()).await.ok();
            // Close the connection so the body read ends at EOF.
            socket.shutdown().await.ok();
        }
    });
    (format!("http://{addr}"), bodies)
}

#[tokio::test]
async fn images_explicit_retry_reuses_frozen_payload() {
    let (base, bodies) = spawn_retry_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let handle = {
        // Re-register to obtain a handle with typed native options.
        let api = OpenRouterImagesApi::new(HttpClient::new(), record.clone());
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        let boxed: std::sync::Arc<dyn opi_ai::api::ImageApi<OpenRouterImagesOptions>> =
            std::sync::Arc::new(api);
        let handle = models
            .register_image_api(
                boxed.clone(),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from("test-key"),
                )),
                AuthProvenanceSource::Static,
            )
            .expect("register");
        assert_eq!(handle.api_id().as_str(), "openrouter-images");
        assert_eq!(api_id.as_str(), "openrouter-images");
        assert_eq!(provider_id, "openrouter");
        handle
    };
    let native = handle.options(OpenRouterImagesOptions {
        max_retries: 1,
        max_retry_delay_ms: None,
    });

    let result = models
        .generate_images_native(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
            native,
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        result.output,
        vec![ImagesOutput::Image {
            mime_type: "image/png".into(),
            data: "QUJD".into()
        }]
    );

    let captured = bodies.lock().expect("bodies").clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1], "retry reuses frozen payload");
}

#[tokio::test]
async fn images_retry_delay_above_cap_fails_typed() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "120000")
                .set_body_json(serde_json::json!({
                    "error": { "message": "Rate limited" }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let native = handle.options(OpenRouterImagesOptions {
        max_retries: 2,
        max_retry_delay_ms: None,
    });

    let result = models
        .generate_images_native(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
            native,
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    let message = result.error_message.expect("error message");
    assert!(
        message.contains("retry delay (max: 60s)"),
        "message: {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Manual server that answers headers plus a partial body, then hangs: the
/// source timeout must cover the body read, not only the response headers.
async fn spawn_hanging_body_images_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind hanging body server");
    let addr = listener.local_addr().expect("hanging server addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0u8; 8192];
        use tokio::io::AsyncReadExt;
        let _ = socket.read(&mut buf).await;
        use tokio::io::AsyncWriteExt;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 1000\r\n\r\n{\"partial\":",
            )
            .await
            .expect("write partial body");
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn images_timeout_is_body_inclusive() {
    let base = spawn_hanging_body_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let started = Instant::now();
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                timeout: Some(Duration::from_millis(300)),
                ..Default::default()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        result.error.as_ref().expect("typed"),
        ProviderError::Timeout
    ));
    assert!(started.elapsed() >= Duration::from_millis(300));
}

#[tokio::test]
async fn images_zero_timeout_rejects_before_request() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                timeout: Some(Duration::ZERO),
                ..Default::default()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        result.error.as_ref().expect("typed"),
        ProviderError::Config(_)
    ));
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("positive"),
        "message: {:?}",
        result.error_message
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_cancellation_maps_to_aborted() {
    let base = spawn_hanging_body_images_server().await;
    let record = image_record(base, vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let cancel = tokio_util::sync::CancellationToken::new();
    let call = ImagesCall {
        timeout: Some(Duration::from_secs(30)),
        cancel: cancel.clone(),
        ..Default::default()
    };
    let models = std::sync::Arc::new(models);
    let spawned = std::sync::Arc::clone(&models);
    let task = tokio::spawn(async move {
        spawned
            .generate_images(
                record,
                ImagesContext {
                    input: vec![ImagesInput::Text { text: "hi".into() }],
                },
                call,
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel.cancel();
    let result = task.await.expect("task joins");

    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
}

/// Resolver that always fails: pins the missing-key pre-dispatch rejection.
struct FailingAuthResolver;

impl AuthResolver for FailingAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ResolvedAuth, ProviderError>> + Send + 'a>,
    > {
        Box::pin(async {
            Err(ProviderError::CredentialNeeded {
                provider_id: "openrouter".into(),
            })
        })
    }
}

#[tokio::test]
async fn images_missing_key_rejects_typed_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(FailingAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("No API key for provider: openrouter"),
        "message: {:?}",
        result.error_message
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_wrong_operation_rejects_before_auth() {
    let resolver_count = std::sync::Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    let record = default_openrouter_image_record();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                resolver_count.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    // A chat model from the registered OpenRouter chat profile is not an
    // image model: operation-qualified selection rejects typed.
    let mut chat_models =
        opi_ai::test_support::single_route_collection(Box::new(openrouter_provider()));
    chat_models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                resolver_count.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let chat_err = chat_models
        .image_model("openrouter", "anthropic/claude-sonnet-4")
        .expect_err("chat model is not an image model");
    assert!(
        chat_err.to_string().contains("not an image model"),
        "error: {chat_err}"
    );

    // Unknown model ids also fail selection.
    let unknown = models
        .image_model("openrouter", "no-such-model")
        .expect_err("unknown model id");
    assert!(unknown.to_string().contains("no-such-model"));

    // The curated image record resolves and dispatch stays image-only.
    let selected = models
        .image_model("openrouter", &record.id)
        .expect("image record resolves");
    assert_eq!(selected.id, record.id);
    assert_eq!(
        resolver_count.load(AtomicOrdering::SeqCst),
        0,
        "selection must not resolve auth"
    );
}

#[tokio::test]
async fn images_payload_and_response_callbacks_fire_in_order() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-openrouter-id", "resp-1")
                .set_body_json(serde_json::json!({
                    "id": "gen-cb",
                    "choices": [
                        { "message": { "role": "assistant", "content": "ok" } }
                    ]
                })),
        )
        .mount(&server)
        .await;

    let record = image_record(
        server.uri(),
        vec![ImageOutputModality::Image, ImageOutputModality::Text],
    );
    let models = image_models(&record);

    let payload_first = std::sync::Arc::new(AtomicBool::new(false));
    let response_seen = std::sync::Arc::new(AtomicBool::new(false));
    let order_ok = std::sync::Arc::new(AtomicBool::new(false));
    let payload_set = std::sync::Arc::clone(&payload_first);
    let payload_check = std::sync::Arc::clone(&payload_first);
    let response_flag = std::sync::Arc::clone(&response_seen);
    let order_flag = std::sync::Arc::clone(&order_ok);

    let callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(move |body: &serde_json::Value| {
            assert_eq!(body["modalities"], serde_json::json!(["image", "text"]));
            assert_eq!(body["stream"], false);
            payload_set.store(true, AtomicOrdering::SeqCst);
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(move |status, headers| {
            assert_eq!(status, 200);
            assert!(
                headers.iter().any(|(name, _)| name == "x-openrouter-id"),
                "response headers observed: {headers:?}"
            );
            if response_flag
                .compare_exchange(false, true, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
                .is_ok()
            {
                let observed = payload_check.load(AtomicOrdering::SeqCst);
                order_flag.store(observed, AtomicOrdering::SeqCst);
            }
            Ok(())
        }));

    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                callbacks,
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert!(
        payload_first.load(AtomicOrdering::SeqCst),
        "payload observed"
    );
    assert!(
        response_seen.load(AtomicOrdering::SeqCst),
        "response observed"
    );
    assert!(
        order_ok.load(AtomicOrdering::SeqCst),
        "payload before response"
    );
}

#[tokio::test]
async fn images_unsupported_callbacks_reject_typed_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);

    let raw_callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_on_raw(std::sync::Arc::new(
            |_event: &serde_json::Value| Ok::<(), opi_ai::callbacks::CallbackError>(()),
        ));
    let raw_result = models
        .generate_images(
            record.clone(),
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                callbacks: raw_callbacks,
                ..images_call()
            },
        )
        .await;
    assert_eq!(raw_result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        raw_result.error.as_ref().expect("typed"),
        ProviderError::UnsupportedCapability { .. }
    ));

    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn image_preflight_rejects_headers_callbacks_and_timeout_before_auth() {
    for invalid in ["headers", "callbacks", "timeout"] {
        let server = wiremock::MockServer::start().await;
        let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        let auth_calls = std::sync::Arc::new(AtomicU32::new(0));
        let mut models = Models::new();
        models
            .register_image_api(
                std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
                std::sync::Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                    auth_calls.clone(),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut call = images_call();
        match invalid {
            "headers" => {
                call.extra_headers = vec![("Authorization".into(), "fixture-override".into())]
            }
            "callbacks" => {
                call.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                    .with_on_raw(std::sync::Arc::new(|_| Ok(())))
            }
            "timeout" => call.timeout = Some(Duration::ZERO),
            _ => unreachable!(),
        }
        let result = models
            .generate_images(
                record,
                ImagesContext {
                    input: vec![ImagesInput::Text {
                        text: "fixture".into(),
                    }],
                },
                call,
            )
            .await;
        assert!(result.error.is_some());
        assert_eq!(
            auth_calls.load(AtomicOrdering::SeqCst),
            0,
            "{invalid} rejects before auth"
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn image_custom_native_option_validation_runs_before_auth() {
    struct CountBearer(std::sync::Arc<AtomicU32>);
    impl AuthResolver for CountBearer {
        fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async {
                self.0.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(opi_ai::test_support::resolved_bearer_auth())
            })
        }
    }
    struct CheckedImages(OpenRouterImagesApi);
    impl ImageApi<bool> for CheckedImages {
        fn api_id(&self) -> &opi_ai::ApiId {
            self.0.api_id()
        }
        fn provider_id(&self) -> &str {
            self.0.provider_id()
        }
        fn complete_record(&self) -> &ImageModelRecord {
            self.0.complete_record()
        }
        fn default_options(&self) -> bool {
            true
        }
        fn validate_options(&self, allowed: &bool) -> Result<(), ProviderError> {
            if *allowed {
                Ok(())
            } else {
                Err(ProviderError::Config(
                    opi_ai::provider::ProviderErrorSummary::sanitized(
                        "fixture native options rejected",
                    ),
                ))
            }
        }
        fn generate_images_prepared<'a>(
            &'a self,
            record: ImageModelRecord,
            context: ImagesContext,
            _options: bool,
            auth: ResolvedAuth,
            call: ImagesCall,
        ) -> opi_ai::api::ImageFuture<'a> {
            self.0.generate_images_prepared(
                record,
                context,
                OpenRouterImagesOptions::default(),
                auth,
                call,
            )
        }
    }
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices":[]})))
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let auth_calls = std::sync::Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    let api = models
        .register_image_api(
            std::sync::Arc::new(CheckedImages(OpenRouterImagesApi::new(
                HttpClient::new(),
                record.clone(),
            ))),
            std::sync::Arc::new(CountBearer(auth_calls.clone())),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let context = ImagesContext {
        input: vec![ImagesInput::Text {
            text: "fixture".into(),
        }],
    };
    let rejected = models
        .generate_images_native(
            record.clone(),
            context.clone(),
            images_call(),
            api.options(false),
        )
        .await;
    assert!(matches!(rejected.error, Some(ProviderError::Config(_))));
    assert_eq!(auth_calls.load(AtomicOrdering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
    let accepted = models
        .generate_images_native(record, context, images_call(), api.options(true))
        .await;
    assert_eq!(accepted.stop_reason, ImagesStopReason::Stop);
    assert_eq!(auth_calls.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn images_telemetry_context_passes_through() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-tel",
            "choices": [
                { "message": { "role": "assistant", "content": "ok" } }
            ]
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                telemetry_context: Some(TelemetryContext::noop()),
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
}

#[tokio::test]
async fn images_base_url_trailing_slash_and_extra_headers() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("x-custom-image", "v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-hdr",
            "choices": []
        })))
        .mount(&server)
        .await;

    let record = image_record(
        format!("{}/", server.uri()),
        vec![ImageOutputModality::Image],
    );
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                extra_headers: vec![("x-custom-image".into(), "v1".into())],
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn images_reused_callbacks_keep_each_calls_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(2)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = observed.clone();
    let callbacks = opi_ai::callbacks::RequestCallbacks::new().with_on_payload(
        std::sync::Arc::new(move |_| {
            counter.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(None)
        }),
    );
    for prompt in ["first image", "second image"] {
        let result = models
            .generate_images(
                record.clone(),
                ImagesContext {
                    input: vec![ImagesInput::Text {
                        text: prompt.into(),
                    }],
                },
                ImagesCall {
                    callbacks: callbacks.clone(),
                    ..images_call()
                },
            )
            .await;
        assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    }
    let sent = server.received_requests().await.unwrap();
    let prompts: Vec<serde_json::Value> = sent
        .iter()
        .map(|request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            body["messages"][0]["content"][0]["text"].clone()
        })
        .collect();
    assert_eq!(
        prompts,
        [
            serde_json::json!("first image"),
            serde_json::json!("second image")
        ]
    );
    assert_eq!(observed.load(AtomicOrdering::SeqCst), 2);
}

#[tokio::test]
async fn images_response_callback_is_after_json_and_outside_attempt_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext::default(),
            ImagesCall {
                timeout: Some(Duration::from_millis(300)),
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
                    |_, _| async {
                        tokio::time::sleep(Duration::from_millis(600)).await;
                        Ok(())
                    },
                ),
                ..Default::default()
            },
        )
        .await;
    assert_eq!(
        result.stop_reason,
        ImagesStopReason::Stop,
        "{:?}",
        result.error
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{incomplete"))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = observed.clone();
    let result = models
        .generate_images(
            record,
            ImagesContext::default(),
            ImagesCall {
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_on_response(
                    std::sync::Arc::new(move |_, _| {
                        counter.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(())
                    }),
                ),
                ..images_call()
            },
        )
        .await;
    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert_eq!(observed.load(AtomicOrdering::SeqCst), 0);
}

#[tokio::test]
async fn images_protected_headers_never_reach_direct_transport() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let adapter = OpenRouterImagesApi::new(HttpClient::new(), record.clone());
    for header in ["Authorization", "aPi-KeY", "Host"] {
        let result = adapter
            .generate_images_prepared(
                record.clone(),
                ImagesContext::default(),
                adapter.default_options(),
                ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::Bearer,
                    secret: secrecy::SecretString::from("fixture-key"),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                },
                ImagesCall {
                    extra_headers: vec![(header.into(), "untrusted-override".into())],
                    ..images_call()
                },
            )
            .await;
        assert!(
            matches!(result.error, Some(ProviderError::RequestFailed(_))),
            "{:?}",
            result.error
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_empty_and_partial_usage_do_not_become_reliable_zero() {
    for raw in [
        serde_json::json!({}),
        serde_json::json!({"prompt_tokens": "many", "completion_tokens": 7}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"choices": [], "usage": raw})),
            )
            .mount(&server)
            .await;
        let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        let models = image_models(&record);
        let result = models
            .generate_images(record, ImagesContext::default(), images_call())
            .await;
        assert_eq!(result.stop_reason, ImagesStopReason::Stop);
        if raw.get("completion_tokens").is_some() {
            let usage = result.usage.expect("valid output count survives");
            assert_eq!(usage.output_tokens, 7);
            assert!(!usage.reported);
            assert_eq!(usage.total_tokens, None);
        } else {
            assert!(result.usage.is_none());
        }
        assert!(result.cost.is_none());
    }
}

#[tokio::test]
async fn images_unknown_prices_stay_distinct_from_known_free_usage() {
    assert!(default_openrouter_image_record().cost.is_none());
    for cost in [None, Some(ImageCostRates::default())] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [], "usage": {"prompt_tokens": 12, "completion_tokens": 0}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let mut record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        record.cost = cost;
        let models = image_models(&record);
        let result = models
            .generate_images(record, ImagesContext::default(), images_call())
            .await;
        let usage = result.usage.expect("valid usage survives unknown prices");
        assert!(usage.reported);
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(result.cost.map(|cost| cost.total), cost.map(|_| 0.0));
    }
}

#[tokio::test]
async fn image_call_owns_binding_before_first_poll() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .expect(2)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let mut models = image_models(&record);
    let ordinary = models.generate_images(record.clone(), ImagesContext::default(), images_call());
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("native-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let native = models.generate_images_native(
        record,
        ImagesContext::default(),
        images_call(),
        handle.options(Default::default()),
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    drop(models);
    assert_eq!(ordinary.await.stop_reason, ImagesStopReason::Stop);
    assert_eq!(native.await.stop_reason, ImagesStopReason::Stop);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].headers.get("authorization").unwrap(),
        "Bearer test-key"
    );
    assert_eq!(
        requests[1].headers.get("authorization").unwrap(),
        "Bearer native-key"
    );
}

#[path = "support/pending_auth.rs"]
mod pending_auth;

#[tokio::test]
async fn image_auth_lifecycle_preaborted_never_starts_resolution() {
    let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
    let record = image_record(
        "http://127.0.0.1:1".into(),
        vec![ImageOutputModality::Image],
    );
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = images_call();
    call.cancel.cancel();
    let result = models.generate_images(record, ImagesContext::default(), call);
    tokio::pin!(result);
    let outcome = futures_util::poll!(&mut result);
    assert_eq!(
        resolver.calls.load(AtomicOrdering::SeqCst),
        0,
        "pre-aborted generation must not start auth"
    );
    let std::task::Poll::Ready(result) = outcome else {
        panic!("pre-aborted image call must settle")
    };
    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
}

#[tokio::test]
async fn image_auth_lifecycle_cancel_releases_pending_resolution() {
    let resolver = std::sync::Arc::new(pending_auth::PendingAuth::default());
    let record = image_record(
        "http://127.0.0.1:1".into(),
        vec![ImageOutputModality::Image],
    );
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = images_call();
    let cancel = call.cancel.clone();
    let result = models.generate_images(record, ImagesContext::default(), call);
    tokio::pin!(result);
    assert!(futures_util::poll!(&mut result).is_pending());
    assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 1);
    cancel.cancel();
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut result) else {
        panic!("cancellation must interrupt pending image auth")
    };
    assert_eq!(result.stop_reason, ImagesStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
    assert!(resolver.released.load(AtomicOrdering::SeqCst));
}

#[tokio::test]
async fn custom_chat_replacement_keeps_its_resolver_after_store_attachment() {
    use opi_ai::credential::{
        BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
    };
    use std::sync::Arc;

    struct NoAmbient;
    impl opi_ai::auth::AuthContext for NoAmbient {
        fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(None)
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            Ok(false)
        }
    }
    struct Store(AtomicU32);
    impl CredentialStore for Store {
        fn read<'a>(
            &'a self,
            _: &'a str,
        ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
            self.0.fetch_add(1, AtomicOrdering::SeqCst);
            Box::pin(async {
                Ok(Some(Credential::ApiKey(
                    secrecy::SecretString::from("builtin-store-key").into(),
                )))
            })
        }
        fn write<'a>(
            &'a self,
            _: &'a str,
            _: &'a Credential,
        ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
            panic!("fixture never writes")
        }
        fn delete<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
            panic!("fixture never deletes")
        }
        fn probe<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
            Box::pin(async { CredentialSource::Absent })
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
        .expect(1).mount(&server).await;
    let mut models = Models::new();
    let context: Arc<dyn opi_ai::auth::AuthContext> = Arc::new(NoAmbient);
    models.set_auth_context(context.clone());
    opi_ai::provider_collection::register_builtin(
        &mut models,
        context,
        Arc::new(HttpClient::new()),
        &Default::default(),
    )
    .unwrap();
    let info = opi_ai::ModelInfo::new(
        "custom",
        "Custom",
        opi_ai::WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(8192, 1024).with_streaming(true),
    )
    .with_base_url(server.uri());
    let api = opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
        Arc::new(opi_ai::openrouter::openrouter_provider(Some(server.uri()))),
        info.clone(),
    )
    .unwrap();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("custom-route-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let store = Arc::new(Store(AtomicU32::new(0)));
    models.set_credential_store(store.clone());
    let request = Request {
        projection: Default::default(),
        model: "openrouter:custom".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
        transport: Default::default(),
    };
    models
        .complete(
            opi_ai::ChatModelRecord::new("openrouter", info),
            request,
            handle.options(Default::default()),
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].headers.get("authorization").unwrap(),
        "Bearer custom-route-key"
    );
    assert_eq!(
        store.0.load(AtomicOrdering::SeqCst),
        0,
        "custom route must not read replaced builtin sources"
    );
}

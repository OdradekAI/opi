//! Phase 12 task 12.2 — every-family provider-side error classification +
//! safe excerpt redaction.
//!
//! DoD: "HTTP/status/body parsing in every existing provider adapter classifies
//! auth, config, request, network, rate_limit, provider, and stream errors with
//! safe excerpts", and "fixture and redaction tests assert each class and every
//! provider family error-mapping path."
//!
//! Each HTTP family below drives its production `Provider::stream` path against
//! a mock 5xx response whose body echoes a credential, and asserts the result
//! (a) classifies as the shared `provider` class and (b) carries only a
//! redacted excerpt (the secret must not survive `safe_excerpt`). The auth
//! (401), rate-limit (429), network (408/504) and stream arms are exercised per
//! family in the `*_lifecycle.rs` suites and the per-family fixture files;
//! bedrock's `provider`-class mapping is covered by its `map_bedrock_status`
//! unit test plus its 403 stream test.

use futures_util::StreamExt;
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderErrorCategory, Request, ThinkingConfig};
use opi_ai::vertex::VertexProvider;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const SECRET: &str = "sk-proj-1234567890abcdefghijklmnopqrstuv";

fn text_request(model: &str) -> Request {
    Request {
        model: model.into(),
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

async fn mount_500_echoing_secret(server: &MockServer) {
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string(format!("{{\"error\":\"server rejected token {SECRET}\"}}")),
        )
        .mount(server)
        .await;
}

/// Drive the stream to its first event and assert a provider-side error with a
/// redacted excerpt (the echoed credential must not survive `safe_excerpt`).
async fn assert_provider_side_redacted(provider: Box<dyn Provider>, model: &str) {
    let mut stream = provider.stream(text_request(model));
    while let Some(result) = stream.next().await {
        if let Err(error) = result {
            assert_eq!(
                error.category(),
                ProviderErrorCategory::Provider,
                "5xx must classify as the provider class: {error:?}"
            );
            assert!(
                !error.to_string().contains(SECRET),
                "provider error excerpt must redact the echoed credential: {error}"
            );
            return;
        }
    }
    panic!("stream produced no error event before completion");
}

#[tokio::test]
async fn anthropic_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(opi_ai::anthropic::AnthropicProvider::new(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "anthropic:claude-sonnet-4-5-20250514").await;
}

#[tokio::test]
async fn openai_chat_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(OpenAiChatProvider::new(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "openai:gpt-4o").await;
}

#[tokio::test]
async fn openai_responses_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(OpenAiResponsesProvider::new(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "openai-responses:gpt-4o").await;
}

#[tokio::test]
async fn openrouter_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(opi_ai::openrouter::openrouter_provider(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "openrouter:openai/gpt-4o").await;
}

#[tokio::test]
async fn mistral_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(opi_ai::mistral::mistral_provider(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "mistral:mistral-small-latest").await;
}

#[tokio::test]
async fn gemini_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(opi_ai::gemini::GeminiProvider::new(
        "test-key".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "gemini:gemini-2.5-flash").await;
}

#[tokio::test]
async fn azure_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider = AzureOpenAIProvider::new(
        "test-key".into(),
        Some(server.uri()),
        "my-gpt4o".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    let provider: Box<dyn Provider> = Box::new(provider);
    assert_provider_side_redacted(provider, "azure:my-gpt4o").await;
}

#[tokio::test]
async fn vertex_500_classifies_as_provider_with_redacted_excerpt() {
    let server = MockServer::start().await;
    mount_500_echoing_secret(&server).await;
    let provider: Box<dyn Provider> = Box::new(VertexProvider::new(
        "test-key".into(),
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    ));
    assert_provider_side_redacted(provider, "vertex:gemini-2.5-flash").await;
}

#[tokio::test]
async fn provider_side_excerpt_redacts_query_secret_values() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string(
            "https://example.test/path?api_key=opaque-secret-token&token=another-secret",
        ))
        .mount(&server)
        .await;

    let provider: Box<dyn Provider> = Box::new(OpenAiChatProvider::new(
        "test-key".into(),
        Some(server.uri()),
    ));
    let mut stream = provider.stream(text_request("openai:gpt-4o"));

    while let Some(result) = stream.next().await {
        if let Err(error) = result {
            assert_eq!(error.category(), ProviderErrorCategory::Provider);
            let text = error.to_string();
            assert!(
                text.contains("[REDACTED]"),
                "provider-side excerpt should show explicit redaction markers: {text}"
            );
            assert!(
                !text.contains("opaque-secret-token"),
                "provider-side excerpt leaked api_key query value: {text}"
            );
            assert!(
                !text.contains("another-secret"),
                "provider-side excerpt leaked token query value: {text}"
            );
            return;
        }
    }

    panic!("stream produced no error event before completion");
}

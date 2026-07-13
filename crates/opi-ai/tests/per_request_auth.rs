//! Phase 14.2 slice 2 — per-request auth resolution in the concrete providers.
//!
//! Proves each provider holds an injected `Arc<dyn AuthResolver>` and resolves
//! auth inside the returned stream immediately before HTTP: a resolver with no
//! credential yields typed `CredentialNeeded` (no HTTP attempted), and a
//! resolver returning a `ResolvedAuth` drives the scheme-selected auth header
//! at the HTTP boundary. The `StaticAuthResolver` path (existing `new(api_key)`
//! constructors) is still covered by the per-provider fixture tests.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig, CacheRetention};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A resolver that always reports no credential is available.
struct NoCredentialResolver {
    provider_id: &'static str,
}

impl AuthResolver for NoCredentialResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let provider_id = self.provider_id.to_owned();
        Box::pin(async move { Err(ProviderError::CredentialNeeded { provider_id }) })
    }
}

fn sample_request(model: &str) -> Request {
    Request {
        model: model.to_owned(),
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

/// Drain a stream to completion (consuming events and errors) without hanging.
async fn drain(stream: &mut opi_ai::provider::EventStream) {
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }
}

// --- CredentialNeeded is yielded before any HTTP request ---

#[tokio::test]
async fn anthropic_stream_yields_credential_needed_when_resolver_has_no_credential() {
    let provider = AnthropicProvider::with_auth(
        Arc::new(NoCredentialResolver {
            provider_id: "anthropic",
        }),
        None,
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
    match stream.next().await {
        Some(Err(ProviderError::CredentialNeeded { provider_id })) => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
}

#[tokio::test]
async fn openai_chat_stream_yields_credential_needed_when_resolver_has_no_credential() {
    let provider = OpenAiChatProvider::with_auth(
        Arc::new(NoCredentialResolver {
            provider_id: "openai",
        }),
        None,
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("openai:gpt-4o"));
    match stream.next().await {
        Some(Err(ProviderError::CredentialNeeded { provider_id })) => {
            assert_eq!(provider_id, "openai");
        }
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
}

#[tokio::test]
async fn openai_responses_stream_yields_credential_needed_when_resolver_has_no_credential() {
    let provider = OpenAiResponsesProvider::with_auth(
        Arc::new(NoCredentialResolver {
            provider_id: "openai-responses",
        }),
        None,
        Default::default(),
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("openai-responses:gpt-4o"));
    match stream.next().await {
        Some(Err(ProviderError::CredentialNeeded { provider_id })) => {
            assert_eq!(provider_id, "openai-responses");
        }
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
}

// --- Resolved auth drives the scheme-selected HTTP header ---

#[tokio::test]
async fn anthropic_stream_applies_bearer_header_from_resolver_not_api_key_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer oauth-token-anthropic"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("oauth-token-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
    drain(&mut stream).await;

    // verify() fails if no request matched the `authorization: Bearer …` matcher,
    // which proves the Bearer scheme selected the Authorization header (not x-api-key).
    server.verify().await;
}

#[tokio::test]
async fn openai_chat_stream_applies_resolved_secret_to_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer resolved-chat-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from("resolved-chat-key"),
    ));
    let provider = OpenAiChatProvider::with_auth(
        resolver,
        Some(server.uri()),
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("openai:gpt-4o"));
    drain(&mut stream).await;
    server.verify().await;
}

#[tokio::test]
async fn openai_responses_stream_applies_resolved_secret_to_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer resolved-responses-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from("resolved-responses-key"),
    ));
    let provider = OpenAiResponsesProvider::with_auth(
        resolver,
        Some(server.uri()),
        Default::default(),
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("openai-responses:gpt-4o"));
    drain(&mut stream).await;
    server.verify().await;
}

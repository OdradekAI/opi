//! Phase 14.2 slice 2 — per-request auth resolution and prepared dispatch.
//!
//! Phase 17.5 moved auth resolution out of the concrete providers: the
//! `Provider::stream` entry point is gone and the sole provider entry is
//! `Provider::stream_prepared(request, auth)`, which consumes an already-resolved
//! [`ResolvedAuth`] directly at the HTTP boundary. Auth resolution now lives in
//! [`Models::prepare_call`], which resolves the route's credential
//! once per logical call and freezes it for every retry attempt.
//!
//! These tests prove both halves of that contract:
//!
//! - A route whose resolver reports no credential yields a typed
//!   `CredentialNeeded` from `prepare_call` (no HTTP attempted).
//! - Driving `stream_prepared` with a supplied `ResolvedAuth` attaches the
//!   scheme-selected auth header at the HTTP boundary, without the provider
//!   consulting any resolver. The secret in `ResolvedAuth` is the only auth
//!   the wire ever sees.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthProvenance, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::gemini::GeminiProvider;
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::vertex::VertexProvider;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
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
        projection: Default::default(),
        transport: Default::default(),
        model: model.to_owned(),
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

// --- CredentialNeeded surfaces from prepare_call before any HTTP request ---

// opi-phase17-acceptance
#[tokio::test]
async fn prepare_call_yields_credential_needed_when_route_resolver_has_no_credential() {
    // Phase 17.5: auth resolution is the collection's job. Registering a route
    // whose resolver always reports no credential must yield a typed
    // `CredentialNeeded` from `prepare_call`, before the provider is ever asked
    // to stream. (This behavior is provider-agnostic: it is the route resolver
    // and the collection that decide it, so one representative route covers the
    // contract the per-provider `stream`-level tests used to assert.)
    let provider = AnthropicProvider::new(Some("http://127.0.0.1:1".into()));
    let mut collection = opi_ai::Models::new();
    collection
        .register_route(
            Box::new(provider),
            Arc::new(NoCredentialResolver {
                provider_id: "anthropic",
            }) as Arc<dyn AuthResolver>,
            opi_ai::AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();

    let error = collection
        .prepare_call(
            "anthropic:claude-sonnet-4-5-20250514",
            sample_request("anthropic:claude-sonnet-4-5-20250514"),
        )
        .await
        .unwrap_err();
    match error {
        opi_ai::ModelsError::Provider(ProviderError::StreamFailure(failure)) => {
            assert!(failure.message().projection.is_some());
            assert!(
                matches!(failure.cause(),ProviderError::CredentialNeeded { provider_id } if provider_id == "anthropic")
            );
        }
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
}

// --- Resolved auth drives the scheme-selected HTTP header ---

#[tokio::test]
async fn anthropic_stream_prepared_applies_bearer_header_not_api_key_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer oauth-token-anthropic"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()));
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("oauth-token-anthropic"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        resolved,
    );
    drain(&mut stream).await;

    // The exact-match expectation proves the supplied Bearer token was sent.
    server.verify().await;
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].headers.contains_key("x-api-key"),
        "Bearer authentication must not also send an API-key header"
    );
}

#[tokio::test]
async fn openai_chat_stream_prepared_applies_resolved_secret_to_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer resolved-chat-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("resolved-chat-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("openai:gpt-4o"), resolved);
    drain(&mut stream).await;
    server.verify().await;
}

#[tokio::test]
async fn openai_responses_stream_prepared_applies_resolved_secret_to_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer resolved-responses-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("resolved-responses-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("openai-responses:gpt-4o"), resolved);
    drain(&mut stream).await;
    server.verify().await;
}

// --- Phase 17: prepared dispatch uses supplied auth directly at the wire ---

#[tokio::test]
async fn anthropic_stream_prepared_uses_supplied_auth_at_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer prepared-token-anthropic"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    // The provider holds no resolver in the Phase 17.5 model; the supplied
    // resolved auth is the only credential that reaches the HTTP boundary.
    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()));
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("prepared-token-anthropic"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        resolved,
    );
    drain(&mut stream).await;

    // verify() fails if no request matched the supplied Bearer matcher.
    server.verify().await;
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].headers.contains_key("x-api-key"),
        "supplied Bearer authentication must not also send an API-key header"
    );
}

#[tokio::test]
async fn openai_chat_stream_prepared_uses_supplied_auth_at_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer prepared-chat-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("prepared-chat-token"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("openai:gpt-4o"), resolved);
    drain(&mut stream).await;
    server.verify().await;
}

#[tokio::test]
async fn openai_responses_stream_prepared_uses_supplied_auth_at_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer prepared-responses-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("prepared-responses-token"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("openai-responses:gpt-4o"), resolved);
    drain(&mut stream).await;
    server.verify().await;
}

#[tokio::test]
async fn openai_codex_stream_prepared_uses_supplied_auth_at_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer prepared-codex-token"))
        .and(header("chatgpt-account-id", "prepared-codex-account"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![ModelInfo::new(
            "gpt-5",
            "gpt-5",
            WireApi::OpenAiCodexResponses,
            ModelCapabilities::new(100_000, 4_096),
        )],
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("prepared-codex-token"),
        base_url: None,
        account_id: Some("prepared-codex-account".into()),
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("openai-codex:gpt-5"), resolved);
    drain(&mut stream).await;
    server.verify().await;
}

// --- Prepared-scheme mismatches are rejected before the wire ---

/// Assert the first stream item is the typed scheme-rejection error and that no
/// HTTP request was dispatched (no mock is mounted, so any request would be
/// recorded by the server as unmatched).
async fn assert_scheme_rejected_before_wire(
    mut stream: opi_ai::provider::EventStream,
    server: &MockServer,
) {
    let error = stream
        .next()
        .await
        .expect("scheme mismatch should produce an event")
        .expect_err("scheme mismatch should produce ProviderError");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection retains failure facts: {error:?}");
    };
    assert!(failure.message().projection.is_some());
    assert!(
        matches!(failure.cause(), ProviderError::Config(_)),
        "expected Config error, got {error:?}"
    );
    assert!(stream.next().await.is_none());
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "scheme mismatch must be rejected before the wire"
    );
}

#[tokio::test]
async fn openai_chat_rejects_api_key_scheme_before_the_wire() {
    let server = MockServer::start().await;
    let provider = OpenAiChatProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("resolved-chat-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    assert_scheme_rejected_before_wire(
        provider.stream_prepared(sample_request("openai:gpt-4o"), resolved),
        &server,
    )
    .await;
}

#[tokio::test]
async fn gemini_rejects_bearer_scheme_before_the_wire() {
    let server = MockServer::start().await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("resolved-gemini-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    assert_scheme_rejected_before_wire(
        provider.stream_prepared(sample_request("gemini:gemini-2.5-flash"), resolved),
        &server,
    )
    .await;
}

#[tokio::test]
async fn azure_rejects_bearer_scheme_before_the_wire() {
    let server = MockServer::start().await;
    let provider = AzureOpenAIProvider::new(Some(server.uri()), "my-gpt4o".into(), None)
        .expect("wiremock endpoint is a valid azure endpoint");
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("resolved-azure-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    assert_scheme_rejected_before_wire(
        provider.stream_prepared(sample_request("azure:my-gpt4o"), resolved),
        &server,
    )
    .await;
}

/// Pinned @google/genai contract (task 20.21.1): an API key selects the
/// Vertex Express wire — the project/location-less collection URL with the
/// `x-goog-api-key` header — even when the provider was built with a
/// project. The old opi-only typed rejection is superseded.
#[tokio::test]
async fn vertex_api_key_selects_the_express_wire() {
    use futures_util::StreamExt;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .and(header("x-goog-api-key", "resolved-vertex-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\ndata: \"done\"\n\n",
                )
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = VertexProvider::new("proj".into(), "loc".into(), Some(server.uri()));
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("resolved-vertex-token"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(sample_request("vertex:gemini-2.5-flash"), resolved);
    let event = stream
        .next()
        .await
        .expect("express request should produce an event")
        .expect("matched Express request must stream");
    assert!(
        !matches!(event, opi_ai::stream::AssistantStreamEvent::Error { .. }),
        "Express request streams instead of failing"
    );
}

#[tokio::test]
async fn vertex_rejects_sigv4_scheme_before_the_wire() {
    let server = MockServer::start().await;
    let provider = VertexProvider::new("proj".into(), "loc".into(), Some(server.uri()));
    let resolved = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::AwsSigV4(opi_ai::auth::AwsSigV4Credentials::new(
            SecretString::from("ak"),
            SecretString::from("sk"),
            "us-east-1",
        )),
        secret: SecretString::from(""),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    assert_scheme_rejected_before_wire(
        provider.stream_prepared(sample_request("vertex:gemini-2.5-flash"), resolved),
        &server,
    )
    .await;
}

//! Phase 14.2 slice 5 — exact OAuth wire shape per provider mapping.
//!
//! Where `per_request_auth.rs` (slice 2) proves each prepared dispatch attaches
//! the scheme-selected header, these tests pin the EXACT OAuth wire contract
//! the factory-built providers must emit — not merely "a Bearer token reached
//! the wire":
//!
//! - Anthropic OAuth selects `authorization: Bearer` AND the required
//!   `anthropic-beta: claude-code-20250219,oauth-2025-04-20` header, while API-key construction
//!   keeps `x-api-key` and emits neither `authorization` nor the beta header.
//! - A 401/403 on credential-managed routes maps to typed non-retryable
//!   `ProviderError::CredentialRevoked`; static routes use bodyless
//!   `AuthFailed`, independent of the header scheme.
//!
//! Phase 17.5: providers no longer resolve auth themselves. Every dispatch
//! supplies a `ResolvedAuth` through `Provider::stream_prepared`, so these
//! tests build the resolved credential directly and drive the prepared seam.
//! The credential-managed vs static 401 policy is now an explicit per-route
//! setting (`with_auth_invalid_policy` / `for_route`), independent of how the
//! secret is delivered.
//!
//! opi-ai tests build `ResolvedAuth` directly (opi-ai cannot depend on
//! opi-coding-agent's `AuthSource`); the factory + `AuthSource` + fake-store
//! coverage lives in opi-coding-agent/tests.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthInvalidPolicy, AuthProvenance, AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::mistral::mistral_provider;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::openrouter::openrouter_provider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::{ModelCapabilities, ModelInfo, ProviderHeaders, WireApi};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The Anthropic OAuth beta tags. Pinned as a literal here (not re-exported
/// from the provider, which keeps them in a private module constant) so a
/// future rotation in `anthropic.rs` is caught at this assertion. The value is
/// pinned to the reviewed pi 0.80.6 profile.
const ANTHROPIC_OAUTH_BETA: &str = "claude-code-20250219,oauth-2025-04-20";

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

/// Build a Bearer `ResolvedAuth` carrying `token` (Phase 17.5: the prepared
/// seam consumes this directly at the HTTP boundary).
fn bearer_auth(token: &str) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(token),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    }
}

/// Build an ApiKey `ResolvedAuth` carrying `key`.
fn apikey_auth(key: &str) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from(key),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    }
}

// --- Anthropic OAuth: Bearer + the required beta header ---

/// Capture the one request the provider sent and assert its headers directly.
/// More rigorous than a matcher+`verify()` pattern (which can pass vacuously):
/// this reads the recorded request and asserts the EXACT header values.
async fn one_captured_request(server: &MockServer) -> wiremock::Request {
    let requests = server.received_requests().await.expect("requests recorded");
    assert_eq!(
        requests.len(),
        1,
        "expected exactly one request, got {}",
        requests.len()
    );
    requests.into_iter().next().unwrap()
}

#[tokio::test]
async fn anthropic_oauth_emits_bearer_plus_exact_beta_header() {
    let server = MockServer::start().await;
    // Permissive mock (any POST) returning 200 so the request is recorded; we
    // assert exact headers on the captured request, not via the matcher.
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    // The OAuth beta header is gated on `direct_oauth_beta`, which the
    // Phase 17.5 `for_route` constructor exposes as its final flag.
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![ModelInfo::new(
            "claude-sonnet-4-5-20250514",
            "Claude Sonnet 4.5",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200_000, 8_192).with_streaming(true),
        )],
        Some(server.uri()),
        ProviderHeaders::default(),
        Arc::new(HttpClient::new()),
        true,
    );
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        bearer_auth("oauth-token-anthropic"),
    );
    drain(&mut stream).await;

    let req = one_captured_request(&server).await;
    // Bearer authorization present.
    let auth = req
        .headers
        .get("authorization")
        .expect("authorization header")
        .to_str()
        .unwrap();
    assert_eq!(auth, "Bearer oauth-token-anthropic");
    // The required OAuth beta header, exact value.
    let beta = req
        .headers
        .get("anthropic-beta")
        .map(|v| v.to_str().unwrap());
    assert_eq!(
        beta,
        Some(ANTHROPIC_OAUTH_BETA),
        "Anthropic OAuth must send the required beta header"
    );
    // API-key header must NOT appear on the OAuth/Bearer path.
    assert!(
        req.headers.get("x-api-key").is_none(),
        "OAuth path must not send x-api-key"
    );
}

// --- Anthropic API-key path is byte-identical (no beta header, no Bearer) ---

#[tokio::test]
async fn anthropic_api_key_path_sends_xapi_key_no_bearer_no_beta() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        apikey_auth("ak-anthropic"),
    );
    drain(&mut stream).await;

    let req = one_captured_request(&server).await;
    assert_eq!(
        req.headers.get("x-api-key").map(|v| v.to_str().unwrap()),
        Some("ak-anthropic")
    );
    assert!(
        req.headers.get("authorization").is_none(),
        "API-key path must not send authorization"
    );
    assert!(
        req.headers.get("anthropic-beta").is_none(),
        "API-key path must not send the OAuth beta header"
    );
}

// --- Explicit auth-invalid policy, independent of the credential scheme ---

#[tokio::test]
async fn anthropic_oauth_401_maps_to_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid token"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()))
        .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged);
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        bearer_auth("oauth-token-anthropic"),
    );
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("a 401 yields an error");
    match err {
        ProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
}

#[tokio::test]
async fn anthropic_api_key_uses_managed_revocation_policy() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("auth error"))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()))
        .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged);
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        apikey_auth("ak-anthropic"),
    );
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    assert!(matches!(
        err,
        ProviderError::CredentialRevoked { ref provider_id } if provider_id == "anthropic"
    ));
}

#[tokio::test]
async fn anthropic_oauth_forged_401_body_does_not_leak_into_display() {
    // An enterprise proxy may echo the submitted Bearer token inside a 401
    // body. The CredentialRevoked mapping DROPS the body, so neither "Bearer"
    // nor the token can reach any Display string.
    let server = MockServer::start().await;
    let canary = "canary-bearer-token-xyz";
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(format!("error: Bearer {canary} echoed")),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()))
        .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged);
    let mut stream = provider.stream_prepared(
        sample_request("anthropic:claude-sonnet-4-5-20250514"),
        bearer_auth("oauth-token-anthropic"),
    );
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    let display = err.to_string();
    assert!(
        !display.contains("Bearer"),
        "CredentialRevoked display leaks 'Bearer': {display}"
    );
    assert!(
        !display.contains(canary),
        "CredentialRevoked display leaks the token: {display}"
    );
}

// --- Copilot (OpenAiChatProvider, managed credentials) 401 -> CredentialRevoked ---

#[tokio::test]
async fn copilot_chat_401_maps_to_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        "github-copilot".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream_prepared(
        sample_request("github-copilot:gpt-4o"),
        bearer_auth("copilot-token"),
    );
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    match err {
        ProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "github-copilot");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
}

#[tokio::test]
async fn openai_chat_api_key_uses_managed_revocation_policy() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::with_auth(
        Some(server.uri()),
        Default::default(),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream =
        provider.stream_prepared(sample_request("openai:gpt-4o"), apikey_auth("sk-openai"));
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    assert!(matches!(
        err,
        ProviderError::CredentialRevoked { ref provider_id } if provider_id == "openai"
    ));
}

#[derive(Clone, Copy, Debug)]
enum ReusableRoute {
    Anthropic,
    OpenAiChat,
    OpenAiResponses,
}

fn reusable_route(
    route: ReusableRoute,
    credential_managed: bool,
    server_uri: String,
) -> (Box<dyn Provider>, String) {
    let client = Arc::new(HttpClient::new());
    match (route, credential_managed) {
        (ReusableRoute::Anthropic, true) => (
            Box::new(
                AnthropicProvider::with_client(Some(server_uri), client)
                    .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged),
            ),
            "anthropic:claude-sonnet-4-5-20250514".into(),
        ),
        (ReusableRoute::Anthropic, false) => {
            let model = ModelInfo::new(
                "model",
                "Model",
                WireApi::AnthropicMessages,
                ModelCapabilities::new(8_192, 1_024).with_streaming(true),
            );
            (
                Box::new(AnthropicProvider::for_route(
                    "custom-anthropic".into(),
                    vec![model],
                    Some(server_uri),
                    ProviderHeaders::default(),
                    client,
                    false,
                )),
                "custom-anthropic:model".into(),
            )
        }
        (ReusableRoute::OpenAiChat, true) => (
            Box::new(OpenAiChatProvider::with_auth(
                Some(server_uri),
                Default::default(),
                "openai".into(),
                vec![],
                client,
            )),
            "openai:gpt-4o".into(),
        ),
        (ReusableRoute::OpenAiChat, false) => {
            let model = ModelInfo::new(
                "model",
                "Model",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(8_192, 1_024).with_streaming(true),
            );
            (
                Box::new(OpenAiChatProvider::for_route(
                    Some(server_uri),
                    "custom-chat".into(),
                    ProviderHeaders::default(),
                    vec![model],
                    client,
                )),
                "custom-chat:model".into(),
            )
        }
        (ReusableRoute::OpenAiResponses, true) => (
            Box::new(OpenAiResponsesProvider::with_auth(
                Some(server_uri),
                Default::default(),
                client,
            )),
            "openai-responses:gpt-4o".into(),
        ),
        (ReusableRoute::OpenAiResponses, false) => {
            let model = ModelInfo::new(
                "model",
                "Model",
                WireApi::OpenAiResponses,
                ModelCapabilities::new(8_192, 1_024).with_streaming(true),
            );
            (
                Box::new(OpenAiResponsesProvider::for_route(
                    Some(server_uri),
                    "custom-responses".into(),
                    ProviderHeaders::default(),
                    vec![model],
                    client,
                )),
                "custom-responses:model".into(),
            )
        }
    }
}

#[tokio::test]
async fn reusable_route_auth_invalid_policy_matrix_is_scheme_independent_and_bodyless() {
    const CANARY: &str = "echoed-key-canary-must-not-surface";
    for route in [
        ReusableRoute::Anthropic,
        ReusableRoute::OpenAiChat,
        ReusableRoute::OpenAiResponses,
    ] {
        for credential_managed in [true, false] {
            for status in [401, 403] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(ResponseTemplate::new(status).set_body_string(CANARY))
                    .mount(&server)
                    .await;
                let (provider, model) = reusable_route(route, credential_managed, server.uri());
                let error = provider
                    .stream_prepared(
                        sample_request(&model),
                        opi_ai::test_support::resolved_auth(),
                    )
                    .next()
                    .await
                    .expect("auth-invalid error")
                    .expect_err("401/403 must fail");

                if credential_managed {
                    assert!(
                        matches!(error, ProviderError::CredentialRevoked { .. }),
                        "{route:?} HTTP {status}: {error:?}"
                    );
                } else {
                    assert!(
                        matches!(error, ProviderError::AuthFailed(_)),
                        "{route:?} HTTP {status}: {error:?}"
                    );
                }
                let rendered = format!("{error:?} {error}");
                assert!(
                    !rendered.contains(CANARY),
                    "{route:?} HTTP {status} leaked the response body: {rendered}"
                );
                assert!(!error.is_retryable());
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum StaticRoute {
    Anthropic,
    OpenAiChat,
    OpenAiResponses,
    OpenRouter,
    Mistral,
}

fn static_route(route: StaticRoute, server_uri: String) -> (Box<dyn Provider>, &'static str) {
    match route {
        StaticRoute::Anthropic => (
            Box::new(AnthropicProvider::new(Some(server_uri))),
            "anthropic:claude-sonnet-4-5-20250514",
        ),
        StaticRoute::OpenAiChat => (
            Box::new(OpenAiChatProvider::new(Some(server_uri))),
            "openai:gpt-4o",
        ),
        StaticRoute::OpenAiResponses => (
            Box::new(OpenAiResponsesProvider::new(Some(server_uri))),
            "openai-responses:gpt-4o",
        ),
        StaticRoute::OpenRouter => (
            Box::new(openrouter_provider(Some(server_uri))),
            "openrouter:openai/gpt-4o",
        ),
        StaticRoute::Mistral => (
            Box::new(mistral_provider(Some(server_uri))),
            "mistral:mistral-large-latest",
        ),
    }
}

#[tokio::test]
async fn static_route_auth_errors_are_bodyless_for_all_profiles_and_schemes() {
    const CANARY: &str = "static-auth-response-canary-must-not-surface";
    for route in [
        StaticRoute::Anthropic,
        StaticRoute::OpenAiChat,
        StaticRoute::OpenAiResponses,
        StaticRoute::OpenRouter,
        StaticRoute::Mistral,
    ] {
        for status in [401, 403] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status).set_body_string(CANARY))
                .mount(&server)
                .await;
            let (provider, model) = static_route(route, server.uri());
            let error = provider
                .stream_prepared(sample_request(model), opi_ai::test_support::resolved_auth())
                .next()
                .await
                .expect("auth-invalid error")
                .expect_err("401/403 must fail");

            assert!(
                matches!(&error, ProviderError::AuthFailed(message) if message == "authentication failed"),
                "{route:?} HTTP {status}: {error:?}"
            );
            let rendered = format!("{error:?} {error}");
            assert!(
                !rendered.contains(CANARY),
                "{route:?} HTTP {status} leaked the response body: {rendered}"
            );
            assert!(!error.is_retryable());
        }
    }
}

// --- Dedicated Codex Responses wire shape ---

fn codex_provider(server: &MockServer) -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![ModelInfo::new(
            "gpt-5",
            "GPT-5",
            WireApi::OpenAiCodexResponses,
            ModelCapabilities::new(128_000, 16_384),
        )],
        Arc::new(HttpClient::new()),
    )
}

/// Build the Bearer auth the dedicated Codex wire consumes, optionally carrying
/// the account id derived from the bearer JWT.
fn codex_auth(secret: &str, account_id: Option<&str>) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(secret),
        base_url: None,
        account_id: account_id.map(str::to_owned),
        provenance: AuthProvenance::default(),
    }
}

#[tokio::test]
async fn codex_responses_targets_codex_path_with_required_headers_and_account_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = codex_provider(&server);
    let resolved = codex_auth("codex-bearer", Some("acct-fixed"));
    let mut stream = provider.stream_prepared(sample_request("openai-codex:gpt-5"), resolved);
    drain(&mut stream).await;

    let req = one_captured_request(&server).await;
    // Exact endpoint path — NOT /v1/responses.
    assert_eq!(req.url.path(), "/codex/responses");
    // Static required headers.
    assert_eq!(
        req.headers.get("OpenAI-Beta").map(|v| v.to_str().unwrap()),
        Some("responses=experimental")
    );
    assert_eq!(
        req.headers.get("originator").map(|v| v.to_str().unwrap()),
        Some("opi")
    );
    assert_eq!(
        req.headers.get("accept").map(|v| v.to_str().unwrap()),
        Some("text/event-stream")
    );
    // Per-request account id derived from the bearer JWT.
    assert_eq!(
        req.headers
            .get("chatgpt-account-id")
            .map(|v| v.to_str().unwrap()),
        Some("acct-fixed")
    );
}

#[tokio::test]
async fn codex_missing_account_id_is_rejected_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = codex_provider(&server);
    let resolved = codex_auth("opaque-not-a-jwt", None);
    let mut stream = provider.stream_prepared(sample_request("openai-codex:gpt-5"), resolved);
    assert!(
        matches!(
            stream.next().await,
            Some(Err(ProviderError::AccountIdMissing { ref provider_id }))
                if provider_id == "openai-codex"
        ),
        "missing account id must fail before HTTP"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn codex_responses_401_maps_to_credential_revoked_without_body_leak() {
    let server = MockServer::start().await;
    // The 401 body echoes the submitted Bearer (proxy token-echo scenario).
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("error: Bearer codex-bearer-xyz echoed"),
        )
        .mount(&server)
        .await;

    let provider = codex_provider(&server);
    let resolved = codex_auth("codex-bearer-xyz", Some("acct"));
    let mut stream = provider.stream_prepared(sample_request("openai-codex:gpt-5"), resolved);
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    match &err {
        ProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "openai-codex");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    let display = err.to_string();
    assert!(
        !display.contains("Bearer"),
        "CredentialRevoked display leaks 'Bearer': {display}"
    );
    assert!(
        !display.contains("codex-bearer-xyz"),
        "CredentialRevoked display leaks the token: {display}"
    );
}

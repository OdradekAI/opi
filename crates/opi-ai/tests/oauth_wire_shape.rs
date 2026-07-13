//! Phase 14.2 slice 5 — exact OAuth wire shape per provider mapping.
//!
//! Where `per_request_auth.rs` (slice 2) proves each provider resolves an
//! injected `Arc<dyn AuthResolver>` and attaches the scheme-selected header,
//! these tests pin the EXACT OAuth wire contract the factory-built providers
//! must emit — not merely "a Bearer token reached the wire":
//!
//! - Anthropic OAuth selects `authorization: Bearer` AND the required
//!   `anthropic-beta: oauth-2025-04-20` header, while API-key construction
//!   keeps `x-api-key` and emits neither `authorization` nor the beta header.
//! - A 401 on a Bearer (OAuth) credential maps to typed non-retryable
//!   `ProviderError::CredentialRevoked`, dropping the body so an enterprise
//!   proxy echoing the submitted Bearer cannot leak it; API-key 401 stays
//!   `AuthFailed`.
//!
//! opi-ai tests use `StaticAuthResolver` (opi-ai cannot depend on
//! opi-coding-agent's `AuthSource`); the factory + `AuthSource` + fake-store
//! coverage lives in opi-coding-agent/tests.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{AuthResolver, AuthScheme, StaticAuthResolver};
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The Anthropic OAuth beta tag. Pinned as a literal here (not re-exported from
/// the provider, which keeps it a private module const) so a future rotation in
/// `anthropic.rs` is caught at this assertion. RESIDUAL: the value itself was
/// not confirmed against a public Anthropic source at slice-5 landing time; it
/// must be re-confirmed against the pi source before a production login is
/// advertised.
const ANTHROPIC_OAUTH_BETA: &str = "oauth-2025-04-20";

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

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("oauth-token-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
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

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from("ak-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
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

// --- Revocation: Bearer 401 -> typed CredentialRevoked; ApiKey 401 unchanged ---

#[tokio::test]
async fn anthropic_oauth_401_maps_to_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid token"}}"#,
        ))
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("oauth-token-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
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
async fn anthropic_api_key_401_stays_authfailed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("auth error"))
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from("ak-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    assert!(
        matches!(err, ProviderError::AuthFailed(_)),
        "API-key 401 must stay AuthFailed, got {err:?}"
    );
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

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("oauth-token-anthropic"),
    ));
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(sample_request("anthropic:claude-sonnet-4-5-20250514"));
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

// --- Copilot (OpenAiChatProvider, Bearer) 401 -> CredentialRevoked ---
// API-key OpenAI profiles keep AuthFailed (scheme==ApiKey).

#[tokio::test]
async fn copilot_chat_401_maps_to_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("copilot-token"),
    ));
    let provider = OpenAiChatProvider::with_auth(
        resolver,
        Some(server.uri()),
        Default::default(),
        "copilot".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("copilot:gpt-4o"));
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    match err {
        ProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "copilot");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
}

#[tokio::test]
async fn openai_chat_api_key_401_stays_authfailed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from("sk-openai"),
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
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    assert!(
        matches!(err, ProviderError::AuthFailed(_)),
        "API-key OpenAI 401 must stay AuthFailed, got {err:?}"
    );
}

// --- Codex (OpenAiResponsesProvider, Codex profile) wire shape ---

/// Forge a JWT-shaped bearer carrying `chatgpt_account_id` under the Codex auth
/// claim. Signature segment is irrelevant — the provider does not verify it.
fn codex_jwt(account_id: &str) -> String {
    use base64::Engine;
    let header = br#"{"alg":"HS256","typ":"JWT"}"#;
    let payload =
        format!(r#"{{"https://api.openai.com/auth":{{"chatgpt_account_id":"{account_id}"}}}}"#);
    let h = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
    let p = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    format!("{h}.{p}.sig")
}

/// The static header set the Codex compatibility profile attaches.
fn codex_static_headers() -> Vec<(String, String)> {
    vec![
        ("OpenAI-Beta".into(), "responses=experimental".into()),
        ("originator".into(), "opi".into()),
        ("accept".into(), "text/event-stream".into()),
    ]
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

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from(codex_jwt("acct-fixed")),
    ));
    let config = ResponsesConfig {
        responses_path: "/codex/responses".into(),
        derive_codex_account_id: true,
        ..Default::default()
    };
    let provider = OpenAiResponsesProvider::with_auth_extra(
        resolver,
        Some(server.uri()),
        config,
        "codex".into(),
        codex_static_headers(),
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("codex:gpt-5"));
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
async fn codex_account_id_invalid_jwt_omits_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    // An opaque (non-JWT) token: derivation must return None and omit the
    // header, never formatting the token into a header or error.
    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("opaque-not-a-jwt"),
    ));
    let config = ResponsesConfig {
        derive_codex_account_id: true,
        ..Default::default()
    };
    let provider = OpenAiResponsesProvider::with_auth_extra(
        resolver,
        Some(server.uri()),
        config,
        "codex".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("codex:gpt-5"));
    drain(&mut stream).await;

    let req = one_captured_request(&server).await;
    assert!(
        req.headers.get("chatgpt-account-id").is_none(),
        "opaque token must not produce a chatgpt-account-id header"
    );
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

    let resolver: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("codex-bearer-xyz"),
    ));
    let config = ResponsesConfig {
        responses_path: "/codex/responses".into(),
        ..Default::default()
    };
    let provider = OpenAiResponsesProvider::with_auth_extra(
        resolver,
        Some(server.uri()),
        config,
        "codex".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut stream = provider.stream(sample_request("codex:gpt-5"));
    let err = stream
        .next()
        .await
        .expect("an event")
        .expect_err("401 yields an error");
    match &err {
        ProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "codex");
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

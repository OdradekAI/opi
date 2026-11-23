//! GitHub Copilot OAuth flow fixtures and acceptance tests.
//!
//! This target owns device login, token exchange, model discovery, enterprise
//! URL derivation, policy activation, refresh and Models store ACK. Shared
//! presenters, assembly assertions and TLS material live in `tests/support`.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::Models;
use opi_ai::auth::{OAuthCredential, OAuthProvider};
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "support/oauth_assembly.rs"]
#[allow(dead_code)]
mod oauth_assembly;
#[path = "support/oauth_copilot.rs"]
#[allow(dead_code)]
mod oauth_copilot;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;
#[path = "support/tls_fixtures.rs"]
#[allow(dead_code)]
mod oauth_tls_fixtures;

use oauth_assembly::{
    assert_oauth_identity, oauth_assembly_request, register_oauth_assembly_route,
};
use oauth_copilot::*;
use oauth_fixtures::ScriptedPresenter;

#[tokio::test]
async fn copilot_login_persists_tokens_models_and_real_expiry() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[("gpt-5-codex", true, "enabled", true)]),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("copilot login");

    // The device start carries the pinned client id + read:user scope.
    let requests = server.received_requests().await.unwrap();
    let device = requests
        .iter()
        .find(|request| request.url.path() == "/login/device/code")
        .expect("device start request");
    let body = String::from_utf8_lossy(&device.body).into_owned();
    assert!(body.contains(&format!("client_id={COPILOT_CLIENT_ID}")));
    assert!(body.contains(&format!(
        "scope={}",
        COPILOT_DEVICE_SCOPE.replace(':', "%3A")
    )));
    // The public user_code is presented, never the device_code.
    let device_codes = presenter.device_codes.lock().unwrap().clone();
    assert_eq!(device_codes.len(), 1);
    assert_eq!(device_codes[0].0, "ABCD-1234");
    assert_eq!(device_codes[0].1, format!("{}/login/device", server.uri()));
    // The exchange carries the GitHub token; models carry the copilot token
    // and the pinned API version header.
    let exchange = requests
        .iter()
        .find(|request| request.url.path() == "/copilot_internal/v2/token")
        .expect("token exchange");
    assert_eq!(
        exchange
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer gh-access-1")
    );
    let models = requests
        .iter()
        .find(|request| request.url.path() == "/models")
        .expect("models request");
    assert_eq!(
        models
            .headers
            .get("x-github-api-version")
            .and_then(|value| value.to_str().ok()),
        Some("2026-06-01")
    );
    // Credential: GitHub token is the refresh grant, copilot token the
    // bearer, real service expiry, discovered model ids, no policy requests
    // (the whitelist is empty).
    assert_eq!(credential.refresh.expose_secret(), "gh-access-1");
    assert!(credential.access.expose_secret().contains("proxy-ep="));
    // The stored expiry is the response's absolute service expiry, roughly
    // one hour from when the mock body was built — not a freshly derived
    // now+N.
    let expiry = credential.expires_at.expect("expiry");
    let until = (expiry - time::OffsetDateTime::now_utc()).whole_seconds();
    assert!(
        (3590..=3610).contains(&until),
        "real service expiry, {until}s remaining"
    );
    assert_eq!(
        credential.available_model_ids.as_deref(),
        Some(&["gpt-5-codex".to_owned()][..])
    );
    assert!(credential.base_url.is_none());
    assert!(credential.enterprise_domain.is_none());
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().contains("/policy")),
        "no policy activation without a whitelist"
    );
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn copilot_models_login_acks_the_store_write() {
    let server = MockServer::start().await;
    let (inference_endpoint, inference_client, inference_task) = copilot_https_target().await;
    let token = format!(
        "tid=selected;proxy-ep={};sch=sha256",
        inference_endpoint.strip_prefix("https://").unwrap()
    );
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": token, "expires_at": time::OffsetDateTime::now_utc().unix_timestamp() + 3600,
        })))
        .mount(&server)
        .await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[("auth-proof", true, "enabled", true)]),
    )
    .await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    let uri = server.uri();
    let models_uri = uri.clone();
    let flow = GitHubCopilotOAuthProvider::new_with_endpoints(
        CopilotUrls {
            device_code_url: format!("{uri}/login/device/code"),
            access_token_url: format!("{uri}/login/oauth/access_token"),
            copilot_token_url: format!("{uri}/copilot_internal/v2/token"),
        },
        Arc::new(move |_| panic!("no enterprise selection in this local proof")),
        Arc::new(move |_, _| models_uri.clone()),
        Vec::new(),
        Duration::from_secs(30),
        reqwest::Client::new(),
    );
    register_oauth_assembly_route(
        &mut models,
        Arc::new(flow),
        opi_ai::WireApi::OpenAiCompletions,
        "http://127.0.0.1:1".into(),
        inference_client,
    );
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("github-copilot", presenter.as_ref())
        .await
        .expect("login");
    let resolved = assert_oauth_identity(&models, "github-copilot", &credential).await;
    assert_eq!(
        resolved.base_url.as_deref(),
        Some(inference_endpoint.as_str())
    );
    assert_eq!(
        credential.available_model_ids.as_deref(),
        Some(["auth-proof".to_owned()].as_slice())
    );
    let call = models
        .prepare_call(
            "github-copilot:auth-proof",
            oauth_assembly_request("github-copilot"),
        )
        .await
        .expect("selected Copilot request");
    opi_ai::models::drain_to_completion(call.start_attempt().unwrap())
        .await
        .expect("local Copilot adapter completed");
    let actual = inference_task.await.expect("local TLS request");
    assert!(actual.starts_with("POST /chat/completions HTTP/1.1\r\n"));
    assert!(actual.contains(&format!("authorization: Bearer {token}\r\n")));
    assert!(actual.to_ascii_lowercase().contains("x-initiator:"));
    assert!(!actual.to_ascii_lowercase().contains("x-api-key:"));

    let stored = store
        .read("github-copilot")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken {
            refresh,
            available_model_ids,
            ..
        } => {
            assert_eq!(refresh.expose_secret(), credential.refresh.expose_secret());
            assert_eq!(available_model_ids, credential.available_model_ids);
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn copilot_public_available_models_use_acknowledged_account_allowlist() {
    for account_models in [vec!["auth-proof"], vec![]] {
        let server = MockServer::start().await;
        mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
        mount_copilot_poll_complete(&server).await;
        mount_copilot_exchange(&server).await;
        let entries: Vec<_> = account_models
            .iter()
            .map(|id| (*id, true, "enabled", true))
            .collect();
        mount_copilot_models(&server, copilot_models_body(&entries)).await;
        let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
        let mut models = Models::new();
        models.set_credential_store(store.clone());
        let catalog = ["auth-proof", "not-authorized"]
            .map(|id| {
                opi_ai::ModelInfo::new(
                    id,
                    id,
                    opi_ai::WireApi::OpenAiCompletions,
                    opi_ai::ModelCapabilities::new(16_384, 1024).with_streaming(true),
                )
            })
            .to_vec();
        let provider = opi_ai::providers::openai::OpenAiChatProvider::for_route(
            Some("http://127.0.0.1:1".into()),
            "github-copilot".into(),
            Default::default(),
            catalog,
            Arc::new(opi_ai::http::HttpClient::new()),
        );
        models
            .register_oauth_route(
                Box::new(provider),
                Arc::new(copilot_flow(
                    &server.uri(),
                    Vec::new(),
                    Duration::from_secs(30),
                )),
                Default::default(),
            )
            .unwrap();
        let credential = models
            .login("github-copilot", &ScriptedPresenter::default())
            .await
            .expect("real Copilot login ACK");
        assert_eq!(
            credential.available_model_ids,
            Some(account_models.iter().map(|id| (*id).to_owned()).collect())
        );
        let available = models
            .get_available(Some("github-copilot"))
            .await
            .expect("account models");
        let actual: Vec<_> = available
            .iter()
            .map(|(_, model)| model.id.as_str())
            .collect();
        assert_eq!(
            actual, account_models,
            "the account allowlist is authoritative, including an empty list"
        );
        let mut legacy = credential;
        legacy.available_model_ids = None;
        store
            .modify("github-copilot", opi_ai::write_mutation(legacy.into()))
            .await
            .expect("legacy envelope ACK");
        let unfiltered = models.get_available(Some("github-copilot")).await.unwrap();
        assert_eq!(
            unfiltered
                .iter()
                .map(|(_, model)| model.id.as_str())
                .collect::<Vec<_>>(),
            ["auth-proof", "not-authorized"],
            "missing legacy metadata preserves the registered catalog"
        );
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| request.url.path() == "/models")
                .count(),
            1,
            "availability inspection must not refresh the account catalog"
        );
    }
}

#[tokio::test]
async fn copilot_enterprise_domain_prompt_normalizes_and_derives_urls() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("POST"))
        .and(path("/ghe.test/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc-secret-1",
            "user_code": "ABCD-1234",
            "verification_uri": format!("{uri}/login/device"),
            "interval": 0,
            "expires_in": 900,
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ghe.test/login/oauth/access_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "access_denied"})),
        )
        .mount(&server)
        .await;
    // Enterprise derivation: `https://{domain}/...` paths become
    // `{domain}/...` on the offline server.
    let flow = GitHubCopilotOAuthProvider::new_with_endpoints(
        CopilotUrls {
            device_code_url: format!("{uri}/login/device/code"),
            access_token_url: format!("{uri}/login/oauth/access_token"),
            copilot_token_url: format!("{uri}/copilot_internal/v2/token"),
        },
        {
            let uri = uri.clone();
            Arc::new(move |domain: &str| CopilotUrls {
                device_code_url: format!("{uri}/{domain}/login/device/code"),
                access_token_url: format!("{uri}/{domain}/login/oauth/access_token"),
                copilot_token_url: format!("{uri}/{domain}/copilot_internal/v2/token"),
            })
        },
        {
            let models_uri = uri.clone();
            Arc::new(move |_token: &str, _domain: Option<&str>| models_uri.clone())
        },
        Vec::new(),
        Duration::from_secs(30),
        reqwest::Client::new(),
    );
    let presenter = copilot_presenter_with("https://ghe.test/orgs/acme");

    // denied ends the flow before the exchange; the enterprise device start
    // already proves the URL derivation.
    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("denied");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .any(|request| request.url.path() == "/ghe.test/login/device/code"),
        "enterprise domain must derive its own device endpoint"
    );
}

#[tokio::test]
async fn copilot_invalid_enterprise_domain_is_typed_before_any_traffic() {
    let server = MockServer::start().await;
    let presenter = copilot_presenter_with("not a domain!");
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("invalid domain");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn copilot_untrusted_verification_uri_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc-secret-1",
            "user_code": "ABCD-1234",
            "verification_uri": "file:///etc/passwd",
            "interval": 0,
            "expires_in": 900,
        })))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("untrusted verification uri");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(
        presenter.device_codes.lock().unwrap().is_empty(),
        "the user code is never presented for an untrusted verification URI"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path() == "/login/oauth/access_token")
    );
}

#[tokio::test]
async fn copilot_denied_poll_is_typed_terminal_without_exchange() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "access_denied"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("denied");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"device authorization denied".to_owned())
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().starts_with("/copilot_internal"))
    );
}

#[tokio::test]
async fn copilot_expired_poll_is_typed_terminal() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "expired_token"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("expired");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"device code expired".to_owned())
    );
}

#[tokio::test]
async fn copilot_unknown_poll_error_is_closed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "unauthorized_client"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("unknown error code");
    assert!(matches!(error, ProviderError::AuthFailed(_)));
    assert!(!matches!(error, ProviderError::CredentialRevoked { .. }));
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().starts_with("/copilot_internal"))
    );
}

#[tokio::test]
async fn copilot_slow_down_uses_server_interval_to_keep_polling() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    // Every poll answers slow_down with server interval 1s: without the
    // server interval the RFC +5s default would allow only one poll inside
    // the 3s budget.
    mount_copilot_poll(
        &server,
        serde_json::json!({"error": "slow_down", "interval": 1}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(3));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("budget bounds the slow_down loop");
    assert!(matches!(error, ProviderError::Timeout));
    let poll_count = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/login/oauth/access_token")
        .count();
    assert!(
        poll_count >= 2,
        "server interval must keep polling: {poll_count}"
    );
}

#[tokio::test]
async fn copilot_token_exchange_401_is_credential_revoked() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("revoked"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("revoked grant");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"token exchange failed".to_owned())
    );
}

#[tokio::test]
async fn copilot_token_response_missing_fields_is_typed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com",
        })))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("missing expires_at");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(
        !server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|request| request.url.path() == "/models")
    );
}

#[tokio::test]
async fn copilot_models_429_retries_then_succeeds() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    // First models response is a 429 with Retry-After: 0; the login policy
    // retries and the second response completes discovery. `up_to_n_times`
    // expires the 429 mock after one hit so the base mock serves the retry.
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("rate limited"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_copilot_models(&server, copilot_models_body(&[])).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("retry recovers");
    assert_eq!(credential.available_model_ids.as_deref(), Some(&[][..]));
    let models_requests = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 2);
}

#[tokio::test]
async fn copilot_models_429_exhaustion_is_typed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("rate limited"),
        )
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("exhausted retries");
    assert!(matches!(error, ProviderError::AuthFailed(_)));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"models discovery failed".to_owned())
    );
    let models_requests = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 3, "login policy is 2 retries + 1 attempt");
}

#[tokio::test]
async fn copilot_policy_activation_is_login_only_and_progress_notified() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[
            ("gpt-5-codex", true, "enabled", true),
            ("claude-sonnet-4-5", true, "unconfigured", true),
            ("gpt-5-mini", true, "enabled", false),
        ]),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/models/claude-sonnet-4-5/policy"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let whitelist = vec![
        "claude-sonnet-4-5".to_owned(),
        "gpt-5-mini".to_owned(),
        "unknown-model".to_owned(),
    ];

    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    let flow = copilot_flow(&server.uri(), whitelist, Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("login with activation");

    assert_eq!(
        presenter.progress.lock().unwrap().as_slice(),
        ["Enabling models...".to_owned()].as_slice()
    );
    // tool_calls=false is filtered from the catalog; the enabled model and
    // the one enabled-by-login policy model are available.
    assert_eq!(
        credential.available_model_ids.as_deref(),
        Some(&["gpt-5-codex".to_owned(), "claude-sonnet-4-5".to_owned()][..])
    );
    let requests = server.received_requests().await.unwrap();
    let policy = requests
        .iter()
        .find(|request| request.url.path() == "/models/claude-sonnet-4-5/policy")
        .expect("policy activation request");
    assert_eq!(
        String::from_utf8_lossy(&policy.body),
        r#"{"state":"enabled"}"#
    );
    assert_eq!(
        policy
            .headers
            .get("openai-intent")
            .and_then(|value| value.to_str().ok()),
        Some("chat-policy")
    );
}

#[tokio::test]
async fn copilot_refresh_reexchanges_rediscovers_and_never_enables_policy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ghe.test/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(copilot_token_body()))
        .mount(&server)
        .await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[("claude-sonnet-4-5", true, "unconfigured", true)]),
    )
    .await;
    let flow = copilot_flow(
        &server.uri(),
        vec!["claude-sonnet-4-5".to_owned()],
        Duration::from_secs(30),
    );
    let cred = OAuthCredential {
        access: secrecy::SecretString::from("old-copilot-token"),
        refresh: secrecy::SecretString::from("gh-access-1"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: Some("ghe.test".to_owned()),
        available_model_ids: Some(vec!["stale-model".to_owned()]),
    };

    let refreshed = OAuthProvider::refresh(&flow, &cred).await.expect("refresh");

    assert_eq!(refreshed.refresh.expose_secret(), "gh-access-1");
    assert!(refreshed.access.expose_secret().contains("proxy-ep="));
    let expiry = refreshed.expires_at.expect("expiry");
    let until = (expiry - time::OffsetDateTime::now_utc()).whole_seconds();
    assert!(
        (3590..=3610).contains(&until),
        "real service expiry, {until}s remaining"
    );
    assert_eq!(refreshed.enterprise_domain.as_deref(), Some("ghe.test"));
    // The picker-on unconfigured model is still picker-listed after
    // discovery, but nothing was enabled: no policy request ever fired and
    // the catalog reflects exactly the strict picker semantics.
    assert_eq!(
        refreshed.available_model_ids.as_deref(),
        Some(&["claude-sonnet-4-5".to_owned()][..]),
        "refresh re-discovers picker availability without enabling anything"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().contains("/policy")),
        "refresh must not activate policies"
    );
    // The refresh-time discovery is a single attempt (policy 0/0).
    let models_requests = requests
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 1);
    // The exchange used the enterprise endpoint derivation.
    assert!(
        requests
            .iter()
            .any(|request| request.url.path() == "/ghe.test/copilot_internal/v2/token"),
        "refresh must derive the enterprise exchange endpoint"
    );
}

#[tokio::test]
async fn copilot_refresh_401_is_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("revoked"))
        .mount(&server)
        .await;
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));
    let cred = OAuthCredential {
        access: secrecy::SecretString::from("old-copilot-token"),
        refresh: secrecy::SecretString::from("gh-access-1"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    };

    let error = OAuthProvider::refresh(&flow, &cred)
        .await
        .expect_err("revoked");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
}

// Pure-contract units (no HTTP): domain normalization, URL derivation and
// the proxy-endpoint base URL precedence.

#[test]
fn copilot_normalize_domain_variants() {
    assert_eq!(copilot::normalize_enterprise_domain(""), None);
    assert_eq!(
        copilot::normalize_enterprise_domain("company.ghe.com"),
        Some("company.ghe.com".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("  https://company.ghe.com/orgs/acme  "),
        Some("company.ghe.com".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("http://localhost:1455/x"),
        Some("localhost".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("user@gh.example.com"),
        Some("gh.example.com".to_owned())
    );
    assert_eq!(copilot::normalize_enterprise_domain("not a domain!"), None);
    assert_eq!(copilot::normalize_enterprise_domain("https://"), None);
}

#[test]
fn copilot_urls_for_domain_shape() {
    let urls = copilot::copilot_urls_for_domain(COPILOT_PUBLIC_DOMAIN);
    assert_eq!(urls.device_code_url, "https://github.com/login/device/code");
    assert_eq!(
        urls.access_token_url,
        "https://github.com/login/oauth/access_token"
    );
    assert_eq!(
        urls.copilot_token_url,
        "https://api.github.com/copilot_internal/v2/token"
    );
}

#[test]
fn copilot_base_url_precedence() {
    let token = "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com;sch=sha256";
    assert_eq!(
        copilot::base_url_from_token(token).as_deref(),
        Some(COPILOT_DEFAULT_BASE_URL)
    );
    // A proxy endpoint without the `proxy.` prefix is used as-is.
    assert_eq!(
        copilot::base_url_from_token("proxy-ep=enterprise.example").as_deref(),
        Some("https://enterprise.example".to_owned()).as_deref()
    );
    assert_eq!(copilot::base_url_from_token("no-hint"), None);
    assert_eq!(
        copilot::request_base_url(Some(token), None),
        COPILOT_DEFAULT_BASE_URL
    );
    assert_eq!(
        copilot::request_base_url(Some(token), Some("ghe.test")),
        COPILOT_DEFAULT_BASE_URL,
        "the token proxy endpoint wins over the enterprise domain"
    );
    assert_eq!(
        copilot::request_base_url(None, Some("ghe.test")),
        "https://copilot-api.ghe.test"
    );
    assert_eq!(
        copilot::request_base_url(None, None),
        COPILOT_DEFAULT_BASE_URL
    );
}

#[test]
fn copilot_models_catalog_parsing_variants() {
    let whitelist = ["known-model".to_owned()];

    // Strict base: only picker-enabled non-disabled models survive; a
    // picker-on unconfigured model stays picker-available (activation is a
    // separate login action); the disabled and no-tools models drop.
    let strict_raw = copilot_models_body(&[
        ("picker-on", true, "enabled", true),
        ("picker-on-unconfigured", true, "unconfigured", true),
        ("picker-disabled", true, "disabled", true),
        ("no-tools", true, "enabled", false),
        ("known-model", false, "unconfigured", true),
    ]);
    let strict = copilot::parse_copilot_models_catalog(&strict_raw, false, &whitelist)
        .expect("strict catalog");
    assert_eq!(
        strict.available_model_ids,
        ["picker-on".to_owned(), "picker-on-unconfigured".to_owned()]
    );
    // Without the picker fallback, an unconfigured whitelisted model with a
    // false picker flag is not offered for activation.
    assert!(strict.policy_model_ids.is_empty());

    // Individual endpoint with every picker flag false: the enabled-policy
    // fallback applies, and the unconfigured whitelisted model becomes
    // eligible for login-time activation.
    let fallback_raw = copilot_models_body(&[
        ("enabled-one", false, "enabled", true),
        ("known-model", false, "unconfigured", true),
        ("disabled-one", false, "disabled", true),
    ]);
    let fallback = copilot::parse_copilot_models_catalog(&fallback_raw, true, &whitelist)
        .expect("fallback catalog");
    assert_eq!(fallback.available_model_ids, ["enabled-one".to_owned()]);
    assert_eq!(fallback.policy_model_ids, ["known-model".to_owned()]);

    // No `data` array is a typed failure.
    let invalid = serde_json::json!({"error": "oops"});
    assert!(copilot::parse_copilot_models_catalog(&invalid, false, &whitelist).is_err());
}

// ---------------------------------------------------------------------------
#[tokio::test]
async fn regression_copilot_source_clamps_before_float_duration_conversion() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "copilot-device",
            "user_code": "COPILOT-CODE",
            "verification_uri": "https://github.com/login/device",
            "interval": 1e300,
            "expires_in": 1e300
        })))
        .mount(&server)
        .await;
    let flow = copilot_flow(&server.uri(), vec![], Duration::from_millis(50));
    let presenter = ScriptedPresenter::default();
    let result = flow.login(&presenter).await;
    assert!(
        matches!(result, Err(ProviderError::Timeout)),
        "source clamps remain in force under the caller budget: {result:?}"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| request.url.path() != "/login/oauth/access_token")
    );
}

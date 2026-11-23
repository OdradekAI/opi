//! OpenRouter OAuth flow fixtures and acceptance tests.
//!
//! This target owns the OpenRouter callback, manual-code, exchange, and
//! permanent-key lifecycle. Shared presenter and Models assembly fixtures are
//! imported from `tests/support`; shared cross-provider regression lives in
//! `oauth_regressions`.

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
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;

use oauth_assembly::{
    assert_oauth_identity, oauth_assembly_request, oauth_assembly_sse,
    register_oauth_assembly_route,
};
use oauth_fixtures::ScriptedPresenter;

// ---------------------------------------------------------------------------
// OpenRouter OAuth callback and permanent-key lifecycle
// ---------------------------------------------------------------------------

use opi_ai::auth::oauth::openrouter::{
    OPENROUTER_AUTHORIZE_URL, OPENROUTER_CALLBACK_PATH_PREFIX, OPENROUTER_PERMANENT_KEY_UNIX_TS,
    OPENROUTER_REDIRECT_HOST, OPENROUTER_TOKEN_URL, OpenRouterOAuthProvider,
    openrouter_permanent_key_expiry,
};

fn openrouter_flow(token_url: String) -> OpenRouterOAuthProvider {
    OpenRouterOAuthProvider::new(
        OPENROUTER_AUTHORIZE_URL.to_owned(),
        token_url,
        Duration::from_secs(10),
    )
}

/// Wait for (and return) the flow's authorize URL.
async fn wait_for_authorize_url(presenter: &ScriptedPresenter) -> String {
    let mut url = None;
    for _ in 0..500 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    url.expect("authorize url never presented")
}

async fn mount_openrouter_key(server: &MockServer, status: u16, body: String) {
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/keys"))
        .respond_with(
            ResponseTemplate::new(status)
                .insert_header("Content-Type", "application/json")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

async fn assert_openrouter_adapter_request(
    models: &Models,
    credential: &OAuthCredential,
    server: &MockServer,
) {
    let _ = assert_oauth_identity(models, "openrouter", credential).await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::header(
            "authorization",
            format!("Bearer {}", credential.access.expose_secret()),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(oauth_assembly_sse(opi_ai::WireApi::OpenAiCompletions)),
        )
        .expect(1)
        .mount(server)
        .await;
    let call = models
        .prepare_call(
            "openrouter:auth-proof",
            oauth_assembly_request("openrouter"),
        )
        .await
        .expect("prepare selected identity");
    opi_ai::models::drain_to_completion(call.start_attempt().expect("adapter attempt"))
        .await
        .expect("local adapter completed");
    server.verify().await;
    let request = server
        .received_requests()
        .await
        .expect("requests")
        .into_iter()
        .find(|request| request.url.path() == "/v1/chat/completions")
        .expect("actual adapter request");
    assert!(!request.headers.contains_key("x-api-key"));
}

/// Wait for the authorize URL, then drive the loopback callback with a code.
async fn drive_openrouter_callback(presenter: &ScriptedPresenter, code: &str) -> u16 {
    let mut url = None;
    for _ in 0..500 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!("{callback}?code={code}"))
        .await
        .expect("callback request");
    response.status().as_u16()
}

#[tokio::test]
async fn openrouter_login_exchanges_code_for_permanent_api_key() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"sk-or-v1-permanent"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    assert_eq!(drive_openrouter_callback(&presenter, "cb-1").await, 200);
    let credential = login.await.expect("join").expect("login failed");

    // Authorize URL shape: callback_url + S256 PKCE, nothing else.
    let url = presenter.last_authorize_url();
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    assert!(callback.starts_with("http://127.0.0.1:"));
    assert!(callback.contains(OPENROUTER_CALLBACK_PATH_PREFIX));
    let challenge = ScriptedPresenter::query_param(&url, "code_challenge").unwrap();
    assert_eq!(challenge.len(), 43);
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    for absent in [
        "response_type",
        "client_id",
        "redirect_uri",
        "state",
        "scope",
    ] {
        assert!(
            ScriptedPresenter::query_param(&url, absent).is_none(),
            "authorize URL must not carry {absent}"
        );
    }
    // Listening progress names the bound redirect URI.
    assert!(
        presenter
            .progress
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.starts_with("Listening for OpenRouter OAuth callback on http://"))
    );

    // Exchange: exactly one JSON POST with the pinned body shape.
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "POST");
    assert_eq!(
        requests[0]
            .headers
            .get("accept")
            .and_then(|v| v.to_str().ok()),
        Some("application/json")
    );
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "cb-1");
    assert_eq!(body["code_challenge_method"], "S256");
    let verifier = body["code_verifier"].as_str().unwrap();
    assert_eq!(verifier.len(), 43);

    // Credential: permanent key, empty refresh slot, sentinel expiry.
    assert_eq!(credential.access.expose_secret(), "sk-or-v1-permanent");
    assert_eq!(credential.refresh.expose_secret(), "");
    assert_eq!(
        credential.expires_at,
        Some(openrouter_permanent_key_expiry())
    );
    assert!(!credential.needs_refresh());
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(presenter.failures.lock().unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_callback_path_is_random_per_login() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));

    let mut paths = Vec::new();
    for _ in 0..2 {
        let presenter = Arc::new(ScriptedPresenter::default());
        let login = {
            let flow = flow.clone();
            let presenter = presenter.clone();
            tokio::spawn(async move { flow.login(presenter.as_ref()).await })
        };
        drive_openrouter_callback(&presenter, "c").await;
        login.await.expect("join").expect("login failed");
        let url = presenter.last_authorize_url();
        let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
        let path = callback
            .split_once(OPENROUTER_CALLBACK_PATH_PREFIX)
            .unwrap()
            .1;
        assert!(!path.is_empty());
        paths.push(path.to_owned());
    }
    assert_ne!(paths[0], paths[1], "callback route must be per-login");
}

#[tokio::test]
async fn openrouter_wrong_callback_route_keeps_waiting() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();

    // A stray request for a different route (and a non-GET probe) gets 404
    // and must not complete or fail the login; the real callback then wins.
    let stray = callback.replace(
        OPENROUTER_CALLBACK_PATH_PREFIX,
        &format!("{OPENROUTER_CALLBACK_PATH_PREFIX}not-the-route-"),
    );
    let response = reqwest::get(format!("{stray}?code=stray-code"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{callback}?code=stray"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
    let response = reqwest::get(format!("{callback}?code=real-code"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "k");
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "real-code");
}

#[tokio::test]
async fn openrouter_callback_denial_is_typed_without_detail_echo() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!(
        "{callback}?error=access_denied&error_description=secret-canary"
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    let error = login.await.expect("join").expect_err("denial must fail");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "authorization denied")
    );
}

#[tokio::test]
async fn openrouter_callback_missing_code_fails_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!("{callback}?unrelated=1"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    let error = login.await.expect("join").expect_err("missing code");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_manual_full_url_paste_exchanges_its_code() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k-manual"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    // Paste the flow's own callback URL back with a code appended.
    *presenter.on_present.lock().unwrap() = Some(Arc::new(|url: &str| {
        let callback = ScriptedPresenter::query_param(url, "callback_url").unwrap();
        format!("{callback}?code=manual-1")
    }));
    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    assert_eq!(credential.access.expose_secret(), "k-manual");
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "manual-1");
}

#[tokio::test]
async fn openrouter_manual_query_fragment_and_bare_code_pastes() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());

    for paste in ["x=1&code=q2", "bare-code-42"] {
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual(paste);
        let credential = flow.login(presenter.as_ref()).await.expect("login failed");
        assert_eq!(credential.access.expose_secret(), "k");
        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&requests.last().unwrap().body).unwrap();
        assert_eq!(
            body["code"],
            if paste == "x=1&code=q2" {
                "q2"
            } else {
                "bare-code-42"
            }
        );
    }
}

#[tokio::test]
async fn openrouter_manual_missing_code_is_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());
    for bad in ["", "   ", "http://127.0.0.1:9/oauth/callback/abc?x=1"] {
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual(bad);
        let error = flow.login(presenter.as_ref()).await.expect_err("must fail");
        assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_exchange_http_error_is_closed_summary() {
    let server = MockServer::start().await;
    mount_openrouter_key(
        &server,
        400,
        r#"{"error":{"message":"canary-secret-detail"}}"#.to_owned(),
    )
    .await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("http error");
    assert!(
        matches!(error, ProviderError::AuthFailed(_)) || matches!(error, ProviderError::Config(_)),
        "got {error:?}"
    );
    let rendered = format!("{error:?}");
    assert!(
        !rendered.contains("canary"),
        "untrusted detail echoed: {rendered}"
    );
}

#[tokio::test]
async fn openrouter_exchange_invalid_json_is_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, "not-json".to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let error = flow.login(presenter.as_ref()).await.expect_err("bad json");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
}

#[tokio::test]
async fn openrouter_exchange_missing_or_empty_key_is_typed() {
    let server = MockServer::start().await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());
    for body in [r#"{"key":""}"#, "{}"] {
        mount_openrouter_key(&server, 200, body.to_owned()).await;
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual("paste-code");
        let error = flow.login(presenter.as_ref()).await.expect_err("no key");
        assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    }
}

#[tokio::test]
async fn openrouter_refresh_is_identity_without_network() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    let before = server.received_requests().await.unwrap().len();

    let refreshed = flow.refresh(&credential).await.expect("identity refresh");
    assert_eq!(
        refreshed.access.expose_secret(),
        credential.access.expose_secret()
    );
    assert_eq!(refreshed.refresh.expose_secret(), "");
    assert_eq!(refreshed.expires_at, credential.expires_at);
    assert_eq!(server.received_requests().await.unwrap().len(), before);
}

#[tokio::test]
async fn openrouter_permanent_key_expiry_is_the_9999_sentinel() {
    assert_eq!(
        openrouter_permanent_key_expiry(),
        time::OffsetDateTime::from_unix_timestamp(OPENROUTER_PERMANENT_KEY_UNIX_TS).unwrap()
    );
    assert_eq!(openrouter_permanent_key_expiry().year(), 9999);
}

#[tokio::test]
async fn openrouter_models_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"sk-or-v1-acked"}"#.to_owned()).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let presenter = Arc::new(ScriptedPresenter::with_manual("paste-code"));
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    register_oauth_assembly_route(
        &mut models,
        Arc::new(flow),
        opi_ai::WireApi::OpenAiCompletions,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );

    let credential = models
        .login("openrouter", presenter.as_ref().as_ref())
        .await
        .expect("models login");
    assert_eq!(credential.access.expose_secret(), "sk-or-v1-acked");
    assert_openrouter_adapter_request(&models, &credential, &server).await;
    assert!(!credential.needs_refresh());
    let stored = store
        .read("openrouter")
        .await
        .expect("store read")
        .expect("stored credential");
    match stored {
        Credential::OAuthToken {
            access,
            refresh,
            expires_at,
            ..
        } => {
            assert_eq!(access.expose_secret(), "sk-or-v1-acked");
            assert_eq!(refresh.expose_secret(), "");
            assert_eq!(expires_at, Some(openrouter_permanent_key_expiry()));
        }
        other => panic!("wrong credential kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn openrouter_login_issues_no_personalization_request() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    drive_openrouter_callback(&presenter, "cb-1").await;
    login.await.expect("join").expect("login failed");
    // Only the key exchange: no authorize fetch, no /models/user, nothing else.
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/api/v1/auth/keys");
}

#[tokio::test]
async fn openrouter_login_timeout_is_typed() {
    let flow = OpenRouterOAuthProvider::new(
        OPENROUTER_AUTHORIZE_URL.to_owned(),
        format!("{OPENROUTER_TOKEN_URL}/unused"),
        Duration::from_millis(300),
    );
    let presenter = ScriptedPresenter::default();
    let error = flow.login(&presenter).await.expect_err("timeout");
    assert!(matches!(error, ProviderError::Timeout), "got {error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "timeout")
    );
}

#[tokio::test]
async fn openrouter_production_constants_are_pinned() {
    let flow = OpenRouterOAuthProvider::production();
    let credential = OAuthCredential {
        access: secrecy::SecretString::new("k".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new(String::new().into_boxed_str()),
        expires_at: Some(openrouter_permanent_key_expiry()),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    };
    // Identity refresh holds for the production construction too; only the
    // endpoints differ from the injected tests.
    let refreshed = flow.refresh(&credential).await.expect("identity");
    assert_eq!(refreshed.access.expose_secret(), "k");
    assert_eq!(OPENROUTER_REDIRECT_HOST, "127.0.0.1");
    assert_eq!(
        OPENROUTER_TOKEN_URL,
        "https://openrouter.ai/api/v1/auth/keys"
    );
    assert_eq!(OPENROUTER_AUTHORIZE_URL, "https://openrouter.ai/auth");
}

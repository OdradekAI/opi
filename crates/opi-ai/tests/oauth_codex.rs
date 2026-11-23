//! OpenAI Codex OAuth flow fixtures and acceptance tests.
//!
//! This target owns Codex browser/device login, fixed-loopback fallback,
//! token exchange, account-id extraction, refresh and Models store ACK tests.
//! Shared OAuth presenters, assembly assertions and loopback fixtures live in
//! `tests/support`.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::Models;
use opi_ai::auth::{OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "support/oauth_assembly.rs"]
#[allow(dead_code)]
mod oauth_assembly;
#[path = "support/oauth_codex.rs"]
#[allow(dead_code)]
mod oauth_codex;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;
#[path = "support/oauth_loopback.rs"]
#[allow(dead_code)]
mod oauth_loopback;

use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_codex::*;
use oauth_fixtures::{ScriptedPresenter, stored_credential};
use oauth_loopback::*;

// Codex browser, device and refresh protocol assertions
// ---------------------------------------------------------------------------

// Codex wire helpers are owned by `support/oauth_codex.rs`.

#[tokio::test]
async fn codex_browser_login_posts_form_token_request_with_fixed_redirect() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let access = codex_jwt(Some("acct-123"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), access);
    assert_eq!(credential.refresh.expose_secret(), "codex-refresh-1");
    assert_eq!(credential.account_id.as_deref(), Some("acct-123"));

    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token request");
    assert_eq!(
        token_req.headers.get("content-type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    let pairs: std::collections::BTreeMap<String, String> = body
        .split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap();
            (k.to_owned(), v.replace("%3A", ":").replace("%2F", "/"))
        })
        .collect();
    assert_eq!(pairs["grant_type"], "authorization_code");
    assert_eq!(pairs["client_id"], "codex-client-id");
    assert_eq!(pairs["redirect_uri"], CODEX_REDIRECT_URI);
    assert!(
        pairs["code_verifier"].len() >= 43,
        "{:?}",
        pairs["code_verifier"]
    );
}

#[tokio::test]
async fn codex_authorize_url_carries_pkce_state_and_codex_params() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_codex_token_success(&server, &codex_jwt(Some("acct-123"))).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    flow.login(presenter.as_ref()).await.expect("login failed");

    let url = presenter.last_authorize_url();
    assert!(url.starts_with(&format!("{}/oauth/authorize?", server.uri())));
    let state = ScriptedPresenter::query_param(&url, "state").expect("state");
    assert!(
        !state.is_empty()
            && state != ScriptedPresenter::query_param(&url, "code_challenge").unwrap()
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "redirect_uri").as_deref(),
        Some(CODEX_REDIRECT_URI)
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "scope").as_deref(),
        Some("openid profile email offline_access")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "id_token_add_organizations").as_deref(),
        Some("true")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "codex_cli_simplified_flow").as_deref(),
        Some("true")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "originator").as_deref(),
        Some("opi")
    );
}

#[tokio::test]
async fn codex_busy_1455_falls_back_to_manual_paste_with_fixed_redirect() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    // Hold the CLI-shared port so the SDK must fall back paste-back only —
    // never to a free loopback port (pi `.catch(() => undefined)`).
    let _occupier = occupy_1455_retrying().await;
    let access = codex_jwt(Some("acct-123"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    assert_eq!(credential.account_id.as_deref(), Some("acct-123"));

    // The manual paste echoed the authorize URL (redirect fixed at 1455) and
    // the exchange kept the same fixed redirect URI.
    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token request");
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    let redirect = body
        .split('&')
        .find_map(|pair| pair.strip_prefix("redirect_uri="))
        .expect("redirect_uri param");
    assert_eq!(redirect, "http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback");
}

#[tokio::test]
async fn codex_device_flow_presents_user_code_and_exchanges_with_server_verifier() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!(0)).await;
    mount_codex_device_poll_complete(&server).await;
    let access = codex_jwt(Some("acct-device"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let credential = match flow.login(presenter.as_ref()).await {
        Ok(c) => c,
        Err(e) => {
            for r in server.received_requests().await.unwrap() {
                eprintln!(
                    "REQ {} {} body={}",
                    r.method,
                    r.url.path(),
                    String::from_utf8_lossy(&r.body)
                );
            }
            panic!("device login failed: {e:?}");
        }
    };
    assert_eq!(credential.account_id.as_deref(), Some("acct-device"));

    let devices = presenter.device_codes.lock().unwrap().clone();
    assert_eq!(
        devices,
        vec![(
            "PUBLIC-CODE".to_owned(),
            CODEX_DEVICE_VERIFICATION.to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token exchange");
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    assert!(body.contains("code=device-code-1"), "{body}");
    assert!(
        body.contains("code_verifier=device-verifier-from-server"),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "redirect_uri={}",
            urlencode(CODEX_DEVICE_REDIRECT_URI)
        )),
        "{body}"
    );
}

#[tokio::test]
async fn codex_device_denied_and_expired_are_typed_without_body_echo() {
    for code in ["deviceauth_access_denied", "expired_token"] {
        let server = MockServer::start().await;
        mount_codex_device_start(&server, serde_json::json!(0)).await;
        mount_codex_device_poll_status(
            &server,
            403,
            serde_json::json!({"error": {"code": code}, "secret": "device-secret"}),
        )
        .await;
        let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

        let error = flow.login(presenter.as_ref()).await.expect_err("denied");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{code}: {error:?}"
        );
        assert!(!format!("{error:?}").contains("device-secret"));
    }
}

#[tokio::test]
async fn codex_device_invalid_interval_is_typed_before_any_poll() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!("not-a-number")).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("invalid interval");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    // The token poll endpoint was never reached.
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| r.url.path() != "/deviceauth/token")
    );
}

#[tokio::test]
async fn codex_device_usercode_404_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "detail": "not enabled server-secret"
        })))
        .mount(&server)
        .await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("404 usercode");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("server-secret"));
}

#[tokio::test]
async fn codex_device_budget_exhaustion_times_out() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!(30)).await;
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": "deviceauth_authorization_pending"
        })))
        .mount(&server)
        .await;
    // Poll interval 30s inside a 250ms device budget: the first sleep exhausts it.
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_millis(250));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow.login(presenter.as_ref()).await.expect_err("timeout");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|f| f.contains("timed out"))
    );
}

#[tokio::test]
async fn codex_refresh_requires_all_token_fields_and_reextracts_account_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": codex_jwt(Some("acct-new")),
            "expires_in": 3600,
        })))
        .mount(&server)
        .await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let cred = OAuthCredential {
        base_url: Some("https://chatgpt.example.test/backend".to_owned()),
        ..stored_credential()
    };

    let error = flow
        .refresh(&cred)
        .await
        .expect_err("missing refresh_token");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");

    // Strict success: full triple, endpoint (base_url) preserved, account id
    // re-extracted from the new access token.
    let server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": codex_jwt(Some("acct-new")),
            "refresh_token": "codex-refresh-2",
            "expires_in": 3600,
        })))
        .mount(&server2)
        .await;
    let flow2 = codex_flow(&server2, Duration::from_secs(30), Duration::from_secs(30));
    let refreshed = flow2.refresh(&cred).await.expect("refresh ok");
    assert_eq!(refreshed.refresh.expose_secret(), "codex-refresh-2");
    assert_eq!(
        refreshed.base_url.as_deref(),
        Some("https://chatgpt.example.test/backend")
    );
    assert_eq!(refreshed.account_id.as_deref(), Some("acct-new"));
    assert!(refreshed.expires_at.is_some());
}

#[tokio::test]
async fn codex_credential_without_account_id_is_typed() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    // Access token carries no chatgpt_account_id claim: pi credentialsFromToken
    // throws; the SDK must fail typed and never yield a credential.
    mount_codex_token_success(&server, &codex_jwt(None)).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("no account id");
    assert!(
        matches!(error, ProviderError::AccountIdMissing { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn codex_models_login_acks_the_store_write() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let access = codex_jwt(Some("acct-models"));
    mount_codex_token_success(&server, &access).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let store_handle = store.clone();
    let presenter = codex_presenter();
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let mut models = Models::new();
    models.set_credential_store(store);
    register_oauth_assembly_route(
        &mut models,
        Arc::new(flow),
        opi_ai::WireApi::OpenAiCodexResponses,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );

    let credential = models
        .login("openai-codex", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.account_id.as_deref(), Some("acct-models"));
    assert_oauth_adapter_request(
        &models,
        "openai-codex",
        &credential,
        &server,
        opi_ai::WireApi::OpenAiCodexResponses,
    )
    .await;
    let expected_access = credential.access.expose_secret().to_owned();

    let stored = store_handle
        .read("openai-codex")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), expected_access);
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

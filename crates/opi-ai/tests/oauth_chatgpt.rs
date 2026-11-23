//! OpenAI ChatGPT OAuth flow fixtures and acceptance tests.
//!
//! This target owns dynamic client registration, browser callback, manual
//! paste validation, device identity checks, refresh and Models store ACK.
//! Shared OAuth presenters, adapter assertions and 1455 loopback fixtures live
//! in `tests/support`.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::Models;
use opi_ai::auth::{OAuthCredential, OAuthProvider};
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::MockServer;

#[path = "support/oauth_assembly.rs"]
#[allow(dead_code)]
mod oauth_assembly;
#[path = "support/oauth_chatgpt.rs"]
#[allow(dead_code)]
mod oauth_chatgpt;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;
#[path = "support/oauth_loopback.rs"]
#[allow(dead_code)]
mod oauth_loopback;

use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_chatgpt::*;
use oauth_fixtures::ScriptedPresenter;
use oauth_loopback::*;

// ChatGPT wire helpers are owned by `support/oauth_chatgpt.rs`.

#[tokio::test]
async fn chatgpt_authorize_url_carries_dynamic_client_resource_and_host_identity() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let credential = flow.login(presenter.as_ref()).await.expect("chatgpt login");
    let url = presenter.last_authorize_url();
    assert_eq!(
        ScriptedPresenter::query_param(&url, "client_id").as_deref(),
        Some("dynamic_agent_client")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "redirect_uri").as_deref(),
        Some(CHATGPT_REDIRECT)
    );
    let scope = ScriptedPresenter::query_param(&url, "scope").unwrap();
    assert!(scope.contains("resource.invoke"));
    assert!(scope.contains("chatgpt.tokens.use.direct"));
    assert_eq!(
        ScriptedPresenter::query_param(&url, "resource").as_deref(),
        Some("https://api.openai.com/v1")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "agent_name_hint").as_deref(),
        Some("opi")
    );
    // Uppercase UUIDs normalize to the lowercase urn form.
    assert_eq!(
        ScriptedPresenter::query_param(&url, "ext_agent_host_id").as_deref(),
        Some("urn:uuid:0f0e8d9a-1111-4222-8333-444455556666")
    );
    assert!(ScriptedPresenter::query_param(&url, "nonce").is_some());
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert!(ScriptedPresenter::query_param(&url, "state").is_some());
    // The credential stores the issued client id, not the dynamic literal.
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-1"));
    assert!(
        credential
            .scopes
            .as_ref()
            .unwrap()
            .iter()
            .any(|scope| scope == "chatgpt.tokens.use.direct")
    );
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn chatgpt_exchange_posts_issued_client_id_and_resource_as_form() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let before = time_now_secs();
    let credential = flow.login(presenter.as_ref()).await.expect("login");
    let requests = server.received_requests().await.unwrap();
    let token_request = requests
        .iter()
        .find(|r| r.url.path() == "/api/accounts/oauth/token")
        .expect("token request");
    let content_type = token_request
        .headers
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    assert!(content_type.starts_with("application/x-www-form-urlencoded"));
    let body = String::from_utf8(token_request.body.clone()).unwrap();
    let pairs = form_pairs(&body);
    let pair = |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(pair("grant_type"), "authorization_code");
    assert_eq!(pair("client_id"), "issued-client-1");
    assert_eq!(pair("code"), "code-1");
    assert_eq!(pair("redirect_uri"), CHATGPT_REDIRECT);
    assert_eq!(pair("resource"), "https://api.openai.com/v1");
    assert!(!pair("code_verifier").is_empty());
    // Real service expiry (expires_in = 3600), not a pre-margin value.
    let expires_delta = credential.expires_at.unwrap().unix_timestamp() as i64 - before;
    assert!((3590..=3610).contains(&expires_delta), "{expires_delta}");
    assert_eq!(credential.access.expose_secret(), "at-1");
    assert_eq!(credential.refresh.expose_secret(), "rt-1");
    assert_eq!(credential.account_id, None);
}

/// Wall-clock seconds for real-expiry assertions.
fn time_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn chatgpt_callback_with_issued_client_id_completes_login() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let driver_presenter = presenter.clone();
    let drive = tokio::spawn(async move {
        let url = chatgpt_wait_for_authorize_url(&driver_presenter).await;
        let state = ScriptedPresenter::query_param(&url, "state").unwrap();
        let response = chatgpt_callback_get(&format!(
            "GET /auth/callback?code=cb-9&state={state}&client_id=issued-client-9"
        ))
        .await
        .expect("callback response");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    });
    let credential = flow.login(presenter.as_ref()).await.expect("login");
    drive.await.expect("driver");
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-9"));
    assert_eq!(credential.access.expose_secret(), "at-1");
}

#[tokio::test]
async fn chatgpt_busy_1455_is_typed_terminal_without_interaction() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    // Hold the CLI-shared port like another pending login would.
    let holder = occupy_1455_retrying().await;

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("bind must fail terminally");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    // The failure happens before the authorize URL is presented and before
    // any HTTP traffic: no silent manual-only or free-port fallback.
    assert!(presenter.authorize_urls.lock().unwrap().is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.contains("in use"))
    );
    drop(holder);
}

#[tokio::test]
async fn chatgpt_manual_paste_rejects_foreign_origin_and_bare_code() {
    let _guard = loopback_1455_guard().await;
    let pastes = [
        // localhost is a different origin than the fixed 127.0.0.1 redirect.
        format!("{CHATGPT_REDIRECT}?code=code-1&state=x&client_id=issued-client-1")
            .replace("127.0.0.1", "localhost"),
        // A bare code is never accepted for this flow.
        "code-1".to_owned(),
        // A foreign path on the right host.
        "http://127.0.0.1:1455/other?code=code-1&state=x&client_id=issued-client-1".to_owned(),
    ];
    for paste in pastes {
        let server = MockServer::start().await;
        mount_chatgpt_token(&server, serde_json::json!({})).await;
        let presenter = ScriptedPresenter::with_manual(paste);
        let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
        assert!(
            presenter
                .failures
                .lock()
                .unwrap()
                .iter()
                .any(|r| r == "manual redirect error")
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn chatgpt_error_param_paste_is_typed_denial() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let presenter = ScriptedPresenter::with_manual(format!(
        "{CHATGPT_REDIRECT}?error=access_denied&error_description=denied+by+user"
    ));
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let error = flow.login(presenter.as_ref()).await.expect_err("denied");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "authorization denied")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_callback_without_client_id_is_typed() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let driver_presenter = presenter.clone();
    let drive = tokio::spawn(async move {
        let url = chatgpt_wait_for_authorize_url(&driver_presenter).await;
        let state = ScriptedPresenter::query_param(&url, "state").unwrap();
        let response =
            chatgpt_callback_get(&format!("GET /auth/callback?code=cb-8&state={state}")).await;
        let response = response.expect("callback response");
        assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    });
    let error = flow.login(presenter.as_ref()).await.expect_err("typed");
    drive.await.expect("driver");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "callback parse error")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_device_id_must_be_a_present_uuid_before_any_traffic() {
    for device_id in [None, Some("not-a-uuid"), Some("")] {
        let server = MockServer::start().await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let flow = chatgpt_flow(&server, device_id, Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "{device_id:?}: {error:?}"
        );
        assert!(presenter.authorize_urls.lock().unwrap().is_empty());
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn chatgpt_token_response_contract_violations_are_typed() {
    let _guard = loopback_1455_guard().await;
    let cases: [(serde_json::Value, &str); 6] = [
        (serde_json::json!({"id_token": null}), "id_token"),
        (
            serde_json::json!({"scope": "openid profile email offline_access"}),
            "chatgpt.tokens.use.direct",
        ),
        (serde_json::json!({"refresh_token": null}), "refresh_token"),
        (serde_json::json!({"expires_in": 0}), "expires_in"),
        (serde_json::json!({"expires_in": -5}), "expires_in"),
        (serde_json::json!({"expires_in": null}), "expires_in"),
    ];
    for (mutate, expected_fragment) in cases {
        let case = mutate.to_string();
        let server = MockServer::start().await;
        mount_chatgpt_token(&server, mutate).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
        let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "{case} ({expected_fragment}): {error:?}"
        );
        // A contract violation never yields a credential or success.
        assert!(presenter.success_at.lock().unwrap().is_none());
    }
}

#[tokio::test]
async fn chatgpt_refresh_reuses_stored_client_id_and_resource_and_revalidates() {
    let server = MockServer::start().await;
    mount_chatgpt_token(
        &server,
        serde_json::json!({
            "access_token": "at-2", "refresh_token": "rt-2", "expires_in": 5400,
            "id_token": null
        }),
    )
    .await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let cred = OAuthCredential {
        access: secrecy::SecretString::new("at-1".into()),
        refresh: secrecy::SecretString::new("rt-1".into()),
        expires_at: None,
        base_url: None,
        account_id: None,
        client_id: Some("issued-client-7".to_owned()),
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    };

    let refreshed = flow.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "at-2");
    assert_eq!(refreshed.refresh.expose_secret(), "rt-2");
    assert_eq!(refreshed.client_id.as_deref(), Some("issued-client-7"));
    assert_eq!(
        refreshed.scopes.as_ref().unwrap(),
        &vec![
            "openid",
            "profile",
            "email",
            "offline_access",
            "resource.invoke",
            "chatgpt.tokens.use.direct"
        ]
    );
    let requests = server.received_requests().await.unwrap();
    let body = String::from_utf8(requests[0].body.clone()).unwrap();
    let pairs = form_pairs(&body);
    let pair = |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(pair("grant_type"), "refresh_token");
    assert_eq!(pair("client_id"), "issued-client-7");
    assert_eq!(pair("refresh_token"), "rt-1");
    assert_eq!(pair("resource"), "https://api.openai.com/v1");

    // A response without the direct-token scope is typed, never a credential.
    let server = MockServer::start().await;
    mount_chatgpt_token(
        &server,
        serde_json::json!({
            "access_token": "at-3", "refresh_token": "rt-3", "expires_in": 5400,
            "scope": "openid profile email offline_access", "id_token": null
        }),
    )
    .await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let error = flow.refresh(&cred).await.expect_err("typed");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");

    // A stored credential without the issued client id is a typed reconnect
    // error before any HTTP traffic.
    let server = MockServer::start().await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let cred = OAuthCredential {
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        ..cred
    };
    let error = flow.refresh(&cred).await.expect_err("typed");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_models_login_acks_the_store_write_as_an_oauth_envelope() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let store_handle = store.clone();
    let mut models = Models::new();
    models.set_credential_store(store);
    register_oauth_assembly_route(
        &mut models,
        Arc::new(flow),
        opi_ai::WireApi::OpenAiResponses,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );

    let credential = models
        .login("openai-chatgpt", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-1"));
    assert_oauth_adapter_request(
        &models,
        "openai-chatgpt",
        &credential,
        &server,
        opi_ai::WireApi::OpenAiResponses,
    )
    .await;
    // The persisted envelope is the OAuth token kind with the issued client
    // id — API-key-only consumers (the Decisions classifier) can never
    // resolve this subscription credential as an API key.
    let stored = store_handle
        .read("openai-chatgpt")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken {
            access,
            client_id,
            scopes,
            ..
        } => {
            assert_eq!(access.expose_secret(), credential.access.expose_secret());
            assert_eq!(client_id.as_deref(), Some("issued-client-1"));
            assert_eq!(scopes, credential.scopes);
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

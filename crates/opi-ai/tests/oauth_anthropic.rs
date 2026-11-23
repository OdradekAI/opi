//! Anthropic OAuth flow fixtures and acceptance tests.
//!
//! This target owns Anthropic browser/copy-code login, callback validation,
//! token exchange, refresh and preferred-port lifecycle assertions. Shared
//! presenter utilities and Anthropic HTTP mounts live in `tests/support`.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "support/oauth_anthropic.rs"]
#[allow(dead_code)]
mod oauth_anthropic;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;

use oauth_anthropic::*;
use oauth_fixtures::{ScriptedPresenter, stored_credential};

// ---------------------------------------------------------------------------
// Login wire shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn browser_login_posts_json_token_request() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    {
        let hook = ScriptedPresenter::echo_redirect_hook();
        *presenter.on_present.lock().unwrap() = Some(hook);
    }

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), "acc-1");
    assert_eq!(credential.refresh.expose_secret(), "ref-1");
    let expires_in = credential
        .expires_at
        .expect("credential carries service expiry")
        .unix_timestamp()
        - time::OffsetDateTime::now_utc().unix_timestamp();
    assert!(
        expires_in > 3500 && expires_in <= 3600,
        "service expiry stored real (within tolerance): {expires_in}"
    );

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(request.headers.get("accept").unwrap(), "application/json");
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["grant_type"], "authorization_code");
    assert_eq!(body["client_id"], "test-client-id");
    assert_eq!(body["code"], "code-1");
    assert_eq!(
        body["state"],
        ScriptedPresenter::query_param(&presenter.last_authorize_url(), "state").unwrap()
    );
    assert!(body["code_verifier"].as_str().unwrap().len() >= 43);
    assert!(
        body["redirect_uri"]
            .as_str()
            .unwrap()
            .starts_with("http://localhost:")
    );
    assert!(
        body["redirect_uri"]
            .as_str()
            .unwrap()
            .ends_with("/callback")
    );
}

#[tokio::test]
async fn authorize_url_carries_pkce_state_scopes_and_callback() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), "acc-1");
    let url = presenter.last_authorize_url();
    assert!(url.starts_with("https://authorize.example.test/authorize?"));
    assert_eq!(
        ScriptedPresenter::query_param(&url, "response_type").as_deref(),
        Some("code")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "client_id").as_deref(),
        Some("test-client-id")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    let challenge = ScriptedPresenter::query_param(&url, "code_challenge").unwrap();
    assert!(challenge.len() >= 43);
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();
    assert!(state.len() >= 16);
    let scope = ScriptedPresenter::query_param(&url, "scope").unwrap();
    let expected_scope = "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
    assert_eq!(scope, expected_scope);
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code").as_deref(),
        Some("true")
    );
}

#[tokio::test]
async fn login_binds_preferred_port_53692_with_callback_path() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    wait_preferred_port_free().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    // Wait for the authorize URL to appear (flow bound its callback).
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let redirect = ScriptedPresenter::query_param(&url, "redirect_uri").unwrap();
    assert_eq!(redirect, "http://localhost:53692/callback");
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();

    // The browser lands on the loopback callback; the callback arm wins.
    let callback_url = format!("http://127.0.0.1:53692/callback?code=cb-code&state={state}");
    let response = reqwest::get(&callback_url).await.expect("callback request");
    assert_eq!(response.status(), 200);

    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "acc-1");
    let body: serde_json::Value =
        serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(body["code"], "cb-code");
}

#[tokio::test]
async fn login_falls_back_to_free_loopback_when_53692_is_busy() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    let _occupier = occupy_preferred_port_retrying().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let redirect = ScriptedPresenter::query_param(&url, "redirect_uri").unwrap();
    let port: u16 = redirect
        .rsplit_once(':')
        .unwrap()
        .1
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(port, 53692, "must not reuse the busy preferred port");
    assert!(redirect.ends_with("/callback"));
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();

    let callback_url = format!("http://127.0.0.1:{port}/callback?code=fb-code&state={state}");
    let response = reqwest::get(&callback_url).await.expect("callback request");
    assert_eq!(response.status(), 200);
    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "acc-1");
}

// ---------------------------------------------------------------------------
// Copy-code login
// ---------------------------------------------------------------------------

#[tokio::test]
async fn copy_code_login_uses_fixed_redirect_uri_and_code_state_paste() {
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::CopyCode);
    let scheme = "https";
    let expected_redirect = format!("{scheme}://platform.claude.com/oauth/code/callback");
    {
        let expected_redirect = expected_redirect.clone();
        *presenter.on_present.lock().unwrap() = Some(Arc::new(move |url: &str| {
            let state = ScriptedPresenter::query_param(url, "state").unwrap();
            assert_eq!(
                ScriptedPresenter::query_param(url, "redirect_uri").unwrap(),
                expected_redirect
            );
            format!("pasted-code#{state}")
        }));
    }

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    let url = presenter.last_authorize_url();
    assert_eq!(
        presenter.offered_methods.lock().unwrap().as_slice(),
        &[OAuthLoginMethod::Browser, OAuthLoginMethod::CopyCode]
    );
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();
    assert_eq!(credential.access.expose_secret(), "acc-1");
    let body: serde_json::Value =
        serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(body["code"], "pasted-code");
    assert_eq!(body["state"], serde_json::json!(state));
    assert_eq!(body["redirect_uri"], serde_json::json!(expected_redirect));
}

#[tokio::test]
async fn copy_code_state_mismatch_is_typed_and_sends_zero_token_requests() {
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::CopyCode);
    *presenter.on_present.lock().unwrap() = Some(Arc::new(|url: &str| {
        let state = ScriptedPresenter::query_param(url, "state").unwrap();
        format!("code-x#wrong-{state}")
    }));

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("login must fail");
    // State mismatch is a typed config failure (the paste never reaches the
    // token endpoint), fully redacted like every token-endpoint summary.
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Callback validation and failure mapping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn callback_state_mismatch_answers_secret_free_400_and_never_exchanges() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    wait_preferred_port_free().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let _ = url.expect("authorize url never presented");

    // Wrong state lands on the bound callback: fixed 400, no token exchange.
    let response = reqwest::get("http://127.0.0.1:53692/callback?code=x&state=wrong")
        .await
        .expect("callback request");
    assert_eq!(response.status(), 400);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Login failed, return to the terminal.");
    assert!(server.received_requests().await.unwrap().is_empty());
    // Complete the flow through the manual arm so the login future finishes.
    login.abort();
}

#[tokio::test]
async fn token_endpoint_denial_and_invalid_grant_map_to_typed_classes() {
    let _port_guard = port_guard().await;
    for error_code in ["access_denied", "invalid_grant"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error": error_code, "error_description": "secret-canary"}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let flow = anthropic_flow(format!("{}/token", server.uri()));

        let error = match login_via_echoed_redirect(&flow).await {
            Ok(_) => panic!("denial must fail"),
            Err(error) => error,
        };
        // Summaries are fully redacted: the closed class never surfaces and
        // the echoed description canary cannot leak.
        assert!(matches!(error, ProviderError::AuthFailed(_)));
        let message = error.to_string();
        assert!(!message.contains(error_code), "{message}");
        assert!(!message.contains("secret-canary"), "{message}");
    }
}

#[tokio::test]
async fn token_response_missing_fields_is_typed() {
    let _port_guard = port_guard().await;
    for missing in ["refresh_token", "expires_in"] {
        let server = MockServer::start().await;
        let mut body = token_body("acc", "ref", 3600);
        body.as_object_mut().unwrap().remove(missing);
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let flow = anthropic_flow(format!("{}/token", server.uri()));

        let error = match login_via_echoed_redirect(&flow).await {
            Ok(_) => panic!("missing {missing} must fail"),
            Err(error) => error,
        };
        assert!(matches!(error, ProviderError::Config(_)));
    }
}

#[tokio::test]
async fn manual_redirect_with_missing_or_duplicate_params_is_typed() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    for pasted in [
        "http://localhost:53692/callback?code=only-code", // missing state
        "http://localhost:53692/callback?code=a&code=b&state=s", // duplicate code
        "",                                               // empty paste
    ] {
        let presenter = ScriptedPresenter::with_manual(pasted.to_owned());
        let authorize = flow.login(presenter.as_ref()).await;
        // Every case must fail typed before any token request; the scripted
        // state never matches, so exchange cannot happen.
        assert!(authorize.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn login_cancelled_presenter_maps_to_login_cancelled() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    struct CancellingPresenter;
    impl LoginPresenter for CancellingPresenter {
        fn present_auth_url<'a>(
            &'a self,
            _url: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn present_device_code<'a>(
            &'a self,
            _user_code: &'a str,
            _verification_uri: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(async {
                Err(ProviderError::LoginCancelled {
                    provider_id: "anthropic".into(),
                })
            })
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = flow
        .login(&CancellingPresenter)
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::LoginCancelled { .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn refresh_posts_json_and_keeps_real_service_expiry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_partial_json(
            serde_json::json!({"grant_type": "refresh_token"}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(token_body("new-acc", "new-ref", 7200)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let refreshed = flow.refresh(&stored_credential()).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "new-acc");
    assert_eq!(refreshed.refresh.expose_secret(), "new-ref");
    let expires_in = refreshed.expires_at.expect("expiry").unix_timestamp()
        - time::OffsetDateTime::now_utc().unix_timestamp();
    assert!(
        expires_in > 7100 && expires_in <= 7200,
        "real service expiry, no pre-subtraction: {expires_in}"
    );

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["grant_type"], "refresh_token");
    assert_eq!(body["client_id"], "test-client-id");
    assert_eq!(body["refresh_token"], "old-refresh");
}

#[tokio::test]
async fn refresh_revoked_on_401_is_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string("{\"error\":\"invalid_grant\",\"detail\":\"canary\"}"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let error = flow
        .refresh(&stored_credential())
        .await
        .expect_err("revoked");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(!error.to_string().contains("canary"));
}

#[tokio::test]
async fn refresh_without_refresh_token_reuses_the_stored_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "new-acc",
            "expires_in": 3600,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let refreshed = flow.refresh(&stored_credential()).await.expect("refresh");
    assert_eq!(refreshed.refresh.expose_secret(), "old-refresh");
}

#[tokio::test]
async fn near_expiry_needs_refresh_uses_single_five_minute_window() {
    let now = time::OffsetDateTime::now_utc();
    let fresh = OAuthCredential {
        expires_at: Some(now + time::Duration::minutes(10)),
        ..stored_credential()
    };
    let near = OAuthCredential {
        expires_at: Some(now + time::Duration::minutes(4)),
        ..stored_credential()
    };
    let expired = OAuthCredential {
        expires_at: Some(now - time::Duration::minutes(1)),
        ..stored_credential()
    };
    assert!(!fresh.needs_refresh());
    assert!(near.needs_refresh());
    assert!(expired.needs_refresh());
    assert!(
        OAuthCredential {
            expires_at: None,
            ..stored_credential()
        }
        .needs_refresh()
    );
}

// ---------------------------------------------------------------------------

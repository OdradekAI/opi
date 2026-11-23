//! xAI OAuth flow fixtures and acceptance tests.
//!
//! This target owns device authorization, polling, refresh, cancellation,
//! typed errors, expiry and Models store ACK. Shared presenter, assembly and
//! xAI wire fixtures live in `tests/support`.

use std::sync::Arc;

use opi_ai::Models;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
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
#[path = "support/oauth_xai.rs"]
#[allow(dead_code)]
mod oauth_xai;

use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_fixtures::{ScriptedPresenter, expires_within};
use oauth_xai::*;

#[tokio::test]
async fn xai_device_login_presents_complete_uri_and_exchanges_tokens() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(&server, 200, xai_token_success()).await;
    let flow = xai_flow(&server);
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = flow.login(presenter.as_ref()).await.expect("login");

    assert_eq!(credential.access.expose_secret(), "xai-access-1");
    assert_eq!(credential.refresh.expose_secret(), "xai-refresh-1");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(!credential.needs_refresh());

    // The presenter gets the public user code and the COMPLETE https
    // verification URI, never the device_code.
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "XAI-CODE".to_owned(),
            "https://auth.x.ai/device?user_code=XAI-CODE".to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let start = &requests[0];
    assert_eq!(start.url.path(), "/oauth2/device/code");
    let start_body = String::from_utf8(start.body.clone()).unwrap();
    // The device start form: fixed client id, the xAI scope, and
    // pi's referrer hint, urlencoded (space as +, colon as %3A).
    assert_eq!(
        start_body,
        format!(
            "client_id={XAI_CLIENT_ID}&scope=openid+profile+email+offline_access+grok-cli%3Aaccess+api%3Aaccess&referrer=pi"
        )
    );
    assert!(
        start
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/x-www-form-urlencoded")
    );
    assert!(
        start
            .headers
            .get("accept")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );

    let poll = &requests[1];
    let poll_body = String::from_utf8(poll.body.clone()).unwrap();
    assert_eq!(poll.url.path(), "/oauth2/token");
    assert!(
        poll_body.contains(&format!("client_id={XAI_CLIENT_ID}")),
        "{poll_body}"
    );
    assert!(
        poll_body.contains("device_code=xai-device-1"),
        "{poll_body}"
    );
    assert!(
        poll_body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{poll_body}"
    );
}

#[test]
fn xai_parse_defaults_interval_and_optional_complete_uri() {
    // interval absent/non-positive falls back to 5s; an absent or empty
    // verification_uri_complete is simply omitted (the presenter falls back
    // to the plain URI); expires_in is required positive.
    let mut body = xai_start_body("soon".into(), 900.into());
    body.as_object_mut()
        .unwrap()
        .remove("verification_uri_complete");
    let parsed = parse_xai_device_authorization(&body).expect("parse");
    assert_eq!(parsed.interval_seconds, 5.0);
    assert_eq!(parsed.verification_uri_complete, None);
    assert_eq!(parsed.verification_uri, "https://auth.x.ai/device");
    assert_eq!(parsed.expires_in_seconds, 900.0);

    let mut with_empty = body.clone();
    with_empty["verification_uri_complete"] = serde_json::json!("");
    let parsed = parse_xai_device_authorization(&with_empty).expect("parse");
    assert_eq!(parsed.verification_uri_complete, None);

    let mut with_complete = body;
    with_complete["verification_uri_complete"] =
        serde_json::json!("https://auth.x.ai/device?user_code=XAI-CODE");
    let parsed = parse_xai_device_authorization(&with_complete).expect("parse");
    assert_eq!(
        parsed.verification_uri_complete.as_deref(),
        Some("https://auth.x.ai/device?user_code=XAI-CODE")
    );

    // expires_in missing, zero, or non-numeric is a typed failure.
    for bad in [
        serde_json::Value::Null,
        0.into(),
        (-5).into(),
        "3600".into(),
    ] {
        let mut b = xai_start_body(1.into(), bad.clone());
        if bad.is_null() {
            b.as_object_mut().unwrap().remove("expires_in");
        }
        assert!(
            parse_xai_device_authorization(&b).is_err(),
            "expires_in {bad:?} must fail"
        );
    }
}

#[tokio::test]
async fn xai_login_without_complete_uri_presents_plain_uri() {
    let server = MockServer::start().await;
    let mut body = xai_start_body(1.into(), 900.into());
    body.as_object_mut()
        .unwrap()
        .remove("verification_uri_complete");
    mount_xai_start(&server, body).await;
    mount_xai_token(&server, 200, xai_token_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "xai-access-1");
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[("XAI-CODE".to_owned(), "https://auth.x.ai/device".to_owned())]
    );
}

#[tokio::test]
async fn xai_start_non_ok_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/device/code"))
        .respond_with(ResponseTemplate::new(500).set_body_string("secret-start-detail-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization failed"]
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn xai_start_missing_or_empty_fields_fail_typed() {
    for label in ["device_code", "user_code", "verification_uri", "expires_in"] {
        for empty in [false, true] {
            let server = MockServer::start().await;
            let mut body = xai_start_body(1.into(), 900.into());
            if empty {
                body[label] = serde_json::json!("");
            } else {
                body.as_object_mut().unwrap().remove(label);
            }
            mount_xai_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = xai_flow(&server)
                .login(presenter.as_ref())
                .await
                .expect_err("must fail");
            assert!(
                matches!(
                    error,
                    ProviderError::Config(_) | ProviderError::AuthFailed(_)
                ),
                "{error:?} for {label} empty={empty}"
            );
            assert_eq!(
                presenter.failures.lock().unwrap().as_slice(),
                &["device authorization response invalid"]
            );
            // A rejected start never reaches the token endpoint.
            assert!(xai_token_requests(&server).await.is_empty());
        }
    }
}

#[tokio::test]
async fn xai_untrusted_verification_uris_fail_typed() {
    // pi validateVerificationUri: https only. An http plain URI fails; an
    // http COMPLETE URI also fails even though the plain URI is fine — pi
    // validates before choosing, it does not coalesce.
    for uri in [
        "http://auth.x.ai/device",
        "ftp://auth.x.ai/device",
        "not-a-url",
    ] {
        let server = MockServer::start().await;
        let mut body = xai_start_body(1.into(), 900.into());
        body["verification_uri"] = serde_json::json!(uri);
        mount_xai_start(&server, body).await;
        let error = xai_flow(&server)
            .login(Arc::new(ScriptedPresenter::default()).as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::Config(_) | ProviderError::AuthFailed(_)
            ),
            "{error:?} for {uri}"
        );
    }
    let server = MockServer::start().await;
    let mut body = xai_start_body(1.into(), 900.into());
    body["verification_uri_complete"] = serde_json::json!("http://auth.x.ai/device?user_code=X");
    mount_xai_start(&server, body).await;
    let error = xai_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect_err("must fail");
    assert!(
        matches!(
            error,
            ProviderError::Config(_) | ProviderError::AuthFailed(_)
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn xai_first_poll_waits_one_interval() {
    // interval 5s > deadline 2s: the pre-poll wait sleeps min(5, 2) and the
    // device deadline elapses before any token poll is issued.
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(5.into(), 2.into())).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out"]
    );
    assert!(xai_token_requests(&server).await.is_empty());
}

#[tokio::test]
async fn xai_slow_down_uses_server_interval() {
    // Initial interval 5s, deadline 8s. Poll #1 (t=5) answers slow_down with
    // interval 1: honoring the server value keeps polling every second
    // (t=6, t=7) until the deadline — exactly three polls.
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(5.into(), 8.into())).await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_xai_token(
        &server,
        400,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(xai_token_requests(&server).await.len(), 3);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn xai_slow_down_without_interval_adds_five_seconds() {
    // Initial interval 1s, deadline 10s. Poll #1 (t=1) answers slow_down
    // with no interval: the RFC 8628 §3.5 default adds 5s, so poll #2 lands
    // at t=6 and the 4s remainder hits the deadline — exactly two polls.
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 10.into())).await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_xai_token(
        &server,
        400,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(xai_token_requests(&server).await.len(), 2);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn xai_pending_then_success_completes_login() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 30.into())).await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    mount_xai_token(&server, 200, xai_token_success()).await;

    let credential = xai_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "xai-access-1");
    assert_eq!(xai_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn xai_denied_spellings_fail_typed() {
    // xAI answers denial as access_denied OR authorization_denied; both are
    // the same dead grant in pi's poll switch.
    for error_code in ["access_denied", "authorization_denied"] {
        let server = MockServer::start().await;
        mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
        mount_xai_token(
            &server,
            400,
            serde_json::json!({
                "error": error_code,
                "error_description": "secret-denial-detail-CANARY"
            }),
        )
        .await;
        let presenter = Arc::new(ScriptedPresenter::default());

        let error = xai_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {error_code}"
        );
        assert!(!format!("{error:?}").contains("CANARY"));
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["device authorization denied"]
        );
        assert_eq!(xai_token_requests(&server).await.len(), 1);
    }
}

#[tokio::test]
async fn xai_expired_token_fails_typed() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(&server, 400, serde_json::json!({"error": "expired_token"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(
        matches!(error, ProviderError::CredentialRevoked { .. }),
        "{error:?}"
    );
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device code expired"]
    );
}

#[tokio::test]
async fn xai_unknown_error_code_fails_typed_without_echo() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(
        &server,
        400,
        serde_json::json!({
            "error": "server_error",
            "error_description": "secret-detail-CANARY"
        }),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device token request failed"]
    );
}

#[tokio::test]
async fn xai_poll_5xx_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(503).set_body_string("poll-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = xai_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device token request failed"]
    );
}

#[tokio::test]
async fn xai_success_with_missing_fields_fails_typed() {
    // The ok branch parses first: a success body must carry non-empty
    // access and (on login) refresh tokens plus a positive-finite
    // expires_in when present (pi credentialsFromTokenResponse).
    for bad in [
        serde_json::json!({}), // everything missing
        serde_json::json!({"refresh_token": "xai-refresh-1", "expires_in": 3600}),
        serde_json::json!({"access_token": "", "refresh_token": "xai-refresh-1"}),
        serde_json::json!({"access_token": "xai-access-1"}), // login requires refresh
        serde_json::json!({"access_token": "xai-access-1", "refresh_token": ""}),
        serde_json::json!({"access_token": "xai-access-1", "refresh_token": "xai-refresh-1", "expires_in": 0}),
        serde_json::json!({"access_token": "xai-access-1", "refresh_token": "xai-refresh-1", "expires_in": "soon"}),
    ] {
        let server = MockServer::start().await;
        mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
        mount_xai_token(&server, 200, bad.clone()).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let error = xai_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::AuthFailed(_) | ProviderError::Config(_)
            ),
            "{error:?} for {bad}"
        );
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["device token response invalid"]
        );
    }
}

#[tokio::test]
async fn xai_token_expires_in_defaults_to_one_hour() {
    // pi DEFAULT_TOKEN_LIFETIME_SECONDS: a token response may omit
    // expires_in; the stored expiry is the real 3600s lifetime (the 5min
    // near-expiry correction lives in needs_refresh).
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(
        &server,
        200,
        serde_json::json!({
            "access_token": "xai-access-1",
            "refresh_token": "xai-refresh-1",
        }),
    )
    .await;

    let credential = xai_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert!(!credential.needs_refresh());
}

#[tokio::test]
async fn xai_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(&server, 200, xai_token_success()).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    register_oauth_assembly_route(
        &mut models,
        Arc::new(xai_flow(&server)),
        opi_ai::WireApi::OpenAiResponses,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("xai", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "xai-access-1");
    assert_oauth_adapter_request(
        &models,
        "xai",
        &credential,
        &server,
        opi_ai::WireApi::OpenAiResponses,
    )
    .await;

    let stored = store
        .read("xai")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken {
            access, refresh, ..
        } => {
            assert_eq!(access.expose_secret(), "xai-access-1");
            assert_eq!(refresh.expose_secret(), "xai-refresh-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    // notify_success fired after the credential existed in the store.
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn xai_refresh_rotates_or_preserves_the_refresh_token() {
    // A refresh response omitting refresh_token
    // falls back to the stored one (single-purpose fixture; never the Codex
    // strict rule). A present token rotates. Either way the grant, client
    // id, and (here) default 3600s expiry apply.
    for (with_refresh, expected_refresh) in [
        (
            serde_json::json!({"refresh_token": "xai-refresh-2"}),
            "xai-refresh-2",
        ),
        (serde_json::json!({}), "xai-old-refresh"),
    ] {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "access_token": "xai-access-2",
            "expires_in": 7200,
        });
        let body = merge_json(body, with_refresh);
        mount_xai_token(&server, 200, body).await;

        let refreshed = xai_flow(&server)
            .refresh(&xai_stored_credential())
            .await
            .expect("refresh");
        assert_eq!(refreshed.access.expose_secret(), "xai-access-2");
        assert_eq!(refreshed.refresh.expose_secret(), expected_refresh);
        expires_within(refreshed.expires_at.expect("expiry"), 7200);

        let requests = xai_token_requests(&server).await;
        assert_eq!(requests.len(), 1, "pi refreshXaiToken is a single pass");
        let form = String::from_utf8(requests[0].body.clone()).unwrap();
        assert!(form.contains("grant_type=refresh_token"), "{form}");
        assert!(
            form.contains(&format!("client_id={XAI_CLIENT_ID}")),
            "{form}"
        );
        assert!(form.contains("refresh_token=xai-old-refresh"), "{form}");
    }
}

#[tokio::test]
async fn xai_refresh_expires_in_defaults_to_one_hour() {
    let server = MockServer::start().await;
    mount_xai_token(
        &server,
        200,
        serde_json::json!({
            "access_token": "xai-access-2",
            "refresh_token": "xai-refresh-2",
        }),
    )
    .await;

    let refreshed = xai_flow(&server)
        .refresh(&xai_stored_credential())
        .await
        .expect("refresh");
    expires_within(refreshed.expires_at.expect("expiry"), 3600);
}

#[tokio::test]
async fn xai_refresh_401_403_revokes_for_relogin() {
    for status in [401u16, 403] {
        let server = MockServer::start().await;
        mount_xai_token(
            &server,
            status,
            serde_json::json!({"error": "invalid_grant", "error_description": "detail-CANARY"}),
        )
        .await;

        let error = xai_flow(&server)
            .refresh(&xai_stored_credential())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {status}"
        );
        assert!(!format!("{error:?}").contains("CANARY"));
        assert_eq!(xai_token_requests(&server).await.len(), 1);
    }
}

#[tokio::test]
async fn xai_refresh_non_ok_fails_typed_without_retry() {
    let server = MockServer::start().await;
    mount_xai_token(
        &server,
        500,
        serde_json::json!({"error": "server_error", "error_description": "detail-CANARY"}),
    )
    .await;

    let error = xai_flow(&server)
        .refresh(&xai_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    // pi refreshXaiToken has no retry loop: one request, transient or not.
    assert_eq!(xai_token_requests(&server).await.len(), 1);
}

#[tokio::test]
async fn xai_refresh_success_with_missing_access_token_fails_typed() {
    let server = MockServer::start().await;
    mount_xai_token(
        &server,
        200,
        serde_json::json!({"refresh_token": "xai-refresh-2"}),
    )
    .await;

    let error = xai_flow(&server)
        .refresh(&xai_stored_credential())
        .await
        .expect_err("must fail");
    assert!(
        matches!(
            error,
            ProviderError::Config(_) | ProviderError::AuthFailed(_)
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn xai_cancellation_during_device_presentation_is_typed() {
    let server = MockServer::start().await;
    mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
    mount_xai_token(&server, 200, xai_token_success()).await;
    struct CancellingPresenter;
    impl LoginPresenter for CancellingPresenter {
        fn present_auth_url<'a>(
            &'a self,
            _url: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn present_device_code<'a>(
            &'a self,
            _user_code: &'a str,
            _verification_uri: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async {
                Err(ProviderError::LoginCancelled {
                    provider_id: "xai".into(),
                })
            })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = xai_flow(&server)
        .login(&CancellingPresenter)
        .await
        .expect_err("must cancel");
    assert!(
        matches!(error, ProviderError::LoginCancelled { .. }),
        "{error:?}"
    );
    // The device start already happened; no token poll follows a cancel.
    assert!(xai_token_requests(&server).await.is_empty());
}

#[test]
fn xai_credential_near_expiry_needs_refresh() {
    let far = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        ..xai_stored_credential()
    };
    assert!(!far.needs_refresh());
    let near = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::minutes(4)),
        ..xai_stored_credential()
    };
    assert!(near.needs_refresh());
}

#[test]
fn xai_production_constants() {
    assert_eq!(XAI_DEVICE_CODE_URL, "https://auth.x.ai/oauth2/device/code");
    assert_eq!(XAI_TOKEN_URL, "https://auth.x.ai/oauth2/token");
    assert_eq!(XAI_CLIENT_ID, "b1a00492-073a-47ea-816f-4c329264a828");
    assert_eq!(
        XAI_SCOPE,
        "openid profile email offline_access grok-cli:access api:access"
    );
    assert_eq!(XaiOAuthProvider::production().id(), "xai");
}

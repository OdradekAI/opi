//! Kimi Coding OAuth flow fixtures and acceptance tests.
//!
//! This target owns device authorization, polling, refresh, cancellation,
//! typed error mapping and Models store ACK. Shared presenter, assembly and
//! Kimi wire fixtures live in `tests/support`.

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
#[path = "support/oauth_kimi.rs"]
#[allow(dead_code)]
mod oauth_kimi;

use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_fixtures::{ScriptedPresenter, expires_within};
use oauth_kimi::*;

#[tokio::test]
async fn kimi_device_login_presents_complete_uri_and_exchanges_strict_triple() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
    let flow = kimi_flow(&server);
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = flow.login(presenter.as_ref()).await.expect("login");

    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
    assert_eq!(credential.refresh.expose_secret(), "kimi-refresh-1");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(!credential.needs_refresh());

    // The presenter gets the public user code and the COMPLETE verification
    // URI, never the device_code.
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "KIMI-CODE".to_owned(),
            "https://auth.kimi.com/device?user_code=KIMI-CODE".to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let start = &requests[0];
    assert_eq!(start.url.path(), "/api/oauth/device_authorization");
    assert_eq!(
        String::from_utf8(start.body.clone()).unwrap(),
        format!("client_id={KIMI_CLIENT_ID}")
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
    let body = String::from_utf8(poll.body.clone()).unwrap();
    assert_eq!(poll.url.path(), "/api/oauth/token");
    assert!(
        body.contains(&format!("client_id={KIMI_CLIENT_ID}")),
        "{body}"
    );
    assert!(body.contains("device_code=kimi-device-1"), "{body}");
    assert!(
        body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{body}"
    );
    assert!(
        poll.headers
            .get("accept")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );
}

#[tokio::test]
async fn kimi_start_non_ok_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(500).set_body_string("secret-start-detail-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization failed"]
    );
    assert!(server.received_requests().await.unwrap().len() == 1);
}

#[tokio::test]
async fn kimi_start_missing_or_empty_fields_fail_typed() {
    for label in ["device_code", "user_code", "verification_uri_complete"] {
        for empty in [false, true] {
            let server = MockServer::start().await;
            let mut body = kimi_start_body(1.into(), 900.into());
            if empty {
                body[label] = serde_json::json!("");
            } else {
                body.as_object_mut().unwrap().remove(label);
            }
            mount_kimi_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = kimi_flow(&server)
                .login(presenter.as_ref())
                .await
                .expect_err("must fail");
            assert!(
                matches!(error, ProviderError::Config(_)),
                "{error:?} for {label} empty={empty}"
            );
        }
    }
}

#[tokio::test]
async fn kimi_start_untrusted_verification_uris_fail_typed() {
    for field in ["verification_uri", "verification_uri_complete"] {
        for bad in ["urn:ietf:oauth:2.0:oob", "file:///etc/passwd"] {
            let server = MockServer::start().await;
            let mut body = kimi_start_body(1.into(), 900.into());
            body[field] = serde_json::json!(bad);
            mount_kimi_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = kimi_flow(&server)
                .login(presenter.as_ref())
                .await
                .expect_err("must fail");
            assert!(
                matches!(error, ProviderError::Config(_)),
                "{error:?} for {field}={bad}"
            );
        }
    }
}

#[test]
fn kimi_parse_defaults_interval_and_expiry_and_rejects_bad_types() {
    // RFC 8628 defaults when the field is absent or not a positive finite
    // number.
    let base = kimi_start_body(serde_json::Value::Null, serde_json::Value::Null);
    for (interval, expect_interval) in [
        (serde_json::Value::Null, 5.0),
        (0.into(), 5.0),
        ((-1).into(), 5.0),
        (2.5.into(), 2.5),
    ] {
        let mut body = base.clone();
        if interval.is_null() {
            body.as_object_mut().unwrap().remove("interval");
        } else {
            body["interval"] = interval;
        }
        let parsed = parse_kimi_device_authorization(&body).expect("parse");
        assert_eq!(parsed.interval_seconds, expect_interval);
        assert_eq!(parsed.expires_in_seconds, 900.0);
    }
    let mut body = base;
    body.as_object_mut().unwrap().remove("expires_in");
    body["expires_in"] = 120.into();
    let parsed = parse_kimi_device_authorization(&body).expect("parse");
    assert_eq!(parsed.expires_in_seconds, 120.0);

    // Non-string fields fail typed.
    let mut body = kimi_start_body(1.into(), 900.into());
    body["user_code"] = serde_json::json!(7);
    assert!(parse_kimi_device_authorization(&body).is_err());
}

#[tokio::test]
async fn kimi_first_poll_waits_one_interval() {
    // interval 5s > deadline 2s: the pre-poll wait sleeps min(5, 2) and the
    // device deadline elapses before any token poll is issued.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(5.into(), 2.into())).await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out"]
    );
    // Only the device_authorization start request; zero polls.
    assert!(kimi_token_requests(&server).await.is_empty());
}

#[tokio::test]
async fn kimi_slow_down_uses_server_interval() {
    // Initial interval 5s, deadline 8s. Poll #1 (t=5) answers slow_down with
    // interval 1: honoring the server value keeps polling every second
    // (t=6, t=7) until the deadline — exactly three polls. The RFC default
    // (+5s -> interval 10) would never poll again after t=5.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(5.into(), 8.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn kimi_slow_down_without_interval_adds_five_seconds() {
    // Initial interval 1s, deadline 10s. Poll #1 (t=1) answers slow_down
    // with no interval: the RFC 8628 §3.5 default adds 5s, so poll #2 lands
    // at t=6 and the 4s remainder hits the deadline — exactly two polls. A
    // smaller increment (say +4s) would fit a third poll at t=9; ignoring
    // slow_down entirely would poll every second.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 10.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 2);
}

#[tokio::test]
async fn kimi_pending_then_success_completes_login() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 30.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let credential = kimi_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn kimi_expired_token_fails_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, serde_json::json!({"error": "expired_token"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
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
async fn kimi_access_denied_fails_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, serde_json::json!({"error": "access_denied"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(
        matches!(error, ProviderError::CredentialRevoked { .. }),
        "{error:?}"
    );
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization denied"]
    );
}

#[tokio::test]
async fn kimi_unknown_error_code_fails_typed_without_echo() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(
        &server,
        400,
        serde_json::json!({
            "error": "server_error",
            "error_description": "secret-detail-CANARY"
        }),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
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
async fn kimi_poll_5xx_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(503).set_body_string("poll-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
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
async fn kimi_success_with_missing_fields_fails_typed() {
    for bad in [
        serde_json::json!({"access_token": "kimi-access-1"}), // missing refresh
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "",
            "expires_in": 3600
        }),
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "kimi-refresh-1",
            "expires_in": 0
        }),
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "kimi-refresh-1",
            "expires_in": "soon"
        }),
    ] {
        let server = MockServer::start().await;
        mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
        mount_kimi_token(&server, 200, bad.clone()).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let error = kimi_flow(&server)
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
async fn kimi_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    register_oauth_assembly_route(
        &mut models,
        Arc::new(kimi_flow(&server)),
        opi_ai::WireApi::OpenAiCompletions,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("kimi-coding", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
    assert_oauth_adapter_request(
        &models,
        "kimi-coding",
        &credential,
        &server,
        opi_ai::WireApi::OpenAiCompletions,
    )
    .await;

    let stored = store
        .read("kimi-coding")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken {
            access, refresh, ..
        } => {
            assert_eq!(access.expose_secret(), "kimi-access-1");
            assert_eq!(refresh.expose_secret(), "kimi-refresh-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    // notify_success fired after the credential existed in the store.
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn kimi_refresh_posts_grant_and_returns_strict_triple() {
    let server = MockServer::start().await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let refreshed = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "kimi-access-1");
    assert_eq!(refreshed.refresh.expose_secret(), "kimi-refresh-1");
    expires_within(refreshed.expires_at.expect("expiry"), 3600);

    let requests = kimi_token_requests(&server).await;
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8(requests[0].body.clone()).unwrap();
    assert!(body.contains("grant_type=refresh_token"), "{body}");
    assert!(body.contains("refresh_token=kimi-old-refresh"), "{body}");
    assert!(
        body.contains(&format!("client_id={KIMI_CLIENT_ID}")),
        "{body}"
    );
    assert!(
        requests[0]
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/x-www-form-urlencoded")
    );
}

#[tokio::test]
async fn kimi_refresh_invalid_grant_and_unauthorized_revoke_without_retry() {
    for (status, body) in [
        (400u16, serde_json::json!({"error": "invalid_grant"})),
        (401, serde_json::json!({"error": "invalid_token"})),
    ] {
        let server = MockServer::start().await;
        mount_kimi_token(&server, status, body).await;
        let error = kimi_flow(&server)
            .refresh(&kimi_stored_credential())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?}"
        );
        // Non-retryable: exactly one request, no backoff retries.
        assert_eq!(kimi_token_requests(&server).await.len(), 1);
    }
}

#[tokio::test]
async fn kimi_refresh_retries_transient_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(502).set_body_string("boom"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let refreshed = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect("refresh succeeds after transient failures");
    assert_eq!(refreshed.access.expose_secret(), "kimi-access-1");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn kimi_refresh_exhausts_retries_fails_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(500).set_body_string("always-CANARY"))
        .mount(&server)
        .await;

    let error = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    // One attempt plus three retries.
    assert_eq!(kimi_token_requests(&server).await.len(), 4);
}

#[tokio::test]
async fn kimi_refresh_non_retryable_terminal_error_fails_without_retry() {
    let server = MockServer::start().await;
    mount_kimi_token(
        &server,
        400,
        serde_json::json!({"error": "invalid_request"}),
    )
    .await;
    let error = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 1);
}

#[test]
fn kimi_production_constants() {
    assert_eq!(KIMI_DEFAULT_OAUTH_HOST, "https://auth.kimi.com");
    assert_eq!(KIMI_CLIENT_ID, "17e5f671-d194-4dfb-9706-5516cb48c098");
    assert_eq!(KimiCodingOAuthProvider::production().id(), "kimi-coding");
}

#[tokio::test]
async fn kimi_oauth_host_trims_trailing_slashes() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let flow = KimiCodingOAuthProvider::new_with_oauth_host(format!("{}/", server.uri()));
    let credential = flow
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login through the trimmed host");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
}

#[tokio::test]
async fn kimi_cancellation_during_device_presentation_is_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
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
                    provider_id: "kimi-coding".into(),
                })
            })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = kimi_flow(&server)
        .login(&CancellingPresenter)
        .await
        .expect_err("must cancel");
    assert!(
        matches!(error, ProviderError::LoginCancelled { .. }),
        "{error:?}"
    );
    // The device start already happened; no token poll follows a cancel.
    assert!(kimi_token_requests(&server).await.is_empty());
}

#[test]
fn kimi_credential_near_expiry_needs_refresh() {
    let far = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        ..kimi_stored_credential()
    };
    assert!(!far.needs_refresh());
    let near = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::minutes(4)),
        ..kimi_stored_credential()
    };
    assert!(near.needs_refresh());
}

// ---------------------------------------------------------------------------

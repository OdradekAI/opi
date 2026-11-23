//! Meta OAuth flow fixtures and acceptance tests.
//!
//! This target owns device authorization, polling, key minting, refresh,
//! cancellation, typed errors, expiry and Models store ACK. Shared presenter,
//! assembly and Meta wire fixtures live in `tests/support`.

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
#[path = "support/oauth_meta.rs"]
#[allow(dead_code)]
mod oauth_meta;

use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_fixtures::{ScriptedPresenter, expires_within};
use oauth_meta::*;

#[tokio::test]
async fn meta_device_login_presents_complete_uri_notifies_progress_and_mints_api_key() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");

    // The minted Model API key is the access token; the identity token is
    // stored as refresh for re-mint; expiry is the fixed 24h client-side
    // lifetime (no server expires_in on this wire).
    assert_eq!(credential.access.expose_secret(), "meta-api-key-1");
    assert_eq!(credential.refresh.expose_secret(), "meta-identity-1");
    expires_within(credential.expires_at.expect("expiry"), 24 * 60 * 60);
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(!credential.needs_refresh());

    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "META-CODE".to_owned(),
            "https://auth.meta.com/device?user_code=META-CODE".to_owned()
        )]
    );
    assert_eq!(
        presenter.progress.lock().unwrap().as_slice(),
        &["Enabling Meta Model API access...".to_owned()]
    );

    let requests = server.received_requests().await.unwrap();
    let start = &requests[0];
    assert_eq!(start.url.path(), "/oidc/device/authorization/");
    assert_eq!(
        String::from_utf8(start.body.clone()).unwrap(),
        format!("client_id={META_CLIENT_ID}")
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

    let poll = &requests[1];
    assert_eq!(poll.url.path(), "/oidc/device/token/");
    let body = String::from_utf8(poll.body.clone()).unwrap();
    assert!(
        body.contains(&format!("client_id={META_CLIENT_ID}")),
        "{body}"
    );
    assert!(body.contains("device_code=meta-device-1"), "{body}");
    assert!(
        body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{body}"
    );

    let mint = &requests[2];
    assert_eq!(mint.url.path(), "/muse-code/key");
    assert_eq!(
        String::from_utf8(mint.body.clone()).unwrap(),
        "{}",
        "mint body is the empty JSON object"
    );
    assert_eq!(
        mint.headers.get("authorization").unwrap().to_str().unwrap(),
        "Bearer meta-identity-1"
    );
    assert_eq!(
        mint.headers.get("x-api-version").unwrap().to_str().unwrap(),
        "1.0.0"
    );
    assert!(
        mint.headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );
}

#[tokio::test]
async fn meta_login_uses_plain_verification_uri_without_complete() {
    let server = MockServer::start().await;
    let mut body = meta_start_body(1.into(), 900.into());
    body.as_object_mut()
        .unwrap()
        .remove("verification_uri_complete");
    mount_meta_start(&server, body).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "meta-api-key-1");
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "META-CODE".to_owned(),
            "https://auth.meta.com/device".to_owned()
        )]
    );
}

#[tokio::test]
async fn meta_untrusted_complete_uri_falls_back_to_plain() {
    // pi coalesces at the trust level: an untrusted verification_uri_complete
    // is null, so the plain verification_uri wins.
    let server = MockServer::start().await;
    let mut body = meta_start_body(1.into(), 900.into());
    body["verification_uri_complete"] = serde_json::json!("file:///etc/passwd");
    mount_meta_start(&server, body).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "meta-api-key-1");
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "META-CODE".to_owned(),
            "https://auth.meta.com/device".to_owned()
        )]
    );
}

#[tokio::test]
async fn meta_start_non_ok_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oidc/device/authorization/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("secret-start-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization failed"]
    );
}

#[tokio::test]
async fn meta_start_missing_or_empty_fields_fail_typed() {
    // A single missing verification URI is fine (the other one wins); both
    // missing or untrusted fails typed.
    let mut both_missing = meta_start_body(1.into(), 900.into());
    both_missing
        .as_object_mut()
        .unwrap()
        .remove("verification_uri");
    both_missing
        .as_object_mut()
        .unwrap()
        .remove("verification_uri_complete");
    for (label, body) in [
        ("device_code", meta_start_body(1.into(), 900.into())),
        ("user_code", meta_start_body(1.into(), 900.into())),
        ("both-uris", both_missing),
    ] {
        for empty in [false, true] {
            let mut body = body.clone();
            if empty {
                if label == "both-uris" {
                    body["verification_uri"] = serde_json::json!("");
                    body["verification_uri_complete"] = serde_json::json!("");
                } else {
                    body[label] = serde_json::json!("");
                }
            } else if label != "both-uris" {
                body.as_object_mut().unwrap().remove(label);
            }
            let server = MockServer::start().await;
            mount_meta_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = meta_flow(&server)
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

#[test]
fn meta_parse_defaults_interval_and_expiry_and_chooses_uri() {
    let base = meta_start_body(serde_json::Value::Null, serde_json::Value::Null);
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
        let parsed = parse_meta_device_authorization(&body).expect("parse");
        assert_eq!(parsed.interval_seconds, expect_interval);
        assert_eq!(parsed.expires_in_seconds, 900.0);
    }

    // The complete URI wins when both are trusted; the plain URI wins when
    // the complete one is missing or untrusted.
    let both = parse_meta_device_authorization(&base).expect("parse");
    assert_eq!(
        both.verification_uri,
        "https://auth.meta.com/device?user_code=META-CODE"
    );
    let mut plain_only = base.clone();
    plain_only
        .as_object_mut()
        .unwrap()
        .remove("verification_uri_complete");
    assert_eq!(
        parse_meta_device_authorization(&plain_only)
            .expect("parse")
            .verification_uri,
        "https://auth.meta.com/device"
    );
    let mut bad_complete = base;
    bad_complete["verification_uri_complete"] = serde_json::json!("urn:ietf:oauth:2.0:oob");
    assert_eq!(
        parse_meta_device_authorization(&bad_complete)
            .expect("parse")
            .verification_uri,
        "https://auth.meta.com/device"
    );

    let mut body = meta_start_body(1.into(), 900.into());
    body["user_code"] = serde_json::json!(7);
    assert!(parse_meta_device_authorization(&body).is_err());
}

#[tokio::test]
async fn meta_first_poll_waits_one_interval() {
    // interval 5s > deadline 2s: the pre-poll wait sleeps min(5, 2) and the
    // device deadline elapses before any token poll or mint is issued.
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(5.into(), 2.into())).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out"]
    );
    assert!(requests_at(&server, "/oidc/device/token/").await.is_empty());
    assert!(requests_at(&server, "/muse-code/key").await.is_empty());
}

#[tokio::test]
async fn meta_slow_down_uses_server_interval() {
    // Initial interval 5s, deadline 8s. Poll #1 (t=5) answers slow_down with
    // interval 1: honoring the server value keeps polling every second
    // (t=6, t=7) until the deadline — exactly three polls.
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(5.into(), 8.into())).await;
    Mock::given(method("POST"))
        .and(path("/oidc/device/token/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_meta_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(requests_at(&server, "/oidc/device/token/").await.len(), 3);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn meta_slow_down_without_interval_adds_five_seconds() {
    // Initial interval 1s, deadline 10s. Poll #1 (t=1) answers slow_down
    // with no interval: the RFC 8628 §3.5 default adds 5s, so poll #2 lands
    // at t=6 and the 4s remainder hits the deadline — exactly two polls.
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 10.into())).await;
    Mock::given(method("POST"))
        .and(path("/oidc/device/token/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_meta_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(requests_at(&server, "/oidc/device/token/").await.len(), 2);
}

#[tokio::test]
async fn meta_pending_then_success_mints() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 30.into())).await;
    Mock::given(method("POST"))
        .and(path("/oidc/device/token/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;

    let credential = meta_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "meta-api-key-1");
    assert_eq!(requests_at(&server, "/oidc/device/token/").await.len(), 3);
    assert_eq!(requests_at(&server, "/muse-code/key").await.len(), 1);
}

#[tokio::test]
async fn meta_expired_token_fails_typed_without_mint() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, serde_json::json!({"error": "expired_token"})).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
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
    assert!(requests_at(&server, "/muse-code/key").await.is_empty());
}

#[tokio::test]
async fn meta_access_denied_fails_typed() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, serde_json::json!({"error": "access_denied"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
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
async fn meta_unknown_error_code_fails_typed_without_echo() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(
        &server,
        400,
        serde_json::json!({
            "error": "server_error",
            "error_description": "secret-detail-CANARY"
        }),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
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
async fn meta_poll_5xx_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    Mock::given(method("POST"))
        .and(path("/oidc/device/token/"))
        .respond_with(ResponseTemplate::new(503).set_body_string("poll-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
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
async fn meta_poll_ok_without_access_token_fails_typed() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, serde_json::json!({})).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    // pi's switch has no code for this body: the status-class failure wins.
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device token request failed"]
    );
    // No key mint happens without an identity token.
    assert!(requests_at(&server, "/muse-code/key").await.is_empty());
}

#[tokio::test]
async fn meta_mint_401_403_revokes_for_relogin() {
    for status in [401u16, 403] {
        let server = MockServer::start().await;
        mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
        mount_meta_token(&server, 200, meta_identity_success()).await;
        Mock::given(method("POST"))
            .and(path("/muse-code/key"))
            .respond_with(ResponseTemplate::new(status).set_body_string("session-detail-CANARY"))
            .mount(&server)
            .await;
        let presenter = Arc::new(ScriptedPresenter::default());

        let error = meta_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {status}"
        );
        assert!(!format!("{error:?}").contains("CANARY"));
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["meta session expired; sign in again"]
        );
        assert_eq!(requests_at(&server, "/muse-code/key").await.len(), 1);
    }
}

#[tokio::test]
async fn meta_mint_non_ok_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    Mock::given(method("POST"))
        .and(path("/muse-code/key"))
        .respond_with(ResponseTemplate::new(500).set_body_string("mint-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = meta_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["model api key mint failed"]
    );
}

#[tokio::test]
async fn meta_mint_missing_api_key_surfaces_trusted_action_url() {
    // Without an api_key the setup action_url — but only a trusted http(s)
    // one — reaches the presenter reason.
    for (action_url, expect_url) in [
        ("https://meta.com/finish-setup", true),
        ("file:///etc/passwd", false),
        ("urn:ietf:oauth:2.0:oob", false),
    ] {
        let server = MockServer::start().await;
        mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
        mount_meta_token(&server, 200, meta_identity_success()).await;
        mount_meta_mint(&server, 200, serde_json::json!({"action_url": action_url})).await;
        let presenter = Arc::new(ScriptedPresenter::default());

        let error = meta_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "{error:?} for {action_url}"
        );
        let failures = presenter.failures.lock().unwrap().clone();
        assert_eq!(failures.len(), 1, "{failures:?} for {action_url}");
        if expect_url {
            assert_eq!(
                failures[0],
                "model api key response invalid; complete setup at https://meta.com/finish-setup"
            );
        } else {
            assert_eq!(failures[0], "model api key response invalid");
        }
    }
}

#[tokio::test]
async fn meta_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    register_oauth_assembly_route(
        &mut models,
        Arc::new(meta_flow(&server)),
        opi_ai::WireApi::OpenAiResponses,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("meta", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "meta-api-key-1");
    assert_oauth_adapter_request(
        &models,
        "meta",
        &credential,
        &server,
        opi_ai::WireApi::OpenAiResponses,
    )
    .await;

    let stored = store
        .read("meta")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken {
            access, refresh, ..
        } => {
            assert_eq!(access.expose_secret(), "meta-api-key-1");
            assert_eq!(refresh.expose_secret(), "meta-identity-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

fn meta_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("meta-old-api-key".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("meta-identity-1".to_owned().into_boxed_str()),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}

#[tokio::test]
async fn meta_refresh_remints_with_same_identity() {
    let server = MockServer::start().await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;

    let refreshed = meta_flow(&server)
        .refresh(&meta_stored_credential())
        .await
        .expect("remint");
    assert_eq!(refreshed.access.expose_secret(), "meta-api-key-1");
    // The identity token survives the re-mint: it is the renewable secret.
    assert_eq!(refreshed.refresh.expose_secret(), "meta-identity-1");
    expires_within(refreshed.expires_at.expect("expiry"), 24 * 60 * 60);

    let mints = requests_at(&server, "/muse-code/key").await;
    assert_eq!(mints.len(), 1, "exactly one re-mint");
    assert_eq!(
        mints[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer meta-identity-1"
    );
    // The identity token is never sent to a refresh_token grant: no token
    // endpoint request happens at all (auth.meta.com answers 404 there).
    assert!(requests_at(&server, "/oidc/device/token/").await.is_empty());
}

#[tokio::test]
async fn meta_refresh_mint_401_403_revokes_without_retry() {
    for status in [401u16, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/muse-code/key"))
            .respond_with(ResponseTemplate::new(status).set_body_string("dead-CANARY"))
            .mount(&server)
            .await;
        let error = meta_flow(&server)
            .refresh(&meta_stored_credential())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {status}"
        );
        assert_eq!(requests_at(&server, "/muse-code/key").await.len(), 1);
    }
}

#[tokio::test]
async fn meta_refresh_mint_transient_failure_does_not_retry() {
    // pi's Meta flow has no retry policy on the mint path: one typed
    // failure, one request.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/muse-code/key"))
        .respond_with(ResponseTemplate::new(500).set_body_string("mint-CANARY"))
        .mount(&server)
        .await;
    let error = meta_flow(&server)
        .refresh(&meta_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(requests_at(&server, "/muse-code/key").await.len(), 1);
}

#[tokio::test]
async fn meta_cancellation_during_device_presentation_is_typed() {
    let server = MockServer::start().await;
    mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
    mount_meta_token(&server, 200, meta_identity_success()).await;
    mount_meta_mint(&server, 200, meta_mint_success()).await;
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
                    provider_id: "meta".into(),
                })
            })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = meta_flow(&server)
        .login(&CancellingPresenter)
        .await
        .expect_err("must cancel");
    assert!(
        matches!(error, ProviderError::LoginCancelled { .. }),
        "{error:?}"
    );
    assert!(requests_at(&server, "/oidc/device/token/").await.is_empty());
    assert!(requests_at(&server, "/muse-code/key").await.is_empty());
}

#[test]
fn meta_production_constants() {
    assert_eq!(META_AUTH_HOST, "https://auth.meta.com");
    assert_eq!(META_API_KEY_MINT_URL, "https://api.meta.ai/muse-code/key");
    assert_eq!(META_CLIENT_ID, "1031625952748946");
    assert_eq!(MetaOAuthProvider::production().id(), "meta");
}

#[test]
fn meta_credential_near_expiry_needs_refresh() {
    let far = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        ..meta_stored_credential()
    };
    assert!(!far.needs_refresh());
    let near = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::minutes(4)),
        ..meta_stored_credential()
    };
    assert!(near.needs_refresh());
}

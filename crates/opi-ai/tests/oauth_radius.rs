//! Radius OAuth gateway flow fixtures and acceptance tests.
//!
//! This target owns Radius discovery, browser and device login, token exchange,
//! refresh and custom identity binding. Shared OAuth presenters, assembly values
//! and Radius HTTP fixtures are imported from `tests/support`.

use std::sync::Arc;

use opi_ai::Models;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
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
#[path = "support/oauth_radius.rs"]
mod oauth_radius;

use oauth_assembly::oauth_assembly_request;
use oauth_fixtures::{ScriptedPresenter, expires_within, percent_decode};
use oauth_radius::*;

fn form_pairs(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

fn radius_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("radius-old-access".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("radius-old-refresh".to_owned().into_boxed_str()),
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
async fn radius_browser_login_discovers_endpoint_and_exchanges_code() {
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    let authorize = format!("{}/authorize", server.uri());
    mount_radius_discovery(&server, &authorize).await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = radius_flow(&server);
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    drive_radius_callback(&presenter, "rad-code-1").await;
    let credential = login.await.expect("join").expect("login failed");

    assert_eq!(credential.access.expose_secret(), "radius-access-1");
    assert_eq!(credential.refresh.expose_secret(), "radius-refresh-1");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert!(!credential.needs_refresh());

    // The authorize URL carries the fixed client, loopback
    // redirect, scope, handoff, S256 PKCE, and a CSRF state.
    let url = presenter.last_authorize_url();
    assert_eq!(
        ScriptedPresenter::query_param(&url, "redirect_uri").as_deref(),
        Some("http://127.0.0.1:1456/oauth/callback")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "client_id").as_deref(),
        Some(RADIUS_OAUTH_CLIENT_ID)
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "scope").as_deref(),
        Some(RADIUS_OAUTH_SCOPE)
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "handoff").as_deref(),
        Some("url")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert!(ScriptedPresenter::query_param(&url, "state").unwrap().len() >= 16);

    // The token exchange uses authorization_code + verifier
    // + the same fixed redirect and client id.
    let requests = server.received_requests().await.unwrap();
    let token = requests
        .iter()
        .find(|r| r.url.path() == "/v1/oauth/token")
        .expect("token request");
    let form = String::from_utf8(token.body.clone()).unwrap();
    assert!(form.contains("grant_type=authorization_code"), "{form}");
    assert!(form.contains("code=rad-code-1"), "{form}");
    assert!(form.contains("code_verifier="), "{form}");
    assert!(form.contains("client_id=pi-gateway"), "{form}");
    assert!(
        form.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1456%2Foauth%2Fcallback"),
        "{form}"
    );
    assert_eq!(
        presenter.offered_methods.lock().unwrap().as_slice(),
        &[OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode]
    );
}

#[tokio::test]
async fn radius_browser_never_starts_a_manual_reader() {
    struct CallbackPresenter {
        inner: ScriptedPresenter,
        manual_calls: std::sync::atomic::AtomicUsize,
    }
    impl LoginPresenter for CallbackPresenter {
        fn present_auth_url<'a>(
            &'a self,
            url: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            self.inner.present_auth_url(url)
        }
        fn present_device_code<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            self.manual_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async {
                Err(ProviderError::Config(
                    opi_ai::provider::ProviderErrorSummary::redacted(),
                ))
            })
        }
        fn notify_success(&self) {
            self.inner.notify_success();
        }
        fn notify_failure(&self, reason: &str) {
            self.inner.notify_failure(reason);
        }
    }
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    mount_radius_discovery(&server, &format!("{}/authorize", server.uri())).await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = CallbackPresenter {
        inner: ScriptedPresenter::default(),
        manual_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    let flow = radius_flow(&server);
    let (credential, _) = tokio::join!(
        flow.login(&presenter),
        drive_radius_callback(&presenter.inner, "rad-code-1")
    );
    assert_eq!(
        credential.expect("callback login").access.expose_secret(),
        "radius-access-1"
    );
    assert_eq!(
        presenter
            .manual_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn radius_discovery_non_ok_fails_typed_without_presenting_or_echo() {
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/oauth"))
        .respond_with(ResponseTemplate::new(500).set_body_string("discovery-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert!(presenter.authorize_urls.lock().unwrap().is_empty());
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["radius oauth discovery failed"]
    );
    // Browser mode discovers before anything else: no token traffic.
    assert!(radius_token_requests(&server).await.is_empty());
}

#[tokio::test]
async fn radius_discovery_missing_endpoint_fails_typed() {
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    for body in [
        serde_json::json!({}),
        serde_json::json!({ "authorizationEndpoint": 7 }),
        serde_json::json!({ "authorizationEndpoint": "" }),
    ] {
        Mock::given(method("GET"))
            .and(path("/v1/oauth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&server)
            .await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let error = radius_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::Config(_) | ProviderError::AuthFailed(_)
            ),
            "{error:?} for {body}"
        );
        assert!(presenter.authorize_urls.lock().unwrap().is_empty());
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["radius oauth discovery failed"]
        );
        server.reset().await;
    }
}

#[tokio::test]
async fn radius_browser_token_non_ok_fails_typed_without_echo() {
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    let authorize = format!("{}/authorize", server.uri());
    mount_radius_discovery(&server, &authorize).await;
    mount_radius_token(
        &server,
        400,
        serde_json::json!({"error": "invalid_grant", "error_description": "detail-CANARY"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = radius_flow(&server);
    let (result, _) = tokio::join!(
        flow.login(presenter.as_ref()),
        drive_radius_callback(presenter.as_ref(), "rejected-radius-code")
    );
    let error = result.expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["token exchange failed"]
    );
    assert_eq!(radius_token_requests(&server).await.len(), 1);
}

#[tokio::test]
async fn radius_device_login_polls_and_completes() {
    let server = MockServer::start().await;
    mount_radius_device_start(&server, radius_device_start(1.into())).await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let credential = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "radius-access-1");
    assert_eq!(credential.refresh.expose_secret(), "radius-refresh-1");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "RAD-CODE".to_owned(),
            "https://radius.example/device".to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let start = requests
        .iter()
        .find(|r| r.url.path() == "/v1/oauth/device")
        .expect("device start");
    let form = String::from_utf8(start.body.clone()).unwrap();
    assert!(form.contains("client_id=pi-gateway"), "{form}");
    assert!(form.contains("scope=gateway+offline_access"), "{form}");

    let poll = &requests[1];
    let poll_form = String::from_utf8(poll.body.clone()).unwrap();
    assert!(
        poll_form.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{poll_form}"
    );
    assert!(
        poll_form.contains("device_code=radius-device-1"),
        "{poll_form}"
    );
    assert!(poll_form.contains("client_id=pi-gateway"), "{poll_form}");
}

/// Radius authentication is usable before the pi-messages generation
/// adapter is delivered. An empty catalog exposes that dispatch boundary.
#[tokio::test]
async fn radius_public_login_refresh_and_auth_keep_the_configured_gateway() {
    let _guard = radius_port_guard().await;
    let gateway = MockServer::start().await;
    let interactive = MockServer::start().await;
    mount_radius_discovery(&gateway, &format!("{}/authorize", interactive.uri())).await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "radius-login", "refresh_token": "radius-refresh", "expires_in": 1,
        })))
        .up_to_n_times(1)
        .mount(&gateway)
        .await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    models
        .register_oauth_route(
            Box::new(opi_ai::test_support::MockProvider::new_with_models(
                "radius",
                vec![],
                vec![],
            )),
            Arc::new(radius_flow(&gateway)),
            Default::default(),
        )
        .expect("auth route without a generation catalog");
    let models = Arc::new(models);
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let models = models.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { models.login("radius", presenter.as_ref()).await })
    };
    drive_radius_callback(&presenter, "radius-code").await;
    let credential = login
        .await
        .unwrap()
        .expect("acknowledged Radius browser login");
    assert_eq!(credential.access.expose_secret(), "radius-login");
    assert!(
        presenter
            .last_authorize_url()
            .starts_with(&interactive.uri())
    );
    assert!(presenter.success_at.lock().unwrap().is_some());
    assert!(
        matches!(store.read("radius").await.expect("stored Radius credential"), Some(Credential::OAuthToken { access, .. }) if access.expose_secret() == "radius-login")
    );

    mount_radius_token(&gateway, 200, serde_json::json!({
        "access_token": "radius-refreshed", "refresh_token": "radius-rotated", "expires_in": 3600,
    })).await;
    let refreshed = models
        .get_auth("radius", Default::default())
        .await
        .expect("public Radius refresh")
        .expect("identity");
    assert_eq!(refreshed.secret.expose_secret(), "radius-refreshed");
    assert!(matches!(refreshed.scheme, opi_ai::auth::AuthScheme::Bearer));
    assert!(
        matches!(&refreshed.provenance.source, opi_ai::auth::AuthProvenanceSource::OAuth { kind } if kind == "radius")
    );
    assert!(
        refreshed.base_url.is_none(),
        "gateway is immutable provider state, not token metadata"
    );
    assert!(refreshed.account_id.is_none());
    assert!(
        matches!(store.read("radius").await.expect("stored Radius credential"), Some(Credential::OAuthToken { access, refresh, .. })
        if access.expose_secret() == "radius-refreshed" && refresh.expose_secret() == "radius-rotated")
    );
    let error = models
        .prepare_call(
            "radius:unavailable-pi-messages",
            oauth_assembly_request("radius"),
        )
        .await
        .expect_err("generation adapter is not registered");
    assert!(matches!(
        error,
        opi_ai::ModelsError::Registry(opi_ai::registry::RegistryError::UnknownModel { .. })
    ));
    assert!(interactive.received_requests().await.unwrap().is_empty());
    let requests = gateway.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == "/v1/oauth")
            .count(),
        1
    );
    let tokens: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(tokens.len(), 2);
    let refresh_form = form_pairs(std::str::from_utf8(&tokens[1].body).unwrap());
    assert!(refresh_form.contains(&("grant_type".into(), "refresh_token".into())));
    assert!(refresh_form.contains(&("refresh_token".into(), "radius-refresh".into())));
    assert!(refresh_form.contains(&("client_id".into(), "pi-gateway".into())));
}

#[test]
fn radius_parse_device_defaults_interval_and_keeps_uri_verbatim() {
    // interval absent/non-positive falls back to the shared 5s default; the
    // verification URI passes through verbatim without scheme validation, but
    // every field must be a non-empty value and
    // expires_in positive finite.
    let mut body = radius_device_start(serde_json::Value::Null);
    body.as_object_mut().unwrap().remove("interval");
    let parsed = parse_radius_device_authorization(&body).expect("parse");
    assert_eq!(parsed.interval_seconds, 5.0);
    assert_eq!(parsed.verification_uri, "https://radius.example/device");
    assert_eq!(parsed.expires_in_seconds, 900.0);

    let parsed = parse_radius_device_authorization(&radius_device_start(2.into())).expect("parse");
    assert_eq!(parsed.interval_seconds, 2.0);

    let parsed = parse_radius_device_authorization(&radius_device_start(0.into())).expect("parse");
    assert_eq!(parsed.interval_seconds, 5.0);

    let mut odd = radius_device_start(1.into());
    odd["verification_uri"] = serde_json::json!("ftp://radius.example/dev");
    let parsed = parse_radius_device_authorization(&odd).expect("parse");
    assert_eq!(parsed.verification_uri, "ftp://radius.example/dev");

    for label in ["device_code", "user_code", "verification_uri", "expires_in"] {
        let mut b = radius_device_start(1.into());
        b.as_object_mut().unwrap().remove(label);
        assert!(
            parse_radius_device_authorization(&b).is_err(),
            "{label} must be required"
        );
        let mut empty = radius_device_start(1.into());
        if label == "expires_in" {
            empty[label] = serde_json::json!(0);
        } else {
            empty[label] = serde_json::json!("");
        }
        assert!(
            parse_radius_device_authorization(&empty).is_err(),
            "{label} empty must fail"
        );
    }
}

#[tokio::test]
async fn radius_device_start_missing_fields_fail_typed() {
    for label in ["device_code", "user_code", "verification_uri", "expires_in"] {
        let server = MockServer::start().await;
        let mut body = radius_device_start(1.into());
        body.as_object_mut().unwrap().remove(label);
        mount_radius_device_start(&server, body).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
        let error = radius_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::Config(_) | ProviderError::AuthFailed(_)
            ),
            "{error:?} for {label}"
        );
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["device authorization response invalid"]
        );
        assert!(radius_token_requests(&server).await.is_empty());
    }
}

#[tokio::test]
async fn radius_device_start_non_ok_fails_typed_without_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/device"))
        .respond_with(ResponseTemplate::new(500).set_body_string("device-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = radius_flow(&server)
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
async fn radius_first_poll_waits_one_interval() {
    let server = MockServer::start().await;
    // expires_in 2s < first wait 5s: the deadline hits before any poll.
    Mock::given(method("POST"))
        .and(path("/v1/oauth/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json({
            let mut b = radius_device_start(5.into());
            b["expires_in"] = serde_json::json!(2);
            b
        }))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out"]
    );
    assert!(radius_token_requests(&server).await.is_empty());
}

#[tokio::test]
async fn radius_slow_down_uses_server_interval() {
    // interval 5s, deadline 8s: poll #1 (t=5) slow_down with interval 1 ->
    // poll every second (t=6, t=7) until the deadline. Exactly 3 polls.
    let server = MockServer::start().await;
    let mut start = radius_device_start(5.into());
    start["expires_in"] = serde_json::json!(8);
    mount_radius_device_start(&server, start).await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_radius_token(
        &server,
        400,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(radius_token_requests(&server).await.len(), 3);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn radius_slow_down_without_interval_adds_five_seconds() {
    // interval 1s, deadline 10s: poll #1 (t=1) slow_down with no interval ->
    // +5s default, poll #2 at t=6, 4s remainder hits the deadline. 2 polls.
    let server = MockServer::start().await;
    let mut start = radius_device_start(1.into());
    start["expires_in"] = serde_json::json!(10);
    mount_radius_device_start(&server, start).await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_radius_token(
        &server,
        400,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(radius_token_requests(&server).await.len(), 2);
}

#[tokio::test]
async fn radius_pending_then_success_completes_login() {
    let server = MockServer::start().await;
    mount_radius_device_start(&server, radius_device_start(1.into())).await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let credential = radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "radius-access-1");
    assert_eq!(radius_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn radius_expired_and_denied_fail_typed() {
    for (code, reason) in [
        ("expired_token", "device code expired"),
        ("access_denied", "device authorization denied"),
    ] {
        let server = MockServer::start().await;
        mount_radius_device_start(&server, radius_device_start(1.into())).await;
        mount_radius_token(
            &server,
            400,
            serde_json::json!({"error": code, "error_description": "detail-CANARY"}),
        )
        .await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

        let error = radius_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {code}"
        );
        assert!(!format!("{error:?}").contains("CANARY"));
        assert_eq!(presenter.failures.lock().unwrap().as_slice(), &[reason]);
        assert_eq!(radius_token_requests(&server).await.len(), 1);
    }
}

#[tokio::test]
async fn radius_unknown_error_and_5xx_fail_typed_without_echo() {
    for (status, body) in [
        (
            400u16,
            serde_json::json!({"error": "server_error", "error_description": "detail-CANARY"}),
        ),
        (503u16, serde_json::json!("poll-CANARY")),
    ] {
        let server = MockServer::start().await;
        mount_radius_device_start(&server, radius_device_start(1.into())).await;
        mount_radius_token(&server, status, body).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

        let error = radius_flow(&server)
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
}

#[tokio::test]
async fn radius_ok_missing_fields_fail_typed() {
    for bad in [
        serde_json::json!({}),
        serde_json::json!({"refresh_token": "radius-refresh-1", "expires_in": 3600}),
        serde_json::json!({"access_token": "", "refresh_token": "r", "expires_in": 3600}),
        serde_json::json!({"access_token": "a"}),
        serde_json::json!({"access_token": "a", "refresh_token": "r", "expires_in": 0}),
        serde_json::json!({"access_token": "a", "refresh_token": "r", "expires_in": "soon"}),
    ] {
        let server = MockServer::start().await;
        mount_radius_device_start(&server, radius_device_start(1.into())).await;
        mount_radius_token(&server, 200, bad.clone()).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
        let error = radius_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::Config(_) | ProviderError::AuthFailed(_)
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
async fn radius_device_mode_skips_discovery() {
    // Device mode never fetches /v1/oauth discovery.
    let server = MockServer::start().await;
    mount_radius_device_start(&server, radius_device_start(1.into())).await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    radius_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect("login");
    let paths: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.url.path().to_owned())
        .collect();
    assert!(!paths.contains(&"/v1/oauth".to_owned()), "{paths:?}");
}

#[tokio::test]
async fn radius_refresh_rotates_strict_triple() {
    let server = MockServer::start().await;
    mount_radius_token(
        &server,
        200,
        serde_json::json!({
            "access_token": "radius-access-2",
            "refresh_token": "radius-refresh-2",
            "expires_in": 7200,
        }),
    )
    .await;
    let cred = radius_stored_credential();

    let refreshed = radius_flow(&server).refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "radius-access-2");
    assert_eq!(refreshed.refresh.expose_secret(), "radius-refresh-2");
    expires_within(refreshed.expires_at.expect("expiry"), 7200);

    let requests = radius_token_requests(&server).await;
    assert_eq!(requests.len(), 1, "refresh is a single pass");
    let form = String::from_utf8(requests[0].body.clone()).unwrap();
    assert!(form.contains("grant_type=refresh_token"), "{form}");
    assert!(form.contains("client_id=pi-gateway"), "{form}");
    assert!(form.contains("refresh_token=radius-old-refresh"), "{form}");
}

#[tokio::test]
async fn radius_refresh_missing_fields_fail_strict() {
    // Radius requires the full access, refresh and expiry triple.
    for bad in [
        serde_json::json!({"access_token": "radius-access-2", "expires_in": 3600}),
        serde_json::json!({"access_token": "radius-access-2", "refresh_token": "r"}),
    ] {
        let server = MockServer::start().await;
        mount_radius_token(&server, 200, bad.clone()).await;
        let error = radius_flow(&server)
            .refresh(&radius_stored_credential())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::Config(_) | ProviderError::AuthFailed(_)
            ),
            "{error:?} for {bad}"
        );
    }
}

#[tokio::test]
async fn radius_refresh_401_403_revokes_for_relogin() {
    for status in [401u16, 403] {
        let server = MockServer::start().await;
        mount_radius_token(
            &server,
            status,
            serde_json::json!({"error": "invalid_grant", "error_description": "detail-CANARY"}),
        )
        .await;
        let error = radius_flow(&server)
            .refresh(&radius_stored_credential())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?} for {status}"
        );
        assert!(!format!("{error:?}").contains("CANARY"));
    }
}

#[tokio::test]
async fn radius_refresh_non_ok_fails_typed_without_retry() {
    let server = MockServer::start().await;
    mount_radius_token(
        &server,
        500,
        serde_json::json!({"error": "server_error", "error_description": "detail-CANARY"}),
    )
    .await;
    let error = radius_flow(&server)
        .refresh(&radius_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(radius_token_requests(&server).await.len(), 1);
}

#[test]
fn radius_production_constants() {
    assert_eq!(RADIUS_DEFAULT_GATEWAY, "https://radius.pi.dev");
    assert_eq!(RADIUS_OAUTH_CLIENT_ID, "pi-gateway");
    assert_eq!(RADIUS_OAUTH_SCOPE, "gateway offline_access");
    assert_eq!(RADIUS_CALLBACK_PORT, 1456);
    assert_eq!(RadiusOAuthProvider::production().id(), "radius");
}

#[tokio::test]
async fn radius_custom_flow_identity_gateway_and_account_remain_bound_on_refresh() {
    let gateway_a = MockServer::start().await;
    let gateway_b = MockServer::start().await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    for (gateway, id, name, account, access, refresh, refreshed) in [
        (
            &gateway_a,
            "radius-a",
            "Radius A",
            "account-a",
            "login-a",
            "refresh-a",
            "refreshed-a",
        ),
        (
            &gateway_b,
            "radius-b",
            "Radius B",
            "account-b",
            "login-b",
            "refresh-b",
            "refreshed-b",
        ),
    ] {
        mount_radius_device_start(gateway, radius_device_start(1.into())).await;
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .and(wiremock::matchers::body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": access, "refresh_token": refresh, "expires_in": 1,
            })))
            .expect(1)
            .mount(gateway)
            .await;
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(wiremock::matchers::body_string_contains("grant_type=refresh_token"))
            .and(wiremock::matchers::body_string_contains(format!("refresh_token={refresh}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": refreshed, "refresh_token": format!("rotated-{refresh}"), "expires_in": 3600,
            }))).expect(1).mount(gateway).await;
        let flow: Arc<dyn OAuthProvider> = Arc::new(custom_radius_flow(gateway.uri(), id, name));
        assert_eq!(flow.id(), id);
        assert_eq!(flow.name(), name);
        models
            .register_oauth_route(
                Box::new(opi_ai::test_support::MockProvider::new_with_models(
                    id,
                    vec![],
                    vec![],
                )),
                flow,
                Default::default(),
            )
            .unwrap();
        let presenter = ScriptedPresenter::default();
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
        let credential = models
            .login(id, &presenter)
            .await
            .expect("custom Radius login ACK");
        assert_eq!(credential.access.expose_secret(), access);
        assert_eq!(
            presenter.selected_provider_ids.lock().unwrap().as_slice(),
            &[id.to_owned()]
        );
        // Radius does not infer an account id from its opaque access token.
        // A host-supplied binding survives same-source refresh.
        let mut stored: Credential = credential.into();
        if let Credential::OAuthToken {
            account_id,
            base_url,
            ..
        } = &mut stored
        {
            *account_id = Some(account.to_owned());
            *base_url = Some("http://127.0.0.1:1/untrusted-token-endpoint".into());
        }
        store
            .modify(id, opi_ai::write_mutation(stored))
            .await
            .unwrap();
    }
    assert!(store.read("radius").await.unwrap().is_none());
    for (id, account, access) in [
        ("radius-a", "account-a", "refreshed-a"),
        ("radius-b", "account-b", "refreshed-b"),
    ] {
        let resolved = models
            .get_auth(id, Default::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.secret.expose_secret(), access);
        assert_eq!(resolved.account_id.as_deref(), Some(account));
        assert!(
            resolved.base_url.is_none(),
            "the gateway remains flow-owned"
        );
        assert!(
            matches!(&resolved.provenance.source, opi_ai::auth::AuthProvenanceSource::OAuth { kind } if kind == id)
        );
        assert!(
            matches!(store.read(id).await.unwrap(), Some(Credential::OAuthToken { access: saved, account_id: Some(saved_account), .. }) if saved.expose_secret() == access && saved_account == account)
        );
    }
    let mut fresh_a = store.read("radius-a").await.unwrap().unwrap();
    if let Credential::OAuthToken {
        expires_at,
        base_url,
        ..
    } = &mut fresh_a
    {
        assert!(expires_at.is_some_and(
            |expiry| expiry > time::OffsetDateTime::now_utc() + time::Duration::minutes(5)
        ));
        *base_url = Some(gateway_b.uri());
    }
    store
        .modify("radius-a", opi_ai::write_mutation(fresh_a))
        .await
        .unwrap();
    let before_a = gateway_a.received_requests().await.unwrap().len();
    let before_b = gateway_b.received_requests().await.unwrap().len();
    let fresh_auth = models
        .get_auth("radius-a", Default::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fresh_auth.secret.expose_secret(), "refreshed-a");
    assert_eq!(fresh_auth.account_id.as_deref(), Some("account-a"));
    assert!(
        fresh_auth.base_url.is_none(),
        "fresh Radius auth cannot promote credential metadata into endpoint authority"
    );
    assert_eq!(gateway_a.received_requests().await.unwrap().len(), before_a);
    assert_eq!(gateway_b.received_requests().await.unwrap().len(), before_b);
    let mut expired_a = store.read("radius-a").await.unwrap().unwrap();
    if let Credential::OAuthToken { expires_at, .. } = &mut expired_a {
        *expires_at = Some(time::OffsetDateTime::now_utc() - time::Duration::seconds(1));
    }
    store
        .modify("radius-a", opi_ai::write_mutation(expired_a))
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(wiremock::matchers::body_string_contains(
            "refresh_token=rotated-refresh-a",
        ))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(serde_json::json!({"error":"invalid_grant"})),
        )
        .expect(1)
        .mount(&gateway_a)
        .await;
    let error = models
        .get_auth("radius-a", Default::default())
        .await
        .expect_err("selected Radius account is revoked");
    match error {
        opi_ai::ModelsError::AuthResolution { error, .. } => {
            assert!(
                matches!(*error, ProviderError::CredentialRevoked { ref provider_id } if provider_id == "radius-a")
            );
        }
        other => panic!("unexpected Radius revocation: {other:?}"),
    }
    assert_eq!(
        gateway_b.received_requests().await.unwrap().len(),
        before_b,
        "a revoked selected flow must not discover the sibling gateway"
    );
    gateway_a.verify().await;
    gateway_b.verify().await;
}

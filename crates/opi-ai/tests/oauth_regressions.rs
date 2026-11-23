//! Cross-provider OAuth regression and metadata tests.
//!
//! This target owns shared provider registration, error boundaries,
//! cancellation, metadata and multi-provider store behavior.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::Models;
use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::oauth::openrouter::OpenRouterOAuthProvider;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, Credential};
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "support/oauth_anthropic.rs"]
#[allow(dead_code)]
mod oauth_anthropic;
#[path = "support/oauth_assembly.rs"]
#[allow(dead_code)]
mod oauth_assembly;
#[path = "support/oauth_chatgpt.rs"]
#[allow(dead_code)]
mod oauth_chatgpt;
#[path = "support/oauth_codex.rs"]
#[allow(dead_code)]
mod oauth_codex;
#[path = "support/oauth_copilot.rs"]
#[allow(dead_code)]
mod oauth_copilot;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;
#[path = "support/oauth_kimi.rs"]
#[allow(dead_code)]
mod oauth_kimi;
#[path = "support/oauth_loopback.rs"]
#[allow(dead_code)]
mod oauth_loopback;
#[path = "support/oauth_meta.rs"]
#[allow(dead_code)]
mod oauth_meta;
#[path = "support/oauth_radius.rs"]
#[allow(dead_code)]
mod oauth_radius;
#[path = "support/oauth_store.rs"]
#[allow(dead_code)]
mod oauth_store;
#[path = "support/tls_fixtures.rs"]
#[allow(dead_code)]
mod oauth_tls_fixtures;
#[path = "support/oauth_xai.rs"]
#[allow(dead_code)]
mod oauth_xai;

use oauth_anthropic::*;
use oauth_chatgpt::*;
use oauth_codex::*;
use oauth_copilot::*;
use oauth_fixtures::*;
use oauth_kimi::*;
use oauth_loopback::*;
use oauth_meta::*;
use oauth_radius::*;
use oauth_store::*;
use oauth_xai::*;

fn openrouter_flow(token_url: String) -> OpenRouterOAuthProvider {
    OpenRouterOAuthProvider::new(
        "https://openrouter.ai/auth".to_owned(),
        token_url,
        Duration::from_secs(10),
    )
}

/// Register all nine delivered OAuth families on one `Models` (V-43: the
/// three legacy flows are a subset of the nine) and share one store across
/// two `Models` instances (V-55): logins ACK per provider id and never
/// clobber the sibling entry, and a later re-login rotates only its own
/// entry (late-write isolation across sources).
#[tokio::test]
async fn nine_flows_register_and_two_models_share_one_store() {
    let kimi = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "k-dc", "user_code": "K-CODE",
            "verification_uri": "https://auth.kimi.com/device",
            "verification_uri_complete": "https://auth.kimi.com/device?code=K-CODE",
            "expires_in": 900, "interval": 1,
        })))
        .mount(&kimi)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "kimi-access-1", "refresh_token": "kimi-refresh-1", "expires_in": 3600,
        })))
        .mount(&kimi)
        .await;
    let radius = MockServer::start().await;
    mount_radius_device_start(&radius, radius_device_start(1.into())).await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(radius_token_success()))
        .up_to_n_times(1)
        .mount(&radius)
        .await;

    let store = FakeStore::new();
    let mut models_a = Models::new();
    // The nine delivered families.
    models_a.register_oauth_flow(
        "anthropic",
        Arc::new(AnthropicOAuthProvider::new(
            "https://example/authorize".into(),
            "https://example/token".into(),
            "anthropic-client".into(),
            Duration::from_secs(300),
        )),
    );
    models_a.register_oauth_flow(
        "openai-chatgpt",
        Arc::new(OpenAiChatGptOAuthProvider::new_with_endpoints(
            "https://example/authorize".into(),
            "https://example/token".into(),
            None,
            None,
            Duration::from_secs(300),
        )),
    );
    models_a.register_oauth_flow(
        "openai-codex",
        Arc::new(CodexOAuthProvider::new(
            "https://example/authorize".into(),
            "https://example/token".into(),
            "codex-client".into(),
            Duration::from_secs(300),
        )),
    );
    // Registration-only construction reuses the copilot test builder.
    models_a.register_oauth_flow(
        "github-copilot",
        Arc::new(copilot_flow(
            "https://example.invalid",
            Vec::new(),
            Duration::from_secs(300),
        )),
    );
    models_a.register_oauth_flow(
        "openrouter",
        Arc::new(OpenRouterOAuthProvider::new(
            "https://example/authorize".into(),
            "https://example/token".into(),
            Duration::from_secs(300),
        )),
    );
    models_a.register_oauth_flow(
        "kimi-coding",
        Arc::new(KimiCodingOAuthProvider::new_with_oauth_host(kimi.uri())),
    );
    models_a.register_oauth_flow(
        "meta",
        Arc::new(MetaOAuthProvider::new_with_endpoints(
            format!("{}/oidc/device/token", kimi.uri()),
            format!("{}/muse-code/key", kimi.uri()),
        )),
    );
    models_a.register_oauth_flow(
        "xai",
        Arc::new(XaiOAuthProvider::new_with_endpoints(
            format!("{}/oauth2/device/code", kimi.uri()),
            format!("{}/oauth2/token", kimi.uri()),
        )),
    );
    models_a.register_oauth_flow(
        "radius",
        Arc::new(RadiusOAuthProvider::new_with_gateway(radius.uri())),
    );
    models_a.set_credential_store(store.clone());

    let mut models_b = Models::new();
    models_b.register_oauth_flow(
        "kimi-coding",
        Arc::new(KimiCodingOAuthProvider::new_with_oauth_host(kimi.uri())),
    );
    models_b.register_oauth_flow(
        "radius",
        Arc::new(RadiusOAuthProvider::new_with_gateway(radius.uri())),
    );
    models_b.set_credential_store(store.clone());

    // Two Models, one store, two sources: each login ACKs its own entry.
    let kimi_presenter = Arc::new(ScriptedPresenter::default());
    *kimi_presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
    let kimi_cred = models_b
        .login("kimi-coding", kimi_presenter.as_ref())
        .await
        .expect("kimi login");
    assert_eq!(kimi_cred.access.expose_secret(), "kimi-access-1");

    let radius_presenter = Arc::new(ScriptedPresenter::default());
    *radius_presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
    let radius_cred = models_a
        .login("radius", radius_presenter.as_ref())
        .await
        .expect("radius login");
    assert_eq!(radius_cred.access.expose_secret(), "radius-access-1");

    match (store.entry("radius"), store.entry("kimi-coding")) {
        (
            Some(Credential::OAuthToken { access, .. }),
            Some(Credential::OAuthToken { access: k, .. }),
        ) => {
            assert_eq!(access.expose_secret(), "radius-access-1");
            assert_eq!(k.expose_secret(), "kimi-access-1");
        }
        (a, b) => panic!("unexpected store entries: {a:?} {b:?}"),
    }

    // The radius entry rotates on a fresh login while the sibling kimi
    // entry is untouched (late-write isolation across sources).
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "radius-access-2", "refresh_token": "radius-refresh-2", "expires_in": 3600,
        })))
        .mount(&radius)
        .await;
    let again = Arc::new(ScriptedPresenter::default());
    *again.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
    let rotated = models_b
        .login("radius", again.as_ref())
        .await
        .expect("radius relogin");
    assert_eq!(rotated.access.expose_secret(), "radius-access-2");
    if let Some(Credential::OAuthToken { access, .. }) = store.entry("radius") {
        assert_eq!(access.expose_secret(), "radius-access-2");
    } else {
        panic!("radius entry missing after rotation");
    }
    if let Some(Credential::OAuthToken { access: k, .. }) = store.entry("kimi-coding") {
        assert_eq!(k.expose_secret(), "kimi-access-1");
    } else {
        panic!("kimi entry missing after radius rotation");
    }
}

// Public OAuth regression boundaries: presenter forwarding, malformed
// credentials and representable source time values.
#[tokio::test]
async fn regression_models_login_forwards_text_and_progress() {
    struct InteractiveFlow(Arc<Mutex<String>>);
    impl OAuthProvider for InteractiveFlow {
        fn id(&self) -> &str {
            "interactive-test"
        }
        fn login<'a>(
            &'a self,
            presenter: &'a dyn LoginPresenter,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            Box::pin(async move {
                *self.0.lock().unwrap() = presenter.prompt_text("enterprise", "domain").await?;
                presenter.notify_progress("flow progress");
                Ok(stored_credential())
            })
        }
        fn refresh<'a>(
            &'a self,
            cred: &'a OAuthCredential,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            let cred = cred.clone();
            Box::pin(async move { Ok(cred) })
        }
    }
    let observed = Arc::new(Mutex::new(String::new()));
    let mut models = Models::new();
    models.set_credential_store(FakeStore::new());
    models.register_oauth_flow(
        "interactive-test",
        Arc::new(InteractiveFlow(observed.clone())),
    );
    let presenter = ScriptedPresenter::default();
    presenter
        .prompt_answers
        .lock()
        .unwrap()
        .push("company.ghe.com".to_owned());
    models
        .login("interactive-test", &presenter)
        .await
        .expect("acknowledged login");
    assert_eq!(*observed.lock().unwrap(), "company.ghe.com");
    assert_eq!(
        presenter.progress.lock().unwrap().as_slice(),
        &["flow progress"]
    );
}

#[tokio::test]
async fn regression_codex_and_radius_reject_empty_refresh_tokens() {
    let codex = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": codex_jwt(Some("acct-new")), "refresh_token": "", "expires_in": 3600
        })))
        .mount(&codex)
        .await;
    assert!(matches!(
        codex_flow(&codex, Duration::from_secs(30), Duration::from_secs(30))
            .refresh(&stored_credential())
            .await,
        Err(ProviderError::Config(_))
    ));
    let radius = MockServer::start().await;
    mount_radius_token(
        &radius,
        200,
        serde_json::json!({"access_token":"new", "refresh_token":"", "expires_in":3600}),
    )
    .await;
    assert!(matches!(
        radius_flow(&radius).refresh(&xai_stored_credential()).await,
        Err(ProviderError::Config(_))
    ));
}

#[tokio::test]
async fn regression_token_expiry_overflow_is_typed() {
    for kind in ["kimi", "xai"] {
        let server = MockServer::start().await;
        let body = serde_json::json!({"access_token":"new", "refresh_token":"new-refresh", "expires_in":1e300});
        let result = if kind == "kimi" {
            mount_kimi_token(&server, 200, body).await;
            kimi_flow(&server).refresh(&kimi_stored_credential()).await
        } else {
            mount_xai_token(&server, 200, body).await;
            xai_flow(&server).refresh(&xai_stored_credential()).await
        };
        assert!(
            matches!(result, Err(ProviderError::Config(_))),
            "{kind}: {result:?}"
        );
    }
}

#[tokio::test]
async fn regression_device_time_overflow_is_typed() {
    for kind in ["kimi", "meta", "xai", "radius"] {
        for field in ["interval", "expires_in"] {
            let server = MockServer::start().await;
            let mut body = match kind {
                "kimi" => kimi_start_body(1.into(), 900.into()),
                "meta" => meta_start_body(1.into(), 900.into()),
                "xai" => xai_start_body(1.into(), 900.into()),
                _ => radius_device_start(1.into()),
            };
            body[field] = serde_json::json!(1e300);
            let flow: Arc<dyn OAuthProvider> = match kind {
                "kimi" => {
                    mount_kimi_start(&server, body).await;
                    Arc::new(kimi_flow(&server))
                }
                "meta" => {
                    mount_meta_start(&server, body).await;
                    Arc::new(meta_flow(&server))
                }
                "xai" => {
                    mount_xai_start(&server, body).await;
                    Arc::new(xai_flow(&server))
                }
                _ => {
                    mount_radius_device_start(&server, body).await;
                    Arc::new(radius_flow(&server))
                }
            };
            let presenter = ScriptedPresenter::default();
            *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
            let result = flow.login(&presenter).await;
            assert!(
                matches!(result, Err(ProviderError::Config(_))),
                "{kind}/{field}: {result:?}"
            );
        }
    }
}

#[tokio::test]
async fn regression_copy_code_obeys_explicit_login_budget() {
    let flow = AnthropicOAuthProvider::new(
        "https://example.invalid/authorize".into(),
        "http://127.0.0.1:1/token".into(),
        "synthetic".into(),
        Duration::from_millis(20),
    );
    let presenter = ScriptedPresenter::default();
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::CopyCode);
    let result = tokio::time::timeout(Duration::from_millis(500), flow.login(&presenter)).await;
    assert!(
        matches!(result, Ok(Err(ProviderError::Timeout))),
        "explicit login budget must finish the pending paste: {result:?}"
    );
}

#[tokio::test]
async fn regression_radius_browser_ignores_manual_paste_until_callback() {
    let _guard = radius_port_guard().await;
    let server = MockServer::start().await;
    mount_radius_discovery(&server, &format!("{}/authorize", server.uri())).await;
    mount_radius_token(&server, 200, radius_token_success()).await;
    let presenter = ScriptedPresenter::default();
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    let flow = radius_flow(&server);
    let pending = tokio::time::timeout(Duration::from_millis(100), flow.login(&presenter)).await;
    assert!(
        pending.is_err(),
        "manual paste must not complete Radius browser login"
    );
    assert!(radius_token_requests(&server).await.is_empty());
    assert!(presenter.success_at.lock().unwrap().is_none());
}

#[tokio::test]
async fn regression_device_slow_down_overflow_is_typed() {
    for kind in ["kimi", "meta", "xai", "radius"] {
        let server = MockServer::start().await;
        let response = serde_json::json!({"error":"slow_down", "interval":1e300});
        let flow: Arc<dyn OAuthProvider> = match kind {
            "kimi" => {
                mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
                mount_kimi_token(&server, 400, response).await;
                Arc::new(kimi_flow(&server))
            }
            "meta" => {
                mount_meta_start(&server, meta_start_body(1.into(), 900.into())).await;
                mount_meta_token(&server, 400, response).await;
                Arc::new(meta_flow(&server))
            }
            "xai" => {
                mount_xai_start(&server, xai_start_body(1.into(), 900.into())).await;
                mount_xai_token(&server, 400, response).await;
                Arc::new(xai_flow(&server))
            }
            _ => {
                mount_radius_device_start(&server, radius_device_start(1.into())).await;
                mount_radius_token(&server, 400, response).await;
                Arc::new(radius_flow(&server))
            }
        };
        let presenter = ScriptedPresenter::default();
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);
        let result = flow.login(&presenter).await;
        assert!(
            matches!(result, Err(ProviderError::Config(_))),
            "{kind}: {result:?}"
        );
    }
}

#[tokio::test]
async fn regression_copy_code_cancellation_interrupts_pending_presentation() {
    struct PendingPresentation {
        presented: tokio::sync::Notify,
        cancelled: tokio_util::sync::CancellationToken,
    }
    impl LoginPresenter for PendingPresentation {
        fn select_login_method<'a>(
            &'a self,
            _: &'a str,
            _: &'a [OAuthLoginMethod],
            _: OAuthLoginMethod,
        ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
            Box::pin(async { Ok(OAuthLoginMethod::CopyCode) })
        }
        fn present_auth_url<'a>(
            &'a self,
            _: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async move {
                self.presented.notify_one();
                std::future::pending().await
            })
        }
        fn present_device_code<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async move {
                self.cancelled.cancelled().await;
                Ok(())
            })
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _: &str) {}
    }
    let presenter = PendingPresentation {
        presented: tokio::sync::Notify::new(),
        cancelled: tokio_util::sync::CancellationToken::new(),
    };
    let flow = AnthropicOAuthProvider::new(
        "https://example.invalid/authorize".into(),
        "http://127.0.0.1:1/token".into(),
        "synthetic".into(),
        Duration::from_secs(10),
    );
    let login = async {
        tokio::join!(flow.login(&presenter), async {
            presenter.presented.notified().await;
            presenter.cancelled.cancel();
        })
        .0
    };
    let result = tokio::time::timeout(Duration::from_millis(500), login).await;
    assert!(
        matches!(result, Ok(Err(ProviderError::LoginCancelled { .. }))),
        "pending presentation must remain cancellable: {result:?}"
    );
}

#[tokio::test]
async fn regression_codex_agent_name_drives_originator() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_codex_token_success(&server, &codex_jwt(Some("acct-123"))).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30))
        .with_agent_name("embedding-host");
    let presenter = codex_presenter();
    flow.login(presenter.as_ref()).await.expect("login");
    assert_eq!(
        ScriptedPresenter::query_param(&presenter.last_authorize_url(), "originator").as_deref(),
        Some("embedding-host")
    );
}

#[tokio::test]
async fn nine_oauth_dyn_metadata_matches_the_fixed_source_without_auth_io() {
    let server = MockServer::start().await;
    let flows: Vec<Box<dyn OAuthProvider>> = vec![
        Box::new(anthropic_flow(format!("{}/token", server.uri()))),
        Box::new(codex_flow(
            &server,
            Duration::from_secs(30),
            Duration::from_secs(30),
        )),
        Box::new(chatgpt_flow(
            &server,
            Some(CHATGPT_DEVICE_UUID),
            Duration::from_secs(30),
        )),
        Box::new(copilot_flow(&server.uri(), vec![], Duration::from_secs(30))),
        Box::new(openrouter_flow(format!(
            "{}/api/v1/auth/keys",
            server.uri()
        ))),
        Box::new(kimi_flow(&server)),
        Box::new(meta_flow(&server)),
        Box::new(xai_flow(&server)),
        Box::new(radius_flow(&server)),
    ];
    let expected = [
        ("anthropic", "Anthropic (Claude Pro/Max)", Some(true), None),
        (
            "openai-codex",
            "OpenAI (ChatGPT Plus/Pro)",
            Some(true),
            None,
        ),
        (
            "openai-chatgpt",
            "OpenAI (ChatGPT subscription)",
            Some(true),
            Some("Sign in with ChatGPT"),
        ),
        ("github-copilot", "GitHub Copilot", Some(true), None),
        (
            "openrouter",
            "OpenRouter OAuth",
            None,
            Some("Sign in with OpenRouter"),
        ),
        (
            "kimi-coding",
            "Kimi Code (subscription)",
            Some(true),
            Some("Sign in with Kimi Code"),
        ),
        (
            "meta",
            "Meta (Muse subscription)",
            Some(true),
            Some("Sign in with Meta"),
        ),
        (
            "xai",
            "xAI (Grok/X subscription)",
            Some(true),
            Some("Sign in with SuperGrok or X Premium"),
        ),
        ("radius", "Radius", None, None),
    ];
    for (flow, (id, name, subscription, label)) in flows.iter().zip(expected) {
        assert_eq!(flow.id(), id);
        assert_eq!(flow.name(), name, "{id}");
        assert_eq!(flow.is_subscription(), subscription, "{id}");
        assert_eq!(flow.login_label(), label, "{id}");
    }
    for default in [
        RadiusOAuthProvider::production(),
        RadiusOAuthProvider::with_http_client(
            reqwest::Client::builder().no_proxy().build().unwrap(),
        ),
    ] {
        assert_eq!(default.id(), "radius");
        assert_eq!(default.name(), "Radius");
        assert_eq!(default.is_subscription(), None);
        assert_eq!(default.login_label(), None);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn custom_oauth_metadata_defaults_keep_existing_implementations_usable() {
    struct CustomFlow;
    impl OAuthProvider for CustomFlow {
        fn id(&self) -> &str {
            "custom-flow"
        }
        fn login<'a>(
            &'a self,
            _: &'a dyn LoginPresenter,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("metadata must not start login")
        }
        fn refresh<'a>(
            &'a self,
            _: &'a OAuthCredential,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("metadata must not start refresh")
        }
    }
    let flow: &dyn OAuthProvider = &CustomFlow;
    assert_eq!(flow.name(), "custom-flow");
    assert_eq!(flow.is_subscription(), None);
    assert_eq!(flow.login_label(), None);
}

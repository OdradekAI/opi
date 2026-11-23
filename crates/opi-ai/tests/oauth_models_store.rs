//! Models OAuth store lifecycle and acknowledgement tests.
//!
//! This target owns default-store isolation, explicit store replacement,
//! acknowledged login outcomes, failure typing and concurrent mutation order.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::Models;
use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialSource, CredentialStore};
use opi_ai::mutation::MutationOutcome;
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::MockServer;

#[path = "support/oauth_anthropic.rs"]
#[allow(dead_code)]
mod oauth_anthropic;
#[path = "support/oauth_assembly.rs"]
#[allow(dead_code)]
mod oauth_assembly;
#[path = "support/oauth_fixtures.rs"]
#[allow(dead_code)]
mod oauth_fixtures;
#[path = "support/oauth_store.rs"]
#[allow(dead_code)]
mod oauth_store;

use oauth_anthropic::*;
use oauth_assembly::{assert_oauth_adapter_request, register_oauth_assembly_route};
use oauth_fixtures::ScriptedPresenter;
use oauth_store::*;

mod default_memory_store {
    use super::{
        BoxAuthFuture, Credential, CredentialSource, LoginPresenter, Models, OAuthCredential,
        OAuthProvider, ProviderError, ScriptedPresenter,
    };
    use secrecy::{ExposeSecret, SecretString};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    const PROVIDER: &str = "local-memory";
    const ACCESS: &str = "synthetic-default-memory-access";

    #[derive(Default)]
    struct Flow {
        logins: AtomicUsize,
    }

    impl OAuthProvider for Flow {
        fn id(&self) -> &str {
            PROVIDER
        }

        fn login<'a>(
            &'a self,
            _: &'a dyn LoginPresenter,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            Box::pin(async move {
                self.logins.fetch_add(1, Ordering::SeqCst);
                Ok(OAuthCredential {
                    access: SecretString::from(ACCESS),
                    refresh: SecretString::from("synthetic-default-memory-refresh"),
                    expires_at: None,
                    base_url: None,
                    account_id: None,
                    client_id: None,
                    enterprise_domain: None,
                    available_model_ids: None,
                    scopes: None,
                })
            })
        }

        fn refresh<'a>(
            &'a self,
            _: &'a OAuthCredential,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("configuration and stored-auth reads must not refresh")
        }
    }

    fn construct(from_registry: bool, flow: Arc<Flow>) -> Models {
        let mut models = if from_registry {
            Models::from_registry(opi_ai::ProviderRegistry::new())
        } else {
            Models::new()
        };
        models
            .register(
                Box::new(opi_ai::test_support::MockProvider::new(PROVIDER, vec![])),
                opi_ai::AuthDescriptor::StoreCredential {
                    key: PROVIDER.into(),
                    display_source: "local memory fixture".into(),
                },
                Default::default(),
            )
            .unwrap();
        models.register_oauth_flow(PROVIDER, flow);
        models
    }

    #[tokio::test]
    async fn constructors_login_to_independent_default_memory_and_ack() {
        for from_registry in [false, true] {
            let flow = Arc::new(Flow::default());
            let mut left = construct(from_registry, flow.clone());
            let right = construct(from_registry, flow.clone());
            let presenter = ScriptedPresenter::default();
            assert_eq!(flow.logins.load(Ordering::SeqCst), 0);
            assert!(left.check_auth(PROVIDER).await.unwrap().is_none());
            let credential = left.login(PROVIDER, &presenter).await.unwrap();
            assert_eq!(credential.access.expose_secret(), ACCESS);
            assert_eq!(flow.logins.load(Ordering::SeqCst), 1);
            let barrier = left.mutation_barrier().expect("constructor owns a store");
            let report = barrier.report();
            assert!(report.is_complete());
            assert_eq!(report.captured.len(), 1);
            assert!(matches!(
                &report.captured[0],
                opi_ai::mutation::BarrierEntry::Settled(receipt)
                    if receipt.provider_id == PROVIDER
                    && matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. })
            ));
            assert!(presenter.success_at.lock().unwrap().is_some());
            assert!(presenter.authorize_urls.lock().unwrap().is_empty());
            assert!(presenter.device_codes.lock().unwrap().is_empty());
            assert!(presenter.selected_provider_ids.lock().unwrap().is_empty());
            assert!(presenter.failures.lock().unwrap().is_empty());

            for probe in [
                CredentialSource::Absent,
                CredentialSource::Present {
                    label: "stale keychain label".into(),
                },
                CredentialSource::BackendUnavailable {
                    reason: "external backend is unavailable".into(),
                },
            ] {
                left.set_probe(PROVIDER, probe);
                let check = left.check_auth(PROVIDER).await.unwrap().unwrap();
                assert_eq!(check.kind, opi_ai::AuthCheckKind::OAuth);
                assert_eq!(check.source.as_deref(), Some("OAuth"));
                let auth = left
                    .get_auth(PROVIDER, Default::default())
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(auth.secret.expose_secret(), ACCESS);
            }
            assert!(right.check_auth(PROVIDER).await.unwrap().is_none());
            assert!(
                right
                    .get_auth(PROVIDER, Default::default())
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                right
                    .mutation_barrier()
                    .expect("the other constructor owns a separate store")
                    .report()
                    .captured
                    .is_empty()
            );
            assert_eq!(flow.logins.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn explicit_store_replaces_default_memory_without_copy_or_probe_fallback() {
        use opi_ai::CredentialStore;

        for from_registry in [false, true] {
            let flow = Arc::new(Flow::default());
            let mut models = construct(from_registry, flow.clone());
            let presenter = ScriptedPresenter::default();
            models.login(PROVIDER, &presenter).await.unwrap();
            let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
            models.set_credential_store(store.clone());
            models.set_probe(
                PROVIDER,
                CredentialSource::Present {
                    label: "stale presence must not override an explicit empty store".into(),
                },
            );
            assert!(models.check_auth(PROVIDER).await.unwrap().is_none());
            assert!(
                models
                    .get_auth(PROVIDER, Default::default())
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(store.read(PROVIDER).await.unwrap().is_none());
            models.login(PROVIDER, &presenter).await.unwrap();
            let Credential::OAuthToken { access, .. } =
                store.read(PROVIDER).await.unwrap().unwrap()
            else {
                panic!("login must acknowledge the explicitly attached store");
            };
            assert_eq!(access.expose_secret(), ACCESS);
            assert_eq!(flow.logins.load(Ordering::SeqCst), 2);
            let report = models.mutation_barrier().unwrap().report();
            assert_eq!(report.captured.len(), 1);
            assert!(report.is_complete());
        }
    }
}
fn models_with_flow(store: Arc<FakeStore>, flow: AnthropicOAuthProvider) -> Models {
    let mut models = Models::new();
    models.register_oauth_flow("anthropic", Arc::new(flow));
    models.set_credential_store(store);
    models
}

async fn wiremock_login(server: &MockServer) -> (Models, Arc<ScriptedPresenter>, Arc<FakeStore>) {
    let _port_guard = port_guard().await;
    mount_token_success(server).await;
    // Every models login drives the browser method: the presenter echoes the
    // authorize URL back as a pasted redirect so the login completes without
    // an actual browser.
    let store = FakeStore::new();
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let models = models_with_flow(store.clone(), flow);
    (models, presenter, store)
}

#[tokio::test]
async fn models_login_acks_the_store_write_before_reporting_success() {
    let server = MockServer::start().await;
    let _guard = port_guard().await;
    mount_token_success(&server).await;
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    register_oauth_assembly_route(
        &mut models,
        Arc::new(anthropic_flow(format!("{}/token", server.uri()))),
        opi_ai::WireApi::AnthropicMessages,
        server.uri(),
        Arc::new(opi_ai::http::HttpClient::new()),
    );

    let credential = match models.login("anthropic", presenter.as_ref()).await {
        Ok(c) => c,
        Err(e) => panic!("login error: {e:?}"),
    };
    assert_eq!(credential.access.expose_secret(), "acc-1");

    assert_oauth_adapter_request(
        &models,
        "anthropic",
        &credential,
        &server,
        opi_ai::WireApi::AnthropicMessages,
    )
    .await;
    // The stored entry is the acknowledged OAuth credential.
    let stored = store
        .read("anthropic")
        .await
        .expect("store read")
        .expect("stored");
    match stored {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), "acc-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    // notify_success fired after the credential existed in the store.
    assert!(presenter.success_at.lock().unwrap().is_some());
    assert!(store.mutation_barrier().report().captured.iter().any(|entry| {
        matches!(entry, opi_ai::mutation::BarrierEntry::Settled(receipt)
            if receipt.provider_id == "anthropic" && matches!(receipt.outcome, MutationOutcome::Written { .. }))
    }));
}

#[tokio::test]
async fn models_login_unknown_provider_is_typed() {
    let server = MockServer::start().await;
    let (models, presenter, _store) = wiremock_login(&server).await;

    let error = models
        .login("not-registered", presenter.as_ref())
        .await
        .expect_err("unknown flow");
    assert!(error.to_string().contains("not-registered"));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn models_login_store_failure_is_typed_and_never_reports_success() {
    let server = MockServer::start().await;
    let (models, presenter, store) = wiremock_login(&server).await;
    store.set_mode(StoreMode::Fail);

    let error = models
        .login("anthropic", presenter.as_ref())
        .await
        .expect_err("store failure");
    assert!(matches!(
        error,
        opi_ai::ModelsError::CredentialStore(_) | opi_ai::ModelsError::AuthResolution { .. }
    ));
    assert!(presenter.success_at.lock().unwrap().is_none());
    assert!(!presenter.failures.lock().unwrap().is_empty());
    // The flow itself completed, so the token endpoint was hit exactly once.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn models_login_unknown_mutation_outcome_surfaces_pending() {
    let server = MockServer::start().await;
    let (models, presenter, store) = wiremock_login(&server).await;
    store.set_mode(StoreMode::Unknown);

    let error = models
        .login("anthropic", presenter.as_ref())
        .await
        .expect_err("unknown outcome");
    assert!(matches!(
        error,
        opi_ai::ModelsError::MutationOutcomeUnknown { .. }
    ));
    assert!(presenter.success_at.lock().unwrap().is_none());
}

#[tokio::test]
async fn concurrent_login_writes_serialize_on_the_shared_store() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let store = FakeStore::new();
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let models = models_with_flow(store.clone(), flow);

    // Each concurrent login gets its own presenter (its own state), exactly
    // like three independent drivers sharing one store.
    let presenters: Vec<Arc<ScriptedPresenter>> = (0..3)
        .map(|_| {
            let presenter = Arc::new(ScriptedPresenter::default());
            *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
            presenter
        })
        .collect();
    let mut logins = Vec::new();
    for presenter in presenters {
        let models_ref = &models;
        logins.push(async move { models_ref.login("anthropic", presenter.as_ref()).await });
    }
    let results = futures_util::future::join_all(logins).await;
    assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
    // Mutations never overlapped on the shared store.
    assert_eq!(*store.max_overlapping.lock().unwrap(), 1);
    assert_eq!(store.modify_log.lock().unwrap().len(), 3);
    // The final state is a complete credential, never a partial write.
    assert!(store.entry("anthropic").is_some());
}

#[tokio::test]
async fn check_auth_reports_the_login_without_refreshing() {
    let server = MockServer::start().await;
    let (models, presenter, _store) = wiremock_login(&server).await;
    models
        .login("anthropic", presenter.as_ref())
        .await
        .expect("login");

    let check = models
        .check_auth("anthropic")
        .await
        .expect("check")
        .expect("configured");
    assert!(matches!(check.kind, opi_ai::AuthCheckKind::OAuth));
    // Checking performs no refresh: only the login token request was sent.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn login_performs_no_browser_or_ambient_fallback() {
    let _port_guard = port_guard().await;
    // The presenter is the only interaction surface: with no manual reply the
    // flow stays pending (no ambient browser launch, no env fallback, no
    // keychain read) until the budget expires or the caller drops it.
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = ScriptedPresenter::default(); // no manual reply

    let mut login = Box::pin(flow.login(&presenter));
    let outcome = tokio::time::timeout(Duration::from_millis(300), &mut login).await;
    assert!(
        outcome.is_err(),
        "login must stay pending without interaction"
    );
    drop(login);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(presenter.failures.lock().unwrap().is_empty());
}
// ---------------------------------------------------------------------------

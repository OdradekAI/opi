//! Phase 14.2 slice 1 — opi-ai auth-resolution contracts (substrate).
//!
//! Pins the object-safe `AuthResolver` / `OAuthProvider` / `LoginPresenter`
//! contracts, the non-retryable typed
//! `ProviderError::{CredentialNeeded, CredentialRevoked}` variants, and secret
//! redaction. Substrate evidence only (no production call site yet); slices
//! 2/5/6 close the acceptance scenarios.

use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::auth::{
    AuthResolver, AuthScheme, LoginPresenter, OAuthCredential, OAuthProvider, ResolvedAuth,
    StaticAuthResolver,
};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::provider::{ProviderError, ProviderErrorCategory};
use secrecy::{ExposeSecret, SecretString};

#[tokio::test]
async fn provider_error_credential_needed_is_non_retryable_auth() {
    let e = ProviderError::CredentialNeeded {
        provider_id: "anthropic".to_owned(),
    };
    assert!(!e.is_retryable());
    assert_eq!(e.category(), ProviderErrorCategory::Auth);
    let s = format!("{e}");
    assert!(s.contains("anthropic"));
}

#[tokio::test]
async fn provider_error_credential_revoked_is_non_retryable_auth() {
    let e = ProviderError::CredentialRevoked {
        provider_id: "github-copilot".to_owned(),
    };
    assert!(!e.is_retryable());
    assert_eq!(e.category(), ProviderErrorCategory::Auth);
    let s = format!("{e}");
    assert!(s.contains("github-copilot"));
}

#[tokio::test]
async fn static_auth_resolver_is_object_safe_and_resolves_baked_secret() {
    let resolver = StaticAuthResolver::new(AuthScheme::ApiKey, SecretString::from("sk-test-123"));
    let boxed: Box<dyn AuthResolver> = Box::new(resolver);
    let resolved = boxed.resolve().await.expect("static resolve");
    assert_eq!(resolved.scheme, AuthScheme::ApiKey);
    assert_eq!(resolved.secret.expose_secret(), "sk-test-123");
}

#[tokio::test]
async fn resolved_auth_debug_redacts_secret() {
    let resolved = ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("sk-secret-xyz"),
        base_url: None,
        account_id: None,
        provenance: Default::default(),
    };
    let dbg = format!("{resolved:?}");
    assert!(!dbg.contains("sk-secret-xyz"), "secret leaked: {dbg}");
    assert!(dbg.contains("redacted"));
    assert!(dbg.contains("Bearer"));
}

#[tokio::test]
async fn oauth_credential_debug_redacts_secrets_and_keeps_base_url() {
    let cred = OAuthCredential {
        access: SecretString::from("access-secret"),
        refresh: SecretString::from("refresh-secret"),
        expires_at: None,
        base_url: Some("https://example.com".to_owned()),
        account_id: None,
    };
    let dbg = format!("{cred:?}");
    assert!(!dbg.contains("access-secret"), "access leaked: {dbg}");
    assert!(!dbg.contains("refresh-secret"), "refresh leaked: {dbg}");
    assert!(dbg.contains("redacted"));
    assert!(
        dbg.contains("https://example.com"),
        "base_url dropped: {dbg}"
    );
}

// --- Mock OAuthProvider + LoginPresenter proving object safety + callability ---

struct MockLoginPresenter {
    manual_code: String,
    url_calls: AtomicUsize,
}

impl LoginPresenter for MockLoginPresenter {
    fn present_auth_url<'a>(
        &'a self,
        _url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let count = &self.url_calls;
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        _user_code: &'a str,
        _uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let code = self.manual_code.clone();
        Box::pin(async move { Ok(code) })
    }

    fn notify_success(&self) {}

    fn notify_failure(&self, _reason: &str) {}
}

struct MockOAuthProvider {
    id: String,
}

impl OAuthProvider for MockOAuthProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            presenter
                .present_auth_url("https://example/oauth/authorize")
                .await?;
            let code = presenter.await_manual_code().await?;
            presenter.notify_success();
            Ok(OAuthCredential {
                access: SecretString::from(format!("access-{code}")),
                refresh: SecretString::from("refresh-fixed"),
                expires_at: None,
                base_url: None,
                account_id: None,
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            Ok(OAuthCredential {
                access: SecretString::from("access-refreshed"),
                refresh: cred.refresh.clone(),
                expires_at: cred.expires_at,
                base_url: cred.base_url.clone(),
                account_id: None,
            })
        })
    }
}

#[tokio::test]
async fn oauth_provider_and_login_presenter_are_object_safe_and_callable() {
    let presenter = MockLoginPresenter {
        manual_code: "code-42".to_owned(),
        url_calls: AtomicUsize::new(0),
    };
    let provider = MockOAuthProvider {
        id: "mock".to_owned(),
    };

    let p: Box<dyn OAuthProvider> = Box::new(provider);
    let lp: &dyn LoginPresenter = &presenter;

    let cred = p.login(lp).await.expect("login");
    assert_eq!(cred.access.expose_secret(), "access-code-42");
    assert_eq!(cred.refresh.expose_secret(), "refresh-fixed");
    assert_eq!(presenter.url_calls.load(Ordering::SeqCst), 1);

    let refreshed = p.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "access-refreshed");
    assert_eq!(refreshed.refresh.expose_secret(), "refresh-fixed");
}

// ---------------------------------------------------------------------------
// Phase 17 slice 1 — collection-carried AuthProvenance (substrate).
//
// Per the 17.1 DoD, `ResolvedAuth` is unchanged in this task; provenance is a
// closed non-secret classification carried on the prepared call's redacted
// route (17.5 moves it onto `ResolvedAuth` once it owns the product resolvers).
// The closed-source round-trip through the real collection path is covered in
// tests/provider_collection.rs (`prepare_call_surfaces_each_registered_auth_source_on_the_route`).
// ---------------------------------------------------------------------------

// opi-phase17-acceptance
#[test]
fn auth_provenance_debug_carries_no_secret() {
    use opi_ai::auth::{AuthFallback, AuthProvenance, AuthProvenanceSource};

    const SECRET_CANARY: &str = "sk-auth-provenance-secret-canary";

    // Environment provenance carries the variable NAME, never a resolved value.
    let provenance = AuthProvenance {
        source: AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".to_owned(),
        },
        fallback: AuthFallback::NotAttempted,
    };
    let resolved = ResolvedAuth {
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from(SECRET_CANARY),
        base_url: None,
        account_id: None,
        provenance: provenance.clone(),
    };
    let dbg = format!("{resolved:?}");
    assert!(
        dbg.contains("ANTHROPIC_API_KEY"),
        "env var name should be visible: {dbg}"
    );
    assert!(!dbg.contains(SECRET_CANARY), "secret leaked: {dbg}");

    // The fallback reason is a stable non-secret diagnostic, not a credential.
    let used = AuthProvenance {
        source: AuthProvenanceSource::OAuth {
            kind: "github-copilot".to_owned(),
        },
        fallback: AuthFallback::Used {
            from: AuthProvenanceSource::CredentialStore {
                kind: "keychain".to_owned(),
            },
            to: AuthProvenanceSource::Environment {
                name: "ANTHROPIC_API_KEY".to_owned(),
            },
            reason: "credential store unavailable".to_owned(),
        },
    };
    let dbg_used = format!("{used:?}");
    assert!(
        dbg_used.contains("credential store unavailable"),
        "fallback reason should be visible: {dbg_used}"
    );
}

// ---------------------------------------------------------------------------
// Phase 20 task 20.18 — lazy AuthContext, default sources, management results.
//
// DoD: separate source configuration checks, on-demand selected resolution;
// construction/register/static lookup do zero env/store/network; injected
// source replaces ambient and an explicit per-call key wins; check reports
// expired OAuth configured without refresh; store/check errors are typed, not
// empty; a selected-source failure never falls back; logout ACKs only the
// selected local deletion.
// ---------------------------------------------------------------------------

// Serialize ambient-environment tests (AmbientAuthContext reads std::env).
static AMBIENT_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Injected AuthContext backed by a map, counting every access so tests can
/// prove which operations consult a source at all.
struct CountingMapContext {
    env: std::collections::HashMap<String, String>,
    files: std::collections::HashMap<String, bool>,
    env_reads: AtomicUsize,
    file_probes: AtomicUsize,
}

impl CountingMapContext {
    fn new(env: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
        Self {
            env: env
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            files: std::collections::HashMap::new(),
            env_reads: AtomicUsize::new(0),
            file_probes: AtomicUsize::new(0),
        }
    }
}

impl opi_ai::auth::AuthContext for CountingMapContext {
    fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        self.env_reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.env.get(name).cloned())
    }

    fn file_exists(&self, path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        self.file_probes.fetch_add(1, Ordering::SeqCst);
        Ok(self.files.get(path).copied().unwrap_or(false))
    }
}

/// In-memory CredentialStore that records every operation.
struct RecordingStore {
    entries: std::collections::HashMap<String, opi_ai::Credential>,
    reads: AtomicUsize,
    writes: AtomicUsize,
    deletes: std::sync::Mutex<Vec<String>>,
    fail_reads: bool,
}

impl RecordingStore {
    fn new(entries: impl IntoIterator<Item = (&'static str, opi_ai::Credential)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
            reads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            deletes: std::sync::Mutex::new(Vec::new()),
            fail_reads: false,
        }
    }
}

impl opi_ai::CredentialStore for RecordingStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>>
    {
        Box::pin(async move {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.fail_reads {
                return Err(opi_ai::CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "keychain daemon unreachable".to_owned(),
                });
            }
            Ok(self.entries.get(provider_id).cloned())
        })
    }

    fn write<'a>(
        &'a self,
        _provider_id: &'a str,
        _cred: &'a opi_ai::Credential,
    ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        Box::pin(async move {
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        Box::pin(async move {
            self.deletes.lock().unwrap().push(provider_id.to_owned());
            Ok(())
        })
    }

    fn probe<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, opi_ai::CredentialSource> {
        Box::pin(async move {
            if self.entries.contains_key(provider_id) {
                opi_ai::CredentialSource::Present {
                    label: "keychain test".to_owned(),
                }
            } else {
                opi_ai::CredentialSource::Absent
            }
        })
    }
}

/// One-model metadata provider over the shared non-network test support.
fn provider_with_model(id: &str) -> Box<dyn opi_ai::Provider> {
    Box::new(opi_ai::test_support::MockProvider::new_with_models(
        id,
        vec![opi_ai::ModelInfo::new(
            format!("{id}-model"),
            format!("{id} model"),
            opi_ai::WireApi::OpenAiCompletions,
            opi_ai::ModelCapabilities::new(128_000, 4096),
        )],
        vec![],
    ))
}

/// Resolver that reports no credential available (typed CredentialNeeded).
struct NeededResolver;

impl opi_ai::AuthResolver for NeededResolver {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async {
            Err(ProviderError::CredentialNeeded {
                provider_id: "prov-needed".to_owned(),
            })
        })
    }
}

/// Resolver that always fails with a non-CredentialNeeded auth error.
struct FailingResolver {
    provider_id: &'static str,
    calls: AtomicUsize,
}

impl opi_ai::AuthResolver for FailingResolver {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let calls = &self.calls;
        let provider_id = self.provider_id;
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(ProviderError::AuthFailed(
                opi_ai::provider::ProviderErrorSummary::from_untrusted(format!(
                    "selected source failed for {provider_id}"
                )),
            ))
        })
    }
}

#[test]
fn construction_register_and_static_lookup_perform_zero_source_io() {
    let context = std::sync::Arc::new(CountingMapContext::new([]));
    let mut collection = opi_ai::Models::new();

    // Construction and registration happen before any context exists; attach
    // one afterwards and verify lookups still consult nothing.
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-a"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let _ = collection.get_model("prov-a", "whatever");
    let _ = collection.provider_ids();
    let _ = collection.auth_descriptor("prov-a");

    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "construction/register/static lookup must not touch any source"
    );
    assert_eq!(context.file_probes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn check_auth_reads_env_only_on_demand_through_the_injected_context() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_PRESENT_7C31",
        "sk-value-irrelevant",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-env"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_PRESENT_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    collection
        .register(
            provider_with_model("prov-unset"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "lazy: no read yet"
    );
    let check = collection
        .check_auth("prov-env")
        .await
        .expect("check")
        .expect("configured");
    assert_eq!(check.source.as_deref(), Some("OPI_TEST_PRESENT_7C31"));
    assert_eq!(check.kind, opi_ai::AuthCheckKind::ApiKey);
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 1);

    // Unset env var: unconfigured, not an error.
    assert!(
        collection.check_auth("prov-unset").await.unwrap().is_none(),
        "unset env var must report unconfigured"
    );
    // Unknown provider: None, zero source access.
    let before = context.env_reads.load(Ordering::SeqCst);
    assert!(
        collection
            .check_auth("no-such-prov")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn injected_source_replaces_ambient_for_checks_and_resolution() {
    // The variable is deliberately unset in the ambient environment; only the
    // injected source can answer, proving ambient is never consulted.
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-replace"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_AMBIENT_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let ambient = collection
        .check_auth("prov-replace")
        .await
        .expect("ambient check");
    assert!(ambient.is_none(), "ambient env is unset, so unconfigured");

    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_AMBIENT_UNSET_7C31",
        "sk-injected",
    )]));
    collection.set_auth_context(context);
    let check = collection
        .check_auth("prov-replace")
        .await
        .expect("injected check")
        .expect("configured through the injected source");
    assert_eq!(check.source.as_deref(), Some("OPI_TEST_AMBIENT_UNSET_7C31"));

    let resolved = collection
        .get_auth("prov-replace", Default::default())
        .await
        .expect("resolve")
        .expect("resolved through the injected source");
    assert_eq!(resolved.secret.expose_secret(), "sk-injected");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Environment {
            name: "OPI_TEST_AMBIENT_UNSET_7C31".to_owned()
        }
    );
}

#[tokio::test]
async fn explicit_per_call_key_wins_over_every_source() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_7C31",
        "sk-from-env",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-override"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_OVERRIDE_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: Some(SecretString::from("sk-explicit-per-call")),
    };
    let resolved = collection
        .get_auth("prov-override", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    assert_eq!(resolved.scheme, AuthScheme::ApiKey);
    assert_eq!(resolved.secret.expose_secret(), "sk-explicit-per-call");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "explicit override must not touch any source"
    );
}

#[tokio::test]
async fn check_reports_expired_oauth_configured_without_refresh() {
    let expired = opi_ai::Credential::OAuthToken {
        access: SecretString::from("expired-access"),
        refresh: SecretString::from("refresh-material"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(3)),
        base_url: None,
        account_id: None,
    };
    let store = std::sync::Arc::new(RecordingStore::new([("prov-oauth", expired)]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection
        .register(
            provider_with_model("prov-oauth"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-oauth".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let check = collection
        .check_auth("prov-oauth")
        .await
        .expect("check")
        .expect("expired OAuth is still configured");
    assert_eq!(check.kind, opi_ai::AuthCheckKind::OAuth);
    assert_eq!(check.source.as_deref(), Some("OAuth"));
    assert_eq!(
        store.reads.load(Ordering::SeqCst),
        1,
        "one read, no retry loop"
    );
    assert_eq!(
        store.writes.load(Ordering::SeqCst),
        0,
        "check must never refresh or write"
    );
}

#[tokio::test]
async fn store_and_check_errors_are_typed_not_empty() {
    // Live store read failure surfaces as a typed error.
    let store = std::sync::Arc::new(RecordingStore {
        entries: std::collections::HashMap::new(),
        reads: AtomicUsize::new(0),
        writes: AtomicUsize::new(0),
        deletes: std::sync::Mutex::new(Vec::new()),
        fail_reads: true,
    });
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store);
    collection
        .register(
            provider_with_model("prov-storefail"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-storefail".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let error = collection
        .check_auth("prov-storefail")
        .await
        .expect_err("store failure must be an error");
    match &error {
        opi_ai::ModelsError::CredentialStore(opi_ai::CredentialStoreError::Backend {
            reason,
            ..
        }) => assert!(reason.contains("keychain daemon")),
        other => panic!("expected typed CredentialStore::Backend, got {other:?}"),
    }
    let rendered = format!("{error}");
    assert!(
        rendered.contains("keychain daemon unreachable"),
        "{rendered}"
    );

    // Probe-snapshot backend-unavailable also surfaces typed, never empty.
    let mut snapshot_collection = opi_ai::Models::new();
    snapshot_collection
        .register(
            provider_with_model("prov-snapshot"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-snapshot".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    snapshot_collection.set_probe(
        "prov-snapshot",
        opi_ai::CredentialSource::BackendUnavailable {
            reason: "no secret service".to_owned(),
        },
    );
    let error = snapshot_collection
        .check_auth("prov-snapshot")
        .await
        .expect_err("backend-unavailable snapshot must be an error");
    assert!(matches!(
        &error,
        opi_ai::ModelsError::CredentialStore(
            opi_ai::CredentialStoreError::BackendUnavailable { reason, .. }
        ) if reason == "no secret service"
    ));
}

#[tokio::test]
async fn selected_source_failure_never_falls_back() {
    // The route's selected source fails; an env var for the same provider is
    // present in the context, and a static descriptor also exists — none may
    // be tried. get_auth must return the typed failure.
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_FALLBACK_TRAP_7C31",
        "sk-must-not-be-used",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    let resolver = std::sync::Arc::new(FailingResolver {
        provider_id: "prov-selected",
        calls: AtomicUsize::new(0),
    });
    collection
        .register_route(
            provider_with_model("prov-selected"),
            resolver.clone(),
            opi_ai::AuthProvenanceSource::Environment {
                name: "OPI_TEST_FALLBACK_TRAP_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register route");

    let error = collection
        .get_auth("prov-selected", Default::default())
        .await
        .expect_err("selected-source failure must propagate");
    match &error {
        opi_ai::ModelsError::AuthResolution { provider, error } => {
            assert_eq!(provider, "prov-selected");
            assert!(
                matches!(error.as_ref(), ProviderError::AuthFailed(_)),
                "the selected source's typed failure is carried unchanged: {error:?}"
            );
            assert!(format!("{error}").contains("auth"));
        }
        other => panic!("expected AuthResolution, got {other:?}"),
    }
    assert_eq!(
        resolver.calls.load(Ordering::SeqCst),
        1,
        "the selected source ran exactly once"
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "no fallback source may be consulted"
    );

    // CredentialNeeded from the selected source is "unconfigured" (None), not
    // a fallback trigger either.
    collection
        .register_route(
            provider_with_model("prov-needed"),
            std::sync::Arc::new(NeededResolver),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .expect("register route");
    assert!(
        collection
            .get_auth("prov-needed", Default::default())
            .await
            .expect("resolve")
            .is_none()
    );
}

#[tokio::test]
async fn logout_acks_only_the_selected_local_deletion() {
    let store = std::sync::Arc::new(RecordingStore::new([
        (
            "prov-logout",
            opi_ai::Credential::ApiKey(SecretString::from("sk-a")),
        ),
        (
            "prov-other",
            opi_ai::Credential::ApiKey(SecretString::from("sk-b")),
        ),
    ]));
    let context = std::sync::Arc::new(CountingMapContext::new([]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection.set_auth_context(context.clone());

    collection
        .logout(store.as_ref(), "prov-logout")
        .await
        .expect("logout ACK");
    assert_eq!(
        store.deletes.lock().unwrap().as_slice(),
        ["prov-logout".to_owned()].as_slice()
    );
    assert!(
        store.entries.contains_key("prov-other"),
        "other providers' entries must stay untouched"
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "logout must not touch the environment"
    );

    // A failing store stays typed instead of faking an ACK.
    struct FailingDeleteStore;
    impl opi_ai::CredentialStore for FailingDeleteStore {
        fn read<'a>(
            &'a self,
            _provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<
            'a,
            Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>,
        > {
            Box::pin(async { Ok(None) })
        }
        fn write<'a>(
            &'a self,
            _provider_id: &'a str,
            _cred: &'a opi_ai::Credential,
        ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
            Box::pin(async { Ok(()) })
        }
        fn delete<'a>(
            &'a self,
            provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
            Box::pin(async {
                Err(opi_ai::CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "lock unavailable".to_owned(),
                })
            })
        }
        fn probe<'a>(
            &'a self,
            _provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<'a, opi_ai::CredentialSource> {
            Box::pin(async { opi_ai::CredentialSource::Absent })
        }
    }
    let error = opi_ai::Models::new()
        .logout(&FailingDeleteStore, "prov-logout")
        .await
        .expect_err("failing store must surface typed");
    assert!(matches!(
        &error,
        opi_ai::ModelsError::CredentialStore(opi_ai::CredentialStoreError::Backend { reason, .. })
            if reason == "lock unavailable"
    ));
}

#[tokio::test]
async fn get_available_lists_only_configured_providers() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_AVAILABLE_7C31",
        "sk-present",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    for (id, env) in [
        ("prov-a", Some("OPI_TEST_AVAILABLE_7C31")),
        ("prov-b", None),
    ] {
        collection
            .register(
                provider_with_model(id),
                opi_ai::AuthDescriptor::EnvApiKey {
                    env_var: env.unwrap_or("OPI_TEST_UNSET_7C31").to_owned(),
                },
                Default::default(),
            )
            .expect("register");
    }

    let available = collection.get_available(None).await.expect("available");
    let providers: Vec<&str> = available.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        providers,
        ["prov-a"],
        "only configured providers are available"
    );
    assert_eq!(available[0].1.id, "prov-a-model");

    let filtered = collection
        .get_available(Some("prov-b"))
        .await
        .expect("filtered");
    assert!(filtered.is_empty(), "prov-b is unconfigured");
}

#[tokio::test]
async fn ambient_context_env_and_file_exists_semantics() {
    use opi_ai::auth::AuthContext as _;
    let ambient = opi_ai::AmbientAuthContext;

    // Existing / missing files (serialized against ambient-env mutation).
    {
        let _lock = AMBIENT_ENV_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let present = dir.path().join("present.txt");
        std::fs::write(&present, b"x").unwrap();
        assert!(ambient.file_exists(present.to_str().unwrap()).unwrap());
        let missing = dir.path().join("missing.txt");
        assert!(!ambient.file_exists(missing.to_str().unwrap()).unwrap());
    }

    // env(): unset is None, not an error.
    assert_eq!(
        ambient.env("OPI_TEST_DEFINITELY_UNSET_7C31").unwrap(),
        None,
        "unset env is Ok(None)"
    );
}

#[tokio::test]
async fn get_auth_resolves_descriptor_sources_without_refresh() {
    // Stored OAuth (expired) resolves as stored, refresh-free.
    let expired = opi_ai::Credential::OAuthToken {
        access: SecretString::from("stored-access"),
        refresh: SecretString::from("stored-refresh"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1)),
        base_url: Some("https://copilot.example.com".to_owned()),
        account_id: None,
    };
    let store = std::sync::Arc::new(RecordingStore::new([("prov-store", expired)]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection
        .register(
            provider_with_model("prov-store"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-store".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let resolved = collection
        .get_auth("prov-store", Default::default())
        .await
        .expect("resolve")
        .expect("stored credential resolves");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), "stored-access");
    assert_eq!(
        resolved.base_url.as_deref(),
        Some("https://copilot.example.com")
    );
    assert_eq!(
        store.writes.load(Ordering::SeqCst),
        0,
        "management resolution must not refresh"
    );

    // Static descriptor resolves its baked value; label-only does not resolve.
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-static"),
            opi_ai::AuthDescriptor::StaticApiKey {
                value: opi_ai::SecretKey::new("sk-static-value"),
            },
            Default::default(),
        )
        .expect("register");
    collection
        .register(
            provider_with_model("prov-label"),
            opi_ai::AuthDescriptor::Resolved {
                source: "route-owned".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let resolved = collection
        .get_auth("prov-static", Default::default())
        .await
        .expect("static resolve")
        .expect("static key present");
    assert_eq!(resolved.secret.expose_secret(), "sk-static-value");
    assert!(
        collection
            .get_auth("prov-label", Default::default())
            .await
            .expect("label resolve")
            .is_none(),
        "label-only descriptor has no secret to resolve"
    );
}

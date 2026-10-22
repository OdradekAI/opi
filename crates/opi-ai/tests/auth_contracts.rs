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
        bearer: None,
        aws_sigv4: None,
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
async fn explicit_per_call_typed_sigv4_credentials_win_without_source_access() {
    struct CountingResolver {
        resolves: std::sync::atomic::AtomicUsize,
    }
    impl AuthResolver for CountingResolver {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            self.resolves.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(ResolvedAuth {
                    scheme: AuthScheme::ApiKey,
                    secret: SecretString::from("route-resolved"),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_SIGV4_7C31",
        "sk-from-env",
    )]));
    let resolver = std::sync::Arc::new(CountingResolver {
        resolves: std::sync::atomic::AtomicUsize::new(0),
    });
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register_route(
            provider_with_model("prov-override-sigv4"),
            resolver.clone(),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .expect("register");

    let credentials = opi_ai::AwsSigV4Credentials {
        access_key_id: SecretString::from("AKIAPERCALL"),
        secret_access_key: SecretString::from("per-call-secret"),
        session_token: None,
        region: "eu-central-1".to_owned(),
        expires_at: None,
    };
    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: None,
        bearer: None,
        aws_sigv4: Some(credentials),
    };
    let resolved = collection
        .get_auth("prov-override-sigv4", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    match resolved.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(credentials.access_key_id.expose_secret(), "AKIAPERCALL");
            assert_eq!(
                credentials.secret_access_key.expose_secret(),
                "per-call-secret"
            );
            assert_eq!(credentials.region, "eu-central-1");
        }
        other => panic!("expected typed AWS SigV4 override, got {other:?}"),
    }
    assert_eq!(resolved.secret.expose_secret(), "");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 0);
    assert_eq!(
        resolver.resolves.load(Ordering::SeqCst),
        0,
        "typed per-call override must not invoke the route resolver"
    );
}

#[tokio::test]
async fn explicit_per_call_bearer_token_wins_without_source_access() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_BEARER_7C31",
        "sk-from-env",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-override-bearer"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_OVERRIDE_BEARER_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: None,
        bearer: Some(SecretString::from("bedrock-bearer-per-call")),
        aws_sigv4: None,
    };
    let resolved = collection
        .get_auth("prov-override-bearer", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), "bedrock-bearer-per-call");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mutually_exclusive_explicit_overrides_are_a_typed_error() {
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-override-conflict"),
            opi_ai::AuthDescriptor::StaticApiKey {
                value: opi_ai::SecretKey::new("static-key"),
            },
            Default::default(),
        )
        .expect("register");

    let sigv4 = || opi_ai::AwsSigV4Credentials {
        access_key_id: SecretString::from("AKIAPERCALL"),
        secret_access_key: SecretString::from("per-call-secret"),
        session_token: None,
        region: "eu-central-1".to_owned(),
        expires_at: None,
    };
    for (index, overrides) in [
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: Some(SecretString::from("token")),
            aws_sigv4: None,
        },
        opi_ai::AuthResolutionOverrides {
            api_key: None,
            bearer: Some(SecretString::from("token")),
            aws_sigv4: Some(sigv4()),
        },
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: None,
            aws_sigv4: Some(sigv4()),
        },
        // All three set is still exactly one typed configuration error.
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: Some(SecretString::from("token")),
            aws_sigv4: Some(sigv4()),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let error = collection
            .get_auth("prov-override-conflict", overrides)
            .await
            .unwrap_err();
        assert!(
            matches!(error, opi_ai::ModelsError::ConflictingAuthOverrides { .. }),
            "case {index} must fail as a typed override conflict"
        );
    }
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
            provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<
            'a,
            Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>,
        > {
            Box::pin(async move {
                Err(opi_ai::CredentialStoreError::CorruptMarker {
                    provider: provider_id.to_owned(),
                })
            })
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
    // The inline default also skips corrupt reads for unconditional replace,
    // while updates still reject the same corruption before their callback.
    let written = FailingDeleteStore
        .modify(
            "prov-logout",
            opi_ai::write_mutation(stored_api_key("replacement")),
        )
        .await
        .unwrap();
    assert!(matches!(written.outcome, MutationOutcome::Written { .. }));
    let error = FailingDeleteStore
        .modify(
            "prov-logout",
            opi_ai::CredentialMutationOp::Update(Box::new(|_, _| {
                panic!("update callback must not run after a corrupt read")
            })),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        opi_ai::CredentialStoreError::CorruptMarker { .. }
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

// ---------------------------------------------------------------------------
// P20-STORE (task 20.22) — store-owned serialized mutations, receipts, and
// the synchronous mutation barrier.
// ---------------------------------------------------------------------------

use std::sync::Arc;

use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::{
    BarrierEntry, InMemoryCredentialStore, MutationDecision, MutationOutcome, MutationQueue,
};

fn stored_api_key(value: &str) -> Credential {
    Credential::ApiKey(SecretString::from(value.to_owned()))
}

fn write_op(value: &'static str) -> opi_ai::CredentialMutationOp {
    opi_ai::write_mutation(stored_api_key(value))
}

#[test]
fn sync_entry_mutations_preserve_fifo_after_runtime_handoff() {
    let store = InMemoryCredentialStore::new();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let first = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            Box::pin(async move {
                released.await.unwrap();
                Ok(MutationDecision::Write(stored_api_key("rotated")))
            })
        })),
    );
    let deletion = store.modify("prov", opi_ai::delete_mutation());
    let barrier = store.mutation_barrier();
    assert_eq!(barrier.cutoff(), 2);
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut first = first;
        let mut deletion = deletion;
        assert!(futures_util::poll!(&mut first).is_pending());
        assert!(
            futures_util::poll!(&mut deletion).is_pending(),
            "delete must wait for the earlier admitted rotation"
        );
        release.send(()).unwrap();
        assert!(matches!(
            first.await.unwrap().outcome,
            MutationOutcome::Written { revision: 1 }
        ));
        assert!(matches!(
            deletion.await.unwrap().outcome,
            MutationOutcome::Deleted { revision: 2 }
        ));
        assert!(store.read("prov").await.unwrap().is_none());
        assert!(barrier.report().is_complete());
    });
}

#[test]
fn sync_entry_started_mutation_survives_waiter_drop() {
    let store = InMemoryCredentialStore::new();
    let (started, start) = tokio::sync::oneshot::channel::<()>();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let mutation = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            Box::pin(async move {
                started.send(()).unwrap();
                released.await.unwrap();
                Ok(MutationDecision::Write(stored_api_key("committed")))
            })
        })),
    );
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut mutation = mutation;
        assert!(futures_util::poll!(&mut mutation).is_pending());
        start.await.unwrap();
        drop(mutation);
        assert!(
            release.send(()).is_ok(),
            "store must retain the running operation"
        );
        let report = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            store.mutation_barrier().wait(),
        )
        .await
        .expect("store-owned operation must settle");
        assert!(report.is_complete());
        assert!(matches!(
            store.read("prov").await.unwrap(),
            Some(Credential::ApiKey(_))
        ));
    });
}

#[test]
fn off_runtime_submission_waits_for_explicit_runtime_execution_entry() {
    let queue = Arc::new(MutationQueue::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let submitted = queue.clone();
    let worker_calls = calls.clone();
    let mut handle = std::thread::spawn(move || {
        submitted.submit(
            "prov",
            Box::new(move || {
                Box::pin(async move {
                    worker_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(opi_ai::MutationApplied::Write)
                })
            }),
        )
    })
    .join()
    .unwrap();
    let waker = futures_util::task::noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    assert!(std::future::Future::poll(std::pin::Pin::new(&mut handle), &mut cx).is_pending());
    drop(handle);
    let barrier = queue.barrier();
    assert!(!barrier.report().is_complete());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), barrier.wait())
                .await
                .is_err(),
            "barrier waiting must not start an admitted auth operation"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(!barrier.report().is_complete());
        let later = queue.submit(
            "prov",
            Box::new(|| Box::pin(async { Ok(opi_ai::MutationApplied::Delete) })),
        );
        assert!(matches!(
            later.await.unwrap().outcome,
            MutationOutcome::Deleted { revision: 2 }
        ));
        assert!(barrier.report().is_complete());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    });
}

#[tokio::test]
async fn store_mutations_serialize_in_submission_order_per_provider() {
    let store = InMemoryCredentialStore::new();
    let first = store
        .modify("prov", write_op("one"))
        .await
        .expect("first ACK");
    let second = store
        .modify("prov", write_op("two"))
        .await
        .expect("second ACK");
    let third = store
        .modify("prov", opi_ai::delete_mutation())
        .await
        .expect("third ACK");

    // Store-wide generations admit in submission order and per-provider FIFO
    // executes them in that order; revisions advance per applied mutation.
    assert_eq!(first.generation, 1);
    assert_eq!(second.generation, 2);
    assert_eq!(third.generation, 3);
    assert!(matches!(
        first.outcome,
        MutationOutcome::Written { revision: 1 }
    ));
    assert!(matches!(
        second.outcome,
        MutationOutcome::Written { revision: 2 }
    ));
    assert!(matches!(
        third.outcome,
        MutationOutcome::Deleted { revision: 3 }
    ));
    assert!(
        store.read("prov").await.expect("read").is_none(),
        "final state is the serialized delete"
    );
}

#[tokio::test]
async fn started_mutation_survives_waiter_cancellation() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let op: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Write(stored_api_key("committed")))
            })
        }));
    let handle = store.modify("prov", op);

    // Cancel the wait while the mutation is still running: dropping the
    // handle stops only the observation, not the mutation.
    drop(handle);
    // The store owns the mutation: releasing the gate commits it even though
    // no waiter remains.
    release.send(()).unwrap();
    let barrier = store.mutation_barrier();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    match report.captured.as_slice() {
        [BarrierEntry::Settled(receipt)] => {
            assert_eq!(receipt.provider_id, "prov");
            assert!(matches!(
                receipt.outcome,
                MutationOutcome::Written { revision: 1 }
            ));
        }
        other => panic!("expected one settled receipt, got {other:?}"),
    }
    match store.read("prov").await.unwrap() {
        Some(Credential::ApiKey(secret)) => {
            assert_eq!(secret.expose_secret(), "committed");
        }
        other => panic!("expected committed api key, got {other:?}"),
    }
}

#[tokio::test]
async fn second_waiter_reuses_committed_result_without_second_operation() {
    let store = InMemoryCredentialStore::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_op = calls.clone();
    let op: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, current| {
            let calls = calls_for_op.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                match current {
                    Some(_) => Ok(MutationDecision::Keep),
                    None => Ok(MutationDecision::Write(stored_api_key("once"))),
                }
            })
        }));
    let r1 = store.modify(
        "prov",
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, current| {
            // Distinct op object with identical single-flight semantics.
            let _ = current;
            Box::pin(async move { Ok(MutationDecision::Write(stored_api_key("once"))) })
        })),
    );
    let r1 = r1.await.expect("first ACK");
    let r2 = store.modify("prov", op).await.expect("second ACK");
    assert!(matches!(r1.outcome, MutationOutcome::Written { .. }));
    // The second serialized slot re-reads the committed state and keeps it:
    // one operation ran for the second waiter.
    assert!(matches!(
        r2.outcome,
        MutationOutcome::Unchanged { revision: 1 }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn barrier_captures_cutoff_across_models_sharing_store() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let mut models_a = opi_ai::Models::new();
    models_a.set_credential_store(store.clone());
    let mut models_b = opi_ai::Models::new();
    models_b.set_credential_store(store.clone());

    store.modify("prov-a", write_op("a")).await.expect("ACK a");
    store.modify("prov-b", write_op("b")).await.expect("ACK b");

    let from_a = models_a.mutation_barrier().expect("store attached to a");
    let from_b = models_b.mutation_barrier().expect("store attached to b");
    // The cutoff lives in the store: both Models observe the same history.
    assert_eq!(from_a.cutoff(), from_b.cutoff());
    assert_eq!(from_a.cutoff(), 2);
    let report = from_b.report();
    assert_eq!(report.captured.len(), 2);
    assert!(report.late.is_empty());
    assert!(report.is_complete());

    // Without an attached store there is no barrier to capture.
    assert!(opi_ai::Models::new().mutation_barrier().is_none());
}

#[tokio::test]
async fn barrier_reports_pending_failure_unknown_and_late_writes() {
    let store = InMemoryCredentialStore::new();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let gated: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Keep)
            })
        }));
    let _pending = store.modify("prov-pending", gated);

    let failing: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move {
                Err(opi_ai::MutationError::Operation(ProviderError::AuthFailed(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("refresh failed"),
                )))
            })
        }));
    let failed = store
        .modify("prov-fail", failing)
        .await
        .expect("failed mutations still ACK with a receipt");
    assert!(matches!(failed.outcome, MutationOutcome::Failed { .. }));

    let panicking: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move { panic!("worker lost") })
        }));
    // Drive the panicking slot to its Unknown receipt by awaiting the handle.
    let unknown = store
        .modify("prov-panic", panicking)
        .await
        .expect("panic mutations settle as Unknown, not lost handles");
    assert!(matches!(unknown.outcome, MutationOutcome::Unknown { .. }));

    let barrier = store.mutation_barrier();
    let report = barrier.report();
    assert_eq!(barrier.cutoff(), 3);
    // Pending, failed, and unknown are all reported inside the cutoff.
    let mut pending = 0;
    let mut failures = 0;
    let mut unknowns = 0;
    for entry in &report.captured {
        match entry {
            BarrierEntry::Pending { .. } => pending += 1,
            BarrierEntry::Settled(receipt) => match receipt.outcome {
                MutationOutcome::Failed { .. } => failures += 1,
                MutationOutcome::Unknown { .. } => unknowns += 1,
                _ => {}
            },
        }
    }
    assert_eq!(pending, 1);
    assert_eq!(failures, 1);
    assert_eq!(unknowns, 1);
    assert!(!report.is_complete());

    // A post-cutoff admission is a fenced late write: reported, never aborted.
    let late = store
        .modify("prov-late", write_op("late"))
        .await
        .expect("late write completes");
    assert!(matches!(late.outcome, MutationOutcome::Written { .. }));
    let report = barrier.report();
    assert_eq!(report.late.len(), 1);
    assert_eq!(report.late[0].provider_id(), "prov-late");

    // Waiting resolves once the pending mutation settles (no default
    // deadline stopped it).
    release.send(()).unwrap();
    let report = barrier.wait().await;
    assert!(report.is_complete());
}

#[tokio::test]
async fn unknown_outcome_for_one_key_survives_another_keys_success() {
    let store = InMemoryCredentialStore::new();
    let panicking: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move { panic!("lost") })
        }));
    let _ = store
        .modify("prov-old", panicking)
        .await
        .expect("Unknown receipt");
    let ok = store
        .modify("prov-new", write_op("fresh"))
        .await
        .expect("ACK");
    assert!(matches!(ok.outcome, MutationOutcome::Written { .. }));

    let report = store.mutation_barrier().report();
    // Receipts are retained by key: the new key's success did not clear the
    // old key's unconfirmed outcome.
    let old = report
        .captured
        .iter()
        .find(|e| e.provider_id() == "prov-old")
        .expect("old key retained");
    match old {
        BarrierEntry::Settled(receipt) => {
            assert!(matches!(receipt.outcome, MutationOutcome::Unknown { .. }))
        }
        other => panic!("old key should be settled, got {other:?}"),
    }
}

#[tokio::test]
async fn mutation_waits_and_receipts_have_no_default_caps() {
    let store = InMemoryCredentialStore::new();
    // More mutations than any historical default cap (256-key style
    // admission limits): all admit and all receipts are retained.
    let mut handles = Vec::new();
    for i in 0..300 {
        let value: &'static str = Box::leak(format!("key-{i}").into_boxed_str());
        handles.push(store.modify("prov-cap", write_op(value)));
    }
    for handle in handles {
        handle.await.expect("every mutation ACKs");
    }
    let report = store.mutation_barrier().report();
    assert_eq!(report.captured.len(), 300);
    assert!(
        report.captured.iter().all(|entry| matches!(
            entry,
            BarrierEntry::Settled(receipt)
                if matches!(receipt.outcome, MutationOutcome::Written { .. })
        )),
        "every receipt retained"
    );
    assert!(report.is_complete());
}

#[tokio::test]
async fn network_timeout_scope_excludes_lock_wait_and_ack() {
    let store = InMemoryCredentialStore::new();
    // A slow prior slot occupies the (per-provider) serialization for far
    // longer than the second operation's own network bound.
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let slow: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(move |_view, _current| {
            let released = released;
            Box::pin(async move {
                let _ = released.await;
                Ok(MutationDecision::Keep)
            })
        }));
    let _slow = store.modify("prov", slow);

    let bounded: opi_ai::CredentialMutationOp =
        opi_ai::CredentialMutationOp::Update(Box::new(|_view, _current| {
            Box::pin(async move {
                // The operation's own network bound (50ms) applies only to the
                // simulated exchange, not to the slot's lock wait or the ACK.
                match tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    std::future::pending::<()>(),
                )
                .await
                {
                    Err(_elapsed) => Ok(MutationDecision::Write(stored_api_key("after-wait"))),
                    Ok(()) => unreachable!("never resolves"),
                }
            })
        }));
    let bounded = store.modify("prov", bounded);
    // Let the queue admit both before releasing the slow slot.
    tokio::task::yield_now().await;
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    release.send(()).unwrap();
    // The bounded operation's own timeout elapsed while waiting for the
    // serialization, yet the mutation completes: lock waits and ACK are
    // outside the operation's network bound.
    let receipt = bounded.await.expect("ACK after slow slot");
    assert!(matches!(receipt.outcome, MutationOutcome::Written { .. }));
    match store.read("prov").await.unwrap() {
        Some(Credential::ApiKey(secret)) => {
            assert_eq!(secret.expose_secret(), "after-wait");
        }
        other => panic!("expected committed api key, got {other:?}"),
    }
}

#[tokio::test]
async fn mutation_queue_admits_synchronously_before_any_await() {
    let queue = MutationQueue::new();
    let barrier_before = queue.barrier();
    assert_eq!(barrier_before.cutoff(), 0);
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let slot: opi_ai::MutationSlot = Box::new(move || {
        let released = released;
        Box::pin(async move {
            let _ = released.await;
            Ok(opi_ai::MutationApplied::Write)
        })
    });
    let _handle = queue.submit("prov", slot);
    // Admission was synchronous: the barrier captured the running mutation
    // even though nothing has been awaited yet.
    let barrier_after = queue.barrier();
    assert_eq!(barrier_after.cutoff(), 1);
    assert!(barrier_before.report().is_complete());
    release.send(()).unwrap();
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
}

// ---------------------------------------------------------------------------
// Task 20.19.3: source near-expiry refresh registers through the shared
// credential mutation owner (P20-STORE consumer closure).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn source_passive_refresh_registers_through_the_mutation_owner() {
    use opi_ai::auth::aws::cache::{
        AwsSourceCache, CacheIdentity, PassiveRefreshDeps, SourceCacheValue, SsoRefreshMaterial,
        SsoTokenSnapshot,
    };
    use opi_ai::auth::aws::source_http::{
        AwsSourceTransport, SourceHttpRequest, SourceHttpResponse,
    };
    use secrecy::SecretString;

    struct DenyingTransport;
    impl AwsSourceTransport for DenyingTransport {
        fn send(
            &self,
            _request: SourceHttpRequest,
        ) -> opi_ai::BoxAuthFuture<
            'static,
            Result<SourceHttpResponse, opi_ai::auth::aws::source_http::SourceHttpError>,
        > {
            Box::pin(async { Err(opi_ai::auth::aws::source_http::SourceHttpError::Transport) })
        }
    }

    struct FixedClock(time::OffsetDateTime);
    impl opi_ai::auth::aws::cache::SourceClock for FixedClock {
        fn now(&self) -> time::OffsetDateTime {
            self.0
        }
    }
    struct FixedJti;
    impl opi_ai::auth::aws::cache::DpopJtiSource for FixedJti {
        fn next_jti(&self) -> String {
            "jti".to_owned()
        }
    }

    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: std::sync::Arc::new(DenyingTransport),
            clock: std::sync::Arc::new(FixedClock(
                time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
            )),
            jti: std::sync::Arc::new(FixedJti),
        }),
    );
    let identity = CacheIdentity::SsoToken {
        cache_key: "session".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: SecretString::from("token"),
            expires_at: time::OffsetDateTime::from_unix_timestamp(120).unwrap(),
            refresh: Some(SsoRefreshMaterial {
                client_id: "client".to_owned(),
                client_secret: SecretString::from("secret"),
                refresh_token: SecretString::from("refresh"),
            }),
            modern: true,
        }),
    );

    let barrier_before = queue.barrier();
    let spawned = cache.maybe_spawn_passive(
        identity,
        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
        Box::new(|| {
            Box::pin(async { Ok(opi_ai::mutation::MutationApplied::Keep) })
                as opi_ai::BoxAuthFuture<
                    'static,
                    Result<opi_ai::mutation::MutationApplied, opi_ai::mutation::MutationError>,
                >
        }),
    );
    assert!(spawned, "near-expiry refresh registers with the owner");
    let barrier_after = queue.barrier();
    assert!(barrier_after.cutoff() > barrier_before.cutoff());
    // The original caller may leave; the barrier still observes completion.
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
}

// ---------------------------------------------------------------------------
// ADC native families (task 20.21.1)
// ---------------------------------------------------------------------------

mod adc_native {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use opi_ai::BoxAuthFuture;
    use opi_ai::auth::adc::metadata::resolve_direct_metadata_token;
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeInput, AdcNativeOwner, AdcResolutionError, AdcSourceHttpError,
        AdcSourceRequest, AdcSourceResponse, AdcSourceTransport, CANONICAL_TOKEN_ENDPOINT,
        CLOUD_PLATFORM_SCOPE, DEFAULT_STS_OAUTHTOKEN_ENDPOINT, ScopedAdcEnv,
    };
    use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError};
    use secrecy::ExposeSecret;

    /// Non-secret synthetic admission key (RDF-23 sealed corpus): the same
    /// material the crate-internal crypto tests use. The crypto fixtures are
    /// `pub(crate)`, so the test embeds a copy.
    pub(super) const SERVICE_ACCOUNT_PEM: &str = r###"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgvXz9l8JQ6kC1
E5/ZBuDBqumYD03J6pq9twlaSZ4cDCSOhadPVn8y9yayRNxEDZ25eWObVr5SrQvJ
QnlnKmN4bIXKThvkFxzGWTX96ESZbBBg2Fbv3EjkaRaE+H8WBUtc/Hfn5av8CfKj
x9GOwSzW3VkBPW4LhBSCbXLB07f4hgQ6tMyTaejx5nNBzYtD8SsTW9LnpW4GE2GX
z+U1axSU8WFLfgUzVIXZMSrHY0rwOSwCcSicgbNGd8uIEIv0juOyakaRrPlIk5Ce
s+BYC2dT/SEDJd7VqH98qAw9KxOMmGw9rD7odHafXIB8VmQsTsm53N0KdrsufhJO
32YWIsWFAgMBAAECggEACR6ES+Z99kq6mztO3zMXcbgBUtM9ccuO9gfqGSv3iHZS
OvQPXajV2b2b/ahP9HFu+xHitNz7OmQJuOQ9+ycfEmWFF/ON/Ab+ptcdVndvWuKU
BvaHGh+90pg5WEJj2As6rxQTYRM5NQ5DFzJz1lzpxGfgOo8yYidfECuRmOMAJd0s
3k8FZ/y6nshQgfQ0Lw96aFGfCu8Ycu+3K7p8mh+wH1Bq5oVeXgExlcdO6DLXjtQO
vq0H52mBEHKTG7uiJRadPwoYL8gNhyMhg8hDMUX0w6cUIUchJkLygEaa6NElisMZ
jnSo+gcvHDl2LGQrknineuwgJT12SQ1mtnhltp//eQKBgQDQR5ssNInh/sMOxD8o
ZbuT1iRYMQYeNZqLZeJjstMtQHy3Ul1NH2REImStR1Y5dJRO3d1vRiGIZZx++SEp
44QXdfKin0IfW9w89+2KrZrxfk6eraWHPbYX6FRCob2cfm84RgI+P9YQ7jNLoiix
264MjWJ59Qoyl+ZxTFKMh6637QKBgQDFkYJQSeOApt2OUqtbozJ6NuiyzqpTJVDd
tx4/gGGwKtNm5c1FhYBF6n3WEq261l3drvDC6ucnoN2f7eItcS7Owv5Vylzx1oAM
8KPr59WM96phpnugAWBrOYa4UyJYUxn1eVyjx9l986IeaSIXdhXa32bVfd220Ouu
CC7Lv/1g+QKBgQCugA/mDW6oRcYvDQTNV868oeF5cjlp/gcpW4c9eImtlCXpgbxh
7TcYTqiC9Q2svZu0J/ZqUBuwJ1Hin9/FNLVq4osS7NN9an0vJ5mBYso6IlTEZUAV
+wIhLanK2a/++X89aopelSVIsgmQCKCHw0IjiiFDhKWylcmgiw1uaA9eUQKBgAaE
gFoJtfmeh5yUWZbSUW36tb6AVtobVwmZauQJZi+bmhHA7v6mIs8rpy8Suqx5OeuA
Q+0KNsn14pnODTIWk8a6ZA2MeXh47QyAO8gig8b8vKAwD8/9e6LN6QXXqEBlrWuN
r2Kvyxr+mJCMQ1WmzZ3E1O2iE1hemAfLuU97a+5xAoGBAKoGwyrOChBroJmIcMc7
RYpC88yq0FXkN4rXD1o/3n0CUzeeb1pad2wlt0efNCWRU5E2dM13aBxdQvg0kGNq
f1JdzC/2b2W3wiseeVHZn7MSsny0JpfbveYtQ9xbrmmRY2amtuMzS/jbnrIKQ+wv
7VCl8xqVmjOIpkF7O/B26nBb
-----END PRIVATE KEY-----"###;

    fn service_account_json() -> String {
        let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
        format!(
            r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}","project_id":"proj"}}"#
        )
    }

    fn authorized_user_json(quota: Option<&str>) -> String {
        match quota {
            Some(q) => format!(
                r#"{{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","quota_project_id":"{q}"}}"#
            ),
            None => r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#.to_owned(),
        }
    }

    fn external_authorized_user_json(token_url: Option<&str>, universe: Option<&str>) -> String {
        let mut fields = vec![
            r#""type":"external_account_authorized_user""#.to_owned(),
            r#""client_id":"ecid""#.to_owned(),
            r#""client_secret":"ecsec""#.to_owned(),
            r#""refresh_token":"ert1""#.to_owned(),
        ];
        if let Some(url) = token_url {
            fields.push(format!(r#""token_url":"{url}""#));
        }
        if let Some(u) = universe {
            fields.push(format!(r#""universe_domain":"{u}""#));
        }
        format!("{{{}}}", fields.join(","))
    }

    /// In-memory ambient source with counted reads.
    #[derive(Default)]
    struct MapContext {
        env: HashMap<String, String>,
        files: HashMap<String, bool>,
        env_reads: AtomicUsize,
        file_probes: AtomicUsize,
    }

    impl MapContext {
        fn with_env(mut self, name: &str, value: &str) -> Self {
            self.env.insert(name.to_owned(), value.to_owned());
            self
        }

        fn with_file(mut self, path: &std::path::Path) -> Self {
            self.files.insert(path.to_string_lossy().into_owned(), true);
            self
        }
    }

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            self.env_reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
            self.file_probes.fetch_add(1, Ordering::SeqCst);
            Ok(self.files.get(path).copied().unwrap_or(false))
        }
    }

    /// Scripted transport: replies per attempt and records every request.
    struct ScriptedTransport {
        replies: Mutex<Vec<Result<AdcSourceResponse, AdcSourceHttpError>>>,
        requests: Mutex<Vec<AdcSourceRequest>>,
    }

    impl ScriptedTransport {
        fn new(replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn token_reply(body: &str) -> Result<AdcSourceResponse, AdcSourceHttpError> {
            Ok(AdcSourceResponse {
                status: 200,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            })
        }

        fn status_reply(status: u16) -> Result<AdcSourceResponse, AdcSourceHttpError> {
            Ok(AdcSourceResponse {
                status,
                headers: Vec::new(),
                body: Vec::new(),
            })
        }

        fn sent(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn request(&self, index: usize) -> AdcSourceRequest {
            self.requests.lock().unwrap()[index].clone()
        }
    }

    impl AdcSourceTransport for ScriptedTransport {
        fn send<'a>(
            &'a self,
            request: AdcSourceRequest,
        ) -> BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>> {
            self.requests.lock().unwrap().push(request);
            let reply = self.replies.lock().unwrap().first().cloned();
            if reply.is_some() {
                let _ = self.replies.lock().unwrap().remove(0);
            }
            Box::pin(async move { reply.expect("scripted replies exhausted") })
        }
    }

    fn fixed_clock(unix: i64) -> opi_ai::auth::adc::native::FixedAdcClock {
        opi_ai::auth::adc::native::FixedAdcClock::new(
            time::OffsetDateTime::from_unix_timestamp(unix).unwrap(),
        )
    }

    fn input<'a>(
        selection: AdcAuthSelection,
        scoped: &'a ScopedAdcEnv,
        ambient: &'a dyn AuthContext,
        transport: &'a dyn AdcSourceTransport,
        clock: &'a dyn opi_ai::auth::adc::native::AdcClock,
    ) -> AdcNativeInput<'a> {
        AdcNativeInput {
            selection,
            scoped,
            ambient,
            transport,
            clock,
            token_endpoint: None,
            universe_domain: None,
            scopes: &[],
            certificate_extra_roots_pem: None,
            impersonation_endpoint: None,
        }
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // -- selection / file semantics ------------------------------------------

    #[tokio::test]
    async fn adc_auto_without_any_file_is_not_configured_and_sends_nothing() {
        let ctx = MapContext::default();
        let transport = ScriptedTransport::new(Vec::new());
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::Auto,
            &scoped,
            &ctx,
            &transport,
            &clock,
        ))
        .await
        .unwrap();
        assert!(resolved.is_none(), "no file anywhere: not configured");
        assert_eq!(transport.sent(), 0, "missing credentials never dial out");
    }

    #[tokio::test]
    async fn adc_explicit_env_file_parse_failure_is_terminal() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, "{not json").unwrap();
        let ctx = MapContext::default()
            .with_env("GOOGLE_APPLICATION_CREDENTIALS", path.to_str().unwrap());
        let transport = ScriptedTransport::new(Vec::new());
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::Auto,
            &scoped,
            &ctx,
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(err, AdcResolutionError::MalformedJson { .. }));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn adc_well_known_unix_path_is_used_when_env_is_unset() {
        let dir = temp_dir();
        let gcloud = dir.path().join(".config").join("gcloud");
        std::fs::create_dir_all(&gcloud).unwrap();
        let file = gcloud.join("application_default_credentials.json");
        std::fs::write(&file, authorized_user_json(None)).unwrap();
        let ctx = MapContext::default()
            .with_env("HOME", dir.path().to_str().unwrap())
            .with_file(&file);
        let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"tok1","expires_in":3600}"#,
        )]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::Auto,
            &scoped,
            &ctx,
            &transport,
            &clock,
        ))
        .await
        .unwrap()
        .expect("well-known file resolves");
        assert!(matches!(
            resolved.source,
            AdcCredentialSource::AuthorizedUser {
                quota_project: None
            }
        ));
        let request = transport.request(0);
        assert_eq!(request.url, CANONICAL_TOKEN_ENDPOINT);
        assert!(
            request
                .body
                .as_deref()
                .unwrap()
                .contains("grant_type=refresh_token")
        );
        assert!(
            request
                .body
                .as_deref()
                .unwrap()
                .contains("refresh_token=rt1")
        );
        assert_eq!(
            resolved.expires_at,
            Some(time::OffsetDateTime::from_unix_timestamp(1_003_600).unwrap())
        );
    }

    #[tokio::test]
    async fn adc_scoped_env_file_shadows_well_known() {
        let dir = temp_dir();
        let explicit = dir.path().join("explicit.json");
        std::fs::write(&explicit, service_account_json()).unwrap();
        let gcloud = dir.path().join(".config").join("gcloud");
        std::fs::create_dir_all(&gcloud).unwrap();
        std::fs::write(
            gcloud.join("application_default_credentials.json"),
            authorized_user_json(None),
        )
        .unwrap();
        let ctx = MapContext::default()
            .with_env("HOME", dir.path().to_str().unwrap())
            .with_env("GOOGLE_APPLICATION_CREDENTIALS", explicit.to_str().unwrap());
        let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"sa-tok","expires_in":3600}"#,
        )]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::Auto,
            &scoped,
            &ctx,
            &transport,
            &clock,
        ))
        .await
        .unwrap()
        .expect("explicit env file wins");
        assert!(matches!(
            resolved.source,
            AdcCredentialSource::ServiceAccount
        ));
    }

    #[tokio::test]
    async fn adc_unknown_and_future_families_fail_typed() {
        let dir = temp_dir();
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        for (contents, expect_admitted) in [
            // `external_account` is admitted since task 20.21.2; its
            // missing-field failures are typed family errors.
            (r#"{"type":"external_account"}"#.to_owned(), false),
            (r#"{"type":"wat"}"#.to_owned(), false),
        ] {
            let path = dir.path().join("creds.json");
            std::fs::write(&path, &contents).unwrap();
            let transport = ScriptedTransport::new(Vec::new());
            let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
                AdcAuthSelection::CredentialsFile(path),
                &scoped,
                &MapContext::default(),
                &transport,
                &clock,
            ))
            .await
            .unwrap_err();
            assert!(
                matches!(
                    err,
                    AdcResolutionError::FamilyNotAdmitted { .. }
                        | AdcResolutionError::UnknownType { .. }
                        | AdcResolutionError::MissingField {
                            family: "external_account",
                            ..
                        }
                ),
                "family dispatch must be typed, got {err:?}"
            );
            assert_eq!(transport.sent(), 0);
            assert!(!expect_admitted);
        }
    }

    #[tokio::test]
    async fn adc_missing_required_field_names_the_field() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(
            &path,
            r#"{"type":"authorized_user","client_id":"cid","refresh_token":"rt"}"#,
        )
        .unwrap();
        let transport = ScriptedTransport::new(Vec::new());
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AdcResolutionError::MissingField { family, field, .. }
                if family == "authorized_user" && field == "client_secret"
        ));
        assert_eq!(transport.sent(), 0);
    }

    // -- family exchanges -----------------------------------------------------

    #[tokio::test]
    async fn adc_service_account_jws_shape_and_canonical_exchange() {
        let dir = temp_dir();
        let path = dir.path().join("sa.json");
        std::fs::write(&path, service_account_json()).unwrap();
        let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"sa-tok","expires_in":3600}"#,
        )]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap()
        .expect("service account resolves");
        assert!(matches!(
            resolved.source,
            AdcCredentialSource::ServiceAccount
        ));

        let request = transport.request(0);
        assert_eq!(request.method, "POST");
        assert_eq!(request.url, CANONICAL_TOKEN_ENDPOINT);
        let body = request.body.unwrap();
        assert!(
            body.starts_with(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion="
            ),
            "form is grant_type then assertion, got {body}"
        );
        let assertion = body.trim_start_matches(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=",
        );
        let parts: Vec<&str> = assertion.split('.').collect();
        assert_eq!(parts.len(), 3, "JWS compact form");

        // Header is alg-only (no typ/kid).
        let header = b64url_decode(parts[0]);
        assert_eq!(header, b"{\"alg\":\"RS256\"}");

        // Claims carry the source-pinned shape with iat/exp exactly 3600 apart.
        let claims: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
        assert_eq!(claims["iss"], "sa@proj.iam.gserviceaccount.com");
        assert_eq!(claims["scope"], CLOUD_PLATFORM_SCOPE);
        assert_eq!(claims["aud"], CANONICAL_TOKEN_ENDPOINT);
        assert_eq!(claims["iat"], 1_000_000);
        assert_eq!(claims["exp"], 1_003_600);
        assert_eq!(
            resolved.expires_at,
            Some(time::OffsetDateTime::from_unix_timestamp(1_003_600).unwrap())
        );
    }

    #[tokio::test]
    async fn adc_rejected_private_key_is_a_typed_failure_with_zero_sends() {
        let dir = temp_dir();
        let path = dir.path().join("badkey.json");
        std::fs::write(
            &path,
            r#"{"type":"service_account","client_email":"sa@x","private_key":"not a pem"}"#,
        )
        .unwrap();
        let transport = ScriptedTransport::new(Vec::new());
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(err, AdcResolutionError::KeyRejected));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn adc_external_authorized_user_uses_basic_auth_and_default_sts_url() {
        let dir = temp_dir();
        let path = dir.path().join("eau.json");
        std::fs::write(&path, external_authorized_user_json(None, None)).unwrap();
        let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"ext-tok","expires_in":1800,"refresh_token":"ert2"}"#,
        )]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap()
        .expect("external authorized user resolves");
        assert!(matches!(
            resolved.source,
            AdcCredentialSource::ExternalAccountAuthorizedUser { rotated: true, .. }
        ));
        assert_eq!(
            resolved
                .rotated_refresh_token
                .as_ref()
                .map(|s| s.expose_secret()),
            Some("ert2")
        );

        let request = transport.request(0);
        assert_eq!(request.url, DEFAULT_STS_OAUTHTOKEN_ENDPOINT);
        let basic = request
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| value.clone())
            .expect("basic auth header present");
        assert_eq!(basic, format!("Basic {}", b64_std(b"ecid:ecsec")));
        let body = request.body.unwrap();
        assert!(body.contains("grant_type=refresh_token"));
        assert!(body.contains("refresh_token=ert1"));
    }

    #[tokio::test]
    async fn adc_external_authorized_user_token_url_and_universe_domain() {
        let dir = temp_dir();
        let path = dir.path().join("eau-url.json");
        std::fs::write(
            &path,
            external_authorized_user_json(Some("https://sts.example.invalid/v1/oauthtoken"), None),
        )
        .unwrap();
        let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"t","expires_in":100}"#,
        )]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap();
        assert!(resolved.is_some());
        assert_eq!(
            transport.request(0).url,
            "https://sts.example.invalid/v1/oauthtoken"
        );

        let path2 = dir.path().join("eau-universe.json");
        std::fs::write(
            &path2,
            external_authorized_user_json(None, Some("example.net")),
        )
        .unwrap();
        let transport2 = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
            r#"{"access_token":"t","expires_in":100}"#,
        )]);
        let resolved2 = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path2),
            &ScopedAdcEnv::default(),
            &MapContext::default(),
            &transport2,
            &clock,
        ))
        .await
        .unwrap();
        assert!(resolved2.is_some());
        assert_eq!(
            transport2.request(0).url,
            "https://sts.example.net/v1/oauthtoken"
        );
    }

    #[tokio::test]
    async fn owner_reuses_fresh_tokens_and_refreshes_after_the_eager_window() {
        use std::sync::Arc;
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, authorized_user_json(None)).unwrap();
        let transport = Arc::new(ScriptedTransport::new(vec![
            ScriptedTransport::token_reply(r#"{"access_token":"tok1","expires_in":3600}"#),
            ScriptedTransport::token_reply(r#"{"access_token":"tok2","expires_in":3600}"#),
        ]));
        let clock = Arc::new(fixed_clock(1_000_000));
        let owner = AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            clock.clone(),
        )
        .with_ambient(Arc::new(MapContext::default()));

        let first = owner.resolve_token().await.unwrap();
        assert_eq!(first.access_token.expose_secret(), "tok1");
        let second = owner.resolve_token().await.unwrap();
        assert_eq!(
            second.access_token.expose_secret(),
            "tok1",
            "fresh token reused"
        );

        // The eager threshold is five minutes: once now + 300s reaches the
        // expiry the owner re-exchanges.
        clock.set(time::OffsetDateTime::from_unix_timestamp(1_003_300).unwrap());
        let third = owner.resolve_token().await.unwrap();
        assert_eq!(third.access_token.expose_secret(), "tok2");
    }

    #[tokio::test]
    async fn owner_rotation_replaces_the_refresh_token_in_place() {
        use std::sync::Arc;
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, external_authorized_user_json(None, None)).unwrap();
        let transport = Arc::new(ScriptedTransport::new(vec![
            ScriptedTransport::token_reply(
                r#"{"access_token":"t1","expires_in":60,"refresh_token":"ert2"}"#,
            ),
            ScriptedTransport::token_reply(r#"{"access_token":"t2","expires_in":60}"#),
        ]));
        let clock = Arc::new(fixed_clock(1_000_000));
        let owner = AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            clock.clone(),
        )
        .with_ambient(Arc::new(MapContext::default()));

        let first = owner.resolve_token().await.unwrap();
        assert_eq!(
            first
                .rotated_refresh_token
                .as_ref()
                .map(|s| s.expose_secret()),
            Some("ert2")
        );
        clock.set(time::OffsetDateTime::from_unix_timestamp(1_000_120).unwrap());
        let second = owner.resolve_token().await.unwrap();
        let body = transport.request(1).body.unwrap();
        assert!(
            body.contains("refresh_token=ert2"),
            "the second exchange sends the rotated refresh token, got {body}"
        );
        assert!(second.rotated_refresh_token.is_none(), "no new rotation");
    }

    // -- retry budgets ---------------------------------------------------------

    #[tokio::test]
    async fn adc_status_retry_is_three_extra_with_gaxios_ranges() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, authorized_user_json(None)).unwrap();
        let transport = ScriptedTransport::new(vec![
            ScriptedTransport::status_reply(503),
            ScriptedTransport::status_reply(503),
            ScriptedTransport::token_reply(r#"{"access_token":"ok","expires_in":3600}"#),
        ]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap()
        .expect("503 x2 then 200 succeeds");
        assert!(resolved.access_token.expose_secret() == "ok");
        assert_eq!(transport.sent(), 3);
    }

    #[tokio::test]
    async fn adc_status_exhaustion_and_non_retryable_statuses() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, authorized_user_json(None)).unwrap();
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();

        // 4 sends exhausts the fixed extra-3 budget.
        let transport = ScriptedTransport::new(vec![
            ScriptedTransport::status_reply(500),
            ScriptedTransport::status_reply(500),
            ScriptedTransport::status_reply(500),
            ScriptedTransport::status_reply(500),
        ]);
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path.clone()),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AdcResolutionError::ExchangeRejected {
                status: 500,
                attempts: 4
            }
        ));
        assert_eq!(transport.sent(), 4);

        // 400 is outside the fixed ranges: terminal on the first send.
        let transport = ScriptedTransport::new(vec![ScriptedTransport::status_reply(400)]);
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AdcResolutionError::ExchangeRejected {
                status: 400,
                attempts: 1
            }
        ));
        assert_eq!(transport.sent(), 1);
    }

    #[tokio::test]
    async fn adc_no_response_budget_is_two_extra() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, authorized_user_json(None)).unwrap();
        let transport = ScriptedTransport::new(vec![
            Err(AdcSourceHttpError::Network),
            Err(AdcSourceHttpError::Network),
            Err(AdcSourceHttpError::Network),
        ]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AdcResolutionError::ExchangeNetwork { attempts: 3 }
        ));
    }

    #[tokio::test]
    async fn adc_redirect_on_exchange_is_terminal() {
        let dir = temp_dir();
        let path = dir.path().join("creds.json");
        std::fs::write(&path, authorized_user_json(None)).unwrap();
        let transport = ScriptedTransport::new(vec![Err(AdcSourceHttpError::Redirect)]);
        let clock = fixed_clock(1_000_000);
        let scoped = ScopedAdcEnv::default();
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(matches!(err, AdcResolutionError::Redirected));
        assert_eq!(transport.sent(), 1);
    }

    // -- direct metadata -------------------------------------------------------

    #[tokio::test]
    async fn metadata_token_carries_flavor_scope_and_expiry() {
        let ctx = MapContext::default();
        let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
            status: 200,
            headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
            body: br#"{"access_token":"md-tok","expires_in":1200}"#.to_vec(),
        })]);
        let clock = fixed_clock(1_000_000);
        let token = resolve_direct_metadata_token(
            &ctx,
            &transport,
            &clock,
            &[CLOUD_PLATFORM_SCOPE.to_owned()],
            Some(false),
        )
        .await
        .unwrap();
        assert!(matches!(token.source, AdcCredentialSource::DirectMetadata));
        assert_eq!(token.access_token.expose_secret(), "md-tok");
        assert_eq!(
            token.expires_at,
            Some(time::OffsetDateTime::from_unix_timestamp(1_001_200).unwrap())
        );

        let request = transport.request(0);
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            "http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/token?scopes=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcloud-platform"
        );
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Metadata-Flavor" && value == "Google")
        );
        assert_eq!(
            request.timeout,
            Some(std::time::Duration::from_millis(3000)),
            "non-resident metadata uses the fixed 3000ms"
        );
    }

    #[tokio::test]
    async fn metadata_residency_true_disables_the_timeout() {
        let ctx = MapContext::default();
        let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
            status: 200,
            headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
            body: br#"{"access_token":"md","expires_in":60}"#.to_vec(),
        })]);
        let clock = fixed_clock(1_000_000);
        resolve_direct_metadata_token(
            &ctx,
            &transport,
            &clock,
            &[CLOUD_PLATFORM_SCOPE.to_owned()],
            Some(true),
        )
        .await
        .unwrap();
        assert_eq!(transport.request(0).timeout, None);
    }

    #[tokio::test]
    async fn metadata_wrong_flavor_is_typed_and_timeout_is_single_send() {
        let ctx = MapContext::default();
        let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
            status: 200,
            headers: Vec::new(),
            body: br#"{"access_token":"x"}"#.to_vec(),
        })]);
        let clock = fixed_clock(1_000_000);
        let err = resolve_direct_metadata_token(
            &ctx,
            &transport,
            &clock,
            &[CLOUD_PLATFORM_SCOPE.to_owned()],
            Some(false),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AdcResolutionError::MetadataFlavor));

        let transport = ScriptedTransport::new(vec![Err(AdcSourceHttpError::Timeout)]);
        let err = resolve_direct_metadata_token(
            &ctx,
            &transport,
            &clock,
            &[CLOUD_PLATFORM_SCOPE.to_owned()],
            Some(false),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            AdcResolutionError::ExchangeTimeout { attempts: 1 }
        ));
        assert_eq!(transport.sent(), 1);
    }

    #[tokio::test]
    async fn metadata_base_honors_scoped_host_env() {
        let ctx = MapContext::default().with_env("GCE_METADATA_HOST", "127.0.0.1:9999");
        let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
            status: 200,
            headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
            body: br#"{"access_token":"md","expires_in":60}"#.to_vec(),
        })]);
        let clock = fixed_clock(1_000_000);
        resolve_direct_metadata_token(
            &ctx,
            &transport,
            &clock,
            &[CLOUD_PLATFORM_SCOPE.to_owned()],
            Some(false),
        )
        .await
        .unwrap();
        assert_eq!(
            transport.request(0).url,
            "http://127.0.0.1:9999/computeMetadata/v1/instance/service-accounts/default/token?scopes=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcloud-platform"
        );
    }

    // -- helpers ---------------------------------------------------------------

    fn b64url_decode(input: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = Vec::new();
        let mut buffer: u32 = 0;
        let mut bits = 0;
        for &byte in input.as_bytes() {
            if byte == b'=' {
                continue;
            }
            let value = ALPHABET
                .iter()
                .position(|&a| a == byte)
                .expect("valid base64url byte") as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    }

    fn b64_std(input: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in input.chunks(3) {
            let b = [
                chunk[0],
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
            ];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }
}

/// Task 20.21.2: ADC `external_account` federation (file/URL/certificate/aws1
/// suppliers -> RFC8693 STS) and the cancelable executable supplier.
mod adc_external {
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeInput, AdcResolutionError, AdcSourceHttpError, AdcSourceRequest,
        AdcSourceResponse, AdcSourceTransport, ScopedAdcEnv,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, ExternalAccountSupplier,
    };
    use opi_ai::credential::BoxAuthFuture;
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::Mutex;

    // ------------------------------------------------------------------
    // Shared local helpers
    // ------------------------------------------------------------------

    struct MapContext {
        env: HashMap<String, String>,
    }

    impl MapContext {
        fn new(env: &[(&str, &str)]) -> Self {
            Self {
                env: env
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            }
        }
    }

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
            Ok(Path::new(path).is_file())
        }
    }

    struct ScriptedTransport {
        replies: Mutex<Vec<Result<AdcSourceResponse, AdcSourceHttpError>>>,
        requests: Mutex<Vec<AdcSourceRequest>>,
    }

    impl ScriptedTransport {
        fn new(replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn text_reply(status: u16, body: &str) -> Result<AdcSourceResponse, AdcSourceHttpError> {
            Ok(AdcSourceResponse {
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            })
        }

        fn sent(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn request(&self, index: usize) -> AdcSourceRequest {
            self.requests.lock().unwrap()[index].clone()
        }
    }

    impl AdcSourceTransport for ScriptedTransport {
        fn send<'a>(
            &'a self,
            request: AdcSourceRequest,
        ) -> BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>> {
            self.requests.lock().unwrap().push(request);
            let reply = self.replies.lock().unwrap().first().cloned();
            if reply.is_some() {
                let _ = self.replies.lock().unwrap().remove(0);
            }
            Box::pin(async move { reply.expect("scripted replies exhausted") })
        }
    }

    fn clock() -> opi_ai::auth::adc::native::FixedAdcClock {
        opi_ai::auth::adc::native::FixedAdcClock::new(
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        )
    }

    /// A leaked frozen clock so `AdcNativeInput` helpers can borrow
    /// `&'static dyn AdcClock`.
    fn leaked_clock() -> &'static opi_ai::auth::adc::native::FixedAdcClock {
        static FROZEN: std::sync::LazyLock<opi_ai::auth::adc::native::FixedAdcClock> =
            std::sync::LazyLock::new(clock);
        &FROZEN
    }

    fn input<'a>(
        selection: AdcAuthSelection,
        ambient: &'a dyn AuthContext,
        transport: &'a dyn AdcSourceTransport,
    ) -> AdcNativeInput<'a> {
        static EMPTY_SCOPED: std::sync::LazyLock<ScopedAdcEnv> =
            std::sync::LazyLock::new(ScopedAdcEnv::empty);
        AdcNativeInput::new(selection, &EMPTY_SCOPED, ambient, transport, leaked_clock())
    }

    fn creds_file(dir: &std::path::Path, name: &str, contents: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn sts_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
        ScriptedTransport::text_reply(200, r#"{"access_token":"fed-token","expires_in":3600}"#)
    }

    fn form_pairs(body: &str) -> Vec<(String, String)> {
        url::form_urlencoded::parse(body.as_bytes())
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    fn form_value(body: &str, key: &str) -> String {
        form_pairs(body)
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    /// JS `decodeURIComponent` subset (percent + nothing else) for decoding
    /// serialized AWS subject tokens in assertions.
    fn percent_decode(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }

    fn audience() -> &'static str {
        "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o"
    }

    fn subject_token_type() -> &'static str {
        "urn:ietf:params:oauth:token-type:jwt"
    }

    fn external_json(credential_source: &str) -> String {
        format!(
            r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}","credential_source":{}}}"#,
            audience(),
            subject_token_type(),
            credential_source
        )
    }

    /// Full file JSON with extra top-level fields (`universe_domain`,
    /// `token_url`, `workforce_pool_user_project`, ...).
    fn external_full_json(extras: &str, credential_source: &str) -> String {
        format!(
            r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}"{extras},"credential_source":{credential_source}}}"#,
            audience(),
            subject_token_type(),
        )
    }

    async fn resolve(
        json: &str,
        ambient: &dyn AuthContext,
        transport: &dyn AdcSourceTransport,
    ) -> Result<Option<opi_ai::auth::adc::native::AdcResolvedToken>, AdcResolutionError> {
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(dir.path(), "external.json", json);
        let file_input = input(AdcAuthSelection::CredentialsFile(path), ambient, transport);
        opi_ai::auth::adc::native::resolve_adc_native(&file_input).await
    }

    // ------------------------------------------------------------------
    // Configuration and identity-pool parsing
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn external_file_supplier_text_exchanges_at_sts() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "raw-subject-token");
        let json = external_json(&format!(r#"{{"file":"{}"}}"#, token_file.display()));
        let ambient = MapContext::new(&[]);
        let transport = ScriptedTransport::new(vec![sts_ok()]);

        let token = resolve(&json, &ambient, &transport).await.unwrap().unwrap();

        let sts = transport.request(0);
        assert_eq!(sts.method, "POST");
        assert_eq!(sts.url, "https://sts.googleapis.com/v1/token");
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "grant_type"),
            "urn:ietf:params:oauth:grant-type:token-exchange"
        );
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "requested_token_type"),
            "urn:ietf:params:oauth:token-type:access_token"
        );
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "audience"),
            audience()
        );
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "subject_token"),
            "raw-subject-token"
        );
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "subject_token_type"),
            subject_token_type()
        );
        assert_eq!(
            form_value(sts.body.as_deref().unwrap(), "scope"),
            "https://www.googleapis.com/auth/cloud-platform"
        );
        assert_eq!(token.access_token.expose_secret(), "fed-token");
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::File,
                workforce_user_project: None,
            }
        );
    }

    #[tokio::test]
    async fn external_file_supplier_json_field_and_raw_text_no_trim() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(
            dir.path(),
            "subject.json",
            r#"{"unrelated":"x","the_token":"  spaced  "}"#,
        );
        let json = external_json(&format!(
            r#"{{"file":"{}","format":{{"type":"json","subject_token_field_name":"the_token"}}}}"#,
            token_file.display()
        ));
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        let token = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        // The file supplier keeps the raw text (no trim) for text format;
        // JSON format extracts the named field exactly.
        assert_eq!(
            form_value(
                transport.request(0).body.as_deref().unwrap(),
                "subject_token"
            ),
            "  spaced  "
        );
        assert_eq!(token.access_token.expose_secret(), "fed-token");
    }

    #[tokio::test]
    async fn external_file_text_keeps_raw_bytes_without_trim() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "\n  padded-token \n");
        let json = external_json(&format!(r#"{{"file":"{}"}}"#, token_file.display()));
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            form_value(
                transport.request(0).body.as_deref().unwrap(),
                "subject_token"
            ),
            "\n  padded-token \n"
        );
    }

    #[tokio::test]
    async fn external_universe_domain_and_explicit_token_url() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = external_full_json(
            r#","universe_domain":"example.net""#,
            &format!(r#"{{"file":"{}"}}"#, token_file.display()),
        );
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(transport.request(0).url, "https://sts.example.net/v1/token");

        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = external_full_json(
            r#","token_url":"https://sts.example.org/exchange""#,
            &format!(r#"{{"file":"{}"}}"#, token_file.display()),
        );
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(transport.request(0).url, "https://sts.example.org/exchange");
    }

    #[tokio::test]
    async fn external_confidential_client_uses_basic() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = format!(
            r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}","client_id":"ecid","client_secret":"ecsec","credential_source":{{"file":"{}"}}}}"#,
            audience(),
            subject_token_type(),
            token_file.display()
        );
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        use base64::Engine as _;
        let expected = base64::engine::general_purpose::STANDARD.encode(b"ecid:ecsec");
        assert!(
            transport
                .request(0)
                .headers
                .iter()
                .any(|(k, v)| k == "authorization" && v == &format!("Basic {expected}"))
        );
        // Confidential clients pass no userProject options field.
        assert!(
            !transport
                .request(0)
                .body
                .as_deref()
                .unwrap()
                .contains("options=")
        );
    }

    #[tokio::test]
    async fn external_workforce_user_project_flows_into_options_field() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = format!(
            r#"{{"type":"external_account","audience":"//iam.googleapis.com/locations/us/workforcePools/wp/providers/oidc","subject_token_type":"{}","workforce_pool_user_project":"wp-proj","credential_source":{{"file":"{}"}}}}"#,
            subject_token_type(),
            token_file.display()
        );
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        let token = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            form_value(transport.request(0).body.as_deref().unwrap(), "options"),
            r#"{"userProject":"wp-proj"}"#
        );
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::File,
                workforce_user_project: Some("wp-proj".to_owned()),
            }
        );
    }

    #[tokio::test]
    async fn external_workforce_project_on_non_workforce_audience_is_typed() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = external_full_json(
            r#","workforce_pool_user_project":"wp""#,
            &format!(r#"{{"file":"{}"}}"#, token_file.display()),
        );
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, AdcResolutionError::WorkforceProjectInvalid));
    }

    #[tokio::test]
    async fn external_missing_audience_is_missing_field() {
        let json = r#"{"type":"external_account","subject_token_type":"st","credential_source":{"file":"/nope"}}"#;
        let error = resolve(
            json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::MissingField {
                family: "external_account",
                field: "audience"
            }
        ));
    }

    #[tokio::test]
    async fn external_token_url_invalid_is_typed() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = external_full_json(
            r#","token_url":"not a url""#,
            &format!(r#"{{"file":"{}"}}"#, token_file.display()),
        );
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, AdcResolutionError::InvalidTokenUrl));
    }

    #[tokio::test]
    async fn external_identity_pool_source_exclusivity_is_typed() {
        let json = external_json(r#"{"file":"/a","url":"https://x.example/t"}"#);
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));

        let json = external_json("{}");
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));
    }

    #[tokio::test]
    async fn external_json_format_requires_field_name() {
        let json = external_json(r#"{"file":"/a","format":{"type":"json"}}"#);
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));

        let json = external_json(r#"{"file":"/a","format":{"type":"cbor"}}"#);
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));
    }

    #[tokio::test]
    async fn external_sts_rejection_and_retry_budget() {
        let dir = tempfile::tempdir().unwrap();
        let token_file = creds_file(dir.path(), "subject.txt", "t");
        let json = external_json(&format!(r#"{{"file":"{}"}}"#, token_file.display()));

        // Non-retryable status is terminal after one send.
        let transport =
            ScriptedTransport::new(vec![ScriptedTransport::text_reply(400, "nope"), sts_ok()]);
        let error = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExchangeRejected {
                status: 400,
                attempts: 1
            }
        ));
        assert_eq!(transport.sent(), 1);

        // Retry-eligible status retries under the fixed Gaxios budget.
        let transport =
            ScriptedTransport::new(vec![ScriptedTransport::text_reply(503, ""), sts_ok()]);
        let token = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.access_token.expose_secret(), "fed-token");
        assert_eq!(transport.sent(), 2);
    }

    // The `impersonated_service_account` family gate test from 20.21.2 was
    // superseded by task 20.21.3: the family is admitted and owned by the
    // `adc_impersonation` module (19 tests: parsing, wire reconstruction,
    // recursion, universe agreement, external composition, keyed cache).

    // ------------------------------------------------------------------
    // URL supplier
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn external_url_supplier_get_with_headers_then_sts() {
        let json = external_json(
            r#"{"url":"https://subject.example/token","headers":{"X-Custom":"yes"},"format":{"type":"json","subject_token_field_name":"tok"}}"#,
        );
        let transport = ScriptedTransport::new(vec![
            ScriptedTransport::text_reply(200, r#"{"tok":"url-subject"}"#),
            sts_ok(),
        ]);
        let token = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();

        let get = transport.request(0);
        assert_eq!(get.method, "GET");
        assert_eq!(get.url, "https://subject.example/token");
        assert!(
            get.headers
                .iter()
                .any(|(k, v)| k == "x-custom" && v == "yes")
        );
        assert_eq!(
            form_value(
                transport.request(1).body.as_deref().unwrap(),
                "subject_token"
            ),
            "url-subject"
        );
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Url,
                workforce_user_project: None,
            }
        );
    }

    #[tokio::test]
    async fn external_url_supplier_terminal_status_is_typed_with_zero_sts() {
        let json = external_json(r#"{"url":"https://subject.example/token"}"#);
        let transport =
            ScriptedTransport::new(vec![ScriptedTransport::text_reply(401, "denied"), sts_ok()]);
        let error = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::SubjectTokenUnavailable { .. }
        ));
        assert_eq!(transport.sent(), 1);
    }

    // ------------------------------------------------------------------
    // AWS (aws1) supplier
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn external_aws_env_credentials_and_region_fixed_vector() {
        let json = external_json(
            r#"{"environment_id":"aws1","regional_cred_verification_url":"https://sts.{region}.example.dev?Action=GetCallerIdentity&Version=2011-06-15"}"#,
        );
        let ambient = MapContext::new(&[
            ("AWS_REGION", "us-east-1"),
            ("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE"),
            (
                "AWS_SECRET_ACCESS_KEY",
                "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            ),
            ("AWS_SESSION_TOKEN", "session-token"),
        ]);
        let transport = ScriptedTransport::new(vec![sts_ok()]);

        let token = resolve(&json, &ambient, &transport).await.unwrap().unwrap();
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Aws,
                workforce_user_project: None,
            }
        );

        let subject = percent_decode(&form_value(
            transport.request(0).body.as_deref().unwrap(),
            "subject_token",
        ));
        let parsed: serde_json::Value = serde_json::from_str(&subject).unwrap();
        assert_eq!(
            parsed["url"].as_str().unwrap(),
            "https://sts.us-east-1.example.dev?Action=GetCallerIdentity&Version=2011-06-15"
        );
        assert_eq!(parsed["method"].as_str().unwrap(), "POST");
        let headers = parsed["headers"].as_array().unwrap();
        let header = |name: &str| -> String {
            headers
                .iter()
                .find(|h| h["key"] == name)
                .map(|h| h["value"].as_str().unwrap().to_owned())
                .unwrap()
        };
        assert_eq!(header("host"), "sts.us-east-1.example.dev");
        assert!(header("x-amz-date").ends_with('Z'));
        assert_eq!(header("x-amz-security-token"), "session-token");
        assert_eq!(header("x-goog-cloud-target-resource"), audience());
        let authorization = header("authorization");
        assert!(authorization.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"));
        assert!(authorization.contains("/us-east-1/sts/aws4_request"));
        // The target-resource header is merged AFTER signing (source truth)
        // and must not appear in SignedHeaders.
        let signed_headers = authorization
            .split("SignedHeaders=")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap();
        assert!(!signed_headers.contains("x-goog-cloud-target-resource"));
        assert!(signed_headers.contains("x-amz-security-token"));
    }

    #[tokio::test]
    async fn external_aws_imds_flow_with_v2_token_and_region_strip() {
        let json = external_json(
            r#"{"environment_id":"aws1","region_url":"https://imdsv2.example/region","url":"https://imdsv2.example/creds","imdsv2_session_token_url":"https://imdsv2.example/token","regional_cred_verification_url":"https://sts.{region}.amazonaws.com?Action=GetCallerIdentity&Version=2011-06-15"}"#,
        );
        let transport = ScriptedTransport::new(vec![
            ScriptedTransport::text_reply(200, "imds-token"),
            ScriptedTransport::text_reply(200, "us-east-1a"),
            ScriptedTransport::text_reply(200, "imds-token"),
            ScriptedTransport::text_reply(200, "role-list"),
            ScriptedTransport::text_reply(
                200,
                r#"{"AccessKeyId":"AKIDROLE","SecretAccessKey":"role-secret","Token":"role-session"}"#,
            ),
            sts_ok(),
        ]);
        let token = resolve(&json, &MapContext::new(&[]), &transport)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.access_token.expose_secret(), "fed-token");

        // PUT session token with the fixed TTL header.
        let put = transport.request(0);
        assert_eq!(put.method, "PUT");
        assert_eq!(put.url, "https://imdsv2.example/token");
        assert!(
            put.headers
                .iter()
                .any(|(k, v)| k == "x-aws-ec2-metadata-token-ttl-seconds" && v == "300")
        );

        // Region GET strips the availability-zone suffix.
        let region_get = transport.request(1);
        assert_eq!(region_get.method, "GET");
        assert_eq!(region_get.url, "https://imdsv2.example/region");
        assert!(
            region_get
                .headers
                .iter()
                .any(|(k, v)| k == "x-aws-ec2-metadata-token" && v == "imds-token")
        );

        // Role list, then credentials for `{url}/{role}`; the STS subject is
        // signed at the stripped region.
        let role_get = transport.request(3);
        assert_eq!(role_get.url, "https://imdsv2.example/creds");
        let creds_get = transport.request(4);
        assert_eq!(creds_get.url, "https://imdsv2.example/creds/role-list");
        let subject = percent_decode(&form_value(
            transport.request(5).body.as_deref().unwrap(),
            "subject_token",
        ));
        assert!(subject.contains("sts.us-east-1.amazonaws.com"));
        assert!(subject.contains("AKIDROLE"));
    }

    #[tokio::test]
    async fn external_aws_env_id_v2_is_typed_unsupported() {
        let json = external_json(
            r#"{"environment_id":"aws2","regional_cred_verification_url":"https://sts.{region}.example.dev"}"#,
        );
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));
    }

    #[tokio::test]
    async fn external_aws_missing_regional_url_is_typed() {
        let json = external_json(r#"{"environment_id":"aws1"}"#);
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));
    }

    #[tokio::test]
    async fn external_aws_missing_region_url_when_needed_is_typed() {
        let json = external_json(
            r#"{"environment_id":"aws1","url":"https://imds.example/creds","regional_cred_verification_url":"https://sts.{region}.example.dev"}"#,
        );
        let ambient = MapContext::new(&[
            ("AWS_ACCESS_KEY_ID", "AKID"),
            ("AWS_SECRET_ACCESS_KEY", "SK"),
        ]);
        let error = resolve(&json, &ambient, &ScriptedTransport::new(vec![sts_ok()]))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::AwsSubjectUnavailable { reason: _ }
        ));
    }

    // ------------------------------------------------------------------
    // Certificate supplier (source-scoped mTLS)
    // ------------------------------------------------------------------

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/adc-external");

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new(FIXTURES).join(name)).unwrap()
    }

    fn pem_first_der(pem: &[u8]) -> Vec<u8> {
        let text = std::str::from_utf8(pem).unwrap();
        let body = text
            .split("-----BEGIN CERTIFICATE-----")
            .nth(1)
            .unwrap()
            .split("-----END CERTIFICATE-----")
            .next()
            .unwrap();
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(body.lines().collect::<String>())
            .unwrap()
    }

    /// One-shot mTLS loopback STS endpoint: requires a client certificate
    /// chaining to the fixture CA, captures the exchanged form, and replies
    /// with a fixed access token.
    async fn mtls_sts_server() -> (
        String,
        std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>,
    ) {
        use rustls::RootCertStore;
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls::server::WebPkiClientVerifier;
        use std::sync::Arc as Arc2;

        let provider = rustls::crypto::aws_lc_rs::default_provider();
        let provider = std::sync::Arc::new(provider);
        let mut roots = RootCertStore::empty();
        roots
            .add(
                CertificateDer::from_pem_file(std::path::Path::new(FIXTURES).join("ca.pem"))
                    .unwrap(),
            )
            .unwrap();
        let verifier =
            WebPkiClientVerifier::builder_with_provider(Arc2::new(roots), provider.clone())
                .build()
                .unwrap();
        let config = rustls::ServerConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![
                    CertificateDer::from_pem_file(
                        std::path::Path::new(FIXTURES).join("server.pem"),
                    )
                    .unwrap(),
                ],
                PrivateKeyDer::from_pem_file(std::path::Path::new(FIXTURES).join("server.key"))
                    .unwrap(),
            )
            .unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let captured: Arc2<Mutex<Option<(String, String)>>> = Arc2::new(Mutex::new(None));
        let captured_task = captured.clone();
        let config = Arc2::new(config);

        tokio::spawn(async move {
            // Serve a handful of one-shot exchanges so multi-part tests can
            // resolve more than once.
            for _ in 0..4 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut tls = tokio_rustls::TlsAcceptor::from(config.clone())
                    .accept(stream)
                    .await
                    .expect("client must present a CA-chained certificate");
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = tls.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buffer);
                    if let Some(header_end) = text.find("\r\n\r\n") {
                        let content_length = text
                            .split("\r\n")
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().ok())
                            })
                            .flatten()
                            .unwrap_or(0);
                        if buffer.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8_lossy(&buffer).into_owned();
                let body = request
                    .split("\r\n\r\n")
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                *captured_task.lock().unwrap() = Some((request, body));
                let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 51\r\n\r\n{\"access_token\":\"mtls-fed-token\",\"expires_in\":3600}   ";
                tls.write_all(response.as_bytes()).await.unwrap();
                tls.shutdown().await.unwrap();
            }
        });

        let scheme = "https";
        (format!("{scheme}://{addr}/v1/token"), captured)
    }

    fn certificate_creds(dir: &std::path::Path, config_name: &str) -> String {
        let config = format!(
            r#"{{"cert_configs":{{"workload":{{"cert_path":"{}","key_path":"{}"}}}}}}"#,
            std::path::Path::new(FIXTURES)
                .join("client-leaf.pem")
                .display(),
            std::path::Path::new(FIXTURES)
                .join("client-leaf.key")
                .display(),
        );
        creds_file(dir, config_name, &config);
        external_json(&format!(
            r#"{{"certificate":{{"certificate_config_location":"{}"}}}}"#,
            dir.join(config_name).display()
        ))
    }

    #[tokio::test]
    async fn external_certificate_mtls_exchange() {
        let (sts_url, captured) = mtls_sts_server().await;
        let dir = tempfile::tempdir().unwrap();
        let json = certificate_creds(dir.path(), "certificate_config.json");

        let ambient = MapContext::new(&[]);
        let transport = ScriptedTransport::new(Vec::new());
        let path = creds_file(dir.path(), "external.json", &json);
        let mut file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
        );
        let extra_roots = fixture("ca.pem");
        file_input.certificate_extra_roots_pem = Some(&extra_roots);
        // The certificate source exchanges over its own mTLS-scoped
        // transport at the loopback endpoint.
        file_input.token_endpoint = Some(&sts_url);

        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Certificate,
                workforce_user_project: None,
            }
        );
        // The plain transport is never used for the certificate STS leg.
        assert_eq!(transport.sent(), 0);

        let (request, body) = captured.lock().unwrap().clone().unwrap();
        assert!(request.starts_with("POST /v1/token"), "{request}");
        assert!(
            request
                .to_lowercase()
                .contains("content-type: application/x-www-form-urlencoded")
        );
        assert_eq!(
            form_value(&body, "grant_type"),
            "urn:ietf:params:oauth:grant-type:token-exchange"
        );
        // The subject token is the JSON certificate chain built from the
        // leaf (no trust chain configured -> leaf only).
        let subject = form_value(&body, "subject_token");
        let chain: Vec<String> = serde_json::from_str(&subject).unwrap();
        assert_eq!(chain.len(), 1);
        use base64::Engine as _;
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&chain[0])
                .unwrap(),
            pem_first_der(&fixture("client-leaf.pem"))
        );
    }

    #[tokio::test]
    async fn external_certificate_config_resolution_order_and_xor() {
        // Neither flag nor location is typed at parse time.
        let error = resolve(
            &external_json(r#"{"certificate":{}}"#),
            &MapContext::new(&[]),
            &ScriptedTransport::new(Vec::new()),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));

        // Both at once is typed as well.
        let dir = tempfile::tempdir().unwrap();
        let error = resolve(
            &external_json(
                r#"{"certificate":{"use_default_certificate_config":true,"certificate_config_location":"/x.json"}}"#,
            ),
            &MapContext::new(&[]),
            &ScriptedTransport::new(Vec::new()),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CredentialSourceInvalid { .. }
        ));
        drop(dir);
    }

    #[tokio::test]
    async fn external_certificate_env_and_well_known_resolution() {
        let (sts_url, _captured) = mtls_sts_server().await;
        let config_dir = tempfile::tempdir().unwrap();
        let config_dir_path = config_dir.path().to_path_buf();

        // GOOGLE_API_CERTIFICATE_CONFIG wins over the well-known path.
        let env_config = config_dir_path.join("env-certificate_config.json");
        let well_known_dir = config_dir_path.join("gcloud");
        std::fs::create_dir_all(&well_known_dir).unwrap();
        std::fs::write(
            &env_config,
            format!(
                r#"{{"cert_configs":{{"workload":{{"cert_path":"{}","key_path":"{}"}}}}}}"#,
                std::path::Path::new(FIXTURES)
                    .join("client-leaf.pem")
                    .display(),
                std::path::Path::new(FIXTURES)
                    .join("client-leaf.key")
                    .display(),
            ),
        )
        .unwrap();
        std::fs::write(
            well_known_dir.join("certificate_config.json"),
            "not-read-when-env-is-set",
        )
        .unwrap();

        let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
        let ambient = MapContext::new(&[
            (
                "GOOGLE_API_CERTIFICATE_CONFIG",
                env_config.to_str().unwrap(),
            ),
            ("HOME", config_dir_path.to_str().unwrap()),
        ]);
        let path = creds_file(config_dir_path.as_path(), "external.json", &json);
        let empty_transport = ScriptedTransport::new(Vec::new());
        let mut file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &empty_transport,
        );
        file_input.token_endpoint = Some(&sts_url);
        let extra_roots = fixture("ca.pem");
        file_input.certificate_extra_roots_pem = Some(&extra_roots);

        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");

        // Without the env var the well-known gcloud path is used
        // (CLOUDSDK_CONFIG pins the gcloud directory).
        let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
        let ambient = MapContext::new(&[("CLOUDSDK_CONFIG", config_dir_path.to_str().unwrap())]);
        let path = creds_file(config_dir_path.as_path(), "external2.json", &json);
        let empty_transport2 = ScriptedTransport::new(Vec::new());
        let well_known_config = format!(
            r#"{{"cert_configs":{{"workload":{{"cert_path":"{}","key_path":"{}"}}}}}}"#,
            std::path::Path::new(FIXTURES)
                .join("client-leaf.pem")
                .display(),
            std::path::Path::new(FIXTURES)
                .join("client-leaf.key")
                .display(),
        );
        std::fs::write(
            well_known_dir.join("certificate_config.json"),
            well_known_config,
        )
        .unwrap();
        let mut file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &empty_transport2,
        );
        file_input.token_endpoint = Some(&sts_url);
        let extra_roots2 = fixture("ca.pem");
        file_input.certificate_extra_roots_pem = Some(&extra_roots2);

        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");
    }

    #[tokio::test]
    async fn external_certificate_missing_everywhere_is_typed() {
        let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
        let error = resolve(
            &json,
            &MapContext::new(&[("HOME", "/nonexistent-adc-test-home")]),
            &ScriptedTransport::new(Vec::new()),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CertificateSourceUnavailable { .. }
        ));
    }

    #[test]
    fn external_certificate_chain_leaf_order_rules() {
        let leaf = pem_first_der(&fixture("client-leaf.pem"));
        let other = pem_first_der(&fixture("other-leaf.pem"));

        // Leaf absent from the chain: prepended.
        let chain = other_leaf_chain_pem();
        let ordered =
            opi_ai::auth::adc::subject::assemble_certificate_chain(&leaf, &chain).unwrap();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0], leaf);
        assert_eq!(ordered[1], other);

        // Leaf already first: order preserved.
        let mut leaf_first = fixture("client-leaf.pem");
        leaf_first.extend_from_slice(&fixture("other-leaf.pem"));
        let ordered =
            opi_ai::auth::adc::subject::assemble_certificate_chain(&leaf, &leaf_first).unwrap();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0], leaf);

        // Leaf present but not first: typed failure.
        let mut chain = fixture("client-leaf.pem");
        chain.extend_from_slice(&fixture("other-leaf.pem"));
        let error =
            opi_ai::auth::adc::subject::assemble_certificate_chain(&other, &chain).unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::CertificateSourceUnavailable { .. }
        ));
    }

    fn other_leaf_chain_pem() -> Vec<u8> {
        fixture("other-leaf.pem")
    }

    // ------------------------------------------------------------------
    // Executable supplier
    // ------------------------------------------------------------------

    /// Write a helper script for the current platform (bodies for both
    /// platforms stay plain strings so the acceptance source carries no
    /// conditional compilation) and return the executable command string.
    fn helper(dir: &std::path::Path, name: &str, unix: &str, windows: &str) -> String {
        opi_ai::test_support::portable_executable_script(dir, name, unix, windows)
    }

    fn json_escape(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn executable_json(command: &str, output_file: Option<&std::path::Path>) -> String {
        let command = json_escape(command);
        match output_file {
            Some(file) => external_json(&format!(
                r#"{{"executable":{{"command":"{command}","output_file":"{}"}}}}"#,
                file.display()
            )),
            None => external_json(&format!(r#"{{"executable":{{"command":"{command}"}}}}"#)),
        }
    }

    fn ok_response() -> String {
        let expires = 1_700_000_000 + 3600;
        format!(
            r#"{{"version":1,"success":true,"token_type":"{}","id_token":"helper-id-token","expiration_time":{expires}}}"#,
            subject_token_type()
        )
    }

    fn spawn_helper(dir: &std::path::Path, env_dump: &std::path::Path, response: &str) -> String {
        let unix = format!(
            "env > \"{}\"\nprintf '%s' '{}'",
            env_dump.display(),
            response
        );
        let windows = format!(
            "Get-ChildItem env: | ForEach-Object {{ \"$($_.Name)=$($_.Value)\" }} | Out-File -Encoding utf8 \"{}\"\nWrite-Host -NoNewline '{}'",
            env_dump.display(),
            response.replace('\'', "\"")
        );
        helper(dir, "helper", &unix, &windows)
    }

    #[test]
    fn parse_command_quotes_and_segments() {
        use opi_ai::auth::adc::executable::parse_command;

        let argv = parse_command("helper --flag \"quoted value\" tail").unwrap();
        assert_eq!(argv, vec!["helper", "--flag", "quoted value", "tail"]);

        // Mixed unquoted and quoted segments concatenate into one argument
        // with inner quotes preserved (source regex behavior).
        let argv = parse_command("a\"b c\"d").unwrap();
        assert_eq!(argv, vec![r#"a"b c"d"#]);

        // Only a fully-quoted argument loses its outer quotes.
        let argv = parse_command("\"x y\" z").unwrap();
        assert_eq!(argv, vec!["x y", "z"]);

        assert!(parse_command("   ").is_none());
    }

    #[tokio::test]
    async fn executable_gate_precedes_cache_and_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("cache.json");
        std::fs::write(&output_file, ok_response()).unwrap();
        let marker = dir.path().join("marker");
        let unix = format!("printf x > \"{}\"", marker.display());
        let windows = format!("Set-Content -Path \"{}\" -Value x", marker.display());
        let command = helper(dir.path(), "gate-helper", &unix, &windows);

        let json = executable_json(&command, Some(&output_file));
        // Gate not set (and not "1"): typed failure, cache never read, the
        // executable never spawns.
        for gate in [None, Some("0"), Some("yes")] {
            let env: Vec<(&str, &str)> = gate
                .map(|v| vec![("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", v)])
                .unwrap_or_default();
            let error = resolve(
                &json,
                &MapContext::new(&env),
                &ScriptedTransport::new(vec![sts_ok()]),
            )
            .await
            .unwrap_err();
            assert!(
                matches!(error, AdcResolutionError::ExecutablesNotAllowed),
                "gate {gate:?}"
            );
        }
        assert!(!marker.exists(), "the gate must precede any spawn");
    }

    #[tokio::test]
    async fn executable_fresh_cache_avoids_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("cache.json");
        std::fs::write(&output_file, ok_response()).unwrap();
        let env_dump = dir.path().join("env.txt");
        let command = spawn_helper(dir.path(), &env_dump, &ok_response());
        let json = executable_json(&command, Some(&output_file));

        let transport = ScriptedTransport::new(vec![sts_ok()]);
        let token = resolve(
            &json,
            &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
            &transport,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!env_dump.exists(), "fresh cache must not spawn");
        assert_eq!(
            form_value(
                transport.request(0).body.as_deref().unwrap(),
                "subject_token"
            ),
            "helper-id-token"
        );
        assert_eq!(
            token.source,
            AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Executable,
                workforce_user_project: None,
            }
        );
    }

    #[tokio::test]
    async fn executable_stale_cache_spawns_with_source_env() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("cache.json");
        // Expired response: stale cache is a miss, the executable runs.
        let stale = ok_response().replace(
            &format!("\"expiration_time\":{}", 1_700_000_000 + 3600),
            "\"expiration_time\":1000",
        );
        std::fs::write(&output_file, stale).unwrap();
        let env_dump = dir.path().join("env.txt");
        let command = spawn_helper(dir.path(), &env_dump, &ok_response());
        let impersonation = ",\"service_account_impersonation_url\":\"https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/sa@proj.iam.gserviceaccount.com:generateAccessToken\"";
        let json = format!(
            r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}"{impersonation},"credential_source":{{"executable":{{"command":"{}","output_file":"{}"}}}}}}"#,
            audience(),
            subject_token_type(),
            json_escape(&command),
            output_file.display()
        );

        // 20.21.3: the file's impersonation URL now activates the IAM leg
        // after the STS exchange, so the transport scripts both replies.
        let transport = ScriptedTransport::new(vec![
            sts_ok(),
            ScriptedTransport::text_reply(
                200,
                r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00Z"}"#,
            ),
        ]);
        resolve(
            &json,
            &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
            &transport,
        )
        .await
        .unwrap()
        .unwrap();

        let dumped = std::fs::read_to_string(&env_dump).unwrap();
        assert!(dumped.contains("GOOGLE_EXTERNAL_ACCOUNT_AUDIENCE="));
        assert!(dumped.contains(&format!(
            "GOOGLE_EXTERNAL_ACCOUNT_TOKEN_TYPE={}",
            subject_token_type()
        )));
        assert!(dumped.contains("GOOGLE_EXTERNAL_ACCOUNT_INTERACTIVE=0"));
        assert!(dumped.contains(&format!(
            "GOOGLE_EXTERNAL_ACCOUNT_OUTPUT_FILE={}",
            output_file.display()
        )));
        assert!(dumped.contains(
            "GOOGLE_EXTERNAL_ACCOUNT_IMPERSONATED_EMAIL=sa@proj.iam.gserviceaccount.com"
        ));
        assert_eq!(
            form_value(
                transport.request(0).body.as_deref().unwrap(),
                "subject_token"
            ),
            "helper-id-token"
        );
    }

    #[tokio::test]
    async fn executable_malformed_cache_is_typed_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("cache.json");
        std::fs::write(&output_file, "not json at all").unwrap();
        let env_dump = dir.path().join("env.txt");
        let command = spawn_helper(dir.path(), &env_dump, &ok_response());
        let json = executable_json(&command, Some(&output_file));

        let error = resolve(
            &json,
            &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableResponseInvalid { .. }
        ));
        // A malformed cache is terminal: the executable never runs.
        assert!(!env_dump.exists());
    }

    #[tokio::test]
    async fn executable_failure_and_validation_rules() {
        let gate = &[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")];

        // Non-success response carries code + message.
        let dir = tempfile::tempdir().unwrap();
        let env_dump = dir.path().join("env.txt");
        let failure =
            r#"{"version":1,"success":false,"code":"401","message":"helper denied"}"#.to_owned();
        let command = spawn_helper(dir.path(), &env_dump, &failure);
        let error = resolve(
            &executable_json(&command, None),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableFailed { ref code, .. } if code == "401"
        ));

        // Invalid token_type URN.
        let bad_type = ok_response().replace(subject_token_type(), "urn:custom:token");
        let dir = tempfile::tempdir().unwrap();
        let env_dump = dir.path().join("env.txt");
        let command = spawn_helper(dir.path(), &env_dump, &bad_type);
        let error = resolve(
            &executable_json(&command, None),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableResponseInvalid { .. }
        ));

        // Unsupported version.
        let dir = tempfile::tempdir().unwrap();
        let env_dump = dir.path().join("env.txt");
        let v2 = ok_response().replace("\"version\":1", "\"version\":2");
        let command = spawn_helper(dir.path(), &env_dump, &v2);
        let error = resolve(
            &executable_json(&command, None),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableResponseInvalid { .. }
        ));

        // Missing expiration_time with an output_file configured.
        let dir = tempfile::tempdir().unwrap();
        let env_dump = dir.path().join("env.txt");
        let no_expiry = ok_response().replace(
            &format!(",\"expiration_time\":{}", 1_700_000_000 + 3600),
            "",
        );
        let command = spawn_helper(dir.path(), &env_dump, &no_expiry);
        let error = resolve(
            &executable_json(&command, Some(&dir.path().join("unused-cache.json"))),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableResponseInvalid { .. }
        ));

        // Non-zero exit merges output and reports the code.
        let dir = tempfile::tempdir().unwrap();
        let unix = "printf '%s' 'boom on stderr' >&2\nexit 3";
        let windows = "Write-Error 'boom on stderr'\nexit 3";
        let command = helper(dir.path(), "failing", unix, windows);
        let error = resolve(
            &executable_json(&command, None),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableFailed { ref code, ref message }
                if code == "3" && message.contains("boom")
        ));

        // Expired response (fresh spawn) is typed.
        let dir = tempfile::tempdir().unwrap();
        let env_dump = dir.path().join("env.txt");
        let expired = ok_response().replace(
            &format!("\"expiration_time\":{}", 1_700_000_000 + 3600),
            "\"expiration_time\":1000",
        );
        let command = spawn_helper(dir.path(), &env_dump, &expired);
        let error = resolve(
            &executable_json(&command, None),
            &MapContext::new(gate),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExecutableResponseExpired
        ));
    }

    #[tokio::test]
    async fn executable_timeout_kills_the_process_group() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        let unix = format!(
            "while :; do printf x >> \"{}\"; sleep 0.1; done",
            marker.display()
        );
        let windows = format!(
            "while ($true) {{ Add-Content -Path \"{}\" -Value x; Start-Sleep -Milliseconds 100 }}",
            marker.display()
        );
        let command = helper(dir.path(), "loop", &unix, &windows);
        let json = external_json(&format!(
            r#"{{"executable":{{"command":"{}","timeout_millis":5000}}}}"#,
            json_escape(&command)
        ));

        let start = std::time::Instant::now();
        let error = resolve(
            &json,
            &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, AdcResolutionError::ExecutableTimeout));
        assert!(start.elapsed() >= std::time::Duration::from_secs(4));

        // The whole process group died: the marker stops growing.
        let size_after_kill = std::fs::metadata(&marker).map(|m| m.len()).unwrap_or(0);
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        let size_later = std::fs::metadata(&marker).map(|m| m.len()).unwrap_or(0);
        assert_eq!(size_after_kill, size_later, "process group must be reaped");
        drop(json);
    }

    #[tokio::test]
    async fn executable_timeout_range_is_parse_time() {
        // Below the 5s floor and above the 120s ceiling are typed at parse.
        for timeout in [4_999u64, 120_001] {
            let json = external_json(&format!(
                r#"{{"executable":{{"command":"sh \"/nonexistent/helper\"","timeout_millis":{timeout}}}}}"#
            ));
            let error = resolve(
                &json,
                &MapContext::new(&[]),
                &ScriptedTransport::new(Vec::new()),
            )
            .await
            .unwrap_err();
            assert!(
                matches!(error, AdcResolutionError::CredentialSourceInvalid { .. }),
                "{timeout}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Task 20.21.3 — ADC impersonation, recursive composition, keyed cache
// ---------------------------------------------------------------------------

mod adc_impersonation {
    use opi_ai::auth::adc::cache::{AdcCacheKey, AdcTokenCache};
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeInput, AdcNativeOwner, AdcResolutionError, AdcSourceHttpError,
        AdcSourceRequest, AdcSourceResponse, AdcSourceTransport, CLOUD_PLATFORM_SCOPE,
        FixedAdcClock,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, ExternalAccountSupplier,
    };
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use time::OffsetDateTime;

    const TOKEN_URL: &str = "https://source-exchange.test/token";
    const IMP_URL: &str = "https://impersonation-exchange.test/v1/generateAccessToken";
    /// The target principal this mod's files impersonate.
    const TARGET: &str = "target@proj.iam.gserviceaccount.com";
    /// Raw impersonation URL from the credentials file (its host/path is
    /// never used as the wire destination: the family reconstructs the
    /// iamcredentials endpoint from the universe domain).
    const FILE_IMP_URL: &str = "https://iamcredentials.example.com/v1beta/projects/p/serviceAccounts/target@proj.iam.gserviceaccount.com:generateAccessToken";

    #[derive(Default)]
    struct MapContext {
        env: HashMap<String, String>,
    }

    impl MapContext {
        fn new(env: &[(&str, &str)]) -> Self {
            Self {
                env: env
                    .iter()
                    .copied()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect(),
            }
        }
    }

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    struct ScriptedTransport {
        replies: Mutex<Vec<Result<AdcSourceResponse, AdcSourceHttpError>>>,
        requests: Mutex<Vec<AdcSourceRequest>>,
    }

    impl ScriptedTransport {
        fn new(replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                requests: Mutex::new(Vec::new()),
            }
        }
        fn text_reply(status: u16, body: &str) -> Result<AdcSourceResponse, AdcSourceHttpError> {
            Ok(AdcSourceResponse {
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            })
        }
        fn sent(&self) -> usize {
            self.requests.lock().unwrap().len()
        }
        fn request(&self, index: usize) -> AdcSourceRequest {
            self.requests.lock().unwrap()[index].clone()
        }
    }

    impl AdcSourceTransport for ScriptedTransport {
        fn send<'a>(
            &'a self,
            request: AdcSourceRequest,
        ) -> opi_ai::credential::BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>>
        {
            self.requests.lock().unwrap().push(request);
            let reply = self.replies.lock().unwrap().first().cloned();
            if reply.is_some() {
                let _ = self.replies.lock().unwrap().remove(0);
            }
            Box::pin(async move { reply.expect("scripted replies exhausted") })
        }
    }

    /// Transport whose sends block while armed, until released; proves
    /// single-flight merging and late-publication fencing without real
    /// waiting.
    struct GatedTransport {
        gate: Arc<tokio::sync::Notify>,
        armed: std::sync::atomic::AtomicBool,
        inner: ScriptedTransport,
    }

    impl GatedTransport {
        fn new(
            replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>,
        ) -> (Arc<Self>, Arc<tokio::sync::Notify>) {
            let gate = Arc::new(tokio::sync::Notify::new());
            (
                Arc::new(Self {
                    gate: gate.clone(),
                    armed: std::sync::atomic::AtomicBool::new(true),
                    inner: ScriptedTransport::new(replies),
                }),
                gate,
            )
        }
        fn sent(&self) -> usize {
            self.inner.sent()
        }
        /// Disarm the gate and release every waiting send.
        fn release(&self) {
            self.armed.store(false, std::sync::atomic::Ordering::SeqCst);
            self.gate.notify_waiters();
        }
    }

    impl AdcSourceTransport for GatedTransport {
        fn send<'a>(
            &'a self,
            request: AdcSourceRequest,
        ) -> opi_ai::credential::BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>>
        {
            let gate = self.gate.clone();
            let armed = self.armed.load(std::sync::atomic::Ordering::SeqCst);
            let inner = &self.inner;
            Box::pin(async move {
                if armed {
                    gate.notified().await;
                }
                inner.send(request).await
            })
        }
    }

    fn clock(unix: i64) -> Arc<FixedAdcClock> {
        Arc::new(FixedAdcClock::new(
            OffsetDateTime::from_unix_timestamp(unix).unwrap(),
        ))
    }

    fn input<'a>(
        selection: AdcAuthSelection,
        ambient: &'a dyn AuthContext,
        transport: &'a dyn AdcSourceTransport,
        clock: &'a FixedAdcClock,
    ) -> AdcNativeInput<'a> {
        AdcNativeInput::new(selection, &SCOPED, ambient, transport, clock)
            .with_token_endpoint(Some(TOKEN_URL))
    }

    static SCOPED: std::sync::LazyLock<opi_ai::auth::adc::native::ScopedAdcEnv> =
        std::sync::LazyLock::new(opi_ai::auth::adc::native::ScopedAdcEnv::empty);

    fn creds_file(dir: &std::path::Path, name: &str, contents: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn authorized_user_source() -> String {
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#.to_owned()
    }

    fn impersonated_json(extras: &str, source: &str) -> String {
        format!(
            r#"{{"type":"impersonated_service_account","source_credentials":{source},"service_account_impersonation_url":"{FILE_IMP_URL}"{extras}}}"#
        )
    }

    fn imp_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
        ScriptedTransport::text_reply(
            200,
            r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00Z"}"#,
        )
    }

    fn source_token(expires_in: i64) -> Result<AdcSourceResponse, AdcSourceHttpError> {
        ScriptedTransport::text_reply(
            200,
            &format!(r#"{{"access_token":"source-token","expires_in":{expires_in}}}"#),
        )
    }

    async fn resolve(
        json: &str,
        transport: &dyn AdcSourceTransport,
        clock: &FixedAdcClock,
    ) -> Result<Option<opi_ai::auth::adc::native::AdcResolvedToken>, AdcResolutionError> {
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(dir.path(), "impersonated.json", json);
        let ambient = MapContext::new(&[]);
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            transport,
            clock,
        );
        opi_ai::auth::adc::native::resolve_adc_native(&file_input).await
    }

    fn body_json(request: &AdcSourceRequest) -> serde_json::Value {
        serde_json::from_slice(request.body.as_deref().unwrap().as_bytes()).unwrap()
    }

    fn header<'a>(request: &'a AdcSourceRequest, name: &str) -> &'a str {
        request
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .unwrap()
    }

    // ------------------------------------------------------------------
    // Parsing
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn impersonated_missing_source_credentials_is_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let json = r#"{"type":"impersonated_service_account","service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/target@proj.iam.gserviceaccount.com:generateAccessToken"}"#;
        let error = resolve(json, &transport, &clock).await.unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::MissingField {
                family: "impersonated_service_account",
                field: "source_credentials"
            }
        ));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn impersonated_missing_impersonation_url_is_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let json = format!(
            r#"{{"type":"impersonated_service_account","source_credentials":{}}}"#,
            authorized_user_source()
        );
        let error = resolve(&json, &transport, &clock).await.unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::MissingField {
                family: "impersonated_service_account",
                field: "service_account_impersonation_url"
            }
        ));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn impersonated_target_principal_unextractable_is_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let bad = "https://iamcredentials.example.com/v1/not-a-token-request";
        let error = resolve(
            &impersonated_json("", &authorized_user_source()).replace(FILE_IMP_URL, bad),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationTargetInvalid
        ));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn impersonated_url_too_long_is_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let mut url =
            String::from("https://iamcredentials.example.com/v1/projects/-/serviceAccounts/");
        while url.len() <= 256 {
            url.push('a');
        }
        url.push_str(":generateAccessToken");
        let json = format!(
            r#"{{"type":"impersonated_service_account","source_credentials":{},"service_account_impersonation_url":"{url}"}}"#,
            authorized_user_source()
        );
        let error = resolve(&json, &transport, &clock).await.unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationTargetInvalid
        ));
        assert_eq!(transport.sent(), 0);
    }

    #[tokio::test]
    async fn impersonated_malformed_delegates_and_lifetime_are_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let error = resolve(
            &impersonated_json(r#","delegates":"not-an-array""#, &authorized_user_source()),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationConfigInvalid { .. }
        ));

        let transport = ScriptedTransport::new(vec![]);
        let error = resolve(
            &impersonated_json(r#","lifetime":-5"#, &authorized_user_source()),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationConfigInvalid { .. }
        ));
        assert_eq!(transport.sent(), 0);
    }

    // ------------------------------------------------------------------
    // Wire requests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn impersonated_reconstructs_the_wire_request_from_the_universe() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
        let token = resolve(
            &impersonated_json(
                r#","delegates":["del@proj.iam.gserviceaccount.com"],"quota_project_id":"quota-proj""#,
                &authorized_user_source(),
            ),
            &transport,
            &clock,
        )
        .await
        .unwrap()
        .unwrap();

        // The file's raw impersonation URL never becomes the destination:
        // the wire request is reconstructed from the universe domain.
        let imp = transport.request(1);
        assert_eq!(
            imp.url,
            format!(
                "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )
        );
        assert_eq!(imp.method, "POST");
        assert_eq!(header(&imp, "authorization"), "Bearer source-token");
        assert_eq!(header(&imp, "content-type"), "application/json");
        let body = body_json(&imp);
        assert_eq!(
            body["delegates"],
            serde_json::json!(["del@proj.iam.gserviceaccount.com"])
        );
        assert_eq!(
            body["scope"],
            serde_json::json!(["https://www.googleapis.com/auth/cloud-platform"])
        );
        assert_eq!(body["lifetime"], serde_json::json!("3600s"));

        // The resolved token is the impersonated one; provenance keeps the
        // source and target identities separate and carries the quota
        // project from the impersonated file.
        assert_eq!(token.access_token.expose_secret(), "imp-token");
        assert_eq!(
            token.expires_at,
            Some(
                OffsetDateTime::parse(
                    "2030-06-01T00:00:00Z",
                    &time::format_description::well_known::Rfc3339
                )
                .unwrap()
            )
        );
        assert_eq!(token.quota_project.as_deref(), Some("quota-proj"));
        match &token.source {
            AdcCredentialSource::Impersonated {
                source,
                target_principal,
                delegates,
                quota_project,
            } => {
                assert!(matches!(
                    source.as_ref(),
                    AdcCredentialSource::AuthorizedUser { quota_project }
                    if quota_project.is_none()
                ));
                assert_eq!(target_principal, TARGET);
                assert_eq!(*delegates, 1);
                assert_eq!(quota_project.as_deref(), Some("quota-proj"));
            }
            other => panic!("unexpected provenance: {other:?}"),
        }
    }

    #[tokio::test]
    async fn impersonated_lifetime_and_scopes_come_from_the_file() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
        resolve(
            &impersonated_json(
                r#","lifetime":1200,"scopes":["scope-a","scope-b"]"#,
                &authorized_user_source(),
            ),
            &transport,
            &clock,
        )
        .await
        .unwrap()
        .unwrap();

        let body = body_json(&transport.request(1));
        assert_eq!(body["lifetime"], serde_json::json!("1200s"));
        assert_eq!(body["scope"], serde_json::json!(["scope-a", "scope-b"]));
    }

    #[tokio::test]
    async fn impersonated_service_account_source_flows_identity() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
        let escaped = super::adc_native::SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
        let source = format!(
            r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}"}}"#
        );
        let token = resolve(&impersonated_json("", &source), &transport, &clock)
            .await
            .unwrap()
            .unwrap();

        // Source leg is the service-account JWT assertion exchange.
        let assertion = transport.request(0);
        let body = assertion.body.as_deref().unwrap();
        let pairs: Vec<(String, String)> = url::form_urlencoded::parse(body.as_bytes())
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert!(pairs.iter().any(|(k, _)| {
            k == "grant_type"
                && pairs
                    .iter()
                    .any(|(k2, v2)| k2 == "assertion" && v2.contains('.'))
        }));

        // The impersonated target is the stable wire identity.
        assert_eq!(token.stable_identity(), Some(TARGET));
    }

    #[tokio::test]
    async fn impersonated_recursive_nesting_has_no_new_depth_cap() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok(), imp_ok()]);
        let inner = impersonated_json("", &authorized_user_source());
        let outer = impersonated_json("", &inner);
        let token = resolve(&outer, &transport, &clock).await.unwrap().unwrap();

        // Three legs: source refresh → inner impersonation → outer
        // impersonation; the final token is the outer target's.
        assert_eq!(transport.sent(), 3);
        let AdcCredentialSource::Impersonated {
            source,
            target_principal,
            ..
        } = &token.source
        else {
            panic!("unexpected provenance: {:?}", token.source)
        };
        assert!(matches!(
            source.as_ref(),
            AdcCredentialSource::Impersonated {
                source: inner,
                ..
            } if matches!(inner.as_ref(), AdcCredentialSource::AuthorizedUser { .. })
        ));
        assert_eq!(target_principal, TARGET);
    }

    #[tokio::test]
    async fn impersonated_universe_mismatch_is_typed_and_match_flows() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![]);
        let error = resolve(
            &impersonated_json(
                r#","universe_domain":"u.example""#,
                &authorized_user_source(),
            ),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationUniverseMismatch
        ));
        assert_eq!(transport.sent(), 0);

        // An explicit universe matching the source's flows into the
        // reconstructed wire endpoint.
        let matching_source = r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","universe_domain":"u.example"}"#;
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
        resolve(
            &impersonated_json(r#","universe_domain":"u.example""#, matching_source),
            &transport,
            &clock,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            transport.request(1).url,
            format!(
                "https://iamcredentials.u.example/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )
        );
    }

    #[tokio::test]
    async fn impersonated_response_and_rejection_are_typed() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![
            source_token(3600),
            ScriptedTransport::text_reply(
                200,
                r#"{"accessToken":"imp-token","expireTime":"not-rfc3339"}"#,
            ),
        ]);
        let error = resolve(
            &impersonated_json("", &authorized_user_source()),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationResponseInvalid { .. }
        ));

        let transport = ScriptedTransport::new(vec![
            source_token(3600),
            ScriptedTransport::text_reply(200, r#"{}"#),
        ]);
        let error = resolve(
            &impersonated_json("", &authorized_user_source()),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationResponseInvalid { .. }
        ));

        let transport = ScriptedTransport::new(vec![
            source_token(3600),
            ScriptedTransport::text_reply(403, r#"{"error":"denied"}"#),
        ]);
        let error = resolve(
            &impersonated_json("", &authorized_user_source()),
            &transport,
            &clock,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ExchangeRejected {
                status: 403,
                attempts: 1
            }
        ));
    }

    // ------------------------------------------------------------------
    // external_account optional IAM impersonation
    // ------------------------------------------------------------------

    fn subject_file(dir: &std::path::Path) -> std::path::PathBuf {
        creds_file(dir, "subject.txt", "raw-subject-token")
    }

    fn external_with_impersonation(dir: &std::path::Path, extras: &str) -> String {
        let token_file = subject_file(dir);
        format!(
            r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","service_account_impersonation_url":"{FILE_IMP_URL}"{extras},"credential_source":{{"file":"{}"}}}}"#,
            token_file.display()
        )
    }

    fn sts_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
        ScriptedTransport::text_reply(200, r#"{"access_token":"fed-token","expires_in":3600}"#)
    }

    #[tokio::test]
    async fn external_impersonation_flows_after_sts_with_forced_scope() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![
            sts_ok(),
            ScriptedTransport::text_reply(
                200,
                r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00+00:00"}"#,
            ),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "external.json",
            &external_with_impersonation(
                dir.path(),
                r#","service_account_impersonation":{"token_lifetime_seconds":2700}"#,
            ),
        );
        let ambient = MapContext::new(&[]);
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
            &clock,
        )
        .with_impersonation_endpoint(Some(IMP_URL));
        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();

        // The STS leg runs with the forced cloud-platform scope.
        let sts = transport.request(0);
        let scope = url::form_urlencoded::parse(sts.body.as_deref().unwrap().as_bytes())
            .find(|(k, _)| k == "scope")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        assert_eq!(scope, "https://www.googleapis.com/auth/cloud-platform");

        // The impersonation leg posts to the configured URL with the
        // federated token; no delegates field on the external path.
        let imp = transport.request(1);
        assert_eq!(imp.url, IMP_URL);
        assert_eq!(header(&imp, "authorization"), "Bearer fed-token");
        assert_eq!(header(&imp, "content-type"), "application/json");
        let body = body_json(&imp);
        assert_eq!(
            body["scope"],
            serde_json::json!(["https://www.googleapis.com/auth/cloud-platform"])
        );
        assert_eq!(body["lifetime"], serde_json::json!("2700s"));
        assert!(body.get("delegates").is_none());

        assert_eq!(token.access_token.expose_secret(), "imp-token");
        match &token.source {
            AdcCredentialSource::Impersonated {
                source,
                target_principal,
                delegates,
                quota_project,
            } => {
                assert!(matches!(
                    source.as_ref(),
                    AdcCredentialSource::ExternalAccount {
                        supplier: ExternalAccountSupplier::File,
                        ..
                    }
                ));
                assert_eq!(target_principal, TARGET);
                assert_eq!(*delegates, 0);
                assert!(quota_project.is_none());
            }
            other => panic!("unexpected provenance: {other:?}"),
        }
    }

    #[tokio::test]
    async fn external_impersonation_defaults_and_validates_the_url() {
        let clock = clock(1_700_000_000);
        let transport = ScriptedTransport::new(vec![sts_ok(), imp_ok()]);
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "external.json",
            &external_with_impersonation(dir.path(), ""),
        );
        let ambient = MapContext::new(&[]);
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
            &clock,
        )
        .with_impersonation_endpoint(Some(IMP_URL));
        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();
        let body = body_json(&transport.request(1));
        assert_eq!(body["lifetime"], serde_json::json!("3600s"));
        assert_eq!(token.stable_identity(), Some(TARGET));

        // A URL without an extractable principal fails typed at parse time.
        let bad = external_with_impersonation(dir.path(), "")
            .replace(FILE_IMP_URL, "https://iamcredentials.example.com/v1/nope");
        let path = creds_file(dir.path(), "external-bad.json", &bad);
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
            &clock,
        );
        let error = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdcResolutionError::ImpersonationTargetInvalid
        ));
    }

    // ------------------------------------------------------------------
    // Keyed access cache
    // ------------------------------------------------------------------

    #[test]
    fn cache_keys_discriminate_every_source_dimension() {
        let delegates = vec!["del@x".to_owned()];
        let scopes = vec![CLOUD_PLATFORM_SCOPE.to_owned()];
        let base = AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        );
        let same = AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        );
        assert_eq!(base, same);

        let variants = [
            AdcCacheKey::from_parts(
                "other-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("other@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &[],
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//other"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &["https://scope.example.test/other".to_owned()],
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("other-project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("other-quota"),
                Some("endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("other-endpoint"),
                Some("universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("other-universe"),
                Some("tls-identity"),
            ),
            AdcCacheKey::from_parts(
                "config-digest",
                Some("principal@x"),
                &delegates,
                Some("//audience"),
                &scopes,
                Some("project"),
                Some("quota"),
                Some("endpoint"),
                Some("universe"),
                Some("other-tls"),
            ),
        ];
        for variant in variants {
            assert_ne!(base, variant);
        }
    }

    #[tokio::test]
    async fn owner_single_flight_merges_concurrent_resolutions() {
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "user.json",
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        );
        let (transport, _gate) = GatedTransport::new(vec![source_token(3600)]);
        let owner = Arc::new(
            AdcNativeOwner::new(
                AdcAuthSelection::CredentialsFile(path),
                transport.clone(),
                clock(1_700_000_000),
            )
            .with_token_endpoint(Some(TOKEN_URL.to_owned())),
        );

        let mut tasks = Vec::new();
        for _ in 0..5 {
            let owner = owner.clone();
            tasks.push(tokio::spawn(async move { owner.resolve_token().await }));
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        transport.release();
        for task in tasks {
            let token = task.await.unwrap().unwrap();
            assert_eq!(token.access_token.expose_secret(), "source-token");
        }
        // One acquisition served all five logical callers.
        assert_eq!(transport.sent(), 1);
    }

    #[tokio::test]
    async fn owner_short_ttl_token_serves_the_current_call_without_looping() {
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "user.json",
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        );
        // First acquisition returns a 200 s token (inside the 5-minute
        // eager window); the next acquisition fails through the budget; the
        // one after returns another short token.
        let transport = Arc::new(ScriptedTransport::new(vec![
            source_token(200),
            ScriptedTransport::text_reply(503, ""),
            ScriptedTransport::text_reply(503, ""),
            ScriptedTransport::text_reply(503, ""),
            ScriptedTransport::text_reply(503, ""),
            source_token(200),
        ]));
        let fixed = clock(1_700_000_000);
        let owner = AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            fixed.clone(),
        )
        .with_token_endpoint(Some(TOKEN_URL.to_owned()));

        let first = owner.resolve_token().await.unwrap();
        // Eager-expired but still valid: one bounded refresh attempt fails;
        // the valid cached token serves the current call.
        let second = owner.resolve_token().await.unwrap();
        assert_eq!(
            second.access_token.expose_secret(),
            first.access_token.expose_secret()
        );
        // A successful refresh that itself returns a short token is served;
        // no loop refreshes again to exceed the eager window.
        let third = owner.resolve_token().await.unwrap();
        assert_eq!(third.access_token.expose_secret(), "source-token");
        // Bounded: exactly the scripted sends were used — one acquisition
        // attempt per logical call, never a loop past the eager window.
        assert_eq!(transport.sent(), 6);
    }

    #[tokio::test]
    async fn owner_late_refresh_does_not_publish_after_invalidation() {
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "user.json",
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        );
        let (transport, gate) = GatedTransport::new(vec![
            source_token(3600),
            ScriptedTransport::text_reply(
                200,
                r#"{"access_token":"late-token","expires_in":3600}"#,
            ),
            ScriptedTransport::text_reply(
                200,
                r#"{"access_token":"fresh-token","expires_in":3600}"#,
            ),
        ]);
        let fixed = clock(1_700_000_000);
        let owner = Arc::new(
            AdcNativeOwner::new(
                AdcAuthSelection::CredentialsFile(path),
                transport.clone(),
                fixed.clone(),
            )
            .with_token_endpoint(Some(TOKEN_URL.to_owned())),
        );

        let first = {
            let owner = owner.clone();
            tokio::spawn(async move { owner.resolve_token().await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        gate.notify_waiters();
        first.await.unwrap().unwrap();

        // Cross the eager window, then start a refresh whose send is gated.
        fixed.set(
            OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
                + time::Duration::seconds(3600),
        );
        let refreshing = {
            let owner = owner.clone();
            tokio::spawn(async move { owner.resolve_token().await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        // The source is replaced while the refresh is in flight.
        owner.invalidate();
        gate.notify_waiters();
        let late = refreshing.await.unwrap().unwrap();
        assert_eq!(late.access_token.expose_secret(), "late-token");

        // The late result was not published: the next resolve re-exchanges.
        transport.release();
        let next = owner.resolve_token().await.unwrap();
        assert_eq!(next.access_token.expose_secret(), "fresh-token");
        assert_eq!(transport.sent(), 3);
    }

    #[tokio::test]
    async fn stable_identity_reports_the_wire_principal_only() {
        let clock_value = clock(1_700_000_000);
        // Impersonated: the target principal is stable wire evidence.
        let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
        let token = resolve(
            &impersonated_json("", &authorized_user_source()),
            &transport,
            &clock_value,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(token.stable_identity(), Some(TARGET));

        // Plain authorized_user and external federation files carry no
        // stable account evidence in the file itself.
        let dir = tempfile::tempdir().unwrap();
        let path = creds_file(
            dir.path(),
            "user.json",
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        );
        let ambient = MapContext::new(&[]);
        let transport = ScriptedTransport::new(vec![source_token(3600)]);
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
            &clock_value,
        );
        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.stable_identity(), None);

        let ambient = MapContext::new(&[]);
        let transport = ScriptedTransport::new(vec![sts_ok()]);
        let path = creds_file(
            dir.path(),
            "external.json",
            &format!(
                r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","credential_source":{{"file":"{}"}}}}"#,
                subject_file(dir.path()).display()
            ),
        );
        let file_input = input(
            AdcAuthSelection::CredentialsFile(path),
            &ambient,
            &transport,
            &clock_value,
        );
        let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.stable_identity(), None);
    }

    #[tokio::test]
    async fn cache_serves_fresh_tokens_and_single_flights_per_key() {
        // Direct cache-level proof of the reuse/fencing contract.
        let cache = AdcTokenCache::new();
        let key = AdcCacheKey::from_parts(
            "config",
            None,
            &[],
            None,
            &["scope".to_owned()],
            None,
            None,
            None,
            None,
            None,
        );
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let exchanges = Arc::new(Mutex::new(0usize));
        let make = |token: &'static str, exchanges: Arc<Mutex<usize>>| {
            move || {
                let exchanges = exchanges.clone();
                async move {
                    *exchanges.lock().unwrap() += 1;
                    Ok(opi_ai::auth::adc::native::AdcResolvedToken {
                        access_token: secrecy::SecretString::from(token),
                        expires_at: Some(
                            OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
                                + time::Duration::seconds(3600),
                        ),
                        quota_project: None,
                        rotated_refresh_token: None,
                        source: AdcCredentialSource::DirectMetadata,
                    })
                }
            }
        };

        let first = cache
            .resolve(key, now, make("a", exchanges.clone()))
            .await
            .unwrap();
        assert_eq!(first.access_token.expose_secret(), "a");
        let second = cache
            .resolve(key, now, make("b", exchanges.clone()))
            .await
            .unwrap();
        assert_eq!(second.access_token.expose_secret(), "a");
        assert_eq!(*exchanges.lock().unwrap(), 1);

        // Invalidation fences the next publication.
        cache.invalidate(key);
        let third = cache
            .resolve(key, now, make("c", exchanges.clone()))
            .await
            .unwrap();
        assert_eq!(third.access_token.expose_secret(), "c");
        assert_eq!(*exchanges.lock().unwrap(), 2);
    }
}

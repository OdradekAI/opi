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

#[test]
fn auth_provenance_debug_carries_no_secret() {
    use opi_ai::auth::{AuthFallback, AuthProvenance, AuthProvenanceSource};

    // Environment provenance carries the variable NAME, never a resolved value.
    let provenance = AuthProvenance {
        source: AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".to_owned(),
        },
        fallback: AuthFallback::NotAttempted,
    };
    let dbg = format!("{provenance:?}");
    assert!(
        dbg.contains("ANTHROPIC_API_KEY"),
        "env var name should be visible: {dbg}"
    );
    assert!(!dbg.contains("sk-super-secret"), "secret leaked: {dbg}");

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
        !dbg_used.contains("sk-super-secret"),
        "secret leaked: {dbg_used}"
    );
    assert!(
        dbg_used.contains("credential store unavailable"),
        "fallback reason should be visible: {dbg_used}"
    );
}

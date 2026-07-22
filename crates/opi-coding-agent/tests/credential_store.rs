//! Phase 14.1 credential store integration tests.
//!
//! Covers the versioned envelope codec (round-trip + malformed/unknown
//! distinct errors), the cross-process mutation lock (serialization + bounded
//! timeout under contention), the credential resolver (keychain-first with
//! headless env fallback), and the redaction invariant (only the secret-free
//! lock may exist on disk; secrets never reach error output). All tests use
//! the injected [`FakeKeyringBackend`] and a temp user-config root; none touch
//! the OS keychain.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
    UnknownEnvelopeField,
};
use opi_ai::provider::ProviderError;
use opi_coding_agent::credential_store::{
    ApiKeySource, BackendError, CredentialResolver, EnvLookup, FakeKeyringBackend,
    KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE, KeychainCredentialStore, KeyringBackend,
};
use secrecy::{ExposeSecret, SecretString};
use tempfile::TempDir;

const API_KEY: &str = "sk-test-api-key-DO-NOT-LEAK";
const ACCESS: &str = "atk-test-access-DO-NOT-LEAK";
const REFRESH: &str = "rtk-test-refresh-DO-NOT-LEAK";
const COPILOT_BASE_URL: &str = "https://copilot.enterprise.example/api";

#[derive(Clone, Copy)]
enum PresenceReply {
    Present,
    Absent,
    BackendUnavailable,
}

struct PresenceOnlyBackend {
    reply: PresenceReply,
    secret_get_calls: Arc<AtomicUsize>,
    presence_calls: Arc<AtomicUsize>,
}

impl KeyringBackend for PresenceOnlyBackend {
    fn get(&self, service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
        if service == "opi.presence" {
            self.presence_calls.fetch_add(1, Ordering::SeqCst);
            match self.reply {
                PresenceReply::Present => Ok(Some("api_key".to_owned())),
                PresenceReply::Absent => Ok(None),
                PresenceReply::BackendUnavailable => Err(BackendError::BackendUnavailable(
                    "no keychain daemon".to_owned(),
                )),
            }
        } else {
            self.secret_get_calls.fetch_add(1, Ordering::SeqCst);
            Err(BackendError::Other("secret get forbidden".to_owned()))
        }
    }

    fn set(&self, _service: &str, _provider_id: &str, _value: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unused set".to_owned()))
    }

    fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unused delete".to_owned()))
    }
}

struct OperationalErrorBackend;

impl KeyringBackend for OperationalErrorBackend {
    fn get(&self, _service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
        Err(BackendError::Other(
            "org.freedesktop.secrets access denied".to_owned(),
        ))
    }

    fn set(&self, _service: &str, _provider_id: &str, _value: &str) -> Result<(), BackendError> {
        Err(BackendError::Other(
            "org.freedesktop.secrets access denied".to_owned(),
        ))
    }

    fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
        Err(BackendError::Other(
            "org.freedesktop.secrets access denied".to_owned(),
        ))
    }
}

struct ScriptedOAuthRereadBackend {
    expired_raw: String,
    fresh_raw: String,
    protected_get_calls: Arc<AtomicUsize>,
}

impl KeyringBackend for ScriptedOAuthRereadBackend {
    fn get(&self, service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
        match service {
            KEYCHAIN_PRESENCE_SERVICE => Ok(Some("oauth_token".to_owned())),
            KEYCHAIN_SERVICE => {
                let call = self.protected_get_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(if call < 2 {
                    self.expired_raw.clone()
                } else {
                    self.fresh_raw.clone()
                }))
            }
            _ => Ok(None),
        }
    }

    fn set(&self, _service: &str, _provider_id: &str, _value: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unexpected scripted write".to_owned()))
    }

    fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unexpected scripted delete".to_owned()))
    }
}

struct FailingOAuthRereadBackend {
    expired_raw: String,
    protected_get_calls: Arc<AtomicUsize>,
}

impl KeyringBackend for FailingOAuthRereadBackend {
    fn get(&self, service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
        match service {
            KEYCHAIN_PRESENCE_SERVICE => Ok(Some("oauth_token".to_owned())),
            KEYCHAIN_SERVICE => {
                let call = self.protected_get_calls.fetch_add(1, Ordering::SeqCst);
                if call < 2 {
                    Ok(Some(self.expired_raw.clone()))
                } else {
                    Err(BackendError::Other("post-failure reread broke".to_owned()))
                }
            }
            _ => Ok(None),
        }
    }

    fn set(&self, _service: &str, _provider_id: &str, _value: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unexpected scripted write".to_owned()))
    }

    fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unexpected scripted delete".to_owned()))
    }
}

struct HangingOAuthProvider;

impl OAuthProvider for HangingOAuthProvider {
    fn id(&self) -> &str {
        "scripted-oauth"
    }

    fn login<'a>(
        &'a self,
        _presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused login".to_owned())) })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(std::future::pending())
    }
}

struct FailingRefreshProvider;

impl OAuthProvider for FailingRefreshProvider {
    fn id(&self) -> &str {
        "scripted-oauth"
    }

    fn login<'a>(
        &'a self,
        _presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused login".to_owned())) })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async {
            Err(ProviderError::Network(
                "original refresh failure".to_owned(),
            ))
        })
    }
}

struct BlockingRefreshProvider {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

impl OAuthProvider for BlockingRefreshProvider {
    fn id(&self) -> &str {
        "scripted-oauth"
    }

    fn login<'a>(
        &'a self,
        _presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused login".to_owned())) })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            self.started.notify_one();
            self.release.notified().await;
            Ok(OAuthCredential {
                access: secret("refreshed-access"),
                refresh: secret("refreshed-refresh"),
                expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::seconds(3600)),
                base_url: None,
                account_id: None,
            })
        })
    }
}

/// Native-adapter-shaped boundary: metadata reads are allowed, while any
/// attempt to inspect the protected entry fails loudly.
struct MarkerOnlyBackend {
    marker: Option<&'static str>,
    protected_get_calls: Arc<AtomicUsize>,
}

impl KeyringBackend for MarkerOnlyBackend {
    fn get(&self, service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
        if service == "opi.presence" {
            Ok(self.marker.map(str::to_owned))
        } else {
            self.protected_get_calls.fetch_add(1, Ordering::SeqCst);
            Err(BackendError::Other(
                "protected credential read forbidden".to_owned(),
            ))
        }
    }

    fn set(&self, _service: &str, _provider_id: &str, _value: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unused set".to_owned()))
    }

    fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
        Err(BackendError::Other("unused delete".to_owned()))
    }
}

#[derive(Clone)]
struct StepFailureBackend {
    inner: FakeKeyringBackend,
    fail_set: Arc<Mutex<VecDeque<&'static str>>>,
    fail_delete: Arc<Mutex<VecDeque<&'static str>>>,
}

#[derive(Clone)]
struct PauseAfterMarkerBackend {
    inner: FakeKeyringBackend,
    marker_written: Arc<tokio::sync::Notify>,
    release: Arc<(Mutex<bool>, std::sync::Condvar)>,
}

impl PauseAfterMarkerBackend {
    fn release(&self) {
        let (released, wake) = &*self.release;
        *released.lock().expect("release state") = true;
        wake.notify_one();
    }
}

impl KeyringBackend for PauseAfterMarkerBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        self.inner.get(service, provider_id)
    }

    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        self.inner.set(service, provider_id, value)?;
        if service == KEYCHAIN_PRESENCE_SERVICE {
            self.marker_written.notify_one();
            let (released, wake) = &*self.release;
            let mut released = released.lock().expect("release state");
            while !*released {
                released = wake.wait(released).expect("release wait");
            }
        }
        Ok(())
    }

    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        self.inner.delete(service, provider_id)
    }
}

struct UnusedOAuthProvider;

impl OAuthProvider for UnusedOAuthProvider {
    fn id(&self) -> &str {
        "unused"
    }

    fn login<'a>(
        &'a self,
        _presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused login".to_owned())) })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused refresh".to_owned())) })
    }
}

impl StepFailureBackend {
    fn new(inner: FakeKeyringBackend) -> Self {
        Self {
            inner,
            fail_set: Arc::new(Mutex::new(VecDeque::new())),
            fail_delete: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn fail_next_set(&self, service: &'static str) {
        self.fail_set
            .lock()
            .expect("set failures")
            .push_back(service);
    }

    fn fail_next_delete(&self, service: &'static str) {
        self.fail_delete
            .lock()
            .expect("delete failures")
            .push_back(service);
    }

    fn take_failure(queue: &Mutex<VecDeque<&'static str>>, service: &str) -> bool {
        let mut queue = queue.lock().expect("failure queue");
        if queue.front().is_some_and(|expected| *expected == service) {
            queue.pop_front();
            true
        } else {
            false
        }
    }
}

impl KeyringBackend for StepFailureBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        self.inner.get(service, provider_id)
    }

    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        if Self::take_failure(&self.fail_set, service) {
            Err(BackendError::Other(format!(
                "deterministic set failure for {service}"
            )))
        } else {
            self.inner.set(service, provider_id, value)
        }
    }

    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        if Self::take_failure(&self.fail_delete, service) {
            Err(BackendError::Other(format!(
                "deterministic delete failure for {service}"
            )))
        } else {
            self.inner.delete(service, provider_id)
        }
    }
}

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned().into_boxed_str())
}

/// A store over a fresh temp user-config root + the given fake backend, with a
/// short lock timeout suitable for contention tests.
fn store_with(backend: FakeKeyringBackend) -> (TempDir, KeychainCredentialStore) {
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend),
        dir.path().to_path_buf(),
        Duration::from_millis(80),
    );
    (dir, store)
}

fn api_key_credential() -> Credential {
    Credential::ApiKey(secret(API_KEY))
}

fn oauth_credential() -> Credential {
    Credential::OAuthToken {
        access: secret(ACCESS),
        refresh: secret(REFRESH),
        expires_at: None,
        base_url: Some(COPILOT_BASE_URL.to_owned()),
        account_id: None,
    }
}

#[tokio::test]
async fn api_key_envelope_round_trips_and_probes_present() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store) = store_with(backend.clone());

    assert_eq!(store.probe("anthropic").await, CredentialSource::Absent);
    assert!(store.read("anthropic").await.unwrap().is_none());

    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();

    assert_eq!(
        backend.raw_entry("opi.presence", "anthropic").as_deref(),
        Some("api_key"),
        "the closed non-secret marker is the presence/kind source"
    );

    let probed = store.probe("anthropic").await;
    assert!(
        matches!(probed, CredentialSource::Present { .. }),
        "expected Present, got {probed:?}"
    );
    // Probe label is non-secret.
    assert!(!format!("{probed:?}").contains(API_KEY));

    let read_back = store
        .read("anthropic")
        .await
        .unwrap()
        .expect("entry present after write");
    match read_back {
        Credential::ApiKey(key) => {
            assert_eq!(key.expose_secret(), API_KEY);
        }
        other => panic!("expected ApiKey, got {other:?}"),
    }
}

#[tokio::test]
async fn oauth_envelope_round_trips_and_preserves_base_url() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store) = store_with(backend.clone());

    store
        .write("github-copilot", &oauth_credential())
        .await
        .unwrap();

    assert_eq!(
        backend
            .raw_entry("opi.presence", "github-copilot")
            .as_deref(),
        Some("oauth_token")
    );

    let read_back = store
        .read("github-copilot")
        .await
        .unwrap()
        .expect("oauth entry present");
    match read_back {
        Credential::OAuthToken {
            access,
            refresh,
            base_url,
            expires_at,
            account_id,
        } => {
            assert_eq!(access.expose_secret(), ACCESS);
            assert_eq!(refresh.expose_secret(), REFRESH);
            assert_eq!(base_url.as_deref(), Some(COPILOT_BASE_URL));
            assert!(account_id.is_none());
            assert!(expires_at.is_none());
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn oauth_envelope_round_trips_optional_account_id() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store) = store_with(backend.clone());
    store
        .write(
            "openai-codex",
            &Credential::OAuthToken {
                access: secret(ACCESS),
                refresh: secret(REFRESH),
                expires_at: None,
                base_url: None,
                account_id: Some("account-123".into()),
            },
        )
        .await
        .unwrap();
    let raw = backend
        .raw_entry(KEYCHAIN_SERVICE, "openai-codex")
        .expect("persisted envelope");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["version"],
        1
    );
    let read = store.read("openai-codex").await.unwrap().unwrap();
    match read {
        Credential::OAuthToken { account_id, .. } => {
            assert_eq!(account_id.as_deref(), Some("account-123"));
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn oauth_envelope_legacy_without_account_id_still_decodes() {
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "openai-codex", "oauth_token");
    backend.seed_raw(
        KEYCHAIN_SERVICE,
        "openai-codex",
        r#"{"version":1,"kind":"oauth","access":"synthetic-access","refresh":"synthetic-refresh"}"#,
    );
    let (_dir, store) = store_with(backend);
    match store.read("openai-codex").await.unwrap().unwrap() {
        Credential::OAuthToken { account_id, .. } => assert!(account_id.is_none()),
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_removes_entry() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store.write("openai", &api_key_credential()).await.unwrap();
    assert!(matches!(
        store.probe("openai").await,
        CredentialSource::Present { .. }
    ));
    store.delete("openai").await.unwrap();
    assert_eq!(store.probe("openai").await, CredentialSource::Absent);
    assert!(store.read("openai").await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_envelope_surfaces_distinct_error() {
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", "{ this is not json");
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::MalformedEnvelope { provider, .. }) => {
            assert_eq!(provider, "anthropic");
        }
        other => panic!("expected MalformedEnvelope, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_envelope_version_surfaces_distinct_error() {
    // version 2 envelope: valid JSON + valid kind, but unknown version.
    let payload = r#"{"version":2,"kind":"api_key","api_key":"x"}"#;
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", payload);
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::UnknownEnvelope {
            version: Some(2),
            field: UnknownEnvelopeField::Version,
            ..
        }) => {}
        other => panic!("expected UnknownEnvelope version=2, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_envelope_kind_surfaces_distinct_error() {
    // version 1 but unknown kind.
    let payload = r#"{"version":1,"kind":"bogus","api_key":"x"}"#;
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", payload);
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::UnknownEnvelope {
            version: Some(1),
            field: UnknownEnvelopeField::Kind,
            ..
        }) => {}
        other => panic!("expected UnknownEnvelope kind=bogus, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_envelope_kind_never_appears_in_display_or_debug() {
    let raw_kind = format!("{API_KEY}:{ACCESS}:{REFRESH}");
    let payload = format!(r#"{{"version":1,"kind":"{raw_kind}","api_key":"x"}}"#);
    let backend = FakeKeyringBackend::new();
    backend.seed_raw("opi.presence", "anthropic", "api_key");
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", &payload);
    let (_dir, store) = store_with(backend);
    let store = Arc::new(store);

    let error = store.read("anthropic").await.unwrap_err();
    for canary in [API_KEY, ACCESS, REFRESH] {
        assert!(!format!("{error}").contains(canary));
        assert!(!format!("{error:?}").contains(canary));
    }

    let resolver = CredentialResolver::new(
        store,
        Arc::new(|_name: &str| Some("env-must-not-hide-corruption".to_owned())),
    );
    let resolver_error = match resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("unknown envelope kind must block env fallback"),
    };
    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.defaults.model = "anthropic:claude-unknown-kind".to_owned();
    let provider_error =
        match opi_coding_agent::provider_factory::build_provider_with_resolver(&config, &resolver)
            .await
        {
            Err(error) => error,
            Ok(_) => panic!("provider construction must retain the typed corruption error"),
        };
    for canary in [API_KEY, ACCESS, REFRESH] {
        for rendered in [
            format!("{resolver_error}"),
            format!("{resolver_error:?}"),
            format!("{provider_error}"),
            format!("{provider_error:?}"),
        ] {
            assert!(
                !rendered.contains(canary),
                "error leaked {canary}: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn valid_json_wrong_type_never_leaks_or_falls_back_to_env() {
    const VERSION_CANARY: &str = "malformed-version-canary-DO-NOT-LEAK";
    const EXPIRY_CANARY: &str = "malformed-expiry-canary-DO-NOT-LEAK";

    for (payload, canary) in [
        (
            format!(r#"{{"version":"{VERSION_CANARY}","kind":"api_key","api_key":"stored"}}"#),
            VERSION_CANARY,
        ),
        (
            format!(
                r#"{{"version":1,"kind":"oauth","access":"stored","refresh":"stored","expires_at":"{EXPIRY_CANARY}"}}"#
            ),
            EXPIRY_CANARY,
        ),
    ] {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw("opi.presence", "anthropic", "api_key");
        backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", &payload);
        let (_dir, store) = store_with(backend);
        let store = Arc::new(store);

        let store_error = store.read("anthropic").await.unwrap_err();
        assert!(matches!(
            store_error,
            CredentialStoreError::MalformedEnvelope { .. }
        ));
        for rendered in [format!("{store_error}"), format!("{store_error:?}")] {
            assert!(
                !rendered.contains(canary),
                "store error leaked wrong-type canary: {rendered}"
            );
        }

        let fallback_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = Arc::clone(&fallback_calls);
        let resolver = CredentialResolver::new(
            store,
            Arc::new(move |_name: &str| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                Some(API_KEY.to_owned())
            }),
        );
        let resolver_error = match resolver
            .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
            .await
        {
            Err(error) => error,
            Ok(_) => panic!("wrong-type envelope must block env fallback"),
        };
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);

        let mut config = opi_coding_agent::config::OpiConfig::default();
        config.defaults.model = "anthropic:claude-malformed-envelope".to_owned();
        let provider_error = match opi_coding_agent::provider_factory::build_provider_with_resolver(
            &config, &resolver,
        )
        .await
        {
            Err(error) => error,
            Ok(_) => panic!("provider construction must retain malformed-envelope failure"),
        };
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
        for rendered in [
            format!("{resolver_error}"),
            format!("{resolver_error:?}"),
            format!("{provider_error}"),
            format!("{provider_error:?}"),
        ] {
            assert!(
                !rendered.contains(canary),
                "resolver/provider surface leaked wrong-type canary: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn protected_entry_without_marker_is_absent_and_never_read_for_kind() {
    let protected_get_calls = Arc::new(AtomicUsize::new(0));
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::new(
        Box::new(MarkerOnlyBackend {
            marker: None,
            protected_get_calls: Arc::clone(&protected_get_calls),
        }),
        dir.path().to_path_buf(),
    );

    assert_eq!(store.probe("anthropic").await, CredentialSource::Absent);
    assert_eq!(protected_get_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn marker_reports_present_without_reading_protected_entry() {
    let protected_get_calls = Arc::new(AtomicUsize::new(0));
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::new(
        Box::new(MarkerOnlyBackend {
            marker: Some("api_key"),
            protected_get_calls: Arc::clone(&protected_get_calls),
        }),
        dir.path().to_path_buf(),
    );

    assert!(matches!(
        store.probe("anthropic").await,
        CredentialSource::Present { .. }
    ));
    assert_eq!(protected_get_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn corrupt_marker_is_redacted_and_blocks_env_fallback() {
    let marker = "sk-marker-DO-NOT-LEAK";
    let backend = FakeKeyringBackend::new();
    backend.seed_raw("opi.presence", "anthropic", marker);
    backend.seed_raw(
        KEYCHAIN_SERVICE,
        "anthropic",
        r#"{"version":1,"kind":"api_key","api_key":"stored"}"#,
    );
    let (_dir, store) = store_with(backend);
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some("env-canary".to_owned())),
    );

    let error = match resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("corrupt marker must not use the protected entry or env"),
    };
    assert!(!format!("{error}").contains(marker));
    assert!(!format!("{error:?}").contains(marker));

    let oauth_error = resolver
        .has_oauth_credential("anthropic")
        .await
        .expect_err("corrupt marker must be an operational error");
    assert!(!format!("{oauth_error}").contains(marker));
    assert!(!format!("{oauth_error:?}").contains(marker));
}

#[tokio::test]
async fn marker_first_step_failure_writes_nothing() {
    let inner = FakeKeyringBackend::new();
    let backend = StepFailureBackend::new(inner.clone());
    backend.fail_next_set(KEYCHAIN_PRESENCE_SERVICE);
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::new(Box::new(backend), dir.path().to_path_buf());

    let error = store
        .write("anthropic", &api_key_credential())
        .await
        .expect_err("marker failure must fail the write");
    assert!(format!("{error}").contains("deterministic set failure"));
    assert_eq!(inner.raw_entry(KEYCHAIN_SERVICE, "anthropic"), None);
    assert_eq!(
        inner.raw_entry(KEYCHAIN_PRESENCE_SERVICE, "anthropic"),
        None
    );
}

#[tokio::test]
async fn marker_only_state_is_typed_and_never_falls_back_to_env() {
    let inner = FakeKeyringBackend::new();
    inner.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
    let (_dir, store) = store_with(inner);
    let fallback_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&fallback_calls);
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(move |_name: &str| {
            observed_calls.fetch_add(1, Ordering::SeqCst);
            Some(API_KEY.to_owned())
        }),
    );

    assert!(matches!(
        resolver
            .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
            .await,
        Err(CredentialStoreError::CorruptMarker { .. })
    ));
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn oauth_marker_only_state_is_typed_and_never_leaks_secrets() {
    let inner = FakeKeyringBackend::new();
    inner.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "github-copilot", "oauth_token");
    let (_dir, store) = store_with(inner);
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let error = resolver
        .read_oauth_base_url("github-copilot")
        .await
        .expect_err("marker-only OAuth must remain a typed store error");
    assert!(matches!(error, ProviderError::Config(_)));
    let rendered = format!("{error:?} {error}");
    assert!(rendered.contains("corrupt credential marker for 'github-copilot'"));
    for canary in [API_KEY, ACCESS, REFRESH] {
        assert!(!rendered.contains(canary), "OAuth error leaked {canary}");
    }
}

#[tokio::test]
async fn protected_write_failure_leaves_marker_and_blocks_env_fallback() {
    let inner = FakeKeyringBackend::new();
    let backend = StepFailureBackend::new(inner.clone());
    backend.fail_next_set(KEYCHAIN_SERVICE);
    let dir = TempDir::new().expect("temp dir");
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(backend),
        dir.path().to_path_buf(),
    ));

    let error = store
        .write("anthropic", &api_key_credential())
        .await
        .expect_err("protected second-step failure must fail write");
    assert!(format!("{error}").contains("deterministic set failure"));
    assert_eq!(
        inner
            .raw_entry(KEYCHAIN_PRESENCE_SERVICE, "anthropic")
            .as_deref(),
        Some("api_key")
    );
    assert_eq!(inner.raw_entry(KEYCHAIN_SERVICE, "anthropic"), None);

    let fallback_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&fallback_calls);
    let resolver = CredentialResolver::new(
        store,
        Arc::new(move |_name: &str| {
            observed_calls.fetch_add(1, Ordering::SeqCst);
            Some(API_KEY.to_owned())
        }),
    );
    assert!(matches!(
        resolver
            .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
            .await,
        Err(CredentialStoreError::CorruptMarker { .. })
    ));
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kind_change_is_fail_closed_between_marker_and_protected_writes() {
    let dir = TempDir::new().unwrap();
    let inner = FakeKeyringBackend::new();
    let seed_store =
        KeychainCredentialStore::new(Box::new(inner.clone()), dir.path().to_path_buf());
    seed_store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();

    let backend = PauseAfterMarkerBackend {
        inner: inner.clone(),
        marker_written: Arc::new(tokio::sync::Notify::new()),
        release: Arc::new((Mutex::new(false), std::sync::Condvar::new())),
    };
    let writer_store =
        KeychainCredentialStore::new(Box::new(backend.clone()), dir.path().to_path_buf());
    let observer_store = Arc::new(KeychainCredentialStore::new(
        Box::new(inner),
        dir.path().to_path_buf(),
    ));
    let marker_written = Arc::clone(&backend.marker_written);
    let writer =
        tokio::spawn(async move { writer_store.write("anthropic", &oauth_credential()).await });
    marker_written.notified().await;

    let resolver = CredentialResolver::new(
        observer_store,
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );
    let transitional = resolver
        .read_oauth_base_url("anthropic")
        .await
        .expect_err("mixed marker/envelope state must fail closed");
    assert!(
        matches!(transitional, ProviderError::Config(ref reason)
            if reason.contains("expected oauth_token, found api_key")),
        "unexpected transitional error: {transitional:?}"
    );

    backend.release();
    writer.await.unwrap().unwrap();
    assert_eq!(
        resolver.read_oauth_base_url("anthropic").await.unwrap(),
        Some(COPILOT_BASE_URL.to_owned())
    );
}

#[tokio::test]
async fn protected_delete_failure_preserves_both_entries() {
    let inner = FakeKeyringBackend::new();
    inner.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
    inner.seed_credential(KEYCHAIN_SERVICE, "anthropic", &api_key_credential());
    let backend = StepFailureBackend::new(inner.clone());
    backend.fail_next_delete(KEYCHAIN_SERVICE);
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::new(Box::new(backend), dir.path().to_path_buf());

    store
        .delete("anthropic")
        .await
        .expect_err("protected first-step failure must fail delete");
    assert!(
        inner
            .raw_entry(KEYCHAIN_PRESENCE_SERVICE, "anthropic")
            .is_some()
    );
    assert!(inner.raw_entry(KEYCHAIN_SERVICE, "anthropic").is_some());
}

#[tokio::test]
async fn marker_delete_failure_leaves_typed_marker_only_state_and_retry_completes() {
    let inner = FakeKeyringBackend::new();
    inner.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
    inner.seed_credential(KEYCHAIN_SERVICE, "anthropic", &api_key_credential());
    let backend = StepFailureBackend::new(inner.clone());
    backend.fail_next_delete(KEYCHAIN_PRESENCE_SERVICE);
    let dir = TempDir::new().expect("temp dir");
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(backend),
        dir.path().to_path_buf(),
    ));

    store
        .delete("anthropic")
        .await
        .expect_err("marker second-step failure must fail delete");
    assert_eq!(inner.raw_entry(KEYCHAIN_SERVICE, "anthropic"), None);
    assert!(
        inner
            .raw_entry(KEYCHAIN_PRESENCE_SERVICE, "anthropic")
            .is_some()
    );

    let fallback_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&fallback_calls);
    let resolver = CredentialResolver::new(
        Arc::clone(&store),
        Arc::new(move |_name: &str| {
            observed_calls.fetch_add(1, Ordering::SeqCst);
            Some(API_KEY.to_owned())
        }),
    );
    assert!(matches!(
        resolver
            .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
            .await,
        Err(CredentialStoreError::CorruptMarker { .. })
    ));
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);

    store.delete("anthropic").await.expect("retry delete");
    assert_eq!(inner.raw_entry(KEYCHAIN_SERVICE, "anthropic"), None);
    assert_eq!(
        inner.raw_entry(KEYCHAIN_PRESENCE_SERVICE, "anthropic"),
        None
    );
}

#[tokio::test]
async fn api_key_marker_is_not_oauth_and_resolve_oauth_returns_safe_wrong_kind() {
    let api_key = "sk-kind-api-DO-NOT-LEAK";
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store
        .write("anthropic", &Credential::ApiKey(secret(api_key)))
        .await
        .unwrap();
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| -> Option<String> { None }),
    );

    assert!(!resolver.has_oauth_credential("anthropic").await.unwrap());
    let error = resolver
        .resolve_oauth("anthropic", &UnusedOAuthProvider)
        .await
        .expect_err("API key must return wrong-kind, not CredentialNeeded");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    assert!(format!("{error}").contains("expected oauth_token, found api_key"));
    assert!(!format!("{error}").contains(api_key));
    assert!(!format!("{error:?}").contains(api_key));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutation_lock_serializes_concurrent_writers() {
    // Two stores over CLONED (Arc-shared) backends + the same lock file: both
    // writes target genuinely shared state. The proof the lock serializes them
    // is that the two `set` critical-section windows do NOT overlap (without
    // the lock, two 120ms sets launched 20ms apart would overlap heavily).
    let dir = TempDir::new().unwrap();
    let backend = FakeKeyringBackend::new().with_set_delay(Duration::from_millis(120));
    let store_a = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend.clone()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );
    let store_b = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend.clone()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );

    let a = tokio::spawn(async move { store_a.write("anthropic", &api_key_credential()).await });
    // Give writer A a head start so it holds the lock first.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let b = tokio::spawn(async move {
        store_b
            .write("anthropic", &Credential::ApiKey(secret("sk-other")))
            .await
    });
    a.await.unwrap().unwrap();
    b.await.unwrap().unwrap();

    // (1) Serialization proof: the two set windows do not overlap. If the lock
    // were removed from KeychainCredentialStore::write, both 120ms sets would
    // run concurrently and these windows would overlap.
    let mut windows = backend.set_windows();
    windows.sort_by_key(|(start, _)| *start);
    assert_eq!(
        windows.len(),
        2,
        "expected exactly two recorded set windows, got {windows:?}"
    );
    let (a_start, a_end) = windows[0];
    let (b_start, b_end) = windows[1];
    assert!(
        b_start >= a_end,
        "writers overlapped (not serialized): A={a_start:?}..{a_end:?}, B={b_start:?}..{b_end:?}"
    );

    // (2) No corruption: the persisted value is exactly one of the two written
    // secrets (the locked overwrite is atomic, never torn).
    let persisted = backend
        .raw_entry(KEYCHAIN_SERVICE, "anthropic")
        .expect("entry present after both writes");
    let valid = persisted.contains(API_KEY) ^ persisted.contains("sk-other");
    assert!(
        valid,
        "persisted envelope should contain exactly one written secret: {persisted:?}"
    );
    // The OTHER writer's secret must not be in the final envelope.
    if persisted.contains(API_KEY) {
        assert!(!persisted.contains("sk-other"));
    } else {
        assert!(!persisted.contains(API_KEY));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oauth_refresh_serializes_against_public_write_on_the_same_lock_path() {
    let dir = TempDir::new().unwrap();
    let backend = FakeKeyringBackend::new();
    let refresh_store = Arc::new(KeychainCredentialStore::with_lock_timeout(
        Box::new(backend.clone()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    ));
    let writer_store = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );
    refresh_store
        .write(
            "scripted-oauth",
            &Credential::OAuthToken {
                access: secret("expired-access"),
                refresh: secret("expired-refresh"),
                expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::seconds(60)),
                base_url: None,
                account_id: None,
            },
        )
        .await
        .unwrap();

    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let resolver = CredentialResolver::new(refresh_store, Arc::new(|_: &str| None));
    let refresh_provider = BlockingRefreshProvider {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let refresh = tokio::spawn(async move {
        resolver
            .resolve_oauth("scripted-oauth", &refresh_provider)
            .await
    });
    started.notified().await;

    let mut writer = tokio::spawn(async move {
        writer_store
            .write(
                "scripted-oauth",
                &Credential::ApiKey(secret("replacement-api-key")),
            )
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut writer)
            .await
            .is_err(),
        "public write completed while refresh held the shared mutation lock"
    );

    release.notify_one();
    refresh.await.unwrap().unwrap();
    writer.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutation_lock_times_out_under_contention() {
    // Writer A holds the lock for ~150ms (slow backend). Writer B has an 80ms
    // lock timeout and must time out with a redacted Backend error.
    let dir = TempDir::new().unwrap();
    let store_a = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new().with_set_delay(Duration::from_millis(150))),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );
    let store_b = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
        Duration::from_millis(80),
    );

    let a = tokio::spawn(async move { store_a.write("anthropic", &api_key_credential()).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let b_err = store_b
        .write("anthropic", &api_key_credential())
        .await
        .unwrap_err();
    // Let A finish so the temp dir cleanup is clean.
    a.await.unwrap().unwrap();

    match b_err {
        CredentialStoreError::Backend { reason, .. } => {
            assert!(
                reason.contains("timeout"),
                "expected timeout reason, got {reason:?}"
            );
            // Reason is coordination-only: never a secret.
            assert!(!reason.contains(API_KEY));
        }
        other => panic!("expected Backend(timeout), got {other:?}"),
    }
}

#[tokio::test]
async fn losing_lock_contender_does_not_mutate_existing_lock_file() {
    let dir = TempDir::new().unwrap();
    let lock_path = dir.path().join("credential.lock");
    let sentinel = b"lock-owner-sentinel";
    std::fs::write(&lock_path, sentinel).unwrap();
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    fs4::FileExt::lock(&owner).unwrap();

    let contender = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
        Duration::from_millis(40),
    );
    let error = contender
        .write("anthropic", &api_key_credential())
        .await
        .expect_err("losing contender must time out");
    assert!(
        matches!(
            error,
            CredentialStoreError::Backend { ref reason, .. } if reason.contains("timeout")
        ),
        "{error:?}"
    );
    fs4::FileExt::unlock(&owner).unwrap();
    assert_eq!(
        std::fs::read(&lock_path).unwrap(),
        sentinel,
        "opening or contending for the lock must not change owner bytes"
    );
}

#[tokio::test]
async fn refresh_timeout_rereads_and_returns_newer_scripted_credential() {
    let dir = TempDir::new().unwrap();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let expired_raw = serde_json::json!({
        "version": 1,
        "kind": "oauth",
        "access": "expired-access",
        "refresh": "expired-refresh",
        "expires_at": now + 60,
    })
    .to_string();
    let fresh_raw = serde_json::json!({
        "version": 1,
        "kind": "oauth",
        "access": "newer-access",
        "refresh": "newer-refresh",
        "expires_at": now + 3600,
    })
    .to_string();
    let protected_get_calls = Arc::new(AtomicUsize::new(0));
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(ScriptedOAuthRereadBackend {
            expired_raw,
            fresh_raw,
            protected_get_calls: Arc::clone(&protected_get_calls),
        }),
        dir.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::with_refresh_timeout(
        store,
        Arc::new(|_: &str| None),
        Duration::from_millis(20),
    );

    let resolved = resolver
        .resolve_oauth("scripted-oauth", &HangingOAuthProvider)
        .await
        .expect("post-timeout reread returns the newer credential");
    assert_eq!(resolved.secret.expose_secret(), "newer-access");
    assert_eq!(
        protected_get_calls.load(Ordering::SeqCst),
        3,
        "fast read, locked read, and post-timeout reread"
    );
}

fn resolver_with_failing_post_refresh_reread(
    refresh_timeout: Duration,
) -> (TempDir, CredentialResolver, Arc<AtomicUsize>) {
    let dir = TempDir::new().unwrap();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let expired_raw = serde_json::json!({
        "version": 1,
        "kind": "oauth",
        "access": "expired-access",
        "refresh": "expired-refresh",
        "expires_at": now + 60,
    })
    .to_string();
    let protected_get_calls = Arc::new(AtomicUsize::new(0));
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FailingOAuthRereadBackend {
            expired_raw,
            protected_get_calls: Arc::clone(&protected_get_calls),
        }),
        dir.path().to_path_buf(),
    ));
    let resolver =
        CredentialResolver::with_refresh_timeout(store, Arc::new(|_: &str| None), refresh_timeout);
    (dir, resolver, protected_get_calls)
}

#[tokio::test]
async fn refresh_timeout_survives_failed_post_failure_reread() {
    let (_dir, resolver, protected_get_calls) =
        resolver_with_failing_post_refresh_reread(Duration::from_millis(20));

    let error = resolver
        .resolve_oauth("scripted-oauth", &HangingOAuthProvider)
        .await
        .expect_err("timeout remains the primary failure");
    assert!(
        matches!(
            error,
            ProviderError::AuthFailed(ref reason)
                if reason == "OAuth refresh timed out for provider 'scripted-oauth'"
        ),
        "post-failure reread must not replace the typed timeout: {error:?}"
    );
    assert_eq!(protected_get_calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn provider_refresh_error_survives_failed_post_failure_reread() {
    let (_dir, resolver, protected_get_calls) =
        resolver_with_failing_post_refresh_reread(Duration::from_secs(1));

    let error = resolver
        .resolve_oauth("scripted-oauth", &FailingRefreshProvider)
        .await
        .expect_err("provider refresh failure remains primary");
    assert!(
        matches!(
            error,
            ProviderError::Network(ref reason) if reason == "original refresh failure"
        ),
        "post-failure reread must not replace the provider error: {error:?}"
    );
    assert_eq!(protected_get_calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn resolver_reads_api_key_from_store_when_present() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| -> Option<String> { None }),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("store read succeeds")
        .expect("resolved");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    assert!(matches!(resolved.source, ApiKeySource::Store));
}

#[tokio::test]
async fn resolver_falls_back_to_env_when_store_absent() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    // Store is empty; env has the key.
    let env_lookup: EnvLookup = {
        let env_value = API_KEY.to_owned();
        Arc::new(move |_name: &str| Some(env_value.clone()))
    };
    let resolver = CredentialResolver::new(Arc::new(store), env_lookup);

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("store read succeeds")
        .expect("resolved from env");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    match resolved.source {
        ApiKeySource::Env {
            env_var,
            backend_unavailable,
        } => {
            assert_eq!(env_var, "ANTHROPIC_API_KEY");
            assert!(
                !backend_unavailable,
                "absent store must not set backend_unavailable"
            );
        }
        other => panic!("expected Env source, got {other:?}"),
    }
}

#[tokio::test]
async fn keychain_probe_uses_presence_without_reading_secret() {
    for reply in [
        PresenceReply::Present,
        PresenceReply::Absent,
        PresenceReply::BackendUnavailable,
    ] {
        let secret_get_calls = Arc::new(AtomicUsize::new(0));
        let presence_calls = Arc::new(AtomicUsize::new(0));
        let dir = TempDir::new().expect("temp dir");
        let store = KeychainCredentialStore::new(
            Box::new(PresenceOnlyBackend {
                reply,
                secret_get_calls: Arc::clone(&secret_get_calls),
                presence_calls: Arc::clone(&presence_calls),
            }),
            dir.path().to_path_buf(),
        );

        let source = store.probe("anthropic").await;
        match reply {
            PresenceReply::Present => {
                assert!(matches!(source, CredentialSource::Present { .. }))
            }
            PresenceReply::Absent => assert_eq!(source, CredentialSource::Absent),
            PresenceReply::BackendUnavailable => assert!(matches!(
                source,
                CredentialSource::BackendUnavailable { .. }
            )),
        }
        assert_eq!(secret_get_calls.load(Ordering::SeqCst), 0);
        assert_eq!(presence_calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn resolver_operational_backend_error_never_falls_back_to_env() {
    let dir = TempDir::new().expect("temp dir");
    let store =
        KeychainCredentialStore::new(Box::new(OperationalErrorBackend), dir.path().to_path_buf());
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let result = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await;
    assert!(matches!(
        result,
        Err(CredentialStoreError::Backend { ref reason, .. })
            if reason.contains("access denied")
    ));
}

#[tokio::test]
async fn resolver_corrupt_envelope_never_falls_back_to_env() {
    let backend = FakeKeyringBackend::new();
    backend.seed_raw("opi.presence", "anthropic", "api_key");
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", "{ malformed");
    let (_dir, store) = store_with(backend);
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await;

    assert!(matches!(
        resolved,
        Err(CredentialStoreError::MalformedEnvelope { .. })
    ));
}

#[tokio::test]
async fn resolver_unknown_envelope_never_falls_back_to_env() {
    let backend = FakeKeyringBackend::new();
    backend.seed_raw("opi.presence", "anthropic", "api_key");
    backend.seed_raw(
        KEYCHAIN_SERVICE,
        "anthropic",
        r#"{"version":999,"kind":"api_key","api_key":"stored"}"#,
    );
    let (_dir, store) = store_with(backend);
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await;

    assert!(matches!(
        resolved,
        Err(CredentialStoreError::UnknownEnvelope {
            version: Some(999),
            ..
        })
    ));
}

#[tokio::test]
async fn resolver_wrong_credential_kind_never_falls_back_to_env() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store.write("anthropic", &oauth_credential()).await.unwrap();
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await;

    assert!(matches!(
        resolved,
        Err(CredentialStoreError::UnexpectedCredentialKind {
            expected: "api_key",
            actual: "oauth_token",
            ..
        })
    ));
}

#[tokio::test]
async fn resolver_backend_unavailable_falls_back_to_env_with_diagnostic_bit() {
    let (_dir, store) = store_with(FakeKeyringBackend::new().with_unavailable());
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| Some(API_KEY.to_owned())),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("store read succeeds")
        .expect("backend unavailability should fall back to env");

    assert!(matches!(
        resolved.source,
        ApiKeySource::Env {
            backend_unavailable: true,
            ..
        }
    ));
}

#[tokio::test]
async fn headless_api_key_env_fallback() {
    // Acceptance scenario: keychain backend unavailable -> resolver resolves
    // the API key from the configured env source, records the
    // backend-unavailable fallback flag, and exposes no plaintext artifact.
    let (_dir, store) = store_with(FakeKeyringBackend::new().with_unavailable());
    // Probe of an unavailable backend surfaces BackendUnavailable.
    assert!(matches!(
        store.probe("anthropic").await,
        CredentialSource::BackendUnavailable { .. }
    ));

    let env_lookup: EnvLookup = {
        let env_value = API_KEY.to_owned();
        Arc::new(move |_name: &str| Some(env_value.clone()))
    };
    let resolver = CredentialResolver::new(Arc::new(store), env_lookup);

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("store read succeeds")
        .expect("resolved from env on headless host");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    match resolved.source {
        ApiKeySource::Env {
            ref env_var,
            backend_unavailable,
        } => {
            assert_eq!(env_var, "ANTHROPIC_API_KEY");
            assert!(
                backend_unavailable,
                "headless fallback must report backend_unavailable"
            );
        }
        other => panic!("expected Env source, got {other:?}"),
    }

    // No plaintext artifact: the resolved key never appears in the source
    // debug, and the resolver holds the value only behind SecretString.
    let source_debug = format!("{:?}", resolved.source);
    assert!(
        !source_debug.contains(API_KEY),
        "source leaked key: {source_debug}"
    );
}

#[tokio::test]
async fn redaction_only_secret_free_lock_exists_outside_fake_keyring() {
    // After persisting a credential, the temp user-config root must contain
    // only the secret-free credential.lock — never a plaintext credential
    // artifact. The FakeKeyringBackend is in-memory, so no envelope is written
    // to disk at all. The scan also covers the read-back Credential Debug and
    // the formatted MalformedEnvelope error channel (spec lines 263-267).
    let backend = FakeKeyringBackend::new();
    // Seed a malformed payload that embeds the API-key secret, so the
    // formatted-error redaction check below is non-vacuous (a regression that
    // echoed the raw payload into the error would leak it).
    let malformed_with_secret = format!(r#"{{ "version": 1, "api_key": "{API_KEY}" "#);
    backend.seed_raw(
        KEYCHAIN_SERVICE,
        "malformed-provider",
        &malformed_with_secret,
    );
    let (dir, store) = store_with(backend);
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    store
        .write("github-copilot", &oauth_credential())
        .await
        .unwrap();

    let mut entries: Vec<std::ffi::OsString> = std::fs::read_dir(dir.path())
        .expect("read temp dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("credential.lock")],
        "only the secret-free lock may exist outside the fake keyring"
    );

    // The lock file itself holds no secret.
    let lock_contents =
        std::fs::read_to_string(dir.path().join("credential.lock")).unwrap_or_default();
    for secret in [API_KEY, ACCESS, REFRESH] {
        assert!(
            !lock_contents.contains(secret),
            "lock file leaked secret {secret:?}"
        );
    }

    // Read-back credential Debug never leaks access/refresh (the serialized
    // envelope leaves the secret only behind SecretString's redacting Debug).
    let read_back = store
        .read("github-copilot")
        .await
        .unwrap()
        .expect("oauth entry present");
    let cred_debug = format!("{read_back:?}");
    for secret in [API_KEY, ACCESS, REFRESH] {
        assert!(
            !cred_debug.contains(secret),
            "Credential Debug leaked secret {secret:?}: {cred_debug}"
        );
    }

    // Formatted-error channel: a malformed envelope error never echoes the
    // secret-bearing payload (seeded above with an embedded API-key secret).
    let err = store.read("malformed-provider").await.unwrap_err();
    let err_display = format!("{err}");
    let err_debug = format!("{err:?}");
    for text in [&err_display, &err_debug] {
        assert!(
            !text.contains(API_KEY),
            "malformed-envelope error leaked the payload secret: {text}"
        );
        assert!(
            !text.contains(ACCESS) && !text.contains(REFRESH),
            "error leaked access/refresh: {text}"
        );
    }
}

#[tokio::test]
async fn keychain_store_reaches_production_construction() {
    // Acceptance scenario `phase14-keychain-backend-production-construction`:
    // instantiate KeychainCredentialStore over an injected fake keyring,
    // round-trip the envelope, compose the store with CredentialResolver, and
    // reach provider construction without touching the user keychain or writing
    // plaintext credential material.
    use opi_coding_agent::credential_store::{CredentialResolver, KeychainCredentialStore};

    let dir = TempDir::new().unwrap();
    let store: Arc<KeychainCredentialStore> = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));

    // Round-trip a stored API-key envelope (the resolver's keychain source).
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    assert!(matches!(
        store.probe("anthropic").await,
        CredentialSource::Present { .. }
    ));

    // Compose with a resolver whose env lookup is empty, so the key must come
    // from the store.
    let resolver = CredentialResolver::new(Arc::clone(&store), {
        let lookup: EnvLookup = Arc::new(|_name: &str| -> Option<String> { None });
        lookup
    });

    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.defaults.model = "anthropic:claude-store-construction".to_owned();
    let provider =
        opi_coding_agent::provider_factory::build_provider_with_resolver(&config, &resolver)
            .await
            .expect("provider constructs from the resolved key");
    assert_eq!(provider.id(), "anthropic");

    // No plaintext credential artifact on disk: only the secret-free lock.
    let mut entries: Vec<std::ffi::OsString> = std::fs::read_dir(dir.path())
        .expect("read temp dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("credential.lock")],
        "only the secret-free lock may exist outside the fake keyring"
    );
    let lock_contents =
        std::fs::read_to_string(dir.path().join("credential.lock")).unwrap_or_default();
    assert!(!lock_contents.contains(API_KEY));
}

//! Behavioral tests for --list-models (task 2.1).
//!
//! Production command output/exit contracts live in the binary unit tests,
//! where the exact command core receives an injected fake keyring factory.
//! These integration tests cover registry and asynchronous listing behavior
//! without spawning `opi` or touching the user keychain.

use std::ffi::{OsStr, OsString};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::AssistantStreamEvent;
use opi_coding_agent::model_listing::model_entries_from_registry;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const BEDROCK_ENV_NAMES: [&str; 5] = [
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_PROFILE",
    "OPI_TEST_BEDROCK_SECRET",
];

struct ScopedEnv {
    original: Vec<(String, Option<OsString>)>,
    lock: Option<MutexGuard<'static, ()>>,
}

impl ScopedEnv {
    fn new(names: &[&str]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut original = Vec::with_capacity(names.len());
        for name in names {
            assert!(
                !original
                    .iter()
                    .any(|(tracked, _): &(String, Option<OsString>)| tracked == name),
                "duplicate env var {name}"
            );
            original.push(((*name).to_owned(), std::env::var_os(name)));
        }
        Self {
            original,
            lock: Some(lock),
        }
    }

    fn cleared_with_values(names: &[&str], values: &[(&str, &str)]) -> Self {
        let env = Self::new(names);
        for name in names {
            env.remove(name);
        }
        for (name, value) in values {
            env.set(name, value);
        }
        env
    }

    fn set(&self, name: &str, value: impl AsRef<OsStr>) {
        self.assert_tracked(name);
        // SAFETY: this guard holds ENV_LOCK and restores the original value on drop.
        unsafe { std::env::set_var(name, value) };
    }

    fn remove(&self, name: &str) {
        self.assert_tracked(name);
        // SAFETY: this guard holds ENV_LOCK and restores the original value on drop.
        unsafe { std::env::remove_var(name) };
    }

    fn assert_tracked(&self, name: &str) {
        assert!(
            self.original.iter().any(|(tracked, _)| tracked == name),
            "untracked env var {name}"
        );
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (name, value) in &self.original {
            match value {
                // SAFETY: this guard still holds ENV_LOCK and is restoring the exact value.
                Some(value) => unsafe { std::env::set_var(name, value) },
                // SAFETY: this guard still holds ENV_LOCK and is restoring absence.
                None => unsafe { std::env::remove_var(name) },
            }
        }
        drop(self.lock.take());
    }
}

#[test]
fn process_env_guard_restores_on_unwind_and_recovers_poisoned_lock() {
    const ENV_NAME: &str = "OPI_TEST_LIST_MODELS_ENV_GUARD";
    let original = std::env::var_os(ENV_NAME);

    let unwind = std::panic::catch_unwind(|| {
        let env = ScopedEnv::new(&[ENV_NAME]);
        env.set(ENV_NAME, "panic-canary");
        assert_eq!(std::env::var(ENV_NAME).as_deref(), Ok("panic-canary"));
        panic!("exercise unwind restoration");
    });
    assert!(unwind.is_err());
    assert_eq!(std::env::var_os(ENV_NAME), original);

    {
        let env = ScopedEnv::new(&[ENV_NAME]);
        env.set(ENV_NAME, "poison-recovery-canary");
        assert_eq!(
            std::env::var(ENV_NAME).as_deref(),
            Ok("poison-recovery-canary")
        );
    }
    assert_eq!(std::env::var_os(ENV_NAME), original);
}

struct ListingPresenceBackend {
    secret_get_calls: Arc<AtomicUsize>,
    presence_calls: Arc<AtomicUsize>,
}

struct ListingOperationalProbeBackend;

impl opi_coding_agent::credential_store::KeyringBackend for ListingOperationalProbeBackend {
    fn get(
        &self,
        service: &str,
        _provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        assert_eq!(service, "opi.presence");
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "credential service access denied".to_owned(),
        ))
    }

    fn set(
        &self,
        _service: &str,
        _provider_id: &str,
        _value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused set".to_owned(),
        ))
    }

    fn delete(
        &self,
        _service: &str,
        _provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused delete".to_owned(),
        ))
    }
}

impl opi_coding_agent::credential_store::KeyringBackend for ListingPresenceBackend {
    fn get(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        if service == "opi.presence" {
            self.presence_calls.fetch_add(1, Ordering::SeqCst);
            Ok((provider_id == "anthropic").then(|| "api_key".to_owned()))
        } else {
            self.secret_get_calls.fetch_add(1, Ordering::SeqCst);
            Err(opi_coding_agent::credential_store::BackendError::Other(
                "secret get forbidden".to_owned(),
            ))
        }
    }

    fn set(
        &self,
        _service: &str,
        _provider_id: &str,
        _value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused set".to_owned(),
        ))
    }

    fn delete(
        &self,
        _service: &str,
        _provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused delete".to_owned(),
        ))
    }
}

struct OrderingKeyringBackend {
    inner: opi_coding_agent::credential_store::FakeKeyringBackend,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl OrderingKeyringBackend {
    fn inner(&self) -> &opi_coding_agent::credential_store::FakeKeyringBackend {
        &self.inner
    }

    fn record_entry_creation(&self) {
        self.events
            .lock()
            .expect("ordering events")
            .push("entry_creation");
    }
}

impl opi_coding_agent::credential_store::KeyringBackend for OrderingKeyringBackend {
    fn get(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::get(self.inner(), service, provider_id)
    }

    fn set(
        &self,
        service: &str,
        provider_id: &str,
        value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::set(
            self.inner(),
            service,
            provider_id,
            value,
        )
    }

    fn delete(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::delete(
            self.inner(),
            service,
            provider_id,
        )
    }
}

impl Drop for OrderingKeyringBackend {
    fn drop(&mut self) {
        self.events
            .lock()
            .expect("ordering events")
            .push("guard_drop");
    }
}

fn assert_native_entry_drop_order(events: &Arc<Mutex<Vec<&'static str>>>) {
    let events = events.lock().expect("ordering events");
    assert_eq!(events.first(), Some(&"native_install"), "{events:?}");
    let first_entry = events
        .iter()
        .position(|event| *event == "entry_creation")
        .expect("at least one keyring entry creation");
    let guard_drop = events
        .iter()
        .position(|event| *event == "guard_drop")
        .expect("native guard drop event");
    assert!(0 < first_entry && first_entry < guard_drop, "{events:?}");
    assert_eq!(events.last(), Some(&"guard_drop"), "{events:?}");
}

struct TestProvider {
    id: String,
    models: Vec<ModelInfo>,
}

impl Provider for TestProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let stream: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        Box::pin(futures_util::stream::iter(stream))
    }
}

fn model(id: &str, display_name: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        display_name,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    )
}

#[test]
fn model_entries_from_registry_include_overrides() {
    let mut registry = opi_ai::ProviderRegistry::new();
    registry
        .register_provider(Box::new(TestProvider {
            id: "provider-a".into(),
            models: vec![model("base", "Base")],
        }))
        .unwrap();
    registry
        .register_model("provider-a", model("extra", "Extra"))
        .unwrap();

    let entries = model_entries_from_registry(&registry);

    assert!(entries.iter().any(|entry| entry.provider_id == "provider-a"
        && entry.model_id == "base"
        && entry.display_name == "Base"));
    assert!(entries.iter().any(|entry| entry.provider_id == "provider-a"
        && entry.model_id == "extra"
        && entry.display_name == "Extra"));
}

// ---------------------------------------------------------------------------
// Phase 14.1: stored-credential metadata is redacted (acceptance scenario)
// ---------------------------------------------------------------------------

/// Acceptance scenario `phase14-store-probe-surfaces` (list-models half):
/// with a keychain-backend config, `build_collection_for_listing` carries the
/// redacted, precomputed probe for the StoreCredential provider and exposes no
/// credential value through the listing entries. The env key is set only so the
/// provider constructs for model enumeration; it never reaches the probe or the
/// listing rows.
#[test]
fn stored_credential_metadata_is_redacted() {
    use std::collections::HashMap;

    use opi_ai::{AuthDescriptor, CredentialSource};
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let env = ScopedEnv::new(&["ANTHROPIC_API_KEY"]);
    let secret = "sk-listmodels-DO-NOT-LEAK";
    env.set("ANTHROPIC_API_KEY", secret);

    let outcome = build_collection_for_listing(
        &{
            let mut config = opi_coding_agent::config::OpiConfig::default();
            config.defaults.model = "anthropic:claude-listmodels".into();
            config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
            config
        },
        &{
            let mut probe_map = HashMap::new();
            probe_map.insert(
                "anthropic".to_string(),
                CredentialSource::Present {
                    label: "keychain opi:anthropic".to_owned(),
                },
            );
            probe_map
        },
    );

    let collection = outcome.expect("listing collection builds with env key");
    // Redacted probe carried, no secret.
    let probe = collection.probe("anthropic").expect("probe injected");
    assert!(
        matches!(probe, CredentialSource::Present { .. }),
        "expected Present probe, got {probe:?}"
    );
    assert!(!format!("{probe:?}").contains(secret));
    // StoreCredential descriptor, no secret.
    let desc = collection
        .auth_descriptor("anthropic")
        .expect("descriptor present");
    assert!(
        matches!(desc, AuthDescriptor::StoreCredential { .. }),
        "expected StoreCredential, got {desc:?}"
    );
    assert!(!format!("{desc:?}").contains(secret));
    // Model rows enumerate and carry no secret.
    let entries = model_entries_from_registry(collection.registry());
    assert!(entries.iter().any(|e| e.provider_id == "anthropic"));
    for e in &entries {
        assert!(!e.provider_id.contains(secret));
        assert!(!e.model_id.contains(secret));
        assert!(!e.display_name.contains(secret));
    }
}

#[test]
fn anthropic_oauth_env_alone_enables_secret_free_listing() {
    use std::collections::HashMap;

    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let env = ScopedEnv::new(&["ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN"]);
    let oauth_canary = "oauth-listing-canary-DO-NOT-LEAK";
    env.remove("ANTHROPIC_API_KEY");
    env.set("ANTHROPIC_OAUTH_TOKEN", oauth_canary);

    let outcome = build_collection_for_listing(
        &opi_coding_agent::config::OpiConfig::default(),
        &HashMap::new(),
    );

    let collection = outcome.expect("OAuth-env-only Anthropic listing");
    let entries = model_entries_from_registry(collection.registry());
    assert!(entries.iter().any(|entry| entry.provider_id == "anthropic"));
    let rendered = format!(
        "{:?}{:?}{}",
        collection.auth_descriptor("anthropic"),
        collection.auth_status("anthropic"),
        entries
            .iter()
            .map(|entry| format!(
                "{}:{}:{}",
                entry.provider_id, entry.model_id, entry.display_name
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(!rendered.contains(oauth_canary));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited listing core never re-acquires this lock.
async fn listing_uses_selected_credential_kind_and_source_label() {
    use opi_ai::credential::Credential;
    use opi_ai::{AuthDescriptor, AuthScheme};
    use opi_coding_agent::config::CustomProviderConfig;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;
    use secrecy::SecretString;

    fn secret(value: &str) -> SecretString {
        SecretString::new(value.to_owned().into_boxed_str())
    }

    let env = ScopedEnv::new(&["ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN", "ACME_API_KEY"]);
    env.remove("ANTHROPIC_API_KEY");
    env.set(
        "ANTHROPIC_OAUTH_TOKEN",
        "oauth-listing-precedence-canary-DO-NOT-LEAK",
    );
    env.set(
        "ACME_API_KEY",
        "custom-listing-wrong-kind-canary-DO-NOT-LEAK",
    );

    let dir = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    );
    store
        .write("anthropic", &Credential::ApiKey(secret("stored-api-key")))
        .await
        .unwrap();
    store
        .write(
            "acme",
            &Credential::OAuthToken {
                access: secret("custom-oauth-access"),
                refresh: secret("custom-oauth-refresh"),
                expires_at: None,
                base_url: None,
                account_id: None,
            },
        )
        .await
        .unwrap();

    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.providers.custom.insert(
        "acme".into(),
        CustomProviderConfig {
            id: "acme".into(),
            name: "Acme".into(),
            base_url: Some("https://api.acme.example".into()),
            api_key_env: "ACME_API_KEY".into(),
            auth_scheme: AuthScheme::Bearer,
            proxy: None,
            headers: Vec::new(),
            models: vec![model("model", "Model")],
        },
    );
    let outcome = build_collection_for_listing_with_store(&config, &store).await;

    let collection = outcome.expect("kind-aware listing");
    assert!(
        matches!(
            collection.auth_descriptor("anthropic"),
            Some(AuthDescriptor::Resolved { source })
                if source == "env ANTHROPIC_OAUTH_TOKEN"
        ),
        "listing metadata must name the source selected by live Anthropic precedence: {:?}",
        collection.auth_descriptor("anthropic")
    );
    assert!(
        collection.registry().get_provider("acme").is_none(),
        "stored OAuth must reject an API-key-only custom provider without falling back to env"
    );
    let rendered = format!(
        "{:?}{:?}",
        collection.auth_descriptor("anthropic"),
        collection.auth_descriptor("acme")
    );
    for canary in [
        "oauth-listing-precedence-canary-DO-NOT-LEAK",
        "custom-listing-wrong-kind-canary-DO-NOT-LEAK",
        "custom-oauth-access",
        "custom-oauth-refresh",
    ] {
        assert!(!rendered.contains(canary), "secret leaked: {rendered}");
    }
}

#[derive(Default)]
struct ProbeOnlyCredentialStore {
    credentials: std::sync::Mutex<std::collections::HashMap<String, Credential>>,
    read_calls: AtomicUsize,
    probed_provider_ids: std::sync::Mutex<Vec<String>>,
}

impl CredentialStore for ProbeOnlyCredentialStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(self.credentials.lock().unwrap().get(provider_id).cloned()) })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        credential: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.credentials
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), credential.clone());
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.credentials.lock().unwrap().remove(provider_id);
            Ok(())
        })
    }

    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            self.probed_provider_ids
                .lock()
                .unwrap()
                .push(provider_id.to_owned());
            if self.credentials.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("fake store {provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }
}

impl opi_coding_agent::credential_store::CredentialMetadataStore for ProbeOnlyCredentialStore {
    fn probe_metadata<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, opi_coding_agent::credential_store::CredentialMetadataProbe> {
        Box::pin(async move {
            use opi_coding_agent::credential_store::{
                CredentialMetadataProbe, StoredCredentialKind,
            };

            self.probed_provider_ids
                .lock()
                .unwrap()
                .push(provider_id.to_owned());
            let credentials = self.credentials.lock().unwrap();
            let kind = match credentials.get(provider_id) {
                Some(Credential::ApiKey(_)) => Some(StoredCredentialKind::ApiKey),
                Some(Credential::OAuthToken { .. }) => Some(StoredCredentialKind::OAuthToken),
                None => None,
            };
            CredentialMetadataProbe {
                source: if kind.is_some() {
                    CredentialSource::Present {
                        label: format!("fake store {provider_id}"),
                    }
                } else {
                    CredentialSource::Absent
                },
                kind,
                failure: None,
            }
        })
    }
}

#[tokio::test]
async fn github_copilot_static_catalog_lists_without_store_reads() {
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;

    let store = ProbeOnlyCredentialStore::default();
    let collection = build_collection_for_listing_with_store(
        &opi_coding_agent::config::OpiConfig::default(),
        &store,
    )
    .await
    .expect("static GitHub Copilot listing");

    let entries = model_entries_from_registry(collection.registry())
        .into_iter()
        .filter(|entry| entry.provider_id == "github-copilot")
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 25);
    assert_eq!(store.read_calls.load(Ordering::SeqCst), 0);
    assert!(
        !store
            .probed_provider_ids
            .lock()
            .unwrap()
            .iter()
            .any(|provider_id| provider_id == "github-copilot"),
        "static catalog listing must not probe GitHub Copilot credentials"
    );
}

#[tokio::test]
async fn openai_codex_static_catalog_lists_without_store_reads() {
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;

    let store = ProbeOnlyCredentialStore::default();
    let collection = build_collection_for_listing_with_store(
        &opi_coding_agent::config::OpiConfig::default(),
        &store,
    )
    .await
    .expect("static OpenAI Codex listing");

    let entries = model_entries_from_registry(collection.registry())
        .into_iter()
        .filter(|entry| entry.provider_id == "openai-codex")
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 7);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.model_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "gpt-5.3-codex-spark",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.5",
            "gpt-5.6-luna",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
        ]
    );
    assert_eq!(store.read_calls.load(Ordering::SeqCst), 0);
    assert!(
        !store
            .probed_provider_ids
            .lock()
            .unwrap()
            .iter()
            .any(|provider_id| provider_id == "openai-codex"),
        "static catalog listing must not probe OpenAI Codex credentials"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited store orchestration never re-acquires this lock.
async fn stored_only_credential_lists_models_through_async_orchestration() {
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;
    use secrecy::SecretString;

    let env = ScopedEnv::new(&["ANTHROPIC_API_KEY"]);
    env.remove("ANTHROPIC_API_KEY");

    let canary = "sk-stored-only-listing-DO-NOT-LEAK";
    let store = ProbeOnlyCredentialStore::default();
    store
        .write(
            "anthropic",
            &Credential::ApiKey(SecretString::new(canary.to_owned().into_boxed_str())),
        )
        .await
        .expect("seed fake store");

    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.defaults.model = "anthropic:claude-stored-only".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
    let outcome = build_collection_for_listing_with_store(&config, &store).await;

    let collection = outcome.expect("stored-only metadata collection");
    assert_eq!(store.read_calls.load(Ordering::SeqCst), 0);
    let entries = model_entries_from_registry(collection.registry());
    assert!(entries.iter().any(|entry| entry.provider_id == "anthropic"));
    let rendered = format!("{:?}", collection.probe("anthropic"))
        + &entries
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}:{}",
                    entry.provider_id, entry.model_id, entry.display_name
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
    assert!(
        !rendered.contains(canary),
        "listing leaked secret: {rendered}"
    );

    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    use futures_util::StreamExt;
    let error = collection
        .registry()
        .get_provider("anthropic")
        .expect("metadata provider registered")
        .stream_prepared(request, opi_ai::test_support::resolved_auth())
        .next()
        .await
        .expect("metadata provider returns one error")
        .expect_err("metadata provider cannot dispatch");
    assert!(
        matches!(
            error,
            ProviderError::StreamError(ref summary) if summary.as_str() == "[REDACTED]"
        ),
        "unexpected redacted metadata-provider error: {error}"
    );
}

#[tokio::test]
async fn native_keyring_precedes_listing_orchestration() {
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KeyringBackendFactory,
    };
    use opi_coding_agent::provider_factory::build_collection_for_listing_command;

    let events = Arc::new(Mutex::new(Vec::new()));
    let factory_events = Arc::clone(&events);
    let backend_factory: KeyringBackendFactory = Box::new(move || {
        let backend = FakeKeyringBackend::new();
        factory_events
            .lock()
            .expect("ordering events")
            .push("native_install");
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        Box::new(OrderingKeyringBackend {
            inner: backend,
            events: Arc::clone(&factory_events),
        })
    });
    let dir = tempfile::tempdir().expect("temp dir");
    assert!(
        events.lock().expect("ordering events").is_empty(),
        "backend construction must remain lazy until the command core"
    );
    let collection = build_collection_for_listing_command(
        &opi_coding_agent::config::OpiConfig::default(),
        dir.path().to_path_buf(),
        backend_factory,
    )
    .await
    .expect("listing orchestration uses mock entries");

    assert!(collection.registry().get_provider("anthropic").is_some());
    drop(collection);
    assert_native_entry_drop_order(&events);
}

#[tokio::test]
async fn listing_presence_probe_never_reads_secret() {
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::KeychainCredentialStore;
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;

    let secret_get_calls = Arc::new(AtomicUsize::new(0));
    let presence_calls = Arc::new(AtomicUsize::new(0));
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KeychainCredentialStore::new(
        Box::new(ListingPresenceBackend {
            secret_get_calls: Arc::clone(&secret_get_calls),
            presence_calls: Arc::clone(&presence_calls),
        }),
        dir.path().to_path_buf(),
    );
    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.defaults.model = "anthropic:listing-presence".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    let collection = build_collection_for_listing_with_store(&config, &store)
        .await
        .expect("metadata listing");
    assert!(collection.registry().get_provider("anthropic").is_some());
    assert_eq!(secret_get_calls.load(Ordering::SeqCst), 0);
    assert!(presence_calls.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited listing core never re-acquires this lock.
async fn listing_fails_closed_on_operational_and_corrupt_store_probes_with_env_present() {
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KeychainCredentialStore, KeyringBackend,
    };
    use opi_coding_agent::provider_factory::build_collection_for_listing_with_store;

    let env = ScopedEnv::new(&["ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN"]);
    let env_canary = "listing-fail-closed-env-canary-DO-NOT-LEAK";
    env.set("ANTHROPIC_API_KEY", env_canary);
    env.remove("ANTHROPIC_OAUTH_TOKEN");

    let corrupt = FakeKeyringBackend::new();
    corrupt.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "corrupt-marker");
    let cases: [(&str, Box<dyn KeyringBackend>); 2] = [
        ("operational", Box::new(ListingOperationalProbeBackend)),
        ("corrupt", Box::new(corrupt)),
    ];
    let mut outcomes = Vec::new();
    for (name, backend) in cases {
        let dir = tempfile::tempdir().unwrap();
        let store = KeychainCredentialStore::new(backend, dir.path().to_path_buf());
        outcomes.push((
            name,
            build_collection_for_listing_with_store(
                &opi_coding_agent::config::OpiConfig::default(),
                &store,
            )
            .await,
        ));
    }

    for (name, outcome) in outcomes {
        let collection = outcome.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            collection.registry().get_provider("anthropic").is_none(),
            "{name}: fail-closed store probe must not use API-key env fallback"
        );
        let rendered = format!(
            "{:?}{:?}",
            collection.auth_descriptor("anthropic"),
            collection.auth_status("anthropic")
        );
        assert!(!rendered.contains(env_canary), "{name}: {rendered}");
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited listing core never re-acquires this lock.
async fn bedrock_listing_matches_secret_free_runtime_auth_presence() {
    use opi_ai::{AuthDescriptor, AuthStatus};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::build_collection_for_listing_command;

    struct Case {
        name: &'static str,
        access_key_id: Option<&'static str>,
        secret_access_key_env: Option<&'static str>,
        profile: Option<&'static str>,
        env: &'static [(&'static str, &'static str)],
        expected_present: bool,
    }

    const CONFIG_ACCESS: &str = "opi-bedrock-access-config-canary";
    const CONFIG_SECRET: &str = "opi-bedrock-secret-config-canary";
    const ENV_ACCESS: &str = "opi-bedrock-access-env-canary";
    const ENV_SECRET: &str = "opi-bedrock-secret-env-canary";
    const SESSION_TOKEN: &str = "opi-bedrock-session-canary";
    const CONFIG_PROFILE: &str = "opi-bedrock-profile-config-canary";
    const ENV_PROFILE: &str = "opi-bedrock-profile-env-canary";
    const CANARIES: [&str; 7] = [
        CONFIG_ACCESS,
        CONFIG_SECRET,
        ENV_ACCESS,
        ENV_SECRET,
        SESSION_TOKEN,
        CONFIG_PROFILE,
        ENV_PROFILE,
    ];
    const CUSTOM_SECRET: &[(&str, &str)] = &[("OPI_TEST_BEDROCK_SECRET", CONFIG_SECRET)];
    const DEFAULT_PAIR: &[(&str, &str)] = &[
        ("AWS_ACCESS_KEY_ID", ENV_ACCESS),
        ("AWS_SECRET_ACCESS_KEY", ENV_SECRET),
    ];
    const DEFAULT_PAIR_WITH_SESSION: &[(&str, &str)] = &[
        ("AWS_ACCESS_KEY_ID", ENV_ACCESS),
        ("AWS_SECRET_ACCESS_KEY", ENV_SECRET),
        ("AWS_SESSION_TOKEN", SESSION_TOKEN),
    ];
    let cases = [
        Case {
            name: "configured access and explicitly configured secret env",
            access_key_id: Some(CONFIG_ACCESS),
            secret_access_key_env: Some("OPI_TEST_BEDROCK_SECRET"),
            profile: None,
            env: CUSTOM_SECRET,
            expected_present: true,
        },
        Case {
            name: "fixed default env pair",
            access_key_id: None,
            secret_access_key_env: None,
            profile: None,
            env: DEFAULT_PAIR,
            expected_present: true,
        },
        Case {
            name: "configured profile",
            access_key_id: None,
            secret_access_key_env: None,
            profile: Some(CONFIG_PROFILE),
            env: &[],
            expected_present: true,
        },
        Case {
            name: "AWS_PROFILE",
            access_key_id: None,
            secret_access_key_env: None,
            profile: None,
            env: &[("AWS_PROFILE", ENV_PROFILE)],
            expected_present: true,
        },
        Case {
            name: "optional AWS_SESSION_TOKEN",
            access_key_id: None,
            secret_access_key_env: None,
            profile: None,
            env: DEFAULT_PAIR_WITH_SESSION,
            expected_present: true,
        },
        Case {
            name: "access only",
            access_key_id: None,
            secret_access_key_env: None,
            profile: None,
            env: &[("AWS_ACCESS_KEY_ID", ENV_ACCESS)],
            expected_present: false,
        },
        Case {
            name: "whitespace credentials",
            access_key_id: Some(" \t "),
            secret_access_key_env: Some("OPI_TEST_BEDROCK_SECRET"),
            profile: Some(" \n "),
            env: &[
                ("OPI_TEST_BEDROCK_SECRET", " \t "),
                ("AWS_ACCESS_KEY_ID", " "),
                ("AWS_SECRET_ACCESS_KEY", "\t"),
                ("AWS_PROFILE", "\n"),
            ],
            expected_present: false,
        },
        Case {
            name: "whitespace configured profile falls through to AWS_PROFILE",
            access_key_id: None,
            secret_access_key_env: None,
            profile: Some(" \t "),
            env: &[("AWS_PROFILE", ENV_PROFILE)],
            expected_present: true,
        },
        Case {
            name: "missing configured secret fails closed before fixed default pair",
            access_key_id: Some(CONFIG_ACCESS),
            secret_access_key_env: Some("OPI_TEST_BEDROCK_SECRET"),
            profile: None,
            env: DEFAULT_PAIR,
            expected_present: false,
        },
        Case {
            name: "configured access does not combine with unconfigured default secret",
            access_key_id: Some(CONFIG_ACCESS),
            secret_access_key_env: None,
            profile: None,
            env: &[("AWS_SECRET_ACCESS_KEY", ENV_SECRET)],
            expected_present: false,
        },
    ];

    for case in cases {
        let _env = ScopedEnv::cleared_with_values(&BEDROCK_ENV_NAMES, case.env);
        let mut config = opi_coding_agent::config::OpiConfig::default();
        config.providers.bedrock.access_key_id =
            case.access_key_id.map(secrecy::SecretString::from);
        config.providers.bedrock.secret_access_key_env =
            case.secret_access_key_env.map(str::to_owned);
        config.providers.bedrock.profile = case.profile.map(str::to_owned);
        let dir = tempfile::tempdir().expect("temp user config dir");

        let collection = build_collection_for_listing_command(
            &config,
            dir.path().to_path_buf(),
            Box::new(|| Box::new(FakeKeyringBackend::new().with_unavailable())),
        )
        .await
        .unwrap_or_else(|error| panic!("{}: listing core failed: {error}", case.name));
        let entries = model_entries_from_registry(collection.registry());
        let bedrock_present = entries.iter().any(|entry| entry.provider_id == "bedrock");
        assert_eq!(
            bedrock_present, case.expected_present,
            "{}: unexpected Bedrock listing presence",
            case.name
        );
        if case.expected_present {
            assert!(
                matches!(
                    collection.auth_descriptor("bedrock"),
                    Some(AuthDescriptor::Resolved { source }) if source == "aws credential chain"
                ),
                "{}: unexpected safe Bedrock auth source: {:?}",
                case.name,
                collection.auth_descriptor("bedrock")
            );
            assert_eq!(
                collection.auth_status("bedrock"),
                Some(AuthStatus::Configured),
                "{}: unexpected Bedrock auth status",
                case.name
            );
        } else {
            assert!(
                collection.auth_descriptor("bedrock").is_none(),
                "{}: unavailable Bedrock provider retained auth metadata",
                case.name
            );
            assert!(
                collection.auth_status("bedrock").is_none(),
                "{}: unavailable Bedrock provider retained auth status",
                case.name
            );
        }
        assert!(
            collection.probe("bedrock").is_none(),
            "{}: Bedrock must not expose store probe metadata",
            case.name
        );

        let rendered = format!(
            "providers={:?}\nauth={:?}\nstatus={:?}\nentries={}",
            collection.provider_ids(),
            collection.auth_descriptor("bedrock"),
            collection.auth_status("bedrock"),
            entries
                .iter()
                .map(|entry| format!(
                    "{}:{}:{}",
                    entry.provider_id, entry.model_id, entry.display_name
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
        for canary in CANARIES {
            assert!(
                !rendered.contains(canary),
                "{}: listing metadata leaked canary {canary}",
                case.name
            );
        }
    }
}

#[test]
fn custom_mapped_provider_lists_one_identity() {
    use std::collections::HashMap;

    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let env_name = "OPI_TEST_CUSTOM_LIST_ONE_IDENTITY_1416";
    let env = ScopedEnv::new(&[env_name]);
    env.set(env_name, "test-key");
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(
        &path,
        format!(
            r#"
[providers.custom.acme]
base_url = "https://api.acme.example"
api_key_env = "{env_name}"
auth_scheme = "bearer"

[[providers.custom.acme.models]]
id = "claude"
api = "anthropic-messages"
context_window = 200000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "chat"
api = "openai-completions"
context_window = 128000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "responses"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    let collection = build_collection_for_listing(&config, &HashMap::new()).unwrap();
    let entries = model_entries_from_registry(collection.registry());

    let custom: Vec<_> = entries
        .iter()
        .filter(|entry| entry.provider_id == "acme")
        .collect();
    assert_eq!(custom.len(), 3);
    assert_eq!(
        custom
            .iter()
            .map(|entry| entry.model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["claude", "chat", "responses"]
    );
    assert!(
        entries.iter().all(|entry| {
            !entry.provider_id.contains("route")
                && !entry.provider_id.contains("anthropic-messages")
                && !entry.provider_id.contains("openai-completions")
                && !entry.provider_id.contains("openai-responses")
        }),
        "hidden route ids leaked into listing"
    );
}

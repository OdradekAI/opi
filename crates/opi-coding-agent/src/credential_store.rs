//! Concrete credential store, resolver, and cross-process lock (Phase 14.1).
//!
//! This module owns all IO and env access for persisted credentials. The
//! abstract [`opi_ai::CredentialStore`] trait lives in `opi-ai`; here we
//! provide the concrete [`KeychainCredentialStore`] (an OS-keychain backend
//! behind an injectable [`KeyringBackend`] seam), the [`CredentialResolver`]
//! (keychain-first with env fallback for API keys), and a single `fs4` advisory
//! lock that coordinates cross-process mutation.
//!
//! # Locking
//!
//! A single exclusive `fs4` lock at `<user_config_dir>/credential.lock` wraps
//! every store mutation (`write`, `delete`, and T2 OAuth refresh). The lock
//! file holds no secret; it is pure coordination, because the OS keychain has
//! no read-refresh-write transaction. Acquire-then-re-read is honored by T2
//! refresh holding one external lock across read/HTTP/write via the
//! package-private unlocked backend operations, so the public locked `write`
//! is never re-entered.
//!
//! # Persistence protocol
//!
//! A credential write updates the non-secret kind marker first and the
//! protected envelope second. These are two keychain entries, not an atomic
//! transaction. A reader observing a kind-change transition receives a typed
//! wrong-kind/corrupt-store error and never falls back to the environment. If
//! the protected write fails, the marker-only state remains fail-closed; a
//! later successful write retries both steps and recovers it.
//!
//! # Secret exposure
//!
//! Encoding necessarily exposes `SecretString` values at this protected
//! keychain-serialization boundary. The JSON string and intermediate envelope
//! fields are zeroized after the backend call. The other intentional exposure
//! boundary is concrete provider HTTP request construction.
//!
//! Tests inject a [`FakeKeyringBackend`] (or any `KeyringBackend`) and never
//! touch the user keychain.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use opi_ai::auth::{AuthResolver, AuthScheme, OAuthCredential, OAuthProvider, ResolvedAuth};
use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
    UnknownEnvelopeField,
};
use opi_ai::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use zeroize::{Zeroize, Zeroizing};

/// Non-secret stored credential kind discovered from the presence marker.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredCredentialKind {
    ApiKey,
    OAuthToken,
}

/// Non-secret classification for a failed metadata probe.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialProbeFailure {
    /// The credential backend is genuinely unavailable, so runtime API-key
    /// resolution permits environment fallback.
    BackendUnavailable,
    /// The backend returned an operational error; runtime fails closed.
    Operational,
    /// The non-secret credential-kind marker is corrupt; runtime fails closed.
    CorruptMarker,
}

/// Redacted credential-store probe plus the optional stored credential kind.
///
/// `kind` is populated only by stores that can inspect non-secret metadata.
/// It never requires reading the protected credential entry.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialMetadataProbe {
    pub source: CredentialSource,
    pub kind: Option<StoredCredentialKind>,
    pub failure: Option<CredentialProbeFailure>,
}

impl From<CredentialSource> for CredentialMetadataProbe {
    fn from(source: CredentialSource) -> Self {
        let failure = matches!(source, CredentialSource::BackendUnavailable { .. })
            .then_some(CredentialProbeFailure::BackendUnavailable);
        Self {
            source,
            kind: None,
            failure,
        }
    }
}

/// Credential store capable of a secret-free presence-and-kind probe.
#[doc(hidden)]
pub trait CredentialMetadataStore: CredentialStore {
    fn probe_metadata<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, CredentialMetadataProbe>;
}

/// Collect redacted credential metadata without reading protected credentials.
pub async fn collect_credential_probes(
    store: &dyn CredentialMetadataStore,
    provider_ids: impl IntoIterator<Item = String>,
) -> HashMap<String, CredentialMetadataProbe> {
    let mut probes = HashMap::new();
    for provider_id in provider_ids {
        probes.insert(
            provider_id.clone(),
            store.probe_metadata(&provider_id).await,
        );
    }
    probes
}

/// Keychain service name. Every opi entry is stored under service `opi` with
/// the provider id as the account key.
pub const KEYCHAIN_SERVICE: &str = "opi";

/// Collision-free service containing only closed, non-secret credential-kind
/// markers. Protected credential envelopes remain under [`KEYCHAIN_SERVICE`].
pub const KEYCHAIN_PRESENCE_SERVICE: &str = "opi.presence";

/// Versioned envelope marker. Unknown versions decode to an explicit
/// [`CredentialStoreError::UnknownEnvelope`].
const ENVELOPE_VERSION: u32 = 1;

/// Maximum time a refresh HTTP future may hold the mutation lock.
const OAUTH_REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Backend injection seam
// ---------------------------------------------------------------------------

/// Errors from a [`KeyringBackend`] operation.
#[derive(Clone, Debug, thiserror::Error)]
pub enum BackendError {
    /// The configured credential backend is explicitly unreachable. On Linux,
    /// this is limited to narrow Secret Service daemon-absence signatures.
    /// API keys may fall back to env.
    #[error("credential backend unavailable: {0}")]
    BackendUnavailable(String),
    /// Any other backend failure.
    #[error("credential backend error: {0}")]
    Other(String),
}

/// Injectable OS-keychain backend. Production uses [`KeyringCoreBackend`]
/// (wrapping `keyring-core`); tests inject [`FakeKeyringBackend`]. The seam
/// keeps tests off the user keychain and off `keyring-core`'s process-global
/// default store (which would race under parallel tests).
pub trait KeyringBackend: Send + Sync {
    /// Read the raw stored payload for `(service, provider_id)`, or `Ok(None)`
    /// when no entry exists. `Err(BackendUnavailable)` means the backend could
    /// not be reached.
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError>;
    /// Persist `value` under `(service, provider_id)`, replacing any entry.
    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError>;
    /// Delete the entry for `(service, provider_id)`, if present.
    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError>;
}

/// One-shot backend factory used by production command/startup orchestration.
///
/// The factory is invoked inside the command core, so native installation
/// cannot be performed early by a caller and then bypassed with an already
/// constructed backend.
#[doc(hidden)]
pub type KeyringBackendFactory = Box<dyn FnOnce() -> Box<dyn KeyringBackend> + Send + 'static>;

/// Factory for the target-native production backend.
#[doc(hidden)]
pub fn native_keyring_backend_factory() -> KeyringBackendFactory {
    Box::new(|| Box::new(KeyringCoreBackend::new()))
}

/// Production backend wrapping `keyring-core` and owning the installed native
/// store for its full lifetime.
pub struct KeyringCoreBackend {
    _native: Option<crate::native_keyring::NativeKeyringGuard>,
    initialization_error: Option<BackendError>,
}

impl KeyringCoreBackend {
    pub fn new() -> Self {
        match crate::native_keyring::install_native_keyring() {
            Ok(guard) => Self {
                _native: Some(guard),
                initialization_error: None,
            },
            Err(error) => Self {
                _native: None,
                initialization_error: Some(error),
            },
        }
    }

    fn ensure_available(&self) -> Result<(), BackendError> {
        match &self.initialization_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

impl Default for KeyringCoreBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringBackend for KeyringCoreBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        self.ensure_available()?;
        let entry = keyring_core::Entry::new(service, provider_id).map_err(map_keyring_error)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        self.ensure_available()?;
        let entry = keyring_core::Entry::new(service, provider_id).map_err(map_keyring_error)?;
        entry.set_password(value).map_err(map_keyring_error)
    }

    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        self.ensure_available()?;
        let entry = keyring_core::Entry::new(service, provider_id).map_err(map_keyring_error)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

fn map_keyring_error(error: keyring_core::Error) -> BackendError {
    map_keyring_error_for_platform(std::env::consts::OS, error)
}

fn map_keyring_error_for_platform(target_os: &str, error: keyring_core::Error) -> BackendError {
    match error {
        error @ keyring_core::Error::NoDefaultStore => BackendError::Other(error.to_string()),
        keyring_core::Error::NoStorageAccess(reason)
            if target_os == "linux" && secret_service_is_unavailable(&reason.to_string()) =>
        {
            BackendError::BackendUnavailable(reason.to_string())
        }
        keyring_core::Error::NoStorageAccess(reason) => BackendError::Other(reason.to_string()),
        keyring_core::Error::PlatformFailure(reason)
            if target_os == "linux" && secret_service_is_unavailable(&reason.to_string()) =>
        {
            BackendError::BackendUnavailable(reason.to_string())
        }
        other => BackendError::Other(other.to_string()),
    }
}

pub(crate) fn secret_service_is_unavailable(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("serviceunknown")
        || reason.contains("namehasnoowner")
        || reason.contains("connection refused")
}

/// In-memory keychain backend for tests. Never touches the OS keychain.
/// In-memory keychain backend for tests. Never touches the OS keychain.
///
/// `Clone` shares the underlying entry map (via `Arc`), so two
/// [`KeychainCredentialStore`] instances built over cloned backends share
/// mutable state — used to prove the cross-process mutation lock serializes
/// writers against genuinely shared state.
#[derive(Clone)]
pub struct FakeKeyringBackend {
    entries: Arc<Mutex<HashMap<(String, String), String>>>,
    /// Recorded `(start, end)` critical-section windows of each `set` call, so
    /// a test can assert two writers did not overlap (i.e. the lock serialized
    /// them). Shared across clones.
    set_windows: Arc<Mutex<Vec<(Instant, Instant)>>>,
    /// When true, every operation reports `BackendUnavailable` (simulates a
    /// headless host with no keychain daemon).
    unavailable: bool,
    /// When non-zero, `set` blocks for this long (simulates a slow keychain so
    /// the mutation lock is held long enough to test contention).
    set_delay: Duration,
}

impl FakeKeyringBackend {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            set_windows: Arc::new(Mutex::new(Vec::new())),
            unavailable: false,
            set_delay: Duration::ZERO,
        }
    }

    /// Configure the backend to behave as if no keychain daemon is present.
    pub fn with_unavailable(mut self) -> Self {
        self.unavailable = true;
        self
    }

    /// Hold each `set` call for `delay` so concurrent writers contend on the
    /// mutation lock.
    pub fn with_set_delay(mut self, delay: Duration) -> Self {
        self.set_delay = delay;
        self
    }

    /// Seed a raw (already-encoded or deliberately malformed) payload for
    /// `(service, provider_id)`. Used to inject corrupt/unknown envelopes so
    /// error-handling paths can be exercised without going through the encoder.
    pub fn seed_raw(&self, service: &str, provider_id: &str, raw: &str) {
        self.entries
            .lock()
            .unwrap()
            .insert((service.to_owned(), provider_id.to_owned()), raw.to_owned());
    }

    /// Snapshot of the recorded `set` critical-section windows (start, end),
    /// in call order. Used to assert writers were serialized (no overlap).
    pub fn set_windows(&self) -> Vec<(Instant, Instant)> {
        self.set_windows.lock().unwrap().clone()
    }

    /// Read the raw stored payload for `(service, provider_id)`, for tests that
    /// inspect the persisted envelope without going through the codec.
    pub fn raw_entry(&self, service: &str, provider_id: &str) -> Option<String> {
        self.entries
            .lock()
            .unwrap()
            .get(&(service.to_owned(), provider_id.to_owned()))
            .cloned()
    }

    /// Seed an already-decoded credential for `(service, provider_id)` by
    /// encoding it through the versioned envelope. Lets a test inject a
    /// concurrent writer's fresh credential into shared backend state without
    /// going through the (locked) store write.
    pub fn seed_credential(&self, service: &str, provider_id: &str, cred: &Credential) {
        self.seed_raw(service, provider_id, &encode_credential(cred));
    }
}

impl Default for FakeKeyringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringBackend for FakeKeyringBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        if self.unavailable {
            return Err(BackendError::BackendUnavailable(
                "no keychain daemon".to_owned(),
            ));
        }
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(&(service.to_owned(), provider_id.to_owned()))
            .cloned())
    }
    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        if self.unavailable {
            return Err(BackendError::BackendUnavailable(
                "no keychain daemon".to_owned(),
            ));
        }
        // Mutation-window tests measure only the protected entry. Marker
        // writes are non-secret metadata and intentionally remain immediate.
        let track_window = service == KEYCHAIN_SERVICE;
        let start = track_window.then(Instant::now);
        if track_window && !self.set_delay.is_zero() {
            std::thread::sleep(self.set_delay);
        }
        self.entries.lock().unwrap().insert(
            (service.to_owned(), provider_id.to_owned()),
            value.to_owned(),
        );
        if let Some(start) = start {
            self.set_windows
                .lock()
                .unwrap()
                .push((start, Instant::now()));
        }
        Ok(())
    }
    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        if self.unavailable {
            return Err(BackendError::BackendUnavailable(
                "no keychain daemon".to_owned(),
            ));
        }
        self.entries
            .lock()
            .unwrap()
            .remove(&(service.to_owned(), provider_id.to_owned()));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Versioned envelope codec
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct Envelope {
    version: u32,
    kind: String,
    #[serde(flatten)]
    fields: EnvelopeFields,
}

#[derive(serde::Serialize, Default)]
struct EnvelopeFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh: Option<String>,
    /// Unix seconds. Non-secret.
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<String>,
}

impl Zeroize for EnvelopeFields {
    fn zeroize(&mut self) {
        self.api_key.zeroize();
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

impl Drop for EnvelopeFields {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(serde::Deserialize)]
struct EnvelopeHeader {
    version: u32,
    kind: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiKeyEnvelopeV1 {
    version: u32,
    kind: String,
    api_key: String,
}

impl Zeroize for ApiKeyEnvelopeV1 {
    fn zeroize(&mut self) {
        self.api_key.zeroize();
    }
}

impl Drop for ApiKeyEnvelopeV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OAuthEnvelopeV1 {
    version: u32,
    kind: String,
    access: String,
    refresh: String,
    expires_at: Option<i64>,
    base_url: Option<String>,
    account_id: Option<String>,
}

impl Zeroize for OAuthEnvelopeV1 {
    fn zeroize(&mut self) {
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

impl Drop for OAuthEnvelopeV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CredentialKind {
    ApiKey,
    OAuthToken,
}

impl From<CredentialKind> for StoredCredentialKind {
    fn from(kind: CredentialKind) -> Self {
        match kind {
            CredentialKind::ApiKey => Self::ApiKey,
            CredentialKind::OAuthToken => Self::OAuthToken,
        }
    }
}

impl CredentialKind {
    fn for_credential(credential: &Credential) -> Self {
        match credential {
            Credential::ApiKey(_) => Self::ApiKey,
            Credential::OAuthToken { .. } => Self::OAuthToken,
        }
    }

    fn marker(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::OAuthToken => "oauth_token",
        }
    }

    fn parse_marker(marker: &str, provider_id: &str) -> Result<Self, CredentialStoreError> {
        match marker {
            "api_key" => Ok(Self::ApiKey),
            "oauth_token" => Ok(Self::OAuthToken),
            _ => Err(CredentialStoreError::CorruptMarker {
                provider: provider_id.to_owned(),
            }),
        }
    }
}

fn secret_string(value: impl Into<String>) -> SecretString {
    // secrecy 0.10 SecretString::new takes `Box<str>`.
    SecretString::new(value.into().into_boxed_str())
}

/// Encode a [`Credential`] as the v1 JSON envelope.
///
/// The serialized envelope contains live access/refresh tokens, so it is
/// returned in a [`Zeroizing`] buffer that wipes its bytes on drop
/// (defense-in-depth: the `secrecy::SecretString` fields on [`Credential`]
/// already zeroize, but this derived JSON `String` otherwise would not).
fn encode_credential(cred: &Credential) -> Zeroizing<String> {
    let (kind, fields) = match cred {
        Credential::ApiKey(key) => (
            "api_key",
            EnvelopeFields {
                api_key: Some(key.expose_secret().to_owned()),
                access: None,
                refresh: None,
                expires_at: None,
                base_url: None,
                account_id: None,
            },
        ),
        Credential::OAuthToken {
            access,
            refresh,
            expires_at,
            base_url,
            account_id,
        } => (
            "oauth",
            EnvelopeFields {
                api_key: None,
                access: Some(access.expose_secret().to_owned()),
                refresh: Some(refresh.expose_secret().to_owned()),
                expires_at: expires_at.map(|t| t.unix_timestamp()),
                base_url: base_url.clone(),
                account_id: account_id.clone(),
            },
        ),
    };
    let envelope = Envelope {
        version: ENVELOPE_VERSION,
        kind: kind.to_owned(),
        fields,
    };
    Zeroizing::new(serde_json::to_string(&envelope).expect("credential envelope serializes"))
}

/// Decode a v1 JSON envelope into a [`Credential`].
///
/// Malformed JSON, an unknown version, or an unknown kind each yield a distinct
/// [`CredentialStoreError`] and are never collapsed into absence or env
/// fallback.
fn decode_credential(raw: &str, provider_id: &str) -> Result<Credential, CredentialStoreError> {
    let header: EnvelopeHeader =
        serde_json::from_str(raw).map_err(|_| CredentialStoreError::MalformedEnvelope {
            provider: provider_id.to_owned(),
            reason: "credential envelope does not match the expected schema".to_owned(),
        })?;
    if header.version != ENVELOPE_VERSION {
        return Err(CredentialStoreError::UnknownEnvelope {
            provider: provider_id.to_owned(),
            version: Some(header.version),
            field: UnknownEnvelopeField::Version,
        });
    }
    match header.kind.as_str() {
        "api_key" => {
            let mut envelope: ApiKeyEnvelopeV1 =
                serde_json::from_str(raw).map_err(|_| CredentialStoreError::MalformedEnvelope {
                    provider: provider_id.to_owned(),
                    reason: "api_key credential envelope does not match the expected schema"
                        .to_owned(),
                })?;
            debug_assert_eq!(envelope.version, ENVELOPE_VERSION);
            debug_assert_eq!(envelope.kind, "api_key");
            let api_key = std::mem::take(&mut envelope.api_key);
            Ok(Credential::ApiKey(secret_string(api_key)))
        }
        "oauth" => {
            let mut envelope: OAuthEnvelopeV1 =
                serde_json::from_str(raw).map_err(|_| CredentialStoreError::MalformedEnvelope {
                    provider: provider_id.to_owned(),
                    reason: "oauth credential envelope does not match the expected schema"
                        .to_owned(),
                })?;
            debug_assert_eq!(envelope.version, ENVELOPE_VERSION);
            debug_assert_eq!(envelope.kind, "oauth");
            let expires_at = match envelope.expires_at {
                Some(secs) => Some(time::OffsetDateTime::from_unix_timestamp(secs).map_err(
                    |error| CredentialStoreError::MalformedEnvelope {
                        provider: provider_id.to_owned(),
                        reason: format!("invalid expires_at: {error}"),
                    },
                )?),
                None => None,
            };
            let access = std::mem::take(&mut envelope.access);
            let refresh = std::mem::take(&mut envelope.refresh);
            let base_url = envelope.base_url.take();
            let account_id = envelope.account_id.take();
            Ok(Credential::OAuthToken {
                access: secret_string(access),
                refresh: secret_string(refresh),
                expires_at,
                base_url,
                account_id,
            })
        }
        _ => Err(CredentialStoreError::UnknownEnvelope {
            provider: provider_id.to_owned(),
            version: Some(header.version),
            field: UnknownEnvelopeField::Kind,
        }),
    }
}

// ---------------------------------------------------------------------------
// Cross-process mutation lock
// ---------------------------------------------------------------------------

/// `fs4` advisory lock coordinator. The lock file holds no secret.
pub(crate) struct LockCoordinator {
    lock_path: PathBuf,
    timeout: Duration,
    poll_interval: Duration,
}

/// RAII guard: dropping releases the lock (the OS releases when the file
/// handle closes; `unlock` is best-effort and immediate).
pub(crate) struct LockGuard {
    _file: tokio::fs::File,
}

impl LockCoordinator {
    pub(crate) fn with_timeout(user_config_dir: PathBuf, timeout: Duration) -> Self {
        Self {
            // user_config_dir already ends in `opi`; the spec's literal
            // "<user_config_dir>/opi/credential.lock" would double the `opi`
            // segment. The single-segment path matches the redaction-test
            // language ("opi/credential.lock").
            lock_path: user_config_dir.join("credential.lock"),
            timeout,
            poll_interval: Duration::from_millis(25),
        }
    }

    /// Acquire the exclusive lock, bounding the wait by `timeout`. On
    /// contention past the timeout, returns a backend error carrying a
    /// redacted reason (no secret).
    pub(crate) async fn acquire(&self) -> Result<LockGuard, CredentialStoreError> {
        if let Some(parent) = self.lock_path.parent() {
            // Best-effort: the user config dir usually exists already.
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.lock_path)
            .await
            .map_err(|error| CredentialStoreError::Backend {
                provider: "*".to_owned(),
                reason: format!("credential lock open failed: {error}"),
            })?;
        use fs4::tokio::AsyncFileExt;
        // fs4's tokio `lock` is a blocking syscall (not a future), so a bounded
        // wait uses non-blocking `try_lock` plus sleep.
        let deadline = Instant::now() + self.timeout;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(LockGuard { _file: file }),
                Err(fs4::TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(CredentialStoreError::Backend {
                            provider: "*".to_owned(),
                            reason: "credential lock contention timeout".to_owned(),
                        });
                    }
                    tokio::time::sleep(self.poll_interval).await;
                }
                Err(error) => {
                    return Err(CredentialStoreError::Backend {
                        provider: "*".to_owned(),
                        reason: format!("credential lock acquire failed: {error}"),
                    });
                }
            }
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        use fs4::tokio::AsyncFileExt;
        // Best-effort immediate release; the OS also releases on handle close.
        let _ = self._file.unlock();
    }
}

// ---------------------------------------------------------------------------
// KeychainCredentialStore — concrete CredentialStore
// ---------------------------------------------------------------------------

/// OS-keychain-backed [`CredentialStore`]. Wraps an injected
/// [`KeyringBackend`] (production: [`KeyringCoreBackend`]; tests:
/// [`FakeKeyringBackend`]) and serializes entries as a versioned JSON envelope
/// keyed by provider id under service [`KEYCHAIN_SERVICE`].
pub struct KeychainCredentialStore {
    backend: Box<dyn KeyringBackend>,
    service: String,
    lock: LockCoordinator,
}

impl KeychainCredentialStore {
    /// Construct a store over `backend`, coordinating mutations with a lock at
    /// `user_config_dir/credential.lock`.
    pub fn new(backend: Box<dyn KeyringBackend>, user_config_dir: PathBuf) -> Self {
        Self::with_lock_timeout(backend, user_config_dir, Duration::from_secs(5))
    }

    /// Same as [`Self::new`] but with a custom mutation-lock timeout. Tests use
    /// a short timeout to exercise contention deterministically.
    pub fn with_lock_timeout(
        backend: Box<dyn KeyringBackend>,
        user_config_dir: PathBuf,
        lock_timeout: Duration,
    ) -> Self {
        Self {
            backend,
            service: KEYCHAIN_SERVICE.to_owned(),
            lock: LockCoordinator::with_timeout(user_config_dir, lock_timeout),
        }
    }

    fn backend_err(&self, provider_id: &str, error: BackendError) -> CredentialStoreError {
        match error {
            BackendError::BackendUnavailable(reason) => CredentialStoreError::BackendUnavailable {
                provider: provider_id.to_owned(),
                reason,
            },
            BackendError::Other(reason) => CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason,
            },
        }
    }

    /// Read the sole credential presence/kind metadata source. The marker is
    /// non-secret, but its raw value is still never retained in an error.
    fn read_marker_kind(
        &self,
        provider_id: &str,
    ) -> Result<Option<CredentialKind>, CredentialStoreError> {
        match self.backend.get(KEYCHAIN_PRESENCE_SERVICE, provider_id) {
            Ok(Some(marker)) => Ok(Some(CredentialKind::parse_marker(&marker, provider_id)?)),
            Ok(None) => Ok(None),
            Err(error) => Err(self.backend_err(provider_id, error)),
        }
    }

    /// Read without acquiring the mutation lock. Reads are lock-free; only
    /// mutations take the lock.
    async fn read_unlocked(
        &self,
        provider_id: &str,
    ) -> Result<Option<Credential>, CredentialStoreError> {
        match self.backend.get(&self.service, provider_id) {
            Ok(Some(raw)) => Ok(Some(decode_credential(&raw, provider_id)?)),
            Ok(None) => Ok(None),
            Err(error) => Err(self.backend_err(provider_id, error)),
        }
    }

    /// Read marker then protected entry and require both entries to describe
    /// the same credential kind. Marker absence is authoritative absence;
    /// marker-only and mixed-kind transitions fail closed.
    async fn read_consistent_unlocked(
        &self,
        provider_id: &str,
    ) -> Result<Option<Credential>, CredentialStoreError> {
        let Some(expected_kind) = self.read_marker_kind(provider_id)? else {
            return Ok(None);
        };
        let Some(credential) = self.read_unlocked(provider_id).await? else {
            return Err(CredentialStoreError::CorruptMarker {
                provider: provider_id.to_owned(),
            });
        };
        let actual_kind = CredentialKind::for_credential(&credential);
        if actual_kind != expected_kind {
            return Err(CredentialStoreError::UnexpectedCredentialKind {
                provider: provider_id.to_owned(),
                expected: expected_kind.marker(),
                actual: actual_kind.marker(),
            });
        }
        Ok(Some(credential))
    }

    /// Write without acquiring the mutation lock. Package-private so T2 OAuth
    /// refresh can hold one external lock across read/HTTP/write without
    /// recursively acquiring the public locked `write`.
    pub(crate) async fn write_unlocked(
        &self,
        provider_id: &str,
        cred: &Credential,
    ) -> Result<(), CredentialStoreError> {
        let marker = CredentialKind::for_credential(cred).marker();
        self.backend
            .set(KEYCHAIN_PRESENCE_SERVICE, provider_id, marker)
            .map_err(|error| self.backend_err(provider_id, error))?;

        let raw = encode_credential(cred);
        self.backend
            .set(&self.service, provider_id, &raw)
            .map_err(|error| self.backend_err(provider_id, error))
    }

    /// Delete without acquiring the mutation lock. Package-private (same
    /// rationale as [`Self::write_unlocked`]).
    pub(crate) async fn delete_unlocked(
        &self,
        provider_id: &str,
    ) -> Result<(), CredentialStoreError> {
        self.backend
            .delete(&self.service, provider_id)
            .map_err(|error| self.backend_err(provider_id, error))?;
        self.backend
            .delete(KEYCHAIN_PRESENCE_SERVICE, provider_id)
            .map_err(|error| self.backend_err(provider_id, error))
    }

    /// Acquire the mutation lock (Phase 14.2 OAuth refresh). Holds the exclusive
    /// `fs4` lock across the refresh-HTTP + write so refresh-token rotation
    /// cannot race a concurrent refresh. Drop the returned guard to release.
    pub(crate) async fn acquire_lock(&self) -> Result<LockGuard, CredentialStoreError> {
        self.lock.acquire().await
    }
}

pub(crate) async fn keychain_store_from_factory(
    user_config_dir: PathBuf,
    backend_factory: KeyringBackendFactory,
) -> KeychainCredentialStore {
    // The native keyring backend eagerly connects to the platform secret store.
    // On Linux that is the Secret Service via zbus, whose blocking adapter
    // (`zbus::utils::block_on`) spins up a nested tokio runtime to drive the
    // D-Bus connection. Constructing it from a runtime worker would nest
    // runtimes and panic ("Cannot start a runtime from within a runtime"), so
    // build the store on a `spawn_blocking` thread, which carries no runtime
    // context. The blocking-pool thread is the established tokio pattern for a
    // sync backend that itself owns a runtime.
    tokio::task::spawn_blocking(move || {
        KeychainCredentialStore::new(backend_factory(), user_config_dir)
    })
    .await
    .expect("keyring store construction thread")
}

impl CredentialStore for KeychainCredentialStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { self.read_consistent_unlocked(provider_id).await })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            // Public writes are unconditional last-writer-wins mutations. The
            // lock serializes them; only T2's refresh read-modify-write path
            // re-reads state after acquiring its external lock.
            let _guard = self.lock.acquire().await?;
            self.write_unlocked(provider_id, cred).await
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            let _guard = self.lock.acquire().await?;
            self.delete_unlocked(provider_id).await
        })
    }

    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move { self.probe_metadata(provider_id).await.source })
    }
}

impl CredentialMetadataStore for KeychainCredentialStore {
    fn probe_metadata<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, CredentialMetadataProbe> {
        let label = format!("keychain {}:{}", self.service, provider_id);
        Box::pin(async move {
            match self.read_marker_kind(provider_id) {
                Ok(Some(kind)) => CredentialMetadataProbe {
                    source: CredentialSource::Present { label },
                    kind: Some(kind.into()),
                    failure: None,
                },
                Ok(None) => CredentialMetadataProbe {
                    source: CredentialSource::Absent,
                    kind: None,
                    failure: None,
                },
                Err(CredentialStoreError::BackendUnavailable { reason, .. }) => {
                    CredentialMetadataProbe {
                        source: CredentialSource::BackendUnavailable { reason },
                        kind: None,
                        failure: Some(CredentialProbeFailure::BackendUnavailable),
                    }
                }
                Err(CredentialStoreError::Backend { reason, .. }) => CredentialMetadataProbe {
                    source: CredentialSource::BackendUnavailable { reason },
                    kind: None,
                    failure: Some(CredentialProbeFailure::Operational),
                },
                Err(CredentialStoreError::CorruptMarker { .. }) => CredentialMetadataProbe {
                    source: CredentialSource::BackendUnavailable {
                        reason: "credential marker is corrupt".to_owned(),
                    },
                    kind: None,
                    failure: Some(CredentialProbeFailure::CorruptMarker),
                },
                Err(_) => CredentialMetadataProbe {
                    source: CredentialSource::BackendUnavailable {
                        reason: "credential marker probe failed".to_owned(),
                    },
                    kind: None,
                    failure: Some(CredentialProbeFailure::Operational),
                },
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Credential resolver (keychain-first, env fallback for API keys)
// ---------------------------------------------------------------------------

/// Environment-variable lookup closure. Production reads `std::env::var`;
/// tests inject a controlled map so they never race on process env.
pub type EnvLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Where a resolved API key came from.
#[derive(Debug, Clone)]
pub enum ApiKeySource {
    /// Read from the OS keychain.
    Store,
    /// Read from an environment variable. `backend_unavailable` records that
    /// the keychain backend was unreachable (headless / no daemon), so callers
    /// can emit the backend-unavailable fallback diagnostic.
    Env {
        env_var: String,
        backend_unavailable: bool,
    },
}

/// A resolved API key + its non-secret source.
pub struct ResolvedApiKey {
    /// The secret key value. Expose only at the concrete-provider HTTP
    /// boundary.
    pub value: SecretString,
    /// Non-secret source label for diagnostics.
    pub source: ApiKeySource,
}

/// Composes a [`CredentialStore`] with environment fallback. For API keys the
/// keychain is primary; on `Absent` or `BackendUnavailable` the resolver falls
/// back to the configured env var. Persisted OAuth remains keychain-required
/// (no env fallback) and is owned by T2.
#[derive(Clone)]
pub struct CredentialResolver {
    store: Arc<KeychainCredentialStore>,
    env_lookup: EnvLookup,
    refresh_timeout: Duration,
}

impl CredentialResolver {
    pub fn new(store: Arc<KeychainCredentialStore>, env_lookup: EnvLookup) -> Self {
        Self::with_refresh_timeout(store, env_lookup, OAUTH_REFRESH_TIMEOUT)
    }

    /// Construct a resolver with an explicit refresh timeout. Production uses
    /// 30 seconds; tests inject a shorter bound without contacting a provider.
    #[doc(hidden)]
    pub fn with_refresh_timeout(
        store: Arc<KeychainCredentialStore>,
        env_lookup: EnvLookup,
        refresh_timeout: Duration,
    ) -> Self {
        Self {
            store,
            env_lookup,
            refresh_timeout,
        }
    }

    /// Production resolver: env access via `std::env::var`.
    pub fn production(store: Arc<KeychainCredentialStore>) -> Self {
        Self::new(store, Arc::new(|name: &str| std::env::var(name).ok()))
    }

    /// Resolve an API key for `provider_id`, keychain-first with env fallback.
    /// Returns `None` if neither keychain nor env has a non-empty key.
    pub async fn resolve_api_key(
        &self,
        provider_id: &str,
        env_var: &str,
    ) -> Result<Option<ResolvedApiKey>, CredentialStoreError> {
        match self.store.read_consistent_unlocked(provider_id).await {
            Ok(Some(Credential::ApiKey(value))) => Ok(Some(ResolvedApiKey {
                value,
                source: ApiKeySource::Store,
            })),
            Ok(Some(Credential::OAuthToken { .. })) => {
                Err(CredentialStoreError::UnexpectedCredentialKind {
                    provider: provider_id.to_owned(),
                    expected: "api_key",
                    actual: "oauth_token",
                })
            }
            Ok(None) => Ok(self.env_fallback(env_var, false)),
            Err(CredentialStoreError::BackendUnavailable { .. }) => {
                Ok(self.env_fallback(env_var, true))
            }
            Err(error) => Err(error),
        }
    }

    fn env_fallback(&self, env_var: &str, backend_unavailable: bool) -> Option<ResolvedApiKey> {
        (self.env_lookup)(env_var)
            .filter(|value| !value.trim().is_empty())
            .map(|value| ResolvedApiKey {
                value: secret_string(value),
                source: ApiKeySource::Env {
                    env_var: env_var.to_owned(),
                    backend_unavailable,
                },
            })
    }

    /// Resolve a stored OAuth credential for `provider_id`, refreshing under the
    /// mutation lock when the access token is near expiry (5-minute skew via
    /// [`OAuthCredential::needs_refresh`]). Double-checks under the lock and
    /// re-reads after a refresh failure so a concurrent writer's fresh token is
    /// used. Writes only on a successful refresh (no partial write). Returns
    /// [`ProviderError::CredentialNeeded`] when no OAuth credential is stored.
    pub async fn resolve_oauth(
        &self,
        provider_id: &str,
        oauth: &dyn OAuthProvider,
    ) -> Result<ResolvedAuth, ProviderError> {
        // Fast path: lock-free read.
        let cred = match self
            .read_oauth(provider_id)
            .await
            .map_err(store_err_to_provider)?
        {
            Some(cred) => cred,
            None => {
                return Err(ProviderError::CredentialNeeded {
                    provider_id: provider_id.to_owned(),
                });
            }
        };
        if !cred.needs_refresh() {
            return Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: cred.access.clone(),
                base_url: cred.base_url.clone(),
                account_id: cred.account_id.clone(),
                provenance: Default::default(),
            });
        }
        // Slow path: hold the lock across re-read + refresh-HTTP + write so
        // refresh-token rotation cannot race a concurrent refresh.
        let _guard = self
            .store
            .acquire_lock()
            .await
            .map_err(store_err_to_provider)?;
        let cred = match self
            .read_oauth(provider_id)
            .await
            .map_err(store_err_to_provider)?
        {
            Some(cred) => cred,
            None => {
                return Err(ProviderError::CredentialNeeded {
                    provider_id: provider_id.to_owned(),
                });
            }
        };
        if !cred.needs_refresh() {
            // Another writer refreshed between our fast read and the lock.
            return Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: cred.access.clone(),
                base_url: cred.base_url.clone(),
                account_id: cred.account_id.clone(),
                provenance: Default::default(),
            });
        }
        let refresh = match tokio::time::timeout(self.refresh_timeout, oauth.refresh(&cred)).await {
            Ok(refresh) => refresh,
            Err(_) => Err(ProviderError::AuthFailed(format!(
                "OAuth refresh timed out for provider '{provider_id}'"
            ))),
        };
        match refresh {
            Ok(refreshed) => {
                let new_store = Credential::from(refreshed.clone());
                self.store
                    .write_unlocked(provider_id, &new_store)
                    .await
                    .map_err(store_err_to_provider)?;
                Ok(ResolvedAuth {
                    scheme: AuthScheme::Bearer,
                    secret: refreshed.access,
                    base_url: refreshed.base_url,
                    account_id: refreshed.account_id,
                    provenance: Default::default(),
                })
            }
            Err(refresh_err) => {
                // Post-failure re-read under the lock: a concurrent writer may
                // have refreshed despite our HTTP failing.
                match self.read_oauth(provider_id).await {
                    Ok(Some(reread)) if !reread.needs_refresh() => Ok(ResolvedAuth {
                        scheme: AuthScheme::Bearer,
                        secret: reread.access.clone(),
                        base_url: reread.base_url.clone(),
                        account_id: reread.account_id.clone(),
                        provenance: Default::default(),
                    }),
                    _ => Err(refresh_err),
                }
            }
        }
    }

    /// Read the stored OAuth credential (lock-free). The marker is the sole
    /// kind source: an API-key marker returns a typed wrong-kind error without
    /// reading the protected entry.
    async fn read_oauth(
        &self,
        provider_id: &str,
    ) -> Result<Option<OAuthCredential>, CredentialStoreError> {
        match self.store.read_consistent_unlocked(provider_id).await {
            Ok(Some(Credential::OAuthToken {
                access,
                refresh,
                expires_at,
                base_url,
                account_id,
            })) => Ok(Some(OAuthCredential {
                access,
                refresh,
                expires_at,
                base_url,
                account_id,
            })),
            Ok(Some(Credential::ApiKey(_))) => {
                Err(CredentialStoreError::UnexpectedCredentialKind {
                    provider: provider_id.to_owned(),
                    expected: "oauth_token",
                    actual: "api_key",
                })
            }
            Ok(None) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Whether a stored OAuth credential exists for `provider_id`. Reads only
    /// the closed non-secret marker. Backend unavailability is treated as no
    /// OAuth so Anthropic may use API-key env fallback; operational and
    /// corrupt-marker errors propagate.
    pub async fn has_oauth_credential(&self, provider_id: &str) -> Result<bool, ProviderError> {
        match self.store.read_marker_kind(provider_id) {
            Ok(Some(CredentialKind::OAuthToken)) => Ok(true),
            Ok(Some(CredentialKind::ApiKey)) | Ok(None) => Ok(false),
            Err(CredentialStoreError::BackendUnavailable { .. }) => Ok(false),
            Err(error) => Err(store_err_to_provider(error)),
        }
    }

    /// The stored OAuth credential's non-secret `base_url` (e.g. a Copilot
    /// enterprise host), or `None` when no credential is stored OR the stored
    /// credential has no base_url (Anthropic PKCE). Returns ONLY the base_url —
    /// access/refresh secrets never leave the resolver. The `None` result is
    /// meaningful only together with [`has_oauth_credential`](Self::has_oauth_credential).
    pub async fn read_oauth_base_url(
        &self,
        provider_id: &str,
    ) -> Result<Option<String>, ProviderError> {
        Ok(self
            .read_oauth(provider_id)
            .await
            .map_err(store_err_to_provider)?
            .and_then(|c| c.base_url))
    }

    /// The injectable environment lookup, for constructing an
    /// [`AuthSource::EnvOAuthToken`] variant that shares this resolver's
    /// injected env (tests pass a controlled map; production reads the real
    /// environment).
    pub fn env_lookup(&self) -> EnvLookup {
        Arc::clone(&self.env_lookup)
    }

    /// Convenience: read `env_var` through the injected lookup (or `None` when
    /// absent/empty). Layered auth calls this for every Anthropic stream.
    pub fn env_value(&self, env_var: &str) -> Option<String> {
        (self.env_lookup)(env_var).filter(|v| !v.trim().is_empty())
    }
}

fn store_err_to_provider(error: CredentialStoreError) -> ProviderError {
    ProviderError::Config(format!("credential store error: {error}"))
}

// ---------------------------------------------------------------------------
// AuthSource — how opi-coding-agent sources auth for a provider (Phase 14.2)
// ---------------------------------------------------------------------------

/// How `opi-coding-agent` sources auth for a provider. Implements
/// [`AuthResolver`]: [`Baked`](Self::Baked) returns a fixed key;
/// [`Store`](Self::Store) reads/refreshes a stored OAuth credential via
/// [`CredentialResolver`]; [`EnvOAuthToken`](Self::EnvOAuthToken) reads a
/// non-refreshable OAuth access token from an environment variable; and
/// [`Layered`](Self::Layered) re-evaluates store/OAuth-env/API-key precedence
/// on every stream.
pub enum AuthSource {
    /// A fixed secret baked at construction (static API keys).
    Baked(SecretString),
    /// A stored OAuth credential refreshed near expiry via the resolver.
    Store {
        resolver: Arc<CredentialResolver>,
        provider_id: String,
        oauth: Arc<dyn OAuthProvider>,
    },
    /// A non-refreshable OAuth access token read from an environment variable
    /// (e.g. `ANTHROPIC_OAUTH_TOKEN`). Used until 401, then explicit re-login.
    EnvOAuthToken {
        provider_id: String,
        env_var: String,
        env_lookup: EnvLookup,
    },
    /// Per-stream precedence for providers that support stored OAuth, an
    /// OAuth access-token environment variable, and an API key.
    Layered {
        resolver: Arc<CredentialResolver>,
        provider_id: String,
        oauth: Arc<dyn OAuthProvider>,
        oauth_env_var: String,
        api_key_env_var: String,
    },
}

impl AuthResolver for AuthSource {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        match self {
            AuthSource::Baked(secret) => {
                let secret = secret.clone();
                Box::pin(async move {
                    Ok(ResolvedAuth {
                        scheme: AuthScheme::ApiKey,
                        secret,
                        base_url: None,
                        account_id: None,
                        provenance: Default::default(),
                    })
                })
            }
            AuthSource::Store {
                resolver,
                provider_id,
                oauth,
            } => {
                let resolver = resolver.clone();
                let provider_id = provider_id.clone();
                let oauth = oauth.clone();
                Box::pin(async move { resolver.resolve_oauth(&provider_id, &*oauth).await })
            }
            AuthSource::EnvOAuthToken {
                provider_id,
                env_var,
                env_lookup,
            } => {
                let provider_id = provider_id.clone();
                let env_var = env_var.clone();
                let env_lookup = env_lookup.clone();
                Box::pin(async move {
                    match (env_lookup)(&env_var) {
                        Some(value) if !value.trim().is_empty() => Ok(ResolvedAuth {
                            scheme: AuthScheme::Bearer,
                            secret: SecretString::new(value.into_boxed_str()),
                            base_url: None,
                            account_id: None,
                            provenance: Default::default(),
                        }),
                        _ => Err(ProviderError::CredentialNeeded { provider_id }),
                    }
                })
            }
            AuthSource::Layered {
                resolver,
                provider_id,
                oauth,
                oauth_env_var,
                api_key_env_var,
            } => {
                let resolver = resolver.clone();
                let provider_id = provider_id.clone();
                let oauth = oauth.clone();
                let oauth_env_var = oauth_env_var.clone();
                let api_key_env_var = api_key_env_var.clone();
                Box::pin(async move {
                    if resolver.has_oauth_credential(&provider_id).await? {
                        return resolver.resolve_oauth(&provider_id, &*oauth).await;
                    }
                    if let Some(value) = resolver.env_value(&oauth_env_var) {
                        return Ok(ResolvedAuth {
                            scheme: AuthScheme::Bearer,
                            secret: SecretString::new(value.into_boxed_str()),
                            base_url: None,
                            account_id: None,
                            provenance: Default::default(),
                        });
                    }
                    match resolver
                        .resolve_api_key(&provider_id, &api_key_env_var)
                        .await
                        .map_err(store_err_to_provider)?
                    {
                        Some(resolved) => Ok(ResolvedAuth {
                            scheme: AuthScheme::ApiKey,
                            secret: resolved.value,
                            base_url: None,
                            account_id: None,
                            provenance: Default::default(),
                        }),
                        None => Err(ProviderError::CredentialNeeded { provider_id }),
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    struct FixedErrorBackend(super::BackendError);

    #[test]
    fn secret_envelope_intermediates_implement_zeroize_for_raii_cleanup() {
        fn assert_zeroize<T: zeroize::Zeroize>() {}

        assert_zeroize::<super::EnvelopeFields>();
        assert_zeroize::<super::ApiKeyEnvelopeV1>();
        assert_zeroize::<super::OAuthEnvelopeV1>();
    }

    #[test]
    fn credential_envelopes_reject_cross_kind_and_unknown_fields_without_leaks() {
        use opi_ai::credential::CredentialStoreError;

        const ACCESS: &str = "oauth-access-canary-must-not-leak";
        const REFRESH: &str = "oauth-refresh-canary-must-not-leak";
        for raw in [
            format!(r#"{{"version":1,"kind":"api_key","api_key":"key","access":"{ACCESS}"}}"#),
            format!(
                r#"{{"version":1,"kind":"oauth","access":"{ACCESS}","refresh":"{REFRESH}","api_key":"key"}}"#
            ),
            format!(
                r#"{{"version":1,"kind":"oauth","access":"{ACCESS}","refresh":"{REFRESH}","future_field":true}}"#
            ),
        ] {
            let error = super::decode_credential(&raw, "provider")
                .expect_err("mixed-kind and unknown fields must fail closed");
            assert!(matches!(
                error,
                CredentialStoreError::MalformedEnvelope { .. }
            ));
            for rendered in [format!("{error}"), format!("{error:?}")] {
                assert!(!rendered.contains(ACCESS));
                assert!(!rendered.contains(REFRESH));
            }
        }
    }

    impl super::KeyringBackend for FixedErrorBackend {
        fn get(
            &self,
            _service: &str,
            _provider_id: &str,
        ) -> Result<Option<String>, super::BackendError> {
            Err(self.0.clone())
        }

        fn set(
            &self,
            _service: &str,
            _provider_id: &str,
            _value: &str,
        ) -> Result<(), super::BackendError> {
            unreachable!("fixed-error test backend is read-only")
        }

        fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), super::BackendError> {
            unreachable!("fixed-error test backend is read-only")
        }
    }

    #[tokio::test]
    async fn initialization_error_category_controls_env_fallback() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use opi_ai::credential::CredentialStoreError;
        use secrecy::ExposeSecret;

        let unavailable_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = Arc::clone(&unavailable_calls);
        let unavailable_store = super::KeychainCredentialStore::new(
            Box::new(super::KeyringCoreBackend {
                _native: None,
                initialization_error: Some(super::BackendError::BackendUnavailable(
                    "no daemon".to_owned(),
                )),
            }),
            tempfile::tempdir().expect("temp dir").path().to_path_buf(),
        );
        let unavailable_resolver = super::CredentialResolver::new(
            Arc::new(unavailable_store),
            Arc::new(move |_name: &str| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                Some("env-fallback-canary".to_owned())
            }),
        );
        let resolved = unavailable_resolver
            .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
            .await
            .expect("backend unavailable permits env fallback")
            .expect("env fallback exists");
        assert_eq!(resolved.value.expose_secret(), "env-fallback-canary");
        assert_eq!(unavailable_calls.load(Ordering::SeqCst), 1);

        let operational_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = Arc::clone(&operational_calls);
        let operational_store = super::KeychainCredentialStore::new(
            Box::new(super::KeyringCoreBackend {
                _native: None,
                initialization_error: Some(super::BackendError::Other(
                    "credential store locked".to_owned(),
                )),
            }),
            tempfile::tempdir().expect("temp dir").path().to_path_buf(),
        );
        let operational_resolver = super::CredentialResolver::new(
            Arc::new(operational_store),
            Arc::new(move |_name: &str| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                Some("must-not-fallback".to_owned())
            }),
        );
        assert!(matches!(
            operational_resolver
                .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
                .await,
            Err(CredentialStoreError::Backend { ref reason, .. })
                if reason.contains("locked")
        ));
        assert_eq!(operational_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn no_storage_access_locked_is_operational_error() {
        let error = keyring_core::Error::NoStorageAccess(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "credential store locked",
        )));

        match super::map_keyring_error(error) {
            super::BackendError::Other(reason) => assert!(reason.contains("locked")),
            other => panic!("expected operational backend error, got {other:?}"),
        }
    }

    #[test]
    fn keyring_error_mapping_only_allows_narrow_linux_daemon_absence() {
        use keyring_core::Error::{NoStorageAccess, PlatformFailure};

        for (target_os, storage_access, reason, unavailable) in [
            (
                "linux",
                true,
                "org.freedesktop.DBus.Error.ServiceUnknown",
                true,
            ),
            ("linux", false, "connection refused", true),
            ("linux", true, "credential store locked", false),
            ("linux", false, "permission denied", false),
            (
                "windows",
                true,
                "org.freedesktop.DBus.Error.ServiceUnknown",
                false,
            ),
            ("windows", false, "connection refused", false),
            (
                "macos",
                true,
                "org.freedesktop.DBus.Error.NameHasNoOwner",
                false,
            ),
            ("macos", false, "failed to connect", false),
        ] {
            let platform_error = Box::new(std::io::Error::other(reason));
            let error = if storage_access {
                NoStorageAccess(platform_error)
            } else {
                PlatformFailure(platform_error)
            };
            let classified = super::map_keyring_error_for_platform(target_os, error);
            assert_eq!(
                matches!(classified, super::BackendError::BackendUnavailable(_)),
                unavailable,
                "{target_os}: {reason}"
            );
        }
    }

    #[tokio::test]
    async fn no_default_store_is_operational_and_never_allows_env_fallback() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use opi_ai::credential::CredentialStoreError;

        let mapped =
            super::map_keyring_error_for_platform("linux", keyring_core::Error::NoDefaultStore);
        assert!(matches!(mapped, super::BackendError::Other(_)));

        let store = super::KeychainCredentialStore::new(
            Box::new(FixedErrorBackend(mapped)),
            tempfile::tempdir().expect("temp dir").path().to_path_buf(),
        );
        let fallback_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = Arc::clone(&fallback_calls);
        let resolver = super::CredentialResolver::new(
            Arc::new(store),
            Arc::new(move |_name: &str| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                Some("must-not-fallback".to_owned())
            }),
        );

        assert!(matches!(
            resolver
                .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
                .await,
            Err(CredentialStoreError::Backend { ref reason, .. })
                if reason.contains("No default store")
        ));
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn failed_to_connect_with_operational_reason_never_allows_env_fallback() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use keyring_core::Error::{NoStorageAccess, PlatformFailure};
        use opi_ai::credential::CredentialStoreError;

        for (reason, storage_access) in [
            (
                "failed to connect to Secret Service: permission denied",
                true,
            ),
            (
                "failed to connect to Secret Service: credential store locked",
                false,
            ),
        ] {
            let platform_error = Box::new(std::io::Error::other(reason));
            let error = if storage_access {
                NoStorageAccess(platform_error)
            } else {
                PlatformFailure(platform_error)
            };
            let mapped = super::map_keyring_error_for_platform("linux", error);
            assert!(
                matches!(mapped, super::BackendError::Other(_)),
                "operational failure was classified as unavailable: {reason}"
            );

            let store = super::KeychainCredentialStore::new(
                Box::new(FixedErrorBackend(mapped)),
                tempfile::tempdir().expect("temp dir").path().to_path_buf(),
            );
            let fallback_calls = Arc::new(AtomicUsize::new(0));
            let observed_calls = Arc::clone(&fallback_calls);
            let resolver = super::CredentialResolver::new(
                Arc::new(store),
                Arc::new(move |_name: &str| {
                    observed_calls.fetch_add(1, Ordering::SeqCst);
                    Some("must-not-fallback".to_owned())
                }),
            );

            assert!(matches!(
                resolver
                    .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
                    .await,
                Err(CredentialStoreError::Backend { ref reason, .. })
                    if reason.contains("failed to connect")
            ));
            assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn secret_service_name_alone_never_means_daemon_unavailable() {
        for reason in [
            "org.freedesktop.secrets: collection locked",
            "org.freedesktop.secrets: access denied",
            "permission denied opening org.freedesktop.secrets",
        ] {
            assert!(
                !super::secret_service_is_unavailable(reason),
                "service name is not an absence signature: {reason}"
            );
        }
    }

    #[test]
    fn explicit_secret_service_absence_signatures_are_unavailable() {
        for reason in [
            "org.freedesktop.DBus.Error.ServiceUnknown",
            "org.freedesktop.DBus.Error.NameHasNoOwner",
            "connection refused",
        ] {
            assert!(
                super::secret_service_is_unavailable(reason),
                "explicit daemon absence must be recognized: {reason}"
            );
        }
    }

    #[test]
    fn keyring_core_backend_probe_reads_only_nonsecret_marker_entry() {
        use super::KeyringBackend;
        use futures_util::FutureExt;
        use opi_ai::credential::{CredentialSource, CredentialStore};

        let _serial = crate::native_keyring::KEYRING_TEST_LOCK
            .lock()
            .expect("keyring test lock");
        keyring_core::unset_default_store();
        let mock: std::sync::Arc<keyring_core::CredentialStore> =
            keyring_core::mock::Store::new().expect("mock keyring store");
        let native = crate::native_keyring::install_store(mock).expect("install mock store");
        let backend = super::KeyringCoreBackend {
            _native: Some(native),
            initialization_error: None,
        };
        backend
            .set(
                super::KEYCHAIN_SERVICE,
                "anthropic",
                "protected-test-secret",
            )
            .expect("write protected entry through production backend");
        backend
            .set(super::KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key")
            .expect("write non-secret marker through production backend");

        let protected = keyring_core::Entry::new(super::KEYCHAIN_SERVICE, "anthropic")
            .expect("protected mock entry");
        let mock_credential = protected
            .as_any()
            .downcast_ref::<keyring_core::mock::Cred>()
            .expect("mock credential");
        mock_credential.set_error(keyring_core::Error::Invalid(
            "protected read".to_owned(),
            "must remain untouched during probe".to_owned(),
        ));

        let dir = tempfile::tempdir().expect("temp config dir");
        let store =
            super::KeychainCredentialStore::new(Box::new(backend), dir.path().to_path_buf());
        let probe = store
            .probe("anthropic")
            .now_or_never()
            .expect("marker probe has no suspension point");
        assert!(matches!(probe, CredentialSource::Present { .. }));
        assert!(
            matches!(
                protected.get_password(),
                Err(keyring_core::Error::Invalid(_, _))
            ),
            "the pending protected-read failure proves probe never touched it"
        );
        drop(store);
        assert!(keyring_core::get_default_store().is_none());
    }
}

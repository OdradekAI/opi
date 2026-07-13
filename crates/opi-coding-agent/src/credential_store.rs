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
};
use opi_ai::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};

/// Keychain service name. Every opi entry is stored under service `opi` with
/// the provider id as the account key.
pub const KEYCHAIN_SERVICE: &str = "opi";

/// Versioned envelope marker. Unknown versions decode to an explicit
/// [`CredentialStoreError::UnknownEnvelope`].
const ENVELOPE_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Backend injection seam
// ---------------------------------------------------------------------------

/// Errors from a [`KeyringBackend`] operation.
#[derive(Debug)]
pub enum BackendError {
    /// The credential backend could not be reached (no keychain daemon, no
    /// compiled native store, platform error). API keys may fall back to env.
    BackendUnavailable(String),
    /// Any other backend failure.
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

/// Production backend wrapping `keyring-core`.
///
/// In Phase 14.1 no native store crate is compiled in, so `Entry::new` returns
/// `NoDefaultStore` and every probe resolves to
/// [`CredentialSource::BackendUnavailable`] -> env fallback. Real platform
/// storage (and the write/login path) ship with T2, which adds the native
/// store crates and `/login`.
pub struct KeyringCoreBackend;

impl KeyringCoreBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringCoreBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringBackend for KeyringCoreBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        let entry = match keyring_core::Entry::new(service, provider_id) {
            Ok(entry) => entry,
            Err(keyring_core::Error::NoDefaultStore) => {
                return Err(BackendError::BackendUnavailable(
                    "no default keyring store".to_owned(),
                ));
            }
            Err(error) => return Err(BackendError::Other(error.to_string())),
        };
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(BackendError::Other(error.to_string())),
        }
    }

    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        let entry = match keyring_core::Entry::new(service, provider_id) {
            Ok(entry) => entry,
            Err(keyring_core::Error::NoDefaultStore) => {
                return Err(BackendError::BackendUnavailable(
                    "no default keyring store".to_owned(),
                ));
            }
            Err(error) => return Err(BackendError::Other(error.to_string())),
        };
        entry
            .set_password(value)
            .map_err(|error| BackendError::Other(error.to_string()))
    }

    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        let entry = match keyring_core::Entry::new(service, provider_id) {
            Ok(entry) => entry,
            Err(keyring_core::Error::NoDefaultStore) => {
                return Err(BackendError::BackendUnavailable(
                    "no default keyring store".to_owned(),
                ));
            }
            Err(error) => return Err(BackendError::Other(error.to_string())),
        };
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(BackendError::Other(error.to_string())),
        }
    }
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
        // Record the critical-section window AROUND the (optional) delay + the
        // map insert, so a test can prove two writers did not overlap.
        let start = Instant::now();
        if !self.set_delay.is_zero() {
            std::thread::sleep(self.set_delay);
        }
        self.entries.lock().unwrap().insert(
            (service.to_owned(), provider_id.to_owned()),
            value.to_owned(),
        );
        let end = Instant::now();
        self.set_windows.lock().unwrap().push((start, end));
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

#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope {
    version: u32,
    kind: String,
    #[serde(flatten)]
    fields: EnvelopeFields,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
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
}

fn secret_string(value: impl Into<String>) -> SecretString {
    // secrecy 0.10 SecretString::new takes `Box<str>`.
    SecretString::new(value.into().into_boxed_str())
}

/// Encode a [`Credential`] as the v1 JSON envelope.
fn encode_credential(cred: &Credential) -> String {
    let (kind, mut fields) = match cred {
        Credential::ApiKey(key) => (
            "api_key",
            EnvelopeFields {
                api_key: Some(key.expose_secret().to_owned()),
                ..Default::default()
            },
        ),
        Credential::OAuthToken {
            access,
            refresh,
            expires_at,
            base_url,
        } => (
            "oauth",
            EnvelopeFields {
                access: Some(access.expose_secret().to_owned()),
                refresh: Some(refresh.expose_secret().to_owned()),
                expires_at: expires_at.map(|t| t.unix_timestamp()),
                base_url: base_url.clone(),
                ..Default::default()
            },
        ),
    };
    let _ = &mut fields;
    let envelope = Envelope {
        version: ENVELOPE_VERSION,
        kind: kind.to_owned(),
        fields,
    };
    serde_json::to_string(&envelope).expect("credential envelope serializes")
}

/// Decode a v1 JSON envelope into a [`Credential`].
///
/// Malformed JSON, an unknown version, or an unknown kind each yield a distinct
/// [`CredentialStoreError`] and are never collapsed into absence or env
/// fallback.
fn decode_credential(raw: &str, provider_id: &str) -> Result<Credential, CredentialStoreError> {
    let envelope: Envelope =
        serde_json::from_str(raw).map_err(|error| CredentialStoreError::MalformedEnvelope {
            provider: provider_id.to_owned(),
            reason: error.to_string(),
        })?;
    if envelope.version != ENVELOPE_VERSION {
        return Err(CredentialStoreError::UnknownEnvelope {
            provider: provider_id.to_owned(),
            version: Some(envelope.version),
            kind: None,
        });
    }
    match envelope.kind.as_str() {
        "api_key" => {
            let api_key =
                envelope
                    .fields
                    .api_key
                    .ok_or_else(|| CredentialStoreError::MalformedEnvelope {
                        provider: provider_id.to_owned(),
                        reason: "api_key envelope missing api_key field".to_owned(),
                    })?;
            Ok(Credential::ApiKey(secret_string(api_key)))
        }
        "oauth" => {
            let access =
                envelope
                    .fields
                    .access
                    .ok_or_else(|| CredentialStoreError::MalformedEnvelope {
                        provider: provider_id.to_owned(),
                        reason: "oauth envelope missing access field".to_owned(),
                    })?;
            let refresh =
                envelope
                    .fields
                    .refresh
                    .ok_or_else(|| CredentialStoreError::MalformedEnvelope {
                        provider: provider_id.to_owned(),
                        reason: "oauth envelope missing refresh field".to_owned(),
                    })?;
            let expires_at = match envelope.fields.expires_at {
                Some(secs) => Some(time::OffsetDateTime::from_unix_timestamp(secs).map_err(
                    |error| CredentialStoreError::MalformedEnvelope {
                        provider: provider_id.to_owned(),
                        reason: format!("invalid expires_at: {error}"),
                    },
                )?),
                None => None,
            };
            Ok(Credential::OAuthToken {
                access: secret_string(access),
                refresh: secret_string(refresh),
                expires_at,
                base_url: envelope.fields.base_url,
            })
        }
        other => Err(CredentialStoreError::UnknownEnvelope {
            provider: provider_id.to_owned(),
            version: None,
            kind: Some(other.to_owned()),
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
            .truncate(true)
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
            BackendError::BackendUnavailable(reason) | BackendError::Other(reason) => {
                CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason,
                }
            }
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

    /// Write without acquiring the mutation lock. Package-private so T2 OAuth
    /// refresh can hold one external lock across read/HTTP/write without
    /// recursively acquiring the public locked `write`.
    pub(crate) async fn write_unlocked(
        &self,
        provider_id: &str,
        cred: &Credential,
    ) -> Result<(), CredentialStoreError> {
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
            .map_err(|error| self.backend_err(provider_id, error))
    }

    /// Acquire the mutation lock (Phase 14.2 OAuth refresh). Holds the exclusive
    /// `fs4` lock across the refresh-HTTP + write so refresh-token rotation
    /// cannot race a concurrent refresh. Drop the returned guard to release.
    pub(crate) async fn acquire_lock(&self) -> Result<LockGuard, CredentialStoreError> {
        self.lock.acquire().await
    }
}

impl CredentialStore for KeychainCredentialStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { self.read_unlocked(provider_id).await })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            // Acquire-then-re-read: hold the exclusive lock across the write.
            // T2 refresh reuses write_unlocked under one external lock so it
            // never recursively enters this locked path.
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
        let label = format!("keychain {}:{}", self.service, provider_id);
        Box::pin(async move {
            match self.backend.get(&self.service, provider_id) {
                Ok(Some(_)) => CredentialSource::Present { label },
                Ok(None) => CredentialSource::Absent,
                Err(BackendError::BackendUnavailable(reason)) => {
                    CredentialSource::BackendUnavailable { reason }
                }
                // Probe surfaces any backend failure as unavailable so doctor
                // reports it distinctly from a missing entry; the corrupt
                // envelope itself is surfaced on read.
                Err(BackendError::Other(reason)) => CredentialSource::BackendUnavailable { reason },
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
}

impl CredentialResolver {
    pub fn new(store: Arc<KeychainCredentialStore>, env_lookup: EnvLookup) -> Self {
        Self { store, env_lookup }
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
    ) -> Option<ResolvedApiKey> {
        match self.store.probe(provider_id).await {
            CredentialSource::Present { .. } => match self.store.read(provider_id).await {
                Ok(Some(Credential::ApiKey(key))) => Some(ResolvedApiKey {
                    value: key,
                    source: ApiKeySource::Store,
                }),
                // Stored an OAuth envelope (T2 territory) or lost a probe race:
                // fall back to env rather than emitting nothing.
                Ok(_) => self.env_fallback(env_var, false),
                Err(_) => self.env_fallback(env_var, true),
            },
            CredentialSource::Absent => self.env_fallback(env_var, false),
            CredentialSource::BackendUnavailable { .. } => self.env_fallback(env_var, true),
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
        let cred = match self.read_oauth(provider_id).await? {
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
            });
        }
        // Slow path: hold the lock across re-read + refresh-HTTP + write so
        // refresh-token rotation cannot race a concurrent refresh.
        let _guard = self
            .store
            .acquire_lock()
            .await
            .map_err(store_err_to_provider)?;
        let cred = match self.read_oauth(provider_id).await? {
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
            });
        }
        match oauth.refresh(&cred).await {
            Ok(refreshed) => {
                let new_store = Credential::from(refreshed.clone());
                self.store
                    .write_unlocked(provider_id, &new_store)
                    .await
                    .map_err(store_err_to_provider)?;
                Ok(ResolvedAuth {
                    scheme: AuthScheme::Bearer,
                    secret: refreshed.access,
                })
            }
            Err(refresh_err) => {
                // Post-failure re-read under the lock: a concurrent writer may
                // have refreshed despite our HTTP failing.
                match self.read_oauth(provider_id).await? {
                    Some(reread) if !reread.needs_refresh() => Ok(ResolvedAuth {
                        scheme: AuthScheme::Bearer,
                        secret: reread.access.clone(),
                    }),
                    _ => Err(refresh_err),
                }
            }
        }
    }

    /// Read the stored OAuth credential (lock-free). Returns `None` for a
    /// missing entry or a stored non-OAuth credential.
    async fn read_oauth(
        &self,
        provider_id: &str,
    ) -> Result<Option<OAuthCredential>, ProviderError> {
        match self.store.read_unlocked(provider_id).await {
            Ok(Some(Credential::OAuthToken {
                access,
                refresh,
                expires_at,
                base_url,
            })) => Ok(Some(OAuthCredential {
                access,
                refresh,
                expires_at,
                base_url,
            })),
            Ok(Some(_)) | Ok(None) => Ok(None),
            Err(error) => Err(store_err_to_provider(error)),
        }
    }

    /// Whether a stored OAuth credential exists for `provider_id`. Uses
    /// `probe()` (secret-free, cannot fail), so a `BackendUnavailable` keychain
    /// is treated as "no credential" — the API-key/env fallback handles routing,
    /// same as `resolve_api_key`. Does not read the credential value.
    pub async fn has_oauth_credential(&self, provider_id: &str) -> Result<bool, ProviderError> {
        let source = self.store.probe(provider_id).await;
        Ok(matches!(
            source,
            opi_ai::credential::CredentialSource::Present { .. }
        ))
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
        Ok(self.read_oauth(provider_id).await?.and_then(|c| c.base_url))
    }

    /// The injectable environment lookup, for constructing an
    /// [`AuthSource::EnvOAuthToken`] variant that shares this resolver's
    /// injected env (tests pass a controlled map; production reads the real
    /// environment).
    pub fn env_lookup(&self) -> EnvLookup {
        Arc::clone(&self.env_lookup)
    }

    /// Convenience: read `env_var` through the injected lookup (or `None` when
    /// absent/empty). Used by the factory to decide the Anthropic OAuth path.
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
/// non-refreshable OAuth access token from an environment variable.
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
        env_var: String,
        env_lookup: EnvLookup,
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
                env_var,
                env_lookup,
            } => {
                let env_var = env_var.clone();
                let env_lookup = env_lookup.clone();
                Box::pin(async move {
                    match (env_lookup)(&env_var) {
                        Some(value) if !value.trim().is_empty() => Ok(ResolvedAuth {
                            scheme: AuthScheme::Bearer,
                            secret: SecretString::new(value.into_boxed_str()),
                        }),
                        _ => Err(ProviderError::CredentialNeeded {
                            provider_id: env_var,
                        }),
                    }
                })
            }
        }
    }
}

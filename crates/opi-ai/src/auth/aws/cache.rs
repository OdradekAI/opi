//! Injectable AWS auth-source cache and timing seams (task 20.19.3).
//!
//! This module owns the source-cache *mechanism* only: distinct cache
//! identities, single-flight active lookups, the fixed refresh windows
//! (5-minute advance window, per-session 30-second refresh throttle),
//! per-key revisions/receipts, explicit invalidation, and registration of
//! near-expiry background refreshes into the same credential mutation owner
//! used by the product store (task 20.22's [`crate::mutation::MutationQueue`]).
//! The refresh *policies* live with each source family in `sso.rs`,
//! `login.rs`, and the chain coordinator in `local.rs`.
//!
//! Two invariants from the verified locked-Node corpus hold here:
//!
//! * Sub-token caches (SSO access tokens, login session tokens) and outer
//!   temporary credentials use distinct [`CacheIdentity`](crate::auth::aws::cache::CacheIdentity)
//!   keys and revisions; a write-back receipt for one never claims the other
//!   was saved.
//! * A strictly expired value is discarded and the next lookup blocks on a
//!   fresh active refresh; values without an expiry never refresh passively
//!   in the background.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use time::OffsetDateTime;

use crate::auth::AwsSigV4Credentials;
use crate::auth::aws::source_http::AwsSourceTransport;
use crate::credential::BoxAuthFuture;
use crate::mutation::{MutationApplied, MutationError, MutationQueue, MutationSlot};
use secrecy::SecretString;

/// Fixed advance window before expiry in which refresh is attempted
/// (the corpus-pinned 300000 ms).
pub const REFRESH_WINDOW: Duration = Duration::from_secs(300);

/// Fixed minimum interval between refresh attempts for one session
/// (the corpus-pinned 30 s per-session throttle).
pub const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// Injectable clock for source expiry decisions.
pub trait SourceClock: Send + Sync {
    /// Current wall-clock time.
    fn now(&self) -> OffsetDateTime;
}

/// Real system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemSourceClock;

impl SourceClock for SystemSourceClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Injectable source for DPoP `jti` nonce values (RFC 9449 uniqueness).
pub trait DpopJtiSource: Send + Sync {
    /// Produce one fresh nonce value.
    fn next_jti(&self) -> String;
}

/// Default `jti` source: random alphanumeric nonces from the process CSPRNG.
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomDpopJti;

impl DpopJtiSource for RandomDpopJti {
    fn next_jti(&self) -> String {
        use rand::Rng;
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        (0..22)
            .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
            .collect()
    }
}

/// Distinct cache identity for one source-owned value. Sub-token caches and
/// outer temporary credentials never share an identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheIdentity {
    /// An SSO access-token bundle, keyed by the standard SSO cache-file key.
    SsoToken { cache_key: String },
    /// A login-session token bundle, keyed by the trimmed session name.
    LoginSession { session: String },
    /// Outer temporary credentials resolved from one named source.
    Temporary {
        /// Non-secret source label (e.g. `sso`, `sts-assume-role`).
        source: String,
        /// Non-secret account id when the source reported one.
        account: Option<String>,
    },
}

impl CacheIdentity {
    /// Stable non-secret label describing the identity (diagnostics only).
    pub fn label(&self) -> String {
        match self {
            CacheIdentity::SsoToken { cache_key } => {
                format!("sso-token:{cache_key}")
            }
            CacheIdentity::LoginSession { session } => format!("login:{session}"),
            CacheIdentity::Temporary { source, account } => match account {
                Some(account) => format!("temporary:{source}:{account}"),
                None => format!("temporary:{source}"),
            },
        }
    }
}

/// One cached source value. Secret fields stay wrapped; diagnostics never
/// render them.
#[derive(Debug, Clone)]
pub enum SourceCacheValue {
    /// SSO access-token bundle (token plus refresh material).
    SsoToken(SsoTokenSnapshot),
    /// Login-session bundle.
    LoginSession(LoginTokenSnapshot),
    /// Outer temporary AWS credentials (expiry carried by the credentials).
    Temporary(AwsSigV4Credentials),
}

impl SourceCacheValue {
    /// Source-reported expiry, if any. Values without an expiry never
    /// refresh passively.
    pub fn expires_at(&self) -> Option<OffsetDateTime> {
        match self {
            SourceCacheValue::SsoToken(token) => Some(token.expires_at),
            SourceCacheValue::LoginSession(login) => login.expires_at,
            SourceCacheValue::Temporary(credentials) => credentials.expires_at,
        }
    }
}

/// Cached SSO access token plus the refresh material from the standard SSO
/// cache file. Legacy tokens carry no refresh material and never refresh.
#[derive(Debug, Clone)]
pub struct SsoTokenSnapshot {
    /// Bearer access token for the SSO portal.
    pub access_token: SecretString,
    /// Token expiry reported by the source (absolute).
    pub expires_at: OffsetDateTime,
    /// Refresh material present only for modern (sso-session) tokens.
    pub refresh: Option<SsoRefreshMaterial>,
    /// Whether this bundle came from a modern `sso_session` profile.
    pub modern: bool,
}

/// OIDC refresh material cached by a prior registration (modern SSO only).
#[derive(Debug, Clone)]
pub struct SsoRefreshMaterial {
    /// Registered OIDC client id (non-secret).
    pub client_id: String,
    /// Registered OIDC client secret.
    pub client_secret: SecretString,
    /// Refresh token for the cached access token.
    pub refresh_token: SecretString,
}

/// Cached login-session token bundle (standard login cache file).
#[derive(Debug, Clone)]
pub struct LoginTokenSnapshot {
    /// SignIn client id (non-secret).
    pub client_id: String,
    /// Current refresh token (may be rotated by a refresh).
    pub refresh_token: SecretString,
    /// DPoP signing key in SEC1 PEM form.
    pub signing_key: SecretString,
    /// Frozen account provenance from the cache/session ARN; the SignIn
    /// refresh response never rewrites it.
    pub account: Option<String>,
    /// Cached credentials-bearing access token, when one was refreshed.
    pub access_token: Option<SecretString>,
    /// Absolute expiry of the access token, when known.
    pub expires_at: Option<OffsetDateTime>,
}

struct EntryState {
    value: Option<SourceCacheValue>,
    revision: u64,
    last_refresh_attempt: Option<OffsetDateTime>,
    passive_inflight: bool,
    /// Set when a file-backed entry changed on disk and the value must be
    /// re-read before use.
    invalidated: bool,
}

/// Owned dependencies for background (passive) refreshes registered with
/// the mutation owner. Construct once with the same transport/clock/jti
/// instances used for active resolutions.
pub struct PassiveRefreshDeps {
    /// Transport used by background refreshes.
    pub transport: Arc<dyn AwsSourceTransport>,
    /// Clock used by background refreshes.
    pub clock: Arc<dyn SourceClock>,
    /// DPoP `jti` source used by background refreshes.
    pub jti: Arc<dyn DpopJtiSource>,
}

struct CacheInner {
    entries: Mutex<HashMap<CacheIdentity, EntryState>>,
    flights: Mutex<HashMap<CacheIdentity, Arc<tokio::sync::Mutex<()>>>>,
    queue: Option<Arc<MutationQueue>>,
    passive: Option<Arc<PassiveRefreshDeps>>,
}

/// Shared, caller-owned AWS auth-source cache. Construct one per process (or
/// per provider bundle) and pass it to every chain resolution; the
/// single-flight and refresh-window state only helps when resolutions share
/// one instance.
///
/// Near-expiry background refreshes are registered into the attached
/// mutation queue when one exists (the §2.2 credential mutation owner); the
/// original caller may leave and still await completion through the queue's
/// barrier. Without an attached queue there is no background refresh: the
/// caller keeps the old value and the next active lookup refreshes.
#[derive(Clone)]
pub struct AwsSourceCache {
    inner: Arc<CacheInner>,
}

impl Default for AwsSourceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AwsSourceCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsSourceCache")
            .field("entries", &self.inner.entries.lock().unwrap().len())
            .field("has_mutation_owner", &self.inner.queue.is_some())
            .finish()
    }
}

impl AwsSourceCache {
    /// Create a cache without a mutation owner (no background refresh).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CacheInner {
                entries: Mutex::new(HashMap::new()),
                flights: Mutex::new(HashMap::new()),
                queue: None,
                passive: None,
            }),
        }
    }

    /// Create a cache whose near-expiry background refreshes register into
    /// `queue` (the same credential mutation owner used by the product
    /// store) so completions are observable and awaitable after the caller
    /// leaves.
    pub fn with_mutation_queue(queue: Arc<MutationQueue>) -> Self {
        Self {
            inner: Arc::new(CacheInner {
                entries: Mutex::new(HashMap::new()),
                flights: Mutex::new(HashMap::new()),
                queue: Some(queue),
                passive: None,
            }),
        }
    }

    /// Attach the owned dependencies used by background refreshes. Returns
    /// a new cache handle sharing the same entries and mutation owner.
    #[must_use]
    pub fn with_passive_deps(mut self, deps: Arc<PassiveRefreshDeps>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("passive deps are attached before the cache is shared");
        inner.passive = Some(deps);
        self
    }

    /// Transport available to background refreshes, when attached.
    pub fn passive_transport(&self) -> Option<Arc<dyn AwsSourceTransport>> {
        self.inner
            .passive
            .as_ref()
            .map(|deps| deps.transport.clone())
    }

    /// Clock available to background refreshes, when attached.
    pub fn passive_clock(&self) -> Option<Arc<dyn SourceClock>> {
        self.inner.passive.as_ref().map(|deps| deps.clock.clone())
    }

    /// DPoP `jti` source available to background refreshes, when attached.
    pub fn passive_jti(&self) -> Option<Arc<dyn DpopJtiSource>> {
        self.inner.passive.as_ref().map(|deps| deps.jti.clone())
    }

    fn entry_state_mut<R>(
        &self,
        identity: &CacheIdentity,
        f: impl FnOnce(&mut EntryState) -> R,
    ) -> Option<R> {
        let mut entries = self.inner.entries.lock().unwrap();
        entries.get_mut(identity).map(f)
    }

    /// Fresh-hit lookup without any refresh: returns the cached value when
    /// it is present, not invalidated, and not strictly expired. Near-expiry
    /// values are still returned (the caller decides whether to spawn a
    /// passive refresh via [`Self::maybe_spawn_passive`]); strictly expired
    /// values are discarded here so the next lookup must refresh.
    pub fn peek(&self, identity: &CacheIdentity, now: OffsetDateTime) -> Option<SourceCacheValue> {
        let mut result = None;
        self.entry_state_mut(identity, |state| {
            if state.invalidated {
                return;
            }
            let Some(value) = state.value.as_ref() else {
                return;
            };
            let expired = value
                .expires_at()
                .is_some_and(|expires_at| expires_at <= now);
            if expired {
                // Strictly expired values are discarded; the next lookup
                // blocks on a fresh active refresh.
                state.value = None;
                state.revision += 1;
                return;
            }
            result = Some(value.clone());
        });
        result
    }

    /// Acquire the single-flight refresh lock for `identity`. Concurrent
    /// lookups await the same lock; re-peek after acquiring to reuse a
    /// completed refresh.
    pub async fn refresh_lock(&self, identity: &CacheIdentity) -> SourceRefreshGuard {
        let mutex = {
            let mut flights = self.inner.flights.lock().unwrap();
            Arc::clone(flights.entry(identity.clone()).or_default())
        };
        let guard = mutex.lock_owned().await;
        SourceRefreshGuard {
            _guard: guard,
            cache: self.clone(),
            identity: identity.clone(),
        }
    }

    /// Store a refreshed value and return the new per-key revision (the
    /// write-back receipt for this identity; distinct identities keep
    /// distinct revisions).
    pub fn store(&self, identity: CacheIdentity, value: SourceCacheValue) -> u64 {
        let mut entries = self.inner.entries.lock().unwrap();
        let state = entries.entry(identity).or_insert_with(|| EntryState {
            value: None,
            revision: 0,
            last_refresh_attempt: None,
            passive_inflight: false,
            invalidated: false,
        });
        state.value = Some(value);
        state.invalidated = false;
        state.revision += 1;
        state.revision
    }

    /// Current per-key revision (0 when the identity was never stored).
    pub fn revision(&self, identity: &CacheIdentity) -> u64 {
        self.inner
            .entries
            .lock()
            .unwrap()
            .get(identity)
            .map(|state| state.revision)
            .unwrap_or(0)
    }

    /// Explicitly invalidate a file-backed identity: the cached value is
    /// dropped and the next lookup re-reads the source file. There is no
    /// eternal module-level promise here.
    pub fn invalidate(&self, identity: &CacheIdentity) {
        self.entry_state_mut(identity, |state| {
            state.value = None;
            state.invalidated = true;
            state.revision += 1;
        });
    }

    /// Whether a new refresh attempt for `identity` is allowed under the
    /// per-session throttle.
    fn refresh_allowed(&self, identity: &CacheIdentity, now: OffsetDateTime) -> bool {
        self.inner
            .entries
            .lock()
            .unwrap()
            .get(identity)
            .and_then(|state| state.last_refresh_attempt)
            .is_none_or(|last| now - last >= MIN_REFRESH_INTERVAL)
    }

    /// Spawn one passive (background) refresh for a near-expiry value when
    /// the throttle allows, no passive refresh is already in flight, and a
    /// mutation owner is attached. The slot runs the source refresh and
    /// applies its own cache write; failures settle into the queue receipt
    /// and are swallowed here (the old value stays usable until expiry).
    /// Returns whether a refresh was registered.
    pub fn maybe_spawn_passive(
        &self,
        identity: CacheIdentity,
        now: OffsetDateTime,
        slot: MutationSlot,
    ) -> bool {
        let Some(queue) = self.inner.queue.clone() else {
            return false;
        };
        let allowed = self.refresh_allowed(&identity, now);
        let already = self
            .inner
            .entries
            .lock()
            .unwrap()
            .get(&identity)
            .is_some_and(|state| state.passive_inflight);
        if !allowed || already {
            return false;
        }
        let admitted = {
            let mut entries = self.inner.entries.lock().unwrap();
            let state = entries
                .entry(identity.clone())
                .or_insert_with(|| EntryState {
                    value: None,
                    revision: 0,
                    last_refresh_attempt: None,
                    passive_inflight: false,
                    invalidated: false,
                });
            if state.passive_inflight {
                false
            } else {
                state.passive_inflight = true;
                state.last_refresh_attempt = Some(now);
                true
            }
        };
        if !admitted {
            return false;
        }
        let cache = self.clone();
        let flight_identity = identity.clone();
        let wrapped: MutationSlot = Box::new(move || {
            let result = slot();
            Box::pin(async move {
                let outcome = result.await;
                cache.entry_state_mut(&flight_identity, |state| {
                    state.passive_inflight = false;
                });
                outcome
            }) as BoxAuthFuture<'static, Result<MutationApplied, MutationError>>
        });
        queue.submit(&identity.label(), wrapped);
        true
    }
}

/// Guard held while one active refresh for an identity runs. Other lookups
/// for the same identity await; dropping the guard releases the flight.
pub struct SourceRefreshGuard {
    _guard: tokio::sync::OwnedMutexGuard<()>,
    cache: AwsSourceCache,
    identity: CacheIdentity,
}

impl SourceRefreshGuard {
    /// Store the refreshed value (delegates to the cache `store`).
    pub fn store(self, value: SourceCacheValue) -> u64 {
        self.cache.store(self.identity, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(seconds).unwrap()
    }

    fn token_value(expires_at: OffsetDateTime) -> SourceCacheValue {
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: SecretString::from("token"),
            expires_at,
            refresh: None,
            modern: false,
        })
    }

    #[test]
    fn peek_returns_fresh_and_discards_strictly_expired_values() {
        let cache = AwsSourceCache::new();
        let identity = CacheIdentity::SsoToken {
            cache_key: "k".to_owned(),
        };
        cache.store(identity.clone(), token_value(at(10_000)));
        assert!(cache.peek(&identity, at(9_000)).is_some());
        assert!(
            cache.peek(&identity, at(10_000)).is_none(),
            "expiry is inclusive"
        );
        assert!(cache.peek(&identity, at(11_000)).is_none());
        assert!(cache.peek(&identity, at(9_000)).is_none(), "discarded");
    }

    #[test]
    fn distinct_identities_keep_distinct_revisions() {
        let cache = AwsSourceCache::new();
        let token = CacheIdentity::SsoToken {
            cache_key: "k".to_owned(),
        };
        let temporary = CacheIdentity::Temporary {
            source: "sso".to_owned(),
            account: Some("123".to_owned()),
        };
        let token_receipt = cache.store(token.clone(), token_value(at(10_000)));
        let temporary_receipt = cache.store(temporary.clone(), token_value(at(20_000)));
        assert_eq!(token_receipt, 1);
        assert_eq!(temporary_receipt, 1);
        assert_eq!(cache.revision(&token), 1);
        assert_eq!(cache.revision(&temporary), 1);
        cache.store(token.clone(), token_value(at(30_000)));
        assert_eq!(cache.revision(&token), 2);
        assert_eq!(cache.revision(&temporary), 1, "receipts stay per-key");
    }

    #[test]
    fn invalidate_forces_a_fresh_lookup() {
        let cache = AwsSourceCache::new();
        let identity = CacheIdentity::SsoToken {
            cache_key: "k".to_owned(),
        };
        cache.store(identity.clone(), token_value(at(10_000)));
        cache.invalidate(&identity);
        assert!(cache.peek(&identity, at(9_000)).is_none());
        assert_eq!(cache.revision(&identity), 2);
    }

    #[test]
    fn values_without_expiry_never_discard_on_peek() {
        let cache = AwsSourceCache::new();
        let identity = CacheIdentity::Temporary {
            source: "static".to_owned(),
            account: None,
        };
        let mut credentials = AwsSigV4Credentials::new(
            SecretString::from("AK"),
            SecretString::from("SK"),
            "us-east-1",
        );
        credentials.expires_at = None;
        cache.store(identity.clone(), SourceCacheValue::Temporary(credentials));
        assert!(cache.peek(&identity, at(1_000_000)).is_some());
    }

    #[tokio::test]
    async fn single_flight_shares_one_refresh_execution() {
        let cache = AwsSourceCache::new();
        let identity = CacheIdentity::SsoToken {
            cache_key: "k".to_owned(),
        };
        let executions = Arc::new(Mutex::new(0u32));
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let cache = cache.clone();
            let identity = identity.clone();
            let executions = executions.clone();
            tasks.push(tokio::spawn(async move {
                let _guard = cache.refresh_lock(&identity).await;
                let already = cache.peek(&identity, at(0)).is_some();
                if !already {
                    *executions.lock().unwrap() += 1;
                    tokio::time::sleep(StdDuration::from_millis(20)).await;
                    cache.store(identity, token_value(at(10_000)));
                }
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(*executions.lock().unwrap(), 1, "one active refresh ran");
    }

    #[tokio::test]
    async fn passive_refresh_registers_into_the_mutation_owner_once() {
        let cache = AwsSourceCache::with_mutation_queue(Arc::new(MutationQueue::new()));
        let identity = CacheIdentity::SsoToken {
            cache_key: "k".to_owned(),
        };
        cache.store(
            identity.clone(),
            token_value(at(3_000)), // inside the 5-minute window
        );
        let runs = Arc::new(Mutex::new(0u32));
        let slot_runs = runs.clone();
        let spawned = cache.maybe_spawn_passive(
            identity.clone(),
            at(0),
            Box::new(move || {
                let runs = slot_runs;
                Box::pin(async move {
                    *runs.lock().unwrap() += 1;
                    Ok(crate::mutation::MutationApplied::Keep)
                }) as _
            }),
        );
        assert!(spawned);
        // A second passive attempt at the same instant is suppressed: one
        // is already in flight (and the 30s throttle also applies).
        let spawned_again = cache.maybe_spawn_passive(
            identity.clone(),
            at(0),
            Box::new(move || {
                Box::pin(async { Ok(crate::mutation::MutationApplied::Keep) })
                    as BoxAuthFuture<'static, Result<MutationApplied, MutationError>>
            }),
        );
        assert!(!spawned_again);
        // The registered mutation runs to completion on the runtime; the
        // original caller may already have left.
        for _ in 0..100 {
            if *runs.lock().unwrap() == 1 {
                break;
            }
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
        assert_eq!(*runs.lock().unwrap(), 1, "passive slot ran to completion");
        // Even after it settled, the 30s throttle suppresses an immediate
        // re-spawn at the same instant.
        let throttled = cache.maybe_spawn_passive(
            identity.clone(),
            at(0),
            Box::new(move || {
                Box::pin(async { Ok(crate::mutation::MutationApplied::Keep) })
                    as BoxAuthFuture<'static, Result<MutationApplied, MutationError>>
            }),
        );
        assert!(!throttled);
        // A later instant inside the window may spawn again.
        let later = cache.maybe_spawn_passive(
            identity,
            at(60),
            Box::new(move || {
                Box::pin(async { Ok(crate::mutation::MutationApplied::Keep) })
                    as BoxAuthFuture<'static, Result<MutationApplied, MutationError>>
            }),
        );
        assert!(later);
    }

    #[tokio::test]
    async fn passive_refresh_without_mutation_owner_is_skipped() {
        let cache = AwsSourceCache::new();
        let identity = CacheIdentity::LoginSession {
            session: "s".to_owned(),
        };
        let spawned = cache.maybe_spawn_passive(
            identity,
            at(0),
            Box::new(move || {
                Box::pin(async { Ok(MutationApplied::Keep) })
                    as BoxAuthFuture<'static, Result<MutationApplied, MutationError>>
            }),
        );
        assert!(
            !spawned,
            "no mutation owner attached, no background refresh"
        );
    }
}

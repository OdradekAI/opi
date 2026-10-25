//! Keyed access-token cache for the ADC owners (task 20.21.3).
//!
//! The pinned §3.4 contract: tokens from the same source are cached **per
//! key**, where the key covers the identity/config/resource digest and the
//! principal/target, delegates, audience, scopes, project, quota, endpoint,
//! universe, and TLS/mTLS identity of the acquisition. Acquisitions are
//! single-flight per key (concurrent callers share one exchange), the cache
//! honors the real expiry plus **one** 5-minute eager window, a still-valid
//! short-TTL token serves the current call (never a loop that refreshes
//! until "more than 5 minutes" remain), and a refresh that completes after
//! its source was replaced or invalidated does **not** publish (generation
//! fencing). Tokens whose source proves no stable wire principal are
//! `identity-unverified`: an automatic whole-call recovery must not reuse
//! them across a source change (see
//! [`AdcResolvedToken::stable_identity`](crate::auth::adc::native::AdcResolvedToken::stable_identity)).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::Duration;

use time::OffsetDateTime;

use super::native::{AdcResolutionError, AdcResolvedToken};

/// The eager refresh threshold shared by the ADC owners (5 minutes).
pub(crate) const EAGER_REFRESH: Duration = Duration::from_secs(5 * 60);

/// Whether a cached token is outside the eager-refresh window.
pub(crate) fn token_needs_refresh(expires_at: Option<OffsetDateTime>, now: OffsetDateTime) -> bool {
    match expires_at {
        None => true,
        Some(expiry) => now + EAGER_REFRESH >= expiry,
    }
}

/// The pinned cache-key grammar (§3.4): config/resource digest plus the
/// principal/target, delegates, audience, scopes, project, quota, endpoint,
/// universe, and TLS/mTLS identity of the acquisition. Any differing
/// component selects a different cache entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdcCacheKey(u64);

impl AdcCacheKey {
    /// Build one cache key from the pinned key components.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        config: &str,
        principal: Option<&str>,
        delegates: &[String],
        audience: Option<&str>,
        scopes: &[String],
        project: Option<&str>,
        quota: Option<&str>,
        endpoint: Option<&str>,
        universe: Option<&str>,
        tls_identity: Option<&str>,
    ) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        config.hash(&mut hasher);
        principal.hash(&mut hasher);
        delegates.hash(&mut hasher);
        audience.hash(&mut hasher);
        scopes.hash(&mut hasher);
        project.hash(&mut hasher);
        quota.hash(&mut hasher);
        endpoint.hash(&mut hasher);
        universe.hash(&mut hasher);
        tls_identity.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Default)]
struct Slot {
    /// Bumped by [`AdcTokenCache::invalidate`]; a refresh that captured an
    /// older generation never publishes (late-publication fencing).
    generation: u64,
    token: Option<AdcResolvedToken>,
    version: u64,
    outcome: Option<Result<AdcResolvedToken, AdcResolutionError>>,
    /// Per-key single-flight lock, held across one acquisition.
    flight: Option<std::sync::Arc<tokio::sync::Mutex<()>>>,
}

impl Slot {
    fn flight(&mut self) -> std::sync::Arc<tokio::sync::Mutex<()>> {
        self.flight
            .get_or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

/// Per-source keyed access-token cache with single-flight acquisitions,
/// the fixed eager window, short-TTL current-call service, and
/// late-publication fencing. The map lock is never held across an await.
///
/// [`Default`]/[`Debug`]: debug is not implemented because entries hold
/// secret material.
pub struct AdcTokenCache {
    slots: Mutex<HashMap<u64, Slot>>,
}

impl Default for AdcTokenCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AdcTokenCache {
    /// Build an empty cache.
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve the token for `key`: reuse a fresh entry, merge concurrent
    /// callers into one in-flight acquisition, serve a still-valid
    /// short-TTL token when its refresh fails, and refuse to publish a
    /// refresh whose source generation moved while it was in flight. The
    /// slot lock is never held across an await.
    pub async fn resolve<F, Fut>(
        &self,
        key: AdcCacheKey,
        now: OffsetDateTime,
        acquire: F,
    ) -> Result<AdcResolvedToken, AdcResolutionError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<AdcResolvedToken, AdcResolutionError>>,
    {
        let observed = self.version(key);
        self.resolve_observed(key, now, observed, acquire).await
    }

    pub(crate) fn version(&self, key: AdcCacheKey) -> u64 {
        self.slots
            .lock()
            .expect("cache lock")
            .get(&key.0)
            .map_or(0, |slot| slot.version)
    }

    pub(crate) async fn resolve_observed<F, Fut>(
        &self,
        key: AdcCacheKey,
        now: OffsetDateTime,
        observed: u64,
        acquire: F,
    ) -> Result<AdcResolvedToken, AdcResolutionError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<AdcResolvedToken, AdcResolutionError>>,
    {
        let flight = {
            let mut slots = self.slots.lock().expect("cache lock");
            slots.entry(key.0).or_default().flight()
        };
        let _guard = flight.lock().await;

        // Re-read after acquiring the flight: the winner may have published
        // a fresh token while this caller waited, and the generation this
        // acquisition runs under is the one visible at its start.
        let (entry_generation, cached, completed) = {
            let slots = self.slots.lock().expect("cache lock");
            let slot = slots.get(&key.0).expect("slot registered with its flight");
            (
                slot.generation,
                slot.token.clone(),
                (slot.version != observed)
                    .then(|| slot.outcome.clone())
                    .flatten(),
            )
        };
        if let Some(outcome) = completed {
            match outcome {
                Ok(token) if token.expires_at.is_none_or(|expiry| now < expiry) => {
                    return Ok(token);
                }
                Err(error) => return Err(error),
                _ => {}
            }
        }
        if let Some(token) = cached
            .as_ref()
            .filter(|token| !token_needs_refresh(token.expires_at, now))
        {
            return Ok(token.clone());
        }

        let acquired = acquire().await;
        match acquired {
            Ok(token) => {
                let mut slots = self.slots.lock().expect("cache lock");
                if let Some(slot) = slots.get_mut(&key.0)
                    && slot.generation == entry_generation
                {
                    slot.token = Some(token.clone());
                    slot.version += 1;
                    slot.outcome = Some(Ok(token.clone()));
                }
                Ok(token)
            }
            Err(error) => {
                // Short-TTL current-call service: a still-valid cached token
                // serves this call; the failure surfaces only when the
                // cached token is truly expired (or absent).
                let result = if let Some(token) = cached
                    .as_ref()
                    .filter(|token| token.expires_at.is_some_and(|expiry| now < expiry))
                {
                    Ok(token.clone())
                } else {
                    Err(error)
                };
                let mut slots = self.slots.lock().expect("cache lock");
                if let Some(slot) = slots.get_mut(&key.0)
                    && slot.generation == entry_generation
                {
                    slot.version += 1;
                    slot.outcome = Some(result.clone());
                }
                result
            }
        }
    }

    /// Invalidate one key: the cached token is dropped and the generation
    /// is bumped, so a refresh still in flight under the old generation
    /// never publishes.
    pub fn invalidate(&self, key: AdcCacheKey) {
        let mut slots = self.slots.lock().expect("cache lock");
        if let Some(slot) = slots.get_mut(&key.0) {
            slot.generation += 1;
            slot.token = None;
            slot.version += 1;
            slot.outcome = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eager_window_matches_the_five_minute_threshold() {
        let now = OffsetDateTime::from_unix_timestamp(1_000_000).unwrap();
        let fresh = Some(now + time::Duration::seconds(301));
        let stale = Some(now + time::Duration::seconds(299));
        assert!(!token_needs_refresh(fresh, now));
        assert!(token_needs_refresh(stale, now));
        assert!(token_needs_refresh(None, now));
    }
}

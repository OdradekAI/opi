//! Instance resource generation, lease tracking and session cleanup
//! (task 20.17; P20-RESOURCES shared seam).
//!
//! This module owns the resource generation registry consumed by the Codex
//! WebSocket transport family: every call that may use the WebSocket path
//! registers one generation before any wire auth or transform work, and a
//! cleanup cutoff cancels the matching old generations, awaits their local
//! owned exits, and reports unconfirmed effects. The registry never owns
//! protocol state: framing, PMD and Close remain with the single private
//! WebSocket protocol owner (`crate::ws::io`).
//!
//! Semantics pinned by the reviewed source and task 20.17:
//!
//! - `Some(nonempty id)` cancels only work pooled or registered under that
//!   exact session; an unmatched id is a no-op.
//! - `None` cancels everything, including sessionless generations.
//! - `Some("")` is a typed failure with zero effects.
//! - Cleanup failure continues the remaining release and reports the
//!   unconfirmed effects instead of aborting.
//! - Dropping a call's resources starts the release but never claims
//!   barrier completion: a barrier confirms only by observing the exit.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

/// Outcome of one cleanup operation. Counts are confirmed effects; closes
/// that could not be confirmed are reported through
/// [`unconfirmed_effects`](CleanupReport::unconfirmed_effects) while the
/// remaining release still ran.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CleanupReport {
    /// In-flight generations cancelled by the cutoff and confirmed exited.
    pub cancelled_generations: usize,
    /// Pooled connections closed with a confirmed close handshake.
    pub closed_connections: usize,
    /// Sticky per-session WS→SSE fallback markers cleared.
    pub cleared_fallback_sessions: usize,
    /// Human-readable descriptions of effects that could not be confirmed.
    pub unconfirmed_effects: Vec<String>,
}

impl CleanupReport {
    /// Merge another report into this one (component aggregation).
    pub fn merge(&mut self, other: CleanupReport) {
        self.cancelled_generations += other.cancelled_generations;
        self.closed_connections += other.closed_connections;
        self.cleared_fallback_sessions += other.cleared_fallback_sessions;
        self.unconfirmed_effects.extend(other.unconfirmed_effects);
    }
}

/// Typed cleanup failures. The empty session id is rejected before any
/// effect runs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CleanupError {
    /// `Some("")` was passed: session-scoped cleanup requires a non-empty id.
    #[error("session-scoped cleanup requires a non-empty session id")]
    EmptySessionId,
}

/// One awaited cleanup barrier returned by a
/// [`SessionResourceOwner`](trait.SessionResourceOwner). Awaiting it waits
/// for the local owned exits of the cancelled generations and the bounded
/// connection closes; it never waits on a remote acknowledgement without a
/// deadline.
pub struct CleanupHandle {
    inner: Pin<Box<dyn Future<Output = Result<CleanupReport, CleanupError>> + Send>>,
}

impl CleanupHandle {
    /// A handle that resolves immediately to a typed failure.
    pub fn failed(error: CleanupError) -> Self {
        Self {
            inner: Box::pin(async move { Err(error) }),
        }
    }

    /// Wrap one owned cleanup future.
    pub fn from_future<F>(future: F) -> Self
    where
        F: Future<Output = Result<CleanupReport, CleanupError>> + Send + 'static,
    {
        Self {
            inner: Box::pin(future),
        }
    }

    /// Await the barrier and collect its confirmed effects.
    pub async fn resolve(self) -> Result<CleanupReport, CleanupError> {
        self.inner.await
    }
}

/// A provider-owned resource lease surface. Implementors register resource
/// generations for their in-flight work and expose the session cleanup
/// barrier. The default [`crate::provider::Provider::session_resources`]
/// returns `None`, so only providers that actually hold connection
/// resources participate in cleanup.
pub trait SessionResourceOwner: Send + Sync {
    /// Cancel the matching old generations and pooled connections, then
    /// await their local owned exits. `Some("")` must resolve to
    /// [`CleanupError::EmptySessionId`] without effects.
    fn cleanup_session_resources(&self, session: Option<&str>) -> CleanupHandle;
}

/// Lifecycle phase of one registered generation, mirroring the work classes
/// the cleanup cutoff has to cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationPhase {
    /// Registered, before any connection work.
    Prepared,
    /// A WebSocket upgrade is in flight.
    Connecting,
    /// Frames are flowing on a pooled connection.
    Busy,
    /// Frames are flowing on a temporary (sessionless) connection.
    Temporary,
    /// The call fell back and is currently on its SSE leg.
    ActiveFallback,
}

impl GenerationPhase {
    fn as_u8(self) -> u8 {
        match self {
            Self::Prepared => 0,
            Self::Connecting => 1,
            Self::Busy => 2,
            Self::Temporary => 3,
            Self::ActiveFallback => 4,
        }
    }
}

struct Generation {
    session: Option<String>,
    cancel: CancellationToken,
    phase: AtomicU8,
}

#[derive(Default)]
struct RegistryInner {
    generations: HashMap<u64, Generation>,
    next_id: u64,
}

/// Provider-scoped registry of resource generations. Shared through an
/// `Arc` so dropped calls can release their entries from any thread.
#[derive(Default)]
pub(crate) struct ResourceRegistry {
    inner: Mutex<RegistryInner>,
    /// Bumped on every generation removal so cleanup barriers can wait for
    /// local exits without polling.
    revision: tokio::sync::watch::Sender<u64>,
}

/// The per-call lease handed to the transport: dropping it starts the
/// release (the entry token is cancelled and the registry entry removed),
/// but only a [`ResourceRegistry`] cleanup barrier observing the exit
/// claims completion.
pub(crate) struct GenerationGuard {
    registry: Arc<ResourceRegistry>,
    id: u64,
    cancel: CancellationToken,
}

impl GenerationGuard {
    /// The generation's own cancellation token. It fires on cleanup cutoff,
    /// on caller-parent cancellation (via the owner's link), or on drop.
    pub(crate) fn token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Record the current lifecycle phase.
    pub(crate) fn set_phase(&self, phase: GenerationPhase) {
        if let Some(generation) = self
            .registry
            .inner
            .lock()
            .unwrap()
            .generations
            .get_mut(&self.id)
        {
            generation.phase.store(phase.as_u8(), Ordering::Relaxed);
        }
    }
}

impl Drop for GenerationGuard {
    fn drop(&mut self) {
        // Start the release: cancel the token (waking every select arm) and
        // remove the entry. This never marks any cleanup barrier complete;
        // barriers confirm by observing the removal below.
        self.cancel.cancel();
        let mut inner = self.registry.inner.lock().unwrap();
        inner.generations.remove(&self.id);
        drop(inner);
        self.registry.revision.send_modify(|value| *value += 1);
    }
}

impl ResourceRegistry {
    /// Register one generation against an `Arc` registry. Returns the guard
    /// owning the entry and the generation's cancellation token.
    pub(crate) fn register_arc(self: &Arc<Self>, session: Option<String>) -> GenerationGuard {
        let cancel = CancellationToken::new();
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let id = inner.next_id;
        inner.generations.insert(
            id,
            Generation {
                session,
                cancel: cancel.clone(),
                phase: AtomicU8::new(GenerationPhase::Prepared.as_u8()),
            },
        );
        drop(inner);
        GenerationGuard {
            registry: self.clone(),
            id,
            cancel,
        }
    }

    /// Cancel every generation matching the cleanup selector and return
    /// `(cancelled_count, pending_ids)`. The entries stay registered until
    /// their guards drop (the local exit); `None` matches everything,
    /// including sessionless generations.
    pub(crate) fn cleanup(&self, session: Option<&str>) -> (usize, Vec<u64>) {
        let inner = self.inner.lock().unwrap();
        let matched: Vec<u64> = inner
            .generations
            .iter()
            .filter(|(_, generation)| match session {
                Some(target) => generation.session.as_deref() == Some(target),
                None => true,
            })
            .map(|(id, generation)| {
                generation.cancel.cancel();
                *id
            })
            .collect();
        let count = matched.len();
        (count, matched)
    }

    /// Wait until every listed generation has exited (its guard dropped).
    /// This is the awaited local release: no remote acknowledgement is
    /// involved.
    pub(crate) async fn await_exits(self: &Arc<Self>, pending: &[u64]) {
        let mut receiver = self.revision.subscribe();
        loop {
            {
                let inner = self.inner.lock().unwrap();
                if pending.iter().all(|id| !inner.generations.contains_key(id)) {
                    return;
                }
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }
}

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
    /// All selected owners were attempted, but some releases remain unconfirmed.
    #[error("resource cleanup could not confirm every selected release")]
    Incomplete {
        /// Confirmed effects and redacted descriptions of remaining uncertainty.
        report: CleanupReport,
    },
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
    /// Create an independent instance namespace sharing only external transport
    /// configuration. Construction performs no I/O and acquires no resources.
    fn new_namespace(&self) -> Arc<dyn SessionResourceOwner>;

    /// Register a logical call before its first asynchronous preparation step.
    /// Resource-less operations return `None`; the lease retains its generation
    /// across advanced attempts, including attempts that have never been polled.
    fn register_call(&self, _request: &crate::provider::Request) -> Option<CallResourceLease> {
        None
    }
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
    active: usize,
}

#[derive(Default)]
struct RegistryInner {
    generations: HashMap<u64, Generation>,
    next_id: u64,
    release_errors: HashMap<u64, (Option<String>, Vec<String>)>,
}

/// Provider-scoped registry of resource generations. Shared through an
/// `Arc` so dropped calls can release their entries from any thread.
#[derive(Default)]
pub(crate) struct ResourceRegistry {
    inner: Mutex<RegistryInner>,
    /// Bumped on local activity exit and final generation removal so cleanup
    /// barriers can wait without polling or waiting for caller-owned handles.
    revision: tokio::sync::watch::Sender<u64>,
}

/// The per-call lease handed to the transport: dropping it starts the
/// release (the entry token is cancelled and the registry entry removed),
/// but only a [`ResourceRegistry`] cleanup barrier observing the exit
/// claims completion.
#[derive(Clone)]
pub(crate) struct GenerationGuard(Arc<GenerationRegistration>);

struct GenerationRegistration {
    registry: Arc<ResourceRegistry>,
    id: u64,
    cancel: CancellationToken,
}

/// Opaque logical-call resource lease. Its namespace and cancellation are
/// independent of the caller's parent token and of transport cache affinity.
#[derive(Clone)]
pub struct CallResourceLease {
    pub(crate) state: Arc<crate::openai_codex_responses::ws_transport::WsTransportState>,
    pub(crate) guard: GenerationGuard,
}

impl CallResourceLease {
    pub(crate) fn new(
        state: Arc<crate::openai_codex_responses::ws_transport::WsTransportState>,
        request: &crate::provider::Request,
    ) -> Self {
        let guard = state
            .registry()
            .register_arc(request.session_id.clone(), &request.cancel);
        Self { state, guard }
    }

    pub(crate) fn token(&self) -> CancellationToken {
        self.guard.token()
    }

    pub(crate) fn activity(&self) -> Option<GenerationActivity> {
        self.guard.activity()
    }
}

/// A local producer that cleanup must observe exiting. Merely
/// retaining a caller-owned PreparedCall is not active library work.
pub(crate) struct GenerationActivity {
    guard: GenerationGuard,
}

impl Drop for GenerationActivity {
    fn drop(&mut self) {
        let mut inner = self.guard.0.registry.inner.lock().unwrap();
        if let Some(generation) = inner.generations.get_mut(&self.guard.0.id) {
            generation.active -= 1;
        }
        drop(inner);
        self.guard
            .0
            .registry
            .revision
            .send_modify(|value| *value += 1);
    }
}

impl GenerationGuard {
    pub(crate) fn record_unconfirmed(&self, detail: String) {
        let mut inner = self.0.registry.inner.lock().unwrap();
        let session = inner
            .generations
            .get(&self.0.id)
            .and_then(|entry| entry.session.clone());
        inner
            .release_errors
            .entry(self.0.id)
            .or_insert_with(|| (session, Vec::new()))
            .1
            .push(detail);
    }
    /// The generation's own cancellation token. It fires on cleanup cutoff,
    /// on caller-parent cancellation (via the owner's link), or on drop.
    pub(crate) fn token(&self) -> CancellationToken {
        self.0.cancel.clone()
    }

    /// Record the current lifecycle phase.
    pub(crate) fn set_phase(&self, phase: GenerationPhase) {
        if let Some(generation) = self
            .0
            .registry
            .inner
            .lock()
            .unwrap()
            .generations
            .get_mut(&self.0.id)
        {
            generation.phase.store(phase.as_u8(), Ordering::Relaxed);
        }
    }

    /// Serialize publication with the cutoff. Cleanup cancels under this same
    /// lock before draining pooled state, so old work cannot publish afterward.
    pub(crate) fn with_live<T>(&self, f: impl FnOnce() -> T) -> Option<T> {
        let inner = self.0.registry.inner.lock().unwrap();
        let generation = inner.generations.get(&self.0.id)?;
        if generation.cancel.is_cancelled() {
            return None;
        }
        Some(f())
    }

    fn activity(&self) -> Option<GenerationActivity> {
        let mut inner = self.0.registry.inner.lock().unwrap();
        let generation = inner.generations.get_mut(&self.0.id)?;
        if generation.cancel.is_cancelled() {
            return None;
        }
        generation.active += 1;
        Some(GenerationActivity {
            guard: self.clone(),
        })
    }
}

impl Drop for GenerationRegistration {
    fn drop(&mut self) {
        // Start the release: cancel the token (waking every select arm) and
        // remove the entry. Awaited barriers independently observe that the
        // selected producers have exited; retaining a PreparedCall is not work.
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
    pub(crate) fn register_arc(
        self: &Arc<Self>,
        session: Option<String>,
        parent: &CancellationToken,
    ) -> GenerationGuard {
        let cancel = parent.child_token();
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let id = inner.next_id;
        inner.generations.insert(
            id,
            Generation {
                session,
                cancel: cancel.clone(),
                phase: AtomicU8::new(GenerationPhase::Prepared.as_u8()),
                active: 0,
            },
        );
        drop(inner);
        GenerationGuard(Arc::new(GenerationRegistration {
            registry: self.clone(),
            id,
            cancel,
        }))
    }

    /// Cancel every generation matching the cleanup selector and return
    /// `(cancelled_count, pending_ids)`. The entries stay registered until
    /// their guards drop (the local exit); `None` matches everything,
    /// including sessionless generations.
    pub(crate) fn cleanup(&self, session: Option<&str>) -> (usize, Vec<u64>) {
        let (count, pending, ()) = self.cleanup_with(session, || ());
        (count, pending)
    }

    pub(crate) fn cleanup_with<T>(
        &self,
        session: Option<&str>,
        take: impl FnOnce() -> T,
    ) -> (usize, Vec<u64>, T) {
        let inner = self.inner.lock().unwrap();
        let mut matched: Vec<u64> = inner
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
        for (id, (stored_session, _)) in &inner.release_errors {
            if session.is_none_or(|target| stored_session.as_deref() == Some(target))
                && !matched.contains(id)
            {
                matched.push(*id);
            }
        }
        let resources = take();
        (count, matched, resources)
    }

    pub(crate) fn take_unconfirmed(&self, pending: &[u64]) -> Vec<String> {
        let mut inner = self.inner.lock().unwrap();
        pending
            .iter()
            .filter_map(|id| inner.release_errors.remove(id))
            .flat_map(|(_, details)| details)
            .collect()
    }

    /// Wait for the listed generations' local producers to exit. Cancelled
    /// caller-owned prepared/unpolled handles do not keep this barrier pending.
    /// No remote acknowledgement is involved in this producer wait.
    pub(crate) async fn await_exits(self: &Arc<Self>, pending: &[u64]) {
        let mut receiver = self.revision.subscribe();
        loop {
            {
                let inner = self.inner.lock().unwrap();
                if pending.iter().all(|id| {
                    inner
                        .generations
                        .get(id)
                        .is_none_or(|entry| entry.active == 0)
                }) {
                    return;
                }
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }
}

//! Store-owned serialized credential mutations (P20-STORE / task 20.22).
//!
//! Mutations of a shared credential store (login, logout, replace, refresh)
//! are owned by the store, not by the waiter that happened to submit them.
//! A mutation admitted through [`MutationQueue`] receives a generation, runs
//! serialized per provider key, and settles into a non-secret
//! [`MutationReceipt`] — the ACK. Dropping the wait never cancels a started
//! mutation: it completes while the host runtime lives. Worker panics record
//! [`MutationOutcome::Unknown`]; runtime loss may leave a pending operation
//! without an ACK. Neither claims the old state still holds.
//!
//! [`MutationQueue::barrier`] synchronously captures the store-wide cutoff.
//! Any number of [`crate::Models`] instances sharing one store observe the
//! same cutoff because the queue lives in the store. A barrier reports every
//! captured mutation (pending or settled, including failures and unknowns),
//! lists post-cutoff admissions as fenced late writes, never starts auth,
//! and never aborts a running rotation. There are no default wait deadlines,
//! receipt caps, or admission caps: a caller that wants a bound passes one
//! itself, and waiting past it merely stops the wait.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

use futures_util::FutureExt;

use crate::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use crate::provider::ProviderError;

/// Store-wide admission number. Monotonically increasing in submission
/// order; barrier cutoffs are expressed in generations.
pub type MutationGeneration = u64;

/// Per-provider entry revision as observed by this queue. Bumped on every
/// applied write/delete. This is an in-process view of one shared store; it
/// does not track writers outside the queue (cross-process mutation
/// serialization stays the store's own lock policy).
pub type CredentialRevision = u64;

/// What a serialized mutation decided after seeing the current credential.
pub enum MutationDecision {
    /// Commit this credential, replacing the entry (login, replace, a
    /// completed refresh).
    Write(Credential),
    /// Leave the entry unchanged (for example another writer already
    /// committed a fresh token).
    Keep,
    /// Remove the entry (logout).
    Delete,
}

/// Typed failure of one serialized mutation. Failures are recorded in the
/// receipt rather than surfaced as an exceptional result: the ACK always
/// tells the truth about what the store did. Carried behind an [`Arc`] in
/// outcomes because receipts are cloned into reports while store errors are
/// not `Clone`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MutationError {
    /// A store primitive (read, write, delete, or the store's exclusion lock)
    /// failed. No write was applied by this mutation.
    #[error("credential store failed during mutation: {0}")]
    Store(#[from] CredentialStoreError),
    /// The caller's operation failed before deciding a write (for example
    /// the refresh network call). Carries the redacted operation error for
    /// waiter propagation; no half credential is written.
    #[error("mutation operation failed: {0}")]
    Operation(ProviderError),
}

/// Recorded outcome of one serialized mutation. Non-secret: receipts may be
/// retained and reported by barriers.
#[derive(Debug, Clone)]
pub enum MutationOutcome {
    /// A credential was committed; the entry revision advanced.
    Written { revision: CredentialRevision },
    /// The entry was deleted; the entry revision advanced.
    Deleted { revision: CredentialRevision },
    /// The operation chose [`MutationDecision::Keep`]; nothing was written.
    Unchanged { revision: CredentialRevision },
    /// The operation or a store primitive failed; nothing further was
    /// written for this mutation.
    Failed { error: Arc<MutationError> },
    /// The outcome is unknown (the worker was lost mid-mutation). The effect
    /// may or may not have landed; unconfirmed outcomes are never cleared by
    /// another key's success.
    Unknown { detail: String },
}

/// The ACK for one serialized mutation: which provider, which generation,
/// and the recorded outcome.
#[derive(Debug, Clone)]
pub struct MutationReceipt {
    /// Provider key the mutation targeted.
    pub provider_id: String,
    /// Admission generation of this mutation.
    pub generation: MutationGeneration,
    /// Recorded outcome (never a secret).
    pub outcome: MutationOutcome,
}

/// Read-back capability handed to an operation executing inside a serialized
/// mutation slot. Reads run under the slot's exclusion, so a re-read always
/// observes either the pre-mutation state or a mutation serialized before
/// this slot — never a half-write.
pub trait MutationSlotView: Send + Sync {
    /// Re-read the current entry for the slot's provider key.
    fn read_current<'a>(
        &'a self,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>>;
}

/// A read-modify-write callback receiving the current credential and a view
/// for post-failure re-reads. The callback may outlive the ACK waiter.
pub type CredentialUpdateOp = Box<
    dyn for<'a> FnOnce(
            &'a (dyn MutationSlotView + 'a),
            Option<&'a Credential>,
        ) -> BoxAuthFuture<'a, Result<MutationDecision, MutationError>>
        + Send
        + 'static,
>;

/// A store-owned credential mutation. Unconditional replacement/deletion
/// never requires decoding the old entry, so either operation can repair a
/// corrupt entry. Updates strictly read the current credential under the
/// slot's exclusion before invoking the callback.
pub enum CredentialMutationOp {
    /// Replace the entry without reading its previous value.
    Write(Credential),
    /// Delete the entry without reading its previous value.
    Delete,
    /// Read-modify-write callback with a view for post-failure re-reads.
    /// The callback must be `'static` because it may outlive the waiter.
    Update(CredentialUpdateOp),
}

/// One queue slot execution built by a store: runs under the store's own
/// exclusion (cross-process lock where the backing store has one), applies
/// the decided write/delete itself, and reports what it applied.
pub type MutationSlot = Box<
    dyn FnOnce() -> BoxAuthFuture<'static, Result<MutationApplied, MutationError>> + Send + 'static,
>;

/// What a slot actually applied to the backing store. The queue derives the
/// receipt outcome and revision bookkeeping from this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationApplied {
    /// The slot committed a credential.
    Write,
    /// The slot deleted the entry.
    Delete,
    /// The slot decided to keep the current entry unchanged.
    Keep,
}

/// Build an unconditional write mutation (login/replace result).
pub fn write_mutation(cred: Credential) -> CredentialMutationOp {
    CredentialMutationOp::Write(cred)
}

/// Build an unconditional delete mutation (logout).
pub fn delete_mutation() -> CredentialMutationOp {
    CredentialMutationOp::Delete
}

// ---------------------------------------------------------------------------
// MutationQueue — the store-owned state machine
// ---------------------------------------------------------------------------

struct PendingMutation {
    provider_id: String,
    ack: Option<tokio::sync::oneshot::Sender<MutationReceipt>>,
}

struct DeferredMutation {
    prev_tail: Option<MutationGeneration>,
    slot: MutationSlot,
}

struct QueueInner {
    next_generation: MutationGeneration,
    /// Admitted and not yet settled, keyed by generation.
    pending: BTreeMap<MutationGeneration, PendingMutation>,
    /// Admitted slots awaiting handoff to a host Tokio runtime. Owned here,
    /// never by an ACK waiter.
    deferred: BTreeMap<MutationGeneration, DeferredMutation>,
    /// Settled receipts in generation order. Retained for the session; there
    /// is deliberately no default cap.
    receipts: VecDeque<MutationReceipt>,
    /// In-process per-provider revision view.
    revisions: HashMap<String, CredentialRevision>,
    /// Wakers for barrier waits and tail chaining.
    wakers: Vec<Waker>,
}

struct QueueState {
    inner: Mutex<QueueInner>,
}

impl QueueState {
    fn start_deferred(self: &Arc<Self>) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let deferred = std::mem::take(&mut self.inner.lock().unwrap().deferred);
        for (generation, DeferredMutation { prev_tail, slot }) in deferred {
            let state = self.clone();
            runtime.spawn(async move {
                if let Some(prev) = prev_tail {
                    wait_settled(&state, prev).await;
                }
                let settled = match std::panic::AssertUnwindSafe(async move { slot().await })
                    .catch_unwind()
                    .await
                {
                    Ok(result) => MutationSettled::Slot(result),
                    Err(_panic) => MutationSettled::Lost(
                        "mutation worker panicked before recording its outcome",
                    ),
                };
                settle(&state, generation, settled);
            });
        }
    }

    fn wake_all(&self) {
        let wakers = {
            let mut inner = self.inner.lock().unwrap();
            std::mem::take(&mut inner.wakers)
        };
        for waker in wakers {
            waker.wake();
        }
    }
}

/// Settled result of one slot execution: the applied/failed outcome, or a
/// worker loss that becomes an Unknown receipt.
enum MutationSettled {
    Slot(Result<MutationApplied, MutationError>),
    Lost(&'static str),
}

fn settle(state: &QueueState, generation: MutationGeneration, settled: MutationSettled) {
    let mut inner = state.inner.lock().unwrap();
    let Some(PendingMutation { provider_id, ack }) = inner.pending.remove(&generation) else {
        return;
    };
    let outcome = match settled {
        MutationSettled::Slot(Ok(MutationApplied::Write)) => MutationOutcome::Written {
            revision: {
                let revision = inner.revisions.entry(provider_id.clone()).or_insert(0);
                *revision = revision.saturating_add(1);
                *revision
            },
        },
        MutationSettled::Slot(Ok(MutationApplied::Delete)) => MutationOutcome::Deleted {
            revision: {
                let revision = inner.revisions.entry(provider_id.clone()).or_insert(0);
                *revision = revision.saturating_add(1);
                *revision
            },
        },
        MutationSettled::Slot(Ok(MutationApplied::Keep)) => MutationOutcome::Unchanged {
            revision: inner.revisions.get(&provider_id).copied().unwrap_or(0),
        },
        MutationSettled::Slot(Err(error)) => MutationOutcome::Failed {
            error: Arc::new(error),
        },
        MutationSettled::Lost(reason) => MutationOutcome::Unknown {
            // Loss details are fixed strings: panic payloads and runtime
            // states are never echoed because they could carry secrets.
            detail: reason.to_owned(),
        },
    };
    let receipt = MutationReceipt {
        provider_id,
        generation,
        outcome,
    };
    inner.receipts.push_back(receipt.clone());
    drop(inner);
    if let Some(ack) = ack {
        let _ = ack.send(receipt);
    }
    state.wake_all();
}

/// Wait until `generation` has settled (tail chaining and barrier waits).
async fn wait_settled(state: &QueueState, generation: MutationGeneration) {
    std::future::poll_fn(|cx| {
        let mut inner = state.inner.lock().unwrap();
        if !inner.pending.contains_key(&generation) {
            return Poll::Ready(());
        }
        inner.wakers.push(cx.waker().clone());
        Poll::Pending
    })
    .await;
}

/// Store-owned serialized mutation state machine. Shared by every caller of
/// one store (including multiple [`crate::Models`] instances over the same
/// `Arc<dyn CredentialStore>`), so generations, receipts, and barrier
/// cutoffs are store-wide.
///
/// Slots execute detached on the host runtime: admission is synchronous, and
/// dropping the returned [`MutationHandle`] cancels only the wait. If no
/// runtime is available at admission, the queue retains the slot as pending.
/// The next submission or ACK poll inside a Tokio runtime hands retained
/// slots to that runtime in admission order. Without that execution entry,
/// slots remain pending even if their waiters are dropped. Barriers never
/// start slots, and no runtime is created implicitly.
pub struct MutationQueue {
    state: Arc<QueueState>,
}

impl Default for MutationQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MutationQueue {
    /// Create an empty queue. Generations start at 1.
    pub fn new() -> Self {
        Self {
            state: Arc::new(QueueState {
                inner: Mutex::new(QueueInner {
                    next_generation: 1,
                    pending: BTreeMap::new(),
                    deferred: BTreeMap::new(),
                    receipts: VecDeque::new(),
                    revisions: HashMap::new(),
                    wakers: Vec::new(),
                }),
            }),
        }
    }

    /// Admit one serialized mutation for `provider_id`. Synchronous: the
    /// generation is reserved immediately and the slot is ordered after the
    /// provider's current tail (per-provider FIFO, store-wide generation
    /// numbering). The returned handle resolves with the receipt when the
    /// mutation settles.
    pub fn submit(&self, provider_id: &str, slot: MutationSlot) -> MutationHandle {
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
        let provider_id = provider_id.to_owned();
        {
            let mut inner = self.state.inner.lock().unwrap();
            let generation = inner.next_generation;
            inner.next_generation += 1;
            // The previous still-pending admission for this key: per-key
            // FIFO. Settled predecessors need no wait; later admissions for
            // other keys may interleave (cross-key concurrency).
            let prev_tail = inner
                .pending
                .range(..generation)
                .rev()
                .find(|(_, entry)| entry.provider_id == provider_id)
                .map(|(g, _)| *g);
            inner.pending.insert(
                generation,
                PendingMutation {
                    provider_id,
                    ack: Some(ack_tx),
                },
            );
            inner
                .deferred
                .insert(generation, DeferredMutation { prev_tail, slot });
        }
        self.state.start_deferred();
        MutationHandle {
            state: self.state.clone(),
            ack: ack_rx,
        }
    }

    /// Synchronously capture the store-wide cutoff: every mutation admitted
    /// so far (running or queued) is inside the cutoff; admissions after
    /// creation are fenced late writes.
    pub fn barrier(&self) -> MutationBarrier {
        let cutoff = self
            .state
            .inner
            .lock()
            .unwrap()
            .next_generation
            .saturating_sub(1);
        MutationBarrier {
            state: self.state.clone(),
            cutoff,
        }
    }
}

/// Why a wait ended without a receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("mutation worker was lost before the ACK was recorded")]
pub struct MutationAbandoned;

/// Wait-for-ACK handle. Resolves with the mutation receipt. Dropping the
/// handle cancels only the wait — never the mutation.
pub struct MutationHandle {
    state: Arc<QueueState>,
    ack: tokio::sync::oneshot::Receiver<MutationReceipt>,
}

impl Future for MutationHandle {
    type Output = Result<MutationReceipt, MutationAbandoned>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.state.start_deferred();
        self.ack.poll_unpin(cx).map_err(|_| MutationAbandoned)
    }
}

// ---------------------------------------------------------------------------
// Barrier
// ---------------------------------------------------------------------------

/// One entry of a [`BarrierReport`].
#[derive(Debug, Clone)]
pub enum BarrierEntry {
    /// Admitted but not settled at report time (queued or running).
    Pending {
        generation: MutationGeneration,
        provider_id: String,
    },
    /// Settled with a recorded outcome (including failures and unknowns).
    Settled(MutationReceipt),
}

impl BarrierEntry {
    /// Generation of the entry, pending or settled.
    pub fn generation(&self) -> MutationGeneration {
        match self {
            BarrierEntry::Pending { generation, .. } => *generation,
            BarrierEntry::Settled(receipt) => receipt.generation,
        }
    }

    /// Provider key of the entry, pending or settled.
    pub fn provider_id(&self) -> &str {
        match self {
            BarrierEntry::Pending { provider_id, .. } => provider_id,
            BarrierEntry::Settled(receipt) => &receipt.provider_id,
        }
    }
}

/// Synchronous snapshot of every mutation relative to one barrier cutoff.
#[derive(Debug, Clone)]
pub struct BarrierReport {
    /// The captured cutoff generation.
    pub cutoff: MutationGeneration,
    /// Mutations admitted at or before the cutoff, in generation order.
    /// Includes pending, settled, failed, and unknown outcomes.
    pub captured: Vec<BarrierEntry>,
    /// Mutations admitted after this barrier was created (late writes),
    /// observed at report time. Fenced: reported, never aborted.
    pub late: Vec<BarrierEntry>,
}

impl BarrierReport {
    /// Whether every captured mutation has settled (no pending entries at or
    /// before the cutoff).
    pub fn is_complete(&self) -> bool {
        !self
            .captured
            .iter()
            .any(|entry| matches!(entry, BarrierEntry::Pending { .. }))
    }
}

/// A captured cutoff over one shared store's mutation queue. Created
/// synchronously from any caller of the same store (including multiple
/// `Models` instances): everyone observes the same generations.
#[derive(Clone)]
pub struct MutationBarrier {
    state: Arc<QueueState>,
    cutoff: MutationGeneration,
}

impl MutationBarrier {
    /// The captured cutoff generation.
    pub fn cutoff(&self) -> MutationGeneration {
        self.cutoff
    }

    /// Synchronous report. Never starts auth, never aborts a rotation, and
    /// imposes no wait deadline. Unknown outcomes for one key are retained
    /// even after another key succeeds.
    pub fn report(&self) -> BarrierReport {
        let inner = self.state.inner.lock().unwrap();
        let mut captured = Vec::new();
        let mut late = Vec::new();
        for (generation, entry) in inner.pending.iter() {
            let barrier_entry = BarrierEntry::Pending {
                generation: *generation,
                provider_id: entry.provider_id.clone(),
            };
            if *generation <= self.cutoff {
                captured.push(barrier_entry);
            } else {
                late.push(barrier_entry);
            }
        }
        for receipt in inner.receipts.iter() {
            let barrier_entry = BarrierEntry::Settled(receipt.clone());
            if receipt.generation <= self.cutoff {
                captured.push(barrier_entry);
            } else {
                late.push(barrier_entry);
            }
        }
        captured.sort_by_key(BarrierEntry::generation);
        late.sort_by_key(BarrierEntry::generation);
        BarrierReport {
            cutoff: self.cutoff,
            captured,
            late,
        }
    }

    /// Wait until every captured (at-or-before-cutoff) mutation settles.
    /// No default deadline: a caller-provided bound only stops the wait and
    /// reports what is still pending; it never aborts a mutation.
    pub async fn wait(&self) -> BarrierReport {
        std::future::poll_fn(|cx| {
            let mut inner = self.state.inner.lock().unwrap();
            if inner.pending.range(..=self.cutoff).next().is_none() {
                return Poll::Ready(());
            }
            inner.wakers.push(cx.waker().clone());
            Poll::Pending
        })
        .await;
        self.report()
    }
}

// ---------------------------------------------------------------------------
// InMemoryCredentialStore — queue-backed reference store
// ---------------------------------------------------------------------------

struct InMemoryState {
    entries: Mutex<HashMap<String, Credential>>,
    read_count: std::sync::atomic::AtomicU64,
}

struct InMemorySlotView {
    state: Arc<InMemoryState>,
    provider_id: String,
}

impl MutationSlotView for InMemorySlotView {
    fn read_current<'a>(
        &'a self,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move {
            Ok(self
                .state
                .entries
                .lock()
                .unwrap()
                .get(&self.provider_id)
                .cloned())
        })
    }
}

/// Default in-memory [`CredentialStore`] with store-owned serialized
/// mutations: every write path (including the [`CredentialStore::write`] and
/// [`CredentialStore::delete`] primitives) routes through the embedded
/// [`MutationQueue`], so receipts, generations, and barriers observe every
/// mutation. Apps with persistent stores inject their own implementation
/// (the production bridge lives in `opi-coding-agent`); this reference
/// implementation keeps single-process exclusion only.
pub struct InMemoryCredentialStore {
    state: Arc<InMemoryState>,
    queue: Arc<MutationQueue>,
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCredentialStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::with_mutation_queue(Arc::new(MutationQueue::new()))
    }

    /// Create an empty store sharing source rotation receipts with `queue`.
    pub fn with_mutation_queue(queue: Arc<MutationQueue>) -> Self {
        Self {
            state: Arc::new(InMemoryState {
                entries: Mutex::new(HashMap::new()),
                read_count: std::sync::atomic::AtomicU64::new(0),
            }),
            queue,
        }
    }

    /// Number of `read` calls served (test observability).
    pub fn read_count(&self) -> u64 {
        self.state
            .read_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    fn modify_slot(&self, provider_id: &str, op: CredentialMutationOp) -> MutationHandle {
        let state = self.state.clone();
        let provider_id = provider_id.to_owned();
        self.queue.submit(
            provider_id.clone().as_str(),
            Box::new(move || {
                let state = state.clone();
                let provider_id = provider_id.clone();
                Box::pin(async move {
                    let decision = match op {
                        CredentialMutationOp::Write(cred) => MutationDecision::Write(cred),
                        CredentialMutationOp::Delete => MutationDecision::Delete,
                        CredentialMutationOp::Update(op) => {
                            let current = state.entries.lock().unwrap().get(&provider_id).cloned();
                            let view = InMemorySlotView {
                                state: state.clone(),
                                provider_id: provider_id.clone(),
                            };
                            op(&view, current.as_ref()).await?
                        }
                    };
                    match decision {
                        MutationDecision::Write(cred) => {
                            state
                                .entries
                                .lock()
                                .unwrap()
                                .insert(provider_id.clone(), cred);
                            Ok(MutationApplied::Write)
                        }
                        MutationDecision::Keep => Ok(MutationApplied::Keep),
                        MutationDecision::Delete => {
                            state.entries.lock().unwrap().remove(&provider_id);
                            Ok(MutationApplied::Delete)
                        }
                    }
                })
            }),
        )
    }
}

/// Map a receipt from the write/delete primitives to their original result
/// shape: applied outcomes succeed, failures propagate their typed store
/// error, unknown outcomes surface as typed backend errors (never as silent
/// success).
fn receipt_to_result(receipt: MutationReceipt) -> Result<(), CredentialStoreError> {
    let provider = receipt.provider_id.clone();
    match receipt.outcome {
        MutationOutcome::Written { .. }
        | MutationOutcome::Deleted { .. }
        | MutationOutcome::Unchanged { .. } => Ok(()),
        MutationOutcome::Failed {
            error: ref error_arc,
        } => match &**error_arc {
            MutationError::Store(error) => Err(error.clone()),
            MutationError::Operation(_) => Err(CredentialStoreError::Backend {
                provider,
                reason: "mutation operation failed".to_owned(),
            }),
        },
        MutationOutcome::Unknown { detail } => Err(CredentialStoreError::Backend {
            provider,
            reason: format!("mutation outcome unknown: {detail}"),
        }),
    }
}

fn abandoned_error(provider_id: &str) -> CredentialStoreError {
    CredentialStoreError::Backend {
        provider: provider_id.to_owned(),
        reason: "mutation worker lost before ACK".to_owned(),
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move {
            self.state
                .read_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.state.entries.lock().unwrap().get(provider_id).cloned())
        })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        // Eager admission: the mutation is submitted when the method is
        // called, so holding (or dropping) the returned wait never defers or
        // cancels it.
        let handle = self.modify_slot(provider_id, write_mutation(cred.clone()));
        Box::pin(async move {
            let receipt = handle.await.map_err(|_| abandoned_error(provider_id))?;
            receipt_to_result(receipt)
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        let handle = self.modify_slot(provider_id, delete_mutation());
        Box::pin(async move {
            let receipt = handle.await.map_err(|_| abandoned_error(provider_id))?;
            receipt_to_result(receipt)
        })
    }

    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            if self.state.entries.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("in-memory store entry {provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }

    fn modify<'a>(
        &'a self,
        provider_id: &'a str,
        op: CredentialMutationOp,
    ) -> BoxAuthFuture<'a, Result<MutationReceipt, CredentialStoreError>> {
        // Eager admission (synchronous at call time): the returned future is
        // only the wait for the ACK.
        let handle = self.modify_slot(provider_id, op);
        Box::pin(async move { handle.await.map_err(|_| abandoned_error(provider_id)) })
    }

    fn mutation_barrier(&self) -> MutationBarrier {
        self.queue.barrier()
    }

    fn mutation_queue(&self) -> Option<Arc<MutationQueue>> {
        Some(self.queue.clone())
    }
}

// ---------------------------------------------------------------------------
// Default inline modify for simple stores
// ---------------------------------------------------------------------------

struct BorrowingSlotView<'a, S: ?Sized> {
    store: &'a S,
    provider_id: &'a str,
}

impl<S: CredentialStore + ?Sized> MutationSlotView for BorrowingSlotView<'_, S> {
    fn read_current<'a>(
        &'a self,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        self.store.read(self.provider_id)
    }
}

/// Default `CredentialStore::modify`: runs the operation inline against the
/// store primitives inside the caller's future. Not serialized and not
/// detached — stores with shared or persistent state must override
/// `modify`/`mutation_barrier` with a real [`MutationQueue`]. The receipt
/// reports generation/revision 0 (untracked).
pub(crate) fn modify_inline<'a, S: CredentialStore + ?Sized>(
    store: &'a S,
    provider_id: &'a str,
    op: CredentialMutationOp,
) -> BoxAuthFuture<'a, Result<MutationReceipt, CredentialStoreError>> {
    Box::pin(async move {
        let decision = match op {
            CredentialMutationOp::Write(cred) => Ok(MutationDecision::Write(cred)),
            CredentialMutationOp::Delete => Ok(MutationDecision::Delete),
            CredentialMutationOp::Update(op) => {
                let current = store.read(provider_id).await?;
                let view = BorrowingSlotView { store, provider_id };
                op(&view, current.as_ref()).await
            }
        };
        let decision = decision.map_err(|error| match error {
            MutationError::Store(store_error) => store_error,
            MutationError::Operation(_) => CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason: "mutation operation failed".to_owned(),
            },
        })?;
        let outcome = match decision {
            MutationDecision::Write(cred) => {
                store.write(provider_id, &cred).await?;
                MutationOutcome::Written { revision: 0 }
            }
            MutationDecision::Keep => MutationOutcome::Unchanged { revision: 0 },
            MutationDecision::Delete => {
                store.delete(provider_id).await?;
                MutationOutcome::Deleted { revision: 0 }
            }
        };
        Ok(MutationReceipt {
            provider_id: provider_id.to_owned(),
            generation: 0,
            outcome,
        })
    })
}

/// An inert barrier over an empty queue (default
/// `CredentialStore::mutation_barrier`): reports no captured mutations.
/// Stores that own real mutations must override with their queue's barrier.
pub(crate) fn inert_barrier() -> MutationBarrier {
    MutationQueue::new().barrier()
}

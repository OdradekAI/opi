//! Generic agent harness seam (Phase 10, Workstream 10.2).
//!
//! AgentHarness is the generic, product-agnostic orchestration layer above
//! [`crate::Agent`]. It owns the conversation lifecycle concerns that are NOT
//! specific to the coding-agent product:
//!
//! - explicit lifecycle [`Phase`]s (`Idle`, `Turn`, `Compaction`, `BranchSummary`);
//! - structural operations rejected while busy;
//! - queue operations accepted only at documented safe points (idle);
//! - runtime-config mutation that affects *future* turn snapshots, never an
//!   in-flight turn;
//! - a pending-write queue whose flush orders agent-emitted messages *before*
//!   pending extension/session writes;
//! - save points at operation settlement;
//! - `abort` that leaves no active operation and never silently discards an
//!   already-accepted pending write.
//!
//! `Agent` remains the low-level loop + provider + hooks runtime. `AgentHarness`
//! wraps an `Agent` by value and delegates control/cancel/message accessors to
//! it unchanged. The phase transitions are state-machine guards plus
//! snapshot/save-point discipline; `AgentHarness` exposes no prompt/continue/step
//! method and never drives the loop itself.
//!
//! This is a published, contract-tested, unstable-0.x library surface and the
//! generic seam the coding-agent product wrapper (`opi_coding_agent::CodingHarness`)
//! composes over. The product wrapper owns coding-agent policy (built-in file
//! tools, CLI config, context files, package resources, interactive commands,
//! product defaults); this module owns the generic turn lifecycle, phase guards,
//! save points, runtime-config snapshots, and pending-write ordering. Routing
//! the product turn loop *through* `AgentHarness` is a later incremental
//! migration, not a thin adapter: `AgentHarness` takes the `Agent` by
//! value, so product adoption needs either a harness API extension or the
//! session-facade migration tracked in Workstream 10.3 (task 10.5). The Phase 10
//! design's implementation notes explicitly prefer letting existing product
//! behavior continue while generic seams are introduced, so this surface ships
//! ahead of that migration.
//!
//! Branch summaries are a guarded lifecycle phase here, but their durable entry
//! representation is intentionally deferred to the session-facade work (task
//! 10.5), which owns the "branch summaries as context messages, metadata, or
//! both" decision recorded in Workstream 10.3.

use std::path::{Path, PathBuf};

use opi_ai::message::Message;

use crate::session::{
    BranchSummaryEntry, CrashRecovery, ExtensionStateEntry, LabelAction, LabelEntry, MessageEntry,
    ModelChangeEntry, ModelInputSource, SessionEntry, SessionHeader, SessionInfoEntry,
    SessionReader, SessionWriter, ThinkingLevelChangeEntry,
};
use crate::session_event::ThinkingLevel;

/// Lifecycle phase of an AgentHarness.
///
/// A harness is `Idle` between operations. `Turn` covers a single
/// provider/tool cycle; `Compaction` and `BranchSummary` are inter-turn
/// operations and may only begin from `Idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No operation in progress. Queue operations and structural operations are
    /// accepted only in this phase.
    Idle,
    /// An agent turn is in progress.
    Turn,
    /// A context compaction is in progress.
    Compaction,
    /// A branch-summary operation is in progress.
    BranchSummary,
}

/// Errors returned by AgentHarness operations.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    /// A structural or queue operation was attempted while the harness was busy
    /// in the carried phase.
    #[error("operation rejected: harness is busy in phase {0:?}")]
    Busy(Phase),
    /// A session write failed during a flush. Accepted pending writes that could
    /// not be flushed remain queued and are never discarded.
    #[error("session write failed: {0}")]
    Write(#[from] std::io::Error),
    /// AgentHarness cancelled the active operation but its best-effort
    /// flush left the carried number of accepted pending writes unflushed. The
    /// harness is reset to [`Phase::Idle`] and the writes remain queued.
    #[error("abort left {0} accepted pending write(s) unflushed")]
    AbortLeftPending(usize),
}

/// Convenience alias.
pub type HarnessResult<T> = Result<T, HarnessError>;

/// Kind of a queued pending write. Flush order places
/// [`PendingWriteKind::AgentMessage`] writes before
/// [`PendingWriteKind::ExtensionState`] and [`PendingWriteKind::Metadata`]
/// writes so agent-emitted messages persist first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingWriteKind {
    /// An agent-emitted write (message or compaction entry).
    AgentMessage,
    /// A pending extension/session-state write.
    ExtensionState,
    /// A Phase 13 metadata/context entry (`session_info`, `model_change`,
    /// `thinking_level_change`, `label`, `branch_summary`). Parented to the
    /// content tip; does not advance it.
    Metadata,
}

/// A queued durable write awaiting a save-point flush.
#[derive(Debug, Clone)]
pub struct PendingWrite {
    /// Monotonic sequence assigned at enqueue; preserves append order within a
    /// kind bucket even if the queue is reordered for flush.
    pub order: u64,
    /// Flush priority bucket.
    pub kind: PendingWriteKind,
    /// The durable session entry to append.
    pub entry: SessionEntry,
}

/// Ordered, priority-sorted queue of accepted pending writes.
///
/// [`PendingWriteQueue::drain_ordered`] yields [`PendingWriteKind::AgentMessage`]
/// writes before [`PendingWriteKind::ExtensionState`] writes, preserving
/// enqueue order within each bucket.
#[derive(Debug, Default)]
pub struct PendingWriteQueue {
    items: Vec<PendingWrite>,
    counter: u64,
}

impl PendingWriteQueue {
    /// Create an empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the queue holds no pending writes.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of pending writes.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Append a write, returning its assigned monotonic order.
    pub fn enqueue(&mut self, entry: SessionEntry, kind: PendingWriteKind) -> u64 {
        self.counter += 1;
        let order = self.counter;
        self.items.push(PendingWrite { order, kind, entry });
        order
    }

    /// Drain the queue in flush order: agent-emitted writes first, then
    /// extension-state writes, each bucket stable by enqueue order.
    pub fn drain_ordered(&mut self) -> Vec<PendingWrite> {
        let mut items = std::mem::take(&mut self.items);
        items.sort_by_key(|pw| (kind_rank(pw.kind), pw.order));
        items
    }

    /// Re-insert writes that survived a failed flush, preserving their original
    /// order so a later flush re-sorts them identically.
    fn reinsert(&mut self, writes: Vec<PendingWrite>) {
        self.items.extend(writes);
    }
}

fn kind_rank(kind: PendingWriteKind) -> u8 {
    match kind {
        PendingWriteKind::AgentMessage => 0,
        PendingWriteKind::ExtensionState | PendingWriteKind::Metadata => 1,
    }
}

fn active_content_tip(entries: &[SessionEntry]) -> Option<String> {
    crate::session_branch::SessionTree::from_entries(entries)
        .active_tip()
        .map(str::to_owned)
}

/// A save point recorded when the pending-write queue is flushed.
///
/// `pending_before`/`pending_after` describe how many accepted writes were
/// drained and how many remain queued after the flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavePoint {
    /// Monotonic save-point sequence.
    pub seq: u64,
    /// Phase at which the save point was recorded.
    pub at_phase: Phase,
    /// Pending writes present before the flush.
    pub pending_before: usize,
    /// Pending writes remaining after the flush (0 on a fully successful flush).
    pub pending_after: usize,
}

/// Read-only snapshot of harness state.
#[derive(Debug, Clone)]
pub struct HarnessSnapshot {
    /// Current lifecycle phase.
    pub phase: Phase,
    /// Durable entry count reported by the session backend.
    pub message_count: usize,
    /// Accepted pending writes not yet flushed.
    pub pending_writes: usize,
    /// Runtime config visible to the current phase (frozen for an in-flight turn).
    /// Most recent save point, if any.
    pub last_save_point: Option<SavePoint>,
}

/// Durable session backend for an AgentHarness.
///
/// `append` is expected to be durable on success (the JSONL implementation
/// syncs each entry). This trait is the facade boundary that lets product
/// harnesses (e.g. `opi_coding_agent::CodingHarness` in task 10.4) bridge their
/// own session coordinator without forcing `opi-agent` to depend on it, and it
/// lets contract tests substitute recording/failing backends.
pub trait HarnessSession {
    /// Durable-append a session entry.
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()>;
    /// Number of durable entries written so far.
    fn message_count(&self) -> std::io::Result<usize>;
}

/// JSONL-backed [`HarnessSession`] over the v1 [`SessionWriter`].
pub struct JsonlHarnessSession {
    writer: SessionWriter,
    count: usize,
}

impl JsonlHarnessSession {
    /// Create a new JSONL session file with the given header.
    pub fn create(path: &Path, header: SessionHeader) -> std::io::Result<Self> {
        let writer = SessionWriter::create(path, header)?;
        Ok(Self { writer, count: 0 })
    }
}

impl HarnessSession for JsonlHarnessSession {
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        self.writer.append(entry)?;
        self.count += 1;
        Ok(())
    }

    fn message_count(&self) -> std::io::Result<usize> {
        Ok(self.count)
    }
}

// ============================================================================
// Session repo + facade (Workstream 10.3, task 10.5)
// ============================================================================

/// Generic durable session repo: the storage/repo trait boundary above a v1
/// JSONL session file.
///
/// This is the generic session-storage abstraction the coding-agent product
/// composes over. It owns durable [`append`](SessionRepo::append),
/// [`load`](SessionRepo::load), and entry-count semantics. `list`/`fork` and
/// directory policy are intentionally NOT part of this trait: they are
/// coding-agent product policy (CLI flags, the `OPI_SESSIONS_DIR` path,
/// `--list-sessions`/`--resume`/`--fork`/`--delete-session`) or Phase 13
/// session-tree work. A library caller (or contract test) substitutes a
/// recording or failing repo.
pub trait SessionRepo {
    /// Durable-append a session entry.
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()>;
    /// Read the header, all deserializable entries, and crash-recovery
    /// metadata. Unknown future entry types are reported via the
    /// [`CrashRecovery`] rather than fatal (Decision D1).
    fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)>;
    /// Number of durable entries written so far.
    fn message_count(&self) -> std::io::Result<usize>;
}

/// JSONL-backed [`SessionRepo`] over a v1 session file.
pub struct JsonlSessionRepo {
    path: PathBuf,
    writer: SessionWriter,
    count: usize,
}

impl JsonlSessionRepo {
    /// Create a new session file with the given header.
    pub fn create(path: &Path, header: SessionHeader) -> std::io::Result<Self> {
        let writer = SessionWriter::create(path, header)?;
        Ok(Self {
            path: path.to_path_buf(),
            writer,
            count: 0,
        })
    }

    /// Open an existing session file for appending. Counts the deserializable
    /// entries already present so [`message_count`](SessionRepo::message_count)
    /// is accurate on resume.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let count = SessionReader::read_all(path).map(|(_header, entries)| entries.len())?;
        let writer = SessionWriter::open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            writer,
            count,
        })
    }

    /// Path of the backing session file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SessionRepo for JsonlSessionRepo {
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        self.writer.append(entry)?;
        self.count += 1;
        Ok(())
    }

    fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        SessionReader::read_with_recovery(&self.path)
    }

    fn message_count(&self) -> std::io::Result<usize> {
        Ok(self.count)
    }
}

/// Agent-free ordered read/write session facade: the harness session-facade
/// boundary above a [`SessionRepo`].
///
/// `SessionFacade` owns the ordered pending-write queue reused from the
/// generic harness seam ([`PendingWriteQueue`]) and exposes append/load
/// semantics over a [`SessionRepo`] backend. It is the `Agent`-free subset of
/// the session contract: a library caller that needs ordered durable session
/// writes and active-branch reads without driving an Agent loop composes
/// this directly. AgentHarness keeps the phase-guarded turn lifecycle (it
/// wraps an `Agent` by value); `SessionFacade` is the stable session seam
/// Phase 13 extends with richer entry types and context reconstruction.
///
/// # Phase 10 decisions (Workstream 10.3)
///
/// These four deferred decisions are resolved here and left documented for
/// Phase 13:
///
/// - **D1 (additive-v1):** the v1 JSONL accepts additive `SessionEntry`
///   variants. `SessionEntry` is `#[non_exhaustive]` and [`SessionReader`]
///   reports (does not fatal on) lines whose `type` tag it does not recognize,
///   so a v1 file written by a newer opi remains readable. Phase 13 may
///   introduce `version = 2`; v1 files stay readable and resumable without a
///   migration command (spec S9.3). No new entry variants are added in
///   Phase 10 -- the seam is the non-exhaustive enum plus the forward-compatible
///   reader.
/// - **D2 (branch summaries):** a durable branch-summary representation is
///   metadata, not an injected provider-context message. Its representation
///   and any context injection are Phase 13 (spec S9.3 lists `branch_summary`
///   as a Phase 13 v2 target); AgentHarness keeps
///   the lifecycle guard introduced in task 10.3.
/// - **D3 (extension custom messages):** the durable seam persists extension
///   state (including extension-provided messages); whether and how such
///   messages enter provider context is governed by the existing
///   `include_in_llm_context` flag on extension messages and is deferred to
///   Phase 13 context reconstruction. Phase 10 adds no provider-context
///   injection.
/// - **D4 (unfinished-operation recovery):** recovery marks an interrupted
///   operation and returns to idle; it never replays an in-flight provider
///   stream and never retries unsafe tools (e.g. `bash`/`write`) on resume.
///   Provider streams are not resumable. Richer recovery is Phase 13.
pub struct SessionFacade {
    repo: Box<dyn SessionRepo>,
    queue: PendingWriteQueue,
    content_tip_entry_id: Option<String>,
    savepoint_seq: u64,
    last_save_point: Option<SavePoint>,
}

impl SessionFacade {
    /// Create a new facade over the given repo backend.
    pub fn new(repo: Box<dyn SessionRepo>) -> std::io::Result<Self> {
        let (_header, entries, _recovery) = repo.load()?;
        Ok(Self {
            repo,
            queue: PendingWriteQueue::new(),
            content_tip_entry_id: active_content_tip(&entries),
            savepoint_seq: 0,
            last_save_point: None,
        })
    }

    /// Enqueue an agent-emitted message write. Returns the assigned
    /// pending-write order. Always flushes before any extension-state write
    /// enqueued in the same batch.
    pub fn enqueue_message(&mut self, message: Message) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::Message(MessageEntry {
            id: id.clone(),
            parent_id,
            timestamp,
            message,
        });
        self.content_tip_entry_id = Some(id);
        Ok(self.queue.enqueue(entry, PendingWriteKind::AgentMessage))
    }

    /// Enqueue a pending extension/session-state write. Returns the assigned
    /// pending-write order. Always flushes after any agent-emitted message
    /// enqueued in the same batch.
    pub fn enqueue_extension_state(&mut self, state: serde_json::Value) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::ExtensionState(ExtensionStateEntry {
            id,
            parent_id,
            timestamp,
            state,
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::ExtensionState))
    }

    // -- Phase 13.1 metadata/context entries --------------------------------
    //
    // Each metadata append is parented to the current content tip and enqueued
    // as [`PendingWriteKind::Metadata`]. None of them advance
    // `content_tip_entry_id`: labels, session metadata, model/thinking changes,
    // and branch summaries are attachments to the existing content branch, not
    // new conversational turns. Flush order persists agent-emitted messages
    // before metadata writes in the same batch.

    /// Record a session rename (`session_info` entry) parented to the content
    /// tip without advancing it.
    pub fn enqueue_session_info(&mut self, name: String) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::SessionInfo(SessionInfoEntry {
            id,
            parent_id,
            timestamp,
            name,
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::Metadata))
    }

    /// Record a provider/model change (`model_change` entry) parented to the
    /// content tip without advancing it.
    pub fn enqueue_model_change(&mut self, model: String) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        // Infer the input source from the recorded string: a canonical
        // `provider:model` spec carries a provider separator, a bare model does
        // not. Bare-input normalization itself is owned by the Reference
        // Product; the facade only records the truthful source of what it was
        // handed.
        let input_source = if model.contains(':') {
            ModelInputSource::Canonical
        } else {
            ModelInputSource::BareNormalized
        };
        let entry = SessionEntry::ModelChange(ModelChangeEntry {
            id,
            parent_id,
            timestamp,
            model,
            input_source: Some(input_source),
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::Metadata))
    }

    /// Record a thinking/reasoning level change (`thinking_level_change`
    /// entry) parented to the content tip without advancing it.
    pub fn enqueue_thinking_level_change(&mut self, level: ThinkingLevel) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            id,
            parent_id,
            timestamp,
            level,
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::Metadata))
    }

    /// Add or remove a label (`label` entry) parented to the content tip
    /// without advancing it. Labels are UI-visible and never enter LLM context.
    pub fn enqueue_label(&mut self, label: String, action: LabelAction) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::Label(LabelEntry {
            id,
            parent_id,
            timestamp,
            label,
            action,
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::Metadata))
    }

    /// Record a branch summary (`branch_summary` entry) parented to the content
    /// tip without advancing it. Whether it enters provider LLM context is
    /// decided by the Phase 13 context builder (task 13.2).
    pub fn enqueue_branch_summary(&mut self, summary: String) -> HarnessResult<u64> {
        let id = self.next_id();
        let parent_id = self.content_tip_entry_id.clone();
        let timestamp = self.next_timestamp();
        let entry = SessionEntry::BranchSummary(BranchSummaryEntry {
            id,
            parent_id,
            timestamp,
            summary,
        });
        Ok(self.queue.enqueue(entry, PendingWriteKind::Metadata))
    }

    /// Flush the pending-write queue at a save point. Agent-emitted messages
    /// are appended before extension-state writes. Returns the settlement
    /// save point.
    pub fn flush(&mut self) -> HarnessResult<SavePoint> {
        let pending_before = self.queue.len();
        self.flush_internal()?;
        let pending_after = self.queue.len();
        Ok(self.record_save_point(pending_before, pending_after))
    }

    /// Abort: best-effort flush ALL accepted pending writes. Never silently
    /// discards an accepted write. On a flush failure, returns
    /// [`HarnessError::AbortLeftPending`] with the number of unflushed writes;
    /// the writes remain queued.
    pub fn abort(&mut self) -> HarnessResult<()> {
        let outcome = self.flush_internal();
        match outcome {
            Ok(()) => Ok(()),
            Err(_) => Err(HarnessError::AbortLeftPending(self.queue.len())),
        }
    }

    /// Read the header, all deserializable entries, and crash-recovery
    /// metadata from the backing repo.
    pub fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        self.repo.load()
    }

    /// Reconstruct the active branch tip from durable entries. Deterministic:
    /// the last `Leaf` entry's `entry_id` when it resolves to a known entry,
    /// otherwise the trunk tip.
    pub fn active_tip(&self) -> std::io::Result<Option<String>> {
        let (_header, entries, _recovery) = self.repo.load()?;
        Ok(crate::session_branch::SessionTree::from_entries(&entries)
            .active_tip()
            .map(str::to_owned))
    }

    /// Accepted pending writes not yet flushed.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Durable entry count reported by the backing repo.
    pub fn message_count(&self) -> std::io::Result<usize> {
        self.repo.message_count()
    }

    /// Most recent save point, if any.
    pub fn last_save_point(&self) -> Option<SavePoint> {
        self.last_save_point
    }

    // -- Internal helpers ---------------------------------------------------

    /// Drain the queue in order and append each entry. On a write failure, the
    /// unflushed tail is re-queued (order preserved) and the error is
    /// returned; the queue is never silently emptied across a failure.
    fn flush_internal(&mut self) -> Result<(), std::io::Error> {
        let mut ordered = self.queue.drain_ordered();
        let mut idx = 0;
        while idx < ordered.len() {
            if let Err(e) = self.repo.append(&ordered[idx].entry) {
                let tail: Vec<_> = ordered.drain(idx..).collect();
                self.queue.reinsert(tail);
                return Err(e);
            }
            idx += 1;
        }
        Ok(())
    }

    fn record_save_point(&mut self, pending_before: usize, pending_after: usize) -> SavePoint {
        self.savepoint_seq += 1;
        let sp = SavePoint {
            seq: self.savepoint_seq,
            // The facade has no phase machine (that lives in `AgentHarness`);
            // every facade save point is recorded at idle.
            at_phase: Phase::Idle,
            pending_before,
            pending_after,
        };
        self.last_save_point = Some(sp);
        sp
    }

    fn next_id(&mut self) -> String {
        format!("entry-{}", uuid::Uuid::now_v7())
    }

    fn next_timestamp(&self) -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_else(|_| "0".to_string())
    }
}

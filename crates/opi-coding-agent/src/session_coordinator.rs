//! Session lifecycle coordinator bridging harness, session writer,
//! compaction engine, and usage accumulation.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use opi_agent::compaction::{CompactionConfig, CompactionEngine, DefaultCompactionHooks, Entry};
use opi_agent::message::{AgentMessage, CompactionSummaryMessage};
use opi_agent::session::{
    CompactionEntry, ExtensionStateEntry, LabelAction, LabelEntry, LeafEntry, MessageEntry,
    ModelChangeEntry, SessionEntry, SessionHeader, SessionInfoEntry, SessionWriter,
    ThinkingLevelChangeEntry,
};
use opi_agent::session_event::{CompactionReason, CompactionResult, ThinkingLevel};
use opi_ai::message::{AssistantContent, Message, ToolResultMessage};
use opi_ai::stream::{CumulativeUsage, Usage};

use crate::pricing::lookup_pricing;

static ENTRY_SEQ: AtomicU64 = AtomicU64::new(1);

/// Result of a compaction triggered during `on_turn_end`.
///
/// The harness uses these fields to (a) install `[summary, ...kept]` as the
/// new Agent message buffer, (b) emit `CompactionStart`/`End` events, and
/// (c) propagate token-before/after on the session event protocol.
pub struct CompactionResultOutput {
    pub summary: CompactionSummaryMessage,
    /// New Agent message buffer to install: `[summary, ...kept_messages]`.
    pub new_agent_messages: Vec<AgentMessage>,
    pub reason: CompactionReason,
    pub tokens_before: u64,
    pub tokens_after: u64,
    pub first_kept_entry_id: String,
    pub diagnostic: opi_agent::Diagnostic,
}

pub struct SessionCoordinator {
    writer: SessionWriter,
    compaction: CompactionEngine,
    usage: CumulativeUsage,
    session_id: String,
    session_path: PathBuf,
    model: String,
    /// Entries accumulated so far, used as compaction input.
    /// Indexed in parallel with `agent_message_indices`.
    entries: Vec<Entry>,
    /// For each `entries[i]`, the index into the Agent's internal message
    /// buffer when that entry was appended. Used to compute which Agent
    /// messages survive a compaction.
    agent_message_indices: Vec<usize>,
    /// Running count of how many messages the Agent has accumulated.
    /// Updated on every `on_turn_end` call.
    agent_message_count: usize,
    /// Cumulative token count at the last compaction. The threshold check
    /// uses `current_total - watermark` so compaction doesn't re-trigger
    /// every turn after the threshold is crossed once.
    compaction_watermark_tokens: u64,
    /// Last persisted Message/Compaction entry on the active branch.
    active_tip_entry_id: Option<String>,
    /// Latest `session_info` name on the active branch (Phase 13.4). UI-visible
    /// metadata; never enters provider context. Seeded by `open_existing` and
    /// updated by `append_session_info`.
    name: Option<String>,
    /// Active label set on the active branch (Phase 13.4): `Add`/`Remove`
    /// applied in append order, deduplicated, preserving first-`Add` order.
    /// UI-visible metadata; never enters provider context. Seeded by
    /// `open_existing` and updated by `append_label`.
    labels: Vec<String>,
}

impl SessionCoordinator {
    pub fn new(
        dir: &Path,
        cwd: &str,
        compaction_config: CompactionConfig,
        model: impl Into<String>,
    ) -> std::io::Result<Self> {
        let id = generate_session_id();
        let timestamp = now_iso();
        let header = SessionHeader::new(id.clone(), timestamp, cwd.into(), None);
        let path = dir.join(format!("{id}.jsonl"));
        std::fs::create_dir_all(dir)?;
        let writer = SessionWriter::create(&path, header)?;
        Ok(Self {
            writer,
            compaction: CompactionEngine::new(compaction_config),
            usage: CumulativeUsage::default(),
            session_id: id,
            session_path: path,
            model: model.into(),
            entries: Vec::new(),
            agent_message_indices: Vec::new(),
            agent_message_count: 0,
            compaction_watermark_tokens: 0,
            active_tip_entry_id: None,
            name: None,
            labels: Vec::new(),
        })
    }

    /// Open an existing session file for appending (resume).
    ///
    /// `existing_entries` are the prior session entries already loaded by the
    /// caller via `SessionReader::read_all`. Only entries on the active branch
    /// (determined by the last `Leaf` pointer) are replayed into the internal
    /// compaction buffer — matching the ordering used by `reconstruct_context`
    /// for the Agent's message buffer. Legacy sessions without Leaf entries
    /// fall back to file-order replay.
    ///
    /// Compaction entries are honored by replaying their semantics: the
    /// kept tail (entries from `first_kept_entry_id` onward, persisted before
    /// the marker) is preserved, the summary occupies a synthetic slot at
    /// index 0 of the post-compaction agent buffer, and indices are rebuilt
    /// to match the runtime layout `[summary, ...kept_tail, ...post_marker]`.
    /// If `first_kept_entry_id` cannot be located among the already-replayed
    /// entries (corrupt or forward-incompatible session), the buffer is
    /// reset entirely — matching the legacy defensive behavior.
    pub fn open_existing(
        path: PathBuf,
        session_id: String,
        existing_entries: &[SessionEntry],
        prior_agent_message_count: usize,
        compaction_config: CompactionConfig,
        model: impl Into<String>,
    ) -> std::io::Result<Self> {
        let writer = SessionWriter::open(&path)?;

        // Advance the global sequence counter past any existing IDs.
        advance_seq_from_entries(existing_entries);

        // Replay entries through the opi-agent active-chain selector so the
        // coordinator's compaction buffer stays aligned with the Agent's
        // reconstructed message buffer, including degraded Leaf fallback.
        let ordered = crate::session_cli::select_ordered_entries(existing_entries);
        let active_tip_entry_id = ordered
            .iter()
            .rev()
            .find_map(|entry| content_entry_id(entry).map(ToOwned::to_owned));

        let mut entries: Vec<Entry> = Vec::new();
        let mut indices: Vec<usize> = Vec::new();
        let mut agent_idx: usize = 0;
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_cache_read: u64 = 0;
        let mut total_cache_write: u64 = 0;
        let mut total_cache_write_1h: Option<u64> = None;
        let mut total_reasoning: Option<u64> = None;
        let mut unknown_turns: u32 = 0;
        // Count turns as user messages — each user prompt drives exactly one
        // on_turn_end call. Counting assistant messages would overcount because
        // a single user turn can produce multiple assistant messages (tool call
        // + final response).
        let mut user_count: u32 = 0;
        let mut saw_user_turn = false;
        let mut turn_has_unknown_usage = false;

        for entry in ordered {
            match entry {
                SessionEntry::Message(m) => {
                    // Accumulate usage from persisted assistant messages and
                    // count turns by user messages.
                    match &m.message {
                        Message::Assistant(a) => {
                            total_input += a.usage.input_tokens as u64;
                            total_output += a.usage.output_tokens as u64;
                            total_cache_read += a.usage.cache_read_tokens as u64;
                            total_cache_write += a.usage.cache_write_tokens as u64;
                            if let Some(tokens) = a.usage.cache_write_1h_tokens {
                                total_cache_write_1h =
                                    Some(total_cache_write_1h.unwrap_or(0).saturating_add(tokens));
                            }
                            if let Some(tokens) = a.usage.reasoning_tokens {
                                total_reasoning =
                                    Some(total_reasoning.unwrap_or(0).saturating_add(tokens));
                            }
                            let usage_is_known =
                                a.usage.is_reported() || a.usage.total_tokens() > 0;
                            if !usage_is_known {
                                if saw_user_turn {
                                    turn_has_unknown_usage = true;
                                } else {
                                    unknown_turns += 1;
                                }
                            }
                        }
                        Message::User(_) => {
                            if saw_user_turn && turn_has_unknown_usage {
                                unknown_turns += 1;
                            }
                            user_count += 1;
                            saw_user_turn = true;
                            turn_has_unknown_usage = false;
                        }
                        _ => {}
                    }
                    entries.push(Entry {
                        id: m.id.clone(),
                        message: AgentMessage::Llm(m.message.clone()),
                    });
                    indices.push(agent_idx);
                    agent_idx += 1;
                }
                SessionEntry::Compaction(c) => {
                    let kept_start = entries.iter().position(|e| e.id == c.first_kept_entry_id);
                    let kept: Vec<Entry> = match kept_start {
                        Some(idx) => entries.split_off(idx),
                        None => Vec::new(),
                    };
                    let kept_count = kept.len();
                    // Rebuild entries with the compaction summary at index 0,
                    // followed by the kept tail. This mirrors the runtime
                    // compaction layout so a subsequent compaction sees the
                    // full context including prior summaries.
                    let summary_entry = Entry {
                        id: format!("sum-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
                        message: AgentMessage::CompactionSummary(CompactionSummaryMessage {
                            summary: c.summary.clone(),
                            first_kept_entry_id: c.first_kept_entry_id.clone(),
                            tokens_before: c.tokens_before,
                            tokens_after: c.tokens_after,
                        }),
                    };
                    let mut rebuilt = Vec::with_capacity(1 + kept_count);
                    rebuilt.push(summary_entry);
                    rebuilt.extend(kept);
                    entries = rebuilt;
                    indices = (0..=kept_count).collect();
                    agent_idx = 1 + kept_count;
                }
                SessionEntry::Leaf(_) => {}
                _ => {}
            }
        }

        if saw_user_turn && turn_has_unknown_usage {
            unknown_turns += 1;
        }

        let usage = CumulativeUsage::from_totals(
            total_input,
            total_output,
            total_cache_read,
            total_cache_write,
            total_cache_write_1h,
            total_reasoning,
            user_count,
            unknown_turns,
        );
        let watermark = usage.as_usage().total_tokens();

        // Phase 13.4: seed the live name/labels view from the persisted
        // active-chain metadata so `/session info` and RPC `session_info`
        // reflect prior writes without a full reconstruct pass. `ordered`
        // is used for the compaction buffer, so we scan the raw entries
        // directly, filtering to the full active entry chain (or accepting
        // rootless metadata when no content exists yet). Mirrors the
        // `reconstruct_context` derivation and the
        // `latest_extension_state_entry_for_active_branch` filter pattern.
        let (name, labels) = seed_metadata_from_entries(existing_entries);

        Ok(Self {
            writer,
            compaction: CompactionEngine::new(compaction_config),
            usage,
            session_id,
            session_path: path,
            model: model.into(),
            entries,
            agent_message_indices: indices,
            agent_message_count: prior_agent_message_count,
            compaction_watermark_tokens: watermark,
            active_tip_entry_id,
            name,
            labels,
        })
    }

    /// Persist only the new messages from a completed turn.
    ///
    /// `new_messages` should contain only the messages produced during this
    /// turn (not the full conversation history). The caller is responsible for
    /// slicing appropriately. `turn_start_agent_index` is the index in the
    /// Agent's full message buffer where `new_messages[0]` lives.
    ///
    /// Returns `Ok(Some(CompactionReason))` if compaction should be triggered
    /// (the caller should emit `CompactionStart`, then call
    /// `execute_compaction`). Returns `Ok(None)` if no compaction is needed.
    /// Returns `Err` if a session write failed.
    pub fn on_turn_end(
        &mut self,
        new_messages: &[AgentMessage],
        usage: &Usage,
        turn_start_agent_index: usize,
    ) -> Result<Option<CompactionReason>, std::io::Error> {
        self.ensure_session_path_available()?;
        self.usage.accumulate(usage);

        let mut agent_idx = turn_start_agent_index;
        let mut parent_id = self.active_tip_entry_id.clone();
        let mut last_persisted_entry_id = None;
        for msg in new_messages {
            if let AgentMessage::Llm(m) = msg {
                let persisted_message = message_for_session(m);
                let entry_id = format!("msg-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed));
                let entry = SessionEntry::Message(MessageEntry {
                    id: entry_id.clone(),
                    parent_id: parent_id.clone(),
                    timestamp: now_iso(),
                    message: persisted_message,
                });
                self.append_entry(&entry)?;
                self.entries.push(Entry {
                    id: entry_id.clone(),
                    message: msg.clone(),
                });
                self.agent_message_indices.push(agent_idx);
                parent_id = Some(entry_id.clone());
                last_persisted_entry_id = Some(entry_id);
            }
            agent_idx += 1;
        }
        self.agent_message_count = agent_idx;
        if let Some(tip) = last_persisted_entry_id {
            self.active_tip_entry_id = Some(tip.clone());
            self.append_leaf_for_tip(&tip)?;
        }

        // Check threshold-based compaction after each turn.
        // Use tokens accumulated since the last compaction (watermark) so
        // compaction doesn't re-trigger every turn after the first crossing.
        let total_tokens = self.usage.as_usage().total_tokens();
        let delta = total_tokens.saturating_sub(self.compaction_watermark_tokens);
        if self
            .compaction
            .should_compact(delta, CompactionReason::Threshold)
        {
            Ok(Some(CompactionReason::Threshold))
        } else {
            Ok(None)
        }
    }

    /// Execute compaction after `on_turn_end` returned `Some(reason)`.
    /// The caller should emit `CompactionStart` before calling this and
    /// `CompactionEnd` afterwards.
    ///
    /// Returns `Err` if the compaction marker could not be persisted — in this
    /// case the in-memory state is left unchanged (no buffer replacement, no
    /// watermark advance) so the session file stays consistent with the
    /// runtime.
    pub fn execute_compaction(
        &mut self,
        reason: CompactionReason,
    ) -> Result<Option<CompactionResultOutput>, std::io::Error> {
        self.run_compaction(reason)
    }

    /// Backwards-compatible variant used by tests that don't track Agent indices.
    /// Assumes `new_messages` are appended starting at the current message count.
    /// Runs compaction inline if needed (no separate event emission).
    pub fn on_turn_end_simple(
        &mut self,
        new_messages: &[AgentMessage],
        usage: &Usage,
    ) -> Result<Option<CompactionResultOutput>, std::io::Error> {
        let start = self.agent_message_count;
        let reason = self.on_turn_end(new_messages, usage, start)?;
        match reason {
            Some(r) => self.execute_compaction(r),
            None => Ok(None),
        }
    }

    fn run_compaction(
        &mut self,
        reason: CompactionReason,
    ) -> Result<Option<CompactionResultOutput>, std::io::Error> {
        let hooks = DefaultCompactionHooks;
        match self.compaction.compact(&self.entries, reason, &hooks) {
            Ok(output) => {
                let diagnostic = output.diagnostic();
                let split = self.entries.len() - output.kept_entries.len();
                let kept_indices: Vec<usize> = self
                    .agent_message_indices
                    .iter()
                    .skip(split)
                    .copied()
                    .collect();
                let kept_messages: Vec<AgentMessage> = output
                    .kept_entries
                    .iter()
                    .map(|e| e.message.clone())
                    .collect();

                let summary = CompactionSummaryMessage {
                    summary: output.summary_text.clone(),
                    first_kept_entry_id: output.first_kept_entry_id.clone(),
                    tokens_before: output.tokens_before,
                    tokens_after: output.tokens_after,
                };

                let compaction_id = format!("cmp-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed));
                let compaction_entry = SessionEntry::Compaction(CompactionEntry {
                    id: compaction_id.clone(),
                    parent_id: self.active_tip_entry_id.clone(),
                    timestamp: now_iso(),
                    summary: output.summary_text.clone(),
                    first_kept_entry_id: output.first_kept_entry_id.clone(),
                    tokens_before: output.tokens_before,
                    tokens_after: output.tokens_after,
                });

                // Persist the compaction marker BEFORE mutating in-memory state.
                // If this fails, the runtime context remains un-compacted so
                // the session file and memory stay consistent.
                self.append_entry(&compaction_entry)?;
                self.append_leaf_for_tip(&compaction_id)?;

                // Reset internal entries to [summary, ...kept]. The summary
                // must be included so that a subsequent compaction can see the
                // full context including earlier compaction summaries.
                let mut new_entries = Vec::with_capacity(1 + output.kept_entries.len());
                new_entries.push(Entry {
                    id: format!("sum-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
                    message: AgentMessage::CompactionSummary(summary.clone()),
                });
                new_entries.extend(output.kept_entries);
                self.entries = new_entries;
                self.agent_message_indices = (0..=kept_indices.len()).collect();
                self.agent_message_count = 1 + kept_messages.len();

                // Advance the watermark so the next threshold check measures
                // tokens accumulated from this point forward.
                self.compaction_watermark_tokens = self.usage.as_usage().total_tokens();
                self.active_tip_entry_id = Some(compaction_id);

                // Build the new Agent buffer: [summary, ...kept].
                let mut new_agent_messages = Vec::with_capacity(1 + kept_messages.len());
                new_agent_messages.push(AgentMessage::CompactionSummary(summary.clone()));
                new_agent_messages.extend(kept_messages);
                Ok(Some(CompactionResultOutput {
                    summary,
                    new_agent_messages,
                    reason: output.reason,
                    tokens_before: output.tokens_before,
                    tokens_after: output.tokens_after,
                    first_kept_entry_id: output.first_kept_entry_id,
                    diagnostic,
                }))
            }
            Err(_) => {
                // Nothing to compact (too few entries) — no-op.
                Ok(None)
            }
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session_path(&self) -> &Path {
        &self.session_path
    }

    pub fn usage(&self) -> &CumulativeUsage {
        &self.usage
    }

    pub fn compaction_engine(&self) -> &CompactionEngine {
        &self.compaction
    }

    /// Read-only view of the entries currently tracked for compaction.
    /// Exposed for tests that need to assert resume correctness.
    pub fn compaction_entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Append a Leaf pointer marking the selected active branch tip.
    pub fn append_leaf(&mut self, entry_id: &str) -> Result<(), std::io::Error> {
        self.active_tip_entry_id = Some(entry_id.to_owned());
        self.append_leaf_for_tip(entry_id)
    }

    pub fn append_extension_state(
        &mut self,
        state: serde_json::Value,
    ) -> Result<(), std::io::Error> {
        let entry = SessionEntry::ExtensionState(ExtensionStateEntry {
            id: format!("state-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.active_tip_entry_id.clone(),
            timestamp: now_iso(),
            state,
        });
        self.append_entry(&entry)
    }

    /// Record a `model_change` entry parented to the current content tip
    /// **without** advancing it (Phase 13.3). Idle `set_model_validated` calls
    /// go through here so a later resume can observe the recorded model on the
    /// active branch. No `Leaf` is appended and `active_tip_entry_id` is
    /// unchanged: model changes are metadata attachments, not new
    /// conversational turns.
    pub fn append_model_change(&mut self, model: String) -> Result<(), std::io::Error> {
        let entry = SessionEntry::ModelChange(ModelChangeEntry {
            id: format!("model-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.active_tip_entry_id.clone(),
            timestamp: now_iso(),
            model,
        });
        self.append_entry(&entry)
    }

    /// Record a `thinking_level_change` entry parented to the current content
    /// tip **without** advancing it (Phase 13.3). Idle `set_thinking_level`
    /// calls go through here so a later resume can observe the recorded
    /// thinking level on the active branch. No `Leaf` is appended and
    /// `active_tip_entry_id` is unchanged.
    pub fn append_thinking_level_change(
        &mut self,
        level: ThinkingLevel,
    ) -> Result<(), std::io::Error> {
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            id: format!("thinking-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.active_tip_entry_id.clone(),
            timestamp: now_iso(),
            level,
        });
        self.append_entry(&entry)
    }

    /// Record a `session_info` entry (session name) parented to the current
    /// content tip **without** advancing it (Phase 13.4). Idle `/name <name>`
    /// goes through here so a later resume observes the latest name on the
    /// active branch. No `Leaf` is appended and `active_tip_entry_id` is
    /// unchanged: session names are UI-visible metadata, not conversational
    /// turns, and never enter provider context.
    pub fn append_session_info(&mut self, name: String) -> Result<(), std::io::Error> {
        let entry = SessionEntry::SessionInfo(SessionInfoEntry {
            id: format!("info-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.active_tip_entry_id.clone(),
            timestamp: now_iso(),
            name: name.clone(),
        });
        self.append_entry(&entry)?;
        self.name = Some(name);
        Ok(())
    }

    /// Record a `label` entry (add or remove) parented to the current content
    /// tip **without** advancing it (Phase 13.4). Idle `/label <label>` and
    /// `/unlabel <label>` go through here so a later resume observes the
    /// active label set on the active branch. The in-memory label set applies
    /// `Add`/`Remove` in append order with first-`Add` deduplication; the
    /// persisted entry records the raw action so reconstruct stays truthful.
    /// No `Leaf` is appended and `active_tip_entry_id` is unchanged. Labels
    /// are UI-visible metadata and never enter provider context.
    pub fn append_label(
        &mut self,
        label: String,
        action: LabelAction,
    ) -> Result<(), std::io::Error> {
        let entry = SessionEntry::Label(LabelEntry {
            id: format!("label-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.active_tip_entry_id.clone(),
            timestamp: now_iso(),
            label: label.clone(),
            action,
        });
        self.append_entry(&entry)?;
        apply_label(&mut self.labels, &label, action);
        Ok(())
    }

    /// Latest session name on the active branch, if any (Phase 13.4 read path).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Active label set on the active branch (Phase 13.4 read path):
    /// `Add`/`Remove` applied in append order, deduplicated, first-`Add` order.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Entry id at the tip of the active branch (Phase 13.4 read path). The
    /// last persisted Message/Compaction entry on the active branch.
    pub fn active_branch_id(&self) -> Option<&str> {
        self.active_tip_entry_id.as_deref()
    }

    fn append_leaf_for_tip(&mut self, entry_id: &str) -> Result<(), std::io::Error> {
        let entry = SessionEntry::Leaf(LeafEntry {
            id: format!("leaf-{}", ENTRY_SEQ.fetch_add(1, Ordering::Relaxed)),
            parent_id: Some(entry_id.to_owned()),
            timestamp: now_iso(),
            entry_id: entry_id.to_owned(),
        });
        self.append_entry(&entry)
    }

    /// Compute the cost summary from the accumulated usage and the model
    /// pricing table. Returns `None` if any turn has unknown usage or if no
    /// pricing is known for the model.
    pub fn cost_summary(&self) -> Option<opi_ai::stream::CostBreakdown> {
        if self.usage.has_unknown_usage() {
            return None;
        }
        let pricing = lookup_pricing(&self.model)?;
        Some(opi_ai::stream::calculate_cost(
            &self.usage.as_usage(),
            &pricing,
        ))
    }

    fn append_entry(&mut self, entry: &SessionEntry) -> Result<(), std::io::Error> {
        self.ensure_session_path_available()?;
        self.writer.append(entry)
    }

    fn ensure_session_path_available(&self) -> Result<(), std::io::Error> {
        if self.session_path.is_file() {
            return Ok(());
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session file missing: {}", self.session_path.display()),
        ))
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

fn message_for_session(message: &Message) -> Message {
    match message {
        Message::Assistant(assistant) => {
            let mut persisted = assistant.clone();
            for content in &mut persisted.content {
                if let AssistantContent::ToolCall { tool_call } = content {
                    tool_call.arguments = tool_arguments_for_session(&tool_call.arguments);
                }
            }
            Message::Assistant(persisted)
        }
        Message::ToolResult(tool_result) => Message::ToolResult(ToolResultMessage {
            tool_call_id: tool_result.tool_call_id.clone(),
            tool_name: tool_result.tool_name.clone(),
            content: tool_result.content.clone(),
            details: tool_result
                .details
                .as_ref()
                .map(opi_agent::diagnostic::redact_public_value),
            is_error: tool_result.is_error,
            truncated: tool_result.truncated,
            timestamp_ms: tool_result.timestamp_ms,
        }),
        other => other.clone(),
    }
}

fn tool_arguments_for_session(arguments: &str) -> String {
    serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .map(|value| opi_agent::diagnostic::redact_public_value(&value))
        .and_then(|value| serde_json::to_string(&value).ok())
        .unwrap_or_else(|| "\"[REDACTED]\"".to_owned())
}

/// Apply a label `Add`/`Remove` action to an in-memory label set in append
/// order. `Add` pushes the label if it is not already present (first-`Add`
/// wins on position); `Remove` drops every occurrence. Mirrors the
/// `reconstruct_context` derivation so the live coordinator view and the
/// disk-derived view stay consistent.
fn apply_label(labels: &mut Vec<String>, label: &str, action: LabelAction) {
    match action {
        LabelAction::Add => {
            if !labels.iter().any(|existing| existing == label) {
                labels.push(label.to_owned());
            }
        }
        LabelAction::Remove => labels.retain(|existing| existing != label),
    }
}

/// Derive the live `(name, labels)` view for `open_existing` from the raw
/// session entries. Scans every entry in file order, considering only metadata
/// whose `parent_id` resolves to the active content chain — or accepting
/// rootless metadata when no content exists yet (the pre-first-turn case).
/// Latest `session_info` wins; `label` Add/Remove apply in append order with
/// first-`Add` deduplication. Mirrors the `reconstruct_context` derivation and
/// the `latest_extension_state_entry_for_active_branch` active-chain filter.
fn seed_metadata_from_entries(entries: &[SessionEntry]) -> (Option<String>, Vec<String>) {
    use std::collections::HashSet;

    let active_ids_vec = crate::session_cli::active_entry_ids(entries);
    let active_ids: HashSet<&str> = active_ids_vec.iter().map(String::as_str).collect();
    let on_active = |parent: &Option<String>| -> bool {
        match parent {
            // Pre-first-turn case: no active content yet, so rootless metadata
            // is on the (single empty) trunk.
            None => active_ids.is_empty(),
            Some(id) => active_ids.contains(id.as_str()),
        }
    };

    let mut name: Option<String> = None;
    let mut labels: Vec<String> = Vec::new();
    for entry in entries {
        match entry {
            SessionEntry::SessionInfo(s) if on_active(&s.parent_id) => {
                name = Some(s.name.clone());
            }
            SessionEntry::Label(l) if on_active(&l.parent_id) => {
                apply_label(&mut labels, &l.label, l.action);
            }
            _ => {}
        }
    }
    (name, labels)
}

fn content_entry_id(entry: &SessionEntry) -> Option<&str> {
    match entry {
        SessionEntry::Message(m) => Some(m.id.as_str()),
        SessionEntry::Compaction(c) => Some(c.id.as_str()),
        SessionEntry::Leaf(_) => None,
        SessionEntry::ExtensionState(_) => None,
        _ => None,
    }
}

pub fn latest_extension_state(entries: &[SessionEntry]) -> Option<serde_json::Value> {
    crate::session_cli::latest_extension_state_entry_for_active_branch(entries)
        .map(|entry| entry.state.clone())
}

/// Extract the numeric suffix from entry IDs like `msg-3` or `cmp-7`.
/// Returns 0 for entries that don't match the pattern.
fn entry_seq(id: &str) -> u64 {
    id.split_once('-')
        .and_then(|(_, rest)| rest.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Advance the global `ENTRY_SEQ` past any IDs found in existing session
/// entries so resumed sessions don't produce duplicate IDs.
fn advance_seq_from_entries(entries: &[SessionEntry]) {
    let max_seq = entries
        .iter()
        .map(|e| entry_seq(e.entry_id()))
        .max()
        .unwrap_or(0);
    if max_seq > 0 {
        ENTRY_SEQ.fetch_max(max_seq + 1, Ordering::Relaxed);
    }
}
pub fn to_wire_result(out: &CompactionResultOutput) -> CompactionResult {
    CompactionResult {
        summary: out.summary.summary.clone(),
        first_kept_entry_id: out.first_kept_entry_id.clone(),
        tokens_before: out.tokens_before,
        tokens_after: out.tokens_after,
    }
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{ts:x}")
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let tod = secs % 86400;
    let h = tod / 3600;
    let m = (tod % 3600) / 60;
    let s = tod % 60;
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let diy = if is_leap(year) { 366 } else { 365 };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }
    let md = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0u64;
    for &d in &md {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    (year, month + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

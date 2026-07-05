//! Reusable session context reconstruction (Phase 13.2).
//!
//! The deterministic, reverse-direction counterpart to session writing:
//! given the ordered [`SessionEntry`] slice read from a session file (and
//! the [`CrashRecovery`] produced alongside it), produce the
//! agent-runtime [`AgentMessage`] sequence and selected metadata that a
//! resume/export/path should observe.
//!
//! # Pipeline
//!
//! ```text
//! ordered session entries
//!   -> SessionTree -> active tip (last valid Leaf, else trunk tip)
//!   -> walk parent_id chain from active tip to root
//!   -> reverse to root->tip order
//!   -> apply Message / Compaction / BranchSummary on the chain
//!   -> collect metadata parented to the active chain
//!   -> emit missing-parent + corrupt/unknown/truncated diagnostics
//!   -> ReconstructedContext
//! ```
//!
//! # Context-entry semantics
//!
//! - [`SessionEntry::Message`] becomes [`AgentMessage::Llm`] at its chain
//!   position.
//! - [`SessionEntry::Compaction`] drops every accumulated message whose chain
//!   index is strictly less than the compaction's `first_kept_entry_id`, then
//!   injects an [`AgentMessage::CompactionSummary`] at the compaction entry's
//!   position. Nested compactions compose: a later compaction may itself
//!   compact away an earlier compaction's summary.
//! - [`SessionEntry::BranchSummary`] injects an [`AgentMessage::BranchSummary`]
//!   at its chain position. Branch summaries **do** enter LLM context (Phase
//!   13 decision); the synthesized message carries only what the durable entry
//!   stores. `parent_session_id` is left empty for same-session summaries
//!   (cross-session injection is owned by the fork wiring in task 13.3); the
//!   field exists on the in-memory variant for forward compatibility.
//! - [`SessionEntry::ExtensionState`] is exposed as restore metadata (latest
//!   wins) and is **never** synthesized into an [`AgentMessage::Custom`]. The
//!   durable `custom_message` variant is deferred (Phase 13.1 decision);
//!   provider-context injection for extension messages remains governed by the
//!   existing `include_in_llm_context` flag and is not produced here.
//! - [`SessionEntry::SessionInfo`], [`SessionEntry::ModelChange`], and
//!   [`SessionEntry::ThinkingLevelChange`] are metadata attachments: latest
//!   value in file order wins, and they never enter LLM context.
//! - [`SessionEntry::Label`] is UI-visible metadata; Add/Remove are applied in
//!   file order to produce the active label set. Labels never enter LLM
//!   context.
//!
//! Metadata entries parented to a content entry that is **not** on the active
//! chain (a sibling branch) do not apply to the reconstructed context.
//!
//! # Diagnostics
//!
//! Missing-parent warnings (an entry whose `parent_id` does not resolve to any
//! known entry) are emitted with [`CODE_SESSION_CONTEXT_MISSING_PARENT`].
//! Corrupt/truncated/unknown observations are forwarded from the input
//! [`CrashRecovery`] so a single call surfaces every load-time anomaly.

use std::collections::{HashMap, HashSet};

use crate::diagnostic::{Diagnostic, SOURCE_SESSION, Severity, code::*};
use crate::message::{AgentMessage, BranchSummaryMessage, CompactionSummaryMessage};
use crate::session::{
    CrashRecovery, ExtensionStateEntry, LabelAction, LabelEntry, ModelChangeEntry, SessionEntry,
    SessionInfoEntry, ThinkingLevelChangeEntry,
};
use crate::session_branch::SessionTree;
use crate::session_event::ThinkingLevel;

/// Output of [`reconstruct_context`]: the agent-runtime message sequence plus
/// selected metadata reconstructed from a session's ordered entries.
///
/// Every field is independently derivable from the input slice; the function
/// is deterministic — the same input always yields the same output.
#[derive(Debug, Clone)]
pub struct ReconstructedContext {
    /// Agent-runtime messages in root->active-tip order, with compaction and
    /// branch-summary entries applied at their documented positions. Metadata
    /// entries (labels, session/model/thinking changes, extension_state) never
    /// appear here.
    pub messages: Vec<AgentMessage>,
    /// Entry id at the tip of the active branch (the last valid Leaf's
    /// `entry_id`, falling back to the trunk tip when no Leaf entries exist).
    /// `None` only when the session has no content entries.
    pub active_tip_entry_id: Option<String>,
    /// Latest `model_change` model spec on the active chain, if any.
    pub model: Option<String>,
    /// Latest `thinking_level_change` level on the active chain, if any.
    pub thinking_level: Option<ThinkingLevel>,
    /// Latest `session_info` name on the active chain, if any.
    pub session_name: Option<String>,
    /// Active label set: Add/Remove applied in file order, deduplicated,
    /// preserving first-Add order. UI-visible; never enters LLM context.
    pub labels: Vec<String>,
    /// Latest `extension_state` blob on the active chain, if any. Restore
    /// metadata only; never synthesized into an `AgentMessage::Custom`.
    pub extension_state: Option<serde_json::Value>,
    /// Diagnostics for missing parents (context reconstruction) plus forwarded
    /// corrupt/truncated/unknown observations from the input [`CrashRecovery`].
    pub diagnostics: Vec<Diagnostic>,
}

/// Reconstruct the agent-runtime context from ordered session entries.
///
/// See the module-level docs for the full pipeline and entry semantics. The
/// `recovery` argument supplies load-time observations (corrupt middle lines,
/// unknown future entry types, truncated trailing line) which are surfaced as
/// diagnostics alongside any missing-parent warnings detected during the
/// chain walk. Pass [`CrashRecovery::default()`] for a clean load.
pub fn reconstruct_context(
    entries: &[SessionEntry],
    recovery: &CrashRecovery,
) -> ReconstructedContext {
    let mut diagnostics = recovery.diagnostics();

    // Index entries by id for parent resolution and metadata attribution.
    // NOTE: this indexes ALL entry variants (content + metadata + leaves),
    // whereas `SessionTree::from_entries` indexes only Message/Compaction for
    // branch-graph purposes. The divergence is deliberate: the missing-parent
    // scan below must recognize any entry id as a valid parent target so that
    // metadata parented to a content tip resolves correctly. Do not "unify"
    // the two maps without revisiting the scan-all contract.
    let entries_by_id: HashMap<&str, &SessionEntry> =
        entries.iter().map(|e| (e.entry_id(), e)).collect();

    // Active tip comes from the existing SessionTree (last valid Leaf, else
    // trunk tip). The tree also ignores metadata entries for branch graph
    // purposes, matching the "metadata does not enter the branch graph" rule.
    let tree = SessionTree::from_entries(entries);
    let active_tip: Option<&str> = tree.active_tip();

    // Missing-parent scan: any entry (on any branch) whose `parent_id` does
    // not resolve to a known entry surfaces a warning. This catches corruption
    // regardless of which branch it sits on; the chain walk below is a
    // separate concern that builds the active branch only.
    for entry in entries {
        if let Some(pid) = entry.parent_id()
            && !entries_by_id.contains_key(pid)
        {
            diagnostics.push(missing_parent_diagnostic(
                entry.entry_id(),
                entry.parent_id(),
            ));
        }
    }

    // Walk parent_id from the active tip to the root, recording chain index
    // for each visited content entry. Metadata entries are attachments and
    // are not part of this walk; they are attributed by parent_id below.
    let mut chain: Vec<&str> = Vec::new();
    if let Some(start) = active_tip {
        let mut cursor: Option<&str> = Some(start);
        let mut visited: HashSet<&str> = HashSet::new();
        while let Some(id) = cursor {
            // Cycle guard (defensive; SessionTree already breaks cycles).
            if !visited.insert(id) {
                break;
            }
            let Some(entry) = entries_by_id.get(id).copied() else {
                break;
            };
            chain.push(id);
            // Stop at entries whose parent does not resolve; the missing-parent
            // diagnostic was already recorded in the scan above.
            cursor = entry
                .parent_id()
                .filter(|pid| entries_by_id.contains_key(*pid));
        }
        // The walk produced tip->root; reverse for root->tip.
        chain.reverse();
    }

    // chain_index: entry id -> position in root->tip order. Used by compaction
    // to decide what to drop (everything strictly before first_kept_entry_id).
    let chain_index: HashMap<&str, usize> =
        chain.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let chain_set: HashSet<&str> = chain.iter().copied().collect();

    // Group branch_summary entries by their parent content entry, in file
    // order. A summary injects immediately after its parent's contribution and
    // inherits the parent's chain index so a later compaction retains/drops it
    // together with the parent. Branch summaries do NOT advance the content
    // tip (Phase 13.1 storage invariant), so the parent is always a content
    // entry already on the chain.
    let mut summaries_by_parent: HashMap<&str, Vec<&str>> = HashMap::new();
    for entry in entries {
        if let SessionEntry::BranchSummary(b) = entry
            && let Some(pid) = b.parent_id.as_deref()
        {
            summaries_by_parent
                .entry(pid)
                .or_default()
                .push(b.summary.as_str());
        }
    }

    // Walk the chain in root->tip order and build the message sequence.
    // `accumulated` carries (chain_index, message) so a later compaction can
    // retain by chain position.
    let mut accumulated: Vec<(usize, AgentMessage)> = Vec::new();
    for id in &chain {
        let Some(entry) = entries_by_id.get(*id).copied() else {
            continue;
        };
        let this_idx = *chain_index.get(*id).unwrap_or(&0);
        match entry {
            SessionEntry::Message(m) => {
                accumulated.push((this_idx, AgentMessage::Llm(m.message.clone())));
            }
            SessionEntry::Compaction(c) => {
                apply_compaction(&mut accumulated, c, &chain_index);
            }
            // Metadata entries are attachments; handled in the metadata pass.
            SessionEntry::ExtensionState(_)
            | SessionEntry::SessionInfo(_)
            | SessionEntry::ModelChange(_)
            | SessionEntry::ThinkingLevelChange(_)
            | SessionEntry::Label(_)
            | SessionEntry::Leaf(_)
            | SessionEntry::BranchSummary(_) => {}
        }
        // Inject branch summaries parented to this chain entry, in file order.
        if let Some(summaries) = summaries_by_parent.get(*id) {
            for summary in summaries {
                let msg = AgentMessage::BranchSummary(BranchSummaryMessage {
                    // Same-session summary; cross-session injection is 13.3.
                    parent_session_id: String::new(),
                    summary: (*summary).to_owned(),
                    // Not derivable from the durable entry alone; left at 0.
                    entry_count: 0,
                });
                accumulated.push((this_idx, msg));
            }
        }
    }

    let messages: Vec<AgentMessage> = accumulated.into_iter().map(|(_, m)| m).collect();

    // Metadata pass: iterate entries in file order so "latest wins" lands on
    // the last attribution. Only metadata parented to the active chain
    // applies; metadata on sibling branches is skipped.
    let metadata = collect_metadata(entries, &chain_set);

    ReconstructedContext {
        messages,
        active_tip_entry_id: active_tip.map(str::to_owned),
        model: metadata.model,
        thinking_level: metadata.thinking_level,
        session_name: metadata.session_name,
        labels: metadata.labels,
        extension_state: metadata.extension_state,
        diagnostics,
    }
}

/// Drop accumulated messages strictly before the compaction's
/// `first_kept_entry_id`, then insert a [`CompactionSummary`] at this
/// compaction entry's position. If `first_kept_entry_id` is not on the chain,
/// no truncation is performed (degenerate fixture) but the summary is still
/// emitted.
fn apply_compaction(
    accumulated: &mut Vec<(usize, AgentMessage)>,
    c: &crate::session::CompactionEntry,
    chain_index: &HashMap<&str, usize>,
) {
    if let Some(&kept_idx) = chain_index.get(c.first_kept_entry_id.as_str()) {
        accumulated.retain(|(idx, _)| *idx >= kept_idx);
    }
    let this_idx = chain_index.get(c.id.as_str()).copied().unwrap_or(0);
    let summary = AgentMessage::CompactionSummary(CompactionSummaryMessage {
        summary: c.summary.clone(),
        first_kept_entry_id: c.first_kept_entry_id.clone(),
        tokens_before: c.tokens_before,
        tokens_after: c.tokens_after,
    });
    accumulated.insert(0, (this_idx, summary));
}

/// Metadata reconstructed from entries parented to the active chain.
///
/// Iteration is in file order, so the last applicable metadata entry wins for
/// scalar fields. Labels apply Add/Remove in file order, deduplicating while
/// preserving first-Add order.
#[derive(Default)]
struct CollectedMetadata {
    model: Option<String>,
    thinking_level: Option<ThinkingLevel>,
    session_name: Option<String>,
    labels: Vec<String>,
    extension_state: Option<serde_json::Value>,
}

/// Collect metadata from entries parented to the active chain.
fn collect_metadata(entries: &[SessionEntry], chain_set: &HashSet<&str>) -> CollectedMetadata {
    let mut out = CollectedMetadata::default();

    let on_chain = |parent_id: Option<&str>| -> bool {
        parent_id.and_then(|pid| chain_set.get(pid)).is_some()
    };

    for entry in entries {
        // Helper to fetch parent_id without borrowing entry across the match.
        let parent_id = entry.parent_id();
        if !on_chain(parent_id) {
            continue;
        }
        match entry {
            SessionEntry::ModelChange(ModelChangeEntry { model: m, .. }) => {
                out.model = Some(m.clone());
            }
            SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry { level, .. }) => {
                out.thinking_level = Some(*level);
            }
            SessionEntry::SessionInfo(SessionInfoEntry { name, .. }) => {
                out.session_name = Some(name.clone());
            }
            SessionEntry::ExtensionState(ExtensionStateEntry { state, .. }) => {
                out.extension_state = Some(state.clone());
            }
            SessionEntry::Label(LabelEntry { label, action, .. }) => {
                apply_label(&mut out.labels, label, *action);
            }
            // Content and pointer entries are not metadata.
            SessionEntry::Message(_)
            | SessionEntry::Compaction(_)
            | SessionEntry::Leaf(_)
            | SessionEntry::BranchSummary(_) => {}
        }
    }

    out
}

/// Apply a label Add/Remove to the active set, preserving first-Add order and
/// deduplicating.
fn apply_label(labels: &mut Vec<String>, label: &str, action: LabelAction) {
    match action {
        LabelAction::Add => {
            if !labels.iter().any(|l| l == label) {
                labels.push(label.to_owned());
            }
        }
        LabelAction::Remove => labels.retain(|l| l != label),
    }
}

/// Build the missing-parent diagnostic for `entry_id` whose `parent_id`
/// reference could not be resolved.
fn missing_parent_diagnostic(entry_id: &str, parent_id: Option<&str>) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_SESSION_CONTEXT_MISSING_PARENT,
        SOURCE_SESSION,
        "session entry references a missing parent entry",
    )
    .details(serde_json::json!({
        "entry_id": entry_id,
        "parent_id": parent_id.unwrap_or(""),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_label_add_then_remove() {
        let mut labels = Vec::new();
        apply_label(&mut labels, "a", LabelAction::Add);
        apply_label(&mut labels, "b", LabelAction::Add);
        apply_label(&mut labels, "a", LabelAction::Add); // dedupe
        assert_eq!(labels, vec!["a".to_owned(), "b".to_owned()]);
        apply_label(&mut labels, "a", LabelAction::Remove);
        assert_eq!(labels, vec!["b".to_owned()]);
    }
}

//! Compaction engine for managing conversation context size (S9.5).
//!
//! Provides manual, threshold-based, and overflow-triggered compaction
//! with hook extensibility for custom summary generation.

use thiserror::Error;

use crate::message::AgentMessage;
use crate::session_event::CompactionReason;

/// Configuration for compaction behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct CompactionConfig {
    pub enabled: bool,
    pub threshold_tokens: u64,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_tokens: 100_000,
        }
    }
}

/// A conversation entry with its session ID, for compaction input.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub message: AgentMessage,
}

/// Whether the summary came from the core engine or a hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummarySource {
    Core,
    Hook,
}

/// Result of a compaction operation.
#[derive(Debug, Clone)]
pub struct CompactionOutput {
    pub reason: CompactionReason,
    pub summary_text: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub tokens_after: u64,
    pub kept_entries: Vec<Entry>,
    pub summary_source: SummarySource,
}

/// Errors from compaction operations.
#[derive(Debug, Error)]
pub enum CompactionError {
    #[error("nothing to compact")]
    NothingToCompact,
}

impl From<&CompactionError> for crate::diagnostic::Diagnostic {
    fn from(error: &CompactionError) -> Self {
        use crate::diagnostic::{SOURCE_SESSION, Severity, code::*};
        match error {
            // NothingToCompact is a benign no-op, not a failure: report it as Info.
            CompactionError::NothingToCompact => crate::diagnostic::Diagnostic::new(
                Severity::Info,
                CODE_COMPACTION_NOTHING_TO_COMPACT,
                SOURCE_SESSION,
                "compaction skipped: nothing to compact",
            ),
        }
    }
}

impl CompactionOutput {
    /// Classify a successful compaction as an informational diagnostic carrying
    /// before/after token counts. The harness records this after a compaction.
    pub fn diagnostic(&self) -> crate::diagnostic::Diagnostic {
        use crate::diagnostic::{Diagnostic, SOURCE_SESSION, Severity, code::*};
        Diagnostic::new(
            Severity::Info,
            CODE_SESSION_COMPACTED,
            SOURCE_SESSION,
            "conversation compacted",
        )
        .details(serde_json::json!({
            "tokens_before": self.tokens_before,
            "tokens_after": self.tokens_after,
            "summary_source": match self.summary_source {
                SummarySource::Core => "core",
                SummarySource::Hook => "hook",
            },
        }))
    }
}

/// Hook trait for customizing compaction summary generation.
pub trait CompactionHooks: Send + Sync {
    /// Generate a summary for the messages being compacted.
    /// Return `None` to fall back to the core summary generator.
    fn generate_summary(&self, messages: &[AgentMessage]) -> Option<String>;
}

/// Default no-op hooks that always return `None` (core summary used).
pub struct DefaultCompactionHooks;

impl CompactionHooks for DefaultCompactionHooks {
    fn generate_summary(&self, _messages: &[AgentMessage]) -> Option<String> {
        None
    }
}

/// The compaction engine.
pub struct CompactionEngine {
    config: CompactionConfig,
}

impl CompactionEngine {
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Check if compaction should be triggered.
    pub fn should_compact(&self, total_tokens: u64, reason: CompactionReason) -> bool {
        match reason {
            CompactionReason::Manual => true,
            CompactionReason::Overflow => self.config.enabled,
            CompactionReason::Threshold => {
                self.config.enabled && total_tokens >= self.config.threshold_tokens
            }
        }
    }

    /// Execute compaction on the given entries.
    pub fn compact(
        &self,
        entries: &[Entry],
        reason: CompactionReason,
        hooks: &dyn CompactionHooks,
    ) -> Result<CompactionOutput, CompactionError> {
        if entries.len() < 2 {
            return Err(CompactionError::NothingToCompact);
        }

        let tokens_before = estimate_total_tokens(entries);

        // Always keep the last entry; try to keep recent entries up to a
        // reasonable fraction of the threshold.
        let split_idx = find_split_point(entries);

        let (compacted, kept) = entries.split_at(split_idx);
        if kept.is_empty() {
            return Err(CompactionError::NothingToCompact);
        }

        let first_kept_entry_id = kept[0].id.clone();

        // Try hook first, fall back to core summary
        let compacted_messages: Vec<AgentMessage> =
            compacted.iter().map(|e| e.message.clone()).collect();
        let (summary_text, source) = match hooks.generate_summary(&compacted_messages) {
            Some(s) => (s, SummarySource::Hook),
            None => (
                generate_core_summary(&compacted_messages),
                SummarySource::Core,
            ),
        };

        let kept_entries = kept.to_vec();
        let tokens_after = estimate_total_tokens(&kept_entries);

        Ok(CompactionOutput {
            reason,
            summary_text,
            first_kept_entry_id,
            tokens_before,
            tokens_after,
            kept_entries,
            summary_source: source,
        })
    }
}

/// Find the split point: keep the last 25% of entries (minimum 1), compact the rest.
fn find_split_point(entries: &[Entry]) -> usize {
    if entries.is_empty() {
        return 0;
    }

    // Always keep at least the last entry
    if entries.len() == 1 {
        return 0;
    }

    // Keep the last 25% of entries, minimum 1
    let min_keep = 1;
    let proportional = entries.len() / 4;
    let keep_count = proportional.max(min_keep);

    entries.len().saturating_sub(keep_count)
}

/// Estimate total tokens for a set of entries.
fn estimate_total_tokens(entries: &[Entry]) -> u64 {
    entries.iter().map(estimate_entry_tokens).sum()
}

/// Estimate tokens for a single entry using character heuristic.
fn estimate_entry_tokens(entry: &Entry) -> u64 {
    estimate_message_tokens(&entry.message)
}

/// Estimate tokens in an AgentMessage (rough: chars / 4).
fn estimate_message_tokens(msg: &AgentMessage) -> u64 {
    let text = extract_text(msg);
    text.len() as u64 / 4
}

/// Extract displayable text from an AgentMessage for summary generation.
fn extract_text(msg: &AgentMessage) -> String {
    match msg {
        AgentMessage::Llm(opi_ai::message::Message::User(u)) => u
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::InputContent::Text { text } => Some(text.clone()),
                opi_ai::message::InputContent::Image { media_type, .. } => {
                    Some(format!("[image: {}]", media_type.as_str()))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        AgentMessage::Llm(opi_ai::message::Message::Assistant(a)) => a
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        AgentMessage::Llm(opi_ai::message::Message::ToolResult(tr)) => tr
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::OutputContent::Text { text } => Some(text.clone()),
                opi_ai::message::OutputContent::Image { media_type, .. } => {
                    Some(format!("[image: {}]", media_type.as_str()))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        AgentMessage::CompactionSummary(cs) => cs.summary.clone(),
        AgentMessage::BranchSummary(bs) => bs.summary.clone(),
        AgentMessage::Custom(c) => c.data.to_string(),
        _ => String::new(),
    }
}

/// Generate a core summary from compacted messages.
fn generate_core_summary(messages: &[AgentMessage]) -> String {
    let texts: Vec<String> = messages.iter().map(extract_text).collect();
    let combined = texts.join(". ");
    let byte_count = combined.len();

    if byte_count <= 500 {
        format!("Compacted {} messages: {}", messages.len(), combined)
    } else {
        // Truncate to ~500 chars, finding a word boundary
        let truncated = &combined[..combined
            .char_indices()
            .take_while(|(i, _)| *i < 497)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(497)];
        format!("Compacted {} messages: {}...", messages.len(), truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_basic() {
        let msg = AgentMessage::Llm(opi_ai::message::Message::User(
            opi_ai::message::UserMessage {
                content: vec![opi_ai::message::InputContent::Text {
                    text: "Hello world test".into(), // 17 chars → ~4 tokens
                }],
                timestamp_ms: 0,
            },
        ));
        let tokens = estimate_message_tokens(&msg);
        assert_eq!(tokens, 4, "17 chars / 4 = 4 tokens");
    }

    #[test]
    fn split_point_keeps_tail() {
        let entries: Vec<Entry> = (0..10)
            .map(|i| Entry {
                id: format!("e{}", i),
                message: AgentMessage::Llm(opi_ai::message::Message::User(
                    opi_ai::message::UserMessage {
                        content: vec![opi_ai::message::InputContent::Text {
                            text: format!("msg {}", i),
                        }],
                        timestamp_ms: 0,
                    },
                )),
            })
            .collect();

        let split = find_split_point(&entries);
        assert_eq!(split, 8, "should keep last 2 of 10 entries");
        assert_eq!(entries[split].id, "e8");
    }
}

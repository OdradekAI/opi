//! Context occupancy estimates for request budgets and compaction decisions.
//!
//! Estimates use 3.5 UTF-16 code units per token and 4,800 equivalent chars
//! per image, rounding up at message granularity: each user or tool-result
//! message rounds once over its blocks, each assistant message rounds once
//! over its text/thinking/tool-call characters, and a system message rounds
//! its rendered body and sections once plus each tool list (added/removed)
//! separately. The last applicable positive successful usage anchors the
//! estimate; only messages after the anchor are estimated. A newer prefix
//! timestamp invalidates older usage (for example after compaction). These
//! are budgeting estimates, not token billing.

use crate::message::{AssistantContent, InputContent, Message, OutputContent, ToolDef};
use crate::stream::{StopReason, Usage};

/// UTF-16 code units per estimated token.
const CHARS_PER_TOKEN_NUMERATOR: u64 = 2;
const CHARS_PER_TOKEN_DENOMINATOR: u64 = 7;
/// Image-equivalent character count per image block.
const ESTIMATED_IMAGE_CHARS: u64 = 4_800;

fn chars_to_tokens(chars: u64) -> u64 {
    (chars * CHARS_PER_TOKEN_NUMERATOR).div_ceil(CHARS_PER_TOKEN_DENOMINATOR)
}

/// Context usage and the applicable provider-usage anchor, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Approximate text tokens without requiring a provider-specific tokenizer.
pub fn estimate_text_tokens(text: &str) -> u64 {
    chars_to_tokens(text.encode_utf16().count() as u64)
}

fn estimate_tools_tokens<T: serde::Serialize>(tools: &[T]) -> u64 {
    if tools.is_empty() {
        return 0;
    }
    match serde_json::to_string(tools) {
        Ok(serialized) => estimate_text_tokens(&serialized),
        Err(_) => estimate_text_tokens("[unserializable]"),
    }
}

/// Approximate one message, including tool arguments and image content.
pub fn estimate_message_tokens(message: &Message) -> u64 {
    match message {
        // A system message rounds its rendered body and sections once, then
        // adds the serialized added/removed tool lists, each rounded once.
        Message::System(system) => {
            estimate_text_tokens(&crate::transcript::system_message_text(system))
                + estimate_tools_tokens(&system.tools_added)
                + estimate_tools_tokens(&system.tools_removed)
        }
        Message::User(message) => chars_to_tokens(
            message
                .content
                .iter()
                .map(|content| match content {
                    InputContent::Text { text } => text.encode_utf16().count() as u64,
                    InputContent::Image { .. } => ESTIMATED_IMAGE_CHARS,
                })
                .sum(),
        ),
        Message::ToolResult(message) => chars_to_tokens(
            message
                .content
                .iter()
                .map(|content| match content {
                    OutputContent::Text { text } => text.encode_utf16().count() as u64,
                    OutputContent::Image { .. } => ESTIMATED_IMAGE_CHARS,
                })
                .sum(),
        ),
        Message::Assistant(message) => chars_to_tokens(
            message
                .content
                .iter()
                .map(|content| match content {
                    AssistantContent::Text { text, .. } => text.encode_utf16().count() as u64,
                    AssistantContent::Thinking { thinking, .. } => {
                        thinking.encode_utf16().count() as u64
                    }
                    AssistantContent::RedactedThinking { data } => {
                        data.encode_utf16().count() as u64
                    }
                    AssistantContent::ToolCall { tool_call } => {
                        (tool_call.name.encode_utf16().count()
                            + tool_call.arguments.encode_utf16().count())
                            as u64
                    }
                })
                .sum(),
        ),
    }
}

/// Anchor tokens from one usage block (V-45).
///
/// A known positive independently reported total wins even when it differs
/// from the bucket sum. When the total is absent or explicit zero, a positive
/// bucket sum applies only when the parent buckets are fully known
/// (`reported`); unknown buckets are never filled with zero and child
/// subsets are never added again.
fn anchor_tokens(usage: &Usage) -> u64 {
    if !usage.is_reported() {
        return 0;
    }
    if let Some(total) = usage.total_tokens.filter(|total| *total > 0) {
        return total;
    }
    let sum = usage.bucket_total_tokens();
    if sum > 0 {
        return sum;
    }
    0
}

/// Estimate the current request context, including its system and tool schema.
///
/// The last applicable positive successful usage (non-error, non-aborted,
/// timestamp not older than its prefix) anchors the estimate; only messages
/// after the anchor are estimated. Reported usage already includes the
/// system and tools, so the `system` and `tools` parameters count only when
/// no anchor exists. Applicable usage crosses model identities: the anchor
/// replaces the content estimate instead of lower-bounding it.
pub fn estimate_context_tokens(
    system: Option<&str>,
    messages: &[Message],
    tools: &[ToolDef],
) -> ContextUsageEstimate {
    let mut latest_prefix_timestamp = i64::MIN;
    let mut usage_tokens = 0;
    let mut usage_index = None;
    for (index, message) in messages.iter().enumerate() {
        if let Message::Assistant(assistant) = message {
            // A newer prefix message inserted after this response (for
            // example a compaction summary) invalidates its usage.
            if assistant.timestamp_ms >= latest_prefix_timestamp
                && !matches!(
                    assistant.stop_reason,
                    StopReason::Error | StopReason::Aborted
                )
            {
                let anchor = anchor_tokens(&assistant.usage);
                if anchor > 0 {
                    usage_tokens = anchor;
                    usage_index = Some(index);
                }
            }
        }
        let timestamp = match message {
            Message::System(message) => message.timestamp_ms,
            Message::User(message) => message.timestamp_ms,
            Message::ToolResult(message) => message.timestamp_ms,
            Message::Assistant(message) => message.timestamp_ms,
        };
        latest_prefix_timestamp = latest_prefix_timestamp.max(timestamp);
    }

    if let Some(index) = usage_index {
        let trailing_tokens = messages[index + 1..]
            .iter()
            .map(estimate_message_tokens)
            .fold(0u64, u64::saturating_add);
        return ContextUsageEstimate {
            tokens: usage_tokens.saturating_add(trailing_tokens),
            usage_tokens,
            trailing_tokens,
            last_usage_index: Some(index),
        };
    }

    let mut estimated = system.map(estimate_text_tokens).unwrap_or(0);
    if !tools.is_empty() {
        estimated = estimated.saturating_add(estimate_tools_tokens(tools));
    }
    for message in messages {
        estimated = estimated.saturating_add(estimate_message_tokens(message));
    }
    ContextUsageEstimate {
        tokens: estimated,
        usage_tokens: 0,
        trailing_tokens: estimated,
        last_usage_index: None,
    }
}

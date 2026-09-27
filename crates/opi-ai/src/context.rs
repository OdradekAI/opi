//! Context occupancy estimates for request budgets and compaction decisions.
//!
//! Estimates use four UTF-16 code units per token and 1,200 tokens per image.
//! A successful, reported response from the selected model may supply a better
//! prefix estimate. A newer prefix timestamp invalidates older usage (for
//! example after compaction). These are budgeting estimates, not token billing.

use crate::message::{AssistantContent, InputContent, Message, OutputContent, ToolDef};
use crate::stream::StopReason;

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
    (text.encode_utf16().count() as u64).div_ceil(4)
}

/// Approximate one message, including tool arguments and image content.
pub fn estimate_message_tokens(message: &Message) -> u64 {
    let chars: u64 = match message {
        Message::User(message) => message
            .content
            .iter()
            .map(|content| match content {
                InputContent::Text { text } => text.encode_utf16().count() as u64,
                InputContent::Image { .. } => 4_800,
            })
            .sum(),
        Message::ToolResult(message) => message
            .content
            .iter()
            .map(|content| match content {
                OutputContent::Text { text } => text.encode_utf16().count() as u64,
                OutputContent::Image { .. } => 4_800,
            })
            .sum(),
        Message::Assistant(message) => message
            .content
            .iter()
            .map(|content| match content {
                AssistantContent::Text { text } => text.encode_utf16().count() as u64,
                AssistantContent::Thinking { thinking, .. } => {
                    thinking.encode_utf16().count() as u64
                }
                AssistantContent::RedactedThinking { data } => data.encode_utf16().count() as u64,
                AssistantContent::ToolCall { tool_call } => {
                    (tool_call.name.encode_utf16().count()
                        + tool_call.arguments.encode_utf16().count()) as u64
                }
            })
            .sum(),
    };
    chars.div_ceil(4)
}

/// Estimate the current request context, including its system and tool schema.
///
/// `model` is the canonical `provider:model` route. Usage from another model,
/// failed responses, or responses older than an inserted prefix is ignored.
/// Reported usage already includes system/tools; the textual estimate remains
/// a lower bound so edited system/tool content cannot disappear from accounting.
pub fn estimate_context_tokens(
    system: Option<&str>,
    messages: &[Message],
    tools: &[ToolDef],
    model: &str,
) -> ContextUsageEstimate {
    let mut estimated = system.map(estimate_text_tokens).unwrap_or(0);
    if !tools.is_empty() {
        estimated = estimated.saturating_add(estimate_text_tokens(
            &serde_json::to_string(tools).expect("tool definitions serialize"),
        ));
    }
    let mut latest_prefix_timestamp = i64::MIN;
    let mut usage_tokens = 0;
    let mut trailing_tokens = 0u64;
    let mut last_usage_index = None;
    for (index, message) in messages.iter().enumerate() {
        let tokens = estimate_message_tokens(message);
        estimated = estimated.saturating_add(tokens);
        trailing_tokens = trailing_tokens.saturating_add(tokens);
        let timestamp = match message {
            Message::User(message) => message.timestamp_ms,
            Message::ToolResult(message) => message.timestamp_ms,
            Message::Assistant(message) => {
                if message.timestamp_ms >= latest_prefix_timestamp
                    && message.usage.is_reported()
                    && message.usage.total_tokens() > 0
                    && !matches!(message.stop_reason, StopReason::Error | StopReason::Aborted)
                    && model == format!("{}:{}", message.provider, message.model)
                {
                    usage_tokens = message.usage.total_tokens();
                    last_usage_index = Some(index);
                    trailing_tokens = 0;
                }
                message.timestamp_ms
            }
        };
        latest_prefix_timestamp = latest_prefix_timestamp.max(timestamp);
    }
    if last_usage_index.is_none() {
        trailing_tokens = estimated;
    }
    ContextUsageEstimate {
        tokens: estimated.max(usage_tokens.saturating_add(trailing_tokens)),
        usage_tokens,
        trailing_tokens,
        last_usage_index,
    }
}

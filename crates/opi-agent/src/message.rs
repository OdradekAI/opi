//! Agent-level message types (S7.2).

use serde::{Deserialize, Serialize};

/// Messages within the agent loop.
///
/// Wraps provider-facing `Message` types and adds session-level variants
/// that never reach the provider.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[expect(
    clippy::large_enum_variant,
    reason = "Inline LLM messages preserve the public construction and matching contract and avoid a per-message allocation"
)]
pub enum AgentMessage {
    /// A provider-facing message (user, assistant, or tool result).
    Llm(#[serde(with = "llm_message_serde")] opi_ai::message::Message),
    /// Summary produced after context compaction.
    CompactionSummary(CompactionSummaryMessage),
    /// Summary of a parent session branch.
    BranchSummary(BranchSummaryMessage),
    /// Extension-provided agent message.
    Custom(CustomAgentMessage),
}

/// Summary produced after context compaction (S9.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummaryMessage {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

/// Summary of a parent session branch (S9.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryMessage {
    pub parent_session_id: String,
    pub summary: String,
    pub entry_count: u64,
}

/// Extension-provided agent message (S7.2).
///
/// Unknown custom messages MUST NOT panic the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgentMessage {
    pub kind: String,
    pub data: serde_json::Value,
    pub include_in_llm_context: bool,
}

/// Custom serde module for `opi_ai::Message` that preserves the internal
/// tagged representation used by `Message`'s own `#[serde(tag = "role")]`.
mod llm_message_serde {
    use opi_ai::message::Message;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(msg: &Message, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::Serialize as _;
        serde_json::to_value(msg)
            .map_err(serde::ser::Error::custom)
            .and_then(|v| v.serialize(serializer))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Message, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        serde_json::from_value(v).map_err(serde::de::Error::custom)
    }
}

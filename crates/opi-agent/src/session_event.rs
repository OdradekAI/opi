//! Session-level event protocol (S7.5).
//!
//! Events emitted during a session's lifetime that are not tied to a single
//! agent loop invocation. These are the events serialized in JSON mode and
//! persisted in session JSONL storage.

use serde::{Deserialize, Serialize};

use crate::diagnostic::DiagnosticPayload;
use crate::event::AgentEvent;

/// Reasons why compaction was triggered (S9.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionReason {
    Manual,
    Threshold,
    Overflow,
}

/// Result of a successful compaction (S9.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

/// Thinking/reasoning level configuration (S9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingLevel {
    None,
    Low,
    Medium,
    High,
}

/// Events emitted during a session's lifetime (S7.5).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentSessionEvent {
    Agent {
        #[serde(with = "agent_event_serde")]
        event: AgentEvent,
    },
    QueueUpdate {
        steering: Vec<String>,
        follow_up: Vec<String>,
    },
    CompactionStart {
        reason: CompactionReason,
    },
    CompactionEnd {
        reason: CompactionReason,
        result: Option<CompactionResult>,
        aborted: bool,
        will_retry: bool,
        error_message: Option<String>,
    },
    AutoRetryStart {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error_message: String,
    },
    AutoRetryEnd {
        success: bool,
        attempt: u32,
        final_error: Option<String>,
    },
    SessionInfoChanged {
        session_id: String,
        name: Option<String>,
    },
    ThinkingLevelChanged {
        level: ThinkingLevel,
    },
    /// Cumulative token usage and (when known) cost breakdown for the
    /// session. Emitted at the end of a non-interactive run, but may also be
    /// emitted on demand. The wire `type` is `session_summary` to preserve
    /// the ad-hoc shape that was used before this variant existed.
    #[serde(rename = "session_summary")]
    SessionSummary {
        session_id: String,
        model: String,
        turns: u32,
        /// Number of provider request/response cycles observed in this run
        /// (`TurnStart` events). A tool-using prompt usually has
        /// `provider_turns > turns`.
        provider_turns: u32,
        tokens: SessionTokenTotals,
        #[serde(skip_serializing_if = "Option::is_none")]
        cost_usd: Option<SessionCostTotals>,
        /// Structured diagnostic counts observed during the run, when the
        /// harness recorded them. Absent (skipped) when no recording sink was
        /// attached, preserving the pre-7.5 wire shape.
        #[serde(skip_serializing_if = "Option::is_none")]
        diagnostics: Option<SessionDiagnosticCounts>,
    },
    /// Startup diagnostics (package/adapter/config/model-registry) surfaced
    /// before the first accepted prompt output. Phase 7 task 7.5 places these
    /// ahead of any `AgentStart` so a consumer learns about degraded startup
    /// state before run output begins. Additive; absent on runs that did not
    /// collect startup diagnostics.
    StartupDiagnostics {
        diagnostics: Vec<DiagnosticPayload>,
    },
    /// A run could not start because no credential is currently available.
    /// Consumers can surface `remediation` without parsing a diagnostic string.
    CredentialNeeded {
        provider_id: String,
        remediation: String,
        diagnostic: DiagnosticPayload,
    },
    /// A provider rejected a previously available credential. The current run
    /// has ended and a later explicit login is required.
    CredentialRevoked {
        provider_id: String,
        remediation: String,
        diagnostic: DiagnosticPayload,
    },
}

/// Severity tally for a run, attached to [`AgentSessionEvent::SessionSummary`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDiagnosticCounts {
    pub info: u64,
    pub warning: u64,
    pub error: u64,
}

/// Token totals carried by `AgentSessionEvent::SessionSummary`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionTokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Cost totals carried by `AgentSessionEvent::SessionSummary`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionCostTotals {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

/// Serde bridge for `AgentEvent` (no derives on the source type).
mod agent_event_serde {
    use crate::event::AgentEvent;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(event: &AgentEvent, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_json::to_value(event)
            .map_err(serde::ser::Error::custom)
            .and_then(|v| v.serialize(serializer))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AgentEvent, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        serde_json::from_value(v).map_err(serde::de::Error::custom)
    }
}

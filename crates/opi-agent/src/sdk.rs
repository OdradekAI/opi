//! SDK embedding surface for programmatic agent control.
//!
//! **Unstable 0.x API** — these types may change between minor versions without
//! notice. Embedders MUST pin an exact version and test against upgrades.
//!
//! # Overview
//!
//! This module provides shared command, response, and event types used by both
//! the RPC JSONL protocol (stdin/stdout) and the programmatic embedding API.
//! By centralising the protocol types here, the coding agent's RPC runner and
//! downstream embedders share the same definitions without duplicating logic.
//!
//! # Commands
//!
//! [`SdkCommand`] covers the full set of operations: prompt, continue, steer,
//! follow_up, abort, set_model, set_thinking_level, compact, session_info,
//! extension_command, and quit. Each variant carries an optional `id` for
//! request/response correlation.
//!
//! # Responses
//!
//! [`SdkResponse`] produces the standard JSON response envelope (`type:
//! "response"`, `success`, optional `id`/`error`/`data` fields) used by the
//! RPC protocol. Embedders can also consume it directly for structured results.
//!
//! # Events
//!
//! [`agent_event_to_value`] converts an [`AgentEvent`]
//! to a [`serde_json::Value`] for JSONL emission or structured inspection.

use crate::event::AgentEvent;

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

/// SDK/RPC protocol schema version. Clients and embedders MUST check this
/// before processing commands or events.
///
/// This is an **unstable 0.x** protocol. The version will remain at 3 until
/// the SDK surface stabilises; breaking changes bump the major version.
pub const SDK_SCHEMA_VERSION: u32 = 3;

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// An SDK command for controlling the agent programmatically.
///
/// This is the canonical command type shared between the RPC runner and the
/// embedding API. It round-trips through JSON with `#[serde(tag = "type")]`.
///
/// Each variant carries an optional `id` field for request/response
/// correlation in multiplexed scenarios.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum SdkCommand {
    /// Send a user prompt, streaming agent events.
    prompt {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    /// Continue conversation with additional text.
    #[serde(rename = "continue")]
    continue_ {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    /// Queue a steering message during agent operation.
    steer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    /// Queue a follow-up message for after agent stops.
    follow_up {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    /// Cancel current agent operation.
    abort {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Switch provider:model.
    set_model {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        model: String,
    },
    /// Set thinking/reasoning level.
    set_thinking_level {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        level: String,
    },
    /// Trigger manual compaction.
    compact {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Query session metadata.
    session_info {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Dispatch a custom command to registered extensions.
    extension_command {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        args: serde_json::Value,
    },
    /// Request the run's evidence records. Unsupported when evidence capture
    /// is not enabled for the session; the runner returns a structured
    /// `unsupported_trace_request` error then.
    trace {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Shut down the session.
    quit {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
}

impl SdkCommand {
    /// Return the optional correlation id.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::prompt { id, .. }
            | Self::continue_ { id, .. }
            | Self::steer { id, .. }
            | Self::follow_up { id, .. }
            | Self::abort { id }
            | Self::set_model { id, .. }
            | Self::set_thinking_level { id, .. }
            | Self::compact { id }
            | Self::session_info { id }
            | Self::extension_command { id, .. }
            | Self::trace { id }
            | Self::quit { id } => id.as_deref(),
        }
    }

    /// Return the command name for response correlation.
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::prompt { .. } => "prompt",
            Self::continue_ { .. } => "continue",
            Self::steer { .. } => "steer",
            Self::follow_up { .. } => "follow_up",
            Self::abort { .. } => "abort",
            Self::set_model { .. } => "set_model",
            Self::set_thinking_level { .. } => "set_thinking_level",
            Self::compact { .. } => "compact",
            Self::session_info { .. } => "session_info",
            Self::extension_command { .. } => "extension_command",
            Self::trace { .. } => "trace",
            Self::quit { .. } => "quit",
        }
    }

    /// Whether this is the quit command.
    pub fn is_quit(&self) -> bool {
        matches!(self, Self::quit { .. })
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A structured SDK/RPC response.
///
/// Serialises to the standard JSONL response envelope:
/// ```json
/// {"type":"response","command":"prompt","success":true,"id":"42"}
/// ```
///
/// For errors:
/// ```json
/// {"type":"response","command":"set_model","success":false,"error":"..."}
/// ```
///
/// For success with data:
/// ```json
/// {"type":"response","command":"session_info","success":true,"data":{...}}
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SdkResponse {
    /// Always `"response"`.
    #[serde(default = "response_type")]
    r#type: String,
    /// The command name this response correlates to.
    pub command: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// Optional correlation id matching the command's `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Error message (only when `success` is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Stable machine-readable error code (Phase 7 task 7.5), present only for
    /// structured errors such as an unsupported trace request. Additive:
    /// `#[serde(default)]` keeps older payloads deserializing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Response data payload (only when `success` is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl SdkResponse {
    /// Build a success response.
    pub fn success(id: Option<&str>, command: &str) -> Self {
        Self {
            r#type: response_type(),
            command: command.to_owned(),
            success: true,
            id: id.map(|s| s.to_owned()),
            error: None,
            error_code: None,
            data: None,
        }
    }

    /// Build a success response with a data payload.
    pub fn success_with_data(id: Option<&str>, command: &str, data: serde_json::Value) -> Self {
        Self {
            r#type: response_type(),
            command: command.to_owned(),
            success: true,
            id: id.map(|s| s.to_owned()),
            error: None,
            error_code: None,
            data: Some(data),
        }
    }

    /// Build an error response.
    pub fn error(id: Option<&str>, command: &str, message: &str) -> Self {
        Self {
            r#type: response_type(),
            command: command.to_owned(),
            success: false,
            id: id.map(|s| s.to_owned()),
            error: Some(message.to_owned()),
            error_code: None,
            data: None,
        }
    }

    /// Build a structured error response carrying a stable machine-readable
    /// `code` (Phase 7 task 7.5), e.g. for an unsupported trace request.
    pub fn error_with_code(id: Option<&str>, command: &str, code: &str, message: &str) -> Self {
        Self {
            r#type: response_type(),
            command: command.to_owned(),
            success: false,
            id: id.map(|s| s.to_owned()),
            error: Some(message.to_owned()),
            error_code: Some(code.to_owned()),
            data: None,
        }
    }
}

fn response_type() -> String {
    "response".to_owned()
}

// ---------------------------------------------------------------------------
// Event conversion
// ---------------------------------------------------------------------------

/// Convert an [`AgentEvent`] to a [`serde_json::Value`] for JSONL emission
/// or structured inspection.
///
/// Reuses the existing `AgentEvent` serde serialization (which includes the
/// `"type"` tag). Falls back to a generic error payload if serialization fails.
pub fn agent_event_to_value(event: &AgentEvent) -> serde_json::Value {
    match serde_json::to_value(event) {
        Ok(v) => v,
        Err(_) => serde_json::json!({
            "type": "SdkSerializationError",
            "message": "failed to serialize agent event",
        }),
    }
}

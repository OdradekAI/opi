//! The closed `command-execution-jsonl-v1` frame set for both wire directions.
//!
//! Frames use an **adjacently-tagged** JSON representation: every frame is an
//! object `{"type": <variant>, "payload": { ...fields... }}`. Adjacent tagging
//! composes with `#[serde(deny_unknown_fields)]` (applied to each enum and each
//! payload struct), so an unknown frame tag or an unknown field inside a known
//! frame is rejected  --  strict, because `v1` is closed.

use std::borrow::Cow;
use std::collections::BTreeMap;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

use super::identity::RequestId;
use super::native::NativeString;

// ---------------------------------------------------------------------------
// Host -> Backend
// ---------------------------------------------------------------------------

/// A frame sent from the host to the backend over stdin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HostToBackend {
    /// First frame; carries the deadline, bounded adapter configuration, and the
    /// host's ordered supported-protocol list.
    Initialize(InitializePayload),
    /// Carries the explicit program and arguments, canonical workspace, working
    /// directory, timeout, environment-inheritance policy, and bounded additions.
    Execute(ExecutePayload),
    /// Cancels the in-flight request. May be sent more than once.
    Cancel(CancelPayload),
}

impl HostToBackend {
    /// The request id carried by this frame.
    pub fn request_id(&self) -> &RequestId {
        match self {
            HostToBackend::Initialize(p) => &p.request_id,
            HostToBackend::Execute(p) => &p.request_id,
            HostToBackend::Cancel(p) => &p.request_id,
        }
    }

    /// The snake_case frame type name (the on-wire `type` discriminator).
    pub fn kind(&self) -> &'static str {
        match self {
            HostToBackend::Initialize(_) => "initialize",
            HostToBackend::Execute(_) => "execute",
            HostToBackend::Cancel(_) => "cancel",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitializePayload {
    pub request_id: RequestId,
    /// Request deadline, in milliseconds from the start of the execution.
    pub deadline_ms: u64,
    /// Bounded adapter configuration (opaque JSON; sized by the codec).
    pub adapter_config: serde_json::Value,
    /// The host's ordered supported-protocol list. Order is preference order.
    pub supported_protocols: Vec<super::identity::ProtocolId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutePayload {
    pub request_id: RequestId,
    /// Explicit program (the host maps a `bash` shell string to a platform
    /// shell program before sending).
    pub program: NativeString,
    /// Explicit argument vector.
    pub args: Vec<NativeString>,
    /// Canonical workspace root.
    pub workspace: NativeString,
    /// Working directory inside the workspace.
    pub cwd: NativeString,
    /// Execution timeout, in milliseconds.
    pub timeout_ms: u64,
    /// Environment-inheritance policy.
    pub env_inherit: EnvInherit,
    /// Bounded environment additions (keys and values are native strings).
    pub env_additions: BTreeMap<NativeString, NativeString>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelPayload {
    pub request_id: RequestId,
    pub reason: CancelReason,
}

/// Environment-inheritance policy for `execute`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvInherit {
    /// Inherit the host process environment, then apply `env_additions`.
    Inherit,
    /// Start from an empty environment; only `env_additions` apply.
    Clear,
}

/// Reason the host is canceling the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    /// The request deadline elapsed.
    Deadline,
    /// The host canceled the request (user-initiated).
    Canceled,
}

// ---------------------------------------------------------------------------
// Backend -> Host
// ---------------------------------------------------------------------------

/// A frame sent from the backend to the host over stdout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BackendToHost {
    /// Selects one protocol and reports implementation identity, version, and
    /// target.
    Ready(ReadyPayload),
    /// The request is valid and the target has not started.
    Accepted(AcceptedPayload),
    /// Setup established the reported placement, guarantee, policy, and
    /// limitations at the atomic target-start gate.
    Started(StartedPayload),
    /// A base64-encoded stdout chunk.
    Stdout(StdoutPayload),
    /// A base64-encoded stderr chunk.
    Stderr(StderrPayload),
    /// One diagnostic event.
    Diagnostic(DiagnosticPayload),
    /// Terminal success (or nonzero exit/signal as an in-band result).
    Completed(CompletedPayload),
    /// Terminal distress. Used for pre-started termination (no exit/signal) and
    /// for post-started protocol/cleanup failures with no target exit.
    Failed(FailedPayload),
}

impl BackendToHost {
    /// The request id carried by this frame.
    pub fn request_id(&self) -> &RequestId {
        match self {
            BackendToHost::Ready(p) => &p.request_id,
            BackendToHost::Accepted(p) => &p.request_id,
            BackendToHost::Started(p) => &p.request_id,
            BackendToHost::Stdout(p) => &p.request_id,
            BackendToHost::Stderr(p) => &p.request_id,
            BackendToHost::Diagnostic(p) => &p.request_id,
            BackendToHost::Completed(p) => &p.request_id,
            BackendToHost::Failed(p) => &p.request_id,
        }
    }

    /// The snake_case frame type name (the on-wire `type` discriminator).
    pub fn kind(&self) -> &'static str {
        match self {
            BackendToHost::Ready(_) => "ready",
            BackendToHost::Accepted(_) => "accepted",
            BackendToHost::Started(_) => "started",
            BackendToHost::Stdout(_) => "stdout",
            BackendToHost::Stderr(_) => "stderr",
            BackendToHost::Diagnostic(_) => "diagnostic",
            BackendToHost::Completed(_) => "completed",
            BackendToHost::Failed(_) => "failed",
        }
    }

    /// Decoded bytes contributed by this frame to cumulative output (stdout +
    /// stderr chunks only).
    pub fn output_bytes(&self) -> usize {
        match self {
            BackendToHost::Stdout(p) => p.data.as_bytes().len(),
            BackendToHost::Stderr(p) => p.data.as_bytes().len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadyPayload {
    pub request_id: RequestId,
    /// The single selected protocol (negotiation result).
    pub selected_protocol: super::identity::ProtocolId,
    /// Backend implementation version (diagnostics only; never a negotiation
    /// input).
    pub implementation_version: String,
    /// Execution target/platform.
    pub target: TargetId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPayload {
    pub request_id: RequestId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartedPayload {
    pub request_id: RequestId,
    /// Reported placement (adapter-defined vocabulary).
    pub placement: String,
    /// Reported confinement guarantee (adapter-defined vocabulary).
    pub guarantee: String,
    /// Reported policy (adapter-defined vocabulary).
    pub policy: String,
    /// Reported limitations.
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StdoutPayload {
    pub request_id: RequestId,
    /// Base64-encoded stdout bytes.
    pub data: Base64Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StderrPayload {
    pub request_id: RequestId,
    /// Base64-encoded stderr bytes.
    pub data: Base64Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticPayload {
    pub request_id: RequestId,
    /// Redacted diagnostic message (redaction is the backend's responsibility).
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompletedPayload {
    pub request_id: RequestId,
    /// Process exit code, when the target exited normally.
    pub exit: Option<u32>,
    /// Signal number, when the target was terminated by a signal.
    pub signal: Option<u32>,
    /// True if the request deadline elapsed.
    pub timed_out: bool,
    /// True if the request was canceled.
    pub cancelled: bool,
    /// Destination cleanup state.
    pub cleanup: CleanupState,
    /// Final diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FailedPayload {
    pub request_id: RequestId,
    /// Closed wire-level failure code.
    pub code: FailureCode,
    /// Where in the lifecycle the failure occurred.
    pub phase: FailurePhase,
    /// Optional redacted message.
    pub message: Option<String>,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Execution target/platform identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TargetId(String);

impl TargetId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Destination cleanup state reported on `completed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CleanupState {
    /// Destination cleanup confirmed within the bounded grace.
    Confirmed,
    /// Destination cleanup did not confirm within the bounded grace.
    Unconfirmed,
}

/// One diagnostic entry (used in the `diagnostics` lists on `completed`/`failed`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    /// Redacted diagnostic message.
    pub message: String,
}

/// Closed wire-level failure code (the codes the `v1` state machine can emit on
/// the wire). The wider architecture-level `ExtensionFailure` envelope (package,
/// trust, permission, policy codes) is a host-side concern and is intentionally
/// not modeled here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    /// Backend unavailable before the target started.
    Unavailable,
    /// Generic pre-started failure.
    Failed,
    /// No supported protocol in common.
    ProtocolIncompatible,
    /// A frame violated the protocol (malformed, oversized, unknown, out of
    /// order).
    ProtocolViolation,
    /// Execution failed after the target started.
    ExecutionFailed,
    /// The execution timeout elapsed.
    ExecutionTimedOut,
    /// Terminal cleanup failure (cleanup did not confirm).
    CleanupUnconfirmed,
}

/// Where in the lifecycle a failure occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailurePhase {
    /// Before the target started (handshake/setup).
    Handshake,
    /// After the target started.
    Execution,
    /// During cleanup.
    Cleanup,
}

/// Raw bytes encoded as standard (padded) base64. Carries command stdout/stderr
/// chunk payloads over the UTF-8 JSONL wire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Base64Bytes(#[serde(with = "base64_field")] Vec<u8>);

impl Base64Bytes {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl JsonSchema for Base64Bytes {
    fn schema_name() -> Cow<'static, str> {
        "Base64Bytes".into()
    }
    fn schema_id() -> Cow<'static, str> {
        "opi-protocol::execution::v1::Base64Bytes".into()
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "contentEncoding": "base64",
            "description": "Raw bytes encoded as standard (padded) base64. Carries command stdout/stderr chunk payloads."
        })
    }
}

/// Serde helper: encode `Vec<u8>` as a standard-base64 JSON string and back.
mod base64_field {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = <String as Deserialize>::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdout_round_trips_binary_via_base64() {
        for bytes in [
            &b""[..],
            &[0u8, 1, 2],
            &[0xFF, 0xFE, 0x00],
            b"hello \xFF binary",
        ] {
            let frame = BackendToHost::Stdout(StdoutPayload {
                request_id: RequestId::new("r1".to_string()).unwrap(),
                data: Base64Bytes::from_bytes(bytes),
            });
            let json = serde_json::to_string(&frame).unwrap();
            let back: BackendToHost = serde_json::from_str(&json).unwrap();
            assert_eq!(back.output_bytes(), bytes.len());
            match back {
                BackendToHost::Stdout(p) => assert_eq!(p.data.as_bytes(), bytes),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn adjacent_tagged_shape_round_trips() {
        let frame = HostToBackend::Initialize(InitializePayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
            deadline_ms: 30_000,
            adapter_config: serde_json::json!({"profile": "strict"}),
            supported_protocols: vec![super::super::identity::ProtocolId::new(
                "command-execution-jsonl-v1",
            )],
        });
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains(r#""type":"initialize""#), "{json}");
        assert!(json.contains(r#""payload""#), "{json}");
        let back: HostToBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind(), "initialize");
    }

    #[test]
    fn deny_unknown_field_rejected() {
        let json = r#"{"type":"accepted","payload":{"request_id":"r1","extra":1}}"#;
        let err = serde_json::from_str::<BackendToHost>(json);
        assert!(err.is_err(), "unknown field must be rejected");
    }

    #[test]
    fn deny_unknown_tag_rejected() {
        let json = r#"{"type":"bogus","payload":{"request_id":"r1"}}"#;
        assert!(serde_json::from_str::<BackendToHost>(json).is_err());
    }
}

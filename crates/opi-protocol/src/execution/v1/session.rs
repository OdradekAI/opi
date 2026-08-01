//! Stateful per-execution checker.
//!
//! The per-frame [`codec`](super::codec) is stateless: it parses one capped
//! JSONL line and enforces line/message/configuration/diagnostics bounds plus
//! request-id presence. Three invariants are inherently cross-frame, so this
//! [`Session`]  --  driven once per execution by the host  --  owns them:
//!
//! - **cumulative output**: decoded stdout+stderr bytes across all chunks must
//!   not exceed `max_cumulative_output`;
//! - **cross-request id**: every frame in one execution carries the same
//!   host-generated [`RequestId`] (the id of the first frame is the seed);
//! - **duplicate once-per-execution frames**: `initialize`, `execute`, `ready`,
//!   `accepted`, `started`, `completed`, and `failed` may each appear at most
//!   once.
//!
//! This substrate does **not** enforce full state-machine transition ordering
//! (for example, `completed` arriving before `started`); that is a runtime
//! responsibility of the host/backend, exercised by the execution-protocol-host
//! task. [`Session`] is a checker consumers may use; it does not launch or
//! supervise any process.

use std::collections::HashSet;

use super::bounds::{Bounds, BoundsError};
use super::codec::{CodecError, decode_backend, decode_host, validate_backend, validate_host};
use super::frames::{BackendToHost, HostToBackend};
use super::identity::RequestId;

/// A session-layer invariant violation, or a codec error from decoding a line.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The underlying codec rejected a line.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// A frame carried a request id different from the execution's seed.
    #[error("cross-request id: expected {expected}, got {got}")]
    CrossRequestId { expected: RequestId, got: RequestId },
    /// A once-per-execution frame type appeared more than once.
    #[error("duplicate once-per-execution frame: {frame}")]
    DuplicateFrame { frame: &'static str },
    /// Cumulative decoded output exceeded `max_cumulative_output`.
    #[error("cumulative output {cumulative} exceeded limit {limit}")]
    CumulativeOutputExceeded { cumulative: usize, limit: usize },
}

/// A stateful checker for one execution.
pub struct Session {
    bounds: Bounds,
    seed: Option<RequestId>,
    cumulative: usize,
    seen: HashSet<&'static str>,
}

impl Session {
    /// Create a session with the given bounds. Returns an error if the bounds
    /// are internally inconsistent.
    pub fn new(bounds: Bounds) -> Result<Self, BoundsError> {
        bounds.validate()?;
        Ok(Self {
            bounds,
            seed: None,
            cumulative: 0,
            seen: HashSet::new(),
        })
    }

    /// Total decoded stdout+stderr bytes observed so far.
    pub fn cumulative_output(&self) -> usize {
        self.cumulative
    }

    /// Observe a host frame, enforcing the cross-request-id and duplicate
    /// invariants.
    pub fn observe_host(&mut self, frame: &HostToBackend) -> Result<(), SessionError> {
        self.check_id(frame.request_id())?;
        self.check_duplicate(frame.kind())?;
        Ok(())
    }

    /// Observe a backend frame, enforcing cumulative output, cross-request id,
    /// and duplicate invariants.
    pub fn observe_backend(&mut self, frame: &BackendToHost) -> Result<(), SessionError> {
        self.account_output(frame)?;
        self.check_id(frame.request_id())?;
        self.check_duplicate(frame.kind())?;
        Ok(())
    }

    /// Decode, enforce per-frame codec bounds (line/message/configuration size),
    /// and observe one host JSONL line (no trailing newline).
    pub fn feed_host_line(&mut self, line: &[u8]) -> Result<HostToBackend, SessionError> {
        let frame = decode_host(line)?;
        validate_host(&frame, &self.bounds)?;
        self.observe_host(&frame)?;
        Ok(frame)
    }

    /// Decode, enforce per-frame codec bounds (line/message/diagnostics size),
    /// and observe one backend JSONL line (no trailing newline).
    pub fn feed_backend_line(&mut self, line: &[u8]) -> Result<BackendToHost, SessionError> {
        let frame = decode_backend(line)?;
        validate_backend(&frame, &self.bounds)?;
        self.observe_backend(&frame)?;
        Ok(frame)
    }

    fn check_id(&mut self, id: &RequestId) -> Result<(), SessionError> {
        match &self.seed {
            None => {
                self.seed = Some(id.clone());
                Ok(())
            }
            Some(seed) if seed == id => Ok(()),
            Some(seed) => Err(SessionError::CrossRequestId {
                expected: seed.clone(),
                got: id.clone(),
            }),
        }
    }

    fn check_duplicate(&mut self, kind: &'static str) -> Result<(), SessionError> {
        let once_per_execution = matches!(
            kind,
            "initialize" | "execute" | "ready" | "accepted" | "started" | "completed" | "failed"
        );
        if once_per_execution && !self.seen.insert(kind) {
            return Err(SessionError::DuplicateFrame { frame: kind });
        }
        Ok(())
    }

    fn account_output(&mut self, frame: &BackendToHost) -> Result<(), SessionError> {
        let bytes = frame.output_bytes();
        if bytes == 0 {
            return Ok(());
        }
        self.cumulative = self.cumulative.saturating_add(bytes);
        if self.cumulative > self.bounds.max_cumulative_output {
            return Err(SessionError::CumulativeOutputExceeded {
                cumulative: self.cumulative,
                limit: self.bounds.max_cumulative_output,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::v1::frames::{
        AcceptedPayload, CompletedPayload, InitializePayload, StdoutPayload,
    };
    use crate::execution::v1::{Base64Bytes, Bounds, CleanupState, CodecError, ProtocolId};

    fn rid(s: &str) -> RequestId {
        RequestId::new(s.to_string()).unwrap()
    }

    // Bounds small enough that configuration and diagnostics limits are easy to
    // exceed, while still satisfying the internal consistency checks.
    fn small_bounds() -> Bounds {
        Bounds {
            max_line_size: 4096,
            max_decoded_chunk_size: 8,
            max_configuration_size: 16,
            max_diagnostics_size: 8,
            max_cumulative_output: 64,
        }
    }

    #[test]
    fn cross_request_id_detected() {
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        let accepted_a = BackendToHost::Accepted(AcceptedPayload {
            request_id: rid("A"),
        });
        let accepted_b = BackendToHost::Accepted(AcceptedPayload {
            request_id: rid("B"),
        });
        session.observe_backend(&accepted_a).unwrap();
        let err = session.observe_backend(&accepted_b).unwrap_err();
        assert!(matches!(err, SessionError::CrossRequestId { .. }));
    }

    #[test]
    fn duplicate_once_per_execution_detected() {
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        // `accepted` is once-per-execution: a second one is a duplicate even
        // with the same id.
        let accepted = BackendToHost::Accepted(AcceptedPayload {
            request_id: rid("A"),
        });
        session.observe_backend(&accepted).unwrap();
        let err = session.observe_backend(&accepted).unwrap_err();
        assert!(matches!(
            err,
            SessionError::DuplicateFrame { frame: "accepted" }
        ));
    }

    #[test]
    fn stdout_and_stderr_may_repeat_and_accumulate() {
        let bounds = Bounds {
            max_cumulative_output: 10,
            ..Bounds::DEFAULT
        };
        let mut session = Session::new(bounds).unwrap();
        let chunk = BackendToHost::Stdout(StdoutPayload {
            request_id: rid("A"),
            data: Base64Bytes::from_bytes([0u8; 6]),
        });
        session.observe_backend(&chunk).unwrap();
        assert_eq!(session.cumulative_output(), 6);
        // A repeat (stdout may repeat) bringing the total to 12 > 10.
        let err = session.observe_backend(&chunk).unwrap_err();
        assert!(matches!(
            err,
            SessionError::CumulativeOutputExceeded {
                cumulative: 12,
                limit: 10
            }
        ));
    }

    #[test]
    fn feed_backend_line_decodes_and_observes() {
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        let line = r#"{"type":"accepted","payload":{"request_id":"A"}}"#;
        let frame = session.feed_backend_line(line.as_bytes()).unwrap();
        assert_eq!(frame.kind(), "accepted");

        // A cross-request id on a later line is rejected.
        let bad = r#"{"type":"completed","payload":{"request_id":"B","exit":0,"signal":null,"timed_out":false,"cancelled":false,"cleanup":"confirmed","diagnostics":[]}}"#;
        let err = session.feed_backend_line(bad.as_bytes()).unwrap_err();
        assert!(matches!(err, SessionError::CrossRequestId { .. }));
        // Diagnostic frame is not once-per-execution and carries no output.
        let diag = r#"{"type":"diagnostic","payload":{"request_id":"A","message":"ok"}}"#;
        session.feed_backend_line(diag.as_bytes()).unwrap();
    }

    #[test]
    fn completed_with_nonzero_exit_is_in_band() {
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        let completed = BackendToHost::Completed(CompletedPayload {
            request_id: rid("A"),
            exit: Some(2),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        });
        // A nonzero exit is an in-band terminal result, not a failure.
        session.observe_backend(&completed).unwrap();
    }

    #[test]
    fn oversized_adapter_config_rejected_on_session_path() {
        // The documented Session path must enforce the configuration-size bound.
        let mut session = Session::new(small_bounds()).unwrap();
        // adapter_config serializes to > max_configuration_size (16) bytes.
        let oversized = serde_json::json!({"padding": "aaaaaaaaaaaaaaaa"});
        let line = serde_json::to_string(&HostToBackend::Initialize(InitializePayload {
            request_id: rid("A"),
            deadline_ms: 1000,
            adapter_config: oversized,
            supported_protocols: vec![ProtocolId::new("command-execution-jsonl-v1")],
        }))
        .unwrap();
        let err = session.feed_host_line(line.as_bytes()).unwrap_err();
        assert!(
            matches!(
                err,
                SessionError::Codec(CodecError::ConfigurationTooLarge { .. })
            ),
            "oversized adapter config must be rejected: {err:?}"
        );
    }

    #[test]
    fn oversized_diagnostics_message_rejected_on_session_path() {
        // The documented Session path must enforce the diagnostics-size bound.
        let mut session = Session::new(small_bounds()).unwrap();
        let accepted = r#"{"type":"accepted","payload":{"request_id":"A"}}"#;
        session.feed_backend_line(accepted.as_bytes()).unwrap();
        let big = "x".repeat(20); // > max_diagnostics_size (8)
        let diag =
            format!(r#"{{"type":"diagnostic","payload":{{"request_id":"A","message":"{big}"}}}}"#);
        let err = session.feed_backend_line(diag.as_bytes()).unwrap_err();
        assert!(
            matches!(
                err,
                SessionError::Codec(CodecError::DiagnosticsTooLarge { .. })
            ),
            "oversized diagnostic message must be rejected: {err:?}"
        );
    }
}

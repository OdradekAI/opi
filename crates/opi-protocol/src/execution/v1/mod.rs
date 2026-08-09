//! `command-execution-jsonl-v1`: the closed one-shot command-execution wire
//! protocol shared between an execution host and an execution backend.
//!
//! # Wire identity and frozen-v1
//!
//! The wire identity is the string [`WIRE_IDENTITY`] = `command-execution-jsonl-v1`.
//! It is **independent of the Cargo crate version**: the version is baked into
//! the identity string. There is no separate numeric version field.
//!
//! `v1` is **frozen** at first release. Adding, removing, or renaming a `v1`
//! frame or field is a breaking change. Evolution is via a *new* wire identity
//! (for example `command-execution-jsonl-v2`) in a sibling module; `v1` and a
//! future `v2` coexist as distinct [`ProtocolId`]s and the host's ordered list
//! may carry both.
//!
//! # Transport
//!
//! The host starts a backend over stdio: stdin carries host-to-backend UTF-8
//! JSONL ([`HostToBackend`] frames), stdout carries backend-to-host UTF-8 JSONL
//! ([`BackendToHost`] frames), and stderr is **out-of-band** bounded crash
//! evidence  --  it is not a `v1` JSONL channel and this crate defines no stderr
//! framing. Command and configuration travel in protocol messages, never in
//! process arguments. Every frame carries one host-generated [`RequestId`].
//! Command stdout/stderr chunks are base64 ([`Base64Bytes`]); command
//! program/args/cwd/env use [`NativeString`]. The two encodings must not be
//! mixed.
//!
//! # State machine
//!
//! ```text
//! host starts backend
//!   -> initialize
//!   <- ready
//!   -> execute
//!   <- accepted
//!   <- started
//!   <- stdout | stderr | diagnostic   (zero or more)
//!   <- completed  |  failed
//!   -> host closes stdin
//!   -> backend exits successfully
//! ```
//!
//! - [`HostToBackend::Initialize`] carries the deadline, bounded adapter
//!   configuration, and the host's ordered supported-protocol list.
//! - [`BackendToHost::Ready`] selects one protocol and reports implementation
//!   identity, version, and target. The command is not disclosed until `ready`
//!   validates (enforced by the host/backend runtime, not by these types).
//! - [`HostToBackend::Execute`] carries the explicit program and arguments (the
//!   host maps a `bash` shell string to an explicit platform shell program and
//!   argument vector before sending), canonical workspace, working directory,
//!   timeout, environment-inheritance policy, and bounded environment additions.
//! - [`BackendToHost::Accepted`] means the request is valid and the target has
//!   not started.
//! - [`BackendToHost::Started`] reports the placement, guarantee, policy, and
//!   limitations established at the atomic target-start gate; the backend
//!   flushes `started` before releasing the target.
//! - [`BackendToHost::Completed`] is terminal and reports exit/signal,
//!   timeout/cancellation, cleanup state, and final diagnostics. Nonzero exit
//!   and signal are in-band results.
//! - [`BackendToHost::Failed`] is the terminal distress frame; it carries a
//!   closed [`FailureCode`] and a [`FailurePhase`]. Pre-started distress (the
//!   target never started) reports no exit/signal and uses `Failed`, not
//!   `Completed`.
//! - One backend process accepts at most one execution.
//!
//! # Bounds
//!
//! The codec rejects frames that exceed these bounds. Per-frame bounds are
//! enforced statelessly by the [`codec`]; the cumulative-output bound is
//! enforced statefully by [`Session`] across one execution.
//!
//! | Bound | Unit | Enforcement | Scope |
//! |---|---|---|---|
//! | line size | JSON data bytes, excluding the LF/CRLF delimiter | [`codec`] capped read, before parse | per frame |
//! | message size | coincident with line size for JSONL | [`codec`] | per frame |
//! | configuration | serialized JSON bytes | [`codec`], on `initialize` | per frame |
//! | diagnostics | bytes per diagnostic or `failed.message` | [`codec`] | per frame |
//! | cumulative output | decoded stdout+stderr bytes | [`Session`] | one execution |
//!
//! Frame count and rate are out of scope for this codec and are owned by host
//! L0 supervision (deadline + kill after bounded grace). `max_line_size` is the
//! decoder's per-stream line-buffer ceiling and thus the per-connection memory
//! cap; it must satisfy `max_line_size >= 4 *
//! ceil(max_decoded_chunk_size / 3) + framing` so a padded base64 chunk fits
//! (asserted on [`Bounds::DEFAULT`]). Cumulative output is counted in
//! **decoded** bytes; base64 inflation is transient and bounded per-frame by
//! `max_line_size`, not by the cumulative counter. `max_configuration_size` is
//! measured after JSON serialization, including escapes such as `\u0000`, so
//! the line-size consistency check reserves that serialized size plus framing
//! without applying a second escaping multiplier.
//!
//! # Version negotiation
//!
//! [`HostToBackend::Initialize`] carries the host's **ordered**
//! supported-protocol list (order = preference). [`select`] walks that list in
//! preference order and returns the first [`ProtocolId`] the backend also
//! supports (first-match-wins  --  **not** numeric-max; the version is baked into
//! the identity string, so numeric-max is not expressible). The backend calls
//! [`select`] and MUST place the result in `ready.selected_protocol`; on no
//! overlap it emits `failed` with [`FailureCode::ProtocolIncompatible`]. The
//! host, on `ready`, can only validate that `selected_protocol` is in its own
//! ordered list. `ready.implementation_version` and `ready.target` are reported
//! for diagnostics only and are never inputs to [`select`].
//!
//! # Compatibility rules
//!
//! - Unknown frame tag (unknown `type`) on either wire direction is a protocol
//!   violation (serde rejects it; maps to [`FailureCode::ProtocolViolation`]).
//! - Unknown field in a known frame is a protocol violation: every frame
//!   payload uses `#[serde(deny_unknown_fields)]` (strict  --  `v1` is closed).
//! - Selecting a `ProtocolId` for which the peer has no codec is a protocol
//!   violation.
//!
//! # Request-id invariant
//!
//! The protocol **defines** the invariant that every frame in one execution
//! carries the same host-generated [`RequestId`] (backend frames echo the id
//! received on the matching host frame) and that one backend process accepts
//! at most one execution. This substrate upholds it as follows: type-level
//! presence (an idless frame is unrepresentable) and non-emptiness (empty ids
//! are rejected); and a stateful [`Session`] checker that consumers may use to
//! reject a subsequent frame whose id differs from the first (cross-request
//! detection) and duplicate once-per-execution frame types. The substrate does
//! **not** force a backend to echo the host id by construction (a forged
//! `ready` is representable) and does not enforce full state-machine transition
//! ordering  --  both are runtime responsibilities of the host/backend.
//!
//! # Failure codes: wire vs. architecture envelope
//!
//! [`FailureCode`] is the **closed wire-level** set (7 codes the `v1` state
//! machine can emit on the wire). The wider architecture-level
//! `ExtensionFailure` envelope (package/trust/permission/policy codes) is a
//! host-side concern and is intentionally **not** modeled here; the host maps
//! `failed` frames and `completed` cleanup state into that envelope.

pub mod bounds;
pub mod codec;
pub mod frames;
pub mod identity;
pub mod native;
pub mod schema;
pub mod session;

pub use bounds::{Bounds, BoundsError};
pub use codec::{CodecError, LineReader, encode_line};
pub use frames::{
    BackendToHost, Base64Bytes, CancelReason, CleanupState, Diagnostic, EnvInherit, FailureCode,
    FailurePhase, HostToBackend, TargetId,
};
pub use identity::{
    ImplementationId, InvalidImplementationId, InvalidProtocolId, InvalidRequestId, ProtocolId,
    ProtocolIncompatible, RequestId, V1, select,
};
pub use native::{NativeString, NativeStringError};
pub use schema::{SCHEMA_DESCRIPTION, SCHEMA_ID_URL, schema, schema_with_bounds};
pub use session::{Session, SessionError};

/// Wire identity of this protocol. Independent of the Cargo crate version.
pub const WIRE_IDENTITY: &str = "command-execution-jsonl-v1";

//! The atomic helper start gate (Phase 16 task 16.12).
//!
//! [`start`] is the all-or-nothing boundary around [`SandboxRunner::run`]: setup
//! either fully succeeds (a [`SandboxRun`] is ready, and the backend will emit +
//! flush the `started` frame before draining it) or fully fails BEFORE the target
//! is released (a closed [`FailureCode`], phase `Handshake`). This is the atomic
//! target-start gate the spec names ("started means setup established the
//! reported placement/guarantee/policy/limitations at an atomic target-start
//! gate"; design `### State machine`).
//!
//! # 16.12 scope (honest)
//!
//! In 16.12 the only confinement is L0 supervision: native mechanism install
//! (Landlock/seccomp on Linux, `sandbox-exec` on macOS) is owned by 16.13 /
//! 16.14.1 and hooks INSIDE the runner's `Restriction::prepare` (called by
//! [`SandboxRunner::run`] before spawn), NOT here. So this gate's "setup" is the
//! platform-posture check + the runner's validate/temp-root/restriction/spawn
//! sequence, and the `started` vocabulary it reports is the EFFECTIVE 16.12
//! contract (`supervised` / `unrestricted`), never `restricted` / `isolated`
//! (those land with the native mechanisms). The substrate-level invariant the
//! backend upholds is "the `started` frame is flushed before any target output
//! is released": the SDK buffers output into the terminal `SandboxResult`, so
//! output cannot be forwarded until the run is polled to completion, which the
//! backend does only AFTER emitting + flushing `started`.
//!
//! Protocol stdin NEVER reaches the target: [`build_request`] pins
//! [`StdinPolicy::Null`] (Phase 16 task 16.12 audit fold: stdin-isolation +
//! helper-gate-atomicity).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;

use opi_protocol::execution::v1::frames::{ExecutePayload, StartedPayload};
use opi_protocol::execution::v1::identity::RequestId;
use opi_protocol::execution::v1::{FailureCode, NativeString};
use tokio_util::sync::CancellationToken;

use crate::policy::{ContractStatus, Mechanism};
use crate::runner::{SandboxRequest, SandboxRun, SandboxRunner, SetupFailureReason, StdinPolicy};

/// The atomic start gate's outcome.
///
/// [`StartOutcome::Ready`] carries the run the backend must drive: it polls the
/// run's first [`crate::SandboxEvent::Started`] to build the `started` frame,
/// flushes it, THEN drains the run to completion. [`StartOutcome::Refused`]
/// carries the closed wire failure code for a `failed{Handshake}` frame; no
/// target was released.
pub(crate) enum StartOutcome {
    /// Setup succeeded; the backend builds + flushes `started`, then drains.
    Ready {
        /// The owned run handle to drain.
        run: SandboxRun,
    },
    /// Setup failed before the target was released.
    Refused {
        /// The closed wire failure code (phase Handshake).
        code: FailureCode,
    },
}

/// Run the atomic start gate: refuse an unsupported platform before constructing
/// a runner request, otherwise let [`SandboxRunner::run`] establish setup
/// (validate -> temp root -> restriction -> spawn + attach, all synchronous and
/// all-or-`Err`). The runner spawns + attaches the tree guard in the same
/// synchronous span (no `.await` between), so on `Ok` the target is started and
/// the backend owns release ordering; on `Err` no target was released.
///
/// This function is SYNCHRONOUS by design: it is the atomic boundary, and
/// polling the run (async) is the backend's responsibility so it can interleave
/// the `started` flush with the drain. `request.cancel` MUST already carry the
/// cooperative cancellation token ([`build_request`] wires it).
pub(crate) fn start(
    supported: bool,
    runner: &SandboxRunner,
    request: SandboxRequest,
) -> StartOutcome {
    if !supported {
        // 16.12: platform::current() is unsupported on every platform; native
        // confinement lands in 16.13 / 16.14.1. Refuse before any target
        // release with the most precise pre-start code.
        return StartOutcome::Refused {
            code: FailureCode::Unavailable,
        };
    }
    match runner.run(request) {
        Ok(run) => StartOutcome::Ready { run },
        Err(failed) => StartOutcome::Refused {
            code: map_setup_failure(failed.reason),
        },
    }
}

/// Build the SDK [`SandboxRequest`] from a wire [`ExecutePayload`]. Pure, so the
/// keystone stdin-isolation invariant ([`StdinPolicy::Null`]) and the field
/// mapping are unit-testable.
///
/// `NativeString` is a lossless byte sequence; the SDK takes UTF-8 `String` /
/// `PathBuf`, so bytes are mapped lossily. In Phase 16 the host maps a bash shell
/// string to an explicit UTF-8 program/args before sending, so the wire carries
/// UTF-8 and the lossy mapping is exact.
pub(crate) fn build_request(exec: &ExecutePayload, cancel: CancellationToken) -> SandboxRequest {
    SandboxRequest {
        program: PathBuf::from(native_to_string(&exec.program)),
        args: exec.args.iter().map(native_to_string).collect::<Vec<_>>(),
        workspace: PathBuf::from(native_to_string(&exec.workspace)),
        cwd: PathBuf::from(native_to_string(&exec.cwd)),
        timeout: Duration::from_millis(exec.timeout_ms),
        env_inherit: exec.env_inherit,
        env_additions: exec
            .env_additions
            .iter()
            .map(|(k, v)| (native_to_string(k), native_to_string(v)))
            .collect(),
        // Protocol stdin is reserved (host->backend JSONL) and NEVER inherited
        // by the target (design `### State machine`: "the backend never inherits
        // protocol stdin as target stdin").
        stdin: StdinPolicy::Null,
        cancel: Some(cancel),
    }
}

/// Map a pre-start [`SetupFailureReason`] to the closed wire [`FailureCode`]
/// (Phase 16 task 16.12 audit fold: pin the determinism gap). Every variant is
/// phase `Handshake` (the target never started).
pub(crate) fn map_setup_failure(reason: SetupFailureReason) -> FailureCode {
    match reason {
        // The wire request was semantically malformed (zero timeout, empty
        // workspace): a host-side bug expressed on the wire.
        SetupFailureReason::InvalidRequest => FailureCode::ProtocolViolation,
        // Generic pre-start execution distress.
        SetupFailureReason::ProgramNotFound
        | SetupFailureReason::RestrictionSetup
        | SetupFailureReason::SpawnFailed => FailureCode::Failed,
        // The platform cannot establish the requested contract (forward-
        // compatible; the posture gate refuses first in 16.12).
        SetupFailureReason::UnsupportedPlatform => FailureCode::Unavailable,
    }
}

/// Build the `started` frame from the effective mechanism/contract and the
/// platform limitations. `Mechanism::None` (L0 supervision only, the 16.12
/// backend under `NoRestriction`) reports `supervised` / `unrestricted`; a
/// native mechanism (`Landlock`/`Seccomp` on supported Linux, 16.13) reports
/// `supervised` / `restricted` — NEVER `isolated` (crate vocabulary contract,
/// `lib.rs`; design `### Common profile`: the package reports `restricted`).
pub(crate) fn started_payload(
    request_id: &RequestId,
    mechanism: Mechanism,
    _contract: ContractStatus,
    limitations: &[String],
) -> StartedPayload {
    let (guarantee, policy) = match mechanism {
        // L0 supervision only (NoRestriction, the protocol backend).
        Mechanism::None => ("supervised", "unrestricted"),
        // A native mechanism installed a confinement contract. Seccomp is
        // always installed alongside Landlock on Linux, so both report the same
        // honest vocabulary; the run is supervised AND restricted.
        Mechanism::Landlock | Mechanism::Seccomp => ("supervised", "restricted"),
    };
    StartedPayload {
        request_id: request_id.clone(),
        placement: "host".to_string(),
        guarantee: guarantee.to_string(),
        policy: policy.to_string(),
        limitations: limitations.to_vec(),
    }
}

/// Lossily map a [`NativeString`] to a UTF-8 [`String`] for the SDK's String-typed
/// fields.
fn native_to_string(ns: &NativeString) -> String {
    String::from_utf8_lossy(ns.as_bytes()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{NetworkPolicy, Profile, SandboxPolicy};
    use opi_protocol::execution::v1::EnvInherit;

    fn rid() -> RequestId {
        RequestId::new("r1".to_string()).unwrap()
    }

    fn exec(program: &str, timeout_ms: u64) -> ExecutePayload {
        ExecutePayload {
            request_id: rid(),
            program: NativeString::from_utf8(program),
            args: vec![
                NativeString::from_utf8("-c"),
                NativeString::from_utf8("echo hi"),
            ],
            workspace: NativeString::from_utf8("/ws"),
            cwd: NativeString::from_utf8("/ws"),
            timeout_ms,
            env_inherit: EnvInherit::Inherit,
            env_additions: [(NativeString::from_utf8("K"), NativeString::from_utf8("v"))]
                .into_iter()
                .collect(),
        }
    }

    /// KEYSTONE: the backend target NEVER inherits protocol stdin.
    #[test]
    fn build_request_pins_stdin_to_null() {
        let cancel = CancellationToken::new();
        let request = build_request(&exec("sh", 1000), cancel);
        assert_eq!(
            request.stdin,
            StdinPolicy::Null,
            "protocol stdin must never reach the target"
        );
    }

    #[test]
    fn build_request_maps_fields_and_cancel() {
        let cancel = CancellationToken::new();
        let request = build_request(&exec("sh", 7000), cancel.clone());
        assert_eq!(request.program, PathBuf::from("sh"));
        assert_eq!(request.args, vec!["-c".to_string(), "echo hi".to_string()]);
        assert_eq!(request.workspace, PathBuf::from("/ws"));
        assert_eq!(request.cwd, PathBuf::from("/ws"));
        assert_eq!(request.timeout, Duration::from_millis(7000));
        assert_eq!(request.env_inherit, EnvInherit::Inherit);
        assert_eq!(
            request.env_additions.get("K").map(String::as_str),
            Some("v")
        );
        // The cancel token is wired (firing it would resolve the run Cancelled).
        let _ = request.cancel.expect("cancel token wired");
    }

    /// The closed SetupFailureReason -> FailureCode table (phase Handshake).
    #[test]
    fn map_setup_failure_table() {
        assert_eq!(
            map_setup_failure(SetupFailureReason::InvalidRequest),
            FailureCode::ProtocolViolation
        );
        assert_eq!(
            map_setup_failure(SetupFailureReason::ProgramNotFound),
            FailureCode::Failed
        );
        assert_eq!(
            map_setup_failure(SetupFailureReason::RestrictionSetup),
            FailureCode::Failed
        );
        assert_eq!(
            map_setup_failure(SetupFailureReason::SpawnFailed),
            FailureCode::Failed
        );
        assert_eq!(
            map_setup_failure(SetupFailureReason::UnsupportedPlatform),
            FailureCode::Unavailable
        );
    }

    /// The 16.12 started vocabulary is honest (supervised/unrestricted), never
    /// restricted/isolated, and echoes the platform limitations.
    #[test]
    fn started_payload_vocabulary_is_honest_l0_only() {
        let limitations = vec!["native confinement not wired".to_string()];
        let frame = started_payload(
            &rid(),
            Mechanism::None,
            ContractStatus::Unrestricted,
            &limitations,
        );
        assert_eq!(frame.placement, "host");
        assert_eq!(frame.guarantee, "supervised");
        assert_eq!(frame.policy, "unrestricted");
        assert_eq!(frame.limitations, limitations);
        // The confinement-CLAIM words `isolated`/`enforced` must NEVER appear in
        // 16.12 (the run is supervised/unrestricted only). `restricted` is checked
        // by the exact-equality assertions above, NOT as a substring, because the
        // honest policy value `unrestricted` legitimately contains it.
        for word in ["isolated", "enforced"] {
            assert!(
                !frame.guarantee.contains(word)
                    && !frame.policy.contains(word)
                    && !frame.placement.contains(word),
                "16.12 started vocabulary must not claim `{word}`"
            );
        }
    }

    /// A native mechanism (16.13 Landlock/Seccomp) reports the honest
    /// `supervised` / `restricted` vocabulary, never `isolated`/`enforced`.
    #[test]
    fn started_payload_native_reports_restricted() {
        for mechanism in [Mechanism::Landlock, Mechanism::Seccomp] {
            let frame = started_payload(&rid(), mechanism, ContractStatus::Restricted, &[]);
            assert_eq!(frame.placement, "host");
            assert_eq!(frame.guarantee, "supervised");
            assert_eq!(frame.policy, "restricted");
            for word in ["isolated", "enforced"] {
                assert!(
                    !frame.guarantee.contains(word)
                        && !frame.policy.contains(word)
                        && !frame.placement.contains(word),
                    "native started vocabulary must not claim `{word}`"
                );
            }
        }
    }

    /// start() refuses an unsupported platform before touching the runner.
    #[test]
    fn start_refuses_unsupported_platform_with_unavailable() {
        let runner = SandboxRunner::new(
            SandboxPolicy::new(Profile::WorkspaceWrite, NetworkPolicy::Deny),
            std::sync::Arc::new(crate::NoRestriction),
        );
        let cancel = CancellationToken::new();
        let request = build_request(&exec("sh", 1000), cancel);
        match start(false, &runner, request) {
            StartOutcome::Refused {
                code: FailureCode::Unavailable,
            } => {}
            _ => panic!("expected Refused(Unavailable) on unsupported platform"),
        }
    }
}

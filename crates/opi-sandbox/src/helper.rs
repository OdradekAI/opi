//! The atomic helper start gate (Phase 16 task 16.12).
//!
//! [`start`] is the all-or-nothing boundary around the runner's preparation and
//! spawn operations: setup either fully succeeds (a [`SandboxRun`] is ready,
//! and the backend will emit +
//! flush the `started` frame before draining it) or fully fails BEFORE the target
//! is released (a closed [`FailureCode`], phase `Handshake`). This is the atomic
//! target-start gate the spec names ("started means setup established the
//! reported placement/guarantee/policy/limitations at an atomic target-start
//! gate"; design `### State machine`).
//!
//! # Shipped mechanism scope
//!
//! Supported Linux and macOS postures install their current native restrictions
//! (Landlock/seccomp on Linux and Seatbelt through canonical
//! `/usr/bin/sandbox-exec` on macOS) inside the runner's restriction seam before
//! spawn. Unsupported postures are refused before target start. This gate's
//! setup is the side-effect-free validation followed by the platform-posture
//! check and the runner's temp-root/restriction/spawn sequence. Its `started`
//! vocabulary reports the effective contract (`restricted` for native
//! confinement, `unrestricted` only for an explicitly supplied no-restriction
//! SDK seam), never `isolated`. The substrate-level invariant the backend
//! upholds is "the `started` frame is flushed before any target output is
//! released": the SDK buffers output into the terminal `SandboxResult`, so
//! output cannot be forwarded until the run is polled to completion, which the
//! backend does only AFTER emitting + flushing `started`.
//!
//! Protocol stdin NEVER reaches the target: [`build_request`] pins
//! [`StdinPolicy::Null`] (Phase 16 task 16.12 audit fold: stdin-isolation +
//! helper-gate-atomicity).

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use opi_protocol::execution::v1::frames::{ExecutePayload, StartedPayload};
use opi_protocol::execution::v1::identity::RequestId;
use opi_protocol::execution::v1::{FailureCode, NativeString};
use tokio_util::sync::CancellationToken;

use crate::policy::{ContractStatus, Mechanism};
use crate::runner::{
    PreparedSandboxRun, RunDeadlinePlan, RunDeadlines, SandboxRequest, SandboxRun, SandboxRunner,
    SetupFailureReason, SpawnPreparedOutcome, StartConfirmationFailure, StdinPolicy,
    ValidatedSandboxRequest, cleanup_prepared_until,
};

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
    /// Setup produced a fully guarded run only after its start cutoff. The
    /// backend must keep it gated and confirm cleanup under the hard deadline.
    Expired {
        /// The late, still-gated run.
        run: SandboxRun,
    },
    /// Setup did not finish before the hard cleanup deadline, so invocation
    /// cleanup could not be confirmed.
    CleanupUnconfirmed,
}

/// Run the atomic start gate for a request that already passed
/// [`validate_request_until`]: refuse an unsupported platform before setup,
/// otherwise prepare temp-root/restriction state on a blocking worker, then
/// spawn + attach synchronously on the awaiting path. The background worker
/// has no spawn operation; spawn + guard attachment remain in one synchronous
/// span (no `.await` between), and the target remains gated throughout. A
/// launcher-based restriction must also produce its in-profile acknowledgement
/// before this gate returns [`StartOutcome::Ready`].
///
/// Restriction setup is synchronous at the platform seam, so it runs on a
/// blocking worker. At the setup cutoff the helper fires the request's
/// cancellation token, then observes that same worker only until the existing
/// hard cleanup deadline. A spawn that returns after its cutoff is returned
/// still gated so the backend can cancel and drain it; a preparation worker
/// still running at the hard deadline reports cleanup as unconfirmed.
/// `request.cancel` MUST already carry the cooperative cancellation token
/// ([`build_request`] wires it).
pub(crate) async fn start(
    supported: bool,
    runner: &SandboxRunner,
    request: ValidatedSandboxRequest,
    deadlines: RunDeadlines,
) -> StartOutcome {
    if !supported {
        // Without a supported native posture the requested restriction cannot
        // be established. Refuse before any target release with the most
        // precise pre-start code.
        return StartOutcome::Refused {
            code: FailureCode::Unavailable,
        };
    }
    let setup_runner = runner.clone();
    let setup_cancel = request.setup_cancel_token();
    let mut setup = tokio::task::spawn_blocking(move || {
        setup_runner.prepare_validated_until(request, Some(deadlines.start_by()))
    });
    match tokio::time::timeout_at(deadlines.start_by(), &mut setup).await {
        Err(_) => {
            setup_cancel.cancel();
            match tokio::time::timeout_at(deadlines.cleanup(), &mut setup).await {
                Err(_) => StartOutcome::CleanupUnconfirmed,
                Ok(joined) => classify_expired_preparation(joined, deadlines.cleanup()).await,
            }
        }
        Ok(joined) if tokio::time::Instant::now() >= deadlines.start_by() => {
            setup_cancel.cancel();
            classify_expired_preparation(joined, deadlines.cleanup()).await
        }
        Ok(Err(_)) => StartOutcome::Refused {
            code: FailureCode::Failed,
        },
        Ok(Ok(Err(failed))) => StartOutcome::Refused {
            code: map_setup_failure(failed.reason),
        },
        Ok(Ok(Ok(prepared))) => spawn_prepared(runner, prepared, deadlines).await,
    }
}

async fn spawn_prepared(
    runner: &SandboxRunner,
    prepared: PreparedSandboxRun,
    deadlines: RunDeadlines,
) -> StartOutcome {
    match runner.spawn_prepared(prepared, RunDeadlinePlan::Fixed(deadlines)) {
        SpawnPreparedOutcome::Spawned(spawned) if spawned.expired => {
            StartOutcome::Expired { run: spawned.run }
        }
        SpawnPreparedOutcome::Spawned(spawned) => {
            let mut run = spawned.run;
            match run
                .confirm_start_until(deadlines.start_by(), deadlines.cleanup())
                .await
            {
                Ok(()) => StartOutcome::Ready { run },
                Err(StartConfirmationFailure::RestrictionSetup {
                    cleanup: crate::runner::CleanupState::Confirmed,
                }) => StartOutcome::Refused {
                    code: map_setup_failure(SetupFailureReason::RestrictionSetup),
                },
                Err(StartConfirmationFailure::RestrictionSetup {
                    cleanup: crate::runner::CleanupState::Unconfirmed,
                }) => StartOutcome::CleanupUnconfirmed,
                Err(StartConfirmationFailure::Deadline) => StartOutcome::Expired { run },
            }
        }
        SpawnPreparedOutcome::Expired(prepared) => {
            classify_expired_prepared(*prepared, deadlines.cleanup()).await
        }
        SpawnPreparedOutcome::Failed(_failed)
            if tokio::time::Instant::now() >= deadlines.start_by() =>
        {
            StartOutcome::Refused {
                code: FailureCode::ExecutionTimedOut,
            }
        }
        SpawnPreparedOutcome::Failed(failed) => StartOutcome::Refused {
            code: map_setup_failure(failed.reason),
        },
    }
}

async fn classify_expired_preparation(
    joined: Result<Result<PreparedSandboxRun, crate::runner::SetupFailed>, tokio::task::JoinError>,
    cleanup_deadline: tokio::time::Instant,
) -> StartOutcome {
    match joined {
        Ok(Ok(prepared)) => classify_expired_prepared(prepared, cleanup_deadline).await,
        Ok(Err(_)) | Err(_) => StartOutcome::Refused {
            code: FailureCode::ExecutionTimedOut,
        },
    }
}

async fn classify_expired_prepared(
    prepared: PreparedSandboxRun,
    cleanup_deadline: tokio::time::Instant,
) -> StartOutcome {
    if cleanup_prepared_until(prepared, cleanup_deadline).await {
        StartOutcome::Refused {
            code: FailureCode::ExecutionTimedOut,
        }
    } else {
        StartOutcome::CleanupUnconfirmed
    }
}

/// Perform the runner's complete side-effect-free request validation and map
/// its closed failure reason to the protocol failure vocabulary.
#[cfg(test)]
pub(crate) fn validate_request(
    runner: &SandboxRunner,
    request: SandboxRequest,
) -> Result<ValidatedSandboxRequest, FailureCode> {
    runner
        .validate_request(request)
        .map_err(|failed| map_setup_failure(failed.reason))
}

pub(crate) async fn validate_request_until(
    runner: &SandboxRunner,
    request: SandboxRequest,
    deadline: tokio::time::Instant,
) -> Result<ValidatedSandboxRequest, FailureCode> {
    let request = runner
        .validate_request_shape(request)
        .map_err(|failed| map_setup_failure(failed.reason))?;
    let runner = runner.clone();
    let mut validation =
        tokio::task::spawn_blocking(move || runner.validate_request_filesystem(request));
    match tokio::time::timeout_at(deadline, &mut validation).await {
        Err(_) => Err(FailureCode::ExecutionTimedOut),
        Ok(_) if tokio::time::Instant::now() >= deadline => Err(FailureCode::ExecutionTimedOut),
        Ok(Err(_)) => Err(FailureCode::Failed),
        Ok(Ok(result)) => result.map_err(|failed| map_setup_failure(failed.reason)),
    }
}

/// Build the SDK [`SandboxRequest`] from a wire [`ExecutePayload`]. Pure, so the
/// keystone stdin-isolation invariant ([`StdinPolicy::Null`]) and the field
/// mapping are unit-testable.
///
/// `NativeString` is converted back to the platform-native domain: Unix bytes
/// become `OsString` bytes verbatim, while Windows bytes are interpreted as
/// little-endian UTF-16 code units. No UTF-8 lossy conversion occurs.
pub(crate) fn build_request(
    exec: &ExecutePayload,
    cancel: CancellationToken,
) -> Result<SandboxRequest, FailureCode> {
    Ok(SandboxRequest {
        program: PathBuf::from(native_to_os_string(&exec.program)?),
        args: exec
            .args
            .iter()
            .map(native_to_os_string)
            .collect::<Result<Vec<_>, _>>()?,
        workspace: PathBuf::from(native_to_os_string(&exec.workspace)?),
        cwd: PathBuf::from(native_to_os_string(&exec.cwd)?),
        timeout: Duration::from_millis(exec.timeout_ms),
        env_inherit: exec.env_inherit,
        env_additions: exec
            .env_additions
            .iter()
            .map(|(key, value)| Ok((native_to_os_string(key)?, native_to_os_string(value)?)))
            .collect::<Result<_, FailureCode>>()?,
        // Protocol stdin is reserved (host->backend JSONL) and NEVER inherited
        // by the target (design `### State machine`: "the backend never inherits
        // protocol stdin as target stdin").
        stdin: StdinPolicy::Null,
        cancel: Some(cancel),
    })
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
        // The platform cannot establish the requested contract; the current
        // posture gate refuses before target start.
        SetupFailureReason::UnsupportedPlatform => FailureCode::Unavailable,
    }
}

/// Build the `started` frame from the effective mechanism/contract and the
/// platform limitations. `Mechanism::None` under an explicitly supplied
/// `NoRestriction` reports `supervised` / `unrestricted`; a native mechanism
/// (`Landlock`/`Seccomp` on supported Linux, `Seatbelt` on supported macOS)
/// reports `restricted` / `restricted` — NEVER `isolated` (crate vocabulary
/// contract, `lib.rs`; design `### Common profile`: the package reports
/// `restricted`).
pub(crate) fn started_payload(
    request_id: &RequestId,
    _mechanism: Mechanism,
    contract: ContractStatus,
    limitations: &[String],
) -> StartedPayload {
    let (guarantee, policy) = match contract {
        ContractStatus::Unrestricted => ("supervised", "unrestricted"),
        ContractStatus::Restricted => ("restricted", "restricted"),
    };
    StartedPayload {
        request_id: request_id.clone(),
        placement: "host".to_string(),
        guarantee: guarantee.to_string(),
        policy: policy.to_string(),
        limitations: limitations.to_vec(),
    }
}

/// Reconstruct the platform-native string domain carried by [`NativeString`].
#[cfg(unix)]
fn native_to_os_string(ns: &NativeString) -> Result<OsString, FailureCode> {
    use std::os::unix::ffi::OsStringExt;
    if ns.as_bytes().contains(&0) {
        return Err(FailureCode::ProtocolViolation);
    }
    Ok(OsString::from_vec(ns.as_bytes().to_vec()))
}

/// Windows native strings are serialized as little-endian UTF-16 code units,
/// including unpaired units. Odd byte lengths are malformed for this target.
#[cfg(windows)]
fn native_to_os_string(ns: &NativeString) -> Result<OsString, FailureCode> {
    use std::os::windows::ffi::OsStringExt;

    let mut chunks = ns.as_bytes().chunks_exact(2);
    let units = chunks
        .by_ref()
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    if !chunks.remainder().is_empty() || units.contains(&0) {
        return Err(FailureCode::ProtocolViolation);
    }
    Ok(OsString::from_wide(&units))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{NetworkPolicy, NoRestriction, Profile, SandboxPolicy};
    use crate::runner::{CleanupState, FaultInjection, PostSpawnGate, SandboxEvent};
    use futures_util::StreamExt;
    use opi_protocol::execution::v1::EnvInherit;

    fn rid() -> RequestId {
        RequestId::new("r1".to_string()).unwrap()
    }

    #[cfg(unix)]
    fn native(value: &str) -> NativeString {
        use std::os::unix::ffi::OsStrExt;
        NativeString::from_bytes(std::ffi::OsStr::new(value).as_bytes())
    }

    #[cfg(windows)]
    fn native(value: &str) -> NativeString {
        use std::os::windows::ffi::OsStrExt;
        NativeString::from_bytes(
            std::ffi::OsStr::new(value)
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
        )
    }

    #[cfg(windows)]
    #[test]
    fn native_conversion_preserves_unpaired_wide_units() {
        let expected = [0xD800u16, 0x0061];
        let bytes = expected
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect::<Vec<_>>();
        use std::os::windows::ffi::OsStrExt;
        let converted = native_to_os_string(&NativeString::from_bytes(bytes)).unwrap();
        let actual = converted.encode_wide().collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    fn exec(program: &str, timeout_ms: u64) -> ExecutePayload {
        ExecutePayload {
            request_id: rid(),
            program: native(program),
            args: vec![native("-c"), native("echo hi")],
            workspace: native("/ws"),
            cwd: native("/ws"),
            timeout_ms,
            env_inherit: EnvInherit::Inherit,
            env_additions: [(native("K"), native("v"))].into_iter().collect(),
        }
    }

    /// KEYSTONE: the backend target NEVER inherits protocol stdin.
    #[test]
    fn build_request_pins_stdin_to_null() {
        let cancel = CancellationToken::new();
        let request = build_request(&exec("sh", 1000), cancel).unwrap();
        assert_eq!(
            request.stdin,
            StdinPolicy::Null,
            "protocol stdin must never reach the target"
        );
    }

    #[test]
    fn build_request_maps_fields_and_cancel() {
        let cancel = CancellationToken::new();
        let request = build_request(&exec("sh", 7000), cancel.clone()).unwrap();
        assert_eq!(request.program, PathBuf::from("sh"));
        assert_eq!(
            request.args,
            vec![OsString::from("-c"), OsString::from("echo hi")]
        );
        assert_eq!(request.workspace, PathBuf::from("/ws"));
        assert_eq!(request.cwd, PathBuf::from("/ws"));
        assert_eq!(request.timeout, Duration::from_millis(7000));
        assert_eq!(request.env_inherit, EnvInherit::Inherit);
        assert_eq!(
            request
                .env_additions
                .get(std::ffi::OsStr::new("K"))
                .map(OsString::as_os_str),
            Some(std::ffi::OsStr::new("v"))
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

    /// A native mechanism (16.13 Landlock/Seccomp, 16.14.1 Seatbelt) reports
    /// the honest `restricted` / `restricted` vocabulary, never
    /// `isolated`/`enforced`.
    #[test]
    fn started_payload_native_reports_restricted() {
        for mechanism in [Mechanism::Landlock, Mechanism::Seccomp, Mechanism::Seatbelt] {
            let frame = started_payload(&rid(), mechanism, ContractStatus::Restricted, &[]);
            assert_eq!(frame.placement, "host");
            assert_eq!(frame.guarantee, "restricted");
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
    #[tokio::test]
    async fn start_refuses_unsupported_platform_with_unavailable() {
        let runner = SandboxRunner::new(
            SandboxPolicy::new(Profile::WorkspaceWrite, NetworkPolicy::Deny),
            std::sync::Arc::new(crate::NoRestriction),
        );
        let cancel = CancellationToken::new();
        let mut request = build_request(&exec("sh", 1000), cancel).unwrap();
        let workspace = tempfile::tempdir().expect("workspace");
        request.workspace = workspace.path().to_path_buf();
        request.cwd = workspace.path().to_path_buf();
        let request = validate_request(&runner, request).expect("valid request");
        let now = tokio::time::Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_secs(1),
            now + Duration::from_secs(2),
            Duration::from_secs(1),
        );
        match start(false, &runner, request, deadlines).await {
            StartOutcome::Refused {
                code: FailureCode::Unavailable,
            } => {}
            _ => panic!("expected Refused(Unavailable) on unsupported platform"),
        }
    }

    #[tokio::test]
    async fn late_gated_run_is_returned_for_confirmed_cleanup() {
        let marker_dir = tempfile::tempdir().expect("marker dir");
        let marker = marker_dir.path().join("must-not-exist");
        let workspace = tempfile::tempdir().expect("workspace");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("powershell"),
                vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-Command"),
                    OsString::from(format!(
                        "Set-Content -LiteralPath '{}' -Value x",
                        marker.display()
                    )),
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![
                    OsString::from("-c"),
                    OsString::from(format!("printf x > '{}'", marker.display())),
                ],
            )
        };
        let cancel = CancellationToken::new();
        // Force the post-spawn expiry branch deterministically. A short real-
        // time cutoff is flaky when parallel Windows process creation delays
        // setup before this test reaches the branch it intends to exercise.
        let post_spawn_gate: &'static PostSpawnGate = Box::leak(Box::new(PostSpawnGate::new()));
        let cancel_after_spawn = cancel.clone();
        let gate_worker = std::thread::spawn(move || {
            post_spawn_gate.cancel_after_spawn(&cancel_after_spawn);
        });
        let request = SandboxRequest {
            program,
            args,
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            env_inherit: EnvInherit::Inherit,
            env_additions: Default::default(),
            stdin: StdinPolicy::Null,
            cancel: Some(cancel.clone()),
        };
        let runner =
            SandboxRunner::new(SandboxPolicy::default(), std::sync::Arc::new(NoRestriction))
                .with_faults(FaultInjection {
                    post_spawn_gate: Some(post_spawn_gate),
                    ..FaultInjection::default()
                });
        let request = validate_request(&runner, request).expect("valid request");
        let now = tokio::time::Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_secs(5),
            now + Duration::from_secs(10),
            Duration::from_secs(5),
        );

        let mut run = match start(true, &runner, request, deadlines).await {
            StartOutcome::Expired { run } => run,
            _ => panic!("late gated run must be returned for cleanup"),
        };
        gate_worker.join().expect("post-spawn cancellation worker");
        let temp_root = run.temp_root().to_path_buf();
        run.keep_gated();
        assert!(matches!(
            run.next().await,
            Some(SandboxEvent::Started { .. })
        ));
        let result = match run.next().await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected completed cleanup, got {other:?}"),
        };

        assert_eq!(result.cleanup, CleanupState::Confirmed);
        assert!(!marker.exists(), "late run crossed its release gate");
        assert!(!temp_root.exists(), "late run temp root was not removed");
    }

    #[tokio::test]
    async fn filesystem_validation_is_bounded_before_admission() {
        let marker_dir = tempfile::tempdir().expect("marker dir");
        let marker = marker_dir.path().join("must-not-exist");
        let workspace = tempfile::tempdir().expect("workspace");
        let request = SandboxRequest {
            program: PathBuf::from("target-must-not-run"),
            args: Vec::new(),
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            env_inherit: EnvInherit::Inherit,
            env_additions: Default::default(),
            stdin: StdinPolicy::Null,
            cancel: Some(CancellationToken::new()),
        };
        let runner =
            SandboxRunner::new(SandboxPolicy::default(), std::sync::Arc::new(NoRestriction))
                .with_faults(FaultInjection {
                    validation_delay: Duration::from_millis(200),
                    ..FaultInjection::default()
                });
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);

        let result = validate_request_until(&runner, request, deadline).await;

        assert!(matches!(result, Err(FailureCode::ExecutionTimedOut)));
        assert!(!marker.exists(), "validation timeout mutated target state");
    }

    fn validated_exit_request(
        runner: &SandboxRunner,
        cancel: CancellationToken,
    ) -> (ValidatedSandboxRequest, tempfile::TempDir) {
        let workspace = tempfile::tempdir().expect("workspace");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("cmd"),
                vec![OsString::from("/C"), OsString::from("exit 0")],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![OsString::from("-c"), OsString::from("exit 0")],
            )
        };
        let request = SandboxRequest {
            program,
            args,
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            env_inherit: EnvInherit::Inherit,
            env_additions: Default::default(),
            stdin: StdinPolicy::Null,
            cancel: Some(cancel),
        };
        (
            validate_request(runner, request).expect("valid request"),
            workspace,
        )
    }

    #[tokio::test]
    async fn expired_preparation_cleanup_is_bounded_by_original_hard_deadline() {
        let runner =
            SandboxRunner::new(SandboxPolicy::default(), std::sync::Arc::new(NoRestriction))
                .with_faults(FaultInjection {
                    prepared_delivery_delay: Duration::from_millis(150),
                    prepared_temp_remove_delay: Duration::from_secs(1),
                    ..FaultInjection::default()
                });
        let cancel = CancellationToken::new();
        let (request, _workspace) = validated_exit_request(&runner, cancel);
        let now = tokio::time::Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_millis(100),
            now + Duration::from_millis(300),
            Duration::from_secs(5),
        );
        let wall_start = std::time::Instant::now();

        let outcome = start(true, &runner, request, deadlines).await;

        assert!(
            wall_start.elapsed() < Duration::from_millis(600),
            "prepared cleanup exceeded the original hard deadline"
        );
        assert!(matches!(outcome, StartOutcome::CleanupUnconfirmed));
    }

    #[tokio::test]
    async fn expired_preparation_with_confirmed_cleanup_stays_execution_timed_out() {
        let runner =
            SandboxRunner::new(SandboxPolicy::default(), std::sync::Arc::new(NoRestriction))
                .with_faults(FaultInjection {
                    prepared_delivery_delay: Duration::from_millis(150),
                    prepared_temp_remove_delay: Duration::from_millis(50),
                    ..FaultInjection::default()
                });
        let cancel = CancellationToken::new();
        let (request, _workspace) = validated_exit_request(&runner, cancel);
        let now = tokio::time::Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_millis(100),
            now + Duration::from_millis(500),
            Duration::from_secs(5),
        );

        let outcome = start(true, &runner, request, deadlines).await;

        assert!(matches!(
            outcome,
            StartOutcome::Refused {
                code: FailureCode::ExecutionTimedOut
            }
        ));
    }

    #[tokio::test]
    async fn expired_preparation_with_failed_cleanup_is_unconfirmed() {
        let runner =
            SandboxRunner::new(SandboxPolicy::default(), std::sync::Arc::new(NoRestriction))
                .with_faults(FaultInjection {
                    prepared_delivery_delay: Duration::from_millis(150),
                    prepared_temp_remove_fail: true,
                    ..FaultInjection::default()
                });
        let cancel = CancellationToken::new();
        let (request, _workspace) = validated_exit_request(&runner, cancel);
        let now = tokio::time::Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_millis(100),
            now + Duration::from_millis(500),
            Duration::from_secs(5),
        );

        let outcome = start(true, &runner, request, deadlines).await;

        assert!(matches!(outcome, StartOutcome::CleanupUnconfirmed));
    }
}

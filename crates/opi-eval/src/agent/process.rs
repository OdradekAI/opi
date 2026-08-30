//! Crate-private shared Agent execution contract (Phase 18 task 18.6).
//!
//! [`AgentExecution`] consumes [`crate::process::ProcessSupervisor`] and one
//! [`AgentAdapter`] and produces a single settled [`AgentRecord`]: exit state,
//! bounded captures, cleanup evidence, native artifacts, usage facts with
//! typed unknown reasons, and an authoritative completion verdict mapped into
//! [`crate::failure::FailureBoundaryCode`]. The contract is product-neutral:
//! argv, environment projection, native schemas, and completion semantics are
//! owned by each adapter, never by this module.

use crate::failure::FailureBoundaryCode;
use crate::process::{
    CleanupEvidence, ExitState, OutputCapture, ProcessSupervisor, SpawnReason, SpawnSpec,
};
use crate::runner::lifecycle::{CancellationSource, SettlementKind};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Exact Agent identity retained on every settled record (P18-AGT-001).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AgentIdentity {
    /// Product name, e.g. `opi` or `pi`.
    pub product: String,
    /// Package providing the executable.
    pub package: String,
    /// Adapter identity plus its contract identity.
    pub adapter: String,
    /// Executable the supervisor will spawn.
    pub executable: PathBuf,
}

impl std::fmt::Debug for AgentIdentity {
    /// Redacted: the executable path never enters diagnostics. Spawn
    /// failures already carry only static tokens; identity diagnostics keep
    /// that guarantee for the executable location itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentIdentity")
            .field("product", &self.product)
            .field("package", &self.package)
            .field("adapter", &self.adapter)
            .field("executable", &"<redacted-path>")
            .finish()
    }
}

/// A capability an adapter declares. A missing capability stays visible: the
/// shared contract never requires pi Harness v2 or Opi evidence vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentCapability {
    /// Native NDJSON event stream on stdout.
    JsonEvents,
    /// Durable evidence manifest capture (Opi Phase 17 evidence).
    EvidenceManifest,
    /// Provider-reported usage facts.
    UsageFacts,
}

/// One frozen run request: bounded task input, isolated workspace, exact
/// environment extras, and the resolved executable. Limits come from the
/// adapter's declarative profile.
#[derive(Debug, Clone)]
pub(crate) struct AgentRunRequest {
    /// Resolved executable to spawn (the exact built binary in conformance;
    /// a scripted helper in unit tests). Never resolved from ambient PATH.
    pub executable: PathBuf,
    /// Task prompt handed to the Agent (single one-shot prompt).
    pub prompt: String,
    /// Fresh isolated workspace; also the child's cwd.
    pub workspace: PathBuf,
    /// Fresh trace root for evidence capture.
    pub trace_root: PathBuf,
    /// Explicit benchmark configuration file (never ambient user config).
    pub config_path: PathBuf,
    /// Exact `provider:model` selection; never empty, never a fallback.
    pub provider_model: String,
    /// Opt-in for mutating tools; `false` keeps the reviewed read-only profile.
    pub allow_mutating: bool,
    /// Isolated directories the adapter projects into the child environment.
    pub isolation: IsolationDirs,
    /// Additional exact environment entries (e.g. a local scripted provider).
    pub extra_env: BTreeMap<OsString, OsString>,
}

/// Fresh per-trial isolation directories (P18-AGT-006).
#[derive(Debug, Clone)]
pub(crate) struct IsolationDirs {
    pub home: PathBuf,
    pub app_data: PathBuf,
    pub sessions: PathBuf,
}

/// A measured fact: known with origin retained, or unknown with a typed
/// reason. An unknown fact is never silently converted to zero or dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fact {
    Known { value: u64, origin: String },
    Unknown { reason: String },
}

/// Provider usage projection retained on the settled record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UsageProjection {
    pub input_tokens: Option<Fact>,
    pub output_tokens: Option<Fact>,
}

/// Native artifacts an adapter imported for one run, as content-addressed
/// references (never inlined bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeArtifact {
    /// Stable role name, e.g. `native/stdout`, `evidence/manifest`.
    pub role: String,
    /// sha256 of the exact bytes.
    pub sha256: String,
    /// Where the bytes live (importer copies; never mutates the source).
    pub path: PathBuf,
}

/// Typed failure carried on a settled record. A failed run is settled
/// evidence, not an unsettable error: the run happened and the verdict is
/// authoritative (P18-AGT-003 fail-closed, no fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentFailure {
    /// Static redacted token describing the failure kind.
    pub kind: &'static str,
    /// Owning failure boundary.
    pub boundary: FailureBoundaryCode,
}

/// Authoritative completion verdict of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentCompletion {
    /// The adapter's completion predicate held with imported evidence.
    Completed { artifacts: Vec<NativeArtifact> },
    /// The run settled as a typed failure.
    Failed(AgentFailure),
}

/// The settled Agent record: one per run, on every path.
#[derive(Debug, Clone)]
pub(crate) struct AgentRecord {
    pub identity: AgentIdentity,
    pub workspace: PathBuf,
    pub exit: ExitState,
    pub stdout: OutputCapture,
    pub stderr: OutputCapture,
    pub cleanup: CleanupEvidence,
    /// Wall-clock duration of the supervised run.
    pub wall_time: Duration,
    pub usage: UsageProjection,
    pub completion: AgentCompletion,
}

impl AgentRecord {
    /// The lifecycle settlement observation for this record (P18-DUR-003).
    /// The cancellation source is caller-owned: only the caller knows whether
    /// the token fired for a user or infrastructure reason.
    pub(crate) fn settlement_kind(&self, cancel_source: CancellationSource) -> SettlementKind {
        match self.exit {
            ExitState::Exited { code } => SettlementKind::Exited { code },
            ExitState::TimedOut => SettlementKind::TimedOut,
            ExitState::Cancelled => SettlementKind::Cancelled {
                source: cancel_source,
            },
            ExitState::FailedToSpawn { .. } => SettlementKind::Exited { code: -1 },
        }
    }

    /// The failure boundary of a failed completion, or `None` when completed.
    pub(crate) fn failure_boundary(&self) -> Option<FailureBoundaryCode> {
        match &self.completion {
            AgentCompletion::Completed { .. } => None,
            AgentCompletion::Failed(failure) => Some(failure.boundary),
        }
    }
}

/// The product-neutral product-specific seam: one implementation per Agent.
/// Crate-private because the only consumers are this crate's execution driver
/// and the shared conformance suite (18.7 pi adapter, 18.10.1 conformance).
pub(crate) trait AgentAdapter {
    /// Exact identity declared by the declarative profile + resolved request.
    fn identity(&self, request: &AgentRunRequest) -> AgentIdentity;

    /// Capabilities this adapter declares.
    fn capabilities(&self) -> &'static [AgentCapability];

    /// Structured spawn request for one run. Owns argv, cwd, environment
    /// projection, and limits from the declarative profile.
    fn spawn_spec(&self, request: &AgentRunRequest) -> SpawnSpec;

    /// Settle one supervised outcome into the native half of the record:
    /// imported artifacts, usage facts, and the authoritative completion
    /// verdict. Infallible: request validation happened at [`AgentExecution::run`]
    /// entry, and every observed run failure settles inside the record.
    fn settle(
        &self,
        outcome: &crate::process::SupervisedOutcome,
        request: &AgentRunRequest,
    ) -> (UsageProjection, AgentCompletion);
}

/// Pre-spawn contract violation: the request itself cannot produce a legal
/// spawn. Static tokens only; request bytes are not echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionError {
    pub token: &'static str,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "agent execution rejected: {}", self.token)
    }
}

impl std::error::Error for ExecutionError {}

/// The shared execution driver. One run, one settled record, every path.
pub(crate) struct AgentExecution;

impl AgentExecution {
    /// Run `request` under `adapter` to settlement.
    ///
    /// Never panics and never returns an `Err` for an observed run: spawn
    /// failures, non-zero exits, timeouts, cancellations, and evidence
    /// failures all settle inside the [`AgentRecord`]. `Err` means the
    /// request itself was structurally unusable before any process existed.
    pub(crate) async fn run(
        request: &AgentRunRequest,
        adapter: &dyn AgentAdapter,
        cancel: &CancellationToken,
    ) -> Result<AgentRecord, ExecutionError> {
        if request.prompt.trim().is_empty() {
            return Err(ExecutionError {
                token: "empty-prompt",
            });
        }
        if request.provider_model.split(':').count() != 2
            || request.provider_model.split(':').any(|s| s.is_empty())
        {
            return Err(ExecutionError {
                token: "provider-model-must-be-exact",
            });
        }
        let identity = adapter.identity(request);
        let spec = adapter.spawn_spec(request);
        let started = Instant::now();
        let outcome = ProcessSupervisor::run(&spec, cancel).await;
        let wall_time = started.elapsed();
        let (usage, completion) = adapter.settle(&outcome, request);
        Ok(AgentRecord {
            identity,
            workspace: request.workspace.clone(),
            exit: outcome.exit,
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            cleanup: outcome.cleanup,
            wall_time,
            usage,
            completion,
        })
    }
}

/// Failure kinds for settled records, kept as static tokens.
pub(crate) mod failure_kinds {
    use crate::failure::FailureBoundaryCode;

    /// Map a spawn failure to its settled failure token + boundary.
    pub(crate) fn spawn(reason: super::SpawnReason) -> super::AgentFailure {
        let token = match reason {
            super::SpawnReason::NotFound => "spawn-not-found",
            super::SpawnReason::PermissionDenied => "spawn-permission-denied",
            super::SpawnReason::BadCwd => "spawn-bad-cwd",
            super::SpawnReason::SpawnFailed => "spawn-failed",
        };
        super::AgentFailure {
            kind: token,
            boundary: FailureBoundaryCode::AgentProcess,
        }
    }

    /// Non-zero Agent exit.
    pub(crate) fn non_zero_exit(code: i32) -> super::AgentFailure {
        let _ = code;
        super::AgentFailure {
            kind: "agent-non-zero-exit",
            boundary: FailureBoundaryCode::AgentProcess,
        }
    }

    /// Timeout settled by the supervisor.
    pub(crate) const TIMED_OUT: super::AgentFailure = super::AgentFailure {
        kind: "agent-timeout",
        boundary: FailureBoundaryCode::AgentProcess,
    };

    /// Cancellation settled by the supervisor.
    pub(crate) const CANCELLED: super::AgentFailure = super::AgentFailure {
        kind: "agent-cancelled",
        boundary: FailureBoundaryCode::AgentProcess,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A helper adapter proving the shared driver without any product
    /// semantics: argv is a real process (`/bin/true`-style), settle maps
    /// exit 0 to Completed and everything else to the shared failure kinds.
    struct HelperAdapter;

    impl AgentAdapter for HelperAdapter {
        fn identity(&self, _request: &AgentRunRequest) -> AgentIdentity {
            AgentIdentity {
                product: "helper".to_owned(),
                package: "helper-package".to_owned(),
                adapter: "helper-adapter/0".to_owned(),
                executable: "/nonexistent/helper".into(),
            }
        }

        fn capabilities(&self) -> &'static [AgentCapability] {
            &[AgentCapability::JsonEvents]
        }

        fn spawn_spec(&self, request: &AgentRunRequest) -> SpawnSpec {
            SpawnSpec {
                argv: vec!["/usr/bin/true".into()],
                cwd: Some(request.workspace.clone()),
                env: BTreeMap::new(),
                stdout_cap: 1024,
                stderr_cap: 1024,
                timeout: Duration::from_secs(10),
            }
        }

        fn settle(
            &self,
            outcome: &crate::process::SupervisedOutcome,
            _request: &AgentRunRequest,
        ) -> (UsageProjection, AgentCompletion) {
            let completion = match outcome.exit {
                ExitState::Exited { code: 0 } => AgentCompletion::Completed { artifacts: vec![] },
                ExitState::Exited { code } => {
                    AgentCompletion::Failed(failure_kinds::non_zero_exit(code))
                }
                ExitState::TimedOut => AgentCompletion::Failed(failure_kinds::TIMED_OUT),
                ExitState::Cancelled => AgentCompletion::Failed(failure_kinds::CANCELLED),
                ExitState::FailedToSpawn { reason } => {
                    AgentCompletion::Failed(failure_kinds::spawn(reason))
                }
            };
            (UsageProjection::default(), completion)
        }
    }

    fn request() -> AgentRunRequest {
        AgentRunRequest {
            executable: "/nonexistent/opi".into(),
            prompt: "do the task".to_owned(),
            workspace: std::env::temp_dir(),
            trace_root: std::env::temp_dir(),
            config_path: std::env::temp_dir().join("bench.toml"),
            provider_model: "local:scripted".to_owned(),
            allow_mutating: false,
            isolation: IsolationDirs {
                home: std::env::temp_dir(),
                app_data: std::env::temp_dir(),
                sessions: std::env::temp_dir(),
            },
            extra_env: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn empty_prompt_and_non_exact_model_reject_before_spawn() {
        let mut bad = request();
        bad.prompt = "   ".to_owned();
        assert_eq!(
            AgentExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "empty-prompt"
            }
        );

        for model in ["", "local", ":scripted", "local:", "a:b:c"] {
            let mut bad = request();
            bad.provider_model = model.to_owned();
            assert_eq!(
                AgentExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                    .await
                    .unwrap_err(),
                ExecutionError {
                    token: "provider-model-must-be-exact"
                }
            );
        }
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn helper_run_settles_a_completed_record_with_identity() {
        let record = AgentExecution::run(&request(), &HelperAdapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 0 });
        assert_eq!(record.cleanup, CleanupEvidence::NotRequired);
        assert_eq!(
            record.completion,
            AgentCompletion::Completed { artifacts: vec![] }
        );
        assert_eq!(record.identity.product, "helper");
        assert_eq!(record.failure_boundary(), None);
        // The lifecycle bridge keeps exit codes exact.
        assert_eq!(
            record.settlement_kind(CancellationSource::User),
            SettlementKind::Exited { code: 0 }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn helper_nonzero_exit_settles_as_typed_agent_failure() {
        struct FailingHelper(HelperAdapter);
        impl AgentAdapter for FailingHelper {
            fn identity(&self, r: &AgentRunRequest) -> AgentIdentity {
                self.0.identity(r)
            }
            fn capabilities(&self) -> &'static [AgentCapability] {
                self.0.capabilities()
            }
            fn spawn_spec(&self, r: &AgentRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sh".into(), "-c".into(), "exit 3".into()];
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &AgentRunRequest,
            ) -> (UsageProjection, AgentCompletion) {
                self.0.settle(o, r)
            }
        }
        let record = AgentExecution::run(
            &request(),
            &FailingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 3 });
        assert_eq!(
            record.completion,
            AgentCompletion::Failed(failure_kinds::non_zero_exit(3))
        );
        assert_eq!(
            record.failure_boundary(),
            Some(FailureBoundaryCode::AgentProcess)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn helper_timeout_settles_with_cleanup_and_typed_failure() {
        struct SleepingHelper(HelperAdapter);
        impl AgentAdapter for SleepingHelper {
            fn identity(&self, r: &AgentRunRequest) -> AgentIdentity {
                self.0.identity(r)
            }
            fn capabilities(&self) -> &'static [AgentCapability] {
                self.0.capabilities()
            }
            fn spawn_spec(&self, r: &AgentRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sleep".into(), "15".into()];
                spec.timeout = Duration::from_millis(200);
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &AgentRunRequest,
            ) -> (UsageProjection, AgentCompletion) {
                self.0.settle(o, r)
            }
        }
        let record = AgentExecution::run(
            &request(),
            &SleepingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(record.exit, ExitState::TimedOut);
        assert_eq!(
            record.completion,
            AgentCompletion::Failed(failure_kinds::TIMED_OUT)
        );
        assert_eq!(
            record.failure_boundary(),
            Some(FailureBoundaryCode::AgentProcess)
        );
        assert!(matches!(
            record.cleanup,
            CleanupEvidence::TreeTerminated { verified: true, .. }
        ));
        assert_eq!(
            record.settlement_kind(CancellationSource::Infrastructure),
            SettlementKind::TimedOut
        );
    }
}

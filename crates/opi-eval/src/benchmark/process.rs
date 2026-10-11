//! Crate-private shared benchmark execution contract.
//!
//! [`BenchmarkExecution`] consumes [`crate::process::ProcessSupervisor`], an
//! admitted external lock digest, a read-only [`crate::integrity::IntegrityRecord`],
//! and one [`BenchmarkAdapter`] to produce a single settled
//! [`BenchmarkRecord`]: exit state, bounded captures, cleanup evidence,
//! native metrics with their benchmark-defined names, the native reward as a
//! typed fact, content-addressed native artifacts, and a typed completion
//! verdict mapped into [`crate::failure::FailureBoundaryCode`]. The contract
//! is benchmark-neutral: task-package shape, verifier argv, native schemas,
//! and failure mapping are owned by each adapter, never by this module
//! (`EVAL-BMK-005`, `EVAL-BMK-007`).

use crate::agent::process::{Fact, NativeArtifact};
use crate::failure::FailureBoundaryCode;
use crate::integrity::IntegrityRecord;
use crate::process::{
    CleanupEvidence, ExitState, OutputCapture, ProcessSupervisor, SpawnReason, SpawnSpec,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Exact benchmark-verifier identity retained on every settled record
/// (`EVAL-BMK-001`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkIdentity {
    /// Benchmark family, e.g. `terminal-bench`.
    pub benchmark: String,
    /// Admitted revision identity, e.g. `2.1`.
    pub revision: String,
    /// Adapter identity plus its contract identity.
    pub adapter: String,
}

/// One frozen verification request: the exact resolved verifier executable,
/// the materialized task package, the admitted external lock digest, the
/// immutable integrity admission, and - in native mode - the explicitly
/// selected Agent integration the dispatched job must run and bind to.
/// The profile bounds the whole process; native stage caps narrow task limits.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkRunRequest {
    /// Resolved verifier entry executable (argv[0]; e.g. the pinned `uv`
    /// that drives `harbor --locked`). Never resolved from ambient PATH.
    pub verifier_executable: PathBuf,
    /// Materialized complete task package root (validated by the adapter).
    pub task_dir: PathBuf,
    /// Task id inside the admitted revision.
    pub task_id: String,
    /// The explicitly selected native integration: when present, the
    /// spawned upstream job runs this Agent inside the graded task
    /// environment and the settled result must bind to it (P19-D01).
    pub native_selection: Option<NativeSelection>,
    /// Accepted native Agent and verifier caps, independent of the job watchdog.
    pub agent_timeout_seconds: Option<u64>,
    pub verifier_timeout_seconds: Option<u64>,
    /// Fresh capture root for native verifier outputs.
    pub trace_root: PathBuf,
    /// Digest of the admitted resolved external lock.
    /// The external-lock verifier scripts bind this digest to the admitted
    /// lock under `crates/opi-eval/external-locks/`.
    pub admitted_lock_digest: String,
    /// Immutable integrity record admitting the revision and classifying the
    /// task. Consumed read-only; no adapter or Agent path can mutate it
    /// (`EVAL-INT-003`).
    pub integrity: IntegrityRecord,
    /// Additional exact environment entries.
    pub extra_env: BTreeMap<OsString, OsString>,
}

/// Native metrics with their benchmark-defined names retained verbatim
/// (`EVAL-BMK-007`): one fact per CTRF summary counter, never a composite
/// score, never renamed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NativeMetrics {
    pub tests: Option<Fact>,
    pub passed: Option<Fact>,
    pub failed: Option<Fact>,
    pub skipped: Option<Fact>,
    pub pending: Option<Fact>,
    pub other: Option<Fact>,
}

/// Projects only explicitly named native test counters. The binary task reward
/// remains independent; opaque reward fields retain their native names. Invalid
/// trials cannot contribute a measured fraction even when raw counters exist.
pub(crate) fn native_grader_observation(
    result: Option<&serde_json::Value>,
    digest: Option<&str>,
    graded: bool,
) -> serde_json::Value {
    let fields = result.map(|value| &value["verifier_result"]["rewards"]);
    let passed = fields.and_then(|fields| native_count(&fields["passed"]));
    let total = fields.and_then(|fields| native_count(&fields["total"]));
    let reason = if !graded {
        Some("invalid-trial")
    } else if result.is_none() || digest.is_none() {
        Some("native-result-not-observed")
    } else if fields.is_none_or(|fields| fields["passed"].is_null() || fields["total"].is_null()) {
        Some("native-test-counts-missing")
    } else {
        match (passed, total) {
            (Some(0), Some(0)) => Some("native-test-total-zero"),
            (Some(passed), Some(total)) if passed <= total => None,
            _ => Some("native-test-counts-invalid"),
        }
    };
    serde_json::json!({
        "test_case_fraction": {
            "state": if reason.is_none() { "measured" } else { "unknown" },
            "reason": reason,
            "numerator": passed,
            "denominator": total,
            "value": if reason.is_none() {
                Some(passed.expect("validated counter") as f64 / total.expect("validated counter") as f64)
            } else { None },
        },
        "native_fields": fields,
        "native_source": digest.map(|digest| serde_json::json!({
            "artifact": "native/trial-result.json", "digest": digest,
        })),
    })
}

/// Native reward maps may encode counters as floats. Only exact nonnegative
/// integers in the common lossless JSON-number range are eligible.
fn native_count(value: &serde_json::Value) -> Option<u64> {
    let count = value.as_f64()?;
    (count.is_finite() && (0.0..=9_007_199_254_740_991.0).contains(&count) && count.fract() == 0.0)
        .then_some(count as u64)
}

/// What the dispatch selected: the pinned opi-eval integration Agent, or
/// an explicitly selected upstream diagnostic witness (oracle/nop). A
/// witness is conformance evidence that never becomes an Agent score and
/// carries no integration terminal observation (P19-D03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectionKind {
    Integration {
        product: String,
        executable_sha256: String,
        trial: String,
        runtime_package_manifest_sha256: Option<String>,
        runtime_package_entrypoint: Option<String>,
    },
    DiagnosticWitness,
}

/// The admitted task lifecycle the dispatch binds against: the resolved
/// verifier environment mode and the task's declared collect list, parsed
/// from the admitted task package (P19-D03). The Pier family additionally
/// binds the committed-patch handoff identity - the base revision and the
/// environment image - so a bound job cannot silently rebase or reimage
/// the graded patch (P19-D04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskLifecycleSelection {
    pub(crate) environment_mode: super::task_lifecycle::VerifierEnvironmentMode,
    pub(crate) declared: Vec<super::task_lifecycle::DeclaredArtifact>,
    /// The base git revision the Pier collect hook diffs against and the
    /// environment image the pristine verifier rebuilds from (Pier family
    /// only; None for the Harbor route).
    pub(crate) base_revision: Option<String>,
    pub(crate) docker_image: Option<String>,
}

/// The explicitly selected native integration identity one dispatched
/// upstream job must bind to, plus the per-dispatch staleness defense
/// (P19-D01, P19-AC-002). Carried on [`BenchmarkRunRequest`] whenever the
/// spawned job must run a selected Agent inside the graded task
/// environment; hermetic requests carry `None`.
#[derive(Debug, Clone)]
pub(crate) struct NativeSelection {
    /// What kind of selection this dispatch made.
    pub(crate) kind: SelectionKind,
    /// The task lifecycle this dispatch admitted.
    pub(crate) lifecycle: TaskLifecycleSelection,
    /// Agent name the selected integration reports in the trial's
    /// `agent_info.name` (the integration's `name()`); `oracle` or any
    /// other name is a typed rejection even on job exit 0.
    pub(crate) agent_name: String,
    /// Integration import path (`module:ClassName`), or the pinned diagnostic
    /// class whose registered name is selected in the upstream job config.
    pub(crate) agent_import_path: String,
    /// The admitted task name every bound trial must report
    /// (`task_name`).
    pub(crate) task_name: String,
    /// Model identity passed to the upstream job and reported in
    /// `agent_info.model_info.name`.
    pub(crate) model: String,
    /// Reward metric name the bound trial's verifier result must carry
    /// (Terminal-Bench: `reward`).
    pub(crate) reward_metric: String,
    /// Directory holding the pinned integration module; prepended to the
    /// spawned job's `PYTHONPATH` so the import path resolves.
    pub(crate) module_dir: PathBuf,
    /// Host-side integration handoff file path, exported to the job as
    /// `OPI_EVAL_NATIVE_HANDOFF` (never inside the task environment and
    /// never sealed: it carries the projected credential material).
    pub(crate) handoff_path: PathBuf,
    /// Job directory names that already existed under the trace root
    /// before this dispatch; the imported job must be a new one.
    pub(crate) stale_job_names: Vec<OsString>,
}

/// One file the selected Agent delivered inside the task environment, as
/// observed by the integration after the Agent phase.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliveredFile {
    /// Absolute source path inside the task environment.
    pub(crate) source: String,
    /// sha256 of the observed bytes.
    pub(crate) sha256: String,
}

/// The typed native Agent terminal observation the pinned integration
/// records next to its Agent logs (host side, never inside the task
/// environment). This is the integration's own contract, so it parses
/// deterministically.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeTerminalObservation {
    pub(crate) schema: String,
    /// Logical trial identity retained independently of the upstream job name.
    pub(crate) trial: String,
    /// Selected Agent product (`opi` or `pi`).
    pub(crate) product: String,
    /// sha256 of the exact Agent executable the integration executed.
    pub(crate) agent_executable_sha256: String,
    /// sha256 of the staged capture helper (schema /3 only; legacy /2
    /// sealed bundles predate the first-sink boundary).
    pub(crate) capture_helper_sha256: Option<String>,
    /// Observed Agent process exit code.
    pub(crate) exit_code: Option<i64>,
    /// Whether the integration issued the task command after runtime admission.
    pub(crate) task_dispatched: bool,
    /// Exact staged-runtime version probe, separate from the task outcome.
    pub(crate) runtime_preflight: RuntimePreflightObservation,
    /// Task-side capture helper self-check (schema /3 only).
    pub(crate) helper_preflight: Option<HelperPreflightObservation>,
    /// First-sink capture summary bound to the receipt (schema /3 only).
    pub(crate) capture: Option<CaptureSummaryObservation>,
    /// Whether the Agent phase hit the task's Agent timeout.
    pub(crate) timed_out: bool,
    /// Files the Agent delivered, with observed digests.
    pub(crate) delivered: Vec<DeliveredFile>,
}

/// Task-side capture helper self-check proof (P19-AUD-003): the staged
/// helper proves its own protocol identity inside the task environment
/// before any credential projection or dispatch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HelperPreflightObservation {
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i64>,
    pub(crate) command_sha256: String,
    pub(crate) stdout_sha256: String,
    pub(crate) stderr_sha256: String,
    pub(crate) stdout_complete: bool,
    pub(crate) stderr_complete: bool,
    pub(crate) stdout_truncated: bool,
    pub(crate) stderr_truncated: bool,
    pub(crate) reason: Option<String>,
}

/// The capture-receipt summary the terminal proof binds (P19-AUD-003).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaptureSummaryObservation {
    /// `sanitized`, `refused`, `incomplete`, or `unreadable`.
    pub(crate) status: String,
    /// `full` or `terminal-only`.
    pub(crate) policy: Option<String>,
    /// sha256 of the captured receipt bytes.
    pub(crate) receipt_sha256: Option<String>,
}

/// Integration-owned runtime compatibility proof. Closed vocabularies and
/// state combinations are validated before any proof is admitted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimePreflightObservation {
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i64>,
    pub(crate) agent_executable_sha256: String,
    pub(crate) runtime_package_manifest_sha256: Option<String>,
    pub(crate) runtime_package_entrypoint: Option<String>,
    pub(crate) command_sha256: String,
    pub(crate) stdout_sha256: String,
    pub(crate) stderr_sha256: String,
    pub(crate) stdout_complete: bool,
    pub(crate) stderr_complete: bool,
    pub(crate) stdout_truncated: bool,
    pub(crate) stderr_truncated: bool,
    pub(crate) reason: Option<String>,
}

/// Bound job and integration observations without a reward or collection
/// requirement. Setup refusal can retain this evidence without inventing a grade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTrialObservation {
    pub(crate) job_name: OsString,
    pub(crate) trial_name: String,
    pub(crate) agent_name: String,
    pub(crate) agent_version: String,
    pub(crate) model_name: Option<String>,
    pub(crate) task_name: String,
    pub(crate) exception_type: Option<String>,
    pub(crate) terminal: Option<NativeTerminalObservation>,
    pub(crate) results_path: PathBuf,
    pub(crate) config_path: PathBuf,
}

/// The bound identity of one imported native job: everything the sealed
/// evidence needs to attribute the native grade to the selected Agent's
/// same-trial work (P19-AC-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTrialBinding {
    /// The new `jobs/<name>` directory the dispatch produced.
    pub(crate) job_name: OsString,
    /// Trial name inside the job (`<trial>`).
    pub(crate) trial_name: String,
    /// Actual Agent name reported by the trial (`agent_info.name`).
    pub(crate) agent_name: String,
    /// Agent version reported by the trial.
    pub(crate) agent_version: String,
    /// Model name reported by the trial, when present.
    pub(crate) model_name: Option<String>,
    /// Task name reported by the trial.
    pub(crate) task_name: String,
    /// The bound native reward for this exact trial.
    pub(crate) reward: Fact,
    /// Exception type the trial recorded, when present (typed terminal
    /// classification input; e.g. `NonZeroAgentExitCodeError`,
    /// `AgentTimeoutError`, `ApiError`).
    pub(crate) exception_type: Option<String>,
    /// The integration's terminal observation; `None` for an explicitly
    /// selected diagnostic witness, which produces no integration
    /// observation and never becomes an Agent score.
    pub(crate) terminal: Option<NativeTerminalObservation>,
    /// Per-trial `result.json` path (evidence reference).
    pub(crate) results_path: PathBuf,
    /// Per-trial `config.json` path (evidence reference).
    pub(crate) config_path: PathBuf,
    /// Collected-artifacts manifest path, when the trial collected any.
    pub(crate) manifest_path: Option<PathBuf>,
    /// The verifier mode resolved from the job-referenced task and, for
    /// Harbor, cross-checked against its native result.
    pub(crate) environment_mode: super::task_lifecycle::VerifierEnvironmentMode,
    /// The base revision and environment image of the job-referenced task
    /// (Pier family only; the committed-patch handoff identity).
    pub(crate) base_revision: Option<String>,
    pub(crate) docker_image: Option<String>,
    /// The verifier-input identity: every declared artifact that entered
    /// the verifier, with the digest of exactly the collected bytes
    /// (P19-D03).
    pub(crate) collected: Vec<CollectedArtifact>,
}

/// One declared artifact that entered the verifier input, with the digest
/// of the collected bytes (a directory digest is the canonical digest of
/// its sorted relative paths and per-file digests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CollectedArtifact {
    pub(crate) source: String,
    /// Host destination relative to the trial's artifacts directory.
    pub(crate) destination: String,
    pub(crate) sha256: String,
}

/// Typed rejections of an imported native job (P19-AC-002: every
/// substitution is rejected even when the job process exited 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeBindingError {
    /// No new job directory exists under the trace root.
    NoNewJob,
    /// More than one new job directory exists; the dispatch must own
    /// exactly one.
    MultipleNewJobs { count: usize },
    /// The job-level `result.json` aggregate is missing or unreadable.
    AggregateUnreadable,
    /// The aggregate drifted from the pinned one-trial shape; the token
    /// names the drift (json-parse, trial-count, eval-count,
    /// no-reward-stats, reward-count, reward-trials, bad-reward-values,
    /// agent-component).
    AggregateInvalid(&'static str),
    /// No unique `<name>/result.json` exists in the new job.
    TrialResultMissing,
    /// The per-trial result drifted; the token names the drift
    /// (json-parse, agent-info, agent-name, task-name, verifier-rewards,
    /// terminal-observation).
    TrialInvalid(&'static str),
    /// The trial's reported Agent is not the selected integration.
    AgentNameMismatch { expected: String, observed: String },
    /// The trial config records no Agent selection at all.
    AgentSelectionMissing,
    /// The trial config's Agent selection is not the selected integration
    /// import path.
    AgentSelectionMismatch { expected: String, observed: String },
    /// The trial's task is not the admitted task.
    TaskNameMismatch { expected: String, observed: String },
    /// The trial result carries no verifier reward for the expected
    /// metric (errored or unverified trial).
    RewardMissing,
    /// The reward value left the admitted zero-or-one integral domain.
    RewardInvalid(&'static str),
    /// The aggregate does not attribute its reward to the bound trial.
    AggregateRewardMismatch,
    /// A delivered file covered by a declared artifact was not collected.
    DeliveredArtifactMissing { source: String },
    /// A collected artifact's bytes differ from the delivered observation.
    DeliveredArtifactSubstituted { source: String },
    /// The job's recorded lifecycle contradicts the admitted task
    /// lifecycle (mode or declared collect list).
    LifecycleMismatch { observed: String },
    /// The trial collected an artifact the task never declared: only
    /// declared artifacts may enter a separate verifier.
    UndeclaredArtifact { source: String },
    /// A declared artifact was not collected into the verifier input.
    DeclaredArtifactMissing { source: String },
}

impl NativeBindingError {
    /// Boundary for a settled binding failure. `task-not-observed` is an
    /// honest setup refusal with a bound identity (the terminal exists but
    /// the task never dispatched): the trial settles as infrastructure
    /// invalid and the run continues. Every other token means identity or
    /// provenance corruption and stays integrity-fatal.
    pub(crate) fn settled_boundary(&self) -> crate::failure::FailureBoundaryCode {
        match self {
            Self::TrialInvalid("task-not-observed") => {
                crate::failure::FailureBoundaryCode::Infrastructure
            }
            _ => crate::failure::FailureBoundaryCode::Integrity,
        }
    }

    /// Static redacted diagnostic token (receipts and stderr never echo
    /// job-tree bytes).
    pub(crate) fn token(&self) -> &'static str {
        match self {
            NativeBindingError::NoNewJob => "native-binding-no-new-job",
            NativeBindingError::MultipleNewJobs { .. } => "native-binding-multiple-new-jobs",
            NativeBindingError::AggregateUnreadable => "native-binding-aggregate-unreadable",
            NativeBindingError::AggregateInvalid(_) => "native-binding-aggregate-invalid",
            NativeBindingError::TrialResultMissing => "native-binding-trial-result-missing",
            NativeBindingError::TrialInvalid(_) => "native-binding-trial-invalid",
            NativeBindingError::AgentNameMismatch { .. } => "native-binding-agent-mismatch",
            NativeBindingError::AgentSelectionMissing => "native-binding-agent-selection-missing",
            NativeBindingError::AgentSelectionMismatch { .. } => {
                "native-binding-agent-selection-mismatch"
            }
            NativeBindingError::TaskNameMismatch { .. } => "native-binding-task-mismatch",
            NativeBindingError::RewardMissing => "native-binding-reward-missing",
            NativeBindingError::RewardInvalid(_) => "native-binding-reward-invalid",
            NativeBindingError::AggregateRewardMismatch => {
                "native-binding-aggregate-reward-mismatch"
            }
            NativeBindingError::DeliveredArtifactMissing { .. } => {
                "native-binding-delivered-artifact-missing"
            }
            NativeBindingError::DeliveredArtifactSubstituted { .. } => {
                "native-binding-delivered-artifact-substituted"
            }
            NativeBindingError::LifecycleMismatch { .. } => "native-binding-lifecycle-mismatch",
            NativeBindingError::UndeclaredArtifact { .. } => "native-binding-undeclared-artifact",
            NativeBindingError::DeclaredArtifactMissing { .. } => {
                "native-binding-declared-artifact-missing"
            }
        }
    }
}

impl std::fmt::Display for NativeBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native result binding rejected: {}", self.token())
    }
}

impl std::error::Error for NativeBindingError {}

/// The one new `jobs/<name>` directory this dispatch produced: every
/// job directory holding a `result.json` that was not in the pre-dispatch
/// set, and exactly one of them.
fn new_job_dir(
    trace_root: &std::path::Path,
    stale: &[OsString],
) -> Result<(OsString, PathBuf), NativeBindingError> {
    let jobs = trace_root.join("jobs");
    let mut fresh: Vec<(OsString, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&jobs) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !entry.path().join("result.json").is_file() {
                continue;
            }
            if stale.contains(&name) {
                continue;
            }
            fresh.push((name.clone(), entry.path()));
        }
    }
    match fresh.len() {
        0 => Err(NativeBindingError::NoNewJob),
        1 => Ok(fresh.remove(0)),
        count => Err(NativeBindingError::MultipleNewJobs { count }),
    }
}

/// Bind the unique fresh job and actual trial/Agent selection, then validate
/// the integration's separate runtime proof before examining any grade.
fn bound_native_identity(
    trace_root: &std::path::Path,
    selection: &NativeSelection,
) -> Result<NativeIdentityEvidence, NativeBindingError> {
    let (job_name, job_dir) = new_job_dir(trace_root, &selection.stale_job_names)?;

    // The job aggregate must contain exactly one selected Agent attribution.
    let aggregate_path = job_dir.join("result.json");
    let aggregate: serde_json::Value = serde_json::from_slice(
        &read_native_evidence(trace_root, &aggregate_path)
            .map_err(|_| NativeBindingError::AggregateUnreadable)?,
    )
    .map_err(|_| NativeBindingError::AggregateInvalid("json-parse"))?;
    if aggregate.get("n_total_trials").and_then(|n| n.as_u64()) != Some(1) {
        return Err(NativeBindingError::AggregateInvalid("trial-count"));
    }
    let evals = aggregate
        .get("stats")
        .and_then(|s| s.get("evals"))
        .and_then(|e| e.as_object());
    // The single per-trial result: the authoritative native identity.
    let mut trial_dirs: Vec<PathBuf> = std::fs::read_dir(&job_dir)
        .map_err(|_| NativeBindingError::TrialResultMissing)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("result.json").is_file())
        .collect();
    trial_dirs.sort();
    let trial_dir = trial_dirs
        .first()
        .filter(|_| trial_dirs.len() == 1)
        .cloned()
        .ok_or(NativeBindingError::TrialResultMissing)?;
    let results_path = trial_dir.join("result.json");
    let results: serde_json::Value = serde_json::from_slice(
        &read_native_evidence(trace_root, &results_path)
            .map_err(|_| NativeBindingError::TrialResultMissing)?,
    )
    .map_err(|_| NativeBindingError::TrialInvalid("json-parse"))?;
    let trial_name = results
        .get("trial_name")
        .and_then(|n| n.as_str())
        .ok_or(NativeBindingError::TrialInvalid("trial-name"))?
        .to_owned();
    if trial_dir.file_name().and_then(|name| name.to_str()) != Some(trial_name.as_str()) {
        return Err(NativeBindingError::TrialInvalid("trial-name"));
    }
    let agent_info = results
        .get("agent_info")
        .and_then(|a| a.as_object())
        .ok_or(NativeBindingError::TrialInvalid("agent-info"))?;
    let agent_name = agent_info
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or(NativeBindingError::TrialInvalid("agent-name"))?;
    if agent_name != selection.agent_name {
        return Err(NativeBindingError::AgentNameMismatch {
            expected: selection.agent_name.clone(),
            observed: agent_name.to_owned(),
        });
    }
    let exception_type = results
        .get("exception_info")
        .and_then(|e| e.get("exception_type"))
        .and_then(|t| t.as_str())
        .map(str::to_owned);
    // A timeout job whose runner wrote no eval stats at all (the pinned
    // runner writes none when the verifier is killed on a half-built
    // repository after an agent timeout, or when a completed agent's
    // verifier is killed before stats) has no score to attribute: identity
    // still binds through the per-trial result, and the reward settles as
    // an honest Unknown instead of failing the whole run at the aggregate
    // boundary. Any other missing or multi-eval aggregate stays invalid.
    let unscored_timeout = matches!(
        exception_type.as_deref(),
        Some("AgentTimeoutError") | Some("VerifierTimeoutError")
    ) && evals.is_none_or(|e| e.is_empty())
        && results.get("verifier_result") == Some(&serde_json::Value::Null);
    let eval_key = match evals {
        Some(e) if e.len() == 1 => e.keys().next().expect("len checked").to_owned(),
        _ if unscored_timeout => String::new(),
        None => return Err(NativeBindingError::AggregateInvalid("no-eval-stats")),
        _ => return Err(NativeBindingError::AggregateInvalid("eval-count")),
    };
    // The eval key embeds the actual Agent name (`agent__model__dataset`
    // or `agent__dataset` in the pinned runner): a job grouped under any
    // other Agent - oracle included - is not this trial's score.
    if !unscored_timeout && eval_key.split("__").next().unwrap_or_default() != selection.agent_name
    {
        return Err(NativeBindingError::AgentNameMismatch {
            expected: selection.agent_name.clone(),
            observed: eval_key,
        });
    }
    let task_name = results
        .get("task_name")
        .and_then(|n| n.as_str())
        .ok_or(NativeBindingError::TrialInvalid("task-name"))?;
    if task_name != selection.task_name.as_str() {
        return Err(NativeBindingError::TaskNameMismatch {
            expected: selection.task_name.clone(),
            observed: task_name.to_owned(),
        });
    }
    let model_name = agent_info
        .get("model_info")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_owned);
    if matches!(selection.kind, SelectionKind::Integration { .. }) {
        // Both pinned BaseAgent implementations split the first slash into
        // ModelInfo.provider/name and otherwise retain the selector verbatim.
        let (provider, name) = match selection.model.split_once('/') {
            Some((provider, name)) => (Some(provider), name),
            None => (None, selection.model.as_str()),
        };
        let observed_provider = match agent_info
            .get("model_info")
            .and_then(|info| info.get("provider"))
        {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(value)) => Some(value.as_str()),
            _ => return Err(NativeBindingError::TrialInvalid("selected-model-mismatch")),
        };
        if model_name.as_deref() != Some(name) || observed_provider != provider {
            return Err(NativeBindingError::TrialInvalid("selected-model-mismatch"));
        }
    }
    // Bind the trial's explicit selection according to its dispatch kind.
    // Diagnostics use registered names; integrations use their pinned import.
    let config_path = trial_dir.join("config.json");
    let config: serde_json::Value = serde_json::from_slice(
        &read_native_evidence(trace_root, &config_path)
            .map_err(|_| NativeBindingError::TrialResultMissing)?,
    )
    .map_err(|_| NativeBindingError::TrialInvalid("json-parse"))?;
    let agent_config = config.get("agent").and_then(|a| a.as_object());
    let (expected, selected) = match selection.kind {
        SelectionKind::Integration { .. } => (
            selection.agent_import_path.as_str(),
            agent_config.and_then(|a| {
                a.get("import_path")
                    .and_then(|p| p.as_str())
                    .or_else(|| a.get("name").and_then(|n| n.as_str()))
            }),
        ),
        SelectionKind::DiagnosticWitness => {
            let name = diagnostic_agent_name(selection).ok_or(NativeBindingError::TrialInvalid(
                "diagnostic-selection-invalid",
            ))?;
            if agent_config
                .and_then(|a| a.get("import_path"))
                .is_some_and(|value| !value.is_null())
            {
                return Err(NativeBindingError::TrialInvalid(
                    "diagnostic-selection-ambiguous",
                ));
            }
            (name, agent_config.and_then(|a| a.get("name")?.as_str()))
        }
    };
    match selected {
        None => return Err(NativeBindingError::AgentSelectionMissing),
        Some(observed) if observed == expected => {}
        Some(observed) => {
            return Err(NativeBindingError::AgentSelectionMismatch {
                expected: expected.to_owned(),
                observed: observed.to_owned(),
            });
        }
    }

    let terminal = if matches!(selection.kind, SelectionKind::Integration { .. }) {
        let terminal_path = trial_dir.join("agent/opi-eval-terminal.json");
        let terminal = parse_terminal_observation(
            &read_native_evidence(trace_root, &terminal_path)
                .map_err(|_| NativeBindingError::TrialInvalid("terminal-observation"))?,
        )?;
        bind_runtime_preflight(trace_root, &trial_dir, selection, &terminal)?;
        Some(terminal)
    } else {
        None
    };
    let agent_version = agent_info
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned();
    let observation = NativeTrialObservation {
        job_name,
        trial_name,
        agent_name: agent_name.to_owned(),
        agent_version,
        model_name,
        task_name: task_name.to_owned(),
        exception_type: exception_type.clone(),
        terminal,
        results_path,
        config_path,
    };
    Ok(NativeIdentityEvidence {
        observation,
        aggregate,
        results,
        config,
        trial_dir,
        unscored_timeout,
    })
}

struct NativeIdentityEvidence {
    observation: NativeTrialObservation,
    aggregate: serde_json::Value,
    results: serde_json::Value,
    config: serde_json::Value,
    trial_dir: PathBuf,
    /// Agent or verifier timed out with no eval stats: nothing is
    /// scoreable, so the reward binds as an honest Unknown.
    unscored_timeout: bool,
}

/// Admit the new job's actual identity and runtime proof without requiring a
/// reward, lifecycle collection, or Agent outcome after setup refusal.
pub(crate) fn import_bound_native_observation(
    trace_root: &std::path::Path,
    selection: &NativeSelection,
) -> Result<NativeTrialObservation, NativeBindingError> {
    Ok(bound_native_identity(trace_root, selection)?.observation)
}

/// Bind the native grade after shared identity admission, requiring the
/// observed task, native reward, lifecycle, collection and delivered bytes.
pub(crate) fn import_bound_native_job(
    trace_root: &std::path::Path,
    selection: &NativeSelection,
) -> Result<NativeTrialBinding, NativeBindingError> {
    let NativeIdentityEvidence {
        observation,
        aggregate,
        results,
        config,
        trial_dir,
        unscored_timeout,
    } = bound_native_identity(trace_root, selection)?;
    let NativeTrialObservation {
        job_name,
        trial_name,
        agent_name,
        agent_version,
        model_name,
        task_name,
        exception_type,
        terminal,
        results_path,
        config_path,
    } = observation;
    if terminal.as_ref().is_some_and(|terminal| {
        terminal.runtime_preflight.status != "passed"
            || !terminal.task_dispatched
            || terminal.exit_code.is_none()
    }) {
        return Err(NativeBindingError::TrialInvalid("task-not-observed"));
    }
    let evals = aggregate["stats"]["evals"]
        .as_object()
        .expect("identity validated");
    // A timed-out verifier has no reward in either pinned runner. Bind its
    // exact exception attribution without manufacturing a zero score.
    let reward = if unscored_timeout {
        // Verified shape from bound identity: agent or verifier timeout, no
        // eval stats, no verifier result. There is nothing to score; keep
        // the trial honest instead of inventing a zero or failing the run.
        Fact::Unknown {
            reason: match exception_type.as_deref() {
                Some("AgentTimeoutError") => "agent-timeout".to_owned(),
                _ => "verifier-timeout".to_owned(),
            },
        }
    } else if matches!(
        exception_type.as_deref(),
        Some("AgentTimeoutError") | Some("VerifierTimeoutError")
    ) && results.get("verifier_result") == Some(&serde_json::Value::Null)
    {
        // A timed-out agent or verifier has no reward in either pinned
        // runner: stats may carry an empty-reward entry attributed to the
        // exception, or no eval entry at all (the unscored shape bound
        // above). Bind the exact attribution without manufacturing a
        // score. A verifier that did run and score (timeout with a graded
        // outcome) binds through the normal reward path below.
        let stats = evals.values().next().expect("len checked");
        let reward_stats = stats
            .get("reward_stats")
            .and_then(|value| value.as_object())
            .ok_or(NativeBindingError::AggregateInvalid("no-reward-stats"))?;
        let expected = serde_json::json!({exception_type.clone().expect("matched"): [trial_name]});
        if !reward_stats.is_empty()
            || results.get("verifier_result") != Some(&serde_json::Value::Null)
            || stats.get("n_trials").and_then(|n| n.as_u64()) != Some(0)
            || stats.get("n_errors").and_then(|n| n.as_u64()) != Some(1)
            || stats.get("exception_stats") != Some(&expected)
        {
            return Err(NativeBindingError::AggregateInvalid("verifier-timeout"));
        }
        Fact::Unknown {
            reason: if exception_type.as_deref() == Some("AgentTimeoutError") {
                "agent-timeout".to_owned()
            } else {
                "verifier-timeout".to_owned()
            },
        }
    } else {
        let reward_stats = evals
            .values()
            .next()
            .expect("identity validated")
            .get("reward_stats")
            .and_then(|value| value.as_object())
            .ok_or(NativeBindingError::AggregateInvalid("no-reward-stats"))?;
        let by_value = reward_stats
            .get(&selection.reward_metric)
            .ok_or(NativeBindingError::AggregateInvalid("no-reward-stats"))?;
        let by_value = by_value
            .as_object()
            .ok_or(NativeBindingError::AggregateInvalid("reward-values"))?;
        if by_value.len() != 1 {
            return Err(NativeBindingError::AggregateInvalid("reward-count"));
        }
        let (aggregate_reward_key, trial_names) = by_value.iter().next().expect("len checked");
        let aggregate_reward: f64 = aggregate_reward_key
            .parse()
            .map_err(|_| NativeBindingError::AggregateInvalid("bad-reward-values"))?;
        if !aggregate_reward.is_finite()
            || aggregate_reward.fract() != 0.0
            || !(0.0..=1.0).contains(&aggregate_reward)
        {
            return Err(NativeBindingError::AggregateInvalid("bad-reward-values"));
        }

        // The native reward of this exact trial.
        let rewards = results
            .get("verifier_result")
            .and_then(|v| v.get("rewards"))
            .and_then(|r| r.as_object())
            .ok_or(NativeBindingError::RewardMissing)?;
        let reward_value = rewards
            .get(&selection.reward_metric)
            .and_then(|v| v.as_f64())
            .ok_or(NativeBindingError::RewardMissing)?;
        if !reward_value.is_finite()
            || reward_value.fract() != 0.0
            || !(0.0..=1.0).contains(&reward_value)
        {
            return Err(NativeBindingError::RewardInvalid("bad-reward-values"));
        }
        if reward_value != aggregate_reward {
            return Err(NativeBindingError::AggregateRewardMismatch);
        }
        // The aggregate must attribute its reward to exactly this trial.
        let names = trial_names
            .as_array()
            .ok_or(NativeBindingError::AggregateInvalid("reward-trials"))?;
        if names.len() != 1 || names[0].as_str() != Some(trial_name.as_str()) {
            return Err(NativeBindingError::AggregateRewardMismatch);
        }
        Fact::Known {
            value: reward_value as u64,
            origin: "native-trial-result".to_owned(),
        }
    };
    // Resolve the job's task reference, trial additions, and native mode
    // against the admitted lifecycle this dispatch selected.
    let observed_lifecycle = parse_recorded_lifecycle(&config, &results, selection)
        .map_err(|_| NativeBindingError::TrialInvalid("task-lifecycle"))?;
    if observed_lifecycle != selection.lifecycle {
        return Err(NativeBindingError::LifecycleMismatch {
            observed: format!(
                "mode {} with {} declared artifacts",
                observed_lifecycle.environment_mode.as_str(),
                observed_lifecycle.declared.len()
            ),
        });
    }

    // Declared-collection binding (P19-D03, P19-AC-004). The trial's
    // collected-artifacts manifest is the record of what left the agent
    // environment for the verifier: every collected entry must be a
    // declared artifact or the pinned convention publish dir (nothing
    // undeclared enters the verifier), every declared artifact must have
    // been collected (status ok), and the collected bytes bind as this
    // trial's verifier-input identity.
    let manifest_path = trial_dir.join("artifacts/manifest.json");
    let manifest: Option<serde_json::Value> = if manifest_path.is_file() {
        Some(
            serde_json::from_slice(
                &std::fs::read(&manifest_path)
                    .map_err(|_| NativeBindingError::TrialInvalid("artifacts-manifest"))?,
            )
            .map_err(|_| NativeBindingError::TrialInvalid("artifacts-manifest"))?,
        )
    } else {
        None
    };
    let empty: Vec<serde_json::Value> = Vec::new();
    let entries = manifest
        .as_ref()
        .and_then(|m| m.as_array())
        .unwrap_or(&empty);
    let artifact_root = trial_dir.join("artifacts");
    for entry in entries {
        let source = entry
            .get("source")
            .and_then(|s| s.as_str())
            .ok_or(NativeBindingError::TrialInvalid("artifacts-manifest"))?;
        if source != HARBOR_CONVENTION_SOURCE
            && !selection
                .lifecycle
                .declared
                .iter()
                .any(|declared| covers(&declared.source, source))
        {
            return Err(NativeBindingError::UndeclaredArtifact {
                source: source.to_owned(),
            });
        }
    }
    let mut collected: Vec<CollectedArtifact> = Vec::new();
    for declared in &selection.lifecycle.declared {
        let entry = entries.iter().find(|entry| {
            let Some(source) = entry.get("source").and_then(|s| s.as_str()) else {
                return false;
            };
            same_artifact(source, &declared.source)
        });
        let Some(entry) = entry else {
            return Err(NativeBindingError::DeclaredArtifactMissing {
                source: declared.source.clone(),
            });
        };
        if entry.get("status").and_then(|s| s.as_str()) != Some("ok") {
            return Err(NativeBindingError::DeclaredArtifactMissing {
                source: declared.source.clone(),
            });
        }
        let destination = manifest_destination(entry)?;
        let host = artifact_root.join(destination);
        let digest =
            collected_digest(&host).map_err(|_| NativeBindingError::DeclaredArtifactMissing {
                source: declared.source.clone(),
            })?;
        collected.push(CollectedArtifact {
            source: declared.source.clone(),
            destination: destination.to_owned(),
            sha256: digest,
        });
    }
    // The delivered cross-check (integration dispatches only): a
    // delivered file covered by a declared artifact must still carry the
    // bytes the integration observed in the task environment
    // (P19-AC-002 substituted-artifact rejection).
    if let Some(terminal) = &terminal {
        for delivered in &terminal.delivered {
            if let Some((entry, relative)) = covering_entry(entries, &delivered.source) {
                if entry.get("status").and_then(|s| s.as_str()) != Some("ok") {
                    return Err(NativeBindingError::DeliveredArtifactMissing {
                        source: delivered.source.clone(),
                    });
                }
                let destination = manifest_destination(entry)?;
                let collected_path = artifact_root.join(destination);
                let collected_path = if relative.is_empty() {
                    collected_path
                } else {
                    collected_path.join(relative)
                };
                let bytes = std::fs::read(&collected_path).map_err(|_| {
                    NativeBindingError::DeliveredArtifactMissing {
                        source: delivered.source.clone(),
                    }
                })?;
                if crate::runner::material::sha256_hex(&bytes) != delivered.sha256 {
                    return Err(NativeBindingError::DeliveredArtifactSubstituted {
                        source: delivered.source.clone(),
                    });
                }
            }
        }
    }

    Ok(NativeTrialBinding {
        job_name,
        trial_name,
        agent_name,
        agent_version,
        model_name,
        task_name,
        reward,
        exception_type,
        terminal,
        results_path,
        config_path,
        manifest_path: manifest_path.is_file().then_some(manifest_path),
        environment_mode: observed_lifecycle.environment_mode,
        base_revision: observed_lifecycle.base_revision,
        docker_image: observed_lifecycle.docker_image,
        collected,
    })
}

/// Harbor's pinned conventional agent publish directory: the one
/// non-task collection every trial legitimately performs.
const HARBOR_CONVENTION_SOURCE: &str = "/logs/artifacts";

/// Both pinned runners express manifest destinations relative to the
/// trial. The Companion stores destinations relative to its artifacts
/// directory, so remove exactly that prefix before reading collected bytes.
fn manifest_destination(entry: &serde_json::Value) -> Result<&str, NativeBindingError> {
    let invalid = NativeBindingError::TrialInvalid("artifacts-manifest");
    let destination = entry
        .get("destination")
        .and_then(|d| d.as_str())
        .ok_or(invalid.clone())?;
    if destination == "artifacts" {
        return Ok(".");
    }
    let relative = destination
        .strip_prefix("artifacts/")
        .ok_or(invalid.clone())?;
    if relative.is_empty()
        || relative.contains('\\')
        || !std::path::Path::new(relative)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(invalid);
    }
    Ok(relative)
}

/// True when a manifest source and a declared source name the same
/// artifact (a declared directory may appear with or without its
/// trailing slash).
fn same_artifact(source: &str, declared: &str) -> bool {
    source.trim_end_matches('/') == declared.trim_end_matches('/')
}

/// True when `source` (a manifest or delivered path) lies inside the
/// declared artifact `root` (equal to it or nested under it).
fn covers(root: &str, source: &str) -> bool {
    let root = root.trim_end_matches('/');
    same_artifact(source, root) || source.starts_with(root) && source[root.len()..].starts_with('/')
}

/// The manifest entry covering a delivered source path, with the
/// delivered file's path relative to the entry's collected root.
fn covering_entry<'a>(
    entries: &'a [serde_json::Value],
    delivered_source: &str,
) -> Option<(&'a serde_json::Value, String)> {
    let mut best: Option<(&'a serde_json::Value, &str)> = None;
    for entry in entries {
        let Some(source) = entry.get("source").and_then(|s| s.as_str()) else {
            continue;
        };
        if covers(source, delivered_source)
            && best.is_none_or(|(_, current)| source.len() > current.len())
        {
            best = Some((entry, source));
        }
    }
    best.map(|(entry, source)| {
        let root = source.trim_end_matches('/');
        let relative = delivered_source
            .strip_prefix(root)
            .unwrap_or_default()
            .trim_start_matches('/');
        (entry, relative.to_owned())
    })
}

/// The digest of exactly the collected bytes: a file digest, or for a
/// collected directory the canonical digest of its sorted relative
/// paths and per-file digests.
pub(crate) fn collected_digest(host: &std::path::Path) -> std::io::Result<String> {
    fn walk(host: &std::path::Path, prefix: &str, rows: &mut Vec<String>) -> std::io::Result<()> {
        let mut children: Vec<std::path::PathBuf> = std::fs::read_dir(host)?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        children.sort();
        for child in children {
            let name = format!(
                "{prefix}{}",
                child
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            );
            if child.is_dir() {
                walk(&child, &format!("{name}/"), rows)?;
            } else {
                rows.push(format!(
                    "{name}\\n{}",
                    crate::runner::material::sha256_hex(&std::fs::read(&child)?)
                ));
            }
        }
        Ok(())
    }
    if host.is_file() {
        return Ok(crate::runner::material::sha256_hex(&std::fs::read(host)?));
    }
    let mut rows = Vec::new();
    walk(host, "", &mut rows)?;
    Ok(crate::runner::material::sha256_hex(
        rows.join("\\n").as_bytes(),
    ))
}

/// Resolve lifecycle facts from the task package referenced by the native
/// trial config. That config records a task reference, not the task's
/// definition. Harbor also records the resolved mode in its result;
/// Pier's pinned result schema leaves it with the referenced task.
fn parse_recorded_lifecycle(
    config: &serde_json::Value,
    results: &serde_json::Value,
    selection: &NativeSelection,
) -> Result<TaskLifecycleSelection, ()> {
    use super::task_lifecycle::{DeclaredArtifact, TaskFamily, VerifierEnvironmentMode};
    let task_dir = config
        .get("task")
        .and_then(|task| task.get("path"))
        .and_then(|path| path.as_str())
        .map(std::path::Path::new)
        .filter(|path| path.is_absolute())
        .ok_or(())?;
    // The admitted committed-patch route carries the Pier base identity;
    // Harbor selections carry no base revision.
    let family = if selection.lifecycle.base_revision.is_some() {
        TaskFamily::Pier
    } else {
        TaskFamily::Harbor
    };
    let task = super::task_lifecycle::parse_task_lifecycle(task_dir, family).map_err(|_| ())?;
    if family == TaskFamily::Harbor {
        let recorded_mode = match results
            .get("verifier_environment_mode")
            .and_then(|v| v.as_str())
        {
            Some("shared") => VerifierEnvironmentMode::Shared,
            Some("separate") => VerifierEnvironmentMode::Separate,
            _ => return Err(()),
        };
        if recorded_mode != task.environment_mode {
            return Err(());
        }
    }
    // Both runners collect the task declarations followed by the trial's
    // additional artifacts. The frozen selection must match that union.
    let mut declared = task.declared_artifacts;
    let empty = Vec::new();
    let additions = match config.get("artifacts") {
        Some(value) => value.as_array().ok_or(())?,
        None => &empty,
    };
    for artifact in additions {
        match artifact {
            serde_json::Value::String(source) => declared.push(DeclaredArtifact {
                source: source.clone(),
                destination: None,
                service: None,
            }),
            serde_json::Value::Object(fields) => {
                let Some(source) = fields.get("source").and_then(|s| s.as_str()) else {
                    return Err(());
                };
                declared.push(DeclaredArtifact {
                    source: source.to_owned(),
                    destination: fields
                        .get("destination")
                        .and_then(|d| d.as_str())
                        .map(str::to_owned),
                    service: fields
                        .get("service")
                        .and_then(|service| service.as_str())
                        .map(str::to_owned),
                });
            }
            _ => return Err(()),
        }
    }
    Ok(TaskLifecycleSelection {
        environment_mode: task.environment_mode,
        declared,
        base_revision: task.base_revision,
        docker_image: task.docker_image,
    })
}
/// Read only a bounded regular file beneath the owned trace, without
/// following links or consuming special files. Missing evidence is an error.
fn read_native_evidence(
    trace_root: &std::path::Path,
    path: &std::path::Path,
) -> std::io::Result<Vec<u8>> {
    let capture = read_native_checked_file(trace_root, path, false)?;
    if capture.truncated {
        return Err(std::io::Error::other(
            "native evidence exceeds artifact bound",
        ));
    }
    Ok(capture.bytes)
}

fn read_native_proof_capture(
    trace_root: &std::path::Path,
    path: &std::path::Path,
) -> std::io::Result<OutputCapture> {
    // Native task environments require the existing Unix/WSL single-link
    // capture contract; no Windows runtime proof is admitted by this reader.
    read_native_checked_file(trace_root, path, true)
}

fn read_native_checked_file(
    trace_root: &std::path::Path,
    path: &std::path::Path,
    require_single_link: bool,
) -> std::io::Result<OutputCapture> {
    use std::io::Read;
    if !path.starts_with(trace_root) || path == trace_root {
        return Err(std::io::Error::other("native evidence outside owned trace"));
    }
    for ancestor in path
        .parent()
        .into_iter()
        .flat_map(std::path::Path::ancestors)
    {
        let metadata = std::fs::symlink_metadata(ancestor)?;
        if !metadata.is_dir() || native_reparse_or_link(&metadata) {
            return Err(std::io::Error::other("unsafe native evidence ancestor"));
        }
        if ancestor == trace_root {
            break;
        }
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    #[cfg(unix)]
    let single_link = {
        use std::os::unix::fs::MetadataExt;
        metadata.nlink() == 1
    };
    #[cfg(not(unix))]
    let single_link = false;
    if !metadata.is_file()
        || native_reparse_or_link(&metadata)
        || (!single_link && (require_single_link || cfg!(unix)))
    {
        return Err(std::io::Error::other("unsafe native evidence file"));
    }
    let mut bytes = Vec::new();
    file.take(crate::bundle::MAX_ARTIFACT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let truncated = bytes.len() > crate::bundle::MAX_ARTIFACT_BYTES;
    bytes.truncate(crate::bundle::MAX_ARTIFACT_BYTES);
    Ok(OutputCapture { bytes, truncated })
}

fn native_reparse_or_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
        {
            return true;
        }
    }
    metadata.is_symlink()
}

/// Parse only the current proof schema and coherent runtime/task states.
fn parse_terminal_observation(
    bytes: &[u8],
) -> Result<NativeTerminalObservation, NativeBindingError> {
    let invalid = || NativeBindingError::TrialInvalid("terminal-observation");
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    // Nullable observations must still be present: absence is not an observed null.
    for field in ["exit_code", "delivered", "runtime_preflight"] {
        if value.get(field).is_none() {
            return Err(invalid());
        }
    }
    let proof = value.get("runtime_preflight").ok_or_else(invalid)?;
    for field in [
        "exit_code",
        "runtime_package_manifest_sha256",
        "runtime_package_entrypoint",
        "reason",
    ] {
        if proof.get(field).is_none() {
            return Err(invalid());
        }
    }
    let terminal: NativeTerminalObservation =
        serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let proof = &terminal.runtime_preflight;
    if !matches!(
        terminal.schema.as_str(),
        "opi-eval-native-agent-terminal/2" | "opi-eval-native-agent-terminal/3"
    ) || terminal.trial.is_empty()
        || !matches!(terminal.product.as_str(), "opi" | "pi")
        || !is_sha256_hex(&terminal.agent_executable_sha256)
        || proof.kind != "version"
        || [
            &proof.agent_executable_sha256,
            &proof.command_sha256,
            &proof.stdout_sha256,
            &proof.stderr_sha256,
        ]
        .iter()
        .any(|digest| !is_sha256_hex(digest))
        || proof
            .runtime_package_manifest_sha256
            .as_ref()
            .is_some_and(|digest| !is_sha256_hex(digest))
        || proof.runtime_package_manifest_sha256.is_some()
            != proof.runtime_package_entrypoint.is_some()
        || proof
            .runtime_package_entrypoint
            .as_ref()
            .is_some_and(|path| {
                path.is_empty()
                    || path.starts_with('/')
                    || path.contains('\\')
                    || path
                        .split('/')
                        .any(|part| part.is_empty() || part == "." || part == "..")
            })
        || (proof.stdout_truncated && proof.stdout_complete)
        || (proof.stderr_truncated && proof.stderr_complete)
        || terminal
            .delivered
            .iter()
            .any(|file| !file.source.starts_with('/') || !is_sha256_hex(&file.sha256))
        || terminal
            .capture_helper_sha256
            .as_ref()
            .is_some_and(|digest| !is_sha256_hex(digest))
    {
        return Err(invalid());
    }
    // Schema /3 binds the first-sink capture: the helper identity, the
    // helper preflight and the receipt summary are mandatory together.
    if terminal.schema == "opi-eval-native-agent-terminal/3"
        && (terminal.capture_helper_sha256.is_none()
            || terminal.helper_preflight.is_none()
            || terminal.capture.is_none()
            || terminal.helper_preflight.as_ref().is_some_and(|proof| {
                proof.kind != "capture-helper-selfcheck"
                    || proof.status != "passed"
                    || !is_sha256_hex(&proof.command_sha256)
                    || !is_sha256_hex(&proof.stdout_sha256)
                    || !is_sha256_hex(&proof.stderr_sha256)
            })
            || !matches!(
                terminal
                    .capture
                    .as_ref()
                    .map(|capture| capture.status.as_str()),
                Some("sanitized" | "refused" | "incomplete" | "unreadable")
            )
            || terminal.capture.as_ref().is_some_and(|capture| {
                capture
                    .policy
                    .as_deref()
                    .is_some_and(|policy| policy != "full" && policy != "terminal-only")
            }))
    {
        return Err(invalid());
    }
    let complete = proof.stdout_complete
        && proof.stderr_complete
        && !proof.stdout_truncated
        && !proof.stderr_truncated;
    match proof.status.as_str() {
        "passed" if proof.exit_code == Some(0) && complete && proof.reason.is_none() => {}
        "failed"
            if proof.exit_code.is_some_and(|code| code != 0)
                && complete
                && proof.reason.is_none() => {}
        "unknown"
            if proof.reason.as_deref().is_some_and(|reason| {
                matches!(
                    reason,
                    "timeout" | "transport" | "termination" | "invalid-output" | "capture-overflow"
                )
            }) =>
        {
            if proof.reason.as_deref() == Some("capture-overflow")
                && !proof.stdout_truncated
                && !proof.stderr_truncated
            {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    if (proof.status != "passed" || !terminal.task_dispatched)
        && (terminal.task_dispatched
            || terminal.exit_code.is_some()
            || terminal.timed_out
            || !terminal.delivered.is_empty())
    {
        return Err(invalid());
    }
    Ok(terminal)
}

fn bind_runtime_preflight(
    trace_root: &std::path::Path,
    trial_dir: &std::path::Path,
    selection: &NativeSelection,
    terminal: &NativeTerminalObservation,
) -> Result<(), NativeBindingError> {
    let SelectionKind::Integration {
        product,
        executable_sha256,
        trial,
        runtime_package_manifest_sha256,
        runtime_package_entrypoint,
    } = &selection.kind
    else {
        return Ok(());
    };
    let proof = &terminal.runtime_preflight;
    if terminal.product != *product
        || terminal.agent_executable_sha256 != *executable_sha256
        || terminal.trial != *trial
        || proof.agent_executable_sha256 != *executable_sha256
        || proof.runtime_package_manifest_sha256 != *runtime_package_manifest_sha256
        || proof.runtime_package_entrypoint != *runtime_package_entrypoint
    {
        return Err(NativeBindingError::TrialInvalid("selected-agent-mismatch"));
    }
    let executable = match runtime_package_entrypoint {
        Some(entrypoint) => format!("/opi-eval-agent/{product}/runtime/{entrypoint}"),
        None => format!("/opi-eval-agent/{product}/agent"),
    };
    let quoted = if executable
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        executable
    } else {
        format!("'{}'", executable.replace('\'', "'\"'\"'"))
    };
    let command = format!("{quoted} --version");
    if proof.command_sha256 != crate::runner::material::sha256_hex(command.as_bytes()) {
        return Err(NativeBindingError::TrialInvalid(
            "runtime-preflight-command",
        ));
    }
    let mut captures = Vec::new();
    for (name, digest, complete, truncated) in [
        (
            "stdout",
            &proof.stdout_sha256,
            proof.stdout_complete,
            proof.stdout_truncated,
        ),
        (
            "stderr",
            &proof.stderr_sha256,
            proof.stderr_complete,
            proof.stderr_truncated,
        ),
    ] {
        let path = trial_dir
            .join("agent")
            .join(format!("runtime-preflight-{name}.log"));
        let capture = read_native_proof_capture(trace_root, &path)
            .map_err(|_| NativeBindingError::TrialInvalid("runtime-preflight-capture"))?;
        if capture.truncated != truncated
            || (capture.truncated && complete)
            || crate::runner::material::sha256_hex(&capture.bytes) != *digest
        {
            return Err(NativeBindingError::TrialInvalid(
                "runtime-preflight-capture",
            ));
        }
        captures.push(capture.bytes);
    }
    if matches!(proof.status.as_str(), "passed" | "failed") {
        let mut readable = false;
        for bytes in &captures {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| NativeBindingError::TrialInvalid("runtime-preflight-output"))?;
            if text.contains('\0') {
                return Err(NativeBindingError::TrialInvalid("runtime-preflight-output"));
            }
            readable |= !text.trim().is_empty();
        }
        if proof.status == "passed" && !readable {
            return Err(NativeBindingError::TrialInvalid("runtime-preflight-output"));
        }
    }
    Ok(())
}

/// Typed failure carried on a settled record. A failed verification run is
/// settled evidence, not an unsettable error (`EVAL-BMK-006`: no fallback to
/// another revision, grader, cached score, heuristic, or LLM judgment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkFailure {
    /// Static redacted token describing the failure kind.
    pub kind: &'static str,
    /// Owning failure boundary.
    pub boundary: FailureBoundaryCode,
}

/// Authoritative completion verdict of one native verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BenchmarkCompletion {
    /// The verifier ran to completion and its native output was imported.
    Verified {
        /// Native summary counters with benchmark-defined names.
        metrics: NativeMetrics,
        /// Raw native outputs as content-addressed references.
        artifacts: Vec<NativeArtifact>,
    },
    /// The dispatched upstream job ran the explicitly selected Agent
    /// inside the graded task environment and its result was bound to
    /// that exact trial (P19-D01): the reward is the bound native grade
    /// of the selected Agent's same-trial work.
    VerifiedNative {
        /// The bound trial identity (agent, task, job, reward, delivered
        /// artifacts, verifier-input identity). Boxed: the binding grew
        /// past the variant-size lint threshold when the Pier family
        /// added its base/image identity.
        binding: Box<NativeTrialBinding>,
        /// Raw native job outputs as content-addressed references.
        artifacts: Vec<NativeArtifact>,
    },
    /// The run settled as a typed failure.
    Failed(BenchmarkFailure),
}

/// Provenance binding one settled record to its admitted inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkProvenance {
    /// Digest of the admitted resolved external lock.
    pub admitted_lock_digest: String,
    /// Identity digest of the consumed integrity record.
    pub integrity_digest: String,
    /// Revision the integrity record admitted.
    pub revision: String,
    /// Task id verified.
    pub task_id: String,
}

/// The settled benchmark record: one per native verification run, on every
/// path.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkRecord {
    pub identity: BenchmarkIdentity,
    pub exit: ExitState,
    pub stdout: OutputCapture,
    pub stderr: OutputCapture,
    /// What termination cleanup was observed for the supervised tree; the
    /// trial receipts project it as durable cleanup evidence.
    pub cleanup: CleanupEvidence,
    /// Wall-clock duration of the supervised verification run.
    pub wall_time: Duration,
    /// Native reward as a typed fact. Until the real native smoke captures
    /// the harbor reward path it stays `Unknown` with a typed reason — it is
    /// never computed from the summary counters (`EVAL-BMK-007`).
    pub reward: Fact,
    pub completion: BenchmarkCompletion,
    pub provenance: BenchmarkProvenance,
}

impl BenchmarkRecord {
    /// The failure boundary of a failed completion, or `None` when verified.
    #[cfg_attr(
        not(all(test, unix)),
        expect(
            dead_code,
            reason = "failure-boundary projection is exercised by Unix process tests"
        )
    )]
    pub(crate) fn failure_boundary(&self) -> Option<FailureBoundaryCode> {
        match &self.completion {
            BenchmarkCompletion::Verified { .. } | BenchmarkCompletion::VerifiedNative { .. } => {
                None
            }
            BenchmarkCompletion::Failed(failure) => Some(failure.boundary),
        }
    }
}

/// The benchmark-neutral benchmark-specific seam: one implementation per
/// pinned benchmark revision. Crate-private because the only consumers are
/// this crate's execution driver and the shared conformance suite.
pub(crate) trait BenchmarkAdapter {
    /// Exact identity declared by the declarative profile.
    fn identity(&self) -> BenchmarkIdentity;

    /// Revision-specific admission check run before any spawn: the request's
    /// integrity admission, task id, and lock must bind to this adapter's
    /// pinned revision.
    fn admission(&self, request: &BenchmarkRunRequest) -> Result<(), ExecutionError>;

    /// Structured spawn request for one verification run. Owns argv, cwd,
    /// environment projection, and limits from the declarative profile.
    fn spawn_spec(&self, request: &BenchmarkRunRequest) -> SpawnSpec;

    /// Settle one supervised outcome into the native half of the record:
    /// imported metrics, reward fact, artifacts, and the authoritative
    /// completion verdict. Infallible: every observed verifier failure
    /// settles inside the record.
    fn settle(
        &self,
        outcome: &crate::process::SupervisedOutcome,
        request: &BenchmarkRunRequest,
    ) -> (Fact, BenchmarkCompletion);
}

/// Pre-spawn contract violation: the request itself cannot produce a legal
/// verification. Static tokens only; request bytes are not echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionError {
    pub token: &'static str,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "benchmark execution rejected: {}", self.token)
    }
}

impl std::error::Error for ExecutionError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The shared execution driver. One verification run, one settled record,
/// every path.
pub(crate) struct BenchmarkExecution;

fn diagnostic_agent_name(selection: &NativeSelection) -> Option<&'static str> {
    match (
        selection.agent_name.as_str(),
        selection.agent_import_path.as_str(),
    ) {
        ("oracle", "harbor.agents.oracle:OracleAgent" | "pier.agents.oracle:OracleAgent") => {
            Some("oracle")
        }
        ("nop", "harbor.agents.nop:NopAgent" | "pier.agents.nop:NopAgent") => Some("nop"),
        _ => None,
    }
}

/// Materialize the pinned runners' single-trial JobConfig without CLI Agent
/// flags, which would replace the configured Agent and discard its stage caps.
fn native_job_config(request: &BenchmarkRunRequest) -> Result<serde_json::Value, ExecutionError> {
    let reject = || ExecutionError {
        token: "native-stage-configuration-invalid",
    };
    let selection = request.native_selection.as_ref().ok_or_else(reject)?;
    let task: toml::Value = toml::from_str(
        &std::fs::read_to_string(request.task_dir.join("task.toml")).map_err(|_| reject())?,
    )
    .map_err(|_| reject())?;
    // Both locked task models default Agent timeout to None. Their timeout
    // resolution returns None before considering max_timeout_sec in that case.
    let agent_default = task.get("agent").and_then(|agent| agent.get("timeout_sec"));
    if let Some(value) = agent_default {
        let seconds = value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64));
        if !seconds.is_some_and(|seconds| seconds.is_finite() && seconds > 0.0) {
            return Err(reject());
        }
    }
    if request.agent_timeout_seconds == Some(0) || request.verifier_timeout_seconds == Some(0) {
        return Err(reject());
    }
    let task_path = request.task_dir.canonicalize().map_err(|_| reject())?;
    let mut agent = serde_json::json!({
        "model_name": selection.model,
        "max_timeout_sec": request.agent_timeout_seconds,
        "override_timeout_sec": if agent_default.is_none() {
            request.agent_timeout_seconds
        } else {
            None
        },
    });
    match selection.kind {
        SelectionKind::Integration { .. } => {
            agent["import_path"] = selection.agent_import_path.clone().into();
        }
        SelectionKind::DiagnosticWitness => {
            agent["name"] = diagnostic_agent_name(selection)
                .ok_or(ExecutionError {
                    token: "native-diagnostic-selection-invalid",
                })?
                .into();
        }
    }
    Ok(serde_json::json!({
        "jobs_dir": "jobs",
        "n_attempts": 1,
        "n_concurrent_trials": 1,
        "retry": {"max_retries": 0},
        "timeout_multiplier": 1.0,
        "agent_timeout_multiplier": 1.0,
        "verifier_timeout_multiplier": 1.0,
        "agents": [agent],
        "verifier": {"max_timeout_sec": request.verifier_timeout_seconds},
        "environment": {
            "type": "docker",
            "import_path": if matches!(selection.kind, SelectionKind::Integration { .. }) {
                Some(format!("{}:OwnedDockerEnvironment", selection.agent_import_path.split(':').next().ok_or_else(reject)?))
            } else {
                None
            },
        },
        "tasks": [{"path": task_path}],
        "datasets": [],
    }))
}

/// The persisted effective trial config must retain the caps sent to the runner.
/// Compare numeric values, since the Python models serialize integer caps as floats.
fn native_stage_config_matches(config: &std::path::Path, expected: &serde_json::Value) -> bool {
    let Ok(bytes) = std::fs::read(config) else {
        return false;
    };
    let Ok(observed) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    // The observed task reference must be exactly the absolute dispatched
    // package (P19-AUD-004): a substituted absolute task package cannot be
    // admitted as the selected dispatch, even when the task name and every
    // lifecycle field still match.
    match (
        observed["task"]["path"].as_str(),
        expected["tasks"][0]["path"].as_str(),
    ) {
        (Some(observed_path), Some(expected_path)) if observed_path == expected_path => {}
        _ => return false,
    }
    if observed["environment"]["import_path"] != expected["environment"]["import_path"] {
        return false;
    }
    for (actual, wanted) in [
        (&observed["agent"], &expected["agents"][0]),
        (&observed["verifier"], &expected["verifier"]),
    ] {
        for key in ["max_timeout_sec", "override_timeout_sec"] {
            if !(actual[key].is_null() && wanted[key].is_null())
                && (actual[key].as_f64().is_none() || actual[key].as_f64() != wanted[key].as_f64())
            {
                return false;
            }
        }
    }
    // Harbor omits model defaults when persisting TrialConfig. Its global
    // multiplier defaults to 1; explicit null or a changed value is invalid.
    observed
        .get("timeout_multiplier")
        .is_none_or(|value| value.as_f64() == Some(1.0))
        && ["agent_timeout_multiplier", "verifier_timeout_multiplier"]
            .iter()
            .all(|key| observed[*key].as_f64() == Some(1.0))
}

impl BenchmarkExecution {
    /// Run `request` under `adapter` to settlement.
    ///
    /// Never panics and never returns an `Err` for an observed run: spawn
    /// failures, non-zero exits, timeouts, cancellations, and invalid native
    /// output all settle inside the [`BenchmarkRecord`]. `Err` means the
    /// request was structurally unusable before any process existed: an
    /// unadmitted revision, an invalid task classification, or a malformed
    /// admitted lock digest.
    pub(crate) async fn run(
        request: &BenchmarkRunRequest,
        adapter: &dyn BenchmarkAdapter,
        cancel: &CancellationToken,
    ) -> Result<BenchmarkRecord, ExecutionError> {
        if request.task_id.trim().is_empty() {
            return Err(ExecutionError {
                token: "empty-task-id",
            });
        }
        if !is_sha256_hex(&request.admitted_lock_digest) {
            return Err(ExecutionError {
                token: "lock-digest-malformed",
            });
        }
        if !request.integrity.admitted() {
            return Err(ExecutionError {
                token: "revision-not-admitted",
            });
        }
        if !request
            .integrity
            .task_classification(&request.task_id)
            .is_some_and(|c| c.is_valid_agent_outcome())
        {
            return Err(ExecutionError {
                token: "task-not-valid",
            });
        }
        adapter.admission(request)?;
        let mut resolved_request;
        let request = if request.native_selection.is_some() {
            resolved_request = request.clone();
            resolved_request.trace_root =
                request
                    .trace_root
                    .canonicalize()
                    .map_err(|_| ExecutionError {
                        token: "native-job-config-unwritable",
                    })?;
            &resolved_request
        } else {
            request
        };
        let native_config = request
            .native_selection
            .as_ref()
            .map(|_| native_job_config(request))
            .transpose()?;
        if let Some(config) = &native_config {
            let bytes = serde_json::to_vec(config).map_err(|_| ExecutionError {
                token: "native-job-config-unwritable",
            })?;
            std::fs::write(request.trace_root.join("native-job-config.json"), bytes).map_err(
                |_| ExecutionError {
                    token: "native-job-config-unwritable",
                },
            )?;
        }
        let identity = adapter.identity();
        let spec = adapter.spawn_spec(request);
        let started = Instant::now();
        let outcome = ProcessSupervisor::run(&spec, cancel).await;
        let wall_time = started.elapsed();
        let (mut reward, mut completion) = adapter.settle(&outcome, request);
        let termination_unknown = request.native_selection.as_ref().is_some_and(|selection| {
            new_job_dir(&request.trace_root, &selection.stale_job_names)
                .ok()
                .and_then(|(_, job)| std::fs::read_dir(job).ok())
                .is_some_and(|entries| {
                    entries
                        .flatten()
                        .any(|entry| entry.path().join("opi-eval-effect-unknown").exists())
                })
        });
        if termination_unknown {
            cancel.cancel();
            reward = Fact::Unknown {
                reason: "native-command-termination-unconfirmed".to_owned(),
            };
            completion = BenchmarkCompletion::Failed(BenchmarkFailure {
                kind: "native-command-termination-unconfirmed",
                boundary: FailureBoundaryCode::Infrastructure,
            });
        }
        let bound_config = match &completion {
            BenchmarkCompletion::VerifiedNative { binding, .. } => {
                Some(binding.config_path.clone())
            }
            _ => request.native_selection.as_ref().and_then(|selection| {
                import_bound_native_observation(&request.trace_root, selection)
                    .ok()
                    .map(|observation| observation.config_path)
            }),
        };
        if !termination_unknown
            && let (Some(expected), Some(config)) = (&native_config, &bound_config)
            && !native_stage_config_matches(config, expected)
        {
            reward = Fact::Unknown {
                reason: "native-stage-configuration-drift".to_owned(),
            };
            completion = BenchmarkCompletion::Failed(BenchmarkFailure {
                kind: "native-stage-configuration-drift",
                boundary: FailureBoundaryCode::Integrity,
            });
        }
        // A selected native dispatch supervises the entire upstream job,
        // including the Agent. Verifier-only executions keep grader ownership.
        if request.native_selection.is_some() && outcome.exit == ExitState::TimedOut {
            completion = BenchmarkCompletion::Failed(BenchmarkFailure {
                kind: "native-job-timeout",
                boundary: crate::failure::FailureBoundaryCode::Infrastructure,
            });
        }
        Ok(BenchmarkRecord {
            identity,
            exit: outcome.exit,
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            cleanup: outcome.cleanup,
            wall_time,
            reward,
            completion,
            provenance: BenchmarkProvenance {
                admitted_lock_digest: request.admitted_lock_digest.clone(),
                integrity_digest: request.integrity.identity_digest().to_owned(),
                revision: request.integrity.revision().to_owned(),
                task_id: request.task_id.clone(),
            },
        })
    }
}

/// Failure kinds for settled records, kept as static tokens.
pub(crate) mod failure_kinds {
    use crate::failure::FailureBoundaryCode;

    /// Map a verifier spawn failure to its settled token + boundary. Tool or
    /// runner acquisition failures are infrastructure, not grader, failures.
    pub(crate) fn spawn(reason: super::SpawnReason) -> super::BenchmarkFailure {
        let token = match reason {
            super::SpawnReason::NotFound => "spawn-not-found",
            super::SpawnReason::PermissionDenied => "spawn-permission-denied",
            super::SpawnReason::BadCwd => "spawn-bad-cwd",
            super::SpawnReason::SpawnFailed => "spawn-failed",
        };
        super::BenchmarkFailure {
            kind: token,
            boundary: FailureBoundaryCode::Infrastructure,
        }
    }

    /// Non-zero verifier exit (`EVAL-BMK-006`: authoritative, no fallback).
    pub(crate) fn non_zero_exit(_code: i32) -> super::BenchmarkFailure {
        super::BenchmarkFailure {
            kind: "verifier-non-zero-exit",
            boundary: FailureBoundaryCode::Grader,
        }
    }

    /// Verifier timeout settled by the supervisor.
    pub(crate) const TIMED_OUT: super::BenchmarkFailure = super::BenchmarkFailure {
        kind: "verifier-timeout",
        boundary: FailureBoundaryCode::Grader,
    };

    /// Verifier cancellation settled by the supervisor.
    pub(crate) const CANCELLED: super::BenchmarkFailure = super::BenchmarkFailure {
        kind: "verifier-cancelled",
        boundary: FailureBoundaryCode::Grader,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{
        IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
    };
    use std::collections::BTreeMap;

    /// A helper adapter proving the shared driver without any benchmark
    /// semantics: argv is a real process, settle maps exit 0 to Verified and
    /// everything else to the shared failure kinds.
    struct HelperAdapter;

    impl BenchmarkAdapter for HelperAdapter {
        fn identity(&self) -> BenchmarkIdentity {
            BenchmarkIdentity {
                benchmark: "helper".to_owned(),
                revision: "helper-1".to_owned(),
                adapter: "helper-adapter/0".to_owned(),
            }
        }

        fn admission(&self, _request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
            Ok(())
        }

        fn spawn_spec(&self, _request: &BenchmarkRunRequest) -> SpawnSpec {
            SpawnSpec {
                argv: vec!["/usr/bin/true".into()],
                cwd: None,
                env: BTreeMap::new(),
                stdout_cap: 1024,
                stderr_cap: 1024,
                timeout: Duration::from_secs(10),
            }
        }

        fn settle(
            &self,
            outcome: &crate::process::SupervisedOutcome,
            _request: &BenchmarkRunRequest,
        ) -> (Fact, BenchmarkCompletion) {
            let completion = match outcome.exit {
                ExitState::Exited { code: 0 } => BenchmarkCompletion::Verified {
                    metrics: NativeMetrics::default(),
                    artifacts: vec![],
                },
                ExitState::Exited { code } => {
                    BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(code))
                }
                ExitState::TimedOut => BenchmarkCompletion::Failed(failure_kinds::TIMED_OUT),
                ExitState::Cancelled => BenchmarkCompletion::Failed(failure_kinds::CANCELLED),
                ExitState::FailedToSpawn { reason } => {
                    BenchmarkCompletion::Failed(failure_kinds::spawn(reason))
                }
            };
            (
                Fact::Unknown {
                    reason: "native-reward-pending-18-15-smoke".to_owned(),
                },
                completion,
            )
        }
    }

    fn integrity(status: RevisionStatus, task: TaskClassification) -> IntegrityRecord {
        IntegrityRecord::review(IntegrityReview {
            benchmark: "helper-bench".to_owned(),
            revision: "helper-1".to_owned(),
            dataset: "helper-dataset".to_owned(),
            grader: "helper-grader".to_owned(),
            environment: "helper-environment".to_owned(),
            upstream_identity: "helper-upstream".to_owned(),
            upstream_digest: "0".repeat(64),
            oracle: Some(OraclePreflight::Passed("six tests passed".to_owned())),
            status,
            tasks: BTreeMap::from([("helper-task".to_owned(), task)]),
            excluded_trials: BTreeMap::new(),
            reviewer: "human-reviewer".to_owned(),
        })
        .unwrap()
    }

    fn request(integrity: IntegrityRecord) -> BenchmarkRunRequest {
        BenchmarkRunRequest {
            verifier_executable: "/nonexistent/uv".into(),
            task_dir: "/nonexistent/task".into(),
            task_id: "helper-task".to_owned(),
            native_selection: None,
            agent_timeout_seconds: None,
            verifier_timeout_seconds: None,
            trace_root: std::env::temp_dir(),
            admitted_lock_digest: "a".repeat(64),
            integrity,
            extra_env: BTreeMap::new(),
        }
    }

    #[test]
    fn diagnostic_job_config_selects_only_registered_names() {
        for family in ["harbor", "pier"] {
            for (name, class) in [("oracle", "OracleAgent"), ("nop", "NopAgent")] {
                let root = tempfile::tempdir().unwrap();
                let mut selection = stage_bound_job(root.path(), "j1", name);
                selection.kind = SelectionKind::DiagnosticWitness;
                selection.agent_import_path = format!("{family}.agents.{name}:{class}");
                let mut request = request(integrity(
                    RevisionStatus::Admitted,
                    TaskClassification::ValidAgentOutcome,
                ));
                request.task_dir = root.path().join("task-packages/j1");
                request.native_selection = Some(selection);
                request.agent_timeout_seconds = Some(7);
                request.verifier_timeout_seconds = Some(9);
                let job = native_job_config(&request).unwrap();
                assert_eq!(job["agents"][0]["name"], name);
                assert!(job["agents"][0].get("import_path").is_none());
                assert_eq!(job["agents"][0]["model_name"], "scripted:eval");
                assert_eq!(job["agents"][0]["max_timeout_sec"], 7);
                assert_eq!(job["agents"][0]["override_timeout_sec"], 7);
                assert_eq!(job["verifier"]["max_timeout_sec"], 9);
                assert_eq!(job["retry"]["max_retries"], 0);
                assert_eq!(job["n_attempts"], 1);
                assert!(job["environment"]["import_path"].is_null());
                let admitted = request.native_selection.as_ref().unwrap().clone();
                for (bad_name, bad_import) in [
                    ("default", admitted.agent_import_path.as_str()),
                    (name, "other.agents.oracle:OracleAgent"),
                    ("nop", "harbor.agents.oracle:OracleAgent"),
                ] {
                    let selected = request.native_selection.as_mut().unwrap();
                    selected.agent_name = bad_name.to_owned();
                    selected.agent_import_path = bad_import.to_owned();
                    assert_eq!(
                        native_job_config(&request).unwrap_err().token,
                        "native-diagnostic-selection-invalid"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn structurally_unusable_requests_reject_before_spawn() {
        // Unadmitted revision (EVAL-INT-001).
        let bad = request(integrity(
            RevisionStatus::NotAdmitted,
            TaskClassification::ValidAgentOutcome,
        ));
        assert_eq!(
            BenchmarkExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "revision-not-admitted"
            }
        );

        // Task not classified as a valid agent outcome (EVAL-INT-002).
        for task in [
            TaskClassification::BrokenOrUnsatisfiable {
                reason: "unsatisfiable".to_owned(),
            },
            TaskClassification::InfrastructureFailure {
                reason: "flaky host".to_owned(),
            },
        ] {
            let bad = request(integrity(RevisionStatus::Admitted, task));
            assert_eq!(
                BenchmarkExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                    .await
                    .unwrap_err(),
                ExecutionError {
                    token: "task-not-valid"
                }
            );
        }

        // Missing task classification entirely.
        let mut unclassified = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        unclassified.task_id = "not-in-record".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&unclassified, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "task-not-valid"
            }
        );

        // Empty task id and malformed lock digest.
        let mut empty = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        empty.task_id = "  ".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&empty, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "empty-task-id"
            }
        );
        let mut bad_digest = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        bad_digest.admitted_lock_digest = "not-a-digest".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&bad_digest, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "lock-digest-malformed"
            }
        );

        // Adapter-owned admission runs after the shared gates.
        struct RejectingAdapter(HelperAdapter);
        impl BenchmarkAdapter for RejectingAdapter {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, _request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                Err(ExecutionError {
                    token: "revision-binding-mismatch",
                })
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                self.0.spawn_spec(r)
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let good = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        assert_eq!(
            BenchmarkExecution::run(
                &good,
                &RejectingAdapter(HelperAdapter),
                &CancellationToken::new()
            )
            .await
            .unwrap_err(),
            ExecutionError {
                token: "revision-binding-mismatch"
            }
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn helper_run_settles_a_verified_record_with_provenance() {
        let record = integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        );
        let digest = record.identity_digest().to_owned();
        let request = request(record);
        let settled = BenchmarkExecution::run(&request, &HelperAdapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(settled.exit, ExitState::Exited { code: 0 });
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Verified {
                metrics: NativeMetrics::default(),
                artifacts: vec![]
            }
        );
        assert_eq!(settled.failure_boundary(), None);
        assert_eq!(
            settled.provenance,
            BenchmarkProvenance {
                admitted_lock_digest: "a".repeat(64),
                integrity_digest: digest,
                revision: "helper-1".to_owned(),
                task_id: "helper-task".to_owned(),
            }
        );
        // The reward fact stays a typed unknown until the real native smoke.
        assert_eq!(
            settled.reward,
            Fact::Unknown {
                reason: "native-reward-pending-18-15-smoke".to_owned()
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn helper_verifier_failures_settle_with_typed_boundaries() {
        struct FailingHelper(HelperAdapter);
        impl BenchmarkAdapter for FailingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sh".into(), "-c".into(), "exit 4".into()];
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let request = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        let settled = BenchmarkExecution::run(
            &request,
            &FailingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(settled.exit, ExitState::Exited { code: 4 });
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(4))
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Grader)
        );

        struct SleepingHelper(HelperAdapter);
        impl BenchmarkAdapter for SleepingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sleep".into(), "15".into()];
                spec.timeout = Duration::from_millis(200);
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let settled = BenchmarkExecution::run(
            &request,
            &SleepingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(settled.exit, ExitState::TimedOut);
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::TIMED_OUT)
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Grader)
        );

        struct MissingHelper(HelperAdapter);
        impl BenchmarkAdapter for MissingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/nonexistent/verifier".into()];
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let settled = BenchmarkExecution::run(
            &request,
            &MissingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::spawn(SpawnReason::NotFound))
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Infrastructure)
        );
    }
}

#[cfg(test)]
fn write_test_terminal(
    trial_dir: &std::path::Path,
    product: &str,
    executable_sha256: &str,
    delivered: serde_json::Value,
) {
    let stdout = b"fixture version\n";
    let stderr = b"";
    std::fs::write(trial_dir.join("agent/runtime-preflight-stdout.log"), stdout).unwrap();
    std::fs::write(trial_dir.join("agent/runtime-preflight-stderr.log"), stderr).unwrap();
    let terminal = serde_json::json!({
        "schema": "opi-eval-native-agent-terminal/2", "trial": "logical-trial",
        "product": product, "agent_executable_sha256": executable_sha256,
        "exit_code": 0, "task_dispatched": true, "timed_out": false, "delivered": delivered,
        "runtime_preflight": {
            "kind": "version", "status": "passed", "exit_code": 0,
            "agent_executable_sha256": executable_sha256,
            "runtime_package_manifest_sha256": null, "runtime_package_entrypoint": null,
            "command_sha256": crate::runner::material::sha256_hex(format!("/opi-eval-agent/{product}/agent --version").as_bytes()),
            "stdout_sha256": crate::runner::material::sha256_hex(stdout),
            "stderr_sha256": crate::runner::material::sha256_hex(stderr),
            "stdout_complete": true, "stderr_complete": true,
            "stdout_truncated": false, "stderr_truncated": false, "reason": null,
        },
    });
    std::fs::write(
        trial_dir.join("agent/opi-eval-terminal.json"),
        serde_json::to_vec(&terminal).unwrap(),
    )
    .unwrap();
}

/// Stages one complete bound native job tree under `root/jobs/<job>`
/// and returns the selection the tree satisfies (test staging for the
/// binding matrix).
#[cfg(test)]
fn stage_bound_job(root: &std::path::Path, job: &str, agent_name: &str) -> NativeSelection {
    let trial = "fixture-task__bZZeEkw";
    let trial_dir = root.join("jobs").join(job).join(trial);
    std::fs::create_dir_all(trial_dir.join("agent")).unwrap();
    std::fs::create_dir_all(trial_dir.join("artifacts")).unwrap();
    let answer = b"native stand-in answer\n";
    let answer_digest = crate::runner::material::sha256_hex(answer);
    // Job aggregate: one trial, eval key grouped under the agent name,
    // the reward attributed to exactly the trial.
    let aggregate = serde_json::json!({
        "n_total_trials": 1,
        "stats": {"evals": {format!("{agent_name}__adhoc"): {"reward_stats": {
            "reward": {"1.0": [trial]}}}}},
    });
    std::fs::write(
        root.join("jobs").join(job).join("result.json"),
        serde_json::to_vec(&aggregate).unwrap(),
    )
    .unwrap();
    // Per-trial result: the actual agent, task, and native reward.
    let results = serde_json::json!({
        "task_name": "fixture-task",
        "trial_name": trial,
        "agent_info": {"name": agent_name, "version": "1.0.0",
            "model_info": {"name": "scripted:eval"}},
        "verifier_result": {"rewards": {"reward": 1.0}},
        "verifier_environment_mode": "shared",
        "exception_info": null,
    });
    std::fs::write(
        trial_dir.join("result.json"),
        serde_json::to_vec(&results).unwrap(),
    )
    .unwrap();
    // Explicit agent selection and a native task reference. Trial-level
    // artifacts are additions to the referenced task's declarations.
    let task_dir = root.join("task-packages").join(job);
    std::fs::create_dir_all(&task_dir).unwrap();
    std::fs::write(task_dir.join("task.toml"), "schema_version = '1.0'\n").unwrap();
    let config = serde_json::json!({
        "agent": {"name": "opi_eval_harbor:OpiAgent", "import_path": null},
        "artifacts": ["/root/answer.txt"],
        "task": {"path": task_dir},
    });
    std::fs::write(
        trial_dir.join("config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    write_test_terminal(
        &trial_dir,
        "opi",
        &"a".repeat(64),
        serde_json::json!([{"source": "/root/answer.txt", "sha256": answer_digest}]),
    );
    // Collected artifacts: the delivered file, byte-identical, mirrored
    // at its source-relative host path exactly as the pinned runner
    // collects it.
    let manifest = serde_json::json!([
        {"source": "/root/answer.txt", "destination": "artifacts/root/answer.txt",
         "type": "file", "status": "ok", "service": null},
    ]);
    std::fs::write(
        trial_dir.join("artifacts/manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::create_dir_all(trial_dir.join("artifacts/root")).unwrap();
    std::fs::write(trial_dir.join("artifacts/root/answer.txt"), answer).unwrap();
    NativeSelection {
        kind: SelectionKind::Integration {
            product: "opi".to_owned(),
            executable_sha256: "a".repeat(64),
            trial: "logical-trial".to_owned(),
            runtime_package_manifest_sha256: None,
            runtime_package_entrypoint: None,
        },
        lifecycle: TaskLifecycleSelection {
            environment_mode: super::task_lifecycle::VerifierEnvironmentMode::Shared,
            declared: vec![super::task_lifecycle::DeclaredArtifact {
                source: "/root/answer.txt".to_owned(),
                destination: None,
                service: None,
            }],
            base_revision: None,
            docker_image: None,
        },
        agent_name: agent_name.to_owned(),
        agent_import_path: "opi_eval_harbor:OpiAgent".to_owned(),
        task_name: "fixture-task".to_owned(),
        model: "scripted:eval".to_owned(),
        reward_metric: "reward".to_owned(),
        module_dir: "/nonexistent/integrations".into(),
        handoff_path: "/nonexistent/handoff.json".into(),
        stale_job_names: Vec::new(),
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_verifier_timeout_binds_without_inventing_reward() {
    for mutation in [
        "none",
        "wrong-trial",
        "missing-error",
        "missing-terminal",
        "wrong-product",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
        let job = dir.path().join("jobs/j1");
        let trial = job.join("fixture-task__bZZeEkw");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(trial.join("result.json")).unwrap()).unwrap();
        result["verifier_result"] = serde_json::Value::Null;
        result["exception_info"] = serde_json::json!({"exception_type": "VerifierTimeoutError"});
        std::fs::write(
            trial.join("result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        let mut aggregate = serde_json::json!({
            "n_total_trials": 1,
            "stats": {"evals": {"opi-eval-opi__adhoc": {
                "n_trials": 0, "n_errors": 1, "reward_stats": {},
                "exception_stats": {"VerifierTimeoutError": ["fixture-task__bZZeEkw"]},
            }}},
        });
        match mutation {
            "wrong-trial" => {
                aggregate["stats"]["evals"]["opi-eval-opi__adhoc"]["exception_stats"] =
                    serde_json::json!({"VerifierTimeoutError": ["other-trial"]})
            }
            "missing-error" => {
                aggregate["stats"]["evals"]["opi-eval-opi__adhoc"]["n_errors"] = 0.into()
            }
            "missing-terminal" => {
                std::fs::remove_file(trial.join("agent/opi-eval-terminal.json")).unwrap()
            }
            "wrong-product" => {
                let path = trial.join("agent/opi-eval-terminal.json");
                let bytes = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("\"product\":\"opi\"", "\"product\":\"pi\"");
                std::fs::write(path, bytes).unwrap();
            }
            _ => {}
        }
        std::fs::write(
            job.join("result.json"),
            serde_json::to_vec(&aggregate).unwrap(),
        )
        .unwrap();
        let binding = import_bound_native_job(dir.path(), &selection);
        if mutation == "none" {
            let binding = binding.unwrap();
            assert_eq!(
                binding.exception_type.as_deref(),
                Some("VerifierTimeoutError")
            );
            assert_eq!(
                binding.reward,
                Fact::Unknown {
                    reason: "verifier-timeout".to_owned()
                }
            );
        } else {
            assert!(binding.is_err(), "{mutation}: {binding:?}");
        }
    }
}

// An agent timeout whose verifier still ran and scored binds the graded
// reward normally (the common shape: capture refused downstream settles
// the trial, the run continues).
#[test]
fn native_agent_timeout_with_graded_verifier_binds_reward() {
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let trial = dir.path().join("jobs/j1/fixture-task__bZZeEkw");
    let mut result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(trial.join("result.json")).unwrap()).unwrap();
    result["exception_info"] = serde_json::json!({"exception_type": "AgentTimeoutError"});
    result["verifier_result"]["rewards"]["reward"] = serde_json::json!(0.0);
    std::fs::write(
        trial.join("result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    let mut aggregate = serde_json::json!({
        "n_total_trials": 1,
        "stats": {"evals": {"opi-eval-opi__adhoc": {
            "n_trials": 1, "n_errors": 1,
            "reward_stats": {"reward": {"0.0": ["fixture-task__bZZeEkw"]}},
            "exception_stats": {"AgentTimeoutError": ["fixture-task__bZZeEkw"]},
        }}},
    });
    aggregate["stats"]["evals"]["opi-eval-opi__adhoc"]["reward_stats"] =
        serde_json::json!({"reward": {"0.0": ["fixture-task__bZZeEkw"]}});
    std::fs::write(
        dir.path().join("jobs/j1/result.json"),
        serde_json::to_vec(&aggregate).unwrap(),
    )
    .unwrap();
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(
        binding.reward,
        Fact::Known {
            value: 0,
            origin: "native-trial-result".to_owned()
        }
    );
}

// An agent-timeout job whose stats carry an empty-reward eval entry
// attributed to the exception (the other pinned-runner shape; observed on
// tb21 `model-extraction-relu-logits`) also binds an honest Unknown.
#[test]
fn native_agent_timeout_with_empty_reward_stats_binds_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let job = dir.path().join("jobs/j1");
    let trial = job.join("fixture-task__bZZeEkw");
    let mut result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(trial.join("result.json")).unwrap()).unwrap();
    result["verifier_result"] = serde_json::Value::Null;
    result["exception_info"] = serde_json::json!({"exception_type": "AgentTimeoutError"});
    std::fs::write(
        trial.join("result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    let aggregate = serde_json::json!({
        "n_total_trials": 1,
        "stats": {"evals": {"opi-eval-opi__adhoc": {
            "n_trials": 0, "n_errors": 1, "reward_stats": {},
            "exception_stats": {"AgentTimeoutError": ["fixture-task__bZZeEkw"]},
        }}},
    });
    std::fs::write(
        job.join("result.json"),
        serde_json::to_vec(&aggregate).unwrap(),
    )
    .unwrap();
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(binding.exception_type.as_deref(), Some("AgentTimeoutError"));
    assert_eq!(
        binding.reward,
        Fact::Unknown {
            reason: "agent-timeout".to_owned()
        }
    );
}

#[test]
fn native_binding_failure_boundaries_separate_setup_refusal_from_corruption() {
    use crate::failure::FailureBoundaryCode;
    // An honest setup refusal (bound identity, terminal observed, task
    // never dispatched) settles as infrastructure: the trial is invalid
    // but the run continues.
    assert_eq!(
        NativeBindingError::TrialInvalid("task-not-observed").settled_boundary(),
        FailureBoundaryCode::Infrastructure
    );
    // Identity or provenance corruption stays integrity-fatal.
    for error in [
        NativeBindingError::TrialInvalid("terminal-observation"),
        NativeBindingError::TrialInvalid("agent-name"),
        NativeBindingError::TrialInvalid("runtime-preflight-output"),
        NativeBindingError::AggregateInvalid("no-eval-stats"),
        NativeBindingError::TrialResultMissing,
        NativeBindingError::NoNewJob,
    ] {
        assert_eq!(error.settled_boundary(), FailureBoundaryCode::Integrity);
    }
}

// A trial whose Agent timed out and whose verifier then also produced no
// eval stats (the pinned runner writes none when the verifier is killed on
// the half-built repository) binds an honest Unknown instead of failing
// the aggregate boundary. Observed on tb21 `torch-pipeline-parallelism`.
#[test]
fn native_agent_timeout_without_evals_binds_unknown() {
    for mutation in ["none", "with-verifier-result", "wrong-exception"] {
        let dir = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
        let job = dir.path().join("jobs/j1");
        let trial = job.join("fixture-task__bZZeEkw");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(trial.join("result.json")).unwrap()).unwrap();
        result["verifier_result"] = serde_json::Value::Null;
        result["exception_info"] = serde_json::json!({"exception_type": "AgentTimeoutError"});
        match mutation {
            "with-verifier-result" => {
                result["verifier_result"] = serde_json::json!({"rewards": {"accuracy": 0.0}})
            }
            "wrong-exception" => {
                result["exception_info"] =
                    serde_json::json!({"exception_type": "InfrastructureError"})
            }
            _ => {}
        }
        std::fs::write(
            trial.join("result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        // The pinned runner writes no eval entries at all for this shape.
        let aggregate = serde_json::json!({
            "n_total_trials": 1,
            "stats": {"evals": {}},
        });
        std::fs::write(
            job.join("result.json"),
            serde_json::to_vec(&aggregate).unwrap(),
        )
        .unwrap();
        let binding = import_bound_native_job(dir.path(), &selection);
        if mutation == "none" {
            let binding = binding.unwrap();
            assert_eq!(binding.exception_type.as_deref(), Some("AgentTimeoutError"));
            assert_eq!(
                binding.reward,
                Fact::Unknown {
                    reason: "agent-timeout".to_owned()
                }
            );
        } else {
            assert!(binding.is_err(), "{mutation}: {binding:?}");
        }
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_model_binding_uses_only_the_pinned_slash_split() {
    for (selector, model, accepted) in [
        (
            "scripted:eval",
            serde_json::json!({"name": "scripted:eval"}),
            true,
        ),
        (
            "scripted:eval",
            serde_json::json!({"name": "scripted:eval", "provider": false}),
            false,
        ),
        (
            "scripted:org/eval",
            serde_json::json!({"name": "eval", "provider": "scripted:org"}),
            true,
        ),
        (
            "scripted:org/eval",
            serde_json::json!({"name": "eval"}),
            false,
        ),
        (
            "scripted:org/eval",
            serde_json::json!({"name": "eval", "provider": "other"}),
            false,
        ),
        (
            "scripted:org/eval",
            serde_json::json!({"name": "scripted:org/eval"}),
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
        selection.model = selector.to_owned();
        let result_path = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
        result["agent_info"]["model_info"] = model;
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        let bound = import_bound_native_job(dir.path(), &selection);
        if accepted {
            assert!(bound.is_ok(), "{selector}: {bound:?}");
        } else {
            assert_eq!(
                bound.unwrap_err(),
                NativeBindingError::TrialInvalid("selected-model-mismatch")
            );
        }
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_imports_harbor_pi_zero_reward() {
    let dir = tempfile::tempdir().unwrap();
    let job = dir.path().join("jobs/2026-09-09__01-55-07");
    let trial_dir = job.join("task-package__Z7PUBXF");
    std::fs::create_dir_all(trial_dir.join("agent")).unwrap();
    // Minimal structural projection of the Harbor v0.22.0 result, with
    // host paths and Agent content omitted.
    std::fs::write(
        job.join("result.json"),
        r#"{
            "n_total_trials": 1,
            "stats": {"n_completed_trials": 1, "n_errored_trials": 0, "evals": {
                "opi-eval-pi__zhipu:glm-5.3-flash__adhoc": {
                    "n_trials": 1, "n_errors": 0,
                    "reward_stats": {"reward": {"0.0": ["task-package__Z7PUBXF"]}},
                    "exception_stats": {}
                }
            }}
        }"#,
    )
    .unwrap();
    std::fs::write(
        trial_dir.join("result.json"),
        r#"{
            "task_name": "terminal-bench/openssl-selfsigned-cert",
            "trial_name": "task-package__Z7PUBXF",
            "agent_info": {"name": "opi-eval-pi", "version": "1",
                "model_info": {"name": "zhipu:glm-5.3-flash", "provider": null}},
            "verifier_result": {"rewards": {"reward": 0.0}},
            "verifier_environment_mode": "shared",
            "exception_info": null
        }"#,
    )
    .unwrap();
    let task_dir = dir.path().join("task-package");
    std::fs::create_dir_all(&task_dir).unwrap();
    std::fs::write(task_dir.join("task.toml"), "schema_version = '1.0'\n").unwrap();
    std::fs::write(
        trial_dir.join("config.json"),
        serde_json::to_vec(&serde_json::json!({
            "agent": {"name": "opi_eval_harbor:PiAgent", "import_path": null},
            "task": {"path": task_dir}, "artifacts": []
        }))
        .unwrap(),
    )
    .unwrap();
    write_test_terminal(
        &trial_dir,
        "pi",
        "443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403",
        serde_json::json!([]),
    );
    let selection = NativeSelection {
        kind: SelectionKind::Integration {
            product: "pi".to_owned(),
            executable_sha256: "443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403"
                .to_owned(),
            trial: "logical-trial".to_owned(),
            runtime_package_manifest_sha256: None,
            runtime_package_entrypoint: None,
        },
        lifecycle: TaskLifecycleSelection {
            environment_mode: super::task_lifecycle::VerifierEnvironmentMode::Shared,
            declared: Vec::new(),
            base_revision: None,
            docker_image: None,
        },
        agent_name: "opi-eval-pi".to_owned(),
        agent_import_path: "opi_eval_harbor:PiAgent".to_owned(),
        task_name: "terminal-bench/openssl-selfsigned-cert".to_owned(),
        model: "zhipu:glm-5.3-flash".to_owned(),
        reward_metric: "reward".to_owned(),
        module_dir: "/nonexistent/integrations".into(),
        handoff_path: "/nonexistent/handoff.json".into(),
        stale_job_names: Vec::new(),
    };
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(binding.results_path, trial_dir.join("result.json"));
    assert_eq!(binding.agent_name, "opi-eval-pi");
    assert_eq!(binding.agent_version, "1");
    assert_eq!(binding.model_name.as_deref(), Some("zhipu:glm-5.3-flash"));
    assert_eq!(binding.task_name, selection.task_name);
    assert_eq!(binding.trial_name, "task-package__Z7PUBXF");
    assert_eq!(
        binding.reward,
        Fact::Known {
            value: 0,
            origin: "native-trial-result".to_owned(),
        }
    );
    let terminal = binding.terminal.unwrap();
    assert_eq!(terminal.product, "pi");
    assert_eq!(terminal.exit_code, Some(0));
    assert_eq!(
        terminal.agent_executable_sha256,
        "443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403"
    );
    assert!(binding.exception_type.is_none());
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_imports_pinned_separate_collection() {
    use super::task_lifecycle::{DeclaredArtifact, VerifierEnvironmentMode};

    for pier in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
        let trial_dir = dir.path().join("jobs/j1/fixture-task__bZZeEkw");
        let task_dir = dir.path().join("task-package");
        std::fs::create_dir_all(&task_dir).unwrap();
        let (source, destination, content_path, task, agent) = if pier {
            (
                "/logs/artifacts/model.patch",
                "model.patch",
                "model.patch",
                "schema_version = '1.0'\nartifacts = ['/logs/artifacts/model.patch']\n\
                 [metadata]\nbase_commit_hash = '0123456789abcdef0123456789abcdef01234567'\n\
                 [environment]\ndocker_image = 'fixture/pristine:locked'\n\
                 [verifier]\nenvironment_mode = 'separate'\nnetwork_mode = 'no-network'\n\
                 [[verifier.collect]]\ncommand = 'git diff HEAD > /logs/artifacts/model.patch'\n",
                serde_json::json!({"name": null, "import_path": "opi_eval_pier:OpiAgent"}),
            )
        } else {
            (
                "/app/evalbench/",
                "app/evalbench",
                "app/evalbench/answer.py",
                "schema_version = '1.0'\nartifacts = ['/app/evalbench/']\n\
                 [verifier]\nenvironment_mode = 'separate'\n[verifier.environment]\n",
                serde_json::json!({"name": "opi_eval_harbor:OpiAgent"}),
            )
        };
        std::fs::write(task_dir.join("task.toml"), task).unwrap();
        let config = serde_json::json!({
            "task": {"path": task_dir}, "agent": agent, "artifacts": []
        });
        std::fs::write(
            trial_dir.join("config.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let result_path = trial_dir.join("result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
        if pier {
            result
                .as_object_mut()
                .unwrap()
                .remove("verifier_environment_mode");
            selection.agent_import_path = "opi_eval_pier:OpiAgent".to_owned();
            selection.lifecycle.base_revision =
                Some("0123456789abcdef0123456789abcdef01234567".to_owned());
            selection.lifecycle.docker_image = Some("fixture/pristine:locked".to_owned());
        } else {
            result["verifier_environment_mode"] = serde_json::json!("separate");
        }
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        selection.lifecycle.environment_mode = VerifierEnvironmentMode::Separate;
        selection.lifecycle.declared = vec![DeclaredArtifact {
            source: source.to_owned(),
            destination: None,
            service: None,
        }];
        let bytes = b"fixture collected content\n";
        let host = trial_dir.join("artifacts").join(content_path);
        std::fs::create_dir_all(host.parent().unwrap()).unwrap();
        std::fs::write(&host, bytes).unwrap();
        let manifest = serde_json::json!([{
            "source": source, "destination": format!("artifacts/{destination}"),
            "type": if pier { "file" } else { "directory" }, "status": "ok"
        }]);
        std::fs::write(
            trial_dir.join("artifacts/manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let terminal_path = trial_dir.join("agent/opi-eval-terminal.json");
        let mut terminal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&terminal_path).unwrap()).unwrap();
        terminal["delivered"] = serde_json::json!([{
            "source": if pier { "/logs/artifacts/model.patch" } else { "/app/evalbench/answer.py" },
            "sha256": crate::runner::material::sha256_hex(bytes)
        }]);
        std::fs::write(&terminal_path, serde_json::to_vec(&terminal).unwrap()).unwrap();

        let binding = import_bound_native_job(dir.path(), &selection).unwrap();
        assert_eq!(binding.environment_mode, VerifierEnvironmentMode::Separate);
        assert_eq!(binding.base_revision, selection.lifecycle.base_revision);
        assert_eq!(binding.docker_image, selection.lifecycle.docker_image);
        assert_eq!(
            binding.collected,
            vec![CollectedArtifact {
                source: source.to_owned(),
                destination: destination.to_owned(),
                sha256: collected_digest(&trial_dir.join("artifacts").join(destination)).unwrap(),
            }]
        );

        // Native result mode and the referenced task must agree. Pier
        // carries no such result field; its task supplies the mode.
        if !pier {
            result["verifier_environment_mode"] = serde_json::json!("shared");
            std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
            assert_eq!(
                import_bound_native_job(dir.path(), &selection),
                Err(NativeBindingError::TrialInvalid("task-lifecycle"))
            );
            result["verifier_environment_mode"] = serde_json::json!("separate");
            std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        }
        let mut wrong_selection = selection.clone();
        if pier {
            wrong_selection.lifecycle.base_revision = Some("f".repeat(40));
        } else {
            wrong_selection.lifecycle.declared.clear();
        }
        assert!(matches!(
            import_bound_native_job(dir.path(), &wrong_selection),
            Err(NativeBindingError::LifecycleMismatch { .. })
        ));

        // A failed native collection never becomes a valid input merely
        // because the host happens to contain bytes at its destination.
        let mut invalid_manifest = manifest.clone();
        invalid_manifest[0]["status"] = serde_json::json!("failed");
        std::fs::write(
            trial_dir.join("artifacts/manifest.json"),
            serde_json::to_vec(&invalid_manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            import_bound_native_job(dir.path(), &selection),
            Err(NativeBindingError::DeclaredArtifactMissing {
                source: source.to_owned()
            })
        );
        for invalid_destination in ["app/evalbench", "artifacts/../outside"] {
            invalid_manifest[0]["status"] = serde_json::json!("ok");
            invalid_manifest[0]["destination"] = serde_json::json!(invalid_destination);
            std::fs::write(
                trial_dir.join("artifacts/manifest.json"),
                serde_json::to_vec(&invalid_manifest).unwrap(),
            )
            .unwrap();
            assert_eq!(
                import_bound_native_job(dir.path(), &selection),
                Err(NativeBindingError::TrialInvalid("artifacts-manifest"))
            );
        }
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_requires_one_direct_trial_result() {
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let trial_dir = dir.path().join("jobs/j1/fixture-task__bZZeEkw");
    let result = trial_dir.join("result.json");
    let bytes = std::fs::read(&result).unwrap();

    // The aggregate alone cannot supply a trial identity or reward.
    std::fs::remove_file(&result).unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::TrialResultMissing)
    );

    // Only the pinned filename and directory depth are admitted.
    std::fs::write(trial_dir.join("results.json"), &bytes).unwrap();
    let nested = dir.path().join("jobs/j1/trials/other-trial");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("result.json"), &bytes).unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::TrialResultMissing)
    );

    // Two direct trial results are ambiguous even if the aggregate names
    // just one of them.
    std::fs::write(&result, &bytes).unwrap();
    let extra = dir.path().join("jobs/j1/other-trial");
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(extra.join("result.json"), &bytes).unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::TrialResultMissing)
    );
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_imports_multi_metric_primary_reward() {
    for (metric, reward) in [("reward", 1), ("reward", 0), ("custom_primary", 1)] {
        let dir = tempfile::tempdir().unwrap();
        let selection = stage_multi_metric_job(dir.path(), metric, reward);
        let aggregate_path = dir.path().join("jobs/j1/result.json");
        let results_path = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
        let aggregate_bytes = std::fs::read(&aggregate_path).unwrap();
        let results_bytes = std::fs::read(&results_path).unwrap();

        let binding = import_bound_native_job(dir.path(), &selection).unwrap();
        assert_eq!(
            binding.reward,
            Fact::Known {
                value: reward,
                origin: "native-trial-result".to_owned()
            }
        );
        assert_eq!(binding.results_path, results_path);
        assert_eq!(std::fs::read(aggregate_path).unwrap(), aggregate_bytes);
        assert_eq!(std::fs::read(binding.results_path).unwrap(), results_bytes);
    }
}

#[cfg(all(test, unix))]
fn stage_multi_metric_job(root: &std::path::Path, metric: &str, reward: u64) -> NativeSelection {
    let mut selection = stage_bound_job(root, "j1", "opi-eval-opi");
    selection.reward_metric = metric.to_owned();
    // Pinned Pier records every native reward metric, including test counts.
    let mut rewards = serde_json::json!({
        "f2p_total": 20, "f2p_passed": 20, "p2p_total": 3, "p2p_passed": 3,
        "f2p": 1.0, "p2p": 1.0, "partial": 1.0,
    });
    rewards[metric] = reward.into();
    let mut stats = serde_json::json!({});
    for (name, value) in rewards.as_object().unwrap() {
        stats[name] = serde_json::json!({value.to_string(): ["fixture-task__bZZeEkw"]});
    }
    let aggregate_path = root.join("jobs/j1/result.json");
    let mut aggregate: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&aggregate_path).unwrap()).unwrap();
    aggregate["stats"]["evals"]["opi-eval-opi__adhoc"]["reward_stats"] = stats;
    std::fs::write(aggregate_path, serde_json::to_vec(&aggregate).unwrap()).unwrap();
    let results_path = root.join("jobs/j1/fixture-task__bZZeEkw/result.json");
    let mut results: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&results_path).unwrap()).unwrap();
    results["verifier_result"]["rewards"] = rewards;
    std::fs::write(results_path, serde_json::to_vec(&results).unwrap()).unwrap();
    selection
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_rejects_invalid_multi_metric_primary_reward() {
    let trial = "fixture-task__bZZeEkw";
    for (case, expected) in [
        (
            "missing-primary",
            NativeBindingError::AggregateInvalid("no-reward-stats"),
        ),
        (
            "multiple-buckets",
            NativeBindingError::AggregateInvalid("reward-count"),
        ),
        ("wrong-trial", NativeBindingError::AggregateRewardMismatch),
        (
            "duplicate-trial",
            NativeBindingError::AggregateRewardMismatch,
        ),
        ("wrong-primary", NativeBindingError::AggregateRewardMismatch),
        (
            "nonbinary-primary",
            NativeBindingError::AggregateInvalid("bad-reward-values"),
        ),
        ("missing-trial-primary", NativeBindingError::RewardMissing),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let selection = stage_multi_metric_job(dir.path(), "reward", 1);
        let path = dir.path().join("jobs/j1/result.json");
        let mut aggregate: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let stats = &mut aggregate["stats"]["evals"]["opi-eval-opi__adhoc"]["reward_stats"];
        match case {
            "missing-primary" => {
                stats.as_object_mut().unwrap().remove("reward");
            }
            "multiple-buckets" => stats["reward"]["0"] = serde_json::json!([trial]),
            "wrong-trial" => stats["reward"]["1"] = serde_json::json!(["other-trial"]),
            "duplicate-trial" => stats["reward"]["1"] = serde_json::json!([trial, trial]),
            "wrong-primary" => stats["reward"] = serde_json::json!({"0": [trial]}),
            "nonbinary-primary" => stats["reward"] = serde_json::json!({"20": [trial]}),
            "missing-trial-primary" => {
                let result_path = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
                let mut result: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
                result["verifier_result"]["rewards"]
                    .as_object_mut()
                    .unwrap()
                    .remove("reward");
                std::fs::write(result_path, serde_json::to_vec(&result).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        std::fs::write(path, serde_json::to_vec(&aggregate).unwrap()).unwrap();
        assert_eq!(
            import_bound_native_job(dir.path(), &selection),
            Err(expected),
            "{case}"
        );
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_admits_one_bound_job() {
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "2026-09-05__10-00-00", "opi-eval-opi");
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(binding.agent_name, "opi-eval-opi");
    assert_eq!(binding.task_name, "fixture-task");
    assert_eq!(binding.trial_name, "fixture-task__bZZeEkw");
    assert_eq!(
        binding.reward,
        Fact::Known {
            value: 1,
            origin: "native-trial-result".to_owned()
        }
    );
    assert_eq!(binding.terminal.as_ref().unwrap().exit_code, Some(0));
    assert_eq!(binding.terminal.as_ref().unwrap().delivered.len(), 1);
    // The declared artifact binds as the verifier-input identity with the
    // digest of exactly the collected bytes.
    assert_eq!(binding.collected.len(), 1);
    assert_eq!(binding.collected[0].source, "/root/answer.txt");
    assert_eq!(binding.collected[0].destination, "root/answer.txt");
    assert_eq!(
        binding.collected[0].sha256,
        crate::runner::material::sha256_hex(b"native stand-in answer\n")
    );
    assert_eq!(binding.model_name.as_deref(), Some("scripted:eval"));
    assert!(binding.manifest_path.is_some());

    // A pre-existing (stale) job is never the imported one: only the new
    // job binds.
    let stale = dir.path().join("jobs/2026-09-04__00-00-00");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(
        stale.join("result.json"),
        br#"{"n_total_trials": 1, "stats": {"evals": {"oracle__adhoc": {"reward_stats": {"reward": {"1.0": ["x"]}}}}}}"#,
    )
    .unwrap();
    let mut guarded = selection.clone();
    guarded.stale_job_names = vec!["2026-09-04__00-00-00".into()];
    let binding = import_bound_native_job(dir.path(), &guarded).unwrap();
    assert_eq!(binding.job_name, "2026-09-05__10-00-00");
}

#[test]
fn diagnostic_result_binding_requires_an_unambiguous_registered_name() {
    for family in ["harbor", "pier"] {
        for (name, class) in [("oracle", "OracleAgent"), ("nop", "NopAgent")] {
            let root = tempfile::tempdir().unwrap();
            let mut selection = stage_bound_job(root.path(), "j1", name);
            selection.kind = SelectionKind::DiagnosticWitness;
            selection.agent_import_path = format!("{family}.agents.{name}:{class}");
            let trial = root.path().join("jobs/j1/fixture-task__bZZeEkw");
            std::fs::remove_file(trial.join("agent/opi-eval-terminal.json")).unwrap();
            let path = trial.join("config.json");
            let mut config: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            for agent in [
                serde_json::json!({"name": name}),
                serde_json::json!({"name": name, "import_path": null}),
            ] {
                config["agent"] = agent;
                std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
                let binding = import_bound_native_job(root.path(), &selection).unwrap();
                assert_eq!(binding.agent_name, name);
                assert!(binding.terminal.is_none());
            }
            for agent in [
                serde_json::json!({}),
                serde_json::json!({"name": null, "import_path": null}),
                serde_json::json!({"name": "default"}),
                serde_json::json!({"name": if name == "oracle" { "nop" } else { "oracle" }}),
                serde_json::json!({"name": selection.agent_import_path}),
                serde_json::json!({"import_path": selection.agent_import_path}),
                serde_json::json!({"name": name, "import_path": selection.agent_import_path}),
                serde_json::json!({"name": name, "import_path": ""}),
                serde_json::json!({"name": name, "import_path": false}),
                serde_json::json!({"name": false}),
            ] {
                config["agent"] = agent;
                std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
                assert!(
                    import_bound_native_job(root.path(), &selection).is_err(),
                    "{config}"
                );
            }
        }
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_result_binding_rejects_every_substitution() {
    // Missing new job (only stale ones exist).
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "2026-09-05__10-00-00", "opi-eval-opi");
    let mut guarded = selection.clone();
    guarded.stale_job_names = vec!["2026-09-05__10-00-00".into()];
    assert_eq!(
        import_bound_native_job(dir.path(), &guarded),
        Err(NativeBindingError::NoNewJob)
    );

    // Oracle attribution: the trial ran under the oracle agent, so both
    // the per-trial identity and the aggregate grouping reject it.
    let dir = tempfile::tempdir().unwrap();
    let mut oracle = stage_bound_job(dir.path(), "j1", "oracle");
    oracle.agent_name = "opi-eval-opi".to_owned();
    assert_eq!(
        import_bound_native_job(dir.path(), &oracle),
        Err(NativeBindingError::AgentNameMismatch {
            expected: "opi-eval-opi".to_owned(),
            observed: "oracle".to_owned(),
        })
    );
    // Even when the per-trial identity is patched away, the aggregate is
    // still grouped under the oracle agent: rejected.
    let results = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
    std::fs::write(
        &results,
        std::fs::read_to_string(&results)
            .unwrap()
            .replace("\"name\":\"oracle\"", "\"name\":\"opi-eval-opi\""),
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &oracle),
        Err(NativeBindingError::AgentNameMismatch {
            expected: "opi-eval-opi".to_owned(),
            observed: "oracle__adhoc".to_owned(),
        })
    );

    // Wrong agent in the per-trial result.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let results = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
    std::fs::write(
        &results,
        std::fs::read_to_string(&results)
            .unwrap()
            .replace("\"name\":\"opi-eval-opi\"", "\"name\":\"opi-eval-pi\""),
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::AgentNameMismatch {
            expected: "opi-eval-opi".to_owned(),
            observed: "opi-eval-pi".to_owned(),
        })
    );

    // Substituted task.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let results = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
    std::fs::write(
        &results,
        std::fs::read_to_string(&results).unwrap().replace(
            "\"task_name\":\"fixture-task\"",
            "\"task_name\":\"other-task\"",
        ),
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::TaskNameMismatch {
            expected: "fixture-task".to_owned(),
            observed: "other-task".to_owned(),
        })
    );

    // Missing explicit agent selection (implicit default/oracle job).
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let config = dir.path().join("jobs/j1/fixture-task__bZZeEkw/config.json");
    std::fs::write(&config, r#"{"agent": {"name": null, "import_path": null}}"#).unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::AgentSelectionMissing)
    );

    // Substituted artifact: collected bytes differ from the delivered
    // observation.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    std::fs::write(
        dir.path()
            .join("jobs/j1/fixture-task__bZZeEkw/artifacts/root/answer.txt"),
        b"substituted answer\n",
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::DeliveredArtifactSubstituted {
            source: "/root/answer.txt".to_owned()
        })
    );

    // Missing declared artifact: the manifest dropped the entry, so the
    // verifier ran without a declared artifact of the task.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    std::fs::write(
        dir.path()
            .join("jobs/j1/fixture-task__bZZeEkw/artifacts/manifest.json"),
        "[]",
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::DeclaredArtifactMissing {
            source: "/root/answer.txt".to_owned()
        })
    );

    // Undeclared artifact: a file the task never declared must not enter
    // the verifier input (P19-AC-004 unrelated-canary rule).
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let manifest = dir
        .path()
        .join("jobs/j1/fixture-task__bZZeEkw/artifacts/manifest.json");
    std::fs::write(
        &manifest,
        std::fs::read_to_string(&manifest)
            .unwrap()
            .trim_end()
            .to_owned()
            + "\n",
    )
    .unwrap();
    let entries =
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&manifest).unwrap())
            .unwrap();
    let mut entries = entries.as_array().unwrap().clone();
    entries.push(serde_json::json!({
        "source": "/app/opi-eval-canary-marker", "destination": "app/opi-eval-canary-marker",
        "type": "file", "status": "ok", "service": null,
    }));
    std::fs::write(&manifest, serde_json::to_vec(&entries).unwrap()).unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::UndeclaredArtifact {
            source: "/app/opi-eval-canary-marker".to_owned()
        })
    );

    // Lifecycle mismatch: the job recorded a different declared collect
    // list than the admitted task lifecycle.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let config = dir.path().join("jobs/j1/fixture-task__bZZeEkw/config.json");
    let mut config_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    config_value["artifacts"] = serde_json::json!([]);
    std::fs::write(&config, serde_json::to_vec(&config_value).unwrap()).unwrap();
    assert!(matches!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::LifecycleMismatch { .. })
    ));

    // An explicitly selected diagnostic witness binds on the job's own
    // identities without an integration terminal observation and never
    // reports one (its result is not an Agent score).
    let dir = tempfile::tempdir().unwrap();
    let mut selection = stage_bound_job(dir.path(), "j1", "oracle");
    selection.kind = SelectionKind::DiagnosticWitness;
    selection.agent_import_path = "harbor.agents.oracle:OracleAgent".to_owned();
    let witness_config = dir.path().join("jobs/j1/fixture-task__bZZeEkw/config.json");
    let mut config_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&witness_config).unwrap()).unwrap();
    config_value["agent"] = serde_json::json!({"name": "oracle", "import_path": null});
    std::fs::write(&witness_config, serde_json::to_vec(&config_value).unwrap()).unwrap();
    std::fs::remove_file(
        dir.path()
            .join("jobs/j1/fixture-task__bZZeEkw/agent/opi-eval-terminal.json"),
    )
    .unwrap();
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(binding.agent_name, "oracle");
    assert!(binding.terminal.is_none());
    assert_eq!(binding.collected.len(), 1);

    // Missing terminal observation: the identity chain cannot bind.
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    std::fs::remove_file(
        dir.path()
            .join("jobs/j1/fixture-task__bZZeEkw/agent/opi-eval-terminal.json"),
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::TrialInvalid("terminal-observation"))
    );

    // Reward missing (errored trial, no verifier reward).
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let results = dir.path().join("jobs/j1/fixture-task__bZZeEkw/result.json");
    std::fs::write(
        &results,
        std::fs::read_to_string(&results).unwrap().replace(
            "\"verifier_result\":{\"rewards\":{\"reward\":1.0}}",
            "\"verifier_result\":null",
        ),
    )
    .unwrap();
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::RewardMissing)
    );

    // Stale job: two complete job trees; without the pre-dispatch set the
    // import refuses the ambiguity, and with it only the new one binds.
    let dir = tempfile::tempdir().unwrap();
    stage_bound_job(dir.path(), "j-old", "opi-eval-opi");
    let selection = stage_bound_job(dir.path(), "j-new", "opi-eval-opi");
    assert_eq!(
        import_bound_native_job(dir.path(), &selection),
        Err(NativeBindingError::MultipleNewJobs { count: 2 })
    );
    let mut guarded = selection;
    guarded.stale_job_names = vec!["j-old".into()];
    assert_eq!(
        import_bound_native_job(dir.path(), &guarded)
            .unwrap()
            .job_name,
        "j-new"
    );
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_runtime_preflight_rejects_substituted_and_malformed_proofs() {
    for case in [
        "legacy",
        "missing-field",
        "extra-field",
        "trial",
        "executable",
        "runtime",
        "command",
        "stdout",
        "stderr",
        "missing-capture",
        "passed-nonzero",
        "failed-zero",
        "unknown-reason",
        "overflow-without-truncation",
        "incomplete-pass",
        "refusal-task",
        "refusal-exit",
        "refusal-timeout",
        "refusal-delivered",
        "invalid-output",
    ] {
        let root = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(root.path(), "j1", "opi-eval-opi");
        let agent = root.path().join("jobs/j1/fixture-task__bZZeEkw/agent");
        let path = agent.join("opi-eval-terminal.json");
        let mut terminal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match case {
            "legacy" => terminal["schema"] = "opi-eval-native-agent-terminal/1".into(),
            "missing-field" => {
                terminal["runtime_preflight"]
                    .as_object_mut()
                    .unwrap()
                    .remove("exit_code");
            }
            "extra-field" => terminal["runtime_preflight"]["invented"] = true.into(),
            "trial" => terminal["trial"] = "other-trial".into(),
            "executable" => {
                terminal["runtime_preflight"]["agent_executable_sha256"] = "b".repeat(64).into()
            }
            "runtime" => {
                terminal["runtime_preflight"]["runtime_package_manifest_sha256"] =
                    "b".repeat(64).into();
                terminal["runtime_preflight"]["runtime_package_entrypoint"] = "bin/pi".into();
            }
            "command" => terminal["runtime_preflight"]["command_sha256"] = "b".repeat(64).into(),
            "stdout" | "stderr" => std::fs::write(
                agent.join(format!("runtime-preflight-{case}.log")),
                b"substituted\n",
            )
            .unwrap(),
            "missing-capture" => {
                std::fs::remove_file(agent.join("runtime-preflight-stdout.log")).unwrap()
            }
            "passed-nonzero" => terminal["runtime_preflight"]["exit_code"] = 1.into(),
            "failed-zero" => terminal["runtime_preflight"]["status"] = "failed".into(),
            "unknown-reason" | "overflow-without-truncation" => {
                terminal["runtime_preflight"]["status"] = "unknown".into();
                terminal["runtime_preflight"]["reason"] = if case == "unknown-reason" {
                    "unregistered"
                } else {
                    "capture-overflow"
                }
                .into();
            }
            "incomplete-pass" => terminal["runtime_preflight"]["stdout_complete"] = false.into(),
            "refusal-task" | "refusal-exit" | "refusal-timeout" | "refusal-delivered" => {
                terminal["runtime_preflight"]["status"] = "failed".into();
                terminal["runtime_preflight"]["exit_code"] = 1.into();
                terminal["task_dispatched"] = (case == "refusal-task").into();
                terminal["exit_code"] = if case == "refusal-exit" {
                    0.into()
                } else {
                    serde_json::Value::Null
                };
                terminal["timed_out"] = (case == "refusal-timeout").into();
                if case != "refusal-delivered" {
                    terminal["delivered"] = serde_json::json!([]);
                }
            }
            "invalid-output" => {
                std::fs::write(agent.join("runtime-preflight-stdout.log"), [0xff]).unwrap();
                terminal["runtime_preflight"]["stdout_sha256"] =
                    crate::runner::material::sha256_hex(&[0xff]).into();
            }
            _ => unreachable!(),
        }
        std::fs::write(&path, serde_json::to_vec(&terminal).unwrap()).unwrap();
        assert!(
            import_bound_native_observation(root.path(), &selection).is_err(),
            "{case}"
        );
        assert!(
            import_bound_native_job(root.path(), &selection).is_err(),
            "{case}"
        );
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_runtime_preflight_refusal_binds_without_reward_or_collection() {
    for status in ["failed", "unknown", "passed"] {
        let root = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(root.path(), "j1", "opi-eval-opi");
        let trial = root.path().join("jobs/j1/fixture-task__bZZeEkw");
        let terminal_path = trial.join("agent/opi-eval-terminal.json");
        let mut terminal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&terminal_path).unwrap()).unwrap();
        terminal["runtime_preflight"]["status"] = status.into();
        terminal["runtime_preflight"]["exit_code"] = match status {
            "failed" => 1.into(),
            "unknown" => serde_json::Value::Null,
            _ => 0.into(),
        };
        if status == "unknown" {
            terminal["runtime_preflight"]["reason"] = "transport".into();
            terminal["runtime_preflight"]["stdout_complete"] = false.into();
            terminal["runtime_preflight"]["stderr_complete"] = false.into();
        }
        terminal["task_dispatched"] = false.into();
        terminal["exit_code"] = serde_json::Value::Null;
        terminal["delivered"] = serde_json::json!([]);
        std::fs::write(&terminal_path, serde_json::to_vec(&terminal).unwrap()).unwrap();
        let result_path = trial.join("result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
        result["verifier_result"] = serde_json::Value::Null;
        std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
        std::fs::remove_dir_all(trial.join("artifacts")).unwrap();
        let observation = import_bound_native_observation(root.path(), &selection).unwrap();
        let terminal = observation.terminal.unwrap();
        assert_eq!(terminal.runtime_preflight.status, status);
        assert!(!terminal.task_dispatched);
        assert_eq!(terminal.exit_code, None);
        assert!(terminal.delivered.is_empty());
        assert_eq!(
            import_bound_native_job(root.path(), &selection),
            Err(NativeBindingError::TrialInvalid("task-not-observed"))
        );
    }
}

#[cfg(unix)]
#[test]
fn native_runtime_preflight_capture_rejects_links_and_special_files() {
    for case in ["symlink", "hardlink", "fifo"] {
        let root = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(root.path(), "j1", "opi-eval-opi");
        let capture = root
            .path()
            .join("jobs/j1/fixture-task__bZZeEkw/agent/runtime-preflight-stdout.log");
        std::fs::remove_file(&capture).unwrap();
        let outside = root.path().join("outside");
        std::fs::write(&outside, b"fixture version\n").unwrap();
        match case {
            "symlink" => std::os::unix::fs::symlink(&outside, &capture).unwrap(),
            "hardlink" => std::fs::hard_link(&outside, &capture).unwrap(),
            "fifo" => {
                assert!(
                    std::process::Command::new("mkfifo")
                        .arg(&capture)
                        .status()
                        .unwrap()
                        .success()
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(
            import_bound_native_observation(root.path(), &selection),
            Err(NativeBindingError::TrialInvalid(
                "runtime-preflight-capture"
            )),
            "{case}"
        );
    }
}

// Native integration proof captures require the Unix/WSL single-link boundary.
#[cfg(unix)]
#[test]
fn native_runtime_preflight_rejects_unreadable_completed_capture() {
    let mut accepted = Vec::new();
    for (status, bytes) in [
        ("failed", vec![0xff]),
        ("passed", b"fixture\0version\n".to_vec()),
    ] {
        let root = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(root.path(), "j1", "opi-eval-opi");
        let agent = root.path().join("jobs/j1/fixture-task__bZZeEkw/agent");
        let path = agent.join("opi-eval-terminal.json");
        let mut terminal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        terminal["runtime_preflight"]["status"] = status.into();
        terminal["runtime_preflight"]["exit_code"] = if status == "failed" {
            1.into()
        } else {
            0.into()
        };
        terminal["runtime_preflight"]["stdout_sha256"] =
            crate::runner::material::sha256_hex(&bytes).into();
        if status == "failed" {
            terminal["task_dispatched"] = false.into();
            terminal["exit_code"] = serde_json::Value::Null;
            terminal["delivered"] = serde_json::json!([]);
        }
        std::fs::write(agent.join("runtime-preflight-stdout.log"), bytes).unwrap();
        std::fs::write(&path, serde_json::to_vec(&terminal).unwrap()).unwrap();
        if import_bound_native_observation(root.path(), &selection).is_ok() {
            accepted.push(status);
        }
    }
    assert!(
        accepted.is_empty(),
        "completed unreadable runtime captures were admitted: {accepted:?}"
    );
}

// A substituted observed task package reference cannot be admitted as the
// selected dispatch (P19-AUD-004), even when the task name and lifecycle
// fields still match.
#[cfg(all(test, unix))]
#[test]
fn remediation_p19_observed_task_substitution_rejected() {
    for case in ["substituted-path", "missing-path", "non-string-path"] {
        let dir = tempfile::tempdir().unwrap();
        let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
        let config_path = dir.path().join("jobs/j1/fixture-task__bZZeEkw/config.json");
        let mut config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
        let original_path = config["task"]["path"].clone();
        let other = dir.path().join("substituted-task-package");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(
            other.join("task.toml"),
            "schema_version = '1.0'\n[task]\nname = 'different-task'\n",
        )
        .unwrap();
        std::fs::write(other.join("test.sh"), "#!/bin/sh\necho 1\n").unwrap();
        match case {
            "substituted-path" => config["task"]["path"] = serde_json::json!(other),
            "missing-path" => {
                config["task"].as_object_mut().unwrap().remove("path");
            }
            _ => config["task"]["path"] = serde_json::json!(42),
        }
        config["agent_timeout_multiplier"] = serde_json::json!(1.0);
        config["verifier_timeout_multiplier"] = serde_json::json!(1.0);
        std::fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        let expected = serde_json::json!({"tasks": [{"path": original_path}],
            "agents": [{}], "verifier": {}, "environment": {}});
        let imported = import_bound_native_job(dir.path(), &selection);
        let config_matches = native_stage_config_matches(&config_path, &expected);
        assert!(
            imported.is_err() || !config_matches,
            "{case}: substituted observed package was admitted"
        );
        assert!(
            !config_matches,
            "{case}: stage config comparison accepted the substituted reference"
        );
    }
}

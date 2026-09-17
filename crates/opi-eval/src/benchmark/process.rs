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
/// Limits come from the adapter's declarative profile.
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

/// What the dispatch selected: the pinned opi-eval integration Agent, or
/// an explicitly selected upstream diagnostic witness (oracle/nop). A
/// witness is conformance evidence that never becomes an Agent score and
/// carries no integration terminal observation (P19-D03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKind {
    Integration,
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
    /// Import path selected on the upstream argv (`module:ClassName`) and
    /// recorded in the trial config's agent selection.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTerminalObservation {
    /// Selected Agent product (`opi` or `pi`).
    pub(crate) product: String,
    /// sha256 of the exact Agent executable the integration executed.
    pub(crate) agent_executable_sha256: String,
    /// Observed Agent process exit code.
    pub(crate) exit_code: i64,
    /// Whether the Agent phase hit the task's Agent timeout.
    pub(crate) timed_out: bool,
    /// Files the Agent delivered, with observed digests.
    pub(crate) delivered: Vec<DeliveredFile>,
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

/// Imports and binds one dispatched native job against `selection`
/// (P19-AC-002). The authority chain: the job aggregate proves the
/// one-trial shape; the per-trial `result.json` proves the actual Agent,
/// task, and native reward of that exact trial; the trial `config.json`
/// proves the explicit Agent selection; the integration's terminal
/// observation proves the delivered artifacts; the collected-artifacts
/// manifest cross-checks the delivered bytes. Any drift is a typed
/// rejection, never a silent score.
pub(crate) fn import_bound_native_job(
    trace_root: &std::path::Path,
    selection: &NativeSelection,
) -> Result<NativeTrialBinding, NativeBindingError> {
    let (job_name, job_dir) = new_job_dir(trace_root, &selection.stale_job_names)?;

    // Job aggregate: one trial, with the selected primary reward attributed
    // to that trial. Pier also records auxiliary metrics such as test counts.
    let aggregate_path = job_dir.join("result.json");
    let aggregate: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&aggregate_path).map_err(|_| NativeBindingError::AggregateUnreadable)?,
    )
    .map_err(|_| NativeBindingError::AggregateInvalid("json-parse"))?;
    if aggregate.get("n_total_trials").and_then(|n| n.as_u64()) != Some(1) {
        return Err(NativeBindingError::AggregateInvalid("trial-count"));
    }
    let evals = aggregate
        .get("stats")
        .and_then(|s| s.get("evals"))
        .and_then(|e| e.as_object())
        .ok_or(NativeBindingError::AggregateInvalid("no-eval-stats"))?;
    if evals.len() != 1 {
        return Err(NativeBindingError::AggregateInvalid("eval-count"));
    }
    let eval_key = evals.keys().next().expect("len checked").to_owned();
    let reward_stats = evals
        .values()
        .next()
        .expect("len checked")
        .get("reward_stats")
        .and_then(|r| r.as_object())
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
        &std::fs::read(&results_path).map_err(|_| NativeBindingError::TrialResultMissing)?,
    )
    .map_err(|_| NativeBindingError::TrialInvalid("json-parse"))?;
    let trial_name = results
        .get("trial_name")
        .and_then(|n| n.as_str())
        .ok_or(NativeBindingError::TrialInvalid("trial-name"))?
        .to_owned();
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
    // The eval key embeds the actual Agent name (`agent__model__dataset`
    // or `agent__dataset` in the pinned runner): a job grouped under any
    // other Agent - oracle included - is not this trial's score.
    if eval_key.split("__").next().unwrap_or_default() != selection.agent_name {
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
    let exception_type = results
        .get("exception_info")
        .and_then(|e| e.get("exception_type"))
        .and_then(|t| t.as_str())
        .map(str::to_owned);

    // The trial config's explicit Agent selection: the import path this
    // dispatch selected, never an implicit default or oracle.
    let config_path = trial_dir.join("config.json");
    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&config_path).map_err(|_| NativeBindingError::TrialResultMissing)?,
    )
    .map_err(|_| NativeBindingError::TrialInvalid("json-parse"))?;
    let agent_config = config.get("agent").and_then(|a| a.as_object());
    let selected = agent_config.and_then(|a| {
        a.get("import_path")
            .and_then(|p| p.as_str())
            .or_else(|| a.get("name").and_then(|n| n.as_str()))
    });
    match selected {
        None => return Err(NativeBindingError::AgentSelectionMissing),
        Some(observed) if observed == selection.agent_import_path => {}
        Some(observed) => {
            return Err(NativeBindingError::AgentSelectionMismatch {
                expected: selection.agent_import_path.clone(),
                observed: observed.to_owned(),
            });
        }
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

    // The integration's typed terminal observation: required evidence for
    // an integration dispatch; a diagnostic witness (upstream oracle or
    // nop agent) produces none, so its binding rests on the job's own
    // identities and collected artifacts and never becomes an Agent
    // score.
    let terminal = if selection.kind == SelectionKind::Integration {
        let terminal_path = trial_dir.join("agent/opi-eval-terminal.json");
        Some(parse_terminal_observation(
            &std::fs::read(&terminal_path)
                .map_err(|_| NativeBindingError::TrialInvalid("terminal-observation"))?,
        )?)
    } else {
        None
    };

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

    let agent_version = agent_info
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned();
    Ok(NativeTrialBinding {
        job_name,
        trial_name,
        agent_name: agent_name.to_owned(),
        agent_version,
        model_name,
        task_name: task_name.to_owned(),
        reward: Fact::Known {
            value: reward_value as u64,
            origin: "native-trial-result".to_owned(),
        },
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
/// Parses the integration's typed terminal observation.
fn parse_terminal_observation(
    bytes: &[u8],
) -> Result<NativeTerminalObservation, NativeBindingError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| NativeBindingError::TrialInvalid("terminal-observation"))?;
    if value.get("schema").and_then(|s| s.as_str()) != Some("opi-eval-native-agent-terminal/1") {
        return Err(NativeBindingError::TrialInvalid("terminal-observation"));
    }
    let product = value
        .get("product")
        .and_then(|p| p.as_str())
        .ok_or(NativeBindingError::TrialInvalid("terminal-observation"))?
        .to_owned();
    let agent_executable_sha256 = value
        .get("agent_executable_sha256")
        .and_then(|p| p.as_str())
        .ok_or(NativeBindingError::TrialInvalid("terminal-observation"))?
        .to_owned();
    let exit_code = value
        .get("exit_code")
        .and_then(|c| c.as_i64())
        .ok_or(NativeBindingError::TrialInvalid("terminal-observation"))?;
    let timed_out = value
        .get("timed_out")
        .and_then(|t| t.as_bool())
        .unwrap_or(false);
    let mut delivered = Vec::new();
    for entry in value
        .get("delivered")
        .and_then(|d| d.as_array())
        .unwrap_or(&Vec::new())
    {
        let source = entry
            .get("source")
            .and_then(|s| s.as_str())
            .ok_or(NativeBindingError::TrialInvalid("terminal-observation"))?;
        let sha256 = entry
            .get("sha256")
            .and_then(|s| s.as_str())
            .ok_or(NativeBindingError::TrialInvalid("terminal-observation"))?;
        delivered.push(DeliveredFile {
            source: source.to_owned(),
            sha256: sha256.to_owned(),
        });
    }
    Ok(NativeTerminalObservation {
        product,
        agent_executable_sha256,
        exit_code,
        timed_out,
        delivered,
    })
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
        let identity = adapter.identity();
        let spec = adapter.spawn_spec(request);
        let started = Instant::now();
        let outcome = ProcessSupervisor::run(&spec, cancel).await;
        let wall_time = started.elapsed();
        let (reward, mut completion) = adapter.settle(&outcome, request);
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
            trace_root: std::env::temp_dir(),
            admitted_lock_digest: "a".repeat(64),
            integrity,
            extra_env: BTreeMap::new(),
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
            "model_info": {"name": "scripted/eval"}},
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
    // The integration's typed terminal observation.
    let terminal = serde_json::json!({
        "schema": "opi-eval-native-agent-terminal/1",
        "product": "opi",
        "agent_executable_sha256": "a".repeat(64),
        "exit_code": 0,
        "timed_out": false,
        "delivered": [{"source": "/root/answer.txt", "sha256": answer_digest}],
    });
    std::fs::write(
        trial_dir.join("agent/opi-eval-terminal.json"),
        serde_json::to_vec(&terminal).unwrap(),
    )
    .unwrap();
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
        kind: SelectionKind::Integration,
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
        model: "scripted/eval".to_owned(),
        reward_metric: "reward".to_owned(),
        module_dir: "/nonexistent/integrations".into(),
        handoff_path: "/nonexistent/handoff.json".into(),
        stale_job_names: Vec::new(),
    }
}

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
    std::fs::write(
        trial_dir.join("agent/opi-eval-terminal.json"),
        r#"{"schema": "opi-eval-native-agent-terminal/1", "product": "pi",
            "agent_executable_sha256": "443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403",
            "exit_code": 0, "timed_out": false, "delivered": []}"#,
    )
    .unwrap();
    let selection = NativeSelection {
        kind: SelectionKind::Integration,
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
    assert_eq!(terminal.exit_code, 0);
    assert_eq!(
        terminal.agent_executable_sha256,
        "443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403"
    );
    assert!(binding.exception_type.is_none());
}

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

#[cfg(test)]
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
    assert_eq!(binding.terminal.as_ref().unwrap().exit_code, 0);
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
    assert_eq!(binding.model_name.as_deref(), Some("scripted/eval"));
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
    config_value["agent"] = serde_json::json!({"import_path": "harbor.agents.oracle:OracleAgent"});
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

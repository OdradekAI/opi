//! Crate-private assembled evaluation runner (Phase 18 task 18.12).
//!
//! [`run_experiment`] is the end-to-end owner of one resolved experiment:
//! durable intent reservation, Agent dispatch, settlement, native verifier
//! dispatch, pre-seal trajectory projection, bundle sealing, receipt
//! emission, and comparison/coverage assembly - every later authority
//! transition mechanically gated by [`crate::authority::AuthorityLedger`]
//! (`P18-FAL-002`). Two driving modes share this flow. The hermetic
//! fixture-grade path resolves runtime-generated deterministic helpers and
//! pinned fixtures-tree native bytes. The native driving mode (task
//! 18.14.1) consumes the resolved-material manifest: exact built agent
//! executables, the materialized official task package, the pinned uv
//! verifier entrypoint, one upstream oracle preflight per task, and the
//! scripted-provider listener endpoint - never synthesized helpers and
//! never fixture native bytes. Executables are never resolved from ambient
//! PATH.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::json;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::agent::opi::OpiProcessAdapter;
use crate::agent::pi::PiProcessAdapter;
use crate::agent::process::{
    AgentAdapter, AgentCompletion, AgentExecution, AgentRunRequest, Fact, IsolationDirs,
};
use crate::authority::{AuthorityLedger, AuthorityTransition, boundary_token};
use crate::benchmark::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkExecution, BenchmarkRunRequest,
};
use crate::benchmark::terminal_bench_21::{Tb21Profile, TerminalBench21Adapter};
use crate::bundle::{
    ArtifactKey, ArtifactRole, ArtifactSpec, IntentRecord, PairIdentity, RunBundle, Sensitivity,
    SettlementMarker, SourceIdentity, TrialIdentity,
};
use crate::comparison::{ComparisonSet, TrialFact, TrialOutcome};
use crate::experiment::ResolvedExperiment;
use crate::failure::FailureBoundaryCode;
use crate::integrity::{
    IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
};
use crate::process::ExitState;
use crate::runner::lifecycle::{CancellationSource, ObservedOutcome, TrialLifecycle};
use crate::runner::material::{NativeMaterial, task_package_manifest_digest};
use crate::trajectory::{ProjectionPipeline, TrialInputs};

/// Run report schema identity.
const RUN_REPORT_SCHEMA: &str = "phase18-run-report/1";
/// Per-trial receipt schema identity.
const TRIAL_RECEIPT_SCHEMA: &str = "phase18-trial-receipt/1";
/// Bundle-internal key reserved for the normalized expected output.
const EXPECTED_OUTPUT_KEY: &str = "normalized/expected-output";

/// One assembled run request. `behavior` is a hermetic staging knob that
/// selects helper-process behavior (never a production claim).
pub(crate) struct RunRequest {
    pub(crate) config_path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) fixtures: PathBuf,
    pub(crate) behavior: String,
    /// Recovery mode: classify the durable state of every existing trial
    /// root instead of running anything.
    pub(crate) recover: bool,
    /// Re-run one crashed trial's whole group under fresh trial identities
    /// and a new paired trial group (P18-DUR-002, P18-EXP-005).
    pub(crate) replacement_for: Option<String>,
    /// Declared canary secrets (one per line from the request). Any
    /// canary found in staged exportable content blocks sealing
    /// (`P18-A18`, `P18-SEC-005`).
    pub(crate) canaries: Vec<String>,
    /// Resolved native material (task 18.14.1): when present the runner
    /// takes the native driving mode over the hermetic fixture path.
    pub(crate) material: Option<crate::runner::material::NativeMaterial>,
    /// Run only the upstream oracle preflight, then stop before any agent
    /// trial (native mode).
    pub(crate) preflight_only: bool,
}

/// The pinned hermetic canary the `canary-leak` fixture behavior emits
/// into agent output. It is a fixture fact, not a real secret; sealing
/// blocks only when it is declared to the run.
const HERMETIC_CANARY: &str = "OPZ-EVAL-CANARY-7f3a9c";

/// Typed run-path failures that abort before any report is rendered.
#[derive(Debug, Error)]
pub(crate) enum RunError {
    /// The experiment document could not be read.
    #[error("cannot read experiment document: {0}")]
    Io(#[from] std::io::Error),
    /// The document did not resolve into a frozen contract.
    #[error("experiment document rejected: {0}")]
    Resolve(#[from] crate::experiment::ResolveError),
    /// The pinned integrity digest does not address the fixture review the
    /// runner derives from this document.
    #[error("integrity digest mismatch: experiment pins {expected}, derived review is {derived}")]
    IntegrityMismatch { expected: String, derived: String },
    /// The experiment declares a benchmark this runner has no concrete
    /// adapter for.
    #[error("unsupported benchmark {benchmark} revision {revision}")]
    UnsupportedBenchmark { benchmark: String, revision: String },
    /// A subject product with no concrete adapter.
    #[error("unsupported subject product {product}")]
    UnsupportedSubject { product: String },
    /// Hermetic staging failed (helper generation, package copy).
    #[error("hermetic staging failed: {0}")]
    Staging(String),
    /// The resolved native material was rejected (drift, missing
    /// identity, or a failed upstream oracle preflight).
    #[error("native material rejected: {0}")]
    Native(String),
    /// A durable bundle operation failed outside the per-trial failure
    /// paths.
    #[error("bundle operation failed: {0}")]
    Bundle(String),
}

/// Settlement status of one assembled trial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrialStatus {
    Sealed,
    Failed,
}

/// Everything one settled trial contributes to the report.
struct TrialResult {
    receipt: serde_json::Value,
    fact: Option<TrialFact>,
    sealed: bool,
}

/// The deterministic fixture-grade integrity review for one experiment.
///
/// Derived only from the frozen document and the staging behavior so its
/// content-addressed identity is stable; experiment documents pin that
/// digest in `benchmark.integrity_digest`.
fn fixture_integrity_review(experiment: &ResolvedExperiment, behavior: &str) -> IntegrityReview {
    let first_task = experiment
        .trials()
        .first()
        .map(|trial| trial.task.clone())
        .unwrap_or_default();
    let tasks: BTreeMap<String, TaskClassification> = experiment
        .trials()
        .iter()
        .map(|trial| trial.task.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|task| {
            let classification = if behavior == "invalid-task" && task == first_task {
                TaskClassification::PromptTestMismatch {
                    reason: "fixture: prompt asks ls; test greps find".to_owned(),
                }
            } else {
                TaskClassification::ValidAgentOutcome
            };
            (task, classification)
        })
        .collect();
    let mut excluded_trials = BTreeMap::new();
    if behavior == "integrity-exclusion"
        && let Some(first) = experiment.trials().first()
    {
        excluded_trials.insert(
            first.id.clone(),
            "fixture exclusion: workspace vanished under sandbox".to_owned(),
        );
    }
    IntegrityReview {
        benchmark: experiment.benchmark().name.clone(),
        revision: experiment.benchmark().revision.clone(),
        dataset: experiment.benchmark().dataset.clone(),
        grader: "harbor-v0.22.0-fixture".to_owned(),
        environment: format!(
            "{}-{}",
            experiment.environment().platform,
            experiment.environment().architecture
        ),
        upstream_identity: "fixture://terminal-bench-2.1".to_owned(),
        upstream_digest: "1111111111111111111111111111111111111111".to_owned(),
        oracle: Some(OraclePreflight::Passed(
            "fixture oracle preflight".to_owned(),
        )),
        status: RevisionStatus::Admitted,
        tasks,
        excluded_trials,
        reviewer: "phase18-fixture".to_owned(),
    }
}

/// Canonical control fingerprint of the frozen shared controls.
fn control_fingerprint(experiment: &ResolvedExperiment) -> String {
    let canonical = serde_json::to_string(&experiment.model_controls())
        .unwrap_or_else(|_| "uncanonicalizable".to_owned());
    crate::agent::opi::sha256_hex(canonical.as_bytes())
}

/// The deterministic helper standing in for one agent product. Behavior is
/// selected via `OPI_EVAL_RUN_BEHAVIOR`; every branch is bounded and emits
/// only pinned fixture bytes into the isolated workspace/trace roots.
fn agent_helper_script(product: &str) -> String {
    let trace_copy = if product == "opi" {
        // Opi receives the fresh trace root as the `--trace` argv slot ($4).
        r#"  cp -r "$OPI_EVAL_TRACE_SOURCE/run-0001" "$4/""#
    } else {
        // pi emits its native stream on stdout.
        r#"  cat "$PI_EVAL_STREAM_SOURCE""#
    };
    let argv_guard = if product == "opi" {
        r#"[ "$1" = "--json" ] || { echo "argv drift: expected --json first" >&2; exit 9; }"#
    } else {
        r#"[ "$1" = "--mode" ] && [ "$2" = "json" ] || { echo "argv drift: expected --mode json" >&2; exit 9; }"#
    };
    // Excess-output arm: opi's importer reads trace files, so capped junk
    // on stderr proves the cap without failing settlement; pi's settlement
    // reads stdout, so the junk itself is the capped native stream.
    let excess_output = if product == "opi" {
        format!("{trace_copy}\n  head -c 2097152 /dev/zero | tr '\\0' 'a' >&2")
    } else {
        "  head -c 2097152 /dev/zero | tr '\\0' 'a'".to_owned()
    };
    format!(
        "#!/bin/sh\n\
# assembled-run fixture helper standing in for the {product} agent (never the real binary)\n\
{argv_guard}\n\
echo answer > answer.txt\n\
case \"$OPI_EVAL_RUN_BEHAVIOR\" in\n\
  happy|prompt-only-package|verifier-failure|seal-failure|agent-unknown-schema|agent-malformed-stream|agent-missing-terminal)\n\
{trace_copy} ;;\n\
  canary-leak)\n\
  printf '%s\n' \"$OPI_EVAL_CANARY_SECRET\" > answer.txt\n\
{trace_copy} ;;\n\
  agent-excess-output)\n\
{excess_output} ;;\n\
  agent-timeout|agent-cancelled)\n\
  sleep 30 ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\nesac\n\
exit 0\n"
    )
}

/// The deterministic helper standing in for the native verifier.
fn verifier_helper_script(report_name: &str) -> String {
    format!(
        "#!/bin/sh\n\
# assembled-run fixture helper standing in for the native verifier (never the real grader)\n\
case \"$OPI_EVAL_RUN_BEHAVIOR\" in\n\
  happy|canary-leak)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" ./{report_name} ;;\n\
  verifier-failure)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" ./{report_name}\n\
    exit 5 ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\nesac\n\
exit 0\n"
    )
}

/// Scans the staged bundle files for declared canaries. Reads exactly the
/// bytes sealing would cover; returns the first leaking logical key.
fn scan_staged_for_canaries(
    bundle_root: &Path,
    staged_keys: &[ArtifactKey],
    canaries: &[String],
) -> Option<ArtifactKey> {
    for canary in canaries {
        for key in staged_keys {
            let Ok(bytes) = std::fs::read(bundle_root.join("artifacts").join(key.as_str())) else {
                continue;
            };
            if !bytes.is_empty()
                && !canary.is_empty()
                && bytes
                    .windows(canary.len())
                    .any(|window| window == canary.as_bytes())
            {
                return Some(key.clone());
            }
        }
    }
    None
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .unwrap_or_else(|error| panic!("helper metadata: {error}"))
            .permissions();
        permissions.set_mode(0o755);
        let _ = std::fs::set_permissions(path, permissions);
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn copy_dir_recursive(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            copy_dir_recursive(&path, &target.join(entry.file_name()))?;
        } else {
            std::fs::copy(&path, target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn write_helper(path: &Path, script: &str) -> Result<(), RunError> {
    std::fs::write(path, script).map_err(|error| RunError::Staging(error.to_string()))?;
    make_executable(path);
    Ok(())
}

/// Run one assembled experiment and return its run report.
pub(crate) async fn run_experiment(request: &RunRequest) -> Result<serde_json::Value, RunError> {
    let mut source = std::fs::read_to_string(&request.config_path)?;
    if let Some(crashed) = &request.replacement_for {
        // The replacement is a new contract: fresh trial identities for
        // every trial of the crashed group, re-resolved and digest-addressed
        // separately. The original document is never edited on disk.
        source = replace_group(source.as_str(), crashed)?;
    }
    let experiment = ResolvedExperiment::resolve(&source)?;
    if request.recover {
        let report = recovery_report(&experiment, &request.root);
        persist_run_report(&request.root, &report)?;
        return Ok(report);
    }
    if request.material.is_some() && request.behavior != "happy" {
        return Err(RunError::Native(
            "the native driving mode admits only the default staging behavior".to_owned(),
        ));
    }
    let native = match &request.material {
        Some(material) => Some(prepare_native(material, &experiment)?),
        None => None,
    };
    let integrity = match &native {
        Some(inputs) => IntegrityRecord::review(native_integrity_review(
            &experiment,
            request.material.as_ref().expect("checked above"),
            inputs,
        ))
        .map_err(|error| RunError::Native(format!("native integrity rejected: {error}")))?,
        None => IntegrityRecord::review(fixture_integrity_review(&experiment, &request.behavior))
            .map_err(|error| {
            RunError::Staging(format!("fixture integrity rejected: {error}"))
        })?,
    };
    match experiment.benchmark().integrity_digest.as_deref() {
        Some(pinned) if pinned == integrity.identity_digest() => {}
        other => {
            return Err(RunError::IntegrityMismatch {
                expected: other.unwrap_or("<absent>").to_owned(),
                derived: integrity.identity_digest().to_owned(),
            });
        }
    }
    if let Some(inputs) = &native {
        // One upstream oracle preflight per selected task through the
        // unchanged native verifier: the reference solution must pass
        // natively before any agent trial starts (task 18.14.1).
        let preflight = run_oracle_preflight(
            &experiment,
            request.material.as_ref().expect("checked above"),
            inputs,
            &integrity,
            request,
        )
        .await?;
        if request.preflight_only {
            let report = json!({
                "schema": RUN_REPORT_SCHEMA,
                "experiment": experiment.experiment_id(),
                "manifest_digest": experiment.manifest_digest(),
                "integrity_digest": integrity.identity_digest(),
                "outcome": "preflight-only",
                "preflight": preflight,
            });
            persist_run_report(&request.root, &report)?;
            return Ok(report);
        }
    }
    let benchmark_revision = match &native {
        Some(inputs) => BenchmarkRevision::from_native(inputs),
        None => BenchmarkRevision::from_experiment(&experiment, &request.fixtures)?,
    };

    let mut trials_out: Vec<serde_json::Value> = Vec::new();
    let mut facts: Vec<TrialFact> = Vec::new();
    let mut all_sealed = true;
    for declared in experiment.trials() {
        let subject = experiment
            .subjects()
            .iter()
            .find(|subject| subject.id == declared.subject)
            .expect("resolved experiments reference known subjects");
        let result = run_trial(
            &experiment,
            &integrity,
            &benchmark_revision,
            declared,
            &subject.product,
            native.as_ref(),
            request,
        )
        .await?;
        if let Some(fact) = result.fact {
            facts.push(fact);
        }
        all_sealed &= result.sealed;
        trials_out.push(result.receipt);
    }

    // Coverage assembly: per-pair states stay visible; a structural
    // assembly failure is a typed non-success outcome, never a hidden pair.
    let (pairs, outcome, comparison_error) =
        match ComparisonSet::assemble(&experiment, &integrity, &facts) {
            Ok(set) => {
                let pairs: Vec<serde_json::Value> = set
                    .pairs()
                    .iter()
                    .map(|pair| {
                        json!({
                            "edge": pair.edge(),
                            "task": pair.task(),
                            "group": pair.group(),
                            "baseline_trial": pair.baseline_trial(),
                            "candidate_trial": pair.candidate_trial(),
                            "comparability": comparability_token(pair.comparability()),
                        })
                    })
                    .collect();
                let all_comparable = set
                    .pairs()
                    .iter()
                    .all(|pair| pair.comparability().is_comparable());
                let outcome = if all_comparable && all_sealed {
                    "completed"
                } else {
                    "incomplete"
                };
                (pairs, outcome, None)
            }
            Err(error) => (Vec::new(), "incomplete", Some(error.to_string())),
        };

    let mut report = json!({
        "schema": RUN_REPORT_SCHEMA,
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "integrity_digest": integrity.identity_digest(),
        "outcome": outcome,
        "trials": trials_out,
        "pairs": pairs,
    });
    if let Some(reason) = comparison_error {
        report["comparison_error"] = json!(reason);
    }
    // The offline report path (task 18.13) recomputes from sealed
    // assembled outputs only: persist the run report at the run root so
    // regrade/report never re-run anything to rebuild the denominator.
    persist_run_report(&request.root, &report)?;
    Ok(report)
}

/// Persists the run report under the run root as one canonical JSON file.
/// Writing it is part of the run itself; offline commands only read it.
fn persist_run_report(root: &Path, report: &serde_json::Value) -> Result<(), RunError> {
    let bytes = serde_json::to_vec(report).map_err(|error| RunError::Staging(error.to_string()))?;
    std::fs::create_dir_all(root).map_err(|error| RunError::Staging(error.to_string()))?;
    std::fs::write(root.join("run-report.json"), &bytes)
        .map_err(|error| RunError::Staging(error.to_string()))
}

/// The concrete benchmark revision an experiment declares, with the staging
/// inputs its adapter needs.
enum BenchmarkRevision {
    TerminalBench21 {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        /// Hermetic-only saved fixture bytes the helper replays; `None`
        /// in native mode, where the real verifier writes the report.
        fixture_report: Option<PathBuf>,
    },
    TerminalBench30 {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        fixture_report: Option<PathBuf>,
    },
    DeepSwe {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        fixture_report: Option<PathBuf>,
    },
}

/// The native report file name a revision's verifier writes into its trace
/// root.
impl BenchmarkRevision {
    fn report_name(&self) -> &'static str {
        match self {
            BenchmarkRevision::TerminalBench21 { .. }
            | BenchmarkRevision::TerminalBench30 { .. } => "ctrf-report.json",
            BenchmarkRevision::DeepSwe { .. } => "pier-report.json",
        }
    }
}

impl BenchmarkRevision {
    fn from_experiment(experiment: &ResolvedExperiment, fixtures: &Path) -> Result<Self, RunError> {
        let benchmark = experiment.benchmark();
        let root = fixtures.join("benchmarks");
        if benchmark.name == "terminal-bench" && benchmark.revision == "2.1" {
            // The synthetic fixture profile pins the synthetic task package
            // bytes; the fixture CTRF report is the verifier's native
            // output source. All paths resolve under the caller-supplied
            // fixtures root.
            let tb21 = root.join("terminal-bench-2.1");
            return Ok(BenchmarkRevision::TerminalBench21 {
                profile_bytes: std::fs::read(tb21.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: tb21.join("task-package"),
                fixture_report: Some(tb21.join("ctrf/ok-six-passed.json")),
            });
        }
        if benchmark.name == "terminal-bench" && benchmark.revision == "3.0" {
            let tb30 = root.join("terminal-bench-3.0");
            return Ok(BenchmarkRevision::TerminalBench30 {
                profile_bytes: std::fs::read(tb30.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: tb30.join("task-package"),
                fixture_report: Some(tb30.join("ctrf/ok-six-passed.json")),
            });
        }
        if benchmark.name == "deepswe" && benchmark.revision == "v1.1" {
            let deepswe = root.join("deepswe-v1.1");
            return Ok(BenchmarkRevision::DeepSwe {
                profile_bytes: std::fs::read(deepswe.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: deepswe.join("task-package"),
                fixture_report: Some(deepswe.join("pier-report/resolved.json")),
            });
        }
        Err(RunError::UnsupportedBenchmark {
            benchmark: benchmark.name.clone(),
            revision: benchmark.revision.clone(),
        })
    }

    /// The native construction: the materialized official task package,
    /// the production profile bytes, and no fixture report - the real
    /// verifier writes the native report into its trace root.
    fn from_native(inputs: &NativeInputs) -> Self {
        match inputs.adapter_key.as_str() {
            "terminal-bench-2.1" => BenchmarkRevision::TerminalBench21 {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
            },
            "terminal-bench-3.0" => BenchmarkRevision::TerminalBench30 {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
            },
            _ => BenchmarkRevision::DeepSwe {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
            },
        }
    }
}

/// The CLI adapter id for an experiment's declared benchmark revision.
fn adapter_key_for(experiment: &ResolvedExperiment) -> Result<String, RunError> {
    let benchmark = experiment.benchmark();
    match (benchmark.name.as_str(), benchmark.revision.as_str()) {
        ("terminal-bench", "2.1") => Ok("terminal-bench-2.1".to_owned()),
        ("terminal-bench", "3.0") => Ok("terminal-bench-3.0".to_owned()),
        ("deepswe", "v1.1") => Ok("deepswe-v1.1".to_owned()),
        _ => Err(RunError::UnsupportedBenchmark {
            benchmark: benchmark.name.clone(),
            revision: benchmark.revision.clone(),
        }),
    }
}

/// Everything the native driving mode resolves from the manifest before
/// any process runs: the concrete benchmark revision inputs, the official
/// prompt, and the fail-closed package verification.
struct NativeInputs {
    adapter_key: String,
    profile_bytes: Vec<u8>,
    task_package: PathBuf,
    task_id: String,
    verifier_executable: PathBuf,
    verifier_env: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    oracle_executable: PathBuf,
    oracle_env: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    prompt: String,
}

/// Resolves and verifies the native material for one experiment.
fn prepare_native(
    material: &NativeMaterial,
    experiment: &ResolvedExperiment,
) -> Result<NativeInputs, RunError> {
    let adapter_key = adapter_key_for(experiment)?;
    let benchmark = material
        .benchmark(&adapter_key)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let profile_bytes = std::fs::read(&benchmark.profile)
        .map_err(|error| RunError::Native(format!("profile unreadable: {error}")))?;
    let task_id = match adapter_key.as_str() {
        "terminal-bench-2.1" => {
            Tb21Profile::parse(profile_bytes.as_slice())
                .map_err(|error| RunError::Native(error.to_string()))?
                .task_id
        }
        "terminal-bench-3.0" => {
            crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                .map_err(|error| RunError::Native(error.to_string()))?
                .task_id
        }
        _ => {
            crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                .map_err(|error| RunError::Native(error.to_string()))?
                .task_id
        }
    };
    // Fail-closed package verification: the runner recomputes the sorted
    // package manifest and compares it with the pinned digest.
    let observed = task_package_manifest_digest(&benchmark.task_package)
        .map_err(|error| RunError::Native(error.to_string()))?;
    if observed != benchmark.task_package_manifest_sha256 {
        return Err(RunError::Native(format!(
            "task package manifest drift for {adapter_key}: pinned {}, observed {}",
            benchmark.task_package_manifest_sha256, observed
        )));
    }
    for declared in experiment.trials() {
        if declared.task != task_id {
            return Err(RunError::Native(format!(
                "trial {} declares task {:?}, material resolves {:?}",
                declared.id, declared.task, task_id
            )));
        }
    }
    let instruction = std::fs::read_to_string(benchmark.task_package.join("instruction.md"))
        .map_err(|error| RunError::Native(format!("instruction unreadable: {error}")))?;
    let prompt = format!(
        "{instruction}\n\nWhen you are finished, write your final answer to a file named          answer.txt in the working directory."
    );
    let to_env = |map: &std::collections::BTreeMap<String, String>| {
        map.iter()
            .map(|(key, value)| (key.clone().into(), value.clone().into()))
            .collect::<std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>>()
    };
    Ok(NativeInputs {
        adapter_key,
        profile_bytes,
        task_package: benchmark.task_package.clone(),
        task_id,
        verifier_executable: benchmark.verifier_executable.path.clone(),
        verifier_env: to_env(&benchmark.verifier_env),
        oracle_executable: benchmark.oracle.path.clone(),
        oracle_env: to_env(&benchmark.oracle_env),
        prompt,
    })
}

/// The native integrity review: every identity derives from the verified
/// material, and the oracle entry records the preflight this run performs
/// before any agent trial.
fn native_integrity_review(
    experiment: &ResolvedExperiment,
    material: &NativeMaterial,
    inputs: &NativeInputs,
) -> IntegrityReview {
    let benchmark = experiment.benchmark();
    let (grader, environment) = match inputs.adapter_key.as_str() {
        "terminal-bench-2.1" => ("harbor-v0.22.0", "separate-container"),
        "terminal-bench-3.0" => ("harbor-v0.22.0", "separate-verifier-container"),
        _ => ("pier-v0.3.1", "separate-pristine-verifier"),
    };
    IntegrityReview {
        benchmark: benchmark.name.clone(),
        revision: benchmark.revision.clone(),
        dataset: benchmark.dataset.clone(),
        grader: grader.to_owned(),
        environment: environment.to_owned(),
        upstream_identity: format!("{}@{}", inputs.adapter_key, material.static_lock.sha256),
        upstream_digest: material
            .benchmark(&inputs.adapter_key)
            .map(|entry| entry.task_package_manifest_sha256.clone())
            .unwrap_or_default(),
        oracle: Some(crate::integrity::OraclePreflight::Passed(
            "native oracle preflight".to_owned(),
        )),
        status: crate::integrity::RevisionStatus::Admitted,
        tasks: std::collections::BTreeMap::from([(
            inputs.task_id.clone(),
            crate::integrity::TaskClassification::ValidAgentOutcome,
        )]),
        excluded_trials: std::collections::BTreeMap::new(),
        reviewer: "phase18-native-material".to_owned(),
    }
}

/// The deterministic Opi configuration for one native trial: one mapped
/// custom provider whose base URL is the pre-resolved scripted-provider
/// listener and whose credential source is the declared dummy environment
/// projection. Written verbatim into the isolated `--config` path.
pub(crate) fn native_opi_config(
    config: &crate::runner::material::AgentConfigMaterial,
    model: &str,
) -> String {
    let provider_id = model.split(':').next().unwrap_or("scripted");
    format!(
        "# Phase 18 native run: deterministic scripted-provider projection.\n\
[providers.custom.{provider_id}]\n\
name = \"Phase 18 scripted provider\"\n\
base_url = \"{base_url}\"\n\
api_key_env = \"OPENAI_API_KEY\"\n\
auth_scheme = \"bearer\"\n\
api = \"openai-completions\"\n\
\n\
[[providers.custom.{provider_id}.models]]\n\
id = \"{model_id}\"\n\
display_name = \"Phase 18 scripted\"\n\
context_window = 8192\n\
max_output_tokens = 4096\n",
        base_url = config.base_url,
        model_id = config.model_id,
    )
}

/// Materializes the per-product deterministic configuration projection
/// into the isolated agent directories and returns the environment
/// additions beyond the profile isolation (the caller merges the
/// manifest's closed credential projection).
pub(crate) fn native_agent_env(
    product: &str,
    config: &crate::runner::material::AgentConfigMaterial,
    isolation: &IsolationDirs,
) -> Result<BTreeMap<std::ffi::OsString, std::ffi::OsString>, String> {
    if product == "pi" {
        // pi selects its model through the isolated agent-dir models.json
        // (the pinned custom-model contract): same endpoint, same model
        // identity, declared dummy credential, no ambient fallback.
        let agent_dir = isolation.app_data.join("pi-agent");
        std::fs::create_dir_all(&agent_dir).map_err(|error| error.to_string())?;
        // The pinned pi models.json schema (docs/models.md at the pinned
        // commit): one provider entry per provider id with baseUrl, api,
        // apiKey, and a models array; the provider id comes from the
        // shared provider:model selection, the model id from the config.
        let provider_id = config
            .model_id
            .split('/')
            .next()
            .unwrap_or("scripted")
            .to_owned();
        let models = json!({
            "providers": {
                provider_id.clone(): {
                    "baseUrl": config.base_url,
                    "api": "openai-completions",
                    "apiKey": config.api_key,
                    "models": [
                        { "id": config.model_id.clone() }
                    ],
                },
            },
        });
        let bytes = serde_json::to_vec(&models).map_err(|error| error.to_string())?;
        std::fs::write(agent_dir.join("models.json"), &bytes).map_err(|error| error.to_string())?;
    }
    Ok(BTreeMap::new())
}

/// Derives the native integrity identity for one experiment document plus
/// material manifest without running anything (the producer's config
/// materialization tool, task 18.14.1).
pub(crate) fn native_integrity_identity(
    config_path: &Path,
    material_path: &Path,
) -> Result<String, String> {
    let source = std::fs::read_to_string(config_path)
        .map_err(|error| format!("cannot read experiment document: {error}"))?;
    let experiment = ResolvedExperiment::resolve(&source)
        .map_err(|error| format!("experiment document rejected: {error}"))?;
    let material = NativeMaterial::load(material_path).map_err(|error| error.to_string())?;
    let inputs = prepare_native(&material, &experiment).map_err(|error| error.to_string())?;
    let review = IntegrityRecord::review(native_integrity_review(&experiment, &material, &inputs))
        .map_err(|error| format!("native integrity rejected: {error}"))?;
    Ok(review.identity_digest().to_owned())
}

/// Runs the upstream oracle preflight through the unchanged native
/// verifier wrapper and requires a passing native reward before trials.
async fn run_oracle_preflight(
    experiment: &ResolvedExperiment,
    material: &NativeMaterial,
    inputs: &NativeInputs,
    integrity: &IntegrityRecord,
    request: &RunRequest,
) -> Result<serde_json::Value, RunError> {
    let root = request.root.join("preflight").join(&inputs.adapter_key);
    let trace_root = root.join("trace");
    let task_dir = root.join("task-package");
    std::fs::create_dir_all(&trace_root).map_err(|error| RunError::Native(error.to_string()))?;
    copy_dir_recursive(&inputs.task_package, &task_dir)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let revision = BenchmarkRevision::from_native(inputs);
    let adapter = benchmark_adapter_for(&revision)?;
    let benchmark_request = BenchmarkRunRequest {
        verifier_executable: inputs.oracle_executable.clone(),
        task_dir,
        task_id: inputs.task_id.clone(),
        agent_output: root.join("oracle-reference-output"),
        trace_root: trace_root.clone(),
        admitted_lock_digest: material.static_lock.sha256.clone(),
        integrity: integrity.clone(),
        extra_env: inputs.oracle_env.clone(),
    };
    let record = BenchmarkExecution::run(
        &benchmark_request,
        adapter.as_ref(),
        &CancellationToken::new(),
    )
    .await
    .map_err(|rejection| RunError::Native(rejection.to_string()))?;
    let passing = match &record.completion {
        BenchmarkCompletion::Verified { metrics, .. } => {
            matches!(&metrics.passed, Some(Fact::Known { value, .. }) if *value > 0)
        }
        _ => false,
    };
    if !passing {
        return Err(RunError::Native(format!(
            "oracle preflight for {} did not pass natively",
            inputs.adapter_key
        )));
    }
    let receipt = json!({
        "schema": "phase18-oracle-preflight/1",
        "experiment": experiment.experiment_id(),
        "benchmark": inputs.adapter_key,
        "task": inputs.task_id,
        "oracle_executable_sha256": material
            .benchmark(&inputs.adapter_key)
            .map(|entry| entry.oracle.sha256.clone())
            .unwrap_or_default(),
        "outcome": "passed",
    });
    let bytes =
        serde_json::to_vec(&receipt).map_err(|error| RunError::Native(error.to_string()))?;
    std::fs::create_dir_all(&root).map_err(|error| RunError::Native(error.to_string()))?;
    std::fs::write(root.join("preflight-receipt.json"), &bytes)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let _ = &record.exit;
    Ok(receipt)
}

/// The concrete adapter for one resolved revision (shared by trials and
/// the oracle preflight).
fn benchmark_adapter_for(
    revision: &BenchmarkRevision,
) -> Result<Box<dyn BenchmarkAdapter>, RunError> {
    let profile_bytes = match revision {
        BenchmarkRevision::TerminalBench21 { profile_bytes, .. }
        | BenchmarkRevision::TerminalBench30 { profile_bytes, .. }
        | BenchmarkRevision::DeepSwe { profile_bytes, .. } => profile_bytes.clone(),
    };
    match revision {
        BenchmarkRevision::TerminalBench21 { .. } => {
            let profile = Tb21Profile::parse(profile_bytes.as_slice())
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(TerminalBench21Adapter::from_profile(profile)))
        }
        BenchmarkRevision::TerminalBench30 { .. } => {
            let profile =
                crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(
                crate::benchmark::terminal_bench_30::TerminalBench30Adapter::from_profile(profile),
            ))
        }
        BenchmarkRevision::DeepSwe { .. } => {
            let profile =
                crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(
                crate::benchmark::deepswe::DeepSweAdapter::from_profile(profile),
            ))
        }
    }
}

/// Run one declared trial end to end and persist its receipts.
async fn run_trial(
    experiment: &ResolvedExperiment,
    integrity: &IntegrityRecord,
    revision: &BenchmarkRevision,
    declared: &crate::experiment::DeclaredTrial,
    product: &str,
    native: Option<&NativeInputs>,
    request: &RunRequest,
) -> Result<TrialResult, RunError> {
    let trial_root = request.root.join("trials").join(&declared.id);
    let workspace = trial_root.join("workspace");
    let agent_trace = trial_root.join("agent-trace");
    let verifier_trace = trial_root.join("verifier-trace");
    let isolation = IsolationDirs {
        home: trial_root.join("iso/home"),
        app_data: trial_root.join("iso/appdata"),
        sessions: trial_root.join("iso/sessions"),
    };
    for dir in [
        &workspace,
        &agent_trace,
        &verifier_trace,
        &isolation.home,
        &isolation.app_data,
        &isolation.sessions,
    ] {
        std::fs::create_dir_all(dir).map_err(|error| RunError::Staging(error.to_string()))?;
    }
    let config_path = trial_root.join("bench.toml");
    match native {
        Some(_) if product == "opi" => {
            let material = request.material.as_ref().expect("native implies material");
            let agent = material
                .agent(product)
                .map_err(|error| RunError::Native(error.to_string()))?;
            std::fs::write(&config_path, native_opi_config(&agent.config, &agent.model))
                .map_err(|error| RunError::Native(error.to_string()))?;
        }
        Some(_) => {
            std::fs::write(
                &config_path,
                "# native run: pi reads its configuration from the isolated agent dir\n",
            )
            .map_err(|error| RunError::Native(error.to_string()))?;
        }
        None => {
            std::fs::write(
                &config_path,
                "# assembled-run fixture config (never a user config)\n",
            )
            .map_err(|error| RunError::Staging(error.to_string()))?;
        }
    }

    // A trial identity is never reused: an existing durable reservation
    // refuses the run before any staging (P18-EXP-005). Retries and
    // replacements take fresh identities.
    if trial_root.join("bundle/intent.json").is_file() {
        return Err(RunError::Bundle(format!(
            "trial {} already has a durable intent reservation",
            declared.id
        )));
    }

    // Durable intent before any process effect (P18-DUR-001).
    let trial_id =
        TrialIdentity::new(&declared.id).map_err(|error| RunError::Staging(error.to_string()))?;
    let mut bundle = RunBundle::create(&trial_root.join("bundle"))
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let intent = IntentRecord {
        trial: trial_id.clone(),
        pair: PairIdentity::new(
            experiment
                .edges()
                .first()
                .map(|edge| edge.id.as_str())
                .unwrap_or("pair-1"),
        )
        .map_err(|error| RunError::Staging(error.to_string()))?,
        artifacts: Vec::new(),
        expected_output: ArtifactKey::new(EXPECTED_OUTPUT_KEY)
            .map_err(|error| RunError::Staging(error.to_string()))?,
    };
    let mut lifecycle = TrialLifecycle::plan(trial_id.clone());
    lifecycle
        .publish_intent(intent.clone())
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let proof = bundle
        .publish_intent(&intent)
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    lifecycle
        .enter_process_effect_pending(proof)
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    if request.behavior == "crash-after-intent" {
        // Hermetic crash simulation at the exact durable point: the
        // process dies with the intent reservation on disk and nothing
        // else, which is precisely the state a real crash leaves.
        std::process::exit(70);
    }

    let mut ledger = AuthorityLedger::new();

    // Agent dispatch through the shared execution driver. Hermetic mode
    // synthesizes the deterministic helper and replays fixture bytes;
    // native mode spawns the exact built executable from the material
    // with the deterministic configuration projection and the official
    // instruction as the prompt.
    let (agent_executable, request_env, provider_model, allow_mutating, prompt) = match native {
        Some(inputs) => {
            let material = request.material.as_ref().expect("native implies material");
            let agent = material
                .agent(product)
                .map_err(|error| RunError::Native(error.to_string()))?;
            let env = native_agent_env(product, &agent.config, &isolation)
                .map_err(|error| RunError::Native(error.to_string()))?;
            let mut env: BTreeMap<std::ffi::OsString, std::ffi::OsString> = env;
            for (key, value) in &agent.provider_env {
                env.insert(key.clone().into(), value.clone().into());
            }
            (
                agent.executable.path.clone(),
                env,
                agent.model.clone(),
                true,
                inputs.prompt.clone(),
            )
        }
        None => {
            let helper = trial_root.join("helper-agent.sh");
            write_helper(&helper, &agent_helper_script(product))?;
            (
                helper,
                agent_env(product, &request.fixtures, &request.behavior)?,
                format!(
                    "{}:{}",
                    experiment.model_controls().provider,
                    experiment.model_controls().model
                ),
                false,
                format!("assembled run: solve task {}", declared.task),
            )
        }
    };
    let agent_request = AgentRunRequest {
        executable: agent_executable,
        prompt,
        workspace: workspace.clone(),
        trace_root: agent_trace.clone(),
        config_path,
        provider_model,
        allow_mutating,
        isolation,
        extra_env: request_env,
    };

    let agent_record = if ledger.attempt(AuthorityTransition::AgentDispatch) {
        let cancel = CancellationToken::new();
        let adapter = if request.behavior == "agent-timeout" {
            timeout_patched_adapter(product)?
        } else {
            agent_adapter(product, &request.behavior)?
        };
        if request.behavior == "agent-cancelled" {
            // The cancellation must race a live process that has provably
            // mutated the workspace: poll the run (which spawns the child),
            // wait for the mutation marker, and only then cancel - never a
            // pre-spawn kill.
            let mut run = Box::pin(AgentExecution::run(
                &agent_request,
                adapter.as_ref(),
                &cancel,
            ));
            let marker = workspace.join("answer.txt");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
            loop {
                tokio::select! {
                    biased;
                    record = &mut run => {
                        break record.map_err(|rejection| {
                            RunError::Staging(rejection.to_string())
                        })?;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {
                        if marker.is_file() || std::time::Instant::now() > deadline {
                            cancel.cancel();
                        }
                    }
                }
            }
        } else {
            AgentExecution::run(&agent_request, adapter.as_ref(), &cancel)
                .await
                .map_err(|rejection| RunError::Staging(rejection.to_string()))?
        }
    } else {
        // Structurally impossible: nothing precedes the agent dispatch.
        return Err(RunError::Staging(
            "agent dispatch refused before any failure".to_owned(),
        ));
    };

    // Settlement of the observed outcome, durably (P18-DUR-003).
    let settlement_kind = agent_record.settlement_kind(CancellationSource::Infrastructure);
    if let AgentCompletion::Failed(failure) = &agent_record.completion {
        ledger.fail(failure.boundary);
    }
    let settle_executed = ledger.attempt(AuthorityTransition::Settle);
    debug_assert!(settle_executed, "settle follows only a dispatch attempt");
    lifecycle
        .settle(ObservedOutcome {
            kind: settlement_kind,
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    // Native agent evidence into the staging bundle.
    let source = SourceIdentity::new(&format!("agent-{product}"))
        .map_err(|error| RunError::Staging(error.to_string()))?;
    let mut staged_keys: Vec<ArtifactKey> = Vec::new();
    staged_keys.push(stage(
        &mut bundle,
        &source,
        "native/agent-stdout.log",
        &agent_record.stdout.bytes,
    )?);
    staged_keys.push(stage(
        &mut bundle,
        &source,
        "native/agent-stderr.log",
        &agent_record.stderr.bytes,
    )?);
    let answer_path = workspace.join("answer.txt");
    if let Ok(answer) = std::fs::read(&answer_path) {
        staged_keys.push(stage(
            &mut bundle,
            &source,
            "native/agent-answer.txt",
            &answer,
        )?);
    }
    if let AgentCompletion::Completed { artifacts } = &agent_record.completion {
        for artifact in artifacts {
            if let Ok(bytes) = std::fs::read(&artifact.path) {
                staged_keys.push(stage(
                    &mut bundle,
                    &source,
                    &format!("native/{}", artifact.role),
                    &bytes,
                )?);
            }
        }
    }
    bundle
        .record_settlement(&SettlementMarker {
            trial: trial_id.clone(),
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    // Native verifier dispatch through the shared execution driver.
    let mut verifier_record = None;
    let mut verifier_rejection: Option<String> = None;
    if ledger.attempt(AuthorityTransition::GradeDispatch) {
        let verifier_helper = match native {
            Some(inputs) => inputs.verifier_executable.clone(),
            None => {
                let helper = trial_root.join("helper-verifier.sh");
                write_helper(&helper, &verifier_helper_script(revision.report_name()))?;
                helper
            }
        };
        let task_package = match revision {
            BenchmarkRevision::TerminalBench21 { task_package, .. }
            | BenchmarkRevision::TerminalBench30 { task_package, .. }
            | BenchmarkRevision::DeepSwe { task_package, .. } => task_package.clone(),
        };
        let task_dir = trial_root.join("task-package");
        if request.behavior == "prompt-only-package" {
            // The incomplete package: the prompt alone, without the
            // image/resource/verifier contract (P18-BMK-002).
            std::fs::create_dir_all(&task_dir)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            std::fs::copy(
                task_package.join("instruction.md"),
                task_dir.join("instruction.md"),
            )
            .map_err(|error| RunError::Staging(error.to_string()))?;
        } else {
            copy_dir_recursive(&task_package, &task_dir)
                .map_err(|error| RunError::Staging(error.to_string()))?;
        }
        let fixture_report = match revision {
            BenchmarkRevision::TerminalBench21 { fixture_report, .. }
            | BenchmarkRevision::TerminalBench30 { fixture_report, .. }
            | BenchmarkRevision::DeepSwe { fixture_report, .. } => fixture_report.clone(),
        };
        let benchmark_request_adapter: Box<dyn BenchmarkAdapter> = match revision {
            BenchmarkRevision::TerminalBench21 { profile_bytes, .. } => {
                let profile = Tb21Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(TerminalBench21Adapter::from_profile(profile))
            }
            BenchmarkRevision::TerminalBench30 { profile_bytes, .. } => {
                let profile = crate::benchmark::terminal_bench_30::Tb30Profile::parse(
                    profile_bytes.as_slice(),
                )
                .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(
                    crate::benchmark::terminal_bench_30::TerminalBench30Adapter::from_profile(
                        profile,
                    ),
                )
            }
            BenchmarkRevision::DeepSwe { profile_bytes, .. } => {
                let profile =
                    crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                        .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(crate::benchmark::deepswe::DeepSweAdapter::from_profile(
                    profile,
                ))
            }
        };
        let mut env: BTreeMap<std::ffi::OsString, std::ffi::OsString> = match native {
            Some(inputs) => inputs.verifier_env.clone(),
            None => BTreeMap::new(),
        };
        if let Some(report) = &fixture_report {
            env.insert(
                "OPI_EVAL_NATIVE_SOURCE".into(),
                report.clone().into_os_string(),
            );
            env.insert(
                "OPI_EVAL_RUN_BEHAVIOR".into(),
                request.behavior.clone().into(),
            );
        }
        let benchmark_request = BenchmarkRunRequest {
            verifier_executable: verifier_helper,
            task_dir,
            task_id: declared.task.clone(),
            agent_output: answer_path,
            trace_root: verifier_trace.clone(),
            // Hermetic staging pins a per-trial fixture digest; the native
            // mode binds the admitted static external lock instead.
            admitted_lock_digest: match native {
                Some(_) => request
                    .material
                    .as_ref()
                    .expect("native implies material")
                    .static_lock
                    .sha256
                    .clone(),
                None => crate::agent::opi::sha256_hex(declared.id.as_bytes()),
            },
            integrity: integrity.clone(),
            extra_env: env,
        };
        let record = BenchmarkExecution::run(
            &benchmark_request,
            benchmark_request_adapter.as_ref(),
            &CancellationToken::new(),
        )
        .await;
        match record {
            Ok(record) => {
                if let BenchmarkCompletion::Failed(failure) = &record.completion {
                    ledger.fail(failure.boundary);
                }
                // Native verifier evidence into the staging bundle.
                staged_keys.push(stage(
                    &mut bundle,
                    &source,
                    "native/verifier-stdout.log",
                    &record.stdout.bytes,
                )?);
                staged_keys.push(stage(
                    &mut bundle,
                    &source,
                    "native/verifier-stderr.log",
                    &record.stderr.bytes,
                )?);
                if let BenchmarkCompletion::Verified { artifacts, .. } = &record.completion {
                    for artifact in artifacts {
                        if let Ok(bytes) = std::fs::read(&artifact.path) {
                            staged_keys.push(stage(
                                &mut bundle,
                                &source,
                                &format!("native/{}", artifact.role),
                                &bytes,
                            )?);
                        }
                    }
                }
                verifier_record = Some(record);
            }
            Err(rejection) => {
                // Pre-spawn admission refusal: no verifier process ever
                // existed. The owning token is persisted, never retried
                // against another revision or grader.
                ledger.fail(FailureBoundaryCode::Integrity);
                verifier_rejection = Some(rejection.token.to_owned());
            }
        }
    }

    // Pre-seal projection over the settled facts (task 18.11). The
    // projection itself fails closed if the ladder ever passed settlement,
    // so no phase guard is needed here.
    let projection = match &verifier_record {
        Some(benchmark) => ProjectionPipeline::project(&TrialInputs {
            agent: &agent_record,
            benchmark: Some(benchmark),
            lifecycle: &lifecycle,
            workspace: Some(&workspace),
        })
        .ok(),
        None => ProjectionPipeline::project(&TrialInputs {
            agent: &agent_record,
            benchmark: None,
            lifecycle: &lifecycle,
            workspace: Some(&workspace),
        })
        .ok(),
    };

    // The authority ledger itself is durable evidence: its execution
    // counts enter the sealed bundle (P18-FAL-002 call-count proof). The
    // sealed copy honestly covers every transition up to sealing; the
    // seal and report states live only in the outer receipt.
    {
        let ledger_json =
            serde_json::to_vec(&ledger).map_err(|error| RunError::Staging(error.to_string()))?;
        staged_keys.push(stage(
            &mut bundle,
            &source,
            "native/authority-ledger.json",
            &ledger_json,
        )?);
    }

    // Pre-seal redaction gate (`P18-A18`, `P18-SEC-005`): a declared
    // canary anywhere in the staged exportable content blocks sealing, so
    // the leak never enters a published manifest. The refusal is the
    // evidence boundary and the bundle stays unsealed on disk.
    let canary_leak =
        scan_staged_for_canaries(&trial_root.join("bundle"), &staged_keys, &request.canaries);
    let seal_outcome = if canary_leak.is_some() {
        ledger.attempt_failed(AuthorityTransition::Seal, FailureBoundaryCode::Evidence);
        None
    } else if ledger.attempt(AuthorityTransition::Seal) {
        if request.behavior == "seal-failure" {
            // Hermetic seal-failure simulation: a staged artifact is
            // tampered between staging and sealing, so canonical sealing
            // fails its digest validation exactly as real drift would.
            let tamper = trial_root.join("bundle/artifacts/native/agent-stdout.log");
            std::fs::write(&tamper, b"tampered\n")
                .map_err(|error| RunError::Staging(error.to_string()))?;
        }
        match bundle.seal() {
            Ok(receipt) => {
                lifecycle
                    .mark_sealed()
                    .map_err(|error| RunError::Bundle(error.to_string()))?;
                Some(receipt)
            }
            Err(error) => {
                ledger.attempt_failed(AuthorityTransition::Seal, error.boundary());
                None
            }
        }
    } else {
        None
    };

    // Trial facts for pairing: a sealed graded trial contributes its
    // outcome class; failures contribute their failure class so coverage
    // shows the exact reason instead of hiding the trial.
    let agent_failed = matches!(agent_record.completion, AgentCompletion::Failed(_));
    let verifier_failed = verifier_rejection.is_some()
        || verifier_record
            .as_ref()
            .is_some_and(|record| matches!(record.completion, BenchmarkCompletion::Failed(_)));
    let fact = if seal_outcome.is_some() {
        let outcome = if agent_failed {
            TrialOutcome::InfrastructureFailure
        } else if verifier_failed {
            TrialOutcome::GraderFailure
        } else {
            TrialOutcome::ValidAgentOutcome
        };
        // Honest per-profile control realization: neither pinned agent
        // profile expresses a reasoning launch control, so a shared
        // reasoning value is unsupported by every subject and the pair
        // must stay non-comparable (P18-EXP-008).
        let unsupported_controls = match experiment.model_controls().reasoning {
            crate::experiment::ControlValue::Value(_) => vec!["reasoning".to_owned()],
            _ => Vec::new(),
        };
        Some(TrialFact {
            id: declared.id.clone(),
            subject: declared.subject.clone(),
            task: declared.task.clone(),
            group: declared.group.clone(),
            manifest_digest: experiment.manifest_digest().to_owned(),
            control_fingerprint: control_fingerprint(experiment),
            unsupported_controls,
            outcome,
        })
    } else {
        None
    };

    let report_executed = ledger.attempt(AuthorityTransition::Report);
    let mut receipt = json!({
        "schema": TRIAL_RECEIPT_SCHEMA,
        "id": declared.id,
        "subject": declared.subject,
        "task": declared.task,
        "group": declared.group,
        "status": if seal_outcome.is_some() { "sealed" } else { "failed" },
        "agent": {
            "product": product,
            "exit_state": exit_state_token(&agent_record.exit),
            "completion": if agent_failed { "failed" } else { "completed" },
            "failure_kind": agent_failure_kind(&agent_record.completion),
            "boundary": agent_failure_boundary(&agent_record.completion),
            "stdout_truncated": agent_record.stdout.truncated,
            "stderr_truncated": agent_record.stderr.truncated,
            "cleanup": cleanup_token(&agent_record.cleanup),
            "stdout_bytes": agent_record.stdout.bytes.len(),
            "stderr_bytes": agent_record.stderr.bytes.len(),
        },
        "verifier": verifier_rejection.as_ref().map(|token| {
            json!({
                "rejected": token,
                "boundary": "integrity",
            })
        }).or_else(|| verifier_record.as_ref().map(|record| {
            json!({
                "exit_state": exit_state_token(&record.exit),
                "reward": fact_token(&record.reward),
                "completion": match &record.completion {
                    BenchmarkCompletion::Verified { .. } => "verified",
                    BenchmarkCompletion::Failed(_) => "failed",
                },
                "failure_kind": benchmark_failure_kind(record),
                "boundary": benchmark_failure_boundary(record),
            })
        })),
        "authority": authority_json(&ledger),
    });
    let status = if seal_outcome.is_some() {
        TrialStatus::Sealed
    } else {
        TrialStatus::Failed
    };
    if let Some(sealed) = &seal_outcome {
        receipt["bundle_identity"] = json!(sealed.bundle_identity());
    }
    if let Some(trajectory) = &projection {
        let mut trajectory_receipt =
            crate::trajectory::TrajectoryReceipt::for_trajectory(trajectory);
        if let Some(sealed) = &seal_outcome {
            trajectory_receipt.record_seal(crate::trajectory::SealOutcome::Sealed {
                bundle_digest: sealed.bundle_identity().to_owned(),
            });
        }
        receipt["pre_seal_digest"] = json!(trajectory_receipt.pre_seal_digest);
        receipt["seal_result"] = match &trajectory_receipt.seal_result {
            Some(crate::trajectory::SealOutcome::Sealed { bundle_digest }) => {
                json!({"sealed": {"bundle_digest": bundle_digest}})
            }
            Some(crate::trajectory::SealOutcome::SealFailed { reason }) => {
                json!({"seal_failed": {"reason": reason}})
            }
            None => serde_json::Value::Null,
        };
    }

    if report_executed {
        let receipt_path = trial_root.join("receipt.json");
        std::fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt).map_err(|error| RunError::Staging(error.to_string()))?,
        )
        .map_err(|error| RunError::Staging(error.to_string()))?;
        lifecycle
            .mark_graded()
            .and_then(|()| lifecycle.mark_reported())
            .map_err(|error| RunError::Bundle(error.to_string()))?;
    }

    Ok(TrialResult {
        receipt,
        fact,
        sealed: status == TrialStatus::Sealed,
    })
}

/// Stage one native artifact into `bundle` and return its key.
fn stage(
    bundle: &mut RunBundle,
    source: &SourceIdentity,
    role: &str,
    bytes: &[u8],
) -> Result<ArtifactKey, RunError> {
    let key = ArtifactKey::new(role).map_err(|error| RunError::Staging(error.to_string()))?;
    bundle
        .insert(
            ArtifactSpec {
                role: ArtifactRole::Native,
                source: source.clone(),
                path: key.clone(),
                bytes: bytes.to_vec(),
                classification: Sensitivity::Exportable,
            },
            Vec::new(),
        )
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    Ok(key)
}

/// Resolve the concrete agent adapter for one subject product.
fn agent_adapter(product: &str, behavior: &str) -> Result<Box<dyn AgentAdapter>, RunError> {
    let _ = behavior;
    match product {
        "opi" => Ok(Box::new(OpiProcessAdapter::new())),
        "pi" => Ok(Box::new(PiProcessAdapter::new())),
        other => Err(RunError::UnsupportedSubject {
            product: other.to_owned(),
        }),
    }
}

/// The bounded-timeout variant of a pinned agent profile for the
/// timeout-race behavior: the same pinned identity and launch surface with
/// only the supervisor timeout lowered. Fails closed when the pinned
/// timeout line is absent.
fn timeout_patched_adapter(product: &str) -> Result<Box<dyn AgentAdapter>, RunError> {
    let text = if product == "opi" {
        include_str!("../../profiles/agents/opi.toml")
    } else {
        include_str!("../../profiles/agents/pi.toml")
    };
    let patched = text
        .replace("timeout_secs = 900", "timeout_secs = 2")
        .replace("timeout_secs = 1800", "timeout_secs = 2");
    if patched == text {
        return Err(RunError::Staging(
            "pinned timeout line not found for the timeout variant".to_owned(),
        ));
    }
    match product {
        "opi" => {
            let profile = crate::agent::opi::OpiProfile::parse(&patched)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(OpiProcessAdapter::from_profile(profile)))
        }
        "pi" => {
            let profile = crate::agent::pi::PiProfile::parse(&patched)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(PiProcessAdapter::from_profile(profile)))
        }
        other => Err(RunError::UnsupportedSubject {
            product: other.to_owned(),
        }),
    }
}

/// Delay before the runner cancels a running trial (cancellation race).
const CANCEL_AFTER_MS: u64 = 300;

/// Native-source environment for the agent helper.
fn agent_env(
    product: &str,
    fixtures: &Path,
    behavior: &str,
) -> Result<BTreeMap<std::ffi::OsString, std::ffi::OsString>, RunError> {
    let mut env: BTreeMap<std::ffi::OsString, std::ffi::OsString> = BTreeMap::new();
    let agents = fixtures.join("agents");
    // The saved-bytes fixture each stream-contract behavior replays.
    let trace_source = match behavior {
        "agent-unknown-schema" => "trace-unknown-schema",
        "agent-malformed-stream" => "trace-corrupt",
        "agent-missing-terminal" => "trace-missing-evidence",
        _ => "trace-complete",
    };
    let stream_source = match behavior {
        "agent-unknown-schema" => "unknown-event.jsonl",
        "agent-malformed-stream" => "corrupt.jsonl",
        "agent-missing-terminal" => "no-agent-end.jsonl",
        _ => "stream-ok.jsonl",
    };
    if behavior == "canary-leak" {
        env.insert("OPI_EVAL_CANARY_SECRET".into(), HERMETIC_CANARY.into());
    }
    if product == "opi" {
        env.insert(
            "OPI_EVAL_TRACE_SOURCE".into(),
            agents.join("opi").join(trace_source).into_os_string(),
        );
    } else {
        env.insert(
            "PI_EVAL_STREAM_SOURCE".into(),
            agents.join("pi").join(stream_source).into_os_string(),
        );
    }
    env.insert("OPI_EVAL_RUN_BEHAVIOR".into(), behavior.to_owned().into());
    Ok(env)
}

fn exit_state_token(exit: &ExitState) -> String {
    match exit {
        ExitState::Exited { code } => format!("exited:{code}"),
        ExitState::TimedOut => "timed-out".to_owned(),
        ExitState::Cancelled => "cancelled".to_owned(),
        ExitState::FailedToSpawn { .. } => "failed-to-spawn".to_owned(),
    }
}

fn cleanup_token(cleanup: &crate::process::CleanupEvidence) -> String {
    match cleanup {
        crate::process::CleanupEvidence::NotRequired => "not-required".to_owned(),
        crate::process::CleanupEvidence::TreeTerminated { layer, verified } => {
            format!(
                "tree-terminated:{layer}:{}",
                if *verified { "verified" } else { "unverified" }
            )
        }
        crate::process::CleanupEvidence::TreeTerminationFailed { layer } => {
            format!("tree-termination-failed:{layer}")
        }
    }
}

fn fact_token(fact: &Fact) -> String {
    match fact {
        Fact::Known { value, origin } => format!("known:{value}({origin})"),
        Fact::Unknown { reason } => format!("unknown:{reason}"),
    }
}

fn agent_failure_kind(completion: &AgentCompletion) -> Option<String> {
    match completion {
        AgentCompletion::Failed(failure) => Some(failure.kind.to_owned()),
        AgentCompletion::Completed { .. } => None,
    }
}

fn agent_failure_boundary(completion: &AgentCompletion) -> Option<String> {
    match completion {
        AgentCompletion::Failed(failure) => Some(boundary_token(failure.boundary).to_owned()),
        AgentCompletion::Completed { .. } => None,
    }
}

fn benchmark_failure_kind(record: &crate::benchmark::process::BenchmarkRecord) -> Option<String> {
    match &record.completion {
        BenchmarkCompletion::Failed(failure) => Some(failure.kind.to_owned()),
        BenchmarkCompletion::Verified { .. } => None,
    }
}

fn benchmark_failure_boundary(
    record: &crate::benchmark::process::BenchmarkRecord,
) -> Option<String> {
    match &record.completion {
        BenchmarkCompletion::Failed(failure) => Some(boundary_token(failure.boundary).to_owned()),
        BenchmarkCompletion::Verified { .. } => None,
    }
}

fn authority_json(ledger: &AuthorityLedger) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for record in ledger.records() {
        map.insert(record.transition.token().to_owned(), json!(record.state));
    }
    serde_json::Value::Object(map)
}

fn comparability_token(comparability: &crate::comparison::Comparability) -> String {
    match comparability {
        crate::comparison::Comparability::Comparable => "comparable".to_owned(),
        crate::comparison::Comparability::NonComparable(reason) => non_comparability_token(reason),
    }
}

fn non_comparability_token(reason: &crate::comparison::NonComparability) -> String {
    use crate::comparison::NonComparability as N;
    match reason {
        N::MissingBaselineTrial => "missing-baseline-trial".to_owned(),
        N::MissingCandidateTrial => "missing-candidate-trial".to_owned(),
        N::ControlMismatch { .. } => "control-mismatch".to_owned(),
        N::UnsupportedControl { control } => format!("unsupported-control:{control}"),
        N::Excluded { trial, reason } => format!("excluded:{trial}:{reason}"),
        N::InfrastructureFailure { trial } => format!("infrastructure-failure:{trial}"),
        N::GraderFailure { trial } => format!("grader-failure:{trial}"),
        N::InvalidTaskClassification { .. } => "invalid-task-classification".to_owned(),
        N::TaskNotCovered => "task-not-covered".to_owned(),
    }
}

/// Rewrite the experiment document so every trial of the group owning
/// `crashed_trial` gets a fresh identity and the group gets a new paired
/// id. Line-scoped: only `id =` and `group =` lines inside the affected
/// `[[trials]]` blocks change.
fn replace_group(source: &str, crashed_trial: &str) -> Result<String, RunError> {
    let original = ResolvedExperiment::resolve(source)?;
    let Some(crashed) = original.trials().iter().find(|t| t.id == crashed_trial) else {
        return Err(RunError::Staging(format!(
            "replacement target {crashed_trial:?} is not a declared trial"
        )));
    };
    let group = &crashed.group;
    let group_trials: Vec<&str> = original
        .trials()
        .iter()
        .filter(|trial| trial.group == *group)
        .map(|trial| trial.id.as_str())
        .collect();
    // Line-scoped rewrite: only `id =` lines naming a trial of the group
    // and `group =` lines naming the group itself change; ids and the
    // group id are unique across the document.
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        let rewrite = if trimmed.starts_with("id =") || trimmed.starts_with("group =") {
            trimmed
                .split('"')
                .nth(1)
                .map(|value| group_trials.contains(&value) || value == group)
        } else {
            None
        };
        match rewrite {
            Some(true) => {
                let value = trimmed.split('"').nth(1).unwrap();
                out.push_str(&line.replacen(value, &format!("{value}.r2"), 1));
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    Ok(out)
}

/// Classify every existing trial root under `root` from its durable files
/// only. Effect-unknown trials keep their original identity and boundary
/// and are never reclassified as not-started (P18-DUR-002).
fn recovery_report(experiment: &ResolvedExperiment, root: &Path) -> serde_json::Value {
    let mut recovery = Vec::new();
    let mut effect_unknown = 0;
    let trials_root = root.join("trials");
    if let Ok(entries) = std::fs::read_dir(&trials_root) {
        let mut ids: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        for id in ids {
            let observed = RunBundle::recover(&trials_root.join(&id).join("bundle"));
            let row = match observed {
                Ok(observed) => match TrialLifecycle::recover(&observed) {
                    crate::runner::lifecycle::RecoveryClassification::NotStarted => {
                        json!({"id": id, "status": "not-started"})
                    }
                    crate::runner::lifecycle::RecoveryClassification::EffectUnknown {
                        trial,
                        boundary,
                    } => {
                        effect_unknown += 1;
                        json!({
                            "id": id,
                            "trial": trial.as_str(),
                            "status": "effect-unknown",
                            "boundary": boundary_token(boundary),
                        })
                    }
                    crate::runner::lifecycle::RecoveryClassification::SettledUnsealed => {
                        json!({"id": id, "status": "settled-unsealed"})
                    }
                    crate::runner::lifecycle::RecoveryClassification::Sealed => {
                        json!({"id": id, "status": "sealed"})
                    }
                },
                Err(error) => json!({"id": id, "status": "unreadable", "error": error.to_string()}),
            };
            recovery.push(row);
        }
    }
    // Pairing visibility for the recovery view: for every edge, the sides
    // whose subjects have no durably settled trial root are named exactly.
    // No fact is reconstructed from disk - recovery reports durable state
    // only.
    let settled_ids: std::collections::BTreeSet<&str> = recovery
        .iter()
        .filter(|row| row["status"] == "sealed" || row["status"] == "settled-unsealed")
        .map(|row| row["id"].as_str().unwrap_or_default())
        .collect();
    let pairs: Vec<serde_json::Value> = experiment
        .edges()
        .iter()
        .map(|edge| {
            let mut missing = Vec::new();
            for (role, subject) in [("baseline", &edge.baseline), ("candidate", &edge.candidate)] {
                let settled = experiment.trials().iter().any(|trial| {
                    trial.subject == *subject && settled_ids.contains(trial.id.as_str())
                });
                if !settled {
                    missing.push(role);
                }
            }
            let comparability = if missing.contains(&"baseline") {
                "missing-baseline-trial"
            } else if missing.contains(&"candidate") {
                "missing-candidate-trial"
            } else {
                "settled-unsealed"
            };
            json!({
                "edge": edge.id,
                "missing_sides": missing,
                "comparability": comparability,
            })
        })
        .collect();
    let _ = &mut effect_unknown;
    json!({
        "schema": RUN_REPORT_SCHEMA,
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "outcome": "incomplete",
        "recovery": recovery,
        "trials": [],
        "pairs": pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_experiment_pins_the_derived_integrity_identity() {
        // (document, staging behavior) pairs and the review they derive.
        let cases: &[(&str, &str)] = &[
            ("phase18-local.toml", "happy"),
            ("phase18-duplicate-pair.toml", "happy"),
            ("phase18-unsupported-control.toml", "happy"),
            ("phase18-tb30.toml", "happy"),
            ("phase18-deepswe.toml", "happy"),
            ("phase18-integrity-exclusion.toml", "integrity-exclusion"),
            ("phase18-invalid-task.toml", "invalid-task"),
        ];
        for (name, behavior) in cases {
            let text =
                std::fs::read_to_string(format!("tests/fixtures/experiment/{name}")).unwrap();
            let experiment = ResolvedExperiment::resolve(&text).unwrap();
            let review = fixture_integrity_review(&experiment, behavior);
            let record = IntegrityRecord::review(review).unwrap();
            assert_eq!(
                experiment.benchmark().integrity_digest.as_deref(),
                Some(record.identity_digest()),
                "{name} must pin the digest of its derived fixture review"
            );
        }
    }
}

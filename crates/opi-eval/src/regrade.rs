//! Offline original byte/seal verification over sealed assembled
//! outputs (the engine behind `opi-eval verify`), plus the actual
//! native offline regrader behind `opi-eval regrade`
//! (P19-AC-009).
//!
//! [`OfflineRegrader::regrade`] walks the sealed trial bundles under one
//! run root and re-verifies each one through [`crate::bundle::RunBundle::verify`]:
//! it recomputes the manifest identity, re-reads every covered artifact,
//! and fails on any mutation. It never repairs, rehashes, or rewrites
//! anything (`EVAL-A15`), never starts an Agent or provider, and never
//! mutates a sealed bundle (`EVAL-RPT-001`): the walk only reads the run
//! root and reports what the durable bytes prove. Machine-local absolute
//! paths never enter the report; trials are identified by their durable
//! ids only.
//!
//! [`NativeRegrader::regrade`] performs the distinct actual-native
//! operation: it verifies every source bundle first, requires the saved
//! real Agent artifacts and the cached immutable task/verifier material,
//! then re-grades the preserved artifacts with the pinned native
//! verifier in a fresh trial-owned environment under the output root -
//! reusing the same binding readers as initial execution
//! ([`crate::benchmark::process::import_bound_native_job`]) - and
//! persists one derived grading record per trial without rewriting any
//! original score. No Agent executable is spawned, no credential is
//! resolved, and no provider is contacted; a verifier whose task
//! declares a network requirement without a permitted offline
//! equivalent rejects fail-closed.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bundle::{BundleError, RunBundle};

/// Regrade report schema identity.
const REGRADE_SCHEMA: &str = "opi-eval-regrade-report/1";

/// One verified sealed bundle: trial id plus content-addressed identity.
#[derive(Debug, Serialize)]
struct VerifiedBundle {
    trial: String,
    bundle_identity: String,
}

/// One verification failure with the typed reason the durable bytes proved.
#[derive(Debug, Serialize)]
struct VerificationFailure {
    trial: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<String>,
}

/// The full regrade report: every sealed bundle verified or the typed
/// failures that blocked verification. Field order is fixed by serde and
/// collections are ordered, so identical sealed inputs serialize to
/// identical bytes (`EVAL-RPT-002`).
#[derive(Debug, Serialize)]
pub(crate) struct RegradeReport {
    schema: &'static str,
    outcome: &'static str,
    bundles: Vec<VerifiedBundle>,
    failures: Vec<VerificationFailure>,
}

impl RegradeReport {
    /// The serialized canonical JSON report (byte-stable).
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("regrade report serializes")
    }
}

/// The offline regrader. Constructing it starts nothing; only
/// [`OfflineRegrader::regrade`] reads the run root.
pub(crate) struct OfflineRegrader;

impl OfflineRegrader {
    /// Verifies every sealed trial bundle under `run_root/trials/*` without
    /// mutating anything. A bundle directory missing `manifest.json` is
    /// reported as unsealed (a crashed or seal-blocked trial), not skipped
    /// silently: the denominator keeps every durable trial visible
    /// (`EVAL-EXP-006`).
    pub(crate) fn regrade(run_root: &Path) -> Result<RegradeReport, std::io::Error> {
        let mut bundles = Vec::new();
        let mut failures = Vec::new();
        let mut trials = read_trial_ids(run_root)?;
        for trial in trials.drain(..) {
            let bundle_root = run_root.join("trials").join(&trial).join("bundle");
            match RunBundle::verify(&bundle_root) {
                Ok(receipt) => bundles.push(VerifiedBundle {
                    trial,
                    bundle_identity: receipt.bundle_identity().to_owned(),
                }),
                Err(error) => failures.push(classify(&trial, error)),
            }
        }
        let outcome = if failures.is_empty() {
            "verified"
        } else {
            "mutation-detected"
        };
        Ok(RegradeReport {
            schema: REGRADE_SCHEMA,
            outcome,
            bundles,
            failures,
        })
    }
}

/// Sorted durable trial ids under one run root. Ordering is by sorted id so
/// the report never depends on directory iteration order.
fn read_trial_ids(run_root: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(run_root.join("trials"))? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Ok(id) = entry.file_name().into_string()
        {
            ids.push(id);
        }
    }
    ids.sort();
    Ok(ids)
}

/// Maps one typed bundle failure to its report token. Symlinks and manifest
/// corruption are verification failures of the same fail-closed kind; the
/// artifact key is carried only for digest mismatches and symlink escapes.
fn classify(trial: &str, error: BundleError) -> VerificationFailure {
    match error {
        BundleError::DigestMismatch { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "digest-mismatch",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::SymlinkEscape { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "symlink-escape",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::ManifestInvalid { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "manifest-invalid",
            artifact: None,
        },
        BundleError::SidecarDrift { which, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "sidecar-drift",
            artifact: Some(which.to_owned()),
        },
        BundleError::UnmanifestedFile { path, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "unmanifested-file",
            artifact: Some(path),
        },
        BundleError::MissingArtifact { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "missing-artifact",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::ReservationBroken { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "reservation-broken",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::UnreservedArtifact { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "unreserved-artifact",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::Io { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "io",
            artifact: None,
        },
        // verify() reads only the sealed manifest and covered artifacts, so
        // the staging-lifecycle variants are unreachable here; they keep one
        // token so the mapping stays exhaustive and fail-closed.
        BundleError::IntentAlreadyReserved { .. }
        | BundleError::IntentNotPublished { .. }
        | BundleError::SettlementAlreadyRecorded { .. }
        | BundleError::SealWithoutIntent { .. }
        | BundleError::SealWithoutSettlement { .. }
        | BundleError::SealedMutation { .. }
        | BundleError::ArtifactTooLarge { .. }
        | BundleError::DuplicateArtifact { .. }
        | BundleError::UnknownCausalEdge { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "lifecycle-invalid",
            artifact: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Actual native offline regrading (P19-AC-009).
// ---------------------------------------------------------------------------

/// Identity schema of the per-trial derived grading record.
const REGRADE_RECORD_SCHEMA: &str = "opi-eval-regrade-record/1";

/// Identity schema of the regrade summary report on stdout.
const NATIVE_REGRADE_SCHEMA: &str = "opi-eval-native-regrade/1";

/// One successfully regraded trial in the summary report.
#[derive(Debug, Serialize)]
struct RegradedTrial {
    trial: String,
    bundle_identity: String,
    /// The fresh native reward of the preserved artifacts.
    reward: u64,
    /// Path of the persisted derived grading record, relative to the
    /// output root.
    record: String,
    /// The fresh job directory name the regrade produced.
    job: String,
    /// Digest of the pinned verifier executable that graded.
    verifier_executable_sha256: String,
}

/// One typed per-trial rejection. The regrade never repairs or downgrades
/// a rejection into a score.
#[derive(Debug, Serialize)]
struct RegradeRejection {
    trial: String,
    reason: String,
}

/// The offline native regrade summary: every sealed trial either
/// regraded with a persisted derived record or rejected with a typed
/// reason. Field order is fixed by serde and collections are ordered,
/// so identical inputs serialize to identical bytes.
#[derive(Debug, Serialize)]
pub(crate) struct NativeRegradeReport {
    schema: &'static str,
    /// `regraded` when every sealed trial regraded; `rejected` when any
    /// trial was rejected.
    outcome: &'static str,
    trials: Vec<RegradedTrial>,
    rejections: Vec<RegradeRejection>,
}

impl NativeRegradeReport {
    /// The serialized canonical JSON report (byte-stable).
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("regrade report serializes")
    }
}

/// Request-level regrade failures (before any trial is examined).
#[derive(Debug)]
pub(crate) enum NativeRegradeError {
    /// The output root already exists; a regrade never overwrites.
    OutputExists(PathBuf),
    /// The run root holds no native trial to regrade (a hermetic fixture
    /// run root is never regraded under this command).
    NotNativeRoot,
    /// The native material was rejected at admission.
    Material(String),
    /// The run root could not be walked.
    Io(std::io::Error),
}

impl std::fmt::Display for NativeRegradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeRegradeError::OutputExists(path) => {
                write!(f, "output root already exists: {}", path.display())
            }
            NativeRegradeError::NotNativeRoot => {
                write!(f, "run root holds no native trial to regrade")
            }
            NativeRegradeError::Material(reason) => write!(f, "native material rejected: {reason}"),
            NativeRegradeError::Io(error) => write!(f, "io: {error}"),
        }
    }
}

/// The actual native offline regrader. Constructing it starts nothing;
/// [`NativeRegrader::regrade`] reads the run root, the material, and
/// writes only under the output root.
pub(crate) struct NativeRegrader;

impl NativeRegrader {
    /// Fresh-grades every sealed native trial bundle under `run_root`
    /// with the pinned verifier from `material`, writing derived records
    /// and fresh verifier environments under `out` only.
    pub(crate) fn regrade(
        run_root: &Path,
        material_path: &Path,
        out: &Path,
    ) -> Result<NativeRegradeReport, NativeRegradeError> {
        // A root with no native trial is not regradable under this
        // command: hermetic fixture evidence never enters native grading.
        let trial_ids = read_trial_ids(run_root).map_err(NativeRegradeError::Io)?;
        let any_native = trial_ids.iter().any(|trial| {
            run_root
                .join("trials")
                .join(trial)
                .join("bundle/artifacts/native/verifier-input.json")
                .is_file()
        });
        if !any_native {
            return Err(NativeRegradeError::NotNativeRoot);
        }
        if out.exists() {
            return Err(NativeRegradeError::OutputExists(out.to_owned()));
        }
        let material = crate::runner::material::NativeMaterial::load(material_path)
            .map_err(|error| NativeRegradeError::Material(error.to_string()))?;
        std::fs::create_dir_all(out).map_err(NativeRegradeError::Io)?;
        let mut trials = Vec::new();
        let mut rejections = Vec::new();
        for trial in trial_ids {
            match regrade_one(run_root, &material, out, &trial) {
                Ok(entry) => trials.push(entry),
                Err(reason) => rejections.push(RegradeRejection { trial, reason }),
            }
        }
        let outcome = if rejections.is_empty() {
            "regraded"
        } else {
            "rejected"
        };
        Ok(NativeRegradeReport {
            schema: NATIVE_REGRADE_SCHEMA,
            outcome,
            trials,
            rejections,
        })
    }
}

/// One sealed bundle artifact read as JSON, fail-closed on any drift.
fn bundle_json(bundle_root: &Path, key: &str) -> Result<serde_json::Value, String> {
    let bytes = std::fs::read(bundle_root.join("artifacts").join(key))
        .map_err(|error| format!("sealed artifact {key} unreadable: {error}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("sealed artifact {key} invalid: {error}"))
}

/// The adapter key of one sealed experiment document.
fn sealed_adapter_key(experiment: &serde_json::Value) -> Result<String, String> {
    let benchmark = experiment
        .get("benchmark")
        .ok_or("sealed experiment carries no benchmark")?;
    let name = benchmark
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let revision = benchmark
        .get("revision")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    match (name, revision) {
        ("terminal-bench", "2.1") => Ok("terminal-bench-2.1".to_owned()),
        ("terminal-bench", "3.0") => Ok("terminal-bench-3.0".to_owned()),
        ("deepswe", "v1.1") => Ok("deepswe-v1.1".to_owned()),
        _ => Err(format!("unsupported benchmark {name:?}/{revision:?}")),
    }
}

/// Recursively copies one directory or file, preserving relative shape.
fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        std::fs::create_dir_all(target)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        }
        Ok(())
    } else if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
        std::fs::copy(source, target)?;
        Ok(())
    } else {
        std::fs::copy(source, target)?;
        Ok(())
    }
}

/// Collects every file under `root` with its path relative to `base`.
fn collect_files(base: &Path, root: &Path, files: &mut Vec<(String, PathBuf)>) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(base, &path, files);
            } else if let Ok(relative) = path.strip_prefix(base) {
                files.push((relative.to_string_lossy().into_owned(), path));
            }
        }
    }
}

/// The newest job directory (by sorted name) holding a `result.json`.
fn newest_job(trace_root: &Path) -> Option<PathBuf> {
    let jobs = trace_root.join("jobs");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&jobs)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("result.json").is_file())
        .collect();
    names.sort();
    names.pop()
}

/// The credential-shaped environment key denylist: the regrade spawn
/// projection fails closed instead of passing any credential material
/// into the fresh verifier environment.
const CREDENTIAL_ENV_MARKERS: [&str; 5] = ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"];

/// Fresh-grades one sealed trial. Every typed failure is a rejection
/// string; the caller keeps the trial visible.
fn regrade_one(
    run_root: &Path,
    material: &crate::runner::material::NativeMaterial,
    out: &Path,
    trial: &str,
) -> Result<RegradedTrial, String> {
    use crate::benchmark::task_lifecycle::{TaskFamily, VerifierEnvironmentMode};

    // Source evidence first: a tampered or unsealed bundle never regrades.
    let bundle_root = run_root.join("trials").join(trial).join("bundle");
    let receipt = RunBundle::verify(&bundle_root)
        .map_err(|error| format!("bundle-{}", classify(trial, error).kind))?;
    let bundle_identity = receipt.bundle_identity().to_owned();

    // The sealed native evidence: without it this trial is not a native
    // grade (a hermetic fixture trial) and never regrades here.
    if !bundle_root
        .join("artifacts/native/verifier-input.json")
        .is_file()
    {
        return Err("not-native".to_owned());
    }
    let experiment = bundle_json(&bundle_root, "control/experiment.json")?;
    let verifier_input = bundle_json(&bundle_root, "native/verifier-input.json")?;
    let trial_result = bundle_json(&bundle_root, "native/trial-result.json")?;
    let trial_config = bundle_json(&bundle_root, "native/trial-config.json")?;

    let adapter_key = sealed_adapter_key(&experiment)?;
    let benchmark = material
        .benchmark(&adapter_key)
        .map_err(|_| format!("material-missing-benchmark:{adapter_key}"))?;
    let family = if adapter_key == "deepswe-v1.1" {
        TaskFamily::Pier
    } else {
        TaskFamily::Harbor
    };
    let runner = match family {
        TaskFamily::Pier => "pier",
        TaskFamily::Harbor => "harbor",
    };

    // The trial's subject product drives the integration identity.
    let subject = experiment
        .get("trials")
        .and_then(|trials| trials.as_array())
        .and_then(|trials| {
            trials
                .iter()
                .find(|entry| entry.get("id").and_then(|id| id.as_str()) == Some(trial))
        })
        .ok_or("sealed-experiment-missing-trial")?;
    let subject_id = subject
        .get("subject")
        .and_then(|value| value.as_str())
        .ok_or("sealed-trial-missing-subject")?;
    let product = experiment
        .get("subjects")
        .and_then(|subjects| subjects.as_array())
        .and_then(|subjects| {
            subjects.iter().find_map(|entry| {
                if entry.get("id").and_then(|id| id.as_str()) == Some(subject_id) {
                    entry.get("product").and_then(|value| value.as_str())
                } else {
                    None
                }
            })
        })
        .ok_or("sealed-experiment-missing-product")?;
    let agent = material
        .agent(product)
        .map_err(|_| format!("material-missing-agent:{product}"))?;
    let integration = agent
        .integration_for(match family {
            TaskFamily::Pier => "pier",
            TaskFamily::Harbor => "harbor",
        })
        .map_err(|_| format!("material-missing-integration:{product}"))?;
    // The sealed job recorded the exact import path this trial selected;
    // the fresh material must pin the same one.
    let sealed_import_path = trial_config
        .get("agent")
        .and_then(|agent| agent.get("name"))
        .and_then(|value| value.as_str())
        .ok_or("sealed-config-missing-agent")?;
    if sealed_import_path != integration.import_path {
        return Err("material-identity-drift".to_owned());
    }

    // Fresh verifier environment: cached immutable task material from
    // the manifest, parsed under the same family policy as the run.
    let env_root = out.join(trial);
    let task_dir = env_root.join("task-package");
    let collected_root = env_root.join("collected");
    let trace_root = env_root.join("verifier-trace");
    for dir in [&task_dir, &collected_root, &trace_root] {
        std::fs::create_dir_all(dir).map_err(|error| format!("staging-failed:{error}"))?;
    }
    copy_tree(&benchmark.task_package, &task_dir)
        .map_err(|error| format!("task-package-copy-failed:{error}"))?;
    let lifecycle = crate::benchmark::task_lifecycle::parse_task_lifecycle(&task_dir, family)
        .map_err(|error| format!("task-package-invalid:{}", error.token()))?;

    // Offline policy and identity cross-checks against the sealed
    // verifier-input: any drift rejects before any verifier process
    // exists, and a network-requiring verifier never silently relaxes.
    if family == TaskFamily::Pier && lifecycle.verifier_network.as_deref() != Some("no-network") {
        return Err("verifier-network-required".to_owned());
    }
    let sealed_mode = verifier_input
        .get("environment_mode")
        .and_then(|value| value.as_str())
        .unwrap_or("shared");
    let fresh_mode = match lifecycle.environment_mode {
        VerifierEnvironmentMode::Shared => "shared",
        VerifierEnvironmentMode::Separate => "separate",
    };
    if sealed_mode != fresh_mode {
        return Err("lifecycle-drift".to_owned());
    }
    let sealed_base = verifier_input
        .get("base_revision")
        .and_then(|value| value.as_str());
    if sealed_base != lifecycle.base_revision.as_deref() {
        return Err("patch-base-drift".to_owned());
    }
    let sealed_image = verifier_input
        .get("docker_image")
        .and_then(|value| value.as_str());
    if sealed_image != lifecycle.docker_image.as_deref() {
        return Err("environment-image-drift".to_owned());
    }
    // The task identity the sealed trial graded must still resolve from
    // the fresh material's profile.
    let profile_bytes =
        std::fs::read(&benchmark.profile).map_err(|error| format!("profile-unreadable:{error}"))?;
    let material_task_id = match adapter_key.as_str() {
        "terminal-bench-2.1" => {
            crate::benchmark::terminal_bench_21::Tb21Profile::parse(profile_bytes.as_slice())
                .map_err(|error| format!("profile-invalid:{error}"))?
                .task_id
        }
        "terminal-bench-3.0" => {
            crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                .map_err(|error| format!("profile-invalid:{error}"))?
                .task_id
        }
        _ => {
            crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                .map_err(|error| format!("profile-invalid:{error}"))?
                .task_id
        }
    };
    let sealed_task = subject
        .get("task")
        .and_then(|value| value.as_str())
        .ok_or("sealed-trial-missing-task")?;
    if sealed_task != material_task_id {
        return Err("task-identity-drift".to_owned());
    }

    // The saved real Agent artifacts: preserved under the run root's
    // job tree, located through the sealed identity, copied into the
    // fresh environment exactly as collected.
    let agent_name = trial_result
        .get("agent_info")
        .and_then(|info| info.get("name"))
        .and_then(|value| value.as_str())
        .ok_or("sealed-result-missing-agent")?;
    let task_name = trial_result
        .get("task_name")
        .and_then(|value| value.as_str())
        .ok_or("sealed-result-missing-task-name")?;
    let trial_name = trial_result
        .get("trial_name")
        .and_then(|value| value.as_str())
        .ok_or("sealed-result-missing-trial-name")?;
    let sealed_collected: Vec<(String, String, String)> = verifier_input
        .get("collected")
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let source = entry.get("source")?.as_str()?.to_owned();
                    let destination = entry.get("destination")?.as_str()?.to_owned();
                    let sha256 = entry.get("sha256")?.as_str()?.to_owned();
                    Some((source, destination, sha256))
                })
                .collect()
        })
        .unwrap_or_default();
    if sealed_collected.is_empty() {
        return Err("agent-artifacts-missing".to_owned());
    }
    let preserved_root = newest_job(&run_root.join("trials").join(trial).join("verifier-trace"))
        .ok_or("agent-artifacts-missing")?
        .join("trials")
        .join(trial_name)
        .join("artifacts");
    let mut delivered: Vec<serde_json::Value> = Vec::new();
    for (source, destination, sealed_sha256) in &sealed_collected {
        let preserved = preserved_root.join(destination);
        if !preserved.exists() {
            return Err("agent-artifacts-missing".to_owned());
        }
        // Every preserved artifact must still digest to exactly the
        // sealed verifier-input identity: altered Agent bytes reject
        // fail-closed instead of grading substituted evidence.
        let observed = if preserved.is_dir() {
            crate::benchmark::process::collected_digest(&preserved)
                .map_err(|_| "agent-artifacts-tampered".to_owned())?
        } else {
            crate::runner::material::sha256_hex(
                &std::fs::read(&preserved).map_err(|_| "agent-artifacts-missing".to_owned())?,
            )
        };
        if observed != *sealed_sha256 {
            return Err("agent-artifacts-tampered".to_owned());
        }
        // The delivered observation mirrors the integration's own: one
        // entry per preserved file under the collected destination.
        let mut files: Vec<(String, PathBuf)> = Vec::new();
        if preserved.is_dir() {
            collect_files(&preserved, &preserved, &mut files);
        } else {
            files.push((String::new(), preserved.clone()));
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        for (relative, file) in files {
            let bytes = std::fs::read(&file).map_err(|_| "agent-artifacts-missing".to_owned())?;
            let delivered_source = if relative.is_empty() {
                source.trim_end_matches('/').to_owned()
            } else {
                format!("{}/{}", source.trim_end_matches('/'), relative)
            };
            delivered.push(serde_json::json!({
                "source": delivered_source,
                "sha256": crate::runner::material::sha256_hex(&bytes),
            }));
        }
        copy_tree(&preserved, &collected_root.join(destination))
            .map_err(|error| format!("artifact-copy-failed:{error}"))?;
    }

    // The identity the pinned verifier reads in regrade mode: every
    // fact derives from sealed evidence or the admitted material; no
    // credential or provider projection exists on this path.
    let model = experiment
        .get("connections")
        .and_then(|connections| connections.as_array())
        .and_then(|connections| {
            connections.iter().find_map(|entry| {
                if entry.get("id").and_then(|id| id.as_str())
                    == experiment.get("connection_id").and_then(|id| id.as_str())
                {
                    let provider = entry.get("provider")?.as_str()?;
                    let model = entry.get("model")?.as_str()?;
                    Some(format!("{provider}:{model}"))
                } else {
                    None
                }
            })
        })
        .ok_or("sealed-experiment-missing-connection")?;
    let identity = serde_json::json!({
        "schema": "opi-eval-native-regrade-identity/1",
        "trial": trial,
        "runner": runner,
        "family": adapter_key,
        "product": product,
        "task_name": task_name,
        "trial_name": trial_name,
        "agent_name": agent_name,
        "agent_import_path": integration.import_path,
        "model": model,
        "agent_executable_sha256": agent.executable.sha256,
        "environment_mode": fresh_mode,
        "base_revision": lifecycle.base_revision,
        "docker_image": lifecycle.docker_image,
        "verifier_network": lifecycle.verifier_network,
        "declared": lifecycle
            .declared_artifacts
            .iter()
            .map(|artifact| artifact.source.clone())
            .collect::<Vec<_>>(),
        "collected": sealed_collected
            .iter()
            .map(|(source, destination, sha256)| serde_json::json!({
                "source": source,
                "destination": destination,
                "sha256": sha256,
            }))
            .collect::<Vec<_>>(),
        "delivered": delivered,
    });
    let identity_path = env_root.join("identity.json");
    std::fs::write(
        &identity_path,
        serde_json::to_vec(&identity).expect("identity serializes"),
    )
    .map_err(|error| format!("staging-failed:{error}"))?;

    // The closed spawn projection: material verifier environment plus
    // the four regrade variables; anything credential-shaped rejects.
    let mut projected: std::collections::BTreeMap<String, String> = benchmark.verifier_env.clone();
    for key in projected.keys() {
        if CREDENTIAL_ENV_MARKERS
            .iter()
            .any(|marker| key.to_uppercase().contains(marker))
        {
            return Err("verifier-env-credential".to_owned());
        }
    }
    projected.insert("OPI_EVAL_REGRADE".to_owned(), "1".to_owned());
    projected.insert("OPI_EVAL_REGRADE_FAMILY".to_owned(), adapter_key.clone());
    projected.insert(
        "OPI_EVAL_REGRADE_IDENTITY".to_owned(),
        identity_path.to_string_lossy().into_owned(),
    );
    projected.insert(
        "OPI_EVAL_REGRADE_ARTIFACTS".to_owned(),
        collected_root.to_string_lossy().into_owned(),
    );
    let env_record = serde_json::json!({
        "schema": "opi-eval-native-regrade-env/1",
        "projected": projected,
        "cwd": "verifier-trace",
        "credential_free": true,
    });
    std::fs::write(
        env_root.join("env.json"),
        serde_json::to_vec_pretty(&env_record).expect("env record serializes"),
    )
    .map_err(|error| format!("staging-failed:{error}"))?;

    // Spawn the pinned verifier in the fresh trace root. No agent
    // executable, no handoff, and no provider network exist here.
    let mut command = std::process::Command::new(&benchmark.verifier_executable.path);
    command
        .current_dir(&trace_root)
        .env_remove("OPI_EVAL_NATIVE_HANDOFF")
        .env_remove("OPI_EVAL_SCRIPTED_KEY")
        .env_remove("OPENAI_API_KEY");
    for (key, value) in &projected {
        command.env(key, value);
    }
    let status = command
        .status()
        .map_err(|error| format!("verifier-spawn-failed:{error}"))?;
    if !status.success() {
        return Err(format!(
            "fresh-verifier-failed:{}",
            status.code().unwrap_or(-1)
        ));
    }

    // The same binding reader as initial execution validates the fresh
    // job: identity, declared lifecycle, delivered-vs-collected bytes.
    let selection = crate::benchmark::process::NativeSelection {
        kind: crate::benchmark::process::SelectionKind::Integration,
        lifecycle: crate::benchmark::process::TaskLifecycleSelection {
            environment_mode: lifecycle.environment_mode,
            declared: lifecycle.declared_artifacts.clone(),
            base_revision: lifecycle.base_revision.clone(),
            docker_image: lifecycle.docker_image.clone(),
        },
        agent_name: agent_name.to_owned(),
        agent_import_path: integration.import_path.clone(),
        task_name: task_name.to_owned(),
        model,
        reward_metric: "reward".to_owned(),
        module_dir: integration.module_dir.clone(),
        handoff_path: identity_path.clone(),
        stale_job_names: Vec::new(),
    };
    let binding = crate::benchmark::process::import_bound_native_job(&trace_root, &selection)
        .map_err(|error| format!("fresh-binding-rejected:{}", error.token()))?;

    // Persist the derived grading record; the source bundle and its
    // original score are never rewritten.
    let reward = match &binding.reward {
        crate::agent::process::Fact::Known { value, .. } => *value,
        crate::agent::process::Fact::Unknown { .. } => {
            return Err("fresh-binding-rejected:reward-unknown".to_owned());
        }
    };
    let record = serde_json::json!({
        "schema": REGRADE_RECORD_SCHEMA,
        "source": {
            "trial": trial,
            "bundle_identity": bundle_identity,
            "collected": sealed_collected
                .iter()
                .map(|(source, destination, sha256)| serde_json::json!({
                    "source": source,
                    "destination": destination,
                    "sha256": sha256,
                }))
                .collect::<Vec<_>>(),
            "original_result": verifier_input
                .get("original_result")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        },
        "fresh": {
            "verifier_executable_sha256": benchmark.verifier_executable.sha256,
            "task_package_schema": lifecycle.schema_version,
            "lifecycle": {
                "environment_mode": fresh_mode,
                "base_revision": lifecycle.base_revision,
                "docker_image": lifecycle.docker_image,
                "verifier_network": lifecycle.verifier_network,
            },
            "agent_name": binding.agent_name,
            "task_name": binding.task_name,
            "job": binding.job_name.to_string_lossy(),
        },
    });
    let records_dir = out.join("records");
    std::fs::create_dir_all(&records_dir).map_err(|error| format!("staging-failed:{error}"))?;
    let record_rel = format!("records/{trial}.json");
    std::fs::write(
        out.join(&record_rel),
        serde_json::to_vec_pretty(&record).expect("record serializes"),
    )
    .map_err(|error| format!("record-write-failed:{error}"))?;

    Ok(RegradedTrial {
        trial: trial.to_owned(),
        bundle_identity,
        reward,
        record: record_rel,
        job: binding.job_name.to_string_lossy().into_owned(),
        verifier_executable_sha256: benchmark.verifier_executable.sha256.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regrade_report_is_byte_stable_for_identical_inputs() {
        let report = RegradeReport {
            schema: REGRADE_SCHEMA,
            outcome: "verified",
            bundles: vec![VerifiedBundle {
                trial: "trial-opi-1".to_owned(),
                bundle_identity: "abc".to_owned(),
            }],
            failures: Vec::new(),
        };
        let first = serde_json::to_vec(&report).unwrap();
        let second = serde_json::to_vec(&report).unwrap();
        assert_eq!(first, second);
        let value = report.to_json();
        assert_eq!(value["schema"], "opi-eval-regrade-report/1");
        assert_eq!(value["outcome"], "verified");
    }
}

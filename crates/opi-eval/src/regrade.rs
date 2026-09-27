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
//! admitting a distinct verifier-only observation - and
//! persists one derived grading record per trial without rewriting any
//! original score. No Agent executable is spawned, no credential is
//! resolved, and no provider is contacted; a verifier whose task
//! declares a network requirement without a permitted offline
//! equivalent rejects fail-closed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

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
        let metadata = std::fs::symlink_metadata(entry.path())?;
        #[cfg(windows)]
        let reparse_point = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                != 0
        };
        #[cfg(not(windows))]
        let reparse_point = false;
        if metadata.file_type().is_symlink() || reparse_point {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "linked trial entry is not admissible: {}",
                    entry.file_name().to_string_lossy()
                ),
            ));
        }
        if metadata.is_dir()
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
const REGRADE_RECORD_SCHEMA: &str = "opi-eval-regrade-record/2";

/// Identity schema of the regrade summary report on stdout.
const NATIVE_REGRADE_SCHEMA: &str = "opi-eval-native-regrade/2";

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
    /// Identity of the fresh verifier, without an Agent job.
    verifier_id: String,
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

/// Request-level regrade failures before any verifier dispatch.
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
    pub(crate) async fn regrade(
        run_root: &Path,
        material_path: &Path,
        out: &Path,
        cancel: &CancellationToken,
    ) -> Result<NativeRegradeReport, NativeRegradeError> {
        let run_root = std::fs::canonicalize(run_root).map_err(NativeRegradeError::Io)?;
        // A root with no native trial is not regradable under this
        // command: hermetic fixture evidence never enters native grading.
        let trial_ids = read_trial_ids(&run_root).map_err(NativeRegradeError::Io)?;
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
        let mut products = std::collections::BTreeSet::new();
        let mut source_rejections = std::collections::BTreeMap::new();
        for trial in &trial_ids {
            let bundle = run_root.join("trials").join(trial).join("bundle");
            let evidence = (|| {
                RunBundle::verify(&bundle)
                    .map_err(|error| format!("bundle-{}", classify(trial, error).kind))?;
                if !bundle
                    .join("artifacts/native/verifier-input.json")
                    .is_file()
                {
                    return Err("not-native".to_owned());
                }
                let experiment = bundle_json(&bundle, "control/experiment.json")?;
                sealed_trial_product(&experiment, trial)?;
                Ok(experiment)
            })();
            match evidence {
                Ok(experiment) => {
                    // One verified experiment also binds the identities of its
                    // damaged sibling trials; their own seals still reject below.
                    for source_trial in &trial_ids {
                        if let Ok(product) = sealed_trial_product(&experiment, source_trial) {
                            products.insert(product.to_owned());
                        }
                    }
                }
                Err(reason) => {
                    source_rejections.insert(trial.clone(), reason);
                }
            }
        }
        if products.is_empty() {
            return Ok(NativeRegradeReport {
                schema: NATIVE_REGRADE_SCHEMA,
                outcome: "rejected",
                trials: Vec::new(),
                rejections: source_rejections
                    .into_iter()
                    .map(|(trial, reason)| RegradeRejection { trial, reason })
                    .collect(),
            });
        }
        let products: Vec<&str> = products.iter().map(String::as_str).collect();
        let material = crate::runner::material::NativeMaterial::load_for(material_path, &products)
            .map_err(|error| NativeRegradeError::Material(error.to_string()))?;
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(NativeRegradeError::Io)?;
        }
        std::fs::create_dir(out).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                NativeRegradeError::OutputExists(out.to_owned())
            } else {
                NativeRegradeError::Io(error)
            }
        })?;
        let out = std::fs::canonicalize(out).map_err(NativeRegradeError::Io)?;
        let mut trials = Vec::new();
        let mut rejections = Vec::new();
        let mut blocked = false;
        for trial in trial_ids {
            if blocked || cancel.is_cancelled() {
                rejections.push(RegradeRejection {
                    trial,
                    reason: "not-dispatched-after-unsettled-verifier".to_owned(),
                });
                continue;
            }
            if let Some(reason) = source_rejections.remove(&trial) {
                rejections.push(RegradeRejection { trial, reason });
                continue;
            }
            match regrade_one(&run_root, &material, &out, &trial, cancel).await {
                Ok(entry) => trials.push(entry),
                Err(reason) => {
                    blocked = reason == "verifier-cleanup-unconfirmed";
                    rejections.push(RegradeRejection { trial, reason });
                }
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

/// Resolve one trial's product from its verified sealed experiment.
fn sealed_trial_product<'a>(
    experiment: &'a serde_json::Value,
    trial: &str,
) -> Result<&'a str, String> {
    let subject = experiment
        .get("trials")
        .and_then(|value| value.as_array())
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
    experiment
        .get("subjects")
        .and_then(|value| value.as_array())
        .and_then(|subjects| {
            subjects.iter().find_map(|entry| {
                (entry.get("id").and_then(|id| id.as_str()) == Some(subject_id))
                    .then(|| entry.get("product").and_then(|value| value.as_str()))
                    .flatten()
            })
        })
        .ok_or_else(|| "sealed-experiment-missing-product".to_owned())
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
async fn regrade_one(
    run_root: &Path,
    material: &crate::runner::material::NativeMaterial,
    out: &Path,
    trial: &str,
    cancel: &CancellationToken,
) -> Result<RegradedTrial, String> {
    use crate::benchmark::task_lifecycle::{TaskFamily, VerifierEnvironmentMode};
    use crate::process::{ExitState, ProcessSupervisor, SpawnSpec};
    use crate::runner::material::sha256_hex;

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
    let sealed_task = subject
        .get("task")
        .and_then(|value| value.as_str())
        .ok_or("sealed-trial-missing-task")?;
    let task_material = benchmark
        .task(sealed_task)
        .map_err(|_| format!("material-missing-task:{sealed_task}"))?;
    let subject_id = subject
        .get("subject")
        .and_then(|value| value.as_str())
        .ok_or("sealed-trial-missing-subject")?;
    let product = sealed_trial_product(&experiment, trial)?;
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
        .and_then(|agent| {
            agent
                .get("import_path")
                .and_then(|value| value.as_str())
                .or_else(|| agent.get("name").and_then(|value| value.as_str()))
        })
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
    copy_tree(&task_material.task_package, &task_dir)
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
    let profile_bytes = std::fs::read(&task_material.profile.path)
        .map_err(|error| format!("profile-unreadable:{error}"))?;
    let (material_task_id, timeout_secs) = match adapter_key.as_str() {
        "terminal-bench-2.1" => {
            let profile =
                crate::benchmark::terminal_bench_21::Tb21Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| format!("profile-invalid:{error}"))?;
            (profile.task_id, profile.timeout_secs)
        }
        "terminal-bench-3.0" => {
            let profile =
                crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| format!("profile-invalid:{error}"))?;
            (profile.task_id, profile.timeout_secs)
        }
        _ => {
            let profile =
                crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                    .map_err(|error| format!("profile-invalid:{error}"))?;
            (profile.task_id, profile.timeout_secs)
        }
    };
    let sealed_task = subject
        .get("task")
        .and_then(|value| value.as_str())
        .ok_or("sealed-trial-missing-task")?;
    if sealed_task != material_task_id {
        return Err("task-identity-drift".to_owned());
    }
    // This is the source identity recorded by native_integrity_review, not a
    // task-name match against a newly substituted package or grader.
    let task_rows = benchmark
        .tasks
        .iter()
        .map(|(id, task)| {
            serde_json::json!({
                "id": id, "native_task_name": task.native_task_name,
                "profile_sha256": task.profile.sha256,
                "task_package_manifest_sha256": task.task_package_manifest_sha256,
            })
        })
        .collect::<Vec<_>>();
    let task_identity = sha256_hex(
        serde_json::json!({
            "source_commit": benchmark.corpus.source_commit,
            "tasks_tree": benchmark.corpus.tasks_tree, "tasks": task_rows,
        })
        .to_string()
        .as_bytes(),
    );
    if bundle_json(&bundle_root, "control/integrity.json")?["upstream_digest"] != task_identity {
        return Err("task-package-drift".to_owned());
    }

    // The saved real Agent artifacts: preserved under the run root's
    // job tree, located through the sealed identity, copied into the
    // fresh environment exactly as collected.
    let task_name = trial_result
        .get("task_name")
        .and_then(|value| value.as_str())
        .ok_or("sealed-result-missing-task-name")?;
    let trial_name = trial_result
        .get("trial_name")
        .and_then(|value| value.as_str())
        .ok_or("sealed-result-missing-trial-name")?;
    let sealed_collected: Vec<RestoredArtifact> = serde_json::from_value(
        verifier_input
            .get("collected")
            .cloned()
            .ok_or("agent-artifacts-missing")?,
    )
    .map_err(|_| "sealed-artifacts-invalid")?;
    if sealed_collected.is_empty() || lifecycle.declared_artifacts.is_empty() {
        return Err("agent-artifacts-missing".to_owned());
    }
    if !safe_relative(trial_name) || trial_name.contains('/') {
        return Err("sealed-trial-name-invalid".to_owned());
    }
    for declared in &lifecycle.declared_artifacts {
        if !sealed_collected.iter().any(|entry| {
            entry.source.trim_end_matches('/') == declared.source.trim_end_matches('/')
                && entry.destination == artifact_destination(declared, family)
        }) {
            return Err("agent-artifacts-missing".to_owned());
        }
    }
    let preserved_root = newest_job(&run_root.join("trials").join(trial).join("verifier-trace"))
        .ok_or("agent-artifacts-missing")?
        .join(trial_name)
        .join("artifacts");
    for entry in &sealed_collected {
        let RestoredArtifact {
            source,
            destination,
            sha256: sealed_sha256,
        } = entry;
        if !safe_relative(destination)
            || !lifecycle.declared_artifacts.iter().any(|declared| {
                declared.source.trim_end_matches('/') == source.trim_end_matches('/')
                    && destination == artifact_destination(declared, family)
            })
        {
            return Err("agent-artifacts-undeclared".to_owned());
        }
        let preserved = preserved_root.join(destination);
        if !preserved.exists() {
            return Err("agent-artifacts-missing".to_owned());
        }
        // Every preserved artifact must still digest to exactly the
        // sealed verifier-input identity: altered Agent bytes reject
        // fail-closed instead of grading substituted evidence.
        if preserved.is_dir() {
            crate::runner::material::runtime_package_manifest_digest(&preserved)
                .map_err(|_| "agent-artifacts-unsafe")?;
        } else if std::fs::symlink_metadata(&preserved)
            .map_err(|_| "agent-artifacts-missing")?
            .file_type()
            .is_symlink()
            || std::fs::canonicalize(&preserved).map_err(|_| "agent-artifacts-missing")?
                != preserved
        {
            return Err("agent-artifacts-unsafe".to_owned());
        }
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
        copy_tree(&preserved, &collected_root.join(destination))
            .map_err(|error| format!("artifact-copy-failed:{error}"))?;
    }

    let verifier_id = format!(
        "opi-eval-regrade-{}",
        &sha256_hex(env_root.to_string_lossy().as_bytes())[..24]
    );
    let module = integration
        .import_path
        .split(':')
        .next()
        .ok_or("integration-module-missing")?;
    let build_context = task_dir.join(if fresh_mode == "shared" {
        "environment"
    } else {
        "tests"
    });
    // A native prebuilt verifier may have no host-side tests directory.
    let context_digest = if build_context.exists() {
        crate::runner::material::runtime_package_manifest_digest(&build_context)
            .map_err(|_| "verifier-build-context-invalid")?
    } else {
        sha256_hex(b"")
    };
    let request = serde_json::json!({
        "schema": "opi-eval-verifier-request/1",
        "verifier_id": verifier_id,
        "source": {
            "trial": trial, "subject": subject_id, "product": product,
            "bundle_identity": bundle_identity,
            "original_result_sha256": sha256_hex(&std::fs::read(
                bundle_root.join("artifacts/native/trial-result.json")
            ).map_err(|_| "sealed-result-missing")?),
        },
        "family": adapter_key, "runner": runner, "task_name": task_name,
        "task_package": task_dir, "collected_root": collected_root,
        "trace_root": trace_root, "environment_mode": fresh_mode,
        "base_revision": lifecycle.base_revision,
        "task_package_manifest_sha256": task_material.task_package_manifest_sha256,
        "build_context_sha256": context_digest,
        "runner_source_sha256": benchmark.runner_project.as_ref().map(|p| &p.source_manifest_sha256),
        "module_manifest_sha256": integration.module_manifest_sha256,
        "verifier_executable_sha256": benchmark.verifier_executable.sha256,
        "restored": sealed_collected,
        "timeout_seconds": timeout_secs,
        "verifier_timeout_seconds": trial_config["verifier"]["max_timeout_sec"],
    });
    let request_bytes = serde_json::to_vec(&request).expect("request serializes");
    let request_sha256 = sha256_hex(&request_bytes);
    let request_path = env_root.join("request.json");
    std::fs::write(&request_path, &request_bytes)
        .map_err(|error| format!("staging-failed:{error}"))?;

    let mut projected = benchmark.verifier_env.clone();
    for key in projected.keys() {
        if CREDENTIAL_ENV_MARKERS
            .iter()
            .any(|marker| key.to_uppercase().contains(marker))
        {
            return Err("verifier-env-credential".to_owned());
        }
    }
    projected.insert(
        "PYTHONPATH".to_owned(),
        integration.module_dir.to_string_lossy().into_owned(),
    );
    projected.insert("PYTHONDONTWRITEBYTECODE".to_owned(), "1".to_owned());
    projected.insert("UV_OFFLINE".to_owned(), "1".to_owned());
    projected.insert("UV_NO_PROGRESS".to_owned(), "1".to_owned());
    projected.insert("OPI_EVAL_REGRADE".to_owned(), "1".to_owned());
    let spec = SpawnSpec {
        argv: vec![
            benchmark.verifier_executable.path.as_os_str().to_owned(),
            "run".into(),
            "--locked".into(),
            "--offline".into(),
            "python".into(),
            "-m".into(),
            module.into(),
            request_path.as_os_str().to_owned(),
        ],
        cwd: Some(trace_root.clone()),
        env: projected
            .iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
        stdout_cap: 1024 * 1024,
        stderr_cap: 1024 * 1024,
        timeout: std::time::Duration::from_secs(timeout_secs),
    };
    std::fs::write(
        env_root.join("env.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "opi-eval-native-regrade-env/2", "projected": projected,
            "argv": spec.argv, "cwd": "verifier-trace", "credential_free": true,
        }))
        .expect("environment serializes"),
    )
    .map_err(|error| format!("staging-failed:{error}"))?;
    if cancel.is_cancelled() {
        return Err("verifier-cancelled".to_owned());
    }
    let outcome = ProcessSupervisor::run(&spec, cancel).await;
    // Docker effects outlive their client process. Verify exact owned roots even
    // when the native module claims cleanup, and settle before the next trial.
    let cleaned = if matches!(outcome.exit, ExitState::FailedToSpawn { .. }) {
        true
    } else {
        cleanup_verifier(&spec, &build_context, &verifier_id)
            .await
            .is_ok()
    };
    std::fs::write(env_root.join("stdout.log"), &outcome.stdout.bytes)
        .and_then(|()| std::fs::write(env_root.join("stderr.log"), &outcome.stderr.bytes))
        .map_err(|error| format!("capture-write-failed:{error}"))?;
    let process_cleaned = matches!(
        outcome.cleanup,
        crate::process::CleanupEvidence::NotRequired
            | crate::process::CleanupEvidence::TreeTerminated { verified: true, .. }
    );
    std::fs::write(env_root.join("settlement.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "exit": format!("{:?}", outcome.exit), "process_cleanup": format!("{:?}", outcome.cleanup),
        "containers_removed": cleaned, "compose_working_directory": build_context,
        "verifier_id": verifier_id, "stdout_truncated": outcome.stdout.truncated,
        "stderr_truncated": outcome.stderr.truncated,
    })).expect("settlement serializes")).map_err(|_| "verifier-cleanup-unconfirmed")?;
    if !cleaned || !process_cleaned {
        return Err("verifier-cleanup-unconfirmed".to_owned());
    }
    match outcome.exit {
        ExitState::TimedOut => return Err("verifier-timeout".to_owned()),
        ExitState::Cancelled => return Err("verifier-cancelled".to_owned()),
        ExitState::FailedToSpawn { .. } => return Err("verifier-spawn-failed".to_owned()),
        ExitState::Exited { code: 0 } => {}
        ExitState::Exited { code } => return Err(format!("fresh-verifier-failed:{code}")),
    }
    let observation: FreshVerifierObservation = serde_json::from_slice(
        &std::fs::read(trace_root.join("observation.json"))
            .map_err(|_| "fresh-observation-missing")?,
    )
    .map_err(|_| "fresh-observation-invalid")?;
    if observation.schema != "opi-eval-verifier-observation/1"
        || observation.request_sha256 != request_sha256
        || observation.verifier_id != verifier_id
        || observation.restored != sealed_collected
        || observation.build_context_sha256 != context_digest
    {
        return Err("fresh-observation-identity-drift".to_owned());
    }
    if !observation.cleanup_confirmed {
        return Err("verifier-cleanup-unconfirmed".to_owned());
    }
    if let Some(failure) = &observation.failure {
        return Err(format!("fresh-verifier-rejected:{failure}"));
    }
    let image = observation.image.as_ref().ok_or("fresh-image-unobserved")?;
    if image.name.is_empty() || !image.id.strip_prefix("sha256:").is_some_and(is_sha256) {
        return Err("fresh-image-invalid".to_owned());
    }
    let raw = observation
        .raw_result
        .as_ref()
        .ok_or("fresh-result-missing")?;
    if raw.path != "native-result.json" {
        return Err("fresh-result-path-invalid".to_owned());
    }
    let raw_bytes =
        std::fs::read(trace_root.join(&raw.path)).map_err(|_| "fresh-result-missing")?;
    if sha256_hex(&raw_bytes) != raw.sha256 {
        return Err("fresh-result-digest-mismatch".to_owned());
    }
    let native: serde_json::Value =
        serde_json::from_slice(&raw_bytes).map_err(|_| "fresh-result-invalid")?;
    let reward = match native["rewards"]["reward"].as_f64() {
        Some(0.0) => 0,
        Some(1.0) => 1,
        _ => return Err("fresh-reward-not-binary".to_owned()),
    };
    for entry in &sealed_collected {
        let observed =
            crate::benchmark::process::collected_digest(&collected_root.join(&entry.destination))
                .map_err(|_| "restored-artifacts-missing")?;
        if observed != entry.sha256 {
            return Err("restored-artifacts-tampered".to_owned());
        }
    }
    let record = serde_json::json!({
        "schema": REGRADE_RECORD_SCHEMA,
        "source": {"trial": trial, "bundle_identity": bundle_identity,
            "collected": sealed_collected, "original_result_sha256": request["source"]["original_result_sha256"]},
        "request_sha256": request_sha256,
        "fresh": {"verifier_id": verifier_id, "task_name": task_name, "runner": runner,
            "verifier_executable_sha256": benchmark.verifier_executable.sha256,
            "runner_source_sha256": request["runner_source_sha256"],
            "module_manifest_sha256": integration.module_manifest_sha256,
            "task_package_manifest_sha256": task_material.task_package_manifest_sha256,
            "observation": observation, "native_result": native, "reward": reward},
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
        verifier_id,
        verifier_executable_sha256: benchmark.verifier_executable.sha256.clone(),
    })
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RestoredArtifact {
    source: String,
    destination: String,
    sha256: String,
}

/// The pinned runners use different default host layouts: Harbor mirrors
/// sources; Pier uses basenames, with its conventional publish root at `.`.
fn artifact_destination(
    declared: &crate::benchmark::task_lifecycle::DeclaredArtifact,
    family: crate::benchmark::task_lifecycle::TaskFamily,
) -> &str {
    let source = declared.source.trim_end_matches('/');
    let explicit = declared
        .destination
        .as_deref()
        .filter(|path| !path.is_empty());
    if family == crate::benchmark::task_lifecycle::TaskFamily::Pier {
        if source == "/logs/artifacts"
            && explicit.is_none_or(|path| path == "." || path.trim_end_matches('/') == source)
        {
            "."
        } else {
            explicit
                .unwrap_or_else(|| source.rsplit('/').next().unwrap_or(source))
                .trim_matches('/')
        }
    } else {
        explicit.unwrap_or(source).trim_matches('/')
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FreshVerifierObservation {
    schema: String,
    request_sha256: String,
    verifier_id: String,
    restored: Vec<RestoredArtifact>,
    build_context_sha256: String,
    image: Option<VerifierImage>,
    raw_result: Option<VerifierResultReference>,
    cleanup_confirmed: bool,
    failure: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VerifierImage {
    name: String,
    id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VerifierResultReference {
    path: String,
    sha256: String,
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', ':'])
        && Path::new(value)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A cancelled verifier cannot clean its own Docker effects. Inspect only
/// identity labels, remove complete owned IDs, then independently query again.
async fn cleanup_verifier(
    spec: &crate::process::SpawnSpec,
    directory: &Path,
    verifier_id: &str,
) -> Result<(), ()> {
    use crate::process::{CleanupEvidence, ExitState, ProcessSupervisor};
    async fn docker(spec: &crate::process::SpawnSpec, args: &[&str]) -> Result<Vec<u8>, ()> {
        let mut cleanup = spec.clone();
        cleanup.argv = std::iter::once("docker".into())
            .chain(args.iter().map(Into::into))
            .collect();
        cleanup.env.retain(|key, _| {
            matches!(
                key.to_str(),
                Some(
                    "PATH"
                        | "HOME"
                        | "USERPROFILE"
                        | "SystemRoot"
                        | "DOCKER_HOST"
                        | "DOCKER_CONTEXT"
                        | "DOCKER_CONFIG"
                        | "DOCKER_TLS_VERIFY"
                        | "DOCKER_CERT_PATH"
                )
            )
        });
        cleanup.timeout = std::time::Duration::from_secs(15);
        cleanup.stdout_cap = 64 * 1024;
        cleanup.stderr_cap = 1024;
        let result = ProcessSupervisor::run(&cleanup, &CancellationToken::new()).await;
        if result.exit != (ExitState::Exited { code: 0 })
            || result.stdout.truncated
            || !matches!(
                result.cleanup,
                CleanupEvidence::NotRequired
                    | CleanupEvidence::TreeTerminated { verified: true, .. }
            )
        {
            return Err(());
        }
        Ok(result.stdout.bytes)
    }
    let label = format!(
        "label=com.docker.compose.project.working_dir={}",
        directory.display()
    );
    let query = ["ps", "--all", "--quiet", "--no-trunc", "--filter", &label];
    let result = async {
        let bytes = docker(spec, &query).await?;
        let ids = std::str::from_utf8(&bytes)
            .map_err(|_| ())?
            .lines()
            .collect::<Vec<_>>();
        if ids.len() > 64 || ids.iter().any(|id| !is_sha256(id)) {
            return Err(());
        }
        for id in &ids {
            let bytes = docker(spec, &["inspect", "--type", "container", "--format",
                r#"{"id":{{json .Id}},"working_dir":{{json (index .Config.Labels "com.docker.compose.project.working_dir")}},"project":{{json (index .Config.Labels "com.docker.compose.project")}}}"#, id]).await?;
            let identity: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
            if identity["id"] != *id
                || identity["working_dir"] != directory.to_string_lossy().as_ref()
                || identity["project"] != verifier_id
            {
                return Err(());
            }
        }
        for id in ids {
            docker(spec, &["rm", "--force", "--volumes", id]).await?;
        }
        if !docker(spec, &query).await?.is_empty() {
            return Err(());
        }
        Ok(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), result)
        .await
        .map_err(|_| ())?
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

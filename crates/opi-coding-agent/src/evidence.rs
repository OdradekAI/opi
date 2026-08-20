//! Reference Product evidence adapter and manifest assembly.
//!
//! The Agent Core evidence contract ([`opi_agent::evidence`]) is storage-
//! neutral: it owns identities, health, the sink lifecycle, and the
//! [`opi_agent::evidence::FinalizedManifest`] value types. This module supplies
//! the Reference Product's side of that contract:
//!
//! - [`FileEvidenceSink`]: the product file adapter. It implements the
//!   [`opi_agent::evidence::EvidenceSink`] lifecycle (setup creates the capture
//!   file, emit appends one JSONL record, finalize_run writes the manifest) and
//!   [`opi_agent::evidence::EvidenceRecorder`] so the harness can assemble the
//!   manifest from the recorded dynamic facts. File paths, on-disk layout, and
//!   retention are product facts that do not enter Agent Core.
//! - [`EvidenceCapture`]: the per-run binding facts (runtime-input binding,
//!   resolved configuration identity, effective policy digest) plus the
//!   recorder handle. A long-lived harness rebinds these facts before each run.
//! - [`build_finalized_manifest`]: combines the capture's static facts with the
//!   recorder's dynamic facts (call-graph correlation, route) and the run's
//!   terminal outcome/usage into one strict manifest.
//!
//! Redaction is the producer's responsibility: structured evidence
//! values cross the sink already redacted via
//! [`opi_agent::evidence::RedactedValue`]; this adapter never makes raw input
//! safe and only durably stores already-redacted values.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use opi_agent::evidence::{
    ArtifactReference, AssemblyIdentity, CallKind, ConfigIdentity, ContentDigest, EnvironmentFacts,
    EvidenceCompleteness, EvidenceError, EvidencePayload, EvidenceRecord, EvidenceRecorder,
    EvidenceRunObservation, EvidenceSink, ExecutionTrigger, FinalizedManifest, ManifestCandidate,
    ManifestCorrelation, Measurement, MeasurementOrigin, PlatformIdentity, ProviderInvocationFacts,
    ProviderNotApplicableReason, RuntimeInputBinding, SessionBinding, TerminalOutcome,
    UnknownReason, UsageFacts, UserPolicyFacts,
};

/// Reference Product assembly identity for CLI-originated runs.
pub static CLI_ASSEMBLY: std::sync::LazyLock<AssemblyIdentity> =
    std::sync::LazyLock::new(|| AssemblyIdentity::new("opi.cli").expect("valid product identity"));
/// Reference Product assembly identity for SDK-originated runs.
pub static SDK_ASSEMBLY: std::sync::LazyLock<AssemblyIdentity> =
    std::sync::LazyLock::new(|| AssemblyIdentity::new("opi.sdk").expect("valid product identity"));
/// Reference Product assembly identity for RPC-originated runs.
pub static RPC_ASSEMBLY: std::sync::LazyLock<AssemblyIdentity> =
    std::sync::LazyLock::new(|| AssemblyIdentity::new("opi.rpc").expect("valid product identity"));

/// One JSONL record file written by [`FileEvidenceSink::setup`].
const RECORDS_FILE: &str = "evidence.jsonl";
/// The finalized manifest file written by [`FileEvidenceSink::finalize_run`].
const MANIFEST_FILE: &str = "manifest.json";

/// Reference Product file adapter for the Agent Core evidence lifecycle.
///
/// The configured path is a capture root. Every [`EvidenceSink::setup`] creates
/// one unique child directory containing `evidence.jsonl` and, only after
/// durable record completion, an atomically published `manifest.json`. A
/// finalized child is never truncated or replaced. The sink keeps an in-memory
/// mirror of the current run so the harness can assemble its manifest without
/// re-reading the file.
pub struct FileEvidenceSink {
    root: PathBuf,
    next_run: AtomicU64,
    state: Mutex<FileEvidenceState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileEvidencePhase {
    Ready,
    SetupFailed,
    Active,
    FailedActive,
    Completed,
    Abandoned,
}

struct FileEvidenceState {
    phase: FileEvidencePhase,
    active_dir: Option<PathBuf>,
    binding: Option<RuntimeInputBinding>,
    records: Vec<EvidenceRecord>,
    artifacts: Vec<ArtifactReference>,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    manifest: Option<FinalizedManifest>,
    failure: Option<EvidenceError>,
    completed_dirs: Vec<PathBuf>,
}

impl FileEvidenceSink {
    /// Configure a sink that writes into `dir`. No file is touched until
    /// [`EvidenceSink::setup`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            next_run: AtomicU64::new(1),
            state: Mutex::new(FileEvidenceState {
                phase: FileEvidencePhase::Ready,
                active_dir: None,
                binding: None,
                records: Vec::new(),
                artifacts: Vec::new(),
                writer: None,
                manifest: None,
                failure: None,
                completed_dirs: Vec::new(),
            }),
        }
    }

    /// The configured capture root.
    pub fn dir(&self) -> &Path {
        &self.root
    }

    /// Finalized immutable run directories in setup order.
    pub fn completed_run_dirs(&self) -> Vec<PathBuf> {
        Self::lock(&self.state).completed_dirs.clone()
    }

    fn lock<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn mark_failure(state: &mut FileEvidenceState, error: EvidenceError) {
        if state.failure.is_none() {
            state.failure = Some(error);
        }
        if state.phase == FileEvidencePhase::Active {
            state.phase = FileEvidencePhase::FailedActive;
        }
    }

    fn allocate_run_dir(&self) -> Result<PathBuf, EvidenceError> {
        std::fs::create_dir_all(&self.root).map_err(|e| EvidenceError::Setup {
            detail: format!("evidence root {}: {e}", self.root.display()),
        })?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..128 {
            let sequence = self.next_run.fetch_add(1, Ordering::SeqCst);
            let dir = self
                .root
                .join(format!("run-{timestamp}-{}-{sequence}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(dir),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(EvidenceError::Setup {
                        detail: format!("evidence run dir {}: {error}", dir.display()),
                    });
                }
            }
        }
        Err(EvidenceError::Setup {
            detail: "could not allocate a unique evidence run directory".to_owned(),
        })
    }
}

impl EvidenceSink for FileEvidenceSink {
    fn setup(&self, binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        let mut state = Self::lock(&self.state);
        if matches!(
            state.phase,
            FileEvidencePhase::Active | FileEvidencePhase::FailedActive
        ) {
            let error = EvidenceError::Setup {
                detail: "previous evidence run has not been finalized".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        state.failure = None;
        state.active_dir = None;
        state.binding = None;
        state.records.clear();
        state.artifacts.clear();
        state.writer = None;
        state.manifest = None;
        state.phase = FileEvidencePhase::Ready;
        let dir = match self.allocate_run_dir() {
            Ok(dir) => dir,
            Err(error) => {
                state.phase = FileEvidencePhase::SetupFailed;
                Self::mark_failure(&mut state, error.clone());
                return Err(error);
            }
        };
        let records_path = dir.join(RECORDS_FILE);
        let file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&records_path)
            .map_err(|e| EvidenceError::Setup {
                detail: format!("evidence file {}: {e}", records_path.display()),
            }) {
            Ok(file) => file,
            Err(error) => {
                let _ = std::fs::remove_dir(&dir);
                state.phase = FileEvidencePhase::SetupFailed;
                Self::mark_failure(&mut state, error.clone());
                return Err(error);
            }
        };
        state.active_dir = Some(dir);
        state.binding = Some(binding.clone());
        state.writer = Some(std::io::BufWriter::new(file));
        state.phase = FileEvidencePhase::Active;
        Ok(())
    }

    fn emit(&self, record: &EvidenceRecord) -> Result<(), EvidenceError> {
        let mut state = Self::lock(&self.state);
        if state.phase == FileEvidencePhase::Completed {
            return Err(EvidenceError::Emission {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if !matches!(
            state.phase,
            FileEvidencePhase::Active | FileEvidencePhase::FailedActive
        ) {
            let error = EvidenceError::Emission {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        if record.validate_kind_payload().is_err() {
            let error = EvidenceError::Emission {
                detail: "evidence record kind does not match its typed payload".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        let line = match serde_json::to_string(record) {
            Ok(line) => line,
            Err(error) => {
                let error = EvidenceError::Emission {
                    detail: error.to_string(),
                };
                Self::mark_failure(&mut state, error.clone());
                return Err(error);
            }
        };
        let Some(writer) = state.writer.as_mut() else {
            let error = EvidenceError::Emission {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        };
        if let Err(e) = writer
            .write_all(line.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .and_then(|()| writer.flush())
        {
            let error = EvidenceError::Emission {
                detail: e.to_string(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        state.records.push(record.clone());
        Ok(())
    }

    fn finalize_artifact(&self, artifact: &ArtifactReference) -> Result<(), EvidenceError> {
        // Artifact references are carried inside emitted records / the manifest;
        // the file adapter has no separate artifact store. References never
        // contain payloads.
        let mut state = Self::lock(&self.state);
        if state.phase == FileEvidencePhase::Completed {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if !matches!(
            state.phase,
            FileEvidencePhase::Active | FileEvidencePhase::FailedActive
        ) || state.binding.is_none()
            || state.writer.is_none()
        {
            let error = EvidenceError::Finalization {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        state.artifacts.push(artifact.clone());
        Ok(())
    }

    fn finalize_run(&self, manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        let mut state = Self::lock(&self.state);
        if state.phase == FileEvidencePhase::Completed {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if state.failure.is_some() {
            let error = EvidenceError::Finalization {
                detail: "evidence lifecycle is incomplete".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        if state.phase != FileEvidencePhase::Active {
            let error = EvidenceError::Finalization {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        let Some(binding) = state.binding.as_ref() else {
            let error = EvidenceError::Finalization {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        };
        if let Err(error) = manifest.validate_observation(EvidenceRunObservation::new(
            binding,
            &state.records,
            &state.artifacts,
        )) {
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        let Some(writer) = state.writer.as_mut() else {
            let error = EvidenceError::Finalization {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        };
        if let Err(error) = writer.flush().and_then(|()| writer.get_ref().sync_all()) {
            let evidence_error = EvidenceError::Finalization {
                detail: format!("evidence record durability: {error}"),
            };
            Self::mark_failure(&mut state, evidence_error.clone());
            return Err(evidence_error);
        }
        let json = match serde_json::to_string_pretty(manifest) {
            Ok(json) => json,
            Err(error) => {
                let error = EvidenceError::Finalization {
                    detail: error.to_string(),
                };
                Self::mark_failure(&mut state, error.clone());
                return Err(error);
            }
        };
        let Some(dir) = state.active_dir.clone() else {
            let error = EvidenceError::Finalization {
                detail: "evidence sink used before setup".to_owned(),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        };
        let manifest_path = dir.join(MANIFEST_FILE);
        let temporary_path = dir.join(".manifest.json.tmp");
        let publish = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary_path)?;
            file.write_all(json.as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            std::fs::rename(&temporary_path, &manifest_path)?;
            Ok(())
        })();
        if let Err(e) = publish {
            let error = EvidenceError::Finalization {
                detail: format!("{}: {e}", manifest_path.display()),
            };
            Self::mark_failure(&mut state, error.clone());
            return Err(error);
        }
        state.writer = None;
        state.manifest = Some(manifest.clone());
        state.phase = FileEvidencePhase::Completed;
        state.completed_dirs.push(dir);
        Ok(())
    }

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        let mut state = Self::lock(&self.state);
        if !matches!(
            state.phase,
            FileEvidencePhase::Active | FileEvidencePhase::FailedActive
        ) {
            let error = EvidenceError::Finalization {
                detail: "no active evidence run to abandon".to_owned(),
            };
            if state.phase != FileEvidencePhase::Completed {
                Self::mark_failure(&mut state, error.clone());
            }
            return Err(error);
        }
        state.writer = None;
        if let Some(dir) = state.active_dir.as_ref() {
            let temporary_path = dir.join(".manifest.json.tmp");
            if let Err(error) = std::fs::remove_file(&temporary_path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                let error = EvidenceError::Finalization {
                    detail: format!("evidence temporary manifest cleanup: {error}"),
                };
                Self::mark_failure(&mut state, error.clone());
                return Err(error);
            }
        }
        state.manifest = None;
        state.phase = FileEvidencePhase::Abandoned;
        Ok(())
    }
}

impl EvidenceRecorder for FileEvidenceSink {
    fn records(&self) -> Vec<EvidenceRecord> {
        Self::lock(&self.state).records.clone()
    }
    fn has_failure(&self) -> bool {
        Self::lock(&self.state).failure.is_some()
    }
    fn completed_manifest(&self) -> Option<FinalizedManifest> {
        let state = Self::lock(&self.state);
        if state.phase != FileEvidencePhase::Completed {
            return None;
        }
        state.manifest.clone()
    }
}

/// Immutable run-binding static facts held by the harness for one evidence run.
/// Combined with the recorder's dynamic facts by [`build_finalized_manifest`].
pub struct EvidenceCapture {
    /// The recording sink (file adapter in production, in-memory oracle in
    /// tests). Also bound to the Agent via `set_evidence_sink` so the loop
    /// emits through it.
    pub recorder: Arc<dyn EvidenceRecorder>,
    /// Direct-assembly origin retained so a long-lived harness can derive a
    /// fresh binding from current runtime state before every run.
    pub source: AssemblyIdentity,
    /// The direct runtime-input binding (never ActiveSnapshot for a direct run).
    pub binding: RuntimeInputBinding,
    /// Resolved harness/runtime/adapter/material configuration identity.
    pub config: ConfigIdentity,
    /// Effective user-policy digest + capability.
    pub policy: UserPolicyFacts,
    /// Exact resolved system instruction identity frozen at setup.
    pub system_digest: Option<ContentDigest>,
    /// Exact provider-visible trusted tool definitions frozen at setup.
    pub tool_schema_digests: Vec<ContentDigest>,
    /// Configured run budget frozen at setup.
    pub budget: Measurement,
}

/// Builder input handed to `CodingHarness::builder().evidence(..)`: the recording
/// sink plus the direct-assembly origin label.
pub struct EvidenceBuilderConfig {
    /// The recording sink (also the Agent's evidence sink).
    pub recorder: Arc<dyn EvidenceRecorder>,
    /// Where the direct assembly originated (CLI / SDK / RPC).
    pub source: AssemblyIdentity,
}

impl EvidenceCapture {
    /// Assemble the capture from the recorder, the assembly source, the
    /// effective-policy digest, the resolved configuration identity, and the
    /// material runtime inputs (system prompt, model selection, resolved config)
    /// whose digest addresses the runtime-input binding. Direct assembly always
    /// binds [`RuntimeInputBinding::DirectRuntimeInput`]; it never fabricates an
    /// `ActiveSnapshot`.
    pub fn new(
        recorder: Arc<dyn EvidenceRecorder>,
        source: AssemblyIdentity,
        policy_digest: ContentDigest,
        config: ConfigIdentity,
        material_inputs: &str,
    ) -> Self {
        let binding_digest =
            runtime_input_digest(&source, &policy_digest, &config, material_inputs);
        Self {
            recorder,
            binding: RuntimeInputBinding::direct(binding_digest, source.clone()),
            source,
            config,
            policy: UserPolicyFacts {
                policy_digest,
                capability: None,
                permission_ref: None,
                permission_scope: None,
                scoped_grant_ref: None,
            },
            system_digest: None,
            tool_schema_digests: Vec::new(),
            budget: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
        }
    }

    /// Replace the per-run configuration and direct runtime-input binding while
    /// retaining the recorder, assembly source, and effective policy identity.
    pub fn rebind(
        &mut self,
        config: ConfigIdentity,
        material_inputs: &str,
        system_digest: Option<ContentDigest>,
        tool_schema_digests: Vec<ContentDigest>,
        budget: Measurement,
    ) {
        let binding_digest = runtime_input_digest(
            &self.source,
            &self.policy.policy_digest,
            &config,
            material_inputs,
        );
        self.binding = RuntimeInputBinding::direct(binding_digest, self.source.clone());
        self.config = config;
        self.system_digest = system_digest;
        self.tool_schema_digests = tool_schema_digests;
        self.budget = budget;
    }
}

/// The runtime-input binding digest covers the resolved material runtime inputs:
/// the assembly source, the effective policy, the resolved configuration
/// identity, and the material input rendering.
fn runtime_input_digest(
    source: &AssemblyIdentity,
    policy_digest: &ContentDigest,
    config: &ConfigIdentity,
    material_inputs: &str,
) -> ContentDigest {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(source.as_str().as_bytes());
    hasher.update(b"\npolicy:");
    hasher.update(digest_bytes(policy_digest).as_bytes());
    hasher.update(b"\nharness:");
    hasher.update(digest_bytes(&config.harness_digest).as_bytes());
    hasher.update(b"\nruntime:");
    hasher.update(digest_bytes(&config.runtime_digest).as_bytes());
    hasher.update(b"\nadapter:");
    hasher.update(digest_bytes(&config.adapter_digest).as_bytes());
    hasher.update(b"\nmaterial-config:");
    hasher.update(digest_bytes(&config.material_digest).as_bytes());
    hasher.update(b"\nmaterial:");
    hasher.update(material_inputs.as_bytes());
    ContentDigest::from_hex(hex::encode(hasher.finalize()))
        .expect("SHA-256 encoder must produce canonical lowercase hex")
}

fn digest_bytes(digest: &ContentDigest) -> String {
    digest.as_hex().to_owned()
}

/// Terminal dynamic facts a run observed, supplied by the harness at finalization.
pub struct RunDynamicFacts {
    /// Terminal run outcome (success / cancelled / failed / ...).
    pub outcome: TerminalOutcome,
    /// Aggregated provider usage for the run.
    pub usage: UsageFacts,
    /// Required session branch or explicit non-session binding.
    pub session: SessionBinding,
    /// Digest of the prompt that drove the run.
    pub prompt_digest: ContentDigest,
    /// Exact product-owned execution trigger.
    pub trigger: ExecutionTrigger,
}

/// Assemble the strict [`FinalizedManifest`] from the capture's static facts,
/// the recorder's ordered records (call-graph correlation + route), and the
/// run's terminal dynamic facts. Validation occurs before this function
/// returns; the caller passes the result to the recorder's `finalize_run`.
pub fn build_finalized_manifest(
    capture: &EvidenceCapture,
    records: &[EvidenceRecord],
    dynamic: RunDynamicFacts,
) -> Result<FinalizedManifest, EvidenceError> {
    let correlation = terminal_correlation(records);
    let provider = extract_provider_facts(records, &dynamic.outcome)?;
    let completeness = if capture_recorder_failed(capture) {
        EvidenceCompleteness::Incomplete
    } else {
        EvidenceCompleteness::Complete
    };
    let candidate = ManifestCandidate {
        correlation,
        outcome: dynamic.outcome,
        session: dynamic.session,
        binding: capture.binding.clone(),
        config: capture.config.clone(),
        provider,
        policy: capture.policy.clone(),
        input_identity: opi_agent::evidence::InputIdentity {
            prompt_digest: dynamic.prompt_digest,
            system_digest: capture.system_digest.clone(),
            tool_schema_digests: capture.tool_schema_digests.clone(),
        },
        environment: EnvironmentFacts {
            budget: capture.budget,
            trigger: dynamic.trigger,
            time: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            platform: current_platform(),
        },
        usage: dynamic.usage,
        // The Reference Product emits zero artifact references: the file
        // adapter writes evidence.jsonl + manifest.json, but no separate
        // tool/provider artifact store exists to reference. The `artifacts`
        // field is therefore empty; "emits only finalized classified artifact
        // references" is a constraint on producers (an empty set satisfies it
        // vacuously), not a requirement to emit. This adapter does not produce
        // `ArtifactRole`/`SensitivityClassification`/`FinalizationState` facts.
        artifacts: Vec::new(),
        completeness,
    };
    candidate.validate(EvidenceRunObservation::new(&capture.binding, records, &[]))
}

fn capture_recorder_failed(capture: &EvidenceCapture) -> bool {
    capture.recorder.has_failure()
}

fn current_platform() -> PlatformIdentity {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    PlatformIdentity::new(format!("{os}-{}", std::env::consts::ARCH))
}

/// Derive the terminal manifest correlation from the ordered records: the run
/// identity is stable across the run; the terminal turn/call/parent/sequence
/// come from the last emitted record.
fn terminal_correlation(records: &[EvidenceRecord]) -> ManifestCorrelation {
    // The caller only builds a manifest for a run that emitted records (a run
    // with no provider call produces no evidence graph and no manifest), so a
    // first/last record always exists here.
    let first = records
        .first()
        .expect("manifest correlation requires at least one record");
    let last = records
        .last()
        .expect("manifest correlation requires at least one record");
    ManifestCorrelation {
        run: first.run,
        turn: last.turn,
        call: Some(last.call),
        parent: last.parent,
        sequence: last.sequence,
    }
}

/// Consume the last typed provider record directly. Malformed or legacy
/// structured payloads fail closed rather than defaulting route/provenance.
/// A compaction-only graph and a run cancelled before provider preparation
/// carry explicit not-applicable reasons.
fn extract_provider_facts(
    records: &[EvidenceRecord],
    outcome: &TerminalOutcome,
) -> Result<ProviderInvocationFacts, EvidenceError> {
    for record in records {
        record.validate_kind_payload()?;
    }
    if let Some(facts) = records
        .iter()
        .rev()
        .find_map(|record| match &record.payload {
            EvidencePayload::Provider(facts) => Some(facts),
            _ => None,
        })
    {
        return Ok(ProviderInvocationFacts::applicable(
            facts.route.clone(),
            facts.provenance.clone(),
        ));
    }
    if records
        .iter()
        .all(|record| record.kind == CallKind::Compaction)
        && !records.is_empty()
    {
        return Ok(ProviderInvocationFacts::not_applicable(
            ProviderNotApplicableReason::StandaloneCompaction,
        ));
    }
    if matches!(outcome, TerminalOutcome::Cancelled) && !records.is_empty() {
        return Ok(ProviderInvocationFacts::not_applicable(
            ProviderNotApplicableReason::CancelledBeforeProvider,
        ));
    }
    Err(EvidenceError::Finalization {
        detail: "evidence graph has no typed provider facts and is not standalone compaction"
            .to_owned(),
    })
}

/// Convert an aggregated provider token usage into evidence usage facts.
/// Unknown stays distinct from a measured zero.
pub fn usage_facts(input_tokens: Option<u64>, output_tokens: Option<u64>) -> UsageFacts {
    UsageFacts {
        input_tokens: measurement(input_tokens),
        output_tokens: measurement(output_tokens),
    }
}

fn measurement(value: Option<u64>) -> Measurement {
    match value {
        Some(v) => Measurement::Known {
            value: v,
            origin: MeasurementOrigin::ProviderReported,
        },
        None => Measurement::Unknown {
            reason: UnknownReason::NotReported,
        },
    }
}

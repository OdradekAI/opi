//! Phase 17 task 17.7 — Reference Product evidence adapter and manifest assembly.
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
//! Redaction is the producer's responsibility (P17-EVD-005): structured evidence
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
    ArtifactReference, AuthProvenanceSource, CallKind, ConfigIdentity, ContentDigest,
    EnvironmentFacts, EvidenceError, EvidenceRecord, EvidenceRecorder, EvidenceSink,
    FinalizedManifest, ManifestCorrelation, Measurement, MeasurementOrigin, PlatformIdentity,
    ProvenanceFacts, RouteFacts, RouteSelection, RuntimeInputBinding, TerminalOutcome,
    UnknownReason, UsageFacts, UserPolicyFacts,
};
use serde::Deserialize;

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
    active_dir: Mutex<Option<PathBuf>>,
    records: Mutex<Vec<EvidenceRecord>>,
    writer: Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    finalized: Mutex<bool>,
    manifest: Mutex<Option<FinalizedManifest>>,
    failure: Mutex<Option<EvidenceError>>,
    completed_dirs: Mutex<Vec<PathBuf>>,
}

impl FileEvidenceSink {
    /// Configure a sink that writes into `dir`. No file is touched until
    /// [`EvidenceSink::setup`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            next_run: AtomicU64::new(1),
            active_dir: Mutex::new(None),
            records: Mutex::new(Vec::new()),
            writer: Mutex::new(None),
            finalized: Mutex::new(false),
            manifest: Mutex::new(None),
            failure: Mutex::new(None),
            completed_dirs: Mutex::new(Vec::new()),
        }
    }

    /// The configured capture root.
    pub fn dir(&self) -> &Path {
        &self.root
    }

    /// Finalized immutable run directories in setup order.
    pub fn completed_run_dirs(&self) -> Vec<PathBuf> {
        Self::lock(&self.completed_dirs).clone()
    }

    fn lock<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn mark_failure(&self, error: EvidenceError) {
        let mut failure = Self::lock(&self.failure);
        if failure.is_none() {
            *failure = Some(error);
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
    fn setup(&self, _binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        if Self::lock(&self.writer).is_some() {
            return Err(EvidenceError::Setup {
                detail: "previous evidence run has not been finalized".to_owned(),
            });
        }
        *Self::lock(&self.failure) = None;
        let dir = self.allocate_run_dir()?;
        let records_path = dir.join(RECORDS_FILE);
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&records_path)
            .map_err(|e| EvidenceError::Setup {
                detail: format!("evidence file {}: {e}", records_path.display()),
            })?;
        *Self::lock(&self.active_dir) = Some(dir);
        *Self::lock(&self.writer) = Some(std::io::BufWriter::new(file));
        Self::lock(&self.records).clear();
        *Self::lock(&self.finalized) = false;
        *Self::lock(&self.manifest) = None;
        Ok(())
    }

    fn emit(&self, record: &EvidenceRecord) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Emission {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        let line = serde_json::to_string(record).map_err(|e| EvidenceError::Emission {
            detail: e.to_string(),
        })?;
        let mut writer_guard = Self::lock(&self.writer);
        let Some(writer) = writer_guard.as_mut() else {
            return Err(EvidenceError::Emission {
                detail: "evidence sink used before setup".to_owned(),
            });
        };
        if let Err(e) = writer
            .write_all(line.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .and_then(|()| writer.flush())
        {
            self.mark_failure(EvidenceError::Emission {
                detail: e.to_string(),
            });
            return Err(EvidenceError::Emission {
                detail: e.to_string(),
            });
        }
        drop(writer_guard);
        Self::lock(&self.records).push(record.clone());
        Ok(())
    }

    fn finalize_artifact(&self, _artifact: &ArtifactReference) -> Result<(), EvidenceError> {
        // Artifact references are carried inside emitted records / the manifest;
        // the file adapter has no separate artifact store (P17-EVD-005: payload
        // references, not payloads).
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        Ok(())
    }

    fn finalize_run(&self, manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        let mut writer =
            Self::lock(&self.writer)
                .take()
                .ok_or_else(|| EvidenceError::Finalization {
                    detail: "evidence sink used before setup".to_owned(),
                })?;
        if let Err(error) = writer.flush().and_then(|()| writer.get_ref().sync_all()) {
            let evidence_error = EvidenceError::Finalization {
                detail: format!("evidence record durability: {error}"),
            };
            self.mark_failure(EvidenceError::Finalization {
                detail: format!("evidence record durability: {error}"),
            });
            return Err(evidence_error);
        }
        let json =
            serde_json::to_string_pretty(manifest).map_err(|e| EvidenceError::Finalization {
                detail: e.to_string(),
            })?;
        let dir =
            Self::lock(&self.active_dir)
                .clone()
                .ok_or_else(|| EvidenceError::Finalization {
                    detail: "evidence sink used before setup".to_owned(),
                })?;
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
            self.mark_failure(EvidenceError::Finalization {
                detail: format!("{}: {e}", manifest_path.display()),
            });
            return Err(EvidenceError::Finalization {
                detail: format!("{}: {e}", manifest_path.display()),
            });
        }
        *Self::lock(&self.manifest) = Some(manifest.clone());
        *Self::lock(&self.finalized) = true;
        Self::lock(&self.completed_dirs).push(dir);
        Ok(())
    }
}

impl EvidenceRecorder for FileEvidenceSink {
    fn records(&self) -> Vec<EvidenceRecord> {
        Self::lock(&self.records).clone()
    }
    fn has_failure(&self) -> bool {
        Self::lock(&self.failure).is_some()
    }
    fn completed_manifest(&self) -> Option<FinalizedManifest> {
        if Self::lock(&self.failure).is_some() {
            return None;
        }
        Self::lock(&self.manifest).clone()
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
    pub source: opi_agent::evidence::AssemblySource,
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
    /// Configured route frozen at setup.
    pub configured_route: RouteSelection,
    /// Configured run budget frozen at setup.
    pub budget: Measurement,
}

/// Builder input handed to `CodingHarness::builder().evidence(..)`: the recording
/// sink plus the direct-assembly origin label.
pub struct EvidenceBuilderConfig {
    /// The recording sink (also the Agent's evidence sink).
    pub recorder: Arc<dyn EvidenceRecorder>,
    /// Where the direct assembly originated (CLI / SDK / RPC).
    pub source: opi_agent::evidence::AssemblySource,
}

impl EvidenceCapture {
    /// Assemble the capture from the recorder, the assembly source, the
    /// effective-policy digest, the resolved configuration identity, and the
    /// material runtime inputs (system prompt, model selection, resolved config)
    /// whose digest addresses the runtime-input binding. Direct assembly always
    /// binds [`RuntimeInputBinding::DirectRuntimeInput`]; it never fabricates an
    /// `ActiveSnapshot` (P17-EVD-003 / INV-008).
    pub fn new(
        recorder: Arc<dyn EvidenceRecorder>,
        source: opi_agent::evidence::AssemblySource,
        policy_digest: ContentDigest,
        config: ConfigIdentity,
        material_inputs: &str,
    ) -> Self {
        let binding_digest = runtime_input_digest(source, &policy_digest, &config, material_inputs);
        Self {
            recorder,
            source,
            binding: RuntimeInputBinding::direct(binding_digest, source),
            config,
            policy: UserPolicyFacts {
                policy_digest,
                capability: None,
            },
            system_digest: None,
            tool_schema_digests: Vec::new(),
            configured_route: empty_selection(),
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
        configured_route: RouteSelection,
        budget: Measurement,
    ) {
        let binding_digest = runtime_input_digest(
            self.source,
            &self.policy.policy_digest,
            &config,
            material_inputs,
        );
        self.binding = RuntimeInputBinding::direct(binding_digest, self.source);
        self.config = config;
        self.system_digest = system_digest;
        self.tool_schema_digests = tool_schema_digests;
        self.configured_route = configured_route;
        self.budget = budget;
    }
}

/// The runtime-input binding digest covers the resolved material runtime inputs:
/// the assembly source, the effective policy, the resolved configuration
/// identity, and the material input rendering.
fn runtime_input_digest(
    source: opi_agent::evidence::AssemblySource,
    policy_digest: &ContentDigest,
    config: &ConfigIdentity,
    material_inputs: &str,
) -> ContentDigest {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(match source {
        opi_agent::evidence::AssemblySource::Cli => b"cli".as_slice(),
        opi_agent::evidence::AssemblySource::Sdk => b"sdk".as_slice(),
        opi_agent::evidence::AssemblySource::Rpc => b"rpc".as_slice(),
    });
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
    /// Session branch reference, when a session is active.
    pub session_branch: Option<opi_agent::evidence::SessionBranchRef>,
    /// Digest of the prompt that drove the run.
    pub prompt_digest: ContentDigest,
    /// Provider-reported route from the terminal assistant message, when the
    /// provider reports it.
    pub actual_route: Option<RouteSelection>,
}

/// Assemble the strict [`FinalizedManifest`] from the capture's static facts,
/// the recorder's ordered records (call-graph correlation + route), and the
/// run's terminal dynamic facts. The caller runs
/// [`FinalizedManifest::require_complete`] and `finalize_run` afterwards.
pub fn build_finalized_manifest(
    capture: &EvidenceCapture,
    records: &[EvidenceRecord],
    dynamic: RunDynamicFacts,
) -> FinalizedManifest {
    let correlation = terminal_correlation(records);
    let mut route = extract_route_facts(records);
    if route.resolved.provider_id.is_empty() || route.resolved.model_id.is_empty() {
        route.requested = capture.configured_route.clone();
        route.resolved = capture.configured_route.clone();
    }
    if let Some(actual) = dynamic.actual_route {
        route.actual = actual;
        route.actual_reason = None;
    }
    let completeness = if capture_recorder_failed(capture) {
        opi_agent::evidence::EvidenceCompleteness::Incomplete
    } else {
        opi_agent::evidence::EvidenceCompleteness::Complete
    };
    FinalizedManifest {
        correlation,
        outcome: dynamic.outcome,
        session_branch: dynamic.session_branch,
        binding: capture.binding.clone(),
        config: capture.config.clone(),
        route,
        // Non-secret provenance extracted from the provider record's resolved
        // authentication (P17-PRV-005), never assumed Static.
        provenance: extract_provenance_facts(records),
        policy: capture.policy.clone(),
        input_identity: opi_agent::evidence::InputIdentity {
            prompt_digest: dynamic.prompt_digest,
            system_digest: capture.system_digest.clone(),
            tool_schema_digests: capture.tool_schema_digests.clone(),
        },
        environment: EnvironmentFacts {
            budget: capture.budget,
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
        // references" is a constraint on any future producer (an empty set
        // satisfies it vacuously), not a requirement to emit.
        // ArtifactRole/SensitivityClassification/FinalizationState are reserved
        // for a later artifact-producing task.
        artifacts: Vec::new(),
        completeness,
    }
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

/// Parsed shape of a Provider record's redacted route payload.
#[derive(Deserialize)]
struct RoutePayload {
    requested_route: String,
    resolved: RouteWire,
    actual: RouteWire,
    /// Non-secret auth source classification token (`static` / `environment` /
    /// `credential_store` / `oauth`), absent on records predating 17.7.
    auth_source: Option<String>,
    /// Non-secret auth fallback token (`not_attempted` / `used`), absent on
    /// records predating 17.7.
    fallback: Option<String>,
    /// Typed reason the actual route is empty (`not_reported`), absent on
    /// records predating 17.7.
    actual_reason: Option<String>,
}

#[derive(Deserialize)]
struct RouteWire {
    provider: String,
    model: String,
    wire: opi_ai::WireApi,
}

/// Extract the requested/resolved/actual route facts from the last Provider
/// record. Falls back to a resolved-equals-actual route from the resolved
/// payload when only that is available.
fn extract_route_facts(records: &[EvidenceRecord]) -> RouteFacts {
    let provider_payload = records
        .iter()
        .rev()
        .find(|r| r.kind == CallKind::Provider)
        .and_then(|r| match &r.payload {
            opi_agent::evidence::EvidencePayload::Structured(rv) => Some(rv.as_value().clone()),
            _ => None,
        });
    let Some(payload) = provider_payload else {
        return RouteFacts {
            requested: empty_selection(),
            resolved: empty_selection(),
            actual: empty_selection(),
            actual_reason: Some(opi_agent::evidence::UnknownReason::NotReported),
        };
    };
    match serde_json::from_value::<RoutePayload>(payload) {
        Ok(parsed) => {
            let requested = split_spec(&parsed.requested_route, &parsed.resolved);
            let resolved = selection_from(&parsed.resolved);
            let actual = selection_from(&parsed.actual);
            RouteFacts {
                requested,
                resolved,
                actual,
                actual_reason: actual_reason_from_token(parsed.actual_reason.as_deref()),
            }
        }
        Err(_) => RouteFacts {
            requested: empty_selection(),
            resolved: empty_selection(),
            actual: empty_selection(),
            actual_reason: Some(opi_agent::evidence::UnknownReason::NotReported),
        },
    }
}

/// Map the Provider record's actual-reason token to a typed [`UnknownReason`].
/// A populated actual route has no reason; a token on an empty actual carries
/// the reason. Absent on records predating the field.
fn actual_reason_from_token(token: Option<&str>) -> Option<opi_agent::evidence::UnknownReason> {
    match token {
        Some("not_reported") => Some(opi_agent::evidence::UnknownReason::NotReported),
        Some("withheld") => Some(opi_agent::evidence::UnknownReason::Withheld),
        Some("pending_finalization") => {
            Some(opi_agent::evidence::UnknownReason::PendingFinalization)
        }
        _ => None,
    }
}

/// Extract the non-secret auth source + fallback classification from the last
/// Provider record (P17-PRV-005). Falls back to a static/unknown provenance when
/// the record predates the provenance fields or the token is unrecognized.
fn extract_provenance_facts(records: &[EvidenceRecord]) -> ProvenanceFacts {
    let provider_payload = records
        .iter()
        .rev()
        .find(|r| r.kind == CallKind::Provider)
        .and_then(|r| match &r.payload {
            opi_agent::evidence::EvidencePayload::Structured(rv) => Some(rv.as_value().clone()),
            _ => None,
        });
    let Some(payload) = provider_payload else {
        return ProvenanceFacts {
            auth_source: AuthProvenanceSource::Static,
            fallback_allowed: None,
        };
    };
    match serde_json::from_value::<RoutePayload>(payload) {
        Ok(parsed) => ProvenanceFacts {
            auth_source: auth_source_from_token(parsed.auth_source.as_deref()),
            fallback_allowed: fallback_allowed_from_token(parsed.fallback.as_deref()),
        },
        Err(_) => ProvenanceFacts {
            auth_source: AuthProvenanceSource::Static,
            fallback_allowed: None,
        },
    }
}

fn auth_source_from_token(token: Option<&str>) -> AuthProvenanceSource {
    match token {
        Some("environment") => AuthProvenanceSource::Environment,
        Some("credential_store") => AuthProvenanceSource::CredentialStore,
        Some("oauth") => AuthProvenanceSource::Oauth,
        _ => AuthProvenanceSource::Static,
    }
}

fn fallback_allowed_from_token(token: Option<&str>) -> Option<bool> {
    match token {
        Some("used") => Some(true),
        Some("not_attempted") => Some(false),
        _ => None,
    }
}

fn selection_from(w: &RouteWire) -> RouteSelection {
    RouteSelection {
        provider_id: w.provider.clone(),
        model_id: w.model.clone(),
        wire: w.wire,
    }
}

/// Split a `provider:model` spec into a route selection, borrowing the resolved
/// wire when the request did not name one.
fn split_spec(spec: &str, resolved: &RouteWire) -> RouteSelection {
    let (provider, model) = spec
        .split_once(':')
        .map(|(p, m)| (p.to_owned(), m.to_owned()))
        .unwrap_or_else(|| (resolved.provider.clone(), resolved.model.clone()));
    RouteSelection {
        provider_id: provider,
        model_id: model,
        wire: resolved.wire,
    }
}

fn empty_selection() -> RouteSelection {
    RouteSelection {
        provider_id: String::new(),
        model_id: String::new(),
        wire: opi_ai::WireApi::OpenAiCompletions,
    }
}

/// Convert an aggregated provider token usage into evidence usage facts.
/// Unknown stays distinct from a measured zero (P17-EVD-004).
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

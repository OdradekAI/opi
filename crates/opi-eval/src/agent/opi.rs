//! Crate-private Opi process adapter and evidence importer (Phase 18 task
//! 18.6).
//!
//! [`OpiProcessAdapter`] owns ONLY Opi-specific launch and evidence rules:
//! argv/profile construction for the real built `opi` binary in one-shot JSON
//! mode with explicit `--trace` capture, the isolated environment projection,
//! the authoritative completion predicate (exit 0 AND exactly one completed
//! trace child in the fresh trace root), and the private importer for Opi's
//! Phase 17 `evidence.jsonl` + finalized `manifest.json`. It never links the
//! Opi runtime: the importer accepts only the exact schema identities covered
//! by `tests/fixtures/agents/opi` and fails closed on unknown required
//! evidence (P18-AGT-003/004). Final workspace capture and RunBundle
//! association belong to the Companion runner (18.12).

use super::process::{
    AgentAdapter, AgentCapability, AgentCompletion, AgentFailure, AgentIdentity, AgentRunRequest,
    Fact, NativeArtifact, UsageProjection,
};
use crate::failure::FailureBoundaryCode;
use crate::process::{ExitState, SpawnSpec};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// The declarative profile checked in at `profiles/agents/opi.toml`.
pub(crate) struct OpiProfile {
    /// Exact schema identity; any other value fails closed.
    schema: String,
    product: String,
    package: String,
    adapter: String,
    isolation: IsolationKeys,
    limits: Limits,
}

#[derive(Deserialize)]
struct LaunchPolicy {
    one_shot_json: bool,
    non_interactive: bool,
    trace_capture: bool,
    no_trust: bool,
}

#[derive(Deserialize)]
struct IsolationKeys {
    home_env: String,
    app_data_env: String,
    sessions_env: String,
}

#[derive(Deserialize)]
struct Limits {
    timeout_secs: u64,
    stdout_cap_bytes: usize,
    stderr_cap_bytes: usize,
}

#[derive(Deserialize)]
struct ProfileBody {
    schema: String,
    product: String,
    identity: IdentityBody,
    launch: LaunchPolicy,
    isolation: IsolationKeys,
    limits: Limits,
}

#[derive(Deserialize)]
struct IdentityBody {
    package: String,
    adapter: String,
}

impl OpiProfile {
    /// Parse the checked-in declarative profile. Fails closed on any schema
    /// identity other than the one this adapter implements (P18-AGT-003).
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let body: ProfileBody = toml::from_str(text).map_err(|e| format!("profile: {e}"))?;
        if body.schema != "phase18-agent-profile/1" {
            return Err(format!("profile schema {} is not supported", body.schema));
        }
        if !body.launch.one_shot_json
            || !body.launch.non_interactive
            || !body.launch.trace_capture
            || !body.launch.no_trust
        {
            return Err("profile disables a required launch invariant".to_owned());
        }
        Ok(Self {
            schema: body.schema,
            product: body.product,
            package: body.identity.package,
            adapter: body.identity.adapter,
            isolation: body.isolation,
            limits: body.limits,
        })
    }

    /// The checked-in profile bytes compiled into the crate.
    pub(crate) fn checked_in() -> Self {
        Self::parse(include_str!("../../profiles/agents/opi.toml"))
            .expect("checked-in opi profile must parse")
    }
}

/// The Opi adapter: launch policy + private importer over saved-bytes schema
/// identities. Constructed from the checked-in declarative profile.
pub(crate) struct OpiProcessAdapter {
    profile: OpiProfile,
}

impl OpiProcessAdapter {
    pub(crate) fn new() -> Self {
        Self {
            profile: OpiProfile::checked_in(),
        }
    }
}

impl Default for OpiProcessAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for OpiProcessAdapter {
    fn identity(&self, request: &AgentRunRequest) -> AgentIdentity {
        AgentIdentity {
            product: self.profile.product.clone(),
            package: self.profile.package.clone(),
            adapter: format!("{} ({})", self.profile.adapter, self.profile.schema),
            executable: request.executable.clone(),
        }
    }

    fn capabilities(&self) -> &'static [AgentCapability] {
        &[
            AgentCapability::JsonEvents,
            AgentCapability::EvidenceManifest,
            AgentCapability::UsageFacts,
        ]
    }

    /// Exact argv for the real production CLI — one-shot JSON mode, explicit
    /// trace capture, no project trust, explicit benchmark configuration, and
    /// the exact `provider:model` with no fallback (P18-AGT-001). Mutating
    /// tools are opt-in per request. No eval-only entry point exists.
    fn spawn_spec(&self, request: &AgentRunRequest) -> SpawnSpec {
        let mut argv: Vec<OsString> = vec![
            request.executable.clone().into(),
            "--json".into(),
            "--non-interactive".into(),
            "--trace".into(),
            request.trace_root.clone().into(),
            "--no-trust".into(),
            "--config".into(),
            request.config_path.clone().into(),
            "--model".into(),
            request.provider_model.clone().into(),
        ];
        if request.allow_mutating {
            argv.push("--allow-mutating".into());
        }
        argv.push(request.prompt.clone().into());

        // The child's ENTIRE environment: the isolated home/app-data/sessions
        // projection from the profile plus the request's exact extras (e.g. a
        // local scripted provider). No ambient leakage.
        let mut env: BTreeMap<OsString, OsString> = BTreeMap::new();
        env.insert(
            self.profile.isolation.home_env.clone().into(),
            request.isolation.home.clone().into(),
        );
        env.insert(
            self.profile.isolation.app_data_env.clone().into(),
            request.isolation.app_data.clone().into(),
        );
        env.insert(
            self.profile.isolation.sessions_env.clone().into(),
            request.isolation.sessions.clone().into(),
        );
        for (key, value) in &request.extra_env {
            env.insert(key.clone(), value.clone());
        }

        SpawnSpec {
            argv,
            cwd: Some(request.workspace.clone()),
            env,
            stdout_cap: self.profile.limits.stdout_cap_bytes,
            stderr_cap: self.profile.limits.stderr_cap_bytes,
            timeout: std::time::Duration::from_secs(self.profile.limits.timeout_secs),
        }
    }

    fn settle(
        &self,
        outcome: &crate::process::SupervisedOutcome,
        request: &AgentRunRequest,
    ) -> (UsageProjection, AgentCompletion) {
        // The process-level verdict is authoritative first: a non-zero exit,
        // timeout, cancellation, or spawn failure can never be rescued by
        // evidence, and no fallback exists (P18-AGT-003).
        let process_failed = |failure: super::process::AgentFailure| {
            (UsageProjection::default(), AgentCompletion::Failed(failure))
        };
        match outcome.exit {
            ExitState::Exited { code: 0 } => {}
            ExitState::Exited { code } => {
                return process_failed(super::process::failure_kinds::non_zero_exit(code));
            }
            ExitState::TimedOut => return process_failed(super::process::failure_kinds::TIMED_OUT),
            ExitState::Cancelled => {
                return process_failed(super::process::failure_kinds::CANCELLED);
            }
            ExitState::FailedToSpawn { reason } => {
                return process_failed(super::process::failure_kinds::spawn(reason));
            }
        }

        // Exit 0 AND exactly one completed trace child AND a success
        // manifest: the Opi completion predicate over the fresh trace root.
        let imported = OpiImporter.import_trace(&request.trace_root);
        let usage = imported.usage.clone();
        let completion = match imported.completion {
            AgentCompletion::Completed { artifacts } if imported.manifest_outcome == "success" => {
                AgentCompletion::Completed { artifacts }
            }
            AgentCompletion::Completed { .. } => AgentCompletion::Failed(AgentFailure {
                kind: "agent-outcome-not-success",
                boundary: FailureBoundaryCode::AgentProcess,
            }),
            AgentCompletion::Failed(failure) => AgentCompletion::Failed(failure),
        };
        (usage, completion)
    }
}

/// sha256 hex digest of `bytes` (content-addressed native artifacts).
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// The private Opi evidence importer. Accepts ONLY the exact Phase 17 schema
/// identities pinned by `tests/fixtures/agents/opi`: the twelve manifest
/// top-level fields, externally-tagged `Measurement` usage, and the exact
/// `EvidenceRecord` line shape. Fields outside the settled projection
/// (session, binding, config, ...) are validated for exact top-level
/// presence but not re-moded — Opi's records are serialize-oriented and define
/// no public reader, so this importer never claims more than the fixture
/// identity proves. Unknown required evidence fails closed; no fallback
/// parser exists (P18-AGT-003/004).
pub(crate) struct OpiImporter;

/// What one import settled to. Import failures are settled values, not
/// errors: the trace root is real observed evidence of a failed capture.
#[derive(Debug, Clone)]
pub(crate) struct ImportedTrace {
    /// Completion verdict over the imported bytes.
    pub completion: AgentCompletion,
    /// Manifest terminal outcome label (empty when import failed).
    pub manifest_outcome: String,
    /// Whether the manifest declared itself complete.
    pub complete: bool,
    /// Usage projection with typed unknown reasons.
    pub usage: UsageProjection,
    /// Content-addressed native artifacts for the imported files.
    pub artifacts: Vec<NativeArtifact>,
}

impl OpiImporter {
    /// Import one trace root: find `run-*` children, require exactly one
    /// finalized (manifest-bearing) child, validate its exact schema
    /// identity, and settle a completion verdict with content-addressed
    /// artifacts. Never mutates the source tree.
    pub(crate) fn import_trace(&self, root: &std::path::Path) -> ImportedTrace {
        let failed = |kind: &'static str, boundary| ImportedTrace {
            completion: AgentCompletion::Failed(AgentFailure { kind, boundary }),
            manifest_outcome: String::new(),
            complete: false,
            usage: UsageProjection::default(),
            artifacts: Vec::new(),
        };

        // Enumerate finalized children in the fresh trace root.
        let Ok(entries) = std::fs::read_dir(root) else {
            return failed("import-evidence-missing", FailureBoundaryCode::Evidence);
        };
        let mut finalized: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("run-"))
                    && p.join("manifest.json").is_file()
            })
            .collect();
        finalized.sort();
        match finalized.len() {
            0 => return failed("import-evidence-incomplete", FailureBoundaryCode::Evidence),
            1 => {}
            _ => return failed("import-ambiguous-trace", FailureBoundaryCode::Adapter),
        }
        let child = &finalized[0];

        let manifest_bytes = match std::fs::read(child.join("manifest.json")) {
            Ok(bytes) => bytes,
            Err(_) => return failed("import-evidence-missing", FailureBoundaryCode::Evidence),
        };
        let manifest: serde_json::Value = match serde_json::from_slice(&manifest_bytes) {
            Ok(value) => value,
            Err(_) => return failed("import-parse-failure", FailureBoundaryCode::Adapter),
        };
        // Exact top-level schema identity: all twelve required fields, no
        // unknown additions.
        const REQUIRED: [&str; 12] = [
            "correlation",
            "outcome",
            "session",
            "binding",
            "config",
            "provider",
            "policy",
            "input_identity",
            "environment",
            "usage",
            "artifacts",
            "completeness",
        ];
        let Some(fields) = manifest.as_object() else {
            return failed("import-parse-failure", FailureBoundaryCode::Adapter);
        };
        if fields.len() != REQUIRED.len() || REQUIRED.iter().any(|key| !fields.contains_key(*key)) {
            return failed("import-unsupported-schema", FailureBoundaryCode::Adapter);
        }

        // Terminal outcome + completeness projections.
        let outcome = match fields.get("outcome").and_then(|v| v.as_str()) {
            Some(
                label @ ("success"
                | "cancelled"
                | "failed"
                | "partial_side_effect"
                | "cleanup_unknown"),
            ) => label.to_owned(),
            _ => return failed("import-unsupported-schema", FailureBoundaryCode::Adapter),
        };
        let complete = match fields.get("completeness").and_then(|v| v.as_str()) {
            Some("complete") => true,
            Some("incomplete") => false,
            _ => return failed("import-unsupported-schema", FailureBoundaryCode::Adapter),
        };
        if !complete {
            return failed("import-evidence-incomplete", FailureBoundaryCode::Evidence);
        }

        // Usage projection: externally-tagged Measurements, unknown reasons
        // kept typed.
        let usage = match (
            parse_measurement(fields.get("usage").and_then(|u| u.get("input_tokens"))),
            parse_measurement(fields.get("usage").and_then(|u| u.get("output_tokens"))),
        ) {
            (Some(input), Some(output)) => UsageProjection {
                input_tokens: input,
                output_tokens: output,
            },
            _ => return failed("import-unsupported-schema", FailureBoundaryCode::Adapter),
        };

        // Evidence records: exact line shape + terminal-sequence coherence
        // with the manifest correlation.
        let evidence_bytes = match std::fs::read(child.join("evidence.jsonl")) {
            Ok(bytes) => bytes,
            Err(_) => return failed("import-evidence-missing", FailureBoundaryCode::Evidence),
        };
        let terminal_sequence = match validate_evidence_records(&evidence_bytes) {
            Ok(sequence) => sequence,
            Err(kind) => {
                return failed(kind, FailureBoundaryCode::Adapter);
            }
        };
        let manifest_sequence = fields
            .get("correlation")
            .and_then(|c| c.get("sequence"))
            .and_then(|s| s.as_u64());
        if manifest_sequence != Some(terminal_sequence) {
            return failed("import-evidence-mismatch", FailureBoundaryCode::Evidence);
        }

        let artifacts = vec![
            NativeArtifact {
                role: "evidence/manifest".to_owned(),
                sha256: sha256_hex(&manifest_bytes),
                path: child.join("manifest.json"),
            },
            NativeArtifact {
                role: "evidence/records".to_owned(),
                sha256: sha256_hex(&evidence_bytes),
                path: child.join("evidence.jsonl"),
            },
        ];
        ImportedTrace {
            completion: AgentCompletion::Completed {
                artifacts: artifacts.clone(),
            },
            manifest_outcome: outcome,
            complete,
            usage,
            artifacts,
        }
    }
}

/// Parse one externally-tagged `Measurement`: `{"Known":{"value":..,
/// "origin":".."}}` or `{"Unknown":{"reason":".."}}`.
fn parse_measurement(value: Option<&serde_json::Value>) -> Option<Option<Fact>> {
    let outer = value?.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    if let Some(known) = outer.get("Known") {
        return Some(Some(Fact::Known {
            value: known.get("value")?.as_u64()?,
            origin: known.get("origin")?.as_str()?.to_owned(),
        }));
    }
    if let Some(unknown) = outer.get("Unknown") {
        return Some(Some(Fact::Unknown {
            reason: unknown.get("reason")?.as_str()?.to_owned(),
        }));
    }
    None
}

/// Validate every NDJSON evidence line against the exact record shape and
/// return the terminal (last) sequence number.
fn validate_evidence_records(bytes: &[u8]) -> Result<u64, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "import-parse-failure")?;
    let mut terminal: Option<u64> = None;
    let mut lines = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        lines += 1;
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| "import-parse-failure")?;
        let record = value.as_object().ok_or("import-parse-failure")?;
        const RECORD_FIELDS: [&str; 7] = [
            "run", "turn", "call", "parent", "sequence", "kind", "payload",
        ];
        if record.len() != RECORD_FIELDS.len()
            || RECORD_FIELDS.iter().any(|k| !record.contains_key(*k))
        {
            return Err("import-unsupported-schema");
        }
        // run must be a UUID-shaped string (version/variant nibbles checked).
        let run = record["run"].as_str().ok_or("import-unsupported-schema")?;
        let bytes = run.as_bytes();
        if bytes.len() != 36
            || bytes.iter().filter(|b| **b == b'-').count() != 4
            || bytes[14] != b'7'
            || !matches!(bytes[19], b'8'..=b'b')
        {
            return Err("import-unsupported-schema");
        }
        if !matches!(
            record["kind"].as_str(),
            Some("provider" | "tool" | "retry" | "compaction" | "diagnostic")
        ) {
            return Err("import-unsupported-schema");
        }
        // Payload is externally tagged with exactly one PascalCase channel.
        let payload = record["payload"]
            .as_object()
            .ok_or("import-parse-failure")?;
        if payload.len() != 1
            || !payload.keys().all(|k| {
                matches!(
                    k.as_str(),
                    "Provider" | "Tool" | "Compaction" | "Structured" | "Digest" | "Diagnostic"
                )
            })
        {
            return Err("import-unsupported-schema");
        }
        terminal = Some(
            record["sequence"]
                .as_u64()
                .ok_or("import-unsupported-schema")?,
        );
    }
    if lines == 0 {
        return Err("import-evidence-missing");
    }
    terminal.ok_or("import-evidence-missing")
}

#[cfg(test)]
mod tests {
    use super::super::process::failure_kinds;
    use super::*;
    use crate::process::OutputCapture;

    fn request() -> AgentRunRequest {
        AgentRunRequest {
            executable: PathBuf::from("/tmp/fake-opi"),
            prompt: "solve the task".to_owned(),
            workspace: PathBuf::from("/tmp/ws"),
            trace_root: PathBuf::from("/tmp/trace"),
            config_path: PathBuf::from("/tmp/bench.toml"),
            provider_model: "local:scripted".to_owned(),
            allow_mutating: false,
            isolation: super::super::process::IsolationDirs {
                home: PathBuf::from("/tmp/iso/home"),
                app_data: PathBuf::from("/tmp/iso/appdata"),
                sessions: PathBuf::from("/tmp/iso/sessions"),
            },
            extra_env: BTreeMap::from([(
                OsString::from("OPI_EVAL_PROVIDER"),
                OsString::from("scripted"),
            )]),
        }
    }

    fn argv_strings(spec: &SpawnSpec) -> Vec<String> {
        spec.argv
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn checked_in_profile_parses_with_exact_schema_identity() {
        let profile = OpiProfile::checked_in();
        assert_eq!(profile.schema, "phase18-agent-profile/1");
        assert_eq!(profile.product, "opi");
        assert_eq!(profile.package, "opi-coding-agent");
        assert_eq!(profile.adapter, "opi-eval-opi-adapter/1");

        // Any other schema identity fails closed.
        let mutated = include_str!("../../profiles/agents/opi.toml")
            .replace("phase18-agent-profile/1", "phase18-agent-profile/2");
        assert!(OpiProfile::parse(&mutated).is_err());
        // Disabling a required launch invariant fails closed.
        let disabled = include_str!("../../profiles/agents/opi.toml")
            .replace("no_trust = true", "no_trust = false");
        assert!(OpiProfile::parse(&disabled).is_err());
    }

    #[test]
    fn opi_argv_is_exact_production_surface_with_no_fallback() {
        let adapter = OpiProcessAdapter::new();
        let spec = adapter.spawn_spec(&request());
        assert_eq!(
            argv_strings(&spec),
            vec![
                "/tmp/fake-opi",
                "--json",
                "--non-interactive",
                "--trace",
                "/tmp/trace",
                "--no-trust",
                "--config",
                "/tmp/bench.toml",
                "--model",
                "local:scripted",
                "solve the task",
            ]
        );
        assert_eq!(spec.cwd, Some(PathBuf::from("/tmp/ws")));

        // Mutating tools are opt-in, appended before the prompt.
        let mut mutating = request();
        mutating.allow_mutating = true;
        let argv = argv_strings(&adapter.spawn_spec(&mutating));
        assert_eq!(argv[10], "--allow-mutating");
        assert_eq!(argv[11], "solve the task");
        assert_eq!(argv.len(), 12);
    }

    #[test]
    fn opi_environment_is_the_isolated_projection_plus_exact_extras() {
        let adapter = OpiProcessAdapter::new();
        let spec = adapter.spawn_spec(&request());
        assert_eq!(
            spec.env,
            BTreeMap::from([
                (OsString::from("HOME"), OsString::from("/tmp/iso/home")),
                (
                    OsString::from("APPDATA"),
                    OsString::from("/tmp/iso/appdata")
                ),
                (
                    OsString::from("OPI_SESSIONS_DIR"),
                    OsString::from("/tmp/iso/sessions")
                ),
                (
                    OsString::from("OPI_EVAL_PROVIDER"),
                    OsString::from("scripted")
                ),
            ])
        );
        assert_eq!(spec.stdout_cap, 1_048_576);
        assert_eq!(spec.stderr_cap, 1_048_576);
        assert_eq!(spec.timeout, std::time::Duration::from_secs(900));
    }

    #[test]
    fn opi_identity_declares_adapter_and_capabilities() {
        let adapter = OpiProcessAdapter::new();
        let identity = adapter.identity(&request());
        assert_eq!(identity.product, "opi");
        assert_eq!(identity.package, "opi-coding-agent");
        assert_eq!(
            identity.adapter,
            "opi-eval-opi-adapter/1 (phase18-agent-profile/1)"
        );
        assert_eq!(
            adapter.capabilities(),
            &[
                AgentCapability::JsonEvents,
                AgentCapability::EvidenceManifest,
                AgentCapability::UsageFacts,
            ]
        );
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/agents/opi")
            .join(name)
    }

    fn settled_completion(trace_root: &std::path::Path) -> AgentCompletion {
        OpiImporter.import_trace(trace_root).completion
    }

    #[test]
    fn importer_accepts_the_exact_saved_schema_and_projects_usage() {
        let imported = OpiImporter.import_trace(&fixture("trace-complete"));
        assert!(matches!(
            imported.completion,
            AgentCompletion::Completed { .. }
        ));
        assert_eq!(imported.manifest_outcome, "success");
        assert!(imported.complete);
        assert_eq!(
            imported.usage,
            UsageProjection {
                input_tokens: Some(Fact::Known {
                    value: 1234,
                    origin: "provider_reported".to_owned(),
                }),
                output_tokens: Some(Fact::Unknown {
                    reason: "not_reported".to_owned(),
                }),
            }
        );
        // Content-addressed native artifacts for both imported files.
        assert_eq!(imported.artifacts.len(), 2);
        let roles: Vec<&str> = imported.artifacts.iter().map(|a| a.role.as_str()).collect();
        assert_eq!(roles, vec!["evidence/manifest", "evidence/records"]);
        for artifact in &imported.artifacts {
            assert_eq!(artifact.sha256.len(), 64);
            assert!(artifact.path.is_file());
        }
    }

    #[test]
    fn importer_fails_closed_on_schema_drift_and_corruption() {
        // Unknown top-level manifest field: exact-schema identity violated.
        assert_eq!(
            settled_completion(&fixture("trace-unknown-schema")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-unsupported-schema",
                boundary: FailureBoundaryCode::Adapter,
            })
        );
        // Unparseable manifest bytes.
        assert_eq!(
            settled_completion(&fixture("trace-corrupt")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-parse-failure",
                boundary: FailureBoundaryCode::Adapter,
            })
        );
        // Manifest sequence disagrees with the evidence terminal record.
        assert_eq!(
            settled_completion(&fixture("trace-sequence-mismatch")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-mismatch",
                boundary: FailureBoundaryCode::Evidence,
            })
        );
    }

    #[test]
    fn importer_fails_closed_on_incomplete_missing_and_ambiguous_traces() {
        // Evidence without a finalized manifest: never represented as complete
        // (P18-AGT-004).
        assert_eq!(
            settled_completion(&fixture("trace-incomplete")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-incomplete",
                boundary: FailureBoundaryCode::Evidence,
            })
        );
        // Finalized manifest without its evidence records.
        assert_eq!(
            settled_completion(&fixture("trace-missing-evidence")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-missing",
                boundary: FailureBoundaryCode::Evidence,
            })
        );
        // Two completed trace children in one fresh root: ambiguous.
        assert_eq!(
            settled_completion(&fixture("trace-multiple")),
            AgentCompletion::Failed(AgentFailure {
                kind: "import-ambiguous-trace",
                boundary: FailureBoundaryCode::Adapter,
            })
        );
    }

    fn settled_outcome(code: i32) -> crate::process::SupervisedOutcome {
        crate::process::SupervisedOutcome {
            exit: ExitState::Exited { code },
            stdout: OutputCapture::default(),
            stderr: OutputCapture::default(),
            cleanup: crate::process::CleanupEvidence::NotRequired,
        }
    }

    #[test]
    fn opi_completion_predicate_requires_exit_zero_and_successful_import() {
        let adapter = OpiProcessAdapter::new();

        // Exit 0 + complete success manifest: completed with usage projected.
        let mut complete_request = request();
        complete_request.trace_root = fixture("trace-complete");
        let (usage, completion) = adapter.settle(&settled_outcome(0), &complete_request);
        assert!(matches!(completion, AgentCompletion::Completed { .. }));
        assert_eq!(
            usage.input_tokens,
            Some(Fact::Known {
                value: 1234,
                origin: "provider_reported".to_owned(),
            })
        );

        // Non-zero exit is authoritative regardless of present evidence.
        let (usage, completion) = adapter.settle(&settled_outcome(3), &complete_request);
        assert_eq!(
            completion,
            AgentCompletion::Failed(failure_kinds::non_zero_exit(3))
        );
        assert_eq!(usage.input_tokens, None);

        // Exit 0 but incomplete evidence settles as an evidence failure.
        let mut incomplete = request();
        incomplete.trace_root = fixture("trace-incomplete");
        let (_, completion) = adapter.settle(&settled_outcome(0), &incomplete);
        assert_eq!(
            completion,
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-incomplete",
                boundary: FailureBoundaryCode::Evidence,
            })
        );

        // Timeout and cancellation map through the shared failure kinds.
        for exit in [ExitState::TimedOut, ExitState::Cancelled] {
            let outcome = crate::process::SupervisedOutcome {
                exit,
                ..settled_outcome(0)
            };
            let (_, completion) = adapter.settle(&outcome, &incomplete);
            assert!(matches!(
                completion,
                AgentCompletion::Failed(AgentFailure {
                    boundary: FailureBoundaryCode::AgentProcess,
                    ..
                })
            ));
        }
    }

    #[cfg(unix)]
    fn fake_opi_script(dir: &std::path::Path, behavior: &str) -> PathBuf {
        let script = dir.join("fake-opi");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\n{behavior}\nmkdir -p \"$4\"\ncp -r \"$OPI_EVAL_TRACE_SOURCE/run-0001\" \"$4/\"\nexit 0\n"
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_opi_process_settles_a_completed_record_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpiProcessAdapter::new();
        let mut request = request();
        request.executable = fake_opi_script(dir.path(), "");
        request.workspace = dir.path().join("ws");
        request.trace_root = dir.path().join("trace");
        request.isolation = super::super::process::IsolationDirs {
            home: dir.path().join("iso/home"),
            app_data: dir.path().join("iso/appdata"),
            sessions: dir.path().join("iso/sessions"),
        };
        request.extra_env.insert(
            OsString::from("OPI_EVAL_TRACE_SOURCE"),
            fixture("trace-complete").into(),
        );
        std::fs::create_dir_all(&request.workspace).unwrap();

        let record = super::super::process::AgentExecution::run(
            &request,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 0 });
        let AgentCompletion::Completed { artifacts } = &record.completion else {
            panic!("expected completed record, got {:?}", record.completion);
        };
        assert_eq!(artifacts.len(), 2);
        assert_eq!(
            record.usage.output_tokens,
            Some(Fact::Unknown {
                reason: "not_reported".to_owned(),
            })
        );
        assert_eq!(record.identity.product, "opi");
        assert_eq!(record.workspace, request.workspace);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_opi_exit_failure_and_spawn_failure_settle_typed() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpiProcessAdapter::new();
        let mut request = request();
        request.executable = fake_opi_script(dir.path(), "echo planned-failure >&2");
        request.executable = {
            // Overwrite with a failing variant: copy evidence then exit 3.
            let script = dir.path().join("fake-opi-fail");
            std::fs::write(
                &script,
                "#!/bin/sh\nmkdir -p \"$4\"\ncp -r \"$OPI_EVAL_TRACE_SOURCE/run-0001\" \"$4/\"\nexit 3\n",
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
            script
        };
        request.workspace = dir.path().join("ws");
        request.trace_root = dir.path().join("trace");
        request.extra_env.insert(
            OsString::from("OPI_EVAL_TRACE_SOURCE"),
            fixture("trace-complete").into(),
        );
        std::fs::create_dir_all(&request.workspace).unwrap();

        let record = super::super::process::AgentExecution::run(
            &request,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 3 });
        assert_eq!(
            record.completion,
            AgentCompletion::Failed(failure_kinds::non_zero_exit(3))
        );
        // Evidence exists but cannot rescue a non-zero exit.
        assert!(request.trace_root.join("run-0001/manifest.json").is_file());

        // A nonexistent executable settles as a redacted spawn failure.
        let mut missing = request.clone();
        missing.executable = dir.path().join("no-such-opi");
        let record = super::super::process::AgentExecution::run(
            &missing,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            record.completion,
            AgentCompletion::Failed(AgentFailure {
                kind: "spawn-not-found",
                boundary: FailureBoundaryCode::AgentProcess,
            })
        );
        // Redaction: the settled record never echoes the missing path.
        assert!(!format!("{record:?}").contains("no-such-opi"));
    }
}

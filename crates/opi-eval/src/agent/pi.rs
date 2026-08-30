//! Crate-private pi coding-agent process adapter and JSON importer
//! (Phase 18 task 18.7).
//!
//! [`PiProcessAdapter`] owns ONLY pi-specific launch and evidence rules:
//! argv/profile construction for the pinned earendil-works coding-agent
//! (v0.84.3 CLI surface) in one-shot JSON print mode with an isolated
//! `PI_CODING_AGENT_DIR`/session projection and no ambient project, user, or
//! extension resources, plus the private importer for pi's documented stdout
//! JSON event stream (session header + `AgentSessionEvent` lines) and the
//! completion predicate over the terminal assistant message, `agent_end`, and
//! process exit. It never links pi code and claims no Harness v2 semantics
//! (P18-AGT-005). Final workspace capture and RunBundle association belong to
//! the Companion runner (18.12).

use super::process::{
    AgentAdapter, AgentCapability, AgentCompletion, AgentFailure, AgentIdentity, AgentRunRequest,
    Fact, NativeArtifact, UsageProjection,
};
use crate::failure::FailureBoundaryCode;
use crate::process::{ExitState, SpawnSpec};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;

/// The declarative profile checked in at `profiles/agents/pi.toml`.
pub(crate) struct PiProfile {
    /// Exact schema identity; any other value fails closed.
    schema: String,
    product: String,
    package: String,
    adapter: String,
    /// Deterministic thinking control passed on the argv.
    thinking_level: String,
    isolation: PiIsolationKeys,
    limits: PiLimits,
}

#[derive(Deserialize)]
struct PiLaunchPolicy {
    one_shot_json: bool,
    non_interactive: bool,
    trace_capture: bool,
    no_trust: bool,
    thinking_level: String,
}

#[derive(Deserialize)]
struct PiIsolationKeys {
    home_env: String,
    app_data_env: String,
    sessions_env: String,
    /// pi's agent config dir override (`~/.pi/agent` replacement).
    agent_dir_env: String,
}

#[derive(Deserialize)]
struct PiLimits {
    timeout_secs: u64,
    stdout_cap_bytes: usize,
    stderr_cap_bytes: usize,
}

#[derive(Deserialize)]
struct ProfileBody {
    schema: String,
    product: String,
    identity: IdentityBody,
    launch: PiLaunchPolicy,
    isolation: PiIsolationKeys,
    limits: PiLimits,
}

#[derive(Deserialize)]
struct IdentityBody {
    package: String,
    adapter: String,
}

impl PiProfile {
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
        if !matches!(
            body.launch.thinking_level.as_str(),
            "off" | "minimal" | "low" | "high"
        ) {
            return Err(format!(
                "thinking level {} is not a pinned deterministic level",
                body.launch.thinking_level
            ));
        }
        Ok(Self {
            schema: body.schema,
            product: body.product,
            package: body.identity.package,
            adapter: body.identity.adapter,
            thinking_level: body.launch.thinking_level,
            isolation: body.isolation,
            limits: body.limits,
        })
    }

    /// The checked-in profile bytes compiled into the crate.
    pub(crate) fn checked_in() -> Self {
        Self::parse(include_str!("../../profiles/agents/pi.toml"))
            .expect("checked-in pi profile must parse")
    }
}

/// The pi adapter: launch policy + private importer over the pinned v0.84.3
/// stdout JSON surface. Constructed from the checked-in declarative profile.
pub(crate) struct PiProcessAdapter {
    profile: PiProfile,
}

impl PiProcessAdapter {
    pub(crate) fn new() -> Self {
        Self {
            profile: PiProfile::checked_in(),
        }
    }

    /// Build from an already-validated profile. Crate-private: used by the
    /// conformance facade to stage bounded-timeout variants of the same
    /// pinned profile; every identity and launch invariant is unchanged.
    pub(crate) fn from_profile(profile: PiProfile) -> Self {
        Self { profile }
    }
}

impl Default for PiProcessAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for PiProcessAdapter {
    fn identity(&self, request: &AgentRunRequest) -> AgentIdentity {
        AgentIdentity {
            product: self.profile.product.clone(),
            package: self.profile.package.clone(),
            adapter: format!("{} ({})", self.profile.adapter, self.profile.schema),
            executable: request.executable.clone(),
        }
    }

    fn capabilities(&self) -> &'static [AgentCapability] {
        // pi has no Opi-equivalent finalized evidence manifest, so it does
        // not claim `EvidenceManifest`: its raw NDJSON stream is retained as
        // a native artifact instead (P18-AGT-007).
        &[AgentCapability::JsonEvents, AgentCapability::UsageFacts]
    }

    /// Exact argv for the pinned pi v0.84.3 production CLI — one-shot JSON
    /// print mode, no persistent session, no project trust or context files,
    /// no extension/skill discovery, explicit provider/model and deterministic
    /// thinking (P18-AGT-001/005). `--` ends option parsing before the prompt.
    fn spawn_spec(&self, request: &AgentRunRequest) -> SpawnSpec {
        // The shared request carries the exact selection as `provider:model`;
        // pi's CLI takes them as separate exact flags (no fuzzy resolution).
        let (provider, model) = request
            .provider_model
            .split_once(':')
            .expect("AgentExecution validates the exact provider:model shape");

        let argv: Vec<OsString> = vec![
            request.executable.clone().into(),
            "--mode".into(),
            "json".into(),
            "--print".into(),
            "--no-session".into(),
            "--no-extensions".into(),
            "--no-skills".into(),
            "--no-context-files".into(),
            "--no-approve".into(),
            "--provider".into(),
            provider.into(),
            "--model".into(),
            model.into(),
            "--thinking".into(),
            self.profile.thinking_level.clone().into(),
            "--".into(),
            request.prompt.clone().into(),
        ];

        // The child's ENTIRE environment: the isolated home/app-data/session
        // and pi agent-dir projection from the profile plus the request's
        // exact extras (e.g. a local scripted provider). No ambient leakage.
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
        env.insert(
            self.profile.isolation.agent_dir_env.clone().into(),
            request.isolation.app_data.join("pi-agent").into(),
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

        // In JSON mode pi exits 0 even when the terminal assistant message
        // reports `error`/`aborted` (that mapping is text-mode-only), so the
        // stream itself — terminal message, `agent_end`, and requested-model
        // identity — carries the authoritative completion facts
        // (P18-AGT-005). No mismatch is concealed.
        let (provider, model) = request
            .provider_model
            .split_once(':')
            .expect("AgentExecution validates the exact provider:model shape");
        let imported = PiImporter.import_stream(&outcome.stdout, &request.trace_root);
        let usage = imported.usage.clone();
        let completion = match imported.completion {
            AgentCompletion::Failed(failure) => AgentCompletion::Failed(failure),
            AgentCompletion::Completed { artifacts } => {
                if imported.stop_reason != "stop" {
                    AgentCompletion::Failed(AgentFailure {
                        kind: "agent-outcome-not-success",
                        boundary: FailureBoundaryCode::AgentProcess,
                    })
                } else if imported.provider != provider || imported.model != model {
                    AgentCompletion::Failed(AgentFailure {
                        kind: "agent-model-mismatch",
                        boundary: FailureBoundaryCode::Evidence,
                    })
                } else {
                    AgentCompletion::Completed { artifacts }
                }
            }
        };
        (usage, completion)
    }
}

/// One imported pi run: settled completion, terminal facts, and the usage
/// projection. Owned by [`PiImporter`].
struct ImportedPi {
    completion: AgentCompletion,
    stop_reason: String,
    provider: String,
    model: String,
    usage: UsageProjection,
}

/// Private importer for pi's documented stdout JSON protocol: one session
/// header line followed by `AgentSessionEvent` NDJSON. Accepts only the
/// pinned v0.84.3 shapes; anything older, unknown, corrupted, or ambiguous
/// fails closed (P18-MIG-002). Never mutates anything outside the fresh
/// trace root.
struct PiImporter;

impl PiImporter {
    fn import_stream(
        &self,
        stdout: &crate::process::OutputCapture,
        trace_root: &std::path::Path,
    ) -> ImportedPi {
        let failed = |kind: &'static str, boundary| ImportedPi {
            completion: AgentCompletion::Failed(AgentFailure { kind, boundary }),
            stop_reason: String::new(),
            provider: String::new(),
            model: String::new(),
            usage: UsageProjection::default(),
        };

        // A cap-truncated stream cannot contain a provably terminal event.
        if stdout.truncated {
            return failed("import-evidence-incomplete", FailureBoundaryCode::Evidence);
        }
        let text = match std::str::from_utf8(&stdout.bytes) {
            Ok(text) => text,
            Err(_) => return failed("import-parse-failure", FailureBoundaryCode::Adapter),
        };
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());

        // Line 1 must be the current session header: exact field identity.
        // The hermetic fixtures pin the v2 sample shape and the pinned pi
        // 0.84.3 stream emits v3 with the same event vocabulary; both
        // admitted, anything older or drifted fails closed.
        let header: serde_json::Value = match lines.next().map(serde_json::from_str) {
            Some(Ok(value)) => value,
            _ => return failed("import-evidence-missing", FailureBoundaryCode::Evidence),
        };
        let header_ok = matches!(
            header.as_object(),
            Some(fields)
                if fields.len() == 5
                    && fields["type"].as_str() == Some("session")
                    && matches!(fields["version"].as_u64(), Some(2 | 3))
                    && fields["id"].is_string()
                    && fields["timestamp"].is_string()
                    && fields["cwd"].is_string()
        );
        if !header_ok {
            return failed("import-unsupported-schema", FailureBoundaryCode::Adapter);
        }

        // Event lines: known `type` vocabulary, exactly one `agent_end`,
        // and a deeply validated terminal assistant `message_end`.
        let mut agent_ends = 0usize;
        let mut terminal: Option<serde_json::Value> = None;
        for line in lines {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => return failed("import-parse-failure", FailureBoundaryCode::Adapter),
            };
            let Some(fields) = value.as_object() else {
                return failed("import-parse-failure", FailureBoundaryCode::Adapter);
            };
            let Some(kind) = fields.get("type").and_then(|v| v.as_str()) else {
                return failed("import-parse-failure", FailureBoundaryCode::Adapter);
            };
            match kind {
                "agent_start"
                | "turn_start"
                | "turn_end"
                | "message_start"
                | "tool_execution_start"
                | "tool_execution_update"
                | "tool_execution_end"
                | "entry_appended"
                | "session_before_compact"
                | "session_before_tree"
                | "session_compact"
                | "session_compact_failed"
                | "session_shutdown"
                | "session_start"
                | "session_tree"
                | "session_info_changed"
                | "auto_retry_start"
                | "auto_retry_end"
                | "compaction_start"
                | "compaction_end"
                | "model_select"
                | "thinking_level_changed"
                | "thinking_level_select"
                | "queue_update"
                | "bash_execution_update"
                | "tool_call"
                | "tool_result"
                | "text"
                | "summarization_retry_attempt_start"
                | "summarization_retry_finished"
                | "summarization_retry_scheduled"
                | "agent_settled" => {}
                "message_update" => {
                    // Strip-partial wire shape: cumulative usage plus one
                    // assistant message event.
                    let event_ok = matches!(
                        (fields.get("usage"), fields.get("assistantMessageEvent")),
                        (Some(usage), Some(event))
                            if usage_is_shape(usage)
                                && event
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .is_some_and(|t| matches!(t,
                                        "start" | "text_start" | "text_delta" | "text_end"
                                        | "thinking_start" | "thinking_delta" | "thinking_end"
                                        | "toolcall_start" | "toolcall_delta" | "toolcall_end" | "done"))
                    );
                    if !event_ok {
                        return failed("import-unsupported-schema", FailureBoundaryCode::Adapter);
                    }
                }
                "message_end" => {
                    let message = match fields.get("message") {
                        Some(message) => message,
                        None => {
                            return failed(
                                "import-unsupported-schema",
                                FailureBoundaryCode::Adapter,
                            );
                        }
                    };
                    // Only assistant turns are validated and tracked as
                    // terminal candidates; the pinned stream also emits
                    // user and toolResult message_end events whose shapes
                    // differ by design.
                    if message.get("role").and_then(|v| v.as_str()) == Some("assistant") {
                        if let Err(kind) = validate_terminal_message(message) {
                            return failed(kind, FailureBoundaryCode::Adapter);
                        }
                        terminal = Some(message.clone());
                    }
                }
                "agent_end" => {
                    if !fields.get("messages").is_some_and(|m| m.is_array()) {
                        return failed("import-unsupported-schema", FailureBoundaryCode::Adapter);
                    }
                    agent_ends += 1;
                }
                _ => return failed("import-unsupported-schema", FailureBoundaryCode::Adapter),
            }
        }

        if agent_ends != 1 {
            return if agent_ends == 0 {
                failed("import-evidence-missing", FailureBoundaryCode::Evidence)
            } else {
                failed("import-ambiguous-stream", FailureBoundaryCode::Adapter)
            };
        }
        let Some(message) = terminal else {
            return failed("import-evidence-missing", FailureBoundaryCode::Evidence);
        };
        let fields = message.as_object().expect("validated terminal message");
        let stop_reason = fields["stopReason"].as_str().expect("validated").to_owned();
        let provider = fields["provider"].as_str().expect("validated").to_owned();
        let model = fields["model"].as_str().expect("validated").to_owned();
        let usage_fields = fields["usage"].as_object().expect("validated");
        let usage = UsageProjection {
            input_tokens: Some(Fact::Known {
                value: usage_fields["input"].as_u64().expect("validated"),
                origin: "provider_reported".to_owned(),
            }),
            output_tokens: Some(Fact::Known {
                value: usage_fields["output"].as_u64().expect("validated"),
                origin: "provider_reported".to_owned(),
            }),
        };

        // Terminal facts are complete; the requested-identity and stopReason
        // checks belong to the adapter's completion predicate. Retain the raw stream as a
        // content-addressed native artifact (P18-AGT-007: native facts are
        // kept, never dropped to make Agents look alike).
        let artifact_path = trace_root.join("pi-events.jsonl");
        if std::fs::create_dir_all(trace_root)
            .and_then(|_| std::fs::write(&artifact_path, &stdout.bytes))
            .is_err()
        {
            return failed("import-evidence-missing", FailureBoundaryCode::Evidence);
        }
        ImportedPi {
            completion: AgentCompletion::Completed {
                artifacts: vec![NativeArtifact {
                    role: "events/stdout".to_owned(),
                    sha256: sha256_hex(&stdout.bytes),
                    path: artifact_path,
                }],
            },
            stop_reason,
            provider,
            model,
            usage,
        }
    }
}

/// Deep validation of the terminal assistant `message_end` message: exact
/// projection fields with presence-only opaque fields.
fn validate_terminal_message(message: &serde_json::Value) -> Result<(), &'static str> {
    let Some(fields) = message.as_object() else {
        return Err("import-unsupported-schema");
    };
    if fields.get("role").and_then(|v| v.as_str()) != Some("assistant") {
        return Err("import-unsupported-schema");
    }
    for key in ["content", "api", "timestamp"] {
        if !fields.contains_key(key) {
            return Err("import-unsupported-schema");
        }
    }
    // Correlation identity: v2 samples carry `id`, the pinned 0.84.3
    // stream carries `responseId`; one of the two must be present.
    if !(fields.contains_key("id") || fields.contains_key("responseId")) {
        return Err("import-unsupported-schema");
    }
    if !fields.get("provider").is_some_and(|v| v.is_string())
        || !fields.get("model").is_some_and(|v| v.is_string())
    {
        return Err("import-unsupported-schema");
    }
    // `toolUse` is the tool-call termination the pinned stream emits on
    // intermediate assistant turns; the terminal turn still carries one
    // of the text terminations.
    if !matches!(
        fields.get("stopReason").and_then(|v| v.as_str()),
        Some("stop" | "length" | "error" | "aborted" | "toolUse")
    ) {
        return Err("import-unsupported-schema");
    }
    let usage = match fields.get("usage") {
        Some(usage) if usage_is_shape(usage) => usage,
        _ => return Err("import-unsupported-schema"),
    };
    let _ = usage;
    Ok(())
}

/// pi's `Usage` object shape: token counters plus a cost object.
fn usage_is_shape(usage: &serde_json::Value) -> bool {
    let Some(fields) = usage.as_object() else {
        return false;
    };
    for key in ["input", "output", "cacheRead", "cacheWrite", "totalTokens"] {
        if !fields.get(key).is_some_and(|v| v.is_u64()) {
            return false;
        }
    }
    fields.get("cost").is_some_and(|c| c.as_object().is_some())
}

/// sha256 hex digest of `bytes` (content-addressed native artifacts).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn request() -> AgentRunRequest {
        AgentRunRequest {
            executable: PathBuf::from("/tmp/fake-pi"),
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
                OsString::from("PI_EVAL_PROVIDER"),
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
        let profile = PiProfile::checked_in();
        assert_eq!(profile.schema, "phase18-agent-profile/1");
        assert_eq!(profile.product, "pi");
        assert_eq!(profile.package, "@earendil-works/pi-coding-agent");
        assert_eq!(profile.adapter, "opi-eval-pi-adapter/1");
        assert_eq!(profile.thinking_level, "off");

        // Any other schema identity fails closed.
        let mutated = include_str!("../../profiles/agents/pi.toml")
            .replace("phase18-agent-profile/1", "phase18-agent-profile/2");
        assert!(PiProfile::parse(&mutated).is_err());
        // Disabling a required launch invariant fails closed.
        let disabled = include_str!("../../profiles/agents/pi.toml")
            .replace("no_trust = true", "no_trust = false");
        assert!(PiProfile::parse(&disabled).is_err());
        // A non-pinned thinking level fails closed.
        let fuzzy = include_str!("../../profiles/agents/pi.toml")
            .replace("thinking_level = \"off\"", "thinking_level = \"xhigh\"");
        assert!(PiProfile::parse(&fuzzy).is_err());
    }

    #[test]
    fn pi_argv_is_exact_production_surface_with_no_ambient_resources() {
        let adapter = PiProcessAdapter::new();
        let spec = adapter.spawn_spec(&request());
        assert_eq!(
            argv_strings(&spec),
            vec![
                "/tmp/fake-pi",
                "--mode",
                "json",
                "--print",
                "--no-session",
                "--no-extensions",
                "--no-skills",
                "--no-context-files",
                "--no-approve",
                "--provider",
                "local",
                "--model",
                "scripted",
                "--thinking",
                "off",
                "--",
                "solve the task",
            ]
        );
        // The environment is exactly the isolated projection plus extras.
        let env = &spec.env;
        assert_eq!(env.len(), 5);
        assert_eq!(
            env[&OsString::from("HOME")],
            OsString::from("/tmp/iso/home")
        );
        assert_eq!(
            env[&OsString::from("USERPROFILE")],
            OsString::from("/tmp/iso/appdata")
        );
        assert_eq!(
            env[&OsString::from("PI_CODING_AGENT_SESSION_DIR")],
            OsString::from("/tmp/iso/sessions")
        );
        assert_eq!(
            env[&OsString::from("PI_CODING_AGENT_DIR")],
            std::path::Path::new("/tmp/iso/appdata")
                .join("pi-agent")
                .into_os_string()
        );
        assert_eq!(
            env[&OsString::from("PI_EVAL_PROVIDER")],
            OsString::from("scripted")
        );
        assert_eq!(spec.cwd.as_deref(), Some(std::path::Path::new("/tmp/ws")));
        assert_eq!(spec.timeout, std::time::Duration::from_secs(900));
        assert_eq!(spec.stdout_cap, 1048576);
        assert_eq!(spec.stderr_cap, 1048576);
    }

    #[test]
    fn identity_carries_the_pinned_package_and_request_executable() {
        let adapter = PiProcessAdapter::new();
        let identity = adapter.identity(&request());
        assert_eq!(identity.product, "pi");
        assert_eq!(identity.package, "@earendil-works/pi-coding-agent");
        assert_eq!(
            identity.adapter,
            "opi-eval-pi-adapter/1 (phase18-agent-profile/1)"
        );
        assert_eq!(identity.executable, PathBuf::from("/tmp/fake-pi"));
    }

    // Slice 2/3: importer acceptance, fail-closed matrix, and the
    // completion predicate over saved-bytes fixtures.
    fn fixture_bytes(name: &str) -> crate::process::OutputCapture {
        let bytes = std::fs::read(format!("tests/fixtures/agents/pi/{name}")).unwrap();
        crate::process::OutputCapture {
            bytes,
            truncated: false,
        }
    }

    fn settled_with(name: &str) -> crate::process::SupervisedOutcome {
        crate::process::SupervisedOutcome {
            exit: ExitState::Exited { code: 0 },
            stdout: fixture_bytes(name),
            stderr: crate::process::OutputCapture::default(),
            cleanup: crate::process::CleanupEvidence::NotRequired,
        }
    }

    #[test]
    fn importer_accepts_the_pinned_stream_and_keeps_it_as_native_artifact() {
        let adapter = PiProcessAdapter::new();
        let dir = tempfile::tempdir().unwrap();
        let mut request = request();
        request.trace_root = dir.path().to_path_buf();
        let (usage, completion) = adapter.settle(&settled_with("stream-ok.jsonl"), &request);

        let AgentCompletion::Completed { artifacts } = &completion else {
            panic!("expected completed, got {completion:?}");
        };
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "events/stdout");
        assert_eq!(artifacts[0].path, dir.path().join("pi-events.jsonl"));
        let written = std::fs::read(&artifacts[0].path).unwrap();
        assert_eq!(written, fixture_bytes("stream-ok.jsonl").bytes);
        assert_eq!(artifacts[0].sha256.len(), 64);
        assert_eq!(
            usage.input_tokens,
            Some(Fact::Known {
                value: 12,
                origin: "provider_reported".to_owned(),
            })
        );
        assert_eq!(
            usage.output_tokens,
            Some(Fact::Known {
                value: 34,
                origin: "provider_reported".to_owned(),
            })
        );
    }

    #[test]
    fn importer_fails_closed_on_drift_corruption_and_ambiguity() {
        let adapter = PiProcessAdapter::new();
        let dir = tempfile::tempdir().unwrap();
        let mut request = request();
        request.trace_root = dir.path().to_path_buf();
        for (name, kind, boundary) in [
            (
                "corrupt.jsonl",
                "import-parse-failure",
                FailureBoundaryCode::Adapter,
            ),
            (
                "unknown-event.jsonl",
                "import-unsupported-schema",
                FailureBoundaryCode::Adapter,
            ),
            (
                "old-header.jsonl",
                "import-unsupported-schema",
                FailureBoundaryCode::Adapter,
            ),
            (
                "multiple-agent-end.jsonl",
                "import-ambiguous-stream",
                FailureBoundaryCode::Adapter,
            ),
            (
                "no-agent-end.jsonl",
                "import-evidence-missing",
                FailureBoundaryCode::Evidence,
            ),
            // A first line that is not the session header is pinned-surface
            // drift, not absence; an empty stdout is the missing case.
            (
                "no-header.jsonl",
                "import-unsupported-schema",
                FailureBoundaryCode::Adapter,
            ),
            (
                "model-mismatch.jsonl",
                "agent-model-mismatch",
                FailureBoundaryCode::Evidence,
            ),
        ] {
            let (_, completion) = adapter.settle(&settled_with(name), &request);
            assert_eq!(
                completion,
                AgentCompletion::Failed(AgentFailure { kind, boundary }),
                "fixture {name}"
            );
        }
    }

    #[test]
    fn json_mode_exit_zero_never_conceals_a_failed_terminal_message() {
        // pi's JSON mode exits 0 even when the terminal assistant message
        // reports error/aborted/length; the predicate reads the message
        // (P18-AGT-005).
        let adapter = PiProcessAdapter::new();
        let dir = tempfile::tempdir().unwrap();
        let mut request = request();
        request.trace_root = dir.path().to_path_buf();
        for name in [
            "stream-error.jsonl",
            "stream-abort.jsonl",
            "stream-length.jsonl",
        ] {
            let (_, completion) = adapter.settle(&settled_with(name), &request);
            assert_eq!(
                completion,
                AgentCompletion::Failed(AgentFailure {
                    kind: "agent-outcome-not-success",
                    boundary: FailureBoundaryCode::AgentProcess,
                }),
                "fixture {name}"
            );
        }

        // A cap-truncated stream can never prove a terminal event.
        let truncated = crate::process::SupervisedOutcome {
            stdout: crate::process::OutputCapture {
                bytes: fixture_bytes("stream-ok.jsonl").bytes,
                truncated: true,
            },
            ..settled_zero()
        };
        let (_, completion) = adapter.settle(&truncated, &request);
        assert_eq!(
            completion,
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-incomplete",
                boundary: FailureBoundaryCode::Evidence,
            })
        );

        // Empty stdout: the agent produced no evidence at all.
        let empty = crate::process::SupervisedOutcome {
            stdout: crate::process::OutputCapture::default(),
            ..settled_zero()
        };
        let (_, completion) = adapter.settle(&empty, &request);
        assert_eq!(
            completion,
            AgentCompletion::Failed(AgentFailure {
                kind: "import-evidence-missing",
                boundary: FailureBoundaryCode::Evidence,
            })
        );

        // Non-zero exit is authoritative regardless of a healthy stream.
        let failed_exit = crate::process::SupervisedOutcome {
            exit: ExitState::Exited { code: 3 },
            ..settled_with("stream-ok.jsonl")
        };
        let (usage, completion) = adapter.settle(&failed_exit, &request);
        assert_eq!(
            completion,
            AgentCompletion::Failed(super::super::process::failure_kinds::non_zero_exit(3))
        );
        assert_eq!(usage.input_tokens, None);

        // Timeout and cancellation map through the shared failure kinds.
        for exit in [ExitState::TimedOut, ExitState::Cancelled] {
            let outcome = crate::process::SupervisedOutcome {
                exit,
                ..settled_with("stream-ok.jsonl")
            };
            let (_, completion) = adapter.settle(&outcome, &request);
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
    fn fake_pi_script(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let script = dir.join(name);
        std::fs::write(&script, format!("#!/bin/sh\n{body}\nexit 0\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[cfg(unix)]
    fn e2e_request(dir: &std::path::Path, executable: PathBuf) -> AgentRunRequest {
        let mut request = request();
        request.executable = executable;
        request.workspace = dir.join("ws");
        request.trace_root = dir.join("trace");
        request.isolation = super::super::process::IsolationDirs {
            home: dir.join("iso/home"),
            app_data: dir.join("iso/appdata"),
            sessions: dir.join("iso/sessions"),
        };
        request.extra_env.insert(
            OsString::from("PI_EVAL_STREAM_SOURCE"),
            std::fs::canonicalize("tests/fixtures/agents/pi/stream-ok.jsonl")
                .unwrap()
                .into(),
        );
        std::fs::create_dir_all(&request.workspace).unwrap();
        request
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_pi_process_settles_a_completed_record_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = PiProcessAdapter::new();
        let script = fake_pi_script(dir.path(), "fake-pi", "cat \"$PI_EVAL_STREAM_SOURCE\"");
        let request = e2e_request(dir.path(), script);

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
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "events/stdout");
        assert!(artifacts[0].path.starts_with(dir.path()));
        assert_eq!(
            record.usage.input_tokens,
            Some(Fact::Known {
                value: 12,
                origin: "provider_reported".to_owned(),
            })
        );
        assert_eq!(record.identity.product, "pi");
        assert_eq!(record.workspace, request.workspace);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_pi_exit_failure_and_spawn_failure_settle_typed() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = PiProcessAdapter::new();
        let script = fake_pi_script(
            dir.path(),
            "fake-pi-fail",
            "cat \"$PI_EVAL_STREAM_SOURCE\"\nexit 3",
        );
        let request = e2e_request(dir.path(), script);

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
            AgentCompletion::Failed(super::super::process::failure_kinds::non_zero_exit(3))
        );
        // A healthy stream cannot rescue a non-zero exit.
        assert!(!record.stdout.bytes.is_empty());

        // A nonexistent executable settles as a redacted spawn failure.
        let mut missing = request.clone();
        missing.executable = dir.path().join("no-such-pi");
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
        assert!(!format!("{record:?}").contains("no-such-pi"));
    }

    fn settled_zero() -> crate::process::SupervisedOutcome {
        crate::process::SupervisedOutcome {
            exit: ExitState::Exited { code: 0 },
            stdout: crate::process::OutputCapture::default(),
            stderr: crate::process::OutputCapture::default(),
            cleanup: crate::process::CleanupEvidence::NotRequired,
        }
    }
}

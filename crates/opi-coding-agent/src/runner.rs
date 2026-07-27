//! Non-interactive runner (S10).
//!
//! Takes a single prompt, runs it through the agent, captures assistant text
//! for stdout, diagnostics for stderr, and returns an exit code.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use opi_agent::event::AgentEvent;
use opi_agent::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_agent::session_event::{AgentSessionEvent, SessionCostTotals, SessionTokenTotals};
use opi_agent::{Diagnostic, FileTraceSink, RedactionMode, redact_text};
use opi_ai::message::InputContent;
use opi_ai::message::Message;
use opi_ai::provider::Provider;
use opi_ai::stream::AssistantStreamEvent;

use crate::config::OpiConfig;
use crate::harness::{CodingHarness, ResumeInfo, TraceConfig};
use crate::policy::{RunMode, ToolPolicyError, ToolRuntimeConfig, ToolSelection, is_mutating_tool};
use crate::project_trust::TrustDecision;
use crate::runtime_packages::RuntimePackageStartup;

/// NDJSON output schema version.
pub const NDJSON_SCHEMA_VERSION: u32 = 2;

// ---------------------------------------------------------------------------
// Exit codes (S10)
// ---------------------------------------------------------------------------

/// Exit codes for the non-interactive runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    RuntimeFailure = 1,
    ConfigError = 2,
    AuthFailure = 3,
    ProviderFailure = 4,
    ToolFailure = 5,
    Interrupted = 130,
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Captured output from a non-interactive run.
#[derive(Debug, Clone)]
pub struct NonInteractiveResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Non-interactive runner that executes a single prompt and captures output.
pub struct NonInteractiveRunner {
    harness: CodingHarness,
    compact_ndjson: bool,
}

/// Serialize a session event for NDJSON, applying compact-mode stripping to
/// `text_delta` MessageUpdate lines. Other lines pass through unchanged.
fn compact_ndjson_line(session_event: &AgentSessionEvent) -> Option<String> {
    let mut value = serde_json::to_value(session_event).ok()?;
    let inner = value.get("event")?;
    let is_compact_target = value.get("type").and_then(|v| v.as_str()) == Some("Agent")
        && inner.get("type").and_then(|v| v.as_str()) == Some("MessageUpdate")
        && inner
            .get("assistant_event")
            .and_then(|a| a.get("type"))
            .and_then(|v| v.as_str())
            == Some("text_delta");
    if !is_compact_target {
        return serde_json::to_string(&value).ok();
    }
    let event_obj = value.get_mut("event").and_then(|e| e.as_object_mut())?;
    // 1) Drop the redundant cumulative snapshot carried on the assistant_event.
    if let Some(ae) = event_obj
        .get_mut("assistant_event")
        .and_then(|a| a.as_object_mut())
    {
        ae.remove("partial");
    }
    // 2) Empty the cumulative text in event.message so the line is constant-size.
    if let Some(content) = event_obj
        .get_mut("message")
        .and_then(|m| m.get_mut("content"))
        .and_then(|c| c.as_array_mut())
    {
        for block in content.iter_mut() {
            if let Some(obj) = block.as_object_mut()
                && obj.get("type").and_then(|v| v.as_str()) == Some("text")
            {
                obj.insert("text".into(), serde_json::Value::String(String::new()));
            }
        }
    }
    serde_json::to_string(&value).ok()
}

impl NonInteractiveRunner {
    /// Create a new non-interactive runner.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        trust_decision: TrustDecision,
    ) -> Self {
        Self::new_with_resume(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            user_system_prompt,
            initial_messages,
            None,
            ToolSelection::Default,
            trust_decision,
        )
        .expect("default non-interactive tool policy should be valid")
    }

    /// Create a new non-interactive runner, optionally adopting an existing
    /// session (resume).
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_resume(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume_info: Option<ResumeInfo>,
        tool_selection: ToolSelection,
        trust_decision: TrustDecision,
    ) -> Result<Self, ToolPolicyError> {
        Self::build(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            user_system_prompt,
            initial_messages,
            resume_info,
            tool_selection,
            None,
            None,
            trust_decision,
        )
    }

    /// Create a non-interactive runner with installed package adapters already
    /// started by runtime startup. The startup's trust decision is the single
    /// source of truth for both package and harness resource loading.
    /// `trace_path`, when set, writes a versioned redacted trace envelope to
    /// that file for the run (Phase 7 task 7.5).
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_resume_and_runtime_packages(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume_info: Option<ResumeInfo>,
        tool_selection: ToolSelection,
        runtime_startup: RuntimePackageStartup,
        trace_path: Option<PathBuf>,
    ) -> Result<Self, ToolPolicyError> {
        let trust_decision = runtime_startup.trust_decision;
        Self::build(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            user_system_prompt,
            initial_messages,
            resume_info,
            tool_selection,
            Some(runtime_startup),
            trace_path,
            trust_decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume_info: Option<ResumeInfo>,
        tool_selection: ToolSelection,
        runtime_startup: Option<RuntimePackageStartup>,
        trace_path: Option<PathBuf>,
        trust_decision: TrustDecision,
    ) -> Result<Self, ToolPolicyError> {
        let tool_config = ToolRuntimeConfig::resolve(
            RunMode::NonInteractive,
            allow_mutating,
            tool_selection.clone(),
        )?;
        let hooks = Box::new(NonInteractiveHooks { allow_mutating });
        let mut builder =
            CodingHarness::builder(provider, model, config, workspace_root, trust_decision)
                .hooks(hooks)
                .initial_messages(initial_messages)
                .tool_selection(tool_selection)
                .tool_config(tool_config)
                // Record runtime diagnostics so the JSON run summary can carry
                // structured severity counts (Phase 7 task 7.5).
                .record_diagnostics(true);
        if let Some(prompt) = user_system_prompt {
            builder = builder.user_system_prompt(prompt);
        }
        if let Some(resume_info) = resume_info {
            builder = builder.resume(resume_info);
        }
        if let Some(runtime_startup) = runtime_startup {
            builder = builder
                .extension_registry(runtime_startup.extension_registry)
                .installed_packages(runtime_startup.installed_packages)
                .startup_diagnostics(runtime_startup.diagnostics);
        }
        if let Some(path) = trace_path {
            builder = builder.trace(Some(TraceConfig {
                sink: Arc::new(FileTraceSink::new(path)),
                mode: RedactionMode::Summary,
            }));
        }
        let harness = builder.build();
        Ok(Self {
            harness,
            compact_ndjson: false,
        })
    }

    /// Enable compact NDJSON output (opt-in `--json-compact`). Streamed
    /// `text_delta` updates become constant-size, so a long streamed turn
    /// scales ~linearly in bytes. Default `--json` output is unchanged.
    pub fn with_compact_ndjson(mut self, enabled: bool) -> Self {
        self.compact_ndjson = enabled;
        self
    }

    /// Run a single prompt in JSON mode, returning NDJSON output in stdout.
    pub async fn run_json(&mut self, prompt: &str) -> NonInteractiveResult {
        let output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

        // Schema version header line
        {
            let header = serde_json::json!({
                "type": "session_header",
                "schema_version": NDJSON_SCHEMA_VERSION,
            });
            let mut out = output.lock().unwrap();
            out.push_str(&header.to_string());
            out.push('\n');
        }

        // Phase 7 task 7.5: surface startup diagnostics BEFORE accepted prompt
        // output (the first agent event is AgentStart). Additive NDJSON line.
        {
            let startup = AgentSessionEvent::StartupDiagnostics {
                diagnostics: self
                    .harness
                    .resource_metadata()
                    .diagnostic_payloads(RedactionMode::Summary),
            };
            if let Ok(json) = serde_json::to_string(&startup)
                && let Ok(mut guard) = output.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }

        let compact = self.compact_ndjson;
        let out = output.clone();
        self.harness.subscribe(Box::new(move |event| {
            let session_event = match event {
                AgentEvent::AutoRetryStart {
                    attempt,
                    max_attempts,
                    delay_ms,
                    error_message,
                } => AgentSessionEvent::AutoRetryStart {
                    attempt: *attempt,
                    max_attempts: *max_attempts,
                    delay_ms: *delay_ms,
                    error_message: error_message.clone(),
                },
                AgentEvent::AutoRetryEnd {
                    success,
                    attempt,
                    final_error,
                } => AgentSessionEvent::AutoRetryEnd {
                    success: *success,
                    attempt: *attempt,
                    final_error: final_error.clone(),
                },
                AgentEvent::CompactionStart { reason } => {
                    AgentSessionEvent::CompactionStart { reason: *reason }
                }
                AgentEvent::CompactionEnd {
                    reason,
                    result,
                    aborted,
                    error_message,
                } => AgentSessionEvent::CompactionEnd {
                    reason: *reason,
                    result: result.clone(),
                    aborted: *aborted,
                    will_retry: false,
                    error_message: error_message.clone(),
                },
                _ => AgentSessionEvent::Agent {
                    event: event.clone(),
                },
            };
            let json_opt = if compact {
                compact_ndjson_line(&session_event)
            } else {
                serde_json::to_string(&session_event).ok()
            };
            if let Some(json) = json_opt
                && let Ok(mut guard) = out.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }));

        let provider_turns = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let turn_counter = Arc::clone(&provider_turns);
        self.harness.subscribe(Box::new(move |event| {
            if matches!(event, AgentEvent::TurnStart) {
                turn_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }));

        let prompt_result = self.harness.prompt(prompt).await;

        // Emit a final `SessionSummary` event with cumulative token totals
        // and (when known) cost breakdown. Emitted before the result match so
        // even error paths surface what the user spent before failing.
        if let Some(session) = self.harness.session() {
            let usage = session.usage();
            let cost = session.cost_summary().map(|c| SessionCostTotals {
                input: c.input_cost,
                output: c.output_cost,
                cache_read: c.cache_read_cost,
                cache_write: c.cache_write_cost,
                total: c.total_cost(),
            });
            let summary_event = AgentSessionEvent::SessionSummary {
                session_id: session.session_id().to_owned(),
                model: session.model().to_owned(),
                turns: usage.turn_count(),
                provider_turns: provider_turns.load(std::sync::atomic::Ordering::Relaxed),
                tokens: SessionTokenTotals {
                    input: usage.total_input_tokens(),
                    output: usage.total_output_tokens(),
                    cache_read: usage.total_cache_read_tokens(),
                    cache_write: usage.total_cache_write_tokens(),
                },
                cost_usd: cost,
                diagnostics: self.harness.diagnostic_counts(),
            };
            if let Ok(json) = serde_json::to_string(&summary_event)
                && let Ok(mut guard) = output.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }

        match prompt_result {
            Ok(messages) => {
                if let Some(error) = find_error_message(&messages) {
                    return NonInteractiveResult {
                        stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                        stderr: error,
                        exit_code: ExitCode::ProviderFailure as i32,
                    };
                }
                NonInteractiveResult {
                    stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                    stderr: String::new(),
                    exit_code: ExitCode::Success as i32,
                }
            }
            Err(error) => {
                append_credential_remediation(&output, &error);
                NonInteractiveResult {
                    stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                    stderr: stderr_for_agent_error(&error, ""),
                    exit_code: exit_code_for_agent_error(&error),
                }
            }
        }
    }

    /// Run with arbitrary content (text + images) and return captured output.
    pub async fn run_with_content(&mut self, content: Vec<InputContent>) -> NonInteractiveResult {
        let text_parts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let persist_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let tp = text_parts.clone();
        let pe = persist_errors.clone();
        self.harness.subscribe(Box::new(move |event| match event {
            AgentEvent::MessageUpdate {
                assistant_event, ..
            } => {
                if let AssistantStreamEvent::TextDelta { delta, .. } = assistant_event.as_ref()
                    && let Ok(mut guard) = tp.lock()
                {
                    guard.push(delta.clone());
                }
            }
            AgentEvent::SessionPersistError { message } => {
                if let Ok(mut guard) = pe.lock() {
                    guard.push(message.clone());
                }
            }
            _ => {}
        }));

        let prompt_result = self.harness.prompt_with_content(content).await;
        let persist_stderr = format_persist_errors(&persist_errors);

        match prompt_result {
            Ok(messages) => {
                if let Some(error) = find_error_message(&messages) {
                    let mut stderr = error;
                    stderr.push_str(&persist_stderr);
                    return NonInteractiveResult {
                        stdout: String::new(),
                        stderr,
                        exit_code: ExitCode::ProviderFailure as i32,
                    };
                }

                let stdout = text_parts.lock().map(|g| g.join("")).unwrap_or_default();
                NonInteractiveResult {
                    stdout,
                    stderr: persist_stderr,
                    exit_code: ExitCode::Success as i32,
                }
            }
            Err(error) => NonInteractiveResult {
                stdout: String::new(),
                stderr: stderr_for_agent_error(&error, &persist_stderr),
                exit_code: exit_code_for_agent_error(&error),
            },
        }
    }

    /// Run with arbitrary content in JSON mode, returning NDJSON output.
    pub async fn run_json_with_content(
        &mut self,
        content: Vec<InputContent>,
    ) -> NonInteractiveResult {
        let output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

        {
            let header = serde_json::json!({
                "type": "session_header",
                "schema_version": NDJSON_SCHEMA_VERSION,
            });
            let mut out = output.lock().unwrap();
            out.push_str(&header.to_string());
            out.push('\n');
        }

        // Phase 7 task 7.5: surface startup diagnostics BEFORE accepted prompt
        // output (the first agent event is AgentStart). Additive NDJSON line.
        {
            let startup = AgentSessionEvent::StartupDiagnostics {
                diagnostics: self
                    .harness
                    .resource_metadata()
                    .diagnostic_payloads(RedactionMode::Summary),
            };
            if let Ok(json) = serde_json::to_string(&startup)
                && let Ok(mut guard) = output.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }

        let compact = self.compact_ndjson;
        let out = output.clone();
        self.harness.subscribe(Box::new(move |event| {
            let session_event = match event {
                AgentEvent::AutoRetryStart {
                    attempt,
                    max_attempts,
                    delay_ms,
                    error_message,
                } => AgentSessionEvent::AutoRetryStart {
                    attempt: *attempt,
                    max_attempts: *max_attempts,
                    delay_ms: *delay_ms,
                    error_message: error_message.clone(),
                },
                AgentEvent::AutoRetryEnd {
                    success,
                    attempt,
                    final_error,
                } => AgentSessionEvent::AutoRetryEnd {
                    success: *success,
                    attempt: *attempt,
                    final_error: final_error.clone(),
                },
                AgentEvent::CompactionStart { reason } => {
                    AgentSessionEvent::CompactionStart { reason: *reason }
                }
                AgentEvent::CompactionEnd {
                    reason,
                    result,
                    aborted,
                    error_message,
                } => AgentSessionEvent::CompactionEnd {
                    reason: *reason,
                    result: result.clone(),
                    aborted: *aborted,
                    will_retry: false,
                    error_message: error_message.clone(),
                },
                _ => AgentSessionEvent::Agent {
                    event: event.clone(),
                },
            };
            let json_opt = if compact {
                compact_ndjson_line(&session_event)
            } else {
                serde_json::to_string(&session_event).ok()
            };
            if let Some(json) = json_opt
                && let Ok(mut guard) = out.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }));

        let provider_turns = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let turn_counter = Arc::clone(&provider_turns);
        self.harness.subscribe(Box::new(move |event| {
            if matches!(event, AgentEvent::TurnStart) {
                turn_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }));

        let prompt_result = self.harness.prompt_with_content(content).await;

        if let Some(session) = self.harness.session() {
            let usage = session.usage();
            let cost = session.cost_summary().map(|c| SessionCostTotals {
                input: c.input_cost,
                output: c.output_cost,
                cache_read: c.cache_read_cost,
                cache_write: c.cache_write_cost,
                total: c.total_cost(),
            });
            let summary_event = AgentSessionEvent::SessionSummary {
                session_id: session.session_id().to_owned(),
                model: session.model().to_owned(),
                turns: usage.turn_count(),
                provider_turns: provider_turns.load(std::sync::atomic::Ordering::Relaxed),
                tokens: SessionTokenTotals {
                    input: usage.total_input_tokens(),
                    output: usage.total_output_tokens(),
                    cache_read: usage.total_cache_read_tokens(),
                    cache_write: usage.total_cache_write_tokens(),
                },
                cost_usd: cost,
                diagnostics: self.harness.diagnostic_counts(),
            };
            if let Ok(json) = serde_json::to_string(&summary_event)
                && let Ok(mut guard) = output.lock()
            {
                guard.push_str(&json);
                guard.push('\n');
            }
        }

        match prompt_result {
            Ok(messages) => {
                if let Some(error) = find_error_message(&messages) {
                    return NonInteractiveResult {
                        stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                        stderr: error,
                        exit_code: ExitCode::ProviderFailure as i32,
                    };
                }
                NonInteractiveResult {
                    stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                    stderr: String::new(),
                    exit_code: ExitCode::Success as i32,
                }
            }
            Err(error) => {
                append_credential_remediation(&output, &error);
                NonInteractiveResult {
                    stdout: output.lock().map(|g| g.clone()).unwrap_or_default(),
                    stderr: stderr_for_agent_error(&error, ""),
                    exit_code: exit_code_for_agent_error(&error),
                }
            }
        }
    }

    /// Cancel the running operation.
    pub fn cancel(&self) {
        self.harness.cancel();
    }

    /// Return the session coordinator, if active.
    pub fn session(&self) -> Option<&crate::session_coordinator::SessionCoordinator> {
        self.harness.session()
    }

    /// Run a single prompt and return captured output.
    pub async fn run(&mut self, prompt: &str) -> NonInteractiveResult {
        // Subscribe to capture text from TextDelta events and persist errors
        let text_parts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let persist_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let tp = text_parts.clone();
        let pe = persist_errors.clone();
        self.harness.subscribe(Box::new(move |event| match event {
            AgentEvent::MessageUpdate {
                assistant_event, ..
            } => {
                if let AssistantStreamEvent::TextDelta { delta, .. } = assistant_event.as_ref()
                    && let Ok(mut guard) = tp.lock()
                {
                    guard.push(delta.clone());
                }
            }
            AgentEvent::SessionPersistError { message } => {
                if let Ok(mut guard) = pe.lock() {
                    guard.push(message.clone());
                }
            }
            _ => {}
        }));

        let prompt_result = self.harness.prompt(prompt).await;

        // Format persist errors AFTER prompt returns so events emitted
        // during the run are captured.
        let persist_stderr = format_persist_errors(&persist_errors);

        match prompt_result {
            Ok(messages) => {
                // Check for provider errors in assistant messages
                if let Some(error) = find_error_message(&messages) {
                    let mut stderr = error;
                    stderr.push_str(&persist_stderr);
                    return NonInteractiveResult {
                        stdout: String::new(),
                        stderr,
                        exit_code: ExitCode::ProviderFailure as i32,
                    };
                }

                let stdout = text_parts.lock().map(|g| g.join("")).unwrap_or_default();
                NonInteractiveResult {
                    stdout,
                    stderr: persist_stderr,
                    exit_code: ExitCode::Success as i32,
                }
            }
            Err(error) => NonInteractiveResult {
                stdout: String::new(),
                stderr: stderr_for_agent_error(&error, &persist_stderr),
                exit_code: exit_code_for_agent_error(&error),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the first error_message in assistant messages.
fn find_error_message(messages: &[AgentMessage]) -> Option<String> {
    for msg in messages {
        if let AgentMessage::Llm(Message::Assistant(asst)) = msg
            && let Some(err) = &asst.error_message
        {
            return Some(provider_error_stderr(err));
        }
    }
    None
}

fn provider_error_stderr(error: &str) -> String {
    let error = AgentError::Provider(error.to_owned());
    Diagnostic::from(&error)
        .redacted_payload(RedactionMode::Summary)
        .message
}

fn append_credential_remediation(output: &Arc<Mutex<String>>, error: &AgentError) {
    let event = match error {
        AgentError::CredentialNeeded { provider_id } => AgentSessionEvent::CredentialNeeded {
            provider_id: provider_id.clone(),
            remediation: format!("/login {provider_id}"),
            diagnostic: Diagnostic::from(error).redacted_payload(RedactionMode::Summary),
        },
        AgentError::CredentialRevoked { provider_id } => AgentSessionEvent::CredentialRevoked {
            provider_id: provider_id.clone(),
            remediation: format!("/login {provider_id}"),
            diagnostic: Diagnostic::from(error).redacted_payload(RedactionMode::Summary),
        },
        // AccountIdMissing is an auth-category failure resolved by re-login.
        // Surface it as a CredentialNeeded remediation event (carrying the
        // AccountIdMissing diagnostic) so JSON/text embedders drive the same
        // /login <provider> re-auth path. Kept distinct from CredentialRevoked.
        AgentError::AccountIdMissing { provider_id } => AgentSessionEvent::CredentialNeeded {
            provider_id: provider_id.clone(),
            remediation: format!("/login {provider_id}"),
            diagnostic: Diagnostic::from(error).redacted_payload(RedactionMode::Summary),
        },
        _ => return,
    };
    if let Ok(json) = serde_json::to_string(&event)
        && let Ok(mut guard) = output.lock()
    {
        guard.push_str(&json);
        guard.push('\n');
    }
}

fn stderr_for_agent_error(error: &AgentError, suffix: &str) -> String {
    let mut stderr = match error {
        AgentError::Cancelled => "cancelled".to_owned(),
        _ => {
            Diagnostic::from(error)
                .redacted_payload(RedactionMode::Summary)
                .message
        }
    };
    stderr.push_str(suffix);
    stderr
}

fn exit_code_for_agent_error(error: &AgentError) -> i32 {
    match error {
        AgentError::Cancelled => ExitCode::Interrupted as i32,
        AgentError::AuthFailed(_)
        | AgentError::CredentialNeeded { .. }
        | AgentError::CredentialRevoked { .. }
        | AgentError::AccountIdMissing { .. } => ExitCode::AuthFailure as i32,
        AgentError::Provider(_) => ExitCode::ProviderFailure as i32,
        AgentError::Tool(_) => ExitCode::ToolFailure as i32,
        AgentError::Hook(_) | AgentError::MaxTurnsExceeded(_) | AgentError::TraceSetup(_) => {
            ExitCode::RuntimeFailure as i32
        }
    }
}

/// Format any captured session persist errors into a stderr suffix.
pub fn format_persist_errors(errors: &Arc<Mutex<Vec<String>>>) -> String {
    let guard = errors.lock().unwrap();
    if guard.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for e in guard.iter() {
        out.push_str("\nsession persist error: ");
        out.push_str(&redact_text(e, RedactionMode::Summary));
    }
    out
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/// Hooks for non-interactive mode with tool safety policy.
pub struct NonInteractiveHooks {
    allow_mutating: bool,
}

impl NonInteractiveHooks {
    /// Create new hooks for non-interactive / RPC mode.
    pub fn new(allow_mutating: bool) -> Self {
        Self { allow_mutating }
    }
}

impl AgentHooks for NonInteractiveHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(crate::harness::agent_messages_to_llm(messages))
    }

    fn before_tool_call(
        &self,
        ctx: BeforeToolCallContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        let allowed = self.allow_mutating;
        let tool_name = ctx.tool_name.clone();
        Box::pin(async move {
            if !allowed && is_mutating_tool(&tool_name) {
                return BeforeToolCallResult::Deny {
                    reason: format!(
                        "tool '{}' is not allowed in non-interactive mode without --allow-mutating",
                        tool_name
                    ),
                };
            }
            BeforeToolCallResult::Allow
        })
    }

    fn after_tool_call(
        &self,
        _ctx: AfterToolCallContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }

    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    fn prepare_next_turn(
        &self,
        _ctx: PrepareNextTurnContext,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Option<opi_agent::loop_types::AgentLoopTurnUpdate>>
                + Send,
        >,
    > {
        Box::pin(async { None })
    }
}

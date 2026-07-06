//! RPC JSONL mode: bidirectional command/event protocol over stdin/stdout.
//!
//! RPC mode enables headless operation of the coding agent via a strict JSONL
//! protocol. Commands arrive on stdin (one JSON object per line), responses
//! and events are emitted on stdout (one JSON object per line). Startup
//! diagnostics (package/adapter degraded-path diagnostics) are surfaced in the
//! `rpc_ready` header's `startup_diagnostics` array and via the `session_info`
//! command's `resources.diagnostics`.
//!
//! # Protocol version
//!
//! This is an unstable 0.x protocol. The schema may change between minor
//! versions without notice. Clients MUST check `schema_version` in the
//! `rpc_ready` header.
//!
//! # Framing
//!
//! LF (`\n`) is the only record delimiter. Clients MUST split on `\n` only
//! and SHOULD strip a trailing `\r` if present.
//!
//! # Commands
//!
//! | Command           | Description                                      |
//! |-------------------|--------------------------------------------------|
//! | `prompt`          | Send user prompt, stream agent events            |
//! | `continue`        | Continue conversation with additional text       |
//! | `steer`           | Queue steering message during agent operation    |
//! | `follow_up`       | Queue follow-up message for after agent stops    |
//! | `abort`           | Cancel current agent operation                   |
//! | `set_model`       | Switch provider:model                            |
//! | `set_thinking_level` | Set reasoning/thinking level                  |
//! | `compact`         | Trigger manual compaction                        |
//! | `session_info`    | Query session metadata                           |
//! | `extension_command` | Dispatch a command to registered extensions    |
//! | `trace`           | Request the versioned redacted trace envelope    |
//! | `quit`            | Shut down the RPC session                        |
//!
//! # Responses and Errors
//!
//! Every command produces at most one `response` object. For `prompt` and
//! `continue`, `success: true` means the turn was accepted; subsequent agent
//! output arrives as asynchronous event lines. Errors after acceptance are
//! surfaced as events, not as a second response.
//!
//! `abort` cancels the active operation and succeeds immediately when a turn is
//! running. A second `abort` while idle is a successful no-op.
//!
//! # Structured error codes
//!
//! Runtime-contract failures carry a stable machine-readable `error_code` on
//! the response (additive on `SdkResponse::error_code`; the SDK schema version
//! is unchanged). The codes are:
//!
//! | `error_code` | Meaning |
//! |---|---|
//! | `unsupported_trace_request` | `trace` issued on a session without a trace sink |
//! | `agent_busy` | a run is already active (starting a run, or a mutating command while running) |
//! | `harness_unavailable` | no coding harness is attached to the runner |
//! | `compaction_failed` | a manual compaction returned an error |
//! | `extension_command_not_handled` | no registered extension handled the command |
//!
//! Idle capability-validation errors from `set_model` / `set_thinking_level`
//! (cross-provider, malformed spec, unknown model) remain free-text: they are
//! capability errors, not runtime-state failures.

use std::io::{self, BufRead, Write as IoWrite};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use opi_agent::agent::AgentControl;
use opi_agent::diagnostic::Diagnostic;
use opi_agent::event::AgentEvent;
use opi_agent::extension::ExtensionRegistry;
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_agent::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse, agent_event_to_value};
use opi_agent::session_event::CompactionReason;
use opi_agent::{RecordingTraceSink, RedactionMode, TRACE_SCHEMA_VERSION, redact_text};
use opi_ai::provider::Provider;

use crate::config::OpiConfig;
use crate::harness::{CodingHarness, ResumeInfo, TraceConfig};
use crate::policy::{RunMode, ToolSelection};
use crate::runner::ExitCode;
use crate::runtime_packages::RuntimePackageStartup;

const ACTIVE_RUN_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Stable machine-readable error codes for RPC runtime-contract failures.
/// Additive on [`SdkResponse::error_code`]; the SDK schema version is unchanged.
const ERR_AGENT_BUSY: &str = "agent_busy";
const ERR_HARNESS_UNAVAILABLE: &str = "harness_unavailable";
const ERR_COMPACTION_FAILED: &str = "compaction_failed";
const ERR_EXTENSION_COMMAND_NOT_HANDLED: &str = "extension_command_not_handled";

/// Re-export the SDK command type as the RPC command type.
pub type RpcCommand = SdkCommand;

/// Re-export the SDK schema version for crate-level access (e.g. tests).
pub const RPC_SCHEMA_VERSION: u32 = SDK_SCHEMA_VERSION;

enum RpcInput {
    Command(SdkCommand),
    ParseError(String),
}

enum ActiveRun {
    Prompt(String),
    Continue(String),
}

type RunResult = (CodingHarness, Result<Vec<AgentMessage>, AgentError>);

/// RPC runner that owns the harness and processes commands.
pub struct RpcRunner {
    harness: Option<CodingHarness>,
    control: AgentControl,
    running: bool,
    /// Optional recording trace sink. When set, runs are traced and the
    /// `trace` command returns the accumulated versioned redacted envelope;
    /// when unset, `trace` returns a structured unsupported error.
    trace_sink: Option<Arc<RecordingTraceSink>>,
}

impl RpcRunner {
    /// Create a new RPC runner.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        tool_selection: ToolSelection,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        Self::new_with_optional_extension_registry(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            tool_selection,
            user_system_prompt,
            initial_messages,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
    }

    /// Create a new RPC runner with an optional recording trace sink (Phase 7
    /// task 7.5). When set, runs are traced and the `trace` command returns the
    /// versioned redacted envelope; when `None`, `trace` returns a structured
    /// unsupported error.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_trace(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        tool_selection: ToolSelection,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        trace_sink: Option<Arc<RecordingTraceSink>>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        Self::new_with_optional_extension_registry(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            tool_selection,
            user_system_prompt,
            initial_messages,
            None,
            None,
            None,
            Vec::new(),
            trace_sink,
        )
    }

    /// Create a new RPC runner with an in-process extension registry.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_extension_registry(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        tool_selection: ToolSelection,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        extension_registry: ExtensionRegistry,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        Self::new_with_optional_extension_registry(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            tool_selection,
            user_system_prompt,
            initial_messages,
            None,
            Some(extension_registry),
            None,
            Vec::new(),
            None,
        )
    }

    /// Create a new RPC runner with installed package adapters already started.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_runtime_packages(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        tool_selection: ToolSelection,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        runtime_startup: RuntimePackageStartup,
        resume_info: Option<ResumeInfo>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        let RuntimePackageStartup {
            extension_registry,
            installed_packages,
            diagnostics,
        } = runtime_startup;
        Self::new_with_optional_extension_registry(
            provider,
            model,
            config,
            workspace_root,
            allow_mutating,
            tool_selection,
            user_system_prompt,
            initial_messages,
            resume_info,
            Some(extension_registry),
            Some(installed_packages),
            diagnostics,
            Some(Arc::new(RecordingTraceSink::new())),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_optional_extension_registry(
        provider: Box<dyn Provider>,
        model: String,
        config: OpiConfig,
        workspace_root: PathBuf,
        allow_mutating: bool,
        tool_selection: ToolSelection,
        user_system_prompt: Option<String>,
        initial_messages: Vec<AgentMessage>,
        resume_info: Option<ResumeInfo>,
        extension_registry: Option<ExtensionRegistry>,
        installed_packages: Option<Vec<crate::package_discovery::PackageResource>>,
        startup_diagnostics: Vec<Diagnostic>,
        trace_sink: Option<Arc<RecordingTraceSink>>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        let tool_config = crate::policy::ToolRuntimeConfig::resolve(
            RunMode::NonInteractive,
            allow_mutating,
            tool_selection.clone(),
        )?;
        let hooks = Box::new(crate::runner::NonInteractiveHooks::new(allow_mutating));
        let mut builder = CodingHarness::builder(provider, model, config, workspace_root)
            .hooks(hooks)
            .initial_messages(initial_messages)
            .tool_selection(tool_selection)
            .tool_config(tool_config)
            .startup_diagnostics(startup_diagnostics)
            // Record runtime diagnostics so run summaries can carry structured
            // severity counts (Phase 7 task 7.5).
            .record_diagnostics(true);
        if let Some(installed_packages) = installed_packages {
            builder = builder.installed_packages(installed_packages);
        }
        if let Some(prompt) = user_system_prompt {
            builder = builder.user_system_prompt(prompt);
        }
        if let Some(registry) = extension_registry {
            builder = builder.extension_registry(registry);
        }
        if let Some(resume_info) = resume_info {
            builder = builder.resume(resume_info);
        }
        if let Some(sink) = trace_sink.clone() {
            builder = builder.trace(Some(TraceConfig {
                sink,
                mode: RedactionMode::Summary,
            }));
        }
        let harness = builder.build();
        let control = harness.control_handle();
        Ok(Self {
            harness: Some(harness),
            control,
            running: false,
            trace_sink,
        })
    }

    /// Return the assembled system prompt while the runner is idle.
    pub fn system_prompt(&self) -> Option<&str> {
        self.harness.as_ref().map(CodingHarness::system_prompt)
    }

    /// Run the RPC main loop over stdin/stdout. Returns an exit code.
    pub async fn run(&mut self) -> i32 {
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn_blocking(move || {
            let stdin = io::stdin();
            let reader = io::BufReader::new(stdin.lock());
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => break,
                };
                let trimmed = line.trim_end_matches('\r').trim();
                if trimmed.is_empty() {
                    continue;
                }
                let input = match serde_json::from_str::<SdkCommand>(trimmed) {
                    Ok(command) => RpcInput::Command(command),
                    Err(e) => RpcInput::ParseError(format!("failed to parse command: {e}")),
                };
                if input_tx.send(input).is_err() {
                    break;
                }
            }
        });

        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        self.run_loop(input_rx, |value| {
            write_jsonl(&mut writer, value)
                .and_then(|_| writer.flush())
                .is_ok()
        })
        .await
    }

    /// Run the RPC main loop with in-process command and output channels.
    ///
    /// This is intended for tests and SDK-style embedders that already have
    /// structured commands. Stdin parsing is covered by `run`.
    pub async fn run_with_channels(
        &mut self,
        mut command_rx: tokio::sync::mpsc::UnboundedReceiver<SdkCommand>,
        output_tx: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    ) -> i32 {
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                if input_tx.send(RpcInput::Command(command)).is_err() {
                    break;
                }
            }
        });

        self.run_loop(input_rx, |value| output_tx.send(value.clone()).is_ok())
            .await
    }

    async fn run_loop(
        &mut self,
        mut input_rx: tokio::sync::mpsc::UnboundedReceiver<RpcInput>,
        mut emit: impl FnMut(&serde_json::Value) -> bool,
    ) -> i32 {
        // Surface startup diagnostics (package/adapter degraded-path
        // diagnostics) proactively in the ready header so a headless client
        // learns about disabled packages the instant the session is ready,
        // without having to poll `session_info`. They are also available on
        // demand via the `session_info` command's `resources.diagnostics`.
        let startup_diagnostics = self
            .harness
            .as_ref()
            .map(|harness| {
                harness
                    .resource_metadata()
                    .diagnostic_payloads(RedactionMode::Summary)
            })
            .unwrap_or_default();
        let header = serde_json::json!({
            "type": "rpc_ready",
            "schema_version": SDK_SCHEMA_VERSION,
            "mode": "rpc",
            "version": env!("CARGO_PKG_VERSION"),
            "startup_diagnostics": startup_diagnostics,
        });
        if !emit(&header) {
            return ExitCode::RuntimeFailure as i32;
        }

        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
        let event_tx = Arc::new(event_tx);
        if let Some(harness) = self.harness.as_mut() {
            let etx = event_tx.clone();
            harness.subscribe(Box::new(move |event: &AgentEvent| {
                let _ = etx.send(agent_event_to_value(event));
            }));
        }

        let mut run_task: Option<tokio::task::JoinHandle<RunResult>> = None;

        loop {
            if run_task.as_ref().is_some_and(|task| task.is_finished()) {
                let task = run_task.take().expect("run task checked above");
                let joined = task.await;
                // Flush the run's queued events (incl. AgentEnd) BEFORE
                // emitting the run_summary so the on-wire order is
                // ...events, AgentEnd, run_summary (Phase 7 task 7.5).
                drain_events(&mut event_rx, &mut emit);
                if !self.complete_run_task(joined, &mut emit) {
                    return ExitCode::RuntimeFailure as i32;
                }
                continue;
            }

            tokio::select! {
                Some(event) = event_rx.recv() => {
                    if !emit(&event) {
                        return self
                            .runtime_failure_after_emit_failure(
                                &mut run_task,
                                &mut event_rx,
                                &mut emit,
                            )
                            .await;
                    }
                }
                input = input_rx.recv() => {
                    match input {
                        None => {
                            if !self
                                .shutdown_active_run(&mut run_task, &mut event_rx, &mut emit)
                                .await
                            {
                                return ExitCode::RuntimeFailure as i32;
                            }
                            drain_events(&mut event_rx, &mut emit);
                            return ExitCode::Success as i32;
                        }
                        Some(input) => match input {
                        RpcInput::ParseError(message) => {
                            let resp = response_error(None, "parse", &message);
                            if !emit(&resp) {
                                return self
                                    .runtime_failure_after_emit_failure(
                                        &mut run_task,
                                        &mut event_rx,
                                        &mut emit,
                                    )
                                    .await;
                            }
                        }
                        RpcInput::Command(command) => {
                            if command.is_quit() {
                                let cmd_id = command.id().map(String::from);
                                let cmd_name = command.command_name();
                                let resp = response_success(cmd_id.as_deref(), cmd_name);
                                if !emit(&resp) {
                                    return self
                                        .runtime_failure_after_emit_failure(
                                            &mut run_task,
                                            &mut event_rx,
                                            &mut emit,
                                        )
                                        .await;
                                }
                                if !self
                                    .shutdown_active_run(&mut run_task, &mut event_rx, &mut emit)
                                    .await
                                {
                                    return ExitCode::RuntimeFailure as i32;
                                }
                                drain_events(&mut event_rx, &mut emit);
                                return ExitCode::Success as i32;
                            }

                            if !self
                                .handle_command(command, &mut run_task, &mut emit)
                                .await
                            {
                                let _ = self
                                    .shutdown_active_run(&mut run_task, &mut event_rx, &mut emit)
                                    .await;
                                return ExitCode::RuntimeFailure as i32;
                            }
                        }
                        },
                    }
                }
                joined = async {
                    match run_task.as_mut() {
                        Some(task) => task.await,
                        None => std::future::pending().await,
                    }
                }, if run_task.is_some() => {
                    let _ = run_task.take();
                    // Flush the run's queued events (incl. AgentEnd) BEFORE
                    // emitting the run_summary so the on-wire order is
                    // ...events, AgentEnd, run_summary (Phase 7 task 7.5).
                    drain_events(&mut event_rx, &mut emit);
                    if !self.complete_run_task(joined, &mut emit) {
                        return ExitCode::RuntimeFailure as i32;
                    }
                }
                else => {
                    if !self
                        .shutdown_active_run(&mut run_task, &mut event_rx, &mut emit)
                        .await
                    {
                        return ExitCode::RuntimeFailure as i32;
                    }
                    drain_events(&mut event_rx, &mut emit);
                    return ExitCode::Success as i32;
                }
            }
        }
    }

    async fn runtime_failure_after_emit_failure(
        &mut self,
        run_task: &mut Option<tokio::task::JoinHandle<RunResult>>,
        event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> i32 {
        let _ = self.shutdown_active_run(run_task, event_rx, emit).await;
        ExitCode::RuntimeFailure as i32
    }

    async fn shutdown_active_run(
        &mut self,
        run_task: &mut Option<tokio::task::JoinHandle<RunResult>>,
        event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> bool {
        if self.running {
            self.control.abort();
        }

        let Some(mut task) = run_task.take() else {
            self.running = false;
            return true;
        };

        match tokio::time::timeout(ACTIVE_RUN_SHUTDOWN_TIMEOUT, &mut task).await {
            Ok(joined) => {
                // Drain queued events (incl. AgentEnd) before the run_summary
                // so ordering is preserved on a clean shutdown too.
                drain_events(event_rx, emit);
                self.complete_run_task(joined, emit)
            }
            Err(_) => {
                task.abort();
                let joined = task.await;
                let ok = self.complete_run_task(joined, emit);
                let timeout_event = serde_json::json!({
                    "type": "SessionPersistError",
                    "message": "rpc active run did not stop before shutdown timeout; task aborted",
                });
                drain_events(event_rx, emit);
                ok && emit(&timeout_event)
            }
        }
    }

    fn complete_run_task(
        &mut self,
        joined: Result<RunResult, tokio::task::JoinError>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> bool {
        self.running = false;
        match joined {
            Ok((harness, result)) => {
                self.harness = Some(harness);
                // Phase 7 task 7.5: emit a run-summary event with structured
                // diagnostic counts after the run completes. Additive event.
                if let Some(harness) = self.harness.as_ref()
                    && let Some(counts) = harness.diagnostic_counts()
                {
                    let event = serde_json::json!({
                        "type": "run_summary",
                        "diagnostics": {
                            "info": counts.info,
                            "warning": counts.warning,
                            "error": counts.error,
                        },
                    });
                    let _ = emit(&event);
                }
                self.handle_agent_result(result);
                true
            }
            Err(e) => {
                let event = serde_json::json!({
                    "type": "SessionPersistError",
                    "message": format!("rpc run task failed: {e}"),
                });
                let _ = emit(&event);
                false
            }
        }
    }

    async fn handle_command(
        &mut self,
        command: SdkCommand,
        run_task: &mut Option<tokio::task::JoinHandle<RunResult>>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> bool {
        let cmd_id = command.id().map(String::from);
        let cmd_name = command.command_name();

        match command {
            SdkCommand::prompt { message, .. } => self.start_run(
                ActiveRun::Prompt(message),
                cmd_id.as_deref(),
                cmd_name,
                run_task,
                emit,
            ),
            SdkCommand::continue_ { message, .. } => self.start_run(
                ActiveRun::Continue(message),
                cmd_id.as_deref(),
                cmd_name,
                run_task,
                emit,
            ),
            SdkCommand::abort { .. } => {
                if self.running {
                    self.control.abort();
                }
                emit(&response_success(cmd_id.as_deref(), cmd_name))
            }
            SdkCommand::steer { message, .. } => {
                if self.running {
                    self.control.steer(message);
                } else if let Some(harness) = self.harness.as_ref() {
                    harness.steer(message);
                }
                emit(&response_success(cmd_id.as_deref(), cmd_name))
            }
            SdkCommand::follow_up { message, .. } => {
                if self.running {
                    self.control.follow_up(message);
                } else if let Some(harness) = self.harness.as_ref() {
                    harness.follow_up(message);
                }
                emit(&response_success(cmd_id.as_deref(), cmd_name))
            }
            SdkCommand::set_model { model, .. } => {
                if self.running {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_AGENT_BUSY,
                        "cannot change model while agent is running",
                    ));
                }
                if let Some(harness) = self.harness.as_mut() {
                    match harness.set_model_validated(model) {
                        Ok(model) => {
                            let data = serde_json::json!({ "model": model });
                            emit(&response_success_with_data(
                                cmd_id.as_deref(),
                                cmd_name,
                                data,
                            ))
                        }
                        Err(e) => emit(&response_error(cmd_id.as_deref(), cmd_name, &e)),
                    }
                } else {
                    emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_HARNESS_UNAVAILABLE,
                        "agent harness is unavailable",
                    ))
                }
            }
            SdkCommand::set_thinking_level { level, .. } => {
                if self.running {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_AGENT_BUSY,
                        "cannot change thinking level while agent is running",
                    ));
                }
                let Some(harness) = self.harness.as_mut() else {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_HARNESS_UNAVAILABLE,
                        "agent harness is unavailable",
                    ));
                };
                match harness.set_thinking_level(&level) {
                    Ok(state) => {
                        let data = serde_json::json!({
                            "level": state.level,
                            "enabled": state.enabled,
                            "budget_tokens": state.budget_tokens,
                        });
                        emit(&response_success_with_data(
                            cmd_id.as_deref(),
                            cmd_name,
                            data,
                        ))
                    }
                    Err(e) => emit(&response_error(cmd_id.as_deref(), cmd_name, &e)),
                }
            }
            SdkCommand::compact { .. } => {
                if self.running {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_AGENT_BUSY,
                        "cannot compact while agent is running",
                    ));
                }
                let Some(harness) = self.harness.as_mut() else {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_HARNESS_UNAVAILABLE,
                        "agent harness is unavailable",
                    ));
                };
                match harness.compact_with_diagnostic(CompactionReason::Manual) {
                    Ok((Some(result), diagnostic)) => {
                        let diagnostic = diagnostic.redacted_payload(RedactionMode::Summary);
                        let data = serde_json::json!({
                            "summary": result.summary,
                            "first_kept_entry_id": result.first_kept_entry_id,
                            "tokens_before": result.tokens_before,
                            "tokens_after": result.tokens_after,
                            "diagnostics": [diagnostic],
                        });
                        emit(&response_success_with_data(
                            cmd_id.as_deref(),
                            cmd_name,
                            data,
                        ))
                    }
                    Ok((None, diagnostic)) => {
                        let diagnostic = diagnostic.redacted_payload(RedactionMode::Summary);
                        let data = serde_json::json!({
                            "compacted": false,
                            "diagnostics": [diagnostic],
                        });
                        emit(&response_success_with_data(
                            cmd_id.as_deref(),
                            cmd_name,
                            data,
                        ))
                    }
                    Err(e) => emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_COMPACTION_FAILED,
                        &e,
                    )),
                }
            }
            SdkCommand::session_info { .. } => {
                if self.running {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_AGENT_BUSY,
                        "cannot query session info while agent is running",
                    ));
                }
                let Some(harness) = self.harness.as_mut() else {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_HARNESS_UNAVAILABLE,
                        "agent harness is unavailable",
                    ));
                };
                let mut data = serde_json::json!({
                    "model": harness.model(),
                    "resources": harness.resource_metadata_json(),
                });
                if let Some(session) = harness.session() {
                    data["session_id"] = serde_json::Value::String(session.session_id().to_owned());
                }
                // Phase 13.4: surface live session metadata (name, labels,
                // active branch, thinking) alongside the existing fields. The
                // metadata view is read straight off the session coordinator.
                if let Some(meta) = harness.session_metadata() {
                    data["name"] = match &meta.name {
                        Some(name) => serde_json::Value::String(name.clone()),
                        None => serde_json::Value::Null,
                    };
                    data["labels"] = serde_json::Value::Array(
                        meta.labels
                            .iter()
                            .map(|label| serde_json::Value::String(label.clone()))
                            .collect(),
                    );
                    if let Some(branch) = meta.active_branch {
                        data["active_branch"] = serde_json::Value::String(branch);
                    }
                    data["thinking"] = serde_json::json!({
                        "enabled": meta.thinking.enabled,
                        "budget_tokens": meta.thinking.budget_tokens,
                    });
                }
                // Phase 13.6: surface the reconstructed branch tree so embedders
                // and Phase 14 can render branch/session pickers without
                // re-parsing JSONL. Summaries are derived from message text, so
                // they are redacted through Phase 7 `redact_text` (Summary mode)
                // before leaving the process; counts and ids are opaque metadata
                // and emitted as-is.
                if harness.session().is_some() {
                    match harness.session_tree() {
                        Ok((tree, recovery)) => {
                            let recovery_diagnostics = recovery
                                .diagnostics()
                                .into_iter()
                                .map(|diagnostic| {
                                    serde_json::to_value(
                                        diagnostic.redacted_payload(RedactionMode::Summary),
                                    )
                                    .unwrap_or(serde_json::Value::Null)
                                })
                                .collect::<Vec<_>>();
                            if !recovery_diagnostics.is_empty() {
                                data["tree_recovery"] =
                                    serde_json::Value::Array(recovery_diagnostics);
                            }
                            let active_idx = tree.active_branch_index();
                            if let Some(idx) = active_idx {
                                let branch = &tree.branches()[idx];
                                data["entry_count"] = serde_json::json!(branch.entry_count);
                                if let Some(summary) = branch.summary.as_deref() {
                                    data["branch_summary"] = serde_json::json!(redact_text(
                                        summary,
                                        RedactionMode::Summary
                                    ));
                                }
                            }
                            let branches_arr = tree
                                .branches()
                                .iter()
                                .enumerate()
                                .map(|(idx, branch)| {
                                    let summary = branch
                                        .summary
                                        .as_deref()
                                        .map(|s| redact_text(s, RedactionMode::Summary));
                                    serde_json::json!({
                                        "tip": branch.tip_id,
                                        "summary": summary,
                                        "entry_count": branch.entry_count,
                                        "depth": branch.depth,
                                        "active": active_idx == Some(idx),
                                    })
                                })
                                .collect::<Vec<_>>();
                            data["branches"] = serde_json::Value::Array(branches_arr);
                        }
                        Err(e) => {
                            data["tree_read_error"] = serde_json::json!(format!(
                                "session file could not be read for branch tree: {e}"
                            ));
                        }
                    }
                }
                emit(&response_success_with_data(
                    cmd_id.as_deref(),
                    cmd_name,
                    data,
                ))
            }
            SdkCommand::extension_command { name, args, .. } => {
                if self.running {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_AGENT_BUSY,
                        "cannot dispatch extension command while agent is running",
                    ));
                }
                let Some(harness) = self.harness.as_mut() else {
                    return emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_HARNESS_UNAVAILABLE,
                        "agent harness is unavailable",
                    ));
                };
                match harness
                    .dispatch_extension_command(&name, cmd_id.as_deref(), args)
                    .await
                {
                    Ok(Some(data)) => emit(&response_success_with_data(
                        cmd_id.as_deref(),
                        cmd_name,
                        data,
                    )),
                    Ok(None) => emit(&response_error_with_code(
                        cmd_id.as_deref(),
                        cmd_name,
                        ERR_EXTENSION_COMMAND_NOT_HANDLED,
                        &format!("extension command not handled: {name}"),
                    )),
                    Err(e) => emit(&response_error(cmd_id.as_deref(), cmd_name, &e)),
                }
            }
            SdkCommand::trace { .. } => match &self.trace_sink {
                Some(sink) => {
                    // Supported path: return the versioned redacted envelope.
                    // Records are already redacted at emit time (Summary mode).
                    let records: Vec<serde_json::Value> = sink
                        .snapshot()
                        .iter()
                        .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                        .collect();
                    let data = serde_json::json!({
                        "schema_version": TRACE_SCHEMA_VERSION,
                        "records": records,
                    });
                    emit(&response_success_with_data(
                        cmd_id.as_deref(),
                        cmd_name,
                        data,
                    ))
                }
                None => emit(&response_error_with_code(
                    cmd_id.as_deref(),
                    cmd_name,
                    "unsupported_trace_request",
                    "trace is not enabled for this RPC session",
                )),
            },
            SdkCommand::quit { .. } => true,
        }
    }

    fn start_run(
        &mut self,
        run: ActiveRun,
        id: Option<&str>,
        command: &str,
        run_task: &mut Option<tokio::task::JoinHandle<RunResult>>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> bool {
        if self.running {
            return emit(&response_error_with_code(
                id,
                command,
                ERR_AGENT_BUSY,
                "agent is already running; use steer or follow_up to queue messages",
            ));
        }

        if self.harness.is_none() {
            return emit(&response_error_with_code(
                id,
                command,
                ERR_HARNESS_UNAVAILABLE,
                "agent harness is unavailable",
            ));
        }

        if !emit(&response_success(id, command)) {
            return false;
        }

        let mut harness = self.harness.take().expect("harness checked above");
        harness.reset_cancel_if_cancelled();
        self.control = harness.control_handle();
        self.running = true;

        *run_task = Some(tokio::spawn(async move {
            let result = match run {
                ActiveRun::Prompt(message) => harness.prompt(&message).await,
                ActiveRun::Continue(message) => harness.continue_(&message).await,
            };
            (harness, result)
        }));
        true
    }

    fn handle_agent_result(&self, result: Result<Vec<AgentMessage>, AgentError>) {
        match result {
            Ok(_) | Err(AgentError::Cancelled) => {}
            Err(_) => {}
        }
    }
}

fn response_success(id: Option<&str>, command: &str) -> serde_json::Value {
    serde_json::to_value(SdkResponse::success(id, command)).unwrap()
}

fn response_success_with_data(
    id: Option<&str>,
    command: &str,
    data: serde_json::Value,
) -> serde_json::Value {
    serde_json::to_value(SdkResponse::success_with_data(id, command, data)).unwrap()
}

fn response_error(id: Option<&str>, command: &str, message: &str) -> serde_json::Value {
    serde_json::to_value(SdkResponse::error(id, command, message)).unwrap()
}

/// Build a structured error response carrying a stable machine-readable code
/// (Phase 7 task 7.5), e.g. for an unsupported trace request.
fn response_error_with_code(
    id: Option<&str>,
    command: &str,
    code: &str,
    message: &str,
) -> serde_json::Value {
    serde_json::to_value(SdkResponse::error_with_code(id, command, code, message)).unwrap()
}

/// Write a JSON value as a single line to the writer.
fn write_jsonl(writer: &mut dyn IoWrite, value: &serde_json::Value) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")
}

fn drain_events(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    emit: &mut impl FnMut(&serde_json::Value) -> bool,
) {
    while let Ok(event) = rx.try_recv() {
        if !emit(&event) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the wire values of the RPC runtime-contract failure error codes.
    /// `agent_busy`, `extension_command_not_handled`, and `unsupported_trace_request`
    /// are also exercised end-to-end by `tests/rpc_jsonl.rs`; `harness_unavailable`
    /// and `compaction_failed` guard defensive paths (no-harness runner, compaction
    /// persist failure) that are impractical to drive through the RPC layer, so their
    /// wire values are pinned here against accidental rename.
    #[test]
    fn error_code_constants_pin_documented_wire_values() {
        assert_eq!(ERR_AGENT_BUSY, "agent_busy");
        assert_eq!(ERR_HARNESS_UNAVAILABLE, "harness_unavailable");
        assert_eq!(ERR_COMPACTION_FAILED, "compaction_failed");
        assert_eq!(
            ERR_EXTENSION_COMMAND_NOT_HANDLED,
            "extension_command_not_handled"
        );
    }
}

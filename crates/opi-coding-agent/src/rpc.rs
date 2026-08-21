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
//! | `trace`           | Request the run's evidence records               |
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

use crate::harness::{ProviderAuthFailure, provider_auth_failure};

use opi_agent::agent::AgentControl;
use opi_agent::diagnostic::Diagnostic;
use opi_agent::event::AgentEvent;
use opi_agent::extension::ExtensionRegistry;
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_agent::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse, agent_event_to_value};
use opi_agent::session_event::CompactionReason;
use opi_agent::{RedactionMode, redact_text};
use opi_ai::provider::Provider;

use crate::config::OpiConfig;
use crate::harness::{CodingHarness, ResumeInfo};
use crate::policy::{RunMode, ToolSelection};
use crate::project_trust::TrustDecision;
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
    /// Optional evidence recorder. When set, runs emit ordered evidence
    /// records and the `trace` command returns them; when unset, `trace`
    /// returns a structured unsupported error.
    evidence_recorder: Option<Arc<dyn opi_agent::evidence::EvidenceRecorder>>,
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
        trust_decision: TrustDecision,
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
            trust_decision,
            None,
            Vec::new(),
        )
    }

    /// Create a new RPC runner with an optional evidence recorder. When set,
    /// runs emit ordered redacted evidence records and the `trace` command
    /// returns them; when `None`, `trace` returns a structured unsupported error.
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
        trace_sink: Option<Arc<dyn opi_agent::evidence::EvidenceRecorder>>,
        trust_decision: TrustDecision,
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
            trust_decision,
            None,
            Vec::new(),
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
        trust_decision: TrustDecision,
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
            trust_decision,
            None,
            Vec::new(),
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
        extra_routes: Vec<crate::provider_factory::ProviderAuthPair>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        let RuntimePackageStartup {
            extension_registry,
            installed_packages,
            diagnostics,
            trust_decision,
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
            Some(Arc::new(opi_agent::evidence::InMemoryEvidenceSink::new())),
            trust_decision,
            None,
            extra_routes,
        )
    }

    /// Production runtime-package constructor with the active route's real
    /// per-call authentication resolver.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_runtime_packages_and_auth(
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
        trace_path: Option<PathBuf>,
        auth_resolver: Arc<dyn opi_ai::AuthResolver>,
        extra_routes: Vec<crate::provider_factory::ProviderAuthPair>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        let RuntimePackageStartup {
            extension_registry,
            installed_packages,
            diagnostics,
            trust_decision,
        } = runtime_startup;
        let trace_sink: Arc<dyn opi_agent::evidence::EvidenceRecorder> = match trace_path {
            Some(path) => Arc::new(crate::evidence::FileEvidenceSink::new(path)),
            None => Arc::new(opi_agent::evidence::InMemoryEvidenceSink::new()),
        };
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
            Some(trace_sink),
            trust_decision,
            Some(auth_resolver),
            extra_routes,
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
        trace_sink: Option<Arc<dyn opi_agent::evidence::EvidenceRecorder>>,
        trust_decision: TrustDecision,
        auth_resolver: Option<Arc<dyn opi_ai::AuthResolver>>,
        extra_routes: Vec<crate::provider_factory::ProviderAuthPair>,
    ) -> Result<Self, crate::policy::ToolPolicyError> {
        let tool_config = crate::policy::ToolRuntimeConfig::resolve(
            RunMode::NonInteractive,
            allow_mutating,
            tool_selection.clone(),
        )?;
        let hooks = Box::new(crate::runner::NonInteractiveHooks::new(allow_mutating));
        let mut builder =
            CodingHarness::builder(provider, model, config, workspace_root, trust_decision)
                .hooks(hooks)
                .initial_messages(initial_messages)
                .tool_selection(tool_selection)
                .tool_config(tool_config)
                .extra_routes(extra_routes)
                .startup_diagnostics(startup_diagnostics)
                // Thread the RPC run mode into ExecutionRuntime::build
                // (cannot be derived from tool_config.run_mode, which collapses
                // RPC into NonInteractive).
                .execution_mode(crate::config::ExecutionRunMode::Rpc)
                // Record runtime diagnostics so run summaries can carry
                // structured severity counts.
                .record_diagnostics(true);
        if let Some(auth_resolver) = auth_resolver {
            builder = builder.auth_resolver(auth_resolver);
        }
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
        if let Some(recorder) = trace_sink.clone() {
            builder = builder.evidence(crate::evidence::EvidenceBuilderConfig {
                recorder,
                source: crate::evidence::RPC_ASSEMBLY.clone(),
            });
        }
        let mut harness = builder.build();
        let control = harness.control_handle();
        Ok(Self {
            harness: Some(harness),
            control,
            running: false,
            evidence_recorder: trace_sink,
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
                let Some(input) = parse_rpc_line(&line) else {
                    continue;
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
                // ...events, AgentEnd, run_summary.
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
                    // ...events, AgentEnd, run_summary.
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
                if !self.handle_agent_result(result, emit) {
                    return false;
                }
                // Emit a run-summary event with structured
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
                    "model": harness.model_spec(),
                    "resources": harness.resource_metadata_json(),
                });
                if let Some(session) = harness.session() {
                    data["session_id"] = serde_json::Value::String(session.session_id().to_owned());
                }
                // Surface live session metadata (name, labels,
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
                // Surface the reconstructed branch tree so embedders can render
                // branch/session pickers without
                // re-parsing JSONL. Summaries are derived from message text, so
                // they are redacted through `redact_text` (Summary mode)
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
                            // The raw read error carries the session path, so it
                            // is summarized like the sibling branch/recovery
                            // text instead of leaving the process verbatim.
                            data["tree_read_error"] = serde_json::json!(redact_text(
                                &format!("session file could not be read for branch tree: {e}"),
                                RedactionMode::Summary
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
            SdkCommand::trace { .. } => match &self.evidence_recorder {
                Some(recorder) => {
                    // Supported path: return the ordered, already-redacted
                    // evidence records captured for this RPC session.
                    let records: Vec<serde_json::Value> = recorder
                        .records()
                        .iter()
                        .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                        .collect();
                    let data = serde_json::json!({
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

    fn handle_agent_result(
        &self,
        result: Result<Vec<AgentMessage>, AgentError>,
        emit: &mut impl FnMut(&serde_json::Value) -> bool,
    ) -> bool {
        match result {
            Ok(_) | Err(AgentError::Cancelled) => true,
            Err(error) => {
                let diagnostic: Diagnostic = (&error).into();
                match provider_auth_failure(&error) {
                    Some(
                        ProviderAuthFailure::CredentialNeeded(provider_id)
                        | ProviderAuthFailure::AccountIdMissing(provider_id),
                    ) => emit(&serde_json::json!({
                        "type": "CredentialNeeded",
                        "provider_id": provider_id,
                        "remediation": format!("/login {provider_id}"),
                        "diagnostic": diagnostic.redacted_payload(RedactionMode::Summary),
                    })),
                    Some(ProviderAuthFailure::CredentialRevoked(provider_id)) => {
                        emit(&serde_json::json!({
                            "type": "CredentialRevoked",
                            "provider_id": provider_id,
                            "remediation": format!("/login {provider_id}"),
                            "diagnostic": diagnostic.redacted_payload(RedactionMode::Summary),
                        }))
                    }
                    None => true,
                }
            }
        }
    }
}

fn parse_rpc_line(line: &str) -> Option<RpcInput> {
    let trimmed = line.trim_end_matches('\r').trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(match serde_json::from_str::<SdkCommand>(trimmed) {
        Ok(command) => RpcInput::Command(command),
        Err(error) => RpcInput::ParseError(format!("failed to parse command: {error}")),
    })
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

/// Build a structured error response carrying a stable machine-readable code,
/// e.g. for an unsupported trace request.
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
    use secrecy::ExposeSecret;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CredentialObservingToolProvider {
        expected_credential: String,
        tool_path: String,
        auth_observations: Arc<AtomicUsize>,
        provider_calls: Arc<AtomicUsize>,
        saw_tool_result: Arc<AtomicBool>,
    }

    impl Provider for CredentialObservingToolProvider {
        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[opi_ai::provider::ModelInfo] {
            static MODELS: std::sync::LazyLock<Vec<opi_ai::provider::ModelInfo>> =
                std::sync::LazyLock::new(|| {
                    vec![opi_ai::provider::ModelInfo::new(
                        "mock-model",
                        "mock-model",
                        opi_ai::WireApi::OpenAiCompletions,
                        opi_ai::ModelCapabilities::new(100_000, 4_096),
                    )]
                });
            &MODELS
        }

        fn stream_prepared(
            &self,
            request: opi_ai::provider::Request,
            auth: opi_ai::auth::ResolvedAuth,
        ) -> opi_ai::provider::EventStream {
            assert_eq!(auth.secret.expose_secret(), self.expected_credential);
            self.auth_observations.fetch_add(1, Ordering::SeqCst);
            let call = self.provider_calls.fetch_add(1, Ordering::SeqCst);
            let events = if call == 0 {
                opi_ai::test_support::tool_call_response(
                    "writer-read-call",
                    "read",
                    &serde_json::json!({ "path": self.tool_path }).to_string(),
                )
            } else {
                if request
                    .messages
                    .iter()
                    .any(|message| matches!(message, opi_ai::message::Message::ToolResult(_)))
                {
                    self.saw_tool_result.store(true, Ordering::SeqCst);
                }
                opi_ai::test_support::text_response("rpc-writer-terminal-control")
            };
            Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
        }
    }

    struct JsonlWriterScenario {
        output: String,
        lines: Vec<serde_json::Value>,
        auth_observations: usize,
        provider_calls: usize,
        saw_tool_result: bool,
    }

    struct JsonlWriterCanaries<'a> {
        prompt: &'a str,
        tool: &'a str,
        path: &'a str,
        credential: &'a str,
    }

    async fn join_run_loop_task<T: Send + 'static>(
        mut task: tokio::task::JoinHandle<T>,
        timeout: Duration,
        label: &str,
    ) -> Result<T, String> {
        match tokio::time::timeout(timeout, &mut task).await {
            Ok(joined) => joined.map_err(|error| format!("{label} task failed: {error}")),
            Err(_) => {
                task.abort();
                let cleanup = match tokio::time::timeout(timeout, &mut task).await {
                    Ok(Ok(_)) => "task completed while abort was being delivered".to_owned(),
                    Ok(Err(error)) => format!("task abort join result: {error}"),
                    Err(_) => format!("abort cleanup did not finish within {timeout:?}"),
                };
                Err(format!(
                    "{label} did not terminate within {timeout:?}; task aborted ({cleanup})"
                ))
            }
        }
    }

    // opi-phase17-acceptance
    #[tokio::test]
    async fn run_loop_join_guard_aborts_and_reports_timeout() {
        struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

        impl Drop for DropSignal {
            fn drop(&mut self) {
                if let Some(signal) = self.0.take() {
                    let _ = signal.send(());
                }
            }
        }

        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _drop_signal = DropSignal(Some(dropped_tx));
            let _ = started_tx.send(());
            std::future::pending::<i32>().await
        });
        tokio::time::timeout(Duration::from_millis(100), started_rx)
            .await
            .expect("pending canary starts")
            .expect("pending canary reports readiness");
        let error = tokio::time::timeout(
            Duration::from_millis(100),
            join_run_loop_task(task, Duration::from_millis(10), "join-guard canary"),
        )
        .await
        .expect("post-abort cleanup has its own bound")
        .expect_err("a stalled run_loop task must time out");
        assert!(
            error.contains("join-guard canary"),
            "timeout identifies the stalled run_loop join: {error}"
        );
        tokio::time::timeout(Duration::from_millis(100), dropped_rx)
            .await
            .expect("aborted pending task is dropped within the bound")
            .expect("aborted pending task reports that it is no longer live");
    }

    async fn run_jsonl_writer_scenario(
        config: OpiConfig,
        remove_session_before_prompt: bool,
        terminal_event: &'static str,
        canaries: JsonlWriterCanaries<'_>,
    ) -> JsonlWriterScenario {
        use opi_agent::session::{SessionHeader, SessionWriter};

        let workspace = tempfile::tempdir().unwrap();
        let sessions = tempfile::tempdir().unwrap();
        std::fs::write(
            workspace.path().join(canaries.tool),
            "rpc-writer-safe-file-control",
        )
        .unwrap();

        let session_id = format!("session-{}", canaries.path);
        let session_path = sessions.path().join(format!("{session_id}.jsonl"));
        assert!(session_path.to_string_lossy().contains(canaries.path));
        SessionWriter::create(
            &session_path,
            SessionHeader::new(
                session_id.clone(),
                "2026-08-20T00:00:00Z".to_owned(),
                workspace.path().display().to_string(),
                None,
            ),
        )
        .unwrap();
        let resume = ResumeInfo {
            path: session_path.clone(),
            session_id,
            entries: Vec::new(),
            original_cwd: workspace.path().to_path_buf(),
            diagnostics: Vec::new(),
            recorded_model: None,
            recorded_thinking: None,
        };
        let auth_observations = Arc::new(AtomicUsize::new(0));
        let provider_calls = Arc::new(AtomicUsize::new(0));
        let saw_tool_result = Arc::new(AtomicBool::new(false));
        let provider = CredentialObservingToolProvider {
            expected_credential: canaries.credential.to_owned(),
            tool_path: canaries.tool.to_owned(),
            auth_observations: auth_observations.clone(),
            provider_calls: provider_calls.clone(),
            saw_tool_result: saw_tool_result.clone(),
        };
        let mut runner = RpcRunner::new_with_runtime_packages_and_auth(
            Box::new(provider),
            "mock:mock-model".to_owned(),
            config,
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Allowlist(vec!["read".to_owned()]),
            None,
            Vec::new(),
            RuntimePackageStartup {
                extension_registry: ExtensionRegistry::new(),
                installed_packages: Vec::new(),
                diagnostics: Vec::new(),
                trust_decision: TrustDecision::Trusted,
            },
            Some(resume),
            None,
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from(canaries.credential),
            )),
            Vec::new(),
        )
        .unwrap();
        if remove_session_before_prompt {
            std::fs::remove_file(&session_path).unwrap();
        }

        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
        let jsonl_bytes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let written = jsonl_bytes.clone();
        let saw_terminal_event = Arc::new(tokio::sync::Notify::new());
        let terminal_notification = saw_terminal_event.clone();
        let task = tokio::spawn(async move {
            runner
                .run_loop(input_rx, move |value| {
                    let result = write_jsonl(&mut *written.lock().unwrap(), value).is_ok();
                    if value["type"] == terminal_event {
                        terminal_notification.notify_one();
                    }
                    result
                })
                .await
        });

        input_tx
            .send(RpcInput::Command(RpcCommand::prompt {
                id: Some("writer-prompt-control".to_owned()),
                message: canaries.prompt.to_owned(),
            }))
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), saw_terminal_event.notified())
            .await
            .unwrap_or_else(|_| panic!("{terminal_event} reaches the production JSONL writer"));
        input_tx
            .send(RpcInput::Command(RpcCommand::quit {
                id: Some("writer-quit-control".to_owned()),
            }))
            .unwrap();
        let exit = join_run_loop_task(task, Duration::from_secs(2), terminal_event)
            .await
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(exit, ExitCode::Success as i32);

        let bytes = jsonl_bytes.lock().unwrap().clone();
        assert!(bytes.ends_with(b"\n"));
        assert!(!bytes.contains(&b'\r'));
        let output = String::from_utf8(bytes).unwrap();
        let lines = output
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert!(lines.iter().all(|line| line.is_object()));
        assert!(lines.iter().any(|line| {
            line["type"] == "rpc_ready" && line["schema_version"] == RPC_SCHEMA_VERSION
        }));
        assert!(lines.iter().any(|line| {
            line["type"] == "response"
                && line["command"] == "prompt"
                && line["id"] == "writer-prompt-control"
                && line["success"] == true
        }));
        assert!(lines.iter().any(|line| {
            line["type"] == "ToolExecutionEnd"
                && line["tool_name"] == "read"
                && line["is_error"] == false
                && line["result"]
                    .to_string()
                    .contains("rpc-writer-safe-file-control")
        }));

        JsonlWriterScenario {
            output,
            lines,
            auth_observations: auth_observations.load(Ordering::SeqCst),
            provider_calls: provider_calls.load(Ordering::SeqCst),
            saw_tool_result: saw_tool_result.load(Ordering::SeqCst),
        }
    }

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

    #[test]
    fn stdin_line_parser_ignores_blanks_and_rejects_malformed_commands() {
        assert!(parse_rpc_line(" \t\r").is_none());
        for line in ["not json", r#"{"type":"fly_to_moon"}"#] {
            assert!(matches!(
                parse_rpc_line(line),
                Some(RpcInput::ParseError(_))
            ));
        }
    }

    #[tokio::test]
    async fn production_jsonl_writer_redacts_real_compaction_event_canaries() {
        let prompt_canary = "prompt-arbitrary-rpc-compaction";
        let tool_canary = "tool-arbitrary-rpc-compaction";
        let path_canary = "path-arbitrary-rpc-compaction";
        let credential_canary = "credential-arbitrary-rpc-compaction";
        let mut config = OpiConfig::default();
        config.compaction.threshold_tokens = 0;
        let scenario = run_jsonl_writer_scenario(
            config,
            false,
            "CompactionEnd",
            JsonlWriterCanaries {
                prompt: prompt_canary,
                tool: tool_canary,
                path: path_canary,
                credential: credential_canary,
            },
        )
        .await;

        assert_eq!(scenario.auth_observations, 2);
        assert_eq!(scenario.provider_calls, 2);
        assert!(scenario.saw_tool_result);
        assert!(scenario.output.contains(prompt_canary));
        assert!(scenario.output.contains(tool_canary));
        assert!(scenario.output.contains("rpc-writer-safe-file-control"));
        assert!(!scenario.output.contains(credential_canary));

        let tool_end_index = scenario
            .lines
            .iter()
            .position(|line| line["type"] == "ToolExecutionEnd")
            .expect("the real read completes before automatic compaction");
        let events = scenario
            .lines
            .iter()
            .enumerate()
            .filter(|line| {
                matches!(
                    line.1["type"].as_str(),
                    Some("CompactionStart" | "CompactionEnd")
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert!(tool_end_index < events[0].0);
        assert_eq!(events[0].1["type"], "CompactionStart");
        assert_eq!(events[0].1["reason"], "threshold");
        assert_eq!(events[1].1["type"], "CompactionEnd");
        assert_eq!(events[1].1["reason"], "threshold");
        assert!(events[1].1["result"]["tokens_before"].is_number());
        assert_eq!(events[1].1["result"]["summary"], "[REDACTED]");
        let event_json =
            serde_json::to_string(&events.iter().map(|(_, event)| event).collect::<Vec<_>>())
                .unwrap();
        for canary in [prompt_canary, tool_canary, path_canary, credential_canary] {
            assert!(!event_json.contains(canary), "event leaked {canary}");
        }
    }

    #[tokio::test]
    async fn production_jsonl_writer_redacts_real_session_persist_event_canaries() {
        let prompt_canary = "prompt-arbitrary-rpc-persist";
        let tool_canary = "tool-arbitrary-rpc-persist";
        let path_canary = "path-arbitrary-rpc-persist";
        let credential_canary = "credential-arbitrary-rpc-persist";
        let scenario = run_jsonl_writer_scenario(
            OpiConfig::default(),
            true,
            "SessionPersistError",
            JsonlWriterCanaries {
                prompt: prompt_canary,
                tool: tool_canary,
                path: path_canary,
                credential: credential_canary,
            },
        )
        .await;

        assert_eq!(scenario.auth_observations, 2);
        assert_eq!(scenario.provider_calls, 2);
        assert!(scenario.saw_tool_result);
        assert!(scenario.output.contains(prompt_canary));
        assert!(scenario.output.contains(tool_canary));
        assert!(scenario.output.contains("rpc-writer-safe-file-control"));
        assert!(!scenario.output.contains(path_canary));
        assert!(!scenario.output.contains(credential_canary));

        let tool_end_index = scenario
            .lines
            .iter()
            .position(|line| line["type"] == "ToolExecutionEnd")
            .expect("the real read completes before session persistence fails");
        let events = scenario
            .lines
            .iter()
            .enumerate()
            .filter(|line| line.1["type"] == "SessionPersistError")
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert!(tool_end_index < events[0].0);
        assert_eq!(events[0].1["message"], "[REDACTED]");
        let event_json =
            serde_json::to_string(&events.iter().map(|(_, event)| event).collect::<Vec<_>>())
                .unwrap();
        for canary in [prompt_canary, tool_canary, path_canary, credential_canary] {
            assert!(!event_json.contains(canary), "event leaked {canary}");
        }
    }
}

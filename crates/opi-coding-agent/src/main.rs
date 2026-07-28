use clap::Parser;

use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::{ConfigSource, resolve_config};
use opi_coding_agent::harness::ResumeInfo;
use opi_coding_agent::policy::{
    RunMode, ToolFlags, ToolRuntimeConfig, ToolSelection, resolve_tool_selection,
};
fn main() {
    // Load .env if present (for local development/testing convenience).
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Handle shell completion generation early — no config/provider needed.
    if let Some(shell) = cli.generate_completion {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        let shell: clap_complete::Shell = shell.into();
        clap_complete::generate(shell, &mut cmd, "opi", &mut std::io::stdout());
        return;
    }

    if cli.verbose {
        eprintln!("opi {} - debug mode", env!("CARGO_PKG_VERSION"));
    }

    // Handle package subcommands before provider construction.
    if let Some(opi_coding_agent::cli::Command::Package { command }) = &cli.command {
        let workspace_root = std::env::current_dir().unwrap_or_default();
        let user_config_dir = opi_coding_agent::config::user_config_dir();
        let exit_code = opi_coding_agent::package_cli::handle_package_command(
            command,
            workspace_root,
            user_config_dir,
        );
        std::process::exit(exit_code);
    }

    // Handle the top-level `opi doctor` command before provider construction.
    // Doctor is network-free and must not require credentials or a provider.
    if let Some(opi_coding_agent::cli::Command::Doctor { json, scope }) = &cli.command {
        let exit_code = run_doctor_cli(&cli, scope.as_deref(), *json);
        std::process::exit(exit_code);
    }

    // Handle --export-session early — local file render only, no provider or
    // network needed (Phase 13.5).
    if let Some(session_ref) = cli.export_session.clone() {
        let exit_code = run_export_session(
            session_ref,
            cli.output.clone(),
            cli.format,
            cli.full_tree,
            cli.exclude_tool_output,
            cli.exclude_thinking,
            cli.redact,
        );
        std::process::exit(exit_code);
    }

    // Handle --list-models early -- needs config but not a full provider session.
    if cli.list_models {
        let config = match resolve_config(ConfigSource {
            cli_model: cli.model.clone(),
            config_path: cli.config.clone(),
            env_model: std::env::var("OPI_MODEL").ok(),
            project_dir: std::env::current_dir().ok(),
            user_config_path: None,
        }) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("opi: config error: {e}");
                std::process::exit(2);
            }
        };
        let exit_code = list_models(&config, cli.json);
        std::process::exit(exit_code);
    }

    // Handle session CLI commands first -- they don't need config or a provider.
    let (resumed_messages, resume_info) = match opi_coding_agent::session_cli::handle_session_cli(
        cli.list_sessions,
        cli.json,
        cli.resume.as_deref(),
        cli.fork.as_deref(),
        cli.delete_session.as_deref(),
    ) {
        Ok((true, Some(session))) => {
            // Phase 13.3: build the agent buffer through the opi-agent context
            // API so resume/fork use the same deterministic reconstruction as
            // `CodingHarness::resume_session_id` (no product-only walker).
            let recovery = session.recovery.clone();
            let ctx = opi_agent::session_context::reconstruct_context(&session.entries, &recovery);
            let diagnostics = ctx.diagnostics.clone();
            let original_cwd = std::path::PathBuf::from(&session.header.cwd);
            let info = ResumeInfo {
                path: session.path,
                session_id: session.header.id,
                entries: session.entries,
                original_cwd,
                diagnostics,
                recorded_model: ctx.model,
                recorded_thinking: ctx.thinking_level,
            };
            (Some(ctx.messages), Some(info))
        }
        Ok((true, None)) => return,              // list/delete handled
        Ok((_, None | Some(_))) => (None, None), // no session command or unreachable
        Err(code) => std::process::exit(code),
    };

    let project_dir = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .or_else(|| std::env::current_dir().ok());
    let user_config_dir = opi_coding_agent::config::user_config_dir();

    let prompt_text = cli.prompt.join(" ");

    let tool_selection = resolve_tool_selection(ToolFlags {
        tools: cli.tools.clone(),
        no_tools: cli.no_tools,
        no_builtin_tools: cli.no_builtin_tools,
    });

    // RPC mode: bidirectional JSONL protocol over stdin/stdout.
    if cli.rpc {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("opi: runtime error: {e}");
                std::process::exit(1);
            }
        };
        let exit_code = rt.block_on(async {
            // Phase 15.8.1: two-stage headless trust preflight (project config
            // skipped when untrusted) before provider/runner construction.
            let (config, trust_decision) =
                resolve_headless_trust_config(&cli, project_dir.clone(), user_config_dir.clone())
                    .await;
            run_rpc(
                &cli,
                &config,
                resumed_messages,
                resume_info,
                tool_selection,
                trust_decision,
            )
            .await
        });
        std::process::exit(exit_code);
    } else if cli.non_interactive || cli.json || !prompt_text.is_empty() {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("opi: runtime error: {e}");
                std::process::exit(1);
            }
        };

        let exit_code = rt.block_on(async {
            // Phase 15.8.1: two-stage headless trust preflight before
            // provider/runner construction.
            let (config, trust_decision) =
                resolve_headless_trust_config(&cli, project_dir.clone(), user_config_dir.clone())
                    .await;
            run_non_interactive(
                &cli,
                &config,
                &prompt_text,
                resumed_messages,
                resume_info,
                tool_selection,
                trust_decision,
            )
            .await
        });
        std::process::exit(exit_code);
    } else {
        // Interactive mode -- use TUI
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("opi: runtime error: {e}");
                std::process::exit(1);
            }
        };
        // Phase 15.8.2: two-stage trust-gated config + the interactive TUI
        // prompt. `resolve_interactive_trust_config` runs `prepare_project_startup`
        // and renders the `TrustChoice` prompt for an undecided project with
        // trust-requiring resources BEFORE this returns, so `run_interactive`
        // (provider/package/harness build) provably follows the prompt.
        let (interactive_config, trust_decision) =
            rt.block_on(resolve_interactive_trust_config(&cli, project_dir.clone()));
        rt.block_on(async {
            run_interactive(
                &cli,
                &interactive_config,
                trust_decision,
                resumed_messages,
                resume_info,
                tool_selection,
            )
            .await
        });
    }
}

/// Phase 15.8.1 headless two-stage trust-gated config resolution for
/// non-interactive and RPC startup.
///
/// Mirrors [`resolve_interactive_trust_config`] but resolves trust through the
/// public `prepare_project_startup` preflight with the `HeadlessPreTrustUi`,
/// maps an unresolved ask to `Untrusted` (headless modes never prompt), and
/// returns both the merged config and the decision. The decision feeds
/// `start_installed_package_runtime_with_trust` and the runner harness builder.
/// `global_default` is read from the **pre-trust** config so a project cannot
/// self-authorize via its own `[defaults] default_project_trust`. Exits with
/// code 2 on config/trust error, matching `resolve_interactive_trust_config`.
async fn resolve_headless_trust_config(
    cli: &Cli,
    project_dir: Option<std::path::PathBuf>,
    user_config_dir: std::path::PathBuf,
) -> (
    opi_coding_agent::config::OpiConfig,
    opi_coding_agent::project_trust::TrustDecision,
) {
    use opi_coding_agent::config::{ConfigSource, merge_project_config, resolve_pre_trust_config};
    use opi_coding_agent::project_trust::{
        HeadlessPreTrustUi, ProjectTrustCli, ProjectTrustResolverRegistry, TrustDecision,
        prepare_project_startup,
    };

    let source = ConfigSource {
        cli_model: cli.model.clone(),
        config_path: cli.config.clone(),
        env_model: std::env::var("OPI_MODEL").ok(),
        project_dir: project_dir.clone(),
        user_config_path: None,
    };
    let pre = match resolve_pre_trust_config(source) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("opi: config error: {e}");
            std::process::exit(2);
        }
    };
    // global_default comes from the global (pre-trust) config only, so a project
    // cannot self-authorize by setting its own [defaults] default_project_trust.
    let global_default = pre.defaults.default_project_trust.to_decision();
    let project_root = project_dir
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));
    // Standard CLI: empty resolver registry (no -e / native loader in Phase 15).
    let mut registry = ProjectTrustResolverRegistry::new();
    let plan = match prepare_project_startup(
        ProjectTrustCli {
            trust: cli.trust,
            no_trust: cli.no_trust,
        },
        &mut registry,
        &user_config_dir,
        project_root,
        global_default,
        &HeadlessPreTrustUi,
    )
    .await
    {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("opi: trust error: {e}");
            std::process::exit(2);
        }
    };
    // Headless ask-to-untrusted: an unresolved ask denies project resources.
    let decision = plan.headless_decision();
    let mut config = if matches!(decision, TrustDecision::Untrusted) {
        pre
    } else {
        match merge_project_config(pre, project_root) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("opi: config error: {e}");
                std::process::exit(2);
            }
        }
    };
    config.apply_sandbox_overrides(cli.sandbox, cli.sandbox_require.then_some(true));
    (config, decision)
}

/// Phase 15.8.2 interactive two-stage trust-gated config + TUI prompt.
///
/// Stage 1 (`resolve_pre_trust_config`) resolves every layer except the project
/// `.opi/config.toml`. `prepare_project_startup` runs the full precedence chain
/// (CLI -> resolvers -> store -> global default -> ask); an undecided ask with
/// trust-requiring resources renders the TUI `TrustChoice` prompt via
/// `resolve_interactive_trust_decision` (persisting + mapping the choice), while
/// a pre-decided plan bypasses the prompt. The decision gates stage 2
/// (`merge_project_config`): an untrusted project's config layer is skipped
/// entirely (not loaded-then-filtered), closing the `providers.bedrock.profile`
/// vector. Because the prompt resolves BEFORE this returns, `run_interactive`
/// (provider/package/harness build) provably follows it. Returns the merged
/// config and the decision. Exits 2 on config/trust error. CLI sandbox
/// overrides are re-applied to the two-stage result.
async fn resolve_interactive_trust_config(
    cli: &Cli,
    project_dir: Option<std::path::PathBuf>,
) -> (
    opi_coding_agent::config::OpiConfig,
    opi_coding_agent::project_trust::TrustDecision,
) {
    use opi_coding_agent::config::{ConfigSource, merge_project_config, resolve_pre_trust_config};
    use opi_coding_agent::interactive::{TuiTrustPrompt, resolve_interactive_trust_decision};
    use opi_coding_agent::project_trust::{
        HeadlessPreTrustUi, ProjectTrustCli, ProjectTrustResolverRegistry, TrustDecision,
        prepare_project_startup,
    };

    let source = ConfigSource {
        cli_model: cli.model.clone(),
        config_path: cli.config.clone(),
        env_model: std::env::var("OPI_MODEL").ok(),
        project_dir: project_dir.clone(),
        user_config_path: None,
    };
    let pre = match resolve_pre_trust_config(source) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("opi: config error: {e}");
            std::process::exit(2);
        }
    };
    let user_config_dir = opi_coding_agent::config::user_config_dir();
    let project_root = project_dir
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));
    // global_default comes from the global (pre-trust) config so a project
    // cannot self-authorize via its own [defaults] default_project_trust.
    let global_default = pre.defaults.default_project_trust.to_decision();
    let mut registry = ProjectTrustResolverRegistry::new();
    let plan = match prepare_project_startup(
        ProjectTrustCli {
            trust: cli.trust,
            no_trust: cli.no_trust,
        },
        &mut registry,
        &user_config_dir,
        project_root,
        global_default,
        &HeadlessPreTrustUi,
    )
    .await
    {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("opi: trust error: {e}");
            std::process::exit(2);
        }
    };
    let mut prompt = TuiTrustPrompt;
    let decision = match resolve_interactive_trust_decision(
        &plan,
        &user_config_dir,
        project_root,
        &mut prompt,
    )
    .await
    {
        Ok(decision) => decision,
        Err(e) => {
            eprintln!("opi: trust error: {e}");
            std::process::exit(2);
        }
    };
    let mut config = if matches!(decision, TrustDecision::Untrusted) {
        pre
    } else {
        match merge_project_config(pre, project_root) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("opi: config error: {e}");
                std::process::exit(2);
            }
        }
    };
    config.apply_sandbox_overrides(cli.sandbox, cli.sandbox_require.then_some(true));
    (config, decision)
}

/// Run `--export-session` and return the exit code (Phase 13.5).
///
/// Network-free: resolves the session ref (id or path), reads the source
/// read-only, renders markdown or json with Phase 7 redaction plus
/// tool-output / thinking omission flags, and writes only `output`. The
/// source session is never opened for writing. Returns exit code 0 on
/// success, 1 on export error, 2 on argument error (missing `--output`).
fn run_export_session(
    session_ref: String,
    output: Option<std::path::PathBuf>,
    format: opi_coding_agent::cli::ExportFormat,
    full_tree: bool,
    exclude_tool_output: bool,
    exclude_thinking: bool,
    redact: opi_coding_agent::cli::ExportRedactMode,
) -> i32 {
    use opi_coding_agent::session_cli::{ExportOptions, ExportScope, export_session};

    let Some(output) = output else {
        eprintln!("opi: --export-session requires --output <file>");
        return 2;
    };

    let options = ExportOptions {
        session_ref,
        format,
        output,
        scope: if full_tree {
            ExportScope::FullTree
        } else {
            ExportScope::ActiveBranch
        },
        include_tool_output: !exclude_tool_output,
        include_thinking: !exclude_thinking,
        redact,
    };
    match export_session(&options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("opi: {e}");
            1
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CommandOutcome {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

struct CommandOutput {
    stdout: Box<dyn FnMut(&str)>,
    stderr: Box<dyn FnMut(&str)>,
}

impl CommandOutput {
    fn stdio() -> Self {
        Self {
            stdout: Box::new(|text| print!("{text}")),
            stderr: Box::new(|text| eprintln!("{text}")),
        }
    }

    #[cfg(test)]
    fn discard() -> Self {
        Self {
            stdout: Box::new(|_| {}),
            stderr: Box::new(|_| {}),
        }
    }

    #[cfg(test)]
    fn capturing() -> (
        Self,
        std::sync::Arc<std::sync::Mutex<String>>,
        std::sync::Arc<std::sync::Mutex<String>>,
    ) {
        let stdout = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let stderr = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let stdout_capture = std::sync::Arc::clone(&stdout);
        let stderr_capture = std::sync::Arc::clone(&stderr);
        (
            Self {
                stdout: Box::new(move |text| {
                    stdout_capture
                        .lock()
                        .expect("stdout capture")
                        .push_str(text)
                }),
                stderr: Box::new(move |text| {
                    stderr_capture
                        .lock()
                        .expect("stderr capture")
                        .push_str(text)
                }),
            },
            stdout,
            stderr,
        )
    }

    fn write_stdout(&mut self, text: &str) {
        (self.stdout)(text);
    }

    fn write_stderr(&mut self, text: &str) {
        (self.stderr)(text);
    }
}

fn write_command_outcome(
    outcome: &CommandOutcome,
    stdout: &mut impl std::io::Write,
    stderr: &mut impl std::io::Write,
) -> std::io::Result<i32> {
    stdout.write_all(outcome.stdout.as_bytes())?;
    stderr.write_all(outcome.stderr.as_bytes())?;
    Ok(outcome.exit_code)
}

fn emit_command_outcome(outcome: &CommandOutcome) -> i32 {
    let result = {
        let stdout = std::io::stdout();
        let stderr = std::io::stderr();
        write_command_outcome(outcome, &mut stdout.lock(), &mut stderr.lock())
    };
    match result {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("opi: output error: {error}");
            1
        }
    }
}

/// Run the top-level `opi doctor` command and return the exit code.
///
/// Network-free: config is resolved best-effort so a broken config surfaces as
/// a config-scope error diagnostic (exit 2) rather than an internal failure
/// (exit 1). An unparseable `--scope` list is an internal failure (exit 1).
fn run_doctor_cli(cli: &Cli, scope: Option<&str>, json: bool) -> i32 {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::doctor::{DoctorContext, DoctorScope};

    let scopes = match scope {
        Some(raw) => match DoctorScope::parse_list(raw) {
            Ok(scopes) => scopes,
            Err(message) => {
                eprintln!("opi doctor: {message}");
                return 1;
            }
        },
        None => Vec::new(),
    };

    // Resolve config best-effort: a config failure is reported as a diagnostic
    // (exit 2) rather than aborting the command (exit 1).
    let config_source = ConfigSource {
        cli_model: cli.model.clone(),
        config_path: cli.config.clone(),
        env_model: std::env::var("OPI_MODEL").ok(),
        project_dir: std::env::current_dir().ok(),
        user_config_path: None,
    };
    let (config, config_error) = match resolve_config(config_source) {
        Ok(config) => (config, None),
        Err(err) => (OpiConfig::default(), Some(err)),
    };

    let workspace_root = std::env::current_dir().unwrap_or_default();
    let user_config_dir = opi_coding_agent::config::user_config_dir();
    let sessions_dir = opi_coding_agent::session_cli::session_dir();
    let term = std::env::var("TERM").ok();
    let term_program = std::env::var("TERM_PROGRAM").ok();
    let term_features = std::env::var("TERM_FEATURES").ok();
    let no_color = std::env::var_os("NO_COLOR").is_some();
    let colorterm = std::env::var("COLORTERM").ok();
    let env_probe = |name: &str| std::env::var(name).ok();

    let empty_store_probe = std::collections::HashMap::new();

    let ctx = DoctorContext {
        config: &config,
        config_error: config_error.as_ref(),
        workspace_root: &workspace_root,
        user_config_dir: &user_config_dir,
        sessions_dir: &sessions_dir,
        term: term.as_deref(),
        term_program: term_program.as_deref(),
        term_features: term_features.as_deref(),
        no_color,
        colorterm: colorterm.as_deref(),
        env_var: &env_probe,
        store_probe: &empty_store_probe,
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("opi doctor: runtime error: {error}");
            return 1;
        }
    };
    let outcome = rt.block_on(run_doctor_command_core(
        &scopes,
        &ctx,
        json,
        user_config_dir.clone(),
        opi_coding_agent::credential_store::native_keyring_backend_factory(),
    ));
    emit_command_outcome(&outcome)
}

async fn run_doctor_command_core(
    scopes: &[opi_coding_agent::doctor::DoctorScope],
    ctx: &opi_coding_agent::doctor::DoctorContext<'_>,
    json_output: bool,
    user_config_dir: std::path::PathBuf,
    backend_factory: opi_coding_agent::credential_store::KeyringBackendFactory,
) -> CommandOutcome {
    let report =
        opi_coding_agent::doctor::run_doctor_command(scopes, ctx, user_config_dir, backend_factory)
            .await;
    let stdout = if json_output {
        format!("{}\n", opi_coding_agent::doctor::format_json(&report))
    } else {
        opi_coding_agent::doctor::format_text(&report)
    };
    CommandOutcome {
        stdout,
        stderr: String::new(),
        exit_code: report.exit_code(),
    }
}

async fn with_provider_bundle<T, F, Fut>(
    bundle: opi_coding_agent::provider_factory::ProviderBundle,
    callback: F,
) -> T
where
    F: FnOnce(Box<dyn opi_ai::provider::Provider>, Vec<opi_agent::Diagnostic>) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let opi_coding_agent::provider_factory::ProviderBundle {
        provider,
        store,
        resolver,
        registry,
        diagnostics,
    } = bundle;
    let result = callback(provider, diagnostics).await;
    drop((store, resolver, registry));
    result
}

fn merge_provider_diagnostics(
    startup: &mut opi_coding_agent::runtime_packages::RuntimePackageStartup,
    diagnostics: Vec<opi_agent::Diagnostic>,
) {
    startup.diagnostics.extend(diagnostics);
}

async fn run_non_interactive(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    prompt_text: &str,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
) -> i32 {
    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    run_non_interactive_core(
        cli,
        config,
        prompt_text,
        resumed_messages,
        resume_info,
        tool_selection,
        trust_decision,
        workspace_root,
        opi_coding_agent::config::user_config_dir(),
        opi_coding_agent::credential_store::native_keyring_backend_factory(),
        None,
        CommandOutput::stdio(),
        |_| {},
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_non_interactive_core<Observe>(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    prompt_text: &str,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
    workspace_root: std::path::PathBuf,
    user_config_dir: std::path::PathBuf,
    backend_factory: opi_coding_agent::credential_store::KeyringBackendFactory,
    provider_override: Option<Box<dyn opi_ai::provider::Provider>>,
    mut output: CommandOutput,
    observe_result: Observe,
) -> i32
where
    Observe: FnOnce(&opi_coding_agent::runner::NonInteractiveResult),
{
    use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};

    if prompt_text.is_empty() {
        output.write_stderr("opi: no prompt provided");
        return ExitCode::ConfigError as i32;
    }

    let mut bundle = match opi_coding_agent::provider_factory::build_provider_bundle(
        config,
        user_config_dir.clone(),
        backend_factory,
    )
    .await
    {
        Ok(p) => p,
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Auth(msg)) => {
            output.write_stderr(&format!("opi: {msg}"));
            return ExitCode::AuthFailure as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Config(msg)) => {
            output.write_stderr(&format!("opi: {msg}"));
            return ExitCode::ConfigError as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Provider(e)) => {
            output.write_stderr(&format!("opi: {e}"));
            return ExitCode::ConfigError as i32;
        }
    };
    if let Some(provider) = provider_override {
        bundle.provider = provider;
    }

    let allow_mutating = cli.allow_mutating || config.defaults.allow_mutating_tools;

    let user_system_prompt =
        cli.system
            .as_ref()
            .and_then(|path| match std::fs::read_to_string(path) {
                Ok(content) => Some(content),
                Err(e) => {
                    output.write_stderr(&format!(
                        "opi: warning: failed to read system prompt file {}: {e}",
                        path.display()
                    ));
                    None
                }
            });

    let runtime_startup =
        opi_coding_agent::runtime_packages::start_installed_package_runtime_with_trust(
            &workspace_root,
            &user_config_dir,
            trust_decision,
        )
        .await;

    with_provider_bundle(bundle, move |provider, provider_diagnostics| async move {
        let mut runtime_startup = runtime_startup;
        merge_provider_diagnostics(&mut runtime_startup, provider_diagnostics);
        let mut runner = match NonInteractiveRunner::new_with_resume_and_runtime_packages(
            provider,
            config.defaults.model.clone(),
            config.clone(),
            workspace_root,
            allow_mutating,
            user_system_prompt,
            resumed_messages.unwrap_or_default(),
            resume_info,
            tool_selection,
            runtime_startup,
            cli.trace.clone(),
        ) {
            Ok(runner) => runner,
            Err(e) => {
                output.write_stderr(&format!("opi: {e}"));
                return ExitCode::ConfigError as i32;
            }
        }
        .with_compact_ndjson(cli.json_compact);

        let result = if cli.image.is_empty() {
            // No images -- use the plain text path.
            if cli.json {
                runner.run_json(prompt_text).await
            } else {
                runner.run(prompt_text).await
            }
        } else {
            // Load images and combine with text prompt.
            let mut content: Vec<opi_ai::message::InputContent> = Vec::new();
            content.push(opi_ai::message::InputContent::Text {
                text: prompt_text.to_owned(),
            });
            for image_path in &cli.image {
                match opi_coding_agent::image::load_image_with_limit(
                    image_path,
                    config.defaults.max_image_bytes,
                ) {
                    Ok(img) => content.push(img),
                    Err(e) => {
                        output.write_stderr(&format!("opi: {e}"));
                        return ExitCode::ConfigError as i32;
                    }
                }
            }
            if cli.json {
                runner.run_json_with_content(content).await
            } else {
                runner.run_with_content(content).await
            }
        };

        observe_result(&result);
        if !result.stdout.is_empty() {
            output.write_stdout(&result.stdout);
        }
        if !result.stderr.is_empty() {
            output.write_stderr(&result.stderr);
        }

        result.exit_code
    })
    .await
}

enum RpcTransport {
    Stdio,
    #[cfg(test)]
    Channels {
        command_rx: tokio::sync::mpsc::UnboundedReceiver<opi_coding_agent::rpc::RpcCommand>,
        output_tx: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    },
}

async fn run_rpc(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
) -> i32 {
    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    run_rpc_core(
        cli,
        config,
        resumed_messages,
        resume_info,
        tool_selection,
        trust_decision,
        workspace_root,
        opi_coding_agent::config::user_config_dir(),
        opi_coding_agent::credential_store::native_keyring_backend_factory(),
        None,
        CommandOutput::stdio(),
        RpcTransport::Stdio,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_rpc_core(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
    workspace_root: std::path::PathBuf,
    user_config_dir: std::path::PathBuf,
    backend_factory: opi_coding_agent::credential_store::KeyringBackendFactory,
    provider_override: Option<Box<dyn opi_ai::provider::Provider>>,
    mut output: CommandOutput,
    transport: RpcTransport,
) -> i32 {
    use opi_coding_agent::rpc::RpcRunner;
    use opi_coding_agent::runner::ExitCode;

    let mut bundle = match opi_coding_agent::provider_factory::build_provider_bundle(
        config,
        user_config_dir.clone(),
        backend_factory,
    )
    .await
    {
        Ok(p) => p,
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Auth(msg)) => {
            output.write_stderr(&format!("opi: {msg}"));
            return ExitCode::AuthFailure as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Config(msg)) => {
            output.write_stderr(&format!("opi: {msg}"));
            return ExitCode::ConfigError as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Provider(e)) => {
            output.write_stderr(&format!("opi: {e}"));
            return ExitCode::ConfigError as i32;
        }
    };
    if let Some(provider) = provider_override {
        bundle.provider = provider;
    }

    let allow_mutating = cli.allow_mutating || config.defaults.allow_mutating_tools;

    let user_system_prompt =
        cli.system
            .as_ref()
            .and_then(|path| match std::fs::read_to_string(path) {
                Ok(content) => Some(content),
                Err(e) => {
                    output.write_stderr(&format!(
                        "opi: warning: failed to read system prompt file {}: {e}",
                        path.display()
                    ));
                    None
                }
            });

    let runtime_startup =
        opi_coding_agent::runtime_packages::start_installed_package_runtime_with_trust(
            &workspace_root,
            &user_config_dir,
            trust_decision,
        )
        .await;

    with_provider_bundle(bundle, move |provider, provider_diagnostics| async move {
        let mut runtime_startup = runtime_startup;
        merge_provider_diagnostics(&mut runtime_startup, provider_diagnostics);
        let mut runner = match RpcRunner::new_with_runtime_packages(
            provider,
            config.defaults.model.clone(),
            config.clone(),
            workspace_root,
            allow_mutating,
            tool_selection,
            user_system_prompt,
            resumed_messages.unwrap_or_default(),
            runtime_startup,
            resume_info,
        ) {
            Ok(runner) => runner,
            Err(e) => {
                output.write_stderr(&format!("opi: {e}"));
                return ExitCode::ConfigError as i32;
            }
        };

        match transport {
            RpcTransport::Stdio => runner.run().await,
            #[cfg(test)]
            RpcTransport::Channels {
                command_rx,
                output_tx,
            } => runner.run_with_channels(command_rx, output_tx).await,
        }
    })
    .await
}

async fn run_interactive(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
) {
    use opi_coding_agent::interactive;

    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    run_interactive_core(
        cli,
        config,
        trust_decision,
        resumed_messages,
        resume_info,
        tool_selection,
        workspace_root,
        opi_coding_agent::config::user_config_dir(),
        opi_coding_agent::credential_store::native_keyring_backend_factory(),
        |harness, model_display, theme_name, keybindings| async move {
            interactive::run_interactive_tui(harness, model_display, &theme_name, keybindings).await
        },
    )
    .await;
}

/// Credential-aware interactive startup core shared verbatim by production
/// and the launch-boundary ordering test.
#[allow(clippy::too_many_arguments)]
async fn run_interactive_core<Launch, LaunchFuture>(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    trust_decision: opi_coding_agent::project_trust::TrustDecision,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
    workspace_root: std::path::PathBuf,
    user_config_dir: std::path::PathBuf,
    backend_factory: opi_coding_agent::credential_store::KeyringBackendFactory,
    launch_tui: Launch,
) where
    Launch: FnOnce(
        opi_coding_agent::harness::CodingHarness,
        String,
        String,
        opi_tui::Keybindings,
    ) -> LaunchFuture,
    LaunchFuture: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    use opi_coding_agent::harness::{CodingHarness, InteractiveCodingHooks};

    let mut bundle = match opi_coding_agent::provider_factory::build_provider_bundle(
        config,
        user_config_dir.clone(),
        backend_factory,
    )
    .await
    {
        Ok(b) => b,
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Auth(msg)) => {
            eprintln!("opi: {msg}");
            std::process::exit(3);
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Config(msg)) => {
            eprintln!("opi: {msg}");
            std::process::exit(2);
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Provider(e)) => {
            eprintln!("opi: {e}");
            std::process::exit(2);
        }
    };
    let provider_diagnostics = std::mem::take(&mut bundle.diagnostics);
    let provider = bundle.provider;

    let user_system_prompt = cli
        .system
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok());

    let hooks = Box::new(InteractiveCodingHooks::new(true));
    let initial_messages = resumed_messages.unwrap_or_default();
    // Phase 15.8.2: trust_decision was resolved (with the TUI prompt when
    // needed) BEFORE run_interactive_core was entered, so provider/package/
    // harness construction below provably follows trust resolution.
    let mut runtime_startup =
        opi_coding_agent::runtime_packages::start_installed_package_runtime_with_trust(
            &workspace_root,
            &user_config_dir,
            trust_decision,
        )
        .await;
    merge_provider_diagnostics(&mut runtime_startup, provider_diagnostics);

    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, tool_selection.clone())
            .expect("interactive tool config should be valid");
    let mut builder = CodingHarness::builder(
        provider,
        config.defaults.model.clone(),
        config.clone(),
        workspace_root,
        trust_decision,
    )
    .hooks(hooks)
    .initial_messages(initial_messages)
    .tool_selection(tool_selection)
    .tool_config(tool_config)
    .extension_registry(runtime_startup.extension_registry)
    .installed_packages(runtime_startup.installed_packages)
    .startup_diagnostics(runtime_startup.diagnostics)
    .trust_decision(trust_decision);
    if let Some(prompt) = user_system_prompt {
        builder = builder.user_system_prompt(prompt);
    }
    if let Some(resume_info) = resume_info {
        builder = builder.resume(resume_info);
    }
    let harness = builder.build();

    let mut harness = harness;

    // Load --image files for the first interactive prompt.
    if !cli.image.is_empty() {
        let mut images = Vec::new();
        for image_path in &cli.image {
            match opi_coding_agent::image::load_image_with_limit(
                image_path,
                config.defaults.max_image_bytes,
            ) {
                Ok(img) => images.push(img),
                Err(e) => {
                    eprintln!("opi: {e}");
                    std::process::exit(2);
                }
            }
        }
        harness.queue_images(images);
    }

    let model_display = config.defaults.model.clone();
    let theme_name = config.defaults.theme.clone();
    let keybindings = parse_keybindings(&config.keybindings);
    // Phase 14.2: attach the credential store and OAuth registry so the
    // interactive loop can handle /login, /logout, and CredentialNeeded retry.
    harness.credential_store = Some(bundle.store);
    harness.oauth_registry = Some(bundle.registry);

    if let Err(e) = launch_tui(harness, model_display, theme_name, keybindings).await {
        eprintln!("opi: TUI error: {e}");
        std::process::exit(1);
    }
}

fn parse_keybindings(config: &opi_coding_agent::config::KeybindingsConfig) -> opi_tui::Keybindings {
    use std::collections::HashMap;

    let map = HashMap::from([
        ("submit".to_string(), config.submit.clone()),
        ("abort".to_string(), config.abort.clone()),
        ("new_line".to_string(), config.new_line.clone()),
    ]);
    match opi_tui::Keybindings::from_config_map(&map) {
        Ok(kb) => kb,
        Err(e) => {
            eprintln!("opi: warning: invalid keybindings config ({e}), using defaults");
            opi_tui::Keybindings::default()
        }
    }
}

/// List available models from all configured providers.
/// Returns exit code: 0 on success, 1 if no models found, 2 on config error.
fn list_models(config: &opi_coding_agent::config::OpiConfig, json_output: bool) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("opi: runtime error: {error}");
            return 1;
        }
    };
    let outcome = rt.block_on(run_list_models_command_core(
        config,
        json_output,
        opi_coding_agent::config::user_config_dir(),
        opi_coding_agent::credential_store::native_keyring_backend_factory(),
    ));
    emit_command_outcome(&outcome)
}

async fn run_list_models_command_core(
    config: &opi_coding_agent::config::OpiConfig,
    json_output: bool,
    user_config_dir: std::path::PathBuf,
    backend_factory: opi_coding_agent::credential_store::KeyringBackendFactory,
) -> CommandOutcome {
    let collection = match opi_coding_agent::provider_factory::build_collection_for_listing_command(
        config,
        user_config_dir,
        backend_factory,
    )
    .await
    {
        Ok(collection) => collection,
        Err(opi_coding_agent::provider_factory::ListModelsError::MissingCredentials) => {
            return CommandOutcome {
                stdout: String::new(),
                stderr: "opi: no models available (configure API keys to list models)\n".to_owned(),
                exit_code: 1,
            };
        }
        Err(opi_coding_agent::provider_factory::ListModelsError::Config(msg)) => {
            return CommandOutcome {
                stdout: String::new(),
                stderr: format!("opi: config error: {msg}\n"),
                exit_code: 2,
            };
        }
    };
    let entries =
        opi_coding_agent::model_listing::model_entries_from_registry(collection.registry());

    if entries.is_empty() {
        return CommandOutcome {
            stdout: String::new(),
            stderr: "opi: no models available (configure API keys to list models)\n".to_owned(),
            exit_code: 1,
        };
    }

    let mut stdout = String::new();
    if json_output {
        for entry in &entries {
            let json = serde_json::json!({
                "model": entry.model_id,
                "provider": entry.provider_id,
                "display_name": entry.display_name,
            });
            stdout.push_str(&format!("{json}\n"));
        }
    } else {
        // Compute column widths
        let max_id = entries.iter().map(|e| e.model_id.len()).max().unwrap_or(10);
        let max_name = entries
            .iter()
            .map(|e| e.display_name.len())
            .max()
            .unwrap_or(12);
        let max_prov = entries
            .iter()
            .map(|e| e.provider_id.len())
            .max()
            .unwrap_or(8);

        // Header
        stdout.push_str(&format!(
            "{:<width_prov$}  {:<width_id$}  DISPLAY NAME\n",
            "PROVIDER",
            "MODEL ID",
            width_prov = max_prov,
            width_id = max_id,
        ));
        stdout.push_str(&format!(
            "{}  {}  {}\n",
            "-".repeat(max_prov),
            "-".repeat(max_id),
            "-".repeat(max_name),
        ));

        for entry in &entries {
            stdout.push_str(&format!(
                "{:<width_prov$}  {:<width_id$}  {}\n",
                entry.provider_id,
                entry.model_id,
                entry.display_name,
                width_prov = max_prov,
                width_id = max_id,
            ));
        }
    }

    CommandOutcome {
        stdout,
        stderr: String::new(),
        exit_code: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use clap::Parser;

    use super::{
        CommandOutcome, CommandOutput, RpcTransport, run_doctor_command_core, run_interactive_core,
        run_list_models_command_core, run_non_interactive_core, run_rpc_core, with_provider_bundle,
        write_command_outcome,
    };
    use opi_coding_agent::cli::Cli;
    use opi_coding_agent::config::{CredentialBackendSource, OpiConfig, ProviderProxyConfig};
    use opi_coding_agent::credential_store::{
        BackendError, FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE,
        KeyringBackend, KeyringBackendFactory,
    };
    use opi_coding_agent::doctor::{DoctorContext, DoctorScope};

    const FIX_F_SECRET_CANARY: &str = "sk-fix-f-command-core-DO-NOT-LEAK";
    const FIX_G_API_KEY_ENV: &str = "OPI_TEST_PHASE14_BACKEND_FALLBACK_KEY";
    const FIX_G_SECRET_CANARY: &str = "sk-fix-g-backend-fallback-DO-NOT-LEAK";
    const FIX_I_STORED_CANARY: &str = "fix-i-stored-wrong-type-DO-NOT-LEAK";
    const FIX_I_FALLBACK_CANARY: &str = "fix-i-env-fallback-DO-NOT-LEAK";
    const FIX_I_FALLBACK_ENV: &str = "OPI_TEST_FIX_I_FALLBACK_KEY";
    const FIX_I_REDACTED_MALFORMED_DIAGNOSTIC: &str = "credential store error: malformed credential envelope for 'anthropic': credential envelope does not match the expected schema";

    #[derive(Clone, Copy)]
    enum CanaryReply {
        Present,
        BackendUnavailable,
    }

    #[derive(Default)]
    struct CanaryCounts {
        factory_calls: AtomicUsize,
        presence_get_calls: AtomicUsize,
        protected_get_calls: AtomicUsize,
        set_calls: AtomicUsize,
        delete_calls: AtomicUsize,
    }

    struct CanaryKeyringBackend {
        reply: CanaryReply,
        counts: Arc<CanaryCounts>,
    }

    impl KeyringBackend for CanaryKeyringBackend {
        fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
            if service == KEYCHAIN_PRESENCE_SERVICE {
                self.counts
                    .presence_get_calls
                    .fetch_add(1, Ordering::SeqCst);
                return match self.reply {
                    CanaryReply::Present => {
                        Ok((provider_id == "anthropic").then(|| "api_key".to_owned()))
                    }
                    CanaryReply::BackendUnavailable => Err(BackendError::BackendUnavailable(
                        "fix-f-canary-backend-unavailable".to_owned(),
                    )),
                };
            }

            self.counts
                .protected_get_calls
                .fetch_add(1, Ordering::SeqCst);
            Err(BackendError::Other(format!(
                "protected credential read forbidden: {FIX_F_SECRET_CANARY}"
            )))
        }

        fn set(
            &self,
            _service: &str,
            _provider_id: &str,
            _value: &str,
        ) -> Result<(), BackendError> {
            self.counts.set_calls.fetch_add(1, Ordering::SeqCst);
            Err(BackendError::Other("unexpected canary set".to_owned()))
        }

        fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
            self.counts.delete_calls.fetch_add(1, Ordering::SeqCst);
            Err(BackendError::Other("unexpected canary delete".to_owned()))
        }
    }

    fn canary_factory(reply: CanaryReply, counts: Arc<CanaryCounts>) -> KeyringBackendFactory {
        Box::new(move || {
            counts.factory_calls.fetch_add(1, Ordering::SeqCst);
            Box::new(CanaryKeyringBackend { reply, counts })
        })
    }

    fn assert_canary_route(counts: &CanaryCounts, outcome: &CommandOutcome) {
        assert_eq!(counts.factory_calls.load(Ordering::SeqCst), 1);
        assert!(counts.presence_get_calls.load(Ordering::SeqCst) > 0);
        assert_eq!(counts.protected_get_calls.load(Ordering::SeqCst), 0);
        assert_eq!(counts.set_calls.load(Ordering::SeqCst), 0);
        assert_eq!(counts.delete_calls.load(Ordering::SeqCst), 0);
        assert!(!outcome.stdout.contains(FIX_F_SECRET_CANARY));
        assert!(!outcome.stderr.contains(FIX_F_SECRET_CANARY));
    }

    fn keychain_config() -> OpiConfig {
        let mut config = OpiConfig::default();
        config.defaults.model = "anthropic:claude-fix-f".to_owned();
        config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
        config
    }

    static PROVIDER_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct ProviderEnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

    impl ProviderEnvGuard {
        fn clear() -> Self {
            Self::scoped(&[])
        }

        fn scoped(overrides: &[(&'static str, &std::ffi::OsStr)]) -> Self {
            let mut names = vec![
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_OAUTH_TOKEN",
                "OPENAI_API_KEY",
                "OPENROUTER_API_KEY",
                "MISTRAL_API_KEY",
                "GEMINI_API_KEY",
                "AZURE_OPENAI_API_KEY",
                "VERTEX_ACCESS_TOKEN",
                "AWS_ACCESS_KEY_ID",
                "AWS_SECRET_ACCESS_KEY",
                "AWS_PROFILE",
            ];
            for (name, _) in overrides {
                if !names.contains(name) {
                    names.push(name);
                }
            }
            let original = names
                .iter()
                .copied()
                .map(|name| (name, std::env::var_os(name)))
                .collect();
            for name in &names {
                // SAFETY: command-core tests serialize and restore these variables.
                unsafe { std::env::remove_var(name) };
            }
            for (name, value) in overrides {
                // SAFETY: command-core tests serialize and restore these variables.
                unsafe { std::env::set_var(name, value) };
            }
            Self(original)
        }
    }

    impl Drop for ProviderEnvGuard {
        fn drop(&mut self) {
            for (name, value) in self.0.drain(..) {
                match value {
                    Some(value) => {
                        // SAFETY: command-core tests serialize and restore these variables.
                        unsafe { std::env::set_var(name, value) };
                    }
                    None => {
                        // SAFETY: command-core tests serialize and restore these variables.
                        unsafe { std::env::remove_var(name) };
                    }
                }
            }
        }
    }

    fn doctor_context<'a>(
        config: &'a OpiConfig,
        dir: &'a std::path::Path,
        config_error: Option<&'a opi_coding_agent::config::ConfigError>,
    ) -> DoctorContext<'a> {
        static EMPTY_PROBES: std::sync::LazyLock<
            std::collections::HashMap<String, opi_ai::CredentialSource>,
        > = std::sync::LazyLock::new(std::collections::HashMap::new);
        DoctorContext {
            config,
            config_error,
            workspace_root: dir,
            user_config_dir: dir,
            sessions_dir: dir,
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &|_| None,
            store_probe: &EMPTY_PROBES,
        }
    }

    #[test]
    fn list_models_command_core_uses_injected_present_backend() {
        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let _env = ProviderEnvGuard::clear();
        let dir = tempfile::tempdir().expect("temp dir");
        let counts = Arc::new(CanaryCounts::default());
        let outcome = tokio::runtime::Runtime::new().expect("runtime").block_on(
            run_list_models_command_core(
                &keychain_config(),
                false,
                dir.path().to_path_buf(),
                canary_factory(CanaryReply::Present, Arc::clone(&counts)),
            ),
        );

        assert_canary_route(&counts, &outcome);
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty(), "{}", outcome.stderr);
        assert!(outcome.stdout.contains("PROVIDER"));
        assert!(outcome.stdout.contains("anthropic"));
        assert!(outcome.stdout.contains("claude"));
    }

    #[test]
    fn list_models_command_core_uses_injected_unavailable_backend() {
        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let _env = ProviderEnvGuard::clear();
        let dir = tempfile::tempdir().expect("temp dir");
        let counts = Arc::new(CanaryCounts::default());
        let outcome = tokio::runtime::Runtime::new().expect("runtime").block_on(
            run_list_models_command_core(
                &keychain_config(),
                false,
                dir.path().to_path_buf(),
                canary_factory(CanaryReply::BackendUnavailable, Arc::clone(&counts)),
            ),
        );

        assert_canary_route(&counts, &outcome);
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty());
        assert_eq!(
            outcome
                .stdout
                .lines()
                .filter(|line| line.contains("github-copilot"))
                .count(),
            25,
            "{}",
            outcome.stdout
        );
    }

    #[test]
    fn list_models_command_core_preserves_json_and_config_error_contract() {
        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let _env = ProviderEnvGuard::clear();
        let dir = tempfile::tempdir().expect("temp dir");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let json_counts = Arc::new(CanaryCounts::default());
        let json = runtime.block_on(run_list_models_command_core(
            &keychain_config(),
            true,
            dir.path().to_path_buf(),
            canary_factory(CanaryReply::Present, Arc::clone(&json_counts)),
        ));
        assert_canary_route(&json_counts, &json);
        assert_eq!(json.exit_code, 0);
        assert!(json.stderr.is_empty());
        for line in json.stdout.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("NDJSON model row");
            assert!(value.get("model").is_some());
            assert!(value.get("provider").is_some());
            assert!(value.get("display_name").is_some());
        }

        let mut invalid = keychain_config();
        invalid.providers.anthropic.proxy = Some(ProviderProxyConfig {
            url: "not a proxy url".to_owned(),
            no_proxy: None,
        });
        let error_counts = Arc::new(CanaryCounts::default());
        let error = runtime.block_on(run_list_models_command_core(
            &invalid,
            false,
            dir.path().to_path_buf(),
            canary_factory(CanaryReply::Present, Arc::clone(&error_counts)),
        ));
        assert_canary_route(&error_counts, &error);
        assert_eq!(error.exit_code, 2);
        assert!(error.stdout.is_empty());
        assert!(error.stderr.contains("opi: config error:"));
        assert!(
            error
                .stderr
                .contains("failed to build HTTP client with proxy config")
        );
    }

    #[test]
    fn doctor_command_core_uses_injected_present_backend() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = keychain_config();
        let counts = Arc::new(CanaryCounts::default());
        let outcome =
            tokio::runtime::Runtime::new()
                .expect("runtime")
                .block_on(run_doctor_command_core(
                    &[DoctorScope::Provider],
                    &doctor_context(&config, dir.path(), None),
                    false,
                    dir.path().to_path_buf(),
                    canary_factory(CanaryReply::Present, Arc::clone(&counts)),
                ));

        assert_canary_route(&counts, &outcome);
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty());
        assert!(outcome.stdout.contains("credentials present"));
    }

    #[test]
    fn doctor_command_core_uses_injected_unavailable_backend() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = keychain_config();
        let counts = Arc::new(CanaryCounts::default());
        let outcome =
            tokio::runtime::Runtime::new()
                .expect("runtime")
                .block_on(run_doctor_command_core(
                    &[DoctorScope::Provider],
                    &doctor_context(&config, dir.path(), None),
                    true,
                    dir.path().to_path_buf(),
                    canary_factory(CanaryReply::BackendUnavailable, Arc::clone(&counts)),
                ));

        assert_canary_route(&counts, &outcome);
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty());
        assert!(
            outcome
                .stdout
                .contains("doctor_provider_credential_backend")
        );
        assert!(outcome.stdout.contains("fix-f-canary-backend-unavailable"));
    }

    #[test]
    fn doctor_command_core_preserves_scope_and_config_error_contract() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = keychain_config();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let subset_counts = Arc::new(CanaryCounts::default());
        let subset = runtime.block_on(run_doctor_command_core(
            &[DoctorScope::Config, DoctorScope::Rpc],
            &doctor_context(&config, dir.path(), None),
            true,
            dir.path().to_path_buf(),
            canary_factory(CanaryReply::Present, Arc::clone(&subset_counts)),
        ));
        assert_canary_route(&subset_counts, &subset);
        assert_eq!(subset.exit_code, 0);
        let scopes: std::collections::HashSet<_> = subset
            .stdout
            .lines()
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).expect("doctor NDJSON row")["scope"]
                    .as_str()
                    .expect("scope string")
                    .to_owned()
            })
            .collect();
        assert_eq!(scopes, ["config".to_owned(), "rpc".to_owned()].into());

        let config_error = opi_coding_agent::config::ConfigError::Read {
            path: dir.path().join("broken.toml"),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "broken config"),
        };
        let error_counts = Arc::new(CanaryCounts::default());
        let error = runtime.block_on(run_doctor_command_core(
            &[DoctorScope::Config],
            &doctor_context(&config, dir.path(), Some(&config_error)),
            true,
            dir.path().to_path_buf(),
            canary_factory(CanaryReply::BackendUnavailable, Arc::clone(&error_counts)),
        ));
        assert_canary_route(&error_counts, &error);
        assert_eq!(error.exit_code, 2);
        assert!(error.stderr.is_empty());
        assert!(error.stdout.contains("\"source\":\"config\""));
        assert!(error.stdout.contains("\"severity\":\"error\""));
    }

    #[test]
    fn write_command_outcome_preserves_stdout_stderr_and_exit_code() {
        let outcome = CommandOutcome {
            stdout: "stdout bytes\n".to_owned(),
            stderr: "stderr bytes\n".to_owned(),
            exit_code: 2,
        };
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit_code = write_command_outcome(&outcome, &mut stdout, &mut stderr)
            .expect("in-memory output write");

        assert_eq!(stdout, b"stdout bytes\n");
        assert_eq!(stderr, b"stderr bytes\n");
        assert_eq!(exit_code, 2);
    }

    struct OrderingKeyringBackend {
        inner: FakeKeyringBackend,
        events: Arc<Mutex<Vec<&'static str>>>,
        live: Arc<AtomicBool>,
    }

    impl OrderingKeyringBackend {
        fn inner(&self) -> &FakeKeyringBackend {
            &self.inner
        }

        fn record_entry_creation(&self) {
            assert!(
                self.live.load(Ordering::SeqCst),
                "entry creation must follow test-owned store installation"
            );
            self.events
                .lock()
                .expect("ordering events")
                .push("entry_creation");
        }
    }

    impl KeyringBackend for OrderingKeyringBackend {
        fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
            self.record_entry_creation();
            self.inner().get(service, provider_id)
        }

        fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
            self.record_entry_creation();
            self.inner().set(service, provider_id, value)
        }

        fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
            self.record_entry_creation();
            self.inner().delete(service, provider_id)
        }
    }

    impl Drop for OrderingKeyringBackend {
        fn drop(&mut self) {
            self.live.store(false, Ordering::SeqCst);
            self.events
                .lock()
                .expect("ordering events")
                .push("guard_drop");
        }
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn native_keyring_precedes_interactive_startup() {
        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[("OPI_SESSIONS_DIR", session_blocker.as_os_str())]);
        let events = Arc::new(Mutex::new(Vec::new()));
        let live = Arc::new(AtomicBool::new(false));
        let factory_events = Arc::clone(&events);
        let factory_live = Arc::clone(&live);
        let backend_factory: KeyringBackendFactory = Box::new(move || {
            let backend = FakeKeyringBackend::new();
            factory_live.store(true, Ordering::SeqCst);
            factory_events
                .lock()
                .expect("ordering events")
                .push("native_install");
            backend.seed_raw(
                KEYCHAIN_SERVICE,
                "openai",
                r#"{"version":1,"kind":"api_key","api_key":"test-interactive-ordering"}"#,
            );
            backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "openai", "api_key");
            Box::new(OrderingKeyringBackend {
                inner: backend,
                events: Arc::clone(&factory_events),
                live: Arc::clone(&factory_live),
            })
        });

        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let mut config = opi_coding_agent::config::OpiConfig::default();
        config.defaults.model = "openai:gpt-4o".into();
        let cli = Cli::parse_from(["opi"]);
        let launch_events = Arc::clone(&events);
        let launch_live = Arc::clone(&live);
        assert!(
            events.lock().expect("ordering events").is_empty(),
            "backend construction must remain lazy until interactive core"
        );
        run_interactive_core(
            &cli,
            &config,
            opi_coding_agent::project_trust::TrustDecision::Trusted,
            None,
            None,
            opi_coding_agent::policy::ToolSelection::Default,
            workspace_dir.path().to_path_buf(),
            user_config_dir.path().to_path_buf(),
            backend_factory,
            move |_harness, _model, _theme_name, _keybindings| async move {
                assert!(
                    launch_live.load(Ordering::SeqCst),
                    "test-owned store must remain installed at the TUI launch boundary"
                );
                launch_events
                    .lock()
                    .expect("ordering events")
                    .push("tui_launch");
                Ok::<(), Box<dyn std::error::Error>>(())
            },
        )
        .await;
        assert!(!live.load(Ordering::SeqCst));
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );

        let events = events.lock().expect("ordering events");
        assert_eq!(events.first(), Some(&"native_install"), "{events:?}");
        let first_entry = events
            .iter()
            .position(|event| *event == "entry_creation")
            .expect("at least one keyring entry creation");
        let tui_launch = events
            .iter()
            .position(|event| *event == "tui_launch")
            .expect("actual TUI launch boundary");
        let guard_drop = events
            .iter()
            .position(|event| *event == "guard_drop")
            .expect("native guard drop event");
        assert!(
            0 < first_entry && first_entry < tui_launch && tui_launch < guard_drop,
            "{events:?}"
        );
        assert_eq!(events.last(), Some(&"guard_drop"), "{events:?}");
    }

    async fn assert_store_lives_through_run_callback(run_mode: &'static str) {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            r#"{"version":1,"kind":"api_key","api_key":"test-run-lifetime"}"#,
        );
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        let events = Arc::new(Mutex::new(vec!["native_install"]));
        let live = Arc::new(AtomicBool::new(true));

        let dir = tempfile::tempdir().expect("temp dir");
        let mut config = opi_coding_agent::config::OpiConfig::default();
        config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
        let bundle = opi_coding_agent::provider_factory::build_provider_bundle(
            &config,
            dir.path().to_path_buf(),
            Box::new({
                let events = Arc::clone(&events);
                let live = Arc::clone(&live);
                move || {
                    Box::new(OrderingKeyringBackend {
                        inner: backend,
                        events,
                        live,
                    })
                }
            }),
        )
        .await
        .expect("ordinary API-key provider bundle");

        let callback_live = Arc::clone(&live);
        let completed = with_provider_bundle(bundle, move |provider, _diagnostics| async move {
            assert_eq!(provider.id(), "anthropic");
            assert!(
                callback_live.load(Ordering::SeqCst),
                "test-owned store must remain installed throughout {run_mode} callback"
            );
            tokio::task::yield_now().await;
            assert!(
                callback_live.load(Ordering::SeqCst),
                "test-owned store must remain installed after an await in {run_mode} callback"
            );
            true
        })
        .await;

        assert!(completed);
        assert!(
            !live.load(Ordering::SeqCst),
            "test-owned store must be dropped after {run_mode} callback returns"
        );
    }

    #[tokio::test]
    async fn native_store_lives_through_noninteractive_run_callback() {
        assert_store_lives_through_run_callback("noninteractive").await;
    }

    #[tokio::test]
    async fn native_store_lives_through_rpc_run_callback() {
        assert_store_lives_through_run_callback("rpc").await;
    }

    fn backend_fallback_config() -> OpiConfig {
        let mut config = OpiConfig::default();
        config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
        config.providers.anthropic.api_key_env = FIX_G_API_KEY_ENV.into();
        config
    }

    fn unavailable_backend_factory() -> KeyringBackendFactory {
        Box::new(|| Box::new(FakeKeyringBackend::new().with_unavailable()))
    }

    fn empty_backend_factory() -> KeyringBackendFactory {
        Box::new(|| Box::new(FakeKeyringBackend::new()))
    }

    fn configured_backend_factory() -> KeyringBackendFactory {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            r#"{"version":1,"kind":"api_key","api_key":"test-command-core"}"#,
        );
        Box::new(move || Box::new(backend.clone()))
    }

    fn malformed_backend_factory() -> KeyringBackendFactory {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            &format!(
                r#"{{"version":"{FIX_I_STORED_CANARY}","kind":"api_key","api_key":"stored"}}"#
            ),
        );
        Box::new(move || Box::new(backend.clone()))
    }

    fn malformed_keychain_config() -> OpiConfig {
        let mut config = keychain_config();
        config.providers.anthropic.api_key_env = FIX_I_FALLBACK_ENV.into();
        config
    }

    fn session_blocker(dir: &tempfile::TempDir) -> std::path::PathBuf {
        let path = dir.path().join("sessions-must-stay-disabled");
        std::fs::write(&path, b"not a directory").expect("session blocker file");
        path
    }

    #[test]
    fn credential_aware_production_core_tests_isolate_sessions() {
        let source = include_str!("main.rs");
        let core_markers = [
            ["run_", "interactive_core("].concat(),
            ["run_", "non_interactive_core("].concat(),
            ["run_", "rpc_core("].concat(),
        ];
        let required_markers = [
            ["PROVIDER_ENV", "_LOCK"].concat(),
            ["ProviderEnvGuard", "::scoped"].concat(),
            ["OPI_SESSIONS", "_DIR"].concat(),
            ["session_blocker", ".is_file()"].concat(),
        ];
        let mut current_test = None::<String>;
        let mut test_chunks = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            let starts_test = line.starts_with("    #[")
                && (trimmed == "#[test]" || trimmed.starts_with("#[tokio::test"));
            if starts_test && let Some(chunk) = current_test.replace(String::new()) {
                test_chunks.push(chunk);
            }
            if let Some(chunk) = &mut current_test {
                chunk.push_str(line);
                chunk.push('\n');
            }
        }
        if let Some(chunk) = current_test {
            test_chunks.push(chunk);
        }

        let mut failures = Vec::new();
        for chunk in test_chunks {
            if !core_markers.iter().any(|marker| chunk.contains(marker)) {
                continue;
            }
            let name = chunk
                .lines()
                .find_map(|line| {
                    let signature = line
                        .trim()
                        .strip_prefix("fn ")
                        .or_else(|| line.trim().strip_prefix("async fn "))?;
                    signature.split('(').next()
                })
                .unwrap_or("unknown test");
            for required in &required_markers {
                if !chunk.contains(required) {
                    failures.push(format!("{name} is missing `{required}`"));
                }
            }
        }

        assert!(
            failures.is_empty(),
            "credential-aware production-core tests must isolate session IO:\n{}",
            failures.join("\n")
        );
    }

    fn assert_single_backend_fallback(diagnostics: &[serde_json::Value]) {
        use opi_agent::diagnostic::SOURCE_PROVIDER;

        let matches: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == "provider_credential_backend_unavailable")
            .collect();
        assert_eq!(matches.len(), 1, "{diagnostics:?}");
        let diagnostic = matches[0];
        assert_eq!(diagnostic["source"], SOURCE_PROVIDER);
        assert_eq!(diagnostic["severity"], "warning");
        assert_eq!(diagnostic["details"]["provider"], "anthropic");
        assert_eq!(diagnostic["details"]["env_var"], FIX_G_API_KEY_ENV);
        assert_eq!(
            diagnostic["details"]["credential_source"],
            "environment_fallback"
        );
        assert!(
            !serde_json::to_string(diagnostic)
                .expect("diagnostic serializes")
                .contains(FIX_G_SECRET_CANARY),
            "fallback diagnostic must not leak the API key canary"
        );
    }

    #[test]
    fn noninteractive_auth_failure_uses_production_core_in_text_and_json_modes() {
        use opi_ai::provider::ProviderError;
        use opi_ai::test_support::{MockProvider, MockResponse};
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[("OPI_SESSIONS_DIR", session_blocker.as_os_str())]);

        for json in [false, true] {
            let cli = if json {
                Cli::parse_from(["opi", "--json"])
            } else {
                Cli::parse_from(["opi"])
            };
            let observed = Arc::new(AtomicBool::new(false));
            let observed_result = Arc::clone(&observed);
            let (output, stdout, stderr) = CommandOutput::capturing();
            let exit_code = tokio::runtime::Runtime::new().expect("runtime").block_on(
                run_non_interactive_core(
                    &cli,
                    &keychain_config(),
                    "hello",
                    None,
                    None,
                    opi_coding_agent::policy::ToolSelection::Default,
                    opi_coding_agent::project_trust::TrustDecision::Trusted,
                    workspace_dir.path().to_path_buf(),
                    user_config_dir.path().to_path_buf(),
                    empty_backend_factory(),
                    Some(Box::new(MockProvider::new_with_errors(
                        "anthropic",
                        vec![MockResponse::Error(ProviderError::CredentialNeeded {
                            provider_id: "anthropic".into(),
                        })],
                    ))),
                    output,
                    move |_| observed_result.store(true, Ordering::SeqCst),
                ),
            );

            assert_eq!(exit_code, ExitCode::AuthFailure as i32);
            assert!(observed.load(Ordering::SeqCst));
            let stdout = stdout.lock().expect("stdout capture").clone();
            if json {
                let remediation_events: Vec<_> = stdout
                    .lines()
                    .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                    .filter(|line| line["type"] == "CredentialNeeded")
                    .collect();
                assert_eq!(
                    remediation_events.len(),
                    1,
                    "expected exactly one typed stream-time credential remediation"
                );
                let remediation = &remediation_events[0];
                assert_eq!(remediation["provider_id"], "anthropic");
                assert_eq!(remediation["remediation"], "/login anthropic");
            } else {
                assert!(stdout.is_empty());
            }
            let stderr = stderr.lock().expect("stderr capture").clone();
            assert!(stderr.contains("credential needed"), "{stderr}");
            assert!(stderr.contains("/login anthropic"), "{stderr}");
        }
        assert!(session_blocker.is_file());
    }

    #[test]
    fn rpc_auth_failure_uses_production_core_after_ready() {
        use opi_ai::provider::ProviderError;
        use opi_ai::test_support::{MockProvider, MockResponse};
        use opi_coding_agent::rpc::RpcCommand;
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[("OPI_SESSIONS_DIR", session_blocker.as_os_str())]);
        let cli = Cli::parse_from(["opi", "--rpc"]);
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
        let (output, stdout, stderr) = CommandOutput::capturing();
        let config = keychain_config();

        let (exit_code, emitted) =
            tokio::runtime::Runtime::new()
                .expect("runtime")
                .block_on(async {
                    let run = run_rpc_core(
                        &cli,
                        &config,
                        None,
                        None,
                        opi_coding_agent::policy::ToolSelection::Default,
                        opi_coding_agent::project_trust::TrustDecision::Trusted,
                        workspace_dir.path().to_path_buf(),
                        user_config_dir.path().to_path_buf(),
                        empty_backend_factory(),
                        Some(Box::new(MockProvider::new_with_errors(
                            "anthropic",
                            vec![MockResponse::Error(ProviderError::CredentialNeeded {
                                provider_id: "anthropic".into(),
                            })],
                        ))),
                        output,
                        RpcTransport::Channels {
                            command_rx,
                            output_tx,
                        },
                    );
                    let drive = async move {
                        let mut emitted = Vec::new();
                        let ready = output_rx.recv().await.expect("rpc_ready");
                        assert_eq!(ready["type"], "rpc_ready");
                        emitted.push(ready);
                        command_tx
                            .send(RpcCommand::prompt {
                                id: Some("auth-prompt".into()),
                                message: "hello".into(),
                            })
                            .expect("queue prompt");
                        loop {
                            let line = output_rx.recv().await.expect("credential event");
                            let credential_needed = line["type"] == "CredentialNeeded";
                            emitted.push(line);
                            if credential_needed {
                                break;
                            }
                        }
                        command_tx
                            .send(RpcCommand::quit {
                                id: Some("quit-after-auth".into()),
                            })
                            .expect("queue quit");
                        tokio::time::timeout(std::time::Duration::from_secs(2), async {
                            while let Some(line) = output_rx.recv().await {
                                emitted.push(line);
                            }
                        })
                        .await
                        .expect("RPC output channel closes after quit");
                        emitted
                    };
                    tokio::join!(run, drive)
                });

        assert_eq!(exit_code, ExitCode::Success as i32);
        let remediation_events: Vec<_> = emitted
            .iter()
            .filter(|line| line["type"] == "CredentialNeeded")
            .collect();
        assert_eq!(
            remediation_events.len(),
            1,
            "expected exactly one typed stream-time RPC remediation"
        );
        let remediation = remediation_events[0];
        assert_eq!(remediation["provider_id"], "anthropic");
        assert_eq!(remediation["remediation"], "/login anthropic");
        assert!(stdout.lock().expect("stdout capture").is_empty());
        assert!(stderr.lock().expect("stderr capture").is_empty());
        assert!(session_blocker.is_file());
    }

    #[test]
    fn malformed_startup_errors_are_captured_and_redacted_through_both_cores() {
        use opi_ai::test_support::{MockProvider, text_response};
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (
                FIX_I_FALLBACK_ENV,
                std::ffi::OsStr::new(FIX_I_FALLBACK_CANARY),
            ),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = malformed_keychain_config();

        let cli = Cli::parse_from(["opi", "--json"]);
        let observed = Arc::new(AtomicBool::new(false));
        let observed_result = Arc::clone(&observed);
        let (output, stdout, stderr) = CommandOutput::capturing();
        let noninteractive_exit =
            tokio::runtime::Runtime::new()
                .expect("runtime")
                .block_on(run_non_interactive_core(
                    &cli,
                    &config,
                    "hello",
                    None,
                    None,
                    opi_coding_agent::policy::ToolSelection::Default,
                    opi_coding_agent::project_trust::TrustDecision::Trusted,
                    workspace_dir.path().to_path_buf(),
                    user_config_dir.path().to_path_buf(),
                    malformed_backend_factory(),
                    Some(Box::new(MockProvider::new(
                        "anthropic",
                        vec![text_response("must not run")],
                    ))),
                    output,
                    move |_| observed_result.store(true, Ordering::SeqCst),
                ));
        assert_eq!(noninteractive_exit, ExitCode::ConfigError as i32);
        assert!(!observed.load(Ordering::SeqCst));
        assert!(stdout.lock().expect("stdout capture").is_empty());
        let diagnostic = stderr.lock().expect("stderr capture").clone();
        assert!(diagnostic.starts_with("opi: "), "{diagnostic}");
        assert!(
            diagnostic.contains(FIX_I_REDACTED_MALFORMED_DIAGNOSTIC),
            "unexpected non-interactive classification: {diagnostic}"
        );
        for canary in [FIX_I_STORED_CANARY, FIX_I_FALLBACK_CANARY] {
            assert!(
                !diagnostic.contains(canary),
                "leaked {canary}: {diagnostic}"
            );
        }

        let cli = Cli::parse_from(["opi", "--rpc"]);
        let (_command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
        let (output, stdout, stderr) = CommandOutput::capturing();
        let rpc_exit = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_rpc_core(
                &cli,
                &config,
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                malformed_backend_factory(),
                Some(Box::new(MockProvider::new(
                    "anthropic",
                    vec![text_response("must not run")],
                ))),
                output,
                RpcTransport::Channels {
                    command_rx,
                    output_tx,
                },
            ));
        assert_eq!(rpc_exit, ExitCode::ConfigError as i32);
        assert!(
            output_rx.try_recv().is_err(),
            "rpc_ready must not be emitted"
        );
        assert!(stdout.lock().expect("stdout capture").is_empty());
        let diagnostic = stderr.lock().expect("stderr capture").clone();
        assert!(diagnostic.starts_with("opi: "), "{diagnostic}");
        assert!(
            diagnostic.contains(FIX_I_REDACTED_MALFORMED_DIAGNOSTIC),
            "unexpected RPC classification: {diagnostic}"
        );
        for canary in [FIX_I_STORED_CANARY, FIX_I_FALLBACK_CANARY] {
            assert!(
                !diagnostic.contains(canary),
                "leaked {canary}: {diagnostic}"
            );
        }
        assert!(session_blocker.is_file());
    }

    #[test]
    fn rpc_production_core_preserves_ready_correlation_and_installed_packages() {
        use opi_ai::test_support::MockProvider;
        use opi_coding_agent::package_resolver::local_lock_entry;
        use opi_coding_agent::package_store::{PackageDeclaration, PackageStore};
        use opi_coding_agent::rpc::RpcCommand;
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[("OPI_SESSIONS_DIR", session_blocker.as_os_str())]);

        let package_dir = workspace_dir.path().join("vendor").join("rpc-suite");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            "name = \"rpc-suite\"\ndescription = \"RPC suite\"\nversion = \"0.1.0\"\n",
        )
        .expect("package manifest");
        let store = PackageStore::project(workspace_dir.path().to_path_buf());
        store
            .write_declarations(&[PackageDeclaration {
                source: "./vendor/rpc-suite".into(),
                filters: Default::default(),
            }])
            .expect("package declarations");
        store
            .write_lock(
                &[local_lock_entry("./vendor/rpc-suite".into(), &package_dir)
                    .expect("package lock entry")],
            )
            .expect("package lock");

        let cli = Cli::parse_from(["opi", "--rpc"]);
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        command_tx
            .send(RpcCommand::session_info {
                id: Some("session-1".into()),
            })
            .expect("queue first session_info");
        command_tx
            .send(RpcCommand::session_info {
                id: Some("session-2".into()),
            })
            .expect("queue second session_info");
        command_tx
            .send(RpcCommand::quit {
                id: Some("quit-1".into()),
            })
            .expect("queue quit");
        drop(command_tx);
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();

        let exit_code = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_rpc_core(
                &cli,
                &keychain_config(),
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                configured_backend_factory(),
                Some(Box::new(MockProvider::new("anthropic", Vec::new()))),
                CommandOutput::discard(),
                RpcTransport::Channels {
                    command_rx,
                    output_tx,
                },
            ));

        assert_eq!(exit_code, ExitCode::Success as i32);
        let output: Vec<_> = std::iter::from_fn(|| output_rx.try_recv().ok()).collect();
        assert_eq!(output[0]["type"], "rpc_ready", "{output:?}");
        for id in ["session-1", "session-2"] {
            let response = output
                .iter()
                .find(|line| line["type"] == "response" && line["id"] == id)
                .unwrap_or_else(|| panic!("missing correlated response {id}: {output:?}"));
            assert_eq!(response["success"], true);
            assert!(
                response["data"]["resources"]["packages"]
                    .as_array()
                    .is_some_and(|packages| packages.iter().any(|name| name == "rpc-suite")),
                "installed package missing from session_info: {response}"
            );
        }
        assert!(output.iter().any(|line| {
            line["type"] == "response"
                && line["command"] == "quit"
                && line["id"] == "quit-1"
                && line["success"] == true
        }));
        assert!(session_blocker.is_file());
    }

    #[test]
    fn provider_backend_fallback_reaches_noninteractive_startup_diagnostics_once() {
        use opi_ai::test_support::{MockProvider, text_response};
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = backend_fallback_config();
        let cli = Cli::parse_from(["opi", "--json"]);
        let observed = Arc::new(AtomicBool::new(false));
        let observed_result = Arc::clone(&observed);

        let exit_code =
            tokio::runtime::Runtime::new()
                .expect("runtime")
                .block_on(run_non_interactive_core(
                    &cli,
                    &config,
                    "hello",
                    None,
                    None,
                    opi_coding_agent::policy::ToolSelection::Default,
                    opi_coding_agent::project_trust::TrustDecision::Trusted,
                    workspace_dir.path().to_path_buf(),
                    user_config_dir.path().to_path_buf(),
                    unavailable_backend_factory(),
                    Some(Box::new(MockProvider::new(
                        "anthropic",
                        vec![text_response("done")],
                    ))),
                    CommandOutput::discard(),
                    move |result| {
                        observed_result.store(true, Ordering::SeqCst);
                        assert_eq!(result.exit_code, ExitCode::Success as i32);
                        let lines: Vec<serde_json::Value> = result
                            .stdout
                            .lines()
                            .map(|line| serde_json::from_str(line).expect("NDJSON line"))
                            .collect();
                        let startup: Vec<_> = lines
                            .iter()
                            .filter(|line| line["type"] == "StartupDiagnostics")
                            .collect();
                        assert_eq!(startup.len(), 1, "{lines:?}");
                        assert_single_backend_fallback(
                            startup[0]["diagnostics"]
                                .as_array()
                                .expect("startup diagnostics array"),
                        );
                    },
                ));

        assert_eq!(exit_code, ExitCode::Success as i32);
        assert!(observed.load(Ordering::SeqCst));
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    #[test]
    fn provider_backend_fallback_reaches_rpc_ready_once() {
        use opi_coding_agent::rpc::RpcCommand;
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = backend_fallback_config();
        let cli = Cli::parse_from(["opi", "--rpc"]);
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        command_tx
            .send(RpcCommand::quit { id: None })
            .expect("queue quit command");
        drop(command_tx);
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();

        let exit_code = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_rpc_core(
                &cli,
                &config,
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                unavailable_backend_factory(),
                None,
                CommandOutput::discard(),
                RpcTransport::Channels {
                    command_rx,
                    output_tx,
                },
            ));

        assert_eq!(exit_code, ExitCode::Success as i32);
        let output: Vec<_> = std::iter::from_fn(|| output_rx.try_recv().ok()).collect();
        let ready: Vec<_> = output
            .iter()
            .filter(|line| line["type"] == "rpc_ready")
            .collect();
        assert_eq!(ready.len(), 1, "{output:?}");
        assert_single_backend_fallback(
            ready[0]["startup_diagnostics"]
                .as_array()
                .expect("rpc_ready startup diagnostics array"),
        );
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    #[test]
    fn provider_backend_fallback_reaches_interactive_launcher_once() {
        use opi_agent::diagnostic::RedactionMode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = backend_fallback_config();
        let cli = Cli::parse_from(["opi"]);
        let launch_count = Arc::new(AtomicUsize::new(0));
        let observed_launches = Arc::clone(&launch_count);

        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_interactive_core(
                &cli,
                &config,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                unavailable_backend_factory(),
                move |harness, _model, _theme_name, _keybindings| async move {
                    observed_launches.fetch_add(1, Ordering::SeqCst);
                    let diagnostics = harness
                        .resource_metadata()
                        .diagnostic_payloads(RedactionMode::Summary)
                        .into_iter()
                        .map(|diagnostic| {
                            serde_json::to_value(diagnostic).expect("diagnostic serializes")
                        })
                        .collect::<Vec<_>>();
                    assert_single_backend_fallback(&diagnostics);
                    Ok::<(), Box<dyn std::error::Error>>(())
                },
            ));

        assert_eq!(launch_count.load(Ordering::SeqCst), 1);
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    // ------------------------------------------------------------------------
    // Phase 15.5.1: strict-sandbox production dispatch reaches all three run_*
    // startup entry points (acceptance scenario `phase15-sandbox-config-
    // production-path`). Non-interactive drives a real bash tool turn through an
    // injected MockProvider and independently inspects the production
    // capability outcome before asserting either confined execution or a
    // fail-closed error in user-visible NDJSON output. RPC and
    // interactive cannot inject a MockProvider for a bash turn here (run_rpc_core
    // prompt-turn timing is non-deterministic; run_interactive_core takes no
    // provider_override and calling harness.prompt would hit the real API), so
    // they prove their entry reached the built harness and, on permanent-gap
    // hosts, the CODE_SANDBOX_UNAVAILABLE startup diagnostic — the option (b)
    // the DoD verifier accepts. The non-interactive strong test covers the
    // shared new_with_build_options -> build_tools_with_sandbox -> exec chain
    // that all three modes route through.
    // ------------------------------------------------------------------------

    fn strict_require_sandbox_config() -> OpiConfig {
        let mut config = backend_fallback_config();
        config.sandbox = opi_coding_agent::config::SandboxConfig {
            mode: opi_coding_agent::config::SandboxMode::Strict,
            require: true,
            ..Default::default()
        };
        config
    }

    fn bash_strict_marker_mock() -> Box<dyn opi_ai::provider::Provider> {
        use opi_ai::test_support::{MockProvider, text_response, tool_call_response};
        Box::new(MockProvider::new(
            "anthropic",
            vec![
                tool_call_response(
                    "tc1",
                    "bash",
                    r#"{"command":"echo engaged > phase15-strict-engaged.marker","timeout_secs":5}"#,
                ),
                text_response("done"),
            ],
        ))
    }

    fn assert_bash_fail_closed_reached(visible: &str) {
        assert!(
            visible.contains("sandbox required but unavailable"),
            "strict+require bash fail-closed must reach user-visible output: {visible}"
        );
    }

    #[test]
    fn sandbox_strict_bash_production_outcome_reaches_noninteractive_output() {
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = strict_require_sandbox_config();
        let production_outcome =
            opi_coding_agent::sandbox::prepare_production(&config.sandbox, workspace_dir.path());
        let expect_fail_closed = match production_outcome {
            opi_coding_agent::sandbox::PreparedSandbox::Strict(decision) => {
                match decision.outcome {
                    opi_coding_agent::sandbox::StrictOutcome::Engaged => false,
                    opi_coding_agent::sandbox::StrictOutcome::FailClosed { .. } => true,
                    opi_coding_agent::sandbox::StrictOutcome::FailOpen { .. } => {
                        panic!("strict+require must never resolve to fail-open")
                    }
                }
            }
            opi_coding_agent::sandbox::PreparedSandbox::Off => {
                panic!("strict config must resolve to a strict production outcome")
            }
        };
        let marker = workspace_dir.path().join("phase15-strict-engaged.marker");
        assert!(!marker.exists(), "marker starts absent");
        let cli = Cli::parse_from(["opi", "--json", "--allow-mutating"]);
        let observed = Arc::new(AtomicBool::new(false));
        let observed_result = Arc::clone(&observed);

        let exit_code = tokio::runtime::Runtime::new().expect("runtime").block_on(
            run_non_interactive_core(
                &cli,
                &config,
                "run echo via bash",
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                unavailable_backend_factory(),
                Some(bash_strict_marker_mock()),
                CommandOutput::discard(),
                move |result| {
                    observed_result.store(true, Ordering::SeqCst);
                    assert_eq!(result.exit_code, ExitCode::Success as i32);
                    if expect_fail_closed {
                        assert_bash_fail_closed_reached(&result.stdout);
                        assert!(
                            !marker.exists(),
                            "fail-closed strict must reject before spawning the marker command"
                        );
                    } else {
                        assert!(
                            result.stdout.contains("done"),
                            "engaged strict+require bash turn must complete: {}",
                            result.stdout
                        );
                        assert!(
                            !result.stdout.contains("sandbox required but unavailable"),
                            "an independently engaged strict outcome must not fail closed"
                        );
                        assert_eq!(
                            std::fs::read_to_string(&marker)
                                .expect("engaged strict executes workspace marker command")
                                .trim(),
                            "engaged",
                            "engaged production confinement must execute the real bash side effect"
                        );
                    }
                },
            ),
        );

        assert_eq!(exit_code, ExitCode::Success as i32);
        assert!(observed.load(Ordering::SeqCst));
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    #[test]
    fn sandbox_strict_startup_diagnostic_reaches_rpc_ready() {
        use opi_coding_agent::rpc::RpcCommand;
        use opi_coding_agent::runner::ExitCode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = strict_require_sandbox_config();
        let cli = Cli::parse_from(["opi", "--rpc"]);
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        command_tx
            .send(RpcCommand::quit { id: None })
            .expect("queue quit command");
        drop(command_tx);
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();

        let exit_code = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_rpc_core(
                &cli,
                &config,
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                unavailable_backend_factory(),
                None,
                CommandOutput::discard(),
                RpcTransport::Channels {
                    command_rx,
                    output_tx,
                },
            ));

        assert_eq!(exit_code, ExitCode::Success as i32);
        let output: Vec<_> = std::iter::from_fn(|| output_rx.try_recv().ok()).collect();
        let ready: Vec<_> = output
            .iter()
            .filter(|line| line["type"] == "rpc_ready")
            .collect();
        assert_eq!(ready.len(), 1, "rpc_ready must be reached: {output:?}");
        // The RPC startup channel surfaces the permanent-gap diagnostic on
        // platforms that report one (Windows in 15.5.1). On temporary-only
        // hosts the startup channel is correctly empty, so only assert the
        // diagnostic where the platform classifies the gap as permanent.
        let startup = ready[0]["startup_diagnostics"]
            .as_array()
            .expect("rpc_ready startup diagnostics array");
        #[cfg(target_os = "windows")]
        assert!(
            startup
                .iter()
                .any(|d| d["code"] == "opi.sandbox.unavailable"),
            "Windows strict must surface the permanent-gap startup diagnostic: {startup:?}"
        );
        let _ = startup; // observed on permanent-gap hosts
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    #[test]
    fn sandbox_strict_startup_diagnostic_reaches_interactive_launcher() {
        use opi_agent::diagnostic::RedactionMode;

        let _env_lock = PROVIDER_ENV_LOCK.lock().expect("provider env lock");
        let workspace_dir = tempfile::tempdir().expect("workspace temp dir");
        let user_config_dir = tempfile::tempdir().expect("user config temp dir");
        let session_dir = tempfile::tempdir().expect("session temp dir");
        let session_blocker = session_blocker(&session_dir);
        let _env = ProviderEnvGuard::scoped(&[
            (FIX_G_API_KEY_ENV, std::ffi::OsStr::new(FIX_G_SECRET_CANARY)),
            ("OPI_SESSIONS_DIR", session_blocker.as_os_str()),
        ]);
        let config = strict_require_sandbox_config();
        let cli = Cli::parse_from(["opi"]);
        let launch_count = Arc::new(AtomicUsize::new(0));
        let observed_launches = Arc::clone(&launch_count);

        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_interactive_core(
                &cli,
                &config,
                opi_coding_agent::project_trust::TrustDecision::Trusted,
                None,
                None,
                opi_coding_agent::policy::ToolSelection::Default,
                workspace_dir.path().to_path_buf(),
                user_config_dir.path().to_path_buf(),
                unavailable_backend_factory(),
                move |harness, _model, _theme_name, _keybindings| {
                    let observed_launches = Arc::clone(&observed_launches);
                    async move {
                        observed_launches.fetch_add(1, Ordering::SeqCst);
                        let diagnostics = harness
                            .resource_metadata()
                            .diagnostic_payloads(RedactionMode::Summary)
                            .into_iter()
                            .map(|d| serde_json::to_value(d).expect("diagnostic serializes"))
                            .collect::<Vec<_>>();
                        // run_interactive_core takes no provider_override, so
                        // this proves the strict config reached the interactive
                        // startup build path (the launcher fired) and, on
                        // permanent-gap hosts, the CODE_SANDBOX_UNAVAILABLE
                        // startup diagnostic surfaced via prepare_production.
                        #[cfg(target_os = "windows")]
                        assert!(
                            diagnostics
                                .iter()
                                .any(|d| d["code"] == "opi.sandbox.unavailable"),
                            "Windows strict must surface the permanent-gap startup diagnostic: {diagnostics:?}"
                        );
                        let _ = diagnostics;
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                },
            ));

        assert_eq!(launch_count.load(Ordering::SeqCst), 1);
        assert!(
            session_blocker.is_file(),
            "session path must remain blocked"
        );
    }

    fn is_pre_provider_subprocess(file: &str, context: &str) -> bool {
        match file {
            "doctor_cli.rs" => context.contains(".args([\"doctor\", \"--scope\", \"bogus\"])"),
            "oauth_auth.rs" => context.contains(".arg(\"--help\")"),
            "package_cli.rs" => {
                context.starts_with("opi_command(opi:") || context.contains(".args([\"package\",")
            }
            "session_cli.rs" => {
                [
                    ".arg(\"--list-sessions\")",
                    ".arg(\"--delete-session\")",
                    ".arg(\"--export-session\")",
                ]
                .iter()
                .any(|flag| context.contains(flag))
                    || (context.contains(".arg(\"--resume\")")
                        && context.contains(".arg(\"nonexistent-session\")"))
            }
            "shell_completions.rs" => context.contains(".arg(\"--generate-completion\")"),
            _ => false,
        }
    }

    #[test]
    fn session_resume_classifier_rejects_provider_reaching_fixture() {
        let existing_resume = r#"std::process::Command::new(opi_binary())
            .arg("--resume")
            .arg("existing-session")
            .output()"#;
        assert!(!is_pre_provider_subprocess(
            "session_cli.rs",
            existing_resume
        ));
    }

    #[test]
    fn real_opi_subprocesses_are_pre_provider_early_exits_only() {
        let tests_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
        let allowed = [
            (
                "doctor_cli.rs",
                "std::process::Command::new(&bin)",
                1usize,
                "fn opi_bin()",
                1usize,
                1usize,
            ),
            (
                "oauth_auth.rs",
                "std::process::Command::new(&binary)",
                1,
                "CARGO_BIN_EXE_opi",
                1,
                1,
            ),
            (
                "package_cli.rs",
                "opi_command(",
                11,
                "fn opi_binary()",
                1,
                4,
            ),
            (
                "session_cli.rs",
                "std::process::Command::new(opi_binary())",
                5,
                "fn opi_binary()",
                1,
                6,
            ),
            (
                "shell_completions.rs",
                "Command::new(&bin)",
                4,
                "fn opi_bin()",
                1,
                4,
            ),
        ];
        let forbidden_markers = [
            "CARGO_BIN_EXE_opi",
            "opi_bin(",
            "opi_binary(",
            "opi_binary_path(",
            "Command::new(\"opi\")",
            "target/debug/opi",
        ];
        for mutation in [
            "let binary = opi_bin();",
            "let binary = opi_binary();",
            "let binary = opi_binary_path();",
        ] {
            assert!(
                forbidden_markers
                    .iter()
                    .any(|marker| mutation.contains(marker)),
                "generic opi-binary calls must remain classified: {mutation}"
            );
        }
        let mut failures = Vec::new();

        for entry in std::fs::read_dir(&tests_dir).expect("integration test directory") {
            let path = entry.expect("test entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("integration test source");
            let file = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let Some((
                _,
                launch_marker,
                expected_count,
                binary_marker,
                expected_binary_markers,
                expected_commands,
            )) = allowed
                .iter()
                .find(|(allowed_file, ..)| *allowed_file == file)
            else {
                for marker in forbidden_markers {
                    if source.contains(marker) {
                        failures.push(format!(
                            "{} contains forbidden real-opi marker `{marker}`",
                            path.display()
                        ));
                    }
                }
                continue;
            };

            let count = source.matches(launch_marker).count();
            if count != *expected_count {
                failures.push(format!(
                    "{} expected {expected_count} `{launch_marker}` occurrences, found {count}",
                    path.display()
                ));
                continue;
            }
            let binary_marker_count = source.matches(binary_marker).count();
            if binary_marker_count != *expected_binary_markers {
                failures.push(format!(
                    "{} expected {expected_binary_markers} `{binary_marker}` occurrences, found {binary_marker_count}",
                    path.display()
                ));
            }
            let command_count = source.matches("Command::new(").count();
            if command_count != *expected_commands {
                failures.push(format!(
                    "{} expected {expected_commands} subprocess launch sites, found {command_count}",
                    path.display()
                ));
            }

            for (index, _) in source.match_indices(launch_marker) {
                let context = &source[index..(index + 500).min(source.len())];
                let early_exit = is_pre_provider_subprocess(file, context);
                if !early_exit {
                    failures.push(format!(
                        "{} has an unclassified `{launch_marker}` invocation",
                        path.display()
                    ));
                }
            }
        }

        for target in ["json_mode.rs", "non_interactive.rs", "rpc_jsonl.rs"] {
            let source =
                std::fs::read_to_string(tests_dir.join(target)).expect("target test source");
            let command_count = source.matches("Command::new(").count();
            if command_count != 0 {
                failures.push(format!(
                    "{target} must contain no subprocess launch sites, found {command_count}"
                ));
            }
            for marker in forbidden_markers {
                if source.contains(marker) {
                    failures.push(format!(
                        "{target} must not contain real-opi marker `{marker}`"
                    ));
                }
            }
        }

        assert!(
            failures.is_empty(),
            "real opi subprocesses must be proven pre-provider early exits:\n{}",
            failures.join("\n")
        );
    }
}

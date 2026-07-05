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
            let mut diagnostics = session.diagnostics;
            diagnostics.extend(ctx.diagnostics);
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

    let config = match resolve_config(ConfigSource {
        cli_model: cli.model.clone(),
        config_path: cli.config.clone(),
        env_model: std::env::var("OPI_MODEL").ok(),
        project_dir: resume_info
            .as_ref()
            .map(|info| info.original_cwd.clone())
            .or_else(|| std::env::current_dir().ok()),
        user_config_path: None,
    }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("opi: config error: {e}");
            std::process::exit(2);
        }
    };

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
            run_rpc(&cli, &config, resumed_messages, resume_info, tool_selection).await
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
            run_non_interactive(
                &cli,
                &config,
                &prompt_text,
                resumed_messages,
                resume_info,
                tool_selection,
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
        rt.block_on(async {
            run_interactive(&cli, &config, resumed_messages, resume_info, tool_selection).await
        });
    }
}

/// Run the top-level `opi doctor` command and return the exit code.
///
/// Network-free: config is resolved best-effort so a broken config surfaces as
/// a config-scope error diagnostic (exit 2) rather than an internal failure
/// (exit 1). An unparseable `--scope` list is an internal failure (exit 1).
fn run_doctor_cli(cli: &Cli, scope: Option<&str>, json: bool) -> i32 {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::doctor::{
        DoctorContext, DoctorScope, format_json, format_text, run_doctor,
    };

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
    };

    let report = run_doctor(&scopes, &ctx);
    if json {
        let json_out = format_json(&report);
        if json_out.is_empty() {
            println!();
        } else {
            println!("{json_out}");
        }
    } else {
        print!("{}", format_text(&report));
    }
    report.exit_code()
}

async fn run_non_interactive(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    prompt_text: &str,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
) -> i32 {
    use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};

    if prompt_text.is_empty() {
        eprintln!("opi: no prompt provided");
        return ExitCode::ConfigError as i32;
    }

    let provider = match opi_coding_agent::provider_factory::build_provider(config) {
        Ok(p) => p,
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Auth(msg)) => {
            eprintln!("opi: {msg}");
            return ExitCode::AuthFailure as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Config(msg)) => {
            eprintln!("opi: {msg}");
            return ExitCode::ConfigError as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Provider(e)) => {
            eprintln!("opi: {e}");
            return ExitCode::ConfigError as i32;
        }
    };

    let allow_mutating = cli.allow_mutating || config.defaults.allow_mutating_tools;

    let user_system_prompt =
        cli.system
            .as_ref()
            .and_then(|path| match std::fs::read_to_string(path) {
                Ok(content) => Some(content),
                Err(e) => {
                    eprintln!(
                        "opi: warning: failed to read system prompt file {}: {e}",
                        path.display()
                    );
                    None
                }
            });

    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let user_config_dir = opi_coding_agent::config::user_config_dir();
    let runtime_startup = opi_coding_agent::runtime_packages::start_installed_package_runtime(
        &workspace_root,
        &user_config_dir,
    )
    .await;

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
        Some(runtime_startup),
        cli.trace.clone(),
    ) {
        Ok(runner) => runner,
        Err(e) => {
            eprintln!("opi: {e}");
            return ExitCode::ConfigError as i32;
        }
    };

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
                    eprintln!("opi: {e}");
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

    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprintln!("{}", result.stderr);
    }

    result.exit_code
}

async fn run_rpc(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
) -> i32 {
    use opi_coding_agent::rpc::RpcRunner;
    use opi_coding_agent::runner::ExitCode;

    let provider = match opi_coding_agent::provider_factory::build_provider(config) {
        Ok(p) => p,
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Auth(msg)) => {
            eprintln!("opi: {msg}");
            return ExitCode::AuthFailure as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Config(msg)) => {
            eprintln!("opi: {msg}");
            return ExitCode::ConfigError as i32;
        }
        Err(opi_coding_agent::provider_factory::ProviderBuildError::Provider(e)) => {
            eprintln!("opi: {e}");
            return ExitCode::ConfigError as i32;
        }
    };

    let allow_mutating = cli.allow_mutating || config.defaults.allow_mutating_tools;

    let user_system_prompt =
        cli.system
            .as_ref()
            .and_then(|path| match std::fs::read_to_string(path) {
                Ok(content) => Some(content),
                Err(e) => {
                    eprintln!(
                        "opi: warning: failed to read system prompt file {}: {e}",
                        path.display()
                    );
                    None
                }
            });

    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let user_config_dir = opi_coding_agent::config::user_config_dir();
    let runtime_startup = opi_coding_agent::runtime_packages::start_installed_package_runtime(
        &workspace_root,
        &user_config_dir,
    )
    .await;

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
            eprintln!("opi: {e}");
            return ExitCode::ConfigError as i32;
        }
    };

    runner.run().await
}

async fn run_interactive(
    cli: &Cli,
    config: &opi_coding_agent::config::OpiConfig,
    resumed_messages: Option<Vec<opi_agent::message::AgentMessage>>,
    resume_info: Option<ResumeInfo>,
    tool_selection: ToolSelection,
) {
    use opi_coding_agent::harness::{CodingHarness, InteractiveCodingHooks};
    use opi_coding_agent::interactive;

    let provider = match opi_coding_agent::provider_factory::build_provider(config) {
        Ok(p) => p,
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

    let user_system_prompt = cli
        .system
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok());

    let hooks = Box::new(InteractiveCodingHooks::new(true));
    let initial_messages = resumed_messages.unwrap_or_default();
    let workspace_root = resume_info
        .as_ref()
        .map(|info| info.original_cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let user_config_dir = opi_coding_agent::config::user_config_dir();
    let runtime_startup = opi_coding_agent::runtime_packages::start_installed_package_runtime(
        &workspace_root,
        &user_config_dir,
    )
    .await;

    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, tool_selection.clone())
            .expect("interactive tool config should be valid");
    let mut builder = CodingHarness::builder(
        provider,
        config.defaults.model.clone(),
        config.clone(),
        workspace_root,
    )
    .hooks(hooks)
    .initial_messages(initial_messages)
    .tool_selection(tool_selection)
    .tool_config(tool_config)
    .extension_registry(runtime_startup.extension_registry)
    .installed_packages(runtime_startup.installed_packages)
    .startup_diagnostics(runtime_startup.diagnostics);
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
    if let Err(e) =
        interactive::run_interactive_tui(harness, model_display, &theme_name, keybindings).await
    {
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
    let collection = match opi_coding_agent::provider_factory::build_collection_for_listing(config)
    {
        Ok(collection) => collection,
        Err(opi_coding_agent::provider_factory::ListModelsError::MissingCredentials) => {
            eprintln!("opi: no models available (configure API keys to list models)");
            return 1;
        }
        Err(opi_coding_agent::provider_factory::ListModelsError::Config(msg)) => {
            eprintln!("opi: config error: {msg}");
            return 2;
        }
    };
    let entries =
        opi_coding_agent::model_listing::model_entries_from_registry(collection.registry());

    if entries.is_empty() {
        eprintln!("opi: no models available (configure API keys to list models)");
        return 1;
    }

    if json_output {
        for entry in &entries {
            let json = serde_json::json!({
                "model": entry.model_id,
                "provider": entry.provider_id,
                "display_name": entry.display_name,
            });
            println!("{json}");
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
        println!(
            "{:<width_prov$}  {:<width_id$}  DISPLAY NAME",
            "PROVIDER",
            "MODEL ID",
            width_prov = max_prov,
            width_id = max_id,
        );
        println!(
            "{}  {}  {}",
            "-".repeat(max_prov),
            "-".repeat(max_id),
            "-".repeat(max_name),
        );

        for entry in &entries {
            println!(
                "{:<width_prov$}  {:<width_id$}  {}",
                entry.provider_id,
                entry.model_id,
                entry.display_name,
                width_prov = max_prov,
                width_id = max_id,
            );
        }
    }

    0
}

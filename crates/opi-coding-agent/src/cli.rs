//! CLI argument parsing (S8.4).

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::config::ExecutionStrategy;

/// Stable remediation text for the removed `[sandbox]` / `--sandbox` /
/// `--sandbox-require` inputs, pointing at the execution-backend surface and
/// the package workflow. Surfaced by the hidden legacy clap args' value
/// parser so a user running the old flags gets a targeted pointer instead of
/// a bare "unexpected argument". This is rejection, NOT an alias: the old
/// behavior is gone and the error always fires.
const LEGACY_SANDBOX_REMEDIATION: &str = "the --sandbox flag was removed with the native sandbox; use --execution-backend <local|adapter-id> or [execution] strategy = \"fixed\", backend = \"opi-sandbox\", and install/enable the opi-sandbox package (opi package add <dir>; opi package enable opi-sandbox)";

/// Value parser for the removed legacy `--sandbox` / `--sandbox-require` flags:
/// it always errors with [`LEGACY_SANDBOX_REMEDIATION`], so any invocation of
/// either flag fails at parse time with migration remediation.
fn legacy_sandbox_flag_rejected(_value: &str) -> Result<String, LegacySandboxInput> {
    Err(LegacySandboxInput)
}

/// Error type backing [`legacy_sandbox_flag_rejected`]; its `Display` is the
/// stable remediation text.
#[derive(Debug)]
struct LegacySandboxInput;

impl std::fmt::Display for LegacySandboxInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(LEGACY_SANDBOX_REMEDIATION)
    }
}

impl std::error::Error for LegacySandboxInput {}

/// Supported shells for completion generation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellName {
    Bash,
    Zsh,
    Fish,
    #[clap(name = "powershell")]
    PowerShell,
    Elvish,
}

/// Output format for `--export-session` (Phase 13.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    #[clap(name = "markdown", alias = "md")]
    Markdown,
    #[clap(name = "json")]
    Json,
}

/// Redaction mode for `--export-session` (Phase 13.5). Maps to the Phase 7
/// `RedactionMode` plus an explicit opt-out for trusted-local exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportRedactMode {
    /// Phase 7 `Summary`: scrub secrets, content-sensitive fields, and
    /// absolute paths. Safe-by-default.
    #[clap(name = "summary")]
    Summary,
    /// Phase 7 `Verbose`: scrub secrets and content-sensitive fields, keep
    /// absolute paths.
    #[clap(name = "verbose")]
    Verbose,
    /// Skip redaction entirely. Only for trusted-local contexts where the
    /// export never leaves the host.
    #[clap(name = "none")]
    None,
}

impl From<ShellName> for clap_complete::Shell {
    fn from(s: ShellName) -> Self {
        match s {
            ShellName::Bash => clap_complete::Shell::Bash,
            ShellName::Zsh => clap_complete::Shell::Zsh,
            ShellName::Fish => clap_complete::Shell::Fish,
            ShellName::PowerShell => clap_complete::Shell::PowerShell,
            ShellName::Elvish => clap_complete::Shell::Elvish,
        }
    }
}

/// opi — AI coding agent.
#[derive(Debug, Parser)]
#[command(
    name = "opi",
    version,
    about = "AI coding agent",
    after_long_help = "\
Tool policy:
  Interactive mode enables read, write, edit, and bash.
  Non-interactive/RPC mode defaults to read, grep, find, ls, and glob.
  write, edit, and bash require --allow-mutating or defaults.allow_mutating_tools = true outside interactive mode.
  --no-tools disables all tools; --tools is an allowlist; --no-builtin-tools removes built-ins.

Bash policy:
  bash runs one foreground command from the workspace root.
  Windows uses cmd /C; Unix uses sh -c.
  The default timeout is 30 seconds; timeout_secs overrides it per call.
  Combined stdout/stderr are capped at 64 KiB. Larger output sets truncated and may write the complete output path in details.full_output.
  This is a tool-selection check, not a permission popup or sandbox subsystem.

Execution policy:
  --execution-backend local|<adapter-id> selects the fixed command.execute backend.
  --execution-strategy fixed|rules|model selects the routing strategy.
  Both are routing overrides only: they cannot grant package trust or adapter permission.
  Capability permission ([execution.permissions]) is read from the USER config only; project permission sections are rejected.
  --allow-mutating gates bash tool availability independently from adapter permission.

Interactive authentication:
  /login <provider> starts login for Anthropic, GitHub Copilot, or OpenAI Codex.
  /logout <provider> deletes that provider's stored credential.
  Non-interactive, JSON, and RPC modes never start login; CredentialNeeded reports the provider and /login remediation.
"
)]
pub struct Cli {
    /// Model spec, e.g. anthropic:claude-sonnet-4.
    #[arg(short = 'm', long)]
    pub model: Option<String>,

    /// Config file path.
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// System prompt file.
    #[arg(short = 's', long)]
    pub system: Option<PathBuf>,

    /// Single prompt mode (non-interactive).
    #[arg(long)]
    pub non_interactive: bool,

    /// Allow mutating tools (write, edit, bash) in non-interactive mode.
    #[arg(long)]
    pub allow_mutating: bool,

    /// Trust this project and load its project resources (`.opi/config.toml`,
    /// project skills/fragments/themes/extensions, project packages, and project
    /// `AGENTS.md`/`CLAUDE.md`). Mutually exclusive with `--no-trust`.
    /// Overrides `[defaults] default_project_trust` and the trust store.
    #[arg(long, conflicts_with = "no_trust")]
    pub trust: bool,

    /// Do not trust this project; skip its project resources (user-global
    /// resources and explicit `--config`/`--system-prompt` still apply, tools are
    /// unchanged). Mutually exclusive with `--trust`.
    #[arg(long, conflicts_with = "trust")]
    pub no_trust: bool,

    /// Removed legacy `--sandbox` flag, kept as a hidden arg whose parser
    /// always errors with `LEGACY_SANDBOX_REMEDIATION` so ANY invocation
    /// (bare or valued) gets a targeted migration pointer (rejection, not an
    /// alias). `num_args = 0..=1` + `default_missing_value` route a bare
    /// `--sandbox` through the value parser too, so it carries remediation
    /// instead of clap's stock "a value is required" error.
    #[arg(
        long,
        hide = true,
        num_args = 0..=1,
        default_missing_value = "",
        value_parser = legacy_sandbox_flag_rejected
    )]
    pub sandbox: Option<String>,

    /// Removed legacy `--sandbox-require` flag, kept hidden so bare usage
    /// (`--sandbox-require`) and valued usage both error with the same
    /// migration remediation at parse time.
    #[arg(
        long,
        hide = true,
        num_args = 0..=1,
        default_missing_value = "",
        value_parser = legacy_sandbox_flag_rejected
    )]
    pub sandbox_require: Option<String>,

    /// Execution backend override for `command.execute`: `local` (built-in) or an
    /// installed adapter id. Selects the `fixed` strategy. This is a routing
    /// override only — it cannot grant trust or permission. Overrides
    /// `[execution] backend` from layered TOML.
    #[arg(long, value_name = "BACKEND")]
    pub execution_backend: Option<String>,

    /// Execution routing strategy override: `fixed`, `rules`, or `model`. This is
    /// a routing override only — it cannot grant trust or permission. Overrides
    /// `[execution] strategy` from layered TOML.
    #[arg(long, value_enum, value_name = "STRATEGY")]
    pub execution_strategy: Option<ExecutionStrategy>,

    /// Output NDJSON events to stdout (non-interactive mode).
    #[arg(long)]
    pub json: bool,

    /// Compact `--json` output: streamed `text_delta` updates omit the
    /// redundant cumulative snapshot, making long streamed turns ~linear in
    /// bytes. Default `--json` output is unchanged.
    #[arg(long, default_value_t = false)]
    pub json_compact: bool,

    /// RPC JSONL mode: bidirectional command/event protocol over stdin/stdout.
    #[arg(long)]
    pub rpc: bool,

    /// List all sessions.
    #[arg(long)]
    pub list_sessions: bool,

    /// Resume a session by ID.
    #[arg(long)]
    pub resume: Option<String>,

    /// Fork a session by ID into a new session.
    #[arg(long)]
    pub fork: Option<String>,

    /// Delete a session by ID.
    #[arg(long)]
    pub delete_session: Option<String>,

    /// Export a session to a local markdown or json file and exit. The
    /// argument is a session id resolved against the sessions directory, or a
    /// path to a session JSONL file (Phase 13.5).
    #[arg(long, value_name = "ID_OR_PATH")]
    pub export_session: Option<String>,

    /// Output file path for `--export-session`. Required when exporting.
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Output format for `--export-session`. Default: markdown.
    #[arg(long, value_enum, value_name = "FORMAT", default_value_t = ExportFormat::Markdown)]
    pub format: ExportFormat,

    /// Export the full branch tree instead of just the active branch.
    #[arg(long)]
    pub full_tree: bool,

    /// Omit tool result output from the export.
    #[arg(long)]
    pub exclude_tool_output: bool,

    /// Omit assistant thinking content from the export.
    #[arg(long)]
    pub exclude_thinking: bool,

    /// Redaction mode for `--export-session`: summary, verbose, or none.
    /// Default: summary.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = ExportRedactMode::Summary)]
    pub redact: ExportRedactMode,

    /// Generate shell completions to stdout.
    #[arg(long, value_name = "SHELL")]
    pub generate_completion: Option<ShellName>,

    /// Enable debug tracing.
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Write each run's evidence.jsonl + manifest.json under PATH
    /// (interactive, non-interactive, JSON, and RPC; 0.x unstable, opt-in).
    #[arg(long, value_name = "PATH")]
    pub trace: Option<PathBuf>,

    /// Tool allowlist (comma-separated, e.g. "read,glob").
    #[arg(long, value_delimiter = ',')]
    pub tools: Option<Vec<String>>,

    /// Disable all tools.
    #[arg(long)]
    pub no_tools: bool,

    /// Disable built-in tools (reserved for Phase 4 extension tools).
    #[arg(long)]
    pub no_builtin_tools: bool,

    /// Attach image file(s) to the prompt.
    #[arg(long)]
    pub image: Vec<PathBuf>,

    /// List available models and exit.
    #[arg(long)]
    pub list_models: bool,

    /// Package subcommand group.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Initial prompt (positional).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub prompt: Vec<String>,
}

/// Top-level subcommands for opi.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage extension packages.
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    /// Summarize local health across config, provider, package, session, tui, rpc.
    ///
    /// Makes no paid model calls or network checks by default. Distinct from
    /// `opi package doctor`.
    Doctor {
        /// Output diagnostics as NDJSON (one JSON object per line).
        #[arg(long)]
        json: bool,
        /// Comma-separated scope list (config,provider,package,session,tui,rpc).
        /// Default: all scopes.
        #[arg(long)]
        scope: Option<String>,
    },
}

/// Package management subcommands.
#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    /// Add a package to the store.
    Add {
        /// Package source (local path or git URL).
        source: String,
        /// Use project-local scope (`.opi/packages.toml`).
        #[arg(short = 'l', long = "local")]
        local: bool,
    },
    /// Remove a package from the store.
    Remove {
        /// Package name or source to remove.
        name_or_source: String,
        /// Use project-local scope.
        #[arg(short = 'l', long = "local")]
        local: bool,
    },
    /// List installed packages.
    List {
        /// Output as JSON (one JSON object per line).
        #[arg(long)]
        json: bool,
    },
    /// Validate installed packages and report diagnostics.
    Doctor {
        /// Output diagnostics as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Grant Package Trust and enable an executable contribution package
    /// (Phase 16.5). First enablement requires interactive confirmation and
    /// refuses in a non-TTY/machine-facing invocation.
    Enable {
        /// Package name to enable.
        name: String,
    },
    /// Disable an enabled contribution package without removing its trust
    /// record (Phase 16.5).
    Disable {
        /// Package name to disable.
        name: String,
    },
}

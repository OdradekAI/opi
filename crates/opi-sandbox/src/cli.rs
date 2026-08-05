//! The human `opi-sandbox` CLI: subcommand dispatch, a hand-rolled `run` parser,
//! the exit-code mapping, and `doctor` (Phase 16 task 16.11.2).
//!
//! # Control flow (Phase 16 task 16.11.2 audit fold: platform-posture-honesty,
//! dod-precision-scope)
//!
//! [`run`] parses `argv` (skipping `argv[0]`) and dispatches: `--help`/`--version`
//! return 0; `doctor` builds the stable report from the platform posture and
//! returns 0 (completed, even when `supported == false`); `run` parses its
//! command, checks the platform posture, and either refuses pre-start or
//! executes. The platform gate lives OUTSIDE [`execute`]: [`execute`] is pure
//! plumbing (`SandboxRequest` + `&SandboxRunner` -> exit code) with no platform
//! check, which is the seam the portable `cli_contract` tests drive directly
//! with an injected [`NoRestriction`](crate::NoRestriction) runner.
//!
//! `platform::current` reports the host posture. On Windows (and on macOS until
//! 16.14.1) `supported == false`, so production `run` refuses before target
//! start (exit 125); on supported Linux (16.13) `supported == true` and `run`
//! executes the target under Landlock + seccomp. The unsupported refusal is
//! exercised directly here; the native Linux run is exercised by
//! `tests/linux_policy`.
//!
//! # Exit mapping (spec `### Human CLI`)
//!
//! | path | exit |
//! |---|---|
//! | target normal exit | the target's exit code, verbatim |
//! | Unix signal termination | `128 + signal` |
//! | `run` timeout | `124` |
//! | cooperative cancellation | `130` |
//! | pre-start setup failure (`ProgramNotFound`/`RestrictionSetup`/`SpawnFailed`/`UnsupportedPlatform`) | `125` |
//! | malformed `run`/usage (`InvalidRequest`, missing flags, bad values) | `2` |
//! | `--help`/`--version`/`doctor` completed | `0` |
//!
//! After target start, an ordinary target exit is returned verbatim even when it
//! equals a reserved code; the structured SDK outcome makes the pre-start
//! `125` and post-start `Exited{125}` paths disjoint at the implementation layer
//! (they collide only at the shell, per spec `### Human CLI`).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::EnvInherit;
use tokio_util::sync::CancellationToken;

use crate::platform;
use crate::policy::{Mechanism, NetworkPolicy, Profile, SandboxPolicy};
use crate::runner::{
    SandboxEvent, SandboxOutcome, SandboxRequest, SandboxRunner, SetupFailureReason, StdinPolicy,
};

/// The fixed `run` timeout applied when the human CLI invokes a target. The
/// spec `### Human CLI` grammar carries no `--timeout`, so the CLI applies a
/// single effectively-unbounded finite default (365 days); this satisfies the
/// SDK's non-zero timeout requirement by construction, so `InvalidRequest` is
/// unreachable from the human `run` path.
pub const DEFAULT_RUN_TIMEOUT: Duration = Duration::from_secs(365 * 86_400);

/// A redacted usage error from the `run` parser; the CLI maps every usage error
/// to exit `2`.
#[derive(Debug, Clone)]
pub struct UsageError {
    /// A short human-readable reason.
    pub message: String,
}

impl UsageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
    fn missing_flag(flag: &str) -> Self {
        Self::new(format!("missing required flag `{flag}`"))
    }
    fn missing_value(flag: &str) -> Self {
        Self::new(format!("missing value for flag `{flag}`"))
    }
    fn duplicate(flag: &str) -> Self {
        Self::new(format!("duplicate flag `{flag}`"))
    }
    fn invalid_value(flag: &str, value: &str) -> Self {
        Self::new(format!("invalid value `{value}` for flag `{flag}`"))
    }
    fn unknown_token(token: &str) -> Self {
        Self::new(format!(
            "unknown flag or positional token `{token}` before `--`"
        ))
    }
    fn missing_program() -> Self {
        Self::new("missing program after `--`")
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// The parsed `run` command: three required flags plus an explicit program and
/// argument vector. Produced by [`parse_run`]; consumed by [`build_request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    /// The canonical workspace root (`--workspace`).
    pub workspace: PathBuf,
    /// The requested profile (`--profile`; only `workspace-write` is valid).
    pub profile: Profile,
    /// The requested network policy (`--network deny|allow`).
    pub network: NetworkPolicy,
    /// The explicit program to execute (after `--`).
    pub program: PathBuf,
    /// The explicit argument vector (after the program).
    pub args: Vec<String>,
}

/// Parse the `run` subcommand tokens (everything after the literal `run`
/// subcommand). The grammar is:
/// `--workspace <PATH> --profile workspace-write --network <deny|allow> -- <PROGRAM> [ARGS...]`.
///
/// The three flags are required and may appear in any order before `--`. `--`
/// terminates flag parsing absolutely: every later token (including
/// `--workspace`-shaped ones) becomes the program or an argument. An empty
/// program, a missing flag, an unknown flag/value, a duplicate flag, or a flag
/// value shaped like a flag (`--foo`) each produce a [`UsageError`] the CLI maps
/// to exit `2`. A present-but-nonexistent program is NOT a parse error — the
/// runner detects it at spawn and the CLI maps it to `125`.
pub fn parse_run(args: &[String]) -> Result<RunCommand, UsageError> {
    let mut workspace: Option<PathBuf> = None;
    let mut profile: Option<Profile> = None;
    let mut network: Option<NetworkPolicy> = None;
    let mut program_and_args: Option<Vec<String>> = None;

    let mut i = 0;
    while i < args.len() {
        let token = args[i].as_str();
        if token == "--" {
            program_and_args = Some(args[i + 1..].to_vec());
            break;
        }
        match token {
            "--workspace" => {
                let value = take_value(args, &mut i, "--workspace")?;
                if value.is_empty() {
                    return Err(UsageError::invalid_value("--workspace", "(empty)"));
                }
                if workspace.is_some() {
                    return Err(UsageError::duplicate("--workspace"));
                }
                workspace = Some(PathBuf::from(value));
            }
            "--profile" => {
                let value = take_value(args, &mut i, "--profile")?;
                if profile.is_some() {
                    return Err(UsageError::duplicate("--profile"));
                }
                profile = Some(match value.as_str() {
                    "workspace-write" => Profile::WorkspaceWrite,
                    other => return Err(UsageError::invalid_value("--profile", other)),
                });
            }
            "--network" => {
                let value = take_value(args, &mut i, "--network")?;
                if network.is_some() {
                    return Err(UsageError::duplicate("--network"));
                }
                network = Some(match value.as_str() {
                    "deny" => NetworkPolicy::Deny,
                    "allow" => NetworkPolicy::Allow,
                    other => return Err(UsageError::invalid_value("--network", other)),
                });
            }
            other => return Err(UsageError::unknown_token(other)),
        }
        i += 1;
    }

    let workspace = workspace.ok_or_else(|| UsageError::missing_flag("--workspace"))?;
    let _ = profile.ok_or_else(|| UsageError::missing_flag("--profile"))?;
    let network = network.ok_or_else(|| UsageError::missing_flag("--network"))?;
    let rest = program_and_args.unwrap_or_default();
    if rest.is_empty() {
        return Err(UsageError::missing_program());
    }
    let program = PathBuf::from(&rest[0]);
    let args = rest[1..].to_vec();
    Ok(RunCommand {
        workspace,
        profile: Profile::WorkspaceWrite,
        network,
        program,
        args,
    })
}

/// Read the value token following the flag at `args[*i]`, rejecting a value that
/// is missing, is the `--` separator, or looks like another flag (so a missing
/// value cannot accidentally consume the next flag). Advances `*i` past the
/// value on success.
fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, UsageError> {
    if *i + 1 >= args.len() {
        return Err(UsageError::missing_value(flag));
    }
    let value = &args[*i + 1];
    if value == "--" || value.starts_with("--") {
        return Err(UsageError::missing_value(flag));
    }
    *i += 1;
    Ok(value.clone())
}

/// Build the SDK [`SandboxRequest`] from a parsed [`RunCommand`]. Pure, so a
/// test can prove the request carries the direct-CLI's policies (terminal-stdin
/// inheritance, host-environment inheritance, the fixed default timeout).
pub fn build_request(cmd: &RunCommand) -> SandboxRequest {
    SandboxRequest {
        program: cmd.program.clone(),
        args: cmd.args.iter().map(OsString::from).collect(),
        workspace: cmd.workspace.clone(),
        cwd: cmd.workspace.clone(),
        timeout: DEFAULT_RUN_TIMEOUT,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
        // The human direct CLI inherits the terminal stdin (spec `### Human CLI`).
        stdin: StdinPolicy::Inherit,
        cancel: None,
    }
}

/// Drive one sandboxed run to terminal completion, pass the target's captured
/// stdout/stderr through to `stdout`/`stderr` as bytes, and return the mapped
/// exit code. This is the pure plumbing seam (no platform check); production
/// [`run`] gates the platform posture before calling this, and the portable
/// `cli_contract` tests call it directly with an injected runner.
///
/// `stdout`/`stderr` are injected `std::io::Write` sinks so byte-exact
/// pass-through (including non-UTF-8 bytes) is testable without capturing
/// process stdout. The SDK buffers the target's output into a `Vec<u8>` before
/// the run completes, so a single synchronous write per stream suffices.
pub async fn execute(
    runner: &SandboxRunner,
    request: SandboxRequest,
    stdout: &mut (dyn std::io::Write + Send),
    stderr: &mut (dyn std::io::Write + Send),
) -> i32 {
    let mut run = match runner.run(request) {
        Ok(run) => run,
        Err(failure) => return map_setup_failure(&failure.reason),
    };
    loop {
        match next_event(&mut run).await {
            Some(SandboxEvent::Completed(result)) => {
                let _ = stdout.write_all(&result.stdout);
                let _ = stderr.write_all(&result.stderr);
                if result.stdout_truncated {
                    let _ = stderr.write_all(b"\nopi-sandbox: stdout capture truncated\n");
                }
                if result.stderr_truncated {
                    let _ = stderr.write_all(b"\nopi-sandbox: stderr capture truncated\n");
                }
                return map_outcome(&result.outcome);
            }
            // Started is emitted first by the library stream; Output/Diagnostic
            // are reserved for the binary/protocol layer and not emitted here.
            Some(_) => continue,
            // The stream ended without a Completed event: an internal failure.
            None => return 1,
        }
    }
}

/// Poll the owned run stream for its next event without a `futures-util`
/// normal dependency (the crate depends only on `futures_core` for the
/// [`Stream`] trait).
async fn next_event(run: &mut crate::runner::SandboxRun) -> Option<SandboxEvent> {
    std::future::poll_fn(|cx| Pin::new(&mut *run).poll_next(cx)).await
}

/// Map a structured terminal outcome to the CLI exit code (spec `### Human CLI`).
fn map_outcome(outcome: &SandboxOutcome) -> i32 {
    match outcome {
        SandboxOutcome::Exited { code } => code.unwrap_or(1),
        SandboxOutcome::Signaled { signal } => 128 + signal,
        SandboxOutcome::TimedOut => 124,
        SandboxOutcome::Cancelled => 130,
    }
}

/// Map a pre-start setup failure to the CLI exit code. `InvalidRequest` is usage
/// (`2`) — unreachable from the human CLI because [`parse_run`] validates the
/// request first, but mapped to `2` as defense-in-depth; every other variant is
/// a pre-start failure (`125`). `ProgramNotFound` is `125` (the CLI does NOT
/// follow the POSIX-shell `127` convention).
fn map_setup_failure(reason: &SetupFailureReason) -> i32 {
    match reason {
        SetupFailureReason::InvalidRequest => 2,
        SetupFailureReason::ProgramNotFound
        | SetupFailureReason::RestrictionSetup
        | SetupFailureReason::SpawnFailed
        | SetupFailureReason::UnsupportedPlatform => 125,
    }
}

/// The stable `doctor --json` object (schema version 1). Built from the platform
/// posture; `target` is the OS family (`std::env::consts::OS`, the value that
/// determines the restriction model), not the full target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// The doctor object schema version.
    pub schema_version: u32,
    /// Whether the platform can establish the requested restriction contract.
    pub supported: bool,
    /// The OS family (matches the `cfg(target_os)` dispatch).
    pub target: &'static str,
    /// The mechanism names a supported platform installs (empty in 16.11.2).
    pub mechanisms: Vec<String>,
    /// The available profile names.
    pub profiles: Vec<String>,
    /// Honest per-platform caveats.
    pub limitations: Vec<String>,
}

/// Build the [`DoctorReport`] from the current platform posture. Pure, so the
/// portable tests assert its content directly.
pub fn doctor_report() -> DoctorReport {
    let posture = platform::current();
    DoctorReport {
        schema_version: 1,
        supported: posture.supported,
        target: std::env::consts::OS,
        mechanisms: posture.mechanisms.iter().map(mechanism_name).collect(),
        profiles: vec!["workspace-write".to_string()],
        limitations: posture.limitations,
    }
}

/// The stable wire name of a mechanism. Each names the KERNEL MECHANISM (not a
/// user-invoked tool), so the macOS sandbox kext is `seatbelt`, not
/// `sandbox-exec` (the helper); this mirrors the `landlock`/`seccomp` convention.
fn mechanism_name(mechanism: &Mechanism) -> String {
    match mechanism {
        Mechanism::None => "none".to_string(),
        Mechanism::Landlock => "landlock".to_string(),
        Mechanism::Seccomp => "seccomp".to_string(),
        Mechanism::Seatbelt => "seatbelt".to_string(),
    }
}

/// Print the doctor report and return the exit code (`0` for a completed
/// diagnostic, even when `supported == false`).
pub fn doctor(json: bool) -> i32 {
    let report = doctor_report();
    if json {
        println!("{}", doctor_json(&report));
    } else {
        println!("opi-sandbox doctor");
        println!("  supported: {}", report.supported);
        println!("  target: {}", report.target);
        let mechanisms = if report.mechanisms.is_empty() {
            "(none)".to_string()
        } else {
            report.mechanisms.join(", ")
        };
        println!("  mechanisms: {mechanisms}");
        println!("  profiles: {}", report.profiles.join(", "));
        for limitation in &report.limitations {
            println!("  limitation: {limitation}");
        }
    }
    0
}

/// Render the doctor report as the stable JSON object. The schema is fixed and
/// every string is a controlled literal (OS family, profile/mechanism names, or
/// static limitation prose), so a small escaper suffices in place of a JSON
/// dependency.
fn doctor_json(report: &DoctorReport) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\"schema_version\":{},\"supported\":{},\"target\":\"{}\"",
        report.schema_version,
        report.supported,
        json_escape(report.target)
    );
    let _ = write!(
        out,
        ",\"mechanisms\":[{}]",
        json_string_array(&report.mechanisms)
    );
    let _ = write!(
        out,
        ",\"profiles\":[{}]",
        json_string_array(&report.profiles)
    );
    let _ = write!(
        out,
        ",\"limitations\":[{}]}}",
        json_string_array(&report.limitations)
    );
    out
}

fn json_string_array(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", json_escape(s)))
        .collect::<Vec<_>>()
        .join(",")
}

/// Escape a string for inclusion in a JSON string literal.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// The top-level CLI entry point. `args` is the full `argv` including the
/// program name at `args[0]` (ignored). Returns the process exit code.
pub async fn run(args: Vec<String>) -> i32 {
    let mut iter = args.iter().skip(1);
    let subcommand = match iter.next() {
        Some(s) => s.as_str(),
        None => {
            print_usage();
            return 2;
        }
    };
    let rest: Vec<String> = iter.cloned().collect();
    match subcommand {
        "--help" | "-h" => {
            print_help();
            0
        }
        "--version" | "-V" => {
            println!("opi-sandbox {}", env!("CARGO_PKG_VERSION"));
            0
        }
        "doctor" => {
            let json = match rest.len() {
                0 => false,
                1 if rest[0] == "--json" => true,
                1 => {
                    eprintln!("opi-sandbox: unknown doctor flag `{}`", rest[0]);
                    return 2;
                }
                _ => {
                    eprintln!("opi-sandbox: doctor takes at most one flag (`--json`)");
                    return 2;
                }
            };
            doctor(json)
        }
        // The backend subcommand speaks command-execution-jsonl-v1 over stdio
        // (stdin = host->backend frames, stdout = backend->host frames). It runs
        // exactly one execution and exits 0 after the terminal frame; the
        // target's exit is in-band in `completed`. Phase 16 task 16.12.
        "backend" => {
            if rest.len() == 1 && rest[0] == "--stdio" {
                crate::backend::run().await
            } else {
                eprintln!("opi-sandbox: backend requires the `--stdio` flag");
                print_usage();
                2
            }
        }
        "run" => match parse_run(&rest) {
            Ok(cmd) => {
                // Platform gate OUTSIDE execute: refuse pre-start on an
                // unsupported platform before constructing a runner.
                let posture = platform::current();
                if !posture.supported {
                    // 16.11.2: every platform is unsupported -> pre-start refusal.
                    return 125;
                }
                let runner = SandboxRunner::new(
                    SandboxPolicy::new(cmd.profile, cmd.network),
                    posture
                        .restriction
                        .expect("a supported platform posture carries a restriction"),
                );
                let mut request = build_request(&cmd);
                let cancel = CancellationToken::new();
                request.cancel = Some(cancel.clone());
                let signal_task = tokio::spawn(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        cancel.cancel();
                    }
                });
                let mut out = std::io::stdout();
                let mut err = std::io::stderr();
                let code = execute(&runner, request, &mut out, &mut err).await;
                signal_task.abort();
                code
            }
            Err(error) => {
                eprintln!("opi-sandbox: {error}");
                2
            }
        },
        other => {
            eprintln!("opi-sandbox: unknown subcommand `{other}`");
            print_usage();
            2
        }
    }
}

fn print_usage() {
    eprintln!("usage: opi-sandbox <run|backend|doctor> [options]");
    eprintln!("       opi-sandbox --help | --version");
}

fn print_help() {
    println!("opi-sandbox {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Standalone command-execution sandbox (L0-supervised; native restriction lands in later tasks)."
    );
    println!();
    println!("usage:");
    println!("  opi-sandbox run --workspace <PATH> --profile workspace-write \\");
    println!("    --network <deny|allow> -- <PROGRAM> [ARGUMENTS...]");
    println!("  opi-sandbox backend --stdio   (command-execution-jsonl-v1 over stdio)");
    println!("  opi-sandbox doctor [--json]");
    println!("  opi-sandbox --help | --version");
}

#[cfg(all(test, not(any(target_os = "linux", target_os = "macos"))))]
mod tests {
    use super::*;

    /// On hosts without a native confinement posture (Windows; other Unix that
    /// is neither Linux nor macOS) the doctor report is unsupported with an
    /// empty mechanism list and the host OS as target (independently sourced).
    /// Supported Linux (16.13) and macOS (16.14.1) doctor contracts are
    /// asserted in `tests/linux_policy` and `tests/macos_policy`.
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn doctor_report_is_unsupported_off_native() {
        let report = doctor_report();
        assert_eq!(report.schema_version, 1);
        assert!(
            !report.supported,
            "16.11.2 doctor must report supported=false"
        );
        assert_eq!(report.target, std::env::consts::OS);
        assert!(
            report.mechanisms.is_empty(),
            "no mechanism is wired in 16.11.2"
        );
        assert!(report.profiles.contains(&"workspace-write".to_string()));
        assert!(
            !report.limitations.is_empty(),
            "doctor must carry an honest limitation"
        );
    }

    /// The hand-rolled JSON is valid for the unsupported off-native report (no
    /// mechanism/profile/limitation string breaks the fixed schema).
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn doctor_json_is_well_formed() {
        let report = doctor_report();
        let json = doctor_json(&report);
        // Re-parse via a minimal structural check (avoids a serde_json dep).
        assert!(json.starts_with('{') && json.ends_with('}'));
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"supported\":false"));
        assert!(json.contains(&format!("\"target\":\"{}\"", std::env::consts::OS)));
        assert!(json.contains("\"mechanisms\":[]"));
    }
}

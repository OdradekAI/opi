//! Portable CLI contract tests for the `opi-sandbox` human CLI (Phase 16 task
//! 16.11.2, SC16-09a).
//!
//! These tests drive the REAL [`opi_sandbox::cli`] surface (`parse_run`,
//! `build_request`, `execute`, `doctor`) with an injected
//! [`NoRestriction`](opi_sandbox::NoRestriction) runner that actually starts
//! targets, so the CLI plumbing — argument preservation, byte stdout/stderr
//! pass-through, exit-code mapping, and verbatim reserved-code handling — is
//! proven without depending on native confinement (production `run` refuses
//! pre-start off-Linux — Windows, and macOS until 16.14.1; on supported Linux
//! (16.13) it runs the target confined).
//!
//! The platform gate lives OUTSIDE [`execute`], so these tests call `execute`
//! directly and never hit the unsupported-platform refusal.

#![cfg(test)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use opi_protocol::execution::v1::EnvInherit;
use opi_sandbox::cli::{RunCommand, build_request, execute, parse_run};
use opi_sandbox::{NoRestriction, SandboxPolicy, SandboxRequest, SandboxRunner, StdinPolicy};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

/// A runner over the no-confinement restriction: targets actually start.
fn runner() -> SandboxRunner {
    SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction))
}

/// Build a request for `program`/`args` with a fresh workspace, a 10s timeout,
/// null stdin (test default), and no cancellation token.
fn request(program: PathBuf, args: Vec<String>) -> (SandboxRequest, TempDir) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let req = SandboxRequest {
        program,
        args,
        workspace: workspace.path().to_path_buf(),
        cwd: workspace.path().to_path_buf(),
        timeout: Duration::from_secs(10),
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
        stdin: StdinPolicy::Null,
        cancel: None,
    };
    (req, workspace)
}

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|i| (*i).to_string()).collect()
}

// =========================================================================
// parse_run edge matrix (Phase 16 task 16.11.2 audit fold: cli-contract)
// =========================================================================

#[test]
fn parse_run_valid_command() {
    let cmd = parse_run(&s(&[
        "--workspace",
        "/tmp/ws",
        "--profile",
        "workspace-write",
        "--network",
        "deny",
        "--",
        "/bin/echo",
        "hello",
        "world",
    ]))
    .expect("valid command parses");
    assert_eq!(cmd.workspace, PathBuf::from("/tmp/ws"));
    assert_eq!(cmd.network, opi_sandbox::NetworkPolicy::Deny);
    assert_eq!(cmd.program, PathBuf::from("/bin/echo"));
    assert_eq!(cmd.args, s(&["hello", "world"]));
}

#[test]
fn parse_run_flags_may_appear_in_any_order() {
    let cmd = parse_run(&s(&[
        "--network",
        "allow",
        "--profile",
        "workspace-write",
        "--workspace",
        "/w",
        "--",
        "p",
    ]))
    .expect("any order parses");
    assert_eq!(cmd.workspace, PathBuf::from("/w"));
    assert_eq!(cmd.network, opi_sandbox::NetworkPolicy::Allow);
    assert_eq!(cmd.program, PathBuf::from("p"));
}

#[test]
fn parse_run_missing_dd_separator_is_usage_error() {
    // A bare positional before `--` is an unknown token -> usage error (2).
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "echo"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_missing_program_after_dd_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "--"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_unknown_profile_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "read-only",
            "--network",
            "deny",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_unknown_network_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "workspace-write",
            "--network",
            "block",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_duplicate_flag_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/a",
            "--workspace",
            "/b",
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_unknown_flag_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "--timeout",
            "5",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_flag_value_shaped_like_a_flag_is_usage_error() {
    // `--workspace --profile` must NOT consume `--profile` as the workspace value.
    assert!(
        parse_run(&s(&[
            "--workspace",
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_missing_workspace_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--profile",
            "workspace-write",
            "--network",
            "deny",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_missing_profile_is_usage_error() {
    assert!(parse_run(&s(&["--workspace", "/w", "--network", "deny", "--", "p"])).is_err());
}

#[test]
fn parse_run_missing_network_is_usage_error() {
    assert!(
        parse_run(&s(&[
            "--workspace",
            "/w",
            "--profile",
            "workspace-write",
            "--",
            "p"
        ]))
        .is_err()
    );
}

#[test]
fn parse_run_dd_terminates_flag_parsing_absolutely() {
    // A `--workspace`-shaped token AFTER `--` becomes the program, not a flag.
    let cmd = parse_run(&s(&[
        "--workspace",
        "/w",
        "--profile",
        "workspace-write",
        "--network",
        "deny",
        "--",
        "--workspace",
    ]))
    .expect("tokens after -- are the program/args");
    assert_eq!(cmd.program, PathBuf::from("--workspace"));
    assert!(cmd.args.is_empty());
}

// =========================================================================
// build_request seam
// =========================================================================

/// Structural proof that the direct CLI's request carries terminal-stdin
/// inheritance (Phase 16 task 16.11.2 audit fold: stdin-sdk-seam-c1a). OS-stdin
/// byte flow is infeasible to drive in-process and production `run` refuses
/// pre-start in 16.11.2, so the honest proof is that the built request sets
/// `StdinPolicy::Inherit`.
#[test]
fn build_request_carries_terminal_stdin_inherit() {
    let cmd = RunCommand {
        workspace: PathBuf::from("/w"),
        profile: opi_sandbox::Profile::WorkspaceWrite,
        network: opi_sandbox::NetworkPolicy::Deny,
        program: PathBuf::from("/bin/echo"),
        args: s(&["hi"]),
    };
    let req = build_request(&cmd);
    assert_eq!(req.stdin, StdinPolicy::Inherit);
    assert_eq!(req.program, PathBuf::from("/bin/echo"));
    assert_eq!(req.args, s(&["hi"]));
    assert_eq!(req.cwd, PathBuf::from("/w"));
    // Non-zero timeout by construction (InvalidRequest unreachable from the CLI).
    assert!(!req.timeout.is_zero());
}

// =========================================================================
// execute exit-code mapping (injected NoRestriction runner, real targets)
// =========================================================================

#[tokio::test]
async fn execute_normal_exit_returned_verbatim() {
    for code in [0i32, 1, 42] {
        let (prog, args) = exit_program(code);
        let (req, _ws) = request(prog, args);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let rc = execute(&runner(), req, &mut out, &mut err).await;
        assert_eq!(rc, code, "target exit {code} must be returned verbatim");
    }
}

/// After target start, an exit of 2 is returned verbatim — NOT reinterpreted as
/// a CLI usage error.
#[tokio::test]
async fn execute_exit_2_returned_verbatim_not_usage() {
    let (prog, args) = exit_program(2);
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 2);
}

/// After target start, an exit of 124 is returned verbatim — NOT reinterpreted
/// as the CLI's timeout code.
#[tokio::test]
async fn execute_exit_124_returned_verbatim_not_timeout() {
    let (prog, args) = exit_program(124);
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 124);
}

/// After target start, an exit of 125 is returned verbatim — NOT reinterpreted
/// as the CLI's pre-start failure code.
#[tokio::test]
async fn execute_exit_125_returned_verbatim_not_setup_failed() {
    let (prog, args) = exit_program(125);
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 125);
}

/// After target start, an exit of 130 is returned verbatim — NOT reinterpreted
/// as the CLI's cancellation code.
#[tokio::test]
async fn execute_exit_130_returned_verbatim_not_cancelled() {
    let (prog, args) = exit_program(130);
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 130);
}

#[tokio::test]
async fn execute_timeout_returns_124() {
    let (prog, args) = sleep_program(30);
    let (mut req, _ws) = request(prog, args);
    req.timeout = Duration::from_millis(400);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 124);
}

#[tokio::test]
async fn execute_cancellation_returns_130() {
    let (prog, args) = sleep_program(30);
    let (mut req, _ws) = request(prog, args);
    let token = CancellationToken::new();
    req.cancel = Some(token.clone());
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        token.cancel();
    });
    let mut out = Vec::new();
    let mut err = Vec::new();
    let rc = execute(&runner(), req, &mut out, &mut err).await;
    canceller.abort();
    assert_eq!(rc, 130);
}

/// A present-but-nonexistent program is a pre-start failure (125), not a parse
/// error. Drives the real `runner.run() -> Err(SetupFailed{ProgramNotFound})`
/// arm of the exit mapping.
#[tokio::test]
async fn execute_program_not_found_returns_125() {
    let (mut req, _ws) = request(
        PathBuf::from("opi-sandbox-not-a-real-program-xyz"),
        Vec::new(),
    );
    req.timeout = Duration::from_secs(5);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(execute(&runner(), req, &mut out, &mut err).await, 125);
}

// =========================================================================
// execute byte pass-through (injected AsyncWrite sinks)
// =========================================================================

#[tokio::test]
async fn execute_byte_stdout_pass_through() {
    let (prog, args) = stdout_program("hello-sandbox");
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    execute(&runner(), req, &mut out, &mut err).await;
    assert!(
        out.windows(b"hello-sandbox".len())
            .any(|w| w == b"hello-sandbox"),
        "expected byte pass-through of stdout, got {:?}",
        out
    );
}

#[tokio::test]
async fn execute_byte_stderr_pass_through() {
    let (prog, args) = stderr_program("sandbox-stderr");
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    execute(&runner(), req, &mut out, &mut err).await;
    assert!(
        err.windows(b"sandbox-stderr".len())
            .any(|w| w == b"sandbox-stderr"),
        "expected byte pass-through of stderr, got {:?}",
        err
    );
}

/// Non-UTF-8 stdout is passed through byte-for-byte (no lossy conversion). The
/// implementation is platform-independent (`write_all` of the captured bytes), so
/// the Unix `printf` form proves the property; verified via WSL2/GHA on Linux
/// (Phase 16 task 16.11.2 audit fold: test-coverage-vacuity).
#[cfg(unix)]
#[tokio::test]
async fn execute_non_utf8_byte_stdout_pass_through_unix() {
    let expected = [0xff, 0xfe, 0x00, 0x80];
    let (prog, args) = raw_bytes_program(&expected);
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    execute(&runner(), req, &mut out, &mut err).await;
    assert_eq!(
        out,
        expected.to_vec(),
        "non-UTF-8 bytes must pass through exactly"
    );
}

/// Under `StdinPolicy::Null`, a target that reads stdin gets immediate EOF and
/// exits promptly (the Null-arm behavioral check; complements the Inherit
/// structural proof in `build_request_carries_terminal_stdin_inherit`).
#[tokio::test]
async fn execute_stdin_null_target_receives_eof() {
    let (prog, args) = read_stdin_program();
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    // A Null-stdin target that reads must not hang; it exits (code is irrelevant).
    let rc = tokio::time::timeout(
        Duration::from_secs(8),
        execute(&runner(), req, &mut out, &mut err),
    )
    .await
    .expect("Null-stdin read must terminate (EOF), not hang");
    // Any prompt exit code is acceptable; the invariant is "no hang".
    let _ = rc;
}

// =========================================================================
// doctor
// =========================================================================

#[test]
fn doctor_returns_zero_completed() {
    assert_eq!(opi_sandbox::cli::doctor(false), 0);
    assert_eq!(opi_sandbox::cli::doctor(true), 0);
}

// =========================================================================
// cli::run dispatcher exit mapping at the PRODUCTION CALL SITE.
// The parse_run tests above prove parse_run() returns Err (helper-level); these
// drive the top-level async cli::run(args) dispatcher and assert the exit code
// it returns, proving the dispatcher arms (Err -> 2, platform gate -> 125,
// help/version/doctor -> 0) through the entry point production uses. Closes the
// Phase D (wf_353c950f-4f1) production-call-site finding.
// =========================================================================

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|i| (*i).to_string()).collect()
}

#[tokio::test]
async fn run_dispatch_malformed_run_argv_returns_2() {
    // An unknown flag in `run` parses to Err -> the dispatcher maps it to 2.
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox", "run", "--bad-flag"])).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_missing_required_flag_returns_2() {
    // Missing --profile/--network parses to Err -> 2.
    let code = opi_sandbox::cli::run(argv(&[
        "opi-sandbox",
        "run",
        "--workspace",
        "/w",
        "--",
        "p",
    ]))
    .await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_doctor_unknown_flag_returns_2() {
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox", "doctor", "--bogus"])).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_unknown_subcommand_returns_2() {
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox", "frobnicate"])).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_no_args_returns_2() {
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox"])).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_valid_argv_runs_or_refuses_by_platform() {
    // A VALID `run` argv reaches the platform gate. On supported Linux (16.13)
    // the target runs confined (echo -> exit 0); off-Linux (Windows; macOS until
    // 16.14.1) the gate refuses pre-start -> 125 without constructing a runner.
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let code = opi_sandbox::cli::run(argv(&[
        "opi-sandbox",
        "run",
        "--workspace",
        workspace.path().to_str().expect("utf8"),
        "--profile",
        "workspace-write",
        "--network",
        "deny",
        "--",
        "echo",
    ]))
    .await;
    if cfg!(target_os = "linux") {
        assert_eq!(
            code, 0,
            "supported Linux runs the confined echo target (exit 0)"
        );
    } else {
        assert_eq!(code, 125, "off-Linux refuses pre-start (125)");
    }
}

#[tokio::test]
async fn run_dispatch_help_and_version_return_0() {
    assert_eq!(
        opi_sandbox::cli::run(argv(&["opi-sandbox", "--help"])).await,
        0
    );
    assert_eq!(
        opi_sandbox::cli::run(argv(&["opi-sandbox", "--version"])).await,
        0
    );
}

#[tokio::test]
async fn run_dispatch_backend_without_stdio_returns_2() {
    // `backend` requires the `--stdio` flag; without it -> usage error 2.
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox", "backend"])).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn run_dispatch_backend_bogus_flag_returns_2() {
    // An unknown `backend` flag -> usage error 2.
    let code = opi_sandbox::cli::run(argv(&["opi-sandbox", "backend", "--bogus"])).await;
    assert_eq!(code, 2);
}

// =========================================================================
// cfg(unix): 128+signal mapping (compiles out on the Windows host; verified via
// WSL2/GHA Linux per the Phase 16 task 16.11.2 audit fold).
// =========================================================================

/// Signal termination maps to `128 + signal`. `sh -c 'kill -TERM $$'` terminates
/// the target with SIGTERM (15), so the CLI must surface `143`.
#[cfg(unix)]
#[tokio::test]
async fn execute_signal_termination_maps_to_128_plus_signal_unix_only() {
    let (prog, args) = signal_self_program();
    let (req, _ws) = request(prog, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        execute(&runner(), req, &mut out, &mut err).await,
        128 + libc_signal::SIGTERM
    );
}

// =========================================================================
// cross-platform target program builders
// =========================================================================

#[cfg(unix)]
fn exit_program(code: i32) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".into(), format!("exit {code}")],
    )
}
#[cfg(windows)]
fn exit_program(code: i32) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("cmd"),
        vec!["/C".into(), format!("exit {code}")],
    )
}

#[cfg(unix)]
fn sleep_program(seconds: u64) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".into(), format!("sleep {seconds}")],
    )
}
#[cfg(windows)]
fn sleep_program(seconds: u64) -> (PathBuf, Vec<String>) {
    // `ping` waits ~N-1s without needing a console (unlike `timeout`).
    (
        PathBuf::from("cmd"),
        vec![
            "/C".into(),
            format!("ping -n {} 127.0.0.1 >NUL", seconds + 1),
        ],
    )
}

#[cfg(unix)]
fn stdout_program(text: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".into(), format!("printf '%s' '{text}'")],
    )
}
#[cfg(windows)]
fn stdout_program(text: &str) -> (PathBuf, Vec<String>) {
    // `echo` adds a trailing newline; the caller matches with a substring.
    (
        PathBuf::from("cmd"),
        vec!["/C".into(), format!("echo {text}")],
    )
}

#[cfg(unix)]
fn stderr_program(text: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".into(), format!("printf '%s' '{text}' >&2")],
    )
}
#[cfg(windows)]
fn stderr_program(text: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("cmd"),
        vec!["/C".into(), format!("echo {text}>&2")],
    )
}

#[cfg(unix)]
fn read_stdin_program() -> (PathBuf, Vec<String>) {
    // `read` blocks for a line; under Null stdin it gets EOF and returns at once.
    (PathBuf::from("sh"), vec!["-c".into(), "read x".to_string()])
}
#[cfg(windows)]
fn read_stdin_program() -> (PathBuf, Vec<String>) {
    // `set /p=` reads a line from stdin; under <NUL it returns immediately.
    (
        PathBuf::from("cmd"),
        vec!["/C".into(), "set /p=".to_string()],
    )
}

#[cfg(unix)]
fn raw_bytes_program(bytes: &[u8]) -> (PathBuf, Vec<String>) {
    let escaped: String = bytes.iter().map(|b| format!("\\{b:o}")).collect();
    (
        PathBuf::from("sh"),
        vec!["-c".into(), format!("printf '{escaped}'")],
    )
}

#[cfg(unix)]
fn signal_self_program() -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".into(), "kill -TERM $$".to_string()],
    )
}

// A tiny indirection so the unix signal test does not depend on the `libc` crate
// directly (opi-sandbox depends on it only as a target.'cfg(unix)' normal dep,
// which is not visible to dev-dependencies).
#[cfg(unix)]
mod libc_signal {
    pub const SIGTERM: i32 = 15;
}

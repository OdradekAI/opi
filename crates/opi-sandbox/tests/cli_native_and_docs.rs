//! Regression coverage for native argv fidelity and public CLI/error wording.

#![cfg(test)]

use std::ffi::OsString;
use std::process::Command;
use std::sync::Arc;

use opi_sandbox::cli::{build_request, execute, parse_run};
use opi_sandbox::process_tree::TreeGuard;
use opi_sandbox::{NoRestriction, SandboxPolicy, SandboxRunner};

#[cfg(target_os = "linux")]
#[test]
fn real_cli_preserves_non_utf8_workspace_and_target_argument() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let root = tempfile::tempdir().expect("temp root");
    let workspace = root
        .path()
        .join(OsString::from_vec(b"workspace-\xff".to_vec()));
    std::fs::create_dir(&workspace).expect("create non-UTF-8 workspace");

    let target_argument = OsString::from_vec(b"argument-\xfe-tail".to_vec());
    let output = Command::new(env!("CARGO_BIN_EXE_opi-sandbox"))
        .arg("run")
        .arg("--workspace")
        .arg(&workspace)
        .arg("--profile")
        .arg("workspace-write")
        .arg("--network")
        .arg("deny")
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg("pwd; printf '%s' \"$1\"")
        .arg("opi-native-argv")
        .arg(&target_argument)
        .output()
        .expect("run opi-sandbox with native argv");

    assert!(
        output.status.success(),
        "native argv run failed or panicked: status={:?}, stderr={:?}",
        output.status,
        output.stderr
    );
    let mut expected = workspace.as_os_str().as_bytes().to_vec();
    expected.push(b'\n');
    expected.extend(target_argument.into_vec());
    assert_eq!(output.stdout, expected);
}

#[cfg(unix)]
#[test]
fn parser_and_request_preserve_non_utf8_unix_paths_and_arguments() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let workspace = OsString::from_vec(b"/workspace-\xff".to_vec());
    let target_argument = OsString::from_vec(b"argument-\xfe".to_vec());
    let command = parse_run(&[
        OsString::from("--workspace"),
        workspace.clone(),
        OsString::from("--profile"),
        OsString::from("workspace-write"),
        OsString::from("--network"),
        OsString::from("deny"),
        OsString::from("--"),
        OsString::from("program"),
        target_argument.clone(),
    ])
    .expect("native Unix argv parses");
    let request = build_request(&command);

    assert_eq!(
        request.workspace.as_os_str().as_bytes(),
        workspace.as_bytes()
    );
    assert_eq!(request.args[0].as_bytes(), target_argument.as_bytes());
}

#[cfg(windows)]
#[test]
fn parser_and_request_preserve_unpaired_windows_wide_units() {
    use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};

    let workspace = OsString::from_wide(&[b'C' as u16, b':' as u16, b'\\' as u16, 0xd800]);
    let target_argument = OsString::from_wide(&[b'a' as u16, 0xdc00, b'z' as u16]);
    let argv = vec![
        OsString::from("--workspace"),
        workspace.clone(),
        OsString::from("--profile"),
        OsString::from("workspace-write"),
        OsString::from("--network"),
        OsString::from("deny"),
        OsString::from("--"),
        OsString::from("cmd"),
        target_argument.clone(),
    ];

    let command = parse_run(&argv).expect("native Windows argv parses");
    assert_eq!(
        command
            .workspace
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>(),
        workspace.encode_wide().collect::<Vec<_>>()
    );
    assert_eq!(
        command.args[0].encode_wide().collect::<Vec<_>>(),
        target_argument.encode_wide().collect::<Vec<_>>()
    );

    let request = build_request(&command);
    assert_eq!(
        request
            .workspace
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>(),
        workspace.encode_wide().collect::<Vec<_>>()
    );
    assert_eq!(
        request.args[0].encode_wide().collect::<Vec<_>>(),
        target_argument.encode_wide().collect::<Vec<_>>()
    );
}

#[test]
fn run_parse_errors_are_redacted() {
    let secret = "do-not-echo-this-option-value";
    let error = parse_run(&[
        OsString::from("--workspace"),
        OsString::from("workspace"),
        OsString::from("--profile"),
        OsString::from(secret),
        OsString::from("--network"),
        OsString::from("deny"),
        OsString::from("--"),
        OsString::from("program"),
    ])
    .expect_err("unknown profile is rejected");

    assert_eq!(error.to_string(), "invalid value for flag `--profile`");
    assert!(!error.to_string().contains(secret));
}

#[test]
fn public_error_display_messages_remain_exact_and_redacted() {
    fn assert_error<T: std::error::Error>() {}

    assert_error::<opi_sandbox::cli::UsageError>();
    assert_error::<opi_sandbox::process_tree::AttachError>();

    let usage = parse_run(&[OsString::from("--workspace")])
        .expect_err("missing workspace value is rejected");
    assert_eq!(usage.to_string(), "missing value for flag `--workspace`");

    let attach = TreeGuard::attach(0).expect_err("PID zero is rejected");
    let layer = if cfg!(windows) {
        "windows-job"
    } else if cfg!(unix) {
        "unix-pgroup"
    } else {
        "unsupported"
    };
    assert_eq!(
        attach.to_string(),
        format!("L0 attach failed ({layer}): MissingChildProcessId")
    );
}

#[tokio::test]
async fn nonexistent_workspace_and_derived_cwd_map_to_exit_2() {
    let root = tempfile::tempdir().expect("temp root");
    let missing = root.path().join("does-not-exist");
    let command = parse_run(&[
        OsString::from("--workspace"),
        missing.as_os_str().to_os_string(),
        OsString::from("--profile"),
        OsString::from("workspace-write"),
        OsString::from("--network"),
        OsString::from("deny"),
        OsString::from("--"),
        OsString::from("unused-program"),
    ])
    .expect("native command parses before filesystem validation");
    let request = build_request(&command);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    assert_eq!(execute(&runner, request, &mut stdout, &mut stderr).await, 2);
}

#[cfg(windows)]
#[tokio::test]
async fn unsupported_platform_refusal_wins_before_workspace_validation() {
    let root = tempfile::tempdir().expect("temp root");
    let missing = root.path().join("does-not-exist");
    let code = opi_sandbox::cli::run(vec![
        OsString::from("opi-sandbox"),
        OsString::from("run"),
        OsString::from("--workspace"),
        missing.into_os_string(),
        OsString::from("--profile"),
        OsString::from("workspace-write"),
        OsString::from("--network"),
        OsString::from("deny"),
        OsString::from("--"),
        OsString::from("unused-program"),
    ])
    .await;

    assert_eq!(code, 125);
}

#[test]
fn help_exposes_the_stable_run_backend_and_doctor_grammar() {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-sandbox"))
        .arg("--help")
        .output()
        .expect("run opi-sandbox --help");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
    for token in [
        "opi-sandbox run --workspace <PATH>",
        "--profile workspace-write",
        "--network <deny|allow>",
        "opi-sandbox backend --stdio",
        "opi-sandbox doctor [--json]",
    ] {
        assert!(
            stdout.contains(token),
            "help is missing grammar token {token:?}"
        );
    }
}

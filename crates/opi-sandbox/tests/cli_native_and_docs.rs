//! Regression coverage for native argv fidelity and public CLI/error wording.

#![cfg(test)]

use std::ffi::OsString;
use std::process::Command;
use std::sync::Arc;

use opi_sandbox::cli::{build_request, execute, parse_run};
use opi_sandbox::process_tree::TreeGuard;
use opi_sandbox::{NoRestriction, SandboxPolicy, SandboxRunner};

fn source_section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing source section start: {start}"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("missing source section end: {end}"))
        .0
}

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
fn help_describes_shipped_native_platform_posture() {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-sandbox"))
        .arg("--help")
        .output()
        .expect("run opi-sandbox --help");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
    assert_eq!(
        stdout.lines().nth(1),
        Some(
            "Standalone command-execution sandbox (native restriction on supported Linux/macOS; Windows run is refused)."
        )
    );
    assert!(!stdout.contains("lands in later tasks"));
}

#[test]
fn source_docs_pin_exit_mapping_and_platform_posture() {
    let main = include_str!("../src/main.rs");
    assert!(main.contains("std::env::args_os()"));

    let cli = include_str!("../src/cli.rs");
    assert!(cli.contains(
        "| nonexistent `--workspace` / derived cwd after the supported-platform posture gate (`InvalidRequest`) | `2` |"
    ));
    assert!(
        cli.contains("| unsupported platform (wins before workspace/cwd validation) | `125` |")
    );
    assert!(!cli.contains("| nonexistent `--workspace` / derived cwd (`InvalidRequest`) | `2` |"));
    assert!(!cli.contains("unreachable from the human CLI"));
    assert!(!cli.contains("16.11.2: every platform is unsupported"));
    assert!(!cli.contains("empty in 16.11.2"));
    assert!(
        cli.contains(
            "The mechanism names installed by a supported posture; empty when unsupported."
        )
    );

    let platform = include_str!("../src/platform/mod.rs");
    assert!(
        platform
            .contains("Linux and macOS install native restrictions when available; Windows has L0")
    );
    assert!(!platform.contains("macOS remains"));
    assert!(!platform.contains("\"not yet wired in this build\" (macOS, temporary)"));
    assert!(!platform.contains("runs are unrestricted under L0 supervision only"));
    assert!(platform.contains(
        "native confinement is not supported on this platform; the human CLI refuses before target start"
    ));
}

#[test]
fn source_docs_pin_leaf_refusal_and_current_restriction_mechanisms() {
    let linux = include_str!("../src/platform/linux.rs");
    let linux_limitations = source_section(linux, "fn limitations(", "\n#[cfg(test)]");
    assert!(linux_limitations.contains(
        "Landlock is absent or disabled on this kernel; the requested restriction cannot be established, so the target is refused before start"
    ));
    assert!(linux_limitations.contains(
        "the host seccomp architecture is unsupported; the requested restriction cannot be established, so the target is refused before start"
    ));
    assert!(!linux_limitations.contains("runs are unrestricted"));

    let macos = include_str!("../src/platform/macos.rs");
    let unsupported_macos = source_section(
        macos,
        "fn unsupported_limitation(",
        "\n/// The pure, host-independent fields",
    );
    assert!(unsupported_macos.contains(
        "canonical /usr/bin/sandbox-exec is missing; the requested restriction cannot be established, so the target is refused before start"
    ));
    assert!(unsupported_macos.contains(
        "canonical /usr/bin/sandbox-exec did not pass the runtime probe; the requested restriction cannot be established, so the target is refused before start"
    ));
    assert!(!unsupported_macos.contains("runs are unrestricted"));
    assert!(!unsupported_macos.contains("PATH"));
    assert!(!macos.contains("on `PATH`"));
    assert!(!macos.contains("on PATH"));

    let macos_probe = source_section(
        macos,
        "const SANDBOX_EXEC_PATH: &str",
        "\n/// The macOS native",
    );
    assert!(macos_probe.starts_with(" = \"/usr/bin/sandbox-exec\";"));
    assert!(macos_probe.contains("std::process::Command::new(&bin)"));
    assert!(!macos_probe.contains("Command::new(\"sandbox-exec\")"));

    let helper = include_str!("../src/helper.rs");
    let helper_header = source_section(helper, "//! The atomic helper", "\n#![forbid")
        .replace("\r\n", "\n")
        .replace("\n//! ", " ");
    assert!(
        helper_header.contains(
            "Supported Linux and macOS postures install their current native restrictions"
        )
    );
    assert!(!helper.contains("lands in 16.13"));
    assert!(!helper.contains("lands in 16.14.1"));
    assert!(!helper.contains("is owned by 16.13"));

    let policy = include_str!("../src/policy.rs");
    let policy_header = source_section(policy, "//! Sandbox policy", "\n#![forbid")
        .replace("\r\n", "\n")
        .replace("\n//! ", " ");
    assert!(policy_header.contains(
        "The shipped Linux and macOS native implementations provide the confinement mechanisms"
    ));
    assert!(!policy.contains("lands in task 16.13"));
    assert!(!policy.contains("lands in 16.14.1"));
}

//! Behavioral tests for shell completions (task 3.10).
//!
//! Validates that `opi --generate-completion <shell>` emits non-empty
//! clap_complete output to stdout and exits with code 0 for each supported
//! shell.

use std::process::Command;

fn opi_bin() -> &'static str {
    env!("CARGO_BIN_EXE_opi")
}

fn assert_completion(shell: &str) {
    let bin = opi_bin();
    let output = Command::new(bin)
        .arg("--generate-completion")
        .arg(shell)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {} --generate-completion {}: {e}", bin, shell));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "{shell}: expected exit 0, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );
    assert!(
        !stdout.trim().is_empty(),
        "{shell}: completion output should not be empty"
    );
}

#[test]
fn generates_bash_completions() {
    assert_completion("bash");
}

#[test]
fn generates_zsh_completions() {
    assert_completion("zsh");
}

#[test]
fn generates_fish_completions() {
    assert_completion("fish");
}

#[test]
fn generates_powershell_completions() {
    assert_completion("powershell");
}

#[test]
fn generates_elvish_completions() {
    assert_completion("elvish");
}

#[test]
fn generate_completion_rejects_unknown_shell() {
    let bin = opi_bin();
    let output = Command::new(bin)
        .arg("--generate-completion")
        .arg("foobar")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "unknown shell should fail: {:?}",
        output.status.code()
    );
}

#[test]
fn generate_completion_rejects_missing_shell() {
    let bin = opi_bin();
    let output = Command::new(bin)
        .arg("--generate-completion")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "missing shell arg should fail: {:?}",
        output.status.code()
    );
}

#[test]
fn completion_output_is_plausible_bash() {
    let bin = opi_bin();
    let output = Command::new(bin)
        .arg("--generate-completion")
        .arg("bash")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Bash completion scripts should contain function definitions or the
    // program name.
    assert!(
        stdout.contains("opi") && (stdout.contains("_opi") || stdout.contains("complete")),
        "bash completion should reference 'opi' and contain completion constructs:\n{stdout}"
    );
}

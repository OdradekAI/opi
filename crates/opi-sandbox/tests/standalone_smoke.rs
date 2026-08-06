//! Standalone smoke: drive the platform smoke script against the isolated built
//! binary, then read back the persisted artifacts and re-assert (Phase 16 task
//! 16.11.2, SC16-09a).
//!
//! The script (not this test) owns the isolation — PATH scrub, Opi sentinel env,
//! and the canary — so CI/release jobs that invoke the script directly get the
//! same no-Opi-access / no-durable-state proof this test asserts. This test runs
//! `scripts/opi-sandbox-smoke.sh` on unix and `scripts/opi-sandbox-smoke.ps1` on
//! Windows (the cfg(unix) arm compiles out on a Windows host; verified via
//! WSL2/GHA Linux per the Phase 16 task 16.11.2 audit fold).

#![cfg(test)]

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `crates/opi-sandbox` is two levels below the repo root.
fn repo_root() -> PathBuf {
    manifest_dir()
        .ancestors()
        .nth(2)
        .expect("manifest is under crates/opi-sandbox")
        .to_path_buf()
}

fn read_artifact(dir: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap_or_else(|_| String::new())
}

/// Re-assert the persisted artifacts so a script that touched the files but
/// wrote empty or stale content still fails the test. The supported posture is
/// OS-dependent: on supported Linux (16.13) the binary reports supported=true +
/// landlock/seccomp and `run` succeeds; on supported macOS (16.14.1) it reports
/// supported=true + seatbelt and `run` succeeds; off-native (Windows, other
/// Unix) it stays unsupported and `run` refuses pre-start (125).
fn assert_artifacts(dir: &std::path::Path) {
    let version = read_artifact(dir, "version.txt");
    assert!(
        version.contains("opi-sandbox"),
        "version artifact missing opi-sandbox: {version:?}"
    );
    let help = read_artifact(dir, "help.txt");
    assert!(
        help.contains("run") && help.contains("doctor"),
        "help artifact: {help:?}"
    );
    let doctor = read_artifact(dir, "doctor.json");
    let run_exit = read_artifact(dir, "run-exit.txt");
    if cfg!(target_os = "linux") {
        assert!(
            doctor.contains("\"supported\":true"),
            "Linux doctor must report supported=true: {doctor:?}"
        );
        assert!(
            doctor.contains("\"landlock\"") && doctor.contains("\"seccomp\""),
            "Linux doctor must list landlock + seccomp: {doctor:?}"
        );
        assert_eq!(
            run_exit.trim(),
            "0",
            "run succeeds (exit 0) on supported Linux: {run_exit:?}"
        );
        assert_complete_native_markers(dir);
    } else if cfg!(target_os = "macos") {
        assert!(
            doctor.contains("\"supported\":true"),
            "macOS doctor must report supported=true: {doctor:?}"
        );
        assert!(
            doctor.contains("\"seatbelt\""),
            "macOS doctor must list seatbelt: {doctor:?}"
        );
        assert_eq!(
            run_exit.trim(),
            "0",
            "run succeeds (exit 0) on supported macOS: {run_exit:?}"
        );
        assert_complete_native_markers(dir);
    } else {
        assert!(
            doctor.contains("\"supported\":false"),
            "off-native doctor must report supported=false: {doctor:?}"
        );
        assert!(
            doctor.contains("\"mechanisms\":[]"),
            "off-native doctor mechanisms must be empty: {doctor:?}"
        );
        assert_eq!(
            run_exit.trim(),
            "125",
            "run must refuse pre-start (125) off-native: {run_exit:?}"
        );
    }
}

fn assert_complete_native_markers(dir: &std::path::Path) {
    for (file, marker) in [
        (
            "empty-cwd-smoke-result.txt",
            "opi-sandbox-empty-cwd-smoke: OK",
        ),
        (
            "setup-failure-smoke-result.txt",
            "opi-sandbox-setup-failure-smoke: OK",
        ),
        (
            "filesystem-allow-smoke-result.txt",
            "opi-sandbox-filesystem-allow-smoke: OK",
        ),
        (
            "filesystem-deny-smoke-result.txt",
            "opi-sandbox-filesystem-deny-smoke: OK",
        ),
        (
            "network-deny-smoke-result.txt",
            "opi-sandbox-network-deny-smoke: OK",
        ),
        (
            "network-allow-smoke-result.txt",
            "opi-sandbox-network-allow-smoke: OK",
        ),
    ] {
        let evidence = read_artifact(dir, file);
        assert!(
            evidence.contains(marker),
            "missing named native marker {marker} in {file}: {evidence:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn standalone_smoke_script_unix() {
    let bin = env!("CARGO_BIN_EXE_opi-sandbox");
    let artifact_dir = tempfile::tempdir().expect("artifact temp dir");
    let status = Command::new("bash")
        .arg(repo_root().join("scripts").join("opi-sandbox-smoke.sh"))
        .args(["--binary", bin])
        .args([
            "--artifact-dir",
            artifact_dir.path().to_str().expect("utf8 artifact dir"),
        ])
        .status()
        .expect("run bash smoke script");
    assert!(status.success(), "smoke script failed: {status}");
    assert_artifacts(artifact_dir.path());
}

#[cfg(windows)]
#[test]
fn standalone_smoke_script_windows() {
    let bin = env!("CARGO_BIN_EXE_opi-sandbox");
    let artifact_dir = tempfile::tempdir().expect("artifact temp dir");
    let artifact_str = artifact_dir.path().to_str().expect("utf8 artifact dir");
    let script = repo_root()
        .join("scripts")
        .join("opi-sandbox-smoke.ps1")
        .to_str()
        .expect("utf8 script path")
        .to_string();
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &script])
        .args(["-BinaryPath", bin])
        .args(["-ArtifactDir", artifact_str])
        .status()
        .expect("run powershell smoke script");
    assert!(status.success(), "smoke script failed: {status}");
    assert_artifacts(artifact_dir.path());
}

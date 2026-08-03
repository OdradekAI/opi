use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crate lives under crates/opi-coding-agent")
        .to_path_buf()
}

fn python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn run_audit_with_args(
    dir: &std::path::Path,
    workspace: &std::path::Path,
    json: bool,
) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--workspace-root")
        .arg(workspace)
        .args(json.then_some("--json"))
        .output()
        .expect("run artifact audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_audit(dir: &std::path::Path) -> (bool, String, String) {
    run_audit_with_args(dir, dir, false)
}

#[test]
fn artifact_audit_fails_on_workspace_root_leak_and_passes_when_removed() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let session_dir = dir.path().join("sessions");
    std::fs::create_dir(&session_dir).unwrap();
    // dir.path() on Windows uses backslashes — exercises the normalization path.
    // Escape them so the embedded JSON is valid on Windows: raw `\U`/`\L` are
    // invalid JSON escapes, and the session-header `cwd` must parse so it can be
    // recognized and skipped by the checker.
    let leaked_root = dir.path().display().to_string().replace('\\', "\\\\");

    // NDJSON: a tool-result-style line that embeds the absolute root.
    std::fs::write(
        dir.path().join("run.ndjson"),
        format!(
            "{{\"type\":\"Agent\",\"event\":{{\"type\":\"MessageUpdate\",\"message\":{{\"timestamp_ms\":1,\"content\":[{{\"type\":\"text\",\"text\":\"{leaked_root}/file.txt\"}}]}},\"assistant_event\":{{\"type\":\"text_delta\",\"delta\":\"x\"}}}}}}\n{{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}}}\n"
        ),
    )
    .unwrap();
    // Session JSONL: a session header (by-design cwd, must be skipped) + a leaking message.
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!(
            "{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"{leaked_root}/file.txt\"}}}}\n"
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        !ok,
        "audit must fail on leak: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("workspace_root_leak") || stdout.contains("session_workspace_root_leak"),
        "expected a leak finding, got: {stdout}"
    );

    // Remove the leaks (keep the session header, which is allowed).
    std::fs::write(
        dir.path().join("run.ndjson"),
        "{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":1,\"content\":[{\"type\":\"text\",\"text\":\"file.txt\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\"}}}\n{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n",
    )
    .unwrap();
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!("{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"file.txt\"}}}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        ok,
        "audit must pass after leak removal: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn artifact_audit_detects_zero_timestamps_turn_mismatch_and_duplicate_partials() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    // 60 text_delta lines, all carrying "partial" -> duplicated_text_delta_partials.
    let mut ndjson = String::new();
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    for _ in 0..59 {
        ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    }
    // 2 TurnStart events but provider_turns=5 -> mismatch.
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":5,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n");
    std::fs::write(dir.path().join("run.ndjson"), ndjson).unwrap();

    let (ok, stdout, _stderr) = run_audit(dir.path());
    assert!(!ok, "audit must fail on the synthetic defects");
    assert!(
        stdout.contains("all_zero_timestamps"),
        "missing zero-timestamp finding: {stdout}"
    );
    assert!(
        stdout.contains("duplicated_text_delta_partials"),
        "missing partial-duplication finding: {stdout}"
    );
    assert!(
        stdout.contains("provider_turn_mismatch"),
        "missing provider-turn mismatch finding: {stdout}"
    );
}

#[test]
fn artifact_audit_rejects_every_missing_declared_commit_reference() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let missing_summary_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4a";
    let missing_metadata_commit = "ffffffffffffffffffffffffffffffffffffffff";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {missing_summary_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{missing_metadata_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        !ok,
        "audit must reject missing commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on failure");
    let issues = report["issues"]
        .as_array()
        .expect("artifact audit issues are an array");
    let missing = issues
        .iter()
        .filter(|issue| issue["code"] == "missing_commit_reference")
        .collect::<Vec<_>>();
    assert_eq!(
        missing.len(),
        2,
        "every declared missing commit must be reported: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_summary_commit),
        "summary commit typo must be attributable: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_metadata_commit),
        "metadata commit must be attributable: {stdout}"
    );
}

#[test]
fn artifact_audit_accepts_real_declared_commit_objects() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let real_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4b";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {real_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{real_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        ok,
        "audit must accept real commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on success");
    assert_eq!(
        report["commit_references"].as_array().map(Vec::len),
        Some(2)
    );
}

// ============================================================================
// Phase 16 task 16.15.2: release-archive audit mode (`--release`).
//
// The release audit validates the published native opi-sandbox topology per
// SC16-12b: native target identity, archive layout, extracted-binary
// provenance, direct/backend smoke evidence, and complete non-skipped /
// non-zero-test Linux/macOS/Windows evidence. It rejects absent, wrong-target,
// workspace-only, skipped, or zero-test evidence. These tests drive the audit
// script on synthetic per-platform evidence bundles (good + each defect class).
// ============================================================================

use sha2::{Digest, Sha256};

fn sha256_hex_local(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Run the artifact audit in RELEASE mode on `dir`.
fn run_release_audit(dir: &std::path::Path, json: bool) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--release")
        .args(json.then_some("--json"))
        .output()
        .expect("run release audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";
const MACOS_TARGET: &str = "aarch64-apple-darwin";
const BINARY_BYTES: &[u8] = b"opi-sandbox extracted release binary payload\n";

fn good_smoke_log() -> &'static str {
    // Real smoke-script marker (opi-sandbox-smoke.sh writes smoke-result.txt).
    // Direct smoke only; backend --stdio is deferred per the script header.
    "opi-sandbox-smoke: OK\n"
}

fn good_windows_log() -> &'static str {
    // Windows unsupported-posture evidence: doctor reports supported=false, and
    // the unsupported-posture cargo tests pass (non-skipped, non-zero-test).
    "doctor: {\"supported\":false,\"mechanisms\":[]}\n\
     run: refused pre-start (exit 125)\n\
     test result: ok. 3 passed; 0 failed; 0 ignored\n"
}

/// Write a native (linux/macos) evidence bundle under `<root>/<platform>/`.
/// `binary_bytes` is the extracted opi-sandbox binary; its sha256 is written
/// into the lock so provenance passes by default. `mismatch_sha` corrupts the
/// locked sha for the provenance-mismatch case; `omit_extracted` drops the
/// extracted tree (workspace-only binary).
fn write_native_bundle(
    root: &std::path::Path,
    platform: &str,
    target: &str,
    binary_bytes: &[u8],
    smoke_log: &str,
    mismatch_sha: bool,
    omit_extracted: bool,
) {
    let dir = root.join(platform);
    std::fs::create_dir_all(dir.join("extracted").join("bin")).unwrap();
    std::fs::write(dir.join("target"), target).unwrap();
    let exe_sha = if mismatch_sha {
        "0".repeat(64)
    } else {
        sha256_hex_local(binary_bytes)
    };
    std::fs::write(
        dir.join("package-lock.toml"),
        format!(
            "manifest_hash = \"abc123\"\n\
             executable_rel_path = \"bin/opi-sandbox\"\n\
             executable_sha256 = \"{exe_sha}\"\n\
             package_version = \"0.8.0\"\n\
             target = \"{target}\"\n\
             opi_range = \">=0.8,<0.9\"\n\
             protocol = \"command-execution-jsonl-v1\"\n\
             adapter_id = \"opi-sandbox\"\n"
        ),
    )
    .unwrap();
    if !omit_extracted {
        std::fs::write(
            dir.join("extracted").join("bin").join("opi-sandbox"),
            binary_bytes,
        )
        .unwrap();
        std::fs::write(
            dir.join("extracted").join("package.toml"),
            "# rendered manifest\n",
        )
        .unwrap();
    }
    std::fs::write(dir.join("smoke.log"), smoke_log).unwrap();
}

fn write_windows_bundle(root: &std::path::Path, log: &str, with_archive: bool) {
    let dir = root.join("windows");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("unsupported.log"), log).unwrap();
    if with_archive {
        // A Windows opi-sandbox archive must NOT exist (16.14.2 unsupported).
        std::fs::create_dir_all(dir.join("extracted").join("bin")).unwrap();
        std::fs::write(
            dir.join("extracted").join("bin").join("opi-sandbox"),
            b"must not exist",
        )
        .unwrap();
    }
}

/// A complete, correct evidence tree: native linux + macos bundles + a windows
/// unsupported-posture bundle. Audit must PASS.
fn write_complete_good_evidence(root: &std::path::Path) {
    write_native_bundle(
        root,
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(root, good_windows_log(), false);
}

#[test]
fn release_audit_passes_complete_native_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        ok,
        "complete native evidence must pass: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn release_audit_rejects_missing_platform() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // Omit macos entirely.
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "missing platform must fail: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("release audit emits JSON on failure");
    assert!(
        report["issues"]
            .as_array()
            .map(|v| v.iter().any(|i| i["code"] == "missing_platform_evidence"))
            .unwrap_or(false),
        "expected missing_platform_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_wrong_target_identity() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux bundle carrying a darwin target triple -> wrong target identity.
    write_native_bundle(
        dir.path(),
        "linux",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "wrong target must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "expected wrong_target_identity: {stdout}"
    );
}

#[test]
fn release_audit_rejects_windows_opi_sandbox_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    // Add a Windows opi-sandbox archive (forbidden: no Windows artifact).
    write_windows_bundle(dir.path(), good_windows_log(), true);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "a Windows opi-sandbox archive must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "Windows archive is a wrong-target defect: {stdout}"
    );
}

#[test]
fn release_audit_rejects_workspace_only_binary() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux bundle with no extracted tree (smoke ran against a workspace
    // target/ binary, not an extracted archive).
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        true,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "workspace-only binary must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("workspace_only_binary"),
        "expected workspace_only_binary: {stdout}"
    );
}

#[test]
fn release_audit_rejects_provenance_mismatch() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux extracted binary sha != locked executable_sha256.
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        true,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "provenance mismatch must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("provenance_mismatch"),
        "expected provenance_mismatch: {stdout}"
    );
}

#[test]
fn release_audit_rejects_skipped_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // smoke evidence shows ignored tests (skipped evidence).
    let skipped = "test result: ok. 8 passed; 0 failed; 2 ignored\n";
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        skipped,
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "skipped evidence must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("skipped_evidence"),
        "expected skipped_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_zero_test_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // smoke evidence shows 0 passed (zero-test evidence).
    let zero = "test result: ok. 0 passed; 0 failed; 0 ignored\n";
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        zero,
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "zero-test evidence must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "expected zero_test_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_windows_unsupported_without_pass_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // Windows unsupported log with no passing test evidence (zero-test).
    let no_pass = "doctor: {\"supported\":false,\"mechanisms\":[]}\nrun: refused pre-start\n";
    write_windows_bundle(dir.path(), no_pass, false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "windows evidence without a pass must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "windows zero-test evidence must be flagged: {stdout}"
    );
}

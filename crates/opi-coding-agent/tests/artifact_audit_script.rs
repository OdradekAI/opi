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

// ============================================================================
// Phase 16 task 16.16.3: phase-exit evidence mode (`--phase-exit`).
//
// The phase-exit audit validates the preserved Phase 16 phase-exit evidence
// (SC16-15b / the 16.16.3 smoke addendum) against the claimed categories and
// rejects absent, skipped, zero-test, wrong-target, and workspace-only
// evidence. Unlike `--release` it accepts CI-sourced native evidence (a
// preserved log with a genuine pass marker + a `source` provenance note) when
// the extracted archive itself is CI-produced and not preservable off-CI. These
// tests drive the audit on synthetic evidence trees (good + each defect class).
// ============================================================================

/// Run the artifact audit in PHASE-EXIT mode on `dir`.
fn run_phase_exit_audit(dir: &std::path::Path) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--phase-exit")
        .output()
        .expect("run phase-exit audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Write a macos CI-sourced bundle (no local archive): a preserved log plus a
/// `source` provenance note. `with_pass` controls whether the log carries a
/// genuine pass marker; `with_source` controls the provenance note.
fn write_macos_ci_bundle(root: &std::path::Path, with_pass: bool, with_source: bool) {
    let dir = root.join("macos");
    std::fs::create_dir_all(&dir).unwrap();
    let log = if with_pass {
        "test result: ok. 10 passed; 0 failed; 0 ignored\n"
    } else {
        "cargo check --target aarch64-apple-darwin\n" // no pass marker
    };
    std::fs::write(dir.join("native.log"), log).unwrap();
    if with_source {
        std::fs::write(
            dir.join("source"),
            "run 123 @deadbeef (sandbox-macos-phase16): native tests pass\n",
        )
        .unwrap();
    }
}

/// Write the six-target bundle: one preserved `cargo check --target` log per
/// triple. `green` triples carry a `Finished` line; `failed` triples carry an
/// `error[` line; `ambiguous` triples carry neither; triples absent from the
/// map are omitted entirely.
fn write_six_target_bundle(
    root: &std::path::Path,
    triples: &[(&str, &str)], // (triple, "green" | "failed" | "ambiguous")
) {
    let dir = root.join("six-target");
    std::fs::create_dir_all(&dir).unwrap();
    // The phase-exit audit requires a provenance note for the preserved logs.
    std::fs::write(
        dir.join("source"),
        "ci run 123 @deadbeef target_check job\n",
    )
    .unwrap();
    for (index, (triple, kind)) in triples.iter().enumerate() {
        let body = match *kind {
            "green" => format!("cargo check --target {triple}\nFinished dev profile\n"),
            "failed" => format!("cargo check --target {triple}\nerror[E0]: boom\n"),
            _ => format!("cargo check --target {triple}\n"),
        };
        std::fs::write(dir.join(format!("check-{index}.log")), body).unwrap();
    }
}

/// The DoD gate categories the phase-exit audit requires a capture for, keyed
/// by the filename marker (mirrors GATE_CATEGORIES in opi-artifact-audit.py).
const GATE_CATEGORY_MARKERS: &[&str] = &[
    "doc-guards",
    "crate-boundary",
    "packaging",
    "release-topology",
    "workspace-test",
    "doctest",
    "fmt",
    "clippy",
    "rustdoc",
];

/// Write one pass-marked (or marker-free, when `with_pass` is false) capture per
/// DoD gate category under `gates/`.
fn write_gates_bundle(root: &std::path::Path, with_pass: bool) {
    let dir = root.join("gates");
    std::fs::create_dir_all(&dir).unwrap();
    for marker in GATE_CATEGORY_MARKERS {
        let body = if with_pass {
            "test result: ok. 3 passed; 0 failed; 0 ignored\n"
        } else {
            "some gate ran\n" // no pass marker
        };
        std::fs::write(dir.join(format!("gate-{marker}.txt")), body).unwrap();
    }
}

/// A complete, correct phase-exit evidence tree: linux archive bundle + macos
/// CI-sourced bundle + windows unsupported bundle + six green target-check logs
/// + a passing gates bundle. Audit must PASS.
fn write_complete_phase_exit_evidence(root: &std::path::Path) {
    write_native_bundle(
        root,
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(root, true, true);
    write_windows_bundle(root, good_windows_log(), false);
    write_six_target_bundle(
        root,
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(root, true);
}

#[test]
fn phase_exit_audit_passes_complete_evidence() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        ok,
        "complete phase-exit evidence must pass: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn phase_exit_audit_rejects_missing_platform() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // macos omitted entirely.
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "missing platform must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_platform_evidence"),
        "expected missing_platform_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_ci_sourced_without_pass_marker() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // macos CI log without a genuine pass marker -> zero-test, not absence-of-error.
    write_macos_ci_bundle(dir.path(), false, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "CI evidence without a pass marker must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "macos CI log without a pass must be flagged zero-test: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_ci_sourced_without_provenance() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // macos has a genuine pass marker but no `source` provenance note.
    write_macos_ci_bundle(dir.path(), true, false);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "CI-sourced evidence without provenance must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_provenance"),
        "expected missing_provenance: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_missing_six_target_triple() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // Only 5 of the 6 release triples are preserved.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a missing six-target triple must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_target_evidence"),
        "expected missing_target_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_ambiguous_six_target_log() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // One log records neither a Finished check nor a compiler error.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "ambiguous"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "an outcome-less target log must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("ambiguous_target_evidence"),
        "expected ambiguous_target_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_gate_without_pass_marker() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), false);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a gate capture without a pass marker must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "gate capture without a pass must be flagged zero-test: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_failed_target_evidence() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // One linux triple records a compiler failure -> the six-target gate is NOT
    // green and must be flagged, even though a preserved log exists.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "failed"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a compiler-failure target log must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("failed_target_evidence"),
        "expected failed_target_evidence: {stdout}"
    );
}

/// A complete, correct evidence tree whose workspace-test capture carries a
/// genuine pass line plus a `test result: FAILED` line (a run that both passed
/// some binaries and failed one). The audit must reject it as failed evidence.
#[test]
fn phase_exit_audit_rejects_gate_with_failed_test() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    // Overwrite the workspace-test capture with a run that ended FAILED.
    std::fs::write(
        dir.path().join("gates").join("gate-workspace-test.txt"),
        "test result: ok. 19 passed; 0 failed; 0 ignored\n\
         test result: FAILED. 1 passed; 1 failed\n\
         error: test failed, to rerun pass `-p opi-coding-agent --test x`\n",
    )
    .unwrap();
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a gate capture recording a failed run must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("failed_gate_evidence"),
        "expected failed_gate_evidence: {stdout}"
    );
}

/// A test-based gate capture with only `0 passed` lines plus a Finished line is
/// zero-test evidence and must be rejected (the Finished fallback is reserved
/// for the non-test gates).
#[test]
fn phase_exit_audit_rejects_zero_test_gate_capture() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_ci_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    // Overwrite the doctest capture: 0 passed but a Finished line -> zero-test.
    std::fs::write(
        dir.path().join("gates").join("gate-doctest.txt"),
        "test result: ok. 0 passed; 0 failed; 0 ignored\nFinished `test` profile\n",
    )
    .unwrap();
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a 0-passed test-based capture must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "expected zero_test_evidence for the 0-passed doctest capture: {stdout}"
    );
}

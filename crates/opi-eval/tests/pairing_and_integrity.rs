//! Pairing and integrity fail-closed suite.
//!
//! Black-box tests over the production `opi-eval run` binary focused on
//! the pairing owner's negative matrix (`EVAL-EXP-002`, `EVAL-EXP-004`,
//! `EVAL-EXP-008`) and the integrity record's exclusion and immutability
//! surfaces (`EVAL-INT-002`..`EVAL-INT-005`): missing, duplicated, and
//! non-expressible-control pairs; digest-addressed admission refusal; and
//! the never-rewrite guarantees on prior evidence and trial identities.
//! Hermetic fixture-grade only - no real agent, verifier, or provider.

// Hermetic opi-eval runs stage posix-sh helpers; the native execution
// surface is Linux (see the eval native smoke workflow).
#![cfg(unix)]
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn run_experiment(config: &str, behavior: &str, root: &Path) -> (i32, serde_json::Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment").join(config))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", behavior])
        .output()
        .expect("spawn the opi-eval run binary");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    let report: serde_json::Value = if stdout.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&stdout)
            .unwrap_or_else(|error| panic!("run stdout is not JSON ({error}): {stdout}"))
    };
    (code, report, stderr)
}

/// `EVAL-A13` (pairing-owner arm): pair and control failures fail closed -
/// an experiment whose pinned integrity digest does not address the
/// derived record is refused before any process starts; a duplicate pair
/// is refused with the typed reason; a missing pair stays visible with its
/// exact side.
#[test]
fn pair_and_control_fail_closed() {
    // Admission gate: a mismatched pinned digest refuses the whole run
    // before any trial directory or process exists.
    let root = tempfile::tempdir().unwrap();
    let tampered = root.path().join("tampered.toml");
    let text = std::fs::read_to_string(fixtures_dir().join("experiment/local-paired.toml"))
        .unwrap()
        .replace(
            "0c493e1e98d8be301a0a0d7837e0681aec685231ba6ae81642f07e57e5c6c135",
            &"f".repeat(64),
        );
    std::fs::write(&tampered, text).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&tampered)
        .arg("--root")
        .arg(root.path().canonicalize().unwrap().join("run"))
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "happy"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("integrity digest mismatch"),
        "the admission refusal must name the owning mismatch"
    );
    assert!(
        !root.path().join("run/trials").exists(),
        "no trial may start after an admission refusal"
    );

    // Duplicate pairing slot: typed refusal, never a silently chosen pair.
    let dup_root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("duplicate-pair.toml", "happy", dup_root.path());
    assert_eq!(code, 1, "{stderr}");
    let reason = report["comparison_error"].as_str().unwrap();
    assert!(reason.contains("duplicate"), "typed reason: {reason}");

    // Missing pair: the crashed trial reopens as effect-unknown and the
    // edge names the exact missing side, never removing the denominator.
    let crash_root = tempfile::tempdir().unwrap();
    let (crash_code, _, _) =
        run_experiment("local-paired.toml", "crash-after-intent", crash_root.path());
    assert_eq!(crash_code, 70);
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(crash_root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "happy"])
        .arg("--recover")
        .output()
        .unwrap();
    let report: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    let missing = pairs[0]["missing_sides"].as_array().unwrap();
    assert!(missing.contains(&serde_json::json!("candidate")));
}

/// `EVAL-A14` (integrity-owner arm): exclusions and reclassifications
/// produce new immutable record identities, stay visible in coverage, and
/// never rewrite prior evidence or reuse a trial identity.
#[test]
fn integrity_exclusion_and_reclassification() {
    // Baseline evidence under one root.
    let root = tempfile::tempdir().unwrap();
    let (code, base_report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "{stderr}");
    let baseline_manifest =
        std::fs::read(root.path().join("trials/trial-opi-1/bundle/manifest.json")).unwrap();
    let base_digest = base_report["integrity_digest"].as_str().unwrap().to_owned();

    // The same trial identity can never be reused: a second run against
    // the same root is refused by the durable reservation (EVAL-EXP-005).
    let (code, _, reuse_stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 2, "identity reuse must be refused: {reuse_stderr}");

    // Prior evidence is never rewritten by later records: the baseline
    // manifest bytes are unchanged after the refused reuse attempt.
    assert_eq!(
        std::fs::read(root.path().join("trials/trial-opi-1/bundle/manifest.json")).unwrap(),
        baseline_manifest
    );

    // A reclassified record is a new immutable identity: the exclusion and
    // invalid-task variants each address different digests while the
    // baseline record stays intact.
    let excl_root = tempfile::tempdir().unwrap();
    let (code, excl_report, stderr) = run_experiment(
        "integrity-exclusion.toml",
        "integrity-exclusion",
        excl_root.path(),
    );
    assert_eq!(code, 1, "{stderr}");
    let invalid_root = tempfile::tempdir().unwrap();
    let (code, invalid_report, stderr) =
        run_experiment("invalid-task.toml", "invalid-task", invalid_root.path());
    assert_eq!(code, 1, "{stderr}");
    let excl_digest = excl_report["integrity_digest"].as_str().unwrap();
    let invalid_digest = invalid_report["integrity_digest"].as_str().unwrap();
    assert_ne!(base_digest, excl_digest);
    assert_ne!(base_digest, invalid_digest);
    assert_ne!(excl_digest, invalid_digest);

    // Both variants stay visible in coverage with their exact reason.
    assert!(
        excl_report["pairs"][0]["comparability"]
            .as_str()
            .unwrap()
            .starts_with("excluded:")
    );
    assert_eq!(
        invalid_report["pairs"][0]["comparability"],
        "invalid-task-classification"
    );
}

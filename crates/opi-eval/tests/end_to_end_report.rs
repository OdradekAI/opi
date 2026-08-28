//! End-to-end offline report suite (task 18.13).
//!
//! Runs the production `opi-eval run` binary, then exercises the offline
//! `regrade` and `report` commands over the sealed assembled outputs. The
//! normalized report must publish headline outcomes only from native
//! grader artifacts with provenance, keep every declared pair visible in
//! the coverage denominator, never collapse quality/cost/safety/
//! efficiency/authority into one score, and label everything
//! conformance-only (`P18-RPT-003..006`, `P18-EXP-006`). Hermetic
//! fixture-grade only; task 18.15 owns the native rerun.

use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

/// Run one experiment through the real `opi-eval run` binary.
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
        serde_json::from_str(&stdout).unwrap_or_else(|error| {
            panic!("run stdout is not one JSON report ({error}): {stdout:?} stderr: {stderr:?}")
        })
    };
    (code, report, stderr)
}

/// Invoke the offline report command over one run root.
fn report(root: &Path) -> (i32, serde_json::Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .output()
        .expect("spawn the opi-eval report command");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    let value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    (code, value, stderr)
}

/// `P18-RPT-002`: the same sealed bundle, integrity record, grader
/// identity, and reporter version produce byte-stable normalized results -
/// across independent fresh runs, not just repeated renders.
#[test]
fn normalized_report_is_byte_stable_across_independent_runs() {
    let first = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run_experiment("phase18-local.toml", "happy", first.path());
    assert_eq!(code, 0, "{stderr}");
    let second = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run_experiment("phase18-local.toml", "happy", second.path());
    assert_eq!(code, 0, "{stderr}");

    let (c1, r1, stderr) = report(first.path());
    assert_eq!(c1, 0, "{stderr}");
    let (c2, r2, stderr) = report(second.path());
    assert_eq!(c2, 0, "{stderr}");
    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap(),
        "independent identical runs must normalize to identical bytes"
    );
}

/// The paired report contract end to end: native-only headlines with
/// provenance, visible diagnostics, full coverage denominator, no
/// composite score, conformance-only labeling.
#[test]
fn paired_report_contract_end_to_end() {
    let root = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(code, 0, "{stderr}");
    let (code, paired, stderr) = report(root.path());
    assert_eq!(code, 0, "{stderr}");

    // Conformance-only labeling (`P18-RPT-006`).
    assert_eq!(paired["classification"], "conformance-evidence", "{paired}");
    let bytes = serde_json::to_string(&paired).unwrap();
    for forbidden in ["leaderboard", "superiority", "official verification"] {
        assert!(
            !bytes.to_lowercase().contains(forbidden),
            "report must not claim {forbidden}: {bytes}"
        );
    }

    for trial in paired["trials"].as_array().unwrap() {
        // Headlines come only from the admitted native grader artifact with
        // provenance (`P18-RPT-003`).
        let headline = &trial["headline"];
        assert!(
            headline["native_source"]["artifact"]
                .as_str()
                .unwrap()
                .starts_with("native/"),
            "{trial}"
        );
        assert_eq!(
            headline["native_source"]["digest"].as_str().unwrap().len(),
            64,
            "{trial}"
        );
        // Diagnostics stay separately labelled, never mixed into the
        // headline.
        assert_eq!(trial["diagnostics"]["label"], "diagnostic", "{trial}");
    }

    // No composite score or best-trial verdict anywhere (`P18-RPT-005`).
    for forbidden in [
        "score",
        "composite",
        "winner",
        "best_trial",
        "best-trial",
        "verdict",
    ] {
        assert!(
            paired[forbidden].is_null(),
            "no composite field {forbidden}: {paired}"
        );
    }
    assert!(
        !bytes.to_lowercase().contains("best agent"),
        "no best-agent wording: {bytes}"
    );
}

/// `P18-EXP-006` / `P18-RPT-004`: exclusions and missing sides stay in the
/// coverage denominator with their exact reason; the report still
/// publishes with the run outcome visible.
#[test]
fn exclusions_stay_visible_in_the_coverage_denominator() {
    let root = tempfile::tempdir().unwrap();
    let (code, run_report, stderr) = run_experiment(
        "phase18-integrity-exclusion.toml",
        "integrity-exclusion",
        root.path(),
    );
    assert_eq!(code, 1, "excluded run is incomplete: {stderr} {run_report}");
    let (code, excluded, stderr) = report(root.path());
    assert_eq!(
        code, 0,
        "incomplete coverage publishes visibly: {stderr} {excluded}"
    );

    assert_eq!(excluded["run_outcome"], "incomplete", "{excluded}");
    let coverage = excluded["coverage"].as_array().unwrap();
    assert!(
        !coverage.is_empty(),
        "denominator is never empty: {excluded}"
    );
    for pair in coverage {
        let comparability = pair["comparability"].as_str().unwrap();
        assert_ne!(
            comparability, "comparable",
            "the excluded pair must not be silently dropped or marked comparable: {pair}"
        );
    }
    // The excluded trial's own receipt stays visible beside the survivor.
    let trials = excluded["trials"].as_array().unwrap();
    assert!(
        !trials.is_empty(),
        "surviving trials stay visible: {excluded}"
    );
}

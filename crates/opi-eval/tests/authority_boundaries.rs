//! Authority-boundary call-count suite (task 18.12, `P18-FAL-002`).
//!
//! For every owning failure boundary, these black-box runs prove ZERO
//! downstream authority-transition executions from the execution counts
//! recorded in the trial receipt and sealed bundle - not merely from
//! absent output files: no grade dispatch after an Agent-process or
//! evidence failure, no report after a sealing or grader failure. Hermetic
//! fixture-grade only - no real agent, verifier, or provider.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(unix)]
fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

#[cfg(unix)]
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

/// Count `transition` executions in one trial's authority map.
#[cfg(unix)]
fn executed(trial: &serde_json::Value, transition: &str) -> i64 {
    match &trial["authority"][transition] {
        serde_json::Value::String(state) if state == "executed" => 1,
        _ => 0,
    }
}

/// Read the sealed bundle's authority-ledger artifact and count executions
/// of `transition` (P18-FAL-002 durable call-count evidence).
#[cfg(unix)]
fn bundle_executed(root: &Path, trial: &str, transition: &str) -> i64 {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("trials").join(trial).join("bundle/manifest.json"))
            .unwrap(),
    )
    .unwrap();
    let present = manifest["entries"]["native/authority-ledger.json"].is_object();
    assert!(present, "the authority ledger must be sealed evidence");
    let ledger: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root.join("trials")
                .join(trial)
                .join("bundle/artifacts/native/authority-ledger.json"),
        )
        .unwrap(),
    )
    .unwrap();
    ledger["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| {
            record["transition"].as_str() == Some(transition)
                && record["state"].as_str() == Some("executed")
        })
        .count() as i64
}

/// Agent-process and evidence failures stop the grade dispatch: zero
/// verifier executions, while settlement, sealing, and the receipt stay
/// executable so the failure itself is retained evidence.
#[cfg(unix)]
#[test]
fn p18_fal002_agent_failures_stop_grade_dispatch() {
    for behavior in ["agent-timeout", "agent-missing-terminal"] {
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) = run_experiment("phase18-local.toml", behavior, root.path());
        assert_eq!(code, 1, "{behavior}: {stderr}");
        for trial in report["trials"].as_array().unwrap() {
            let id = trial["id"].as_str().unwrap();
            // Zero downstream grade executions, proven from both the
            // receipt and the sealed bundle.
            assert_eq!(executed(trial, "grade_dispatch"), 0, "{behavior}: {trial}");
            assert!(
                trial["authority"]["grade_dispatch"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("refused:"),
                "{behavior}: {trial}"
            );
            assert_eq!(
                bundle_executed(root.path(), id, "grade_dispatch"),
                0,
                "{behavior}"
            );
            // The refusal is visible in the sealed bundle: the ledger
            // artifact proves zero grade dispatches entered the sealed
            // evidence (its scope ends at sealing).
            assert_eq!(bundle_executed(root.path(), id, "settle"), 1, "{behavior}");
            // Settlement, sealing, and the receipt executed: the failure
            // is retained, not discarded.
            assert_eq!(executed(trial, "settle"), 1, "{behavior}: {trial}");
            assert_eq!(executed(trial, "seal"), 1, "{behavior}: {trial}");
            assert_eq!(executed(trial, "report"), 1, "{behavior}: {trial}");
        }
    }
}

/// A sealing failure (staged evidence drifts before canonical sealing)
/// stops the report transition mechanically: the bundle never publishes a
/// manifest and the receipt is never written, with the counts proving the
/// stopped transitions rather than merely absent files.
#[cfg(unix)]
#[test]
fn p18_fal002_seal_failure_stops_report() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "seal-failure", root.path());
    assert_eq!(code, 1, "{stderr}");
    for trial in report["trials"].as_array().unwrap() {
        let id = trial["id"].as_str().unwrap();
        // The seal transition itself failed; the report is refused.
        assert_eq!(trial["authority"]["seal"], "failed:at-evidence", "{trial}");
        assert_eq!(executed(trial, "seal"), 0, "{trial}");
        assert_eq!(
            trial["authority"]["report"], "refused:seal-failed",
            "{trial}"
        );
        assert_eq!(executed(trial, "report"), 0, "{trial}");
        // No manifest was published and no receipt written - and the
        // counts above prove WHY, instead of the files merely being absent.
        let bundle_root = root.path().join("trials").join(id).join("bundle");
        assert!(!bundle_root.join("manifest.json").exists());
        assert!(
            !root
                .path()
                .join("trials")
                .join(id)
                .join("receipt.json")
                .is_file()
        );
        // The grade itself ran: proven by the receipt counts (the
        // bundle never sealed, so it carries no manifest).
        assert_eq!(executed(trial, "grade_dispatch"), 1, "{trial}");
    }
}

/// A grader failure stops the report transition while the sealed bundle
/// retains the verifier failure evidence.
#[cfg(unix)]
#[test]
fn p18_fal002_grader_failure_stops_report() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("phase18-local.toml", "verifier-failure", root.path());
    assert_eq!(code, 1, "{stderr}");
    for trial in report["trials"].as_array().unwrap() {
        let id = trial["id"].as_str().unwrap();
        assert_eq!(executed(trial, "grade_dispatch"), 1, "{trial}");
        assert_eq!(executed(trial, "seal"), 1, "{trial}");
        assert_eq!(executed(trial, "report"), 0, "{trial}");
        assert_eq!(
            trial["authority"]["report"], "refused:stopped-at-grader",
            "{trial}"
        );
        // The sealed bundle retains the verifier failure; the receipt
        // transition stayed refused.
        assert_eq!(bundle_executed(root.path(), id, "grade_dispatch"), 1);
        assert!(
            !root
                .path()
                .join("trials")
                .join(id)
                .join("receipt.json")
                .is_file()
        );
    }
}

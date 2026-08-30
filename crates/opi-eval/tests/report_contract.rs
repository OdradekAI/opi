//! Offline report contract suite (task 18.13).
//!
//! Every case drives the production `opi-eval` binary: `report` consumes
//! the sealed assembled outputs of `opi-eval run` (task 18.12), recomputes
//! the normalized view through `ReportBuilder::recompute_from_bundle`
//! before rendering, and publishes one conformance-only report. Asymmetric
//! native facts stay measured values or typed unknowns - never fabricated
//! parity - and canary leakage blocks publication. This proves the
//! hermetic fixture-grade offline path only (task 18.15 owns the native
//! rerun).

// Hermetic Phase 18 runs stage posix-sh helpers; the native execution
// surface is Linux (see the phase18 native smoke workflow).
#![cfg(unix)]
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

/// Invoke one offline subcommand against a run root.
fn invoke(command: &str, args: &[(&str, &std::ffi::OsStr)]) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
    cmd.arg(command);
    for (flag, value) in args {
        cmd.arg(flag).arg(value);
    }
    let output = cmd.output().expect("spawn the opi-eval offline command");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `P18-A16`: Opi and pi expose asymmetric call, usage, cost, retry, and
/// compaction facts. Native facts remain retained; the common report uses
/// measured values or typed unknowns and never fabricates parity.
#[test]
fn p18_a16_asymmetric_native_facts() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");

    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} stdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));

    let trials = report["trials"].as_array().unwrap();
    let opi = trials
        .iter()
        .find(|t| t["subject"] == "candidate-opi")
        .expect("opi trial present");
    let pi = trials
        .iter()
        .find(|t| t["subject"] == "baseline-pi")
        .expect("pi trial present");

    // Opi's usage is a measured native fact: cited by bundle artifact and
    // digest, never a bare number detached from its evidence.
    let usage = &opi["native_facts"]["usage"];
    assert_eq!(usage["state"], "measured", "{opi}");
    assert_eq!(usage["artifact"], "native/evidence/records", "{usage}");
    assert_eq!(usage["digest"].as_str().unwrap().len(), 64, "{usage}");

    // Facts no hermetic product exposes natively stay typed unknowns for
    // both sides - never zero, never copied across subjects.
    for fact in ["cost", "retry", "compaction"] {
        assert_eq!(
            opi["native_facts"][fact]["state"],
            format!("unknown:opi-{fact}-not-native"),
            "{opi}"
        );
        assert_eq!(
            pi["native_facts"][fact]["state"],
            format!("unknown:pi-{fact}-not-native"),
            "{pi}"
        );
    }

    // Pi's session events carry no native usage projection: the report
    // keeps the gap visible as a typed unknown and never balances the pair
    // by copying Opi's measured fact or fabricating a zero.
    let pi_usage = &pi["native_facts"]["usage"];
    assert_eq!(pi_usage["state"], "unknown:pi-usage-not-native", "{pi}");
    assert!(pi_usage.get("digest").is_none(), "{pi_usage}");
    assert!(pi_usage.get("value").is_none(), "{pi_usage}");
    assert_ne!(
        serde_json::to_string(pi_usage).unwrap(),
        serde_json::to_string(usage).unwrap(),
        "parity must not be fabricated"
    );
}

/// `P18-A18` / `P18-SEC-005`: canary secrets in sealed content block
/// bundle sealing and report publication. The blocked report never echoes
/// the canary itself.
#[test]
fn p18_a18_canary_leakage_blocks_bundle_and_report() {
    let canaries = fixtures_dir().join("reports/canaries/hermetic.txt");
    assert!(canaries.is_file(), "pinned hermetic canary fixture");

    // Seal-side block: with the canary declared at run time, the leaky
    // agent output never enters a sealed bundle - no manifest is
    // published and the trial fails with the evidence boundary.
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/phase18-local.toml"))
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "canary-leak"])
        .arg("--canaries")
        .arg(&canaries)
        .output()
        .expect("spawn the opi-eval run binary");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("run stdout is not one JSON report: {stdout:?}"));
    assert_eq!(code, 1, "leaky run must not complete: {report}");
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["status"], "failed", "{trial}");
        assert_eq!(trial["authority"]["seal"], "failed:at-evidence", "{trial}");
        let manifest = root
            .path()
            .join("trials")
            .join(trial["id"].as_str().unwrap())
            .join("bundle/manifest.json");
        assert!(
            !manifest.is_file(),
            "a leaking bundle must never be sealed: {trial}"
        );
    }

    // Report-side block: a bundle sealed while the canary was undeclared
    // (the leak slipped into sealed content) is blocked from publication
    // the moment the canary is declared to the report command.
    let leaky = tempfile::tempdir().unwrap();
    let (code, run_report, stderr) =
        run_experiment("phase18-local.toml", "canary-leak", leaky.path());
    assert_eq!(
        code, 0,
        "undeclared leak seals for the report-side test: {stderr} {run_report}"
    );
    let root_arg = leaky.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--canaries", canaries.as_os_str()),
        ],
    );
    assert_eq!(code, 1, "leaky report must be blocked: {stderr} {stdout}");
    let blocked: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    assert_eq!(blocked["outcome"], "publication-blocked", "{blocked}");
    let leak = &blocked["leak"];
    assert!(leak["trial"].is_string(), "{blocked}");
    assert!(leak["artifact"].is_string(), "{blocked}");
    // No raw canary and no machine-local path is echoed by the blocked
    // report.
    assert!(
        !stdout.contains("OPZ-EVAL-CANARY-7f3a9c"),
        "the blocked report must never echo the canary: {stdout}"
    );
    assert!(
        !stdout.contains(root_arg.to_string_lossy().as_ref()),
        "{stdout}"
    );
    assert!(
        blocked["trials"].is_null() && blocked["coverage"].is_null(),
        "no normalized content is published: {blocked}"
    );
}

//! Offline report contract suite.
//!
//! Every case drives the production `opi-eval` binary: `report` consumes
//! the sealed assembled outputs of `opi-eval run`, recomputes
//! the normalized view through `ReportBuilder::recompute_from_bundle`
//! before rendering, and publishes one conformance-only report. Asymmetric
//! native facts stay measured values or typed unknowns - never fabricated
//! parity - and canary leakage blocks publication. This proves the
//! hermetic fixture-grade offline path only; native execution is verified
//! separately by the native-smoke workflow.

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

/// The report output is isolated from the run root :
/// an `--out` target inside the run root is rejected outright, and an
/// existing target is never replaced.
#[test]
fn output_isolation_rejects_in_root_and_existing_targets() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let root_arg = root.path().canonicalize().unwrap();

    // A target inside the run root is rejected: sealed bytes and outer
    // run artifacts can never be replaced by report output.
    let inside = root
        .path()
        .join("trials")
        .join("trial-opi-1")
        .join("report.json");
    let (code, stdout, stderr) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--out", inside.as_os_str()),
        ],
    );
    assert_eq!(code, 2, "in-root --out must be rejected: {stderr} {stdout}");
    assert!(stderr.contains("inside the run root"), "{stderr}");
    assert!(
        !inside.exists(),
        "no bytes may be written to the rejected target"
    );

    // An existing target outside the run root is never replaced.
    let outputs = tempfile::tempdir().unwrap();
    let existing = outputs.path().join("report.json");
    std::fs::write(&existing, b"prior evidence\n").unwrap();
    let (code, stdout, stderr) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--out", existing.as_os_str()),
        ],
    );
    assert_eq!(
        code, 2,
        "existing --out must be rejected: {stderr} {stdout}"
    );
    assert!(stderr.contains("already exists"), "{stderr}");
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        b"prior evidence\n",
        "a prior report is never replaced"
    );
}

/// `EVAL-A16`: Opi and pi expose asymmetric call, usage, cost, retry, and
/// compaction facts. Native facts remain retained; the common report uses
/// measured values or typed unknowns and never fabricates parity.
#[test]
fn asymmetric_native_facts() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
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

/// `P19-AC-008`: the published report keeps native metric meanings,
/// denominators, and honest failure classes. Graded trials (including
/// benchmark-defined Agent timeouts and zero scores) stay in the valid
/// denominator; evidence-boundary stops stay attempted-but-invalid and
/// never score; no cross-benchmark average exists.
#[test]
fn native_outcome_counts_keep_denominators_and_failure_classes() {
    // Happy path: both subjects graded, no failures.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let counts = &published["coverage_counts"];
    assert_eq!(counts["planned"], 2, "{published}");
    assert_eq!(counts["attempted"], 2, "{published}");
    assert_eq!(counts["valid"], 2, "{published}");
    assert_eq!(counts["agent_failure"], 0, "{published}");
    assert_eq!(counts["invalid"], 0, "{published}");
    assert_eq!(counts["unexecuted"], 0, "{published}");

    // Each benchmark reports its own metric definition and denominator;
    // results are never averaged into one cross-benchmark headline.
    let metrics = published["metrics"].as_array().unwrap();
    assert_eq!(metrics.len(), 1, "{published}");
    assert_eq!(metrics[0]["benchmark"], "terminal-bench/2.1", "{published}");
    assert_eq!(metrics[0]["denominator"], 2, "{published}");
    let definition = metrics[0]["definition"].as_str().unwrap();
    assert!(definition.contains("zero"), "{definition}");
    assert!(
        !stdout.to_lowercase().contains("average"),
        "no cross-benchmark average may exist: {stdout}"
    );

    // No selection block: the scope label stays absent, not fabricated.
    assert!(published["selection_scope"].is_null(), "{published}");
    for trial in published["trials"].as_array().unwrap() {
        assert_eq!(trial["outcome"]["class"], "graded", "{trial}");
        assert_eq!(trial["outcome"]["agent_failed"], false, "{trial}");
    }

    // Benchmark-defined Agent timeouts stay graded Agent outcomes in
    // the denominator, counted as Agent failures - never as invalid.
    let timeout = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("local-paired.toml", "agent-timeout", timeout.path());
    assert_eq!(code, 0, "timeout run completes: {stderr} {report}");
    let root_arg = timeout.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let counts = &published["coverage_counts"];
    assert_eq!(counts["valid"], 2, "{published}");
    assert_eq!(counts["agent_failure"], 2, "{published}");
    assert_eq!(counts["invalid"], 0, "{published}");
    assert_eq!(published["metrics"][0]["denominator"], 2, "{published}");
    for trial in published["trials"].as_array().unwrap() {
        assert_eq!(trial["outcome"]["class"], "graded", "{trial}");
        assert_eq!(trial["outcome"]["agent_failed"], true, "{trial}");
        assert_eq!(trial["outcome"]["timed_out"], true, "{trial}");
    }

    // An evidence-boundary stop (agent-model-mismatch) keeps the trial
    // attempted and visible but out of the graded denominator: never an
    // Agent success or failure, never silently dropped.
    let mismatch = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("model-mismatch-pair.toml", "happy", mismatch.path());
    assert_eq!(
        code, 1,
        "mismatch run honestly exits incomplete: {stderr} {report}"
    );
    assert_eq!(report["outcome"], "incomplete", "{report}");
    let root_arg = mismatch.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let counts = &published["coverage_counts"];
    assert_eq!(counts["attempted"], 2, "{published}");
    assert_eq!(counts["valid"], 1, "{published}");
    assert_eq!(counts["agent_failure"], 0, "{published}");
    assert_eq!(counts["invalid"], 1, "{published}");
    assert_eq!(published["metrics"][0]["denominator"], 1, "{published}");
    let pi = published["trials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trial| trial["subject"] == "baseline-pi")
        .unwrap();
    assert_eq!(pi["outcome"]["class"], "invalid", "{pi}");
    assert!(
        pi["outcome"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("agent-model-mismatch")),
        "{pi}"
    );
    assert_eq!(published["run_outcome"], "incomplete", "{published}");
}

/// `P19-AC-009` (verify split): byte verification and actual native
/// regrading are distinct operations. `verify --root R` owns the
/// original byte/seal check; `regrade` returns an explicit
/// unavailable-operation diagnostic (exit 2) until P19-D08 supplies the
/// native regrade workflow, never performing byte verification under
/// that name.
#[test]
fn verify_regrade_split_names_each_operation() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let root_arg = root.path().canonicalize().unwrap();

    // Byte verification is `verify`: every sealed bundle re-checks its
    // original seals with no Agent, provider, or mutation.
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(
        code, 0,
        "verify must confirm intact seals: {stderr} {stdout}"
    );
    let verified: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("verify stdout is not one JSON report: {stdout:?}"));
    assert_eq!(verified["outcome"], "verified", "{verified}");
    assert!(
        verified["bundles"]
            .as_array()
            .is_some_and(|b| !b.is_empty()),
        "every sealed bundle stays listed: {verified}"
    );

    // A tampered sealed byte fails verification with a typed failure and
    // untouched bytes.
    let mut sealed = None;
    let trials_dir = root.path().join("trials");
    for entry in std::fs::read_dir(&trials_dir).expect("trials directory exists") {
        let manifest = entry
            .expect("trial directory entry")
            .path()
            .join("bundle/manifest.json");
        if manifest.is_file() {
            sealed = Some(manifest);
            break;
        }
    }
    let manifest = sealed.expect("a sealed bundle manifest exists");
    let original = std::fs::read(&manifest).unwrap();
    std::fs::write(&manifest, b"{\"tampered\": true}\n").unwrap();
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(
        code, 1,
        "tampered bytes must fail verify: {stderr} {stdout}"
    );
    assert_eq!(std::fs::read(&manifest).unwrap(), b"{\"tampered\": true}\n");
    std::fs::write(&manifest, &original).unwrap();

    // Actual native regrading is not implemented yet: the command says
    // so explicitly instead of quietly re-running byte checks.
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 2, "regrade must reject: {stderr} {stdout}");
    assert!(stdout.is_empty(), "no byte-verification output: {stdout}");
    assert!(
        stderr.contains("unavailable-operation")
            && stderr.contains("native regrade")
            && stderr.contains("verify"),
        "the diagnostic names the operation and the interim alternative: {stderr}"
    );
}

/// `P19-AC-008` (protocol-safe pairing): paired comparisons never match
/// two protocol groups of one product; single-subject reports need no
/// artificial comparison edge, and repetition pairs surface as control
/// pairs with their exact state.
#[test]
fn protocol_safe_pairing_and_single_subject_coverage() {
    // Same-product edge: the pair stays visible but never comparable -
    // a paired claim across protocol groups is refused by name.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("protocol-mismatch.toml", "happy", root.path());
    assert_eq!(
        code, 1,
        "the protocol-mismatch run honestly exits incomplete: {stderr} {report}"
    );
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let coverage = published["coverage"].as_array().unwrap();
    assert_eq!(coverage.len(), 1, "{published}");
    assert_eq!(
        coverage[0]["comparability"], "protocol-group-mismatch",
        "{published}"
    );
    assert_eq!(
        coverage[0]["baseline_trial"], "trial-opi-a-1",
        "{published}"
    );
    assert_eq!(
        coverage[0]["candidate_trial"], "trial-opi-b-1",
        "{published}"
    );

    // Single subject, two repetitions, no edge: the report publishes
    // without any artificial pair and the repetitions surface as control
    // pairs with their state.
    let solo = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("pi-repetitions.toml", "happy", solo.path());
    assert_eq!(code, 0, "single-subject run completes: {stderr} {report}");
    let root_arg = solo.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    assert_eq!(published["run_outcome"], "completed", "{published}");
    assert_eq!(
        published["coverage"].as_array().unwrap().len(),
        0,
        "no artificial comparison edge: {published}"
    );
    let control = published["control_pairs"].as_array().unwrap();
    assert_eq!(control.len(), 1, "{published}");
    assert_eq!(control[0]["subject"], "baseline-pi", "{published}");
    assert_eq!(control[0]["task"], "fixture-task", "{published}");
    assert_eq!(control[0]["state"], "consistent", "{published}");
    assert_eq!(
        control[0]["groups"].as_array().unwrap().len(),
        2,
        "{published}"
    );

    // A missing repetition keeps the control pair visible with its
    // exact state: the crashed group is never silently dropped.
    let crashed = tempfile::tempdir().unwrap();
    let (code, _, _) = run_experiment("pi-repetitions.toml", "crash-after-intent", crashed.path());
    assert_eq!(code, 70, "the crash behavior must exit 70");
    let root_arg = crashed.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let control = published["control_pairs"].as_array().unwrap();
    assert_eq!(control.len(), 1, "{published}");
    assert_eq!(control[0]["state"], "incomplete", "{published}");
    let missing = control[0]["missing_groups"].as_array().unwrap();
    assert_eq!(
        missing.len(),
        2,
        "both repetitions stay missing: {published}"
    );
}

/// `EVAL-A18` / `EVAL-SEC-005`: canary secrets in sealed content block
/// bundle sealing and report publication. The blocked report never echoes
/// the canary itself.
#[test]
fn canary_leakage_blocks_bundle_and_report() {
    let canaries = fixtures_dir().join("reports/canaries/hermetic.txt");
    assert!(canaries.is_file(), "pinned hermetic canary fixture");

    // Seal-side block: with the canary declared at run time, the leaky
    // agent output never enters a sealed bundle - no manifest is
    // published and the trial fails with the evidence boundary.
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
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
        run_experiment("local-paired.toml", "canary-leak", leaky.path());
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

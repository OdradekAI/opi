//! Cross-platform report-output containment regressions .

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn seed_run(root: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "happy"])
        .output()
        .expect("spawn the opi-eval run binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "seed run must succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn report(root: &Path, out: &Path, current_dir: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
    command
        .arg("report")
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg(out);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    command.output().expect("spawn opi-eval report")
}

#[cfg(windows)]
fn create_directory_alias(link: &Path, target: &Path) {
    let output = Command::new("cmd")
        .args([
            "/D",
            "/C",
            "mklink",
            "/J",
            &link.to_string_lossy().replace('/', "\\"),
            &target.to_string_lossy().replace('/', "\\"),
        ])
        .output()
        .expect("spawn mklink for report containment regression");
    assert!(
        output.status.success(),
        "create report-output junction: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn create_directory_alias(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create report-output symlink");
}

#[test]
fn output_rejects_ancestor_alias_into_run_root() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let bundle = canonical_root.join("trials/trial-opi-1/bundle");
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("into-run");
    create_directory_alias(&alias, &bundle);
    let out = alias.join("unmanifested-report.json");
    let in_bundle = bundle.join("unmanifested-report.json");

    let output = report(&canonical_root, &out, None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "ancestor alias into the run root must be rejected: stdout={} stderr={stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(stderr.contains("inside the run root"), "{stderr}");
    assert!(!out.exists(), "the alias target must not be created");
    assert!(
        !in_bundle.exists(),
        "no unmanifested report may appear inside the sealed bundle"
    );
}

#[test]
fn output_rejects_relative_in_root_target() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let parent = canonical_root.parent().unwrap();
    let relative = canonical_root
        .strip_prefix(parent)
        .unwrap()
        .join("relative-report.json");

    let output = report(&canonical_root, &relative, Some(parent));
    assert_eq!(output.status.code(), Some(2));
    assert!(!canonical_root.join("relative-report.json").exists());
}

#[test]
fn output_accepts_one_fresh_target_under_resolved_external_parent() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let outputs = tempfile::tempdir().unwrap();
    let out = outputs.path().join("report.json");

    let first = report(&canonical_root, &out, None);
    assert_eq!(
        first.status.code(),
        Some(0),
        "fresh external target must succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(out.is_file());

    let second = report(&canonical_root, &out, None);
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn write_json(path: &Path, value: &serde_json::Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Stage a changed synthetic fixture and seal it with the bundle wire order.
/// Only isolated test bundles are passed here, never preserved run evidence.
fn fixture_artifact(root: &Path, trial: &str, key: &str, value: serde_json::Value) {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    #[derive(Serialize, Deserialize)]
    struct Intent {
        trial: String,
        pair: String,
        artifacts: Vec<String>,
        expected_output: String,
    }
    #[derive(Serialize, Deserialize)]
    struct Entry {
        role: String,
        source: String,
        digest: String,
        classification: String,
        causal: Vec<String>,
    }
    #[derive(Serialize, Deserialize)]
    struct Body {
        format: String,
        version: u32,
        intent: Intent,
        settlement: serde_json::Value,
        entries: BTreeMap<String, Entry>,
    }
    let bundle = root.join("trials").join(trial).join("bundle");
    let manifest_path = bundle.join("manifest.json");
    let mut body: Body = serde_json::from_value(read_json(&manifest_path)).unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    let path = bundle.join("artifacts").join(key);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, &bytes).unwrap();
    body.entries
        .entry(key.to_owned())
        .or_insert_with(|| Entry {
            role: "Native".to_owned(),
            source: "grader-fixture".to_owned(),
            digest: String::new(),
            classification: "Exportable".to_owned(),
            causal: Vec::new(),
        })
        .digest = sha256(&bytes);
    let digest = sha256(&serde_json::to_vec(&body).unwrap());
    let mut manifest = serde_json::to_value(body).unwrap();
    manifest["identity"] = serde_json::json!(digest);
    write_json(&manifest_path, &manifest);
}

fn normalized(root: &Path) -> (i32, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .output()
        .unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "report JSON: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap(), value)
}

fn native_report_fixture(root: &Path) {
    seed_run(root);
    for (trial, reward) in [("trial-opi-1", 1), ("trial-pi-1", 0)] {
        let trajectory_path = root
            .join("trials")
            .join(trial)
            .join("bundle/artifacts/evidence/trajectory.json");
        let mut trajectory = read_json(&trajectory_path);
        let grader = trajectory["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["kind"].as_str().unwrap().starts_with("grader/"))
            .unwrap();
        grader["facts"]["reward"] =
            serde_json::json!({"Known": {"value": reward, "origin": "native-trial-result"}});
        // Historical VerifiedNative projection incorrectly recorded zero.
        grader["facts"]["completion"]["Known"]["value"] = serde_json::json!(0);
        fixture_artifact(root, trial, "evidence/trajectory.json", trajectory);
        fixture_artifact(
            root,
            trial,
            "native/artifacts-manifest.json",
            serde_json::json!([]),
        );
        fixture_artifact(
            root,
            trial,
            "native/trial-result.json",
            serde_json::json!({
                "verifier_result": {"rewards": {"reward": reward}}
            }),
        );
    }
}

#[test]
fn native_completion_survives_historical_projection_zero() {
    let root = tempfile::tempdir().unwrap();
    native_report_fixture(root.path());
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(
            trial["diagnostics"]["verifier"]["completion"], "verified",
            "{trial}"
        );
        assert_eq!(trial["outcome"]["class"], "graded", "{trial}");
    }
    assert_eq!(report["coverage_counts"]["valid"], 2, "{report}");
}

#[test]
fn native_reward_cites_exact_trial_result() {
    let root = tempfile::tempdir().unwrap();
    native_report_fixture(root.path());
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    for trial in report["trials"].as_array().unwrap() {
        let source = &trial["headline"]["native_source"];
        assert_eq!(source["artifact"], "native/trial-result.json", "{trial}");
        let path = root
            .path()
            .join("trials")
            .join(trial["id"].as_str().unwrap())
            .join("bundle/artifacts/native/trial-result.json");
        assert_eq!(
            source["digest"],
            sha256(&std::fs::read(path).unwrap()),
            "{trial}"
        );
    }
}

#[test]
fn native_test_counts_are_separate_provenance_bound_observations() {
    for (fields, state, reason, value) in [
        (
            serde_json::json!({"passed": 5.0, "total": 5.0}),
            "measured",
            None,
            Some(1.0),
        ),
        (
            serde_json::json!({"passed": 0, "total": 5}),
            "measured",
            None,
            Some(0.0),
        ),
        (
            serde_json::json!({"passed": 3, "total": 5, "f2p": 7}),
            "measured",
            None,
            Some(0.6),
        ),
        (
            serde_json::json!({}),
            "unknown",
            Some("native-test-counts-missing"),
            None,
        ),
        (
            serde_json::json!({"passed": 0, "total": 0}),
            "unknown",
            Some("native-test-total-zero"),
            None,
        ),
        (
            serde_json::json!({"passed": 6, "total": 5}),
            "unknown",
            Some("native-test-counts-invalid"),
            None,
        ),
        (
            serde_json::json!({"passed": 0.5, "total": 5}),
            "unknown",
            Some("native-test-counts-invalid"),
            None,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        native_report_fixture(root.path());
        let mut rewards = fields.clone();
        rewards["reward"] = serde_json::json!(0);
        fixture_artifact(
            root.path(),
            "trial-pi-1",
            "native/trial-result.json",
            serde_json::json!({"verifier_result": {"rewards": rewards}}),
        );
        let before = fixture_tree_bytes(root.path());
        let (code, report) = normalized(root.path());
        assert_eq!(code, 0, "{report}");
        assert_eq!(report["schema"], "opi-eval-normalized-report/2");
        assert_eq!(report["reporter_version"], "opi-eval-reporter/2");
        let trial = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["id"] == "trial-pi-1")
            .unwrap();
        assert_eq!(trial["headline"]["reward"], "known:0(native-trial-result)");
        let observation = &trial["native_grader"];
        let fraction = &observation["test_case_fraction"];
        assert_eq!(fraction["state"], state, "{trial}");
        assert_eq!(fraction["reason"].as_str(), reason, "{trial}");
        assert_eq!(fraction["value"].as_f64(), value, "{trial}");
        assert_eq!(observation["native_fields"], rewards, "{trial}");
        assert_eq!(
            observation["native_source"],
            trial["headline"]["native_source"]
        );
        if state == "measured" {
            assert_eq!(fraction["numerator"].as_f64(), fields["passed"].as_f64());
            assert_eq!(fraction["denominator"].as_f64(), fields["total"].as_f64());
        }
        assert_eq!(report["metrics"][0]["denominator"], 2);
        assert_eq!(
            report["metrics"][1]["observation_coverage"],
            serde_json::json!({
                "eligible": 2, "measured": usize::from(state == "measured"),
                "unknown": 2 - usize::from(state == "measured"),
            })
        );
        assert_eq!(normalized(root.path()).1, report);
        assert_eq!(
            fixture_tree_bytes(root.path()),
            before,
            "source bytes changed"
        );
    }
}

#[test]
fn native_reward_without_matching_score_evidence_blocks_publication() {
    for result in [
        serde_json::json!([]),
        serde_json::json!({"verifier_result": {"rewards": {"reward": 0}}}),
    ] {
        let root = tempfile::tempdir().unwrap();
        native_report_fixture(root.path());
        fixture_artifact(
            root.path(),
            "trial-opi-1",
            "native/trial-result.json",
            result,
        );
        let (code, report) = normalized(root.path());
        assert_eq!(code, 1, "{report}");
        assert_eq!(report["outcome"], "verification-failed", "{report}");
        assert!(report["trials"].is_null(), "{report}");
    }
}

fn unsealed_failure_fixture(root: &Path) {
    native_report_fixture(root);
    let trial = root.join("trials/trial-pi-1");
    std::fs::remove_file(trial.join("bundle/manifest.json")).unwrap();
    std::fs::remove_file(trial.join("receipt.json")).unwrap();
    write_json(
        &trial.join("bundle/artifacts/control/settlement.json"),
        &serde_json::json!({
            "schema": "opi-eval-trial-settlement/1", "trial": "trial-pi-1",
            "graded": false, "agent_failure": false, "timed_out": false,
            "failure_kind": "verifier-timeout", "boundary": "grader"
        }),
    );
}

#[test]
fn unsealed_settled_failure_keeps_attempt_and_invalid_denominators() {
    let root = tempfile::tempdir().unwrap();
    unsealed_failure_fixture(root.path());
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    let counts = &report["coverage_counts"];
    assert_eq!(counts["attempted"], 2, "{report}");
    assert_eq!(counts["planned"], 2, "{report}");
    assert_eq!(counts["valid"], 1, "{report}");
    assert_eq!(counts["invalid"], 1, "{report}");
    assert_eq!(counts["unexecuted"], 0, "{report}");
    assert_eq!(report["metrics"][0]["denominator"], 1, "{report}");
    let pi = report["trials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trial| trial["id"] == "trial-pi-1")
        .unwrap();
    assert_eq!(pi["status"], "unsealed", "{pi}");
    assert_eq!(pi["outcome"]["class"], "invalid", "{pi}");
    assert_eq!(pi["outcome"]["reason"], "verifier-timeout", "{pi}");
    assert_eq!(pi["outcome"]["source"], "unsealed-settlement", "{pi}");
    assert_eq!(
        pi["native_grader"]["test_case_fraction"]["reason"],
        "invalid-trial"
    );
    assert_eq!(report["metrics"][1]["observation_coverage"]["eligible"], 1);
    assert!(pi.get("headline").is_none(), "{pi}");
    assert!(pi.get("seal_result").is_none(), "{pi}");
    assert_eq!(
        report["bundle_verification"]["trial-pi-1"]["state"], "unsealed",
        "{report}"
    );
    assert_eq!(report["run_outcome"], "incomplete", "{report}");
    assert_eq!(
        report["coverage"][0]["baseline_trial"], "trial-pi-1",
        "{report}"
    );
    assert_eq!(
        report["coverage"][0]["comparability"], "invalid:trial-pi-1:verifier-timeout",
        "{report}"
    );
    assert_eq!(
        normalized(root.path()).1,
        report,
        "offline reconstruction must be deterministic"
    );
}

#[test]
fn unsealed_metadata_conflicts_block_publication() {
    for case in [
        "control-selection",
        "intent-pair",
        "planned-subject",
        "planned-duplicate",
        "control-malformed",
        "control-identity",
        "staged-experiment",
        "settlement-trial",
        "settlement-schema",
        "settlement-flags",
        "settlement-reason",
        "intent-trial",
        "marker-trial",
        "receipt-classification",
    ] {
        let root = tempfile::tempdir().unwrap();
        unsealed_failure_fixture(root.path());
        let bundle = root.path().join("trials/trial-pi-1/bundle");
        let (path, pointer, replacement) = match case {
            "control-selection" => (
                root.path().join("run-control.json"),
                "/selection",
                serde_json::json!({"scope": "full", "task_ids": ["fixture-task"], "repetitions": 1}),
            ),
            "intent-pair" => (
                bundle.join("intent.json"),
                "/pair",
                serde_json::json!("unrelated-edge"),
            ),
            "planned-subject" => (
                root.path().join("run-control.json"),
                "/planned_trials/0/subject",
                serde_json::json!("other"),
            ),
            "planned-duplicate" => {
                let path = root.path().join("run-control.json");
                let mut control = read_json(&path);
                let extra = control["planned_trials"][0].clone();
                control["planned_trials"]
                    .as_array_mut()
                    .unwrap()
                    .push(extra);
                write_json(&path, &control);
                (path, "/schema", serde_json::json!("opi-eval-run-control/1"))
            }
            "control-malformed" => (
                root.path().join("run-control.json"),
                "/schema",
                serde_json::json!("bad-schema"),
            ),
            "control-identity" => (
                root.path().join("run-control.json"),
                "/experiment",
                serde_json::json!("other"),
            ),
            "staged-experiment" => (
                bundle.join("artifacts/control/experiment.json"),
                "/manifest_digest",
                serde_json::json!("f".repeat(64)),
            ),
            "settlement-trial" => (
                bundle.join("artifacts/control/settlement.json"),
                "/trial",
                serde_json::json!("other"),
            ),
            "settlement-schema" => (
                bundle.join("artifacts/control/settlement.json"),
                "/schema",
                serde_json::json!("other"),
            ),
            "settlement-flags" => (
                bundle.join("artifacts/control/settlement.json"),
                "/agent_failure",
                serde_json::json!(true),
            ),
            "settlement-reason" => (
                bundle.join("artifacts/control/settlement.json"),
                "/failure_kind",
                serde_json::Value::Null,
            ),
            "intent-trial" => (
                bundle.join("intent.json"),
                "/trial",
                serde_json::json!("other"),
            ),
            "marker-trial" => (
                bundle.join("settlement.json"),
                "/trial",
                serde_json::json!("other"),
            ),
            "receipt-classification" => {
                let path = root.path().join("trials/trial-pi-1/receipt.json");
                write_json(
                    &path,
                    &serde_json::json!({
                        "schema": "opi-eval-trial-receipt/1", "id": "trial-pi-1",
                        "subject": "baseline-pi", "task": "fixture-task", "group": "group-hello",
                        "status": "failed", "classification": {"class": "invalid-execution", "reason": "different", "boundary": "grader"}
                    }),
                );
                (path, "/status", serde_json::json!("failed"))
            }
            _ => unreachable!(),
        };
        let mut value = read_json(&path);
        *value.pointer_mut(pointer).unwrap() = replacement;
        write_json(&path, &value);
        let (code, report) = normalized(root.path());
        assert_eq!(code, 1, "{case}: {report}");
        assert_eq!(report["outcome"], "verification-failed", "{case}: {report}");
        assert!(report["trials"].is_null(), "{case}: {report}");
    }
}

#[test]
fn unsealed_metadata_never_becomes_a_grade_or_a_seal() {
    let root = tempfile::tempdir().unwrap();
    unsealed_failure_fixture(root.path());
    let trial = root.path().join("trials/trial-pi-1");
    let settlement = trial.join("bundle/artifacts/control/settlement.json");
    let mut value = read_json(&settlement);
    value["graded"] = serde_json::json!(true);
    value["failure_kind"] = serde_json::Value::Null;
    value["boundary"] = serde_json::Value::Null;
    write_json(&settlement, &value);
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["coverage_counts"]["valid"], 1, "{report}");
    assert_eq!(report["coverage_counts"]["invalid"], 1, "{report}");
    let pi = report["trials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trial| trial["id"] == "trial-pi-1")
        .unwrap();
    assert_eq!(pi["outcome"]["reason"], "unsealed-grade", "{pi}");
    assert!(pi.get("headline").is_none(), "{pi}");
    assert_eq!(
        report["bundle_verification"]["trial-pi-1"]["state"], "unsealed",
        "{report}"
    );
    // Mutable summaries and receipts for sealed trials have no authority.
    std::fs::write(root.path().join("run-report.json"), b"garbage").unwrap();
    std::fs::write(
        root.path().join("trials/trial-opi-1/receipt.json"),
        b"garbage",
    )
    .unwrap();
    assert_eq!(normalized(root.path()).1, report);

    // An intent alone cannot prove that an Agent ran.
    std::fs::remove_file(trial.join("bundle/settlement.json")).unwrap();
    std::fs::remove_file(settlement).unwrap();
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["coverage_counts"]["attempted"], 1, "{report}");
    assert_eq!(report["coverage_counts"]["invalid"], 0, "{report}");
    assert_eq!(report["coverage_counts"]["unexecuted"], 1, "{report}");
    assert!(!trial.join("bundle/manifest.json").exists());
}

#[test]
fn all_unsealed_attempts_keep_coverage_and_reject_conflicting_contracts() {
    let root = tempfile::tempdir().unwrap();
    unsealed_failure_fixture(root.path());
    let opi = root.path().join("trials/trial-opi-1");
    std::fs::remove_file(opi.join("bundle/manifest.json")).unwrap();
    std::fs::remove_file(opi.join("receipt.json")).unwrap();
    let (code, report) = normalized(root.path());
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["coverage_counts"]["attempted"], 2, "{report}");
    assert_eq!(report["coverage_counts"]["invalid"], 2, "{report}");
    assert_eq!(report["metrics"][0]["denominator"], 0, "{report}");
    assert!(
        report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .all(|trial| trial.get("headline").is_none())
    );

    // Matching manifest labels cannot excuse disagreeing retained contracts.
    let contract = opi.join("bundle/artifacts/control/experiment.json");
    let mut value = read_json(&contract);
    value["benchmark"]["revision"] = serde_json::json!("other");
    write_json(&contract, &value);
    let (code, report) = normalized(root.path());
    assert_eq!(code, 1, "{report}");
    assert_eq!(report["outcome"], "verification-failed", "{report}");
}

#[test]
fn unsealed_failure_canaries_block_publication() {
    let root = tempfile::tempdir().unwrap();
    unsealed_failure_fixture(root.path());
    let path = root
        .path()
        .join("trials/trial-pi-1/bundle/artifacts/control/settlement.json");
    let mut value = read_json(&path);
    value["failure_kind"] = serde_json::json!("FIXTURE-UNSEALED-CANARY");
    write_json(&path, &value);
    let declaration = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(declaration.path(), b"FIXTURE-UNSEALED-CANARY\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--canaries")
        .arg(declaration.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "publication-blocked", "{report}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("FIXTURE-UNSEALED-CANARY"));
}

fn fixture_tree_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    files
}

#[test]
fn unsealed_input_directory_aliases_block_without_changing_evidence() {
    for relative in [
        "trials",
        "trials/trial-pi-1",
        "trials/trial-pi-1/bundle",
        "trials/trial-pi-1/bundle/artifacts/control",
    ] {
        let root = tempfile::tempdir().unwrap();
        unsealed_failure_fixture(root.path());
        let external = tempfile::tempdir().unwrap();
        let original = root.path().join(relative);
        let target = external.path().join("retained");
        let before = fixture_tree_bytes(&original);
        std::fs::rename(&original, &target).unwrap();
        create_directory_alias(&original, &target);
        let summary = std::fs::read(root.path().join("run-report.json")).unwrap();
        let (code, report) = normalized(root.path());
        assert_eq!(code, 1, "{relative}: {report}");
        assert_eq!(report["outcome"], "verification-failed", "{report}");
        assert!(report["trials"].is_null(), "{report}");
        assert_eq!(fixture_tree_bytes(&target), before);
        assert_eq!(
            std::fs::read(root.path().join("run-report.json")).unwrap(),
            summary
        );
    }
}

#[test]
fn unsealed_input_file_symlinks_block_without_changing_evidence() {
    for relative in [
        "run-control.json",
        "trials/trial-pi-1/bundle/intent.json",
        "trials/trial-pi-1/bundle/settlement.json",
        "trials/trial-pi-1/bundle/artifacts/control/experiment.json",
        "trials/trial-pi-1/bundle/artifacts/control/settlement.json",
        "trials/trial-pi-1/receipt.json",
    ] {
        let root = tempfile::tempdir().unwrap();
        unsealed_failure_fixture(root.path());
        let external = tempfile::tempdir().unwrap();
        let path = root.path().join(relative);
        if relative.ends_with("receipt.json") {
            write_json(
                &path,
                &serde_json::json!({
                    "schema": "opi-eval-trial-receipt/1", "id": "trial-pi-1",
                    "subject": "baseline-pi", "task": "fixture-task", "group": "group-hello",
                    "status": "failed", "classification": {"class": "invalid-execution", "reason": "verifier-timeout", "boundary": "grader"}
                }),
            );
        }
        let bytes = std::fs::read(&path).unwrap();
        let target = external.path().join("metadata.json");
        std::fs::rename(&path, &target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &path).unwrap();
        let (code, report) = normalized(root.path());
        assert_eq!(code, 1, "{relative}: {report}");
        assert_eq!(report["outcome"], "verification-failed", "{report}");
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}

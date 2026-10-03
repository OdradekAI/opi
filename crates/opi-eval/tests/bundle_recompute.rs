//! Offline bundle recomputation suite.
//!
//! Every case drives the production `opi-eval` binary: the offline
//! `verify`/`report` commands consume sealed assembled outputs produced by
//! `opi-eval run` and must never start an Agent, call a
//! provider, mutate a sealed bundle, or repair drifted bytes. All offline
//! operations are effect-free by construction: they only read the run root
//! and write outside it (stdout or an explicit output path). This proves
//! the hermetic fixture-grade offline path only: no real executable or
//! provider is claimed; native execution is verified separately by the
//! native-smoke workflow.

#[cfg(unix)]
#[test]
fn verify_rejects_linked_trial_without_mutating_source() {
    let root = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "{stderr}");
    let (code, stdout, stderr) = invoke("verify", &[("--root", root.path().as_os_str())]);
    assert_eq!(code, 0, "{stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["bundles"].as_array().unwrap().len(), 2);
    let saved = root.path().join("original-trial");
    let replaced = root.path().join("trials/trial-opi-1");
    std::fs::rename(&replaced, &saved).unwrap();
    let manifest = std::fs::read(saved.join("bundle/manifest.json")).unwrap();
    std::os::unix::fs::symlink(&saved, &replaced).unwrap();
    let (code, _, stderr) = invoke("verify", &[("--root", root.path().as_os_str())]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("linked trial entry"), "{stderr}");
    let out = root.path().join("derived");
    let missing = root.path().join("missing-material.json");
    let (code, _, stderr) = invoke(
        "regrade",
        &[
            ("--root", root.path().as_os_str()),
            ("--native-material", missing.as_os_str()),
            ("--out", out.as_os_str()),
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("linked trial entry"), "{stderr}");
    assert!(!out.exists());
    assert_eq!(
        std::fs::read(saved.join("bundle/manifest.json")).unwrap(),
        manifest
    );
    assert_eq!(std::fs::read_link(&replaced).unwrap(), saved);
}

#[cfg(windows)]
#[test]
fn verify_rejects_trial_junction_without_following_it() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let trials = root.path().join("trials");
    std::fs::create_dir(&trials).unwrap();
    let marker = outside.path().join("untouched.txt");
    std::fs::write(&marker, "untouched").unwrap();
    let link = trials.join("trial-junction");
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["verify", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("linked trial entry"));
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "untouched");
}

#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(unix)]
fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

/// Run one experiment through the real `opi-eval run` binary.
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
        serde_json::from_str(&stdout).unwrap_or_else(|error| {
            panic!("run stdout is not one JSON report ({error}): {stdout:?} stderr: {stderr:?}")
        })
    };
    (code, report, stderr)
}

/// Invoke one offline subcommand (`verify` or `report`) against a run root.
#[cfg(unix)]
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

/// A sealed bundle directory under one run root: `trials/<id>/bundle`.
#[cfg(unix)]
fn bundle_dir(root: &Path, trial: &str) -> PathBuf {
    root.join("trials").join(trial).join("bundle")
}

/// `P19-D06` planned-coverage binding: the run root carries a
/// `run-control.json` written before any trial effect, so offline
/// recomputation preserves unstarted trials even when no trial seals -
/// and a drifted run-control identity never silently normalizes.
#[cfg(unix)]
#[test]
fn planned_coverage_survives_total_bundle_loss() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");

    // The planned coverage is bound before any effect: the durable
    // run-control artifact names the experiment identity, the manifest
    // digest, and the exact planned trial ids.
    let control_path = root.path().join("run-control.json");
    let control: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&control_path)
            .unwrap_or_else(|error| panic!("run-control must exist: {error}")),
    )
    .unwrap();
    assert_eq!(control["schema"], "opi-eval-run-control/2", "{control}");
    assert_eq!(control["experiment"], "local-paired-hermetic", "{control}");
    assert!(
        control["experiment_contract"].is_object(),
        "the frozen experiment contract binds before effects: {control}"
    );
    assert!(
        control["integrity_record"].is_object(),
        "the frozen integrity record binds before effects: {control}"
    );
    assert_eq!(
        control["manifest_digest"].as_str().map(str::len),
        Some(64),
        "{control}"
    );
    let planned = control["planned_trials"].as_array().unwrap();
    assert_eq!(planned.len(), 2, "{control}");
    let planned_ids: Vec<&str> = planned
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    assert!(planned_ids.contains(&"trial-opi-1"), "{control}");
    assert!(planned_ids.contains(&"trial-pi-1"), "{control}");

    // Total bundle loss: with every trial directory gone, the report
    // still reconstructs the planned coverage - attempted 0, unexecuted
    // 2 - instead of failing with no evidence, and the frozen /2
    // contract restores the declared edge/control pair, the known
    // benchmark identity and a zero valid denominator (P19-AUD-004).
    std::fs::remove_dir_all(root.path().join("trials")).unwrap();
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "report must publish: {stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    assert_eq!(published["outcome"], "published", "{published}");
    assert_eq!(published["run_outcome"], "incomplete", "{published}");
    let counts = &published["coverage_counts"];
    assert!(counts["selected"].is_null(), "{published}");
    assert_eq!(counts["planned"], 2, "{published}");
    assert_eq!(counts["attempted"], 0, "{published}");
    assert_eq!(counts["unexecuted"], 2, "{published}");
    assert_eq!(counts["valid"], 0, "{published}");
    assert_eq!(
        published["metrics"][0]["benchmark"], "terminal-bench/2.1",
        "{published}"
    );
    assert_eq!(published["metrics"][0]["denominator"], 0, "{published}");
    let coverage = published["coverage"].as_array().unwrap();
    assert_eq!(
        coverage.len(),
        1,
        "declared edge pair restored: {published}"
    );
    assert_eq!(coverage[0]["edge"], "edge-1", "{published}");
    assert_eq!(
        coverage[0]["comparability"], "missing-baseline-trial",
        "{published}"
    );
    assert_eq!(
        published["planned_binding"]["source"], "run-control",
        "{published}"
    );
}

/// The binding happens before any effect: a process crash at the first
/// durable point still leaves the run-control artifact, and a tampered
/// run-control identity is rejected instead of normalizing silently.
#[cfg(unix)]
#[test]
fn run_control_binds_before_any_effect_and_rejects_drift() {
    let root = tempfile::tempdir().unwrap();
    let (code, _report, _stderr) =
        run_experiment("local-paired.toml", "crash-after-intent", root.path());
    assert_eq!(code, 70, "the crash behavior must exit 70");
    let control_path = root.path().join("run-control.json");
    assert!(
        control_path.is_file(),
        "planned coverage bound before effects"
    );

    // A tampered run-control identity cannot normalize sealed evidence:
    // a frozen /2 integrity record that drifted from the sealed bytes
    // refuses publication.
    let tampered = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", tampered.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let control: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tampered.path().join("run-control.json")).unwrap(),
    )
    .unwrap();
    let mut drifted = control.clone();
    drifted["integrity_record"] = serde_json::json!({"tampered": true});
    std::fs::write(
        tampered.path().join("run-control.json"),
        serde_json::to_string_pretty(&drifted).unwrap(),
    )
    .unwrap();
    let root_arg = tampered.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 1, "drifted run-control must block: {stderr} {stdout}");
    let blocked: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    assert_eq!(blocked["outcome"], "verification-failed", "{blocked}");
    assert_eq!(
        blocked["failures"][0]["kind"], "run-control-drift",
        "{blocked}"
    );

    // Interruption before any seal (P19-AUD-004): the frozen /2
    // contract restores the declared edge pair, the known benchmark
    // identity and zero-valid coverage from the crash point.
    let interrupted = tempfile::tempdir().unwrap();
    let (code, _report, stderr) = run_experiment(
        "local-paired.toml",
        "crash-after-intent",
        interrupted.path(),
    );
    assert_eq!(code, 70, "the crash behavior must exit 70: {stderr}");
    let root_arg = interrupted.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "zero-seal report must publish: {stderr} {stdout}");
    let zero_seal: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    assert_eq!(zero_seal["outcome"], "published", "{zero_seal}");
    assert_eq!(zero_seal["run_outcome"], "incomplete", "{zero_seal}");
    assert_eq!(
        zero_seal["metrics"][0]["benchmark"], "terminal-bench/2.1",
        "{zero_seal}"
    );
    assert_eq!(zero_seal["metrics"][0]["denominator"], 0, "{zero_seal}");
    let coverage = zero_seal["coverage"].as_array().unwrap();
    assert_eq!(coverage.len(), 1, "{zero_seal}");
    assert_eq!(
        coverage[0]["comparability"],
        if coverage[0]["baseline_trial"]
            .as_str()
            .is_some_and(str::is_empty)
        {
            "missing-baseline-trial"
        } else {
            "missing-candidate-trial"
        },
        "{zero_seal}"
    );
    let counts = &zero_seal["coverage_counts"];
    assert_eq!(counts["planned"], 2, "{zero_seal}");
    assert_eq!(counts["attempted"], 0, "{zero_seal}");
    assert_eq!(counts["unexecuted"], 2, "{zero_seal}");
    assert_eq!(counts["valid"], 0, "{zero_seal}");
}

/// `EVAL-A15`: a sealed artifact byte change is rejected by verification
/// without repair, rehash, or manifest mutation.
#[cfg(unix)]
#[test]
fn covered_byte_mutation_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");

    // Byte verification over the intact run root verifies every sealed bundle.
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "intact verify must confirm: {stderr}");
    let verified: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("verify stdout is not one JSON report: {stdout:?}"));
    assert_eq!(verified["outcome"], "verified", "{verified}");
    assert_eq!(
        verified["bundles"].as_array().map(Vec::len),
        Some(2),
        "both sealed trial bundles must be listed: {verified}"
    );

    // Tamper one covered non-empty artifact byte in the first sealed bundle.
    let tampered = bundle_dir(root.path(), "trial-opi-1").join("artifacts/native/agent-answer.txt");
    let original = std::fs::read(&tampered).unwrap();
    assert!(!original.is_empty(), "the answer artifact carries content");
    let mut mutated = original.clone();
    mutated[0] = mutated[0].wrapping_add(1);
    std::fs::write(&tampered, &mutated).unwrap();
    let manifest_before =
        std::fs::read(bundle_dir(root.path(), "trial-opi-1").join("manifest.json")).unwrap();

    // Verification fails without repair or silent rehash: the manifest and
    // the tampered bytes stay exactly as they are, and the failure is the
    // typed digest mismatch, not an unparsable report.
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(
        code, 1,
        "mutated verify must fail: {stderr} stdout: {stdout}"
    );
    let failed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("verify stdout is not one JSON report: {stdout:?}"));
    assert_eq!(failed["outcome"], "mutation-detected", "{failed}");
    let failures = failed["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1, "{failed}");
    assert_eq!(failures[0]["trial"], "trial-opi-1", "{failed}");
    assert_eq!(failures[0]["kind"], "digest-mismatch", "{failed}");
    assert!(
        failures[0]["artifact"]
            .as_str()
            .unwrap()
            .contains("agent-answer.txt"),
        "{failed}"
    );

    // No repair: the sealed manifest and tampered bytes are byte-identical
    // after the failed verification.
    let manifest_after =
        std::fs::read(bundle_dir(root.path(), "trial-opi-1").join("manifest.json")).unwrap();
    assert_eq!(
        manifest_before, manifest_after,
        "manifest must not be rewritten"
    );
    assert_eq!(
        std::fs::read(&tampered).unwrap(),
        mutated,
        "tampered artifact must not be repaired"
    );
}

/// `EVAL-BND-001` closure: an unmanifested file added to a sealed bundle's
/// artifact tree, a corrupted durable intent sidecar, and a deleted
/// expected-output artifact each fail re-verification with the typed
/// reason, without repair or rewrite.
#[cfg(unix)]
#[test]
fn bnd001_retained_byte_closure_rejects_drift() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let root_arg = root.path().canonicalize().unwrap();
    let bundle = bundle_dir(root.path(), "trial-opi-1");

    // An unmanifested rogue file inside the sealed artifact tree.
    let rogue = bundle.join("artifacts/native/rogue.txt");
    std::fs::write(&rogue, b"rogue\n").unwrap();
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 1, "{stderr} {stdout}");
    let failed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(failed["outcome"], "mutation-detected", "{failed}");
    let failure = failed["failures"][0].as_object().unwrap().clone();
    assert_eq!(failure["trial"], "trial-opi-1", "{failed}");
    assert_eq!(failure["kind"], "unmanifested-file", "{failed}");
    assert_eq!(failure["artifact"], "native/rogue.txt", "{failed}");
    assert_eq!(std::fs::read(&rogue).unwrap(), b"rogue\n");
    std::fs::remove_file(&rogue).unwrap();
    let (code, _, _) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "removing the rogue file restores verification");

    // A corrupted durable intent sidecar diverges from the manifest even
    // though the manifest bytes are untouched.
    let manifest_before = std::fs::read(bundle.join("manifest.json")).unwrap();
    std::fs::write(bundle.join("intent.json"), b"not json").unwrap();
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 1, "{stderr} {stdout}");
    let failed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(failed["failures"][0]["kind"], "sidecar-drift", "{failed}");
    assert_eq!(failed["failures"][0]["artifact"], "intent", "{failed}");
    assert_eq!(
        std::fs::read(bundle.join("manifest.json")).unwrap(),
        manifest_before,
        "no repair or rewrite"
    );
    // Restore the durable sidecar so the next phase isolates one drift.
    let intent_bytes = {
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_before).unwrap();
        serde_json::to_vec(&manifest["intent"]).unwrap()
    };
    std::fs::write(bundle.join("intent.json"), &intent_bytes).unwrap();
    let (code, _, _) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "the restored sidecar matches the manifest");

    // A missing sealed expected output fails as a missing artifact.
    let expected = bundle.join("artifacts/normalized/expected-output");
    assert!(expected.is_file(), "the sealed expected output exists");
    std::fs::remove_file(&expected).unwrap();
    let (code, stdout, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 1, "{stderr} {stdout}");
    let failed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        failed["failures"][0]["kind"], "missing-artifact",
        "{failed}"
    );
    assert_eq!(
        failed["failures"][0]["artifact"], "normalized/expected-output",
        "{failed}"
    );
}

/// `EVAL-BND`-adjacent provenance: the sealed manifest names the producer of
/// every retained byte - agent-executed artifacts under `agent-<product>`,
/// verifier streams and the native grader report under the pinned grader
/// identity - and the offline headline selects the grader-sourced native
/// report .
#[cfg(unix)]
#[test]
fn sealed_manifest_attributes_artifacts_to_their_producers() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");

    for (trial, product) in [("trial-opi-1", "agent-opi"), ("trial-pi-1", "agent-pi")] {
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_dir(root.path(), trial).join("manifest.json")).unwrap(),
        )
        .unwrap();
        let entries = manifest["entries"].as_object().unwrap();
        // Agent-executed evidence carries the agent source.
        for key in [
            "native/agent-stdout.log",
            "native/agent-stderr.log",
            "native/agent-answer.txt",
        ] {
            assert_eq!(entries[key]["source"], product, "{trial} {key}");
        }
        // Verifier streams and the imported native report carry the pinned
        // grader identity, never the agent's.
        let grader = "grader-harbor-v0.22.0-fixture";
        for key in [
            "native/verifier-stdout.log",
            "native/verifier-stderr.log",
            "native/native/ctrf-report",
        ] {
            assert!(entries.contains_key(key), "{trial} {key}: {manifest}");
            assert_eq!(entries[key]["source"], grader, "{trial} {key}");
        }
    }

    // The offline headline derives from the grader-sourced native report.
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("report", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    for trial in published["trials"].as_array().unwrap() {
        assert_eq!(
            trial["headline"]["native_source"]["artifact"], "native/native/ctrf-report",
            "{trial}"
        );
        assert_eq!(
            trial["headline"]["native_source"]["digest"]
                .as_str()
                .unwrap()
                .len(),
            64,
            "{trial}"
        );
    }
}

/// Deterministic content digest of a directory tree: sorted relative
/// paths plus file bytes, hashed in order. Used to prove offline
/// operations never mutate the run root.
#[cfg(unix)]
fn tree_digest(root: &Path) -> String {
    use std::collections::BTreeMap;
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    collect_files(root, root, &mut files);
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    for (path, bytes) in &files {
        hasher.update(path.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(unix)]
fn collect_files(root: &Path, dir: &Path, files: &mut std::collections::BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).expect("readable run root") {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            files.insert(relative, std::fs::read(&path).expect("readable file"));
        }
    }
}

/// `EVAL-A17`: regrade, recompute, and render run repeatedly over the same
/// sealed bundle. No Agent/provider starts, the bundle is unchanged, and
/// normalized outputs are byte-stable for the same tool identities.
/// `EVAL-OUT-004`: a new real execution never reuses the sealed trial
/// identities, while the report stays reproducible offline.
#[cfg(unix)]
#[test]
fn repeated_offline_operations_are_stable() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let before = tree_digest(root.path());
    let root_arg = root.path().canonicalize().unwrap();

    // Repeated regrade: identical stdout bytes every time.
    let (c1, regrade1, stderr) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(c1, 0, "{stderr}");
    let (c2, regrade2, _) = invoke("verify", &[("--root", root_arg.as_os_str())]);
    assert_eq!(c2, 0);
    assert_eq!(regrade1, regrade2, "regrade output must be byte-stable");

    // Repeated recompute/render: identical stdout and --out bytes. The
    // outputs are written OUTSIDE the run root: offline operations leave
    // the sealed assembled outputs untouched.
    let outputs = tempfile::tempdir().unwrap();
    let out1 = outputs.path().join("report-1.json");
    let out2 = outputs.path().join("report-2.json");
    let (c3, render1, stderr) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--out", out1.as_os_str()),
        ],
    );
    assert_eq!(c3, 0, "{stderr} {render1}");
    let (c4, render2, _) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--out", out2.as_os_str()),
        ],
    );
    assert_eq!(c4, 0);
    assert_eq!(render1, render2, "report stdout must be byte-stable");
    let normalized: serde_json::Value = serde_json::from_str(&render1).unwrap();
    assert_eq!(normalized["schema"], "opi-eval-normalized-report/2");
    assert_eq!(normalized["reporter_version"], "opi-eval-reporter/2");
    assert_eq!(
        normalized["metrics"][1]["observation_coverage"]["unknown"],
        2
    );
    let written1 = std::fs::read(&out1).unwrap();
    let written2 = std::fs::read(&out2).unwrap();
    assert_eq!(
        written1, written2,
        "report output files must be byte-stable"
    );
    assert_eq!(
        String::from_utf8_lossy(&written1).trim(),
        render1,
        "saved evidence must equal the stdout claim"
    );

    // A new real execution never reuses sealed trial identities
    // (`EVAL-OUT-004`): the same run root is refused outright.
    let (code, _, refusal) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(code, 2, "identity reuse must be refused: {refusal}");
    assert!(refusal.contains("already"), "typed refusal: {refusal}");

    // Effect-free proof: the run root (bundles, receipts, run report) is
    // byte-identical after every offline operation; report outputs were
    // written outside it.
    let after = tree_digest(root.path());
    assert_eq!(
        before, after,
        "offline operations must not mutate the run root"
    );

    // The report still reproduces after the refused execution attempt.
    let (c5, render3, _) = invoke(
        "report",
        &[
            ("--root", root_arg.as_os_str()),
            ("--out", outputs.path().join("report-3.json").as_os_str()),
        ],
    );
    assert_eq!(c5, 0);
    assert_eq!(render3, render1, "report must stay reproducible offline");
}

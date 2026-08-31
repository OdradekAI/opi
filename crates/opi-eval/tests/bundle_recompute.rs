//! Offline bundle recomputation suite (task 18.13).
//!
//! Every case drives the production `opi-eval` binary: the offline
//! `regrade`/`report` commands consume sealed assembled outputs produced by
//! `opi-eval run` (task 18.12) and must never start an Agent, call a
//! provider, mutate a sealed bundle, or repair drifted bytes. All offline
//! operations are effect-free by construction: they only read the run root
//! and write outside it (stdout or an explicit output path). This proves
//! the hermetic fixture-grade offline path only: no real executable or
//! provider is claimed (task 18.15 owns the native rerun).

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

/// Invoke one offline subcommand (`regrade` or `report`) against a run root.
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

/// `P18-A15`: a sealed artifact byte change is rejected by verification
/// without repair, rehash, or manifest mutation.
#[cfg(unix)]
#[test]
fn p18_a15_mutation_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");

    // Regrade over the intact run root verifies every sealed bundle.
    let root_arg = root.path().canonicalize().unwrap();
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "intact regrade must verify: {stderr}");
    let verified: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("regrade stdout is not one JSON report: {stdout:?}"));
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
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(
        code, 1,
        "mutated regrade must fail: {stderr} stdout: {stdout}"
    );
    let failed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("regrade stdout is not one JSON report: {stdout:?}"));
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
    // after the failed regrade.
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

/// `P18-BND-001` closure: an unmanifested file added to a sealed bundle's
/// artifact tree, a corrupted durable intent sidecar, and a deleted
/// expected-output artifact each fail re-verification with the typed
/// reason, without repair or rewrite.
#[cfg(unix)]
#[test]
fn p18_bnd001_retained_byte_closure_rejects_drift() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let root_arg = root.path().canonicalize().unwrap();
    let bundle = bundle_dir(root.path(), "trial-opi-1");

    // An unmanifested rogue file inside the sealed artifact tree.
    let rogue = bundle.join("artifacts/native/rogue.txt");
    std::fs::write(&rogue, b"rogue\n").unwrap();
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 1, "{stderr} {stdout}");
    let failed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(failed["outcome"], "mutation-detected", "{failed}");
    let failure = failed["failures"][0].as_object().unwrap().clone();
    assert_eq!(failure["trial"], "trial-opi-1", "{failed}");
    assert_eq!(failure["kind"], "unmanifested-file", "{failed}");
    assert_eq!(failure["artifact"], "native/rogue.txt", "{failed}");
    assert_eq!(std::fs::read(&rogue).unwrap(), b"rogue\n");
    std::fs::remove_file(&rogue).unwrap();
    let (code, _, _) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "removing the rogue file restores verification");

    // A corrupted durable intent sidecar diverges from the manifest even
    // though the manifest bytes are untouched.
    let manifest_before = std::fs::read(bundle.join("manifest.json")).unwrap();
    std::fs::write(bundle.join("intent.json"), b"not json").unwrap();
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
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
    let (code, _, _) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(code, 0, "the restored sidecar matches the manifest");

    // A missing sealed expected output fails as a missing artifact.
    let expected = bundle.join("artifacts/normalized/expected-output");
    assert!(expected.is_file(), "the sealed expected output exists");
    std::fs::remove_file(&expected).unwrap();
    let (code, stdout, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
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

/// `P18-BND`-adjacent provenance: the sealed manifest names the producer of
/// every retained byte - agent-executed artifacts under `agent-<product>`,
/// verifier streams and the native grader report under the pinned grader
/// identity - and the offline headline selects the grader-sourced native
/// report (Phase 18 remediation).
#[cfg(unix)]
#[test]
fn sealed_manifest_attributes_artifacts_to_their_producers() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
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

/// `P18-A17`: regrade, recompute, and render run repeatedly over the same
/// sealed bundle. No Agent/provider starts, the bundle is unchanged, and
/// normalized outputs are byte-stable for the same tool identities.
/// `P18-OUT-004`: a new real execution never reuses the sealed trial
/// identities, while the report stays reproducible offline.
#[cfg(unix)]
#[test]
fn p18_a17_repeated_offline_operations_are_stable() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(code, 0, "seed run must succeed: {stderr} report: {report}");
    let before = tree_digest(root.path());
    let root_arg = root.path().canonicalize().unwrap();

    // Repeated regrade: identical stdout bytes every time.
    let (c1, regrade1, stderr) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
    assert_eq!(c1, 0, "{stderr}");
    let (c2, regrade2, _) = invoke("regrade", &[("--root", root_arg.as_os_str())]);
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
    // (`P18-OUT-004`): the same run root is refused outright.
    let (code, _, refusal) = run_experiment("phase18-local.toml", "happy", root.path());
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

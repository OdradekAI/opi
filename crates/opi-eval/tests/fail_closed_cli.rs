//! Cross-platform fail-closed regressions over the production `opi-eval` binary.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn run_experiment(root: &Path, behavior: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", behavior])
        .output()
        .expect("spawn the opi-eval run command")
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, snapshot: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                snapshot.insert(relative, None);
                visit(root, &path, snapshot);
            } else {
                snapshot.insert(relative, Some(std::fs::read(&path).unwrap()));
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

#[test]
fn run_requires_one_source_and_hermetic_fixtures_before_effects() {
    let dir = tempfile::tempdir().unwrap();
    let config = fixtures_dir().join("experiment/local-paired.toml");
    for case in ["neither", "both", "missing-fixtures"] {
        let root = dir.path().join(case);
        let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
        command.args(["run", "--root"]).arg(&root);
        if case != "neither" {
            command.arg("--config").arg(&config);
        }
        if case == "both" {
            command
                .arg("--plan")
                .arg(dir.path().join("missing-plan.json"));
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{case}");
        assert!(!root.exists(), "{case}: no run effects");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("panicked"), "{stderr}");
        if case == "missing-fixtures" {
            assert!(
                stderr.contains("--fixtures is required for hermetic execution"),
                "{stderr}"
            );
        }
    }
}

#[test]
fn unreadable_verify_root_fails_closed() {
    let parent = tempfile::tempdir().unwrap();
    let missing_root = parent.path().canonicalize().unwrap().join("missing-run");
    assert!(!missing_root.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("verify")
        .arg("--root")
        .arg(&missing_root)
        .output()
        .expect("spawn the opi-eval verify command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "a missing run root must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "no report may be published: {stdout}"
    );
    assert!(stderr.contains("cannot read run root"), "{stderr}");
}

#[test]
fn reused_trial_identity_is_rejected_before_staging() {
    let root = tempfile::tempdir().unwrap();
    let crashed = run_experiment(root.path(), "crash-after-intent");
    assert_eq!(
        crashed.status.code(),
        Some(70),
        "the seed run must stop after durable intent: stdout={} stderr={}",
        String::from_utf8_lossy(&crashed.stdout),
        String::from_utf8_lossy(&crashed.stderr)
    );

    let trial_root = root.path().join("trials/trial-opi-1");
    assert!(trial_root.join("bundle/intent.json").is_file());
    std::fs::write(trial_root.join("bench.toml"), b"sentinel-opi\n").unwrap();
    let before = snapshot_tree(&trial_root);

    let reused = run_experiment(root.path(), "happy");
    let stdout = String::from_utf8_lossy(&reused.stdout);
    let stderr = String::from_utf8_lossy(&reused.stderr);
    assert_eq!(
        reused.status.code(),
        Some(2),
        "identity reuse must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stderr.contains("run root already contains execution evidence"),
        "{stderr}"
    );
    assert_eq!(
        snapshot_tree(&trial_root),
        before,
        "a refused retry must preserve every existing path and byte"
    );
}

/// Spawns `opi-eval doctor` with an environment override.
fn run_doctor(config: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
    command.arg("doctor").arg("--config").arg(config);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("spawn the opi-eval doctor command")
}

/// Copies the pinned paired fixture into a temp directory with the
/// storage cache redirected to an absolute sibling directory.
fn staged_config(dir: &Path, cache: &str) -> PathBuf {
    let source =
        std::fs::read_to_string(fixtures_dir().join("experiment/local-paired.toml")).unwrap();
    let redirected = source
        .lines()
        .map(|line| {
            if line.strip_prefix("cache = ").is_some() {
                format!("cache = {cache:?}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let config = dir.join("experiment.toml");
    std::fs::write(&config, redirected).unwrap();
    config
}

#[test]
fn doctor_reports_observations_without_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let config = staged_config(dir.path(), &cache.to_string_lossy());

    // A closed loopback endpoint is never a reachable engine, on hosts
    // with or without a Docker client: the observation must carry the
    // failure reason instead of guessing.
    let output = run_doctor(&config, &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "diagnosis must complete without starting a model: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["schema"], "opi-eval-doctor/1");
    let observations = report["observations"].as_array().unwrap();
    let by_id = |id: &str| {
        observations
            .iter()
            .find(|observation| observation["id"] == id)
            .unwrap_or_else(|| panic!("missing observation {id}: {stdout}"))
            .clone()
    };
    assert_eq!(by_id("linux-engine")["status"], "unreachable");
    assert!(
        !by_id("linux-engine")["reason"].as_str().unwrap().is_empty(),
        "an unreachable engine must carry a reason: {stdout}"
    );
    assert_eq!(by_id("architecture")["status"], "ok");
    assert_eq!(by_id("architecture")["detail"], "x86_64");
    assert_eq!(by_id("cpu")["status"], "ok");
    assert!(by_id("cpu")["count"].as_u64().unwrap() >= 1);
    assert_eq!(by_id("memory")["status"], "ok");
    assert!(by_id("memory")["total_bytes"].as_u64().unwrap() > 0);
    // Without a reachable engine the GPU observation is unknown with a
    // reason, never silently absent.
    assert_eq!(by_id("gpu")["status"], "unknown");
    assert!(
        by_id("gpu")["reason"].as_str().unwrap().contains("engine"),
        "gpu unknown reason must name the engine: {stdout}"
    );
    assert_eq!(by_id("disk")["status"], "ok");
    assert!(by_id("disk")["available_bytes"].as_u64().is_some());
    // The empty cache is reported as missing material, not a guess.
    let static_lock = by_id("material:static-lock");
    assert_eq!(static_lock["status"], "missing");
    assert_eq!(by_id("material:agent:opi")["status"], "missing");
    assert_eq!(by_id("material:agent:pi")["status"], "missing");
    // No model or provider call happened: the report holds only
    // observations.
    assert!(report.get("provider").is_none());
}

#[test]
fn doctor_rejects_an_invalid_config_without_observing() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("broken.toml");
    std::fs::write(&config, b"not a toml document\n").unwrap();
    let output = run_doctor(&config, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    assert!(stdout.trim().is_empty(), "no report may print: {stdout}");
    assert!(!stderr.trim().is_empty(), "a diagnostic must print");
}

/// P19-AC-011: a populated dummy credential environment does not activate
/// live behavior. A complete fixture run with dummy credentials exported
/// stays the deterministic hermetic contract - completed, sealed, scripted
/// rewards - and no credential byte reaches any durable artifact.
#[test]
fn dummy_credentials_do_not_activate_live_behavior() {
    let guard = tempfile::tempdir().unwrap();
    let root = guard.path().join("run-root");
    fs::create_dir(&root).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "happy"])
        .env("OPENAI_API_KEY", "dummy-credential")
        .env("ANTHROPIC_API_KEY", "dummy-credential")
        .env("OPI_EVAL_SCRIPTED_KEY", "dummy-credential")
        .output()
        .expect("spawn the opi-eval run command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The deterministic hermetic contract, unchanged by the credential
    // environment: both trials complete, seal, and grade through the
    // scripted fixture path.
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("run stdout is one JSON report");
    assert_eq!(report["outcome"], "completed", "{report}");
    let trials = report["trials"].as_array().unwrap();
    assert_eq!(trials.len(), 2);
    for trial in trials {
        assert_eq!(trial["status"], "sealed", "{trial}");
        assert_eq!(
            trial["verifier"]["reward"], "unknown:native-reward-pending-18-15-smoke",
            "a live provider never graded this trial: {trial}"
        );
    }

    // No credential material crosses into the durable evidence.
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                stack.push(entry.unwrap().path());
            }
        } else {
            let bytes = fs::read(&path).unwrap();
            assert!(
                !bytes
                    .windows(b"dummy-credential".len())
                    .any(|w| w == b"dummy-credential"),
                "credential material leaked into {}",
                path.display()
            );
        }
    }
}

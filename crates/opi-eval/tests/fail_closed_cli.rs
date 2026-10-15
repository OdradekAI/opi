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
    assert_eq!(
        by_id("architecture")["status"],
        if cfg!(target_arch = "x86_64") {
            "ok"
        } else {
            "unsupported"
        }
    );
    assert_eq!(by_id("architecture")["detail"], std::env::consts::ARCH);
    if cfg!(target_os = "linux") {
        assert_eq!(by_id("cpu")["status"], "ok");
        assert!(by_id("cpu")["count"].as_u64().unwrap() >= 1);
        assert_eq!(by_id("memory")["status"], "ok");
        assert!(by_id("memory")["total_bytes"].as_u64().unwrap() > 0);
    } else {
        // The /proc-backed observers are honestly unknown off Linux.
        assert_eq!(by_id("cpu")["status"], "unknown");
        assert_eq!(by_id("memory")["status"], "unknown");
    }
    // The disk observer shells out to GNU `df -B1 -P`: Linux ships it
    // natively and Windows CI hosts expose Git's GNU coreutils `df`, while
    // BSD df (macOS) has no `-B` and honestly reports unknown.
    let expected_disk = if cfg!(any(target_os = "linux", target_os = "windows")) {
        "ok"
    } else {
        "unknown"
    };
    assert_eq!(by_id("disk")["status"], expected_disk);
    if cfg!(any(target_os = "linux", target_os = "windows")) {
        assert!(by_id("disk")["available_bytes"].as_u64().is_some());
    }
    // Without a reachable engine the GPU observation is unknown with a
    // reason, never silently absent.
    assert_eq!(by_id("gpu")["status"], "unknown");
    assert!(
        by_id("gpu")["reason"].as_str().unwrap().contains("engine"),
        "gpu unknown reason must name the engine: {stdout}"
    );
    // The empty cache is reported as missing material, not a guess.
    let static_lock = by_id("material:static-lock");
    assert_eq!(static_lock["status"], "missing");
    assert_eq!(by_id("material:agent:opi")["status"], "missing");
    assert_eq!(by_id("material:agent:pi")["status"], "missing");
    // No model or provider call happened: the report holds only
    // observations.
    assert!(report.get("provider").is_none());
}

// The staged peer is a shell script on PATH, so the interception is a
// Unix contract; on Windows the probe honestly reports the named-pipe
// engine as unreachable instead.
#[cfg(unix)]
#[test]
fn doctor_bounds_stalled_and_overflowing_observers() {
    // P19-AUD-003: every selected observer is supervised with a finite
    // bound. A stalled or overflowing selected Docker client yields a
    // truthful bounded report inside the finite observation budget
    // instead of an unbounded wait, and no owned observer child survives.
    for case in ["stall", "overflow"] {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let config = staged_config(dir.path(), &cache.to_string_lossy());
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let docker = bin.join("docker");
        let script = if case == "stall" {
            "#!/bin/sh\nexec /bin/sleep 60\n"
        } else {
            "#!/bin/sh\nhead -c 131072 /dev/zero\n"
        };
        std::fs::write(&docker, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = format!("{}:/usr/bin:/bin", bin.display());
        let started = std::time::Instant::now();
        let output = run_doctor(&config, &[("PATH", &path)]);
        let elapsed = started.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: bounded diagnosis must complete: stdout={stdout} stderr={stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(15),
            "{case}: bounded diagnosis took {elapsed:?}"
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let observations = report["observations"].as_array().unwrap();
        let engine = observations
            .iter()
            .find(|observation| observation["id"] == "linux-engine")
            .unwrap();
        assert_eq!(engine["status"], "unreachable", "{case}: {stdout}");
        let reason = engine["reason"].as_str().unwrap();
        assert!(!reason.is_empty(), "{case}: {stdout}");
        let expected = if case == "stall" {
            "timed out"
        } else {
            "capture ceiling"
        };
        assert!(reason.contains(expected), "{case}: {reason}");
    }
}

#[test]
fn doctor_reports_every_selected_preparation_input() {
    // P19-AUD-010: doctor derives its material observations from the same
    // selected benchmark/runner requirements `prepare` verifies. A cache
    // holding only the static lock and selected agents still reports
    // every remaining selected requirement with its expected location,
    // a pi-only selection requires no opi input, and an empty cache
    // reports the unknownable categories with reasons.
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(cache.join("agents")).unwrap();
    let lock_source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("external-locks/static/linux-x86_64.json");
    std::fs::create_dir_all(cache.join("pins/static")).unwrap();
    std::fs::copy(&lock_source, cache.join("pins/static/linux-x86_64.json")).unwrap();
    std::fs::write(
        cache.join("agents/pi"),
        b"fixture agent bytes, never executed",
    )
    .unwrap();
    std::fs::write(
        cache.join("agents/opi"),
        b"fixture agent bytes, never executed",
    )
    .unwrap();
    let config = staged_config(dir.path(), &cache.to_string_lossy());
    let output = run_doctor(&config, &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let material: Vec<&serde_json::Value> = report["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|observation| {
            observation["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("material:"))
        })
        .collect();
    let ids: Vec<&str> = material
        .iter()
        .map(|observation| observation["id"].as_str().unwrap())
        .collect();
    for category in [
        "corpora:",
        "profiles:",
        "tools:uv",
        "runners:",
        "integrations:",
        "scripted-provider",
        "native-capture-helper",
    ] {
        assert!(
            ids.iter().any(|id| id.contains(category)),
            "missing {category} observation: {ids:?}"
        );
    }
    for observation in &material {
        assert!(
            observation["detail"]
                .as_str()
                .is_some_and(|detail| !detail.is_empty()),
            "every selected input names its expected location: {observation}"
        );
    }
    let by_id = |needle: &str| {
        material
            .iter()
            .find(|observation| observation["id"].as_str().unwrap().contains(needle))
            .unwrap_or_else(|| panic!("missing {needle}: {ids:?}"))
    };
    assert_eq!(by_id("profiles:")["status"], "missing");
    assert_eq!(by_id("corpora:")["status"], "missing");
    assert_eq!(by_id("tools:uv")["status"], "missing");
    assert_eq!(by_id("runners:harbor")["status"], "missing");
    assert_eq!(by_id("native-capture-helper")["status"], "missing");
    assert_eq!(by_id("agent:opi")["status"], "present");
    assert_eq!(by_id("agent:pi")["status"], "present");

    // Pi-only selection: the unselected product's input is not required.
    let pi_only = fixtures_dir().join("experiment/pi-repetitions.toml");
    let pi_config = dir.path().join("pi-only.toml");
    let text = std::fs::read_to_string(&pi_only)
        .unwrap()
        .lines()
        .map(|line| {
            if line.strip_prefix("cache = ").is_some() {
                format!("cache = {:?}", cache.to_string_lossy())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&pi_config, text).unwrap();
    let output = run_doctor(&pi_config, &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let ids: Vec<&str> = report["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|observation| observation["id"].as_str())
        .collect();
    assert!(ids.contains(&"material:agent:pi"), "{ids:?}");
    assert!(!ids.contains(&"material:agent:opi"), "{ids:?}");
    assert!(
        ids.iter().any(|id| id.contains("runners:harbor")),
        "{ids:?}"
    );

    // An empty cache keeps every category visible with unknown/missing
    // reasons instead of guessing.
    let empty = tempfile::tempdir().unwrap();
    let empty_cache = empty.path().join("cache");
    std::fs::create_dir_all(&empty_cache).unwrap();
    let config = staged_config(empty.path(), &empty_cache.to_string_lossy());
    let output = run_doctor(&config, &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let by_id = |needle: &str| {
        report["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|observation| {
                observation["id"]
                    .as_str()
                    .is_some_and(|id| id.contains(needle))
            })
            .unwrap_or_else(|| panic!("missing {needle}: {stdout}"))
    };
    assert_eq!(by_id("material:static-lock")["status"], "missing");
    assert_eq!(by_id("tools:uv")["status"], "unknown");
    assert!(
        by_id("tools:uv")["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "{stdout}"
    );
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

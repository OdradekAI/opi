//! Assembled hermetic run-path suite.
//!
//! Every case drives the production `opi-eval run` binary end to end:
//! the CLI resolves the fixture experiment document, the runner reserves
//! durable intent, dispatches a runtime-generated deterministic helper as
//! the resolved executable (never an ambient PATH lookup), settles, grades
//! through the native verifier helper, seals the trial bundle, projects the
//! pre-seal trajectory, and assembles comparison coverage. This proves the
//! hermetic fixture-grade run path only: no real Opi/pi executable, real
//! provider, official task package, or network is claimed. Native execution
//! is verified separately by the native-smoke workflow. No paid provider, credential, or user-global resource
//! is touched (`EVAL-AGT-002`, `EVAL-AGT-006`).

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

fn original_files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    if root.exists() {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.extend(original_files(&path));
            } else {
                result.insert(path.clone(), std::fs::read(path).unwrap());
            }
        }
    }
    result
}

#[test]
fn replacement_fresh_resolved_pairs_preserve_original_scope() {
    for selected in [false, true] {
        let room = tempfile::tempdir().unwrap();
        let config = room.path().join("original.toml");
        let mut source: toml::Value = toml::from_str(
            &std::fs::read_to_string(fixtures_dir().join("experiment/local-paired.toml")).unwrap(),
        )
        .unwrap();
        if selected {
            source.as_table_mut().unwrap().remove("trials");
            source.as_table_mut().unwrap().insert("selection".into(), toml::from_str::<toml::Value>(
                "scope = 'subset'\ntask_ids = ['fixture-task']\nreason = 'replacement regression'\nrepetitions = 2\n",
            ).unwrap());
        } else {
            let mut other = source["trials"].as_array().unwrap().clone();
            for trial in &mut other {
                trial["id"] =
                    toml::Value::String(format!("{}-other", trial["id"].as_str().unwrap()));
                trial["group"] = toml::Value::String("unrelated-group".into());
            }
            source["trials"].as_array_mut().unwrap().extend(other);
        }
        std::fs::write(&config, toml::to_string(&source).unwrap()).unwrap();
        let frozen = room.path().join("plan.json");
        let input = if selected {
            let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["plan", "--config"])
                .arg(&config)
                .arg("--fixtures")
                .arg(fixtures_dir())
                .arg("--out")
                .arg(&frozen)
                .env("DOCKER_HOST", "tcp://127.0.0.1:1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            &frozen
        } else {
            &config
        };
        let target = if selected {
            "trial-baseline-pi-fixture-task-r1"
        } else {
            "trial-pi-1"
        };
        let invoke = |root: &Path, replace: bool| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
            command
                .arg("run")
                .arg(if selected { "--plan" } else { "--config" })
                .arg(input)
                .arg("--fixtures")
                .arg(fixtures_dir())
                .arg("--root")
                .arg(root);
            if replace {
                command.args(["--replacement-for", target]);
            }
            command.output().unwrap()
        };
        let original = room.path().join("original-run");
        let output = invoke(&original, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let before = original_files(&original);
        let input_before = std::fs::read(input).unwrap();
        let original_control: serde_json::Value =
            serde_json::from_slice(&before[&original.join("run-control.json")]).unwrap();
        let planned = original_control["planned_trials"].as_array().unwrap();
        let group = planned.iter().find(|row| row["id"] == target).unwrap()["group"]
            .as_str()
            .unwrap();
        let mut attempts = Vec::new();
        for index in 0..2 {
            let root = room.path().join(format!("replacement-{index}"));
            let output = invoke(&root, true);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let control: serde_json::Value =
                serde_json::from_slice(&std::fs::read(root.join("run-control.json")).unwrap())
                    .unwrap();
            let token = control["experiment"]
                .as_str()
                .unwrap()
                .strip_prefix("local-paired-hermetic.r")
                .unwrap();
            assert_eq!(token.len(), 64);
            assert_ne!(
                control["manifest_digest"],
                original_control["manifest_digest"]
            );
            assert_eq!(control["selection"], original_control["selection"]);
            assert_eq!(control["plan_digest"], original_control["plan_digest"]);
            let rows = control["planned_trials"].as_array().unwrap();
            assert_eq!(rows.len(), planned.len());
            for old in planned {
                let old_id = old["id"].as_str().unwrap();
                // Every re-executed trial carries a fresh identity,
                // including retained unrelated groups (CTRL-006).
                assert!(
                    !rows.iter().any(|row| row["id"] == old["id"]),
                    "new execution reused prior trial identity {} in group {}",
                    old["id"],
                    old["group"]
                );
                let changed = old["group"] == group;
                let new_id = format!("{old_id}.r{token}");
                let row = rows.iter().find(|row| row["id"] == new_id).unwrap();
                assert_eq!(row["subject"], old["subject"]);
                assert_eq!(row["task"], old["task"]);
                assert_eq!(
                    row["group"],
                    if changed {
                        serde_json::json!(format!("{group}.r{token}"))
                    } else {
                        old["group"].clone()
                    }
                );
                let sealed = |root: &Path, id: &str| -> serde_json::Value {
                    serde_json::from_slice(
                        &std::fs::read(
                            root.join("trials")
                                .join(id)
                                .join("bundle/artifacts/control/experiment.json"),
                        )
                        .unwrap(),
                    )
                    .unwrap()
                };
                let old_sealed = sealed(&original, old_id);
                let new_sealed = sealed(&root, &new_id);
                for field in [
                    "connections",
                    "connection",
                    "model_controls",
                    "benchmark",
                    "execution",
                    "selection",
                ] {
                    assert_eq!(new_sealed[field], old_sealed[field], "{field}");
                }
            }
            let replaced_before = original_files(&root);
            assert_eq!(invoke(&root, true).status.code(), Some(2));
            assert_eq!(original_files(&root), replaced_before);
            attempts.push(control);
        }
        assert_ne!(attempts[0]["experiment"], attempts[1]["experiment"]);
        assert_ne!(attempts[0]["planned_trials"], attempts[1]["planned_trials"]);
        assert_eq!(original_files(&original), before);
        assert_eq!(std::fs::read(input).unwrap(), input_before);
        if selected {
            let mut plan: serde_json::Value = serde_json::from_slice(&input_before).unwrap();
            plan["trials"][0]["id"] = serde_json::json!("tampered");
            std::fs::write(input, serde_json::to_vec(&plan).unwrap()).unwrap();
            let rejected = room.path().join("rejected");
            assert_eq!(invoke(&rejected, true).status.code(), Some(2));
            assert!(!rejected.exists());
        }
    }
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

/// The full hermetic paired run: durable intent, agent dispatch, settlement,
/// grade, seal, projection, and one comparable pair - all transitions
/// executed and all receipts persisted.
#[test]
fn hermetic_paired_group_runs_end_to_end() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(
        code, 0,
        "hermetic paired run must succeed: {stderr} report: {report}"
    );
    assert_eq!(report["schema"], "opi-eval-run-report/1");
    assert_eq!(report["outcome"], "completed");

    // Both declared trials ran, settled, and sealed under fresh identities.
    let trials = report["trials"].as_array().unwrap();
    assert_eq!(trials.len(), 2);
    for trial in trials {
        assert_eq!(trial["status"], "sealed", "trial: {trial}");
        let id = trial["id"].as_str().unwrap();
        assert_eq!(trial["bundle_identity"].as_str().unwrap().len(), 64);
        assert_eq!(trial["pre_seal_digest"].as_str().unwrap().len(), 64);
        assert!(trial["seal_result"].is_object(), "trial: {trial}");
        // Every authority transition executed: no refusal, no skip.
        for transition in [
            "agent_dispatch",
            "settle",
            "grade_dispatch",
            "seal",
            "report",
        ] {
            assert_eq!(
                trial["authority"][transition], "executed",
                "trial {id} transition {transition}: {trial}"
            );
        }
        // The durable receipts and bundle exist under the trial root.
        let trial_root = root.path().join("trials").join(id);
        assert!(trial_root.join("receipt.json").is_file());
        assert!(trial_root.join("bundle").join("manifest.json").is_file());
        // The sealed closure covers the complete retained set (EVAL-BND-001):
        // the control evidence, the trajectory, the normalized expected
        // output, and the native execution streams are all manifest
        // entries reserved by the durable intent.
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(trial_root.join("bundle/manifest.json")).unwrap(),
        )
        .unwrap();
        let intent: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(trial_root.join("bundle/intent.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["intent"], intent,
            "the manifest and the durable sidecar agree: {trial}"
        );
        for key in [
            "control/experiment.json",
            "control/integrity.json",
            "evidence/trajectory.json",
            "native/agent-stdout.log",
            "native/agent-stderr.log",
            "native/agent-answer.txt",
            "native/authority-ledger.json",
            "normalized/expected-output",
        ] {
            assert!(
                manifest["entries"][key].is_object(),
                "trial {id} must seal {key}: {manifest}"
            );
            assert!(
                intent["artifacts"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!(key)),
                "trial {id} must reserve {key}: {intent}"
            );
        }
        assert_eq!(
            intent["expected_output"], "normalized/expected-output",
            "{trial}"
        );
        // The final workspace capture retains the agent's answer.
        assert!(trial_root.join("workspace").join("answer.txt").is_file());
    }

    // The declared edge assembled into exactly one comparable pair.
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0]["edge"], "edge-1");
    assert_eq!(pairs[0]["comparability"], "comparable");
    assert_eq!(pairs[0]["baseline_trial"], "trial-pi-1");
    assert_eq!(pairs[0]["candidate_trial"], "trial-opi-1");
}

/// `EVAL-A05`: through the production `run` path, each selected adapter
/// rejects an unknown required schema, malformed stream, missing
/// authoritative terminal event, and excessive output with the owning
/// typed failure visible and no fallback Agent or parser started (the
/// grade dispatch is mechanically refused; the verifier never runs).
#[test]
fn explicit_trial_count_admits_declared_max_trials() {
    // P19-AUD-002: explicitly declared hermetic trials admit the declared
    // max_trials on the shared run path — an over-cap request refuses
    // before the run root exists, equality executes exactly the declared
    // pair, and omission keeps the current unbounded behavior.
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::read_to_string(fixtures_dir().join("experiment").join("local-paired.toml"))
        .unwrap();
    for (name, execution) in [
        (
            "over",
            "\n[execution]\nmode = \"hermetic\"\nmax_trials = 1\n",
        ),
        (
            "equal",
            "\n[execution]\nmode = \"hermetic\"\nmax_trials = 2\n",
        ),
        ("omitted", "\n[execution]\nmode = \"hermetic\"\n"),
    ] {
        let config = dir.path().join(format!("config-{name}.toml"));
        std::fs::write(&config, base.clone() + execution).unwrap();
        let root = dir.path().join(format!("run-{name}"));
        std::fs::create_dir_all(&root).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("run")
            .arg("--config")
            .arg(&config)
            .arg("--root")
            .arg(root.canonicalize().unwrap())
            .arg("--fixtures")
            .arg(fixtures_dir().canonicalize().unwrap())
            .args(["--behavior", "happy"])
            .output()
            .expect("spawn the opi-eval run binary");
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if name == "over" {
            assert_ne!(output.status.code(), Some(0), "{stdout} {stderr}");
            assert!(stderr.contains("max_trials"), "{stderr}");
            assert!(!root.join("run-control.json").exists());
            assert!(!root.join("trials").exists());
        } else {
            assert_eq!(output.status.code(), Some(0), "{name}: {stderr} {stdout}");
            let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let trials = report["trials"].as_array().unwrap();
            assert_eq!(trials.len(), 2, "{name}: {report}");
            for trial in trials {
                assert_eq!(trial["status"], "sealed", "{name}: {trial}");
            }
        }
    }
}

#[test]
fn agent_stream_contract_fails_closed() {
    struct Case {
        behavior: &'static str,
        /// (failure kind, boundary) for the opi subject; None = the pinned
        /// opi contract stays completed (its importer reads trace files,
        /// not the capped stream) with the truncation visible.
        opi: Option<(&'static str, &'static str)>,
        pi: (&'static str, &'static str),
    }
    let cases = [
        Case {
            behavior: "agent-unknown-schema",
            opi: Some(("import-unsupported-schema", "adapter")),
            pi: ("import-unsupported-schema", "adapter"),
        },
        Case {
            behavior: "agent-malformed-stream",
            opi: Some(("import-parse-failure", "adapter")),
            pi: ("import-parse-failure", "adapter"),
        },
        Case {
            behavior: "agent-missing-terminal",
            opi: Some(("import-evidence-missing", "evidence")),
            pi: ("import-evidence-missing", "evidence"),
        },
        Case {
            behavior: "agent-excess-output",
            opi: None,
            pi: ("import-evidence-incomplete", "evidence"),
        },
    ];

    for case in &cases {
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) =
            run_experiment("local-paired.toml", case.behavior, root.path());
        assert_eq!(code, 1, "{}: {stderr}", case.behavior);
        assert_eq!(report["outcome"], "incomplete", "{}", case.behavior);

        let trials = report["trials"].as_array().unwrap();
        assert_eq!(trials.len(), 2, "{}", case.behavior);
        for trial in trials {
            let product = trial["agent"]["product"].as_str().unwrap();
            let expected = if product == "opi" {
                &case.opi
            } else {
                &Some(case.pi)
            };
            let agent = &trial["agent"];
            match expected {
                Some((kind, boundary)) => {
                    assert_eq!(
                        agent["completion"], "failed",
                        "{} {}: {trial}",
                        case.behavior, product
                    );
                    assert_eq!(
                        agent["failure_kind"], *kind,
                        "{} {}: {trial}",
                        case.behavior, product
                    );
                    assert_eq!(
                        agent["boundary"], *boundary,
                        "{} {}: {trial}",
                        case.behavior, product
                    );
                    // No fallback parser or verifier ran: the grade
                    // dispatch is mechanically refused after the failure,
                    // with the owning boundary named in the refusal.
                    assert_eq!(
                        trial["authority"]["grade_dispatch"]
                            .as_str()
                            .unwrap_or_default(),
                        format!("refused:stopped-at-{boundary}"),
                        "{} {}: {trial}",
                        case.behavior,
                        product
                    );
                    assert!(
                        trial["verifier"].is_null(),
                        "{} {}: no verifier may run after an adapter/evidence failure: {trial}",
                        case.behavior,
                        product
                    );
                }
                None => {
                    // opi's pinned contract: the capped stream is retained
                    // visibly; the trace-based completion stays valid.
                    assert_eq!(
                        agent["completion"], "completed",
                        "{} {}: {trial}",
                        case.behavior, product
                    );
                    assert!(
                        agent["stderr_truncated"] == serde_json::Value::Bool(true)
                            || agent["stdout_truncated"] == serde_json::Value::Bool(true),
                        "{} {}: truncation must stay visible: {trial}",
                        case.behavior,
                        product
                    );
                }
            }
            // Exactly one agent dispatch per trial - no fallback agent.
            assert_eq!(
                trial["authority"]["agent_dispatch"], "executed",
                "{}: {trial}",
                case.behavior
            );
        }

        // The pair stays visible as non-comparable with the failing trial.
        let pairs = report["pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 1, "{}", case.behavior);
        let comparability = pairs[0]["comparability"].as_str().unwrap();
        assert!(
            comparability.starts_with("infrastructure-failure:"),
            "{}: {report}",
            case.behavior
        );
    }
}

/// Runs the offline `report` command over one run root.
fn offline_report(root: &Path) -> (i32, serde_json::Value, String) {
    // Report output never lies inside the sealed run root.
    let out = std::env::temp_dir().join(format!(
        "opi-eval-offline-report-{}-{}.json",
        root.file_name().and_then(|n| n.to_str()).unwrap_or("run"),
        std::process::id()
    ));
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--out")
        .arg(&out)
        .output()
        .expect("spawn the opi-eval report command");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("report stdout is not one JSON report ({error}): {stdout:?} stderr: {stderr:?}")
    });
    (output.status.code().unwrap_or(-1), report, stderr)
}

/// `EVAL-A06`: timeout or cancellation racing a bounded process, workspace
/// mutation, and process exit through `run` retains actual partial
/// artifacts and cleanup status, never infers replay safety (no replacement
/// trial or second dispatch), and routes each failure to its owning class:
/// an Agent-owned timeout is a scored Agent outcome under the native
/// grader; a user cancellation stays a boundary failure.
#[test]
fn process_timeout_cancellation_race() {
    // Agent-owned timeout: scored under the native grader. One grade
    // dispatch over the settled workspace, graded outcome retained in the
    // pairing denominator, exactly one agent dispatch.
    {
        let behavior = "agent-timeout";
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) = run_experiment("local-paired.toml", behavior, root.path());
        assert_eq!(
            code, 0,
            "{behavior}: the scored run completes: {stderr} {report}"
        );
        assert_eq!(report["outcome"], "completed", "{behavior}: {report}");
        let trials = report["trials"].as_array().unwrap();
        assert_eq!(trials.len(), 2, "{behavior}: no replacement trials");
        for trial in trials {
            let id = trial["id"].as_str().unwrap();
            assert_eq!(
                trial["agent"]["failure_kind"], "agent-timeout",
                "{behavior}: {trial}"
            );
            assert_eq!(trial["agent"]["boundary"], "agent-process");
            assert_eq!(
                trial["agent"]["exit_state"], "timed-out",
                "{behavior}: {trial}"
            );
            // Actual partial artifacts retained: bounded captures recorded
            // with their truncation state and the process cleanup status.
            assert!(
                trial["agent"].get("stdout_bytes").is_some()
                    && trial["agent"].get("stderr_bytes").is_some(),
                "{behavior}: {trial}"
            );
            let cleanup = trial["agent"]["cleanup"].as_str().unwrap_or_default();
            assert!(
                cleanup.starts_with("tree-terminated")
                    || cleanup.starts_with("tree-termination-failed")
                    || cleanup == "not-required",
                "{behavior}: cleanup status must stay visible: {trial}"
            );
            // The native grader dispatched exactly once and verified.
            assert_eq!(
                trial["authority"]["grade_dispatch"], "executed",
                "{behavior}: {trial}"
            );
            assert_eq!(
                trial["verifier"]["completion"], "verified",
                "{behavior}: {trial}"
            );
            // The raced workspace mutation is retained, not discarded.
            let workspace = root.path().join("trials").join(id).join("workspace");
            assert!(
                workspace.join("answer.txt").is_file(),
                "{behavior}: the mutated workspace must be retained"
            );
            // Replay safety is never inferred: exactly one dispatch.
            assert_eq!(
                trial["authority"]["agent_dispatch"], "executed",
                "{behavior}: {trial}"
            );
        }
        let pairs = report["pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 1, "{behavior}");
        assert_eq!(
            pairs[0]["comparability"], "comparable",
            "{behavior}: {report}"
        );
        // The settled classifications are immutable offline evidence: the
        // scored Agent timeout stays a valid graded zero in the coverage
        // denominator, never reclassified from logs or exit codes.
        let (code, offline, stderr) = offline_report(root.path());
        assert_eq!(code, 0, "{behavior}: {stderr}");
        assert_eq!(offline["run_outcome"], "completed", "{behavior}: {offline}");
        let counts = &offline["coverage_counts"];
        assert_eq!(counts["planned"], 2, "{behavior}: {counts}");
        assert_eq!(counts["attempted"], 2, "{behavior}: {counts}");
        assert_eq!(counts["valid"], 2, "{behavior}: {counts}");
    }

    // User cancellation: a boundary failure. The grade dispatch is
    // mechanically refused and the pair stays visible as infrastructure.
    {
        let behavior = "agent-cancelled";
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) = run_experiment("local-paired.toml", behavior, root.path());
        assert_eq!(code, 1, "{behavior}: {stderr}");
        assert_eq!(report["outcome"], "incomplete", "{behavior}: {report}");
        let trials = report["trials"].as_array().unwrap();
        assert_eq!(trials.len(), 2, "{behavior}: no replacement trials");
        for trial in trials {
            let id = trial["id"].as_str().unwrap();
            assert_eq!(
                trial["agent"]["failure_kind"], "agent-cancelled",
                "{behavior}: {trial}"
            );
            assert_eq!(trial["agent"]["boundary"], "agent-process");
            assert_eq!(
                trial["agent"]["exit_state"], "cancelled",
                "{behavior}: {trial}"
            );
            assert!(
                trial["agent"].get("stdout_bytes").is_some()
                    && trial["agent"].get("stderr_bytes").is_some(),
                "{behavior}: {trial}"
            );
            let workspace = root.path().join("trials").join(id).join("workspace");
            assert!(
                workspace.join("answer.txt").is_file(),
                "{behavior}: the mutated workspace must be retained"
            );
            // Grade never ran after the process failure.
            assert_eq!(
                trial["authority"]["grade_dispatch"], "refused:stopped-at-agent-process",
                "{behavior}: {trial}"
            );
            assert!(trial["verifier"].is_null(), "{behavior}: {trial}");
            assert_eq!(
                trial["authority"]["agent_dispatch"], "executed",
                "{behavior}: {trial}"
            );
        }
        let pairs = report["pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 1, "{behavior}");
        assert!(
            pairs[0]["comparability"]
                .as_str()
                .unwrap()
                .starts_with("infrastructure-failure:")
        );

        // The cancelled settlement is durable and immutable: recovery
        // classifies both trials by their settled state - never
        // not-started or retryable under the same identity - and the
        // offline report keeps them as explicit missing coverage with no
        // fabricated score (P19-AC-010).
        let settlement = root
            .path()
            .join("trials/trial-opi-1/bundle/settlement.json");
        let settlement_before = std::fs::read(&settlement)
            .unwrap_or_else(|error| panic!("{behavior}: settlement must be durable: {error}"));
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("run")
            .arg("--config")
            .arg(fixtures_dir().join("experiment/local-paired.toml"))
            .arg("--root")
            .arg(root.path().canonicalize().unwrap())
            .arg("--fixtures")
            .arg(fixtures_dir().canonicalize().unwrap())
            .arg("--behavior")
            .arg("happy")
            .arg("--recover")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{behavior}: recover exits nonzero"
        );
        let recovery: serde_json::Value =
            serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
        for row in recovery["recovery"].as_array().unwrap() {
            assert_eq!(
                row["status"], "sealed",
                "{behavior}: the cancelled settlement classifies immutably: {row}"
            );
        }
        assert_eq!(
            std::fs::read(&settlement).unwrap(),
            settlement_before,
            "{behavior}: recovery never rewrites the settled evidence"
        );
        let (code, offline, stderr) = offline_report(root.path());
        assert_eq!(code, 0, "{behavior}: {stderr}");
        assert_eq!(
            offline["run_outcome"], "incomplete",
            "{behavior}: {offline}"
        );
        let counts = &offline["coverage_counts"];
        assert_eq!(counts["planned"], 2, "{behavior}: {counts}");
        assert_eq!(counts["attempted"], 2, "{behavior}: {counts}");
        assert_eq!(counts["valid"], 0, "{behavior}: {counts}");
        assert_eq!(counts["invalid"], 2, "{behavior}: {counts}");
        assert_eq!(counts["unexecuted"], 0, "{behavior}: {counts}");
    }
}

/// `EVAL-A07`: a crash after durable intent and before settlement reopens
/// as effect-unknown with the original identity and artifacts, and any
/// replacement uses a new paired trial group.
#[test]
fn recovery_preserves_every_original_file_and_planned_trial() {
    fn files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        let mut result = std::collections::BTreeMap::new();
        if root.exists() {
            for entry in std::fs::read_dir(root).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    result.extend(files(&path));
                } else {
                    result.insert(path.clone(), std::fs::read(path).unwrap());
                }
            }
        }
        result
    }
    for behavior in [
        "unstarted",
        "happy",
        "agent-crash",
        "agent-cancelled",
        "crash-after-intent",
    ] {
        let room = tempfile::tempdir().unwrap();
        let root = room.path().join("run");
        if behavior != "unstarted" {
            std::fs::create_dir(&root).unwrap();
            let (code, _, _) = run_experiment("local-paired.toml", behavior, &root);
            assert_eq!(
                code,
                match behavior {
                    "happy" | "agent-crash" => 0,
                    "crash-after-intent" => 70,
                    _ => 1,
                },
                "{behavior}"
            );
        }
        let before = files(&root);
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("run")
            .arg("--config")
            .arg(fixtures_dir().join("experiment/local-paired.toml"))
            .arg("--fixtures")
            .arg(fixtures_dir())
            .arg("--root")
            .arg(&root)
            .arg("--recover")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let rows = report["recovery"].as_array().unwrap();
        assert_eq!(rows.len(), 2, "{behavior}: {report}");
        assert_eq!(
            files(&root),
            before,
            "{behavior}: recovery mutated original files"
        );
        if behavior == "unstarted" {
            assert!(!root.exists());
            assert!(rows.iter().all(|row| row["status"] == "not-started"));
        } else if behavior == "crash-after-intent" {
            assert_eq!(rows[0]["status"], "effect-unknown");
            assert_eq!(rows[1]["status"], "not-started");
        } else {
            assert!(rows.iter().all(|row| row["status"] == "sealed"));
            assert!(rows.iter().all(|row| row["settlement"].is_object()));
        }
    }
}

#[test]
fn recovery_rejects_frozen_inventory_identity_drift() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        run_experiment("local-paired.toml", "happy", root.path()).0,
        0
    );
    let control_path = root.path().join("run-control.json");
    let mut control: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&control_path).unwrap()).unwrap();
    control["planned_trials"][0]["subject"] = "wrong-subject".into();
    let bytes = serde_json::to_vec(&control).unwrap();
    std::fs::write(&control_path, &bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--fixtures")
        .arg(fixtures_dir())
        .arg("--root")
        .arg(root.path())
        .arg("--recover")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("identity drift"));
    assert_eq!(std::fs::read(control_path).unwrap(), bytes);
}

#[test]
fn effect_unknown_reopen() {
    let root = tempfile::tempdir().unwrap();

    // The crash: the run process dies right after the durable intent
    // reservation, before settlement.
    let (code, _report, _stderr) =
        run_experiment("local-paired.toml", "crash-after-intent", root.path());
    assert_eq!(code, 70, "the crash exit must be observable");
    let crashed = root.path().join("trials/trial-opi-1/bundle");
    assert!(crashed.join("intent.json").is_file());
    assert!(!crashed.join("settlement.json").exists());
    assert!(!crashed.join("manifest.json").exists());
    let intent_before = std::fs::read(crashed.join("intent.json")).unwrap();
    let control_before = std::fs::read(root.path().join("run-control.json")).unwrap();
    let (code, _, _) = run_experiment("local-paired.toml", "happy", root.path());
    assert_eq!(
        code, 2,
        "crashed roots require explicit recovery or replacement"
    );
    assert_eq!(
        std::fs::read(root.path().join("run-control.json")).unwrap(),
        control_before
    );
    assert_eq!(
        std::fs::read(crashed.join("intent.json")).unwrap(),
        intent_before
    );

    // Reopen: the durable files prove effect-unknown with the original
    // identity; the trial is never classified not-started or retryable
    // under the same identity (EVAL-DUR-002).
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .arg("--behavior")
        .arg("happy")
        .arg("--recover")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    assert_eq!(report["outcome"], "incomplete");
    let recovery = report["recovery"].as_array().unwrap();
    let crashed_row = recovery
        .iter()
        .find(|row| row["id"] == "trial-opi-1")
        .expect("the crashed trial reopens under its original identity");
    assert_eq!(crashed_row["status"], "effect-unknown");
    assert_eq!(crashed_row["boundary"], "trial-durability");
    // The edge stays visible with the exact missing sides: the crashed
    // candidate never ran to settlement, and the baseline never started.
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    let missing = pairs[0]["missing_sides"].as_array().unwrap();
    assert!(missing.contains(&serde_json::json!("candidate")));
    assert_eq!(pairs[0]["comparability"], "missing-baseline-trial");

    // The replacement re-runs the whole group under fresh trial identities
    // and a new paired trial group; prior evidence is never rewritten.
    let replacement_root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(replacement_root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .arg("--behavior")
        .arg("happy")
        .args(["--replacement-for", "trial-opi-1"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "replacement run: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let replacement: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    assert_eq!(replacement["outcome"], "completed");
    let trials = replacement["trials"].as_array().unwrap();
    let ids: Vec<&str> = trials
        .iter()
        .map(|trial| trial["id"].as_str().unwrap())
        .collect();
    let token = ids[0].strip_prefix("trial-opi-1.r").unwrap();
    assert_eq!(token.len(), 64);
    assert_eq!(ids[1], format!("trial-pi-1.r{token}"));
    for trial in trials {
        assert_eq!(trial["group"], format!("group-hello.r{token}"));
        assert_eq!(trial["status"], "sealed");
    }
    let pairs = replacement["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0]["comparability"], "comparable");
    assert_eq!(pairs[0]["baseline_trial"], ids[1]);
    assert_eq!(pairs[0]["candidate_trial"], ids[0]);

    // Prior evidence is untouched by the replacement.
    assert_eq!(
        std::fs::read(crashed.join("intent.json")).unwrap(),
        intent_before
    );
    assert!(!crashed.join("settlement.json").exists());

    // The prior attempt classification survives the replacement (P19-AC-010):
    // recovery retains the crashed root independently of the replacement
    // root; offline reports preserve both scopes without merging them.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .arg("--behavior")
        .arg("happy")
        .arg("--recover")
        .output()
        .unwrap();
    let after: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    let statuses: std::collections::BTreeMap<&str, &str> = after["recovery"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["id"].as_str().unwrap_or_default(),
                row["status"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        statuses.get("trial-opi-1").copied(),
        Some("effect-unknown"),
        "the prior attempt keeps its immutable classification: {statuses:?}"
    );
    // The frozen inventory retains the baseline without inventing an attempt.
    assert_eq!(statuses.get("trial-pi-1").copied(), Some("not-started"));
    assert!(!statuses.contains_key(ids[0]));
    assert!(!statuses.contains_key(ids[1]));
    assert_eq!(
        std::fs::read(root.path().join("run-control.json")).unwrap(),
        control_before
    );
    // The offline report publishes the replacement scope (same frozen
    // scope, fresh identities); the prior attempt is never merged into it
    // - it stays explicit and immutable in the recovery classification
    // above, with its unsealed evidence untouched on disk.
    let (code, offline, stderr) = offline_report(replacement_root.path());
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(offline["run_outcome"], "completed", "{offline}");
    let counts = &offline["coverage_counts"];
    assert_eq!(counts["planned"], 2, "{counts}");
    assert_eq!(counts["attempted"], 2, "{counts}");
    assert_eq!(counts["valid"], 2, "{counts}");
    assert_eq!(counts["unexecuted"], 0, "{counts}");
    let reported_ids: Vec<&str> = offline["trials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|trial| trial["id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(reported_ids, ids);
}

/// `EVAL-A11`: prompt-only/incomplete packages and selected native-verifier
/// failures stop `run` with a non-success exit, the owning package or
/// verifier failure persisted, and no cached score, alternate revision,
/// heuristic, or LLM fallback.
#[test]
fn incomplete_package_and_native_failure() {
    // Prompt-only package: the admission refuses before any verifier
    // process exists; the owning package failure is persisted on every
    // trial and the pair is excluded from a paired claim.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("local-paired.toml", "prompt-only-package", root.path());
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(report["outcome"], "incomplete");
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(
            trial["verifier"]["rejected"], "task-package-drift",
            "the owning package failure must be persisted: {trial}"
        );
        // No verifier output exists - the package was refused pre-spawn.
        assert_eq!(trial["authority"]["grade_dispatch"], "executed");
        assert_eq!(trial["status"], "sealed");
    }
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(
        pairs[0]["comparability"]
            .as_str()
            .unwrap()
            .starts_with("grader-failure:")
    );

    // Native verifier failure: the run settles the typed grader failure,
    // retains the verifier evidence, never invents a score, and the report
    // transition is mechanically refused.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("local-paired.toml", "verifier-failure", root.path());
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(report["outcome"], "incomplete");
    for trial in report["trials"].as_array().unwrap() {
        let id = trial["id"].as_str().unwrap();
        assert_eq!(trial["verifier"]["completion"], "failed", "{trial}");
        assert_eq!(trial["verifier"]["failure_kind"], "verifier-non-zero-exit");
        assert_eq!(trial["verifier"]["boundary"], "grader");
        // No score is invented from anything: the reward stays a typed
        // unknown, never a cached or heuristic value.
        assert_eq!(
            trial["verifier"]["reward"], "unknown:native-reward-pending-18-15-smoke",
            "{trial}"
        );
        // The grade dispatched exactly once - no alternate revision or
        // fallback grader ran.
        assert_eq!(trial["authority"]["grade_dispatch"], "executed");
        assert_eq!(
            trial["authority"]["report"], "refused:stopped-at-grader",
            "{trial}"
        );
        // Evidence is retained durably (sealed bundle), but the receipt
        // transition is mechanically refused.
        assert_eq!(trial["status"], "sealed");
        assert!(
            !root
                .path()
                .join("trials")
                .join(id)
                .join("receipt.json")
                .is_file()
        );
    }
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(
        pairs[0]["comparability"]
            .as_str()
            .unwrap()
            .starts_with("grader-failure:")
    );
}

/// `EVAL-A13` (run-path arm): a duplicate pair or an unsupported required
/// control yields an incomplete or non-comparable edge through `run`, with
/// native facts still visible and the exact coverage reason stated.
#[test]
fn non_comparable_surfaces_through_run() {
    // Duplicate slot: two declared trials for the same pairing role; the
    // assembly refuses the ambiguous contract with the typed reason.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("duplicate-pair.toml", "happy", root.path());
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(report["outcome"], "incomplete");
    let reason = report["comparison_error"].as_str().unwrap();
    assert!(
        reason.contains("duplicate") && reason.contains("baseline"),
        "the duplicate-role reason must be exact: {reason}"
    );
    assert_eq!(report["pairs"].as_array().unwrap().len(), 0);

    // Unsupported control: the shared reasoning value is not expressible by
    // either pinned profile, so the edge stays non-comparable with the
    // exact control named while native facts remain visible.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("unsupported-control.toml", "happy", root.path());
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(report["outcome"], "incomplete");
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0]["comparability"], "unsupported-control:reasoning");
    // Native facts stay visible despite the non-comparable edge: both
    // trials settled with their verifier reward facts in the receipts.
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["verifier"]["completion"], "verified", "{trial}");
        assert!(
            trial["verifier"]["reward"]
                .as_str()
                .unwrap()
                .starts_with("unknown:")
        );
    }
}

/// `EVAL-A14` (run-path arm): a broken/prompt-test-misaligned task or an
/// integrity exclusion removes the pair from Agent scoring via a new
/// immutable integrity record, stays visible in coverage, and never
/// rewrites prior evidence.
#[test]
fn integrity_exclusion_surfaces_through_run() {
    // The exclusion: the first declared trial is excluded with a stable
    // reason; the pair stays visible but carries no paired claim.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment(
        "integrity-exclusion.toml",
        "integrity-exclusion",
        root.path(),
    );
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(report["outcome"], "incomplete");
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(
        pairs[0]["comparability"],
        "excluded:trial-opi-1:fixture exclusion: workspace vanished under sandbox",
        "the exact exclusion reason must stay visible: {report}"
    );
    // Excluded from scoring, not from evidence: the trial still settled
    // and sealed under its own receipts.
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["status"], "sealed", "{trial}");
    }

    // The misaligned task: classification excludes it from Agent scoring
    // through a different immutable record identity.
    let root = tempfile::tempdir().unwrap();
    let (code, invalid_report, stderr) =
        run_experiment("invalid-task.toml", "invalid-task", root.path());
    assert_eq!(code, 1, "{stderr}");
    let pairs = invalid_report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0]["comparability"], "invalid-task-classification");

    // Every variant record is a new immutable identity: no run rewrote a
    // prior record (each document pins its own digest and the reports
    // carry three distinct ones).
    let digests = [
        report["integrity_digest"].as_str().unwrap(),
        invalid_report["integrity_digest"].as_str().unwrap(),
    ];
    assert_ne!(digests[0], digests[1]);
}

/// The durable intent of every trial binds to the comparison edge that
/// owns it : a multi-edge document seals each
/// group's trials under their own edge identity, and no intent defaults
/// to the first declared edge.
#[test]
fn multi_edge_intents_bind_to_their_owning_edge() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("multi-edge.toml", "happy", root.path());
    assert_eq!(
        code, 1,
        "the cross-edge pairing universe keeps unpaired sides visible: {stderr} {report}"
    );
    // Every declared trial still settled and sealed.
    assert_eq!(report["trials"].as_array().unwrap().len(), 4, "{report}");
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["status"], "sealed", "{report}");
    }

    // Each group's trials seal under their own edge's pair identity.
    let expected_pair = [
        ("trial-pi-a", "edge-pi-vs-opi"),
        ("trial-opi-a", "edge-pi-vs-opi"),
        ("trial-opi-b", "edge-opi-vs-pi2"),
        ("trial-pi2-b", "edge-opi-vs-pi2"),
    ];
    for (trial, edge) in expected_pair {
        let intent: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                root.path()
                    .join("trials")
                    .join(trial)
                    .join("bundle/intent.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(intent["pair"], edge, "{trial} must bind its owning edge");
    }

    // Both edges assemble their own pairing universe: the two real groups
    // pair comparably under their owning edge, while the cross-edge
    // (task, group) slots stay visible as missing sides.
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 4, "{report}");
    let by: std::collections::BTreeMap<(String, String), &serde_json::Value> = pairs
        .iter()
        .map(|pair| {
            (
                (
                    pair["edge"].as_str().unwrap().to_owned(),
                    pair["group"].as_str().unwrap().to_owned(),
                ),
                pair,
            )
        })
        .collect();
    assert_eq!(
        by[&("edge-pi-vs-opi".to_owned(), "group-a".to_owned())]["baseline_trial"],
        "trial-pi-a",
        "{report}"
    );
    assert_eq!(
        by[&("edge-pi-vs-opi".to_owned(), "group-a".to_owned())]["candidate_trial"],
        "trial-opi-a",
        "{report}"
    );
    assert_eq!(
        by[&("edge-pi-vs-opi".to_owned(), "group-a".to_owned())]["comparability"],
        "comparable",
        "{report}"
    );
    assert_eq!(
        by[&("edge-opi-vs-pi2".to_owned(), "group-b".to_owned())]["baseline_trial"],
        "trial-opi-b",
        "{report}"
    );
    assert_eq!(
        by[&("edge-opi-vs-pi2".to_owned(), "group-b".to_owned())]["candidate_trial"],
        "trial-pi2-b",
        "{report}"
    );
    assert_eq!(
        by[&("edge-opi-vs-pi2".to_owned(), "group-b".to_owned())]["comparability"],
        "comparable",
        "{report}"
    );
    assert_eq!(
        by[&("edge-pi-vs-opi".to_owned(), "group-b".to_owned())]["comparability"],
        "missing-baseline-trial",
        "{report}"
    );
    assert_eq!(
        by[&("edge-opi-vs-pi2".to_owned(), "group-a".to_owned())]["comparability"],
        "missing-candidate-trial",
        "{report}"
    );
}

/// All three concrete grader families assemble and grade end to end
/// through the run path, each against its own synthetic pinned package and
/// native report bytes; production-package execution belongs to native smoke.
#[test]
fn all_grader_families_assemble_through_run() {
    // (config, expected reward fact): 3.0 keeps the typed unknown pending
    // its native smoke; DeepSWE's resolved pier report carries reward 1.
    for (config, expected_reward) in [
        (
            "terminal-bench-3.0.toml",
            "unknown:native-reward-pending-18-15-smoke",
        ),
        ("deepswe.toml", "known:1(pier-report)"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) = run_experiment(config, "happy", root.path());
        assert_eq!(code, 0, "{config}: {stderr} report: {report}");
        assert_eq!(report["outcome"], "completed", "{config}: {report}");
        for trial in report["trials"].as_array().unwrap() {
            assert_eq!(trial["status"], "sealed", "{config}: {trial}");
            assert_eq!(
                trial["verifier"]["completion"], "verified",
                "{config}: {trial}"
            );
            assert_eq!(
                trial["verifier"]["reward"], expected_reward,
                "{config}: {trial}"
            );
        }
        let pairs = report["pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 1, "{config}");
        assert_eq!(
            pairs[0]["comparability"], "comparable",
            "{config}: {report}"
        );
    }
}

// --- Task 19.6: frozen plan execution, subset selection (P19-D05) ---

/// Recursively copies one directory tree (test staging only).
fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_tree(&path, &target.join(entry.file_name()));
        } else {
            std::fs::copy(&path, target.join(entry.file_name())).unwrap();
        }
    }
}

/// A fixtures tree whose terminal-bench-3.0 package is the multi-task
/// corpus fixture (alpha-notes, beta-render, gpu-train).
fn corpus_fixtures() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    // The whole fixture tree travels (agent stream fixtures included);
    // only the terminal-bench-3.0 package is replaced by the corpus.
    for entry in std::fs::read_dir(fixtures_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() && entry.file_name() != "external-locks" {
            copy_tree(&path, &dir.path().join(entry.file_name()));
        }
    }
    let package = dir
        .path()
        .join("benchmarks/terminal-bench-3.0/task-package");
    std::fs::remove_dir_all(&package).unwrap();
    copy_tree(
        &fixtures_dir().join("benchmarks/terminal-bench-3.0/corpus"),
        &package,
    );
    dir
}

fn selection_document(dir: &Path, task_ids: &str, repetitions: u32, concurrency: u32) -> PathBuf {
    let path = dir.join("selection.toml");
    std::fs::write(
        &path,
        format!(
            r#"schema = "opi-eval-experiment/2"
experiment_id = "corpus-subset-plan"
connection = "fixture-local"

[benchmark]
name = "terminal-bench"
revision = "3.0"
dataset = "corpus-fixture"

[[subjects]]
id = "opi"
product = "opi"
version = "0.8.1"

[[subjects]]
id = "pi"
product = "pi"
version = "0.84.1"

[[edges]]
id = "opi-baseline-vs-pi-candidate"
baseline = "opi"
candidate = "pi"

[[connections]]
id = "fixture-local"
provider = "local"
api = "openai-completions"
base_url = "http://127.0.0.1:48127/v1"
model = "scripted"
credential_env = "OPI_EVAL_FIXTURE_KEY"

[model_controls]
temperature = 0.0
top_p = "omitted"
max_output_tokens = 4096
context_window = 8192
reasoning = "omitted"

[execution]
mode = "hermetic"
concurrency = {concurrency}
agent_timeout_seconds = 600
verifier_timeout_seconds = 900

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "workspace-root"

[selection]
scope = "subset"
task_ids = {task_ids}
reason = "feasible corpus subset for the deterministic multi-task run"
repetitions = {repetitions}

[selection.seeds]
alpha-notes = 11
"#,
        ),
    )
    .unwrap();
    path
}

/// Pins the hermetic integrity digest into a selection document: the
/// first run derives the review identity (the expansion of the
/// selection is part of it), the failure names it, and the document is
/// patched exactly as the fixture documents were produced.
fn pin_hermetic_integrity(config: &Path, fixtures_root: &Path) {
    let scratch = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(config)
        .arg("--root")
        .arg(scratch.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_root)
        .args(["--behavior", "happy"])
        .output()
        .expect("spawn the opi-eval run binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let digest = stderr
        .split("derived review is ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no derived digest in: {stderr}"))
        .to_owned();
    let source = std::fs::read_to_string(config).unwrap();
    let pinned = source.replacen(
        "dataset = \"corpus-fixture\"",
        &format!("dataset = \"corpus-fixture\"\nintegrity_digest = \"{digest}\""),
        1,
    );
    std::fs::write(config, pinned).unwrap();
}

/// P19-AC-007 (selection) + P19-AC-001 (plan/run): a frozen feasible
/// subset executes exactly its tasks and repetitions from an
/// independent directory, with stage/settlement progress on stderr and
/// a structured saved result; the unmet GPU task never silently joins.
#[test]
fn frozen_subset_execution() {
    let fixtures = corpus_fixtures();
    let work = tempfile::tempdir().unwrap();
    let config = selection_document(work.path(), "[\"alpha-notes\", \"beta-render\"]", 2, 2);
    pin_hermetic_integrity(&config, &fixtures.path().canonicalize().unwrap());
    let plan_path = work.path().join("plan.json");

    let plan_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("plan")
        .arg("--config")
        .arg(&config)
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .arg("--out")
        .arg(&plan_path)
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .current_dir(work.path())
        .output()
        .expect("spawn the opi-eval plan command");
    let stdout = String::from_utf8_lossy(&plan_output.stdout);
    let stderr = String::from_utf8_lossy(&plan_output.stderr);
    assert_eq!(
        plan_output.status.code(),
        Some(0),
        "plan must freeze: stdout={stdout} stderr={stderr}"
    );
    let preview: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(preview["trial_count"], 8);
    assert_eq!(preview["concurrency"], 2);
    assert_eq!(
        preview["preflight"]["unknown"],
        serde_json::json!(["gpu-train"])
    );
    let plan_digest = preview["plan_digest"].as_str().unwrap().to_owned();

    let root = tempfile::tempdir().unwrap();
    let run_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .current_dir(work.path())
        .output()
        .expect("spawn the opi-eval run --plan command");
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    let stderr = String::from_utf8_lossy(&run_output.stderr);
    assert_eq!(
        run_output.status.code(),
        Some(0),
        "run --plan must complete: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stderr.contains("execution: 8 trials, width 2"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("stage=start"), "stderr: {stderr}");
    assert!(stderr.contains("settled=sealed"), "stderr: {stderr}");

    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    assert_eq!(report["plan_digest"], serde_json::json!(plan_digest));
    assert_eq!(report["execution_width"], 2);
    let tasks: std::collections::BTreeSet<&str> = report["trials"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|trial| trial["task"].as_str())
        .collect();
    assert_eq!(
        tasks,
        ["alpha-notes", "beta-render"].into_iter().collect(),
        "exactly the frozen subset executes"
    );
    assert_eq!(report["trials"].as_array().unwrap().len(), 8);
    // The paired groups stay comparable across both repetitions.
    assert_eq!(report["pairs"].as_array().unwrap().len(), 4);

    let saved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.path().join("run-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["plan_digest"], serde_json::json!(plan_digest));

    // The offline report keeps the subset label and corpus coverage from
    // the pre-effect run-control binding (P19-AC-008).
    let offline = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .output()
        .expect("spawn the offline report command");
    let offline_stdout = String::from_utf8_lossy(&offline.stdout).trim().to_owned();
    assert_eq!(
        offline.status.code(),
        Some(0),
        "offline report publishes: {offline_stdout}"
    );
    let published: serde_json::Value = serde_json::from_str(&offline_stdout)
        .unwrap_or_else(|_error| panic!("offline report is one JSON report: {offline_stdout:?}"));
    assert_eq!(published["selection_scope"], "subset", "{published}");
    assert_eq!(
        published["coverage_counts"]["corpus"],
        serde_json::json!(3),
        "{published}"
    );
    assert_eq!(
        published["coverage_counts"]["selected"],
        serde_json::json!(2),
        "{published}"
    );
}

/// A tampered plan never executes: the digest addresses the whole
/// frozen contract, so configuration changes require a new plan.
#[test]
fn tampered_plan_rejects_before_any_trial() {
    let fixtures = corpus_fixtures();
    let work = tempfile::tempdir().unwrap();
    let config = selection_document(work.path(), "[\"alpha-notes\"]", 1, 1);
    pin_hermetic_integrity(&config, &fixtures.path().canonicalize().unwrap());
    let plan_path = work.path().join("plan.json");
    let plan_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("plan")
        .arg("--config")
        .arg(&config)
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .arg("--out")
        .arg(&plan_path)
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .output()
        .expect("spawn the opi-eval plan command");
    assert_eq!(plan_output.status.code(), Some(0));

    let mut plan: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plan_path).unwrap()).unwrap();
    plan["experiment"]["source"] = serde_json::json!(
        plan["experiment"]["source"]
            .as_str()
            .unwrap()
            .replace("corpus-subset-plan", "quietly-changed")
    );
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();

    let root = tempfile::tempdir().unwrap();
    let run_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .output()
        .expect("spawn the opi-eval run --plan command");
    let stderr = String::from_utf8_lossy(&run_output.stderr);
    assert_eq!(run_output.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("digest drift"), "stderr: {stderr}");
}

/// A pi-only selection plans and runs with no artificial comparison
/// edge and no Opi material anywhere in the flow.
#[test]
fn pi_only_selection_runs_without_an_artificial_edge() {
    let fixtures = corpus_fixtures();
    let work = tempfile::tempdir().unwrap();
    let mut source =
        std::fs::read_to_string(selection_document(work.path(), "[\"alpha-notes\"]", 1, 1))
            .unwrap();
    // Single subject, no edges: the report stands on its own.
    let subjects_start = source.find("[[subjects]]").unwrap();
    let edges_end = source.find("[[connections]]").unwrap();
    source.replace_range(
        subjects_start..edges_end,
        "[[subjects]]\nid = \"pi\"\nproduct = \"pi\"\nversion = \"0.84.1\"\n\n",
    );
    let config = work.path().join("pi-only.toml");
    std::fs::write(&config, source).unwrap();
    pin_hermetic_integrity(&config, &fixtures.path().canonicalize().unwrap());

    let plan_path = work.path().join("pi-plan.json");
    let plan_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("plan")
        .arg("--config")
        .arg(&config)
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .arg("--out")
        .arg(&plan_path)
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .output()
        .expect("spawn the opi-eval plan command");
    assert_eq!(plan_output.status.code(), Some(0));
    let preview: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&plan_output.stdout).trim()).unwrap();
    assert_eq!(
        preview["subjects"],
        serde_json::json!([{"id": "pi", "product": "pi"}])
    );
    assert_eq!(preview["trial_count"], 1);

    let missing_fixtures_root = work.path().join("missing-fixtures");
    let rejected = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--plan"])
        .arg(&plan_path)
        .arg("--root")
        .arg(&missing_fixtures_root)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("--fixtures is required"));
    assert!(!missing_fixtures_root.exists());

    let root = tempfile::tempdir().unwrap();
    let run_output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures.path().canonicalize().unwrap())
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .output()
        .expect("spawn the opi-eval run --plan command");
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    let stderr = String::from_utf8_lossy(&run_output.stderr);
    assert_eq!(
        run_output.status.code(),
        Some(0),
        "pi-only run must complete: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    assert_eq!(report["trials"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["trials"][0]["subject"], "pi",
        "no absent Agent fills the pair"
    );
    assert_eq!(report["pairs"].as_array().unwrap().len(), 0);
}

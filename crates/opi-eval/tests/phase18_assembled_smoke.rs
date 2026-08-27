//! Assembled hermetic run-path suite (task 18.12).
//!
//! Every case drives the production `opi-eval run` binary end to end:
//! the CLI resolves the fixture experiment document, the runner reserves
//! durable intent, dispatches a runtime-generated deterministic helper as
//! the resolved executable (never an ambient PATH lookup), settles, grades
//! through the native verifier helper, seals the trial bundle, projects the
//! pre-seal trajectory, and assembles comparison coverage. This proves the
//! hermetic fixture-grade run path only: no real Opi/pi executable, real
//! provider, official task package, or network is claimed (task 18.15 owns
//! the native rerun). No paid provider, credential, or user-global resource
//! is touched (`P18-AGT-002`, `P18-AGT-006`).

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

/// The full hermetic paired run: durable intent, agent dispatch, settlement,
/// grade, seal, projection, and one comparable pair - all transitions
/// executed and all receipts persisted.
#[test]
fn p18_exp001_hermetic_paired_group_runs_end_to_end() {
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment("phase18-local.toml", "happy", root.path());
    assert_eq!(
        code, 0,
        "hermetic paired run must succeed: {stderr} report: {report}"
    );
    assert_eq!(report["schema"], "phase18-run-report/1");
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

/// `P18-A05`: through the production `run` path, each selected adapter
/// rejects an unknown required schema, malformed stream, missing
/// authoritative terminal event, and excessive output with the owning
/// typed failure visible and no fallback Agent or parser started (the
/// grade dispatch is mechanically refused; the verifier never runs).
#[test]
fn p18_a05_agent_stream_contract_fails_closed() {
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
            run_experiment("phase18-local.toml", case.behavior, root.path());
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

/// `P18-A06`: timeout or cancellation racing a bounded process, workspace
/// mutation, and process exit through `run` retains actual partial
/// artifacts and cleanup status, reports a non-success outcome, and never
/// infers replay safety (no replacement trial or second dispatch).
#[test]
fn p18_a06_process_timeout_cancellation_race() {
    for behavior in ["agent-timeout", "agent-cancelled"] {
        let root = tempfile::tempdir().unwrap();
        let (code, report, stderr) = run_experiment("phase18-local.toml", behavior, root.path());
        assert_eq!(code, 1, "{behavior}: {stderr}");
        assert_eq!(report["outcome"], "incomplete", "{behavior}: {report}");

        let expected_failure = if behavior == "agent-timeout" {
            "agent-timeout"
        } else {
            "agent-cancelled"
        };
        let trials = report["trials"].as_array().unwrap();
        assert_eq!(trials.len(), 2, "{behavior}: no replacement trials");
        for trial in trials {
            let id = trial["id"].as_str().unwrap();
            assert_eq!(
                trial["agent"]["failure_kind"], expected_failure,
                "{behavior}: {trial}"
            );
            assert_eq!(trial["agent"]["boundary"], "agent-process");
            assert_eq!(
                trial["agent"]["exit_state"],
                if behavior == "agent-timeout" {
                    "timed-out"
                } else {
                    "cancelled"
                },
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
            // The raced workspace mutation is retained, not discarded.
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
            // Replay safety is never inferred: exactly one dispatch, no
            // replacement identity.
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
    }
}

/// `P18-A07`: a crash after durable intent and before settlement reopens
/// as effect-unknown with the original identity and artifacts, and any
/// replacement uses a new paired trial group.
#[test]
fn p18_a07_effect_unknown_reopen() {
    let root = tempfile::tempdir().unwrap();

    // The crash: the run process dies right after the durable intent
    // reservation, before settlement.
    let (code, _report, _stderr) =
        run_experiment("phase18-local.toml", "crash-after-intent", root.path());
    assert_eq!(code, 70, "the crash exit must be observable");
    let crashed = root.path().join("trials/trial-opi-1/bundle");
    assert!(crashed.join("intent.json").is_file());
    assert!(!crashed.join("settlement.json").exists());
    assert!(!crashed.join("manifest.json").exists());
    let intent_before = std::fs::read(crashed.join("intent.json")).unwrap();

    // Reopen: the durable files prove effect-unknown with the original
    // identity; the trial is never classified not-started or retryable
    // under the same identity (P18-DUR-002).
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/phase18-local.toml"))
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
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/phase18-local.toml"))
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
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
    assert_eq!(ids, vec!["trial-opi-1.r2", "trial-pi-1.r2"]);
    for trial in trials {
        assert_eq!(trial["group"], "group-hello.r2");
        assert_eq!(trial["status"], "sealed");
    }
    let pairs = replacement["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0]["comparability"], "comparable");
    assert_eq!(pairs[0]["baseline_trial"], "trial-pi-1.r2");
    assert_eq!(pairs[0]["candidate_trial"], "trial-opi-1.r2");

    // Prior evidence is untouched by the replacement.
    assert_eq!(
        std::fs::read(crashed.join("intent.json")).unwrap(),
        intent_before
    );
    assert!(!crashed.join("settlement.json").exists());
}

/// `P18-A11`: prompt-only/incomplete packages and selected native-verifier
/// failures stop `run` with a non-success exit, the owning package or
/// verifier failure persisted, and no cached score, alternate revision,
/// heuristic, or LLM fallback.
#[test]
fn p18_a11_incomplete_package_and_native_failure() {
    // Prompt-only package: the admission refuses before any verifier
    // process exists; the owning package failure is persisted on every
    // trial and the pair is excluded from a paired claim.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("phase18-local.toml", "prompt-only-package", root.path());
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
        run_experiment("phase18-local.toml", "verifier-failure", root.path());
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

/// `P18-A13` (run-path arm): a duplicate pair or an unsupported required
/// control yields an incomplete or non-comparable edge through `run`, with
/// native facts still visible and the exact coverage reason stated.
#[test]
fn p18_a13_non_comparable_surfaces_through_run() {
    // Duplicate slot: two declared trials for the same pairing role; the
    // assembly refuses the ambiguous contract with the typed reason.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) =
        run_experiment("phase18-duplicate-pair.toml", "happy", root.path());
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
    let (code, report, stderr) =
        run_experiment("phase18-unsupported-control.toml", "happy", root.path());
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

/// `P18-A14` (run-path arm): a broken/prompt-test-misaligned task or an
/// integrity exclusion removes the pair from Agent scoring via a new
/// immutable integrity record, stays visible in coverage, and never
/// rewrites prior evidence.
#[test]
fn p18_a14_integrity_exclusion_surfaces_through_run() {
    // The exclusion: the first declared trial is excluded with a stable
    // reason; the pair stays visible but carries no paired claim.
    let root = tempfile::tempdir().unwrap();
    let (code, report, stderr) = run_experiment(
        "phase18-integrity-exclusion.toml",
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
        run_experiment("phase18-invalid-task.toml", "invalid-task", root.path());
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

/// All three concrete grader families assemble and grade end to end
/// through the run path, each against its own synthetic pinned package and
/// native report bytes (task 18.15 owns the production-package rerun).
#[test]
fn p18_exp002_all_grader_families_assemble_through_run() {
    // (config, expected reward fact): 3.0 keeps the typed unknown pending
    // its native smoke; DeepSWE's resolved pier report carries reward 1.
    for (config, expected_reward) in [
        (
            "phase18-tb30.toml",
            "unknown:native-reward-pending-18-15-smoke",
        ),
        ("phase18-deepswe.toml", "known:1(pier-report)"),
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

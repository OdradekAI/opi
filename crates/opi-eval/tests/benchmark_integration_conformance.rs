//! Parameterized benchmark-revision conformance suite (task 18.10.1).
//!
//! Each case spawns the production `opi-eval conformance` binary — the
//! minimum provisional process facade over the crate-private
//! `BenchmarkExecution` seam — with a bounded deterministic helper process
//! standing in for the native verifier and pinned saved native bytes.
//! This proves fixture-level hermetic conformance only: it never claims an
//! official task environment, a real native verifier, or a real graded run
//! (those remain task 18.15). No network, no Hub login, no cached-score
//! fallback (`P18-BMK-002`, `P18-BMK-005`, `P18-BMK-006`, `P18-BMK-009`).

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run one conformance case through the real binary; returns
/// (exit code, parsed report, stderr).
fn run_case(adapter: &str, case: &str) -> (i32, serde_json::Value, String) {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("conformance")
        .args(["--suite", "benchmark"])
        .args(["--adapter", adapter])
        .args(["--case", case])
        .arg("--root")
        .arg(root.path().canonicalize().unwrap())
        .arg("--fixtures")
        .arg(
            manifest_dir()
                .join("tests/fixtures")
                .canonicalize()
                .unwrap(),
        )
        .arg("--provider")
        .arg(
            manifest_dir()
                .join("scripts/phase18-scripted-provider.py")
                .canonicalize()
                .unwrap(),
        )
        .output()
        .expect("spawn the opi-eval conformance binary");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    let report: serde_json::Value = if stdout.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!("conformance stdout is not one JSON report ({e}): {stdout:?} stderr: {stderr:?}")
        })
    };
    (code, report, stderr)
}

/// The pinned per-(adapter, case) settlement truth table, asserted
/// independently of the driver's own `met` flag.
#[cfg(unix)]
struct Row {
    adapter: &'static str,
    case: &'static str,
    outcome: &'static str,
    kind: Option<&'static str>,
    boundary: Option<&'static str>,
    exit_state: &'static str,
}

#[cfg(unix)]
const GRADER: &str = "Grader";
#[cfg(unix)]
const ADAPTER: &str = "Adapter";
#[cfg(unix)]
const INFRA: &str = "Infrastructure";

#[cfg(unix)]
fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    // Terminal-Bench 2.1: synthetic byte-table pin plus the production
    // byte-table admission over the pinned fixture package.
    for adapter in ["terminal-bench-2.1", "terminal-bench-3.0"] {
        for case in ["completed", "identity", "immutable-capture"] {
            rows.push(Row {
                adapter,
                case,
                outcome: "verified",
                kind: None,
                boundary: None,
                exit_state: "exited:0",
            });
        }
        rows.push(Row {
            adapter,
            case: "one-failed",
            outcome: "verified",
            kind: None,
            boundary: None,
            exit_state: "exited:0",
        });
        rows.push(Row {
            adapter,
            case: "parse-failure",
            outcome: "failed",
            kind: Some("import-parse-failure"),
            boundary: Some(ADAPTER),
            exit_state: "exited:0",
        });
        rows.push(Row {
            adapter,
            case: "schema-drift",
            outcome: "failed",
            kind: Some("import-unsupported-schema"),
            boundary: Some(ADAPTER),
            exit_state: "exited:0",
        });
        rows.push(Row {
            adapter,
            case: "nonzero-exit",
            outcome: "failed",
            kind: Some("verifier-non-zero-exit"),
            boundary: Some(GRADER),
            exit_state: "exited:5",
        });
        rows.push(Row {
            adapter,
            case: "timeout",
            outcome: "failed",
            kind: Some("verifier-timeout"),
            boundary: Some(GRADER),
            exit_state: "timed-out",
        });
        rows.push(Row {
            adapter,
            case: "cancellation",
            outcome: "failed",
            kind: Some("verifier-cancelled"),
            boundary: Some(GRADER),
            exit_state: "cancelled",
        });
        rows.push(Row {
            adapter,
            case: "package-drift",
            outcome: "rejected",
            kind: None,
            boundary: None,
            exit_state: "rejected:task-package-drift",
        });
        rows.push(Row {
            adapter,
            case: "spawn-failure",
            outcome: "failed",
            kind: Some("spawn-not-found"),
            boundary: Some(INFRA),
            exit_state: "spawn:spawn-not-found",
        });
    }
    // Terminal-Bench 2.1 only: the production byte-table pin pins the real
    // official package bytes, so it refuses the synthetic fixture bytes
    // (real bytes arrive with task 18.15).
    rows.push(Row {
        adapter: "terminal-bench-2.1",
        case: "production-pin-drift",
        outcome: "rejected",
        kind: None,
        boundary: None,
        exit_state: "rejected:task-package-drift",
    });
    // Terminal-Bench 3.0 and DeepSWE: since task 18.15 registered the
    // reviewed byte tables, the production pins reject the synthetic
    // fixture bytes as drift (the case name is historical).
    for adapter in ["terminal-bench-3.0", "deepswe"] {
        rows.push(Row {
            adapter,
            case: "package-not-materialized",
            outcome: "rejected",
            kind: None,
            boundary: None,
            exit_state: "rejected:task-package-drift",
        });
    }
    // DeepSWE: synthetic Pier wiring; zero is an authoritative Verified
    // reward, never a failure.
    for case in ["completed", "identity", "immutable-capture"] {
        rows.push(Row {
            adapter: "deepswe",
            case,
            outcome: "verified",
            kind: None,
            boundary: None,
            exit_state: "exited:0",
        });
    }
    rows.push(Row {
        adapter: "deepswe",
        case: "zero-reward",
        outcome: "verified",
        kind: None,
        boundary: None,
        exit_state: "exited:0",
    });
    for (case, kind, boundary) in [
        ("parse-failure", "import-parse-failure", ADAPTER),
        ("schema-drift", "import-unsupported-schema", ADAPTER),
        ("nonzero-exit", "verifier-non-zero-exit", GRADER),
        ("timeout", "verifier-timeout", GRADER),
        ("cancellation", "verifier-cancelled", GRADER),
        ("spawn-failure", "spawn-not-found", INFRA),
    ] {
        rows.push(Row {
            adapter: "deepswe",
            case,
            outcome: "failed",
            kind: Some(kind),
            boundary: Some(boundary),
            exit_state: if case == "nonzero-exit" {
                "exited:5"
            } else if case == "timeout" {
                "timed-out"
            } else if case == "cancellation" {
                "cancelled"
            } else if case == "spawn-failure" {
                "spawn:spawn-not-found"
            } else {
                "exited:0"
            },
        });
    }
    rows.push(Row {
        adapter: "deepswe",
        case: "package-drift",
        outcome: "rejected",
        kind: None,
        boundary: None,
        exit_state: "rejected:task-package-drift",
    });
    rows
}

#[test]
fn unknown_case_is_rejected_with_exit_two() {
    let (code, report, stderr) = run_case("deepswe", "no-such-case");
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(report.is_null());
    assert!(stderr.contains("error:"));
}

#[cfg(unix)]
#[test]
fn benchmark_conformance_matrix_settles_every_pinned_case() {
    for row in rows() {
        let (code, report, stderr) = run_case(row.adapter, row.case);
        assert_eq!(
            code, 0,
            "case {} {} failed: {stderr}",
            row.adapter, row.case
        );
        assert_eq!(report["schema"], "phase18-conformance-report/1");
        assert_eq!(report["suite"], "benchmark");
        assert_eq!(report["adapter"], row.adapter);
        assert_eq!(report["case"], row.case);
        assert_eq!(report["met"], true, "report: {report}");
        assert_eq!(report["outcome"], row.outcome, "report: {report}");
        assert_eq!(
            report["failure_kind"],
            row.kind.map(Into::into).unwrap_or(serde_json::Value::Null),
            "report: {report}"
        );
        assert_eq!(
            report["boundary"],
            row.boundary
                .map(Into::into)
                .unwrap_or(serde_json::Value::Null),
            "report: {report}"
        );
        assert_eq!(report["exit_state"], row.exit_state, "report: {report}");

        match (row.adapter, row.case) {
            ("terminal-bench-2.1", "identity") => {
                assert_eq!(report["identity"]["benchmark"], "terminal-bench");
                assert_eq!(report["identity"]["revision"], "2.1");
                assert_eq!(report["identity"]["task_id"], "fixture-task");
            }
            ("terminal-bench-3.0", "identity") => {
                assert_eq!(report["identity"]["benchmark"], "terminal-bench");
                assert_eq!(report["identity"]["revision"], "3.0");
                assert_eq!(report["identity"]["task_id"], "synthetic-fixture-task");
            }
            ("deepswe", "identity") => {
                assert_eq!(report["identity"]["benchmark"], "deepswe");
                assert_eq!(report["identity"]["revision"], "v1.1");
                assert_eq!(report["identity"]["task_id"], "synthetic-fixture-task");
            }
            ("terminal-bench-2.1", "completed") => {
                // Terminal-Bench rewards stay authoritative-native: the
                // resolved reward is pending the 18.15 native smoke.
                assert_eq!(report["reward"]["state"], "unknown");
                assert_eq!(
                    report["reward"]["reason"],
                    "native-reward-pending-18-15-smoke"
                );
                assert_eq!(report["metrics"]["passed"]["value"], 6);
                assert_eq!(
                    report["artifact_roles"],
                    serde_json::json!(["native/ctrf-report"])
                );
            }
            ("terminal-bench-3.0", "completed") => {
                assert_eq!(report["reward"]["state"], "unknown");
                assert_eq!(
                    report["reward"]["reason"],
                    "native-reward-pending-18-15-smoke"
                );
                assert_eq!(report["metrics"]["passed"]["value"], 6);
                assert_eq!(
                    report["artifact_roles"],
                    serde_json::json!(["native/ctrf-report"])
                );
            }
            ("deepswe", "completed") => {
                assert_eq!(report["reward"]["state"], "known");
                assert_eq!(report["reward"]["value"], 1);
                assert_eq!(report["reward"]["origin"], "pier-report");
                assert_eq!(
                    report["artifact_roles"],
                    serde_json::json!(["native/pier-report"])
                );
            }
            ("deepswe", "zero-reward") => {
                // Zero is an authoritative Verified verdict, never a failure.
                assert_eq!(report["reward"]["state"], "known");
                assert_eq!(report["reward"]["value"], 0);
                assert_eq!(report["outcome"], "verified");
            }
            ("terminal-bench-2.1", "one-failed") | ("terminal-bench-3.0", "one-failed") => {
                // A failing native test is a settled Verified verdict with
                // the native failure visible in the metrics projection.
                assert_eq!(report["outcome"], "verified");
                assert_eq!(report["metrics"]["failed"]["value"], 1);
                assert_eq!(report["metrics"]["passed"]["value"], 5);
            }
            (_, "immutable-capture") => {
                let notes = report["notes"].as_array().unwrap();
                assert!(
                    notes.iter().any(|n| n == "immutable-capture-verified"),
                    "report: {report}"
                );
                assert!(report["identity"]["admitted_lock_digest"].is_string());
            }
            (_, "spawn-failure") => {
                assert!(!report.to_string().contains("no-such-verifier"));
            }
            _ => {}
        }
    }
}

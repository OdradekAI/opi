//! Parameterized Agent-adapter conformance suite (task 18.10.1).
//!
//! Each case spawns the production `opi-eval conformance` binary — the
//! minimum provisional process facade over the crate-private
//! `AgentExecution` seam — with a bounded deterministic helper process
//! standing in for the real agent product and pinned saved native bytes.
//! This proves fixture-level hermetic conformance only: it never claims an
//! exact built Opi/pi program, a real provider call, or an official task
//! environment (those remain task 18.15). No paid provider, credential, or
//! user-global resource is touched (`P18-AGT-002`, `P18-AGT-006`).

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run one conformance case through the real binary; returns
/// (exit code, parsed report, stderr).
fn run_case(suite: &str, adapter: &str, case: &str) -> (i32, serde_json::Value, String) {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("conformance")
        .args(["--suite", suite])
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
                .join("../../scripts/phase18-scripted-provider.py")
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

/// The pinned per-(adapter, case) settlement truth table. These
/// expectations are asserted independently of the driver's own `met` flag.
struct Row {
    adapter: &'static str,
    case: &'static str,
    outcome: &'static str,
    kind: Option<&'static str>,
    boundary: Option<&'static str>,
    exit_state: &'static str,
}

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for adapter in ["opi", "pi"] {
        // Success family.
        for case in ["completed", "identity", "isolation", "provider-fixture"] {
            rows.push(Row {
                adapter,
                case,
                outcome: "completed",
                kind: None,
                boundary: None,
                exit_state: "exited:0",
            });
        }
        // Typed failure family (process verdicts are authoritative).
        rows.push(Row {
            adapter,
            case: "nonzero-exit",
            outcome: "failed",
            kind: Some("agent-non-zero-exit"),
            boundary: Some("AgentProcess"),
            exit_state: "exited:3",
        });
        // Importer fail-closed family.
        rows.push(Row {
            adapter,
            case: "invalid-output",
            outcome: "failed",
            kind: Some("import-unsupported-schema"),
            boundary: Some("Adapter"),
            exit_state: "exited:0",
        });
        rows.push(Row {
            adapter,
            case: "parse-failure",
            outcome: "failed",
            kind: Some("import-parse-failure"),
            boundary: Some("Adapter"),
            exit_state: "exited:0",
        });
        // Bounded output: opi tolerates a capped stderr stream; pi's
        // capped stdout stream can never prove a terminal event.
        if adapter == "opi" {
            rows.push(Row {
                adapter,
                case: "bounded-output",
                outcome: "completed",
                kind: None,
                boundary: None,
                exit_state: "exited:0",
            });
        } else {
            rows.push(Row {
                adapter,
                case: "bounded-output",
                outcome: "failed",
                kind: Some("import-evidence-incomplete"),
                boundary: Some("Evidence"),
                exit_state: "exited:0",
            });
        }
        // Supervisor-settled family.
        rows.push(Row {
            adapter,
            case: "timeout",
            outcome: "failed",
            kind: Some("agent-timeout"),
            boundary: Some("AgentProcess"),
            exit_state: "timed-out",
        });
        rows.push(Row {
            adapter,
            case: "cancellation",
            outcome: "failed",
            kind: Some("agent-cancelled"),
            boundary: Some("AgentProcess"),
            exit_state: "cancelled",
        });
        rows.push(Row {
            adapter,
            case: "spawn-failure",
            outcome: "failed",
            kind: Some("spawn-not-found"),
            boundary: Some("AgentProcess"),
            exit_state: "spawn:spawn-not-found",
        });
    }
    rows
}

#[test]
fn unknown_case_is_rejected_with_exit_two() {
    let (code, report, stderr) = run_case("agent", "opi", "no-such-case");
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(report.is_null());
    assert!(stderr.contains("error:"));
}

#[cfg(unix)]
#[test]
fn agent_conformance_matrix_settles_every_pinned_case() {
    for row in rows() {
        let (code, report, stderr) = run_case("agent", row.adapter, row.case);
        assert_eq!(
            code, 0,
            "case {} {} failed: {stderr}",
            row.adapter, row.case
        );
        assert_eq!(report["schema"], "phase18-conformance-report/1");
        assert_eq!(report["suite"], "agent");
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
        assert_eq!(report["identity"]["product"], row.adapter);

        match (row.adapter, row.case) {
            ("opi", "identity") => {
                assert_eq!(report["identity"]["package"], "opi-coding-agent");
                assert_eq!(
                    report["identity"]["adapter"],
                    "opi-eval-opi-adapter/1 (phase18-agent-profile/1)"
                );
            }
            ("pi", "identity") => {
                assert_eq!(
                    report["identity"]["package"],
                    "@earendil-works/pi-coding-agent"
                );
                assert_eq!(
                    report["identity"]["adapter"],
                    "opi-eval-pi-adapter/1 (phase18-agent-profile/1)"
                );
            }
            ("opi", "completed") => {
                assert_eq!(report["usage"]["input_tokens"]["state"], "known");
                assert_eq!(report["usage"]["input_tokens"]["value"], 1234);
                assert_eq!(
                    report["usage"]["input_tokens"]["origin"],
                    "provider_reported"
                );
                assert_eq!(
                    report["artifact_roles"],
                    serde_json::json!(["evidence/manifest", "evidence/records"])
                );
            }
            ("pi", "completed") => {
                assert_eq!(report["usage"]["input_tokens"]["value"], 12);
                assert_eq!(report["usage"]["output_tokens"]["value"], 34);
                assert_eq!(
                    report["artifact_roles"],
                    serde_json::json!(["events/stdout"])
                );
            }
            (_, "isolation") => {
                let notes = report["notes"].as_array().unwrap();
                assert!(
                    notes.iter().any(|n| n == "isolation-verified"),
                    "report: {report}"
                );
            }
            (_, "provider-fixture") => {
                let notes = report["notes"].as_array().unwrap();
                assert!(
                    notes.iter().any(|n| n == "provider-fixture-verified"),
                    "report: {report}"
                );
            }
            ("opi", "bounded-output") => {
                assert_eq!(report["stderr_truncated"], true, "report: {report}");
            }
            (_, "spawn-failure") => {
                // Redaction: the settled report never echoes the missing path.
                assert!(!report.to_string().contains("no-such-agent"));
                assert_eq!(report["exit_state"], "spawn:spawn-not-found");
            }
            _ => {}
        }
    }
}

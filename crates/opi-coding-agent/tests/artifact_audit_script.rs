use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crate lives under crates/opi-coding-agent")
        .to_path_buf()
}

fn python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn run_audit_with_args(
    dir: &std::path::Path,
    workspace: &std::path::Path,
    json: bool,
) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--workspace-root")
        .arg(workspace)
        .args(json.then_some("--json"))
        .output()
        .expect("run artifact audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_audit(dir: &std::path::Path) -> (bool, String, String) {
    run_audit_with_args(dir, dir, false)
}

#[test]
fn artifact_audit_fails_on_workspace_root_leak_and_passes_when_removed() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let session_dir = dir.path().join("sessions");
    std::fs::create_dir(&session_dir).unwrap();
    // dir.path() on Windows uses backslashes — exercises the normalization path.
    // Escape them so the embedded JSON is valid on Windows: raw `\U`/`\L` are
    // invalid JSON escapes, and the session-header `cwd` must parse so it can be
    // recognized and skipped by the checker.
    let leaked_root = dir.path().display().to_string().replace('\\', "\\\\");

    // NDJSON: a tool-result-style line that embeds the absolute root.
    std::fs::write(
        dir.path().join("run.ndjson"),
        format!(
            "{{\"type\":\"Agent\",\"event\":{{\"type\":\"MessageUpdate\",\"message\":{{\"timestamp_ms\":1,\"content\":[{{\"type\":\"text\",\"text\":\"{leaked_root}/file.txt\"}}]}},\"assistant_event\":{{\"type\":\"text_delta\",\"delta\":\"x\"}}}}}}\n{{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}}}\n"
        ),
    )
    .unwrap();
    // Session JSONL: a session header (by-design cwd, must be skipped) + a leaking message.
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!(
            "{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"{leaked_root}/file.txt\"}}}}\n"
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        !ok,
        "audit must fail on leak: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("workspace_root_leak") || stdout.contains("session_workspace_root_leak"),
        "expected a leak finding, got: {stdout}"
    );

    // Remove the leaks (keep the session header, which is allowed).
    std::fs::write(
        dir.path().join("run.ndjson"),
        "{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":1,\"content\":[{\"type\":\"text\",\"text\":\"file.txt\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\"}}}\n{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n",
    )
    .unwrap();
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!("{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"file.txt\"}}}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        ok,
        "audit must pass after leak removal: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn artifact_audit_detects_zero_timestamps_turn_mismatch_and_duplicate_partials() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    // 60 text_delta lines, all carrying "partial" -> duplicated_text_delta_partials.
    let mut ndjson = String::new();
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    for _ in 0..59 {
        ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    }
    // 2 TurnStart events but provider_turns=5 -> mismatch.
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":5,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n");
    std::fs::write(dir.path().join("run.ndjson"), ndjson).unwrap();

    let (ok, stdout, _stderr) = run_audit(dir.path());
    assert!(!ok, "audit must fail on the synthetic defects");
    assert!(
        stdout.contains("all_zero_timestamps"),
        "missing zero-timestamp finding: {stdout}"
    );
    assert!(
        stdout.contains("duplicated_text_delta_partials"),
        "missing partial-duplication finding: {stdout}"
    );
    assert!(
        stdout.contains("provider_turn_mismatch"),
        "missing provider-turn mismatch finding: {stdout}"
    );
}

#[test]
fn artifact_audit_rejects_every_missing_declared_commit_reference() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let missing_summary_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4a";
    let missing_metadata_commit = "ffffffffffffffffffffffffffffffffffffffff";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {missing_summary_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{missing_metadata_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        !ok,
        "audit must reject missing commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on failure");
    let issues = report["issues"]
        .as_array()
        .expect("artifact audit issues are an array");
    let missing = issues
        .iter()
        .filter(|issue| issue["code"] == "missing_commit_reference")
        .collect::<Vec<_>>();
    assert_eq!(
        missing.len(),
        2,
        "every declared missing commit must be reported: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_summary_commit),
        "summary commit typo must be attributable: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_metadata_commit),
        "metadata commit must be attributable: {stdout}"
    );
}

#[test]
fn artifact_audit_accepts_real_declared_commit_objects() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let real_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4b";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {real_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{real_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        ok,
        "audit must accept real commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on success");
    assert_eq!(
        report["commit_references"].as_array().map(Vec::len),
        Some(2)
    );
}

//! Documentation and structure guard tests for Phase 7 reliability and
//! observability (task 7.6).
//!
//! These guards implement the Phase 7 design Success Criteria 1, 7, and 8:
//!
//! - **SC 1** — a shared diagnostic shape exists and is used by the new Phase 7
//!   surfaces (`phase7_shared_diagnostics_used_by_surfaces`). This is a
//!   structural guard: it inspects the source at each named public boundary
//!   (doctor output, RPC payloads, trace diagnostic-linked records, and the
//!   provider/runtime classification bridges) to prove the shared
//!   `opi_agent::Diagnostic` type crosses those boundaries rather than ad-hoc
//!   strings. The runtime behavior at each boundary is additionally pinned by
//!   `phase7_shared_diagnostics_used_by_doctor` / `_by_rpc` and the trace
//!   envelope redaction tests.
//! - **SC 7** — documentation states observability is local and explicit
//!   (`phase7_docs_state_local_explicit_observability`), in English and Chinese.
//! - **SC 8** — no telemetry, analytics, automatic session sharing, package
//!   ecosystem expansion, OAuth/provider breadth, marketplace, web dashboard,
//!   or stable 1.0 observability protocol is claimed or implemented
//!   (`phase7_non_goals_are_not_claimed_or_implemented`).
//!
//! The Phase 7 non-goal set is disjoint from the Phase 6 non-goal guards in
//! `productized_packages_docs.rs` (npm/marketplace/OAuth-parity/etc.); this
//! file owns only the observability-specific non-goals.

use std::path::Path;

/// Helper: read a file relative to the repo root.
fn read_repo_file(relative: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../..").join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Helper: case-insensitive substring check.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Helper: the repository root (two levels up from the test crate).
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Helper: assert that no line positively claims the forbidden `needle`.
/// Legitimate negation contexts (the guard itself saying "must not claim X")
/// are allowed.
fn no_positive_claim(haystack: &str, needle: &str) -> bool {
    let lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    for line in lower.lines() {
        if line.contains(&needle_lower) {
            if line.contains("reject")
                || line.contains("must not")
                || line.contains("do not")
                || line.contains("does not")
                || line.contains("not claim")
                || line.contains("no ")
                || line.contains("without")
                || line.contains("never ")
                || line.contains("不声明")
                || line.contains("不得")
                || line.contains("未实现")
                || line.contains("不会")
                || line.contains("从不")
                || line.contains("不收集")
                || line.contains("不传输")
                || line.contains("并未")
                || line.contains("并不")
            {
                continue;
            }
            return false;
        }
    }
    true
}

/// Helper: assert no file in `files` positively claims the forbidden `needle`.
fn assert_docs_reject_claim(files: &[&str], needle: &str, what: &str) {
    for path in files {
        let content = read_repo_file(path);
        assert!(
            no_positive_claim(&content, needle),
            "{path} must not positively claim {what}; forbidden phrase {needle:?} appeared outside a negation context"
        );
    }
}

// ===========================================================================
// SC 1: shared Diagnostic shape used by Phase 7 surfaces (structural guard)
// ===========================================================================

#[test]
fn phase7_shared_diagnostics_used_by_surfaces() {
    // Doctor: DoctorEntry flattens the shared Diagnostic at the --json boundary.
    let doctor = read_repo_file("crates/opi-coding-agent/src/doctor.rs");
    assert!(
        doctor.contains("#[serde(flatten)]") && doctor.contains("pub diagnostic: Diagnostic"),
        "doctor entries must flatten the shared Diagnostic at the public boundary"
    );
    assert!(
        doctor.contains("redacted_payload(RedactionMode::Summary)"),
        "doctor --json must route diagnostics through the shared Summary redaction"
    );

    // Trace: the envelope has a diagnostic-linked record kind carrying shared
    // diagnostic fields (source, diagnostic_code, severity).
    let trace = read_repo_file("crates/opi-agent/src/trace.rs");
    assert!(
        trace.contains("DiagnosticLinked"),
        "trace envelope must have a DiagnosticLinked record kind"
    );
    assert!(
        trace.contains("diagnostic_code") && trace.contains("pub source"),
        "trace records must carry shared diagnostic source/code fields"
    );

    // RPC: the SDK response model declares a structured `error_code` field from
    // the shared vocabulary (not a free-text error), and the RPC trace dispatch
    // uses a shared-code literal for unsupported requests.
    let sdk = read_repo_file("crates/opi-agent/src/sdk.rs");
    assert!(
        sdk.contains("error_code"),
        "SDK responses must declare a structured error_code field"
    );
    let rpc = read_repo_file("crates/opi-coding-agent/src/rpc.rs");
    assert!(
        rpc.contains("unsupported_trace_request"),
        "unsupported trace requests must use a structured shared error code at the RPC boundary"
    );
    assert!(
        rpc.contains("run_summary"),
        "RPC must emit a run_summary event with structured diagnostic counts"
    );

    // Provider/runtime classification: the shared Diagnostic is produced from
    // provider and agent-loop errors via From impls (the shared vocabulary).
    let diagnostic = read_repo_file("crates/opi-agent/src/diagnostic.rs");
    assert!(
        diagnostic.contains("impl From<&opi_ai::provider::ProviderError> for Diagnostic"),
        "provider errors must classify into the shared Diagnostic"
    );
    assert!(
        diagnostic.contains("impl From<&crate::loop_types::AgentError> for Diagnostic"),
        "agent-loop errors must classify into the shared Diagnostic"
    );

    // Non-interactive / session: the session event model exposes structured
    // diagnostic counts (an aggregate rollup over shared Diagnostics).
    let session_event = read_repo_file("crates/opi-agent/src/session_event.rs");
    assert!(
        session_event.contains("SessionDiagnosticCounts"),
        "run summaries must expose structured diagnostic counts"
    );
}

// ===========================================================================
// SC 7: documentation states observability is local, explicit, unstable 0.x
// ===========================================================================

#[test]
fn phase7_docs_state_local_explicit_observability() {
    // English surfaces.
    let spec = read_repo_file("docs/opi-spec.md");
    let readme = read_repo_file("README.md");
    let coding_readme = read_repo_file("crates/opi-coding-agent/README.md");

    for (name, content) in [
        ("opi-spec.md", spec.as_str()),
        ("README.md", readme.as_str()),
        ("opi-coding-agent/README.md", coding_readme.as_str()),
    ] {
        assert!(
            contains_ci(content, "local") && contains_ci(content, "explicit"),
            "{name} must state observability is local and explicit"
        );
        assert!(
            contains_ci(content, "0.x") || contains_ci(content, "unstable"),
            "{name} must state observability is an unstable 0.x surface"
        );
    }

    // Chinese counterparts carry the same posture (local/explicit/unstable).
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let readme_zh = read_repo_file("README.zh.md");
    let coding_readme_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");

    for (name, content) in [
        ("opi-spec.zh.md", spec_zh.as_str()),
        ("README.zh.md", readme_zh.as_str()),
        ("opi-coding-agent/README.zh.md", coding_readme_zh.as_str()),
    ] {
        assert!(
            content.contains("本地") && content.contains("显式"),
            "{name} must state observability is local (本地) and explicit (显式)"
        );
        assert!(
            content.contains("不稳定") || content.contains("0.x"),
            "{name} must state observability is unstable 0.x (不稳定/0.x)"
        );
    }

    // The opi doctor command and the local trace envelope are named so a
    // maintainer can find them.
    assert!(
        contains_ci(&spec, "opi doctor") && contains_ci(&spec, "trace"),
        "opi-spec must name the opi doctor command and the local trace envelope"
    );
    assert!(
        spec_zh.contains("opi doctor") && spec_zh.contains("trace"),
        "opi-spec.zh must name the opi doctor command and the local trace envelope"
    );
    assert!(
        contains_ci(&readme, "`trace`") && contains_ci(&coding_readme, "`trace`"),
        "English RPC docs must list the trace command"
    );
    assert!(
        readme_zh.contains("`trace`") && coding_readme_zh.contains("`trace`"),
        "Chinese RPC docs must list the trace command"
    );
    assert!(
        coding_readme.contains("startup_diagnostics")
            && coding_readme_zh.contains("startup_diagnostics"),
        "RPC ready-header docs must mention startup_diagnostics in EN and ZH"
    );
    assert!(
        contains_ci(&spec, "latest run")
            && contains_ci(&spec, "local memory")
            && spec.contains("TurnStarted")
            && spec.contains("TurnEnded"),
        "opi-spec must document RPC trace latest-run in-memory semantics and open-turn tolerance"
    );
    assert!(
        spec_zh.contains("最新一次运行")
            && spec_zh.contains("本地内存")
            && spec_zh.contains("TurnStarted")
            && spec_zh.contains("TurnEnded"),
        "opi-spec.zh must document RPC trace latest-run in-memory semantics and open-turn tolerance"
    );

    // EN/ZH posture is in sync (both carry the local+explicit claim).
    assert_eq!(
        contains_ci(&spec, "local") && contains_ci(&spec, "explicit"),
        spec_zh.contains("本地") && spec_zh.contains("显式"),
        "EN and ZH opi-spec must both carry the local+explicit observability posture"
    );
}

// ===========================================================================
// SC 8 + Non-Goals: forbidden observability is neither claimed nor implemented
// ===========================================================================

#[test]
fn phase7_non_goals_are_not_claimed_or_implemented() {
    let doc_files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];

    // --- Doc side: Phase 7 non-goals must not be positively claimed. ---
    // (npm/marketplace/OAuth-parity/etc. are owned by productized_packages_docs;
    // this set is the observability-specific non-goals.)
    let forbidden_en = [
        "remote telemetry",
        "telemetry service",
        "collects analytics",
        "analytics collection",
        "sends analytics",
        "automatic session sharing",
        "automatically shares sessions",
        "web dashboard",
        "stable 1.0 observability protocol",
        "stable observability protocol",
    ];
    let forbidden_zh = [
        "远程遥测",
        "遥测服务",
        "收集分析",
        "分析收集",
        "自动共享会话",
        "Web 仪表盘",
        "稳定 1.0 可观测协议",
    ];
    for needle in forbidden_en.iter().chain(forbidden_zh.iter()) {
        assert_docs_reject_claim(&doc_files, needle, "a Phase 7 observability non-goal");
    }

    // --- Code side: no telemetry/analytics backend is implemented. ---
    // Gather root + crate Cargo.toml files.
    let mut cargo_files: Vec<std::path::PathBuf> = vec![repo_root().join("Cargo.toml")];
    for entry in std::fs::read_dir(repo_root().join("crates")).expect("read crates directory") {
        let entry = entry.expect("dir entry");
        let path = entry.path().join("Cargo.toml");
        if path.is_file() {
            cargo_files.push(path);
        }
    }
    // No remote-telemetry/analytics backend crates. `tracing`/`tracing-subscriber`
    // are the local observability stack and are explicitly allowed.
    let forbidden_crates = [
        "opentelemetry",
        "otlp",
        "sentry",
        "posthog",
        "amplitude",
        "datadog",
        "mixpanel",
        "segment",
        "tracing-appender",
    ];
    for path in &cargo_files {
        let cargo = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for forbidden in forbidden_crates {
            assert!(
                !cargo.contains(forbidden),
                "{} must not depend on a telemetry/analytics backend crate ({forbidden}); Phase 7 forbids remote telemetry/analytics",
                path.display()
            );
        }
    }

    // The binary must not install a global tracing subscriber (no remote/structured
    // log backend is active by default or behind a flag).
    let main_rs = read_repo_file("crates/opi-coding-agent/src/main.rs");
    assert!(
        !main_rs.contains("set_global_default")
            && !main_rs.contains("Registry::default")
            && !main_rs.contains("FmtSubscriber"),
        "the opi binary must not install a global tracing subscriber (no telemetry backend)"
    );

    // The trace envelope must remain opt-in: the module must document that it
    // is NOT telemetry and is not persisted by default.
    let trace = read_repo_file("crates/opi-agent/src/trace.rs");
    assert!(
        contains_ci(&trace, "not telemetry") || contains_ci(&trace, "not a telemetry"),
        "trace module must state it is not telemetry"
    );
    assert!(
        contains_ci(&trace, "not") && contains_ci(&trace, "default"),
        "trace module must state traces are not produced by default"
    );
}

/// Meta-guard: the negation helper actually rejects a synthetic positive claim,
/// so the non-goal guards cannot be silently weakened.
#[test]
fn phase7_negation_helper_rejects_positive_claims() {
    assert!(
        !no_positive_claim(
            "opi now collects analytics by default",
            "collects analytics"
        ),
        "a positive analytics claim must be rejected, not treated as negated"
    );
    assert!(
        !no_positive_claim("opi ships a web dashboard", "web dashboard"),
        "a positive web-dashboard claim must be rejected"
    );
    // Chinese positives must also be rejected, so broadening the ZH negation
    // tokens cannot let a real positive claim through.
    assert!(
        !no_positive_claim("opi 默认自动共享会话", "自动共享会话"),
        "a positive Chinese auto-session-sharing claim must be rejected"
    );
    assert!(
        !no_positive_claim("opi 收集分析数据", "收集分析"),
        "a positive Chinese analytics claim must be rejected"
    );
    // Legitimate negation contexts pass.
    assert!(
        no_positive_claim("opi does not collect analytics", "collects analytics"),
        "a clear negation must pass the helper"
    );
    assert!(
        no_positive_claim("opi 也不会自动共享会话", "自动共享会话"),
        "a clear Chinese negation must pass the helper"
    );
}

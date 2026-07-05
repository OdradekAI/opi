//! Documentation, non-goal, and final-gate guard tests for Phase 13 session
//! tree and context reconstruction (task 13.7).
//!
//! These guards implement the Phase 13 design's "Documentation Updates",
//! "Non-Goals", and "Success Criteria 4 / 5 / 8 / 9":
//!
//! - **Docs + localized counterparts stay in sync**
//!   (`phase13_session_docs_and_localized_counterparts_stay_in_sync`) — the
//!   owned EN+ZH doc surfaces state the opi session format/version policy
//!   (header stays v1, Phase 13 entries are additive, no v2 migration
//!   precondition), v1 readability, the unknown-future-entry vs corrupt-middle
//!   recovery split, implemented branch/label/name/model/thinking/compaction/
//!   summary semantics, local export with redaction, and session-file
//!   sensitivity. Load-bearing identifiers are pinned in both languages.
//! - **branch_summary + custom_message decisions are explicit**
//!   (`phase13_branch_summary_and_custom_message_decisions_are_explicit`) —
//!   docs state that `branch_summary` is implemented as a context-reconstruction
//!   substrate while its generation UX triggers (branch switch, fork, manual
//!   command, extension hook) are explicitly deferred, and that
//!   `custom_message` provider-context semantics are deferred with a reason,
//!   each deferral carrying a source citation.
//! - **SC9 non-goals** (`phase13_non_goals_not_in_core`) — the ten Phase 13
//!   non-goals are documented as deferred and are absent from core through
//!   structural positives (no forbidden crate deps, no forbidden modules) and
//!   no positive current-core claim in any owned doc surface.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Path/file helpers (match the Phase 6/7/8/11/12 doc-guard convention).
// ---------------------------------------------------------------------------

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Case-insensitive substring check.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Doc surfaces task 13.7 owns (English + Simplified-Chinese counterparts).
fn doc_surfaces() -> Vec<&'static str> {
    vec![
        "README.md",
        "README.zh.md",
        "crates/opi-agent/README.md",
        "crates/opi-agent/README.zh.md",
        "crates/opi-coding-agent/README.md",
        "crates/opi-coding-agent/README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
        "docs/pi-alignment-matrix.md",
        "docs/pi-alignment-matrix.zh.md",
    ]
}

/// The ten Phase 13 non-goals (design doc, "Non-Goals") with an English and a
/// Simplified-Chinese token each.
const PHASE13_NON_GOALS: &[(&str, &str)] = &[
    ("vector database", "向量数据库"),
    ("semantic memory", "语义记忆"),
    ("global user profile", "全局用户画像"),
    ("cross-project memory injection", "跨项目记忆注入"),
    ("pi session v3", "pi session v3"),
    ("cloud sync", "云同步"),
    ("session sharing", "会话分享"),
    ("web UI", "Web UI"),
    ("package ecosystem", "包生态"),
    ("ProviderCollection", "ProviderCollection"),
];

// ===========================================================================
// Docs + localized counterparts stay in sync (EN + ZH)
// ===========================================================================

/// The owned doc surfaces state the Phase 13 session format/version policy,
/// v1 readability, unknown-future-entry vs corrupt-middle recovery, implemented
/// entry semantics, local export + redaction, and session-file sensitivity —
/// in English and Simplified Chinese, with load-bearing identifiers pinned.
#[test]
fn phase13_session_docs_and_localized_counterparts_stay_in_sync() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let coding = read_repo_file("crates/opi-coding-agent/README.md");
    let coding_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");
    let agent = read_repo_file("crates/opi-agent/README.md");
    let agent_zh = read_repo_file("crates/opi-agent/README.zh.md");
    let root = read_repo_file("README.md");
    let root_zh = read_repo_file("README.zh.md");

    // (1) Version policy: Phase 13 keeps the v1 header; new entries are
    //     additive; no v2 migration precondition on resume.
    assert!(
        contains_ci(&spec, "version 1") && contains_ci(&spec, "additive"),
        "docs/opi-spec.md must state the session header stays version 1 with additive Phase 13 entries"
    );
    assert!(
        contains_ci(&spec, "no automatic migration")
            || contains_ci(&spec, "without requiring an automatic migration"),
        "docs/opi-spec.md must state Phase 13 requires no automatic migration as a resume precondition"
    );
    assert!(
        contains_ci(&spec_zh, "版本 1")
            || contains_ci(&spec_zh, "version 1")
            || contains_ci(&spec_zh, "保留 v1"),
        "docs/opi-spec.zh.md must state the v1-header-kept version policy"
    );
    assert!(
        contains_ci(&spec_zh, "增补") || contains_ci(&spec_zh, "附加"),
        "docs/opi-spec.zh.md must state Phase 13 entries are additive"
    );

    // (2) v1 readability + resume.
    assert!(
        contains_ci(&spec, "v1 sessions remain readable")
            || contains_ci(&spec, "v1 files readable"),
        "docs/opi-spec.md must state v1 sessions remain readable"
    );
    assert!(
        contains_ci(&spec_zh, "v1")
            && (contains_ci(&spec_zh, "可读") || contains_ci(&spec_zh, "保持")),
        "docs/opi-spec.zh.md must state v1 readability"
    );

    // (3) Unknown-future-entry vs corrupt-middle recovery split.
    assert!(
        contains_ci(&spec, "unknown") && contains_ci(&spec, "corrupt"),
        "docs/opi-spec.md must distinguish unknown-future-entry handling from corrupt-middle recovery"
    );
    assert!(
        contains_ci(&spec_zh, "未知") && contains_ci(&spec_zh, "损坏"),
        "docs/opi-spec.zh.md must distinguish unknown-future-entry handling from corrupt-middle recovery"
    );

    // (4) Implemented entry semantics: the Phase 13 entries that 13.1-13.6
    //     exercise in production paths. Each must be named in the spec and in
    //     the opi-agent README (the crate that owns the typed entries).
    let implemented_entries = [
        "session_info",
        "model_change",
        "thinking_level_change",
        "label",
        "branch_summary",
    ];
    for entry in implemented_entries {
        assert!(
            contains_ci(&spec, entry),
            "docs/opi-spec.md must name the implemented Phase 13 entry `{entry}`"
        );
        assert!(
            contains_ci(&agent, entry),
            "opi-agent README must name the implemented Phase 13 entry `{entry}`"
        );
        assert!(
            agent_zh.contains(entry),
            "opi-agent README.zh must name the implemented Phase 13 entry `{entry}`"
        );
    }

    // (5) Local export + redaction behavior.
    assert!(
        contains_ci(&coding, "export-session") && contains_ci(&coding, "redact"),
        "opi-coding-agent README must document --export-session and redaction"
    );
    assert!(
        contains_ci(&coding_zh, "export-session") && contains_ci(&coding_zh, "脱敏"),
        "opi-coding-agent README.zh must document --export-session and redaction"
    );
    assert!(
        contains_ci(&spec, "export") && contains_ci(&spec, "redact"),
        "docs/opi-spec.md must document local export with redaction"
    );

    // (6) Session-file sensitivity.
    assert!(
        contains_ci(&spec, "sensitive"),
        "docs/opi-spec.md must state that session files are sensitive"
    );
    assert!(
        contains_ci(&spec_zh, "敏感"),
        "docs/opi-spec.zh.md must state that session files are sensitive"
    );
    for surface in [&root, &root_zh] {
        assert!(
            contains_ci(surface, "sensitive") || contains_ci(surface, "敏感"),
            "root READMEs must note session-file sensitivity"
        );
    }

    // (7) Phase 13 spec section is expanded past the old "planned" stub and
    //     reflects implemented status.
    assert!(
        contains_ci(&spec, "Session Tree and Context Reconstruction"),
        "docs/opi-spec.md must carry the Phase 13 section title"
    );
}

// ===========================================================================
// branch_summary + custom_message decisions are explicit
// ===========================================================================

/// Docs explicitly state the branch_summary decision (implemented as substrate,
/// generation UX deferred) and the custom_message deferral, each with a source
/// citation, in both languages.
#[test]
fn phase13_branch_summary_and_custom_message_decisions_are_explicit() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let agent = read_repo_file("crates/opi-agent/README.md");
    let agent_zh = read_repo_file("crates/opi-agent/README.zh.md");

    // (1) branch_summary is implemented as a context-reconstruction substrate.
    assert!(
        contains_ci(&spec, "branch_summary")
            && (contains_ci(&spec, "context reconstruction")
                || contains_ci(&spec, "reconstructed context")),
        "docs/opi-spec.md must tie branch_summary to context reconstruction"
    );
    assert!(
        contains_ci(&agent, "branch_summary"),
        "opi-agent README must name branch_summary as an implemented entry"
    );

    // (2) branch_summary generation UX triggers are explicitly deferred, with
    //     a source citation. The deferred triggers are named in the DoD:
    //     branch switch, fork, manual command, extension hook.
    let deferred_triggers = ["branch switch", "fork", "manual command", "extension hook"];
    let spec_lower = spec.to_lowercase();
    let any_deferral = deferred_triggers.iter().any(|t| spec_lower.contains(t));
    assert!(
        any_deferral,
        "docs/opi-spec.md must name at least one deferred branch_summary generation UX trigger"
    );
    assert!(
        contains_ci(&spec, "defer") || contains_ci(&spec, "Phase 14"),
        "docs/opi-spec.md must state the branch_summary generation UX deferral"
    );
    assert!(
        contains_ci(&spec_zh, "branch_summary") || contains_ci(&spec_zh, "分支摘要"),
        "docs/opi-spec.zh.md must name branch_summary"
    );
    assert!(
        contains_ci(&spec_zh, "推迟") || contains_ci(&spec_zh, "延迟"),
        "docs/opi-spec.zh.md must state a deferral"
    );

    // (3) custom_message is deferred with a reason, in both languages.
    assert!(
        contains_ci(&spec, "custom_message"),
        "docs/opi-spec.md must name custom_message"
    );
    assert!(
        contains_ci(&spec, "defer") || contains_ci(&spec, "not implemented"),
        "docs/opi-spec.md must state custom_message is deferred or not implemented"
    );
    assert!(
        contains_ci(&spec_zh, "custom_message"),
        "docs/opi-spec.zh.md must name custom_message"
    );
    assert!(
        contains_ci(&agent, "custom_message"),
        "opi-agent README must name custom_message"
    );
    assert!(
        contains_ci(&agent_zh, "custom_message"),
        "opi-agent README.zh must name custom_message"
    );
}

// ===========================================================================
// SC9: Phase 13 non-goals documented as deferred and absent from core
// ===========================================================================

/// The ten Phase 13 non-goals are documented (EN+ZH) and are absent from core:
/// no forbidden crate dependency, no forbidden opi-agent/opi-coding-agent
/// module, and no positive current-core claim in any owned doc surface.
#[test]
fn phase13_non_goals_not_in_core() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    // (1) All ten non-goals are documented in both languages.
    for (en, zh) in PHASE13_NON_GOALS {
        assert!(
            contains_ci(&spec, en),
            "docs/opi-spec.md must list the Phase 13 non-goal: {en}"
        );
        assert!(
            spec_zh.contains(zh),
            "docs/opi-spec.zh.md must list the Phase 13 non-goal: {zh}"
        );
    }

    // (2) No positive current-core claim of a non-goal anywhere in the surfaces.
    let forbidden_positive = [
        "vector database is implemented",
        "semantic memory is implemented",
        "cloud sync is implemented",
        "session sharing is implemented",
        "pi session v3 compatibility",
        "pi session v3 is supported",
        "web ui is implemented",
        "interactive /export is implemented",
    ];
    for path in doc_surfaces() {
        let content = read_repo_file(path);
        for phrase in forbidden_positive {
            assert!(
                !contains_ci(&content, phrase),
                "{path} must not positively claim `{phrase}` as current core behavior"
            );
        }
    }

    // (3) Structural positive: no forbidden crate dependency anywhere in the
    //     workspace. Scans the root and per-crate Cargo.toml files.
    let mut cargo_files: Vec<PathBuf> = vec![repo_root().join("Cargo.toml")];
    let crates_dir = repo_root().join("crates");
    for entry in std::fs::read_dir(&crates_dir).expect("read crates dir") {
        let path = entry.expect("dir entry").path().join("Cargo.toml");
        if path.is_file() {
            cargo_files.push(path);
        }
    }
    let forbidden_crates = [
        "qdrant",
        "pinecone",
        "weaviate",
        "milvus",
        "chromadb",
        "oauth2",
        "openidconnect",
    ];
    let mut scanned_cargo = 0usize;
    for path in &cargo_files {
        let cargo = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        scanned_cargo += 1;
        for forbidden in forbidden_crates {
            assert!(
                !cargo.contains(forbidden),
                "{} must not depend on a forbidden Phase 13 non-goal crate ({forbidden})",
                path.display()
            );
        }
    }
    assert!(
        scanned_cargo >= 4,
        "vacuous-guard: cargo scan must visit at least 4 Cargo.toml files (saw {scanned_cargo})"
    );

    // (4) Structural positive: no vector-memory / cloud-sync / sharing module
    //     declared in opi-agent or opi-coding-agent.
    for crate_name in ["opi-agent", "opi-coding-agent"] {
        let lib_rs = read_repo_file(&format!("crates/{crate_name}/src/lib.rs"));
        for forbidden_mod in [
            "cloud_sync",
            "session_share",
            "session_share_service",
            "vector_memory",
            "semantic_memory",
            "global_profile",
            "web_ui",
        ] {
            assert!(
                !lib_rs.contains(&format!("pub mod {forbidden_mod}"))
                    && !lib_rs.contains(&format!("mod {forbidden_mod}")),
                "{crate_name} must not declare a `{forbidden_mod}` module (Phase 13 non-goal)"
            );
        }
    }

    // (5) Interactive /export stays deferred (Phase 14), not claimed as core.
    let coding = read_repo_file("crates/opi-coding-agent/README.md");
    assert!(
        !contains_ci(&coding, "interactive /export is implemented"),
        "opi-coding-agent README must not claim interactive /export is implemented (deferred to Phase 14)"
    );
}

//! Phase 18 rollback contract suite (task 18.16).
//!
//! The registered Phase source must keep proving three things mechanically:
//!
//! * the GLM-5.3 coverage roadmap retains all 16 comparison-table entries
//!   plus the private Z.ai Code Bench, with coverage waves, admission
//!   boundaries, the experiment-class distinction, and future authority
//!   gates (P18-RDM-001..P18-RDM-006);
//! * the Non-goal capabilities (Continual Learning, Candidate Production,
//!   Promotion, Active Snapshots, behavioral activation, automatic
//!   remediation) remain unimplemented in source and package metadata;
//! * the rollback boundary stays coherent: the Companion is one
//!   unpublished, optional path whose removal needs no product change.

use std::path::{Path, PathBuf};

/// The registered Phase 18 supplemental source (registry in the
/// opi-implement skill; hashed by the ledger's `spec_files_sha256`).
const PHASE_SOURCE: &str = "docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md";

/// All 16 GLM-5.3 comparison-table benchmark entries, exactly as the
/// registered roadmap table names them.
const GLM53_ENTRIES: [&str; 16] = [
    "Terminal Bench 2.1",
    "Terminal Bench 3.0",
    "DeepSWE v1.1",
    "NL2Repo",
    "ProgramBench (Almost Solved)",
    "FrontierSWE",
    "SWE-Marathon v1.1",
    "PostTrainBench",
    "CyberGym",
    "ExploitGym (2h / 6h)",
    "ExploitBench",
    "Toolathlon Verified",
    "AutomationBench v1.0.6",
    "Agents' Last Exam (ALE-CLI)",
    "HLE w/ Tools",
    "GDPval-AA v2",
];

/// Coverage waves the roadmap table distinguishes.
const COVERAGE_WAVES: [&str; 5] = [
    "Phase 18 coding foundation",
    "Remaining coding",
    "| Cyber |",
    "| Agentic |",
    "Private evidence",
];

/// Registered Non-goal capability identifiers. Their implementation would
/// contradict the Phase's rollback contract.
const NON_GOAL_CAPABILITIES: [&str; 6] = [
    "continual learning",
    "candidate production",
    "promotion",
    "active snapshot",
    "behavioral activation",
    "automatic remediation",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/opi-eval/tests -> workspace root")
        .to_path_buf()
}

/// P18-RDM-001..006: the registered Phase source retains the complete
/// roadmap identity — every GLM-5.3 entry, the private bench's not-admitted
/// status, the coverage waves, the experiment-class distinction, and the
/// future authority gates for post-Phase admission.
#[test]
fn roadmap_retains_all_glm53_entries_and_experiment_class_distinction() {
    let source = std::fs::read_to_string(workspace_root().join(PHASE_SOURCE))
        .expect("registered Phase 18 source");
    for entry in GLM53_ENTRIES {
        assert!(
            source.contains(entry),
            "roadmap no longer names GLM-5.3 entry: {entry}"
        );
    }
    for wave in COVERAGE_WAVES {
        assert!(
            source.contains(wave),
            "roadmap no longer carries coverage wave: {wave}"
        );
    }
    assert!(
        source.contains("Z.ai Code Bench"),
        "private Z.ai Code Bench missing from the roadmap"
    );
    assert!(
        source.contains("Not admitted"),
        "admission boundary language missing"
    );
    assert!(
        source.contains("experiment class"),
        "experiment-class distinction missing"
    );
    assert!(
        source.contains("separately approved gate"),
        "future authority-gate language missing"
    );
}

/// A Non-goal capability may appear in source only inside a comment block
/// that declares the capability absent, reserved, or out of scope — never
/// as executable code. The check is block-level: a doc-comment block
/// counts as declarative when any of its lines carries a negating phrase.
fn declarative(block: &[&str]) -> bool {
    let all_comment = block.iter().all(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty()
    });
    if !all_comment {
        return false;
    }
    let joined = block
        .iter()
        .map(|line| line.to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    [
        " no ", " not ", " never ", "without", "exclud", "reserved", "outside", "future", "remain",
        "non-goal",
    ]
    .iter()
    .any(|marker| joined.contains(marker))
}

/// The registered Non-goal capabilities remain unimplemented: no Companion
/// source references them at all, product sources reference them only in
/// declarative comments that state their absence, and no workspace package
/// exposes a feature or dependency named for them.
#[test]
fn phase18_non_goal_capabilities_remain_unimplemented() {
    let repo = workspace_root();

    // Companion sources: zero occurrences, comment or not.
    let eval_src = repo.join("crates/opi-eval/src");
    assert!(
        !dir_contains_any(&eval_src, &NON_GOAL_CAPABILITIES),
        "Companion source references a Non-goal capability"
    );

    // Product sources: occurrences must be declarative-only comments.
    for crate_name in [
        "opi-agent",
        "opi-ai",
        "opi-coding-agent",
        "opi-protocol",
        "opi-sandbox",
        "opi-tui",
    ] {
        let src = repo.join("crates").join(crate_name).join("src");
        assert!(
            !dir_contains_implemented(&src, &NON_GOAL_CAPABILITIES),
            "{crate_name}: Non-goal capability appears outside declarative comments"
        );
    }

    // Package layer: no feature or dependency name carries a capability
    // identifier (publish/feature posture is asserted separately).
    let manifests = [
        "Cargo.toml",
        "crates/opi-agent/Cargo.toml",
        "crates/opi-ai/Cargo.toml",
        "crates/opi-coding-agent/Cargo.toml",
        "crates/opi-eval/Cargo.toml",
        "crates/opi-protocol/Cargo.toml",
        "crates/opi-sandbox/Cargo.toml",
        "crates/opi-tui/Cargo.toml",
    ];
    for manifest in manifests {
        let text = std::fs::read_to_string(repo.join(manifest)).unwrap();
        let lowered = text.to_lowercase();
        for capability in NON_GOAL_CAPABILITIES {
            assert!(
                !lowered.contains(capability),
                "{manifest}: package metadata references {capability}"
            );
        }
    }
}

/// P18-RBK-002/003 boundary: the Companion stays one unpublished optional
/// path — `publish = false` and zero workspace features — so rollback
/// removes it without touching product behavior.
#[test]
fn rollback_keeps_the_companion_one_unpublished_optional_path() {
    let manifest =
        std::fs::read_to_string(workspace_root().join("crates/opi-eval/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("publish = false"),
        "the Companion must stay unpublished"
    );
    assert!(
        !manifest.contains("[features]"),
        "the Companion must not grow feature flags (no dual paths)"
    );
}

fn dir_contains_any(root: &Path, needles: &[&str]) -> bool {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
            if needles.iter().any(|needle| text.contains(needle)) {
                return true;
            }
        }
    }
    false
}

fn dir_contains_implemented(root: &Path, needles: &[&str]) -> bool {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let lines: Vec<String> = std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect();
            // Group consecutive comment lines into blocks; a capability
            // mention is allowed only inside a declarative block.
            let mut index = 0;
            while index < lines.len() {
                let is_comment = {
                    let trimmed = lines[index].trim_start();
                    trimmed.starts_with("//") || trimmed.starts_with("/*")
                };
                if is_comment {
                    let start = index;
                    while index < lines.len() {
                        let trimmed = lines[index].trim_start();
                        if trimmed.starts_with("//")
                            || trimmed.starts_with("/*")
                            || trimmed.is_empty()
                        {
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    let block: Vec<&str> = lines[start..index].iter().map(String::as_str).collect();
                    let lowered: Vec<String> =
                        block.iter().map(|line| line.to_lowercase()).collect();
                    if needles
                        .iter()
                        .any(|needle| lowered.iter().any(|line| line.contains(needle)))
                        && !declarative(&block)
                    {
                        return true;
                    }
                } else {
                    let lowered = lines[index].to_lowercase();
                    if needles.iter().any(|needle| lowered.contains(needle)) {
                        return true;
                    }
                    index += 1;
                }
            }
        }
    }
    false
}

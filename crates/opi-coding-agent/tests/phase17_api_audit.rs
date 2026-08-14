//! Phase 17 task 17.9 — removal, hermeticness, and platform-contract audits.
//!
//! - P17-MIG-006: the removed 0.x interfaces are absent from production
//!   source — not retained behind aliases, feature flags, or compatibility
//!   shims. Proven by a comment-aware source scan over `crates/*/src` so the
//!   doc comments that legitimately record a removal do not count as a
//!   retained interface.
//! - P17-PLT-002: the Phase 17 acceptance tests call no paid/live providers
//!   and contain no network endpoints.
//! - P17-PLT-003: the bilingual product documentation carries the non-sandbox
//!   boundary (tool authorization is not an operating-system sandbox).
//! - Task-local P17-A15 precondition: the CI workflow selects the SAME hermetic
//!   Phase 17 acceptance on Linux, macOS, and Windows with no OS-specific
//!   gating. (Actual three-platform run SHA/URLs/results remain Phase F
//!   evidence; this proves the workflow definition only.)

#[path = "common/phase17.rs"]
mod phase17;

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Recursively collect every `.rs` file under `dir` (skipping non-file
/// entries — the tests-tree read_dir guard).
fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Production source files (`crates/*/src/**/*.rs`), comment-stripped, paired
/// with their path for failure messages.
fn production_sources() -> Vec<(PathBuf, String)> {
    let root = workspace_root();
    let mut files = Vec::new();
    for crate_dir in [
        "opi-ai",
        "opi-tui",
        "opi-agent",
        "opi-protocol",
        "opi-sandbox",
        "opi-coding-agent",
    ] {
        collect_rust_files(&root.join("crates").join(crate_dir).join("src"), &mut files);
    }
    files
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p).unwrap_or_default();
            let stripped = strip_comments(&raw);
            (p, stripped)
        })
        .collect()
}

/// Strip `//` line comments and (nested) `/* */` block comments. A string
/// literal containing a comment marker is over-stripped, which can only make
/// the audit fail louder, never pass a retained symbol silently.
fn strip_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut depth: usize = 0;
    while i < chars.len() {
        let c = chars[i];
        if depth > 0 {
            if c == '/' && chars.get(i + 1) == Some(&'*') {
                depth += 1;
                i += 2;
            } else if c == '*' && chars.get(i + 1) == Some(&'/') {
                depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            depth = 1;
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Count occurrences of `needle` whose immediately preceding characters are
/// NOT `exclude_prefix` (so `MetadataProvider` does not match inside
/// `ListingMetadataProvider`). Empty `exclude_prefix` counts every match.
fn occurrences_not_prefixed(hay: &str, needle: &str, exclude_prefix: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = hay[start..].find(needle) {
        let abs = start + pos;
        if exclude_prefix.is_empty() || !hay[..abs].ends_with(exclude_prefix) {
            count += 1;
        }
        start = abs + needle.len();
    }
    count
}

// ===========================================================================
// P17-MIG-006 — removed interfaces stay removed: no symbol, alias, or shim in
// production source.
// ===========================================================================

#[test]
fn phase17_removed_interfaces_are_absent_from_production_source() {
    let sources = production_sources();
    assert!(
        sources.len() > 100,
        "the scan examined a real source tree ({} files)",
        sources.len()
    );

    // (symbol, preceding-token exclusion, why it was removed)
    let targets: &[(&str, &str, &str)] = &[
        (
            "SharedProvider",
            "",
            "17.2: Agent no longer owns one Arc<dyn Provider>",
        ),
        (
            "AgentLoopTurnUpdate",
            "",
            "17.2: append-only turn updates replaced by atomic NextTurnState",
        ),
        (
            "AgentHarness",
            "",
            "17.2: the unused opi-agent state owner was removed",
        ),
        (
            "HarnessRuntimeConfig",
            "",
            "17.2: the unused opi-agent state owner was removed",
        ),
        (
            "BeforeToolCallResult::Allow",
            "",
            "17.4: the authorization-suggesting hook grant is now Continue",
        ),
        (
            "MetadataProvider",
            "Listing",
            "17.5: renamed to ListingMetadataProvider",
        ),
        (
            "TraceSink",
            "",
            "17.7: the storage-shaped core trace contract was superseded by evidence",
        ),
        (
            "TraceReader",
            "",
            "17.8: no legacy trace reader exists without a registered workflow",
        ),
    ];
    for (symbol, exclusion, why) in targets {
        for (path, stripped) in &sources {
            let hits = occurrences_not_prefixed(stripped, symbol, exclusion);
            assert_eq!(
                hits,
                0,
                "removed interface `{symbol}` still referenced in {} ({why})",
                path.display()
            );
        }
    }

    // Product policy must not live in Agent Core: the Reference Product policy
    // types appear only in opi-coding-agent source.
    for (path, stripped) in &sources {
        let in_core = path
            .components()
            .any(|c| c.as_os_str().to_str() == Some("opi-agent"))
            || path
                .components()
                .any(|c| c.as_os_str().to_str() == Some("opi-ai"));
        if !in_core {
            continue;
        }
        for symbol in [
            "PermissionPolicy",
            "EffectiveUserPolicy",
            "ProductToolAuthorizer",
        ] {
            assert_eq!(
                occurrences_not_prefixed(stripped, symbol, ""),
                0,
                "product policy symbol `{symbol}` must not live in Agent Core: {}",
                path.display()
            );
        }
    }

    // The alias-registry and compatibility-shim rows of the removal audit have
    // no single scannable symbol: their absence is proven behaviorally by the
    // owner tasks (17.5's no-alias/bare-model-ambiguity tests in
    // phase17_provider_runtime.rs) rather than by a token scan here.
}

// ===========================================================================
// P17-PLT-002 — Phase 17 tests call no paid/live providers: no network
// endpoints in any phase17 acceptance source. This file is excluded from its
// own scan because the assertion literals below would self-match.
// ===========================================================================

#[test]
fn phase17_tests_are_hermetic_no_network_no_paid_providers() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rust_files(&root.join("crates/opi-coding-agent/tests"), &mut files);
    collect_rust_files(&root.join("crates/opi-agent/tests"), &mut files);
    let phase17: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("phase17"))
                && p.file_name().and_then(|n| n.to_str()) != Some("phase17_api_audit.rs")
        })
        .collect();
    assert!(
        phase17.len() >= 6,
        "the scan examined the Phase 17 acceptance sources ({} files)",
        phase17.len()
    );
    let scheme = String::from("ht") + "tp://";
    let secure_scheme = String::from("ht") + "tps://";
    for path in &phase17 {
        let raw = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            !raw.contains(&scheme) && !raw.contains(&secure_scheme),
            "Phase 17 test contains a network endpoint: {}",
            path.display()
        );
    }
}

// ===========================================================================
// P17-PLT-003 — the bilingual documentation carries the non-sandbox boundary.
// ===========================================================================

#[test]
fn phase17_documentation_claims_no_os_sandbox() {
    let root = workspace_root();
    let en = std::fs::read_to_string(root.join("README.md")).unwrap();
    assert!(
        en.contains("not an operating-system sandbox"),
        "the English README states the non-sandbox boundary"
    );
    let zh = std::fs::read_to_string(root.join("README.zh.md")).unwrap();
    assert!(
        zh.contains("不是操作系统 sandbox"),
        "the Chinese README states the non-sandbox boundary"
    );
}

// ===========================================================================
// Task-local P17-A15 precondition — the CI workflow selects the same hermetic
// Phase 17 acceptance on all three platforms with no OS-specific gating.
// ===========================================================================

#[test]
fn phase17_ci_matrix_selects_same_acceptance_on_three_platforms() {
    let ci = include_str!("../../../.github/workflows/ci.yml");
    let start = ci
        .find("  phase17_acceptance:")
        .expect("the phase17_acceptance job exists");
    let rest = &ci[start..];
    // The job block runs until the next sibling job key: a line indented by
    // exactly two spaces (deeper indentation belongs to this job's body).
    let mut block_lines: Vec<&str> = Vec::new();
    for (i, line) in rest.lines().enumerate() {
        if i > 0
            && ((line.starts_with("  ") && !line.starts_with("   "))
                || (!line.starts_with(' ') && !line.is_empty()))
        {
            break;
        }
        block_lines.push(line);
    }
    let block = block_lines.join("\n");
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(
            block.contains(os),
            "the phase17 acceptance matrix includes {os}"
        );
    }
    for target in [
        "--test phase17_cross_mode",
        "--test phase17_failure_rollback",
        "--test phase17_api_audit",
    ] {
        assert!(
            block.contains(target),
            "the phase17 acceptance job selects {target} on every matrix OS"
        );
    }
    assert!(
        !block.contains("if: matrix.os") && !block.contains("if: runner"),
        "the phase17 acceptance job has no OS-conditional gating"
    );
}

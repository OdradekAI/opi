//! Phase 8 runtime-contract documentation and non-goal guard tests (task 8.7).
//!
//! These guards implement the Phase 8 design Success Criteria 6 and 8:
//!
//! - **SC 6** — public `opi-agent` runtime, extension, event, session, SDK/RPC,
//!   and streaming-proxy surfaces are classified in the docs (English and
//!   Chinese) as supported 0.x, unstable internal, or candidate removal
//!   (`phase8_api_surface_classification`). Each classified surface is pinned
//!   to its exact `pub use` re-export line in `crates/opi-agent/src/lib.rs`
//!   (renaming or removing a re-export fails the test), and `SessionEntry` is
//!   confirmed module-path-only.
//! - **SC 8** — Phase 8 did not claim or implement a stable 1.0 API, a
//!   TypeScript extension API, package ecosystem expansion, a new adapter kind,
//!   web UI, provider OAuth, in-core workflow tools, an MCP runtime, a shared
//!   `opi-types` crate, unjustified public type migration, or a whole-loop
//!   rewrite (`phase8_non_goals_not_claimed_or_implemented`).
//!
//! The Phase 8 non-goal set is disjoint from the Phase 6 guards in
//! `productized_packages_docs.rs` (npm/marketplace/OAuth-parity) and the
//! Phase 7 guards in `observability_docs.rs` (telemetry/analytics); this file
//! owns only the Phase 8 runtime-stabilization non-goals.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Helper: read a file relative to the repo root.
fn read_repo_file(relative: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../..").join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn section_between<'a>(content: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = content
        .find(start)
        .unwrap_or_else(|| panic!("missing section start {start}"));
    let after_start = &content[start_index..];
    let end_index = after_start
        .find(end)
        .unwrap_or_else(|| panic!("missing section end {end}"));
    &after_start[..end_index]
}

fn backticked_names(section: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut remaining = section;
    while let Some(start) = remaining.find('`') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let name = &after_start[..end];
        if name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            names.insert(name.to_string());
        }
        remaining = &after_start[end + 1..];
    }
    names
}

fn api_surface_classification_rows(section: &str) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();

    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }

        let cells: Vec<_> = trimmed
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        if cells.len() < 3 {
            continue;
        }

        let surface = cells[0];
        let tier = cells[1];
        if surface == "Surface"
            || surface == "Tier"
            || surface.chars().all(|ch| ch == '-')
            || tier.chars().all(|ch| ch == '-')
        {
            continue;
        }

        for name in backticked_names(surface) {
            rows.insert(name, tier.to_string());
        }
    }

    rows
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (index, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(input[start..].trim());
    parts
}

fn split_top_level_alias(input: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;

    for (index, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ' ' if depth == 0 && input[index..].starts_with(" as ") => {
                let left = input[..index].trim();
                let right = input[index + 4..].trim();
                return Some((left, right));
            }
            _ => {}
        }
    }

    None
}

fn split_top_level_group(input: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;

    for (index, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 && input[index..].starts_with("::{") => {
                let prefix = input[..index].trim();
                let suffix = input[index + 3..].trim();
                return Some((prefix, suffix));
            }
            _ => {}
        }
    }

    None
}

fn last_path_segment(path: &str) -> &str {
    path.rsplit("::")
        .next()
        .expect("path should have at least one segment")
        .trim()
}

fn collect_reexport_names(tree: &str, prefix: &[&str], names: &mut BTreeSet<String>) {
    for item in split_top_level(tree, ',') {
        if item.is_empty() {
            continue;
        }

        if let Some((base, alias)) = split_top_level_alias(item) {
            let export_name = if alias == "self" {
                last_path_segment(base)
            } else {
                last_path_segment(alias)
            };
            names.insert(export_name.to_string());
            continue;
        }

        if let Some((base, grouped)) = split_top_level_group(item) {
            let grouped = grouped
                .strip_suffix('}')
                .expect("grouped pub use should end with `}`");
            let mut next_prefix = prefix.to_vec();
            next_prefix.extend(
                base.split("::")
                    .map(str::trim)
                    .filter(|segment| !segment.is_empty()),
            );
            collect_reexport_names(grouped, &next_prefix, names);
            continue;
        }

        if item == "self" {
            let export_name = prefix
                .last()
                .expect("`self` in a pub use group needs a parent path");
            names.insert((*export_name).to_string());
            continue;
        }

        if item == "*" {
            panic!("glob pub use is not supported in crate_root_reexport_names");
        }

        names.insert(last_path_segment(item).to_string());
    }
}

fn crate_root_reexport_names(lib: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut statement = String::new();
    let mut collecting = false;

    for line in lib.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub use ") {
            statement.clear();
            statement.push_str(trimmed);
            collecting = !trimmed.ends_with(';');
        } else if collecting {
            statement.push(' ');
            statement.push_str(trimmed);
            collecting = !trimmed.ends_with(';');
        } else {
            continue;
        }

        if !collecting {
            let rest = statement
                .trim_start_matches("pub use ")
                .trim_end_matches(';')
                .trim();
            collect_reexport_names(rest, &[], &mut names);
        }
    }

    names
}

#[test]
fn crate_root_reexport_names_handles_aliases_and_nested_paths() {
    let lib = r#"
pub use foo::Bar as Baz;
pub use nested::Qux;
pub use outer::{
    inner::Leaf as Renamed,
    branch::{Twig, stem::Bud as Bloom},
};
"#;

    let names = crate_root_reexport_names(lib);

    assert!(names.contains("Baz"), "alias should export the alias name");
    assert!(
        names.contains("Qux"),
        "nested path should export the final segment"
    );
    assert!(
        names.contains("Renamed"),
        "grouped alias should export the alias name"
    );
    assert!(
        names.contains("Twig"),
        "grouped nested path should export the final segment"
    );
    assert!(
        names.contains("Bloom"),
        "nested grouped alias should export the alias name"
    );
}

#[test]
fn api_surface_classification_rows_require_table_rows_with_tiers() {
    let section = r#"
## API Surface Classification

`AgentState` appears in prose but not in the classification table.

| Surface | Tier | Notes |
|---|---|---|
| `Agent` | supported 0.x | Stateful loop wrapper. |
| `Tool`, `ToolResult` | unstable internal | Example grouped row. |
"#;

    let rows = api_surface_classification_rows(section);

    assert_eq!(rows.get("Agent").map(String::as_str), Some("supported 0.x"));
    assert_eq!(
        rows.get("Tool").map(String::as_str),
        Some("unstable internal")
    );
    assert_eq!(
        rows.get("ToolResult").map(String::as_str),
        Some("unstable internal")
    );
    assert!(
        !rows.contains_key("AgentState"),
        "prose mentions must not count as classification rows"
    );
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
///
/// Limitation: this is a per-line substring model shared with the Phase 6/7
/// guards. A line is treated as negated if any negation token co-occurs on it,
/// so a deliberately deceptive line that places a negation word next to a
/// positive claim (e.g. "Web UI is not optional") can leak through. The token
/// set is kept to the narrow Phase 7 baseline (no overly-common broadeners),
/// and the meta-guard pins the reliable behavior; fully closing co-occurrence
/// leaks would require structural clause parsing across all three phase guards
/// and is out of scope for this task.
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
// SC 6: public opi-agent surfaces are classified (EN + ZH)
// ===========================================================================

#[test]
fn phase8_api_surface_classification() {
    let en = read_repo_file("crates/opi-agent/README.md");
    let zh = read_repo_file("crates/opi-agent/README.zh.md");
    let lib = read_repo_file("crates/opi-agent/src/lib.rs");
    let expected_reexports = crate_root_reexport_names(&lib);
    let en_api = section_between(&en, "## API Surface Classification", "## Non-Goals");
    let zh_api = section_between(&zh, "## API 表面分类", "## 非目标（Non-Goals）");
    let en_rows = api_surface_classification_rows(en_api);
    let zh_rows = api_surface_classification_rows(zh_api);
    let en_names: BTreeSet<_> = en_rows.keys().cloned().collect();
    let zh_names: BTreeSet<_> = zh_rows.keys().cloned().collect();
    let missing_en: Vec<_> = expected_reexports.difference(&en_names).cloned().collect();
    let missing_zh: Vec<_> = expected_reexports.difference(&zh_names).cloned().collect();

    assert!(
        missing_en.is_empty(),
        "EN API Surface Classification missing crate-root re-exports: {missing_en:?}"
    );
    assert!(
        missing_zh.is_empty(),
        "ZH API Surface Classification missing crate-root re-exports: {missing_zh:?}"
    );

    // The classification section exists in both languages.
    assert!(
        contains_ci(&en, "API Surface Classification"),
        "EN opi-agent README must have an API Surface Classification section"
    );
    assert!(
        zh.contains("API 表面分类"),
        "ZH opi-agent README must have an API 表面分类 section"
    );

    // All three classification tiers are documented in both languages.
    for (tier_en, tier_zh) in [
        ("supported 0.x", "支持的 0.x"),
        ("unstable internal", "不稳定内部"),
        ("candidate removal", "候选移除"),
    ] {
        assert!(
            contains_ci(&en, tier_en),
            "EN README must document the {tier_en:?} classification tier"
        );
        assert!(
            zh.contains(tier_zh),
            "ZH README must document the {tier_zh:?} classification tier"
        );
    }

    // Every named surface is bound to its classification tier on a single doc
    // line (the table row), in both languages. Asserting surface + tier
    // co-occurrence prevents a silent misclassification (e.g. promoting an
    // unstable-internal surface to supported 0.x) from passing the guard.
    let classification: &[(&str, &str, &str)] = &[
        // (surface, tier EN, tier ZH)
        ("Agent", "supported 0.x", "支持的 0.x"),
        ("agent_loop", "supported 0.x", "支持的 0.x"),
        ("AgentHooks", "supported 0.x", "支持的 0.x"),
        ("Tool", "supported 0.x", "支持的 0.x"),
        ("AgentEvent", "supported 0.x", "支持的 0.x"),
        ("AgentSessionEvent", "unstable internal", "不稳定内部"),
        ("SessionEntry", "unstable internal", "不稳定内部"),
        ("Extension", "unstable internal", "不稳定内部"),
        ("ExtensionRegistry", "unstable internal", "不稳定内部"),
        ("SdkCommand", "unstable internal", "不稳定内部"),
        ("SdkResponse", "unstable internal", "不稳定内部"),
        ("StreamingProxy", "unstable internal", "不稳定内部"),
    ];
    for (surface, tier_en, tier_zh) in classification {
        assert!(
            en.lines()
                .any(|l| l.contains(surface) && contains_ci(l, tier_en)),
            "EN README must classify {surface} as {tier_en} on one line"
        );
        assert!(
            zh.lines()
                .any(|l| l.contains(surface) && l.contains(tier_zh)),
            "ZH README must classify {surface} as {tier_zh} on one line"
        );
    }

    // Honesty: the docs state there is no stable 1.0 promise and name the real
    // stability mechanism (#[non_exhaustive] plus module "# Unstable" prose;
    // no #[doc(hidden)] / #[unstable] feature gate).
    assert!(
        en.contains("no stable 1.0") && en.contains("#[non_exhaustive]"),
        "EN README must state no stable 1.0 promise and document #[non_exhaustive]"
    );
    assert!(
        zh.contains("不会给出稳定 1.0") && zh.contains("#[non_exhaustive]"),
        "ZH README must state 不会给出稳定 1.0 and document #[non_exhaustive]"
    );

    // The three wire schema versions are documented alongside the classification.
    for version in [
        "SDK_SCHEMA_VERSION = 3",
        "NDJSON_SCHEMA_VERSION = 2",
        "TRACE_SCHEMA_VERSION = 1",
    ] {
        assert!(
            en.contains(version),
            "EN README must document {version} with the classification"
        );
        assert!(
            zh.contains(version),
            "ZH README must document {version} with the classification"
        );
    }

    // Production cross-check: each classified crate-root surface is pinned to
    // its exact `pub use` re-export line in lib.rs (not a bare substring), so
    // renaming or removing a re-export fails the guard rather than being masked
    // by an unrelated identifier (e.g. a bare `Agent` check would also match
    // `AgentError`). SessionEntry is confirmed module-path-only.
    for reexport in [
        "pub use agent::Agent;",
        "pub use agent_loop::agent_loop;",
        "pub use hooks::AgentHooks;",
        "pub use event::{AgentEvent, AgentEventSink};",
        "pub use session_event::AgentSessionEvent;",
        "pub use sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse};",
        "Extension, ExtensionCommand, ExtensionError, ExtensionHookResult, ExtensionRegistry",
        "ProxyConfig, ProxyEvent, ProxyHandler, SecretRedactor, StreamingProxy, StreamingProxyError",
        "ExecutionMode, Tool, ToolError, ToolResult",
    ] {
        assert!(
            lib.contains(reexport),
            "opi-agent lib.rs must contain the classified re-export: {reexport}"
        );
    }
    let session = read_repo_file("crates/opi-agent/src/session.rs");
    assert!(
        session.contains("SessionEntry"),
        "SessionEntry must remain in the session module"
    );
    assert!(
        !lib.contains("pub use session::SessionEntry"),
        "SessionEntry must NOT be hoisted to the crate root (unstable internal, module-path-only)"
    );
}

// ===========================================================================
// SC 8 + Non-Goals: forbidden scope is neither claimed nor implemented
// ===========================================================================

#[test]
fn phase8_non_goals_not_claimed_or_implemented() {
    let doc_files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
        "crates/opi-agent/README.md",
        "crates/opi-agent/README.zh.md",
    ];

    // --- Doc side: Phase 8 non-goals must not be positively claimed. ---
    let forbidden_en = [
        "stable 1.0 public API",
        "stable 1.0 API promise",
        "TypeScript extension API compatibility",
        "TypeScript extension API",
        "package ecosystem expansion",
        "package marketplace",
        "new adapter kind",
        "web UI",
        "web dashboard",
        "provider OAuth login",
        "OAuth client credentials",
        "in-core plan mode",
        "in-core sub-agent",
        "MCP runtime",
        "shared opi-types crate",
        "whole agent loop rewrite",
    ];
    let forbidden_zh = [
        "稳定 1.0 公共 API",
        "稳定 1.0 API 承诺",
        "TypeScript 扩展 API 兼容",
        "package 生态扩张",
        "package 市场",
        "新 adapter 类型",
        "Web UI",
        "Web 仪表盘",
        "供应商 OAuth 登录",
        "OAuth 客户端凭据",
        "内核 plan mode",
        "MCP 运行时",
        "共享 opi-types crate",
        "整个 agent loop 重写",
    ];
    for needle in forbidden_en.iter().chain(forbidden_zh.iter()) {
        assert_docs_reject_claim(&doc_files, needle, "a Phase 8 non-goal");
    }

    // --- Code side: forbidden surfaces are not implemented. ---

    // Gather root + crate Cargo.toml files.
    let mut cargo_files: Vec<std::path::PathBuf> = vec![repo_root().join("Cargo.toml")];
    for entry in std::fs::read_dir(repo_root().join("crates")).expect("read crates directory") {
        let entry = entry.expect("dir entry");
        let path = entry.path().join("Cargo.toml");
        if path.is_file() {
            cargo_files.push(path);
        }
    }

    // No shared opi-types crate (no workspace member, no dependency).
    for path in &cargo_files {
        let cargo = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        assert!(
            !cargo.contains("opi-types"),
            "{} must not add an opi-types crate; Phase 8 forbids a shared types crate",
            path.display()
        );
    }

    // No opi-web-ui workspace member (web UI product work is out of scope).
    let root_cargo =
        std::fs::read_to_string(repo_root().join("Cargo.toml")).expect("root Cargo.toml");
    assert!(
        !root_cargo.contains("opi-web-ui"),
        "root Cargo.toml must not list opi-web-ui; Phase 8 forbids web UI product work"
    );

    // No OAuth client/credential crates (provider OAuth login is out of scope;
    // Vertex's bearer-token usage is not an OAuth client flow).
    for path in &cargo_files {
        let cargo = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for forbidden in ["oauth2", "openidconnect", "tame-oauth"] {
            assert!(
                !cargo.contains(forbidden),
                "{} must not depend on an OAuth client crate ({forbidden}); Phase 8 forbids provider OAuth work",
                path.display()
            );
        }
    }

    // No in-core workflow / web / oauth / mcp modules in opi-agent.
    let lib = read_repo_file("crates/opi-agent/src/lib.rs");
    for forbidden_mod in [
        "pub mod web_ui",
        "pub mod oauth",
        "pub mod mcp",
        "pub mod plan_mode",
        "pub mod sub_agent",
        "pub mod todo",
        "pub mod permission_popup",
    ] {
        assert!(
            !lib.contains(forbidden_mod),
            "opi-agent lib.rs must not declare {forbidden_mod}; Phase 8 forbids in-core workflow/web/oauth/mcp runtime"
        );
    }

    // No whole-loop rewrite: the public agent_loop entry point still exists.
    let agent_loop = read_repo_file("crates/opi-agent/src/agent_loop.rs");
    assert!(
        agent_loop.contains("pub async fn agent_loop"),
        "the public agent_loop entry point must still exist; Phase 8 forbids a whole-loop rewrite"
    );

    // No new adapter kind: the adapter protocol is still opi-extension-jsonl-v1 only.
    let adapter_protocol = read_repo_file("crates/opi-coding-agent/src/adapter_protocol.rs");
    assert!(
        adapter_protocol.contains("opi-extension-jsonl-v1"),
        "the adapter protocol must remain opi-extension-jsonl-v1"
    );
    for forbidden_proto in [
        "opi-extension-jsonl-v2",
        "opi-extension-mcp",
        "opi-extension-grpc",
        "opi-extension-websocket",
    ] {
        assert!(
            !adapter_protocol.contains(forbidden_proto),
            "adapter_protocol must not introduce a second adapter kind ({forbidden_proto})"
        );
    }
}

/// Meta-guard: the negation helper still rejects synthetic positive claims, so
/// the non-goal guards cannot be silently weakened. Pins the helper's reliable
/// behavior (positives without any negation token are rejected; clear
/// negations pass). See `no_positive_claim` for the documented substring
/// co-occurrence limitation.
#[test]
fn phase8_negation_helper_rejects_positive_claims() {
    // Positives with no negation token are rejected.
    assert!(
        !no_positive_claim("opi now ships a stable 1.0 API", "stable 1.0 API"),
        "a positive 1.0 claim must be rejected"
    );
    assert!(
        !no_positive_claim(
            "opi adds a TypeScript extension API",
            "TypeScript extension API"
        ),
        "a positive TypeScript extension API claim must be rejected"
    );
    assert!(
        !no_positive_claim("opi ships a web UI dashboard", "web UI dashboard"),
        "a positive web UI dashboard claim must be rejected"
    );
    assert!(
        !no_positive_claim("opi 提供 稳定 1.0 公共 API", "稳定 1.0 公共 API"),
        "a positive Chinese 1.0 claim must be rejected"
    );
    assert!(
        !no_positive_claim("opi 新增 共享 opi-types crate", "共享 opi-types crate"),
        "a positive Chinese opi-types claim must be rejected"
    );
    // Legitimate negation contexts pass.
    assert!(
        no_positive_claim("opi does not ship a stable 1.0 API", "stable 1.0 API"),
        "a clear English negation must pass the helper"
    );
    assert!(
        no_positive_claim("opi does not add a new adapter kind", "new adapter kind"),
        "a clear English negation must pass the helper"
    );
    assert!(
        no_positive_claim("opi 不得引入 MCP 运行时", "MCP 运行时"),
        "a clear Chinese negation must pass the helper"
    );
}

//! Documentation guard tests for the productized extension/package ecosystem
//! and the Phase 6 documentation-truth and version-synchronization workstreams.
//!
//! The Phase 5 tests verify that user-facing documentation describes the Phase 5
//! MVP truthfully and does NOT claim features that are not implemented. The
//! Phase 6 tests verify that current-state documentation identifies the
//! workspace/crate state at the current released version (matching the
//! workspace version) while historical release rows stay historical, and that
//! English and Chinese counterparts carry the same current-version claims.

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

fn readme_npm_line_has_clear_negation(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("not npm")
        || lower.contains("no npm")
        || lower.contains("does not use npm")
        || lower.contains("without npm")
        || lower.contains("without node, npm")
        || lower.contains("无需 node、npm")
        || lower.contains("不是 npm")
        || lower.contains("而不是 npm")
}

// ===========================================================================
// Negative guards: features that MUST NOT be claimed as complete
// ===========================================================================

#[test]
fn readme_does_not_claim_npm() {
    let en = read_repo_file("README.md");
    let zh = read_repo_file("README.zh.md");

    // npm is a pi mechanism that opi deliberately does not use.
    // The only acceptable mention is "not npm" / "no npm" / "does not use npm".
    for (lang, content) in [("en", &en), ("zh", &zh)] {
        // Raw "npm" should only appear in a negation context.
        if content.contains("npm") {
            // Allow: "does not", "without", "no ", "not " before npm
            let lower = content.to_lowercase();
            for line in lower.lines() {
                if line.contains("npm") {
                    assert!(
                        readme_npm_line_has_clear_negation(line),
                        "[{lang}] README mentions npm without clear negation: {line}"
                    );
                }
            }
        }
    }
}

#[test]
fn readme_does_not_claim_marketplace() {
    let en = read_repo_file("README.md");
    let zh = read_repo_file("README.zh.md");

    // No package marketplace exists.
    assert!(
        !contains_ci(&en, "marketplace"),
        "en README must not claim a package marketplace"
    );
    assert!(
        !contains_ci(&zh, "marketplace"),
        "zh README must not claim a package marketplace"
    );
    assert!(
        !contains_ci(&zh, "市场"),
        "zh README must not claim a package marketplace (市场)"
    );
}

#[test]
fn docs_do_not_claim_package_marketplace_or_gallery() {
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "package marketplace",
        "package gallery",
        "marketplace/gallery",
        "marketplace/registry",
        "registry-backed package sources",
        "package 市场",
        "package 画廊",
    ] {
        assert_docs_reject_claim(&files, needle, "a package marketplace or gallery");
    }
}

#[test]
fn readme_does_not_claim_hot_reload() {
    let en = read_repo_file("README.md");
    let zh = read_repo_file("README.zh.md");

    assert!(
        !contains_ci(&en, "hot reload"),
        "en README must not claim hot reload"
    );
    assert!(
        !contains_ci(&zh, "热重载"),
        "zh README must not claim hot reload"
    );
}

/// Helper: check that a forbidden phrase does not appear as a positive claim.
/// Allows matches only within a rejection/negation context
/// (lines containing "reject", "must not", "do not", etc.).
fn no_positive_claim(haystack: &str, needle: &str) -> bool {
    let lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    for line in lower.lines() {
        if line.contains(&needle_lower) {
            // If the line is itself a rejection/negation context, it's OK.
            // This covers exit criteria like "guard tests reject claims about X".
            if line.contains("reject")
                || line.contains("must not")
                || line.contains("do not")
                || line.contains("does not")
                || line.contains("not claim")
                || line.contains("不声明")
                || line.contains("不得")
                || line.contains("未实现")
                || line.contains("不支持")
                || line.contains("不提供")
                || line.contains("未引入")
                || line.contains("推迟")
            {
                continue;
            }
            return false;
        }
    }
    true
}

#[test]
fn positive_non_goal_claims_are_rejected_by_helpers() {
    assert!(
        !readme_npm_line_has_clear_negation("opi package add supports npm sources"),
        "an opi/npm positive claim must not be treated as negated just because it contains 'opi'"
    );
    assert!(
        !no_positive_claim(
            "opi now bundles Node without external dependencies",
            "bundles Node"
        ),
        "a positive bundled-runtime claim must not be treated as negated just because it says without"
    );
    assert!(
        !no_positive_claim(
            "TypeScript extension API compatibility is complete",
            "TypeScript extension API"
        ),
        "positive TypeScript extension API compatibility claims must be rejected"
    );
}

#[test]
fn spec_does_not_claim_provider_streaming_adapters() {
    let en = read_repo_file("docs/opi-spec.md");
    let zh = read_repo_file("docs/opi-spec.zh.md");

    // "provider streaming adapter" would mean adapters can intercept/modify
    // the LLM response stream. Phase 5 adapters only provide tools, commands,
    // hooks, and events.
    assert!(
        no_positive_claim(&en, "provider streaming adapter"),
        "opi-spec must not claim provider streaming adapters"
    );
    assert!(
        !contains_ci(&zh, "供应商流式适配器"),
        "opi-spec.zh must not claim provider streaming adapters"
    );
}

#[test]
fn spec_does_not_claim_custom_tui_adapters() {
    let en = read_repo_file("docs/opi-spec.md");
    let zh = read_repo_file("docs/opi-spec.zh.md");

    // TUI adapters (custom terminal rendering from packages) do not exist.
    assert!(
        no_positive_claim(&en, "TUI adapter"),
        "opi-spec must not claim TUI adapters"
    );
    assert!(
        !contains_ci(&zh, "TUI 适配器"),
        "opi-spec.zh must not claim TUI adapters"
    );
}

#[test]
fn docs_do_not_claim_package_permission_enforcement() {
    let en = read_repo_file("docs/opi-spec.md");
    let readme_en = read_repo_file("README.md");

    // "package permission enforcement" would mean the package system enforces
    // permission policies. Phase 5 only provides hooks that packages can use;
    // there is no built-in permission enforcement layer.
    assert!(
        no_positive_claim(&en, "package permission enforcement"),
        "opi-spec must not claim package permission enforcement"
    );
    assert!(
        !contains_ci(&readme_en, "package permission enforcement"),
        "README must not claim package permission enforcement"
    );
}

#[test]
fn spec_does_not_claim_hot_reload() {
    let en = read_repo_file("docs/opi-spec.md");

    assert!(
        no_positive_claim(&en, "hot reload"),
        "opi-spec must not claim hot reload"
    );
    assert!(
        no_positive_claim(&en, "hot-reload"),
        "opi-spec must not claim hot reload"
    );
}

#[test]
fn spec_documents_shutdown_contract_without_harness_overclaim() {
    let en = read_repo_file("docs/opi-spec.md");
    let zh = read_repo_file("docs/opi-spec.zh.md");

    assert!(
        !en.contains("On shutdown, the harness sends a `shutdown` message"),
        "opi-spec must not claim ordinary harness teardown sends adapter shutdown"
    );
    assert!(
        !zh.lines().any(|line| line.contains("harness")
            && line.contains("`shutdown`")
            && line.contains("发送")),
        "opi-spec.zh must not claim ordinary harness teardown sends adapter shutdown"
    );
    assert!(
        en.contains("Explicit `AdapterHost::shutdown`")
            && contains_ci(&en, "ordinary registry teardown")
            && en.contains("best-effort kill-only"),
        "opi-spec must document explicit shutdown separately from ordinary registry teardown"
    );
    assert!(
        zh.contains("`AdapterHost::shutdown`")
            && zh.contains("registry")
            && zh.contains("best-effort kill-only"),
        "opi-spec.zh must document explicit shutdown separately from ordinary registry teardown"
    );
}

// ===========================================================================
// Positive guards: Phase 5 MVP truth that MUST be present
// ===========================================================================

#[test]
fn readme_mentions_package_cli_commands() {
    let en = read_repo_file("README.md");

    // README should mention opi package commands.
    assert!(
        contains_ci(&en, "package add")
            || contains_ci(&en, "opi package")
            || contains_ci(&en, "package remove"),
        "en README must mention package CLI commands (package add/remove/list/doctor)"
    );
}

#[test]
fn readme_mentions_process_adapters() {
    let en = read_repo_file("README.md");

    // README should mention process adapters.
    assert!(
        contains_ci(&en, "process") && contains_ci(&en, "adapter"),
        "en README must mention process adapters"
    );
}

#[test]
fn spec_has_phase_five_roadmap() {
    let en = read_repo_file("docs/opi-spec.md");

    // opi-spec must have a Phase 5 section.
    assert!(
        contains_ci(&en, "Phase 5") || contains_ci(&en, "phase 5"),
        "opi-spec must include Phase 5 in the implementation roadmap"
    );
}

#[test]
fn spec_mentions_adapter_protocol() {
    let en = read_repo_file("docs/opi-spec.md");

    // opi-spec should mention the adapter JSONL protocol.
    assert!(
        contains_ci(&en, "opi-extension-jsonl"),
        "opi-spec must mention the opi-extension-jsonl-v1 adapter protocol"
    );
}

#[test]
fn spec_mentions_package_cli() {
    let en = read_repo_file("docs/opi-spec.md");

    assert!(
        contains_ci(&en, "package add")
            || contains_ci(&en, "package remove")
            || contains_ci(&en, "package list")
            || contains_ci(&en, "package doctor")
            || contains_ci(&en, "`opi package`"),
        "opi-spec must mention package CLI commands"
    );
}

#[test]
fn docs_warn_packages_are_trusted_code() {
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    assert!(
        readme.contains("Packages are trusted code"),
        "README must warn that packages are trusted code"
    );
    assert!(
        readme.contains("not enforced sandbox policy"),
        "README must warn that package code is not sandboxed"
    );
    assert!(
        readme_zh.contains("Package 是受信任代码"),
        "README.zh must warn that packages are trusted code"
    );
    assert!(
        readme_zh.contains("不是强制 sandbox 策略"),
        "README.zh must warn that package code is not sandboxed"
    );
    assert!(
        spec.contains("Packages are trusted code"),
        "opi-spec must warn that packages are trusted code"
    );
    assert!(
        spec.contains("not sandboxed"),
        "opi-spec must warn that package code is not sandboxed"
    );
    assert!(
        spec_zh.contains("Package 是受信任代码"),
        "opi-spec.zh must warn that packages are trusted code"
    );
    assert!(
        spec_zh.contains("不会被 sandbox"),
        "opi-spec.zh must warn that package code is not sandboxed"
    );
}

#[test]
fn changelog_mentions_phase_five_package_loop() {
    let changelog = read_repo_file("CHANGELOG.md");

    assert!(
        changelog.contains("opi package add/remove/list/doctor"),
        "CHANGELOG must mention package CLI lifecycle coverage"
    );
    assert!(
        changelog.contains("opi-extension-jsonl-v1"),
        "CHANGELOG must mention the adapter JSONL protocol"
    );
    assert!(
        changelog.contains("Adapter state snapshots"),
        "CHANGELOG must mention adapter state persistence"
    );
}

#[test]
fn docs_guard_package_lifecycle_claims() {
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    assert!(
        readme.contains("Package manifests can start `process-jsonl` adapters"),
        "README must summarize the package adapter capability"
    );
    assert!(
        readme_zh.contains("Package")
            && readme_zh.contains("manifest")
            && readme_zh.contains("process-jsonl"),
        "README.zh must summarize the package adapter capability"
    );

    assert!(
        spec.contains("validates the package manifest"),
        "opi-spec must say package add validates manifests"
    );
    assert!(
        spec.contains("writes a lock entry"),
        "opi-spec must say package add writes lock entries"
    );
    assert!(
        spec.contains("reads installed declarations and lock state"),
        "opi-spec must say runtime startup reads installed declarations and lock state"
    );
    assert!(
        spec_zh.contains("验证 package manifest"),
        "opi-spec.zh must say package add validates manifests"
    );
    assert!(
        spec_zh.contains("写入 lock 条目"),
        "opi-spec.zh must say package add writes lock entries"
    );
    assert!(
        spec_zh.contains("读取已安装声明和 lock 状态"),
        "opi-spec.zh must say runtime startup reads installed declarations and lock state"
    );

    for phrase in [
        "source availability",
        "lock consistency",
        "manifest V2",
        "resource containment",
        "opi version constraints",
        "adapter command resolution",
    ] {
        assert!(
            spec.contains(phrase),
            "opi-spec must say doctor validates {phrase}"
        );
    }
    for phrase in [
        "来源可用性",
        "lock 一致性",
        "manifest V2",
        "资源路径包含关系",
        "opi 版本约束",
        "adapter 命令解析",
    ] {
        assert!(
            spec_zh.contains(phrase),
            "opi-spec.zh must say doctor validates {phrase}"
        );
    }
}

// ===========================================================================
// Synchronization guards: EN/ZH must be in sync on key claims
// ===========================================================================

#[test]
fn readme_en_zh_both_mention_packages() {
    let en = read_repo_file("README.md");
    let zh = read_repo_file("README.zh.md");

    let en_has_package_cli = contains_ci(&en, "package add")
        || contains_ci(&en, "package remove")
        || contains_ci(&en, "opi package");
    let zh_has_package_cli = contains_ci(&zh, "package add")
        || contains_ci(&zh, "package remove")
        || contains_ci(&zh, "opi package");

    assert_eq!(
        en_has_package_cli, zh_has_package_cli,
        "EN and ZH READMEs must both mention package CLI commands"
    );
}

#[test]
fn spec_en_zh_both_have_phase_five() {
    let en = read_repo_file("docs/opi-spec.md");
    let zh = read_repo_file("docs/opi-spec.zh.md");

    let en_has = contains_ci(&en, "Phase 5") || contains_ci(&en, "phase 5");
    let zh_has =
        contains_ci(&zh, "Phase 5") || contains_ci(&zh, "phase 5") || contains_ci(&zh, "第五阶段");

    assert_eq!(
        en_has, zh_has,
        "EN and ZH opi-specs must both include Phase 5"
    );
}

// ===========================================================================
// Phase 6 guards: documentation truth and version synchronization
//
// Phase 6 Success Criteria 1 and 2 require current-state documentation to
// identify the workspace/crate state at the current released version (matching
// the workspace version) while historical release rows stay historical, and
// require English and Chinese counterparts to carry the same current-version
// claims. Lockstep versioning makes the compiled crate
// version the single source of truth, so both tests read it from
// `env!("CARGO_PKG_VERSION")` rather than hardcoding a number.
// ===========================================================================

#[test]
fn phase6_current_docs_match_workspace_version() {
    // Lockstep versioning: every crate shares the workspace version, so the
    // compiled crate version is the authoritative current version the docs that
    // describe the *current* implementation must match. Historical release
    // records (CHANGELOG sections, roadmap rows) are exempt and stay historical.
    let version = env!("CARGO_PKG_VERSION");

    // Root README names the current workspace version.
    let readme = read_repo_file("README.md");
    assert!(
        readme.contains(&format!(
            "The workspace package version in `Cargo.toml` is `{version}`"
        )),
        "README must name the current workspace version `{version}`"
    );
    assert!(
        read_repo_file("AGENTS.md").contains(&format!("Current workspace version: `{version}`")),
        "AGENTS.md is live agent context and must name the current workspace version `{version}`"
    );
    assert!(
        read_repo_file("CLAUDE.md").contains(&format!("Current workspace version: `{version}`")),
        "CLAUDE.md is live agent context and must name the current workspace version `{version}`"
    );

    // Each publishable crate README names its current crate version.
    for crate_name in ["opi-ai", "opi-agent", "opi-tui", "opi-coding-agent"] {
        let crate_readme = read_repo_file(&format!("crates/{crate_name}/README.md"));
        assert!(
            crate_readme.contains(&format!("Current crate version: `{version}`")),
            "{crate_name} README must name the current crate version `{version}`"
        );
    }

    // opi-spec describes the current workspace, not a historical release, in its
    // Document Control "Current implementation" row, its Current Baseline
    // versioning row, and its Phase 4/5 status lines.
    let spec = read_repo_file("docs/opi-spec.md");
    let current_impl = spec
        .lines()
        .find(|line| line.contains("Current implementation"))
        .expect("opi-spec must have a Current implementation row");
    assert!(
        current_impl.contains(&format!("{version} workspace")),
        "opi-spec Current implementation row must describe the {version} workspace, got: {current_impl}"
    );
    assert!(
        spec.contains(&format!("| Versioning | lockstep `{version}` |")),
        "opi-spec Current Baseline versioning must be lockstep `{version}`"
    );
    assert!(
        spec.contains(&format!("current `{version}` workspace")),
        "opi-spec Phase 4/5 status lines must reference the current `{version}` workspace"
    );

    // Historical 0.5.0 release row is preserved, not rewritten to the current version.
    assert!(
        read_repo_file("CHANGELOG.md").contains("## [0.5.0]"),
        "CHANGELOG must preserve the historical 0.5.0 release section"
    );
}

#[test]
fn phase6_localized_docs_stay_in_sync() {
    // Every Phase 6 current-version claim made in English
    // documentation must be carried by its Chinese counterpart in the same form.
    // Assertions are per-language and positive, so a stale-but-matched pair
    // (both EN and ZH wrong) cannot satisfy the sync requirement.
    let version = env!("CARGO_PKG_VERSION");

    // Root README.
    assert!(
        read_repo_file("README.zh.md")
            .contains(&format!("`Cargo.toml` 中的 workspace 包版本是 `{version}`")),
        "README.zh must name the current workspace version `{version}`"
    );

    // Publishable crate READMEs.
    for crate_name in ["opi-ai", "opi-agent", "opi-tui", "opi-coding-agent"] {
        let crate_readme_zh = read_repo_file(&format!("crates/{crate_name}/README.zh.md"));
        assert!(
            crate_readme_zh.contains(&format!("当前 crate 版本是 `{version}`")),
            "{crate_name} README.zh must name the current crate version `{version}`"
        );
    }

    // opi-spec Document Control "Current implementation" row.
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    assert!(
        spec_zh
            .lines()
            .any(|line| line.contains("当前实现") && line.contains(&format!("{version} workspace"))),
        "opi-spec.zh Current implementation row must reference the {version} workspace"
    );
}

#[test]
fn current_docs_do_not_reference_removed_web_ui_crate() {
    let removed_crate = ["opi", "web", "ui"].join("-");
    let current_docs = [
        "README.md",
        "README.zh.md",
        "AGENTS.md",
        "CLAUDE.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
        ".claude/skills/opi-release/skill.md",
    ];

    for path in current_docs {
        let content = read_repo_file(path);
        assert!(
            !content.contains(&removed_crate),
            "{path} must not describe the removed web-facing crate as current"
        );
    }

    assert!(
        !read_repo_file("Cargo.toml").contains(&removed_crate),
        "workspace Cargo.toml must not include the removed web-facing crate"
    );
    assert!(
        !repo_root().join("crates").join(&removed_crate).exists(),
        "removed web-facing crate directory must not exist"
    );
}

// ===========================================================================
// Phase 6 task 6.6: alignment guards and final Phase 6 gates.
//
// These guards implement the Phase 6 design Workstream 6 (Alignment Guards),
// Success Criteria 7 (guards prevent overclaiming deferred pi ecosystem
// features), Success Criteria 9 (no npm/marketplace/OAuth/provider parity,
// permission-enforcement, TS-extension-compat, or new shared type crate is
// added in Phase 6), and the DoD's expanded coverage of every Phase 6 non-goal.
//
// Negative doc guards reject current-phase claims for each non-goal. Because
// the docs are already clean, these guards also serve as regression guards;
// each is written so a hypothetical positive claim (e.g. "opi supports OAuth
// parity") would fail while legitimate negations ("OAuth remains a separate
// product decision") pass. Negative code guards assert that no non-goal
// implementation exists in the workspace. A positive guard keeps the Phase 5
// MVP adapter capability surface documented.
// ===========================================================================

/// Helper: the repository root (two levels up from the test crate).
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Helper: assert that no file in `files` positively claims the forbidden
/// `needle`. Uses [`no_positive_claim`] so legitimate negations pass.
fn assert_docs_reject_claim(files: &[&str], needle: &str, what: &str) {
    for path in files {
        let content = read_repo_file(path);
        assert!(
            no_positive_claim(&content, needle),
            "{path} must not positively claim {what}; forbidden phrase {needle:?} appeared outside a negation context"
        );
    }
}

// ---------------------------------------------------------------------------
// Negative doc guards: deferred ecosystem features must not be claimed complete
// ---------------------------------------------------------------------------

#[test]
fn docs_do_not_claim_package_update_enable_disable() {
    // Phase 5 ships add/remove/list/doctor only. update/enable/disable are
    // deferred ecosystem candidates and must not be claimed as commands.
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "opi package update",
        "opi package enable",
        "opi package disable",
        "update a package",
        "enable a package",
        "disable a package",
        "package 更新",
        "package 启用",
        "package 禁用",
    ] {
        assert_docs_reject_claim(&files, needle, "a package update/enable/disable command");
    }
}

#[test]
fn docs_do_not_claim_bundled_js_ts_runtime() {
    // The core binary must not bundle a Node.js/TypeScript/jiti runtime.
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "bundled Node",
        "bundled TypeScript",
        "bundled JavaScript",
        "bundled Node.js",
        "bundles Node",
        "bundles TypeScript",
        "jiti runtime",
        "Node.js runtime",
        "TypeScript runtime",
        "JavaScript runtime",
        "ships Node",
        "includes Node.js",
        "embeds Node",
        "内置 Node",
        "内置 TypeScript",
        "内置 JavaScript",
    ] {
        assert_docs_reject_claim(&files, needle, "a bundled Node.js/TypeScript/jiti runtime");
    }
}

#[test]
fn docs_do_not_claim_ts_extension_api_compat() {
    // opi is not TypeScript-extension-API compatible with pi.
    let files = ["docs/opi-spec.md", "docs/opi-spec.zh.md"];
    // "TypeScript extension API" / "TypeScript-compatible" scope the claim to the
    // extension surface; the bare "TypeScript API compatibility" phrase is
    // intentionally avoided because docs legitimately disclaim it ("It is not a
    // TypeScript API compatibility checklist").
    for needle in [
        "TypeScript extension API",
        "TypeScript extension compatibility",
        "TypeScript-compatible",
    ] {
        assert_docs_reject_claim(&files, needle, "TypeScript extension API compatibility");
    }
}

#[test]
fn docs_do_not_claim_pi_session_v3_compat() {
    // opi session JSONL is Rust-native and does not promise pi session v3
    // read/write file compatibility.
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "pi session v3 compatibility",
        "session v3 read/write compatibility",
        "supports pi session v3",
        "reads pi session v3",
        "writes pi session v3",
        "兼容 pi session v3",
    ] {
        assert_docs_reject_claim(&files, needle, "pi session v3 file compatibility");
    }
}

#[test]
fn docs_do_not_claim_broad_oauth_provider_parity() {
    // OAuth and broad provider coverage are deferred/separate product decisions.
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "OAuth parity",
        "provider parity",
        "parity with OAuth",
        "broad OAuth",
        "provider coverage parity",
        "OAuth 对等",
        "provider 对等",
    ] {
        assert_docs_reject_claim(&files, needle, "broad OAuth or provider parity");
    }
}

#[test]
fn docs_do_not_claim_opi_types_or_protocol_migration() {
    // Adapter protocol types must stay in opi-coding-agent. Needles are scoped
    // to positive migration/creation claims so the legitimate "Why There Is No
    // opi-types" section is not tripped.
    let files = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    for needle in [
        "moved to opi-types",
        "migrated to opi-types",
        "introduced opi-types",
        "opi-types crate",
        "extracted into opi-types",
        "shared opi-types crate",
        "adapter protocol types now live in opi-types",
    ] {
        assert_docs_reject_claim(
            &files,
            needle,
            "migration of adapter protocol types to an opi-types crate",
        );
    }
}

// ---------------------------------------------------------------------------
// Negative code guards: no Phase 6 non-goal implementation exists (SC 9)
// ---------------------------------------------------------------------------

#[test]
fn workspace_has_no_opi_types_crate() {
    // Phase 6 forbids a shared types crate.
    let cargo = read_repo_file("Cargo.toml");
    assert!(
        !cargo.contains("opi-types") && !cargo.contains("opi_types"),
        "root Cargo.toml must not declare an opi-types workspace member or dependency"
    );
    assert!(
        !repo_root().join("crates/opi-types").exists(),
        "no crates/opi-types directory may exist (Phase 6 forbids a shared types crate)"
    );
}

#[test]
fn workspace_has_no_bundled_js_ts_runtime() {
    // A bundled JS/TS runtime would pull in one of these crates. Phase 6 forbids it.
    let cargo_files: Vec<_> = std::iter::once(repo_root().join("Cargo.toml"))
        .chain(
            std::fs::read_dir(repo_root().join("crates"))
                .expect("read crates directory")
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path().join("Cargo.toml");
                    path.is_file().then_some(path)
                }),
        )
        .collect();
    for forbidden in [
        "jiti",
        "deno_core",
        "deno_runtime",
        "boa_engine",
        "rquickjs",
        "rusty_v8",
        "swc",
        "oxc",
        "neon",
        "napi",
    ] {
        for path in &cargo_files {
            let cargo = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            assert!(
                !cargo.contains(forbidden),
                "{} must not depend on a JS/TS runtime crate ({forbidden}); Phase 6 forbids a bundled Node.js/TypeScript/jiti runtime",
                path.display()
            );
        }
    }
}

#[test]
fn first_class_provider_set_is_unchanged() {
    // First-class providers arrive only as the known nine. Any additional
    // provider module under opi-ai/src would indicate core provider broadening
    // (a Phase 6 non-goal); custom providers must use runtime registration.
    let providers_dir = repo_root().join("crates/opi-ai/src");
    let known_providers: std::collections::BTreeSet<&str> = [
        "anthropic",
        "azure_openai",
        "bedrock",
        "gemini",
        "mistral",
        "openai_chat",
        "openai_responses",
        "openrouter",
        "vertex",
    ]
    .into_iter()
    .collect();
    let infra: std::collections::BTreeSet<&str> = [
        "lib",
        "config",
        "endpoint",
        "http",
        "message",
        "model",
        "model_info",
        "provider",
        "provider_collection",
        "registry",
        "retry",
        "stream",
        "test_support",
        "time",
        // Phase 14 provider/auth and API-mapped routing contracts; these are
        // support modules, not additional first-class provider families.
        "api_mapped",
        "auth",
        "credential",
        // OpenAI Codex uses a dedicated Responses wire while sharing only
        // wire-neutral conversion/SSE infrastructure with standard Responses.
        // Neither module broadens the general first-class provider family set.
        "openai_codex_responses",
        "openai_responses_shared",
        "provider_headers",
    ]
    .into_iter()
    .collect();

    let mut actual_providers = std::collections::BTreeSet::<String>::new();
    let entries = std::fs::read_dir(&providers_dir)
        .unwrap_or_else(|e| panic!("failed to read opi-ai/src: {e}"));
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let stem = name.strip_suffix(".rs").map(str::to_owned).unwrap_or(name);
        if infra.contains(stem.as_str()) {
            continue;
        }
        assert!(
            known_providers.contains(stem.as_str()),
            "opi-ai/src has an unexpected module '{stem}'; the first-class provider set must stay at the nine known providers (a new first-class provider is a Phase 6 non-goal)"
        );
        actual_providers.insert(stem);
    }
    for provider in known_providers {
        assert!(
            actual_providers.contains(provider),
            "expected first-class provider module '{provider}' is missing from opi-ai/src"
        );
    }
}

#[test]
fn adapter_protocol_types_stay_in_coding_agent() {
    // The adapter protocol module remains a coding-agent product surface and
    // must not migrate into opi-agent or a shared crate during Phase 6.
    let root = repo_root();
    assert!(
        root.join("crates/opi-coding-agent/src/adapter_protocol.rs")
            .exists(),
        "adapter_protocol.rs must remain in opi-coding-agent"
    );
    assert!(
        !root
            .join("crates/opi-agent/src/adapter_protocol.rs")
            .exists(),
        "opi-agent must not host adapter_protocol.rs; protocol types stay in opi-coding-agent"
    );
    let agent_cargo = read_repo_file("crates/opi-agent/Cargo.toml");
    assert!(
        !agent_cargo.contains("adapter_protocol"),
        "opi-agent Cargo.toml must not reference adapter_protocol"
    );
}

// ---------------------------------------------------------------------------
// Positive guard: Phase 5 MVP adapter capability surface stays documented
// ---------------------------------------------------------------------------

#[test]
fn docs_describe_phase5_adapter_capability_surface() {
    let spec = read_repo_file("docs/opi-spec.md");
    // Phase 5 adapters bridge the full capability surface; docs must keep
    // stating each capability so the MVP claim remains truthful.
    for phrase in [
        "`opi package add/remove/list/doctor` works",
        "packages with `[adapter]` sections start as child processes using `opi-extension-jsonl-v1`",
        "tools, commands, hooks, and events through child process adapters",
        "adapter tools, commands, hooks, state, and cancellation bridge into the existing extension API",
        "before_tool_call",
        "after_tool_call",
        "transform_context",
        "prepare_next_turn",
    ] {
        assert!(
            spec.to_lowercase().contains(&phrase.to_lowercase()),
            "opi-spec must describe the Phase 5 adapter capability surface (missing: {phrase})"
        );
    }
}

// ===========================================================================
// Phase 9 guards: pi 0.80.2 baseline realignment documentation gates.
//
// These guards implement the Phase 9 design Testing and Guard Strategy and
// Success Criteria 1-8. They assert that the normative spec (`docs/opi-spec.md`)
// and its Chinese counterpart name `.repo/pi-0.80.2` as the current upstream,
// keep the Phase 9-14 roadmap consistent, document future ecosystem candidates
// with entry conditions, and reject current-scope overclaims for deferred
// ecosystem breadth (OAuth parity, image generation, custom extension UI parity,
// npm/gallery, web/share, and pi session compatibility). Phase 9 is
// documentation-only; Success Criterion 9
// (no runtime behavior change) is enforced by the task's no-runtime-scope
// library gate rather than a Rust test.
//
// Tasks 9.1-9.3 already landed the documentation, so these guards pass green
// and then serve as permanent regression guards. Each is written so a
// hypothetical regression (for example re-adding `.repo/pi-0.75.3` as the
// current baseline, or claiming OAuth/image-generation parity) would fail.
// ===========================================================================

/// Helper for SC 2: true when a line pairs the older `.repo/pi-0.75.3` snapshot
/// with a current-baseline row marker, i.e. claims the stale snapshot as the
/// CURRENT studied upstream. Legitimate prior-baseline mentions (for example
/// "compared with the older `.repo/pi-0.75.3` baseline") do not pair the
/// snapshot with a current-baseline marker and are allowed.
fn line_claims_pi_0753_as_current_baseline(line: &str) -> bool {
    if !line.contains("pi-0.75.3") {
        return false;
    }
    let lower = line.to_lowercase();
    // Current-baseline row markers from the matrix/spec document-control tables
    // and current-baseline prose, in English and Chinese.
    lower.contains("upstream path")
        || lower.contains("upstream studied")
        || lower.contains("current baseline")
        || lower.contains("current upstream")
        || lower.contains("studied upstream")
        || lower.contains("上游路径")
        || lower.contains("参考上游")
        || lower.contains("当前基线")
        || lower.contains("当前上游")
}

/// Helper: collapse runs of whitespace (including line-wrap newlines) to single
/// spaces so prose-phrase assertions are robust to markdown rewrapping.
fn ws_normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn phase9_current_baseline_is_pi_0_80_2() {
    // SC 2: current-baseline statements name `.repo/pi-0.80.2`; the older
    // `.repo/pi-0.75.3` snapshot may appear only as historical prior-baseline
    // context, never as the current studied upstream baseline.
    let docs = ["docs/opi-spec.md", "docs/opi-spec.zh.md"];

    for path in docs {
        let content = read_repo_file(path);

        // Current baseline must name pi-0.80.2.
        assert!(
            content.contains(".repo/pi-0.80.2"),
            "{path} must name `.repo/pi-0.80.2` as the current upstream baseline"
        );

        // No line may claim pi-0.75.3 as the CURRENT baseline.
        for line in content.lines() {
            assert!(
                !line_claims_pi_0753_as_current_baseline(line),
                "{path} must not describe `.repo/pi-0.75.3` as the current upstream baseline: {line}"
            );
        }
    }

    // The normative document-control rows positively anchor pi-0.80.2 in both
    // languages.
    assert!(
        read_repo_file("docs/opi-spec.md")
            .contains("| Upstream studied | `pi` 0.80.2 at `.repo/pi-0.80.2/`"),
        "opi-spec Document Control must name `pi` 0.80.2 at `.repo/pi-0.80.2/` as upstream studied"
    );
    assert!(
        read_repo_file("docs/opi-spec.zh.md")
            .contains("| 参考上游 | `pi` 0.80.2，位于 `.repo/pi-0.80.2/`"),
        "opi-spec.zh Document Control must name `pi` 0.80.2 at `.repo/pi-0.80.2/` as upstream studied"
    );
}

#[test]
fn phase9_localized_docs_stay_in_sync() {
    // SC 3 + Normative Documentation Changes: English and Chinese normative
    // docs carry equivalent baseline, roadmap, dashboard, and non-goal
    // statements. Assertions are per-language and positive, so a stale-but-
    // matched pair (both EN and ZH wrong) cannot satisfy the sync requirement.
    let spec_en = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    // Phase 9 section headings.
    assert!(
        spec_en.contains("### Phase 9 - pi 0.80.2 Baseline Realignment"),
        "opi-spec must include the Phase 9 baseline realignment section"
    );
    assert!(
        spec_zh.contains("### 第九阶段 - pi 0.80.2 基线重校准"),
        "opi-spec.zh must include the Phase 9 baseline realignment section"
    );

    // Future ecosystem section headings.
    assert!(
        spec_en.contains("### Future Ecosystem Candidates"),
        "opi-spec must include the Future Ecosystem Candidates section"
    );
    assert!(
        spec_zh.contains("### 未来生态候选"),
        "opi-spec.zh must include the Future Ecosystem Candidates section"
    );

    // Non-goal framing stays synchronized: both specs list the deferred
    // ecosystem breadth (npm/gallery workflow + web/share flow) as out of
    // current scope in the Phase 9 section.
    assert!(
        spec_en.contains("npm/gallery workflow") && spec_en.contains("web/share flow"),
        "opi-spec Phase 9 section must list npm/gallery and web/share as out of current scope (EN)"
    );
    assert!(
        spec_zh.contains("npm/gallery 工作流") && spec_zh.contains("web/share 流程"),
        "opi-spec.zh Phase 9 section must list npm/gallery and web/share as out of current scope (ZH)"
    );
}

#[test]
fn phase9_roadmap_numbering_consistent() {
    // SC 4 + 6 + Revised Roadmap: the roadmap consistently lists Phase 9-17
    // with the revised names, and Models/Auth + AgentHarness are named as the
    // Phase 10 deepening targets.
    let spec_en = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    let en_headings = [
        "### Phase 9 - pi 0.80.2 Baseline Realignment",
        "### Phase 10 - Core Architecture Deepening",
        "### Phase 11 - Tooling Quality",
        "### Phase 12 - Provider Correctness",
        "### Phase 13 - Session Tree and Context Reconstruction",
        "### Phase 14 - Provider & Auth",
        "### Phase 15 - Safety & Sandbox",
        "### Phase 16 - Agent Intelligence",
        "### Phase 17 - TUI Engine, Extension Surface, and Product Polish",
    ];
    let zh_headings = [
        "### 第九阶段 - pi 0.80.2 基线重校准",
        "### 第十阶段 - 核心架构深化",
        "### 第十一阶段 - 工具质量",
        "### 第十二阶段 - Provider 正确性",
        "### 第十三阶段 - 会话树与上下文重建",
        "### 第十四阶段 - Provider & Auth",
        "### 第十五阶段 - Safety & Sandbox",
        "### 第十六阶段 - Agent Intelligence",
        "### 第十七阶段 - TUI 引擎、扩展表面与产品打磨",
    ];
    for heading in en_headings {
        assert!(
            spec_en.contains(heading),
            "opi-spec must include roadmap heading `{heading}`"
        );
    }
    for heading in zh_headings {
        assert!(
            spec_zh.contains(heading),
            "opi-spec.zh must include roadmap heading `{heading}`"
        );
    }

    // Spec Phase 10 workstream table names both seams with their owning crate.
    assert!(
        spec_en.contains("| `Models/Auth` seam | `opi-ai` |"),
        "opi-spec Phase 10 must name the Models/Auth seam owned by opi-ai"
    );
    assert!(
        spec_en.contains("Generic `AgentHarness` | `opi-agent`"),
        "opi-spec Phase 10 must name the generic AgentHarness owned by opi-agent"
    );
    assert!(
        spec_zh.contains("| `Models/Auth` 缝合点 | `opi-ai` |"),
        "opi-spec.zh Phase 10 must name the Models/Auth seam owned by opi-ai"
    );
    assert!(
        spec_zh.contains("通用 `AgentHarness` | `opi-agent`"),
        "opi-spec.zh Phase 10 must name the generic AgentHarness owned by opi-agent"
    );
}

#[test]
fn phase9_future_ecosystem_candidates_have_entry_conditions() {
    // SC 7 + Future Ecosystem Candidates: future ecosystem breadth (OAuth,
    // broad provider catalog, image generation, custom extension UI,
    // npm/gallery, web/share, provider hooks, pi session import) is documented
    // with entry conditions, not as scheduled near-term phase promises.
    let spec_en = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    // Section headings present in both languages.
    assert!(
        spec_en.contains("### Future Ecosystem Candidates"),
        "opi-spec must include the Future Ecosystem Candidates section"
    );
    assert!(
        spec_zh.contains("### 未来生态候选"),
        "opi-spec.zh must include the Future Ecosystem Candidates section"
    );
    // Entry-condition column header present.
    assert!(
        spec_en.contains("| Candidate | Entry condition |"),
        "opi-spec Future Ecosystem Candidates must have an Entry condition column"
    );
    // Non-committal framing: candidates are NOT scheduled phases yet.
    assert!(
        spec_en.contains("not scheduled phases"),
        "opi-spec must frame future ecosystem candidates as not-yet-scheduled"
    );
    assert!(
        spec_zh.contains("不是已排期阶段"),
        "opi-spec.zh must frame future ecosystem candidates as not-yet-scheduled"
    );

    // Each candidate is named in the spec and the matrix.
    for candidate in [
        "Provider OAuth",
        "Broad provider catalog",
        "Image generation",
        "Custom extension UI",
        "npm/gallery",
        "Web/share",
        "session import/migration",
    ] {
        assert!(
            contains_ci(&spec_en, candidate),
            "opi-spec Future Ecosystem Candidates must name `{candidate}`"
        );
    }
}

#[test]
fn phase9_forbidden_current_scope_claims_rejected() {
    // SC 8 + Non-Goals + Testing and Guard Strategy: docs guard tests reject
    // current-scope overclaims for OAuth parity, image generation, custom
    // extension UI parity, npm/gallery, web/share, and pi session
    // compatibility. Needles are scoped to parity/compatibility claim phrases
    // so legitimate "Missing / future candidate / does not support / explicitly
    // exclude" framing is not tripped. The existing Phase 5/6 guards in this
    // file continue to reject bare npm/marketplace/update/enable/disable and
    // TypeScript-extension-API claims.
    let docs = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];

    // Parity / compatibility overclaims for deferred ecosystem breadth.
    for needle in [
        "OAuth parity",
        "OAuth 对等",
        "image generation parity",
        "图像生成对等",
        "supports image generation",
        "支持图像生成",
        "web UI parity",
        "web/share parity",
        "web UI 对等",
        "pi session compatibility",
        "pi session v3 compatibility",
    ] {
        assert_docs_reject_claim(
            &docs,
            needle,
            "a deferred-ecosystem parity/compat overclaim",
        );
    }

    // Positive counterpart: the spec executive summary explicitly disclaims the
    // ecosystem breadth that remains out of current scope. Phase 14 implements
    // OAuth login for three approved profiles, while the parity guard above
    // continues to reject broad OAuth/provider parity. These prose claims are
    // checked against whitespace-normalized content so markdown rewrapping does
    // not weaken the guard.
    let spec_en = ws_normalized(&read_repo_file("docs/opi-spec.md"));
    let spec_zh = ws_normalized(&read_repo_file("docs/opi-spec.zh.md"));
    assert!(
        spec_en.contains("does not claim pi package ecosystem parity"),
        "opi-spec must disclaim pi package ecosystem parity (EN)"
    );
    assert!(
        spec_en.contains("does not support npm package install"),
        "opi-spec must disclaim npm package install support (EN)"
    );
    assert!(
        spec_en.contains("image generation") && spec_en.contains("web/share flows"),
        "opi-spec must list image generation and web/share flows as not supported (EN)"
    );
    assert!(
        spec_zh.contains("不声称 pi package 生态对等"),
        "opi-spec.zh must disclaim pi package ecosystem parity (ZH)"
    );
    assert!(
        spec_zh.contains("也不支持 npm package 安装"),
        "opi-spec.zh must disclaim npm package install support (ZH)"
    );
    assert!(
        spec_zh.contains("图像生成或 web/share 流程"),
        "opi-spec.zh must list image generation and web/share flows as not supported (ZH)"
    );

    // Custom extension UI parity is explicitly excluded from Phase 17 scope, not
    // claimed as a current capability.
    assert!(
        spec_en.contains("does not promise web UI parity"),
        "opi-spec Phase 17 must disclaim web UI / custom extension UI parity (EN)"
    );
    assert!(
        spec_zh.contains("不声明 web UI parity"),
        "opi-spec.zh Phase 17 must disclaim web UI / custom extension UI parity (ZH)"
    );
}

// ===========================================================================
// Phase 10 guards: runtime hook boundary documentation + source structure.
//
// Workstream 10.4 requires runtime hook boundaries to be documented and tested:
// core loop hooks stay narrow in opi-agent, coding-agent product extensions and
// process adapter hosting stay in opi-coding-agent, typed hook result
// composition is tested where it affects runtime behavior (covered by the
// Phase 8.2 hook-order/short-circuit contract tests in opi-agent/tests plus the
// adapter_runtime product-adapter-boundary suite), and provider request/
// response hooks plus custom TUI UI/message renderer hooks stay deferred with
// explicit prerequisites. These guards pin the documentation claims and the
// positive source-structure assertion that process adapter protocol parsing/
// hosting remains out of opi-agent.
// ===========================================================================

/// Strip Rust line and (nested) block comments while preserving string/char
/// literal contents, so doc prose naming adapter types does not trip the
/// source-structure scan. Mirrors the task 10.5 session_facade boundary guard.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            } else if bytes[i + 1] == b'*' {
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }
        if c == b'"' || c == b'\'' {
            let quote = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(bytes[i] as char);
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Recursively collect `.rs` file paths under `dir`.
fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn phase10_runtime_hook_boundaries() {
    // SC 6 + Workstream 10.4 SC1/SC2: a dedicated normative section documents
    // the 6-surface runtime hook boundary model (EN + ZH), the provider-hook
    // and UI/renderer deferral prerequisites exist, and no doc implies pi
    // TypeScript extension API surfaces are current opi scope.
    let spec_en = ws_normalized(&read_repo_file("docs/opi-spec.md"));
    let spec_zh = ws_normalized(&read_repo_file("docs/opi-spec.zh.md"));

    // (a) Dedicated boundary section naming the 6 surfaces + the narrowness and
    // no-migration claims (EN).
    assert!(
        spec_en.contains("Runtime hook boundaries"),
        "opi-spec must have a dedicated Runtime hook boundaries section (EN)"
    );
    for phrase in [
        "Core loop hooks",
        "Generic harness events/results",
        "Coding-agent extension registry",
        "Process adapter protocol",
        "Provider request/response hooks",
        "Custom TUI UI / message renderer",
    ] {
        assert!(
            spec_en.contains(phrase),
            "opi-spec Runtime hook boundaries section must name the `{phrase}` surface (EN)"
        );
    }
    assert!(
        spec_en.contains("Contract-tested and narrow"),
        "opi-spec must state core loop hooks stay narrow in opi-agent (EN)"
    );
    assert!(
        spec_en.contains("does not migrate into `opi-agent`"),
        "opi-spec must state the process adapter does not migrate into opi-agent (EN)"
    );

    // (b) ZH counterpart carries the same boundary model.
    assert!(
        spec_zh.contains("运行时钩子边界"),
        "opi-spec.zh must have a dedicated runtime hook boundaries section (ZH)"
    );
    for phrase in [
        "核心循环钩子",
        "进程适配器协议",
        "Provider 请求/响应钩子",
        "自定义 TUI UI / 消息渲染器",
    ] {
        assert!(
            spec_zh.contains(phrase),
            "opi-spec.zh runtime hook boundaries section must name the `{phrase}` surface (ZH)"
        );
    }

    // (c) Provider-hook + UI/renderer DEFERRAL prerequisites are documented in
    // the Future Ecosystem Candidates table (presence checks so the
    // prerequisite text must accompany each deferral).
    assert!(
        spec_en.contains("Provider request/response adapter hooks")
            && spec_en.contains("hook ordering, redaction, and trace semantics are stable"),
        "opi-spec must document the provider request/response hook deferral prerequisite (EN)"
    );
    assert!(
        spec_en.contains("Custom extension UI / message renderer")
            && spec_en.contains("Phase 17 built-in TUI is stable"),
        "opi-spec must document the custom UI/message renderer deferral prerequisite (EN)"
    );

    // (d) SC1: docs must not imply pi TypeScript extension API surfaces are
    // current opi scope. Complementary to the Phase 5/9 TypeScript-extension
    // guards already in this file.
    let docs = ["docs/opi-spec.md", "docs/opi-spec.zh.md"];
    for needle in [
        "TypeScript extension API compatibility",
        "TypeScript 扩展 API 兼容",
        "pi TypeScript extension API parity",
    ] {
        assert_docs_reject_claim(
            &docs,
            needle,
            "a pi TypeScript extension API current-scope claim",
        );
    }
}

#[test]
fn phase10_process_adapter_stays_out_of_opi_agent() {
    // Workstream 10.4 SC3 + Crate Boundary Rules: process adapter protocol
    // parsing and hosting remain out of opi-agent unless a concrete non-CLI
    // host needs them. Positive source-structure assertion: a comment-stripped
    // scan of opi-agent/src finds ZERO adapter tokens, while the same tokens
    // remain present in opi-coding-agent/src (non-vacuous sanity) and the
    // narrow core loop hook trait lives in opi-agent.
    let opi_agent_src = repo_root().join("crates/opi-agent/src");
    let opi_coding_src = repo_root().join("crates/opi-coding-agent/src");

    // CamelCase type names + the protocol string + the startup fn + the kind
    // are unambiguous: opi-agent's only adapter-adjacent text is the
    // `adapter_protocol_unsupported` / `adapter_host_diagnostic` snake_case
    // diagnostic constants (substrings that do not match these tokens) and a
    // doc-comment name-drop of `ProcessAdapter` (removed by strip_rust_comments).
    let adapter_tokens = [
        "AdapterHost",
        "ProcessAdapter",
        "AdapterCapabilities",
        "AdapterManifest",
        "start_adapters_from_packages",
        "opi-extension-jsonl-v1",
        "process-jsonl",
    ];

    let mut agent_files = Vec::new();
    collect_rs_files(&opi_agent_src, &mut agent_files);
    assert!(
        !agent_files.is_empty(),
        "expected opi-agent src files to scan"
    );
    for file in &agent_files {
        let src = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
        let stripped = strip_rust_comments(&src);
        for token in adapter_tokens {
            assert!(
                !stripped.contains(token),
                "adapter token `{token}` leaked into opi-agent non-comment code at {} \
                 (process adapter hosting must stay in opi-coding-agent)",
                file.display()
            );
        }
    }

    // Non-vacuous sanity: the same tokens DO live in opi-coding-agent, and the
    // narrow core loop hook trait lives in opi-agent.
    let mut coding_files = Vec::new();
    collect_rs_files(&opi_coding_src, &mut coding_files);
    assert!(
        !coding_files.is_empty(),
        "expected opi-coding-agent src files"
    );
    let coding_blob: String = coding_files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    for token in ["AdapterHost", "ProcessAdapter", "opi-extension-jsonl-v1"] {
        assert!(
            coding_blob.contains(token),
            "non-vacuous sanity: `{token}` must remain present in opi-coding-agent/src"
        );
    }
    let agent_blob: String = agent_files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        agent_blob.contains("trait AgentHooks"),
        "non-vacuous sanity: the narrow AgentHooks core loop trait must live in opi-agent/src"
    );
}

#[test]
fn phase10_forbidden_current_scope_claims_rejected() {
    // Phase 10 SC8 (top-level) + the 11 Non-Goals: documentation guards reject
    // current-scope overclaims for the Phase-10-NOVEL deferred surfaces. The
    // overlap set (OAuth parity, image generation, npm, web, pi session
    // compatibility, pi TypeScript extension API) is already rejected by the
    // Phase 5/9 guards and the phase10_runtime_hook_boundaries guard in this
    // file; this test covers the remaining novel non-goals (subscription auth,
    // broad provider catalog, custom TUI extension protocol, shared opi-types
    // crate, whole-loop rewrite, current-scope OAuth login).
    //
    let docs = [
        "README.md",
        "README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
    ];
    let claims: [(&[&str], &[&str]); 6] = [
        (
            &["provider OAuth login", "provider OAuth 登录"],
            &[
                "supports",
                "implements",
                "provides",
                "ships",
                "支持",
                "实现",
                "提供",
                "发布",
            ],
        ),
        (
            &["subscription auth"],
            &[
                "supports",
                "implements",
                "provides",
                "ships",
                "支持",
                "实现",
                "提供",
                "发布",
            ],
        ),
        (
            &[
                "broad provider catalog",
                "provider catalog",
                "provider catalog expansion",
                "广泛 provider catalog",
                "provider catalog 扩张",
            ],
            &[
                "supports",
                "implements",
                "provides",
                "ships",
                "expands",
                "adds",
                "支持",
                "实现",
                "提供",
                "发布",
                "扩张",
            ],
        ),
        (
            &[
                "custom TUI extension protocol",
                "custom TUI protocol",
                "自定义 TUI extension protocol",
                "自定义 TUI 扩展协议",
            ],
            &[
                "supports",
                "implements",
                "provides",
                "ships",
                "adds",
                "支持",
                "实现",
                "提供",
                "发布",
                "新增",
            ],
        ),
        (
            &["shared opi-types crate", "共享 opi-types crate"],
            &[
                "introduced",
                "adds",
                "ships",
                "provides",
                "引入",
                "新增",
                "提供",
                "发布",
            ],
        ),
        (
            &["whole agent loop", "agent loop"],
            &[
                "rewrote",
                "replaces",
                "migrates",
                "routes entirely through",
                "重写",
                "替换",
                "迁移",
                "完全路由",
            ],
        ),
    ];

    let positive_claim_match = |line: &str| {
        let line_lower = line.to_lowercase();
        claims.iter().find_map(|(features, verbs)| {
            features.iter().find_map(|feature| {
                let feature_lower = feature.to_lowercase();
                if !line_lower.contains(&feature_lower) {
                    return None;
                }
                verbs.iter().find_map(|verb| {
                    let verb_lower = verb.to_lowercase();
                    if line_lower.contains(&verb_lower) && !no_positive_claim(line, feature) {
                        Some((*feature, *verb))
                    } else {
                        None
                    }
                })
            })
        })
    };

    for doc in docs {
        let content = read_repo_file(doc);
        for line in content.lines() {
            if let Some((feature, verb)) = positive_claim_match(line) {
                panic!("{doc} must not positively claim `{verb}` + `{feature}`: {line}");
            }
        }
    }

    // Non-vacuity: prove the helper catches each overclaim shape when it is
    // positively asserted, so the pass above is meaningful and not a silently
    // vacuous guard (no_positive_claim is per-line; these claim-verb phrases
    // carry no negation keyword and must be caught).
    for (line, needle) in [
        (
            "opi supports provider OAuth login today",
            "supports provider OAuth login",
        ),
        (
            "opi supports subscription auth for Copilot",
            "supports subscription auth",
        ),
        (
            "opi expands the broad provider catalog today",
            "broad provider catalog",
        ),
        (
            "opi supports custom TUI extension protocol today",
            "custom TUI extension protocol",
        ),
        (
            "opi provides a shared opi-types crate",
            "shared opi-types crate",
        ),
        (
            "opi routes entirely through the whole agent loop migration",
            "whole agent loop",
        ),
    ] {
        assert!(
            !no_positive_claim(line, needle),
            "non-vacuity: no_positive_claim must catch `{needle}` as a positive claim"
        );
    }
    for line in [
        "opi \u{652f}\u{6301} subscription auth today",
        "opi \u{6269}\u{5f20} provider catalog today",
        "opi \u{652f}\u{6301} \u{81ea}\u{5b9a}\u{4e49} TUI \u{6269}\u{5c55}\u{534f}\u{8bae}",
        "opi \u{5f15}\u{5165}\u{4e86}\u{5171}\u{4eab} opi-types crate",
    ] {
        assert!(
            positive_claim_match(line).is_some(),
            "non-vacuity: grouped Phase 10 guard must catch localized positive claim `{line}`"
        );
    }
}

#[test]
fn phase10_exit_trace_completeness() {
    // Phase 10 final gate (DoD): reconstruct SC1-SC8, the 4 workstream goals,
    // and the 11 non-goals from the current docs/opi-spec.md (+zh) so no
    // success criterion is silently absent from the normative docs. This is the
    // executable Phase F.1a phase-exit trace for Phase 10: each assertion maps
    // to a design-doc success criterion / workstream goal / non-goal, and
    // failure means a criterion has no doc-attested owner.
    let spec_en = ws_normalized(&read_repo_file("docs/opi-spec.md"));
    let spec_zh = ws_normalized(&read_repo_file("docs/opi-spec.zh.md"));

    // (a) Top-level SC1-SC8, each named in the EN spec.
    // SC1: opi-ai provider collection/auth seam exists.
    assert!(
        spec_en.contains("provider collection/auth seam"),
        "SC1: opi-spec must name the opi-ai provider collection/auth seam (EN)"
    );
    // SC2: opi-coding-agent routes model listing/registry construction through
    // the seam, while runtime provider dispatch remains on the existing path.
    assert!(
        spec_en.contains("routes model listing and model-registry construction through"),
        "SC2: opi-spec must state opi-coding-agent routes model listing/model-registry construction through the seam (EN)"
    );
    // SC3: generic AgentHarness with phase/snapshot/save-point semantics.
    assert!(
        spec_en.contains("AgentHarness") && spec_en.contains("save points"),
        "SC3: opi-spec must name the generic AgentHarness with save-point semantics (EN)"
    );
    // SC4: CodingHarness documented as a product wrapper.
    assert!(
        spec_en.contains("CodingHarness") && spec_en.contains("product wrapper"),
        "SC4: opi-spec must document CodingHarness as a product wrapper (EN)"
    );
    // SC5: session repo/facade boundaries (typed seam).
    assert!(
        spec_en.contains("SessionFacade") && spec_en.contains("SessionRepo"),
        "SC5: opi-spec must name the SessionFacade/SessionRepo session seam (EN)"
    );
    // SC6: runtime hook boundaries.
    assert!(
        spec_en.contains("Runtime hook boundaries"),
        "SC6: opi-spec must document runtime hook boundaries (EN)"
    );
    // SC7: existing behavior covered by focused regression tests.
    assert!(
        spec_en.contains("focused regression tests"),
        "SC7: opi-spec must state focused regression tests cover existing behavior (EN)"
    );
    // SC8: no ecosystem breadth (non-goals paragraph present).
    assert!(
        spec_en.contains("Non-goals do not claim"),
        "SC8: opi-spec must carry the Phase 10 non-goals enumeration (EN)"
    );

    // (b) ZH counterpart carries the typed session seam (SC5) + non-goals (SC8).
    assert!(
        spec_zh.contains("SessionFacade") && spec_zh.contains("SessionRepo"),
        "SC5: opi-spec.zh must name the SessionFacade/SessionRepo session seam (ZH)"
    );
    assert!(
        spec_zh.contains("非目标不声明"),
        "SC8: opi-spec.zh must carry the Phase 10 non-goals enumeration (ZH)"
    );
    for phrase in ["list/fork 仍由产品层拥有", "产品 turn loop 采用已推迟"] {
        assert!(
            spec_zh.contains(phrase),
            "Phase 10 exit trace must honestly state `{phrase}` (ZH)"
        );
    }

    // (c) The 4 workstream goals are all named in the Phase 10 workstream table.
    for ws in [
        "Models/Auth",
        "AgentHarness",
        "Session repo/facade",
        "Runtime hook boundaries",
    ] {
        assert!(
            spec_en.contains(ws),
            "Phase 10 workstream table must name the `{ws}` workstream (EN)"
        );
    }

    // (d) The 11 non-goals are enumerated in the Phase 10 non-goals paragraph
    // (whitespace-normalized so markdown wrapping does not hide them).
    for ng in [
        "OAuth login",
        "subscription auth",
        "broad provider catalog expansion",
        "image generation",
        "custom TUI extension protocol",
        "npm/package marketplace",
        "browser/web",
        "TypeScript API compatibility",
        "session file compatibility",
        "opi-types",
        "whole-loop rewrite",
    ] {
        assert!(
            spec_en.contains(ng),
            "Phase 10 non-goals paragraph must enumerate `{ng}` (EN)"
        );
    }

    // (e) Honest exit-trace phrases distinguish published seams from product
    // adoption that remains deferred.
    for phrase in [
        "published provider collection/auth seam",
        "runtime provider dispatch still uses",
        "published generic `AgentHarness`",
        "product turn loop adoption is deferred",
        "list/fork stay product-owned",
    ] {
        assert!(
            spec_en.contains(phrase),
            "Phase 10 exit trace must honestly state `{phrase}`"
        );
    }
}

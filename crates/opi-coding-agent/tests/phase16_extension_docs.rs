//! Phase 16 pluggable-extension documentation contract guard (task 16.1).
//!
//! This substrate guard pins the *designed* Phase 16 contract in the paired
//! EN/ZH product spec before any Phase 16 implementation lands. It freezes the
//! canonical design binding (and rejects the superseded architecture filename
//! as a second normative source), keeps Phase 17 a reserved benchmark
//! placeholder with no premature spec, binds the renamed Phase 18 source, and
//! pins Minimal Runtime, the five independent lifecycle gates, no local
//! fallback, the standalone CLI/SDK/protocol surface, and the Phase 19/20
//! deferrals exactly as authored. It asserts documentation invariants only; it
//! does not claim shipped runtime behavior, which later Phase 16 tasks own.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn heading_slice<'a>(
    path: &str,
    content: &'a str,
    start_heading: &str,
    next_heading: &str,
) -> &'a str {
    let start = content
        .find(start_heading)
        .unwrap_or_else(|| panic!("{path} is missing heading `{start_heading}`"));
    let after_start = start + start_heading.len();
    let end = content[after_start..]
        .find(next_heading)
        .map(|offset| after_start + offset)
        .unwrap_or_else(|| {
            panic!("{path} heading `{start_heading}` is missing boundary `{next_heading}`")
        });
    &content[start..end]
}

fn assert_claims(path: &str, content: &str, claims: &[&str]) {
    let normalized = normalize_whitespace(content);
    for claim in claims {
        assert!(
            normalized.contains(&normalize_whitespace(claim)),
            "{path} must contain the exact Phase 16 claim `{claim}`"
        );
    }
}

fn assert_absent(path: &str, content: &str, claims: &[&str]) {
    let normalized = normalize_whitespace(content);
    for claim in claims {
        assert!(
            !normalized.contains(&normalize_whitespace(claim)),
            "{path} must not retain the superseded Phase 16 claim `{claim}`"
        );
    }
}

const CANONICAL_PHASE16_DESIGN: &str =
    "docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md";
const CANONICAL_PHASE16_DESIGN_BASENAME: &str =
    "2026-07-28-phase16-pluggable-extension-command-execution-design.md";
const OLD_ARCHITECTURE_DESIGN: &str =
    "docs/superpowers/specs/2026-07-28-pluggable-extension-architecture-design.md";
const RENAMED_PHASE18_DESIGN: &str =
    "docs/superpowers/specs/2026-07-11-phase18-agent-intelligence-design.md";

#[test]
fn phase16_section_binds_canonical_contract_en_zh() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 16 - Pluggable Extensions and Command Execution",
        "### Phase 17 - Benchmark and Regression Evaluation",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十六阶段 - 可插拔扩展与命令执行",
        "### 第十七阶段 - Benchmark 与回归评估",
    );

    // The canonical Phase 16 design source is bound in both locales; the
    // superseded architecture filename is not presented as a canonical source.
    assert_claims("docs/opi-spec.md", spec, &[CANONICAL_PHASE16_DESIGN]);
    assert_claims("docs/opi-spec.zh.md", spec_zh, &[CANONICAL_PHASE16_DESIGN]);
    assert_absent("docs/opi-spec.md", spec, &[OLD_ARCHITECTURE_DESIGN]);
    assert_absent("docs/opi-spec.zh.md", spec_zh, &[OLD_ARCHITECTURE_DESIGN]);

    // Minimal Runtime, the five independent lifecycle gates, no local fallback,
    // and the standalone CLI/SDK/protocol surface are pinned exactly.
    assert_claims(
        "docs/opi-spec.md",
        spec,
        &[
            "Phase 16 keeps the default `opi` process in the Minimal Runtime on a direct local execution path",
            "The first adapters are built-in `local` and external `opi-sandbox`",
            "the latter remains independently usable through its SDK, human CLI, and `command-execution-jsonl-v1` protocol",
            "Package installation does not imply Package Trust or activation: Installed, Trusted, Enabled, Selected, and Permitted are separate gates.",
            "Routing supports `fixed`, deterministic `rules`, and model recommendation under user policy, with `deny`/`ask`/`allow` permission outcomes.",
            "The Opi binary does not link `opi-sandbox`; with no enabled extension, it runs locally without extension processes or package-store scans.",
            "Once an external adapter is selected, failure is fail-closed and never falls back to local execution.",
            "`opi-protocol` initially owns only the versioned execution protocol.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh,
        &[
            "默认 `opi` 进程保持最小运行时（Minimal Runtime）的直接本地执行路径",
            "首批 adapter 是内置 `local` 与外部 `opi-sandbox`",
            "后者还可通过 SDK、面向用户的 CLI 和 `command-execution-jsonl-v1` 协议独立使用",
            "Installed、Trusted、Enabled、Selected、Permitted 是五个独立门",
            "路由支持 `fixed`、确定性的 `rules` 与受用户策略约束的模型建议，权限结果为 `deny`/`ask`/`allow`",
            "Opi 二进制不链接 `opi-sandbox`；没有启用扩展时，本地运行且不启动扩展进程、不扫描 package store",
            "外部 adapter 一旦被选择，失败即 fail-closed，绝不回退到本地执行",
            "`opi-protocol` 初始只承载版本化的执行协议",
        ],
    );
}

#[test]
fn phase17_remains_reserved_with_no_premature_benchmark_spec() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 17 - Benchmark and Regression Evaluation",
        "### Phase 18 - Agent Intelligence",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十七阶段 - Benchmark 与回归评估",
        "### 第十八阶段 - Agent Intelligence",
    );

    assert_claims(
        "docs/opi-spec.md",
        spec,
        &[
            "Status: reserved.",
            "Its specification will be discussed and written only after Phase 16 satisfies its exit criteria.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh,
        &["状态：已预留。仅在第十六阶段达到退出标准后，才讨论并编写本阶段 spec。"],
    );

    // No premature Phase 17 benchmark design is bound as a normative source.
    assert_absent("docs/opi-spec.md", spec, &["docs/superpowers/specs/"]);
    assert_absent("docs/opi-spec.zh.md", spec_zh, &["docs/superpowers/specs/"]);
}

#[test]
fn phase18_binds_renamed_agent_intelligence_source_en_zh() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 18 - Agent Intelligence",
        "### Phase 19 - Extension Architecture Completion",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十八阶段 - Agent Intelligence",
        "### 第十九阶段 - 扩展架构完善",
    );

    assert_claims(
        "docs/opi-spec.md",
        spec,
        &[
            RENAMED_PHASE18_DESIGN,
            "Status: designed; implementation deferred until the Phase 17 baseline exists.",
        ],
    );
    assert_claims("docs/opi-spec.zh.md", spec_zh, &[RENAMED_PHASE18_DESIGN]);
}

#[test]
fn phase19_and_phase20_deferrals_are_pinned_en_zh() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    let spec_p19 = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 19 - Extension Architecture Completion",
        "### Phase 20 - UI Productization",
    );
    let spec_p20 = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 20 - UI Productization",
        "### Future Ecosystem Candidates",
    );
    let spec_zh_p19 = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十九阶段 - 扩展架构完善",
        "### 第二十阶段 - 界面产品化",
    );
    let spec_zh_p20 = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第二十阶段 - 界面产品化",
        "### 未来生态候选",
    );

    assert_claims(
        "docs/opi-spec.md",
        spec_p19,
        &[
            "Status: roadmap placeholder; design follows benchmark and Agent Intelligence evidence.",
            "Phase 19 broadens the Phase 16 capability/adapter model",
        ],
    );
    assert_claims(
        "docs/opi-spec.md",
        spec_p20,
        &[
            "Status: deferred until the core, benchmark, intelligence, and extension foundations are stable.",
            "Phase 20 lands the event-driven TUI engine (`OverlayStack`, streaming-redraw throttle)",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh_p19,
        &[
            "状态：roadmap 占位；在 benchmark 与 Agent Intelligence 证据形成后再设计。",
            "第十九阶段把第十六阶段的 capability/adapter 模型扩展到更多贡献类型与执行 adapter",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh_p20,
        &[
            "状态：推迟至核心、benchmark、智能能力与扩展基础稳定后。",
            "第二十阶段落地事件驱动 TUI 引擎（`OverlayStack`、streaming-redraw throttle）",
        ],
    );
}

#[test]
fn canonical_design_files_exist_and_architecture_doc_defers() {
    // Both bound design files are present on disk.
    let phase16_design = read_repo_file(CANONICAL_PHASE16_DESIGN);
    let phase18_design = read_repo_file(RENAMED_PHASE18_DESIGN);
    assert!(
        phase16_design.contains("# Phase 16: Pluggable Extensions and Command Execution"),
        "canonical Phase 16 design must retain its top-level title"
    );
    assert!(
        phase18_design.contains("# Phase 18") || phase18_design.contains("Agent Intelligence"),
        "renamed Phase 18 design must retain its identity"
    );

    // The superseded architecture document exists only as supporting rationale
    // and explicitly defers to the canonical Phase 16 specification by basename;
    // it is not a second normative ledger source. (The full-path binding of the
    // canonical source lives in opi-spec.md, pinned in the test above.)
    let architecture = read_repo_file(OLD_ARCHITECTURE_DESIGN);
    assert!(
        architecture.contains("The canonical Phase 16 specification is")
            && architecture.contains(CANONICAL_PHASE16_DESIGN_BASENAME),
        "the architecture design must subordinate itself to the canonical Phase 16 source"
    );
}

#[test]
fn heading_slices_reject_markers_moved_outside_the_target_section() {
    let en = "### Other\nforbidden marker\n### Phase 16 - Pluggable Extensions and Command Execution\nbody\n### Phase 17 - Benchmark and Regression Evaluation\nforbidden marker\n";
    let en_phase = heading_slice(
        "fixture-en",
        en,
        "### Phase 16 - Pluggable Extensions and Command Execution",
        "### Phase 17 - Benchmark and Regression Evaluation",
    );
    assert!(!en_phase.contains("forbidden marker"));

    let zh = "### 其他\nforbidden marker\n### 第十六阶段 - 可插拔扩展与命令执行\n正文\n### 第十七阶段 - Benchmark 与回归评估\nforbidden marker\n";
    let zh_phase = heading_slice(
        "fixture-zh",
        zh,
        "### 第十六阶段 - 可插拔扩展与命令执行",
        "### 第十七阶段 - Benchmark 与回归评估",
    );
    assert!(!zh_phase.contains("forbidden marker"));
}

/// Task 16.16.3 shipped-state lockstep: after 16.16.1/16.16.2 the Phase 16
/// spec describes the SHIPPED Minimal Runtime, native guarantees, Windows
/// posture, migration, and non-goals in EN and ZH. Flipping the status away
/// from `implementation pending` while the designed-contract claims pinned
/// above stay intact is the documentation side of closing Phase 16.
#[test]
fn shipped_phase16_state_pinned_en_zh() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 16 - Pluggable Extensions and Command Execution",
        "### Phase 17 - Benchmark and Regression Evaluation",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十六阶段 - 可插拔扩展与命令执行",
        "### 第十七阶段 - Benchmark 与回归评估",
    );

    // Shipped status replaces the stale designed/pending status.
    assert_claims("docs/opi-spec.md", spec, &["Status: implemented."]);
    assert_claims("docs/opi-spec.zh.md", spec_zh, &["状态：已实现。"]);
    assert!(
        !normalize_whitespace(spec).contains(&normalize_whitespace(
            "Status: approved; implementation pending."
        )),
        "docs/opi-spec.md Phase 16 must not retain the designed/pending status"
    );
    assert!(
        !normalize_whitespace(spec_zh).contains(&normalize_whitespace("状态：已批准；实现待定。")),
        "docs/opi-spec.zh.md Phase 16 must not retain the designed/pending status"
    );

    // Migration, native guarantees, Windows posture, and non-goals in lockstep.
    assert_claims(
        "docs/opi-spec.md",
        spec,
        &[
            "Native restriction and its helper/capability-selection code leave the Opi core",
            "L0 subprocess-tree supervision remains in core for both local and external adapter processes",
            "is rejected in core without compatibility aliases",
            "`opi-sandbox` is one Rust package with a library SDK",
            "depends only on `opi-protocol` plus standalone dependencies",
            "reports `restricted`, never `isolated`",
            "Windows Job Objects provide L0 supervision, not command restriction",
            "publishes no official Windows `opi-sandbox` artifact",
            "Docker/VM/SSH/Gondolin or remote adapters",
            "letting extensions replace a core tool by name",
            "Windows AppContainer or restricted-token restriction",
            "preserving unreleased Phase 15 sandbox configuration aliases",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh,
        &[
            "原生限制及其 helper/capability-selection 代码离开 Opi 核心",
            "L0 子进程树监督对 local 与外部 adapter 进程仍保留在核心",
            "在核心被拒绝，不提供兼容 alias",
            "`opi-sandbox` 是一个 Rust package",
            "只依赖 `opi-protocol` 加独立依赖",
            "报告 `restricted`，绝不报告 `isolated`",
            "Windows Job Object 只提供 L0 监督，而非命令限制",
            "不发布官方 Windows `opi-sandbox` artifact",
            "Docker/VM/SSH/Gondolin 或远程 adapter",
            "让扩展按名称替换核心工具",
            "Windows AppContainer 或 restricted-token 限制",
            "保留未发布的第十五阶段 sandbox 配置 alias",
        ],
    );
}

/// The shipped-state claims travel to the user-facing README (EN+ZH), the
/// AGENTS/CLAUDE guidance files, and the Unreleased changelog in lockstep with
/// the spec. A regression that drops `command.execute` / `opi-sandbox` /
/// fail-closed no-fallback from one surface while another keeps it fails here.
#[test]
fn shipped_state_readme_guides_and_changelog_in_lockstep() {
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let agents = read_repo_file("AGENTS.md");
    let claude = read_repo_file("CLAUDE.md");
    let changelog = read_repo_file("CHANGELOG.md");
    // `split_once` so a missing/renamed `## [0.7.2]` marker fails loudly instead
    // of silently widening `unreleased` to the whole changelog.
    let (unreleased, _) = changelog
        .split_once("## [0.7.2]")
        .expect("Unreleased section precedes 0.7.2");

    let surfaces = [
        ("README.md", readme.as_str()),
        ("README.zh.md", readme_zh.as_str()),
        ("AGENTS.md", agents.as_str()),
        ("CLAUDE.md", claude.as_str()),
        ("CHANGELOG.md [Unreleased]", unreleased),
    ];
    for (name, content) in surfaces {
        assert!(
            content.contains("command.execute") || content.contains("command-execution-jsonl-v1"),
            "{name} must describe the shipped command.execute capability"
        );
        assert!(
            content.contains("opi-sandbox"),
            "{name} must name the standalone opi-sandbox package"
        );
        // A discriminating no-fallback phrase per surface (the EN `fail-closed`
        // token alone is pre-existing in the Phase 15 text, so pin the phrase
        // that only the shipped command-execution section carries).
        let no_fallback = if name == "README.zh.md" {
            "绝不重试"
        } else if name == "README.md" {
            "never retries through"
        } else {
            "never falls back to local"
        };
        assert!(
            content.contains(no_fallback),
            "{name} must describe fail-closed no-fallback semantics (`{no_fallback}`)"
        );
        assert!(
            content.contains("Installed") && content.contains("Permitted"),
            "{name} must name the Installed..Permitted lifecycle gates"
        );
        // A Phase 16 non-goal marker travels to every surface, not just the spec.
        assert!(
            content.contains("Docker/VM/SSH"),
            "{name} must name a Phase 16 non-goal marker (Docker/VM/SSH)"
        );
    }

    // The Minimal Runtime default is named on every surface.
    for (name, content) in surfaces {
        assert!(
            content.contains("Minimal Runtime") || content.contains("minimal runtime"),
            "{name} must name the Minimal Runtime default"
        );
    }
}

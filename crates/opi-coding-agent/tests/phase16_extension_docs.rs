//! Phase 16 pluggable-extension documentation contract guard.
//!
//! These guards pin the implemented Phase 16 contract in the paired EN/ZH
//! product spec and current source/help documentation. They freeze the
//! canonical design binding (and reject the superseded architecture filename
//! as a second normative source), keep Phase 17 a reserved benchmark
//! placeholder with no premature spec, bind the renamed Phase 18 source, and
//! pin the scoped Minimal Runtime, lifecycle gates, no-local-fallback rule,
//! standalone CLI/SDK/protocol surface, and Phase 19/20 deferrals.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn module_docs(content: &str) -> String {
    content
        .lines()
        .take_while(|line| line.starts_with("//!") || line.trim().is_empty())
        .map(|line| {
            line.strip_prefix("//! ")
                .or_else(|| line.strip_prefix("//!"))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn item_docs(content: &str) -> String {
    content
        .lines()
        .take_while(|line| line.starts_with("///") || line.trim().is_empty())
        .map(|line| {
            line.strip_prefix("/// ")
                .or_else(|| line.strip_prefix("///"))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn marker_slice<'a>(path: &str, content: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = content
        .find(start_marker)
        .unwrap_or_else(|| panic!("{path} is missing marker `{start_marker}`"));
    let after_start = start + start_marker.len();
    let end = content[after_start..]
        .find(end_marker)
        .map(|offset| after_start + offset)
        .unwrap_or_else(|| panic!("{path} marker `{start_marker}` is missing `{end_marker}`"));
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

fn workspace_layout_block<'a>(path: &str, content: &'a str) -> &'a str {
    let section = heading_slice(path, content, "## Workspace layout", "## Architecture");
    let (_, after_fence) = section
        .split_once("```text\n")
        .unwrap_or_else(|| panic!("{path} workspace layout is missing its `text` graph fence"));
    let (graph, _) = after_fence
        .split_once("\n```")
        .unwrap_or_else(|| panic!("{path} workspace graph fence is not closed"));
    graph
}

fn parse_workspace_graph(
    path: &str,
    content: &str,
) -> BTreeMap<String, (BTreeSet<String>, String)> {
    let mut graph = BTreeMap::new();
    for line in workspace_layout_block(path, content)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let (edge, role) = line
            .split_once(" - ")
            .unwrap_or_else(|| panic!("{path} has malformed workspace graph line `{line}`"));
        assert!(
            !role.trim().is_empty(),
            "{path} has an empty role in `{line}`"
        );

        let edge = edge.trim();
        let (name, dependencies) = if let Some(name) = edge.strip_suffix("(no internal deps)") {
            (name.trim(), BTreeSet::new())
        } else {
            let (name, dependencies) = edge
                .split_once(" -> ")
                .unwrap_or_else(|| panic!("{path} has malformed workspace edge `{edge}`"));
            let dependencies = dependencies
                .split(',')
                .map(str::trim)
                .map(str::to_owned)
                .collect();
            (name.trim(), dependencies)
        };
        assert!(
            graph
                .insert(name.to_owned(), (dependencies, role.trim().to_owned()))
                .is_none(),
            "{path} repeats workspace crate `{name}`"
        );
    }
    graph
}

fn workspace_graph_from_metadata(
    metadata: &serde_json::Value,
) -> BTreeMap<String, BTreeSet<String>> {
    let workspace_members: BTreeSet<String> = metadata["workspace_members"]
        .as_array()
        .expect("cargo metadata has workspace_members")
        .iter()
        .map(|member| member.as_str().expect("workspace member id").to_owned())
        .collect();
    let packages = metadata["packages"]
        .as_array()
        .expect("cargo metadata has packages");
    let workspace_packages_by_path: BTreeMap<PathBuf, String> = packages
        .iter()
        .filter(|package| workspace_members.contains(package["id"].as_str().expect("package id")))
        .map(|package| {
            let manifest_path = Path::new(
                package["manifest_path"]
                    .as_str()
                    .expect("workspace package manifest_path"),
            );
            let package_path = manifest_path
                .parent()
                .expect("workspace package manifest has a parent")
                .to_owned();
            let package_name = package["name"].as_str().expect("package name").to_owned();
            (package_path, package_name)
        })
        .collect();

    packages
        .iter()
        .filter(|package| workspace_members.contains(package["id"].as_str().expect("package id")))
        .map(|package| {
            let name = package["name"].as_str().expect("package name").to_owned();
            let dependencies = package["dependencies"]
                .as_array()
                .expect("package dependencies")
                .iter()
                // Cargo metadata represents normal dependencies as `kind:
                // null`; dev/build edges are not links in the shipped crate
                // graph.
                .filter(|dependency| {
                    dependency
                        .get("kind")
                        .is_some_and(serde_json::Value::is_null)
                })
                // Bind the dependency's resolved local path to a workspace
                // member manifest. This excludes a registry/git package that
                // happens to share a workspace package name and naturally
                // handles manifest-local dependency renames.
                .filter_map(|dependency| dependency["path"].as_str())
                .filter_map(|path| workspace_packages_by_path.get(Path::new(path)))
                .cloned()
                .collect();
            (name, dependencies)
        })
        .collect()
}

fn cargo_metadata_workspace_graph() -> BTreeMap<String, BTreeSet<String>> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(repo_root())
        .output()
        .expect("run cargo metadata for the workspace documentation guard");
    assert!(
        output.status.success(),
        "cargo metadata failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits valid JSON");
    workspace_graph_from_metadata(&metadata)
}

fn normalize_guidance_identity(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let normalized = if let Some(rest) = normalized.strip_prefix("# AGENTS.md\n") {
        format!("# GUIDANCE.md\n{rest}")
    } else if let Some(rest) = normalized.strip_prefix("# CLAUDE.md\n") {
        format!("# GUIDANCE.md\n{rest}")
    } else {
        normalized
    };
    normalized
        .replace(
            "This file provides guidance to Codex (Codex.ai/code) when working with code in\nthis repository.",
            "This file provides guidance to the coding assistant when working with code in\nthis repository.",
        )
        .replace(
            "This file provides guidance to Claude Code (claude.ai/code) when working with\ncode in this repository.",
            "This file provides guidance to the coding assistant when working with code in\nthis repository.",
        )
        .replace(
            "`CLAUDE.md` is the Claude Code-flavored sibling of this file. When project\nrules change, update both in lockstep to avoid drift.",
            "The other guidance file is the product-flavored sibling of this file. When project\nrules change, update both in lockstep to avoid drift.",
        )
        .replace(
            "`AGENTS.md` is the Codex-flavored sibling of this file. When project rules\nchange, update both in lockstep to avoid drift.",
            "The other guidance file is the product-flavored sibling of this file. When project\nrules change, update both in lockstep to avoid drift.",
        )
        .replace(
            "`Co-Authored-By: Codex ...`",
            "`Co-Authored-By: ASSISTANT ...`",
        )
        .replace(
            "`Co-Authored-By: Claude ...`",
            "`Co-Authored-By: ASSISTANT ...`",
        )
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
            "Phase 16 keeps the `command.execute` path of the default `opi` process in the Minimal Runtime on a direct local execution path",
            "The first adapters are built-in `local` and external `opi-sandbox`",
            "the latter remains independently usable through its SDK, human CLI, and `command-execution-jsonl-v1` protocol",
            "Package installation does not imply Package Trust or activation: Installed, Trusted, Enabled, Selected, and Permitted are separate gates.",
            "Routing supports `fixed`, deterministic `rules`, and model recommendation under user policy, with `deny`/`ask`/`allow` permission outcomes.",
            "The Opi binary does not link `opi-sandbox`.",
            "The Minimal Runtime label describes only this command-execution path; it does not disable separately configured resource-package discovery or legacy `opi-extension-jsonl-v1` adapter startup.",
            "Once an external adapter is selected, failure is fail-closed and never falls back to local execution.",
            "`opi-protocol` initially owns only the versioned execution protocol.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh,
        &[
            "默认 `opi` 进程的 `command.execute` 路径保持最小运行时（Minimal Runtime）的直接本地执行路径",
            "首批 adapter 是内置 `local` 与外部 `opi-sandbox`",
            "后者还可通过 SDK、面向用户的 CLI 和 `command-execution-jsonl-v1` 协议独立使用",
            "Installed、Trusted、Enabled、Selected、Permitted 是五个独立门",
            "路由支持 `fixed`、确定性的 `rules` 与受用户策略约束的模型建议，权限结果为 `deny`/`ask`/`allow`",
            "Opi 二进制不链接 `opi-sandbox`",
            "Minimal Runtime 标签只描述这条命令执行路径；它不会停用另行配置的资源 package 发现或既有 `opi-extension-jsonl-v1` adapter 启动路径",
            "外部 adapter 一旦被选择，失败即 fail-closed，绝不回退到本地执行",
            "`opi-protocol` 初始只承载版本化的执行协议",
        ],
    );
}

#[test]
fn phase16_runtime_scope_and_source_mechanism_docs_are_exact() {
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

    assert_claims(
        "docs/opi-spec.md Phase 16",
        spec,
        &[
            "fixed-local `allow` directly constructs `LocalBashOperations` without opening the command-execution package activation store",
            "This narrow statement does not disable the separate resource-package discovery and legacy `opi-extension-jsonl-v1` process-adapter runtime.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md Phase 16",
        spec_zh,
        &[
            "fixed-local `allow` 会直接构造 `LocalBashOperations`，且不会打开 command-execution package activation store",
            "这一窄范围声明不会停用独立的资源 package 发现与既有 `opi-extension-jsonl-v1` process-adapter runtime",
        ],
    );
    assert_absent(
        "docs/opi-spec.md Phase 16",
        spec,
        &[
            "with no enabled extension, it runs locally without extension processes, package activation, or per-package scans",
            "starts no extension or package adapter process",
        ],
    );
    assert_absent(
        "docs/opi-spec.zh.md Phase 16",
        spec_zh,
        &[
            "没有启用扩展时，本地运行且不启动扩展进程、不执行 package activation 或逐 package 扫描",
            "不启动 extension 或 package adapter 进程",
        ],
    );

    let sandbox_lib = module_docs(&read_repo_file("crates/opi-sandbox/src/lib.rs"));
    assert_claims(
        "crates/opi-sandbox/src/lib.rs module docs",
        &sandbox_lib,
        &[
            "A supported Linux run reports [`Mechanism::Landlock`] as the lead mechanism in its per-run `Started` event, while `opi-sandbox doctor --json` reports the full observed Landlock-plus-seccomp posture.",
            "A supported macOS run reports [`Mechanism::Seatbelt`] in `Started`.",
        ],
    );
    assert_absent(
        "crates/opi-sandbox/src/lib.rs module docs",
        &sandbox_lib,
        &["reports [`Mechanism::Landlock`]/[`Mechanism::Seccomp`] (Linux)"],
    );

    let router_source = read_repo_file("crates/opi-coding-agent/src/execution/router.rs");
    let router = module_docs(&router_source);
    assert_claims(
        "crates/opi-coding-agent/src/execution/router.rs module docs",
        &router,
        &[
            "The production catalog contains construction-validated identities, but it is not an authoritative process-start availability claim: the selected external package is revalidated at invocation time immediately before spawn.",
        ],
    );
    assert_absent(
        "crates/opi-coding-agent/src/execution/router.rs module docs",
        &router,
        &["builds the `Eligibility` input from the activated package store"],
    );

    let eligible_adapter_docs = item_docs(marker_slice(
        "crates/opi-coding-agent/src/execution/router.rs",
        &router_source,
        "/// A router eligibility entry.",
        "pub struct EligibleAdapter",
    ));
    assert_claims(
        "crates/opi-coding-agent/src/execution/router.rs EligibleAdapter docs",
        &eligible_adapter_docs,
        &[
            "`local` is a synthesized built-in entry.",
            "External entries come from construction-validated package identities that are installed, trusted, enabled, and target-compatible.",
            "a router input, not an authoritative external process-start guarantee",
        ],
    );
    assert_absent(
        "crates/opi-coding-agent/src/execution/router.rs EligibleAdapter docs",
        &eligible_adapter_docs,
        &["activated package store"],
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

#[test]
fn guidance_workspace_graph_matches_cargo_metadata() {
    let actual = cargo_metadata_workspace_graph();
    let required_graph = BTreeMap::from([
        (
            "opi-agent".to_owned(),
            BTreeSet::from(["opi-ai".to_owned()]),
        ),
        ("opi-ai".to_owned(), BTreeSet::new()),
        (
            "opi-coding-agent".to_owned(),
            BTreeSet::from([
                "opi-agent".to_owned(),
                "opi-ai".to_owned(),
                "opi-protocol".to_owned(),
                "opi-tui".to_owned(),
            ]),
        ),
        ("opi-protocol".to_owned(), BTreeSet::new()),
        (
            "opi-sandbox".to_owned(),
            BTreeSet::from(["opi-protocol".to_owned()]),
        ),
        ("opi-tui".to_owned(), BTreeSet::new()),
    ]);
    assert_eq!(
        actual, required_graph,
        "cargo metadata must retain the intended six-crate dependency topology"
    );

    let expected_roles = BTreeMap::from([
        (
            "opi-agent",
            "agent runtime, tool calling, sessions, compaction",
        ),
        ("opi-ai", "multi-provider LLM API"),
        (
            "opi-coding-agent",
            "produces the `opi` binary; coding harness, execution routing, and package activation",
        ),
        (
            "opi-protocol",
            "versioned `command-execution-jsonl-v1` protocol types, codecs, schemas, and fixtures",
        ),
        (
            "opi-sandbox",
            "standalone native-restriction SDK/CLI/backend; not linked into the `opi` binary",
        ),
        (
            "opi-tui",
            "terminal UI widgets, pickers, diff and image rendering",
        ),
    ]);

    for path in ["AGENTS.md", "CLAUDE.md"] {
        let content = read_repo_file(path);
        let documented = parse_workspace_graph(path, &content);
        let documented_dependencies: BTreeMap<String, BTreeSet<String>> = documented
            .iter()
            .map(|(name, (dependencies, _))| (name.clone(), dependencies.clone()))
            .collect();
        assert_eq!(
            documented_dependencies, actual,
            "{path} workspace graph must match cargo metadata exactly"
        );
        let documented_roles: BTreeMap<&str, &str> = documented
            .iter()
            .map(|(name, (_, role))| (name.as_str(), role.as_str()))
            .collect();
        assert_eq!(
            documented_roles, expected_roles,
            "{path} must describe each crate's current responsibility"
        );
    }

    assert_eq!(
        actual["opi-sandbox"],
        BTreeSet::from(["opi-protocol".to_owned()]),
        "standalone opi-sandbox must depend on opi-protocol only"
    );
    assert!(
        !actual["opi-coding-agent"].contains("opi-sandbox"),
        "the opi binary must not link opi-sandbox"
    );
}

#[test]
fn metadata_graph_uses_only_normal_dependencies_and_package_names_for_renames() {
    let metadata = serde_json::json!({
        "workspace_members": ["opi-ai id", "opi-coding-agent id", "opi-protocol id", "opi-sandbox id"],
        "packages": [
            {
                "id": "opi-ai id",
                "name": "opi-ai",
                "manifest_path": "C:/workspace/crates/opi-ai/Cargo.toml",
                "dependencies": []
            },
            {
                "id": "opi-coding-agent id",
                "name": "opi-coding-agent",
                "manifest_path": "C:/workspace/crates/opi-coding-agent/Cargo.toml",
                "dependencies": [
                    {
                        "name": "opi-protocol",
                        "rename": "execution-wire",
                        "kind": null,
                        "path": "C:/workspace/crates/opi-protocol",
                        "source": null
                    },
                    {
                        "name": "opi-sandbox",
                        "rename": null,
                        "kind": "dev",
                        "path": "C:/workspace/crates/opi-sandbox",
                        "source": null
                    },
                    {
                        "name": "opi-ai",
                        "rename": null,
                        "kind": "build",
                        "path": "C:/workspace/crates/opi-ai",
                        "source": null
                    },
                    {
                        "name": "opi-ai",
                        "rename": "registry-opi-ai",
                        "kind": null,
                        "path": null,
                        "source": "registry+https://github.com/rust-lang/crates.io-index"
                    },
                    {
                        "name": "external-crate",
                        "rename": null,
                        "kind": null,
                        "path": null,
                        "source": "registry+https://github.com/rust-lang/crates.io-index"
                    }
                ]
            },
            {
                "id": "opi-protocol id",
                "name": "opi-protocol",
                "manifest_path": "C:/workspace/crates/opi-protocol/Cargo.toml",
                "dependencies": []
            },
            {
                "id": "opi-sandbox id",
                "name": "opi-sandbox",
                "manifest_path": "C:/workspace/crates/opi-sandbox/Cargo.toml",
                "dependencies": []
            }
        ]
    });

    assert_eq!(
        workspace_graph_from_metadata(&metadata),
        BTreeMap::from([
            ("opi-ai".to_owned(), BTreeSet::new()),
            (
                "opi-coding-agent".to_owned(),
                BTreeSet::from(["opi-protocol".to_owned()]),
            ),
            ("opi-protocol".to_owned(), BTreeSet::new()),
            ("opi-sandbox".to_owned(), BTreeSet::new()),
        ])
    );
}

#[test]
fn guidance_identity_normalization_preserves_markdown_whitespace() {
    let agents = "# AGENTS.md\n\n- item\n  continued\n\n```text\n  indented\n```\n";
    let claude = "# CLAUDE.md\n\n- item\n continued\n\n```text\n indented\n```\n";

    assert_ne!(
        normalize_guidance_identity(agents),
        normalize_guidance_identity(claude),
        "list and code-block indentation drift must remain visible"
    );
}

#[test]
fn guidance_files_differ_only_in_expected_assistant_identity() {
    let agents = normalize_guidance_identity(&read_repo_file("AGENTS.md"));
    let claude = normalize_guidance_identity(&read_repo_file("CLAUDE.md"));
    assert_eq!(
        agents, claude,
        "AGENTS.md and CLAUDE.md may differ only in their assistant identity wording"
    );
}

#[test]
fn current_docs_separate_phase16_from_historical_phase15_en_zh() {
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    let cli = heading_slice("README.md", &readme, "## Main CLI Surface", "## Providers");
    let cli_zh = heading_slice(
        "README.zh.md",
        &readme_zh,
        "## 主要 CLI 表面",
        "## Provider",
    );
    assert_absent(
        "README.md current CLI",
        cli,
        &["--sandbox", "--sandbox-require"],
    );
    assert_absent(
        "README.zh.md current CLI",
        cli_zh,
        &["--sandbox", "--sandbox-require"],
    );
    assert_claims(
        "README.md current CLI",
        cli,
        &["--execution-strategy", "--execution-backend"],
    );
    assert_claims(
        "README.zh.md current CLI",
        cli_zh,
        &["--execution-strategy", "--execution-backend"],
    );

    assert_claims(
        "README.md",
        &readme,
        &["### Historical Phase 15 sandbox and project trust"],
    );
    assert_claims(
        "README.zh.md",
        &readme_zh,
        &["### 历史记录：第十五阶段沙箱与项目信任"],
    );

    let control = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "## 0. Document Control",
        "## 2. Design Philosophy",
    );
    let control_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "## 0. 文档控制",
        "## 2. 设计理念",
    );
    assert_claims(
        "docs/opi-spec.md current status",
        control,
        &[
            "Phases 1-16 implemented",
            "Next milestone | Phase 17",
            "six Rust crates",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md current status",
        control_zh,
        &[
            "第 1-16 阶段已实现",
            "下一里程碑 | 第十七阶段",
            "六个 Rust crate",
        ],
    );
    assert_absent(
        "docs/opi-spec.md current status",
        control,
        &["Phases 1-15 implemented", "four Rust crates"],
    );
    assert_absent(
        "docs/opi-spec.zh.md current status",
        control_zh,
        &["第 1-15 阶段已实现", "四个 Rust crate"],
    );

    let phase16 = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 16 - Pluggable Extensions and Command Execution",
        "### Phase 17 - Benchmark and Regression Evaluation",
    );
    let phase16_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十六阶段 - 可插拔扩展与命令执行",
        "### 第十七阶段 - Benchmark 与回归评估",
    );
    assert_claims(
        "docs/opi-spec.md Phase 16",
        phase16,
        &["without opening the command-execution package activation store"],
    );
    assert_claims(
        "docs/opi-spec.zh.md Phase 16",
        phase16_zh,
        &["不会打开 command-execution package activation store"],
    );
    assert_absent(
        "docs/opi-spec.md Phase 16",
        phase16,
        &[
            "touches no package-store sentinel",
            "performs no package activation or per-package scan",
        ],
    );
    assert_absent(
        "docs/opi-spec.zh.md Phase 16",
        phase16_zh,
        &[
            "不触碰 package-store sentinel",
            "不执行 package activation 或逐 package 扫描",
        ],
    );
}

#[test]
fn readmes_pin_tool_policy_and_sandbox_trust_boundaries_en_zh() {
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");

    let tools = heading_slice(
        "README.md",
        &readme,
        "## Built-in Tools",
        "## Config and Sessions",
    );
    let tools_zh = heading_slice("README.zh.md", &readme_zh, "## 内置工具", "## 配置与会话");
    assert_claims(
        "README.md Built-in Tools",
        tools,
        &[
            "These are tool-policy and file-operation hardening measures, not an operating-system sandbox.",
        ],
    );
    assert_claims(
        "README.zh.md 内置工具",
        tools_zh,
        &["这些是工具策略与文件操作加固，不是操作系统级 sandbox。"],
    );

    let execution = heading_slice(
        "README.md",
        &readme,
        "## Command Execution and opi-sandbox",
        "## Permissions and Trust Boundaries",
    );
    let execution_zh = heading_slice(
        "README.zh.md",
        &readme_zh,
        "## 命令执行与 opi-sandbox",
        "## 权限与信任边界",
    );
    assert_claims(
        "README.md Command Execution",
        execution,
        &[
            "The Opi binary never links `opi-sandbox`.",
            "`opi-sandbox` is a standalone crate",
            "depends only on `opi-protocol`",
            "Windows Job Objects provide L0 supervision only, and no official Windows `opi-sandbox` artifact is published.",
        ],
    );
    assert_claims(
        "README.zh.md 命令执行",
        execution_zh,
        &[
            "Opi 二进制绝不链接 `opi-sandbox`",
            "`opi-sandbox` 是独立 crate",
            "只依赖 `opi-protocol`",
            "Windows Job Object 只提供 L0 监督，且不发布官方 Windows `opi-sandbox` artifact",
        ],
    );

    let trust = heading_slice(
        "README.md",
        &readme,
        "## Permissions and Trust Boundaries",
        "### Historical Phase 15 sandbox and project trust",
    );
    let trust_zh = heading_slice(
        "README.zh.md",
        &readme_zh,
        "## 权限与信任边界",
        "### 历史记录：第十五阶段沙箱与项目信任",
    );
    assert_claims(
        "README.md Permissions",
        trust,
        &["package permission declarations are metadata, not enforced sandbox policy"],
    );
    assert_claims(
        "README.zh.md 权限",
        trust_zh,
        &["package 权限声明是元数据，不是强制 sandbox 策略"],
    );
}

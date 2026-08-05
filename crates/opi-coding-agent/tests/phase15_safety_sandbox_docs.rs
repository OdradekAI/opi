//! Phase 15 safety/sandbox documentation and non-goal guards (task 15.9).
//!
//! These tests pin the *shipped* sandbox, Operations, and project-trust
//! behavior in the paired EN/ZH public docs and reject every Phase 15
//! non-goal. Source-level assertions enforce the structural invariants behind
//! the doc claims (no opi-side `unsafe` on the production sandbox path, no
//! built-in `/trust`, no CLI `-e`, an empty standard-CLI resolver registry,
//! and the narrowed Linux L2/L3 mechanism).

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
            "{path} must contain the exact Phase 15 claim `{claim}`"
        );
    }
}

fn assert_absent(path: &str, content: &str, claims: &[&str]) {
    let normalized = normalize_whitespace(content);
    for claim in claims {
        assert!(
            !normalized.contains(&normalize_whitespace(claim)),
            "{path} must not retain the superseded Phase 15 claim `{claim}`"
        );
    }
}

fn rust_sources_under(relative: &str) -> String {
    fn visit(path: &Path, output: &mut String) {
        for entry in std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        {
            let path = entry.expect("source directory entry").path();
            if path.is_dir() {
                visit(&path, output);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                output.push_str(
                    &std::fs::read_to_string(&path).unwrap_or_else(|error| {
                        panic!("failed to read {}: {error}", path.display())
                    }),
                );
            }
        }
    }

    let mut output = String::new();
    visit(&repo_root().join(relative), &mut output);
    output
}

fn rust_source_files_under(relative: &str) -> Vec<(String, String)> {
    fn visit(root: &Path, path: &Path, output: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        {
            let path = entry.expect("source directory entry").path();
            if path.is_dir() {
                visit(root, &path, output);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("source remains below crate root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let source = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
                output.push((relative, source));
            }
        }
    }

    let root = repo_root().join(relative);
    let mut output = Vec::new();
    visit(&root, &root, &mut output);
    output.sort_by(|left, right| left.0.cmp(&right.0));
    output
}

const PHASE15_OWNED_MODULE_PATHS: &[&str] = &[
    "sandbox.rs",
    "sandbox/",
    "project_trust.rs",
    "project_trust/",
    "trust_prompt.rs",
    "trust_prompt/",
    "tool/operations.rs",
];

const PHASE15_OWNED_SURFACE: &[&str] = &[
    "::sandbox::",
    "SandboxConfig",
    "SandboxMode",
    "PreparedSandbox",
    "StrictBackend",
    "SandboxLayer",
    "StrictOutcome",
    "ConfinementBuild",
    "LayerAvailability",
    "ProjectTrust",
    "TrustDecision",
    "ProjectStartupPlan",
    "TrustChoice",
    "PreTrustUi",
    "HeadlessPreTrustUi",
    "AwaitingTrust",
    "InteractiveTrustPrompt",
    "TrustPrompt",
    "TrustVote",
    "TrustContext",
    "TrustError",
    "TrustResource",
    "ProjectTrustResolverRegistry",
    "ProjectTrustStore",
    "prepare_project_startup",
    "apply_ui_choice",
    "resolve_interactive_trust_decision",
    "resolve_project_trust_decision",
    "cli_trust_override",
    "project_trust",
    "trust_prompt",
    "default_project_trust",
    "prepared_sandbox",
    "trust_decision",
    "FileOperations",
    "BashOperations",
    "LocalFileOperations",
    "LocalBashOperations",
];

fn phase15_construction_ownership_violations(path: &str, source: &str) -> Vec<String> {
    let normalized_path = path.replace('\\', "/");
    let mut violations = PHASE15_OWNED_MODULE_PATHS
        .iter()
        .filter(|segment| normalized_path.ends_with(*segment) || normalized_path.contains(*segment))
        .map(|segment| format!("module path `{segment}`"))
        .collect::<Vec<_>>();
    violations.extend(
        PHASE15_OWNED_SURFACE
            .iter()
            .filter(|marker| source.contains(*marker))
            .map(|marker| format!("surface marker `{marker}`")),
    );
    violations
}

#[test]
fn localized_docs_pin_exact_phase15_claims() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 15 - Safety & Sandbox",
        "### Phase 16 - Pluggable Extensions and Command Execution",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十五阶段 - Safety & Sandbox",
        "### 第十六阶段 - 可插拔扩展与命令执行",
    );
    let readme = heading_slice(
        "README.md",
        &readme,
        "### Historical Phase 15 sandbox and project trust",
        "## Development",
    );
    let readme_zh = heading_slice(
        "README.zh.md",
        &readme_zh,
        "### 历史记录：第十五阶段沙箱与项目信任",
        "## 开发",
    );

    assert_claims(
        "docs/opi-spec.md",
        spec,
        &[
            "Status: implemented; pi-0.80.6 posture parity complete.",
            "The sandbox confines only the `bash` subprocess tree.",
            "Linux L2 is a narrowed new-socket creation gate, not the six-syscall domain-filter the 2026-07-11 design described",
            "returns a stable `EPERM` errno for `socket(AF_INET, ...)`, `socket(AF_INET6, ...)`, and `socket(AF_NETLINK, ...)`",
            "while `socket(AF_UNIX, ...)` and the generic socket operations needed for Unix-domain IPC remain allowed",
            "On Landlock ABI 4 (Linux 6.7+; runtime probed via `landlock_create_ruleset`, never inferred from the kernel release)",
            "On Landlock ABI 1-3, the independent seccomp socket-creation gate remains engaged while the missing TCP bind/connect sub-capability is reported as a degraded gap.",
            "Fail-open continues at that partial baseline; `require = true` fails closed before spawning.",
            "L3 is a danger-blocklist, not a strict allowlist.",
            "It denies `open_by_handle_at`, `bpf`, `perf_event_open`, `ptrace`, `kexec_load`, `kexec_file_load`, `reboot`, `init_module`, `finit_module`, `delete_module`, `swapon`, `swapoff`, `acct`, and `settimeofday`; on x86_64 it additionally denies `iopl` and `ioperm`",
            "`clone` and `unshare` remain allowed",
            "Linux L2 does not claim complete network isolation.",
            "and `io_uring`-initiated socket/connect/accept operations bypass the audited `socket(2)` path and are an explicit uncovered residual",
            "`/usr/bin/sandbox-exec -p <templated profile>`",
            "The probed helper identity is retained and the same absolute path is launched",
            "The Job Object is implemented via direct `windows-sys` FFI (`CreateJobObjectW` / `SetInformationJobObject` / `AssignProcessToJobObject` / `TerminateJobObject`), not a wrapper crate",
            "Diagnostics are additive `&'static str` codes — `opi.sandbox.degraded` (`CODE_SANDBOX_DEGRADED`) and `opi.sandbox.unavailable` (`CODE_SANDBOX_UNAVAILABLE`) under source `sandbox`",
            "`reason` comes from the closed `SandboxReason` enum and serializes only curated static text",
            "The `Operations` seam is a pure FS/exec backend layered below `PathPolicy`.",
            "The shipped `LocalFileOperations` additionally resolves workspace paths relative to a held canonical workspace-root capability",
            "explicitly authorized external interactive reads retain ambient-path behavior.",
            "the production-path `sandbox.rs`, `tool/operations.rs`, and `sandbox/windows.rs` modules retain `#![forbid(unsafe_code)]`.",
            "The project-trust gate gates *loading* of project-local resources, not tool execution.",
            "stored at `{user_config_dir}/trust.json` — i.e. `%APPDATA%\\opi\\trust.json` on Windows and `~/.config/opi/trust.json` on Unix, alongside `config.toml`",
            "there is no live mid-session trust mutation, no built-in `/trust` command, and no project-resource reload.",
            "Trust resolvers are registered through an explicit embedder-only API, `ProjectTrustResolverRegistry::register`",
            "the standard CLI ships an empty registry (it registers no resolvers), there is no CLI `-e` extension flag and no native resolver auto-loading",
            "and no provider or session-schema changes.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        spec_zh,
        &[
            "状态：已实现；pi-0.80.6 posture 对齐完成。",
            "沙箱只 confine `bash` 子进程树。",
            "Linux L2 是收窄的新建 socket 创建门，而非 2026-07-11 设计描述的六 syscall domain-filter",
            "seccomp deny-overlay 对 `socket(AF_INET, ...)`、`socket(AF_INET6, ...)` 与 `socket(AF_NETLINK, ...)` 返回稳定的 `EPERM` errno",
            "而 `socket(AF_UNIX, ...)` 与 Unix-domain IPC 所需的通用 socket 操作保持允许",
            "运行时经 `landlock_create_ruleset` 探测，绝不从内核版本推断",
            "在 Landlock ABI 1-3 上，独立的 seccomp socket-creation 门保持生效",
            "fail-open 会按该部分基线继续；`require = true` 会在 spawn 前 fail-closed。",
            "L3 是危险 blocklist，而非严格 allowlist。",
            "它拒绝 `open_by_handle_at`、`bpf`、`perf_event_open`、`ptrace`、`kexec_load`、`kexec_file_load`、`reboot`、`init_module`、`finit_module`、`delete_module`、`swapon`、`swapoff`、`acct` 与 `settimeofday`",
            "`clone` 与 `unshare` 保持允许",
            "Linux L2 不声称完整的网络隔离。",
            "`io_uring` 发起的 socket/connect/accept 操作绕过已审计的 `socket(2)` 路径，是显式的未覆盖残留",
            "`/usr/bin/sandbox-exec -p <templated profile>`",
            "探测到的 helper identity 会被保留，并启动同一绝对路径",
            "Job Object 经直接的 `windows-sys` FFI 实现",
            "`reason` 来自封闭的 `SandboxReason` 枚举，只序列化经审查的静态文本",
            "`Operations` 缝合点是分层位于 `PathPolicy` 之下的纯 FS/exec 后端。",
            "已交付的 `LocalFileOperations` 还会相对已持有且 canonical 的 workspace-root capability 解析 workspace 路径",
            "显式授权的外部交互式读取仍保留 ambient-path 行为。",
            "生产路径 `sandbox.rs`、`tool/operations.rs` 与 `sandbox/windows.rs` 模块保持 `#![forbid(unsafe_code)]`。",
            "项目信任门门控的是项目本地资源的*加载*，而非工具执行。",
            "存储于 `{user_config_dir}/trust.json`——即 Windows 上 `%APPDATA%\\opi\\trust.json`、Unix 上 `~/.config/opi/trust.json`，与 `config.toml` 并列",
            "不存在 live mid-session trust mutation，不存在内置 `/trust` 命令，不存在 project-resource reload。",
            "信任 resolver 经显式的 embedder-only API `ProjectTrustResolverRegistry::register` 注册",
            "标准 CLI 交付空 registry（不注册任何 resolver），不存在 CLI `-e` 扩展标志，不存在原生 resolver 自动加载",
            "不修改 provider 或 session schema。",
        ],
    );
    assert_claims(
        "README.md",
        readme,
        &[
            "### Historical Phase 15 sandbox and project trust",
            "The sandbox confines only the `bash` subprocess tree, not `opi` itself.",
            "Linux `strict` L2 is a narrowed new-socket creation gate: seccomp denies `socket(AF_INET)`, `socket(AF_INET6)`, and `socket(AF_NETLINK)` while preserving `socket(AF_UNIX)`",
            "On Landlock ABI 1-3, fail-open retains the seccomp new-socket gate",
            "`require = true` fails closed before spawning.",
            "the shipped local file backend performs workspace operations relative to a held root capability",
            "There is no built-in `/trust` command, no live mid-session trust mutation, and no project-resource reload.",
            "the standard CLI ships an empty resolver registry, exposes no CLI `-e` flag, and performs no native resolver loading.",
        ],
    );
    assert_claims(
        "README.zh.md",
        readme_zh,
        &[
            "### 历史记录：第十五阶段沙箱与项目信任",
            "沙箱只 confine `bash` 子进程树，不 confine `opi` 自身。",
            "Linux `strict` L2 是收窄的新建 socket 创建门：seccomp 拒绝 `socket(AF_INET)`、 `socket(AF_INET6)` 与 `socket(AF_NETLINK)`，同时保留 `socket(AF_UNIX)`",
            "在 Landlock ABI 1-3 上， fail-open 会保留 seccomp 新建 socket 门",
            "`require = true` 会在 spawn 前 fail-closed。",
            "已交付的 local 文件后端相对已持有的 root capability 执行 workspace 操作",
            "不存在内置 `/trust` 命令，不存在 live mid-session trust mutation",
            "标准 CLI 交付空 resolver registry，不暴露 CLI `-e` 标志，也不进行原生 resolver 加载。",
        ],
    );

    // The Phase 15 section in particular must no longer open with the stale
    // designed/pending status (other designed phases legitimately retain it,
    // so this is a scoped heading+status check, not a whole-file absence).
    let spec_norm = normalize_whitespace(spec);
    let spec_zh_norm = normalize_whitespace(spec_zh);
    assert!(
        !spec_norm.contains("Phase 15 - Safety & Sandbox Status: designed; implementation pending"),
        "docs/opi-spec.md Phase 15 section must not retain the stale designed/pending status"
    );
    assert!(
        !spec_zh_norm.contains("第十五阶段 - Safety & Sandbox 状态：已设计；实现待定"),
        "docs/opi-spec.zh.md Phase 15 section must not retain the stale designed/pending status"
    );
}

#[test]
fn phase15_docs_reject_superseded_design_and_nongoal_claims() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");
    let spec = heading_slice(
        "docs/opi-spec.md",
        &spec,
        "### Phase 15 - Safety & Sandbox",
        "### Phase 16 - Pluggable Extensions and Command Execution",
    );
    let spec_zh = heading_slice(
        "docs/opi-spec.zh.md",
        &spec_zh,
        "### 第十五阶段 - Safety & Sandbox",
        "### 第十六阶段 - 可插拔扩展与命令执行",
    );
    let readme = heading_slice(
        "README.md",
        &readme,
        "### Historical Phase 15 sandbox and project trust",
        "## Development",
    );
    let readme_zh = heading_slice(
        "README.zh.md",
        &readme_zh,
        "### 历史记录：第十五阶段沙箱与项目信任",
        "## 开发",
    );

    for (path, content) in [
        ("docs/opi-spec.md", spec),
        ("docs/opi-spec.zh.md", spec_zh),
        ("README.md", readme),
        ("README.zh.md", readme_zh),
    ] {
        // The shipped Windows Job Object uses `windows-sys` FFI, not a wrapper
        // crate; the 2026-07-11 design named `win32job` and must not leak back.
        assert_absent(path, content, &["win32job"]);
    }

    // The old Phase 15 block's combined non-goals sentence is gone (the
    // shipped section rewords it and expands the list).
    assert_absent(
        "docs/opi-spec.md",
        spec,
        &[
            "Non-goals: opi-self confinement, adapter strict-confinement, remote backends, and nav-tool Operations.",
            "The network *layer* reports `TemporarilyUnavailable` on ABI < 4",
            "`FileOperations` is unsandboxed",
        ],
    );
    assert_absent(
        "docs/opi-spec.zh.md",
        spec_zh,
        &[
            "网络*层*在 ABI < 4 时仍报告 `TemporarilyUnavailable`",
            "`FileOperations` 不被沙箱",
        ],
    );
}

#[test]
fn heading_slices_reject_markers_moved_outside_the_target_section() {
    let en = "### Other\nrequired marker\n### Phase 15 - Safety & Sandbox\nbody\n### Phase 16 - Pluggable Extensions and Command Execution\nrequired marker\n";
    let en_phase = heading_slice(
        "fixture-en",
        en,
        "### Phase 15 - Safety & Sandbox",
        "### Phase 16 - Pluggable Extensions and Command Execution",
    );
    assert!(!en_phase.contains("required marker"));

    let zh = "### 其他\nrequired marker\n### 第十五阶段 - Safety & Sandbox\n正文\n### 第十六阶段 - 可插拔扩展与命令执行\nrequired marker\n";
    let zh_phase = heading_slice(
        "fixture-zh",
        zh,
        "### 第十五阶段 - Safety & Sandbox",
        "### 第十六阶段 - 可插拔扩展与命令执行",
    );
    assert!(!zh_phase.contains("required marker"));

    let readme =
        "### Sandbox and project trust\nrequired marker\n## Development\nrequired marker\n";
    assert_eq!(
        heading_slice(
            "fixture-readme",
            readme,
            "### Sandbox and project trust",
            "## Development",
        )
        .matches("required marker")
        .count(),
        1
    );
}

#[test]
fn phase15_nongoals_have_structural_evidence() {
    // No opi-side `unsafe` on the retained production tool path
    // (`operations.rs`). The Phase 15 native-sandbox modules (`sandbox.rs`,
    // `sandbox/windows.rs`, `sandbox/linux.rs`) were removed by 16.16.1 when
    // native restriction left core; their forbid(unsafe) guarantee is preserved
    // as historical evidence by the Phase 15 doc claims pinned above.
    let operations_forbid = read_repo_file("crates/opi-coding-agent/src/tool/operations.rs");
    assert!(
        operations_forbid.contains("#![forbid(unsafe_code)]"),
        "operations.rs must retain `#![forbid(unsafe_code)]`"
    );

    // No built-in `/trust` slash command, no CLI `-e`/`--extension` flag.
    let interactive = read_repo_file("crates/opi-coding-agent/src/interactive.rs");
    assert!(
        !interactive.contains("/trust"),
        "interactive.rs must not introduce a built-in `/trust` command"
    );
    let cli = read_repo_file("crates/opi-coding-agent/src/cli.rs");
    for forbidden in [
        "long = \"extension\"",
        "short = 'e'",
        "pub extension",
        "pub extensions",
    ] {
        assert!(
            !cli.contains(forbidden),
            "cli.rs must not introduce a CLI `-e`/`--extension` loader (`{forbidden}`)"
        );
    }

    // The standard CLI ships an EMPTY resolver registry: no production source
    // implements `ProjectTrustResolver`, and main constructs the registry with
    // `::new()` (no `.register(...)` call).
    let coding_sources = rust_sources_under("crates/opi-coding-agent/src");
    assert!(
        !coding_sources.contains("impl ProjectTrustResolver for"),
        "no production source may implement `ProjectTrustResolver` (embedder-only, empty in the standard CLI)"
    );
    let main_src = read_repo_file("crates/opi-coding-agent/src/main.rs");
    assert!(
        main_src.contains("ProjectTrustResolverRegistry::new()"),
        "main.rs must construct the empty standard-CLI resolver registry"
    );
    assert!(
        !main_src.contains(".register("),
        "main.rs must not register a production trust resolver"
    );

    // No live mid-session trust mutation path on the built harness/runtime.
    for forbidden in [
        "fn set_trust",
        "fn update_trust",
        "fn reload_trust",
        "fn mutate_trust",
        "fn re_resolve_trust",
        "fn reload_project_resources",
    ] {
        assert!(
            !coding_sources.contains(forbidden),
            "production sources must not introduce a live trust-mutation path `{forbidden}`"
        );
    }

    // trust.json has no schema version/metadata (flat canonical-path map).
    let trust = read_repo_file("crates/opi-coding-agent/src/project_trust.rs");
    for forbidden in ["schema_version", "\"version\"", "trust_schema"] {
        assert!(
            !trust.contains(forbidden),
            "ProjectTrustStore must not carry schema metadata `{forbidden}`"
        );
    }

    // File tools are capability-relative within the workspace, while process
    // sandboxing remains exclusive to BashOperations.
    let operations = read_repo_file("crates/opi-coding-agent/src/tool/operations.rs");
    let operations_doc = normalize_whitespace(&operations.replace("///", ""));
    assert!(
        operations_doc.contains(
            "Paths inside `workspace_root` are resolved beneath a held directory capability"
        ),
        "LocalFileOperations must document its held workspace capability"
    );
    assert!(
        operations_doc.contains("Explicitly allowed external reads retain ambient-path behavior"),
        "LocalFileOperations must document the deliberately ambient external-read path"
    );

    // Construction-ownership invariant: Phase 15 sandbox/trust/Operations code
    // lives only in opi-coding-agent. Check each lower-crate source path and
    // source body independently so a new module or one unchecked PascalCase API
    // cannot hide in an aggregate string.
    for crate_path in ["crates/opi-ai/src", "crates/opi-agent/src"] {
        for (path, source) in rust_source_files_under(crate_path) {
            let violations = phase15_construction_ownership_violations(&path, &source);
            assert!(
                violations.is_empty(),
                "{crate_path}/{path} must not gain Phase 15 construction-owned surface: {}",
                violations.join(", ")
            );
        }
    }
}

#[test]
fn construction_ownership_guard_rejects_mutated_module_and_api_fixtures() {
    for module_path in PHASE15_OWNED_MODULE_PATHS {
        let fixture_path = format!("src/{module_path}");
        assert!(
            !phase15_construction_ownership_violations(&fixture_path, "").is_empty(),
            "module-path mutation `{fixture_path}` must trip the ownership guard"
        );
    }

    for marker in PHASE15_OWNED_SURFACE {
        let fixture = format!("mod mutation {{ /* {marker} */ }}");
        assert!(
            !phase15_construction_ownership_violations("src/mutation.rs", &fixture).is_empty(),
            "surface mutation `{marker}` must trip the ownership guard"
        );
    }

    assert!(
        phase15_construction_ownership_violations(
            "src/ordinary.rs",
            "pub struct OrdinaryAgentSurface;"
        )
        .is_empty(),
        "unrelated lower-crate code must not be rejected"
    );
}

#[test]
fn workspace_test_ci_fetches_git_history_for_artifact_audit() {
    let workflow = read_repo_file(".github/workflows/ci.yml");
    let test_job = workflow
        .split_once("\n  test:\n")
        .expect("ci.yml retains the workspace test job")
        .1
        .split_once("\n  doctest:\n")
        .expect("workspace test job precedes doctest")
        .0;

    assert!(
        test_job.contains("- uses: actions/checkout@v4\n        with:\n          fetch-depth: 0"),
        "the workspace test job runs artifact_audit_script and must fetch full Git history"
    );
}

#[test]
fn changelog_unreleased_records_phase15_additions() {
    let changelog = read_repo_file("CHANGELOG.md");
    let unreleased = changelog
        .split("## [0.7.1]")
        .next()
        .expect("Unreleased section precedes 0.7.1");
    for marker in [
        "AwaitingTrust",
        "subprocess-tree sandbox",
        "Operations` seam",
        "ProjectTrustStore",
        "trust.json",
        "--sandbox",
        "--trust",
        "forbid(unsafe_code)",
        "`AppState` no longer implements `Copy`, `Clone`, `PartialEq`, or `Eq`",
    ] {
        assert!(
            unreleased.contains(marker),
            "Unreleased must record Phase 15 marker `{marker}`"
        );
    }
    // The user-facing changelog names the Job Object, not a wrapper crate; the
    // superseded `win32job` name must not leak in.
    assert_absent("CHANGELOG.md [Unreleased]", unreleased, &["win32job"]);
}

#[test]
fn paired_specs_cite_the_adapter_contract_from_its_actual_test_binary() {
    for (path, start, end) in [
        (
            "docs/opi-spec.md",
            "### Phase 15 - Safety & Sandbox",
            "### Phase 16 - Pluggable Extensions and Command Execution",
        ),
        (
            "docs/opi-spec.zh.md",
            "### 第十五阶段 - Safety & Sandbox",
            "### 第十六阶段 - 可插拔扩展与命令执行",
        ),
    ] {
        let content = read_repo_file(path);
        let content = heading_slice(path, &content, start, end);
        assert!(
            content.contains("sandbox_l0::adapter_process_group_contract"),
            "{path} must cite the adapter process-group contract from sandbox_l0"
        );
        assert!(
            !content.contains("adapter_host_mock::adapter_process_group_contract"),
            "{path} must not cite the adapter contract under the wrong test binary"
        );
    }
}

#[test]
fn phase15_test_fixtures_are_not_registered_as_installable_binaries() {
    let cargo_toml = read_repo_file("crates/opi-coding-agent/Cargo.toml");
    assert!(
        !cargo_toml.contains("name = \"phase15-adapter-host-mock\""),
        "Phase 15 adapter fixtures must remain test-only and must not be shipped by `cargo install`"
    );
}

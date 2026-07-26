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

#[test]
fn localized_docs_pin_exact_phase15_claims() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let readme = read_repo_file("README.md");
    let readme_zh = read_repo_file("README.zh.md");

    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "Status: implemented; pi-0.80.6 posture parity complete.",
            "The sandbox confines only the `bash` subprocess tree.",
            "Linux L2 is a narrowed new-socket creation gate, not the six-syscall domain-filter the 2026-07-11 design described",
            "returns a stable `EPERM` errno for `socket(AF_INET, ...)`, `socket(AF_INET6, ...)`, and `socket(AF_NETLINK, ...)`",
            "while `socket(AF_UNIX, ...)` and the generic socket operations needed for Unix-domain IPC remain allowed",
            "On Landlock ABI 4 (Linux 6.7+; runtime probed via `landlock_create_ruleset`, never inferred from the kernel release)",
            "The network *layer* reports `TemporarilyUnavailable` on ABI < 4 even though the seccomp socket-creation denial is always engaged",
            "L3 is a danger-blocklist, not a strict allowlist.",
            "It denies `open_by_handle_at`, `bpf`, `perf_event_open`, `ptrace`, `kexec_load`, `kexec_file_load`, `reboot`, `init_module`, `finit_module`, `delete_module`, `swapon`, `swapoff`, `acct`, and `settimeofday`; on x86_64 it additionally denies `iopl` and `ioperm`",
            "`clone` and `unshare` remain allowed",
            "Linux L2 does not claim complete network isolation.",
            "and `io_uring`-initiated socket/connect/accept operations bypass the audited `socket(2)` path and are an explicit uncovered residual",
            "The Job Object is implemented via direct `windows-sys` FFI (`CreateJobObjectW` / `SetInformationJobObject` / `AssignProcessToJobObject` / `TerminateJobObject`), not a wrapper crate",
            "Diagnostics are additive `&'static str` codes — `opi.sandbox.degraded` (`CODE_SANDBOX_DEGRADED`) and `opi.sandbox.unavailable` (`CODE_SANDBOX_UNAVAILABLE`) under source `sandbox`",
            "The `Operations` seam is a pure FS/exec backend layered below `PathPolicy`.",
            "`FileOperations` is unsandboxed — file tools stay `PathPolicy`-guarded, since Phase 15 confines only the bash subprocess tree.",
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
        &spec_zh,
        &[
            "状态：已实现；pi-0.80.6 posture 对齐完成。",
            "Linux L2 是收窄的新建 socket 创建门，而非 2026-07-11 设计描述的六 syscall domain-filter",
            "Job Object 经直接的 `windows-sys` FFI 实现",
            "不存在 live mid-session trust mutation，不存在内置 `/trust` 命令，不存在 project-resource reload。",
            "标准 CLI 交付空 registry（不注册任何 resolver），不存在 CLI `-e` 扩展标志，不存在原生 resolver 自动加载",
            "不修改 provider 或 session schema。",
        ],
    );
    assert_claims(
        "README.md",
        &readme,
        &[
            "### Sandbox and project trust",
            "The sandbox confines only the `bash` subprocess tree, not `opi` itself.",
            "Linux `strict` L2 is a narrowed new-socket creation gate: seccomp denies `socket(AF_INET)`, `socket(AF_INET6)`, and `socket(AF_NETLINK)` while preserving `socket(AF_UNIX)`",
            "There is no built-in `/trust` command, no live mid-session trust mutation, and no project-resource reload.",
            "the standard CLI ships an empty resolver registry, exposes no CLI `-e` flag, and performs no native resolver loading.",
        ],
    );
    assert_claims(
        "README.zh.md",
        &readme_zh,
        &[
            "### 沙箱与项目信任",
            "沙箱只 confine `bash` 子进程树，不 confine `opi` 自身。",
            "不存在内置 `/trust` 命令，不存在 live mid-session trust mutation",
            "标准 CLI 交付空 resolver registry，不暴露 CLI `-e` 标志，也不进行原生 resolver 加载。",
        ],
    );

    // The Phase 15 section in particular must no longer open with the stale
    // designed/pending status (other designed phases legitimately retain it,
    // so this is a scoped heading+status check, not a whole-file absence).
    let spec_norm = normalize_whitespace(&spec);
    let spec_zh_norm = normalize_whitespace(&spec_zh);
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

    for (path, content) in [
        ("docs/opi-spec.md", &spec),
        ("docs/opi-spec.zh.md", &spec_zh),
        ("README.md", &readme),
        ("README.zh.md", &readme_zh),
    ] {
        // The shipped Windows Job Object uses `windows-sys` FFI, not a wrapper
        // crate; the 2026-07-11 design named `win32job` and must not leak back.
        assert_absent(path, content, &["win32job"]);
    }

    // The old Phase 15 block's combined non-goals sentence is gone (the
    // shipped section rewords it and expands the list).
    assert_absent(
        "docs/opi-spec.md",
        &spec,
        &[
            "Non-goals: opi-self confinement, adapter strict-confinement, remote backends, and nav-tool Operations.",
        ],
    );
}

#[test]
fn phase15_nongoals_have_structural_evidence() {
    // No opi-side `unsafe` on the production sandbox path.
    for path in [
        "crates/opi-coding-agent/src/sandbox.rs",
        "crates/opi-coding-agent/src/tool/operations.rs",
        "crates/opi-coding-agent/src/sandbox/windows.rs",
    ] {
        let source = read_repo_file(path);
        assert!(
            source.contains("#![forbid(unsafe_code)]"),
            "{path} must retain `#![forbid(unsafe_code)]`"
        );
    }

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

    // File tools are unsandboxed (FileOperations has no PreparedSandbox).
    let operations = read_repo_file("crates/opi-coding-agent/src/tool/operations.rs");
    let operations_doc = normalize_whitespace(&operations.replace("///", ""));
    assert!(
        operations_doc.contains("Plain `tokio::fs::*` wrapper with NO sandbox"),
        "LocalFileOperations must document itself as an unsandboxed tokio::fs wrapper"
    );

    // The narrowed Linux L2/L3 mechanism is pinned in source.
    let linux = read_repo_file("crates/opi-coding-agent/src/sandbox/linux.rs");
    for exact in [
        "(\"AF_INET\", libc::AF_INET as i64)",
        "(\"AF_INET6\", libc::AF_INET6 as i64)",
        "(\"AF_NETLINK\", libc::AF_NETLINK as i64)",
        "(\"open_by_handle_at\", libc::SYS_open_by_handle_at)",
        "(\"bpf\", libc::SYS_bpf)",
        "(\"ptrace\", libc::SYS_ptrace)",
        "(\"kexec_load\", libc::SYS_kexec_load)",
        "(\"reboot\", libc::SYS_reboot)",
        "(\"init_module\", libc::SYS_init_module)",
        "(\"finit_module\", libc::SYS_finit_module)",
        "(\"delete_module\", libc::SYS_delete_module)",
        "(\"swapon\", libc::SYS_swapon)",
        "(\"swapoff\", libc::SYS_swapoff)",
        "(\"acct\", libc::SYS_acct)",
        "(\"settimeofday\", libc::SYS_settimeofday)",
    ] {
        assert!(
            linux.contains(exact),
            "linux.rs must pin the narrowed L2/L3 deny entry `{exact}`"
        );
    }
    // clone/unshare must NOT be in the danger blocklist.
    assert!(
        !linux.contains("(\"clone\",") && !linux.contains("(\"unshare\","),
        "linux.rs danger blocklist must not deny `clone`/`unshare`"
    );
    // The alternate-surface audit retains an uncovered residual (no completeness claim).
    assert!(
        linux.contains("uncovered-residual") && linux.contains("io_uring"),
        "linux.rs alternate-surface audit must retain the io_uring uncovered residual"
    );

    // Diagnostics codes are stable `&'static str` literals.
    let diagnostics = read_repo_file("crates/opi-coding-agent/src/diagnostics.rs");
    for exact in [
        "pub const CODE_SANDBOX_DEGRADED: &str = \"opi.sandbox.degraded\";",
        "pub const CODE_SANDBOX_UNAVAILABLE: &str = \"opi.sandbox.unavailable\";",
        "pub const SOURCE_SANDBOX: &str = \"sandbox\";",
    ] {
        assert!(
            diagnostics.contains(exact),
            "diagnostics.rs must pin the stable sandbox diagnostic `{exact}`"
        );
    }

    // Construction-ownership invariant: Phase 15 sandbox/trust code lives only
    // in opi-coding-agent; opi-ai and opi-agent gain no sandbox/trust surface.
    let ai_sources = rust_sources_under("crates/opi-ai/src");
    for forbidden in [
        "ProjectTrust",
        "SandboxConfig",
        "prepared_sandbox",
        "trust_decision",
    ] {
        assert!(
            !ai_sources.contains(forbidden),
            "opi-ai must not gain Phase 15 sandbox/trust surface `{forbidden}`"
        );
    }
    let agent_sources = rust_sources_under("crates/opi-agent/src");
    for forbidden in [
        "ProjectTrust",
        "SandboxConfig",
        "prepared_sandbox",
        "trust_decision",
    ] {
        assert!(
            !agent_sources.contains(forbidden),
            "opi-agent must not gain Phase 15 sandbox/trust surface `{forbidden}`"
        );
    }
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

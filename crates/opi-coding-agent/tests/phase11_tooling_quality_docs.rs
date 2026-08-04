//! Documentation, help, and non-goal guard tests for Phase 11 tooling quality
//! (task 11.11).
//!
//! These guards pin the Phase 11 design's "Documentation Updates" and "Success
//! Criteria 8":
//!
//! - **Docs/help sync** (`policy_docs_and_help_stay_in_sync`) — README.md,
//!   README.zh.md, and docs/opi-spec.md section 8.4 stay aligned with
//!   `policy.rs` on read-only/mutating classification, flag precedence, the
//!   `--allow-mutating` requirement, bash execution policy, truncation and
//!   full-output behavior, and the rationale for why permission prompts are
//!   not a core feature. The classification and precedence are additionally
//!   cross-checked against the `policy` API itself, not just the prose.
//! - **SC8 non-goals** (`sc8_non_goals_not_in_core`) — the nine Phase 11
//!   non-goals are documented, and SC8's "absent from core" subset is pinned
//!   through structural positives: only the eight built-in tool names are
//!   registered, mutating-tool gating is a policy-level `--allow-mutating`
//!   check (not an interactive permission popup), and `bash` awaits one
//!   foreground child per call.
//!
//! The interactive hook gate-free behavior that completes the SC8 subset is
//! pinned by `interactive_allows_mutating_tools` in `safety_hooks.rs`.

use std::path::Path;

use opi_coding_agent::policy::{
    self, RunMode, ToolFlags, ToolRuntimeConfig, ToolSelection, is_mutating_tool,
    resolve_tool_selection,
};

/// Helper: read a file relative to the repo root (matches the Phase 6/7/8
/// doc-guard convention used by `observability_docs.rs` et al.).
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

/// The mutating built-ins, per `policy::is_mutating_tool`.
const MUTATING_TOOLS: &[&str] = &["write", "edit", "bash"];
/// The read-only built-ins.
const READ_ONLY_TOOLS: &[&str] = &["read", "grep", "find", "ls", "glob"];

// ===========================================================================
// Docs / help / policy sync
// ===========================================================================

/// README.md, README.zh.md, docs/opi-spec.md section 8.4, CLI help, and
/// `policy.rs` agree on tool classification, flag precedence, allow_mutating,
/// bash policy, truncation/full-output, and the permission-prompt rationale.
#[test]
fn policy_docs_and_help_stay_in_sync() {
    let readme = read_repo_file("crates/opi-coding-agent/README.md");
    let readme_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let changelog = read_repo_file("CHANGELOG.md");

    // --- (1) Read-only / mutating classification matches policy.rs. -------
    for name in MUTATING_TOOLS {
        assert!(
            is_mutating_tool(name),
            "policy.rs must classify {name} as mutating"
        );
    }
    for name in READ_ONLY_TOOLS {
        assert!(
            !is_mutating_tool(name),
            "policy.rs must NOT classify {name} as mutating"
        );
    }
    // Every registered built-in is exactly one of the two sets.
    let registered: std::collections::HashSet<&str> =
        policy::BUILTIN_TOOL_NAMES.iter().copied().collect();
    let expected: std::collections::HashSet<&str> = MUTATING_TOOLS
        .iter()
        .chain(READ_ONLY_TOOLS.iter())
        .copied()
        .collect();
    assert_eq!(
        registered, expected,
        "built-in set must be exactly the 8 tools"
    );

    // Docs carry both classes in both languages (class words + tool names).
    assert!(
        contains_ci(&readme, "read-only") && contains_ci(&readme, "mutating"),
        "README must name both tool classes"
    );
    assert!(
        readme_zh.contains("只读") && readme_zh.contains("修改性"),
        "README.zh must name both tool classes (只读 / 修改性)"
    );

    // --- (2) Flag precedence: documented order matches resolve_tool_selection.
    // The programmatic contract is load-bearing; doc presence is secondary.
    assert_eq!(
        resolve_tool_selection(ToolFlags {
            tools: None,
            no_tools: true,
            no_builtin_tools: false,
        }),
        ToolSelection::Disabled,
        "--no-tools wins over default"
    );
    assert_eq!(
        resolve_tool_selection(ToolFlags {
            tools: Some(vec!["read".into()]),
            no_tools: true,
            no_builtin_tools: false,
        }),
        ToolSelection::Disabled,
        "--no-tools wins over --tools"
    );
    assert_eq!(
        resolve_tool_selection(ToolFlags {
            tools: Some(vec!["read".into()]),
            no_tools: false,
            no_builtin_tools: true,
        }),
        ToolSelection::Allowlist(vec!["read".into()]),
        "--tools wins over --no-builtin-tools"
    );
    assert_eq!(
        resolve_tool_selection(ToolFlags {
            tools: None,
            no_tools: false,
            no_builtin_tools: false,
        }),
        ToolSelection::Default,
        "no flags -> Default"
    );
    assert_eq!(
        resolve_tool_selection(ToolFlags {
            tools: None,
            no_tools: false,
            no_builtin_tools: true,
        }),
        ToolSelection::NoBuiltin,
        "--no-builtin-tools alone -> NoBuiltin"
    );
    for flag in [
        "--tools",
        "--no-tools",
        "--no-builtin-tools",
        "--allow-mutating",
    ] {
        assert!(
            readme.contains(flag) && readme_zh.contains(flag),
            "both READMEs must reference {flag}"
        );
    }

    // --- allow_mutating requirement (policy gate is real, not a popup). ---
    let denied = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["bash".into()]),
    );
    assert!(
        denied.is_err(),
        "non-interactive bash must require --allow-mutating"
    );
    let allowed = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        true,
        ToolSelection::Allowlist(vec!["bash".into()]),
    );
    assert!(
        allowed.is_ok(),
        "bash must be allowed with --allow-mutating"
    );
    assert!(
        contains_ci(&readme, "--allow-mutating") && contains_ci(&spec, "--allow-mutating"),
        "README and opi-spec must document --allow-mutating"
    );

    // --- (3) bash execution policy. -------------------------------------
    assert!(
        readme.contains("cmd /C") && readme.contains("sh -c"),
        "README must document the bash shell (cmd /C / sh -c)"
    );
    assert!(
        readme_zh.contains("cmd /C") && readme_zh.contains("sh -c"),
        "README.zh must document the bash shell (cmd /C / sh -c)"
    );
    assert!(
        contains_ci(&readme, "workspace root") && readme_zh.contains("工作区根目录"),
        "README/README.zh must document the bash working directory"
    );
    assert!(
        readme.contains("30 seconds") && readme_zh.contains("30 秒"),
        "README/README.zh must document the 30s default bash timeout"
    );
    assert!(
        readme.contains("timeout_secs") && readme_zh.contains("timeout_secs"),
        "README/README.zh must name the timeout_secs override"
    );
    // Cancellation wire fields (load-bearing exact pins; same field names in ZH).
    assert!(
        readme.contains("cancelled=true")
            && readme.contains("timed_out=true")
            && readme_zh.contains("cancelled=true")
            && readme_zh.contains("timed_out=true"),
        "README/README.zh must document cancelled/timed_out wire fields"
    );
    assert!(
        readme.contains("not restricted to the workspace"),
        "README must state bash is not path-confined"
    );
    assert!(
        readme_zh.contains("不限制在工作区内"),
        "README.zh must state bash is not path-confined"
    );
    assert!(
        readme.contains("details.env") && readme_zh.contains("details.env"),
        "README/README.zh must document the details.env policy"
    );

    // --- (4) Truncation / full-output behavior. --------------------------
    assert!(
        readme.contains("2000") && readme.contains("lines omitted"),
        "README must document the read 2000-line cap and omitted marker"
    );
    assert!(
        readme_zh.contains("2000") && readme_zh.contains("lines omitted"),
        "README.zh must document the read 2000-line cap and omitted marker"
    );
    assert!(
        readme.contains("64 KiB") && readme_zh.contains("64 KiB"),
        "README/README.zh must document the 64 KiB bash cap"
    );
    assert!(
        readme.contains("details.full_output") && readme_zh.contains("details.full_output"),
        "README/README.zh must document the details.full_output spill"
    );

    // --- (5) Permission-prompt rationale. --------------------------------
    assert!(
        readme.contains("tool-selection check, not a permission or sandbox subsystem"),
        "README must explain mutating-tool safety is tool-selection, not permission/sandbox"
    );
    assert!(
        readme_zh.contains("工具选择校验") && readme_zh.contains("不是权限或 sandbox"),
        "README.zh must carry the same permission/sandbox rationale"
    );

    // --- (6) docs/opi-spec.md section 8.4 stays aligned. -----------------
    assert!(
        spec.contains("### 8.4"),
        "opi-spec must carry the section 8.4 opi-coding-agent policy header"
    );
    assert!(
        spec.contains("permission popups are not core"),
        "opi-spec section 8.4 must state permission popups are not core behavior"
    );
    assert!(
        spec.contains("--tools")
            && spec.contains("--no-tools")
            && spec.contains("--no-builtin-tools"),
        "opi-spec section 8.4 must name the tool-selection flags"
    );
    assert!(
        spec.contains(
            "Non-interactive mode SHOULD default to a conservative read-only tool set: `read`, `grep`, `find`, `ls`, and `glob`."
        ),
        "docs/opi-spec.md must include glob in the non-interactive/RPC default set"
    );
    assert!(
        !spec.contains("`glob` MAY remain available"),
        "docs/opi-spec.md must not weaken the implemented glob default to MAY"
    );
    assert!(
        spec_zh.contains("--allow-mutating"),
        "docs/opi-spec.zh.md must document --allow-mutating"
    );
    assert!(
        spec_zh.contains("非交互/RPC 默认工具：`read`、`grep`、`find`、`ls` 和 `glob`。"),
        "docs/opi-spec.zh.md must include glob in the non-interactive/RPC default set"
    );
    assert!(
        !spec.contains("Built-in failure results SHOULD keep `details: None`"),
        "opi-spec must not claim every built-in failure result omits details"
    );
    assert!(
        spec.contains("Most built-in failure results SHOULD keep `details: None`")
            && spec.contains("bash operation failures"),
        "opi-spec must document the bash exception to error-result details"
    );
    assert!(
        !changelog.contains("command/exit_code"),
        "changelog must not claim public bash diagnostics carry the raw command"
    );
    assert!(
        changelog.contains("exit_code/cancelled/timed_out/truncated")
            && changelog.contains("raw command omitted"),
        "changelog must describe bash diagnostic metadata without the raw command"
    );
    assert!(
        spec_zh.contains(
            "\u{6743}\u{9650}\u{5f39}\u{7a97}\u{4e0d}\u{662f}\u{6838}\u{5fc3}\u{884c}\u{4e3a}"
        ),
        "docs/opi-spec.zh.md must state permission popups are not core behavior"
    );
    assert!(
        spec_zh.contains("\u{72b6}\u{6001}\u{ff1a}\u{5df2}\u{5b8c}\u{6210}"),
        "docs/opi-spec.zh.md must mark Phase 11 completed after docs update"
    );

    // --- CLI help carries the same tool-selection flags at the boundary. --
    let help = <opi_coding_agent::cli::Cli as clap::CommandFactory>::command()
        .render_long_help()
        .to_string();
    for flag in [
        "--tools",
        "--no-tools",
        "--no-builtin-tools",
        "--allow-mutating",
    ] {
        assert!(help.contains(flag), "opi --help must expose {flag}");
    }
    assert!(
        contains_ci(&help, "mutating"),
        "opi --help must document the mutating-tool opt-in"
    );
}

// ===========================================================================
// SC8: Phase 11 non-goals documented and absent from core
// ===========================================================================

/// The nine Phase 11 non-goals (design doc, "Non-Goals") with an English and a
/// Simplified-Chinese token each. Used to confirm the README non-goal list
/// carries all nine in both languages.
const NINE_NON_GOALS: &[(&str, &str)] = &[
    ("permission popup", "权限弹窗"),
    ("background bash", "后台 bash"),
    ("remote execution", "远程执行"),
    ("IDE project index", "IDE 项目索引"),
    ("language-server", "语言服务器"),
    ("automatic formatting", "自动格式化"),
    ("package ecosystem", "package 生态"),
    ("workflow tools", "工作流工具"),
    ("sandbox", "sandbox"),
];

/// SC8: all nine Phase 11 non-goals are documented, and the SC8 subset
/// (permission popup, background bash, remote execution, sandbox, workflow
/// tools) is pinned absent from core through structural positives rather than
/// brittle identifier greps.
#[test]
fn sc8_non_goals_not_in_core() {
    let readme = read_repo_file("crates/opi-coding-agent/README.md");
    let readme_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");

    // README carries all nine non-goals in both languages.
    for (en, zh) in NINE_NON_GOALS {
        assert!(
            contains_ci(&readme, en),
            "README must list the Phase 11 non-goal: {en}"
        );
        assert!(
            readme_zh.contains(zh),
            "README.zh must list the Phase 11 non-goal: {zh}"
        );
    }

    // --- SC8 structural positives: the non-goals are absent from CORE. ---

    // (a) Only the eight built-in tools are registered — no workflow tool
    //     (todo / plan-mode / sub-agent), no extra builtin, is in core.
    assert_eq!(
        policy::BUILTIN_TOOL_NAMES.len(),
        8,
        "exactly eight built-in tools are registered"
    );
    let registered: std::collections::HashSet<&str> =
        policy::BUILTIN_TOOL_NAMES.iter().copied().collect();
    for workflow in ["todo", "plan_mode", "sub_agent", "subagent"] {
        assert!(
            !registered.contains(workflow),
            "no workflow tool '{workflow}' may be a registered built-in"
        );
    }

    // (b) Mutating-tool gating is a policy-level --allow-mutating check, not
    //     an interactive permission popup. The actual enforcement path is
    //     `ToolRuntimeConfig::resolve`, which rejects mutating tools in
    //     non-interactive mode unless `allow_mutating` is set. Interactive
    //     `before_tool_call` inherits the default `Allow` (see
    //     `interactive_allows_mutating_tools` in safety_hooks.rs) — there is
    //     no in-core gate hook.
    let denied = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["write".into()]),
    )
    .expect_err("write must be gated without --allow-mutating");
    let err = denied.to_string();
    assert!(
        err.contains("mutating tool") && err.contains("--allow-mutating"),
        "the mutating gate must be the policy allow-mutating check, not a popup: {err}"
    );

    // (c) bash awaits one foreground child per call — no background/daemon
    //     shell. Behavioral coverage is owned by the bash tool tests (11.6);
    //     this guard pins the foreground-await call site in the source. The
    //     spawn+wait moved from bash.rs into LocalBashOperations::exec during
    //     Phase 15.2 (T5 Operations injection), and the foreground wait plus
    //     L0 tree race relocated into tool/supervision.rs when the native
    //     sandbox left core (16.16.1); the structural pin lives there now, and
    //     bash.rs delegates to the local backend through operations.rs.
    let supervision_src = read_repo_file("crates/opi-coding-agent/src/tool/supervision.rs");
    assert!(
        supervision_src.contains("status = child.wait()"),
        "bash must await the child (foreground) rather than spawn a background session"
    );
}

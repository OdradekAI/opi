//! Tool selection flag tests for task 3.8.
//!
//! Validates --tools allowlist, --no-tools, --no-builtin-tools flag parsing,
//! ToolSelection resolution, tool filtering through the harness, and
//! precedence/conflict behavior.

use std::fs;

use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::policy::{
    RunMode, ToolFlags, ToolRuntimeConfig, ToolSelection, filter_tool_names, resolve_tool_selection,
};

use clap::Parser;

// --- Helpers ---

fn create_temp_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    fs::create_dir_all(dir.path().join(".git")).expect("failed to create .git");
    dir
}

// --- ToolSelection resolution ---

#[test]
fn resolve_default_when_no_flags() {
    let flags = ToolFlags {
        tools: None,
        no_tools: false,
        no_builtin_tools: false,
    };
    assert_eq!(resolve_tool_selection(flags), ToolSelection::Default);
}

#[test]
fn resolve_disabled_when_no_tools() {
    let flags = ToolFlags {
        tools: None,
        no_tools: true,
        no_builtin_tools: false,
    };
    assert_eq!(resolve_tool_selection(flags), ToolSelection::Disabled);
}

#[test]
fn resolve_no_builtin_when_flag() {
    let flags = ToolFlags {
        tools: None,
        no_tools: false,
        no_builtin_tools: true,
    };
    assert_eq!(resolve_tool_selection(flags), ToolSelection::NoBuiltin);
}

#[test]
fn resolve_allowlist_when_tools_specified() {
    let flags = ToolFlags {
        tools: Some(vec!["read".into(), "glob".into()]),
        no_tools: false,
        no_builtin_tools: false,
    };
    assert_eq!(
        resolve_tool_selection(flags),
        ToolSelection::Allowlist(vec!["read".into(), "glob".into()])
    );
}

#[test]
fn no_tools_takes_precedence_over_tools() {
    let flags = ToolFlags {
        tools: Some(vec!["read".into()]),
        no_tools: true,
        no_builtin_tools: false,
    };
    assert_eq!(resolve_tool_selection(flags), ToolSelection::Disabled);
}

#[test]
fn no_tools_takes_precedence_over_no_builtin() {
    let flags = ToolFlags {
        tools: None,
        no_tools: true,
        no_builtin_tools: true,
    };
    assert_eq!(resolve_tool_selection(flags), ToolSelection::Disabled);
}

#[test]
fn tools_takes_precedence_over_no_builtin() {
    let flags = ToolFlags {
        tools: Some(vec!["read".into()]),
        no_tools: false,
        no_builtin_tools: true,
    };
    assert_eq!(
        resolve_tool_selection(flags),
        ToolSelection::Allowlist(vec!["read".into()])
    );
}

// --- Tool name filtering ---

#[test]
fn interactive_default_active_tools_match_pi() {
    let config = ToolRuntimeConfig::resolve(RunMode::Interactive, false, ToolSelection::Default)
        .expect("interactive default should be valid");
    assert_eq!(
        config.active_tool_names,
        vec!["read", "write", "edit", "bash"]
    );
}

#[test]
fn non_interactive_default_active_tools_are_read_only() {
    let config = ToolRuntimeConfig::resolve(RunMode::NonInteractive, false, ToolSelection::Default)
        .expect("non-interactive read-only default should be valid");
    assert_eq!(
        config.active_tool_names,
        vec!["read", "grep", "find", "ls", "glob"]
    );
}

#[test]
fn non_interactive_mutating_opt_in_uses_coding_tools() {
    let config = ToolRuntimeConfig::resolve(RunMode::NonInteractive, true, ToolSelection::Default)
        .expect("non-interactive mutating default should be valid");
    assert_eq!(
        config.active_tool_names,
        vec!["read", "write", "edit", "bash"]
    );
}

#[test]
fn non_interactive_allowlisted_mutating_tool_requires_opt_in() {
    let error = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["read".into(), "bash".into()]),
    )
    .expect_err("bash should require mutating opt-in");
    assert!(
        error
            .to_string()
            .contains("mutating tool 'bash' requires --allow-mutating")
    );
}

#[test]
fn non_interactive_allowlisted_mutating_tool_allowed_with_opt_in() {
    let config = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        true,
        ToolSelection::Allowlist(vec!["read".into(), "bash".into()]),
    )
    .expect("bash should be valid with mutating opt-in");
    assert_eq!(config.active_tool_names, vec!["read", "bash"]);
}

#[test]
fn filter_disabled_excludes_all() {
    let all = vec!["read", "write", "edit", "bash", "glob", "grep"];
    let result = filter_tool_names(&all, &ToolSelection::Disabled);
    assert!(result.is_empty(), "Disabled should filter out all tools");
}

#[test]
fn filter_no_builtin_excludes_all_phase3() {
    // Phase 3: NoBuiltin behaves like Disabled since no extension tools exist yet
    let all = vec!["read", "write", "edit", "bash", "glob", "grep"];
    let result = filter_tool_names(&all, &ToolSelection::NoBuiltin);
    assert!(result.is_empty(), "NoBuiltin should exclude built-in tools");
}

#[test]
fn filter_allowlist_keeps_only_named() {
    let all = vec!["read", "write", "edit", "bash", "glob", "grep"];
    let result = filter_tool_names(
        &all,
        &ToolSelection::Allowlist(vec!["read".into(), "glob".into()]),
    );
    assert_eq!(result, vec!["read", "glob"]);
}

#[test]
fn filter_allowlist_unknown_names_excluded() {
    let all = vec!["read", "write"];
    let result = filter_tool_names(
        &all,
        &ToolSelection::Allowlist(vec!["read".into(), "nonexistent".into()]),
    );
    assert_eq!(result, vec!["read"]);
}

#[test]
fn filter_allowlist_preserves_builtin_order() {
    let all = vec![
        "read", "write", "edit", "bash", "grep", "find", "ls", "glob",
    ];
    let result = filter_tool_names(
        &all,
        &ToolSelection::Allowlist(vec!["grep".into(), "read".into(), "bash".into()]),
    );
    // Result preserves the order from all_tools, not from the allowlist
    assert_eq!(result, vec!["read", "bash", "grep"]);
}

#[test]
fn filter_allowlist_empty_excludes_all() {
    let all = vec!["read", "write"];
    let result = filter_tool_names(&all, &ToolSelection::Allowlist(vec![]));
    assert!(result.is_empty());
}

// --- CLI flag parsing ---

#[test]
fn cli_parse_tools_flag() {
    let cli = Cli::try_parse_from(["opi", "--tools", "read,glob"]).unwrap();
    assert_eq!(cli.tools, Some(vec!["read".into(), "glob".into()]));
}

#[test]
fn cli_parse_no_tools_flag() {
    let cli = Cli::try_parse_from(["opi", "--no-tools"]).unwrap();
    assert!(cli.no_tools);
}

#[test]
fn cli_parse_no_builtin_tools_flag() {
    let cli = Cli::try_parse_from(["opi", "--no-builtin-tools"]).unwrap();
    assert!(cli.no_builtin_tools);
}

#[test]
fn cli_parse_tools_single_tool() {
    let cli = Cli::try_parse_from(["opi", "--tools", "read"]).unwrap();
    assert_eq!(cli.tools, Some(vec!["read".into()]));
}

#[test]
fn cli_parse_no_flags_defaults() {
    let cli = Cli::try_parse_from(["opi"]).unwrap();
    assert!(cli.tools.is_none());
    assert!(!cli.no_tools);
    assert!(!cli.no_builtin_tools);
}

// --- Harness tool filtering (integration) ---

#[tokio::test]
async fn harness_default_includes_interactive_coding_tools() {
    let workspace = create_temp_workspace();
    let mock = MockProvider::new("mock", vec![text_response("done")]);

    let harness = CodingHarness::new(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
    );

    let system = harness.system_prompt();
    for tool in &["read", "write", "edit", "bash"] {
        assert!(
            system.contains(&format!("- {tool}:")),
            "Default interactive selection should include tool '{tool}'"
        );
    }
    for tool in &["grep", "find", "ls", "glob"] {
        assert!(
            !system.contains(&format!("- {tool}:")),
            "Default interactive selection should not include tool '{tool}'"
        );
    }
}

#[tokio::test]
async fn harness_non_interactive_default_includes_read_only_tools() {
    let workspace = create_temp_workspace();
    let mock = MockProvider::new("mock", vec![text_response("done")]);
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::NonInteractive, false, ToolSelection::Default)
            .expect("tool config");

    let harness = CodingHarness::new_with_tool_config(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        tool_config,
    );

    let system = harness.system_prompt();
    for tool in &["read", "grep", "find", "ls", "glob"] {
        assert!(
            system.contains(&format!("- {tool}:")),
            "Non-interactive default should include tool '{tool}'"
        );
    }
    for tool in &["bash", "edit", "write"] {
        assert!(
            !system.contains(&format!("- {tool}:")),
            "Non-interactive default should not include tool '{tool}'"
        );
    }
}

#[tokio::test]
async fn harness_disabled_removes_all_tools() {
    let workspace = create_temp_workspace();
    let mock = MockProvider::new("mock", vec![text_response("done")]);

    let harness = CodingHarness::new_with_selection(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        ToolSelection::Disabled,
    );

    let system = harness.system_prompt();
    assert!(
        !system.contains("Available tools:"),
        "Disabled selection should not include any tools"
    );
}

#[tokio::test]
async fn harness_no_builtin_removes_all_tools_phase3() {
    let workspace = create_temp_workspace();
    let mock = MockProvider::new("mock", vec![text_response("done")]);

    let harness = CodingHarness::new_with_selection(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        ToolSelection::NoBuiltin,
    );

    let system = harness.system_prompt();
    assert!(
        !system.contains("Available tools:"),
        "NoBuiltin should not include built-in tools in system prompt"
    );
}

#[tokio::test]
async fn harness_allowlist_filters_tools() {
    let workspace = create_temp_workspace();
    let mock = MockProvider::new("mock", vec![text_response("done")]);

    let harness = CodingHarness::new_with_selection(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        ToolSelection::Allowlist(vec!["read".into(), "glob".into()]),
    );

    let system = harness.system_prompt();
    assert!(
        system.contains("- read:"),
        "Allowlist should include 'read'"
    );
    assert!(
        system.contains("- glob:"),
        "Allowlist should include 'glob'"
    );
    assert!(
        !system.contains("- write:"),
        "Allowlist should exclude 'write'"
    );
    assert!(
        !system.contains("- edit:"),
        "Allowlist should exclude 'edit'"
    );
    assert!(
        !system.contains("- bash:"),
        "Allowlist should exclude 'bash'"
    );
    assert!(
        !system.contains("- grep:"),
        "Allowlist should exclude 'grep'"
    );
}

// ---------------------------------------------------------------------------
// Phase 11.8 S5: non-interactive tool visibility contract
// ---------------------------------------------------------------------------

/// Non-interactive mode without `--allow-mutating` does not advertise the
/// mutating built-ins (write/edit/bash); the default active set is the five
/// read-only navigation tools. With `allow_mutating=true` the coding default
/// (read/write/edit/bash) is advertised. Pins `policy::resolve_active_tool_names`.
#[test]
fn non_interactive_mutating_tools_not_advertised_without_allow_mutating() {
    let denied = ToolRuntimeConfig::resolve(RunMode::NonInteractive, false, ToolSelection::Default)
        .expect("default NI policy resolves");
    assert_eq!(
        denied.active_tool_names,
        vec!["read", "grep", "find", "ls", "glob"],
        "NI without --allow-mutating advertises only read-only tools: {:?}",
        denied.active_tool_names,
    );
    assert!(
        !denied
            .active_tool_names
            .iter()
            .any(|n| matches!(n.as_str(), "write" | "edit" | "bash")),
        "mutating tools must NOT be advertised without --allow-mutating: {:?}",
        denied.active_tool_names,
    );

    let opted_in =
        ToolRuntimeConfig::resolve(RunMode::NonInteractive, true, ToolSelection::Default)
            .expect("opted-in NI policy resolves");
    for expected in ["read", "write", "edit", "bash"] {
        assert!(
            opted_in.active_tool_names.iter().any(|n| n == expected),
            "{expected} IS advertised with --allow-mutating: {:?}",
            opted_in.active_tool_names,
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 15.2: build_tools constructs all eight built-in tools (T5 injection)
// ---------------------------------------------------------------------------

/// `CodingHarness::build_tools` constructs the local Operations defaults and
/// injects them into read/write/edit/bash while leaving grep/find/ls/glob on
/// their local-walk constructors. An Allowlist of all eight in Interactive mode
/// activates every tool so the filtered output is the full canonical set, in
/// canonical order (the production path the harness wires through build_tools).
#[test]
fn build_tools_constructs_expected_default_set() {
    let workspace = create_temp_workspace();
    let all_eight = vec![
        "read".into(),
        "write".into(),
        "edit".into(),
        "bash".into(),
        "grep".into(),
        "find".into(),
        "ls".into(),
        "glob".into(),
    ];
    let config = ToolRuntimeConfig::resolve(
        RunMode::Interactive,
        false,
        ToolSelection::Allowlist(all_eight),
    )
    .expect("interactive allowlist of all eight resolves");

    let tools = CodingHarness::build_tools(workspace.path(), &config);
    let names: Vec<String> = tools
        .iter()
        .map(|t| t.definition().name.to_string())
        .collect();

    assert_eq!(
        names,
        vec![
            "read", "write", "edit", "bash", "grep", "find", "ls", "glob"
        ],
        "build_tools must construct all eight built-in tools in canonical order"
    );
}

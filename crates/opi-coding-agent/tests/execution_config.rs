//! Task 16.6: `[execution]` configuration — layered resolution, deterministic
//! rules validation, CLI overrides, and the project-permission rejection gate.
//!
//! These tests drive the REAL production loaders (`load_config_file`,
//! `resolve_config`, `stage_config`/`finalize_with_project`, `merge_project_config`,
//! `Cli::parse_from`) so the `[execution]` section is proven to ride the existing
//! config pipeline. The router/permission resolution seam itself is exercised in
//! `execution_routing.rs` / `execution_permission.rs`.

use std::path::PathBuf;

use clap::Parser;
use opi_agent::diagnostic::code::CODE_CONFIG_PARSE_FAILED;
use opi_agent::diagnostic::{SOURCE_CONFIG, Severity};
use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::{
    ConfigError, ConfigSource, ExecutionStrategy, OpiConfig, PermissionDecision, load_config_file,
    merge_project_config, resolve_config, stage_config,
};
use opi_coding_agent::diagnostic_bridge::diagnostic_from_config;

// --- helpers ---

fn write_config(dir: &std::path::Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn write_temp_config(dir: &std::path::Path, contents: &str) -> PathBuf {
    write_config(dir, "config.toml", contents)
}

// ---------------------------------------------------------------------------
// Defaults + parsing
// ---------------------------------------------------------------------------

#[test]
fn execution_default_is_fixed_local_empty_maps() {
    let c = OpiConfig::default();
    assert_eq!(c.execution.strategy, ExecutionStrategy::Fixed);
    assert_eq!(c.execution.backend, "local");
    assert!(c.execution.rules.is_empty());
    assert!(c.execution.permissions.is_empty());
}

#[test]
fn empty_config_leaves_execution_at_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let c = load_config_file(&path).unwrap();
    assert_eq!(c.execution.strategy, ExecutionStrategy::Fixed);
    assert_eq!(c.execution.backend, "local");
}

#[test]
fn parse_fixed_explicit_backend() {
    let toml = "[execution]\nstrategy = \"fixed\"\nbackend = \"opi-sandbox\"\n";
    let dir = tempfile::tempdir().unwrap();
    let c = load_config_file(&write_temp_config(dir.path(), toml)).unwrap();
    assert_eq!(c.execution.strategy, ExecutionStrategy::Fixed);
    assert_eq!(c.execution.backend, "opi-sandbox");
}

#[test]
fn parse_rules_with_modes_and_catch_all() {
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nmodes = [\"non-interactive\", \"rpc\"]\nbackend = \"opi-sandbox\"\n\
                [[execution.rules]]\nbackend = \"local\"\n";
    let dir = tempfile::tempdir().unwrap();
    let c = load_config_file(&write_temp_config(dir.path(), toml)).unwrap();
    assert_eq!(c.execution.strategy, ExecutionStrategy::Rules);
    assert_eq!(c.execution.rules.len(), 2);
    // First rule: two modes; second rule: catch-all (no modes).
    assert_eq!(c.execution.rules[0].modes.as_ref().unwrap().len(), 2);
    assert!(c.execution.rules[1].modes.is_none());
}

#[test]
fn parse_permissions_map_accepts_quoted_hyphen_keys() {
    let toml = "[execution.permissions]\nlocal = \"allow\"\n\"opi-sandbox\" = \"ask\"\n";
    let dir = tempfile::tempdir().unwrap();
    let c = load_config_file(&write_temp_config(dir.path(), toml)).unwrap();
    assert_eq!(
        c.execution.permissions.get("local"),
        Some(&PermissionDecision::Allow)
    );
    assert_eq!(
        c.execution.permissions.get("opi-sandbox"),
        Some(&PermissionDecision::Ask)
    );
}

// ---------------------------------------------------------------------------
// Rules structural validation (DoD: mode charset, exactly one final catch-all,
// declaration order; F5: empty modes array rejected)
// ---------------------------------------------------------------------------

fn expect_invalid_exec(result: Result<OpiConfig, ConfigError>, field_contains: &str) {
    let err = result.expect_err("expected InvalidExecutionConfig");
    match &err {
        ConfigError::InvalidExecutionConfig { field, .. } => {
            assert!(
                field.contains(field_contains),
                "field {field:?} should contain {field_contains:?}"
            );
        }
        other => panic!("expected InvalidExecutionConfig, got {other:?}"),
    }
}

#[test]
fn rules_rejects_unknown_mode_value() {
    // Mode charset is enforced by the serde enum -> ConfigError::Parse.
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nmodes = [\"interactive\", \"batch\"]\nbackend = \"local\"\n";
    let dir = tempfile::tempdir().unwrap();
    let err = load_config_file(&write_temp_config(dir.path(), toml))
        .expect_err("unknown mode must be rejected");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

#[test]
fn rules_rejects_empty_modes_array() {
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nmodes = []\nbackend = \"local\"\n";
    let dir = tempfile::tempdir().unwrap();
    expect_invalid_exec(
        load_config_file(&write_temp_config(dir.path(), toml)),
        "rules.modes",
    );
}

#[test]
fn rules_strategy_rejects_missing_catch_all() {
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nmodes = [\"interactive\"]\nbackend = \"local\"\n";
    let dir = tempfile::tempdir().unwrap();
    expect_invalid_config(load_config_file(&write_temp_config(dir.path(), toml)));
}

fn expect_invalid_config(result: Result<OpiConfig, ConfigError>) {
    result.expect_err("expected execution-config rejection");
}

#[test]
fn rules_strategy_rejects_empty_rules() {
    let toml = "[execution]\nstrategy = \"rules\"\n";
    let dir = tempfile::tempdir().unwrap();
    expect_invalid_config(load_config_file(&write_temp_config(dir.path(), toml)));
}

#[test]
fn rules_rejects_catch_all_not_last() {
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nbackend = \"local\"\n\
                [[execution.rules]]\nmodes = [\"interactive\"]\nbackend = \"opi-sandbox\"\n";
    let dir = tempfile::tempdir().unwrap();
    expect_invalid_exec(
        load_config_file(&write_temp_config(dir.path(), toml)),
        "rules",
    );
}

#[test]
fn rules_rejects_multiple_catch_all() {
    let toml = "[execution]\nstrategy = \"rules\"\n\
                [[execution.rules]]\nbackend = \"local\"\n\
                [[execution.rules]]\nbackend = \"local\"\n";
    let dir = tempfile::tempdir().unwrap();
    expect_invalid_exec(
        load_config_file(&write_temp_config(dir.path(), toml)),
        "rules",
    );
}

// ---------------------------------------------------------------------------
// Layered precedence + project-trust gate (DoD 9: project strategy/backend ride
// the existing include_project gate)
// ---------------------------------------------------------------------------

#[test]
fn project_execution_strategy_backend_merged_only_when_trusted() {
    let root = tempfile::tempdir().unwrap();
    let user_dir = root.path().join("user");
    let project_dir = root.path().join("project");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(project_dir.join(".opi")).unwrap();
    let user_cfg = write_config(
        &user_dir,
        "config.toml",
        "[execution]\nbackend = \"local\"\n",
    );
    write_config(
        &project_dir.join(".opi"),
        "config.toml",
        "[execution]\nbackend = \"opi-sandbox\"\nstrategy = \"fixed\"\n",
    );

    // Untrusted (include_project=false): project execution request is NOT applied.
    let untrusted = stage_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir.clone()),
        user_config_path: Some(user_cfg.clone()),
    })
    .unwrap()
    .finalize_with_project(false)
    .unwrap();
    assert_eq!(untrusted.execution.backend, "local");

    // Trusted (include_project=true): project execution request IS applied.
    let trusted = stage_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: Some(user_cfg),
    })
    .unwrap()
    .finalize_with_project(true)
    .unwrap();
    assert_eq!(trusted.execution.backend, "opi-sandbox");
}

// ---------------------------------------------------------------------------
// Project-permission rejection (DoD 8 + F1: BOTH project-merge APIs reject)
// ---------------------------------------------------------------------------

fn project_with_execution_permissions() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let project_dir = root.path().join("project");
    std::fs::create_dir_all(project_dir.join(".opi")).unwrap();
    write_config(
        &project_dir.join(".opi"),
        "config.toml",
        "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\n",
    );
    (root, project_dir)
}

#[test]
fn project_permissions_rejected_via_resolve_config() {
    let (_root, project_dir) = project_with_execution_permissions();
    // resolve_config always includes the project layer; a trusted-equivalent load
    // still rejects project permissions.
    let err = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: None,
    })
    .expect_err("project [execution.permissions] must be rejected");
    match err {
        ConfigError::InvalidExecutionConfig { field, .. } => {
            assert!(field.contains("permissions"), "field was {field:?}");
        }
        other => panic!("expected InvalidExecutionConfig, got {other:?}"),
    }
}

#[test]
fn project_permissions_rejected_via_merge_project_config() {
    // The second project-merge API (stage-2 trust flow) must also reject.
    let (_root, project_dir) = project_with_execution_permissions();
    let pretrust = stage_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: None,
    })
    .unwrap()
    .finalize_with_project(false)
    .unwrap();
    let err = merge_project_config(pretrust, &project_dir)
        .expect_err("merge_project_config must reject project [execution.permissions]");
    assert!(matches!(err, ConfigError::InvalidExecutionConfig { .. }));
}

#[test]
fn empty_project_permissions_table_is_rejected_via_resolve_config() {
    let root = tempfile::tempdir().unwrap();
    let user_dir = root.path().join("user");
    let project_dir = root.path().join("project");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(project_dir.join(".opi")).unwrap();
    let user_config = write_config(
        &user_dir,
        "config.toml",
        "[execution.permissions]\nlocal = \"deny\"\n",
    );
    write_config(
        &project_dir.join(".opi"),
        "config.toml",
        "[execution.permissions]\n",
    );
    let result = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: Some(user_config),
    });
    expect_invalid_exec(result, "permissions");
}

#[test]
fn empty_project_permissions_table_is_rejected_via_merge_project_config() {
    let root = tempfile::tempdir().unwrap();
    let user_config = write_temp_config(root.path(), "[execution.permissions]\nlocal = \"deny\"\n");
    let project_dir = root.path().join("project");
    std::fs::create_dir_all(project_dir.join(".opi")).unwrap();
    write_config(
        &project_dir.join(".opi"),
        "config.toml",
        "[execution.permissions]\n",
    );
    let pretrust = load_config_file(&user_config).unwrap();
    expect_invalid_exec(merge_project_config(pretrust, &project_dir), "permissions");
}

#[test]
fn empty_user_permissions_table_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_config_file(&write_temp_config(dir.path(), "[execution.permissions]\n"))
        .expect("user-owned empty permission table is valid");
    assert!(config.execution.permissions.is_empty());
}

#[test]
fn user_layer_permissions_are_accepted() {
    // Control: the user layer MAY set permissions (the rejection is project-only).
    let dir = tempfile::tempdir().unwrap();
    let c = load_config_file(&write_temp_config(
        dir.path(),
        "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\n",
    ))
    .unwrap();
    assert_eq!(
        c.execution.permissions.get("opi-sandbox"),
        Some(&PermissionDecision::Allow)
    );
}

// ---------------------------------------------------------------------------
// CLI overrides (DoD 6: --execution-backend/--execution-strategy cannot grant
// trust or permission; F14: defined here, wired in 16.7)
// ---------------------------------------------------------------------------

#[test]
fn apply_execution_overrides_backend_forces_fixed() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = load_config_file(&write_temp_config(
        dir.path(),
        "[execution]\nstrategy = \"rules\"\n[[execution.rules]]\nbackend = \"local\"\n",
    ))
    .unwrap();
    assert_eq!(c.execution.strategy, ExecutionStrategy::Rules);
    c.apply_execution_overrides(Some("opi-sandbox"), None);
    assert_eq!(c.execution.strategy, ExecutionStrategy::Fixed);
    assert_eq!(c.execution.backend, "opi-sandbox");
}

#[test]
fn apply_execution_overrides_strategy_only() {
    let mut c = OpiConfig::default();
    c.apply_execution_overrides(None, Some(ExecutionStrategy::Model));
    assert_eq!(c.execution.strategy, ExecutionStrategy::Model);
    // backend untouched.
    assert_eq!(c.execution.backend, "local");
}

#[test]
fn apply_execution_overrides_does_not_touch_permissions() {
    // DoD 6: overrides touch strategy/backend ONLY. Permissions byte-identical.
    let dir = tempfile::tempdir().unwrap();
    let mut c = load_config_file(&write_temp_config(
        dir.path(),
        "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\nlocal = \"deny\"\n",
    ))
    .unwrap();
    let perms_before = c.execution.permissions.clone();
    c.apply_execution_overrides(Some("opi-sandbox"), Some(ExecutionStrategy::Model));
    assert_eq!(c.execution.permissions, perms_before);
}

#[test]
fn apply_execution_overrides_none_is_noop() {
    let mut c = OpiConfig::default();
    let before = c.execution.clone();
    c.apply_execution_overrides(None, None);
    assert_eq!(c.execution, before);
}

// ---------------------------------------------------------------------------
// CLI parsing (DoD 6: both override flags exist + cannot grant permission)
// ---------------------------------------------------------------------------

#[test]
fn cli_execution_backend_parses() {
    let cli = Cli::try_parse_from(["opi", "--execution-backend", "opi-sandbox"]).unwrap();
    assert_eq!(cli.execution_backend.as_deref(), Some("opi-sandbox"));
}

#[test]
fn cli_execution_strategy_parses() {
    let cli = Cli::try_parse_from(["opi", "--execution-strategy", "rules"]).unwrap();
    assert_eq!(cli.execution_strategy, Some(ExecutionStrategy::Rules));
}

#[test]
fn cli_execution_flags_default_none() {
    let cli = Cli::try_parse_from(["opi"]).unwrap();
    assert!(cli.execution_backend.is_none());
    assert!(cli.execution_strategy.is_none());
}

#[test]
fn cli_execution_strategy_invalid_value_rejected_at_parse() {
    Cli::try_parse_from(["opi", "--execution-strategy", "bogus"])
        .expect_err("invalid --execution-strategy must be rejected at parse");
}

// ---------------------------------------------------------------------------
// diagnostic_from_config mapping for the new variant (D.2 flag: the C.1a arm
// added to the production diagnostic_from_config fn must be exercised through
// that fn, not only via the loader).
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_from_config_maps_invalid_execution_config() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_config_file(&write_temp_config(
        dir.path(),
        "[execution]\nstrategy = \"rules\"\n[[execution.rules]]\nmodes = []\nbackend = \"local\"\n",
    ))
    .expect_err("invalid execution rules must error");
    let diag = diagnostic_from_config(&err);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_CONFIG_PARSE_FAILED);
    assert_eq!(diag.source, SOURCE_CONFIG);
    assert!(diag.action.is_some(), "must carry a remediation action");
    let details = diag
        .details
        .as_ref()
        .expect("carries field/message details");
    assert!(details.get("field").is_some());
    assert!(details.get("message").is_some());
}

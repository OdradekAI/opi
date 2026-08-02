//! Task 16.6: capability-permission policy resolution and its independence from
//! the bash tool-availability gate.
//!
//! DoD 7 — persistent permission resolves from user/explicit (user-authorized)
//! layers. DoD 11 — `--allow-mutating` controls `bash` tool availability
//! *independently* from adapter permission: the two are resolved by separate
//! pipelines (`ToolRuntimeConfig::resolve` and `resolve_config`) with disjoint
//! inputs, so neither can influence the other's output. (Project-permission
//! rejection, DoD 8, is covered in `execution_config.rs`.)

use std::collections::BTreeMap;

use opi_coding_agent::config::{ConfigSource, PermissionDecision, resolve_config};
use opi_coding_agent::execution::PermissionPolicy;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};

fn map(pairs: &[(&str, PermissionDecision)]) -> BTreeMap<String, PermissionDecision> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

fn src(config_path: std::path::PathBuf) -> ConfigSource {
    ConfigSource {
        cli_model: None,
        config_path: Some(config_path),
        env_model: None,
        project_dir: None,
        user_config_path: None,
    }
}

// ---------------------------------------------------------------------------
// Defaulting (D2/F10: local -> Allow when absent; external -> Ask when absent;
// explicit entries always honored)
// ---------------------------------------------------------------------------

#[test]
fn local_defaults_to_allow_external_defaults_to_ask() {
    let p = PermissionPolicy::empty();
    assert_eq!(p.decision_for("local"), PermissionDecision::Allow);
    assert_eq!(p.decision_for("opi-sandbox"), PermissionDecision::Ask);
}

#[test]
fn explicit_local_deny_is_honored_over_default() {
    let p = PermissionPolicy::from_map(map(&[("local", PermissionDecision::Deny)]));
    assert_eq!(p.decision_for("local"), PermissionDecision::Deny);
    assert!(p.is_denied("local"));
}

#[test]
fn explicit_external_allow_is_honored() {
    let p = PermissionPolicy::from_map(map(&[("opi-sandbox", PermissionDecision::Allow)]));
    assert_eq!(p.decision_for("opi-sandbox"), PermissionDecision::Allow);
}

// ---------------------------------------------------------------------------
// DoD 11: independence of `--allow-mutating` (bash availability) and adapter
// permission, driven through BOTH real resolvers. The two pipelines have
// disjoint inputs: ToolRuntimeConfig::resolve takes (run_mode, allow_mutating,
// selection); resolve_config takes the config layers. Each output is shown to
// depend only on its own inputs.
// ---------------------------------------------------------------------------

fn bash_available(allow_mutating: bool) -> bool {
    ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        allow_mutating,
        ToolSelection::Default,
    )
    .unwrap()
    .active_tool_names
    .contains(&"bash".to_string())
}

#[test]
fn bash_availability_flips_with_allow_mutating_only() {
    // In non-interactive mode, bash is present only with allow_mutating.
    assert!(!bash_available(false));
    assert!(bash_available(true));
}

#[test]
fn bash_availability_and_permission_resolve_through_independent_pipelines() {
    // Permission pipeline: resolve_config over a permissions layer.
    let dir = tempfile::tempdir().unwrap();
    let perm_path = dir.path().join("p.toml");
    std::fs::write(
        &perm_path,
        "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\nlocal = \"ask\"\n",
    )
    .unwrap();
    let config = resolve_config(src(perm_path.clone())).unwrap();
    let perms = config.execution.permissions.clone();
    assert_eq!(perms.get("opi-sandbox"), Some(&PermissionDecision::Allow));
    assert_eq!(perms.get("local"), Some(&PermissionDecision::Ask));

    // Tool pipeline: ToolRuntimeConfig::resolve over (run_mode, allow_mutating).
    let bash_off = bash_available(false);
    let bash_on = bash_available(true);
    assert!(!bash_off);
    assert!(bash_on);

    // (a) The resolved permission map is a function of the config layers only:
    // re-resolving the SAME layers yields the identical map; the allow_mutating
    // value the tool pipeline used never reaches resolve_config.
    let config_again = resolve_config(src(perm_path)).unwrap();
    assert_eq!(config_again.execution.permissions, perms);

    // (b) resolve_config is sensitive to its REAL input (the config layers): a
    // different permission layer resolves to a different map. This confirms the
    // map is driven by config layers, while bash availability above is driven by
    // allow_mutating through a separate resolver — the two knobs are independent.
    let dir2 = tempfile::tempdir().unwrap();
    let perm_path2 = dir2.path().join("p2.toml");
    std::fs::write(
        &perm_path2,
        "[execution.permissions]\n\"opi-sandbox\" = \"deny\"\nlocal = \"deny\"\n",
    )
    .unwrap();
    let config_deny = resolve_config(src(perm_path2)).unwrap();
    assert_ne!(
        config.execution.permissions, config_deny.execution.permissions,
        "permission maps must actually differ across config layers"
    );
}

// ---------------------------------------------------------------------------
// DoD 7: persistent permission resolves from user/explicit (user-authorized)
// layers
// ---------------------------------------------------------------------------

#[test]
fn explicit_config_layer_permission_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perm.toml");
    std::fs::write(
        &path,
        "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\nlocal = \"ask\"\n",
    )
    .unwrap();
    let config = resolve_config(src(path)).unwrap();
    assert_eq!(
        config.execution.permissions.get("opi-sandbox"),
        Some(&PermissionDecision::Allow)
    );
    assert_eq!(
        config.execution.permissions.get("local"),
        Some(&PermissionDecision::Ask)
    );
}

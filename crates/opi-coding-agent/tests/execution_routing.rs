//! Task 16.6: deterministic `command.execute` routing via the public
//! `resolve_selection` seam.
//!
//! DoD 4 — rules preserve declaration order (first match wins). DoD 5 — a
//! selected rule that fails does NOT fall through to a later rule (pinned to the
//! exact stable code). DoD 10 — the model cannot mutate install/trust/enablement/
//! policy/grants: under `fixed`/`rules` a model `backend` is ignored, and under
//! `model` it can only select from the eligible (non-denied) set. Plus the design
//! guarantee that no selected-adapter failure retries through `local`, and a
//! purity check that the inputs are never mutated.

use opi_coding_agent::config::{
    ExecutionConfig, ExecutionRule, ExecutionRunMode, ExecutionStrategy, PermissionDecision,
};
use opi_coding_agent::execution::{Eligibility, EligibleAdapter, Selection, resolve_selection};

fn adapter(id: &str, available: bool, permission: PermissionDecision) -> EligibleAdapter {
    EligibleAdapter {
        id: id.to_string(),
        available,
        permission,
    }
}

fn eligibility(adapters: &[EligibleAdapter]) -> Eligibility {
    Eligibility(adapters.to_vec())
}

fn fixed(backend: &str) -> ExecutionConfig {
    ExecutionConfig {
        strategy: ExecutionStrategy::Fixed,
        backend: backend.to_string(),
        ..ExecutionConfig::default()
    }
}

fn rules(r: Vec<ExecutionRule>) -> ExecutionConfig {
    ExecutionConfig {
        strategy: ExecutionStrategy::Rules,
        rules: r,
        ..ExecutionConfig::default()
    }
}

fn rule(modes: Option<Vec<ExecutionRunMode>>, backend: &str) -> ExecutionRule {
    ExecutionRule {
        modes,
        backend: backend.to_string(),
    }
}

const INTERACTIVE: ExecutionRunMode = ExecutionRunMode::Interactive;
const NONINTERACTIVE: ExecutionRunMode = ExecutionRunMode::NonInteractive;

// ---------------------------------------------------------------------------
// DoD 10: model non-authority (fixed/rules ignore the model backend)
// ---------------------------------------------------------------------------

#[test]
fn fixed_ignores_model_backend() {
    let cfg = fixed("local");
    let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
    let without = resolve_selection(&cfg, INTERACTIVE, &elig, None).unwrap();
    let with = resolve_selection(&cfg, INTERACTIVE, &elig, Some("opi-sandbox")).unwrap();
    assert_eq!(without, with);
    assert_eq!(with.backend, "local");
}

#[test]
fn rules_ignore_model_backend() {
    let cfg = rules(vec![
        rule(Some(vec![NONINTERACTIVE]), "opi-sandbox"),
        rule(None, "local"),
    ]);
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Allow),
    ]);
    // NonInteractive -> rule[0] -> opi-sandbox, regardless of model backend.
    let sel = resolve_selection(&cfg, NONINTERACTIVE, &elig, Some("local")).unwrap();
    assert_eq!(sel.backend, "opi-sandbox");
}

#[test]
fn model_cannot_mutate_permission_or_trust() {
    // The model names an allowed backend; the resolution does not alter the
    // eligibility/permission inputs (purity) and grants nothing beyond selection.
    let cfg = ExecutionConfig {
        strategy: ExecutionStrategy::Model,
        ..ExecutionConfig::default()
    };
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Ask),
    ]);
    let elig_before = elig.clone();
    let sel = resolve_selection(&cfg, INTERACTIVE, &elig, Some("local")).unwrap();
    assert_eq!(sel.backend, "local");
    assert_eq!(
        elig, elig_before,
        "model selection must not mutate eligibility"
    );
    // opi-sandbox remains ask (the model did not grant it).
    assert_eq!(
        elig.find("opi-sandbox").unwrap().permission,
        PermissionDecision::Ask
    );
}

// ---------------------------------------------------------------------------
// DoD 4: rules preserve declaration order (first match wins)
// ---------------------------------------------------------------------------

#[test]
fn rules_first_match_wins_in_declaration_order() {
    let cfg = rules(vec![
        rule(Some(vec![NONINTERACTIVE]), "opi-sandbox"),
        rule(Some(vec![INTERACTIVE]), "local"),
        rule(None, "local"),
    ]);
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Allow),
    ]);
    assert_eq!(
        resolve_selection(&cfg, NONINTERACTIVE, &elig, None)
            .unwrap()
            .backend,
        "opi-sandbox"
    );
    assert_eq!(
        resolve_selection(&cfg, INTERACTIVE, &elig, None)
            .unwrap()
            .backend,
        "local"
    );
}

// ---------------------------------------------------------------------------
// DoD 5: a selected rule that fails does NOT fall through (exact code pinned)
// ---------------------------------------------------------------------------

#[test]
fn rules_selected_deny_does_not_fall_through_to_catch_all() {
    // rule[0] matches NonInteractive but is DENIED -> policy_denied, NOT local.
    let cfg = rules(vec![
        rule(Some(vec![NONINTERACTIVE]), "opi-sandbox"),
        rule(None, "local"),
    ]);
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Deny),
    ]);
    let err = resolve_selection(&cfg, NONINTERACTIVE, &elig, None).unwrap_err();
    assert_eq!(err.code(), "policy_denied");
}

#[test]
fn rules_selected_unavailable_does_not_fall_through() {
    // rule[0] matches NonInteractive but its backend is UNAVAILABLE -> adapter_unavailable.
    let cfg = rules(vec![
        rule(Some(vec![NONINTERACTIVE]), "opi-sandbox"),
        rule(None, "local"),
    ]);
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", false, PermissionDecision::Allow),
    ]);
    let err = resolve_selection(&cfg, NONINTERACTIVE, &elig, None).unwrap_err();
    assert_eq!(err.code(), "adapter_unavailable");
}

#[test]
fn no_selected_failure_retries_through_local() {
    // Design guarantee: a failing external selection never falls back to local.
    let cfg = fixed("opi-sandbox");
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Ask),
    ]);
    let err = resolve_selection(&cfg, INTERACTIVE, &elig, None).unwrap_err();
    assert_eq!(err.code(), "permission_required");
    // Not a silent local selection.
    assert!(matches!(
        err,
        opi_coding_agent::execution::ExecutionFailure::PermissionRequired { .. }
    ));
}

// ---------------------------------------------------------------------------
// DoD 10 / model strategy: the model can only select from the eligible set
// ---------------------------------------------------------------------------

#[test]
fn model_names_absent_backend_is_adapter_not_selected() {
    let cfg = ExecutionConfig {
        strategy: ExecutionStrategy::Model,
        ..ExecutionConfig::default()
    };
    let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
    let err = resolve_selection(&cfg, INTERACTIVE, &elig, Some("ghost")).unwrap_err();
    assert_eq!(err.code(), "adapter_not_selected");
}

#[test]
fn model_names_denied_backend_is_adapter_not_selected() {
    let cfg = ExecutionConfig {
        strategy: ExecutionStrategy::Model,
        ..ExecutionConfig::default()
    };
    let elig = eligibility(&[adapter("opi-sandbox", true, PermissionDecision::Deny)]);
    assert!(elig.model_visible_ids().is_empty());
    let err = resolve_selection(&cfg, INTERACTIVE, &elig, Some("opi-sandbox")).unwrap_err();
    assert_eq!(err.code(), "adapter_not_selected");
}

#[test]
fn model_visible_set_excludes_deny_keeps_ask() {
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Ask),
        adapter("denied-pkg", true, PermissionDecision::Deny),
    ]);
    let mut visible: Vec<&str> = elig.model_visible_ids();
    visible.sort();
    assert_eq!(visible, vec!["local", "opi-sandbox"]);
}

// ---------------------------------------------------------------------------
// Purity + no-degraded Ok shape (DoD 13 surface)
// ---------------------------------------------------------------------------

#[test]
fn resolve_selection_does_not_mutate_inputs() {
    let cfg = fixed("opi-sandbox");
    let elig = eligibility(&[
        adapter("local", true, PermissionDecision::Allow),
        adapter("opi-sandbox", true, PermissionDecision::Allow),
    ]);
    let cfg_before = cfg.clone();
    let elig_before = elig.clone();
    let _ = resolve_selection(&cfg, INTERACTIVE, &elig, Some("opi-sandbox"));
    assert_eq!(cfg, cfg_before);
    assert_eq!(elig, elig_before);
}

#[test]
fn ok_selection_carries_only_backend_and_mode() {
    // DoD 13: no degraded-success state. The Ok path is a plain Selection with
    // no warning/diagnostic side-channel.
    let cfg = fixed("local");
    let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
    let sel = resolve_selection(&cfg, NONINTERACTIVE, &elig, None).unwrap();
    assert_eq!(sel.backend, "local");
    assert_eq!(sel.mode, NONINTERACTIVE);
    // Compile-time shape guard: Selection has exactly these two fields.
    let _ = Selection {
        backend: String::new(),
        mode: INTERACTIVE,
    };
}

// ---------------------------------------------------------------------------
// Task 16.9 SC16-04: PRODUCTION startup path (CodingHarness::build_tools_with_sandbox)
// ---------------------------------------------------------------------------
// These drive the real production chokepoint (not `resolve_selection` directly)
// to prove the dynamic bash schema and the no-fallback diagnostic surfacing are
// wired end-to-end. The substrate router guarantees above cover
// `resolve_selection` in isolation; these cover that the harness actually calls
// the runtime assembly and injects its selected backend + schema into the
// production BashTool.

use std::sync::Arc;

use opi_agent::diagnostic::Severity;
use opi_coding_agent::config::{OpiConfig, SandboxConfig};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::execution::{EnabledIdentity, IdentitySource};
use opi_coding_agent::harness::{CodingHarness, ExecutionWiring};
use opi_coding_agent::package_activation::{
    ActivatedContribution, ActivationError, host_opi_version, host_target_triple,
};
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::sandbox::prepare_production;
use opi_coding_agent::tool::default_bash_schema;

/// A store that panics if activated. The production-path routing tests below
/// never EXECUTE a backend (they inspect the assembled schema / startup
/// diagnostics), so this surviving proves no activation happened at startup.
struct PanicSource;
impl IdentitySource for PanicSource {
    fn activate(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        panic!("production-path routing tests must not activate any package");
    }
}

fn identity(adapter_id: &str, pkg: &str) -> EnabledIdentity {
    EnabledIdentity {
        adapter_id: adapter_id.to_string(),
        package_name: pkg.to_string(),
    }
}

fn wiring(
    strategy: ExecutionStrategy,
    backend: &str,
    enabled: Vec<EnabledIdentity>,
    permissions: &[(&str, PermissionDecision)],
) -> ExecutionWiring {
    let perms: std::collections::BTreeMap<String, PermissionDecision> = permissions
        .iter()
        .map(|(k, v)| ((*k).to_string(), *v))
        .collect();
    let config = ExecutionConfig {
        strategy,
        backend: backend.to_string(),
        permissions: perms.clone(),
        ..ExecutionConfig::default()
    };
    ExecutionWiring {
        config,
        enabled,
        policy: PermissionPolicy::from_map(perms),
        store: Arc::new(PanicSource),
        mode: ExecutionRunMode::Interactive,
        host_target: host_target_triple().to_string(),
        host_opi_version: host_opi_version().to_string(),
    }
}

fn build_prod_tools(
    w: &ExecutionWiring,
) -> (
    Vec<Box<dyn opi_agent::tool::Tool>>,
    Vec<opi_agent::Diagnostic>,
) {
    let ws = tempfile::tempdir().unwrap();
    let tool_config =
        ToolRuntimeConfig::resolve(RunMode::Interactive, true, ToolSelection::Default)
            .expect("interactive tool config");
    let prepared = prepare_production(&SandboxConfig::default(), ws.path());
    CodingHarness::build_tools_with_sandbox(ws.path(), &tool_config, prepared, w)
}

fn bash_schema(tools: &[Box<dyn opi_agent::tool::Tool>]) -> serde_json::Value {
    tools
        .iter()
        .find(|t| t.definition().name == "bash")
        .expect("bash tool present")
        .definition()
        .input_schema
}

fn has_bash(tools: &[Box<dyn opi_agent::tool::Tool>]) -> bool {
    tools.iter().any(|t| t.definition().name == "bash")
}

/// Extract the `const` adapter ids from a model-strategy `backend` oneOf field.
fn backend_const_values(backend: &serde_json::Value) -> Vec<&str> {
    backend["oneOf"]
        .as_array()
        .expect("backend oneOf")
        .iter()
        .map(|v| v["const"].as_str().expect("const id"))
        .collect()
}

/// SC16-04: under `strategy = "model"`, the production bash schema gains a
/// REQUIRED bounded `backend` enum listing the model-visible (non-denied)
/// adapters. The schema diverges from the default.
#[test]
fn production_model_strategy_adds_required_backend_enum() {
    let w = wiring(
        ExecutionStrategy::Model,
        "local",
        vec![identity("opi-sandbox", "pkg")],
        &[("opi-sandbox", PermissionDecision::Allow)],
    );
    let (tools, diags) = build_prod_tools(&w);
    assert!(diags.is_empty(), "model allow must not warn: {diags:?}");
    let schema = bash_schema(&tools);
    let backend = &schema["properties"]["backend"];
    // oneOf of {const,...} variants — the bounded, model-visible backend set.
    let const_vals = backend_const_values(backend);
    assert!(
        const_vals.contains(&"local"),
        "local is always model-visible: {const_vals:?}"
    );
    assert!(
        const_vals.contains(&"opi-sandbox"),
        "allowed external must be model-visible: {const_vals:?}"
    );
    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("required")
        .iter()
        .map(|v| v.as_str().expect("required str"))
        .collect();
    assert!(
        required.contains(&"backend"),
        "backend must be required under model: {required:?}"
    );
    assert_eq!(
        schema["additionalProperties"], false,
        "model schema must close additional properties"
    );
    assert_ne!(
        schema,
        default_bash_schema(),
        "model schema must diverge from the default"
    );
}

/// SC16-04: a DENIED adapter is absent from the model `backend` enum — the model
/// is never offered a backend it cannot select.
#[test]
fn production_model_strategy_excludes_denied_adapter_from_enum() {
    let w = wiring(
        ExecutionStrategy::Model,
        "local",
        vec![identity("opi-sandbox", "pkg")],
        &[("opi-sandbox", PermissionDecision::Deny)],
    );
    let (tools, _diags) = build_prod_tools(&w);
    let schema = bash_schema(&tools);
    let const_vals = backend_const_values(&schema["properties"]["backend"]);
    assert!(const_vals.contains(&"local"), "local remains visible");
    assert!(
        !const_vals.contains(&"opi-sandbox"),
        "denied adapter must be absent from the model backend set: {const_vals:?}"
    );
}

/// SC16-04 + design §Model routing: an `ask` candidate is VISIBLE (selectable)
/// but its oneOf variant describes that it requires interactive approval, while
/// an `allow` candidate's description does not. (A `deny` candidate is absent —
/// covered above.) This pins the per-candidate approval hint the model schema
/// owes the model.
#[test]
fn production_model_strategy_ask_candidate_describes_interactive_approval() {
    let w = wiring(
        ExecutionStrategy::Model,
        "local",
        vec![identity("opi-sandbox", "pkg")],
        &[("opi-sandbox", PermissionDecision::Ask)],
    );
    let (tools, _diags) = build_prod_tools(&w);
    let backend = &bash_schema(&tools)["properties"]["backend"];
    let by_id: std::collections::HashMap<&str, &str> = backend["oneOf"]
        .as_array()
        .expect("backend oneOf")
        .iter()
        .map(|v| {
            (
                v["const"].as_str().expect("const id"),
                v["description"].as_str().unwrap_or(""),
            )
        })
        .collect();
    let ask_desc = by_id.get("opi-sandbox").copied().unwrap_or("");
    assert!(
        ask_desc.contains("interactive approval"),
        "ask candidate must describe interactive approval: {ask_desc:?}"
    );
    let allow_desc = by_id.get("local").copied().unwrap_or("");
    assert!(
        !allow_desc.contains("interactive approval"),
        "allow candidate must not mention approval: {allow_desc:?}"
    );
}

/// SC16-04: `fixed`/`rules`/default modes do NOT add the `backend` field — the
/// schema is the default byte-for-byte even when routing selects an external.
#[test]
fn production_fixed_strategy_omits_backend_field() {
    let w = wiring(
        ExecutionStrategy::Fixed,
        "opi-sandbox",
        vec![identity("opi-sandbox", "pkg")],
        &[("opi-sandbox", PermissionDecision::Allow)],
    );
    let (tools, _diags) = build_prod_tools(&w);
    assert_eq!(
        bash_schema(&tools),
        default_bash_schema(),
        "fixed strategy must keep the default schema (no backend field)"
    );
}

/// SC16-04 + SC16-07: a startup build failure OMITS the bash tool — no `local`
/// fallback — and surfaces the stable `policy_denied` code via the startup
/// diagnostic channel (the cross-mode surface). `ExecutionRuntime::build`
/// Branch 1 (Minimal Runtime) is the only build-time Err path: an explicit
/// `local = "deny"` under default-local routing. (Routed configs construct
/// `RoutedBashOperations` at build and surface their deny/unavailable at EXEC
/// time as a tool failure — covered by the substrate suite.) This is the
/// production-path proof of the no-fallback + stable-code contract.
#[test]
fn production_minimal_runtime_local_deny_omits_bash_and_surfaces_stable_code() {
    let w = wiring(
        ExecutionStrategy::Fixed,
        "local",
        vec![],
        &[("local", PermissionDecision::Deny)],
    );
    let (tools, diags) = build_prod_tools(&w);
    assert!(
        !has_bash(&tools),
        "a denied local backend must omit the bash tool (no fallback)"
    );
    assert_eq!(diags.len(), 1, "exactly one startup diagnostic: {diags:?}");
    let diag = &diags[0];
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(
        diag.code, "adapter_startup_failed",
        "stable shared startup code"
    );
    let details = diag
        .details
        .as_ref()
        .expect("details carry the granular code");
    assert_eq!(
        details["code"], "policy_denied",
        "granular execution failure code"
    );
}

/// SC16-04: `--execution-backend` / `--execution-strategy` are direct-testable
/// config overrides that touch ONLY strategy/backend — never trust or permission
/// (the permissions map is byte-identical before and after). The startup
/// resolvers call this hook after config resolution (main.rs); the model-strategy
/// production test above proves the resolved strategy reaches the harness schema.
#[test]
fn apply_execution_overrides_sets_backend_strategy_without_touching_permissions() {
    let mut config = OpiConfig::default();
    config
        .execution
        .permissions
        .insert("local".to_string(), PermissionDecision::Ask);
    let permissions_before = config.execution.permissions.clone();

    // Backend override selects the fixed strategy with that backend.
    config.apply_execution_overrides(Some("opi-sandbox"), None);
    assert_eq!(config.execution.backend, "opi-sandbox");
    assert_eq!(config.execution.strategy, ExecutionStrategy::Fixed);
    assert_eq!(
        config.execution.permissions, permissions_before,
        "permissions map must be untouched"
    );

    // Strategy override alone.
    let mut config = OpiConfig::default();
    config.apply_execution_overrides(None, Some(ExecutionStrategy::Model));
    assert_eq!(config.execution.strategy, ExecutionStrategy::Model);

    // Both: backend sets Fixed, then strategy overrides while keeping the backend.
    let mut config = OpiConfig::default();
    config.apply_execution_overrides(Some("opi-sandbox"), Some(ExecutionStrategy::Model));
    assert_eq!(config.execution.backend, "opi-sandbox");
    assert_eq!(config.execution.strategy, ExecutionStrategy::Model);
}

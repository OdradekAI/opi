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

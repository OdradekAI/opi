//! Deterministic `command.execute` routing: resolve which backend adapter serves
//! an invocation under the configured `fixed` / `rules` / `model` strategy.
//!
//! This is a **pure resolution seam**: it takes the resolved
//! [`ExecutionConfig`], the invocation [`ExecutionRunMode`], a server-derived
//! [`Eligibility`] set, and (for `model` strategy) an optional model-supplied
//! backend, and returns either a [`Selection`] or a stable
//! [`ExecutionFailure`]. It never queries the package store, never spawns a
//! process, never reads the model, and never mutates its inputs. The 16.8
//! runtime builds the `Eligibility` input from a construction-validated package
//! identity catalog plus the resolved permission policy. The production catalog
//! contains construction-validated identities, but it is not an authoritative
//! process-start availability claim: the selected external package is
//! revalidated at invocation time immediately before spawn. The 16.7 protocol
//! host then turns a [`Selection`] into a live backend process.
//!
//! # Guarantees (Phase 16 design)
//!
//! - **Deterministic**: `fixed`, `rules`, and `model` each select exactly one
//!   backend or fail.
//! - **No fallthrough after selection**: under `rules`, the first rule matching
//!   the run mode wins; if that backend is then denied/unavailable/unpermitted,
//!   the resolution fails and does NOT continue to the next rule.
//! - **Model non-authority**: a model-supplied `backend` only *selects* from the
//!   eligible set; it cannot name a denied/absent adapter, and `fixed`/`rules`
//!   ignore it entirely. The model cannot mutate install, trust, enablement,
//!   policy, or grants.
//! - **One process, no retry**: no failure path selects another adapter.

use crate::config::{ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision};

use super::failure::ExecutionFailure;

/// A router eligibility entry. `local` is a synthesized built-in entry.
/// External entries come from construction-validated package identities that
/// are installed, trusted, enabled, and target-compatible. Each entry is
/// annotated with the resolved permission decision.
/// `available` is a router input, not an authoritative external process-start
/// guarantee;
/// production revalidates the selected external package immediately before
/// spawn. `permission` is the resolved deny/ask/allow for that adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleAdapter {
    pub id: String,
    pub available: bool,
    pub permission: PermissionDecision,
}

/// The full eligible-adapter set passed to the router. Includes denied entries
/// (so fixed/rules can report `policy_denied`); the 16.8 model-strategy JSON
/// enum is built by filtering this set to `available && !deny`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Eligibility(pub Vec<EligibleAdapter>);

impl Eligibility {
    /// Look up an adapter by id.
    pub fn find(&self, id: &str) -> Option<&EligibleAdapter> {
        self.0.iter().find(|a| a.id == id)
    }

    /// The model-visible adapter ids: `available && !deny`. (`deny` is absent;
    /// `ask` is visible as requiring approval.) 16.8 uses this to build the
    /// `backend` enum in the model-strategy tool schema.
    pub fn model_visible_ids(&self) -> Vec<&str> {
        self.0
            .iter()
            .filter(|a| a.available && a.permission != PermissionDecision::Deny)
            .map(|a| a.id.as_str())
            .collect()
    }
}

/// A resolved backend selection. The backend id is always a member of the input
/// [`Eligibility`]. Carries no warning/diagnostic field: there is no degraded
/// success state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub backend: String,
    pub mode: ExecutionRunMode,
}

/// Router result that still identifies the construction-validated candidate
/// by position. The execution runtime uses this index to carry the concrete
/// dispatch target through permission approval without a second id lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateSelection {
    pub(crate) index: usize,
    pub(crate) backend: String,
    pub(crate) mode: ExecutionRunMode,
}

/// Permission-aware result of resolving one configured/model-supplied
/// candidate. `Ask` deliberately retains the selected candidate so the
/// interactive grant path cannot lose its concrete dispatch target.
pub(crate) enum CandidateDecision {
    Allowed(CandidateSelection),
    Ask(CandidateSelection),
    Refused(ExecutionFailure),
}

/// Resolve the backend for one invocation.
///
/// See the module docs for the determinism, no-fallthrough, and model
/// non-authority guarantees.
pub fn resolve_selection(
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: &Eligibility,
    model_backend: Option<&str>,
) -> Result<Selection, ExecutionFailure> {
    match resolve_candidate(config, mode, eligibility, model_backend) {
        CandidateDecision::Allowed(candidate) => Ok(Selection {
            backend: candidate.backend,
            mode: candidate.mode,
        }),
        CandidateDecision::Ask(candidate) => Err(ExecutionFailure::PermissionRequired {
            adapter_id: candidate.backend,
            mode: candidate.mode,
        }),
        CandidateDecision::Refused(failure) => Err(failure),
    }
}

/// Resolve a candidate while retaining its construction index across an
/// interactive `ask` decision. This is the runtime-facing form of
/// [`resolve_selection`]; both functions share the exact strategy semantics.
pub(crate) fn resolve_candidate(
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: &Eligibility,
    model_backend: Option<&str>,
) -> CandidateDecision {
    match config.strategy {
        ExecutionStrategy::Fixed => {
            select_named_candidate(&config.backend, ExecutionStrategy::Fixed, mode, eligibility)
        }
        ExecutionStrategy::Rules => resolve_rules_candidate(config, mode, eligibility),
        ExecutionStrategy::Model => resolve_model_candidate(model_backend, mode, eligibility),
    }
}

/// Resolve the concrete adapter named by a deterministic Routing Strategy.
///
/// Fixed routing names its configured backend. Rules routing names only the
/// first rule matching `mode`; a later catch-all is not considered once an
/// earlier rule matches. Model routing has no concrete identity until the
/// invocation supplies one.
pub(crate) fn concrete_adapter_id(
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
) -> Option<&str> {
    match config.strategy {
        ExecutionStrategy::Fixed => Some(&config.backend),
        ExecutionStrategy::Rules => config.rules.iter().find_map(|rule| {
            rule.modes
                .as_ref()
                .is_none_or(|modes| modes.contains(&mode))
                .then_some(rule.backend.as_str())
        }),
        ExecutionStrategy::Model => None,
    }
}

/// `fixed`: select the configured backend, then gate it on availability and
/// permission. The model-supplied backend is ignored under `fixed`.
fn select_named_candidate(
    backend: &str,
    strategy: ExecutionStrategy,
    mode: ExecutionRunMode,
    eligibility: &Eligibility,
) -> CandidateDecision {
    let Some((index, entry)) = eligibility
        .0
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.id == backend)
    else {
        return CandidateDecision::Refused(ExecutionFailure::NoEligibleAdapter { strategy, mode });
    };
    gate_candidate(index, entry, mode)
}

/// `rules`: the first rule whose `modes` matches wins. A catch-all rule
/// (`modes` absent) matches every mode. The matched rule's
/// backend is then gated; a gate failure does NOT fall through to a later rule.
fn resolve_rules_candidate(
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: &Eligibility,
) -> CandidateDecision {
    let Some(backend) = concrete_adapter_id(config, mode) else {
        return CandidateDecision::Refused(ExecutionFailure::NoEligibleAdapter {
            strategy: ExecutionStrategy::Rules,
            mode,
        });
    };
    select_named_candidate(backend, ExecutionStrategy::Rules, mode, eligibility)
}

/// `model`: the model supplies a backend id. It must be model-visible
/// (`available && !deny`); otherwise the model attempted to select something it
/// was never offered. Then gate on ask/allow.
fn resolve_model_candidate(
    model_backend: Option<&str>,
    mode: ExecutionRunMode,
    eligibility: &Eligibility,
) -> CandidateDecision {
    let Some(requested) = model_backend else {
        return CandidateDecision::Refused(ExecutionFailure::AdapterNotSelected {
            requested: "<model omitted backend>".to_string(),
            strategy: ExecutionStrategy::Model,
        });
    };
    let Some((index, entry)) = eligibility
        .0
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.id == requested)
    else {
        return CandidateDecision::Refused(ExecutionFailure::AdapterNotSelected {
            requested: requested.to_string(),
            strategy: ExecutionStrategy::Model,
        });
    };
    if !entry.available || entry.permission == PermissionDecision::Deny {
        return CandidateDecision::Refused(ExecutionFailure::AdapterNotSelected {
            requested: requested.to_string(),
            strategy: ExecutionStrategy::Model,
        });
    }
    gate_candidate(index, entry, mode)
}

/// Gate a selected, present, available adapter on its permission decision.
fn gate_candidate(
    index: usize,
    entry: &EligibleAdapter,
    mode: ExecutionRunMode,
) -> CandidateDecision {
    if !entry.available {
        return CandidateDecision::Refused(ExecutionFailure::AdapterUnavailable {
            adapter_id: Some(entry.id.clone()),
            detail: super::failure::UnavailableDetail::Ineligible,
        });
    }
    let candidate = CandidateSelection {
        index,
        backend: entry.id.clone(),
        mode,
    };
    match entry.permission {
        PermissionDecision::Deny => CandidateDecision::Refused(ExecutionFailure::PolicyDenied {
            adapter_id: entry.id.clone(),
        }),
        PermissionDecision::Ask => CandidateDecision::Ask(candidate),
        PermissionDecision::Allow => CandidateDecision::Allowed(candidate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionRule;

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

    #[test]
    fn fixed_selects_local_allow() {
        let cfg = fixed("local");
        let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
        let sel = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap();
        assert_eq!(sel.backend, "local");
    }

    #[test]
    fn fixed_ignores_model_backend() {
        // DoD: under fixed, the model backend is ignored — same selection as None.
        let cfg = fixed("local");
        let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
        let none_sel = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap();
        let some_sel = resolve_selection(
            &cfg,
            ExecutionRunMode::Interactive,
            &elig,
            Some("opi-sandbox"),
        )
        .unwrap();
        assert_eq!(none_sel, some_sel);
        assert_eq!(some_sel.backend, "local");
    }

    #[test]
    fn fixed_deny_backend_is_policy_denied() {
        let cfg = fixed("opi-sandbox");
        let elig = eligibility(&[adapter("opi-sandbox", true, PermissionDecision::Deny)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap_err();
        assert_eq!(err.code(), "policy_denied");
    }

    #[test]
    fn fixed_ask_backend_is_permission_required() {
        let cfg = fixed("opi-sandbox");
        let elig = eligibility(&[adapter("opi-sandbox", true, PermissionDecision::Ask)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Rpc, &elig, None).unwrap_err();
        assert_eq!(err.code(), "permission_required");
    }

    #[test]
    fn fixed_unknown_backend_is_no_eligible_adapter() {
        let cfg = fixed("ghost");
        let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap_err();
        assert_eq!(err.code(), "no_eligible_adapter");
    }

    #[test]
    fn fixed_unavailable_backend_is_adapter_unavailable() {
        let cfg = fixed("opi-sandbox");
        let elig = eligibility(&[adapter("opi-sandbox", false, PermissionDecision::Allow)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap_err();
        assert_eq!(err.code(), "adapter_unavailable");
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

    #[test]
    fn rules_first_match_wins_in_declaration_order() {
        let cfg = rules(vec![
            rule(Some(vec![ExecutionRunMode::NonInteractive]), "opi-sandbox"),
            rule(None, "local"),
        ]);
        let elig = eligibility(&[
            adapter("local", true, PermissionDecision::Allow),
            adapter("opi-sandbox", true, PermissionDecision::Allow),
        ]);
        // NonInteractive matches rule[0] -> opi-sandbox.
        let sel = resolve_selection(&cfg, ExecutionRunMode::NonInteractive, &elig, None).unwrap();
        assert_eq!(sel.backend, "opi-sandbox");
        // Interactive matches no specific rule -> catch-all -> local.
        let sel = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap();
        assert_eq!(sel.backend, "local");
    }

    #[test]
    fn rules_selected_backend_failure_does_not_fall_through() {
        // rule[0] matches NonInteractive but its backend is DENIED; resolution
        // must fail with policy_denied, NOT fall through to the catch-all local.
        let cfg = rules(vec![
            rule(Some(vec![ExecutionRunMode::NonInteractive]), "opi-sandbox"),
            rule(None, "local"),
        ]);
        let elig = eligibility(&[
            adapter("local", true, PermissionDecision::Allow),
            adapter("opi-sandbox", true, PermissionDecision::Deny),
        ]);
        let err =
            resolve_selection(&cfg, ExecutionRunMode::NonInteractive, &elig, None).unwrap_err();
        assert_eq!(err.code(), "policy_denied");
    }

    #[test]
    fn rules_missing_selected_backend_reports_rules_strategy() {
        let cfg = rules(vec![rule(None, "missing")]);
        let err = resolve_selection(
            &cfg,
            ExecutionRunMode::Interactive,
            &Eligibility::default(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ExecutionFailure::NoEligibleAdapter {
                strategy: ExecutionStrategy::Rules,
                mode: ExecutionRunMode::Interactive,
            }
        ));
    }

    #[test]
    fn model_selects_visible_allow_backend() {
        let cfg = ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        };
        let elig = eligibility(&[
            adapter("local", true, PermissionDecision::Allow),
            adapter("opi-sandbox", true, PermissionDecision::Allow),
        ]);
        let sel = resolve_selection(
            &cfg,
            ExecutionRunMode::Interactive,
            &elig,
            Some("opi-sandbox"),
        )
        .unwrap();
        assert_eq!(sel.backend, "opi-sandbox");
    }

    #[test]
    fn model_cannot_select_absent_backend() {
        let cfg = ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        };
        let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, Some("ghost"))
            .unwrap_err();
        assert_eq!(err.code(), "adapter_not_selected");
    }

    #[test]
    fn model_cannot_select_denied_backend() {
        // deny is filtered from the model-visible enum; naming it is rejected.
        let cfg = ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        };
        let elig = eligibility(&[adapter("opi-sandbox", true, PermissionDecision::Deny)]);
        assert_eq!(elig.model_visible_ids(), Vec::<&str>::new());
        let err = resolve_selection(
            &cfg,
            ExecutionRunMode::Interactive,
            &elig,
            Some("opi-sandbox"),
        )
        .unwrap_err();
        assert_eq!(err.code(), "adapter_not_selected");
    }

    #[test]
    fn model_omitting_backend_is_adapter_not_selected() {
        let cfg = ExecutionConfig {
            strategy: ExecutionStrategy::Model,
            ..ExecutionConfig::default()
        };
        let elig = eligibility(&[adapter("local", true, PermissionDecision::Allow)]);
        let err = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, None).unwrap_err();
        assert_eq!(err.code(), "adapter_not_selected");
    }

    #[test]
    fn resolve_selection_does_not_mutate_inputs() {
        // Purity: inputs are byte-identical after the call.
        let cfg = fixed("opi-sandbox");
        let elig = eligibility(&[
            adapter("local", true, PermissionDecision::Allow),
            adapter("opi-sandbox", true, PermissionDecision::Ask),
        ]);
        let cfg_before = cfg.clone();
        let elig_before = elig.clone();
        let _ = resolve_selection(&cfg, ExecutionRunMode::Interactive, &elig, Some("local"));
        assert_eq!(cfg, cfg_before);
        assert_eq!(elig, elig_before);
    }
}

//! Capability Permission policy for the `command.execute` Capability.
//!
//! This owns the resolved per-adapter permission *decision* and the defaulting
//! rule. It is intentionally decoupled from the bash tool-availability gate
//! (`--allow-mutating` / `policy::resolve_active_tool_names`): enabling the `bash`
//! tool does not authorize an external adapter, and authorizing an adapter does
//! not enable the `bash` tool. The two concerns share no state.
//!
//! The resolved permission *map* is user-owned: it is loaded from the user +
//! explicit config layers only; a project `[execution.permissions]` section is
//! rejected upstream in `config` (even when the project is trusted). This module
//! consumes the already-resolved map and applies the defaulting rule below.
//!
//! # Defaulting (Phase 16 design)
//!
//! - `local` (the built-in host backend) defaults to [`PermissionDecision::Allow`]
//!   when absent from the resolved map. An explicit user `deny`/`ask` for `local`
//!   is honored exactly like any other adapter.
//! - Any other (external) adapter id defaults to [`PermissionDecision::Ask`] when
//!   absent — selectable, but requiring an interactive grant.
//!
//! `deny` makes an adapter ineligible and model-invisible; `ask` makes it
//! selectable but requires a one-invocation or current-session grant; `allow` is
//! persistent user authorization. The current-session grant is memory-only and
//! does not survive restart/resume/fork (owned by 16.7/16.8 runtime, not here).

use std::collections::BTreeMap;

use crate::config::PermissionDecision;

/// The reserved adapter id of the built-in host backend.
pub const LOCAL_ADAPTER_ID: &str = "local";

/// Resolved capability-permission policy over adapter ids.
///
/// Wraps the user-owned `[execution.permissions]` map and applies the `local`
/// (Allow) / external (Ask) defaulting rule for ids absent from the map.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionPolicy {
    decisions: BTreeMap<String, PermissionDecision>,
}

impl PermissionPolicy {
    /// Build a policy from a resolved permission map (user + explicit layers;
    /// project layers are rejected before this is constructed).
    pub fn from_map(decisions: BTreeMap<String, PermissionDecision>) -> Self {
        Self { decisions }
    }

    /// An empty policy (everything defaults: `local` -> Allow, others -> Ask).
    pub fn empty() -> Self {
        Self::default()
    }

    /// The resolved decision for `adapter_id`, applying the defaulting rule.
    pub fn decision_for(&self, adapter_id: &str) -> PermissionDecision {
        if let Some(decision) = self.decisions.get(adapter_id) {
            *decision
        } else if adapter_id == LOCAL_ADAPTER_ID {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Ask
        }
    }

    /// True iff `adapter_id` is denied (explicitly, or — for `local` — only when
    /// the user explicitly denies it).
    pub fn is_denied(&self, adapter_id: &str) -> bool {
        self.decision_for(adapter_id) == PermissionDecision::Deny
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, PermissionDecision)]) -> BTreeMap<String, PermissionDecision> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn local_defaults_to_allow_when_absent() {
        let p = PermissionPolicy::empty();
        assert_eq!(p.decision_for("local"), PermissionDecision::Allow);
    }

    #[test]
    fn external_defaults_to_ask_when_absent() {
        let p = PermissionPolicy::empty();
        assert_eq!(p.decision_for("opi-sandbox"), PermissionDecision::Ask);
    }

    #[test]
    fn explicit_local_deny_is_honored() {
        let p = PermissionPolicy::from_map(map(&[("local", PermissionDecision::Deny)]));
        assert_eq!(p.decision_for("local"), PermissionDecision::Deny);
        assert!(p.is_denied("local"));
    }

    #[test]
    fn explicit_external_allow_is_honored() {
        let p = PermissionPolicy::from_map(map(&[("opi-sandbox", PermissionDecision::Allow)]));
        assert_eq!(p.decision_for("opi-sandbox"), PermissionDecision::Allow);
        assert!(!p.is_denied("opi-sandbox"));
    }

    #[test]
    fn local_explicit_ask_is_honored_over_default() {
        let p = PermissionPolicy::from_map(map(&[("local", PermissionDecision::Ask)]));
        assert_eq!(p.decision_for("local"), PermissionDecision::Ask);
    }
}

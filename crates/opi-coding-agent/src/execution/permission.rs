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
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use opi_tui::{PermissionChoice, PermissionSummary};

use crate::config::{ExecutionRunMode, PermissionDecision};

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

    /// The resolved (adapter_id, decision) entries in canonical sorted order, for
    /// digest-addressing an immutable policy snapshot. Exposes only the
    /// already-resolved user/explicit-layer decisions, not the defaulting rule.
    pub fn canonical_entries(&self) -> Vec<(&str, PermissionDecision)> {
        self.decisions
            .iter()
            .map(|(id, d)| (id.as_str(), *d))
            .collect()
    }
}

// =========================================================================
// PermissionManager — in-memory session grants (Phase 16 task 16.10)
// =========================================================================

/// In-memory store of interactive `ask` grants for the current harness session.
///
/// Holds session grants only (`allow-for-session` choices). It is **memory-only**:
/// it has no `Serialize`/`Deserialize`, is never registered with the
/// `ExtensionRegistry`, is never passed to the session writer, and never touches
/// `trust.json` / `ProjectTrustStore`. A grant therefore cannot survive process
/// restart, session resume, fork, or branch — those either start a fresh process
/// (fresh harness → fresh manager) or, for the in-process interactive switchers,
/// call [`Self::reset_grants`] at the top of the switch.
///
/// Shared by reference (`Arc<PermissionManager>`) between the routed bash backend
/// (which checks + records grants during `exec`) and the harness (which resets
/// them on in-process session switches), so a reset is immediately visible to the
/// next tool call. One fresh manager is constructed per routed harness; Minimal
/// Runtime and startup-refused harnesses construct no permission manager.
#[derive(Debug, Default)]
pub struct PermissionManager {
    session_grants: Mutex<HashSet<String>>,
}

impl PermissionManager {
    /// A fresh, grant-less manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// True iff `adapter_id` has a live session grant (an `allow-for-session`
    /// choice recorded earlier this session). Checked by the routed backend
    /// before prompting, so a session grant suppresses re-prompts.
    pub fn has_session_grant(&self, adapter_id: &str) -> bool {
        self.session_grants
            .lock()
            .expect("permission grant set poisoned")
            .contains(adapter_id)
    }

    /// Record an `allow-for-session` grant for `adapter_id`. The grant is
    /// memory-only and lives until [`Self::reset_grants`] or process exit.
    pub fn grant_session(&self, adapter_id: &str) {
        self.session_grants
            .lock()
            .expect("permission grant set poisoned")
            .insert(adapter_id.to_string());
    }

    /// Drop every session grant. Called by the harness on in-process session
    /// switches (resume / fork / branch) so an `allow-for-session` choice made
    /// in one session does not authorize an adapter in another.
    pub fn reset_grants(&self) {
        self.session_grants
            .lock()
            .expect("permission grant set poisoned")
            .clear();
    }
}

/// A short, redaction-safe run-mode label for a [`PermissionSummary`].
pub(crate) fn run_mode_label(mode: ExecutionRunMode) -> &'static str {
    match mode {
        ExecutionRunMode::Interactive => "interactive",
        ExecutionRunMode::NonInteractive => "non-interactive",
        ExecutionRunMode::Rpc => "rpc",
    }
}

// =========================================================================
// InteractivePermissionBroker — the ask-prompt seam (Phase 16 task 16.10)
// =========================================================================

/// The interactive `ask`-prompt seam.
///
/// Consulted by `RoutedBashOperations::exec` **only** in
/// [`ExecutionRunMode::Interactive`] when an adapter's resolved permission is
/// `ask` and no session grant covers it. Headless modes (`NonInteractive`,
/// `Rpc`) never invoke this — the pure router gate returns `permission_required`
/// unchanged, surfacing the DoD-mandated `permission_required` code.
///
/// `None` (no broker installed) is the fail-closed default: an interactive `ask`
/// surfaces `permission_required` rather than silently dispatching or falling
/// back to `local`.
///
/// Implementations MUST be cancellation- and drop-safe: a cancelled future or a
/// dropped responder (terminal close) resolves to [`PermissionChoice::Deny`],
/// never a panic or hang, so the tool call surfaces a stable `permission_denied`.
pub trait InteractivePermissionBroker: Send + Sync {
    /// Render the prompt for `summary` and resolve to the user's choice. The
    /// summary is redaction-safe (adapter id, package name, run-mode label);
    /// implementations MUST NOT receive or render command text, env, or paths.
    fn resolve_ask(
        &self,
        summary: PermissionSummary,
    ) -> Pin<Box<dyn Future<Output = PermissionChoice> + Send + '_>>;
}

/// Wrap a fixed [`PermissionChoice`] as a broker (test/substrate seam). Production
/// uses the TUI-backed broker; headless installs no broker at all.
pub struct FixedChoiceBroker {
    choice: PermissionChoice,
}

impl FixedChoiceBroker {
    pub fn new(choice: PermissionChoice) -> Arc<Self> {
        Arc::new(Self { choice })
    }
}

impl InteractivePermissionBroker for FixedChoiceBroker {
    fn resolve_ask(
        &self,
        _summary: PermissionSummary,
    ) -> Pin<Box<dyn Future<Output = PermissionChoice> + Send + '_>> {
        let choice = self.choice;
        Box::pin(async move { choice })
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

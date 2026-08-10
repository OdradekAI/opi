//! Stable `ExecutionFailure` envelope for the `command.execute` Capability.
//!
//! This is the architecture-level failure surface the router, permission policy,
//! protocol host (Phase 16.7), and runtime (16.8/16.9) surface on every selected
//! external adapter failure. It carries the **stable redacted code** set named by
//! the Phase 16 design plus actionable remediation. There is intentionally **no
//! degraded-success state**: a command either reports its effective contract or
//! fails with one of these codes.
//!
//! # Redaction
//!
//! [`ExecutionFailure::remediation`] and the `Display` impl interpolate ONLY safe
//! identifiers — package name, adapter id, run-mode/strategy label. They never
//! interpolate command text, environment values, credentials, absolute paths, or
//! raw backend stderr. In particular, `From<ActivationError>` drops the untrusted
//! `detail` string (which may carry an absolute path) and the opaque `Store`
//! display, mapping them to code-keyed remediation instead.
//!
//! # Producer split
//!
//! `permission_denied` is produced by the Phase 16.10 interactive grant layer
//! (a user denies / cancels an `ask` prompt). The `protocol_*`/`execution_*`
//! codes and `cleanup_unconfirmed` are produced by the 16.7 protocol host
//! ([`crate::execution::protocol_host`]), which is driven end-to-end by the
//! production `ExecutionRuntime::build` -> `RoutedBashOperations` ->
//! `ProcessCommandAdapter` -> `ExecutionProtocolHost::execute` path (proven by
//! `tests/execution_product.rs::mock_peer_failure_modes_surface_stable_codes_via_production_path`
//! and `tests/execution_protocol_host.rs`). Every variant is constructed in
//! reachable non-test code, so no `#[allow(dead_code)]` is needed.

use crate::config::{ExecutionRunMode, ExecutionStrategy};
use crate::package_activation::ActivationError;

/// Stable public placeholder for an omitted, unknown, or otherwise
/// unselectable model-supplied backend. Raw model input is never echoed.
pub const REDACTED_BACKEND_PLACEHOLDER: &str = "<unavailable>";

/// One stable, redacted command-execution failure. See the module docs for the
/// redaction and phase-split rules.
#[derive(Debug, thiserror::Error)]
pub enum ExecutionFailure {
    #[error("package {name:?} is not installed")]
    PackageNotInstalled { name: String },

    #[error("package {name:?} is not trusted")]
    PackageUntrusted { name: String },

    #[error("contribution from package {name:?} is disabled")]
    ContributionDisabled { name: String },

    #[error("adapter {adapter_id:?} is denied by execution permission policy")]
    PolicyDenied { adapter_id: String },

    #[error("adapter {adapter_id:?} requires interactive approval in {mode} mode")]
    PermissionRequired {
        adapter_id: String,
        mode: ExecutionRunMode,
    },

    /// Produced by the interactive grant layer when the user rejects an `ask`
    /// prompt (Phase 16.10), or when a prompt is cancelled/aborted/dropped
    /// (Esc / abort / terminal close all resolve to deny). Distinct from
    /// [`Self::PermissionRequired`] (headless `ask`, not-yet-granted) and
    /// [`Self::PolicyDenied`] (pre-policy `deny`).
    #[error("adapter {adapter_id:?} was not approved for this invocation")]
    PermissionDenied { adapter_id: String },

    #[error("no eligible command.execute adapter for strategy {strategy} in {mode} mode")]
    NoEligibleAdapter {
        strategy: ExecutionStrategy,
        mode: ExecutionRunMode,
    },

    #[error("requested backend <unavailable> is not selectable under strategy {strategy}")]
    AdapterNotSelected {
        requested: String,
        strategy: ExecutionStrategy,
    },

    #[error("adapter could not be activated")]
    AdapterUnavailable {
        adapter_id: Option<String>,
        detail: UnavailableDetail,
    },

    /// Produced by the 16.7 protocol host on a wire/identity mismatch.
    #[error("adapter protocol mismatch")]
    ProtocolIncompatible,

    /// Produced by the 16.7 protocol host on a malformed/out-of-order frame.
    #[error("adapter protocol violation")]
    ProtocolViolation,

    /// Produced by the 16.7 protocol host on a non-protocol execution failure.
    #[error("command execution failed")]
    ExecutionFailed,

    /// Produced by the 16.7 protocol host from a backend-reported
    /// `Failed{ExecutionTimedOut}` frame, including a reason-consistent
    /// pre-disclosure response to the host deadline. Host deadline expiry with
    /// no valid terminal cleanup report maps to [`Self::CleanupUnconfirmed`].
    #[error("command execution timed out")]
    ExecutionTimedOut,

    /// Produced by the 16.7 protocol host when cleanup state is unconfirmed.
    #[error("command cleanup state unconfirmed")]
    CleanupUnconfirmed,
}

/// Closed, redaction-safe reason carried by [`ExecutionFailure::AdapterUnavailable`].
/// Never carries command text, env values, credentials, absolute paths, or raw
/// stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableDetail {
    /// Package store I/O or integrity error (no specific adapter identity).
    Store,
    /// Selected eligibility entry is not currently usable. This is distinct
    /// from an activation-store read/integrity failure.
    Ineligible,
    /// Adapter id collides with another package.
    Collision,
    /// Backend reported that it was unavailable before target start.
    Handshake,
}

impl ExecutionFailure {
    /// Validated adapter identity safe to expose in structured diagnostics.
    /// Raw model-supplied backend text is never returned here.
    pub fn adapter_id(&self) -> Option<&str> {
        match self {
            Self::AdapterUnavailable {
                adapter_id: Some(adapter_id),
                ..
            } => Some(adapter_id),
            _ => None,
        }
    }

    /// The stable wire code (one of the 14 design codes). Embedders match this
    /// string exactly; it never changes for a given failure kind.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PackageNotInstalled { .. } => "package_not_installed",
            Self::PackageUntrusted { .. } => "package_untrusted",
            Self::ContributionDisabled { .. } => "contribution_disabled",
            Self::PolicyDenied { .. } => "policy_denied",
            Self::PermissionRequired { .. } => "permission_required",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::NoEligibleAdapter { .. } => "no_eligible_adapter",
            Self::AdapterNotSelected { .. } => "adapter_not_selected",
            Self::AdapterUnavailable { .. } => "adapter_unavailable",
            Self::ProtocolIncompatible => "protocol_incompatible",
            Self::ProtocolViolation => "protocol_violation",
            Self::ExecutionFailed => "execution_failed",
            Self::ExecutionTimedOut => "execution_timed_out",
            Self::CleanupUnconfirmed => "cleanup_unconfirmed",
        }
    }

    /// Actionable, redacted remediation text. Interpolates only safe identifiers
    /// (package name, adapter id, mode/strategy label); never command text, env
    /// values, credentials, absolute paths, or raw backend stderr.
    pub fn remediation(&self) -> String {
        match self {
            Self::PackageNotInstalled { name } => format!(
                "Adapter package {name:?} is not installed. Install it with \
                 `opi package add <source>`, then `opi package enable {name}`."
            ),
            Self::PackageUntrusted { name } => format!(
                "Adapter package {name:?} is not trusted (never confirmed or its \
                 manifest/lock/executable drifted). Review it with `opi package \
                 doctor`, then re-confirm trust with `opi package enable {name}`."
            ),
            Self::ContributionDisabled { name } => format!(
                "Adapter package {name:?} is trusted but not enabled. Run `opi \
                 package enable {name}`."
            ),
            Self::PolicyDenied { adapter_id } => format!(
                "Adapter {adapter_id:?} is denied by your execution permission \
                 policy. To allow it, set `[execution.permissions]` in your USER \
                 config (project permission sections are not honored)."
            ),
            Self::PermissionRequired { adapter_id, mode } => {
                if matches!(mode, ExecutionRunMode::Interactive) {
                    // Interactive mode does not need a "run interactively" nudge
                    // (the user already is) and no prompt can be relied upon to
                    // appear when this surfaces (startup build failure or the
                    // fail-closed no-broker path). The actionable path is
                    // persistent USER-config allowance.
                    format!(
                        "Adapter {adapter_id:?} requires interactive approval (policy \
                         `ask`). Allow it persistently in your USER config."
                    )
                } else {
                    // Headless modes cannot grant; the actionable paths are
                    // persistent USER-config allowance or an interactive run.
                    format!(
                        "Adapter {adapter_id:?} requires interactive approval (policy \
                         `ask`) and cannot be granted non-interactively in {mode} mode. \
                         Allow it persistently in your USER config, or run interactively."
                    )
                }
            }
            Self::PermissionDenied { adapter_id } => {
                format!("Adapter {adapter_id:?} was not approved for this invocation.")
            }
            Self::NoEligibleAdapter { strategy, mode } => format!(
                "No eligible command.execute adapter for strategy {strategy} in \
                 {mode} mode. Install, trust, and enable an adapter, or select a \
                 different backend."
            ),
            Self::AdapterNotSelected { strategy, .. } => format!(
                "Requested backend {REDACTED_BACKEND_PLACEHOLDER} is not selectable under strategy \
                 {strategy}. It must be installed, trusted, enabled, \
                 target-compatible, and not denied."
            ),
            Self::AdapterUnavailable { adapter_id, detail } => {
                let who = adapter_id
                    .as_deref()
                    .map(|id| format!("Adapter {id:?}"))
                    .unwrap_or_else(|| "An adapter".to_string());
                let cause = match detail {
                    UnavailableDetail::Store => "a package-store error",
                    UnavailableDetail::Ineligible => {
                        return format!(
                            "{who} is unavailable because it is not installed, trusted, enabled, or target-compatible. Install a package providing it, then review and enable that package with `opi package doctor` and `opi package enable <name>`."
                        );
                    }
                    UnavailableDetail::Collision => "an adapter-id collision",
                    UnavailableDetail::Handshake => "a pre-start handshake failure",
                };
                format!(
                    "{who} could not be activated ({cause}). Run `opi package \
                     doctor` and review the package store."
                )
            }
            Self::ProtocolIncompatible => "Adapter reported an incompatible protocol. \
                 Ensure the package targets the supported wire version."
                .to_string(),
            Self::ProtocolViolation => "Adapter violated the execution protocol. \
                 Report this to the package maintainer."
                .to_string(),
            Self::ExecutionFailed => "Command execution failed. See the run's \
                 redacted diagnostics for the phase."
                .to_string(),
            Self::ExecutionTimedOut => "Command execution exceeded the deadline and \
                 was cancelled."
                .to_string(),
            Self::CleanupUnconfirmed => "The adapter process tree could not confirm \
                 cleanup within the grace period."
                .to_string(),
        }
    }
}

/// Surface [`ActivationError`] (16.5's pre-spawn revalidation seam) as a stable
/// [`ExecutionFailure`]. Honors the pinned mapping documented on
/// [`ActivationError`]: `NotInstalled` -> `package_not_installed`, `Untrusted` ->
/// `package_untrusted`, `Disabled` -> `contribution_disabled`. The untrusted
/// `detail` and the opaque `Store` display are intentionally dropped (redaction:
/// they may carry absolute paths or store internals); `CollidingAdapterId` and
/// `Store` map to `adapter_unavailable`. This conversion is side-effect-free.
impl From<ActivationError> for ExecutionFailure {
    fn from(error: ActivationError) -> Self {
        match error {
            ActivationError::NotInstalled(name) => Self::PackageNotInstalled { name },
            ActivationError::Untrusted { name, .. } => Self::PackageUntrusted { name },
            ActivationError::Disabled(name) => Self::ContributionDisabled { name },
            ActivationError::CollidingAdapterId { adapter_id, .. } => Self::AdapterUnavailable {
                adapter_id: Some(adapter_id),
                detail: UnavailableDetail::Collision,
            },
            ActivationError::Store(_) => Self::AdapterUnavailable {
                adapter_id: None,
                detail: UnavailableDetail::Store,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_values_are_the_14_stable_literals() {
        assert_eq!(
            ExecutionFailure::PackageNotInstalled { name: "x".into() }.code(),
            "package_not_installed"
        );
        assert_eq!(
            ExecutionFailure::PackageUntrusted { name: "x".into() }.code(),
            "package_untrusted"
        );
        assert_eq!(
            ExecutionFailure::ContributionDisabled { name: "x".into() }.code(),
            "contribution_disabled"
        );
        assert_eq!(
            ExecutionFailure::PolicyDenied {
                adapter_id: "x".into()
            }
            .code(),
            "policy_denied"
        );
        assert_eq!(
            ExecutionFailure::PermissionRequired {
                adapter_id: "x".into(),
                mode: ExecutionRunMode::Rpc
            }
            .code(),
            "permission_required"
        );
        assert_eq!(
            ExecutionFailure::PermissionDenied {
                adapter_id: "x".into()
            }
            .code(),
            "permission_denied"
        );
        assert_eq!(
            ExecutionFailure::NoEligibleAdapter {
                strategy: ExecutionStrategy::Model,
                mode: ExecutionRunMode::Interactive
            }
            .code(),
            "no_eligible_adapter"
        );
        assert_eq!(
            ExecutionFailure::AdapterNotSelected {
                requested: "x".into(),
                strategy: ExecutionStrategy::Fixed
            }
            .code(),
            "adapter_not_selected"
        );
        assert_eq!(
            ExecutionFailure::AdapterUnavailable {
                adapter_id: None,
                detail: UnavailableDetail::Store
            }
            .code(),
            "adapter_unavailable"
        );
        assert_eq!(
            ExecutionFailure::ProtocolIncompatible.code(),
            "protocol_incompatible"
        );
        assert_eq!(
            ExecutionFailure::ProtocolViolation.code(),
            "protocol_violation"
        );
        assert_eq!(ExecutionFailure::ExecutionFailed.code(), "execution_failed");
        assert_eq!(
            ExecutionFailure::ExecutionTimedOut.code(),
            "execution_timed_out"
        );
        assert_eq!(
            ExecutionFailure::CleanupUnconfirmed.code(),
            "cleanup_unconfirmed"
        );
    }

    #[test]
    fn unselectable_model_backend_is_redacted_from_public_text() {
        let canary = r#"C:\private\HOSTILE sk-proj-012345678901234567890123456789"#;
        let failure = ExecutionFailure::AdapterNotSelected {
            requested: canary.to_string(),
            strategy: ExecutionStrategy::Model,
        };

        for surface in [failure.to_string(), failure.remediation()] {
            assert!(
                !surface.contains(canary),
                "raw model input leaked: {surface}"
            );
            assert!(
                surface.contains("<unavailable>"),
                "stable safe placeholder missing: {surface}"
            );
        }
    }
}

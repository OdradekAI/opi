//! Sandbox fallback diagnostic contract (Phase 15 T4).
//!
//! Stable `&'static str` codes identify the two sandbox fallback conditions so
//! embedders and tests can match them by literal. The shared
//! [`opi_agent::Diagnostic`] model is constructed by value here; the opi-agent
//! struct itself is unchanged. Both conditions carry a redacted
//! `{ layer, reason }` payload in `Diagnostic.details`: `layer` names the
//! sandbox layer (e.g. `"landlock"`, `"seccomp"`, `"windows-l3"`) and `reason`
//! is a short, curator-controlled explanation. No command, environment
//! variable, absolute path, or credential is ever placed in the payload.
//!
//! - [`CODE_SANDBOX_DEGRADED`]: a temporary/per-host layer degradation. The
//!   layer failed to engage on this host or attempt; the default sandbox policy
//!   is fail-open, so execution continues at the engaged baseline.
//! - [`CODE_SANDBOX_UNAVAILABLE`]: a permanent platform gap (e.g. Windows
//!   L1-L3, macOS L3). Reported once per startup rather than once per command.

use opi_agent::diagnostic::{Diagnostic, Severity};

/// Stable code for a temporary sandbox layer degradation (fail-open baseline).
pub const CODE_SANDBOX_DEGRADED: &str = "opi.sandbox.degraded";

/// Stable code for a permanent sandbox platform unavailability.
pub const CODE_SANDBOX_UNAVAILABLE: &str = "opi.sandbox.unavailable";

/// Owning subsystem for sandbox diagnostics.
pub const SOURCE_SANDBOX: &str = "sandbox";

/// Construct a redacted sandbox-layer-degraded diagnostic.
///
/// `layer` names the sandbox layer (e.g. `"landlock"`); `reason` is a short
/// curator-controlled explanation (e.g. `"kernel < 5.13"`). The payload is
/// restricted to `{ layer, reason }` and never carries command text, env
/// vars, paths, or secrets.
pub fn sandbox_degraded_diagnostic(layer: &'static str, reason: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_SANDBOX_DEGRADED,
        SOURCE_SANDBOX,
        "sandbox layer degraded",
    )
    .details(serde_json::json!({
        "layer": layer,
        "reason": reason.into(),
    }))
}

/// Construct a redacted sandbox-platform-unavailable diagnostic.
///
/// Semantics mirror [`sandbox_degraded_diagnostic`] but identify a permanent
/// platform gap rather than a temporary degradation; callers should emit it at
/// most once per startup.
pub fn sandbox_unavailable_diagnostic(
    layer: &'static str,
    reason: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_SANDBOX_UNAVAILABLE,
        SOURCE_SANDBOX,
        "sandbox layer permanently unavailable",
    )
    .details(serde_json::json!({
        "layer": layer,
        "reason": reason.into(),
    }))
}

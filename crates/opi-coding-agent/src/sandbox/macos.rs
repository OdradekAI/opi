//! Phase 15 task 15.5.4 — macOS `sandbox-exec` strict backend (substrate).
//!
//! This module owns the **host-independent** half of the macOS T4 sandbox: the
//! seatbelt deny-overlay *profile model*, the `sandbox-exec` wrapper *argv* model,
//! the per-layer *capability matrix*, and the exact missing/unusable-helper
//! reasons. It is pure Rust — no macOS kernel, no `unsafe`, no elevated
//! privileges — and compiles on every target so the profile/capability/argv
//! invariants are TDD'd by `macos_profile_and_capability_matrix` on any host.
//!
//! The macOS *runtime* — the `sandbox-exec` PATH probe, the parent-built
//! [`crate::sandbox::Confinement`] that launches bash under `sandbox-exec -p
//! <profile>`, the `MacosStrictBackend` selected by
//! [`crate::sandbox::production_sandbox_backend`], and the three native engaged
//! product assertions (outside-write deny, network deny, workspace+temp allow) —
//! is `cfg(target_os = "macos")` and is verified on a native macOS runner. It is
//! not compiled here: only `x86_64-pc-windows-msvc` is installable on this host,
//! and the macOS mechanism (`sandbox-exec` as the subprocess launcher) must
//! *prepend* to the spawn `Command`, which `std::process::Command` cannot do (no
//! program setter / arg insert; a `mem::replace` rebuild would drop the L0
//! `process_group(0)` + `kill_on_drop` state applied before
//! [`crate::sandbox::Confinement::apply`]). Resolving that exec integration point
//! needs macOS iteration, so the runtime + dispatcher wiring are deferred to the
//! macOS-runner follow-up and the acceptance scenario
//! `phase15-macos-strict-backend` stays open.
//!
//! # Profile shape (DoD: deterministic, escaped deny-overlay)
//!
//! The profile is a deny-only overlay on the seatbelt allow-all default (T4
//! design): when `fs` is engaged it denies every file-write under `/` and punches
//! workspace + temp exceptions back through; when `network` is engaged it denies
//! `network*`. Reads, process execution, and signals stay at the allow-all
//! default so required child behavior (exec the shell, read system libraries) is
//! preserved. In seatbelt the more-specific `subpath` allow wins over the
//! root deny, so the workspace/temp exceptions punch through regardless of rule
//! order; the deny-root is emitted first for readability.
//!
//! ```text
//! (version 1)
//! (deny file-write* (subpath "/"))             ; fs engaged only
//! (allow file-write* (subpath "<workspace>"))  ; fs engaged only
//! (allow file-write* (subpath "<temp>"))       ; fs engaged only
//! (deny network*)                               ; network engaged only
//! ```

/// Exact reason returned when `sandbox-exec` is not on `PATH`. Surfaced as the
/// `TemporarilyUnavailable` reason for the fs and network layers (the shared
/// 15.5.1 fail-open / fail-closed policy consumes it verbatim).
pub const SANDBOX_EXEC_MISSING_REASON: &str = "sandbox-exec not found on PATH";

/// Stable prefix for the reason returned when `sandbox-exec` is present but
/// failed the runtime probe. The probe detail is appended after this prefix.
pub const SANDBOX_EXEC_UNUSABLE_PREFIX: &str = "sandbox-exec unusable:";

/// Exact reason macOS L3 (syscall) confinement is permanently unavailable.
/// `sandbox-exec` exposes L1 (filesystem) and L2 (network) only; there is no
/// syscall-level surface, so this layer is a permanent platform gap (one-time
/// startup diagnostic, never per-command).
pub const MACOS_L3_UNAVAILABLE_REASON: &str =
    "macOS sandbox-exec provides L1/L2 confinement only; no syscall-level (L3) confinement";

/// Status of the `sandbox-exec` helper as discovered by the runtime probe.
///
/// Pure data carried by the (deferred) runtime; the capability model below is
/// computed from it so the matrix is testable without invoking `sandbox-exec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxExecStatus {
    /// `sandbox-exec` is on `PATH` and answered the probe; L1/L2 can engage.
    Available,
    /// `sandbox-exec` was not found on `PATH`.
    Missing,
    /// `sandbox-exec` is present but the probe rejected it (e.g. MDM-blocked,
    /// non-zero exit). Carries the probe detail for the reason string.
    Unusable(String),
}

impl SandboxExecStatus {
    /// Whether the helper is usable (L1/L2 can engage).
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// The exact reason the helper cannot engage, or `None` when available.
    /// Consumed verbatim by the shared fail-open / fail-closed policy.
    pub fn unavailability_reason(&self) -> Option<String> {
        match self {
            Self::Available => None,
            Self::Missing => Some(SANDBOX_EXEC_MISSING_REASON.to_string()),
            Self::Unusable(detail) => Some(format!("{SANDBOX_EXEC_UNUSABLE_PREFIX} {detail}")),
        }
    }
}

/// Per-layer macOS strict capability, computed from the `sandbox-exec` status.
/// Pure: each field is a [`crate::sandbox::LayerAvailability`] the deferred
/// runtime hands to [`crate::sandbox::prepare`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosStrictCapability {
    /// L1 filesystem writes via the seatbelt deny-overlay.
    pub fs: super::LayerAvailability,
    /// L2 network via `(deny network*)`.
    pub network: super::LayerAvailability,
    /// L3 syscalls: always permanently unavailable on macOS.
    pub syscalls: super::LayerAvailability,
}

/// Compute the macOS per-layer capability from the `sandbox-exec` status.
///
/// - `syscalls` is **always** permanently unavailable (sandbox-exec is L1/L2
///   only) — independent of the helper.
/// - `fs` and `network` engage iff the helper is [`SandboxExecStatus::Available`];
///   otherwise they are temporarily unavailable with the helper's exact reason
///   (missing vs. unusable).
pub fn macos_strict_capability(sandbox_exec: &SandboxExecStatus) -> MacosStrictCapability {
    let (fs, network) = match sandbox_exec {
        SandboxExecStatus::Available => (
            super::LayerAvailability::Engaged,
            super::LayerAvailability::Engaged,
        ),
        SandboxExecStatus::Missing => {
            let reason = SANDBOX_EXEC_MISSING_REASON.to_string();
            (
                super::LayerAvailability::TemporarilyUnavailable {
                    reason: reason.clone(),
                },
                super::LayerAvailability::TemporarilyUnavailable { reason },
            )
        }
        SandboxExecStatus::Unusable(detail) => {
            let reason = format!("{SANDBOX_EXEC_UNUSABLE_PREFIX} {detail}");
            (
                super::LayerAvailability::TemporarilyUnavailable {
                    reason: reason.clone(),
                },
                super::LayerAvailability::TemporarilyUnavailable { reason },
            )
        }
    };
    MacosStrictCapability {
        fs,
        network,
        syscalls: super::LayerAvailability::PermanentlyUnavailable {
            reason: MACOS_L3_UNAVAILABLE_REASON.to_string(),
        },
    }
}

/// Escape a path for embedding in a seatbelt `(subpath "...")` string literal.
///
/// Backslash, double-quote, and dollar are backslash-escaped. The dollar escape
/// is load-bearing: seatbelt expands `${var}` inside profile strings, so an
/// unescaped `$` in a workspace path would let a crafted path inject or expand a
/// variable. Every special char is prefixed with `\`; the raw path therefore
/// never appears verbatim in the rendered profile.
fn escape_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for ch in path.chars() {
        match ch {
            '\\' | '"' | '$' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Render the macOS seatbelt deny-overlay profile string.
///
/// Deterministic and escaped (DoD). `fs_enabled` emits the file-write deny
/// overlay on `/` with workspace + temp exceptions; `network_enabled` emits
/// `(deny network*)`. With both disabled the profile is the no-op version header
/// (the runtime would not engage `sandbox-exec` for it). Pure: produces a string,
/// never invokes `sandbox-exec`.
pub fn render_profile(
    workspace: &str,
    temp_dir: &str,
    fs_enabled: bool,
    network_enabled: bool,
) -> String {
    let mut out = String::from("(version 1)\n");
    if fs_enabled {
        out.push_str("(deny file-write* (subpath \"/\"))\n");
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape_path(workspace)
        ));
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape_path(temp_dir)
        ));
    }
    if network_enabled {
        out.push_str("(deny network*)\n");
    }
    out
}

/// Build the `sandbox-exec` wrapper argv (DoD: argv preservation).
///
/// Returns `["sandbox-exec", "-p", <profile>, <program>, <args...>]`: the wrapper
/// prepends `sandbox-exec -p <profile>` and preserves the original program and
/// args verbatim in the tail. Pure: produces the argv vector, never spawns. The
/// deferred runtime uses this to launch bash under the rendered profile.
pub fn build_wrapped_argv(profile: &str, program: &str, args: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(3 + 1 + args.len());
    argv.push("sandbox-exec".to_string());
    argv.push("-p".to_string());
    argv.push(profile.to_string());
    argv.push(program.to_string());
    argv.extend(args.iter().cloned());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Escaping is the security-critical invariant: every special char is
    /// backslash-doubled, so a crafted path cannot inject seatbelt syntax or
    /// expand a `${var}` (the leading `\` neutralizes the `$` even though the
    /// literal `${` substring is still present). Asserted directly here; the
    /// integration test checks the composed profile.
    #[test]
    fn escape_path_neutralizes_seatbelt_metacharacters() {
        assert_eq!(escape_path("/clean/path"), "/clean/path");
        // quote, dollar, backslash each gain a leading backslash.
        assert_eq!(escape_path("a\"b"), "a\\\"b");
        assert_eq!(escape_path("a$b"), "a\\$b");
        assert_eq!(escape_path("a\\b"), "a\\\\b");
        // ${var}: the dollar is backslash-escaped, so seatbelt treats it as a
        // literal `$` and does not expand the variable.
        assert_eq!(escape_path("${HOME}"), "\\${HOME}");
    }
}

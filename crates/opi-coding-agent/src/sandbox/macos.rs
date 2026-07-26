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
//! preserved. Seatbelt is first-match-wins, so the workspace/temp allow
//! exceptions MUST precede the root write deny (the runtime emits them in that
//! order); a root deny emitted first would shadow the exceptions and reject
//! workspace writes too. `(deny network*)` blocks bind/connect/inbound/outbound
//! but not `socket()` creation itself, so the engaged network assertion
//! exercises `bind`.
//!
//! ```text
//! (version 1)
//! (allow default)                              ; seatbelt default is DENY; allow-all base
//! (allow file-write* (subpath "<workspace>"))  ; fs engaged only (exceptions first)
//! (allow file-write* (subpath "<temp>"))       ; fs engaged only
//! (deny file-write* (subpath "/"))             ; fs engaged only (then deny the rest)
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
/// Deterministic and escaped (DoD). The profile is a deny-overlay on a seatbelt
/// allow-default base: `(allow default)` is the fallback for every operation no
/// explicit rule matches, so the confined child can still exec its shell and
/// read system libraries. `fs_enabled` denies every file-write under `/` with
/// workspace + temp exceptions; `network_enabled` denies `network*` (bind /
/// connect / inbound / outbound — note `socket()` creation itself is NOT a
/// `network*` operation, so the engaged network test exercises `bind`).
///
/// **Seatbelt is first-match-wins:** the first explicit rule that matches an
/// operation decides it. The workspace/temp allow exceptions therefore MUST
/// precede the root write deny, or the deny would shadow them and reject
/// workspace writes too. Order: `(allow default)` base, then workspace/temp
/// write exceptions, then the root write deny, then the network deny. With both
/// disabled the profile is just the version + allow-default header (the runtime
/// would not engage `sandbox-exec` for it). Pure: produces a string, never
/// invokes `sandbox-exec`.
pub fn render_profile(
    workspace: &str,
    temp_dir: &str,
    fs_enabled: bool,
    network_enabled: bool,
) -> String {
    let mut out = String::from("(version 1)\n");
    // seatbelt's default decision is DENY; an explicit allow-default base is
    // load-bearing — without it the confined bash cannot exec or read system
    // files, and even the sandbox-exec probe's own helper exec is rejected.
    out.push_str("(allow default)\n");
    if fs_enabled {
        // First-match-wins: emit the workspace/temp write exceptions BEFORE the
        // root deny so they take precedence; the root deny then rejects every
        // other write under /.
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape_path(workspace)
        ));
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape_path(temp_dir)
        ));
        out.push_str("(deny file-write* (subpath \"/\"))\n");
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

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.4 — production runtime (cfg(target_os = "macos")).
//
// Probes `sandbox-exec` on PATH, reports per-layer capability via the substrate
// [`macos_strict_capability`] matrix, and builds a [`super::Confinement::launcher`]
// plan (`sandbox-exec -p <profile>`) when every requested layer engages. The
// launcher — not a `pre_exec` hook — is the macOS mechanism: `sandbox-exec` IS
// the helper, so it must prepend itself to the spawn argv, which the spawn site
// (`crate::tool::operations::exec`) does via [`super::Confinement::launcher_prefix`].
// Compiled only on macOS; the engaged product assertions run on a native runner.
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::sync::Arc;

/// Resolve `program` on `PATH`. Equivalent to a `which` lookup; returns the
/// first matching regular file.
#[cfg(target_os = "macos")]
fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Probe `sandbox-exec`: PATH lookup, then launch `/usr/bin/true` under a no-op
/// profile. A usable helper exits 0; an MDM-blocked or broken install exits
/// non-zero or fails to spawn. The probe detail is capped to keep the reason
/// short for the shared fail-open / fail-closed diagnostic.
#[cfg(target_os = "macos")]
fn probe_sandbox_exec() -> SandboxExecStatus {
    let bin = match find_on_path("sandbox-exec") {
        Some(p) => p,
        None => return SandboxExecStatus::Missing,
    };
    let profile = "(version 1)\n(allow default)\n";
    match std::process::Command::new(&bin)
        .arg("-p")
        .arg(profile)
        .arg("/usr/bin/true")
        .output()
    {
        Ok(o) if o.status.success() => SandboxExecStatus::Available,
        Ok(o) => {
            let code = o
                .status
                .code()
                .map(|c| format!("exit {c}"))
                .unwrap_or_else(|| "signaled".to_string());
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let suffix = if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            };
            let detail: String = format!("probe {code}{suffix}").chars().take(160).collect();
            SandboxExecStatus::Unusable(detail)
        }
        Err(e) => SandboxExecStatus::Unusable(format!("spawn failed: {e}")),
    }
}

/// Build the macOS confinement plan: a `sandbox-exec` launcher carrying the
/// rendered deny-overlay profile (fs + network engaged; workspace + temp
/// write-through exceptions). Returns `None` unless the helper probed usable.
#[cfg(target_os = "macos")]
pub fn build_macos_confinement(
    workspace: &Path,
    status: &SandboxExecStatus,
) -> Option<super::Confinement> {
    if !status.is_available() {
        return None;
    }
    let ws = workspace.to_string_lossy().into_owned();
    let tmp = std::env::temp_dir().to_string_lossy().into_owned();
    let profile = render_profile(&ws, &tmp, true, true);
    Some(super::Confinement::launcher(
        "sandbox-exec",
        vec!["-p".to_string(), profile],
    ))
}

/// Production macOS strict backend. Probes `sandbox-exec` at construction and
/// reports per-layer availability via the substrate capability matrix: L3/
/// syscalls permanently unavailable; L1 fs + L2 network engaged iff the helper
/// probed usable. Builds a [`super::Confinement::launcher`] plan when every
/// requested layer engages.
#[cfg(target_os = "macos")]
pub struct MacosStrictBackend {
    status: SandboxExecStatus,
    workspace: Arc<Path>,
}

#[cfg(target_os = "macos")]
impl MacosStrictBackend {
    /// Production constructor: probe `sandbox-exec` on PATH.
    pub fn new(workspace: Arc<Path>) -> Self {
        Self {
            status: probe_sandbox_exec(),
            workspace,
        }
    }

    /// Inject the probe status instead of probing (capability-matrix coverage of
    /// the available / missing / unusable branches without a real helper).
    pub fn with_status(workspace: Arc<Path>, status: SandboxExecStatus) -> Self {
        Self { status, workspace }
    }

    /// The probed `sandbox-exec` status.
    pub fn status(&self) -> &SandboxExecStatus {
        &self.status
    }
}

#[cfg(target_os = "macos")]
impl super::StrictBackend for MacosStrictBackend {
    fn availability(&self, layer: super::SandboxLayer) -> super::LayerAvailability {
        let cap = macos_strict_capability(&self.status);
        match layer {
            super::SandboxLayer::Fs => cap.fs,
            super::SandboxLayer::Network => cap.network,
            super::SandboxLayer::Syscalls => cap.syscalls,
        }
    }

    fn build_confinement(&self, _workspace: &Path) -> Option<super::Confinement> {
        build_macos_confinement(&self.workspace, &self.status)
    }
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

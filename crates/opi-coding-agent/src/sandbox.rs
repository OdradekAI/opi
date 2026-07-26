//! Strict-sandbox policy resolver and production dispatch (Phase 15 task 15.5.1).
//!
//! This module owns the **cross-platform** half of the T4 sandbox: it turns a
//! resolved [`crate::config::SandboxConfig`] plus a capability-injected platform
//! backend into a [`PreparedSandbox`] decision. It does NOT implement any OS
//! confinement: the per-platform L1/L2/L3 backends (Landlock+seccomp on Linux,
//! `sandbox-exec` on macOS, L0-only on Windows) plug in by implementing
//! [`StrictBackend`]. Task 15.5.5 has landed the Windows L0-only backend in
//! `sandbox/windows.rs` (a permanent platform gap); Linux and macOS remain
//! not-yet-wired stubs until 15.5.3 / 15.5.4, so on those targets the production
//! backend selected by [`prepare_production`] truthfully reports every strict
//! layer as temporarily unavailable and `strict` mode flows through the shared
//! fail-open / fail-closed policy here.
//!
//! The resolver is pure and host-independent: every policy branch is covered by
//! inline tests that inject a fake [`StrictBackend`], so verification needs no
//! host kernel feature (DoD).
//!
//! # Layer availability model
//!
//! - [`LayerAvailability::Engaged`] — the backend confined the layer; no
//!   diagnostic.
//! - [`LayerAvailability::TemporarilyUnavailable`] — the layer could engage on a
//!   different host/kernel (or, for 15.5.1 on Linux/macOS, is not wired yet in
//!   this build). Fail-open emits one `CODE_SANDBOX_DEGRADED` diagnostic per
//!   command; fail-closed aborts the turn.
//! - [`LayerAvailability::PermanentlyUnavailable`] — the platform will never
//!   provide the layer (Windows L1-L3). Reported ONCE per startup via
//!   `CODE_SANDBOX_UNAVAILABLE`, never per command.
//!
//! # `forbid(unsafe_code)`
//!
//! Confinement FFI (process groups, Job Objects, seccomp, Landlock) lives in
//! `tool/process_tree.rs` and the per-platform backend tasks. This policy module
//! and `tool/operations.rs` stay `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use opi_agent::diagnostic::Diagnostic;

use crate::config::SandboxConfig;
use crate::diagnostics::sandbox_unavailable_diagnostic;

/// Windows strict backend (L0-only); landed in task 15.5.5.
#[cfg(target_os = "windows")]
mod windows;

/// Linux strict backend (seccomp deny-overlay + Landlock); landed in task 15.5.3.
#[cfg(target_os = "linux")]
pub mod linux;

/// One strict-sandbox layer. The names match the `[sandbox]` TOML toggles
/// (`fs`/`network`/`syscalls`) so diagnostics carry the same identifier a user
/// configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxLayer {
    Fs,
    Network,
    Syscalls,
}

impl SandboxLayer {
    /// Stable identifier used in diagnostic `{ layer, reason }` payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            SandboxLayer::Fs => "fs",
            SandboxLayer::Network => "network",
            SandboxLayer::Syscalls => "syscalls",
        }
    }
}

/// Per-layer availability as reported by a platform backend. See the module docs
/// for the diagnostic/de-fail policy each variant implies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerAvailability {
    /// The backend confined this layer.
    Engaged,
    /// The layer cannot engage on this host right now but is not a permanent
    /// platform gap (old kernel, missing tool, or not-yet-wired backend).
    TemporarilyUnavailable { reason: String },
    /// The platform will never provide this layer.
    PermanentlyUnavailable { reason: String },
}

/// A parent-built, child-applied confinement plan: a closure that registers the
/// platform's `pre_exec` hook(s) on a `tokio::process::Command`. The cross-platform
/// resolver carries an `Option<Confinement>` on an `Engaged` strict decision;
/// `LocalBashOperations::exec` applies it to the spawn `Command` between the L0
/// tree setup and `spawn()`. `Confinement` is `Clone` (cheap — the closure is
/// shared behind an `Arc`) so a resolved `PreparedSandbox` can be reused across
/// commands; each `apply` rebuilds any per-fork state (the Linux backend rebuilds
/// its Landlock ruleset per spawn, since `restrict_self` consumes it).
#[derive(Clone)]
pub struct Confinement(Arc<dyn Fn(&mut tokio::process::Command) + Send + Sync>);

impl std::fmt::Debug for Confinement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Confinement")
            .field("hook", &"<closure>")
            .finish()
    }
}

impl Confinement {
    /// Wrap a confinement-installing closure.
    pub fn new<F>(hook: F) -> Self
    where
        F: Fn(&mut tokio::process::Command) + Send + Sync + 'static,
    {
        Self(Arc::new(hook))
    }

    /// Apply the confinement hook to `cmd` (register the `pre_exec` child setup).
    pub fn apply(&self, cmd: &mut tokio::process::Command) {
        (self.0)(cmd);
    }
}

/// Capability-injected platform backend.
///
/// Production backends implement this to report what their platform can engage
/// and, when it can, build the parent-side [`Confinement`] plan that
/// [`prepare_production`] attaches to an `Engaged` decision. Tests inject a fake
/// implementation to drive every policy branch without a host kernel feature.
pub trait StrictBackend: Send + Sync {
    /// Report the availability of `layer` on this platform/backend.
    fn availability(&self, layer: SandboxLayer) -> LayerAvailability;

    /// Build the confinement plan (parent side) the backend can engage. Returns
    /// `None` for backends that report engagement but apply no Opi-side `pre_exec`
    /// hook (e.g. the L0-only Windows backend), and for capability-injected fakes.
    /// The default is `None`; the Linux backend (15.5.3) overrides it.
    fn build_confinement(&self, _workspace: &std::path::Path) -> Option<Confinement> {
        None
    }
}

/// A temporary gap carried alongside a fail-open decision so
/// [`crate::tool::LocalBashOperations`] can emit one
/// `CODE_SANDBOX_DEGRADED` diagnostic per command (the permanent gaps are
/// already emitted once at startup and are NOT repeated here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporaryGap {
    pub layer: SandboxLayer,
    pub reason: String,
}

/// The per-exec decision for a strict request. [`PreparedSandbox`] carries this
/// plus the once-per-startup permanent diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrictOutcome {
    /// Every requested layer engaged. (In 15.5.1 this is reachable only via an
    /// injected fake backend reporting engagement — no production backend is
    /// wired yet. It carries no confinement action until 15.5.3/15.5.4 attach a
    /// real backend, so exec still runs the L0 baseline.)
    Engaged,
    /// `require = false` and at least one requested layer was unavailable:
    /// proceed at the L0 baseline. `per_command_temporary` are the TEMPORARY
    /// gaps emitted as degraded diagnostics each command; permanent gaps were
    /// already reported once at startup.
    FailOpen {
        per_command_temporary: Vec<TemporaryGap>,
    },
    /// `require = true` and at least one requested layer was unavailable:
    /// [`crate::tool::LocalBashOperations`] must return a
    /// named error before any command side effect.
    FailClosed { reason: String },
}

/// A fully resolved strict-mode decision plus its once-per-startup permanent-gap
/// diagnostics. Not `Eq`: [`Diagnostic`] carries a `serde_json::Value` payload,
/// and [`PartialEq`] deliberately ignores [`Self::confinement`] (a closure has no
/// identity comparison).
#[derive(Debug, Clone)]
pub struct StrictDecision {
    pub outcome: StrictOutcome,
    permanent_diagnostics: Vec<Diagnostic>,
    /// Parent-built confinement plan attached by [`prepare_production`] when every
    /// requested layer engaged. `None` for `Off`, fail-open, fail-closed, and the
    /// pure [`prepare`] path (capability-injected fakes build no confinement).
    pub confinement: Option<Confinement>,
}

impl PartialEq for StrictDecision {
    fn eq(&self, other: &Self) -> bool {
        // Compare only the observable policy decision. `confinement` holds an
        // opaque closure and is intentionally excluded from equality.
        self.outcome == other.outcome && self.permanent_diagnostics == other.permanent_diagnostics
    }
}

/// The result of [`prepare`] / [`prepare_production`]. `Off` runs the always-on
/// L0 baseline only (no diagnostics); `Strict` carries the decision.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PreparedSandbox {
    /// Default: sandbox off, L0 baseline only.
    #[default]
    Off,
    Strict(StrictDecision),
}

impl PreparedSandbox {
    /// Permanent platform-gap diagnostics to emit ONCE at startup. Empty for
    /// `Off`, `Engaged`, and the temporary-only case; non-empty for any
    /// permanent gap (whether the outcome is fail-open or fail-closed).
    pub fn startup_diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            PreparedSandbox::Off => Vec::new(),
            PreparedSandbox::Strict(decision) => decision.permanent_diagnostics.clone(),
        }
    }
}

/// Resolve the sandbox policy for `config` against an injected `backend`.
///
/// Pure and host-independent: the caller selects the backend (production uses
/// [`prepare_production`], which selects by cfg; tests pass a fake). A layer is
/// "requested" unless its toggle is explicitly `Some(false)`; `None` means
/// "engage at the platform default".
pub fn prepare(config: &SandboxConfig, backend: &dyn StrictBackend) -> PreparedSandbox {
    if config.mode == crate::config::SandboxMode::Off {
        return PreparedSandbox::Off;
    }

    let requested_layers = [
        (SandboxLayer::Fs, config.fs),
        (SandboxLayer::Network, config.network),
        (SandboxLayer::Syscalls, config.syscalls),
    ];

    let mut permanent: Vec<(SandboxLayer, String)> = Vec::new();
    let mut temporary: Vec<(SandboxLayer, String)> = Vec::new();
    for (layer, toggle) in requested_layers {
        // Some(false) = explicit opt-out: do not query, do not diagnose.
        if toggle == Some(false) {
            continue;
        }
        match backend.availability(layer) {
            LayerAvailability::Engaged => {}
            LayerAvailability::TemporarilyUnavailable { reason } => {
                temporary.push((layer, reason));
            }
            LayerAvailability::PermanentlyUnavailable { reason } => {
                permanent.push((layer, reason));
            }
        }
    }

    let permanent_diagnostics = permanent
        .iter()
        .map(|(layer, reason)| sandbox_unavailable_diagnostic(layer.as_str(), reason.clone()))
        .collect::<Vec<_>>();

    let outcome = if temporary.is_empty() && permanent.is_empty() {
        StrictOutcome::Engaged
    } else if config.require {
        StrictOutcome::FailClosed {
            reason: summarize_gaps(&permanent, &temporary),
        }
    } else {
        StrictOutcome::FailOpen {
            per_command_temporary: temporary
                .iter()
                .map(|(layer, reason)| TemporaryGap {
                    layer: *layer,
                    reason: reason.clone(),
                })
                .collect(),
        }
    };

    PreparedSandbox::Strict(StrictDecision {
        outcome,
        permanent_diagnostics,
        confinement: None,
    })
}

/// Build a short, redacted reason summarizing which layers were unavailable, for
/// the fail-closed error message. Layer names only — no command/env/paths.
fn summarize_gaps(
    permanent: &[(SandboxLayer, String)],
    temporary: &[(SandboxLayer, String)],
) -> String {
    let mut names: Vec<&str> = permanent
        .iter()
        .chain(temporary.iter())
        .map(|(layer, _)| layer.as_str())
        .collect();
    names.sort_unstable();
    names.dedup();
    format!(
        "strict sandbox unavailable for layer(s): {}",
        names.join(", ")
    )
}

/// Resolve the sandbox policy against the cfg-selected production backend.
///
/// This is the production entry point used by
/// [`crate::harness::CodingHarness::build_tools`]. It selects the platform
/// backend via [`production_sandbox_backend`] and dispatches through [`prepare`].
/// `workspace` is the harness workspace root; the Linux L1 fs layer grants
/// workspace+temp writes against it (Windows/macOS backends ignore it).
pub fn prepare_production(config: &SandboxConfig, workspace: &std::path::Path) -> PreparedSandbox {
    #[cfg(target_os = "windows")]
    {
        let _ = workspace;
        crate::sandbox::windows::prepare(config)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let backend = production_sandbox_backend(workspace);
        let mut prepared = prepare(config, backend.as_ref());
        // Attach the parent-built confinement plan only when every requested
        // layer engaged; `LocalBashOperations::exec` applies it to the spawn
        // Command. The Linux backend (15.5.3) builds the seccomp+Landlock plan;
        // the macOS not-yet-wired stub reports temporary gaps, never `Engaged`,
        // so it builds no confinement.
        if let PreparedSandbox::Strict(decision) = &mut prepared
            && matches!(decision.outcome, StrictOutcome::Engaged)
        {
            decision.confinement = backend.build_confinement(workspace);
        }
        prepared
    }
}

/// Select the production strict backend for the current platform.
///
/// - **Linux (15.5.3)**: `LinuxStrictBackend` queries the observed Landlock ABI
///   and reports per-layer engagement; it builds the seccomp+Landlock confinement
///   plan from `workspace` when every requested layer is available.
/// - **macOS (15.5.4)**: not yet wired — `NotYetWiredBackend` reports every strict
///   layer as temporarily unavailable, so strict fails open honestly.
/// - **Windows (15.5.5)**: `L0OnlyBackend` — every strict layer is a permanent
///   platform gap.
/// - Any other target is permanently unsupported.
pub fn production_sandbox_backend(workspace: &std::path::Path) -> Box<dyn StrictBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(crate::sandbox::linux::LinuxStrictBackend::new(Arc::from(
            workspace,
        )))
    }
    #[cfg(target_os = "macos")]
    {
        let _ = workspace;
        Box::new(NotYetWiredBackend {
            platform: "macos",
            phase: "15.5.4",
        })
    }
    #[cfg(target_os = "windows")]
    {
        let _ = workspace;
        Box::new(crate::sandbox::windows::L0OnlyBackend)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = workspace;
        Box::new(UnsupportedPlatformBackend)
    }
}

/// Linux/macOS production backend for 15.5.1: strict layers exist on the platform
/// but are not wired into this build yet. Reported as TEMPORARY so fail-open
/// emits a per-command degraded diagnostic (honest: strict did not engage) until
/// 15.5.3 / 15.5.4 replace this stub with a real engaged/temporary/permanent
/// backend. Defined only where it is selected (avoids dead code on other
/// targets).
#[cfg(target_os = "macos")]
struct NotYetWiredBackend {
    platform: &'static str,
    phase: &'static str,
}

#[cfg(target_os = "macos")]
impl StrictBackend for NotYetWiredBackend {
    fn availability(&self, _layer: SandboxLayer) -> LayerAvailability {
        LayerAvailability::TemporarilyUnavailable {
            reason: format!(
                "{platform} strict backend not yet implemented (phase {phase})",
                platform = self.platform,
                phase = self.phase
            ),
        }
    }
}

/// Fallback for targets outside the Linux/macOS/Windows release matrix. Defined
/// only where selected.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
struct UnsupportedPlatformBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl StrictBackend for UnsupportedPlatformBackend {
    fn availability(&self, _layer: SandboxLayer) -> LayerAvailability {
        LayerAvailability::PermanentlyUnavailable {
            reason: "strict sandbox unsupported on this platform".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SandboxConfig, SandboxMode};

    /// Fake backend backed by a closure, for policy tests.
    struct FakeBackend<F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync>(F);

    impl<F> StrictBackend for FakeBackend<F>
    where
        F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync,
    {
        fn availability(&self, layer: SandboxLayer) -> LayerAvailability {
            (self.0)(layer)
        }
    }

    fn fake<F>(f: F) -> Box<FakeBackend<F>>
    where
        F: Fn(SandboxLayer) -> LayerAvailability + Send + Sync,
    {
        Box::new(FakeBackend(f))
    }

    fn strict_config(require: bool) -> SandboxConfig {
        SandboxConfig {
            mode: SandboxMode::Strict,
            require,
            fs: None,
            network: None,
            syscalls: None,
        }
    }

    fn off_config() -> SandboxConfig {
        SandboxConfig::default()
    }

    #[test]
    fn off_mode_is_off_with_no_diagnostics() {
        let backend = fake(|_| LayerAvailability::Engaged);
        let prepared = prepare(&off_config(), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        assert_eq!(prepared, PreparedSandbox::Off);
        assert!(startup.is_empty());
    }

    #[test]
    fn strict_all_engaged_is_engaged_with_no_diagnostics() {
        let backend = fake(|_| LayerAvailability::Engaged);
        let prepared = prepare(&strict_config(false), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert_eq!(decision.outcome, StrictOutcome::Engaged);
                assert!(decision.permanent_diagnostics.is_empty());
            }
            _ => panic!("expected Strict"),
        }
        assert!(startup.is_empty());
    }

    #[test]
    fn strict_temporary_gap_fail_open_emits_per_command_not_startup() {
        let backend = fake(|layer| match layer {
            SandboxLayer::Network => LayerAvailability::TemporarilyUnavailable {
                reason: "kernel < 5.13".to_string(),
            },
            _ => LayerAvailability::Engaged,
        });
        let prepared = prepare(&strict_config(false), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                match decision.outcome {
                    StrictOutcome::FailOpen {
                        per_command_temporary,
                    } => {
                        assert_eq!(per_command_temporary.len(), 1);
                        assert_eq!(per_command_temporary[0].layer, SandboxLayer::Network);
                        assert!(per_command_temporary[0].reason.contains("kernel < 5.13"));
                    }
                    other => panic!("expected FailOpen, got {other:?}"),
                }
                // Temporary gaps are NOT startup diagnostics.
                assert!(decision.permanent_diagnostics.is_empty());
            }
            _ => panic!("expected Strict"),
        }
        assert!(startup.is_empty());
    }

    #[test]
    fn strict_permanent_gap_fail_open_emits_one_startup_diagnostic() {
        let backend = fake(|_| LayerAvailability::PermanentlyUnavailable {
            reason: "windows L0-only".to_string(),
        });
        let prepared = prepare(&strict_config(false), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                // All three layers are permanent -> three startup diagnostics, but
                // ZERO per-command (de-duped to startup).
                assert_eq!(decision.permanent_diagnostics.len(), 3);
                assert!(
                    decision
                        .permanent_diagnostics
                        .iter()
                        .all(|d| d.code == crate::diagnostics::CODE_SANDBOX_UNAVAILABLE)
                );
                match &decision.outcome {
                    StrictOutcome::FailOpen {
                        per_command_temporary,
                    } => {
                        assert!(
                            per_command_temporary.is_empty(),
                            "permanent gaps must not also be per-command"
                        );
                    }
                    other => panic!("expected FailOpen, got {other:?}"),
                }
            }
            _ => panic!("expected Strict"),
        }
        assert_eq!(startup.len(), 3);
    }

    #[test]
    fn strict_permanent_gap_require_true_is_fail_closed() {
        let backend = fake(|_| LayerAvailability::PermanentlyUnavailable {
            reason: "windows L0-only".to_string(),
        });
        let prepared = prepare(&strict_config(true), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => match decision.outcome {
                StrictOutcome::FailClosed { reason } => {
                    assert!(reason.contains("fs"));
                    assert!(reason.contains("network"));
                    assert!(reason.contains("syscalls"));
                }
                other => panic!("expected FailClosed, got {other:?}"),
            },
            _ => panic!("expected Strict"),
        }
        // Fail-closed still surfaces the permanent gaps as startup diagnostics.
        assert_eq!(startup.len(), 3);
    }

    #[test]
    fn strict_temporary_gap_require_true_is_fail_closed() {
        let backend = fake(|_| LayerAvailability::TemporarilyUnavailable {
            reason: "not wired".to_string(),
        });
        let prepared = prepare(&strict_config(true), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert!(matches!(decision.outcome, StrictOutcome::FailClosed { .. }));
                // Temporary-only -> no permanent startup diagnostics.
                assert!(decision.permanent_diagnostics.is_empty());
            }
            _ => panic!("expected Strict"),
        }
        assert!(startup.is_empty());
    }

    #[test]
    fn strict_mixed_gaps_fail_open_carries_temporary_and_startup_permanent() {
        let backend = fake(|layer| match layer {
            SandboxLayer::Fs => LayerAvailability::PermanentlyUnavailable {
                reason: "windows L0-only".to_string(),
            },
            SandboxLayer::Network => LayerAvailability::TemporarilyUnavailable {
                reason: "kernel too old".to_string(),
            },
            SandboxLayer::Syscalls => LayerAvailability::Engaged,
        });
        let prepared = prepare(&strict_config(false), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                if let StrictOutcome::FailOpen {
                    per_command_temporary,
                } = decision.outcome
                {
                    assert_eq!(per_command_temporary.len(), 1);
                    assert_eq!(per_command_temporary[0].layer, SandboxLayer::Network);
                } else {
                    panic!("expected FailOpen");
                }
                // Only the Fs permanent gap -> one startup diagnostic.
                assert_eq!(decision.permanent_diagnostics.len(), 1);
            }
            _ => panic!("expected Strict"),
        }
        assert_eq!(startup.len(), 1);
    }

    #[test]
    fn explicit_opt_out_layer_is_not_queried() {
        let mut config = strict_config(false);
        config.network = Some(false); // explicit opt-out
        let backend = fake(|layer| {
            if matches!(layer, SandboxLayer::Network) {
                panic!("opted-out layer must not be queried");
            }
            LayerAvailability::Engaged
        });
        let prepared = prepare(&config, backend.as_ref());
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert_eq!(decision.outcome, StrictOutcome::Engaged);
            }
            _ => panic!("expected Strict"),
        }
    }

    #[test]
    fn explicit_true_request_treats_layer_as_requested() {
        let mut config = strict_config(false);
        config.fs = Some(true);
        let backend = fake(|_| LayerAvailability::TemporarilyUnavailable {
            reason: "x".to_string(),
        });
        let prepared = prepare(&config, backend.as_ref());
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert!(matches!(decision.outcome, StrictOutcome::FailOpen { .. }));
            }
            _ => panic!("expected Strict"),
        }
    }

    #[test]
    fn production_backend_classifies_current_platform_truthfully() {
        // 15.5.3 wires the Linux backend (seccomp + Landlock), which MAY report
        // Engaged on a capable kernel (observed Landlock ABI >= 4); Windows
        // (L0-only) and macOS (not-yet-wired) must not claim engagement.
        let backend = production_sandbox_backend(std::path::Path::new("."));
        for layer in [
            SandboxLayer::Fs,
            SandboxLayer::Network,
            SandboxLayer::Syscalls,
        ] {
            let avail = backend.availability(layer);
            #[cfg(not(target_os = "linux"))]
            assert!(
                !matches!(avail, LayerAvailability::Engaged),
                "non-Linux platforms must not claim engagement: {avail:?}"
            );
            let _ = avail;
        }
        #[cfg(target_os = "linux")]
        {
            // The seccomp L3 danger blocklist is ABI-independent; it always engages.
            let syscalls = backend.availability(SandboxLayer::Syscalls);
            assert!(
                matches!(syscalls, LayerAvailability::Engaged),
                "Linux syscalls layer must engage (seccomp is ABI-independent): {syscalls:?}"
            );
        }
    }

    #[test]
    fn production_prepare_off_has_no_diagnostics() {
        let prepared = prepare_production(&off_config(), std::path::Path::new("."));
        assert_eq!(prepared, PreparedSandbox::Off);
        assert!(prepared.startup_diagnostics().is_empty());
    }

    #[test]
    fn production_prepare_strict_truthful_on_current_platform() {
        // 15.5.3: a capable Linux kernel CAN engage strict (it is no longer forced
        // to fail-open as in 15.5.1). require=false must therefore either Engage
        // (capable Linux) or FailOpen (old kernel / Windows L0-only / macOS stub),
        // and an engaged Linux decision must carry a real confinement plan.
        let prepared = prepare_production(&strict_config(false), std::path::Path::new("."));
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert!(
                    matches!(
                        decision.outcome,
                        StrictOutcome::Engaged | StrictOutcome::FailOpen { .. }
                    ),
                    "strict require=false must engage or fail open, got {:?}",
                    decision.outcome
                );
                #[cfg(target_os = "linux")]
                if matches!(decision.outcome, StrictOutcome::Engaged) {
                    assert!(
                        decision.confinement.is_some(),
                        "an engaged Linux decision must carry a confinement plan"
                    );
                }
            }
            _ => panic!("expected Strict"),
        }
    }
}

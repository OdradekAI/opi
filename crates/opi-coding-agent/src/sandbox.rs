//! Strict-sandbox policy resolver and production dispatch (Phase 15 task 15.5.1).
//!
//! This module owns the **cross-platform** half of the T4 sandbox: it turns a
//! resolved [`crate::config::SandboxConfig`] plus a capability-injected platform
//! backend into a [`PreparedSandbox`] decision. It does NOT implement any OS
//! confinement: the per-platform L1/L2/L3 backends (Landlock+seccomp on Linux,
//! `sandbox-exec` on macOS, L0-only on Windows) are tasks 15.5.2-15.5.5 and plug
//! in by implementing [`StrictBackend`]. Until they land, the production backend
//! selected by [`prepare_production`] truthfully reports every strict layer as
//! unavailable (permanently on platforms that will never have it, temporarily on
//! platforms whose backend ships in a later task), so `strict` mode flows through
//! the shared fail-open / fail-closed policy here.
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

use opi_agent::diagnostic::Diagnostic;

use crate::config::SandboxConfig;
use crate::diagnostics::sandbox_unavailable_diagnostic;

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

/// Capability-injected platform backend.
///
/// Production backends (15.5.2-15.5.5) implement this to report what their
/// platform can engage; tests inject a fake implementation to drive every policy
/// branch without a host kernel feature. Task 15.5.1 needs only the availability
/// query — the actual confinement `engage` seam (e.g. a `pre_exec` hook) is added
/// by the per-platform runtime tasks, not by this cross-platform policy task.
pub trait StrictBackend: Send + Sync {
    /// Report the availability of `layer` on this platform/backend.
    fn availability(&self, layer: SandboxLayer) -> LayerAvailability;
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
/// diagnostics. Not `Eq`: [`Diagnostic`] carries a `serde_json::Value` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct StrictDecision {
    pub outcome: StrictOutcome,
    permanent_diagnostics: Vec<Diagnostic>,
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
pub fn prepare_production(config: &SandboxConfig) -> PreparedSandbox {
    let backend = production_sandbox_backend();
    prepare(config, backend.as_ref())
}

/// Select the production strict backend for the current platform.
///
/// 15.5.1 ships no engaged backend; each platform truthfully reports its strict
/// layers as unavailable. Windows L1-L3 is a *permanent* platform gap (the OS
/// provides no Landlock/seccomp/sandbox-exec equivalent in scope, confirmed by
/// the T4 matrix — 15.5.5 keeps the L0-only truth). Linux and macOS layers are
/// *temporarily* unavailable until 15.5.3 / 15.5.4 wire the real backends, so a
/// user who opts into `strict` during that window sees an honest degraded
/// diagnostic rather than a silent miss. Any other target is permanently
/// unsupported.
pub fn production_sandbox_backend() -> Box<dyn StrictBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(NotYetWiredBackend {
            platform: "linux",
            phase: "15.5.3",
        })
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(NotYetWiredBackend {
            platform: "macos",
            phase: "15.5.4",
        })
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsL0OnlyBackend)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(UnsupportedPlatformBackend)
    }
}

/// Linux/macOS production backend for 15.5.1: strict layers exist on the platform
/// but are not wired into this build yet. Reported as TEMPORARY so fail-open
/// emits a per-command degraded diagnostic (honest: strict did not engage) until
/// 15.5.3 / 15.5.4 replace this stub with a real engaged/temporary/permanent
/// backend. Defined only where it is selected (avoids dead code on other
/// targets).
#[cfg(any(target_os = "linux", target_os = "macos"))]
struct NotYetWiredBackend {
    platform: &'static str,
    phase: &'static str,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
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

/// Windows production backend: L0 Job-Object only, no L1-L3. The T4 matrix fixes
/// this as a permanent platform gap; 15.5.5 owns and refines this same truth.
/// Defined only on Windows.
#[cfg(target_os = "windows")]
struct WindowsL0OnlyBackend;

#[cfg(target_os = "windows")]
impl StrictBackend for WindowsL0OnlyBackend {
    fn availability(&self, _layer: SandboxLayer) -> LayerAvailability {
        LayerAvailability::PermanentlyUnavailable {
            reason: "windows provides no L1-L3 strict confinement (L0 Job-Object only)".to_string(),
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
        // This test runs on the host platform; assert the production backend
        // returns the expected availability family for THIS target. It must not
        // claim engagement (no backend is wired in 15.5.1).
        let backend = production_sandbox_backend();
        for layer in [
            SandboxLayer::Fs,
            SandboxLayer::Network,
            SandboxLayer::Syscalls,
        ] {
            match backend.availability(layer) {
                LayerAvailability::Engaged => panic!("no production backend is wired in 15.5.1"),
                LayerAvailability::TemporarilyUnavailable { .. } => {}
                LayerAvailability::PermanentlyUnavailable { .. } => {}
            }
        }
    }

    #[test]
    fn production_prepare_off_has_no_diagnostics() {
        let prepared = prepare_production(&off_config());
        assert_eq!(prepared, PreparedSandbox::Off);
        assert!(prepared.startup_diagnostics().is_empty());
    }

    #[test]
    fn production_prepare_strict_never_claims_engaged() {
        // On every supported platform the 15.5.1 production backend reports all
        // strict layers unavailable, so a strict request must fail open or
        // closed — never claim engagement it cannot deliver.
        let prepared = prepare_production(&strict_config(false));
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert!(
                    matches!(decision.outcome, StrictOutcome::FailOpen { .. }),
                    "15.5.1 production strict must fail open, got {:?}",
                    decision.outcome
                );
            }
            _ => panic!("expected Strict"),
        }
    }
}

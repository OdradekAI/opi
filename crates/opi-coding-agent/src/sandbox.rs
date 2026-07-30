//! Strict-sandbox policy resolver and production dispatch (Phase 15 task 15.5.1).
//!
//! This module owns the **cross-platform** half of the T4 sandbox: it turns a
//! resolved [`crate::config::SandboxConfig`] plus a capability-injected platform
//! backend into a [`PreparedSandbox`] decision. It does NOT implement any OS
//! confinement: the per-platform L1/L2/L3 backends (Landlock+seccomp on Linux,
//! `sandbox-exec` on macOS, L0-only on Windows) plug in by implementing
//! [`StrictBackend`]. Task 15.5.5 has landed the Windows L0-only backend in
//! `sandbox/windows.rs` (a permanent platform gap); task 15.5.3 has landed the
//! Linux backend (`sandbox/linux.rs`, seccomp + Landlock, selected by
//! [`prepare_production`] on Linux); task 15.5.4 has landed the macOS backend
//! (`sandbox/macos.rs`, `sandbox-exec` L1/L2 deny-overlay with a permanent L3
//! gap, selected by [`prepare_production`] on macOS). `strict` mode flows
//! through the shared fail-open / fail-closed policy here on every platform.
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
use crate::diagnostics::{SandboxReason, sandbox_unavailable_diagnostic};

/// Windows strict backend (L0-only); landed in task 15.5.5.
#[cfg(target_os = "windows")]
mod windows;

/// Linux strict backend (seccomp deny-overlay + Landlock); landed in task 15.5.3.
#[cfg(target_os = "linux")]
pub mod linux;

/// macOS strict backend and host-independent profile/capability model.
pub mod macos;

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
    TemporarilyUnavailable { reason: SandboxReason },
    /// The platform will never provide this layer.
    PermanentlyUnavailable { reason: SandboxReason },
}

/// A parent-built, child-applied confinement plan. The cross-platform resolver
/// carries an `Option<Confinement>` on an `Engaged` strict decision;
/// `LocalBashOperations::exec` applies it to the spawn `Command` between the L0
/// tree setup and `spawn()`. `Confinement` is `Clone` (cheap — the inner state is
/// shared behind an `Arc`) so a resolved `PreparedSandbox` can be reused across
/// commands; each `apply` rebuilds any per-fork state (the Linux backend rebuilds
/// its Landlock ruleset per spawn, since `restrict_self` consumes it).
///
/// Two plans, one per confinement mechanism:
/// - [`Confinement::new`] wraps a `pre_exec` child-setup closure (**Linux**:
///   seccomp + Landlock). `apply` registers it on the child `Command`.
/// - [`Confinement::launcher`] describes a re-launch under a helper that IS the
///   subprocess launcher (**macOS**: `sandbox-exec -p <profile> sh -c …`). The
///   helper must *prepend* itself to the spawn argv, which a `pre_exec` hook
///   cannot do (it cannot change the program), so `apply` is a no-op for a
///   launcher plan and the spawn site (`tool::operations::exec`) rebuilds the
///   `Command` from [`Confinement::launcher_prefix`], re-applying the L0
///   `process_group(0)` + `kill_on_drop` + `current_dir` + `env`.
#[derive(Clone)]
pub struct Confinement(ConfinementKind);

type PreExecHook = dyn Fn(&mut tokio::process::Command) -> Vec<TemporaryGap> + Send + Sync;

#[derive(Clone)]
enum ConfinementKind {
    /// Register a `pre_exec` child-setup hook on the child `Command` (Linux
    /// seccomp + Landlock). `apply` runs it.
    PreExec(Arc<PreExecHook>),
    /// Re-launch the child under `program` followed by `prefix_args`, then the
    /// original (program, args). macOS `sandbox-exec -p <profile>`.
    Launcher {
        program: Arc<str>,
        prefix_args: Arc<[String]>,
    },
}

impl std::fmt::Debug for Confinement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ConfinementKind::PreExec(_) => f
                .debug_struct("Confinement")
                .field("kind", &"pre_exec")
                .finish(),
            ConfinementKind::Launcher { program, .. } => f
                .debug_struct("Confinement")
                .field("kind", &"launcher")
                .field("program", program)
                .finish(),
        }
    }
}

impl Confinement {
    /// Wrap a parent-side Linux setup closure. Returned gaps are resolved before
    /// spawn; the closure may also register the allocation-free child hook.
    pub fn new<F>(hook: F) -> Self
    where
        F: Fn(&mut tokio::process::Command) -> Vec<TemporaryGap> + Send + Sync + 'static,
    {
        Self(ConfinementKind::PreExec(Arc::new(hook)))
    }

    /// Build a launcher confinement (macOS `sandbox-exec`). The spawn site runs
    /// `program prefix_args... <original program> <original args>...`.
    pub fn launcher(program: &str, prefix_args: Vec<String>) -> Self {
        Self(ConfinementKind::Launcher {
            program: Arc::from(program),
            prefix_args: Arc::from(prefix_args),
        })
    }

    /// Apply the `pre_exec` hook to `cmd` (Linux). A no-op for a launcher plan:
    /// the spawn site handles the launcher rebuild because `apply` cannot change
    /// the `Command`'s program/args.
    pub fn apply(&self, cmd: &mut tokio::process::Command) -> Vec<TemporaryGap> {
        if let ConfinementKind::PreExec(hook) = &self.0 {
            hook(cmd)
        } else {
            Vec::new()
        }
    }

    /// If this is a launcher plan, the launcher program and prefix args the
    /// spawn site must prepend before the original program/args. `None` for a
    /// `pre_exec` plan.
    pub fn launcher_prefix(&self) -> Option<(&str, &[String])> {
        match &self.0 {
            ConfinementKind::Launcher {
                program,
                prefix_args,
            } => Some((program, prefix_args)),
            ConfinementKind::PreExec(_) => None,
        }
    }
}

/// Capability-injected platform backend.
///
/// Production backends implement this to report what their platform can engage
/// and build a parent-side [`Confinement`] for the independently engaged subset.
pub trait StrictBackend: Send + Sync {
    /// Report the availability of `layer` on this platform/backend.
    fn availability(&self, layer: SandboxLayer) -> LayerAvailability;

    /// Build the confinement plan for `engaged_layers`. Construction failures
    /// are returned as per-layer gaps and pass through the common require policy.
    fn build_confinement(
        &self,
        _workspace: &std::path::Path,
        _engaged_layers: &[SandboxLayer],
    ) -> ConfinementBuild {
        ConfinementBuild::default()
    }

    /// Optional aggregate diagnostic for a platform whose strict layers are
    /// one capability boundary rather than three independently actionable
    /// facilities. Windows uses this to report its L0-only posture once.
    fn aggregate_permanent_gap(&self) -> Option<(&'static str, SandboxReason)> {
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
    pub reason: SandboxReason,
}

#[derive(Debug, Clone, Default)]
pub struct ConfinementBuild {
    pub confinement: Option<Confinement>,
    /// Complete layer construction failures. The named layers are removed from
    /// the engaged set.
    pub gaps: Vec<TemporaryGap>,
    /// Partial capability gaps where the strongest available sub-capability
    /// remains engaged (for example Linux's seccomp socket gate below Landlock
    /// ABI 4).
    pub degraded: Vec<TemporaryGap>,
}

/// The per-exec decision for a strict request. [`PreparedSandbox`] carries this
/// plus the once-per-startup permanent diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrictOutcome {
    /// Every requested layer engaged.
    Engaged,
    /// `require = false` and at least one requested layer was unavailable.
    /// Independently engaged layers remain active.
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
    /// Whether any construction-time gap must fail closed before spawn.
    pub require: bool,
    /// Strict layers selected by configuration, in stable fs/network/syscalls
    /// order. Explicit `false` toggles are omitted.
    pub requested_layers: Vec<SandboxLayer>,
    /// Requested layers the backend reported as engaged. This remains populated
    /// on fail-open decisions so production can retain partial confinement.
    pub engaged_layers: Vec<SandboxLayer>,
    permanent_diagnostics: Vec<Diagnostic>,
    /// Parent-built plan for the engaged subset. The pure [`prepare`] resolver
    /// leaves this empty; [`prepare_production`] builds it.
    pub confinement: Option<Confinement>,
}

impl PartialEq for StrictDecision {
    fn eq(&self, other: &Self) -> bool {
        // Compare only the observable policy decision. `confinement` holds an
        // opaque closure and is intentionally excluded from equality.
        self.outcome == other.outcome
            && self.require == other.require
            && self.requested_layers == other.requested_layers
            && self.engaged_layers == other.engaged_layers
            && self.permanent_diagnostics == other.permanent_diagnostics
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

    let mut requested = Vec::new();
    let mut engaged = Vec::new();
    let mut permanent: Vec<(SandboxLayer, SandboxReason)> = Vec::new();
    let mut temporary = Vec::new();
    for (layer, toggle) in requested_layers {
        // Some(false) = explicit opt-out: do not query, do not diagnose.
        if toggle == Some(false) {
            continue;
        }
        requested.push(layer);
        match backend.availability(layer) {
            LayerAvailability::Engaged => engaged.push(layer),
            LayerAvailability::TemporarilyUnavailable { reason } => {
                temporary.push(TemporaryGap { layer, reason });
            }
            LayerAvailability::PermanentlyUnavailable { reason } => {
                permanent.push((layer, reason));
            }
        }
    }

    let permanent_diagnostics = if permanent.is_empty() {
        Vec::new()
    } else if let Some((layer, reason)) = backend.aggregate_permanent_gap() {
        vec![sandbox_unavailable_diagnostic(layer, reason)]
    } else {
        permanent
            .iter()
            .map(|(layer, reason)| sandbox_unavailable_diagnostic(layer.as_str(), *reason))
            .collect::<Vec<_>>()
    };

    let outcome = if temporary.is_empty() && permanent.is_empty() {
        StrictOutcome::Engaged
    } else if config.require {
        StrictOutcome::FailClosed {
            reason: summarize_layers(
                permanent
                    .iter()
                    .map(|(layer, _)| *layer)
                    .chain(temporary.iter().map(|gap| gap.layer)),
            ),
        }
    } else {
        StrictOutcome::FailOpen {
            per_command_temporary: temporary,
        }
    };

    PreparedSandbox::Strict(StrictDecision {
        outcome,
        require: config.require,
        requested_layers: requested,
        engaged_layers: engaged,
        permanent_diagnostics,
        confinement: None,
    })
}

/// Build a short, redacted reason summarizing which layers were unavailable, for
/// the fail-closed error message. Layer names only — no command/env/paths.
fn summarize_layers(layers: impl IntoIterator<Item = SandboxLayer>) -> String {
    let mut names: Vec<&str> = layers.into_iter().map(SandboxLayer::as_str).collect();
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
fn prepare_with_backend(
    config: &SandboxConfig,
    workspace: &std::path::Path,
    backend: &dyn StrictBackend,
) -> PreparedSandbox {
    let mut prepared = prepare(config, backend);
    if let PreparedSandbox::Strict(decision) = &mut prepared
        && !matches!(decision.outcome, StrictOutcome::FailClosed { .. })
        && !decision.engaged_layers.is_empty()
    {
        let build = backend.build_confinement(workspace, decision.engaged_layers.as_slice());
        decision.confinement = build.confinement;
        if !build.degraded.is_empty() {
            if decision.require {
                decision.outcome = StrictOutcome::FailClosed {
                    reason: summarize_layers(build.degraded.iter().map(|gap| gap.layer)),
                };
            } else {
                match &mut decision.outcome {
                    StrictOutcome::FailOpen {
                        per_command_temporary,
                    } => per_command_temporary.extend(build.degraded),
                    StrictOutcome::Engaged => {
                        decision.outcome = StrictOutcome::FailOpen {
                            per_command_temporary: build.degraded,
                        };
                    }
                    StrictOutcome::FailClosed { .. } => unreachable!(),
                }
            }
        }
        if !build.gaps.is_empty() {
            let build_gaps = build.gaps;
            decision
                .engaged_layers
                .retain(|layer| !build_gaps.iter().any(|gap| gap.layer == *layer));
            if decision.require {
                decision.outcome = StrictOutcome::FailClosed {
                    reason: summarize_layers(build_gaps.iter().map(|gap| gap.layer)),
                };
            } else {
                match &mut decision.outcome {
                    StrictOutcome::FailOpen {
                        per_command_temporary,
                    } => per_command_temporary.extend(build_gaps),
                    StrictOutcome::Engaged => {
                        decision.outcome = StrictOutcome::FailOpen {
                            per_command_temporary: build_gaps,
                        };
                    }
                    StrictOutcome::FailClosed { .. } => unreachable!(),
                }
            }
        }
    }
    prepared
}

pub fn prepare_production(config: &SandboxConfig, workspace: &std::path::Path) -> PreparedSandbox {
    #[cfg(target_os = "windows")]
    {
        crate::sandbox::windows::prepare(config, workspace)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let backend = production_sandbox_backend(workspace);
        prepare_with_backend(config, workspace, backend.as_ref())
    }
}

/// Select the production strict backend for the current platform.
///
/// - **Linux (15.5.3)**: `LinuxStrictBackend` queries the observed Landlock ABI
///   and builds seccomp+Landlock for the engaged subset.
/// - **macOS (15.5.4)**: `MacosStrictBackend` probes `sandbox-exec` on `PATH` and
///   reports L1 fs + L2 network as engaged when the helper is usable (L3/syscalls
///   is a permanent platform gap). It keeps L1/L2 active under fail-open.
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
        Box::new(crate::sandbox::macos::MacosStrictBackend::new(Arc::from(
            workspace,
        )))
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

/// Fallback for targets outside the Linux/macOS/Windows release matrix. Defined
/// only where selected.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
struct UnsupportedPlatformBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl StrictBackend for UnsupportedPlatformBackend {
    fn availability(&self, _layer: SandboxLayer) -> LayerAvailability {
        LayerAvailability::PermanentlyUnavailable {
            reason: SandboxReason::StrictConfinementUnsupportedPlatform,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SandboxConfig, SandboxMode};
    use crate::tool::{BashOperations, BashRequest, LocalBashOperations};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

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
                assert_eq!(
                    decision.requested_layers,
                    vec![
                        SandboxLayer::Fs,
                        SandboxLayer::Network,
                        SandboxLayer::Syscalls
                    ]
                );
                assert_eq!(decision.engaged_layers, decision.requested_layers);
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
                reason: SandboxReason::LandlockTcpUnavailable,
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
                        assert_eq!(
                            per_command_temporary[0].reason,
                            SandboxReason::LandlockTcpUnavailable
                        );
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
            reason: SandboxReason::WindowsStrictConfinementUnavailable,
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
            reason: SandboxReason::WindowsStrictConfinementUnavailable,
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
            reason: SandboxReason::SeccompFilterBuildFailed,
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
                reason: SandboxReason::WindowsStrictConfinementUnavailable,
            },
            SandboxLayer::Network => LayerAvailability::TemporarilyUnavailable {
                reason: SandboxReason::LandlockTcpUnavailable,
            },
            SandboxLayer::Syscalls => LayerAvailability::Engaged,
        });
        let prepared = prepare(&strict_config(false), backend.as_ref());
        let startup = prepared.startup_diagnostics();
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert_eq!(
                    decision.engaged_layers,
                    vec![SandboxLayer::Syscalls],
                    "fail-open must retain every independently engaged layer"
                );
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
    fn strict_all_layers_disabled_requests_and_engages_nothing() {
        let config = SandboxConfig {
            mode: SandboxMode::Strict,
            require: true,
            fs: Some(false),
            network: Some(false),
            syscalls: Some(false),
        };
        let backend = fake(|layer| panic!("disabled layer was queried: {layer:?}"));
        let prepared = prepare(&config, backend.as_ref());
        match prepared {
            PreparedSandbox::Strict(decision) => {
                assert_eq!(decision.outcome, StrictOutcome::Engaged);
                assert!(decision.requested_layers.is_empty());
                assert!(decision.engaged_layers.is_empty());
                assert!(decision.confinement.is_none());
            }
            PreparedSandbox::Off => panic!("strict with no enabled layers is still strict"),
        }
    }

    #[test]
    fn strict_single_enabled_layer_matrix() {
        for selected in [
            SandboxLayer::Fs,
            SandboxLayer::Network,
            SandboxLayer::Syscalls,
        ] {
            let config = SandboxConfig {
                mode: SandboxMode::Strict,
                require: false,
                fs: Some(selected == SandboxLayer::Fs),
                network: Some(selected == SandboxLayer::Network),
                syscalls: Some(selected == SandboxLayer::Syscalls),
            };
            let PreparedSandbox::Strict(decision) =
                prepare(&config, fake(|_| LayerAvailability::Engaged).as_ref())
            else {
                panic!("expected strict decision");
            };
            assert_eq!(decision.requested_layers, vec![selected]);
            assert_eq!(decision.engaged_layers, vec![selected]);
            assert_eq!(decision.outcome, StrictOutcome::Engaged);
        }
    }

    struct RecordingBackend {
        unavailable: SandboxLayer,
        permanent: bool,
        built_for: Mutex<Vec<SandboxLayer>>,
        applied: Option<Arc<AtomicUsize>>,
    }

    impl StrictBackend for RecordingBackend {
        fn availability(&self, layer: SandboxLayer) -> LayerAvailability {
            if layer != self.unavailable {
                LayerAvailability::Engaged
            } else if self.permanent {
                LayerAvailability::PermanentlyUnavailable {
                    reason: SandboxReason::WindowsStrictConfinementUnavailable,
                }
            } else {
                LayerAvailability::TemporarilyUnavailable {
                    reason: SandboxReason::LandlockTcpUnavailable,
                }
            }
        }

        fn build_confinement(
            &self,
            _workspace: &std::path::Path,
            engaged_layers: &[SandboxLayer],
        ) -> ConfinementBuild {
            *self.built_for.lock().unwrap() = engaged_layers.to_vec();
            let confinement = self.applied.as_ref().map_or_else(
                || Confinement::launcher("launcher", Vec::new()),
                |applied| {
                    let applied = Arc::clone(applied);
                    Confinement::new(move |_| {
                        applied.fetch_add(1, Ordering::SeqCst);
                        Vec::new()
                    })
                },
            );
            ConfinementBuild {
                confinement: Some(confinement),
                gaps: Vec::new(),
                degraded: Vec::new(),
            }
        }
    }

    #[test]
    fn fail_open_builds_confinement_for_the_engaged_subset() {
        let backend = RecordingBackend {
            unavailable: SandboxLayer::Network,
            permanent: false,
            built_for: Mutex::new(Vec::new()),
            applied: None,
        };
        let prepared =
            prepare_with_backend(&strict_config(false), std::path::Path::new("."), &backend);
        let PreparedSandbox::Strict(decision) = prepared else {
            panic!("expected strict decision");
        };
        assert!(matches!(decision.outcome, StrictOutcome::FailOpen { .. }));
        assert!(decision.confinement.is_some());
        assert_eq!(
            *backend.built_for.lock().unwrap(),
            vec![SandboxLayer::Fs, SandboxLayer::Syscalls]
        );
    }

    #[test]
    fn confinement_apply_reports_parent_side_layer_construction_gaps() {
        let confinement = Confinement::new(|_command| {
            vec![TemporaryGap {
                layer: SandboxLayer::Fs,
                reason: SandboxReason::LandlockFilesystemConstructionFailed,
            }]
        });
        let mut command = tokio::process::Command::new("unused");
        let gaps = confinement.apply(&mut command);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].layer, SandboxLayer::Fs);
    }

    #[test]
    fn default_macos_capability_shape_retains_l1_l2_while_reporting_l3() {
        let backend = RecordingBackend {
            unavailable: SandboxLayer::Syscalls,
            permanent: true,
            built_for: Mutex::new(Vec::new()),
            applied: None,
        };
        let prepared =
            prepare_with_backend(&strict_config(false), std::path::Path::new("."), &backend);
        let PreparedSandbox::Strict(decision) = prepared else {
            panic!("expected strict decision");
        };
        assert!(matches!(decision.outcome, StrictOutcome::FailOpen { .. }));
        assert_eq!(
            decision.engaged_layers,
            vec![SandboxLayer::Fs, SandboxLayer::Network]
        );
        assert!(decision.confinement.is_some());
        assert_eq!(
            *backend.built_for.lock().unwrap(),
            vec![SandboxLayer::Fs, SandboxLayer::Network]
        );
        assert_eq!(decision.permanent_diagnostics.len(), 1);
    }

    #[tokio::test]
    async fn default_fail_open_exec_applies_the_engaged_confinement() {
        let applied = Arc::new(AtomicUsize::new(0));
        let backend = RecordingBackend {
            unavailable: SandboxLayer::Syscalls,
            permanent: true,
            built_for: Mutex::new(Vec::new()),
            applied: Some(Arc::clone(&applied)),
        };
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare_with_backend(&strict_config(false), dir.path(), &backend);
        let PreparedSandbox::Strict(decision) = &prepared else {
            panic!("expected strict decision");
        };
        assert!(matches!(decision.outcome, StrictOutcome::FailOpen { .. }));

        LocalBashOperations::with_prepared(prepared)
            .exec(BashRequest {
                command: "printf partial".to_string(),
                cwd: dir.path().to_path_buf(),
                timeout: Duration::from_secs(5),
                signal: CancellationToken::new(),
                env: Vec::new(),
            })
            .await
            .expect("default fail-open execution succeeds");
        assert_eq!(applied.load(Ordering::SeqCst), 1);
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
            reason: SandboxReason::SeccompFilterBuildFailed,
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
        // Engaged on a capable kernel (observed Landlock ABI >= 4). 15.5.4 wires
        // the macOS backend (sandbox-exec L1/L2), which MAY report fs/network
        // Engaged when the helper is usable. The durable cross-platform
        // invariant is syscalls: macOS (L1/L2-only) and Windows (L0-only) can
        // NEVER engage L3, so the syscall layer must not claim engagement off
        // Linux.
        let backend = production_sandbox_backend(std::path::Path::new("."));
        #[cfg(not(target_os = "linux"))]
        {
            let syscalls = backend.availability(SandboxLayer::Syscalls);
            assert!(
                !matches!(syscalls, LayerAvailability::Engaged),
                "non-Linux platforms must not claim syscall engagement: {syscalls:?}"
            );
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

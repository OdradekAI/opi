//! Sandbox policy data and the platform-neutral restriction seam.
//!
//! [`SandboxPolicy`] is the REQUESTED contract (a profile + a network policy),
//! supplied as an explicit input. It is plain data: it carries NO `Deserialize`
//! impl and is never loaded from a file, so an Opi configuration read cannot be
//! wired in later without a visible contract break (design `### State model`;
//! Phase 16 task 16.11.1 audit fold: drop "serde for SandboxPolicy").
//!
//! [`Restriction`] is the seam the runner calls ONCE before spawn to install
//! platform confinement, handing over the command AND a [`RestrictionCtx`]
//! (the per-request workspace + network policy). Its default implementation
//! [`NoRestriction`] applies NO kernel confinement and reports
//! `Mechanism::None` / `ContractStatus::Unrestricted` — the honest status for
//! an unrestricted run. The shipped Linux and macOS native implementations
//! provide the confinement mechanisms in `platform/linux.rs` and
//! `platform/macos.rs`; unsupported postures refuse before target start. The
//! seam shape here (a `&mut tokio::process::Command` plus a [`RestrictionCtx`]
//! handed over before spawn) is the current common integration boundary.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// The requested sandbox profile. Only [`Profile::WorkspaceWrite`] is defined
/// here; the confinement it names is enforced by the native restriction
/// implementations (Landlock/seccomp on Linux, canonical
/// `/usr/bin/sandbox-exec` on macOS), NOT by the default [`NoRestriction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// Writes, creates, removes, and renames are intended to be restricted to
    /// the canonical workspace and an invocation-owned temporary root, with host
    /// reads and binaries remaining executable. The SDK reports this as a
    /// REQUEST only; whether it can be established is platform-dependent and
    /// reported via the restriction seam.
    #[default]
    WorkspaceWrite,
}

/// The requested network policy, enforced separately from filesystem mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkPolicy {
    /// New network access is intended to be denied (the default).
    #[default]
    Deny,
    /// Network access is intended to be allowed.
    Allow,
}

/// The requested sandbox contract: a profile plus a network policy. This is an
/// EXPLICIT INPUT to the SDK; it is never deserialized from disk.
///
/// Construct with [`SandboxPolicy::new`] or [`SandboxPolicy::default`]
/// (`WorkspaceWrite` + `Deny`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SandboxPolicy {
    /// The requested filesystem-mutation profile.
    pub profile: Profile,
    /// The requested network policy.
    pub network: NetworkPolicy,
}

impl SandboxPolicy {
    /// Create a policy from a profile and a network policy.
    pub const fn new(profile: Profile, network: NetworkPolicy) -> Self {
        Self { profile, network }
    }
}

/// The mechanism a [`Restriction`] actually installed. [`Mechanism::None`] is
/// the default (no confinement); [`Mechanism::Landlock`] and
/// [`Mechanism::Seccomp`] are produced together on supported Linux;
/// [`Mechanism::Seatbelt`] is produced on supported macOS by the Apple
/// `sandbox-exec`/Seatbelt deny-overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    /// No kernel confinement was applied (the default restriction).
    None,
    /// Linux Landlock filesystem-write + TCP bind/connect confinement
    /// (ABI-gated).
    Landlock,
    /// A fixed Linux seccomp deny-overlay: the L3 danger blocklist, io_uring
    /// setup denial, and (for `network = deny`) the AF_INET/AF_INET6/AF_NETLINK
    /// socket-creation gate.
    Seccomp,
    /// The macOS Seatbelt deny-overlay installed via `sandbox-exec` (task
    /// 16.14.1): a last-match-wins deny-overlay on a seatbelt allow-default
    /// base, denying file-writes outside the workspace + invocation temp root
    /// and (for `network = deny`) all `network*` operations. Installed by a
    /// [`Restriction::launcher`] parent program, not a `pre_exec` hook. Labeled
    /// legacy/experimental (Apple soft-deprecated `sandbox-exec`); see the
    /// macOS limitations reported by `doctor`.
    Seatbelt,
}

/// The effective contract status a [`Restriction`] reports after `prepare`.
/// [`ContractStatus::Unrestricted`] is the default;
/// [`ContractStatus::Restricted`] is produced by the native implementations
/// on supported Linux and macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractStatus {
    /// No confinement contract was established (the target runs unrestricted,
    /// under L0 supervision only).
    Unrestricted,
    /// A confinement contract was established: the target runs confined under
    /// the reported mechanism(s). The package reports `restricted`, never
    /// `isolated` (design `### Common profile`).
    Restricted,
}

/// What a [`Restriction`] installed on the command, surfaced to the caller via
/// the `started` event so the effective contract is reported honestly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedRestriction {
    /// The mechanism that was applied.
    pub mechanism: Mechanism,
    /// The effective contract status after setup.
    pub contract: ContractStatus,
}

impl AppliedRestriction {
    /// The no-confinement applied restriction reported by [`NoRestriction`].
    pub const fn none() -> Self {
        Self {
            mechanism: Mechanism::None,
            contract: ContractStatus::Unrestricted,
        }
    }
}

/// A redacted reason a [`Restriction::prepare`] call failed. Carries only a
/// static layer/reason token — never command text, arguments, environment
/// values, or paths.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RestrictionSetupError {
    /// The requested contract could not be established before target start.
    /// The string is a static, redacted layer/reason token.
    #[error("restriction setup failed: {0}")]
    Failed(&'static str),
}

/// Per-request context handed to [`Restriction::prepare`]: the canonical
/// workspace root and invocation-owned temporary root a filesystem-confinement
/// ruleset grants writes beneath, the requested network policy, and the
/// cooperative setup cutoff exposed by [`RestrictionCtx::setup_cancelled`].
/// Native restriction implementations consume this to build per-spawn
/// confinement.
#[derive(Debug, Clone, Copy)]
pub struct RestrictionCtx<'a> {
    /// The canonical workspace root the target may write beneath (and that host
    /// reads remain unrestricted around).
    pub workspace: &'a Path,
    /// The canonical invocation-owned temporary root the target may write
    /// beneath. No sibling system-temporary directory is granted.
    pub temp_root: &'a Path,
    /// The requested network policy.
    pub network: NetworkPolicy,
    /// Absolute cutoff for cooperative protocol pre-spawn setup. Direct SDK
    /// preparation has no setup deadline and stores `None`.
    pub(crate) setup_deadline: Option<Instant>,
    /// Cancellation fired when setup reaches its cutoff or the request is
    /// cancelled by its owner.
    pub(crate) setup_cancel: &'a CancellationToken,
}

impl RestrictionCtx<'_> {
    /// Whether cooperative setup must stop before creating more side effects.
    pub fn setup_cancelled(&self) -> bool {
        self.setup_cancel.is_cancelled()
            || self
                .setup_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

/// A parent-program prefix a [`Restriction`] can ask the runner to wrap the
/// target in BEFORE the command is built (task 16.14.1). The only current use is
/// the macOS Seatbelt confinement, where `sandbox-exec` must be the parent
/// process that applies the rendered profile to its child (the target). The
/// runner builds `Command::new(spec.program).args(spec.prefix).arg(target).args(target_args)`
/// and then applies `current_dir`/stdio/env/process-tree configuration
/// IDENTICALLY to the bare-program path, so the launcher inherits the target's
/// cwd/env/stdio.
///
/// `std::process::Command` exposes no getter for stdio/kill_on_drop/env_clear
/// and no prepend/reprogram API, so a launcher cannot be installed later inside
/// [`Restriction::prepare`] (a rebuild would drop the runner's piped stdio and
/// env policy). Computing the launcher spec up front — before the command is
/// built — is the only faithful way to wrap the target, which is why this is a
/// separate seam entry point rather than a `prepare` side effect.
#[derive(Debug, Clone)]
pub struct LauncherSpec {
    /// The parent program to spawn (e.g. `/usr/bin/sandbox-exec`).
    pub program: PathBuf,
    /// The prefix arguments inserted before the target program (e.g.
    /// `["-p", "<profile>"]`).
    pub prefix: Vec<String>,
}

/// Platform-neutral restriction seam. The runner drives a restriction in two
/// cooperative steps, both BEFORE spawn:
///
/// 1. [`Restriction::launcher`] — ask whether the target should be wrapped in a
///    parent program (returns `Ok(None)` for the default and for in-place
///    `pre_exec`-style confinement such as Linux Landlock/seccomp; `Ok(Some)`
///    for macOS Seatbelt, which needs `sandbox-exec` as the parent). Invalid
///    per-invocation launcher inputs fail before spawn.
/// 2. [`Restriction::prepare`] — install any in-place confinement on the built
///    command and report the effective mechanism/contract.
///
/// These two are COOPERATIVE, not independent: an implementation that returns
/// `Ok(Some(launcher))` MUST make [`Restriction::prepare`] a no-op-on-`cmd` that
/// only reports the mechanism/contract (the launcher already installed the
/// confinement); an implementation that returns `Ok(None)` from
/// [`Restriction::launcher`] does its confinement work in
/// [`Restriction::prepare`] (e.g. a `pre_exec` hook). The reported
/// mechanism/contract MUST agree with whether [`Restriction::launcher`] wrapped
/// the command. The returned [`AppliedRestriction`] is reported on the `started`
/// event so the effective contract is always honest.
/// Implementations must check [`RestrictionCtx::setup_cancelled`] before and
/// after potentially blocking setup steps and stop without spawning helpers or
/// creating further side effects when it returns true.
///
/// The default implementation [`NoRestriction`] applies nothing and reports
/// `Mechanism::None` / `ContractStatus::Unrestricted`.
pub trait Restriction: Send + Sync {
    /// Ask whether the runner should wrap the target in a parent program before
    /// the command is built. The default returns `None` (no launcher; the
    /// default and the Linux `pre_exec`-style paths). An implementation returns
    /// `Some` only when confinement requires a parent process (macOS Seatbelt).
    /// Fallible because a per-invocation launcher can reject native inputs that
    /// its policy language cannot represent exactly. Such failures happen
    /// before the launcher or target is spawned.
    fn launcher(
        &self,
        _ctx: &RestrictionCtx<'_>,
    ) -> Result<Option<LauncherSpec>, RestrictionSetupError> {
        Ok(None)
    }

    /// Install in-place confinement on `cmd` before it is spawned, using the
    /// per-request `ctx` (workspace + network policy), returning the effective
    /// mechanism/contract. Called once, pre-spawn, from the runner, AFTER the
    /// launcher (if any) has been applied. A launcher-based implementation
    /// (macOS) makes this a no-op-on-`cmd` that only reports the
    /// mechanism/contract; a `pre_exec`-style implementation (Linux) does its
    /// work here. An `Err` fails closed before the target is released.
    fn prepare(
        &self,
        cmd: &mut Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError>;
}

/// The default [`Restriction`]: applies NO kernel confinement and reports the
/// run as unrestricted (L0 supervision only). Direct SDK callers may supply it
/// explicitly; the platform-gated CLI/backend instead use the current native
/// restriction and refuse an unsupported posture before target start.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRestriction;

impl Restriction for NoRestriction {
    fn prepare(
        &self,
        _cmd: &mut Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        Ok(AppliedRestriction::none())
    }
}

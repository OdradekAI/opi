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
//! an unrestricted run. The Linux native implementation (`Landlock` +
//! `Seccomp`) is supplied by task 16.13 in `platform/linux.rs`; the macOS
//! implementation lands in 16.14.1, and the Windows unsupported posture in
//! 16.14.2. The seam shape here (a `&mut tokio::process::Command` plus a
//! [`RestrictionCtx`] handed over before spawn) is what makes those fillable
//! without a breaking change.

#![forbid(unsafe_code)]

use std::path::Path;
use tokio::process::Command;

/// The requested sandbox profile. Only [`Profile::WorkspaceWrite`] is defined
/// here; the confinement it names is enforced by the native restriction
/// implementations (Landlock/seccomp on Linux in 16.13, `sandbox-exec` on macOS
/// in 16.14.1), NOT by the default [`NoRestriction`].
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
/// [`Mechanism::Seccomp`] are produced together on supported Linux by task
/// 16.13. macOS (`sandbox-exec`) lands in 16.14.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    /// No kernel confinement was applied (the default restriction).
    None,
    /// Linux Landlock filesystem-write + TCP bind/connect confinement
    /// (ABI-gated; task 16.13).
    Landlock,
    /// A fixed Linux seccomp deny-overlay: the L3 danger blocklist, io_uring
    /// setup denial, and (for `network = deny`) the AF_INET/AF_INET6/AF_NETLINK
    /// socket-creation gate (task 16.13).
    Seccomp,
}

/// The effective contract status a [`Restriction`] reports after `prepare`.
/// [`ContractStatus::Unrestricted`] is the default;
/// [`ContractStatus::Restricted`] is produced by the native implementations
/// (Linux in 16.13, macOS in 16.14.1).
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
/// workspace root a filesystem-confinement ruleset grants writes beneath, and
/// the requested network policy. Native restriction implementations consume
/// this to build per-spawn confinement; [`NoRestriction`] ignores it.
#[derive(Debug, Clone, Copy)]
pub struct RestrictionCtx<'a> {
    /// The canonical workspace root the target may write beneath (and that host
    /// reads remain unrestricted around).
    pub workspace: &'a Path,
    /// The requested network policy.
    pub network: NetworkPolicy,
}

/// Platform-neutral restriction seam. The runner calls [`Restriction::prepare`]
/// exactly once, BEFORE spawn, handing over the command and a [`RestrictionCtx`]
/// so an implementation can install confinement (e.g. register a `pre_exec`
/// hook or rewrite the argv). The returned [`AppliedRestriction`] is reported on
/// the `started` event so the effective contract is always honest.
///
/// The default implementation [`NoRestriction`] applies nothing and reports
/// `Mechanism::None` / `ContractStatus::Unrestricted`.
pub trait Restriction: Send + Sync {
    /// Install confinement on `cmd` before it is spawned, using the per-request
    /// `ctx` (workspace + network policy), returning the effective
    /// mechanism/contract. Called once, pre-spawn, from the runner. An `Err`
    /// fails closed before the target is released.
    fn prepare(
        &self,
        cmd: &mut Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError>;
}

/// The default [`Restriction`]: applies NO kernel confinement and reports the
/// run as unrestricted (L0 supervision only). The native Linux replacement
/// (`Landlock` + `Seccomp`) lands in task 16.13; macOS in 16.14.1.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRestriction;

impl Restriction for NoRestriction {
    fn prepare(
        &self,
        _cmd: &mut Command,
        _ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        Ok(AppliedRestriction::none())
    }
}

//! Sandbox policy data and the platform-neutral restriction seam.
//!
//! [`SandboxPolicy`] is the REQUESTED contract (a profile + a network policy),
//! supplied as an explicit input. It is plain data: it carries NO `Deserialize`
//! impl and is never loaded from a file, so an Opi configuration read cannot be
//! wired in later without a visible contract break (design `### State model`;
//! Phase 16 task 16.11.1 audit fold: drop "serde for SandboxPolicy").
//!
//! [`Restriction`] is the seam the runner calls ONCE before spawn to install
//! platform confinement. Its only implementation in this crate is
//! [`NoRestriction`], which applies NO kernel confinement and reports
//! `Mechanism::None` / `ContractStatus::Unrestricted` — the honest status for
//! an unrestricted run. Native implementations (Landlock/seccomp, `sandbox-exec`,
//! Windows refusal) are supplied by tasks 16.13 / 16.14.1 / 16.14.2 by replacing
//! the restriction installed on the [`crate::SandboxRunner`]; the seam shape here
//! (a `&mut tokio::process::Command` handed over before spawn) is what makes
//! those fillable without a breaking change.

#![forbid(unsafe_code)]

use tokio::process::Command;

/// The requested sandbox profile. Only [`Profile::WorkspaceWrite`] is defined
/// here; the confinement it names is enforced by later native tasks
/// (16.13 / 16.14.1), NOT by this library's default restriction.
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

/// The mechanism a [`Restriction`] actually installed. Only [`Mechanism::None`]
/// is produced by this crate; native tasks add the platform mechanisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    /// No kernel confinement was applied (the default restriction).
    None,
}

/// The effective contract status a [`Restriction`] reports after `prepare`.
/// Only [`ContractStatus::Unrestricted`] is produced by this crate; native tasks
/// add the restricted status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractStatus {
    /// No confinement contract was established (the target runs unrestricted,
    /// under L0 supervision only).
    Unrestricted,
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

/// Platform-neutral restriction seam. The runner calls [`Restriction::prepare`]
/// exactly once, BEFORE spawn, handing over the command so an implementation
/// can install confinement (e.g. register a `pre_exec` hook or rewrite the
/// argv). The returned [`AppliedRestriction`] is reported on the `started`
/// event so the effective contract is always honest.
///
/// The default implementation [`NoRestriction`] applies nothing and reports
/// `Mechanism::None` / `ContractStatus::Unrestricted`.
pub trait Restriction: Send + Sync {
    /// Install confinement on `cmd` before it is spawned, returning the
    /// effective mechanism/contract. Called once, pre-spawn, from the runner.
    fn prepare(&self, cmd: &mut Command) -> Result<AppliedRestriction, RestrictionSetupError>;
}

/// The default [`Restriction`]: applies NO kernel confinement and reports the
/// run as unrestricted (L0 supervision only). This is the only restriction
/// implementation in this crate; native replacements land in 16.13 / 16.14.1.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRestriction;

impl Restriction for NoRestriction {
    fn prepare(&self, _cmd: &mut Command) -> Result<AppliedRestriction, RestrictionSetupError> {
        Ok(AppliedRestriction::none())
    }
}

//! Standalone dependency-neutral command-execution sandbox SDK (Phase 16 task 16.11.1).
//!
//! This crate is the LIBRARY half of the independent `opi-sandbox` product
//! (the human/protocol CLI binary is task 16.11.2). It exposes five documented
//! public types over EXPLICIT inputs — [`SandboxPolicy`], [`SandboxRequest`],
//! [`SandboxRunner`], [`SandboxEvent`], and [`SandboxResult`] — and provides the
//! always-on L0 process-tree supervision that owns and removes an
//! invocation-owned temporary root and the whole child tree on every terminal
//! path (success, timeout, cancellation, error, and dropped future).
//!
//! # Dependency neutrality
//!
//! The crate depends on `opi_protocol` (for the shared `execution::v1::EnvInherit`
//! policy and the wire identity it sits above) and standalone async/runtime
//! crates only. It does NOT depend on `opi-agent` or `opi-coding-agent`, and it
//! never reads Opi configuration, sessions, package storage, or trust state.
//! Policy and configuration are explicit inputs; there is no daemon, session,
//! history, credential, or package database (design `### State model`).
//!
//! # Restriction scope (honest)
//!
//! This library ships a platform-neutral [`policy::Restriction`] SEAM whose
//! default ([`policy::NoRestriction`]) applies NO kernel confinement and reports
//! `Mechanism::None` / `ContractStatus::Unrestricted`. Native restriction is
//! implemented for Linux (Landlock + seccomp, task 16.13) and for macOS
//! (`sandbox-exec`/Seatbelt, task 16.14.1); Windows publishes no confinement
//! artifact in Phase 16 (the unsupported posture, task 16.14.2). A supported
//! Linux run reports [`Mechanism::Landlock`] as the lead mechanism in its
//! per-run `Started` event, while `opi-sandbox doctor --json` reports the full
//! observed Landlock-plus-seccomp posture. A supported macOS run reports
//! [`Mechanism::Seatbelt`] in `Started`. Both use
//! [`ContractStatus::Restricted`] — never `isolated` (design `### Common
//! profile`: the package reports `restricted`).
//!
//! # L0 supervision
//!
//! [`SandboxRunner::run`] spawns the target under an owned [`SandboxRun`] handle
//! that attaches the OS process tree ([`process_tree::TreeGuard`]), races `wait`
//! / timeout / cancellation, and terminates the whole tree on every outcome.
//! Dropping an in-flight [`SandboxRun`] drops the owned child (`kill_on_drop`),
//! the tree guard, and the invocation-owned temp root, so a dropped future cannot
//! orphan descendants or leak the temp root (design `### L0 supervision`).

#![deny(missing_docs)]

pub mod backend;
pub mod cli;
pub mod policy;
pub mod process_tree;
pub mod runner;

pub(crate) mod helper;
pub(crate) mod platform;

pub use policy::{
    AppliedRestriction, ContractStatus, Mechanism, NetworkPolicy, NoRestriction, Profile,
    Restriction, SandboxPolicy,
};
pub use process_tree::{AttachError, TerminationOutcome, TreeGuard, TreeReason};
pub use runner::{
    CleanupState, OutputStream, SandboxEvent, SandboxOutcome, SandboxRequest, SandboxResult,
    SandboxRun, SandboxRunner, SetupFailed, SetupFailureReason, StdinPolicy,
};

//! Platform restriction posture: whether the host can establish a confinement
//! contract, and with which mechanism/limitations.
//!
//! Task 16.11.2 shipped every platform as [`Posture::supported`] == false (no
//! native confinement wired). Task 16.13 flips the Linux arm to `Supported`:
//! `linux::posture()` probes the observed Landlock ABI, reports the
//! `Landlock` + `Seccomp` mechanisms and honest limitations, and installs a
//! `Landlock`+`seccomp` [`crate::policy::Restriction`]. macOS remains
//! `Unsupported` until 16.14.1, and Phase 16 publishes no Windows confinement
//! artifact. The human CLI's `run` therefore refuses BEFORE target start
//! (exit 125) on Windows/macOS, and executes confined on supported Linux;
//! `doctor` serializes a [`Posture`] directly, so there is no split brain
//! between the dispatcher and the diagnostic (Phase 16 task 16.11.2 audit fold:
//! platform-posture-honesty).
//!
//! # `forbid(unsafe_code)`
//!
//! This module is `#![forbid(unsafe_code)]`, which propagates to the `linux`
//! submodule and cannot be overridden (the Phase 15 trap, mirrored at
//! `opi-coding-agent/src/sandbox/linux.rs:36-38`). The audited confinement FFI
//! (the Landlock ABI probe and the `pre_exec` child-setup) therefore lives in
//! `crate::process_tree`, the crate's documented FFI home; `linux.rs` builds the
//! parent-side confinement plan from the safe `landlock`/`seccompiler` APIs and
//! wires that FFI.

#![forbid(unsafe_code)]

use crate::policy::{Mechanism, Restriction};
use std::sync::Arc;

#[cfg(windows)]
mod windows;

// The native Linux leaf (Landlock + seccomp). Compiled only on Linux; on every
// other target this module is absent and `current()` falls through to the
// unsupported arms. Declared under this forbid(unsafe_code) module, so it
// inherits the forbid and contains NO `unsafe` itself; the FFI it wires lives
// in `crate::process_tree`.
#[cfg(target_os = "linux")]
mod linux;

/// The platform's restriction posture. [`current`] returns this for the host;
/// the CLI consumes `supported` to gate `run` and `doctor` serializes the rest.
#[derive(Clone)]
pub(crate) struct Posture {
    /// Whether this platform can establish the requested restriction contract.
    pub supported: bool,
    /// The mechanisms a `Supported` platform installs. Empty while `supported`
    /// is false.
    pub mechanisms: Vec<Mechanism>,
    /// Honest per-platform caveats reported by `doctor`. The strings distinguish
    /// "not yet wired in this build" (macOS, temporary) from "no command
    /// restriction" (Windows, permanent) so users do not infer permanent
    /// platform inferiority.
    pub limitations: Vec<String>,
    /// The restriction to install on a `Supported` platform. `None` while
    /// `supported` is false; the CLI refuses `run` before constructing a runner.
    pub restriction: Option<Arc<dyn Restriction>>,
}

/// The current host's restriction posture. Linux (16.13) reports `Supported`
/// (Landlock + seccomp when the observed ABI/seccomp arch allow it); macOS and
/// other Unix report `Unsupported` (sandbox-exec lands in 16.14.1); Windows
/// reports its permanent `Unsupported` posture.
pub(crate) fn current() -> Posture {
    #[cfg(target_os = "linux")]
    {
        linux::posture()
    }
    // macOS and any other Unix that is not Linux.
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        default_unix_posture()
    }
    // Windows (and any non-Unix target with a windows posture).
    #[cfg(not(any(target_os = "linux", unix)))]
    {
        windows::posture()
    }
}

/// The non-Linux Unix-family posture (macOS/other). `Unsupported` — the native
/// mechanisms are not wired in this build (`sandbox-exec` lands in 16.14.1).
/// `std::env::consts::OS` is a compile-time constant used only to choose the
/// honest limitation string; it is not a host-configuration read. Compiled only
/// on Unix-that-is-not-Linux so it is not dead code on Linux (the Linux arm of
/// [`current`] dispatches to `linux::posture` instead).
#[cfg(all(unix, not(target_os = "linux")))]
fn default_unix_posture() -> Posture {
    let limitation = match std::env::consts::OS {
        "macos" => {
            "native filesystem/network confinement is not yet wired \
                    (sandbox-exec lands in task 16.14.1); runs are unrestricted \
                    under L0 supervision only"
        }
        _ => {
            "native confinement is not supported on this platform; runs are \
              unrestricted under L0 supervision only"
        }
    };
    Posture {
        supported: false,
        mechanisms: Vec::new(),
        limitations: vec![limitation.to_string()],
        restriction: None,
    }
}

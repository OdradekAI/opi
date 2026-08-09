//! Platform restriction posture: whether the host can establish a confinement
//! contract, and with which mechanism/limitations.
//!
//! Linux and macOS install native restrictions when available; Windows has L0
//! Job-Object supervision but Phase 16 publishes no Windows confinement
//! artifact. `linux::posture()` reports Landlock + seccomp, while
//! `macos::posture()` reports Seatbelt through `sandbox-exec`. The human CLI's
//! `run` executes under those supported native postures and refuses BEFORE
//! target start (exit 125) on Windows or any host without an available native
//! restriction. `doctor` serializes a [`Posture`] directly, so there is no split
//! brain between the dispatcher and the diagnostic.
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

// The native macOS leaf (Seatbelt/sandbox-exec). Unlike `linux`, this module is
// declared UNGATED: its pure profile model compiles on every target so the
// host-independent profile invariants are tested on any host, while only the
// runtime items (probe/`MacosRestriction`/`posture`) are individually
// `cfg(target_os = "macos")`. A file-level cfg would compile the pure model out
// on Windows and silently report 0 tests (Phase 16 task 16.14.1 design-audit
// fold). Inherits the forbid(unsafe_code); macOS confinement is a launcher, so
// it contains NO `unsafe` and wires NO `process_tree` FFI.
mod macos;

/// The platform's restriction posture. [`current`] returns this for the host;
/// the CLI consumes `supported` to gate `run` and `doctor` serializes the rest.
#[derive(Clone)]
pub(crate) struct Posture {
    /// Whether this platform can establish the requested restriction contract.
    pub supported: bool,
    /// The mechanisms a `Supported` platform installs. Empty while `supported`
    /// is false.
    pub mechanisms: Vec<Mechanism>,
    /// Honest per-platform caveats reported by `doctor`: residual limitations
    /// for a supported native posture, or why this host cannot establish one.
    /// Unsupported postures never imply an unrestricted execution fallback.
    pub limitations: Vec<String>,
    /// The restriction to install on a `Supported` platform. `None` while
    /// `supported` is false; the CLI refuses `run` before constructing a runner.
    pub restriction: Option<Arc<dyn Restriction>>,
}

/// The current host's restriction posture. Linux (16.13) reports `Supported`
/// (Landlock + seccomp when the observed ABI/seccomp arch allow it); macOS
/// (16.14.1) reports `Supported` (Seatbelt/sandbox-exec when the helper probes
/// usable); other Unix report `Unsupported`; Windows reports its permanent
/// `Unsupported` posture.
pub(crate) fn current() -> Posture {
    #[cfg(target_os = "linux")]
    {
        linux::posture()
    }
    // macOS (16.14.1): the Seatbelt/sandbox-exec deny-overlay.
    #[cfg(target_os = "macos")]
    {
        macos::posture()
    }
    // Any other Unix that is neither Linux nor macOS: unsupported.
    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        default_unix_posture()
    }
    // Windows (and any non-Unix target with a windows posture).
    #[cfg(not(any(target_os = "linux", target_os = "macos", unix)))]
    {
        windows::posture()
    }
}

/// The non-Linux non-macOS Unix-family posture (FreeBSD/other). `Unsupported` —
/// no native confinement is wired for this build. Compiled only on Unix that is
/// neither Linux nor macOS (matching its only call site in [`current`]), so it
/// is not dead code on Linux/macOS — those dispatch to their native `posture`
/// instead.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn default_unix_posture() -> Posture {
    Posture {
        supported: false,
        mechanisms: Vec::new(),
        limitations: vec![
            "native confinement is not supported on this platform; the human CLI refuses before target start"
                .to_string(),
        ],
        restriction: None,
    }
}

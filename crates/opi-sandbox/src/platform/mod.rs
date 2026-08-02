//! Platform restriction posture: whether the host can establish a confinement
//! contract, and with which mechanism/limitations.
//!
//! In task 16.11.2 EVERY platform reports [`Posture::supported`] == false — no
//! native confinement mechanism is wired yet (Linux Landlock/seccomp lands in
//! 16.13; macOS `sandbox-exec` in 16.14.1; Phase 16 publishes no Windows
//! confinement artifact). The human CLI's `run` therefore refuses BEFORE target
//! start (exit 125) on every platform this phase; successful native run is owned
//! by 16.13/16.14.1. `doctor` serializes a [`Posture`] directly, so there is no
//! split brain between the dispatcher and the diagnostic (Phase 16 task 16.11.2
//! audit fold: platform-posture-honesty).
//!
//! Forward compatibility: 16.13 replaces the Linux arm with a `Supported` posture
//! (mechanisms + restriction), and 16.14.1 the macOS arm; the shape here carries
//! the doctor metadata so those tasks only fill values, not structure.

#![forbid(unsafe_code)]

use crate::policy::{Mechanism, Restriction};
use std::sync::Arc;

#[cfg(windows)]
mod windows;

/// The platform's restriction posture. [`current`] returns this for the host;
/// the CLI consumes `supported` to gate `run` and `doctor` serializes the rest.
#[derive(Clone)]
pub(crate) struct Posture {
    /// Whether this platform can establish the requested restriction contract.
    pub supported: bool,
    /// The mechanisms a `Supported` platform installs. Empty while `supported`
    /// is false (16.11.2 every platform).
    pub mechanisms: Vec<Mechanism>,
    /// Honest per-platform caveats reported by `doctor`. The strings distinguish
    /// "not yet wired in this build" (Linux/macOS, temporary) from "no command
    /// restriction" (Windows, permanent) so users do not infer permanent
    /// platform inferiority.
    pub limitations: Vec<String>,
    /// The restriction to install on a `Supported` platform. `None` while
    /// `supported` is false; the CLI refuses `run` before constructing a runner.
    pub restriction: Option<Arc<dyn Restriction>>,
}

/// The current host's restriction posture. Task 16.11.2: `Unsupported` on every
/// platform (no native mechanism wired). 16.13/16.14.1 flip the Linux/macOS
/// arms to `Supported`.
pub(crate) fn current() -> Posture {
    #[cfg(windows)]
    {
        windows::posture()
    }
    #[cfg(not(windows))]
    {
        default_unix_posture()
    }
}

/// The default Unix-family posture (linux/macos/other). Task 16.11.2:
/// `Unsupported` — the native mechanisms are not wired yet. `std::env::consts::OS`
/// is a compile-time constant used only to choose the honest limitation string;
/// it is not a host-configuration read. Only compiled on non-Windows hosts (the
/// Windows arm has its own posture); on Windows this would be dead code.
#[cfg(not(windows))]
fn default_unix_posture() -> Posture {
    let limitation = match std::env::consts::OS {
        "linux" => {
            "native filesystem/network confinement is not yet wired \
                    (Landlock/seccomp land in task 16.13); runs are unrestricted \
                    under L0 supervision only"
        }
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

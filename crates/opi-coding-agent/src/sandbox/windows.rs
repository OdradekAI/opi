//! Windows strict-sandbox backend (Phase 15 task 15.5.5).
//!
//! Windows has no in-scope L1-L3 confinement — no Landlock, seccomp, or
//! `sandbox-exec` equivalent — so the strict policy truthfully reports every
//! strict layer as a PERMANENT platform gap. The always-on L0 Job-Object
//! lifecycle from task 15.4 (kill-on-close, no breakaway) is unchanged and
//! remains the only confinement a bash subprocess tree receives.
//!
//! With this backend the shared policy in the parent module resolves a strict
//! request as:
//! - `require = false` -> fail-open at the L0 baseline; one aggregate permanent
//!   gap surfaces ONCE at startup via `CODE_SANDBOX_UNAVAILABLE`, never per
//!   command.
//! - `require = true` -> fail-closed: `LocalBashOperations` returns
//!   `BashOpError::SandboxUnavailable` before any process is created.
//!
//! This module does NO FFI; the L0 Job-Object code lives in `tool/process_tree.rs`.
//! Like the rest of the sandbox policy it stays `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]

use super::{LayerAvailability, PreparedSandbox, SandboxLayer, StrictBackend};
use crate::config::SandboxConfig;

const WINDOWS_STRICT_GAP_LAYER: &str = "strict";
const WINDOWS_STRICT_GAP_REASON: &str =
    "windows provides no L1-L3 strict confinement (L0 Job-Object only)";

/// Windows L0-only strict backend: every strict layer is a permanent platform
/// gap. On Windows, `prepare_production` routes through [`prepare`] (which feeds
/// this backend to the shared `prepare` resolver), and
/// `production_sandbox_backend` returns this backend for the host-classification
/// path.
pub(crate) struct L0OnlyBackend;

impl StrictBackend for L0OnlyBackend {
    fn availability(&self, _layer: SandboxLayer) -> LayerAvailability {
        LayerAvailability::PermanentlyUnavailable {
            reason: WINDOWS_STRICT_GAP_REASON.to_string(),
        }
    }

    fn aggregate_permanent_gap(&self) -> Option<(&'static str, &'static str)> {
        Some((WINDOWS_STRICT_GAP_LAYER, WINDOWS_STRICT_GAP_REASON))
    }
}

/// Resolve the Windows strict policy against the L0-only backend. This is the
/// Windows entry point on the production dispatch path: `prepare_production`
/// calls it on `target_os = "windows"`.
pub(crate) fn prepare(config: &SandboxConfig, workspace: &std::path::Path) -> PreparedSandbox {
    super::prepare_with_backend(config, workspace, &L0OnlyBackend)
}

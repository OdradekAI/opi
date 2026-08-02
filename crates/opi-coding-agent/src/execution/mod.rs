//! Phase 16 pluggable execution: `command.execute` contribution validation,
//! routing, the protocol host, and the execution runtime assembly.
//!
//! Built up incrementally across Phase 16. Task 16.4 contributes the static
//! contribution-validation seam ([`contribution`]); task 16.6 adds the stable
//! failure envelope ([`failure`]), the capability-permission policy
//! ([`permission`]), and the deterministic router ([`router`]); task 16.7 adds
//! the one-shot protocol host ([`protocol_host`]); later tasks add the runtime
//! assembly.

pub mod contribution;
pub mod failure;
pub mod permission;
pub mod protocol_host;
pub mod router;

pub use contribution::{
    ContributionValidationError, LockMaterial, PackageSource, ValidatedExecutableContribution,
    validate_executable_contributions,
};
pub use failure::{ExecutionFailure, UnavailableDetail};
pub use permission::{LOCAL_ADAPTER_ID, PermissionPolicy};
pub use protocol_host::{
    BackendLaunch, CompletedOutcome, ExecutionProtocolHost, ExecutionRequest, ReadyReport,
    StartedReport,
};
pub use router::{Eligibility, EligibleAdapter, Selection, resolve_selection};

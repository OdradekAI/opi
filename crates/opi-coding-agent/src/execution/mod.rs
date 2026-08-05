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
pub mod runtime;

pub use contribution::{
    ContributionValidationError, LockMaterial, PackageSource, ValidatedExecutableContribution,
    validate_executable_contributions,
};
pub use failure::{ExecutionFailure, UnavailableDetail};
pub use permission::{
    InteractivePermissionBroker, LOCAL_ADAPTER_ID, PermissionManager, PermissionPolicy,
};
pub use protocol_host::{
    BackendLaunch, CompletedOutcome, ExecutionProtocolFailure, ExecutionProtocolHost,
    ExecutionRequest, ReadyReport, StartedReport,
};
pub use router::{Eligibility, EligibleAdapter, Selection, resolve_selection};
// 16.8 re-exports only the 16.9-facing assembly surfaces. The concrete routed
// backend (`RoutedBashOperations`) and external adapter (`ProcessCommandAdapter`)
// stay pub-in-module: `build` returns `Arc<dyn BashOperations>`, so callers and
// tests drive them opaquely.
pub use runtime::{EnabledIdentity, ExecutionRuntime, IdentitySource};

//! Phase 16 pluggable execution: `command.execute` contribution validation,
//! routing, the protocol host, and the execution runtime assembly.
//!
//! Built up incrementally across Phase 16. Task 16.4 contributes the static
//! contribution-validation seam ([`contribution`]); later tasks add routing,
//! the protocol host, and runtime assembly.

pub mod contribution;

pub use contribution::{
    ContributionValidationError, LockMaterial, PackageSource, ValidatedExecutableContribution,
    validate_executable_contributions,
};

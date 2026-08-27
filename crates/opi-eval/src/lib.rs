//! Unpublished Independent Companion for cross-agent evaluation experiments.
//!
//! Every type, module, and command in this crate is a **provisional Phase 18
//! seam**: nothing here is a durable public promise until the complete Phase 18
//! integration matrix proves the seam, and the crate is `publish = false`.
//! The crate is an Agent-neutral workspace member: it depends on no Opi crate
//! (normal, dev, build, optional, or target-specific), no Opi product depends
//! on it, and it registers no provider, tool, package, command, extension,
//! startup hook, or default capture path in `opi`.
//!
//! The library currently exposes the minimum entry seam required by the
//! same-package CLI and integration tests: [`experiment::ResolvedExperiment`]
//! for canonical experiment resolution and [`cli::validate`] for the
//! `opi-eval validate` command.

#![forbid(unsafe_code)]

pub mod cli;
pub mod experiment;

// Crate-private admission contract for Phase 18 external execution locks. It
// is deliberately not part of the provisional entry seam; runner and adapter
// modules inside this crate consume it.
#[allow(dead_code)]
mod external_lock;

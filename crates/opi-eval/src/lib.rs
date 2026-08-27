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

// `deny` (not `forbid`) because `process::tree` is this crate's single
// documented unsafe-FFI home: the OS tree-termination primitives (Unix
// process groups, Windows Job Objects) have no safe alternative. Every other
// module stays unsafe-free; `process::tree` overrides the lint locally and
// wraps each call in a safe API.
#![deny(unsafe_code)]

pub mod cli;
pub mod experiment;

// Crate-private admission contract for Phase 18 external execution locks. It
// is deliberately not part of the provisional entry seam; runner and adapter
// modules inside this crate consume it.
#[allow(dead_code)]
mod external_lock;

// Crate-private failure-boundary codes (Phase 18 task 18.5). One stable
// owning-boundary code per failure table row; typed failures carry it.
#[allow(dead_code)]
mod failure;

// Crate-private benchmark integrity records (Phase 18 task 18.5.1). Owns
// revision admission, per-task validity classification, and reclassification
// identity; nothing an Agent, adapter, or LLM produces can reach it.
#[allow(dead_code)]
mod integrity;

// Crate-private pairing and comparability assembly (Phase 18 task 18.5.1).
// Consumes the frozen ResolvedExperiment and an admitted IntegrityRecord
// read-only; assembles exactly one baseline/candidate pair per
// edge-task-group only when every control fingerprint agrees.
#[allow(dead_code)]
mod comparison;

// Crate-private durable trial bundle (Phase 18 task 18.5). Owns canonical
// sealing, mutation rejection, and intent-before-effect persistence; the
// runner lifecycle and later regrade/report consumers stay inside this crate.
#[allow(dead_code)]
mod bundle;

// Crate-private trial runner substrate (Phase 18 task 18.5).
#[allow(dead_code)]
mod runner;

// Crate-private external-process supervision (Phase 18 task 18.4). Owns the
// shared state machine used by the future AgentExecution and
// BenchmarkExecution adapters; never exposes OS primitives outside the crate.
#[allow(dead_code)]
mod process;

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
//! The library entry surface consists of the provisional [`cli`] and
//! [`experiment`] modules used by the same-package binary and integration
//! tests. This crate is unpublished, and those module APIs carry no
//! compatibility promise.

// `deny` (not `forbid`) because `process::tree` is this crate's single
// documented unsafe-FFI home: the OS tree-termination primitives (Unix
// process groups, Windows Job Objects) have no safe alternative. Every other
// module stays unsafe-free; `process::tree` overrides the lint locally and
// wraps each call in a safe API.
#![deny(unsafe_code)]

pub mod cli;

pub(crate) mod authority;
pub mod experiment;

// Crate-private failure-boundary codes (Phase 18 task 18.5). One stable
// owning-boundary code per failure table row; typed failures carry it.
mod failure;

// Crate-private benchmark integrity records (Phase 18 task 18.5.1). Owns
// revision admission, per-task validity classification, and reclassification
// identity; nothing an Agent, adapter, or LLM produces can reach it.
mod integrity;

// Crate-private pairing and comparability assembly (Phase 18 task 18.5.1).
// Consumes the frozen ResolvedExperiment and an admitted IntegrityRecord
// read-only; assembles exactly one baseline/candidate pair per
// edge-task-group only when every control fingerprint agrees.
mod comparison;

// Crate-private durable trial bundle (Phase 18 task 18.5). Owns canonical
// sealing, mutation rejection, and intent-before-effect persistence; the
// runner lifecycle and later regrade/report consumers stay inside this crate.
mod bundle;

// Crate-private trial runner substrate (Phase 18 task 18.5).
mod runner;

// Crate-private external-process supervision (Phase 18 task 18.4). Owns the
// shared state machine used by the future AgentExecution and
// BenchmarkExecution adapters; never exposes OS primitives outside the crate.
mod process;

/// Crate-private Agent contract and per-product adapters (Phase 18 task 18.6).
mod agent;

/// Crate-private benchmark execution contract and the Terminal-Bench 2.1
/// adapter (Phase 18 task 18.8).
mod benchmark;

/// Crate-private provisional trajectory and causal-span projection
/// (Phase 18 task 18.11).
mod trajectory;

// Crate-private offline recomputation contract (task 18.13). The regrade
// and report paths consume sealed assembled outputs only and stay behind
// the provisional CLI seam; they are deliberately not part of the public
// library surface until tasks 18.15 and 18.16.
pub(crate) mod regrade;

// Crate-private normalized report contract (task 18.13): the offline
// report path consumes sealed assembled outputs only, recomputes before
// rendering, and stays behind the provisional CLI seam.
pub(crate) mod report;

//! Crate-private benchmark integration module (Phase 18 task 18.8).
//!
//! `benchmark::process` owns the shared, benchmark-neutral execution
//! contract; `benchmark::terminal_bench_21` owns the Terminal-Bench 2.1
//! adapter. Neither is a public seam.

pub(crate) mod deepswe;
pub(crate) mod process;
pub(crate) mod terminal_bench_21;
pub(crate) mod terminal_bench_30;

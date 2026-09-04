//! Crate-private Agent contract and per-product process adapters (opi-eval
//! the Agent adapter contract).
//!
//! [`process::AgentExecution`] is the one shared N-harness contract: it drives
//! [`crate::process::ProcessSupervisor`] under the trial's limits and settles
//! exactly one [`process::AgentRecord`] per run. Product-specific launch and
//! evidence rules live in per-product adapters ([`opi::OpiProcessAdapter`],
//! [`pi::PiProcessAdapter`]); the adapters never link the products' runtimes —
//! they speak only argv, environment, native artifacts, and saved-bytes
//! schema identities.

pub(crate) mod opi;
pub(crate) mod pi;
pub(crate) mod process;

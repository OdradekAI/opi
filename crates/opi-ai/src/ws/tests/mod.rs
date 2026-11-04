//! Private WebSocket protocol and loopback Upgrade regression tests.

mod deflate_audit;

mod boundary;
mod close;
mod frame;
mod gaps;
mod io;
pub(super) mod logical_cancel_probe;
mod packaged_cases;
mod pmd;
mod pmd_async;
mod pmd_red;
mod replay;
mod upgrade;

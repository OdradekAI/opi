//! Private WebSocket protocol and loopback Upgrade regression tests.

mod deflate_audit;
#[allow(dead_code)]
mod tls_fixtures;

mod boundary;
mod close;
mod frame;
mod gaps;
mod io;
mod packaged_cases;
mod pmd;
mod pmd_async;
mod pmd_red;
mod replay;
mod upgrade;

//! Bedrock Converse adapter topic index.

use super::*;

#[path = "converse_adapter/support.rs"]
mod support;
pub(crate) use support::*;

#[path = "converse_adapter/lifecycle.rs"]
mod lifecycle;
#[path = "converse_adapter/projection.rs"]
mod projection;
#[path = "converse_adapter/replay.rs"]
mod replay;
#[path = "converse_adapter/stream.rs"]
mod stream;

//! AWS credential-source topic index.

use super::*;

#[path = "aws_credentials/support.rs"]
mod support;
pub(crate) use support::*;

#[path = "aws_credentials/chain.rs"]
mod chain;
#[path = "aws_credentials/lifecycle.rs"]
mod lifecycle;
#[path = "aws_credentials/metadata.rs"]
mod metadata;
#[path = "aws_credentials/role_sources.rs"]
mod role_sources;

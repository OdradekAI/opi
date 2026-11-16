//! Historical Bedrock path for the shared AWS SigV4 owner.
//!
//! The implementation lives under `auth::aws::sigv4` so cloud credential
//! consumers can select it without acquiring the Bedrock provider adapter.

pub use crate::auth::aws::sigv4::*;

//! Deterministic domain values, metadata and validation.
//!
//! These modules do not own network clients, credentials, callbacks, cancellation,
//! execution tasks or embedded catalogs. Per-call control lives with its operation.
pub mod catalog;
mod chat;
pub mod classifier;
pub mod deferred;
pub mod headers;
mod identity;
pub mod images;
pub mod message;
pub mod model;
pub mod replay;
pub mod tools;
pub mod transcript;
pub mod usage;
pub use chat::ChatModelRecord;
pub use identity::{ApiId, ApiKind};

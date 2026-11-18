//! Provider-independent transport owners.
//!
//! Protocol adapters use these owners for long-lived connection state while
//! provider profiles remain free of wire-specific request and stream policy.

#[cfg(feature = "provider-openai-codex-responses")]
pub(crate) mod codex;

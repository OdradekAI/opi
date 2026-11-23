//! Shared wire-level contracts and mappers used by multiple API adapters.
//!
//! [`headers`] and [`mod@env`] are public, protocol-neutral contracts shared by
//! API adapters. The crate-private `endpoint` and `text_decode` children are
//! low-level seams shared by API adapters; `text_decode` handles byte-level
//! streaming while `endpoint` joins already-resolved URLs. Other children own
//! message projection, stream decoding, wire accounting and request headers
//! shared by adapters serving the same service. Provider identity, endpoint
//! configuration and model catalogs remain in
//! `crate::providers`, while public native API types remain beside their owning
//! adapter under [`super`].

pub mod env;
pub mod headers;

#[cfg(any(
    feature = "provider-anthropic",
    feature = "provider-openai-chat",
    feature = "provider-openai-responses"
))]
pub(super) mod copilot_headers;

#[cfg(feature = "provider-protocol-common")]
pub(crate) mod endpoint;

#[cfg(feature = "provider-protocol-common")]
pub(crate) mod text_decode;

#[cfg(feature = "provider-openai-responses-shared")]
pub(crate) mod openai_responses;

#[cfg(feature = "provider-openrouter-usage")]
pub(crate) mod openrouter_usage;

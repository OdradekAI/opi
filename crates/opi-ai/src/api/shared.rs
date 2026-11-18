//! Shared wire-level contracts and mappers used by multiple API adapters.
//!
//! [`headers`] and [`mod@env`] are public, protocol-neutral contracts shared by
//! API adapters. The crate-private `endpoint` and `text_decode` children are
//! low-level seams shared by API adapters; `text_decode` handles byte-level
//! streaming while `endpoint` joins already-resolved URLs. Protocol-specific children contain
//! projection, stream decoding and wire-level accounting only. Provider
//! identity, endpoint configuration and model catalogs remain in
//! `crate::providers`, while public native API types remain beside their owning
//! adapter under [`super`].

pub mod env;
pub mod headers;

#[cfg(feature = "provider-protocol-common")]
pub(crate) mod endpoint;

#[cfg(feature = "provider-protocol-common")]
pub(crate) mod text_decode;

#[cfg(feature = "provider-openai-responses-shared")]
pub(crate) mod openai_responses;

#[cfg(feature = "provider-openrouter-usage")]
pub(crate) mod openrouter_usage;

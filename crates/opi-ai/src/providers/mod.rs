//! The builtin provider profile table.
//!
//! The 42 profiles describe canonical IDs, display names, endpoints,
//! credential sources and supported auth schemes. With the `builtins`
//! feature, `providers::collection` combines this table and the bundled
//! catalog with the wire adapters. The coding agent maps legacy provider
//! IDs to canonical IDs at its configuration boundary.

#[cfg(feature = "builtins")]
mod catalog_routes;
mod profiles;
pub use profiles::{
    BuiltinEndpoint, BuiltinProviderProfile, builtin_profile, builtin_provider_profiles,
};

#[cfg(feature = "provider-anthropic")]
pub mod anthropic;
#[cfg(feature = "provider-azure-openai")]
pub mod azure_openai;
#[cfg(feature = "provider-bedrock")]
pub mod bedrock;
#[cfg(feature = "builtins")]
pub mod collection;
#[cfg(feature = "provider-gemini")]
pub mod gemini;
pub mod mapped;
#[cfg(feature = "provider-mistral")]
pub mod mistral;
#[cfg(feature = "provider-openai-chat")]
pub mod openai;
#[cfg(feature = "provider-openai-codex-responses")]
pub mod openai_codex;
#[cfg(feature = "provider-openai-responses")]
pub mod openai_responses;
#[cfg(feature = "provider-openrouter")]
pub mod openrouter;
#[cfg(feature = "provider-pi-messages")]
pub mod pi_messages;
#[cfg(feature = "provider-vertex")]
pub mod vertex;

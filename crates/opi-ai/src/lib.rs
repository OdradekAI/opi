//! Unified multi-provider LLM API with streaming support.
//!
//! Provides a standardized interface for interacting with multiple LLM providers:
//! Anthropic, OpenAI Chat Completions, OpenAI Responses, Google Gemini, plus
//! OpenAI-compatible profiles for OpenRouter and Mistral.

pub mod anthropic;
pub mod api_mapped;
pub mod auth;
pub mod azure_openai;
pub mod bedrock;
pub mod config;
pub mod credential;
mod endpoint;
pub mod gemini;
pub mod http;
pub mod message;
pub mod mistral;
pub mod model;
pub mod model_info;
pub mod openai_chat;
pub mod openai_codex_responses;
pub mod openai_responses;
mod openai_responses_shared;
pub mod openrouter;
pub mod provider;
pub mod provider_collection;
pub mod provider_headers;
pub mod registry;
pub mod retry;
pub mod stream;
#[doc(hidden)]
pub mod test_support;
pub mod time;
pub mod vertex;

pub use api_mapped::{ApiMapError, ApiMappedProvider};
pub use auth::{
    AuthFallback, AuthInvalidPolicy, AuthProvenance, AuthProvenanceSource, AuthResolver,
    AuthScheme, AwsCredentialSource, AwsSigV4Credentials, LoginPresenter, OAuthCredential,
    OAuthLoginMethod, OAuthProvider, ResolvedAuth, StaticAuthResolver,
};
pub use config::{Config, Error};
pub use credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
pub use model::Model;
pub use model_info::{
    ModelCapabilities, ModelInfo, ModelInfoError, ModelPricing, PricingTier, ThinkingLevel,
    ThinkingLevelMap, ThinkingLevelMapping, WireApi, WireCompat,
};
pub use provider::Provider;
pub use provider_collection::{
    AuthDescriptor, AuthStatus, CollectionError, CompatMetadata, CompletedRequest,
    PreparedProviderCall, PreparedRoute, ProviderCollection, SecretKey,
};
pub use provider_headers::{ProviderHeaders, ProviderHeadersError};
pub use registry::{ProviderRegistry, RegistrationError, RegistryError};
pub use stream::AssistantStreamEvent;
pub use stream::{CostBreakdown, CumulativeUsage, Pricing, calculate_cost};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApiKind {
    Anthropic,
    OpenAi,
    Google,
    Mistral,
}

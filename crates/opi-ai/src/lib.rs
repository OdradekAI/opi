//! Unified multi-provider LLM API with streaming support.
//!
//! Provides a standardized interface for interacting with multiple LLM providers:
//! Anthropic, OpenAI Chat Completions, OpenAI Responses, Google Gemini, plus
//! OpenAI-compatible profiles for OpenRouter and Mistral.

pub mod anthropic;
pub mod api;
pub mod api_mapped;
pub mod auth;
pub mod azure_openai;
pub mod bedrock;
pub mod callbacks;
pub mod cancel;
pub mod catalog;
pub mod classifier;
pub mod codex_resources;
pub mod config;
pub mod constrained_sampling;
pub mod context;
pub mod credential;
mod endpoint;
// Private upstream crypto substrate (RDF-23): explicit-provider key
// admission/signing owner over unmodified crates.io rustls/aws-lc-rs.
// Consumed by the TLS host wiring (task 20.14) and real ADC callers (task
// 20.21.1); targeted allow until the first production consumer lands.
#[allow(dead_code)]
mod crypto;
pub mod gemini;
pub mod http;
pub mod identity;
pub mod images;
pub mod message;
pub mod mistral;
pub mod model;
pub mod model_info;
pub mod models;
pub mod mutation;
pub mod openai_chat;
pub mod openai_codex_responses;
pub mod openai_responses;
mod openai_responses_shared;
pub mod openrouter;
pub mod pi_messages;
pub mod provider;
pub mod provider_collection;
pub mod provider_env;
pub mod provider_headers;
pub mod providers;
pub mod registry;
pub mod replay;
pub mod retry;

pub use retry::retry_assistant_call;
pub mod schema;
pub mod stream;
pub mod telemetry;
#[doc(hidden)]
pub mod test_support;
mod text_decode;
pub mod time;
mod tls;
pub mod transcript;
pub mod vertex;
pub mod ws;

pub use api_mapped::{ApiMapError, ApiMappedProvider};
pub use auth::{
    AmbientAuthContext, AuthContext, AuthContextError, AuthFallback, AuthInvalidPolicy,
    AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, AwsCredentialSource,
    AwsSigV4Credentials, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
    ResolvedAuth, StaticAuthResolver,
};
pub use callbacks::{CallbackError, ORDINARY_PAYLOAD_VARIANT, RequestCallbacks};
pub use catalog::{DynamicCatalogSource, HttpCatalogSource, ModelsStore};
pub use config::{Config, Error};
pub use credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
pub use model::Model;
pub use model_info::{
    ModelCapabilities, ModelInfo, ModelInfoError, ModelPricing, PricingTier, SamplingParams,
    ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping, WireApi, WireCompat,
    calculate_cost_tiered, resolve_sampling_params,
};
pub use models::{
    ApiId, AuthCheck, AuthCheckKind, AuthDescriptor, AuthResolutionOverrides, ChatApi,
    ChatApiHandle, ChatCall, ChatModelRecord, CompatMetadata, Models, ModelsError, NativeOptions,
    PreparedProviderCall, PreparedRoute, RefreshOutcome, RefreshReport, RefreshReportEntry,
    RefreshStatus, SecretKey, SimpleOptions, SimpleToolChoice,
};
pub use mutation::{
    BarrierEntry, BarrierReport, CredentialMutationOp, CredentialRevision, CredentialUpdateOp,
    InMemoryCredentialStore, MutationAbandoned, MutationApplied, MutationBarrier, MutationDecision,
    MutationError, MutationGeneration, MutationHandle, MutationOutcome, MutationQueue,
    MutationReceipt, MutationSlot, MutationSlotView, delete_mutation, write_mutation,
};
pub use provider::Provider;
pub use provider_headers::{ProviderHeaders, ProviderHeadersError};
pub use registry::{ProviderRegistry, RegistrationError, RegistryError};
pub use stream::AssistantStreamEvent;
pub use stream::{
    CallFailure, ChatStream, CostBreakdown, CumulativeUsage, Pricing, ServiceTier, calculate_cost,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApiKind {
    Anthropic,
    OpenAi,
    Google,
    Mistral,
}

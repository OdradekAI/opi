//! Unified multi-provider LLM API with streaming support.
//!
//! Provides a standardized interface for interacting with multiple LLM providers:
//! Anthropic, OpenAI Chat Completions, OpenAI Responses, Google Gemini, plus
//! OpenAI-compatible profiles for OpenRouter and Mistral.

// A single provider owner intentionally leaves shared lifecycle helpers
// unreferenced. Keep warnings-denied validation strict for the compatibility
// group/default build while allowing those helpers in isolated owner builds.
#![cfg_attr(
    all(
        feature = "provider-protocol-common",
        not(feature = "protocol-adapters")
    ),
    allow(
        dead_code,
        unused_imports,
        unused_variables,
        unused_mut,
        unfulfilled_lint_expectations
    )
)]

#[cfg(feature = "provider-anthropic")]
pub mod anthropic;
pub mod api;
pub mod api_mapped;
pub mod auth;
#[cfg(feature = "provider-azure-openai")]
pub mod azure_openai;
#[cfg(feature = "provider-bedrock")]
pub mod bedrock;
pub mod callbacks;
pub mod cancel;
pub mod catalog;
pub mod classifier;
pub mod codex_resources;
pub mod constrained_sampling;
pub mod context;
pub use auth::credential;
pub mod deferred;
#[cfg(feature = "provider-protocol-common")]
mod endpoint;
mod execution;
// Private upstream crypto substrate (RDF-23): explicit-provider key
// admission/signing owner over unmodified crates.io rustls/aws-lc-rs.
// Consumed by the TLS host wiring (task 20.14) and real ADC callers (task
// 20.21.1); targeted allow until the first production consumer lands.
#[allow(dead_code)]
#[cfg(feature = "tls-backend")]
mod crypto;
#[cfg(feature = "provider-gemini")]
pub mod gemini;
#[cfg(feature = "http-transport")]
pub mod http;
pub mod identity;
pub mod images;
pub use types::message;
#[cfg(feature = "provider-mistral")]
pub mod mistral;
pub use types::model as model_info;
pub mod models;
pub use auth::mutation;
#[cfg(feature = "provider-openai-chat")]
pub mod openai_chat;
#[cfg(feature = "provider-openai-codex-responses")]
pub mod openai_codex_responses;
#[cfg(feature = "provider-openai-responses")]
pub mod openai_responses;
#[cfg(feature = "provider-openai-responses-shared")]
mod openai_responses_shared;
#[cfg(feature = "provider-openrouter")]
pub mod openrouter;
#[cfg(feature = "provider-openrouter-usage")]
mod openrouter_usage;
#[cfg(feature = "provider-pi-messages")]
pub mod pi_messages;
pub mod provider;
#[cfg(feature = "builtins")]
pub mod provider_collection;
pub mod provider_env;
pub mod provider_headers;
#[cfg(feature = "provider-protocol-common")]
pub mod providers;
pub mod registry;
pub mod replay;
pub mod retry;

pub use retry::retry_assistant_call;
pub mod schema;
pub mod stream;
pub mod streaming_json;
pub mod telemetry;
#[doc(hidden)]
pub mod test_support;
#[cfg(feature = "provider-protocol-common")]
mod text_decode;
pub mod time;
#[cfg(feature = "tls-backend")]
pub mod tls;
pub mod transcript;
#[cfg(feature = "provider-vertex")]
pub mod vertex;
#[cfg(feature = "websocket-compression")]
pub mod ws;

pub use api_mapped::{ApiMapError, ApiMappedProvider};
pub use auth::{
    AmbientAuthContext, AuthContext, AuthContextError, AuthFallback, AuthInvalidPolicy,
    AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, AwsCredentialSource,
    AwsSigV4Credentials, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
    ResolvedAuth, StaticAuthResolver,
};
pub use callbacks::{CallbackError, ORDINARY_PAYLOAD_VARIANT, RequestCallbacks};
#[cfg(feature = "provider-protocol-common")]
pub use catalog::HttpCatalogSource;
pub use catalog::{CatalogRefreshContext, DynamicCatalogSource, ModelsStore};
pub use credential::{
    ApiKeyCredential, BoxAuthFuture, Credential, CredentialSource, CredentialStore,
    CredentialStoreError,
};
pub use model_info::{
    ModelCapabilities, ModelInfo, ModelInfoError, ModelPricing, PricingTier, SamplingParams,
    ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping, WireApi, WireCompat,
    calculate_cost_tiered, resolve_sampling_params,
};
pub use models::{
    ApiId, AuthCheck, AuthCheckKind, AuthDescriptor, AuthResolutionOverrides, ChatApi,
    ChatApiHandle, ChatCall, ChatModelRecord, CompatMetadata, Models, ModelsError,
    ModelsRefreshOptions, NativeOptions, PreparedProviderCall, PreparedRoute, RefreshOutcome,
    RefreshReport, RefreshReportEntry, RefreshStatus, SecretKey, SimpleOptions, SimpleToolChoice,
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
    CallFailure, ChatStream, CostBreakdown, CumulativeUsage, Pricing, ServiceTier,
    StartedChatStream, calculate_cost,
};

pub mod errors;
pub mod types;
pub use types::ApiKind;

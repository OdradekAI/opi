//! Unified multi-provider LLM API with streaming support.
//!
//! Provides provider-neutral chat, image, classifier and deferred-operation
//! contracts, built-in provider profiles and explicit Models composition. Wire
//! adapters, authentication, catalogs and transports are independently
//! selectable through Cargo features.

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

pub mod api;
pub mod auth;
pub mod callbacks;
pub mod cancel;
pub mod catalog;
pub mod classifier;
pub mod codex_resources;
pub mod constrained_sampling;
pub mod context;
pub use auth::credential;
mod execution;
// Private upstream crypto substrate (RDF-23): explicit-provider key
// admission/signing owner over unmodified crates.io rustls/aws-lc-rs.
// Consumed by the TLS host wiring (task 20.14) and real ADC callers (task
// 20.21.1); targeted allow until the first production consumer lands.
#[allow(dead_code)]
#[cfg(feature = "tls-backend")]
mod crypto;
#[cfg(feature = "http-transport")]
pub mod http;
pub mod identity;
pub mod images;
pub use types::message;
pub use types::model as model_info;
pub mod models;
pub use auth::mutation;
pub mod provider;
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
pub mod test_support;
pub mod time;
#[cfg(feature = "tls-backend")]
pub mod tls;
pub mod transcript;
#[cfg(feature = "provider-openai-codex-responses")]
pub(crate) mod transport;
#[cfg(feature = "websocket-compression")]
pub mod ws;

pub use api::contract::{
    ChatApi, ChatApiHandle, DeferredApi, DeferredApiHandle, DeferredCancelCall, DeferredFetchCall,
    NativeOptions, SimpleOptions, SimpleToolChoice,
};
pub use api::shared::headers::{
    ProviderHeaders, ProviderHeadersError, merge_provider_header_sources,
};
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
    AuthCheck, AuthCheckKind, AuthDescriptor, AuthResolutionOverrides, ChatCall, ChatModelRecord,
    CompatMetadata, Models, ModelsError, ModelsRefreshOptions, PreparedProviderCall, PreparedRoute,
    RefreshOutcome, RefreshReport, RefreshReportEntry, RefreshStatus, SecretKey,
};
pub use mutation::{
    BarrierEntry, BarrierReport, CredentialMutationOp, CredentialRevision, CredentialUpdateOp,
    InMemoryCredentialStore, MutationAbandoned, MutationApplied, MutationBarrier, MutationDecision,
    MutationError, MutationGeneration, MutationHandle, MutationOutcome, MutationQueue,
    MutationReceipt, MutationSlot, MutationSlotView, delete_mutation, write_mutation,
};
pub use provider::Provider;
pub use registry::{ProviderRegistry, RegistrationError, RegistryError};
pub use stream::AssistantStreamEvent;
pub use stream::{
    CallFailure, ChatStream, CostBreakdown, CumulativeUsage, Pricing, ServiceTier,
    StartedChatStream, calculate_cost,
};
pub use types::ApiId;
pub use types::deferred::{DeferredError, DeferredHandle, RouteMismatchDetail};

pub mod errors;
pub mod types;
pub use types::ApiKind;

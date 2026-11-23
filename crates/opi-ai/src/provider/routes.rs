//! Provider-neutral route contexts consumed by protocol adapters.
//!
//! A route context carries identity, catalog, endpoint and injected transport
//! state. Concrete provider profiles may expose aliases and constructors, but
//! API adapters depend on these lower contract values instead of profile types.

#[cfg(feature = "provider-protocol-common")]
use std::sync::Arc;

#[cfg(any(feature = "provider-gemini", feature = "provider-bedrock"))]
use crate::http::HttpClient;
#[cfg(any(feature = "provider-gemini", feature = "provider-openai-chat"))]
use crate::model_info::WireApi;
#[cfg(feature = "provider-protocol-common")]
use crate::provider::ModelInfo;

/// Route state shared by Google Gemini-compatible wire adapters.
#[cfg(feature = "provider-gemini")]
#[derive(Clone)]
pub struct GeminiRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) uses_default_catalog: bool,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) wire: WireApi,
}

/// Route state owned by Google Vertex AI and consumed by its wire adapter.
#[cfg(feature = "provider-vertex")]
#[derive(Clone)]
pub struct VertexRouteContext {
    pub(crate) project: String,
    pub(crate) location: String,
    pub(crate) base_url: Option<String>,
    pub(crate) inner: GeminiRouteContext,
    pub(crate) express: bool,
}

/// Route state owned by the AWS Bedrock Converse adapter.
#[cfg(feature = "provider-bedrock")]
#[derive(Clone)]
pub struct BedrockRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) base_url: Option<String>,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) placeholder_client: Option<Arc<HttpClient>>,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) retry: crate::retry::smithy::SmithyRetryConfig,
}

/// Route state shared by OpenAI Chat Completions-compatible profiles.
#[cfg(feature = "provider-openai-chat")]
#[derive(Clone)]
pub struct OpenAiRouteContext {
    pub(crate) wire: WireApi,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) provider_id: String,
    pub(crate) wire_model_override: Option<String>,
    pub(crate) headers: crate::types::headers::ProviderHeaders,
    pub(crate) route_headers: Vec<(String, String)>,
    pub(crate) client: Arc<crate::http::HttpClient>,
}

/// A configured OpenAI Chat route pairing neutral route state with the
/// provider-selected policy contract consumed by the wire adapter.
#[cfg(feature = "provider-openai-chat")]
#[derive(Clone)]
pub struct OpenAiConfiguredRoute {
    pub(crate) route: OpenAiRouteContext,
    pub(crate) policy: crate::provider::policies::OpenAiRoutePolicy,
}

/// Route state shared by the native Mistral Conversations adapter.
#[cfg(feature = "provider-mistral")]
#[derive(Clone)]
pub struct MistralRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) models: Vec<ModelInfo>,
}

/// Route state shared by the Pi Messages adapter.
#[cfg(feature = "provider-pi-messages")]
#[derive(Clone)]
pub struct PiMessagesRouteContext {
    pub(crate) base_url: String,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) models: Vec<ModelInfo>,
}

/// Route state shared by the Anthropic Messages adapter.
#[cfg(feature = "provider-anthropic")]
#[derive(Clone)]
pub struct AnthropicRouteContext {
    pub(crate) provider_id: String,
    pub(crate) default_base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) headers: crate::types::headers::ProviderHeaders,
    pub(crate) auth_invalid_policy: crate::auth::AuthInvalidPolicy,
    pub(crate) direct_oauth_beta: bool,
    pub(crate) copilot_headers: bool,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) federation: crate::auth::anthropic_federation::FederationCache,
}

/// Route state shared by OpenAI Responses-compatible adapters.
#[cfg(feature = "provider-openai-responses")]
#[derive(Clone)]
pub struct OpenAiResponsesRouteContext {
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) config: crate::provider::policies::ResponsesConfig,
    pub(crate) provider_id: String,
    pub(crate) headers: crate::types::headers::ProviderHeaders,
    pub(crate) route_headers: Vec<(String, String)>,
    pub(crate) client: Arc<crate::http::HttpClient>,
}

/// A configured OpenAI Responses route pairing neutral route state with the
/// provider-selected policy contract consumed by the wire adapter.
#[cfg(feature = "provider-openai-responses")]
#[derive(Clone)]
pub struct OpenAiResponsesConfiguredRoute {
    pub(crate) route: OpenAiResponsesRouteContext,
    pub(crate) policy: crate::provider::policies::OpenAiResponsesRoutePolicy,
}

/// Route state shared by Azure's Chat and Responses wire adapters.
#[cfg(feature = "provider-azure-openai")]
#[derive(Clone)]
pub struct AzureRouteContext {
    pub(crate) endpoint: String,
    pub(crate) api_version: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: OpenAiConfiguredRoute,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) deployment_map: Option<String>,
}

/// Route state shared by the subscription-specific Codex Responses adapters.
#[cfg(feature = "provider-openai-codex-responses")]
#[derive(Clone)]
pub struct OpenAiCodexRouteContext {
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) ws_state: Arc<crate::transport::codex::WsTransportState>,
}

#[cfg(feature = "provider-openai-codex-responses")]
pub const OPENAI_CODEX_PROVIDER_ID: &str = "openai-codex";

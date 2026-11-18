//! Provider-neutral route contexts consumed by protocol adapters.
//!
//! A route context carries identity, catalog, endpoint and injected transport
//! state. Concrete provider profiles may expose aliases and constructors, but
//! API adapters depend on these lower contract values instead of profile types.

#[cfg(any(feature = "provider-gemini", feature = "provider-bedrock"))]
use std::sync::Arc;

#[cfg(any(feature = "provider-gemini", feature = "provider-bedrock"))]
use crate::http::HttpClient;
#[cfg(any(feature = "provider-gemini", feature = "provider-bedrock"))]
use crate::model_info::WireApi;
#[cfg(any(feature = "provider-gemini", feature = "provider-bedrock"))]
use crate::provider::ModelInfo;

/// Route state shared by Google Gemini-compatible wire adapters.
#[cfg(feature = "provider-gemini")]
#[derive(Clone)]
pub struct GeminiRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) wire: WireApi,
}

/// Route state owned by Google Vertex AI and consumed by its wire adapter.
#[cfg(feature = "provider-vertex")]
#[derive(Clone)]
pub struct VertexRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) project: String,
    pub(crate) location: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: GeminiRouteContext,
    pub(crate) client: Arc<HttpClient>,
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

/// Whether `base_url` is a standard AWS Bedrock runtime endpoint.
#[cfg(feature = "provider-bedrock")]
pub fn is_standard_bedrock_endpoint(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    let Some(rest) = host.strip_prefix("bedrock-runtime") else {
        return false;
    };
    let rest = rest.strip_prefix("-fips").unwrap_or(rest);
    let Some(rest) = rest.strip_prefix('.') else {
        return false;
    };
    let Some((region, suffix)) = rest.split_once('.') else {
        return false;
    };
    !region.is_empty()
        && !region.contains('.')
        && (suffix == "amazonaws.com" || suffix == "amazonaws.com.cn")
}

/// Reject the explicit placeholder-signing mode for standard AWS endpoints.
#[cfg(feature = "provider-bedrock")]
pub fn validate_proxy_placeholder_endpoint(
    base_url: &str,
) -> Result<(), crate::provider::ProviderError> {
    if is_standard_bedrock_endpoint(base_url) {
        return Err(crate::provider::ProviderError::Config(
            crate::provider::ProviderErrorSummary::sanitized(
                "Bedrock ProxyPlaceholderSigning requires a caller-supplied nonstandard endpoint; \
                 standard AWS endpoints reject this mode",
            ),
        ));
    }
    Ok(())
}

/// Per-model OpenAI-compatible compatibility override.
#[cfg(feature = "provider-openai-chat")]
#[derive(Debug, Clone, Default)]
pub struct OpenAiModelCompatOverride {
    pub instruction_role: Option<crate::model_info::ChatInstructionRole>,
    pub store: Option<crate::model_info::ChatStoreField>,
    pub thinking_format: Option<crate::model_info::ChatThinkingFormat>,
    pub max_tokens_field: Option<String>,
}

/// Route state shared by OpenAI Chat Completions-compatible profiles.
#[cfg(feature = "provider-openai-chat")]
#[derive(Clone)]
pub struct OpenAiRouteContext {
    pub(crate) wire: WireApi,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) compat: crate::model_info::OpenAiCompletionsCompat,
    pub(crate) model_overrides: std::collections::HashMap<String, OpenAiModelCompatOverride>,
    pub(crate) provider_id: String,
    pub(crate) wire_model_override: Option<String>,
    pub(crate) headers: crate::types::headers::ProviderHeaders,
    pub(crate) route_headers: Vec<(String, String)>,
    pub(crate) catalog_compat: bool,
    pub(crate) auth_invalid_policy: crate::auth::AuthInvalidPolicy,
    pub(crate) direct_prompt_cache_key: bool,
    pub(crate) copilot_initiator: bool,
    pub(crate) client: Arc<crate::http::HttpClient>,
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

/// Native OpenAI Responses route configuration.
#[cfg(feature = "provider-openai-responses")]
#[derive(Debug, Clone)]
pub struct ResponsesConfig {
    pub source: crate::model_info::OpenAiResponsesSourceCompat,
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub send_session_id_header: bool,
}

#[cfg(feature = "provider-openai-responses")]
impl Default for ResponsesConfig {
    fn default() -> Self {
        Self {
            source: crate::model_info::OpenAiResponsesCompat::default().source,
            store: None,
            reasoning_effort: None,
            send_session_id_header: true,
        }
    }
}

/// Construction-time affinity policy for the reusable Responses route.
#[cfg(feature = "provider-openai-responses")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesAffinityPolicy {
    Direct,
    CustomDisabled,
    CustomOptIn,
}

/// Route state shared by OpenAI Responses-compatible adapters.
#[cfg(feature = "provider-openai-responses")]
#[derive(Clone)]
pub struct OpenAiResponsesRouteContext {
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) config: ResponsesConfig,
    pub(crate) provider_id: String,
    pub(crate) headers: crate::types::headers::ProviderHeaders,
    pub(crate) route_headers: Vec<(String, String)>,
    pub(crate) catalog_compat: bool,
    pub(crate) auth_invalid_policy: crate::auth::AuthInvalidPolicy,
    pub(crate) affinity_policy: ResponsesAffinityPolicy,
    pub(crate) copilot_headers: bool,
    pub(crate) client: Arc<crate::http::HttpClient>,
    pub(crate) chatgpt_signin: Option<bool>,
}

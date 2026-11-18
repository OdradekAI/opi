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

/// Route state shared by Azure's Chat and Responses wire adapters.
#[cfg(feature = "provider-azure-openai")]
#[derive(Clone)]
pub struct AzureRouteContext {
    pub(crate) endpoint: String,
    pub(crate) api_version: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: OpenAiRouteContext,
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

#[cfg(feature = "provider-azure-openai")]
const AZURE_API_VERSION: &str = "v1";
#[cfg(feature = "provider-azure-openai")]
const AZURE_HOST_SUFFIXES: [&str; 3] = [
    ".openai.azure.com",
    ".cognitiveservices.azure.com",
    ".ai.azure.com",
];
#[cfg(feature = "provider-azure-openai")]
const AZURE_ROOT_PATHS: [&str; 5] = ["", "/", "/openai", "/openai/v1", "/openai/v1/responses"];

#[cfg(feature = "provider-azure-openai")]
fn azure_config_error(message: &'static str) -> crate::provider::ProviderError {
    crate::provider::ProviderError::Config(crate::provider::ProviderErrorSummary::attested_static(
        message,
    ))
}

/// Resolve and normalize an Azure route base URL.
#[cfg(feature = "provider-azure-openai")]
pub fn resolve_azure_base_url(
    explicit_base_url: Option<&str>,
    resource_name: Option<&str>,
    model_base_url: Option<&str>,
) -> Result<String, crate::provider::ProviderError> {
    let explicit = explicit_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let resolved = explicit
        .map(str::to_owned)
        .or_else(|| {
            resource_name
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|resource| format!("https://{resource}.openai.azure.com"))
        })
        .or_else(|| {
            model_base_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            azure_config_error(
                "Azure OpenAI base URL is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var.",
            )
        })?;
    normalize_azure_base_url(&resolved)
}

#[cfg(feature = "provider-azure-openai")]
fn normalize_azure_base_url(base_url: &str) -> Result<String, crate::provider::ProviderError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(azure_config_error(
            "Azure OpenAI base URL must not be empty.",
        ));
    }
    let url = url::Url::parse(trimmed).map_err(|_| {
        crate::provider::ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
            format!("Invalid Azure OpenAI base URL: {trimmed}"),
        ))
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(azure_config_error(
            "Azure OpenAI requires an absolute HTTP or HTTPS API base URL.",
        ));
    }
    let host = url.host_str().unwrap_or_default();
    if !AZURE_HOST_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix))
    {
        return Ok(trimmed.to_owned());
    }
    let path = url.path().trim_end_matches('/');
    if AZURE_ROOT_PATHS.contains(&path) {
        return Ok(format!("{}://{host}", url.scheme()));
    }
    Ok(trimmed.to_owned())
}

#[cfg(feature = "provider-azure-openai")]
pub(crate) fn responses_url(endpoint: &str, api_version: &str) -> String {
    let mut url = url::Url::parse(endpoint).expect("Azure endpoint was validated at resolution");
    let path = url.path().trim_end_matches('/');
    let standard_resource = url.host_str().is_some_and(|host| {
        AZURE_HOST_SUFFIXES
            .iter()
            .any(|suffix| host.ends_with(suffix))
    }) && AZURE_ROOT_PATHS.contains(&path);
    let path = if standard_resource {
        "/openai/v1/responses".to_owned()
    } else if path.ends_with("/responses") {
        path.to_owned()
    } else {
        format!("{path}/responses")
    };
    url.set_path(&path);
    url.query_pairs_mut()
        .append_pair("api-version", api_version);
    url.into()
}

#[cfg(feature = "provider-azure-openai")]
pub fn resolve_azure_api_version(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(AZURE_API_VERSION)
        .to_owned()
}

#[cfg(feature = "provider-azure-openai")]
pub(crate) fn parse_deployment_map(
    value: Option<&str>,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Some(value) = value else { return map };
    for entry in value.split(',') {
        let Some((model_id, deployment)) = entry.trim().split_once('=') else {
            continue;
        };
        let (model_id, deployment) = (model_id.trim(), deployment.trim());
        if model_id.is_empty() || deployment.is_empty() {
            continue;
        }
        map.insert(model_id.to_owned(), deployment.to_owned());
    }
    map
}

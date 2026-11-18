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

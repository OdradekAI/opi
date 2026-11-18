//! Anthropic Provider profile.
//!
//! Endpoint, model catalog and route configuration live here. Anthropic
//! Messages framing, request projection and transport live in the canonical
//! wire adapter; this profile does not retain request-scoped wire options.

use std::sync::Arc;

use crate::auth::AuthInvalidPolicy;
use crate::http::HttpClient;
use crate::model_info::{AnthropicMessagesCompat, WireApi, WireCompat};
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;
use crate::types::headers::ProviderHeaders;

/// Built-in Anthropic model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "claude-sonnet-4-5-20250514",
            "Claude Sonnet 4.5",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
        ModelInfo::new(
            "claude-opus-4-20250514",
            "Claude Opus 4",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
        ModelInfo::new(
            "claude-haiku-4-5-20250514",
            "Claude Haiku 4.5",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true)
                .with_cache_control(true)
                .with_long_cache_retention(true),
        ),
    ]
    .into_iter()
    .map(|model| {
        model
            .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
                supports_eager_tool_input_streaming: false,
                force_adaptive_thinking: false,
                supports_temperature: true,
                supports_strict_tools: false,
                supports_mid_convo_system_messages: false,
                supports_mid_convo_tool_changes: false,
                ..Default::default()
            }))
            .expect("built-in Anthropic compatibility uses the matching wire")
    })
    .collect()
}

/// Anthropic Messages Provider profile.
#[derive(Clone)]
pub struct AnthropicProvider {
    pub(crate) provider_id: String,
    pub(crate) default_base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) headers: ProviderHeaders,
    pub(crate) auth_invalid_policy: AuthInvalidPolicy,
    pub(crate) direct_oauth_beta: bool,
    pub(crate) copilot_headers: bool,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) federation: crate::auth::anthropic_federation::FederationCache,
}

impl AnthropicProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client for connection pooling.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        Self::for_route(
            "anthropic".into(),
            model_catalog(),
            base_url,
            ProviderHeaders::default(),
            client,
            false,
        )
    }

    /// Build an Anthropic Messages route for a mapped provider.
    #[allow(clippy::too_many_arguments)]
    pub fn for_route(
        provider_id: String,
        models: Vec<ModelInfo>,
        default_base_url: Option<String>,
        headers: ProviderHeaders,
        client: Arc<HttpClient>,
        direct_oauth_beta: bool,
    ) -> Self {
        let default_base_url =
            default_base_url.unwrap_or_else(|| "https://api.anthropic.com".into());
        Self {
            provider_id,
            default_base_url,
            models,
            headers,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            direct_oauth_beta,
            copilot_headers: false,
            client,
            federation: Default::default(),
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Enable GitHub Copilot's request-dependent route headers.
    pub fn with_copilot_headers(mut self) -> Self {
        self.copilot_headers = true;
        self
    }

    /// Access the shared HTTP client (for testing client reuse).
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }
}

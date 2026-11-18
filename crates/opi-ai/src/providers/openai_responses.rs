//! OpenAI Responses Provider profile.
//!
//! This module owns OpenAI Responses identity, endpoint, model catalog,
//! compatibility configuration, shared HTTP client and per-dispatch state.
//! Wire serialization, transport and stream adapters live in
//! [`crate::api::openai_responses`].

use std::sync::Arc;

use crate::api::shared::headers::ProviderHeaders;
use crate::auth::AuthInvalidPolicy;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// Native OpenAI Responses request flags.
#[derive(Debug, Clone)]
pub struct ResponsesConfig {
    pub source: crate::model_info::OpenAiResponsesSourceCompat,
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub send_session_id_header: bool,
}
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

/// Construction-time session-affinity policy for the reusable Responses wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesAffinityPolicy {
    /// Built-in direct Responses: cache key and request id are automatic.
    Direct,
    /// Custom/proxy Responses without the reviewed affinity opt-in.
    CustomDisabled,
    /// Custom/proxy Responses with the reviewed full affinity mapping enabled.
    CustomOptIn,
}

/// Standard OpenAI Responses provider, also used as a route by mapped
/// providers whose model metadata declares the standard Responses wire.
#[derive(Clone)]
pub struct OpenAiResponsesProvider {
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) config: ResponsesConfig,
    pub(crate) provider_id: String,
    pub(crate) headers: ProviderHeaders,
    pub(crate) route_headers: Vec<(String, String)>,
    pub(crate) catalog_compat: bool,
    pub(crate) auth_invalid_policy: AuthInvalidPolicy,
    pub(crate) affinity_policy: ResponsesAffinityPolicy,
    pub(crate) copilot_headers: bool,
    pub(crate) client: Arc<HttpClient>,
    /// Frozen secret-free classification for this prepared call.
    pub(crate) chatgpt_signin: Option<bool>,
}

/// Built-in OpenAI Responses model metadata.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(128_000, 16_384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gpt-4o-mini",
            "GPT-4o Mini",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(128_000, 16_384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "o3",
            "o3",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(200_000, 100_000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "o4-mini",
            "o4-mini",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(200_000, 100_000)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}
impl OpenAiResponsesProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        Self::with_auth(base_url, ResponsesConfig::default(), client)
            .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Create with native Responses request flags.
    pub fn new_with_config(base_url: Option<String>, config: ResponsesConfig) -> Self {
        Self::with_auth(base_url, config, Arc::new(HttpClient::new()))
            .with_auth_invalid_policy(AuthInvalidPolicy::Static)
    }

    /// Build with provider route config. The secret arrives per-call via
    /// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared`.
    pub fn with_auth(
        base_url: Option<String>,
        config: ResponsesConfig,
        client: Arc<HttpClient>,
    ) -> Self {
        Self::with_auth_extra(
            base_url,
            config,
            "openai-responses".into(),
            Vec::new(),
            client,
        )
        .with_auth_invalid_policy(AuthInvalidPolicy::CredentialManaged)
        .with_affinity_policy(ResponsesAffinityPolicy::Direct)
    }

    /// Build with provider id and route headers.
    pub fn with_auth_extra(
        base_url: Option<String>,
        config: ResponsesConfig,
        provider_id: String,
        extra_headers: Vec<(String, String)>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models: model_catalog()
                .into_iter()
                .map(|mut model| {
                    if let crate::WireCompat::OpenAiResponses(compat) = &mut model.compat {
                        compat.source = config.source.clone();
                    }
                    model
                })
                .collect(),
            config,
            provider_id,
            headers: ProviderHeaders::default(),
            route_headers: extra_headers,
            catalog_compat: false,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            affinity_policy: ResponsesAffinityPolicy::CustomDisabled,
            copilot_headers: false,
            client,
            chatgpt_signin: None,
        }
    }

    /// Clone with the selected model for one call without mutating the
    /// registered provider. Wire-native options stay owned by the API
    /// adapter and are passed explicitly at compilation time.
    pub(crate) fn dispatch_configured(&self, model: ModelInfo) -> Self {
        let mut provider = self.clone();
        provider.models = vec![model];
        provider.catalog_compat = true;
        provider
    }

    /// Build a standard Responses route for a mapped provider.
    pub fn for_route(
        default_base_url: Option<String>,
        provider_id: String,
        headers: ProviderHeaders,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            base_url: default_base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            config: ResponsesConfig::default(),
            provider_id,
            headers,
            route_headers: Vec::new(),
            catalog_compat: true,
            auth_invalid_policy: AuthInvalidPolicy::Static,
            affinity_policy: ResponsesAffinityPolicy::CustomOptIn,
            copilot_headers: false,
            client,
            chatgpt_signin: None,
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.auth_invalid_policy = policy;
        self
    }

    /// Override direct-versus-custom session-affinity behavior.
    pub fn with_affinity_policy(mut self, policy: ResponsesAffinityPolicy) -> Self {
        self.affinity_policy = policy;
        self
    }

    /// Enable GitHub Copilot's request-dependent route headers.
    pub fn with_copilot_headers(mut self) -> Self {
        self.copilot_headers = true;
        self
    }
}

impl OpenAiResponsesProvider {
    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }
}

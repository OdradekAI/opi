//! OpenAI Responses Provider profile.
//!
//! This module owns OpenAI Responses identity, endpoint, model catalog,
//! compatibility configuration, shared HTTP client and per-dispatch state.
//! Wire serialization, transport and stream adapters live in
//! [`crate::api::openai_responses`].

use std::sync::{Arc, OnceLock};

use crate::auth::AuthInvalidPolicy;
use crate::catalog::{CatalogModel, CatalogOperation};
use crate::http::HttpClient;
use crate::provider::ModelInfo;
use crate::provider::policies::OpenAiResponsesRoutePolicy;
use crate::provider::routes::{OpenAiResponsesConfiguredRoute, OpenAiResponsesRouteContext};
use crate::types::headers::ProviderHeaders;

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

pub use crate::provider::policies::{ResponsesAffinityPolicy, ResponsesConfig};

/// Canonical provider-facing name for the neutral Responses route context.
pub type OpenAiResponsesProvider = OpenAiResponsesConfiguredRoute;

/// Chat metadata from the reviewed OpenAI Responses catalog.
pub fn model_catalog() -> Vec<ModelInfo> {
    static MODELS: OnceLock<Vec<ModelInfo>> = OnceLock::new();
    MODELS
        .get_or_init(|| {
            let entries: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
                "../../resources/catalog/providers/openai.json"
            ))
            .expect("validated OpenAI provider catalog");
            entries
                .iter()
                .map(|entry| CatalogModel::parse("openai", entry).expect("validated OpenAI record"))
                .filter(|entry| {
                    entry.operation == CatalogOperation::Chat && entry.api == "openai-responses"
                })
                .map(|entry| {
                    entry
                        .to_chat_model_info()
                        .expect("validated OpenAI Responses chat record")
                })
                .collect()
        })
        .clone()
}
impl OpenAiResponsesConfiguredRoute {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        Self::with_builtin_catalog(base_url, "openai-responses".into(), client)
    }

    /// Build a named OpenAI Responses route with the reviewed chat catalog.
    /// A configured base can be an origin or an API prefix ending in `/v1`.
    pub fn with_builtin_catalog(
        base_url: Option<String>,
        provider_id: String,
        client: Arc<HttpClient>,
    ) -> Self {
        let mut models = model_catalog();
        let origin = base_url.map(|base| {
            let base = base.trim_end_matches('/');
            let origin = base.strip_suffix("/v1").unwrap_or(base);
            let api_base = format!("{origin}/v1");
            for model in &mut models {
                model.base_url = Some(api_base.clone());
            }
            origin.to_owned()
        });
        Self::for_route(
            origin,
            provider_id,
            ProviderHeaders::default(),
            models,
            client,
        )
        .with_affinity_policy(ResponsesAffinityPolicy::Direct)
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
            route: OpenAiResponsesRouteContext {
                base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
                models: model_catalog()
                    .into_iter()
                    .map(|mut model| {
                        // Explicit protocol configuration supplies the endpoint.
                        model.base_url = None;
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
                client,
            },
            policy: OpenAiResponsesRoutePolicy {
                catalog_compat: false,
                auth_invalid_policy: AuthInvalidPolicy::Static,
                affinity_policy: ResponsesAffinityPolicy::CustomDisabled,
                copilot_headers: false,
                chatgpt_signin: None,
            },
        }
    }

    /// Clone with the selected model for one call without mutating the
    /// registered provider. Wire-native options stay owned by the API
    /// adapter and are passed explicitly at compilation time.
    pub(crate) fn dispatch_configured(&self, model: ModelInfo) -> Self {
        let mut provider = self.clone();
        provider.route.models = vec![model];
        provider.policy.catalog_compat = true;
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
            route: OpenAiResponsesRouteContext {
                base_url: default_base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
                models,
                config: ResponsesConfig::default(),
                provider_id,
                headers,
                route_headers: Vec::new(),
                client,
            },
            policy: OpenAiResponsesRoutePolicy {
                catalog_compat: true,
                auth_invalid_policy: AuthInvalidPolicy::Static,
                affinity_policy: ResponsesAffinityPolicy::CustomOptIn,
                copilot_headers: false,
                chatgpt_signin: None,
            },
        }
    }

    /// Override how this route classifies provider 401/403 responses.
    pub fn with_auth_invalid_policy(mut self, policy: AuthInvalidPolicy) -> Self {
        self.policy.auth_invalid_policy = policy;
        self
    }

    /// Override direct-versus-custom session-affinity behavior.
    pub fn with_affinity_policy(mut self, policy: ResponsesAffinityPolicy) -> Self {
        self.policy.affinity_policy = policy;
        self
    }

    /// Enable GitHub Copilot's request-dependent route headers.
    pub fn with_copilot_headers(mut self) -> Self {
        self.policy.copilot_headers = true;
        self
    }
}

impl OpenAiResponsesConfiguredRoute {
    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.route.client
    }
}

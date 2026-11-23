//! Google Gemini provider profile.
//!
//! Endpoint, model catalog, route identity, wire selection, and dispatch
//! configuration live here. Gemini request framing and transport live in
//! [`crate::api::gemini`].

use std::sync::{Arc, OnceLock};

use crate::catalog::CatalogModel;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::provider::routes::GeminiRouteContext;

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Canonical provider-facing name for the neutral Gemini route context.
pub type GeminiProvider = GeminiRouteContext;

/// Chat metadata from the reviewed Google catalog.
pub fn model_catalog() -> Vec<ModelInfo> {
    static MODELS: OnceLock<Vec<ModelInfo>> = OnceLock::new();
    MODELS
        .get_or_init(|| {
            let entries: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
                "../../resources/catalog/providers/google.json"
            ))
            .expect("validated Google provider catalog");
            entries
                .iter()
                .map(|entry| {
                    CatalogModel::parse("google", entry)
                        .and_then(|row| row.to_chat_model_info())
                        .expect("validated Google chat record")
                })
                .collect()
        })
        .clone()
}

impl GeminiRouteContext {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with the Google catalog and a shared HTTP client.
    ///
    /// The `/v1beta` API path is appended to an explicit host or proxy prefix.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let mut provider = Self::from_models(
            "gemini",
            WireApi::GoogleGenerativeAi,
            None,
            model_catalog(),
            client,
        );
        provider.uses_default_catalog = true;
        match base_url {
            Some(base_url) => provider.with_base_url(base_url),
            None => provider,
        }
    }

    pub(crate) fn from_models(
        provider_identity: &str,
        wire: WireApi,
        base_url: Option<String>,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        Self {
            provider_identity: provider_identity.into(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            uses_default_catalog: false,
            client,
            wire,
        }
    }

    /// Bind registration, capture, and replay to the route identity.
    pub(crate) fn with_provider_identity(mut self, provider: &str) -> Self {
        self.provider_identity = provider.to_owned();
        self
    }

    /// Override the catalog records.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self.uses_default_catalog = false;
        self
    }

    /// Set the host or proxy prefix for the default `/v1beta` endpoint.
    ///
    /// Complete endpoints on explicitly supplied records take precedence.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = format!("{}/v1beta", base_url.into().trim_end_matches('/'));
        if self.uses_default_catalog {
            for model in &mut self.models {
                model.base_url = Some(self.base_url.clone());
            }
        }
        self
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }
}

/// Build the catalog-backed route registered as `google`.
pub fn google_provider(base_url: Option<String>, client: Arc<HttpClient>) -> GeminiProvider {
    GeminiProvider::with_client(base_url, client).with_provider_identity("google")
}

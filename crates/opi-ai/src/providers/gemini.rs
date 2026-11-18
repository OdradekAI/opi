//! Google Gemini provider profile.
//!
//! Endpoint, model catalog, route identity, wire selection, and dispatch
//! configuration live here. Gemini request framing and transport live in
//! [`crate::api::gemini`].

use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// Gemini provider profile.
#[derive(Clone)]
pub struct GeminiProvider {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) client: Arc<HttpClient>,
    /// Owning Google wire; Vertex reuses the Gemini body engine with a
    /// different wire-specific policy.
    pub(crate) wire: WireApi,
}

/// Built-in Gemini model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "gemini-2.0-flash",
            "Gemini 2.0 Flash",
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 8192)
                .with_images(true)
                .with_streaming(true),
        ),
    ]
}

impl GeminiProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self::with_client(base_url, Arc::new(HttpClient::new()))
    }

    /// Create with a shared HTTP client.
    pub fn with_client(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let base_url =
            base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".into());
        Self {
            provider_identity: "gemini".into(),
            base_url,
            models: model_catalog(),
            client,
            wire: WireApi::GoogleGenerativeAi,
        }
    }

    /// Pin the owning Google wire; Vertex reuses this profile for its
    /// per-wire neutral-policy branches.
    pub(crate) fn with_wire(mut self, wire: WireApi) -> Self {
        self.wire = wire;
        if wire == WireApi::GoogleVertex {
            self.provider_identity = "vertex".into();
        }
        self
    }

    /// Bind capture and replay to the registered canonical provider identity.
    #[cfg(feature = "builtins")]
    pub(crate) fn with_provider_identity(mut self, provider: &str) -> Self {
        self.provider_identity = provider.to_owned();
        self
    }

    /// Override the catalog records.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Override the API base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }
}

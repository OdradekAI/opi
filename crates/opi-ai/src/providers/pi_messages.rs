//! pi-messages Provider profile.
//!
//! Radius endpoint selection, the built-in model baseline, route identity and
//! HTTP-client ownership live here. pi-messages request framing, event mapping
//! and the typed Chat API live in [`crate::api::pi_messages`].

use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// Default Radius gateway (pi `DEFAULT_RADIUS_GATEWAY`).
pub const BASE_URL: &str = "https://radius.pi.dev";

/// The native `pi-messages` provider profile (Radius gateway).
#[derive(Clone)]
pub struct PiMessagesProvider {
    pub(crate) base_url: String,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) models: Vec<ModelInfo>,
}

impl std::fmt::Debug for PiMessagesProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PiMessagesProvider")
            .field("base_url", &self.base_url)
            .field("models", &self.models)
            .finish_non_exhaustive()
    }
}

impl PiMessagesProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let models = if base_url.as_deref().is_none_or(|base| base == BASE_URL) {
            model_catalog().to_vec()
        } else {
            Vec::new()
        };
        Self {
            base_url: base_url.unwrap_or_else(|| BASE_URL.into()),
            client: Arc::new(HttpClient::new()),
            models,
        }
    }

    /// Replace the profile's advertised model records.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Share the caller's HTTP client with the profile.
    #[cfg(feature = "builtins")]
    pub(crate) fn with_client(mut self, client: Arc<HttpClient>) -> Self {
        self.client = client;
        self
    }
}

/// opi-curated baseline catalog. pi ships `data/radius.json` as a
/// pre-catalog baseline for the default gateway; the generated data file is
/// absent from the pinned source, so opi pins one conservative entry per
/// family. A configured gateway's dynamic catalog replaces the baseline at
/// registration time (pi `radiusProvider.refreshModels`).
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![ModelInfo::new(
        "claude-sonnet-4-5",
        "claude-sonnet-4-5",
        WireApi::PiMessages,
        ModelCapabilities::new(200_000, 16_384)
            .with_streaming(true)
            .with_thinking(true)
            .with_images(true),
    )]
}

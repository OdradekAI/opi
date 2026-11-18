//! Mistral Provider profiles.
//!
//! Ordinary Mistral traffic uses the shared OpenAI-compatible Chat wire. The
//! native conversations profile state lives here, while its wire behavior
//! lives in the canonical API module.

use std::sync::Arc;

use crate::api::mistral::MistralOptions;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::providers::openai::{CompatConfig, OpenAiChatProvider};
use crate::registry::ModelCapabilities;

/// Default Mistral API base URL (without the /v1 suffix, which the adapter adds).
const BASE_URL: &str = "https://api.mistral.ai";

/// Create a Mistral-configured provider.
///
/// The provider resolves mistral:model specs and routes through the
/// OpenAI Chat Completions adapter. Authentication arrives per call via the
/// resolved auth passed to the Provider dispatch seam.
pub fn mistral_provider(base_url: Option<String>) -> OpenAiChatProvider {
    let base = base_url.unwrap_or_else(|| BASE_URL.into());
    OpenAiChatProvider::new_for_profile(
        base,
        "mistral".into(),
        CompatConfig::default(),
        vec![],
        model_catalog(),
    )
}

/// Built-in Mistral model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "mistral-large-latest",
            "Mistral Large",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "mistral-medium-latest",
            "Mistral Medium",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(32000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "mistral-small-latest",
            "Mistral Small",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(32000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "codestral-latest",
            "Codestral",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(256000, 8192).with_streaming(true),
        ),
    ]
}

/// Native Mistral conversations provider profile.
///
/// This profile owns endpoint, model records, shared transport and the
/// per-dispatch native option state. The API module implements its wire
/// projection and Provider/ChatApi behavior.
#[derive(Clone)]
pub struct MistralConversationsProvider {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) native_options: Option<MistralOptions>,
}

impl std::fmt::Debug for MistralConversationsProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MistralConversationsProvider")
            .field("provider_identity", &self.provider_identity)
            .field("base_url", &self.base_url)
            .field("models", &self.models)
            .finish_non_exhaustive()
    }
}

impl MistralConversationsProvider {
    #[cfg(feature = "builtins")]
    pub(crate) fn with_client(mut self, client: Arc<HttpClient>) -> Self {
        self.client = client;
        self
    }

    pub fn new(base_url: Option<String>) -> Self {
        Self {
            provider_identity: "mistral".into(),
            base_url: base_url.unwrap_or_else(|| BASE_URL.into()),
            client: Arc::new(HttpClient::new()),
            models: Vec::new(),
            native_options: None,
        }
    }

    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    pub fn with_native_options(mut self, options: MistralOptions) -> Self {
        self.native_options = Some(options);
        self
    }
}

//! Mistral provider profile  - routes through the OpenAI-compatible adapter.
//!
//! Mistral AI (<https://mistral.ai>) provides an OpenAI-compatible Chat
//! Completions API at `https://api.mistral.ai/v1/chat/completions`. This
//! module creates a pre-configured [`OpenAiChatProvider`] with Mistral's
//! base URL and a curated model list.

use crate::model_info::WireApi;
use crate::openai_chat::{CompatConfig, OpenAiChatProvider};
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// Default Mistral API base URL (without the `/v1` suffix, which the adapter adds).
const BASE_URL: &str = "https://api.mistral.ai";

/// Create a Mistral-configured provider.
///
/// The provider resolves `mistral:model` specs and routes through the
/// OpenAI Chat Completions adapter using standard Bearer token auth.
pub fn mistral_provider(api_key: String, base_url: Option<String>) -> OpenAiChatProvider {
    let base = base_url.unwrap_or_else(|| BASE_URL.into());
    OpenAiChatProvider::new_for_profile(
        api_key,
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

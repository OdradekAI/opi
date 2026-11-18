//! OpenRouter Provider profile.
//!
//! OpenRouter exposes an OpenAI-compatible chat endpoint. This module owns the
//! Provider-specific endpoint, identification headers and bundled model
//! metadata; the wire implementation remains in [`crate::api::openai`].

use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::providers::openai::{CompatConfig, OpenAiChatProvider};
use crate::registry::ModelCapabilities;

pub use crate::api::shared::openrouter_usage::{
    OpenRouterCostDetails, OpenRouterUsageAccounting, apply_openrouter_reported_cost,
};

// The re-export is intentional public provider surface. The implementation
// remains crate-private under `api::shared` so profile code does not own wire
// accounting or create a second root namespace.

/// Default OpenRouter API base URL (without the `/v1` suffix, which the
/// OpenAI-compatible adapter adds).
const BASE_URL: &str = "https://openrouter.ai/api";

/// Create an OpenRouter-configured Provider.
///
/// Authentication remains per-call: the resolved auth arrives through the
/// existing Provider dispatch seam instead of being captured by this profile.
pub fn openrouter_provider(base_url: Option<String>) -> OpenAiChatProvider {
    let base = base_url.unwrap_or_else(|| BASE_URL.into());
    OpenAiChatProvider::new_for_profile(
        base,
        "openrouter".into(),
        CompatConfig::default(),
        vec![
            (
                "HTTP-Referer".into(),
                "https://github.com/OdradekAI/opi".into(),
            ),
            ("X-Title".into(), "opi".into()),
        ],
        model_catalog(),
    )
}

/// Built-in OpenRouter model metadata without credentials or HTTP
/// construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "anthropic/claude-sonnet-4",
            "Claude Sonnet 4 (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 64000)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "anthropic/claude-haiku-4",
            "Claude Haiku 4 (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "openai/gpt-4o",
            "GPT-4o (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "openai/gpt-4o-mini",
            "GPT-4o Mini (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 16384)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "google/gemini-2.5-pro",
            "Gemini 2.5 Pro (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(1048576, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "deepseek/deepseek-r1",
            "DeepSeek R1 (via OpenRouter)",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(131072, 32768)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}

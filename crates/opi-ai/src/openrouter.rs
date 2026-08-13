//! OpenRouter provider profile  - routes through the OpenAI-compatible adapter.
//!
//! OpenRouter (<https://openrouter.ai>) provides an OpenAI-compatible API that
//! routes requests to many model providers. This module creates a pre-configured
//! [`OpenAiChatProvider`] with OpenRouter's base URL, identification headers, and
//! a curated model list.

use crate::model_info::WireApi;
use crate::openai_chat::{CompatConfig, OpenAiChatProvider};
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// Default OpenRouter API base URL (without the `/v1` suffix, which the adapter adds).
const BASE_URL: &str = "https://openrouter.ai/api";

/// Create an OpenRouter-configured provider.
///
/// The provider resolves `openrouter:model` specs, routes through the
/// OpenAI Chat Completions adapter, and sends `HTTP-Referer` and `X-Title`
/// headers for app identification on the OpenRouter platform. Authentication
/// is no longer baked into the provider; it arrives per call via the resolved
/// auth passed to [`Provider::stream_prepared`](crate::provider::Provider::stream_prepared).
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

/// Built-in OpenRouter model metadata without credentials or HTTP construction.
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

//! OpenRouter provider profile  - routes through the OpenAI-compatible adapter.
//!
//! OpenRouter (<https://openrouter.ai>) provides an OpenAI-compatible API that
//! routes requests to many model providers. This module creates a pre-configured
//! [`OpenAiChatProvider`] with OpenRouter's base URL, identification headers, and
//! a curated model list.

use serde::Deserialize;

use crate::model_info::WireApi;
use crate::openai_chat::{CompatConfig, OpenAiChatProvider};
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// OpenRouter's optional billing details attached to a provider usage record.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenRouterCostDetails {
    /// Total inference cost billed through the upstream provider account.
    #[serde(default)]
    pub upstream_inference_cost: Option<f64>,
    /// Upstream prompt processing cost, including provider cache pricing.
    #[serde(default)]
    pub upstream_inference_prompt_cost: Option<f64>,
    /// Upstream completion generation cost.
    #[serde(default)]
    pub upstream_inference_completions_cost: Option<f64>,
    /// Metered server-tool cost already included in OpenRouter's charge.
    #[serde(default)]
    pub server_tool_cost: Option<f64>,
}

/// OpenRouter's optional accounting fields on OpenAI-compatible usage.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenRouterUsageAccounting {
    /// Amount charged to the user's OpenRouter account.
    #[serde(default)]
    pub cost: Option<f64>,
    /// Whether OpenRouter used the user's upstream provider key.
    #[serde(default)]
    pub is_byok: Option<bool>,
    #[serde(default)]
    pub cost_details: Option<OpenRouterCostDetails>,
}

fn valid_cost(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

/// Apply a complete OpenRouter billed total while preserving catalog components.
///
/// Non-BYOK requests use OpenRouter's charged total directly. BYOK requests add
/// the upstream inference total, falling back to prompt plus completion detail
/// when the aggregate is absent. Incomplete or invalid accounting leaves the
/// existing catalog total untouched.
pub fn apply_openrouter_reported_cost(
    usage: &mut crate::types::usage::Usage,
    reported: &OpenRouterUsageAccounting,
) {
    let Some(cost) = valid_cost(reported.cost) else {
        return;
    };
    match reported.is_byok {
        Some(false) => {
            usage.cost.total = cost;
            usage.cost.total_from_provider = true;
        }
        Some(true) => {
            let Some(details) = reported.cost_details.as_ref() else {
                return;
            };
            let upstream = match valid_cost(details.upstream_inference_cost) {
                Some(upstream) => Some(upstream),
                None => match (
                    valid_cost(details.upstream_inference_prompt_cost),
                    valid_cost(details.upstream_inference_completions_cost),
                ) {
                    (Some(prompt), Some(completions)) => Some(prompt + completions),
                    _ => None,
                },
            };
            let Some(upstream) = upstream.filter(|value| value.is_finite()) else {
                return;
            };
            usage.cost.total = cost + upstream;
            usage.cost.total_from_provider = true;
        }
        None => {}
    }
}

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

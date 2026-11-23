//! OpenRouter Provider profile.
//!
//! OpenRouter exposes an OpenAI-compatible chat endpoint. This module owns the
//! Provider-specific endpoint, identification headers and bundled model
//! metadata; the wire implementation remains in [`crate::api::openai_completions`].

use crate::model_info::OpenAiCompletionsCompat as CompatConfig;
use crate::provider::ModelInfo;
use crate::provider::routes::OpenAiConfiguredRoute;

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
pub fn openrouter_provider(base_url: Option<String>) -> OpenAiConfiguredRoute {
    let base = base_url.unwrap_or_else(|| BASE_URL.into());
    OpenAiConfiguredRoute::new_for_profile(
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

/// Chat Completions models selected from the reviewed OpenRouter catalog.
/// The endpoint is supplied by the factory so custom proxy URLs apply to every row.
pub fn model_catalog() -> Vec<ModelInfo> {
    static MODELS: std::sync::OnceLock<Vec<ModelInfo>> = std::sync::OnceLock::new();
    MODELS
        .get_or_init(|| {
            let entries: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
                "../../resources/catalog/builtin/openrouter-profile.json"
            ))
            .expect("validated OpenRouter profile resource");
            entries
                .iter()
                .map(|entry| {
                    let mut model = crate::catalog::CatalogModel::parse("openrouter", entry)
                        .and_then(|row| row.to_chat_model_info())
                        .expect("validated OpenRouter chat record");
                    model.base_url = None;
                    model
                })
                .collect()
        })
        .clone()
}

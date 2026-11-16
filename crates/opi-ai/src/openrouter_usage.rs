//! Protocol-neutral OpenRouter accounting shapes shared by wire adapters.
//!
//! The concrete OpenRouter provider remains independently selectable. These
//! value types stay in a small shared module so Anthropic and OpenAI Chat can
//! preserve their existing cost handling without selecting that provider's
//! route, model catalog, or headers.

use serde::Deserialize;

/// OpenRouter's optional billing details attached to a provider usage record.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenRouterCostDetails {
    #[serde(default)]
    pub upstream_inference_cost: Option<f64>,
    #[serde(default)]
    pub upstream_inference_prompt_cost: Option<f64>,
    #[serde(default)]
    pub upstream_inference_completions_cost: Option<f64>,
    #[serde(default)]
    pub server_tool_cost: Option<f64>,
}

/// OpenRouter's optional accounting fields on OpenAI-compatible usage.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenRouterUsageAccounting {
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub is_byok: Option<bool>,
    #[serde(default)]
    pub cost_details: Option<OpenRouterCostDetails>,
}

fn valid_cost(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

/// Apply a complete OpenRouter billed total while preserving catalog components.
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

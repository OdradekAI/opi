//! Offline OpenAI Codex catalog pinned to pi 0.80.6.

use opi_ai::{
    ModelCapabilities, ModelInfo, ModelPricing, Pricing, PricingTier, ThinkingLevel,
    ThinkingLevelMap, ThinkingLevelMapping, WireApi,
};

pub const OPENAI_CODEX_DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";

struct CatalogRecord {
    id: &'static str,
    name: &'static str,
    context_window: u64,
    images: bool,
    max: bool,
    pricing: (f64, f64, f64, f64),
    tier: Option<(f64, f64, f64, f64)>,
}

const RECORDS: &[CatalogRecord] = &[
    CatalogRecord {
        id: "gpt-5.3-codex-spark",
        name: "GPT-5.3 Codex Spark",
        context_window: 128_000,
        images: false,
        max: false,
        pricing: (1.75, 14.0, 0.175, 0.0),
        tier: None,
    },
    CatalogRecord {
        id: "gpt-5.4",
        name: "GPT-5.4",
        context_window: 272_000,
        images: true,
        max: false,
        pricing: (2.5, 15.0, 0.25, 0.0),
        tier: Some((5.0, 22.5, 0.5, 0.0)),
    },
    CatalogRecord {
        id: "gpt-5.4-mini",
        name: "GPT-5.4 mini",
        context_window: 272_000,
        images: true,
        max: false,
        pricing: (0.75, 4.5, 0.075, 0.0),
        tier: None,
    },
    CatalogRecord {
        id: "gpt-5.5",
        name: "GPT-5.5",
        context_window: 272_000,
        images: true,
        max: false,
        pricing: (5.0, 30.0, 0.5, 0.0),
        tier: Some((10.0, 45.0, 1.0, 0.0)),
    },
    CatalogRecord {
        id: "gpt-5.6-luna",
        name: "GPT-5.6 Luna",
        context_window: 372_000,
        images: true,
        max: true,
        pricing: (1.0, 6.0, 0.1, 1.25),
        tier: Some((2.0, 9.0, 0.2, 2.5)),
    },
    CatalogRecord {
        id: "gpt-5.6-sol",
        name: "GPT-5.6 Sol",
        context_window: 372_000,
        images: true,
        max: true,
        pricing: (5.0, 30.0, 0.5, 6.25),
        tier: Some((10.0, 45.0, 1.0, 12.5)),
    },
    CatalogRecord {
        id: "gpt-5.6-terra",
        name: "GPT-5.6 Terra",
        context_window: 372_000,
        images: true,
        max: true,
        pricing: (2.5, 15.0, 0.25, 3.125),
        tier: Some((5.0, 22.5, 0.5, 6.25)),
    },
];

fn pricing((input, output, cache_read, cache_write): (f64, f64, f64, f64)) -> Pricing {
    Pricing {
        input_cost_per_mtok: input,
        output_cost_per_mtok: output,
        cache_read_cost_per_mtok: cache_read,
        cache_write_cost_per_mtok: cache_write,
    }
}

/// Exact static OpenAI Codex catalog used by listing and runtime construction.
pub fn openai_codex_catalog() -> Vec<ModelInfo> {
    RECORDS
        .iter()
        .map(|record| {
            let mut thinking = ThinkingLevelMap::reasoning_default()
                .with_mapping(
                    ThinkingLevel::Minimal,
                    ThinkingLevelMapping::Mapped("low".into()),
                )
                .with_mapping(ThinkingLevel::XHigh, ThinkingLevelMapping::Identity);
            if record.max {
                thinking =
                    thinking.with_mapping(ThinkingLevel::Max, ThinkingLevelMapping::Identity);
            }
            let tiers = record
                .tier
                .map(|tier| {
                    vec![PricingTier {
                        input_tokens_above: 272_000,
                        pricing: pricing(tier),
                    }]
                })
                .unwrap_or_default();
            ModelInfo::new(
                record.id,
                record.name,
                WireApi::OpenAiCodexResponses,
                ModelCapabilities::new(record.context_window, 128_000)
                    .with_images(record.images)
                    .with_streaming(true)
                    .with_thinking(true),
            )
            .with_base_url(OPENAI_CODEX_DEFAULT_BASE_URL)
            .with_thinking_level_map(thinking)
            .with_pricing(ModelPricing::try_new(pricing(record.pricing), tiers).unwrap())
            .unwrap()
        })
        .collect()
}

use std::str::FromStr;

use opi_ai::{ThinkingLevel, WireApi};
use opi_coding_agent::openai_codex::{OPENAI_CODEX_DEFAULT_BASE_URL, openai_codex_catalog};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/pi-0.80.6/openai-codex.models.json"))
        .expect("valid normalized fixture")
}

#[test]
fn openai_codex_tests_do_not_reference_ignored_repo() {
    let source = include_str!("openai_codex_provider.rs");
    assert!(!source.contains(&[".", "repo"].concat()));
    assert!(!source.contains(&["std::fs", "::read"].concat()));
}

#[test]
fn openai_codex_catalog_matches_pi_0806_fixture() {
    let fixture = fixture();
    assert_eq!(fixture["pi_version"], "0.80.6");
    assert_eq!(
        fixture["source_path"],
        "packages/ai/src/providers/openai-codex.models.ts"
    );
    assert_eq!(
        fixture["source_sha256"],
        "5F4E155179DA36F67177C18181FB6E23AB884D75126A983310456EA60AFFDEED"
    );
    assert_eq!(fixture["provider_id"], "openai-codex");
    assert_eq!(fixture["default_base_url"], OPENAI_CODEX_DEFAULT_BASE_URL);

    let fixture_models = fixture["models"].as_array().unwrap();
    let catalog = openai_codex_catalog();
    assert_eq!(fixture_models.len(), 7);
    assert_eq!(catalog.len(), 7);
    for (actual, expected) in catalog.iter().zip(fixture_models) {
        assert_eq!(actual.id, expected["id"]);
        assert_eq!(actual.display_name, expected["display_name"]);
        assert_eq!(actual.wire_api.as_str(), expected["wire"]);
        assert_eq!(actual.base_url.as_deref(), expected["base_url"].as_str());
        let capabilities = &expected["capabilities"];
        assert_eq!(
            actual.capabilities.context_window,
            capabilities["context_window"].as_u64().unwrap()
        );
        assert_eq!(
            actual.capabilities.max_output_tokens,
            capabilities["max_output_tokens"].as_u64().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_images,
            capabilities["supports_images"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_streaming,
            capabilities["supports_streaming"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_thinking,
            capabilities["supports_thinking"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_cache_control,
            capabilities["supports_cache_control"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_long_cache_retention,
            capabilities["supports_long_cache_retention"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            expected["input_modes"].as_array().unwrap().len(),
            if actual.capabilities.supports_images {
                2
            } else {
                1
            }
        );
        for (level, mapped) in expected["thinking_map"].as_object().unwrap() {
            let level = ThinkingLevel::from_str(level).unwrap();
            assert_eq!(
                actual.thinking_level_map.resolve(level).unwrap().as_deref(),
                mapped.as_str()
            );
        }
        assert_eq!(actual.compat.wire_api(), WireApi::OpenAiCodexResponses);
        let pricing = actual.pricing.as_ref().expect("embedded pricing");
        assert_eq!(
            pricing.base.input_cost_per_mtok,
            expected["pricing"]["input"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.base.output_cost_per_mtok,
            expected["pricing"]["output"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.base.cache_read_cost_per_mtok,
            expected["pricing"]["cache_read"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.base.cache_write_cost_per_mtok,
            expected["pricing"]["cache_write"].as_f64().unwrap()
        );
        let expected_tiers = expected["pricing"]["tiers"].as_array().unwrap();
        assert_eq!(pricing.tiers.len(), expected_tiers.len());
        for (tier, expected) in pricing.tiers.iter().zip(expected_tiers) {
            assert_eq!(
                tier.input_tokens_above,
                expected["input_tokens_above"].as_u64().unwrap()
            );
            assert_eq!(
                tier.pricing.input_cost_per_mtok,
                expected["input"].as_f64().unwrap()
            );
            assert_eq!(
                tier.pricing.output_cost_per_mtok,
                expected["output"].as_f64().unwrap()
            );
            assert_eq!(
                tier.pricing.cache_read_cost_per_mtok,
                expected["cache_read"].as_f64().unwrap()
            );
            assert_eq!(
                tier.pricing.cache_write_cost_per_mtok,
                expected["cache_write"].as_f64().unwrap()
            );
        }
    }
}

#[test]
fn openai_codex_catalog_uses_only_dedicated_wire() {
    let catalog = openai_codex_catalog();
    assert_eq!(
        catalog
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        [
            "gpt-5.3-codex-spark",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.5",
            "gpt-5.6-luna",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
        ]
    );
    assert!(
        catalog
            .iter()
            .all(|model| model.wire_api == WireApi::OpenAiCodexResponses)
    );
}

#[test]
fn openai_codex_pricing_tiers_keep_equality_on_base_rate() {
    for model in openai_codex_catalog()
        .into_iter()
        .filter(|model| !model.pricing.as_ref().unwrap().tiers.is_empty())
    {
        let pricing = model.pricing.unwrap();
        assert_eq!(pricing.effective(272_000), pricing.base, "{}", model.id);
        assert_ne!(pricing.effective(272_001), pricing.base, "{}", model.id);
    }
}

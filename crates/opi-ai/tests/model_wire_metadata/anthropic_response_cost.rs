//! Attribute Anthropic response cost to an eligible reported fallback model.

use opi_ai::message::AssistantMessage;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{Pricing, Usage};
use opi_ai::types::model::{
    AnthropicAllowedFallbackModel, ModelInfo, ModelPricing, PricingTier, WireApi, WireCompat,
};

fn rates(scale: f64) -> Pricing {
    Pricing {
        input_cost_per_mtok: scale,
        output_cost_per_mtok: scale * 2.0,
        cache_read_cost_per_mtok: scale * 3.0,
        cache_write_cost_per_mtok: scale * 4.0,
    }
}

fn prices(scale: f64) -> ModelPricing {
    ModelPricing {
        base: rates(scale),
        tiers: vec![],
    }
}

fn model() -> ModelInfo {
    let mut model = ModelInfo::new(
        "requested",
        "Requested",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(200_000, 8192),
    );
    model.pricing = Some(prices(1.0));
    let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
        unreachable!()
    };
    compat.allowed_fallback_models = Some(vec![
        AnthropicAllowedFallbackModel {
            provider: "anthropic".into(),
            model: "fallback".into(),
            cost: prices(10.0),
        },
        AnthropicAllowedFallbackModel {
            provider: "anthropic".into(),
            model: "requested".into(),
            cost: prices(999.0),
        },
        AnthropicAllowedFallbackModel {
            provider: "different".into(),
            model: "other-target".into(),
            cost: prices(20.0),
        },
    ]);
    model
}

fn fallback(model: &mut ModelInfo) -> &mut AnthropicAllowedFallbackModel {
    let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
        unreachable!()
    };
    &mut compat.allowed_fallback_models.as_mut().unwrap()[0]
}

fn response(reported: Option<&str>, provider: &str) -> AssistantMessage {
    let mut response = opi_ai::test_support::base_assistant();
    response.provider = provider.into();
    response.model = "requested".into();
    response.response_model = reported.map(str::to_owned);
    response.wire = Some(WireApi::AnthropicMessages);
    response.usage = Usage::reported(
        1_000_000,
        1_000_000,
        1_000_000,
        1_000_000,
        Some(500_000),
        None,
    );
    response
}

#[test]
fn cost_selects_only_a_different_reported_model_with_matching_declared_provider() {
    let model = model();
    for (reported, provider, expected) in [
        (Some("fallback"), "anthropic", 90.0),
        (Some("fallback"), "different", 9.0),
        (Some("requested"), "anthropic", 9.0),
        (Some("unlisted"), "anthropic", 9.0),
        (Some("other-target"), "anthropic", 9.0),
        (Some(""), "anthropic", 9.0),
        (None, "anthropic", 9.0),
    ] {
        let response = response(reported, provider);
        let before = serde_json::to_vec(&response).unwrap();
        let cost = model.calculate_response_cost(&response).unwrap();
        assert_eq!(cost.total_cost(), expected);
        assert_eq!(serde_json::to_vec(&response).unwrap(), before);
    }
    let mut other_wire = ModelInfo::new(
        "requested",
        "Requested",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(200_000, 8192),
    );
    other_wire.pricing = model.pricing.clone();
    assert_eq!(
        other_wire
            .calculate_response_cost(&response(Some("fallback"), "anthropic"))
            .unwrap()
            .total_cost(),
        9.0
    );
}

#[test]
fn fallback_pricing_uses_prompt_tiers_and_prices_the_one_hour_subset_once() {
    let mut model = model();
    fallback(&mut model).cost.tiers.push(PricingTier {
        input_tokens_above: 3_000_000.into(),
        pricing: rates(20.0),
    });
    let mut reply = response(Some("fallback"), "anthropic");
    let cost = model.calculate_response_cost(&reply).unwrap();
    assert_eq!(
        (
            cost.input_cost,
            cost.output_cost,
            cost.cache_read_cost,
            cost.cache_write_cost
        ),
        (10.0, 20.0, 30.0, 30.0)
    );
    assert_eq!(cost.total_cost(), 90.0);
    reply.usage.input_tokens += 1;
    let cost = model.calculate_response_cost(&reply).unwrap();
    assert!((cost.input_cost - 20.00002).abs() < 1e-10);
    assert_eq!(
        (
            cost.output_cost,
            cost.cache_read_cost,
            cost.cache_write_cost
        ),
        (40.0, 60.0, 60.0)
    );
    assert!((cost.total_cost() - 180.00002).abs() < 1e-10);
    fallback(&mut model).cost.base.input_cost_per_mtok = -1.0;
    assert!(
        (model.calculate_response_cost(&reply).unwrap().total_cost() - 180.00002).abs() < 1e-10
    );
    reply.usage.input_tokens -= 1;
    assert!(model.calculate_response_cost(&reply).is_none());
}

#[test]
fn unknown_usage_or_selected_prices_remain_unknown_and_known_free_remains_zero() {
    let mut model = model();
    let mut reply = response(Some("fallback"), "anthropic");
    model.pricing = None;
    assert_eq!(
        model.calculate_response_cost(&reply).unwrap().total_cost(),
        90.0
    );
    assert!(
        model
            .calculate_response_cost(&response(None, "anthropic"))
            .is_none()
    );
    reply.usage = Usage::unknown();
    assert!(model.calculate_response_cost(&reply).is_none());
    reply.usage = Usage::reported(0, 0, 0, 0, None, None);
    assert_eq!(
        model.calculate_response_cost(&reply).unwrap().total_cost(),
        0.0
    );
    for unknown in [-1.0, f64::NAN, f64::INFINITY] {
        fallback(&mut model).cost.base.input_cost_per_mtok = unknown;
        assert!(model.calculate_response_cost(&reply).is_none());
    }
    fallback(&mut model).cost = prices(0.0);
    reply.usage = response(Some("fallback"), "anthropic").usage;
    assert_eq!(
        model.calculate_response_cost(&reply).unwrap().total_cost(),
        0.0
    );
}

#[test]
fn fallback_declarations_reject_missing_identity_cost_and_excess_targets() {
    use opi_ai::catalog::{Catalog, CatalogOperation};
    let record = Catalog::builtin()
        .unwrap()
        .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
        .unwrap()
        .clone();
    for missing in ["provider", "model", "cost"] {
        let mut record = record.clone();
        let mut target = record.raw["compat"]["allowedFallbackModels"][0].clone();
        target.as_object_mut().unwrap().remove(missing);
        record.raw["compat"]["allowedFallbackModels"] = serde_json::json!([target]);
        assert!(record.to_chat_model_info().is_err(), "missing {missing}");
    }
    let mut invalid = model();
    let WireCompat::AnthropicMessages(compat) = &mut invalid.compat else {
        unreachable!()
    };
    let target = compat.allowed_fallback_models.as_ref().unwrap()[0].clone();
    compat.allowed_fallback_models = Some(vec![target; 4]);
    assert!(invalid.validate().is_err());
    for missing in ["provider", "model"] {
        let mut invalid = model();
        if missing == "provider" {
            fallback(&mut invalid).provider.clear();
        } else {
            fallback(&mut invalid).model.clear();
        }
        assert!(invalid.validate().is_err());
    }
}

//! Pricing validation, strict tier thresholds and usage service-tier metadata.

use opi_ai::stream::{Pricing, ServiceTier};
use opi_ai::types::model::{ModelPricing, PricingTier};

fn pricing(input: f64) -> Pricing {
    Pricing {
        input_cost_per_mtok: input,
        output_cost_per_mtok: input * 2.0,
        cache_read_cost_per_mtok: input / 10.0,
        cache_write_cost_per_mtok: input * 1.25,
    }
}

#[test]
fn pricing_tier_uses_strict_greater_than_boundary() {
    let model_pricing = ModelPricing::try_new(
        pricing(1.0),
        vec![PricingTier {
            input_tokens_above: 200_000.into(),
            pricing: pricing(2.0),
        }],
    )
    .unwrap();

    assert_eq!(model_pricing.effective(200_000), pricing(1.0));
    assert_eq!(model_pricing.effective(200_001), pricing(2.0));
}

#[test]
fn pricing_validates_rates_without_narrowing_source_tier_order() {
    assert!(ModelPricing::try_new(pricing(f64::NAN), Vec::new()).is_err());
    assert!(ModelPricing::try_new(pricing(-1.0), Vec::new()).is_err());
    let tiers = ModelPricing::try_new(
        pricing(1.0),
        vec![
            PricingTier {
                input_tokens_above: 20.into(),
                pricing: pricing(2.0),
            },
            PricingTier {
                input_tokens_above: 10.into(),
                pricing: pricing(3.0),
            },
            PricingTier {
                input_tokens_above: 10.into(),
                pricing: pricing(4.0),
            },
            PricingTier {
                input_tokens_above: 0.into(),
                pricing: pricing(5.0),
            },
        ],
    )
    .expect("zero, duplicate and unordered thresholds are source-valid");
    assert_eq!(tiers.effective(1), pricing(5.0));
    assert_eq!(tiers.effective(11), pricing(3.0));
    assert_eq!(tiers.effective(21), pricing(2.0));
    assert!(
        ModelPricing::try_new(
            pricing(1.0),
            vec![PricingTier {
                input_tokens_above: 0.into(),
                pricing: pricing(-1.0),
            }]
        )
        .is_err()
    );
}

#[test]
fn service_tier_usage_field_round_trips() {
    let mut usage = opi_ai::stream::Usage::reported(10, 5, 0, 0, None, None);
    usage.service_tier = Some(ServiceTier::Flex);
    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("flex"));
    let round_trip: opi_ai::stream::Usage = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip.service_tier, Some(ServiceTier::Flex));
}

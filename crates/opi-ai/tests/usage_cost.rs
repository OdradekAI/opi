//! Usage and cost tracking tests (task 2.10 + 14.5).
//!
//! DoD: "per-turn and cumulative usage accumulation with
//! cache_read_tokens/cache_write_tokens fields, cost calculation from
//! model pricing table with cache cost breakdown, tested"
//!
//! Task 14.5 adds cache_write_1h_tokens and reasoning_tokens subsets.

use opi_ai::stream::{CostBreakdown, CumulativeUsage, Pricing, Usage, calculate_cost};

// ---------------------------------------------------------------------------
// Usage struct with cache fields
// ---------------------------------------------------------------------------

#[test]
fn usage_default_has_zero_cache_tokens() {
    let u = Usage::default();
    assert_eq!(u.input_tokens, 0);
    assert_eq!(u.output_tokens, 0);
    assert_eq!(u.cache_read_tokens, 0);
    assert_eq!(u.cache_write_tokens, 0);
    assert_eq!(u.cache_write_1h_tokens, 0);
    assert_eq!(u.reasoning_tokens, 0);
}

#[test]
fn usage_with_cache_fields() {
    let u = Usage::reported(100, 50, 200, 75, 30, 10);
    assert_eq!(u.cache_read_tokens, 200);
    assert_eq!(u.cache_write_tokens, 75);
    assert_eq!(u.cache_write_1h_tokens, 30);
    assert_eq!(u.reasoning_tokens, 10);
}

#[test]
fn usage_total_tokens_includes_cache() {
    let u = Usage::reported(100, 50, 200, 75, 30, 15);
    // total = input + output + cache_read + cache_write
    // subsets (cache_write_1h, reasoning) are NOT added separately
    assert_eq!(u.total_tokens(), 425);
}

#[test]
fn usage_total_tokens_zero_when_all_zero() {
    assert_eq!(Usage::default().total_tokens(), 0);
}

#[test]
fn missing_usage_is_not_reported_as_known_zero_cost() {
    let usage = Usage::unknown();
    assert!(!usage.is_reported());
}

#[test]
fn usage_cache_write_1h_is_subset_of_cache_write() {
    // Construction accepts the subset; validation is at the provider level.
    let u = Usage::reported(100, 50, 200, 75, 30, 10);
    assert_eq!(u.cache_write_1h_tokens, 30);
    assert!(u.cache_write_1h_tokens <= u.cache_write_tokens);
}

#[test]
fn usage_reasoning_is_subset_of_output() {
    let u = Usage::reported(100, 50, 200, 0, 0, 10);
    assert_eq!(u.reasoning_tokens, 10);
    assert!(u.reasoning_tokens <= u.output_tokens);
}

#[test]
fn usage_new_fields_default_to_zero_with_serde() {
    // Simulate deserialization of old Usage JSON missing the new fields.
    let json = r#"{"input_tokens":100,"output_tokens":50,"cache_read_tokens":10,"cache_write_tokens":5,"reported":true}"#;
    let u: Usage = serde_json::from_str(json).unwrap();
    assert_eq!(u.cache_write_1h_tokens, 0);
    assert_eq!(u.reasoning_tokens, 0);
}

// ---------------------------------------------------------------------------
// CumulativeUsage
// ---------------------------------------------------------------------------

#[test]
fn cumulative_usage_starts_at_zero() {
    let cu = CumulativeUsage::default();
    assert_eq!(cu.total_input_tokens(), 0);
    assert_eq!(cu.total_output_tokens(), 0);
    assert_eq!(cu.total_cache_read_tokens(), 0);
    assert_eq!(cu.total_cache_write_tokens(), 0);
    assert_eq!(cu.total_cache_write_1h_tokens(), 0);
    assert_eq!(cu.total_reasoning_tokens(), 0);
    assert_eq!(cu.turn_count(), 0);
}

#[test]
fn cumulative_usage_accumulates_single_turn() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75, 30, 10));
    assert_eq!(cu.total_input_tokens(), 100);
    assert_eq!(cu.total_output_tokens(), 50);
    assert_eq!(cu.total_cache_read_tokens(), 200);
    assert_eq!(cu.total_cache_write_tokens(), 75);
    assert_eq!(cu.total_cache_write_1h_tokens(), 30);
    assert_eq!(cu.total_reasoning_tokens(), 10);
    assert_eq!(cu.turn_count(), 1);
}

#[test]
fn cumulative_usage_accumulates_multiple_turns() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75, 30, 10));
    cu.accumulate(&Usage::reported(150, 75, 300, 0, 0, 20));
    assert_eq!(cu.total_input_tokens(), 250);
    assert_eq!(cu.total_output_tokens(), 125);
    assert_eq!(cu.total_cache_read_tokens(), 500);
    assert_eq!(cu.total_cache_write_tokens(), 75);
    assert_eq!(cu.total_cache_write_1h_tokens(), 30);
    assert_eq!(cu.total_reasoning_tokens(), 30);
    assert_eq!(cu.turn_count(), 2);
}

#[test]
fn cumulative_usage_as_usage_returns_aggregate() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75, 30, 10));
    cu.accumulate(&Usage::reported(50, 25, 100, 25, 10, 5));
    let aggregate = cu.as_usage();
    assert_eq!(aggregate.input_tokens, 150);
    assert_eq!(aggregate.output_tokens, 75);
    assert_eq!(aggregate.cache_read_tokens, 300);
    assert_eq!(aggregate.cache_write_tokens, 100);
    assert_eq!(aggregate.cache_write_1h_tokens, 40);
    assert_eq!(aggregate.reasoning_tokens, 15);
    assert!(aggregate.is_reported());
}

// ---------------------------------------------------------------------------
// Pricing struct
// ---------------------------------------------------------------------------

#[test]
fn pricing_holds_per_million_token_rates() {
    let p = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    assert!((p.input_cost_per_mtok - 3.0).abs() < f64::EPSILON);
    assert!((p.output_cost_per_mtok - 15.0).abs() < f64::EPSILON);
    assert!((p.cache_read_cost_per_mtok - 0.30).abs() < f64::EPSILON);
    assert!((p.cache_write_cost_per_mtok - 3.75).abs() < f64::EPSILON);
}

#[test]
fn pricing_default_is_zero() {
    let p = Pricing::default();
    assert_eq!(p.input_cost_per_mtok, 0.0);
    assert_eq!(p.output_cost_per_mtok, 0.0);
    assert_eq!(p.cache_read_cost_per_mtok, 0.0);
    assert_eq!(p.cache_write_cost_per_mtok, 0.0);
}

// ---------------------------------------------------------------------------
// Cost calculation
// ---------------------------------------------------------------------------

#[test]
fn calculate_cost_basic_usage() {
    let usage = Usage::reported(1_000_000, 1_000_000, 0, 0, 0, 0);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    assert!((cost.input_cost - 3.0).abs() < 1e-10);
    assert!((cost.output_cost - 15.0).abs() < 1e-10);
    assert!((cost.cache_read_cost - 0.0).abs() < 1e-10);
    assert!((cost.cache_write_cost - 0.0).abs() < 1e-10);
    assert!((cost.cache_write_1h_cost - 0.0).abs() < 1e-10);
    assert!((cost.total_cost() - 18.0).abs() < 1e-10);
}

#[test]
fn calculate_cost_with_cache_tokens() {
    let usage = Usage::reported(500_000, 250_000, 1_000_000, 500_000, 150_000, 0);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    // input: 500k * 3.0/1M = 1.5
    // output: 250k * 15.0/1M = 3.75
    // cache_read: 1M * 0.30/1M = 0.30
    // cache_write (short remainder): (500k-150k) * 3.75/1M = 1.3125
    // cache_write_1h: 150k * 6.0/1M (2x input) = 0.90
    assert!((cost.input_cost - 1.5).abs() < 1e-10);
    assert!((cost.output_cost - 3.75).abs() < 1e-10);
    assert!((cost.cache_read_cost - 0.30).abs() < 1e-10);
    assert!((cost.cache_write_cost - 1.3125).abs() < 1e-10);
    assert!((cost.cache_write_1h_cost - 0.90).abs() < 1e-10);
    // total = 1.5 + 3.75 + 0.30 + 1.3125 + 0.90 = 7.7625
    assert!((cost.total_cost() - 7.7625).abs() < 1e-10);
}

#[test]
fn calculate_cost_zero_usage() {
    let usage = Usage::default();
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    assert!((cost.total_cost() - 0.0).abs() < 1e-10);
}

#[test]
fn calculate_cost_fractional_tokens() {
    // 500 tokens at $3.0/mtok = $0.0015
    let usage = Usage::reported(500, 0, 0, 0, 0, 0);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 0.0,
        cache_read_cost_per_mtok: 0.0,
        cache_write_cost_per_mtok: 0.0,
    };
    let cost = calculate_cost(&usage, &pricing);
    assert!((cost.input_cost - 0.0015).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// CostBreakdown
// ---------------------------------------------------------------------------

#[test]
fn cost_breakdown_total_sums_fields() {
    let cb = CostBreakdown {
        input_cost: 1.0,
        output_cost: 2.0,
        cache_read_cost: 0.5,
        cache_write_cost: 0.25,
        cache_write_1h_cost: 0.30,
    };
    // total = 1.0 + 2.0 + 0.5 + 0.25 + 0.30 = 4.05
    assert!((cb.total_cost() - 4.05).abs() < 1e-10);
}

#[test]
fn cost_breakdown_default_is_zero() {
    let cb = CostBreakdown::default();
    assert!((cb.total_cost()).abs() < 1e-10);
    assert!((cb.cache_write_1h_cost).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// CumulativeUsage + cost calculation integration
// ---------------------------------------------------------------------------

#[test]
fn cumulative_cost_across_turns() {
    let mut cu = CumulativeUsage::default();
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    cu.accumulate(&Usage::reported(
        100_000, 50_000, 200_000, 50_000, 20_000, 5_000,
    ));
    cu.accumulate(&Usage::reported(50_000, 25_000, 100_000, 0, 0, 10_000));
    let aggregate = cu.as_usage();
    let cost = calculate_cost(&aggregate, &pricing);
    // 150k input @ $3/mtok = $0.45
    // 75k output @ $15/mtok = $1.125
    // 300k cache_read @ $0.30/mtok = $0.09
    // cache_write (short remainder): (50k - 20k) * $3.75/mtok = $0.1125
    // cache_write_1h: 20k @ $6.0/mtok (2x input) = $0.12
    // total = 0.45 + 1.125 + 0.09 + 0.1125 + 0.12 = 1.8975
    assert!((cost.total_cost() - 1.8975).abs() < 1e-10);
    assert_eq!(aggregate.cache_write_1h_tokens, 20_000);
    assert_eq!(aggregate.reasoning_tokens, 15_000);
}

#[test]
fn calculate_cost_all_cache_writes_are_1h() {
    // When all cache writes are 1h, the short-remainder cost is zero.
    let usage = Usage::reported(100_000, 50_000, 0, 10_000, 10_000, 0);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    // cache_write short: (10k-10k) * 3.75 = 0
    assert!((cost.cache_write_cost - 0.0).abs() < 1e-10);
    // cache_write_1h: 10k * 6.0 (2x input) = 0.06
    assert!((cost.cache_write_1h_cost - 0.06).abs() < 1e-10);
}

#[test]
fn calculate_cost_no_1h_subset_is_regression_safe() {
    // When cache_write_1h is 0, cost matches legacy behavior.
    let usage = Usage::reported(500_000, 250_000, 1_000_000, 500_000, 0, 0);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    // cache_write: 500k * 3.75/1M = 1.875 (all short, no 1h)
    assert!((cost.cache_write_cost - 1.875).abs() < 1e-10);
    assert!((cost.cache_write_1h_cost - 0.0).abs() < 1e-10);
}

//! Usage and cost tracking tests (task 2.10).
//!
//! DoD: "per-turn and cumulative usage accumulation with
//! cache_read_tokens/cache_write_tokens fields, cost calculation from
//! model pricing table with cache cost breakdown, tested"

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
}

#[test]
fn usage_with_cache_fields() {
    let u = Usage::reported(100, 50, 200, 75);
    assert_eq!(u.cache_read_tokens, 200);
    assert_eq!(u.cache_write_tokens, 75);
}

#[test]
fn usage_total_tokens_includes_cache() {
    let u = Usage::reported(100, 50, 200, 75);
    // total = input + output + cache_read + cache_write
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
    assert_eq!(cu.turn_count(), 0);
}

#[test]
fn cumulative_usage_accumulates_single_turn() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75));
    assert_eq!(cu.total_input_tokens(), 100);
    assert_eq!(cu.total_output_tokens(), 50);
    assert_eq!(cu.total_cache_read_tokens(), 200);
    assert_eq!(cu.total_cache_write_tokens(), 75);
    assert_eq!(cu.turn_count(), 1);
}

#[test]
fn cumulative_usage_accumulates_multiple_turns() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75));
    cu.accumulate(&Usage::reported(150, 75, 300, 0));
    assert_eq!(cu.total_input_tokens(), 250);
    assert_eq!(cu.total_output_tokens(), 125);
    assert_eq!(cu.total_cache_read_tokens(), 500);
    assert_eq!(cu.total_cache_write_tokens(), 75);
    assert_eq!(cu.turn_count(), 2);
}

#[test]
fn cumulative_usage_as_usage_returns_aggregate() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 200, 75));
    cu.accumulate(&Usage::reported(50, 25, 100, 25));
    let aggregate = cu.as_usage();
    assert_eq!(aggregate.input_tokens, 150);
    assert_eq!(aggregate.output_tokens, 75);
    assert_eq!(aggregate.cache_read_tokens, 300);
    assert_eq!(aggregate.cache_write_tokens, 100);
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
    let usage = Usage::reported(1_000_000, 1_000_000, 0, 0);
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
    assert!((cost.total_cost() - 18.0).abs() < 1e-10);
}

#[test]
fn calculate_cost_with_cache_tokens() {
    let usage = Usage::reported(500_000, 250_000, 1_000_000, 500_000);
    let pricing = Pricing {
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    };
    let cost = calculate_cost(&usage, &pricing);
    assert!((cost.input_cost - 1.5).abs() < 1e-10);
    assert!((cost.output_cost - 3.75).abs() < 1e-10);
    assert!((cost.cache_read_cost - 0.30).abs() < 1e-10);
    assert!((cost.cache_write_cost - 1.875).abs() < 1e-10);
    assert!((cost.total_cost() - 7.425).abs() < 1e-10);
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
    let usage = Usage::reported(500, 0, 0, 0);
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
    };
    assert!((cb.total_cost() - 3.75).abs() < 1e-10);
}

#[test]
fn cost_breakdown_default_is_zero() {
    let cb = CostBreakdown::default();
    assert!((cb.total_cost()).abs() < 1e-10);
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
    cu.accumulate(&Usage::reported(100_000, 50_000, 200_000, 50_000));
    cu.accumulate(&Usage::reported(50_000, 25_000, 100_000, 0));
    let aggregate = cu.as_usage();
    let cost = calculate_cost(&aggregate, &pricing);
    // 150k input @ $3/mtok = $0.45
    // 75k output @ $15/mtok = $1.125
    // 300k cache_read @ $0.30/mtok = $0.09
    // 50k cache_write @ $3.75/mtok = $0.1875
    // total = $1.8525
    assert!((cost.total_cost() - 1.8525).abs() < 1e-10);
}

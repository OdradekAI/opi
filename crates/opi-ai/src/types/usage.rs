//! Token usage, terminal values and deterministic cost calculation.

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "tool_use")]
    ToolUse,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "aborted")]
    Aborted,
    /// Deferred submission outcome: the turn is still pending behind a
    /// deferred handle (task 20.50, AIR-043). Terminal `Deferred` results
    /// are `Ok` outcomes exactly like `Stop`/`Length`/`ToolUse`; the
    /// message carries no content and the handle rides the assistant
    /// message's `deferred` field.
    #[serde(rename = "deferred")]
    Deferred,
}

/// A message-level constrained-sampling diagnostic (task 20.11): the
/// recorded strict-prefer downgrade (kind
/// `anthropic_strict_tools_fallback`, reason
/// `strict_tool_limits_exceeded`) or a grammar-target fallback loss
/// (kind `constrained_sampling_fallback`). `detail` carries the
/// redacted-safe provider message when one exists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantDiagnostic {
    /// Diagnostic kind, e.g. `anthropic_strict_tools_fallback`.
    pub kind: String,
    /// Machine-readable reason, e.g. `strict_tool_limits_exceeded` or
    /// `grammar_target_unsupported`.
    pub reason: String,
    /// Redacted-safe detail line, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolUse => "tool_use",
            Self::Error => "error",
            Self::Aborted => "aborted",
            Self::Deferred => "deferred",
        }
    }
}

/// Provider-selected service tier for one response.
///
/// Known wire values retain their exact names. `Unknown` records a present
/// unrecognized response value; it never permits a guessed price multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
    Fast,
    /// A provider reported a tier that this SDK cannot price reliably.
    Unknown,
}

impl ServiceTier {
    /// Response-sensitive cost multiplier applied to every cost component.
    ///
    /// Mirrors the Responses annex: `flex` halves, `priority`/`fast` double,
    /// with the one model-specific exception (`gpt-5.5` prices priority and
    /// fast at 2.5). Absent/default tiers return 1. Per-adapter tier
    /// resolution stays with the owning adapters.
    pub fn cost_multiplier(self, model_id: &str) -> f64 {
        match self {
            Self::Auto | Self::Default | Self::Scale | Self::Unknown => 1.0,
            Self::Flex => 0.5,
            Self::Priority | Self::Fast if model_id == "gpt-5.5" => 2.5,
            Self::Priority | Self::Fast => 2.0,
        }
    }
}

/// Provider-reported token usage.
///
/// The optional `u64` child subsets preserve absent versus explicit zero:
/// `cache_write_1h_tokens` is contained by `cache_write_tokens`, and
/// `reasoning_tokens` is contained by `output_tokens`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_read_tokens: u32,
    #[serde(default)]
    pub cache_write_tokens: u32,
    /// Subset of `cache_write_tokens` eligible for 1-hour TTL retention.
    #[serde(default)]
    pub cache_write_1h_tokens: Option<u64>,
    /// Subset of `output_tokens` consumed by reasoning/thinking.
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
    /// Independently reported grand total, when the provider sends one. It is
    /// kept separate from the parent buckets: it may legitimately differ from
    /// [`Usage::bucket_total_tokens`], and an explicit zero stays distinct
    /// from an absent value.
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// Service tier the provider selected for this response, when reported.
    /// This original response fact is never replaced by a requested fallback.
    #[serde(default)]
    pub service_tier: Option<ServiceTier>,
    /// Wire-qualified billing tier after the adapter resolves request fallback
    /// and response precedence. Absent on wires without service-tier pricing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_service_tier: Option<ServiceTier>,
    #[serde(default)]
    pub reported: bool,
}

impl Default for Usage {
    fn default() -> Self {
        Self::unknown()
    }
}

impl Usage {
    pub fn unknown() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cache_write_1h_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: false,
        }
    }

    pub fn reported(
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
        cache_write_1h_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            cache_write_1h_tokens,
            reasoning_tokens,
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: true,
        }
    }

    /// Attach an independently reported grand total.
    pub fn with_total_tokens(mut self, total_tokens: Option<u64>) -> Self {
        self.total_tokens = total_tokens;
        self
    }

    /// Attach the provider-selected service tier.
    pub fn with_service_tier(mut self, service_tier: ServiceTier) -> Self {
        self.service_tier = Some(service_tier);
        self
    }

    pub fn is_reported(&self) -> bool {
        self.reported
    }

    /// Total tokens counted across parent buckets only.
    ///
    /// `cache_write_1h_tokens` is a subset of `cache_write_tokens` and
    /// `reasoning_tokens` is a subset of `output_tokens`, so they are
    /// not added here. An independently reported grand total, when present,
    /// lives in the [`Usage::total_tokens`] field and is not folded in.
    pub fn bucket_total_tokens(&self) -> u64 {
        self.input_tokens as u64
            + self.output_tokens as u64
            + self.cache_read_tokens as u64
            + self.cache_write_tokens as u64
    }
}

/// Accumulated usage across multiple turns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CumulativeUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cache_write_1h_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
    turns: u32,
    unknown_turns: u32,
}

/// Largest cumulative token count representable by the public [`Usage`] buckets.
const MAX_USAGE_BUCKET_TOKENS: u64 = u32::MAX as u64;

/// Convert an internal cumulative total to a public bucket without wrapping.
fn saturating_usage_bucket(tokens: u64) -> u32 {
    u32::try_from(tokens.min(MAX_USAGE_BUCKET_TOKENS))
        .expect("token total was clamped to the u32 range")
}

impl CumulativeUsage {
    /// Construct from pre-computed totals (e.g. when replaying a session).
    #[allow(clippy::too_many_arguments)]
    pub fn from_totals(
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
        cache_write_1h_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
        turns: u32,
        unknown_turns: u32,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            cache_write_1h_tokens,
            reasoning_tokens,
            turns,
            unknown_turns,
        }
    }

    pub fn total_input_tokens(&self) -> u64 {
        self.input_tokens
    }

    pub fn total_output_tokens(&self) -> u64 {
        self.output_tokens
    }

    pub fn total_cache_read_tokens(&self) -> u64 {
        self.cache_read_tokens
    }

    pub fn total_cache_write_tokens(&self) -> u64 {
        self.cache_write_tokens
    }

    pub fn total_cache_write_1h_tokens(&self) -> Option<u64> {
        self.cache_write_1h_tokens
    }

    pub fn total_reasoning_tokens(&self) -> Option<u64> {
        self.reasoning_tokens
    }

    pub fn turn_count(&self) -> u32 {
        self.turns
    }

    pub fn has_unknown_usage(&self) -> bool {
        self.unknown_turns > 0
    }

    pub fn accumulate(&mut self, turn: &Usage) {
        self.input_tokens = self
            .input_tokens
            .saturating_add(u64::from(turn.input_tokens));
        self.output_tokens = self
            .output_tokens
            .saturating_add(u64::from(turn.output_tokens));
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(u64::from(turn.cache_read_tokens));
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(u64::from(turn.cache_write_tokens));
        if let Some(tokens) = turn.cache_write_1h_tokens {
            self.cache_write_1h_tokens = Some(
                self.cache_write_1h_tokens
                    .unwrap_or(0)
                    .saturating_add(tokens),
            );
        }
        if let Some(tokens) = turn.reasoning_tokens {
            self.reasoning_tokens = Some(self.reasoning_tokens.unwrap_or(0).saturating_add(tokens));
        }
        self.turns = self.turns.saturating_add(1);
        if !turn.reported {
            self.unknown_turns = self.unknown_turns.saturating_add(1);
        }
    }

    pub fn as_usage(&self) -> Usage {
        let output_tokens = saturating_usage_bucket(self.output_tokens);
        let cache_write_tokens = saturating_usage_bucket(self.cache_write_tokens);
        Usage {
            input_tokens: saturating_usage_bucket(self.input_tokens),
            output_tokens,
            cache_read_tokens: saturating_usage_bucket(self.cache_read_tokens),
            cache_write_tokens,
            cache_write_1h_tokens: self
                .cache_write_1h_tokens
                .map(|tokens| tokens.min(u64::from(cache_write_tokens))),
            reasoning_tokens: self
                .reasoning_tokens
                .map(|tokens| tokens.min(u64::from(output_tokens))),
            total_tokens: None,
            service_tier: None,
            effective_service_tier: None,
            reported: self.unknown_turns == 0,
        }
    }
}

/// Per-million-token pricing for a model (USD).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pricing {
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub cache_read_cost_per_mtok: f64,
    pub cache_write_cost_per_mtok: f64,
}

/// Cost breakdown from a usage + pricing calculation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    /// Weighted cache-write cost: the short-cache remainder uses
    /// `cache_write_cost_per_mtok`, while the 1-hour subset uses twice the
    /// input-token price.
    pub cache_write_cost: f64,
}

impl CostBreakdown {
    pub fn total_cost(&self) -> f64 {
        self.input_cost + self.output_cost + self.cache_read_cost + self.cache_write_cost
    }
}

/// Calculate cost from usage and pricing.
///
/// The short-cache remainder (`cache_write - cache_write_1h`) is charged at
/// `cache_write_cost_per_mtok`. The 1h subset is charged at twice the input
/// price. `reasoning_tokens` is a subset of `output_tokens` and is already
/// accounted for in `output_cost` — no separate line is produced.
pub fn calculate_cost(usage: &Usage, pricing: &Pricing) -> CostBreakdown {
    calculate_cost_totals(
        u64::from(usage.input_tokens),
        u64::from(usage.output_tokens),
        u64::from(usage.cache_read_tokens),
        u64::from(usage.cache_write_tokens),
        usage.cache_write_1h_tokens,
        pricing,
    )
}

/// Calculate cost from exact cumulative totals without projecting parent
/// buckets through the public `u32` [`Usage`] representation.
pub fn calculate_cumulative_cost(usage: &CumulativeUsage, pricing: &Pricing) -> CostBreakdown {
    calculate_cost_totals(
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_read_tokens,
        usage.cache_write_tokens,
        usage.cache_write_1h_tokens,
        pricing,
    )
}

fn calculate_cost_totals(
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cache_write_1h_tokens: Option<u64>,
    pricing: &Pricing,
) -> CostBreakdown {
    let per_tok = |cost_per_mtok: f64| cost_per_mtok / 1_000_000.0;
    let cache_write_1h = cache_write_1h_tokens.unwrap_or(0).min(cache_write_tokens);
    let short_cache_write = cache_write_tokens.saturating_sub(cache_write_1h);
    CostBreakdown {
        input_cost: input_tokens as f64 * per_tok(pricing.input_cost_per_mtok),
        output_cost: output_tokens as f64 * per_tok(pricing.output_cost_per_mtok),
        cache_read_cost: cache_read_tokens as f64 * per_tok(pricing.cache_read_cost_per_mtok),
        cache_write_cost: short_cache_write as f64 * per_tok(pricing.cache_write_cost_per_mtok)
            + cache_write_1h as f64 * per_tok(pricing.input_cost_per_mtok) * 2.0,
    }
}

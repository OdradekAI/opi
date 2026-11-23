# Read usage and estimate cost

[简体中文](cost-pricing.zh.md) · [SDK quick start](../README.md#quick-start)

## Read usage from a result

Chat replies expose `AssistantMessage::usage`. Use `usage.total_tokens`
when the provider reports a total, including `Some(0)`. Otherwise use
`bucket_total_tokens()` only when `usage.is_reported()` is true.
Reasoning tokens are part of output tokens. One-hour cache writes are part
of cache-write tokens. Count each parent bucket once.

For a conversation, `CumulativeUsage` sums the known buckets and
`has_unknown_usage()` flags missing reports. A partial response can already
contain usage from the provider. Image and classifier results expose optional
`usage` and `cost` fields.

## Distinguish billed cost, estimates, and missing prices

When `usage.cost.total_from_provider` is true, `total_cost()` returns the
provider's total, including a reported zero. The component amounts remain
catalog estimates and can add up to a different amount. For example,
OpenRouter can report the gateway charge plus upstream cost for BYOK calls.
`catalog_total_cost()` sums the components.

To display an optional chat cost, pass the same `ChatModelRecord` used for
the call. A known free model produces `Some(0.0)`. Unreported usage, missing
or invalid rates, or an unknown tier on a tier-sensitive wire can produce
`None`. Store the original usage with the message if you need the
response-time accounting snapshot.

```rust
use opi_ai::message::AssistantMessage;
use opi_ai::{ChatModelRecord, calculate_cost_tiered};

pub fn chat_cost(reply: &AssistantMessage, model: &ChatModelRecord) -> Option<f64> {
    let usage = &reply.usage;
    if usage.cost.total_from_provider {
        return Some(usage.cost.total_cost());
    }
    calculate_cost_tiered(
        usage,
        model.info().pricing.as_ref()?,
        model.model_id(),
        model.wire_api(),
    )
    .map(|cost| cost.total_cost())
}
```

## How catalog pricing applies

- Rates are per million tokens. The prompt count for selecting a pricing
  tier is input + cache read + cache write. The highest
  `PricingTier::input_tokens_above` strictly exceeded by that count applies
  to the entire request. Equality alone keeps the previous tier, or base
  rates if there is no lower tier.
- One-hour cache writes are priced at twice the input rate. Short-cache
  rates apply to the remaining writes.
- `Usage::service_tier` records the provider's reported tier.
  `effective_service_tier` records the tier selected for billing by the wire.
  This matters for OpenAI Responses. Codex does not mark up Fast, and Azure
  does not apply a service-tier multiplier.
- OpenRouter image generation uses base rates. Read the estimate from
  `ImagesResult::cost`.

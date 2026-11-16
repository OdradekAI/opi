# Cost and pricing reference

This topic reference contains the detailed usage estimator, sampling, pricing-tier,
cache-pricing, service-tier, and compatibility rules for `opi-ai`. The main
[README](../README.md) keeps the onboarding path and a short best-effort cost
summary; this document is the source of truth for the full tables and
precedence rules.

## Estimator, Sampling and Pricing Metadata

The `context` estimator uses 3.5 UTF-16 code units per token
(`ceil(chars * 2 / 7)`) and 4,800 equivalent chars per image, rounding up at
message granularity: each user or tool-result message rounds once over its
blocks, each assistant message rounds once over text/thinking/tool-call
characters, and a system message rounds its body plus sections once with the
serialized added/removed tool lists rounded separately. The last applicable
positive successful usage anchors the estimate (any model identity,
non-error, non-aborted, timestamp not older than its prefix); only messages
after the anchor are estimated, and there is no same-model or full-text
lower bound. An independently reported positive `Usage::total_tokens` wins
even when it differs from the bucket sum; an explicit zero or absent total
falls back to the bucket sum only when the parent buckets are reported.
Unknown stays distinct from zero everywhere; child subsets are never added
twice.

| Parameter | Unit / formula | Omission → default | Zero / null / unknown | Precedence |
|---|---|---|---|---|
| text estimate | `ceil(utf16 / 3.5)` | empty → 0 | — | — |
| image estimate | 4800 chars per image | — | — | — |
| usage anchor | total or bucket sum | none → content estimate | total 0 → sum if reported; unknown → skip | total > sum |
| simple max tokens | `clamp(M)`: `max(1,M)` if window ≤ 0, else `min(M, max(1, W−E−4096))` | `None` → model default output | explicit `M` is not re-capped at the model max | request < window arithmetic |
| numeric-Claude thinking budget | level table 1024/2048/8192/16384 (xhigh/max → high), then clamped to leave answer room | omitted budget → level default | native enabled-without-budget uses 1024; simple budgets keep their wire-specific zero behavior | level default < explicit budget |
| sampling params | optional per-key JSON map | `None` → absent | `Some({})` retains empty presence; absent keys retain lower-layer values | model base < effective level < request |
| temperature | named field | `None` → omitted per adapter gating | NaN omitted | sampling map overrides the named field (merged last) |
| pricing tiers | selection on input+cacheRead+cacheWrite, strict `>`, highest wins, first of equal duplicates | no tiers → base rates | finite negative/fractional thresholds retained; only thresholds `> -1` can match | tier rates apply to the whole request |
| 1h cache write | subset at 2× input rate | `None` → all short-cache | explicit 0 valid, distinct from absent | subset, never double counted |
| service tier | Responses: flex 0.5×, priority/fast 2× (`gpt-5.5`: 2.5×); Codex fast and all Azure tiers: 1× | effective request/response fallback follows the selected wire | unknown tier → unknown cost on tier-sensitive wires | applies to every cost component |

`ModelInfo::sampling_params` and `Request::sampling_params` use
`Option<SamplingParams>`: `None` is absent and `Some(Default::default())` is
explicitly empty. `with_sampling_params(map)` preserves `Some(map)`, including
an empty map, through owned catalog store/reopen. Pass
`request.sampling_params.as_ref()` to `resolve_sampling_params`; it returns
`None` only when the model map, effective-level entry and request map are all
absent. A present empty map resolves to `Some` and does not clear lower keys;
nested values replace the whole value rather than merging recursively.

Only OpenAI-compatible adapters (Chat, Responses, Azure Responses) merge the resolved
sampling map, as the last body step so model and request sampling override
named request fields; other adapters ignore it. `Usage::service_tier` retains
the observed response category; `effective_service_tier` retains the selected
wire's request/response billing rule. `calculate_cost_tiered(usage, pricing,
model_id, wire)` returns `None` for unknown usage, invalid pricing or an unknown
tier on a tier-sensitive wire. OpenRouter images use base rates without tiers.
`PricingTier::input_tokens_above` uses `serde_json::Number`: exact integers and
finite fractional values. Integer Rust literals use `100_000_u64.into()`;
fractions use `Number::from_f64`. Existing integer catalog snapshots remain
readable without rounding. Thresholds and `u64` token comparisons stay exact
above `2^53`.
Native calls preserve their adapter budget semantics; Pi Messages simple calls
forward their own whitelist. `ModelInfo::input_limits` is metadata, never an
automatic rejection or resize policy. Retired static `reasoning_effort` compat
keys fail with a migration error.

## Public entry points

Anthropic `message_delta` updates each reported token field, including zero;
missing/null fields retain their previous values. An omitted stop reason also
retains the previous reason. Catalog cost is refreshed at message start and
after each accepted usage update, before a canonical partial or failure can be
published. A malformed merged cache-write subset rejects the update atomically.
Provider-reported totals retain their authority, including an explicit zero.
Earlier owned event snapshots remain unchanged. Automatic Models observers
read the canonical outcome; raw prepared attempts still require explicit
observation and do not acquire an AI span merely by carrying a context.

`Usage.cost` is serialized with its message as a response-time accounting
snapshot. It retains the catalog components and the independent `total`, with
`total_from_provider` identifying a provider-reported total (including zero).
Deserializing an older usage object without `cost` supplies the default
breakdown and does not claim a provider total. A host's cumulative cost estimate
is a separate policy; serializing a message does not recompute its cost from a
current catalog or establish that all prices are known.

The primary public values are `Usage`, `CumulativeUsage`, `Pricing`,
`PricingTier`, `SamplingParams`, `ModelInfo::sampling_params`, and
`Request::sampling_params`. Cost calculation is exposed through
`calculate_cost_tiered`; the estimator and sampling resolver are available from
the corresponding `context` and request/model APIs described in the crate
rustdoc.

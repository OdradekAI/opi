# Align pricing tiers, cache pricing, and provider-reported cost

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Keep request duration metric attributes bounded](24-request-duration-metric-attributes.md)

## Scope

Pi 1.5.0 exposes a four-component catalog estimate and a separate `total`.
Pricing tiers use the strict prompt-side token boundary, one-hour cache writes
are a subset of cache writes with a two-times input rate, and OpenRouter may
replace only the final total with provider billing. The provider billing fields
are interpreted only on an OpenRouter route.

## Decision and implementation

- `Usage.cost` now carries catalog components, `total`, and
  `total_from_provider`; `catalog_total_cost()` and `total_cost()` make the
  distinction explicit.
- `Usage::apply_catalog_cost` and `ModelInfo::apply_response_cost` recompute
  catalog pricing while preserving a provider-reported total.
- OpenAI Chat and Anthropic usage parsers accept OpenRouter billing extensions,
  but only their OpenRouter provider paths apply them. Non-BYOK uses the
  charged `cost`; BYOK adds the upstream aggregate or prompt-plus-completion
  fallback. Invalid or incomplete accounting leaves catalog pricing unchanged.
- OpenAI-compatible usage separates `cache_write_tokens` from cache reads and
  subtracts both child buckets from the prompt parent exactly once.

## Acceptance

- tier selection counts `input + cache_read + cache_write` and uses strict `>`;
- cache-write 1h is clamped to its parent and never double-counted;
- provider totals can differ from catalog components and drive telemetry;
- generic OpenAI/Anthropic parsers do not apply OpenRouter billing fields;
- existing provider fixture behavior remains green.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/models.ts:1236-1256`.
- Pi source: `.pi/pi-1.5.0/packages/ai/src/utils/openrouter-cost.ts:1-56`.
- Pi source: `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts:1510-1553`.
- Rust implementation: `crates/opi-ai/src/types/usage.rs`,
  `crates/opi-ai/src/types/model.rs`, `crates/opi-ai/src/openrouter.rs`.
- Provider paths: `crates/opi-ai/src/openai_chat.rs`,
  `crates/opi-ai/src/anthropic.rs`.
- Regressions: `crates/opi-ai/tests/usage_cost.rs`,
  `crates/opi-ai/tests/openrouter_cost.rs`,
  `crates/opi-ai/tests/ai_telemetry.rs::provider_reported_cost_drives_telemetry_total`.

## Limits

This slice does not claim live provider billing, complete TS/Rust differential,
feature/platform/MSRV, performance, or the six-gate release acceptance. Direct
low-level parsers without a provider route do not calculate catalog pricing;
the provider dispatch path supplies model metadata and applies it at terminal
response time.

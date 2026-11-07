# Keep request duration metric attributes bounded

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Preserve optional response id in AI telemetry](23-optional-response-id-telemetry.md)

## Scope

Pi records `pi.ai.request.duration` with the low-cardinality attributes
`provider`, `model`, and `stop`. Opi previously reused the complete request
span end-attribute map, leaking usage values, cost and the high-cardinality
response id into the histogram measurement.

## Decision and implementation

- build a dedicated duration attribute map containing only provider, model and
  normalized Pi stop reason;
- keep the full usage, cost and optional response id attributes on the request
  span;
- leave `pi.ai.first_chunk.duration`, token and cost metric attributes unchanged.

## Acceptance

- request duration measurements contain exactly provider, model and stop;
- request spans still retain usage, cost and an optional response id;
- pending/deferred poll and tool-use stop vocabulary behavior remains green.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/telemetry.ts:198-199`.
- Rust implementation: `crates/opi-ai/src/telemetry.rs::AiRequestTelemetry::finish`.
- Regression: `crates/opi-ai/tests/ai_telemetry.rs::records_request_events_and_result_after_the_terminal_payload`.

## Limits

This slice only bounds request-duration metric attributes. It does not claim
live provider, complete TS/Rust differential, feature/platform/MSRV,
performance or six-gate release acceptance.

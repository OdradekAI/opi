# Preserve optional response id in AI telemetry

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Align pending stop state and AI telemetry vocabulary](22-pending-stop-and-telemetry-vocabulary.md)

## Scope

Pi writes `response.id` from the optional `AssistantMessage.responseId`. The
telemetry adapter omits the attribute when the provider did not report an id;
it does not turn an absent value into an empty string. Opi previously inserted
`"response.id": ""` for `None`, changing the optional high-cardinality
attribute's observable shape.

## Decision and implementation

- insert `response.id` only when `AssistantMessage.response_id` is `Some`;
- keep the existing string value and high-cardinality schema when an id exists;
- leave usage, cost, stop, event and span settlement behavior unchanged.

## Acceptance

- a result with no response id has no `response.id` span attribute;
- a result with an id preserves the exact value;
- the existing AI telemetry and provider fixture suites remain green.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/telemetry.ts:182-198` and
  `.pi/pi-1.5.0/packages/ai/src/types.ts:592`.
- Rust implementation: `crates/opi-ai/src/telemetry.rs::AiRequestTelemetry::finish`.
- Regression: `crates/opi-ai/tests/ai_telemetry.rs::telemetry_omits_absent_response_id_attribute`.

## Limits

This slice only fixes the optional span-attribute shape. It does not claim
live provider, complete TS/Rust differential, feature/platform/MSRV,
performance or six-gate release acceptance.

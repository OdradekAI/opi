# Align pending stop state and AI telemetry vocabulary

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Bound AI telemetry error status messages in UTF-16 units](21-utf16-error-status-boundary.md)

## Scope

Pi 1.5.0 exposes `StopReason` values `pending`, `stop`, `length`, `toolUse`,
`error`, `aborted`, and `deferred`. Opi had no public `Pending` value, used
`Stop` for provider-created partial messages, and emitted `tool_use` from AI
telemetry even though Pi telemetry uses the camel-case `toolUse` vocabulary.
Pi also excludes both pending and deferred polls from token and cost counters.

## Decision and implementation

- add the Rust-native `StopReason::Pending` value with serde and `as_str`
  support;
- initialize provider stream partial messages with `Pending`, while terminal
  `Done` handling rejects a pending reason as a protocol error;
- map telemetry stop attributes and request-duration metric attributes through a
  Pi-specific vocabulary helper (`toolUse`, not the canonical Rust
  `tool_use` spelling);
- declare the complete Pi stop vocabulary in the AI telemetry schema;
- exclude both `Pending` and `Deferred` poll results from token and cost
  counters.

## Acceptance

- a pending poll records duration only;
- terminal telemetry for tool use records `stop = "toolUse"` on both the span
  and request-duration metric;
- the schema advertises all seven Pi stop values;
- provider fixture streams and the external crate consumer remain green.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/types.ts:489` and
  `.pi/pi-1.5.0/packages/ai/src/telemetry.ts:20-21,199`.
- Rust values: `crates/opi-ai/src/types/usage.rs::StopReason`.
- Rust telemetry: `crates/opi-ai/src/telemetry.rs::telemetry_stop_reason` and
  `AiRequestTelemetry::finish`.
- Provider partial constructors: Anthropic, Gemini, Bedrock, OpenAI Chat,
  OpenAI Responses, Mistral, and Pi Messages adapters.
- Regression: `crates/opi-ai/tests/ai_telemetry.rs` pending poll, tool-use
  spelling, and schema vocabulary tests.

## Limits

This slice does not add a new provider protocol that returns `pending` as a
terminal event; such an event is rejected because Pi treats `pending` as a
non-terminal partial state. Live provider, complete TS/Rust differential,
feature/platform/MSRV, performance and six-gate release acceptance remain
separate work.

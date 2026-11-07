# Non-chat and deferred-cancellation AI telemetry boundary

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Prepared observed attempt request-token cancellation telemetry](18-prepared-request-token-cancel-telemetry.md)

## Scope

Pin which public operations install the Pi-owned `pi.ai.*` vocabulary and
which operations only carry the caller's frozen `TelemetryContext` to an
adapter. This closes the ambiguity between carrier propagation and SDK span
creation for deferred cancellation, image generation and classification.

## Decision

- ordinary chat entries install one `pi.ai.request` observer when the caller
  supplies a telemetry context;
- deferred fetch installs one `pi.ai.poll` observer per fetch operation;
- `cancel_deferred`, image generation and classifier calls do not install a
  `pi.ai.*` observer, matching Pi 1.5.0's `Models` implementation;
- those non-observed operations still forward the explicit context to the
  provider/API adapter, so an adapter or host can add operation-specific
  instrumentation without silently expanding the Pi AI schema;
- default and `noop` contexts remain passive, and direct provider/API calls
  remain outside SDK-owned AI telemetry.

## Acceptance

- cancelling a registered deferred response with a recorder context succeeds
  without emitting a `pi.ai.poll` or `pi.ai.request` span;
- existing image and classifier carrier tests continue to prove context
  presence reaches the adapter;
- bilingual README text distinguishes carrier propagation from AI observer
  creation;
- deferred fetch continues to emit exactly one `pi.ai.poll` per fetch.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/models.ts:969-1023` only wraps
  `streamDeferred` with `observeModelRequest`; `cancelDeferred`,
  `generateImages` and `classify` do not call it.
- Pi source: `.pi/pi-1.5.0/packages/ai/src/telemetry.ts:123-153` defines the
  request/poll observer and its `poll` switch.
- Rust regression: `crates/opi-ai/tests/ai_telemetry.rs`.
- Carrier regressions: `crates/opi-ai/tests/deferred_contract.rs`,
  `crates/opi-ai/tests/openrouter_fixtures.rs`,
  `crates/opi-ai/tests/classifier_contract.rs`.

## Limits

This slice does not add image/classifier/cancel spans, does not define a new
operation-specific telemetry schema, and does not claim adapter-owned spans
are equivalent to Pi AI spans. Live providers, the full matrix, differential,
performance and six-gate release acceptance remain separate work.

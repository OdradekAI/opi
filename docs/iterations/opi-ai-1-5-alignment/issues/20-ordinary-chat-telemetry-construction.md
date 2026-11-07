# Ordinary chat telemetry starts at construction

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Non-chat and deferred-cancellation AI telemetry boundary](19-non-chat-ai-telemetry-boundary.md)

## Scope

Align the ordinary `Models` chat observer boundary with Pi 1.5.0. Pi's
`Models.stream` and `streamSimple` install `observeModelRequest` while the
stream is constructed, before lazy authentication, provider setup, or event
consumption. The Rust implementation previously created the observer only
after the independently scheduled setup had completed.

## Decision

- `ChatCall::new` creates the shared `pi.ai.request` observer synchronously
  from the frozen route, model, request reasoning, and pricing facts;
- the execution driver and `ChatStream` share that one observer, so normal
  result, source-tail drain, explicit abort, and setup-failure settlement keep
  their existing one-span/idempotent behavior;
- a call with an explicit context therefore has a visible unfinished span even
  before its execution task reaches the first poll; no context remains passive;
- setup failures finish the construction-time observer with the canonical
  `CallFailure.message()` so the span receives the same error stop/usage facts
  as the public failure contract.

## Acceptance

- constructing a hanging ordinary call with a recorder context immediately
  records one unfinished `pi.ai.request` span before provider progress;
- explicit abort still finishes that same span once with `stop = aborted`;
- the existing ordinary, prepared, deferred, raw stream and non-chat telemetry
  suites remain green;
- the README and phase evidence describe construction-time observation without
  claiming that authentication or provider setup is synchronous.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/models.ts:910-968` passes
  `observeModelRequest` directly to `lazyStream` during `stream` construction.
- Pi source: `.pi/pi-1.5.0/packages/ai/src/api/lazy.ts:58-123` settles setup
  failures through the same observer result callback.
- Rust implementation: `crates/opi-ai/src/models.rs` (`ChatCall::new`,
  `ChatCallDriver`, and `materialize_chat_call`).
- Regression: `crates/opi-ai/tests/ai_telemetry.rs` asserts the span exists and
  is unfinished immediately after construction, then verifies abort settlement.

## Limits

This slice does not make authentication or provider setup synchronous, does not
automatically instrument prepared raw attempts, and does not add spans to
image/classifier/cancel operations. Full provider coverage, differential,
feature/platform/MSRV, performance, and six-gate release acceptance remain
separate work.

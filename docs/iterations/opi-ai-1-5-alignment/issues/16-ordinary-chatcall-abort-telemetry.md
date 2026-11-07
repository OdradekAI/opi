# Ordinary ChatCall explicit-abort telemetry settlement

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Started ChatStream explicit-abort telemetry settlement](15-started-chatstream-abort-telemetry.md)

## Scope

Close the same lifecycle hole for the automatic observer installed by ordinary
`Models::stream*` and `Models::complete*` calls: `ChatCall::abort()` must settle
the request span when it force-cancels the outer execution task while the inner
observed `ChatStream` is still pending.

## Decision

- `ChatCallDriver` owns a shared observer slot that is populated when the inner
  observed `ChatStream` is materialized;
- the outer `StartedChatStream` abort handler uses that same observer only after
  Tokio confirms task cancellation;
- normal `ChatStream` settlement remains the completion authority, so a normal
  terminal that wins the race is not overwritten;
- setup failures before an observer exists, raw streams, request-token
  cancellation and non-chat operations retain their existing boundaries.

## Acceptance

- an ordinary `Models` call reaches a hanging provider and `call.abort().await`
  returns the existing typed `Aborted` failure;
- exactly one `pi.ai.request` span is settled with `stop=aborted` and an end
  sequence;
- ordinary normal completion, started-stream abort, prepared-attempt and
  execution lifecycle tests remain green.

## Limits

This slice covers force-abort of the ordinary `ChatCall` execution owner.
Cooperative request-token cancellation still relies on the provider's typed
`Cancelled` terminal path. Automatic observation for raw `ChatStream::new` and
raw `PreparedProviderCall::start_attempt`, image/classifier/deferred-cancel,
live providers, the full matrix, differential, performance and six-gate
release acceptance remain separate work.

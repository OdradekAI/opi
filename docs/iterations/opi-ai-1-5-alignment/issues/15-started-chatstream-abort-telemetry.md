# Started ChatStream explicit-abort telemetry settlement

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Started ChatStream telemetry settlement](13-started-chatstream-telemetry-settlement.md)

## Scope

Close the remaining lifecycle hole for an observed `ChatStream` moved into a
`StartedChatStream`: an explicit `abort()` must settle the same
`pi.ai.request` observer when Tokio confirms that the execution task was
cancelled before a business terminal was produced.

## Decision

- `ChatStream` and its started execution share one idempotent observer cell;
- normal terminal settlement remains owned by `ChatStream::result_without_tail`;
- `StartedChatStream::abort()` invokes an abort finalizer only after the joined
  task reports Tokio cancellation, using the typed `Aborted` partial returned
  by the existing execution contract;
- a completion/abort race is harmless: observer finish is idempotent and a
  normal terminal that won the race is not overwritten;
- runtime shutdown, consumer drop, raw `ChatStream::new`, and raw prepared
  attempts keep their existing contracts.

## Acceptance

- an observed started stream with a partial event and a pending source returns
  the existing `CallFailure` with `StopReason::Aborted` after `abort()`;
- exactly one `pi.ai.request` span is settled with `stop=aborted` and an end
  sequence;
- existing normal, already-terminal, and per-attempt retry telemetry tests
  remain green.

## Limits

This is force-abort settlement. Cooperative request-token cancellation still
uses the provider's typed `Cancelled` terminal path. No observer is installed
for raw `ChatStream::new` or raw `PreparedProviderCall::start_attempt`, and
image/classifier/deferred-cancel, live-provider, full matrix, differential,
performance, and six-gate release acceptance remain separate work.

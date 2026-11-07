# Prepared observed attempt request-token cancellation telemetry

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Ordinary request-token cancellation telemetry](17-request-token-cancel-telemetry.md)

## Scope

Pin the cooperative cancellation contract for the explicit
`PreparedProviderCall::start_attempt_with_telemetry()` seam without changing
the raw `start_attempt()` return type or introducing an implicit whole-call
aggregator.

## Decision

- the observed helper wraps one prepared attempt in the standard `ChatStream`
  result and telemetry contract;
- cancelling the frozen `Request::cancel` token reaches the provider's typed
  `ProviderError::Cancelled` terminal;
- the attempt observer settles once with `stop=aborted` and the raw attempt
  seam remains business-only `EventStream`;
- sequential attempts continue to own separate spans; any whole-call view is
  caller-owned.

## Acceptance

- a prepared observed attempt reaches a hanging provider before its retained
  token is cancelled;
- the result retains `ProviderError::Cancelled` and `StopReason::Aborted`;
- exactly one `pi.ai.request` span is settled with `stop=aborted` and an end
  sequence;
- ordinary request-token cancellation, explicit abort and retry observation
  regressions remain green.

## Limits

This slice does not change raw `PreparedProviderCall::start_attempt()` or raw
`ChatStream::new`, and does not add image/classifier/deferred-cancel AI spans.
Live providers, the full matrix, differential, performance and six-gate release
acceptance remain separate work.

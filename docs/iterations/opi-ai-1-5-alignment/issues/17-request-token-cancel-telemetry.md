# Ordinary request-token cancellation telemetry

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Ordinary ChatCall explicit-abort telemetry settlement](16-ordinary-chatcall-abort-telemetry.md)

## Scope

Pin the cooperative cancellation boundary for the automatic observer installed
by ordinary `Models::stream*` and `Models::complete*` calls. Cancelling the
retained `Request::cancel` token must reach the provider's typed
`ProviderError::Cancelled` terminal and settle the same `pi.ai.request` span.

## Decision

- request-token cancellation remains provider-cooperative and is not routed
  through the force-abort handler;
- the normal `ChatStream` result path remains the observer completion
  authority;
- `ProviderError::Cancelled` maps to `StopReason::Aborted` and telemetry
  `stop=aborted`, with the observer's existing exactly-once settlement;
- force-abort and request-token cancellation stay separate observable paths.

## Acceptance

- an ordinary `Models` call reaches a hanging provider before its retained
  request token is cancelled;
- `call.result().await` returns the existing typed `ProviderError::Cancelled`
  failure with `StopReason::Aborted`;
- exactly one `pi.ai.request` span is settled with `stop=aborted` and an end
  sequence;
- explicit `ChatCall::abort()` and normal completion regressions remain green.

## Limits

This slice covers ordinary chat's cooperative request-token cancellation.
Automatic observation for raw `ChatStream::new` and raw
`PreparedProviderCall::start_attempt`, image/classifier/deferred-cancel,
live providers, the full matrix, differential, performance and six-gate release
acceptance remain separate work.

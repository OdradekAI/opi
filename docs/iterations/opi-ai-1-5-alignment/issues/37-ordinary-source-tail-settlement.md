# Drain the ordinary provider source tail before settlement

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [material and differential hardening](36-supplemented-materials-and-gated-differential.md)

## Agreed scope

The user approved continuing with the ordinary-call source-tail truncation found
in the gated lifecycle differential. Fix the existing owner, preserve first-terminal
absorption, retry/admission, explicit cancellation and consumer-Drop behavior, and
append new evidence without replacing historical failures or approving unrelated
differences. Upstream reinstall, lock replacement and catalog hydration are not
performed by this repair.

## Acceptance and outcome

- A public `Models::stream_simple` regression fails before the fix because the
  source never reaches its terminal-tail barrier (exit 101, `RecvError`).
- `drive_resolved_call_registered` now drains only its retained attempt after a
  business terminal. It neither re-enters admission nor publishes tail items.
- A gated tail proves result/terminal/telemetry settlement waits for source EOF,
  including no further consumption, success, Error/Aborted and legacy error
  terminals. Late delta, timeout and repeated Done cannot replace the first result.
- Explicit abort during a pending tail joins source release and returns Aborted
  without falsely claiming EOF. Cooperative token cancellation allows the source
  to finish its tail without replacing the already captured business outcome.
  Transport failures still release without starting a tail.
- A review follow-up combines one admitted pre-handoff retry with the second
  attempt's gated tail. It verifies exactly two dispatches and retention of the
  second attempt's terminal through tail EOF, without a third dispatch.
- The two existing consumer-Drop example tests now require observed source EOF.
  The Rust differential regression no longer accepts the previous product failure.
- Actual TS/Rust replay has three matching shared-observation cases. Its overall
  verdict remains `BLOCKED_WITH_RECORDED_OBSERVATIONS`, not strict alignment:
  full-lock identity and generated/pinned catalog identity remain unresolved;
  startup timing and setup-error differences remain unapproved.

Exact commands, checks, review and limitations are owned by the
[repair report](../phase5-ordinary-source-tail.md) and its evidence links.

## Test impact

`add`: five gated public lifecycle tests (the business-terminal test covers eight
consumption/terminal combinations). `update`: two existing channel fixtures close
the producer at EOF; two Drop example assertions and the Rust driver regression
require completion. `retain`: first-terminal, retry, cleanup, telemetry, raw
started-stream contracts and historical evidence. `delete`: none.

# #96 Decide poll settlement for two zero-wait deferred fetches

The #55 replay matches the original-lock #89 case in all four receipts.
Both SDKs dispatch twice on one handle and deliver the same projected
results and settled telemetry. Retain the result-time telemetry difference
for the finite zero-wait case.
[#55 original case](55-observe-deferred-wait0-pending-then-ready.md) ·
[Targeted replay](../evidence/phase5-deferred-wait0-pending-then-ready-replay.json)

## Result and poll timing

Each call dispatches once with wait zero and the same handle, including
`pollAfterMs: 25`. The first response is `deferred` with the original
handle, empty content and zero usage. The second is `stop` with text
`ready`, no handle, usage 2 input, 3 output, 4 cache read and 5 cache
write, and cost 0.25.

| Fetch | Pi at return | Opi at return | Both at the settled checkpoint |
| --- | --- | --- | --- |
| Pending | Open poll, first-chunk duration | Settled poll, both durations | One settled poll with first-chunk and request duration, no token or cost samples |
| Ready | Open poll, first-chunk duration | Settled poll, both durations, four token samples and one cost sample | One settled poll with the same duration sample definitions, token counts and cost |

The comparator projects out elapsed duration values. The observation
covers in-memory recorder visibility and the scripted dispatch.
[Pi driver](../tools/observe_upstream.mjs#L806) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1517) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L740)

## Decision

At Pi `f9a26365568127461c47a07a37dcb42c140a3ac5`, the public helper awaits
the outer stream result, which resolves on the first terminal push. The
forwarder continues through source EOF and the source result before
finishing the poll. Pi `71023aec1546d6d677a65669c5665c0927ce0dd0`
already used that result boundary. The relevant 1.5.0 sources are
`packages/ai/src/models.ts:950–975`,
`packages/ai/src/utils/event-stream.ts:43–47` and
`packages/ai/src/api/lazy.ts:37–53`. Opi's AIR-043 collector released the
source at the terminal. The current drain and deferred poll integration
were added in `1495260d1d3be40cab957b5558da57fe8494d937` and
`80e706b9e2f93b21cd185929dd0d4947d044e010`.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history)

Keep `fetch_deferred` as the completion entry. For these finite sources,
one await gives the caller the outcome, releases the SDK-owned source
and settles the enabled poll observer when recording succeeds. Reading
the recorder immediately after the await can use the complete poll.
[Public helper](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Collector](../../../../crates/opi-ai/src/stream.rs#L560)

Durable's `poll` phase classifies the returned message. A pending message
saves the handle and derives the next `pollAt` from `pollAfterMs`. A ready
message can write an answer or start a tool round. See
`packages/durable/src/harness/generation.ts:323–338,758–799` at the
pinned revision. The #75 Harness probe on the same Pi snapshot and lock
reproduced the second fetch and answer write while both source tails were
held. The probe uses Faux's default wait and a 1 ms poll hint. #55 sets
wait zero and a 25 ms hint explicitly.
[#75 caller evidence](75-review-deferred-result-consumer.md#the-caller-and-the-held-tail-proof)

On Tokio, a result-only port uses `fetch_deferred_ready`. The worker sends
the first normalized outcome before it drains the source and finishes
the poll. The #76 held-EOF pair replays with the ready result visible
while both polls are open. The existing Rust controls also exercise a
pending message with the handle at that boundary. With an immediately
available tail, the worker may finish before the awakened caller reads
the recorder. Callers who need a settled poll can await completion.
[#76 readiness proof](76-return-deferred-outcome-before-source-release.md#evidence) ·
[Readiness method](../../../../crates/opi-ai/src/models.rs#L1172) ·
[Worker notification](../../../../crates/opi-ai/src/stream.rs#L584)

The decision covers the result-time telemetry in #55.
[#97](97-decide-deferred-fetch-held-eof.md) records the subsequent
decision on direct fetch at held EOF in #56. Both local cancellation
cases, #60 and the residual #67 observations remain open. Real deferred
integration and the #59 and #64 error mapping remain blocked by #94.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#deferred-结果式就绪与-poll-结算) ·
[#94 integration gap](94-record-real-deferred-integration-gap.md)

## Verification

- The [#89 material manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and bound executable verify before and after execution. Three Pi
  runs and one Rust run match #89 exactly. Their traces and comparison
  match the original #55 record. The targeted receipt SHA-256 is
  `0a0ab03d5d64dd4370dca4b6a05af0e52f99ee224b76065d2a2205766feadad1`.
  The raw case remains `difference` with zero comparator mismatches
  and `strict_alignment: false`.
- The #75 probe reproduces its frozen receipt after verification of
  its driver and pinned sources. The four #76 receipts on #89 materials
  match the frozen readiness control. Its shared comparison passes.
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
  Test impact `retain` for existing cases and focused controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The spec, index, lifecycle decision, bilingual transport docs and
differential README are carried in and touched. The carried-in #55
and #95 issues link the decision. This issue and replay receipt are new.

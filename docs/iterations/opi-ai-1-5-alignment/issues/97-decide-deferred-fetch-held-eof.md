# #97 Decide the deferred fetch boundary with source EOF held

The #56 completion case and the #76 readiness control match the
original-lock #89 record in all eight receipts. Keep the direct
`fetch_deferred` completion boundary for the observed successful
fetch. Use `fetch_deferred_ready` on Tokio for the result-only
`Models.fetchDeferred` use in Durable.
[#56 original replay](56-observe-deferred-result-before-source-eof.md) ·
[#76 readiness control](76-return-deferred-outcome-before-source-release.md) ·
[Fresh receipts](../evidence/phase5-deferred-held-eof-fetch-replay.json)

## Result, source and poll

Each run dispatches once with the same seeded handle, `wait = 0` and
`pollAfterMs: 25`. The terminal gate and EOF gate control one fetch
in each run. They separate a source that has yet to deliver its result
from a source that is still held after delivering `Done`.

| Checkpoint | Pi `fetchDeferred` | Opi `fetch_deferred` | Opi `fetch_deferred_ready` |
| --- | --- | --- | --- |
| Before `Done` | Pending | Pending | Pending |
| After `Done` with EOF held | Returns `stop` with text `ready` | Pending | Returns the same ready result |
| After EOF release | Source reaches EOF, runs `finally` once and exposes its result once. Poll settles. | Source reaches EOF and is dropped, poll settles, and the fetch returns the same ready result. | Worker reaches EOF, drops the source and settles the poll. The caller already has the result. |

At both held checkpoints the source is retained and the poll span is
open with one first-chunk duration sample. After release, all three
paths show one settled `pi.ai.poll` child span with `stop: stop`,
two duration samples, four token samples and one cost sample. The
projected results and settled samples match. Elapsed duration values
are validated by the drivers and excluded from equality. The two Rust
traces differ only in result availability at the held EOF checkpoint.
[Pi driver](../tools/observe_upstream.mjs#L840) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1582) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L769)

## Caller and decision

Pi `f9a26365568127461c47a07a37dcb42c140a3ac5` resolves the outer
result on the terminal push. Its forwarding task continues through
source EOF and the source result read, then finishes the poll. Pi
`71023aec1546d6d677a65669c5665c0927ce0dd0` already used early delivery.
See `packages/ai/src/models.ts:950–975`,
`packages/ai/src/utils/event-stream.ts:43–47` and
`packages/ai/src/api/lazy.ts:37–53` in 1.5.0. Opi's AIR-043 collector
released at the terminal. This iteration added EOF draining and then
bound deferred poll settlement to that drain.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history)

The direct Rust helper drives `ChatStream::result()` in the caller's
future. Normal completion releases the SDK-owned source before
settling an enabled poll observer and returning. This boundary gives
callers that sequence cleanup before their next operation one await for
the result and release. A held tail keeps the await pending, including
at zero wait. `wait_ms` sets the provider's long-poll budget, while the
SDK-owned tail is driven until completion or local cancellation. The
readiness entry instead sends the first normalized outcome from the
Tokio worker, which then uses the same collector to finish.
[Fetch entries](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Collector and readiness notification](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Provider wait parameter](../../../../crates/opi-ai/src/deferred.rs#L45)

At the pinned Pi revision, Durable restores the model from its saved
provider and model ID, then awaits a fetch with the saved handle, task
signal and telemetry. Its classifier persists a pending handle and
computes the next `pollAt` from `pollAfterMs`. A ready message can
write an answer or start a tool round. See
`packages/durable/src/harness/generation.ts:323–338,758–799,858–871,1013–1055`.
The frozen Harness probe with MemoryStorage and a wrapped Faux
reproduced a second fetch of the same handle and an answer commit
while both source tails were held. Both sources finished after
release. This probe uses the provider's default wait and a 1 ms
poll hint.
[#75 application caller](75-review-deferred-result-consumer.md#the-caller-and-the-held-tail-proof) ·
[Durable probe](../tools/observe_deferred_result_consumer.mjs#L44)

Record the direct entry's result timing in #56 as an intentional
difference. The cleanup guarantee is useful to a caller that requires
it. A direct port of Durable to that entry would defer its answer or
next poll until source release. The readiness entry preserves the
observed business continuation point while the worker owns cleanup.
The decision covers the successful terminal and controlled natural
EOF release. [#98](98-decide-deferred-local-cancel-after-done.md)
records the subsequent decision on #57 local cancellation.
#58, #60 and the residual #67 observations remain open. The #94
real integration blocker and the original #59 and #64 differences
still apply.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#deferred-结果式就绪与-poll-结算) ·
[#94 integration gap](94-record-real-deferred-integration-gap.md)

## Verification

- The [#89 manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and executable verified before and after replay. Three Pi and one
  Rust run per case match the full #89 records. Their traces and
  comparator outputs match the original #56 and #76 cases.
- #56 remains `difference` and #76 remains `pass`, both with zero
  comparator mismatches. The Durable probe also matches its original
  receipt. The new evidence SHA-256 is
  `257185dfa2f8aadb183a5426e1ae36e7e2c152c5d34c7c4a28577f64e2ce8b72`.
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
  Test impact `retain` for existing differential and Durable controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The spec, index, lifecycle decision, bilingual transport docs and
differential README are carried in and touched. The carried-in #56
and #96 issues link the decision. This issue and replay receipt are new.

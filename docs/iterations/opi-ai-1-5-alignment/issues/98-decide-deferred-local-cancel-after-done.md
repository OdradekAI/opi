# #98 Decide local deferred cancellation after Done

All four fresh #57 receipts match the original-lock #89 case.
Keep Opi's local release and poll settlement when cancellation wins
against a pending source tail after a successful `Done` is cached.
The ready result and explicit refetch of the same handle agree on both
sides. Each remote cancel entry sees zero calls.
[#57 original case](57-observe-deferred-local-cancel-after-done.md) ·
[Fresh paired receipts](../evidence/phase5-deferred-local-cancel-after-done-replay.json)

## Result and cleanup

Each SDK dispatches one fetch with the same handle, zero wait and a
25 ms poll hint. The scripted sources hold the EOF gate after `Done`.
Both backends receive the caller's cancellation state, and both
sources keep waiting for the gate on their own.

| Checkpoint | Pi `fetchDeferred` | Opi `fetch_deferred` |
| --- | --- | --- |
| `Done` with EOF held | The caller has `stop` with text `ready`. Source and poll remain open. | The caller is still waiting. Source and poll remain open. |
| Local cancellation with EOF held | The result stays ready. The forwarded signal is aborted, and the source and poll remain open. | The SDK drops the source before natural EOF, settles the poll with `stop: stop` and returns the cached ready message. |
| Cleanup and explicit refetch | Releasing the gate runs source `finally` and reads its result once. The poll settles, and an explicit fetch returns the same ready message. | The EOF gate remains closed through cleanup and refetch. An explicit fetch returns the same ready message. |

The final poll records match, including the span attributes, the two
duration sample definitions, four token counters and one cost counter.
Elapsed duration values are projected out. The replay observes release
of the SDK-owned stream. Adapter work that requires natural EOF or owns
its own task has a separate lifetime.
[Pi driver](../tools/observe_upstream.mjs#L840) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1582) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L813)

## Caller and decision

Pi `f9a26365568127461c47a07a37dcb42c140a3ac5` carries the caller
signal into the provider options and forwards the source through EOF.
The source controls how it responds to the signal. See
`packages/ai/src/models.ts:856–898,950–975` and
`packages/ai/src/api/lazy.ts:37–53`. Pi 1.3.0 at
`71023aec1546d6d677a65669c5665c0927ce0dd0` has the same forwarding
boundary in `packages/ai/src/models.ts:849–891,924–945` and
`packages/ai/src/api/lazy.ts:31–39`.

Opi's SDK-owned token wrapper dates to AIR-043 at
`4b3600fcdeb3ade41bf4f66249337c382630af23`. Its current effect on
post-terminal cleanup follows from the later EOF drain and observer
integration. The wrapper races the pending read against the token.
The collector completes that drain and settles the poll from its
cached message. Keeping this behavior lets a caller stop waiting for
a stalled local stream without discarding a result it already has
or giving up the ability to fetch the remote job again. The existing
readiness control also exercises cancelling a held local tail after
the ready result was delivered to its caller.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history) ·
[Rust wrapper](../../../../crates/opi-ai/src/models.rs#L1101) ·
[Collector](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Readiness control](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1196)

Durable passes its task signal into fetch and checks that signal at
the start of classification. Its task abort handler separately calls
`cancelDeferred` when the checkpoint retains a deferred handle.
The handler settles the task as aborted, and reports a remote cancel
failure separately. See the pinned
`packages/durable/src/harness/generation.ts:323–338,390–416,582–596,758–769`.
The frozen Durable tests cover explicit cancellation during polling
and reporting a failed cancel while ending the run as aborted. This
keeps app-level abort and settlement with the app even when the SDK
has a successful cached result.
[#92 Durable test receipts](../evidence/phase5-deferred-route-authority-observations.json)

Record #57's local cleanup and poll timing as an intentional difference.
The decision covers cancellation with a successful terminal cached
and a pending tail. [#99](99-decide-deferred-local-cancel-before-done.md)
records the subsequent decision on #58 cancellation before a terminal.
The #94 real-integration blocker and the original #59 and #64
differences still apply.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#deferred-结果式就绪与-poll-结算) ·
[#94 integration gap](94-record-real-deferred-integration-gap.md)

## Verification

- The [#89 manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and bound executable verified before and after replay. Three fresh
  Pi runs and one Rust run match the full #89 case. The traces and
  comparison also match the original #57 record.
- The raw status remains `difference` with zero comparator mismatches
  and `strict_alignment: false`. The new evidence SHA-256 is
  `79eda6bc6bdfaa4839fae1483e7b0b3b7413902495826f0516b4af221b94370a`.
- The prior Durable controls apply to the same snapshot. Their recorded
  inputs and the held-tail probe driver revalidate by hash.
  `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
  Test impact `retain` for existing differential and caller controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The spec, index, lifecycle decision, bilingual transport docs and
differential README are carried in and touched. The carried-in #57
and #97 issues link the decision. This issue and replay receipt are new.

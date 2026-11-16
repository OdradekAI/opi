# Compare deferred stream completion and source cleanup

The public streaming replay and the Rust started control established the
baseline. #68 changed the started path and #70 changed the direct EOF path.
[#101](101-decide-deferred-stream-result-and-drop.md) replays the current
case and records the result timing, source ownership and poll disposition.
[Iteration](../spec.md) ·
[#66 fetch review](66-review-deferred-fetch-result-and-source-cleanup.md) ·
[#68 started drain](68-drain-started-deferred-source-tail.md) ·
[#70 direct EOF drain](70-drain-deferred-source-before-direct-eof.md)

## Observations

The paired drivers reuse the ready job and separate terminal and EOF
gates from #56. They read `Start`, release the terminal, read `Done`,
and ask the same public iterator for EOF while the source gate is held.
Pi releases the gate and awaits the existing result. Rust also drives
the same collector's `result()` and records its state at the held EOF
gate before release. A separate Rust call uses `.start()`. Its source
reaches the terminal gate before the caller consumes any events.
[Pi driver](../tools/observe_upstream.mjs#L912) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1750) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211)

| Path | At public `Done` | At public EOF | Source completion |
| --- | --- | --- | --- |
| Pi `Models.streamDeferred` | Forwarder reaches the held EOF gate. The poll is open. | The gate is still held and the poll remains open with one first-chunk sample. | Releasing the gate produces natural source EOF, one iterator `finally`, one source result read and a settled poll. |
| Rust `Models::stream_deferred`, direct iteration | Source is retained. The poll is open. | The poll is settled with all seven measurements. The source is retained and the EOF gate has yet to be entered. | `result()` on the same collector enters the gate and stays pending. After release the source reaches natural EOF and is dropped before result return. The poll record stays complete with seven measurements. |
| Rust `Models::stream_deferred(...).start()` | The source has been dropped and the poll is settled. The EOF gate has yet to be entered. | The source drop and poll record are already observed. | `StartedChatStream::result()` returns the cached ready result. The source was dropped at its business terminal before natural EOF. |

All three calls deliver one successful `Done` and an equal projected
ready result. Each dispatches once with the original handle and zero wait.
All remote cancel counts are zero. The final poll attributes, event and
measurement values agree. The durations are validated and their elapsed
values are omitted from equality. The named gates establish ordering.

## Source and assessment

Frozen Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5`
uses `packages/ai/src/models.ts:950–975`,
`packages/ai/src/utils/event-stream.ts:43–84` and
`packages/ai/src/api/lazy.ts:37–53`. A terminal push closes public
iteration while the forwarder continues reading its source. Poll
settlement follows that forwarder's source completion.

In the Opi baseline, `ChatStream::poll_next` cached and delivered the terminal
and finished the observer when the consumer next read EOF. Direct
`ChatStream::result()` drove the retained source. The started control used the
generic `ChatStream::start()` finisher `result_without_tail`, which finished
the observer and released the collector before the worker sent the public
terminal.
The generic started path was preserved when
`1495260d1d3be40cab957b5558da57fe8494d937` added the direct EOF drain.
`80e706b9e2f93b21cd185929dd0d4947d044e010` reused that collector for
deferred streaming. The transport guide describes the current caller
boundaries.
[Current iterator](../../../../crates/opi-ai/src/stream.rs#L619) ·
[Direct result](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Started collector](../../../../crates/opi-ai/src/stream.rs#L435) ·
[Worker finalization](../../../../crates/opi-ai/src/execution.rs#L127) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

This exposed a consequence of reusing the generic started collector.
A backend's asynchronous tail work can complete through direct
`result()`. The original started call dropped the stream before entering
that tail. #68 checked callers and gave the started execution task
responsibility for driving and releasing the finite source tail after
consumer Drop. The direct iterator's settlement difference
remains a separate public behavior to evaluate. The probe covers
call-owned scripted sources and in-memory telemetry.

## Verification

- [Material manifest](../evidence/phase5-deferred-stream-source-cleanup-materials.json)
  `6438772aeb2e6f8cd14ba3222e56e32454bafb6637be2222ebcc101fa5874d9a`.
  [Paired replay](../evidence/phase5-deferred-stream-source-cleanup-differential.json)
  `843248cffc0c95afe26e0a88cd1990d6671b01b491ea6d25581fae3249ac6617`.
- The original-lock Pi snapshot, installed dependencies, toolchains
  and both Rust library source trees match #65. The changed bindings
  are the two drivers and comparator. The #65 manifest returns
  `STALE_MATERIALS` with exit 2 before either driver or build starts.
- All 69 Pi and 23 Rust receipts match stdout and SHA-256. Each Pi
  triplet is identical. The prior 22 case records match #65 exactly.
  All 23 shared comparisons pass. Three cases pass outright, ten have
  previously accepted scoped differences and ten deferred comparisons
  have open differences. The raw verdict is `DIFFERENCES_REQUIRE_DECISION`
  with exit 0, zero material blockers and `strict_alignment: false`.
- The differential suite passed 73 Python tests, including the real Rust
  subprocess check. The lifecycle example passed five Rust tests and
  Clippy with `-D warnings`. Node syntax, Cargo format, documentation and
  diff checks passed.

Test impact `add` for the paired streaming case, the started control and
comparator regressions. `update` for subprocess and aggregate comparison
assertions. `retain` for the prior 22 cases and the SDK regressions.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` is `ee18dd1323a089d536bdc7e293d0b4869777c344`. The drivers,
comparison tests, guides and iteration records are carried in and
touched. Source links were refreshed for moved anchors. This issue and
its two evidence files are new.

# Observe first chunk duration recording failure on a terminal first event

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [request span start failure](51-observe-request-span-start-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #51 worktree inputs

## Frozen replay

`models-first-chunk-duration-record-failure` extends the
[pinned Pi source observation](../evidence/phase3-ai-telemetry-observations.json)
to the compiled Pi public `Models.streamSimple` and Rust's
`Models::stream_simple`. Each controlled provider holds its first event, a
successful `Done`, and has a separate gate before natural source EOF. A
fresh real recorder backs each adapter. Both create the histogram and
record `pi.ai.first_chunk` on the request span. The first
`pi.ai.first_chunk.duration` histogram record fails with
`controlled-first-chunk-duration-record-failure` after the source emits
`Done` and before source EOF. Each public `result()` is read after public
iteration reaches EOF.
[Pi driver](../tools/observe_upstream.mjs#L590) · [Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1065)

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Before `Done` | One dispatch, public `next()` pending and request span open | Same |
| Source exit after the fault | Iterator `return()` completes once and the generator runs `finally`. The EOF gate is never entered. Natural EOF is false and source `result()` calls are zero | Reaches the held EOF gate. Gate release produces natural EOF and source Drop |
| Public terminal and result | The first `next()` is ready after iterator return. Its sole `Done` has `stop`, followed by outer EOF and `result()` fulfilled with the original provider message | The first `next()` is pending at the EOF gate. After release and Drop it delivers its sole successful `Done`, outer EOF and matching `Ok` result |
| Request span | Settled with the recorded first chunk event, `stop: error` and `provider_error` status carrying the injected marker | Already settled at the held EOF gate with the first chunk event, absent `stop` and recorder status `ok`. The final checkpoint confirms one backend finish and wrapper Drop |
| Duration measurements | Zero first chunk samples. One request duration sample with `stop: error` | Zero first chunk or request duration samples |
| Host diagnostic | Zero `unhandledRejection` events through the final checkpoint with the listener installed | One tracing WARN with the original record error |

In Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5`, the
forwarder pushes `Done` before `onEvent` (`packages/ai/src/api/lazy.ts:37–52`).
`EventStream.push` seals delivery and resolves the successful message at
that push (`packages/ai/src/utils/event-stream.ts:43–57`). The histogram
throw follows the successful event write (`packages/ai/src/telemetry.ts:155–161`).
The forwarding loop closes the source iterator. Its catch tries to push a
synthetic error into the sealed outer stream and observes that error as the
telemetry result (`api/lazy.ts:68–83`). The result observer records the
error stop, status and request duration before finishing the span
(`telemetry.ts:181–208`).

Rust reports the typed recording error and closes the observer with
`finish(None)`. The execution still drains the source tail and delivers the
original business terminal after release. The recorder retains its default
span status, the first chunk event and the missing final attributes.
[Rust event recording](../../../../crates/opi-ai/src/telemetry.rs#L230) · [Automatic observer](../../../../crates/opi-ai/src/telemetry.rs#L445) · [Terminal delivery](../../../../crates/opi-ai/src/execution.rs#L125)

## Accepted decision

On 2026-11-11 the user replied “继续推进” to the recommendation to keep
Rust's source drain and observer fault isolation in this case. The accepted
`first-chunk-duration-record-failure` difference applies to one typed
`TelemetryError` from `pi.ai.first_chunk.duration` recording after the
first chunk event has been written on the first source item, a successful
`Done`, at the public Models boundary. It covers terminal timing, source
exit, span status and attributes, duration samples and host diagnostics.
Rust delivers the successful terminal after natural EOF and source release.
The WARN identifies the telemetry fault. The finished span retains the
first chunk event with no `stop`, and the request duration is unrecorded.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#首事件为-done-时的首块时长记录故障)

## Verification

- [Material manifest](../evidence/phase5-first-chunk-duration-record-failure-materials.json), SHA-256 `1bbdc6d728bada6737413ff133668fb83a06b0162e265f697453382615f2628b`.
- [Replay commands and raw traces](../evidence/phase5-first-chunk-duration-record-failure-differential.json), SHA-256 `bdc0b806081de7734a4a5f38d22e4bc6599cd0c645a5d04009c953d27a04094b`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 33 TypeScript and eleven Rust invocations on Windows x64. Each
  TypeScript triplet has identical stdout. All eleven shared comparisons
  pass with zero material blockers. Two cases pass and all nine recorded
  differences have documented decisions. The raw verdict remains
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- All ten prior case records match #51. Both standalone Pi `lazyStream`
  completion cases match their six #42 stdout streams.
- The #51 manifest is rejected before build and replay with
  `STALE_MATERIALS` / exit 2 for the two drivers and comparator. Upstream
  source, original and installed locks, catalog, installed identities,
  generated files, Rust library trees and toolchains match #51.
- The Python differential suite passed 41 tests, including comparator
  checks against the frozen trace and the Rust subprocess cases. The
  cached Rust example passed five tests. Example Clippy with `-D warnings`,
  Node syntax, Cargo format, documentation and diff checks passed.
- The decision update reverified the frozen material inputs, toolchain,
  executable, both evidence digests and all 44 invocation records. The
  decision index covers the nine cases with observed differences.

Test impact `add` for the paired recording failure and comparator
regressions for source exit, the histogram and terminal/result identity.
`update` for the Rust subprocess assertions. `retain` for the ten prior
replays, standalone Pi completion and existing Rust example regressions.
Test impact for the decision update is `none`.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The
drivers, comparator, Python tests, differential README, spec, decision and
index were carried in and updated. Earlier issue source links were
updated. This issue and its two JSON records are new untracked paths.
The replay exercises existing production code.

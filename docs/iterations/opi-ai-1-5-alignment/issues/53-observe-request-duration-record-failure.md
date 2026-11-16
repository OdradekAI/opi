# Observe a request duration recording failure after source EOF

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [first chunk duration recording failure](52-observe-first-chunk-duration-record-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #52 worktree inputs

## Frozen replay

`models-request-duration-record-failure` extends the
[pinned Pi source observation](../evidence/phase3-ai-telemetry-observations.json)
to public `Models.streamSimple` and `Models::stream_simple`. Each controlled
provider emits `Start` and holds its successful `Done` at a gate. Both
adapters delegate instrument creation and the first chunk measurement to
a real in-memory recorder. The first `pi.ai.request.duration` histogram
record fails with `controlled-request-duration-record-failure` after
natural source EOF and after `stop` and `usage.input` have been recorded.
[Pi driver](../tools/observe_upstream.mjs#L647) · [Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1166)

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| At the fault | One dispatch and one source terminal. Source reaches natural EOF, executes `finally` once and calls its own `result()` once. Zero iterator returns | One dispatch and one source terminal. Natural EOF and source Drop have both occurred |
| Public terminal and result | Successful `Done`, outer EOF, and a fulfilled `result()` with the original provider message | Successful `Done`, outer EOF, and matching `Ok` from `result()` |
| Request span at final checkpoint | Open with the first chunk event, `stop: stop`, result usage and recorder status `ok` | Open with the same observed stop, first chunk event and recorder status. Zero backend finish calls and wrapper Drop observed |
| Recorded measurements | One first chunk duration sample. Zero request duration, token and cost samples | Same |
| Host diagnostic | Zero `unhandledRejection` events through the final checkpoint with the listener installed | One tracing WARN with the original record fault |

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` observes the
result after source EOF and `source.result()` (`packages/ai/src/api/lazy.ts:37–52`).
Its result observer writes final attributes and calls the request duration
histogram before the token and cost counters and the span completion
callback (`packages/ai/src/telemetry.ts:181–208`). The throw leaves that
callback pending. The forwarding catch keeps the outer stream sealed on
its original `Done`. The result observation guard has already been set
and skips the second result observation (`api/lazy.ts:68–83`).

In the Rust baseline, `AiRequestObserver::finish` reported the original
record error and dropped the observer. The backend wrapper dropped
without a finish attempt. The explicit completion path covered the
successful request's final attribute write error, and the histogram
fault reached the `FinishError::Other` branch.

## Repair

This replay is the baseline for the open request span after the business
call and source have completed. #54 reuses the explicit completion path
for this single typed recording fault and verifies the span settles once
with the written result attributes and original WARN. Source release and
the successful business result match this baseline.
[#54 implementation and decision](54-finish-request-span-after-duration-record-failure.md)

## Verification

- [Material manifest](../evidence/phase5-request-duration-record-failure-materials.json), SHA-256 `3301d151802b8bd3a51719b637b7bc1ced7cbb57f5cddf33284907b22ca56019`.
- [Replay commands and raw traces](../evidence/phase5-request-duration-record-failure-differential.json), SHA-256 `aeef3a5b88130bacd4c277e889eb0910ad4330b14e27293d5c9f334907896938`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 36 TypeScript and twelve Rust invocations on Windows x64.
  Each TypeScript triplet has identical stdout. All twelve shared
  comparisons pass with zero material blockers. Two cases pass and ten
  record differences, nine of which have prior decisions. The raw verdict
  is `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- All eleven prior case records match #52. Both standalone Pi `lazyStream`
  completion cases match their six #42 stdout streams.
- The #52 manifest is rejected before build and replay with
  `STALE_MATERIALS` / exit 2 for the two drivers and comparator. Upstream
  source, original and installed locks, catalog, installed identities,
  generated files, Rust library trees and toolchains match #52.
- The Python differential suite passed 44 tests. The cached Rust example
  passed five tests. Example Clippy with `-D warnings`, Node syntax, Cargo
  format, documentation and diff checks passed.

Test impact `add` for the paired recording failure and comparator
regressions for the fault stage, source release, prior measurements and
observed settlement. `update` for the shared probe fixture and Rust
subprocess assertions. `retain` for eleven prior replays, standalone Pi
completion and the existing Rust example regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The drivers,
comparator, Python tests, differential README, spec, decision and index
were carried in and updated. Earlier issue source links were updated.
This issue and its two JSON records are new untracked paths. The replay
exercises existing production code.

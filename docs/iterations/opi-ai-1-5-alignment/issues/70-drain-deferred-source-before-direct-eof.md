# Drain a directly consumed deferred source before public EOF

Direct iteration delivers the cached business terminal as soon as it is
available. The following `next()` drives the retained source to release,
settles the enabled poll observer, and returns public EOF.
[#69 stop baseline](69-observe-deferred-direct-consumer-stop.md) ·
[#68 started drain](68-drain-started-deferred-source-tail.md) ·
[Iteration](../spec.md)

## Implementation

The deferred constructor selects tail draining at completion. Direct
`next()`, `ChatStream::result()`, and the projection wrapper share the
poll-based source drain. It discards source items after the cached
terminal and releases the source at EOF. A direct iterator drives this
work from the caller's existing poll context. The started deferred
execution still uses `result()` as its finisher.
[Deferred constructor](../../../../crates/opi-ai/src/models.rs#L1037) ·
[Tail drain](../../../../crates/opi-ai/src/stream.rs#L330) ·
[Result](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Direct iteration](../../../../crates/opi-ai/src/stream.rs#L622)

For a finite tail, reaching public EOF now gives a caller a released
source and a settled normal poll. The tail contributes to the wait for
EOF, while the business terminal is already available. When the local
fetch token wins against a pending tail, the source wrapper ends the
local attempt and the drain preserves the first terminal.
[Local cancellation](../../../../crates/opi-ai/src/models.rs#L1101) ·
[English guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[中文指南](../../../../crates/opi-ai/docs/transport.zh.md#deferred-transport-语义)

## Replay

| Rust control | Source EOF gate held | Completion |
| --- | --- | --- |
| Direct iteration to EOF | One public `Done` is already delivered. The next read enters the gate and stays pending. The source is retained and the poll is open with one measurement. | Gate release yields natural source EOF and Drop. The poll settles with seven measurements before public EOF. Further reads return EOF and `result()` returns the cached ready result. |
| Direct stop after public EOF | The final read waits at the same gate. | Natural EOF, source Drop and poll settlement precede the caller dropping the collector. |
| Local cancel during the final read | The terminal is cached. The caller and backend tokens are active and the final read is pending. | Cancelling the token releases the source with the EOF gate still held. Public EOF follows a single poll settlement. The cached ready result matches the natural completion. |

The control that drops the direct collector at `Done` still ends the
local source before entering its tail. The in-memory poll in that
control remains open. The started and detached controls match #69.
Each Rust control resolves auth and dispatches once with the original
handle and zero wait. Remote cancel counts are zero.

Frozen Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` still
delivers public EOF while its independent forwarder holds the source
gate (`packages/ai/src/utils/event-stream.ts:43–84`,
`packages/ai/src/api/lazy.ts:37–53`). The Rust change ties full direct
iteration to source release with an explicit EOF wait. The ready
payload and settled measurements match. Public EOF timing and early
consumer stop remain observable differences.
[Pi driver](../tools/observe_upstream.mjs#L912) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1750) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211)

## Verification

- [Original-lock manifest](../evidence/phase5-deferred-direct-eof-drain-materials.json)
  `9afcc4718b27390f1660c6aba8ad69dbaadd65c095b09182204636f1db716fb2`.
  [Paired replay](../evidence/phase5-deferred-direct-eof-drain-differential.json)
  `23cdd83c9d5931f9a4d35b256392f3dc34b33f4bc816ec449c181d50fc055f82`.
  The new probe failed before the runtime change with exit 101 at the
  assertion that direct iteration waits for the retained tail. The #69
  manifest returns `STALE_MATERIALS` with exit 2 before a build or replay
  after the Rust source tree, driver and comparator change.
- All 69 Pi and 23 Rust receipts match stdout, decoded traces and SHA-256.
  All Pi triplets agree. The other 22 cases, the target Pi receipts and
  20 Rust checkpoints match #69 exactly. All 23 shared comparisons pass.
  Three pass outright, ten have previously accepted scoped differences
  and ten deferred comparisons remain open. The replay exits 0 with
  `DIFFERENCES_REQUIRE_DECISION`, zero material blockers and
  `strict_alignment: false`.
- The five integration targets and lifecycle example passed 114 Rust
  tests. The `--lib stream::` filter passed 23 tests, including the
  shared projection drain. The differential suite passed 75 Python
  tests including the live Rust subprocess. Focused Clippy and
  `cargo doc -p opi-ai --no-deps` passed with warnings denied. Cargo
  format, Node syntax, documentation and diff checks passed.

Test impact `add` for local cancellation during the final read and
comparator rejection checks. `update` for the direct EOF and stop
controls. `retain` for the started controls, stop at `Done`, the other
22 cases and existing SDK regressions.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` is `ee18dd1323a089d536bdc7e293d0b4869777c344`. The two library
files, driver, comparator, tests, guides and iteration records are
carried in and touched. Related source links were refreshed. This
issue and its two evidence files are new.

# Compare deferred source cleanup after direct consumption stops

Two controls in `models-deferred-stream-source-cleanup` stop direct
consumption after `Done` and after public EOF. Each uses a fresh call
with a finite source tail held behind the same named EOF gate. Pi calls
`return()` on the public iterator. Rust drops the direct collector.
Neither control awaits the public result or starts another consumer.
[Iteration](../spec.md) ·
[#68 started drain](68-drain-started-deferred-source-tail.md) ·
[#70 direct EOF drain](70-drain-deferred-source-before-direct-eof.md)

## Replay

| Consumer stop | Pi at stop | Pi after source gate release | Rust after Drop and gate release attempt |
| --- | --- | --- | --- |
| After public `Done` | The forwarder is inside the EOF gate. The poll is open with one first-chunk sample. | Natural EOF, one source `finally` and one source result read. The poll is settled with seven measurements. | The source was dropped before entering the gate, with no natural EOF. The in-memory poll stays open with one first-chunk sample. |
| After public EOF | The forwarder is inside the EOF gate. The poll is open with one first-chunk sample. | Natural EOF, one source `finally` and one source result read. The poll is settled with seven measurements. | The source was dropped before entering the gate, with no natural EOF. The in-memory poll had already settled at public EOF with seven measurements. |

The Rust drop closes the gate receiver. The Pi forwarder completes even
after the public iterator returns, with zero source iterator `return()`
calls. Both runtimes resolve auth and dispatch once per control with the
original handle and zero wait. Both deliver matching ready message fields
at `Done` and record zero remote cancels. In the second control both
deliver public EOF while source EOF is still held.
[Pi driver](../tools/observe_upstream.mjs#L912) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1750) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211)

## Origin and next change

Pi `v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0` already ran
the independent forwarder from `packages/ai/src/models.ts:924–945` and
`packages/ai/src/api/lazy.ts:31–59`. In frozen `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/models.ts:950–975` installs the deferred observer and
`packages/ai/src/api/lazy.ts:37–53` observes the result after source
iteration and source result retrieval. Public iterator closure and
source iteration are independent in both revisions.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history)

The early Rust collector at `4b3600fcdeb3ade41bf4f66249337c382630af23`
absorbed the terminal, dropped the source, and returned permanent public
EOF on the next poll (`crates/opi-ai/src/stream.rs:704–744`).
`1495260d1d3be40cab957b5558da57fe8494d937` retained the source for
`result()` and added observer settlement to that pre-existing public EOF
branch (`stream.rs:467–490,511–515`). Deferred inherited both through
`80e706b9e2f93b21cd185929dd0d4947d044e010`
(`crates/opi-ai/src/models.rs:1024–1043`). The resulting poll state depends
on one additional consumer poll, even when both calls discard the same
unfinished tail. This comes from the earlier collector boundary and the
later telemetry hookup. The frozen Pi forwarder ties settlement to its
own completion.
[Current direct iterator](../../../../crates/opi-ai/src/stream.rs#L622) ·
[Result drain](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Current transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

The #70 follow-up validates sharing the retained-source drain with the
final `next()` of a directly consumed deferred stream. The caller can
receive `Done` promptly, then drive the finite tail to release before
public EOF and normal poll settlement. Measure the cost at the held EOF
gate and verify local cancellation can end that wait. Public EOF would
then come later than in Pi. An early break at `Done` still ends a
consumer-owned direct call, so callers who need detached completion can
use the explicitly started execution from #68. Preserving immediate
public EOF while continuing the tail would require transferring source
ownership to a task, with an execution-runtime and cancellation contract.
The replay keeps both direct timing and cleanup differences open.
[Execution ownership decision](../decisions/11-telemetry-lifecycle.md#answer)

## Verification

- [Original-lock manifest](../evidence/phase5-deferred-direct-consumer-stop-materials.json)
  `c8a72b4f6f424efc50b0fbf97dd6aa96d421e21b6709e379b7e4dbb7a2f726f5`.
  [Paired replay](../evidence/phase5-deferred-direct-consumer-stop-differential.json)
  `0d38c436d86e638f9c04b9781e066012caf89b556702a3aef22e1679192e1c51`.
  The Rust source trees, Pi snapshot and toolchains match #68. The #68
  manifest returns `STALE_MATERIALS` with exit 2 before a build or replay
  because the two drivers and comparator changed.
- All 69 Pi and 23 Rust receipts match recorded stdout and SHA-256.
  Each Pi triplet is identical. The other 22 cases and the four Pi and
  16 Rust checkpoints of the #68 target are identical. All 23 shared
  comparisons pass. Three cases pass outright, ten have previously
  accepted scoped differences, and ten deferred comparisons remain
  open. The replay exits 0 with `DIFFERENCES_REQUIRE_DECISION`, zero
  material blockers and `strict_alignment: false`.
- The lifecycle differential suite passed 74 Python tests including
  the live Rust subprocess. The lifecycle example passed six Rust
  tests and focused Clippy with warnings denied. Node syntax, Cargo
  format, documentation and diff checks passed. The library source and
  existing SDK test inputs match the focused regressions verified in #68.

Test impact `add` for the two paired stop controls and their comparison
assertions. `update` for the live Rust subprocess checks. `retain` for
the 22 other cases, the #68 target checkpoints and existing SDK tests.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` is `ee18dd1323a089d536bdc7e293d0b4869777c344`. The two
drivers, comparator, tests, guides and iteration records are carried in
and touched. Related source links were refreshed. This issue and its
two evidence files are new.

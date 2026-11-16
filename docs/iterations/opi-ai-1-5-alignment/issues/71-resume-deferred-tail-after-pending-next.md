# Resume a deferred source after dropping a pending `next()`

After `Models::stream_deferred` delivers `Done`, dropping a pending final
`next()` future keeps the source and the terminal cache in the retained
`ChatStream`. Both `result()` and `.start()` can take over the same tail.
[#70 direct EOF drain](70-drain-deferred-source-before-direct-eof.md) ·
[Iteration](../spec.md)

## Controlled replay

Each Rust control polls the final `next()` with a no-op waker and drops
the borrowed future once it returns `Pending`. The first notification
confirms entry to the EOF gate. The source is still live and the poll
is open with one first-chunk measurement. The next owner must reach the
second notification on the same gate before the driver releases it.
The release wakes that owner.
The detached control drops the started handle before the task's first
poll on the current-thread executor.
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2037) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211)

| Owner after the pending read is dropped | With the EOF gate held | After release |
| --- | --- | --- |
| `result()` | The result future repolls the source and stays pending. | The source reaches natural EOF and is dropped. The poll settles before the cached ready fields are returned. |
| `.start()` with a retained handle | The task repolls the source and the next public read stays pending. | The first started read reaches EOF after source Drop and poll settlement. `result()` returns the cached ready fields. |
| `.start()` with immediate consumer Drop | The task repolls the source after the consumer has gone. | The task reaches natural EOF, releases the source and settles the poll. |

All three controls use the original handle with `wait_ms = 0`. Each
resolves auth and dispatches once, leaves the caller and backend
cancellation tokens uncancelled, records zero remote cancels and settles
one poll with seven measurements. The retained consumers' ready fields
match the initial `Done`. The started iterator produces no second
terminal. The product source trees match #70. The English and Chinese
transport guides now document this handoff.
[English guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[中文指南](../../../../crates/opi-ai/docs/transport.zh.md#deferred-transport-语义)

## Verification

- [Original-lock manifest](../evidence/phase5-deferred-next-wait-resume-materials.json)
  `bc3f80519f24355a6b2481e23ddd427aa8548e77e6f589d4a494548775dd7105`.
  [Paired replay](../evidence/phase5-deferred-next-wait-resume-differential.json)
  `01d9cd4d1b3017ec25cd0484514a03d827c018ac3dcd6f4304e7bec94514f830`.
  The original Pi `v1.5.0` lock and installed snapshot match #70.
  Only the Rust driver and comparator inputs changed. The #70 material
  manifest returns `STALE_MATERIALS` with exit 2 before building or
  running the drivers.
- All 69 Pi and 23 Rust receipts match the stdout hashes and decoded
  traces. The other 22 case objects, target Pi receipts and 30 prior
  Rust checkpoints match #70. The target adds 16 Rust checkpoints.
  All 23 shared comparisons pass. Three pass outright, ten have
  previously accepted scoped differences and ten deferred differences
  remain open. The replay exits 0 with `DIFFERENCES_REQUIRE_DECISION`,
  zero material blockers and `strict_alignment: false`.
- `cargo test -p opi-ai --example lifecycle_differential --offline -j4`
  passed all six tests. The differential suite passed 77 Python tests
  including a live Rust subprocess for every scenario and rejection
  tests for missing repoll evidence, early settlement, duplicate
  terminal or dispatch, cancellation and missed source Drop. Example
  Clippy passed with warnings denied. Cargo format, documentation and
  diff checks passed.

Test impact `add` for the resumption controls and comparator tests.
`update` for the source-cleanup fixture binding. `retain` for the 30
prior target checkpoints, the other 22 cases and the #70 SDK regressions.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` is `ee18dd1323a089d536bdc7e293d0b4869777c344`. The driver,
comparator, test fixture binding, guides and iteration records are
carried in and touched. Related source links were refreshed. This
issue and its two evidence files are new.

Next, use the ownership and timing observations to resolve the Rust
mapping of Pi's eager stream, including public EOF timing and the
cleanup responsibility when the caller drops the entire direct
collector. Real-provider wire behavior and six-gate acceptance remain
open.

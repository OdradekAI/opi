# Compare two zero-wait fetches of one deferred handle

Status: replay complete, finite zero-wait telemetry timing accepted
Date: 2026-11-12
Parent: [alignment specification](../spec.md)
Predecessor: [request duration span completion](54-finish-request-span-after-duration-record-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #54 worktree inputs

[#96 replay and decision](96-decide-deferred-wait0-poll-settlement.md)

## Observations

The public `Models.fetchDeferred` and `Models::fetch_deferred` paths
each receive the same handle twice under the original-lock Pi `v1.5.0`
materials. Both scripted backends receive one dispatch per fetch with
wait `0` and the same handle value, including its `pollAfterMs: 25`
advice. Each driver waits for the first result and takes a bounded
recorder checkpoint before making the second call.
[Pi driver](../tools/observe_upstream.mjs#L806) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1517)

| Fetch | Public result on both sides | Settled telemetry on both sides |
| --- | --- | --- |
| First | `deferred`, empty content, original handle, zero usage buckets and cost | One `pi.ai.poll` child span with `stop: deferred`. One first-chunk and one request duration sample tied to that span. Zero token or cost counter samples. |
| Second | `stop`, text `ready`, no handle, usage buckets 2 input, 3 output, 4 cache read and 5 cache write, cost 0.25 | One `pi.ai.poll` child span with `stop: stop`. Both duration samples, four token samples and one cost sample with those values. |

Immediately after each public await, Pi has an open poll span and
only the first-chunk duration sample. Its span settles and the rest of
the measurements appear by the next event-loop checkpoint. Rust has a
settled span and every measurement at each return. The replay reports
`deferred-poll-telemetry-at-result` as a timing difference with
matching dispatches, projected result fields and settled telemetry.
The drivers validate finite nonnegative duration samples and omit
their elapsed values from the comparison.

## Source and trade-off

At Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/models.ts:950–975` implements the helper through
`streamDeferred(...).result()`. `utils/event-stream.ts:43–47` resolves
the result at the first terminal push. `api/lazy.ts:37–53` observes the
result after the source iterator ends and its result is read.
`telemetry.ts:178–208` records final measurements and resolves the
observer callback, which the in-memory adapter settles in
`packages/telemetry/src/memory.ts:232–245`.

The Rust helper consumes `ChatStream::result()`, which drains and
releases the source before completing its owned observer and returning.
The deferred observer boundary was chosen in #10 to share that
settlement path. The ordering gives callers a complete poll record
when they read telemetry immediately after a successful await.
Retaining it is reasonable for these finite sources. The
[held-EOF follow-up](56-observe-deferred-result-before-source-eof.md)
checks how the source tail affects result availability.
[Phase 4 integration](10-deferred-ai-telemetry.md) ·
[Rust public helper](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Rust settlement](../../../../crates/opi-ai/src/stream.rs#L560)

The observation covers two explicit zero-wait calls on one seeded
handle and finite scripted streams. The compared message fields are
stop, provider/model, content, handle, usage buckets and cost. Real
provider HTTP behavior, deferred cancellation, positive waits,
held-EOF behavior and complete six-gate acceptance remain open.

## Verification

- [Material manifest](../evidence/phase5-deferred-wait0-materials.json), SHA-256 `9f47b29ea1d4e62369ea529c33d6196a2ce1bf522b829dd09ab5519fac15512d`.
- [Paired replay and raw traces](../evidence/phase5-deferred-wait0-differential.json), SHA-256 `70328ca7e956501ad525e3cd661a8353104730bd5410e6ffcb065eb498a2b2f0`.
- The original-lock Windows x64 candidate supplied 39 TypeScript
  invocations and 13 Rust invocations. Each TypeScript triplet has
  identical stdout. The 12 previous case records match #54. All 13
  shared comparisons pass with zero material blockers. Two cases
  pass and eleven retain differences, ten previously accepted and one
  open. The raw verdict is `DIFFERENCES_REQUIRE_DECISION` / exit 0,
  with `strict_alignment: false`.
- The #54 manifest returns `STALE_MATERIALS` / exit 2 before build or
  case execution. Only the two drivers, comparator and runner differ
  among bound inputs. Source, locks, installed and generated inputs,
  toolchain and Rust library trees match #54.
- The Python differential suite passed 47 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --test deferred_contract --test ai_telemetry --offline -j4`
  passed 41 tests. Clippy on the same targets with `-D warnings`, Cargo
  format, documentation and diff checks passed.

Test impact `add` for the paired wait0 case and comparator regressions.
`update` for the Rust subprocess assertions. `retain` for the prior
replay cases and the existing deferred and telemetry regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`.
The paired drivers, comparator, runner, Python tests, iteration
index and specification, and differential README were carried in
and updated. Earlier issue links were refreshed to the moved driver
anchors. This issue and its two evidence JSON files are new.

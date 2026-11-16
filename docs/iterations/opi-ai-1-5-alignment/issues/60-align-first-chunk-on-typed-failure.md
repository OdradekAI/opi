# Record first-chunk telemetry on the first typed failure

Status: implemented. [#100](100-decide-deferred-auth-setup-failure.md) records the auth failure delivery and diagnostic decision.  
Parent: [opi-ai 1.5.0 alignment](../spec.md)  
Predecessor: [#59 remote cancellation and refetch](59-verify-deferred-remote-cancel-then-fetch.md)

## Observed behavior

`models-deferred-auth-setup-failure` calls the public fetch helper once with
`wait = 0`, the route and normalized handle fields from #59, and an explicit
caller telemetry context. The registered resolver enters once. While its gate
is held, both fetches are pending with one open `pi.ai.poll` child span and
zero measurements. Releasing the gate fails authentication. Both dispatch and
remote-cancel counts stay zero. The two failure messages identify the route
and contain `stop: error`, empty content, no handle and zero usage and cost.
[Pi driver](../tools/observe_upstream.mjs#L1042) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2199)

| First delivered failure | Backend fetch dispatches | Frozen Pi | Rust baseline | Rust repaired |
| --- | ---: | --- | --- | --- |
| Auth rejected before dispatch (#60) | 0 | One first-chunk event and duration | Zero | One event and duration |
| Cancelled handle fails on explicit refetch (#59) | 1 on the refetch | One first-chunk event and duration | Zero | One event and duration |

In both cases the completed poll span retains the caller parent, route,
`stop: error`, and `provider_error` status. Request duration sample counts
and zero-valued token and cost measurements agree. Each affected Rust trace
gains one first-chunk event and duration sample. The comparator requires those
observations on both sides and the drivers check that durations are finite
and nonnegative.
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L708)

## Source and decision

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` defines
`pi.ai.first_chunk` and its duration by the first normalized stream event.
Its README includes setup errors and its telemetry test asserts the event
on a setup failure. `lazyStream` calls the same observer after pushing a
forwarded provider event or a synthetic setup error. The definition and
observation were introduced together in `2ec55d808`.
`packages/ai/README.md:1134`, `packages/ai/src/telemetry.ts:41,77–83,155–161`,
`packages/ai/test/telemetry.test.ts:224–244`,
`packages/ai/src/api/lazy.ts:37–53,74–81`.

Rust already had the typed failure branch in `b2ff2f398b8d2416f0284ac7a77731338cf0a436`.
`1495260d1d3be40cab957b5558da57fe8494d937` added the matching metric definition
and attached the observer to the `Ok` branch. The missing measurements
followed that wiring. The two replays demonstrate the same gap on both
sides of the dispatch boundary. Using the existing metric for first delivery
keeps its sample population consistent with Pi, including fast failures.
The linked span carries the stop reason and error status.

`ChatStream` now passes the normalized stream item to the observer when it
delivers it. Both ordinary events and typed failures share the observer's
one-time first-delivery recording. The ordinary chat driver also observes its
typed setup failure before settling the span, since that branch precedes
collector construction. The Rust stream and result keep `Err(CallFailure)`.
The failure after an earlier `Start` in #58 still contributes exactly one
first-chunk sample for that poll.
[Collector delivery](../../../../crates/opi-ai/src/stream.rs#L698) ·
[First-delivery recording](../../../../crates/opi-ai/src/telemetry.rs#L230) ·
[Ordinary setup delivery](../../../../crates/opi-ai/src/models.rs#L5257) ·
[Earlier Start case](58-observe-deferred-local-cancel-before-done.md)

Pi fulfills the two fetches with error messages. Rust returns typed
`AuthFailed` on auth rejection and the scripted backend's `Config` on
refetch. The result channels, cause and diagnostic text remain in the
named differences. The auth resolver example is controlled, and the
`Config` cause is specific to the Rust refetch fixture.
[#95 auth failure replay](95-replay-deferred-auth-setup-failure.md) ·
[#100 Durable verification and decision](100-decide-deferred-auth-setup-failure.md)

## Verification

| Run | Material manifest and SHA-256 | Replay and SHA-256 |
| --- | --- | --- |
| Before repair | [Manifest](../evidence/phase5-deferred-auth-setup-failure-baseline-materials.json) `608b2d31d8a807c9e97f9cc428fdc90d19d718fb9e7e2a225936769a69abcfde` | [Replay](../evidence/phase5-deferred-auth-setup-failure-baseline-differential.json) `d174177af1328a5fabe1d75c631d2e45837d777107a05117f76e64cc26e06789` |
| After repair | [Manifest](../evidence/phase5-deferred-auth-setup-failure-materials.json) `4e56fb5d8065af4291191c4584402480c7f3633caa44e3849c3a268fa846d85a` | [Replay](../evidence/phase5-deferred-auth-setup-failure-differential.json) `1ceb6384f4b212bc842e83d0df6ecd83ac0c0e6de1ff6b2a86e250f8d9842208` |

- The manifests use the original-lock frozen Pi snapshot and the same
  toolchains as #59. The old #59 manifest rejects the changed drivers
  and comparator. The #60 baseline manifest rejects the repaired Rust
  source tree and updated comparator. Both negative replays return
  `STALE_MATERIALS` / exit 2 before any driver or build runs.
- The two replays each record 54 TypeScript and 18 Rust invocations.
  All 144 receipts match their raw stdout and SHA-256, and each TypeScript
  triplet has identical stdout. The baseline preserves all 17 #59 cases.
  The repair preserves 16 cases exactly and both affected Pi traces exactly.
- The ordinary chat setup regression fails before repair on zero first-chunk
  events. The current comparator also rejects both frozen Rust failures
  before repair and a duration sample removed from the repaired trace.
  [Ordinary regression](../../../../crates/opi-ai/tests/ai_telemetry.rs#L113)
- The final replay passes all 18 shared comparisons with zero material
  blockers. Two cases pass, ten have accepted scoped differences and six
  deferred comparisons have open differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The Python differential suite passed 62 tests. Cached Rust `--lib`
  tests filtered to `stream::` passed 23 tests. The `ai_telemetry`,
  `custom_provider_registration`, `deferred_contract`, `execution_lifecycle`,
  `telemetry_operations` and `lifecycle_differential` targets passed 118 tests.
  Clippy on the library and those targets with `-D warnings`, Cargo format,
  Node syntax, documentation and diff checks passed.

Test impact `add` for the auth replay, comparator regressions and ordinary
setup regression. `update` for the refetch first-chunk assertion and Rust
subprocess coverage. `retain` for the other 16 case records and focused Rust
regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The issue,
four frozen evidence files and the Rust telemetry test changes belong to this
slice. The carried-in runtime, drivers, comparator, docs and changelog were
updated. The [#61 replay](61-verify-deferred-positive-wait.md) covers positive
wait forwarding and controlled backend outcomes. Live-provider behavior and
six-gate acceptance remain open.

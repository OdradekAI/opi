# Compare deferred handle expiry at the public fetch boundary

Status: paired replay complete. [#91 records the expiry decision](91-defer-expiry-to-provider-by-default.md).  
Parent: [opi-ai 1.5.0 alignment](../spec.md)  
Predecessor: [#61 positive wait](61-verify-deferred-positive-wait.md)

## Observations

The Pi driver submits through public Models and the compiled Faux provider.
Faux creates the real job ID and route. The driver copies the returned handle
and adds `expiresAt: 1700000000000` in epoch milliseconds. The registered
provider receives that exact handle. The evidence canonicalizes only the
random ID. Rust seeds a ready job with the same normalized handle in
`ScriptedDeferredBackend`. Each fetch requests `wait=0`.
[Pi driver](../tools/observe_upstream.mjs#L1115) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2319)

| Call | Rust `now_ms` | Pi auth / fetch / result | Rust auth / fetch / result |
| --- | --- | --- | --- |
| One millisecond before expiry | `1699999999999` | 1 / 1 / ready | 1 / 1 / ready |
| At expiry | `1700000000000` | 1 / 1 / ready | 0 / 0 / `Err(CallFailure)` |
| Without SDK clock | `None` | 1 / 1 / ready | 1 / 1 / ready |

The Rust clock values define the checkpoints. Pi uses the same options and
handle on all three calls. At expiry the Rust error has a `Config` cause and
a `stop: error` message with the handle's provider and model IDs, empty
content and zero usage. Its poll span settles with the expiry diagnostic
and both duration samples. The successful controls match on the ready
payload and settled poll measurements. All remote cancel counts are zero.
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1025)

## Source and assessment

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` declares the
optional `expiresAt` field and resolves auth before forwarding the handle
to the provider. Faux looks up the stored job by ID and checks route and
cancellation. It returns the ready job in this probe.
`packages/ai/src/types.ts:539–549`, `packages/ai/src/models.ts:950–974`,
`packages/ai/src/providers/faux.ts:591–656`.

Opi added caller-clock expiry in `4b3600fcdeb3ade41bf4f66249337c382630af23`
under AIR-043. The archived `docs/research/pi-ai-opi-ai-alignment/spec-draft.md:201`
requires expired-handle validation. Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0` already forwarded the handle
after auth and used ID, route and cancellation checks in Faux. The
early refusal traces to Opi's own dispatcher contract.
`packages/ai/src/types.ts:524–534`, `packages/ai/src/models.ts:924–945`,
`packages/ai/src/providers/faux.ts:586–651` at `v1.3.0`.

Rust checks `now_ms >= expires_at_ms` after route validation and before
resolving auth. `fetch_deferred` collects that existing failure into
`Err(CallFailure)`. The direct validation helper exposes
`DeferredError::Expired`. The previous rustdoc described the caller's
reading as monotonic and the deadline as epoch milliseconds. Both inputs
to the comparison now explicitly use Unix epoch milliseconds in rustdoc
and the transport guides. The check itself uses the existing code.
[Time input](../../../../crates/opi-ai/src/api/contract.rs) ·
[Expiry check](../../../../crates/opi-ai/src/types/deferred.rs#L61) ·
[Dispatch order](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

Keep the local check available as an opt-in for callers with an authoritative
provider deadline and a comparable clock. It avoids an auth resolution and
a backend dispatch for that expired request. A clock ahead of the provider
or a provider that still serves the result past the hint can make the local
refusal premature. Omitting `now_ms` delegates the expiry decision to the
backend. The equality case changes both the result and dispatch. The
`deferred-handle-expiry` comparison retains that difference, with its
disposition recorded in [#91](91-defer-expiry-to-provider-by-default.md). Real provider expiry
and wire behavior remain open.

## Verification

- [Material manifest](../evidence/phase5-deferred-handle-expiry-materials.json)
  `548c2f136c69b6d6bc2aade7a61bd248f3bdce2d308244dbe7fdde6398ab13b9`.
  [Paired replay](../evidence/phase5-deferred-handle-expiry-differential.json)
  `77aaaaa6682c9c6a9bef912c5ac5e9f56f7ff8b8ae9a276758090758cdfc6dec`.
- The replay uses #61's original-lock Pi snapshot, installed dependencies
  and toolchains. The two drivers, comparator and Rust source tree bindings
  reflect the new case and rustdoc corrections. The #61 manifest returns
  `STALE_MATERIALS` / exit 2 before launching a build or either driver.
- All 60 TypeScript and 20 Rust raw receipts match stdout and SHA-256. Each
  TypeScript triplet is identical. The prior 19 case records match #61
  exactly. All 20 shared comparisons pass. Three cases pass outright,
  ten have previously accepted scoped differences and seven deferred
  comparisons have open differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with zero material blockers and
  `strict_alignment: false`.
- The differential suite passed 66 Python tests, including the real Rust
  subprocess test. Cached `deferred_contract`, `ai_telemetry` and the
  `lifecycle_differential` example passed 42 Rust tests. Example Clippy and
  crate rustdoc with `-D warnings`, Cargo format, Node syntax, documentation
  and diff checks passed.

Test impact `add` for the paired expiry case and comparator regressions.
`update` for the Rust subprocess coverage and aggregate difference assertion.
`retain` for the 19 prior cases and focused Rust deferred and telemetry tests.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The new issue
and two evidence files belong to this slice. The expiry rustdoc is updated
on two previously clean paths. Drivers, comparison tests, guides and
iteration records were carried in and updated.
[#63 compares auth and dispatch for a mismatched handle route.](63-compare-deferred-handle-route-mismatch.md)

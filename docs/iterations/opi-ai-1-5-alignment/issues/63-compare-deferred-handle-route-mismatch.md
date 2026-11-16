# Compare deferred handle route mismatches

Status: paired replay complete. [#92 records the fetch and cancel decision](92-validate-deferred-routes-before-dispatch.md).  
Parent: [opi-ai 1.5.0 alignment](../spec.md)  
Predecessor: [#62 handle expiry](62-observe-deferred-handle-expiry.md)

## Observations

The Pi driver submits through public Models to compiled Faux. Rust seeds a
ready job with the same normalized handle in `ScriptedDeferredBackend`.
The fixture has one registered deferred route. Each of the first three
zero-wait fetches changes one of `provider`, `modelId` or `api` on a copy
of the original handle. The job ID stays the same. Pi receives the original
model argument on all calls and Faux receives the exact input handle.
The fourth fetch uses the original handle.
[Pi driver](../tools/observe_upstream.mjs#L1115) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2319)

| Handle change | Pi auth / backend fetch | Rust auth / backend fetch | Rust direct validation |
| --- | --- | --- | --- |
| Provider to `other-provider` | 1 / 1 | 0 / 0 | `Unsupported` during registration lookup |
| Model to `other-model` | 1 / 1 | 0 / 0 | `RouteMismatch` against the registered model |
| API to `other-api` | 1 / 1 | 0 / 0 | `Unsupported` during registration lookup |
| Original handle | 1 / 1 | 1 / 1 | `Ok(())` |

All three mismatches produce `stop: error`, empty content, no returned
handle and zero usage on both sides. Pi fulfills the Promise with Faux's
`Unknown faux deferred response` message. Rust returns `Err(CallFailure)`
with a `Config` cause. Pi labels the error message and settled poll span
from the selected model. Rust uses the submitted handle's provider and
model for its synthesized message and the submitted provider, model and
API for the poll span. Each failure records a first-chunk event and both
duration samples. The original handle returns the same ready payload and
settled telemetry on both sides. All remote cancel counts are zero.
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1074)

## Source and assessment

At frozen Pi `v1.5.0` revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
Models selects the provider from the explicit model, resolves auth and
forwards the handle. Faux looks up the job ID and compares its stored
provider, model and API to that handle. The same order appears in Pi
`v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0`.
`packages/ai/src/models.ts:950–974` and `packages/ai/src/providers/faux.ts:591–612`
at `v1.5.0`. `packages/ai/src/models.ts:924–945` and
`packages/ai/src/providers/faux.ts:586–607` at `v1.3.0`.

Opi's `4b3600fcdeb3ade41bf4f66249337c382630af23` introduced handle-based
route validation under AIR-043. The registry selects by exact `ApiId` and
requires the registration's provider to match. A resolved registration
then checks the model. `Models::validate_deferred_handle` exposes the
validation type directly. `fetch_deferred` maps these failures to `Config`.
Rustdoc and both transport guides now describe that lookup boundary.
[Registration lookup](../../../../crates/opi-ai/src/models.rs#L1219) ·
[Direct validation](../../../../crates/opi-ai/src/models.rs#L995) ·
[Fetch setup](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

Keep pre-dispatch validation of locally resolvable route constraints. In
this fixture it avoids auth resolution and backend dispatch for each
invalid input and the original handle remains usable. This replay's
unknown-API diagnostic attributes the failure to the provider, which
supports deferred responses on the registered API.
[#65 corrects the lookup diagnostic.](65-clarify-deferred-api-lookup-diagnostic.md)
The result channel, diagnostic, identity and dispatch differences remain
recorded as `deferred-handle-route-mismatch`. [#92](92-validate-deferred-routes-before-dispatch.md) verifies
the cancellation effect and keeps pre-auth validation. Real provider
wire behavior needs separate evidence.

## Verification

- [Material manifest](../evidence/phase5-deferred-handle-route-mismatch-materials.json)
  `814549471db7968c893f91f899bba9696f6db7678b6dc2b3609864d058feed06`.
  [Paired replay](../evidence/phase5-deferred-handle-route-mismatch-differential.json)
  `09bb4eb6c2537f449af60f357a65df40f828800e4ba8c10e34a922716e1b522d`.
- The replay uses #62's original-lock Pi snapshot, installed dependencies
  and toolchains. The new case changes both driver bindings and the
  comparator binding. Rustdoc changes update the Rust source tree binding.
  The #62 manifest returns `STALE_MATERIALS` / exit 2 before either driver
  or build starts.
- All 63 TypeScript and 21 Rust raw receipts match stdout and SHA-256.
  Each TypeScript triplet is identical. The prior 20 case records match
  #62 exactly. All 21 shared comparisons pass. Three cases pass outright,
  ten have previously accepted scoped differences and eight deferred
  comparisons have open differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with zero material blockers and
  `strict_alignment: false`.
- The differential suite passed 68 Python tests, including the real Rust
  subprocess test. Cached `deferred_contract`, `ai_telemetry` and the
  `lifecycle_differential` example passed 42 Rust tests. Example Clippy and
  crate rustdoc with `-D warnings`, Cargo format, Node syntax, documentation
  and diff checks passed.

Test impact `add` for the paired route mismatch case and comparator
regressions. `update` for Rust subprocess coverage and the aggregate
difference assertion. `retain` for the 20 prior cases and the focused
Rust deferred and telemetry tests.

Worktree handoff `committed=none-for-this-task`, `staged=none`. This issue
and its two evidence files are new. The drivers, comparison tests, rustdoc,
guides and iteration records were carried in and updated.
[#64 compares fetching an unknown job ID through a valid route.](64-compare-deferred-unknown-job-id.md)

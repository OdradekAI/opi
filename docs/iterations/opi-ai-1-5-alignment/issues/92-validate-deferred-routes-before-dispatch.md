# #92 Validate deferred routes before fetch and cancel

Keep Opi's pre-auth checks for the handle's provider, model and API.
The SDK can resolve those fields against its registered deferred routes.
After dispatch, the backend owns the job ID, its availability and the
effect of cancellation. The current public methods already implement
this boundary.
[Direct validator](../../../../crates/opi-ai/src/models.rs#L995) ·
[Fetch dispatch](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Cancel dispatch](../../../../crates/opi-ai/src/models.rs#L1186)

## Durable recovery

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` stores two
values in the poll checkpoint. The `ModelRef` comes from the request and
contains `provider` and `modelId`. The handle comes from the latest deferred
result and carries its own route, including `api`. SQLite writes and reads
the whole task state as JSON. At the next poll, Durable resolves the saved
model reference through the current `Models.getModel(provider, id)`, then
passes the saved handle verbatim. A missing model settles fetch as
`unanswered/no_model`. The abort path resolves the same saved reference
for best-effort cancel, returns before cancel if it is missing, and
reports a provider cancel failure separately from the local abort.

The paths at the 1.5.0 pin are
`packages/durable/src/harness/types.ts:55–59`,
`packages/durable/src/harness/generation.ts:184–187,258–281,323–337,582–613,767–788`,
and `packages/durable/src/storage/sqlite/storage.ts:73–74,513–516,904–918`.
Pi 1.3.0 at `71023aec1546d6d677a65669c5665c0927ce0dd0` also uses the saved
reference and handle in `packages/durable/src/harness/generation.ts:236–244,265–275,457–475`.

The current model record can carry a new API for the same saved provider
and model ID. Pi's `Models` selects the provider from that record and
resolves auth before forwarding the handle. For a provider with multiple
API implementations, the wrapper selects by the record's `model.api`.
The saved handle keeps its own API. These are distinct inputs at
`packages/ai/src/models.ts:490–492,843–854,950–987,1106,1164–1186`.
The catalog lookup by itself supplies no evidence that a new API can
retrieve or cancel a job created by the old API.

## Paired dispatch and results

The current original-lock replay of #63 matches its full #89 case record.
Each mismatch changes exactly one field of a copied handle and keeps the
original job ID. The fixture has one registered route and the Pi call
uses its resolved model for every input. Fresh cancel rows use a new job
per case and refetch with the intact handle afterward.
[#63 fetch case](63-compare-deferred-handle-route-mismatch.md) ·
[Frozen cancel probe](../tools/observe_deferred_cancel_route.mjs#L16) ·
[Rust cancel regression](../../../../crates/opi-ai/tests/deferred_contract.rs#L1084) ·
[Paired receipts](../evidence/phase5-deferred-route-authority-observations.json)

| Changed handle field | Pi fetch auth / dispatch / result | Opi fetch auth / dispatch / result | Pi cancel auth / dispatch / result | Opi cancel auth / dispatch / result |
| --- | --- | --- | --- | --- |
| Provider | 1 / 1 / fulfilled error message | 0 / 0 / `Err(CallFailure)`, `Config` | 1 / 1 / fulfilled `void` | 0 / 0 / `Err(Config)` |
| Model | 1 / 1 / fulfilled error message | 0 / 0 / `Err(CallFailure)`, `Config` | 1 / 1 / fulfilled `void` | 0 / 0 / `Err(Config)` |
| API | 1 / 1 / fulfilled error message | 0 / 0 / `Err(CallFailure)`, `Config` | 1 / 1 / fulfilled `void` | 0 / 0 / `Err(Config)` |

Direct Opi validation returns `Unsupported` for the provider and API
mutations because their provider/API registration is absent. The model
mutation finds the registration and returns `RouteMismatch`. The exact
diagnostic matches between fetch and cancel for each mutation.

All three Pi mismatched cancels mark the job cancelled in Faux. The
subsequent fetch with the intact handle returns the provider's cancelled
job error. The same refetch on Opi returns the ready answer. In the
matching-handle control, both sides authenticate and dispatch once,
cancel successfully and then report the job cancelled. With a valid
route and an unknown ID, both authenticate and dispatch cancel once.
The controlled backends acknowledge the call and the intact handle
still returns the ready answer. The retained #64 fetch observes a
backend error after one auth resolution and one fetch on each side.
[#64 unknown job ID](64-compare-deferred-unknown-job-id.md)

At the 1.5.0 pin, Faux fetch compares the handle against the stored
route after finding the ID. Faux cancel finds the ID and marks the
entry without a route comparison. See
`packages/ai/src/providers/faux.ts:591–665`. Pi 1.3.0 has the same
fetch/cancel split at lines 586–660. Opi's route check originated in
`4b3600fcdeb3ade41bf4f66249337c382630af23` for AIR-043. The decision to
keep it follows the known local route contract and the observed
cancel side effect.

## Local route and provider authority

The handle selects the deferred registration in Opi. An application
with a separately saved expected route can compare against that route
before dispatch. A complete handle that identifies another valid
registration is routed there. The registry check establishes local
route consistency. Job ownership, retrieval and cancellation remain
backend decisions.

For a changed catalog record, an intact handle can still use its original
registered deferred API. A migration to a different API requires a
provider-defined compatible route for the original job. A missing
provider/API registration or a mismatched registered model stays a
local error. Preserve the early rejection as an intentional difference
in dispatch and cancellation outcome.
[Transport guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[中文](../../../../crates/opi-ai/docs/transport.zh.md)

## Verification

The #89 original-lock manifest verifies before and after with zero
drift and zero blockers. Three fresh Pi fetch traces and one Rust
trace match the complete #89 case. Three Pi cancel runs are
byte-identical. The missing-snapshot invocation exits 2. The five
cancel rows agree across Rust feature configurations.

The frozen Durable suite passes four focused tests, covering SQLite
reopen, the missing saved model, abort during polling, and independent
reporting of cancel failure. Rust `deferred_contract` passes 18 tests
with default and no default features. Affected-target Clippy with
warnings denied, formatting, documentation and diff checks pass.
The 24-case record still carries ten raw deferred differences.
Real-provider behavior and six-gate acceptance remain open.
[Material manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-websocket-idle-timeout-differential.json)

Test impact `add` for the cancel probe and paired Rust regression.
Test impact `retain` for the bound fetch probe, existing route and
cancellation tests, and frozen Durable tests.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The Rust test target, iteration brief, contract index, prior issue links,
differential guide and bilingual transport guides were carried in and
updated. The cancel probe, paired receipts and issue are new files.

# #91 Leave deferred result availability to the provider by default

Preserve the current Opi contract. Pass the full handle to the registered
backend when `DeferredFetchCall::now_ms` is omitted. A caller can opt into
local rejection when the provider specifies a hard deadline for retrieving
the result and the caller has a clock on the same time base. A processing
deadline or clock skew can otherwise make local rejection discard a
result that the provider still serves.
[English transport guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[Chinese transport guide](../../../../crates/opi-ai/docs/transport.zh.md)

## Producers and consumers

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` declares
`expiresAt?: number` in `packages/ai/src/types.ts:539–549`. It originated
in `382aa641cc4c197dfa95ed684f187b5a39bc30ce`, which added deferred
dispatch and its Faux implementation. The Faux submit branch sets the route, job ID
and optional `pollAfterMs` at `packages/ai/src/providers/faux.ts:548–564`.
It leaves expiry absent. The fetch branch checks stored ID, route and
cancellation, then returns pending or ready. The explicit cancel branch
marks the stored job at lines 591–665. Faux is the pinned repository's
only built-in implementation of deferred submit, fetch and cancel.
A custom provider that issues a handle can supply the optional expiry metadata.
Pi 1.3.0 at `71023aec1546d6d677a65669c5665c0927ce0dd0` has the same
field and behavior in `types.ts:524–534`, `providers/faux.ts:543–559`,
and `models.ts:924–957` under `packages/ai/src`.

Pi's public `Models` resolves auth and forwards the entire handle for
fetch and cancel (`packages/ai/src/models.ts:950–987`). The coding-agent
request facade likewise forwards it
(`packages/coding-agent/src/core/model-runtime.ts:763–796`). Durable
stores the latest handle in its `poll` checkpoint. It computes `pollAt`
from `pollAfterMs` and the host clock, then calls `fetchDeferred` with
the stored handle when due. Restart recovery can schedule the same
handle again. Explicit cancellation forwards that handle independently.
These are the paths in `packages/durable/src/harness/generation.ts:301–337`,
`:582–595`, and `:770–788` at 1.5.0. SQLite stores the task state as JSON
in `packages/durable/src/storage/sqlite/storage.ts:73–74,513–516,907–918`.
The reopening test keeps the same Faux backend across SQLite reopen and
retrieves the answer on one fetch at `pollAt`
(`packages/durable/test/harness-generation-recovery.test.ts:257–289`).

Opi's scripted `submit` leaves `expires_at_ms` empty.
`submit_with_expiry` copies a fixture-supplied absolute Unix epoch
millisecond value. The in-tree implementation is the scripted
backend. Opi exposes `DeferredApi` registration for custom backends.
The Rust reopen fixture serializes the entire handle, uses a fake
clock for the next poll and passes that time as `now_ms`. Its test job
has no expiry. The explicit expiry tests supply a deadline.
[Scripted submission](../../../../crates/opi-ai/src/test_support.rs#L786) ·
[Reopen fixture](../../../../crates/opi-ai/tests/deferred_contract.rs#L771) ·
[Explicit expiry test](../../../../crates/opi-ai/tests/deferred_contract.rs#L441)

## Same-material replay

The existing #62 probe confirms that Faux omitted `expiresAt` and adds
`1700000000000` to the returned handle. Rust's scripted submit uses
the same deadline. Both backends have the result ready. The phase
names describe Opi's injected clock. Pi receives the same handle in
all three calls and records no SDK clock input. Every admitted fetch
uses the original route, normalized ID, deadline and `wait=0`.
[Pi fixture](../tools/observe_upstream.mjs#L1142) ·
[Rust fixture](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2350)

| Opi clock input | Pi auth / fetch / result | Opi auth / fetch / result |
| --- | --- | --- |
| Deadline minus 1 ms | 1 / 1 / `stop`, text `ready` | 1 / 1 / `stop`, text `ready` |
| Exactly at the deadline | 1 / 1 / `stop`, text `ready` | 0 / 0 / `Err(CallFailure)` with `Config` |
| Omitted | 1 / 1 / `stop`, text `ready` | 1 / 1 / `stop`, text `ready` |

The result payload and provider dispatch differ only for the opt-in equality
case. Opi first validates the route, then checks expiry at
`now_ms >= expires_at_ms`, then resolves auth for admitted calls. The
direct validator exposes `DeferredError::Expired`. All three fetch
entries share the check. Explicit remote cancel uses its own route,
auth and cancellation checks before dispatch. The replay
records zero remote cancellations.
[Expiry predicate](../../../../crates/opi-ai/src/types/deferred.rs#L61) ·
[Dispatch](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Cancel](../../../../crates/opi-ai/src/models.rs#L1186)

The opt-in check originated in Opi commit
`4b3600fcdeb3ade41bf4f66249337c382630af23` for AIR-043. Both pinned Pi
revisions delegate availability to the provider. The original-lock
Anthropic SDK `0.129.0` provides an example of why the adapter owns
this decision. In `resources/messages/batches.d.ts:146–173`,
`expires_at` ends batch processing and `archived_at` describes when
results become unavailable. The time needed for a local fetch refusal
is the provider's result retrieval cutoff. Record the Opi opt-in as an
intentional difference in dispatch and result.

## Verification

Three frozen TypeScript runs are byte-identical. The fresh Rust run
uses the executable whose SHA-256 matches the #89 build receipt. The
full case, including the comparator and all four raw receipts, matches
the existing #89 case exactly. The original-lock manifest has zero
drift before and after replay and zero material blockers. The frozen
Vitest case `resumes polling a deferred response after reopen` passes.
All 17 Rust `deferred_contract` tests pass, including handle serialization,
local expiry before auth and fetch, and the reopened consumer.
The existing 24-case record retains ten raw deferred differences.
Real-provider expiry and six-gate acceptance remain open.
[Targeted replay](../evidence/phase5-deferred-expiry-authority-replay.json) ·
[Material manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json) ·
[24-case replay](../evidence/phase5-codex-websocket-idle-timeout-differential.json)

Next, evaluate route validation at the fetch and cancel boundaries
against provider identity and the caller's restored model.
[#63 route mismatch](63-compare-deferred-handle-route-mismatch.md) ·
[#92 route decision](92-validate-deferred-routes-before-dispatch.md)

Test impact `retain` for the existing expiry probe, deferred contract
tests and frozen Durable reopen test. This slice adds the targeted
replay and decision using the #89 manifest.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The iteration brief, contract index, prior issue links, differential
guide and bilingual transport guides were carried in and updated.

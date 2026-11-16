# Verify positive deferred fetch waits

Status: verified for the public SDK boundary with controlled backends.  
Parent: [opi-ai 1.5.0 alignment](../spec.md)  
Predecessor: [#60 first-chunk telemetry](60-align-first-chunk-on-typed-failure.md)

## Observed behavior

`models-deferred-positive-wait` uses two explicit calls to Pi `fetchDeferred`
and Rust `fetch_deferred` with the same handle. Each backend receives `50` ms
as the wait budget. The handle carries `pollAfterMs: 25` as the scheduling
hint. Each call dispatches once and both remote cancel counts stay zero.
The fixture holds the backend stream before its first event and releases a
scripted outcome for each call.
[Pi driver](../tools/observe_upstream.mjs#L1078) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2259)

| Checkpoint | Result | Poll telemetry |
| --- | --- | --- |
| First provider outcome held | Fetch pending | One open child poll span and zero samples |
| First provider outcome released | `deferred` with the original handle | Settled poll with first-chunk and request duration samples |
| Second explicit fetch held | Fetch pending | One new open child poll span and zero samples |
| Second provider outcome released | Ready message with `stop`, no handle | Settled poll with both duration samples and reported tokens and cost |

The first fetch returns even though the job remains pending. The caller's
second fetch is the only subsequent dispatch. The settled span attributes,
duration sample shapes, token and cost counters agree at the recorded
checkpoints. Real provider wait enforcement and wire behavior remain open.
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1005)

## Source and contract

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` defines `wait` as
the provider's maximum long-poll duration in milliseconds. Its public
Models path resolves auth and forwards the options to the selected provider
once. The pinned provider test asserts forwarding `wait: 50` alongside
`timeoutMs: 100`.
`packages/ai/src/types.ts:274–279`, `packages/ai/src/models.ts:950–974`,
`packages/ai/test/providers.test.ts:624–686`.

Rust passes the `DeferredFetchCall` to the registered backend after handle
validation and auth. Its `wait_ms` field has the same budget meaning. The
transport guides now state which party interprets the budget and which
party initiates a later fetch. The fixture uses a gate for backend response
availability. Actual elapsed provider wait and result delivery around
source EOF are outside the observations in this probe.
[Rust wait option](../../../../crates/opi-ai/src/deferred.rs#L34) ·
[Dispatch path](../../../../crates/opi-ai/src/models.rs#L1091) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Verification

- [Material manifest](../evidence/phase5-deferred-positive-wait-materials.json)
  `4838efb67433beb479737ec52decefb28bf2cee920ec30b458b35ba770ea7f2d`.
  [Paired replay](../evidence/phase5-deferred-positive-wait-differential.json)
  `3cd7f61355fff4a70041dc545f2d1bd45ca4ae70f5c8ffd20be06947f6dd7da5`.
- The replay uses #60's original-lock Pi snapshot, installed dependencies,
  toolchains and Rust SDK source tree. The driver and comparator hashes
  reflect the new case. The #60 manifest rejects those three changed inputs
  as `STALE_MATERIALS` / exit 2 before launching either driver or a build.
- All 57 TypeScript and 19 Rust raw receipts match stdout and SHA-256.
  Each TypeScript triplet is identical. All 18 earlier case records match
  #60 exactly. The new case passes its shared assertions with no observed
  difference. Across 19 cases, three pass, ten have accepted scoped
  differences and six deferred comparisons still have open differences.
  The raw verdict is `DIFFERENCES_REQUIRE_DECISION` / exit 0 with zero
  material blockers and `strict_alignment: false`.
- The differential suite passed 64 Python tests, including the real Rust
  subprocess case. Cached `deferred_contract`, `ai_telemetry` and the
  `lifecycle_differential` example passed 42 Rust tests. Example Clippy
  with `-D warnings`, Cargo format, Node syntax, documentation and diff
  checks passed.

Test impact `add` for the paired case and comparator regressions. `update`
for the real Rust subprocess coverage. `retain` for the 18 prior cases and
focused Rust deferred and telemetry tests.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The new issue
and two frozen evidence files belong to this slice. The drivers, comparison
tests, transport guides and iteration records were carried in and updated.
[#62 compares expired-handle behavior and dispatch at the public boundary.](62-observe-deferred-handle-expiry.md)

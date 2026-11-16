# Compare a deferred fetch with an unknown job ID

Status: paired replay complete. [#93 assesses the result and diagnostic](93-assess-deferred-job-failure-classification.md).  
Parent: [opi-ai 1.5.0 alignment](../spec.md)  
Predecessor: [#63 handle route mismatch](63-compare-deferred-handle-route-mismatch.md)

## Observations

The Pi driver submits through public Models to compiled Faux. Rust seeds
the matching ready job in `ScriptedDeferredBackend`. The first zero-wait
fetch changes only the handle ID to `deferred-missing`. The second fetch
uses the original handle. The trace normalizes the generated Faux ID and
records the unknown ID verbatim. The Rust validator accepts both handles
with zero auth resolutions and backend calls during validation.
[Pi driver](../tools/observe_upstream.mjs#L1115) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2319)

| Fetch | Pi auth / backend fetch | Rust auth / backend fetch | Outcome |
| --- | --- | --- | --- |
| Unknown job ID | 1 / 1 | 1 / 1 | Backend reports missing job |
| Original handle | 1 / 1 | 1 / 1 | Same ready payload and settled telemetry |

Each dispatch carries the exact submitted handle and zero wait. Both
remote cancel counts are zero. The failed message carries `stop: error`,
the requested provider and model, empty content, no returned handle and
zero usage. Both polls settle with the same route attributes, first-chunk
event, duration samples and zero-valued token and cost samples.
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1159)

Pi fulfills its Promise with an error message containing
`Unknown faux deferred response: deferred-missing`. Rust returns
`Err(CallFailure)` with the backend's `ProviderError::Config` and
`invalid provider configuration: Unknown scripted deferred response: deferred-missing`.
The poll status carries the respective diagnostic. This case records the
result channel, fixture cause and message as `deferred-unknown-job-id`.

## Source and assessment

Frozen Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` resolves
auth and dispatches with the supplied handle. Faux reads its job map by ID
and emits an `error` event with the selected route on a missing entry.
Pi `v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0` uses the same
lookup and error text.
`packages/ai/src/models.ts:950–974` and `packages/ai/src/providers/faux.ts:591–654`
at `v1.5.0`. `packages/ai/src/providers/faux.ts:586–649` at `v1.3.0`.

Rust validates the registered route and optional expiry before resolving
auth. The registered backend reads the ID. The scripted backend creates
the `Config` cause in the missing-entry branch introduced by Opi
`4b3600fcdeb3ade41bf4f66249337c382630af23` under AIR-043. `ChatStream`
preserves that cause and uses the requested route when synthesizing a
failure before a partial arrives. The transport guides now document who
looks up the job ID.
[Fetch setup](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Scripted backend](../../../../crates/opi-ai/src/test_support.rs#L875) ·
[Failure collection](../../../../crates/opi-ai/src/stream.rs#L542) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

The replay supports leaving job existence to the backend and preserving
its cause through the Rust result channel. `Config` describes this fixture's
choice. The appropriate cause and retry behavior for a real provider need
evidence from its response contract. #93 verifies Durable's handling of
both diagnostics and retains the observed error delivery difference.

## Verification

- [Material manifest](../evidence/phase5-deferred-unknown-job-id-materials.json)
  `37f07f4d46f12eac0d626f49cb38a3ed7a7be6c8cbb489751b8eba9d42bf2f72`.
  [Paired replay](../evidence/phase5-deferred-unknown-job-id-differential.json)
  `385a47bf73e9ab863f088fb8b1b87cc63d2da83d74c1170586b48b451be42044`.
- The replay uses #63's original-lock Pi snapshot, installed dependencies,
  toolchains and Rust SDK source tree. Both driver bindings and the
  comparator binding include the new case. The #63 manifest returns
  `STALE_MATERIALS` / exit 2 before either driver or build starts.
- All 66 TypeScript and 22 Rust raw receipts match stdout and SHA-256.
  Each TypeScript triplet is identical. All 21 prior case records match
  #63 exactly. All 22 shared comparisons pass. Three cases pass outright,
  ten have previously accepted scoped differences and nine deferred
  comparisons have open differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with zero material blockers and
  `strict_alignment: false`.
- The differential suite passed 70 Python tests, including the real Rust
  subprocess test. Cached `deferred_contract`, `ai_telemetry` and the
  `lifecycle_differential` example passed 42 Rust tests. Example Clippy
  with `-D warnings`, Cargo format, Node syntax, documentation and diff
  checks passed.

Test impact `add` for the paired unknown-ID case and comparator
regressions. `update` for Rust subprocess coverage and the aggregate
difference assertion. `retain` for the 21 prior cases and focused Rust
deferred and telemetry tests.

Worktree handoff `committed=none-for-this-task`, `staged=none`. This issue
and its two evidence files are new. The drivers, comparison tests, guides
and iteration records were carried in and updated.
[#65 corrects the local lookup diagnostic and replays both failures.](65-clarify-deferred-api-lookup-diagnostic.md)

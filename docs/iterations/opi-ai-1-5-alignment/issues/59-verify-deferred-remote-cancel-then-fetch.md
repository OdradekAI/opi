# Verify explicit deferred cancellation and repair refetch failure identity

Status: identity repaired and paired replay complete. #60 aligns first-chunk telemetry. #93 assesses the result and diagnostic.
Date: 2026-11-12
Parent: [alignment specification](../spec.md)
Predecessor: [local cancellation before Done](58-observe-deferred-local-cancel-before-done.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #58 worktree inputs

## Observations

The Pi driver submits through public `Models` and the compiled Faux
provider. Rust seeds the equivalent handle in `ScriptedDeferredBackend`.
Both backends start with one pending check and `pollAfterMs: 25`.
The driver verifies equality with the original handle on each Pi fetch,
on the pending result and in the backend's cancellation log. It substitutes
the random Pi ID only in the trace, including the failure diagnostic.
[Pi driver](../tools/observe_upstream.mjs#L965) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2127)

| Operation | Observed on both sides |
| --- | --- |
| First `wait=0` fetch | One backend check returns `deferred` with the original handle. The poll settles with the first-chunk and request duration samples and no token or cost counters. |
| Explicit remote cancel | The call succeeds and the backend records one cancellation of the original handle. The telemetry carrier reaches the backend. The SDK creates zero spans and measurements for this call and dispatches no fetch. |
| Explicit refetch | The same handle produces one backend check and an `error` result with empty content. The settled poll carries `stop: error`, `provider_error`, the request duration and zero-valued token and cost samples. |

The pre-repair Rust message carried empty provider and model IDs even
though its span had the requested route. This is a payload mismatch.
The repaired message carries `lifecycle-differential` and `mock-model`.
With those fields restored, the two traces agree on the normalized
refetch payload, poll attributes, duration instrument and counters.

## Source and repair

At frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/models.ts:950–987` forwards the options and handle.
`cancelDeferred` directly awaits the backend. Faux marks the handle's
entry cancelled at `packages/ai/src/providers/faux.ts:657–666` and its
fetch rejects a marked entry at `faux.ts:591–654`. The error message
constructor at `faux.ts:327–339` carries the route identity.

Rust's `Models::stream_deferred` supplies the handle's requested route to
`ChatStream`. When the source fails without a partial, the collector uses
that route to synthesize the message, reusing the fallback construction
from ordinary chat setup. A partial supplied by the producer, then the
latest event-time partial, takes precedence over the fallback. This
also covers pre-dispatch failures such as expired handles.
[Deferred entry](../../../../crates/opi-ai/src/models.rs#L1037) ·
[Failure fallback](../../../../crates/opi-ai/src/stream.rs#L542) ·
[Ordinary chat identity repair](42-preserve-chat-setup-failure-identity.md#behavior)

## Remaining differences

Pi fulfills `fetchDeferred` with the error message. The Rust SDK returns
`Err(CallFailure)` with the cause supplied by its backend. The Rust
scripted backend uses `Config` and its own diagnostic for this failure.
The fixture-specific cause and diagnostic are recorded alongside the
public result channel. #93 verifies that both diagnostics settle the
Durable generation as `model_error`.
[#93 application outcome](93-assess-deferred-job-failure-classification.md)

Pi's observer records `pi.ai.first_chunk` and its duration on the first
forwarded event, which is the Faux `error` here
(`packages/ai/src/api/lazy.ts:37–53`, `packages/ai/src/telemetry.ts:155–161`).
In the #59 replay, Rust received a typed `Err` as the first source item and
settled the poll from its failure message. The observer recorded only `Ok`
stream items, so this failure had zero first-chunk samples. The #60 replay
compares a pre-dispatch auth failure and aligns first-chunk telemetry for
both cases while preserving the observed result channels and diagnostics.
[#60 first-chunk repair](60-align-first-chunk-on-typed-failure.md) ·
[Cancel telemetry policy](19-non-chat-ai-telemetry-boundary.md#decision)

This evidence covers the compiled Pi Faux and Rust scripted backends.
Positive waits, live provider protocol behavior and six-gate acceptance
remain open. The transport guides and Unreleased section record the
Rust failure identity contract.
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Verification

| Run | Material manifest and SHA-256 | Replay and SHA-256 |
| --- | --- | --- |
| Before repair | [Manifest](../evidence/phase5-deferred-remote-cancel-before-identity-repair-materials.json) `e10054037f0f35b27a7ae9d71377e13ee6a2b160bb7dc5b7463fa2884bcc0c22` | [Replay](../evidence/phase5-deferred-remote-cancel-before-identity-repair-differential.json) `1d7e77e886384b51153b053e8f31bfb029e1f9c96d6d5f7a38cfb32d8d9dd696` |
| After repair | [Manifest](../evidence/phase5-deferred-remote-cancel-then-fetch-materials.json) `d3801a53924d2252b68051646139544a43c3ba3a2c57ccaae4b2f6d4819d691e` | [Replay](../evidence/phase5-deferred-remote-cancel-then-fetch-differential.json) `765c4f2853b117e19a42364f985912dcdbfd4f84a1639875d694bdacbe34cd25` |

- The pre-repair replay returns `FAIL` / exit 1 on the missing IDs.
  The updated `explicit_cancel_is_recorded_and_best_effort` regression
  also fails before the repair on an empty provider ID.
- Each replay records 51 TypeScript invocations and 17 Rust invocations.
  All 136 receipts match their raw stdout and hashes. TypeScript triplets
  have identical stdout. The 16 prior cases match #58 in both runs.
  Only the two failure message IDs change in the repaired Rust case.
- Both material manifests bind the same original-lock Windows x64 Pi
  candidate. The initial manifest adds the new hashes for the drivers,
  comparator and runner. The repair changes only the Rust source tree
  binding. The #58 manifest and the pre-repair manifest each return
  `STALE_MATERIALS` / exit 2 against the later inputs before launching a
  build or either driver.
- All 17 shared comparisons pass after repair with zero material
  blockers. Two cases pass, ten have accepted differences and five
  deferred differences remain open. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The Python differential suite passed 59 tests. Cached Rust `--lib`
  tests filtered to `stream::` passed 23 tests. The cached `deferred_contract`,
  `ai_telemetry`, `custom_provider_registration`, `execution_lifecycle`
  and `lifecycle_differential` targets passed 105 tests. Clippy on the
  library and those targets with `-D warnings`, Cargo format,
  documentation and diff checks passed.

Test impact `add` for the paired case and comparator regressions.
`update` for the subprocess, cancelled refetch and expiry assertions.
`retain` for the 16 prior cases and Rust chat, deferred and telemetry
regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`.
The carried-in runtime, drivers, tests, changelog and iteration records
were updated. Source links affected by line shifts were refreshed.
This issue and four evidence files are new.

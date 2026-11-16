# #95 Replay deferred authentication failure

The 24-case original-lock record has four passes and 20 differences.
All 24 shared comparisons pass. Ten ordinary-call differences have
recorded decisions. The deferred cases include two route or expiry
decisions, two cases whose real-provider acceptance is blocked, and six
with an open observation. This slice replays #60 because its controlled
resolver fails before backend dispatch.
[Frozen record](../evidence/phase5-codex-websocket-idle-timeout-differential.json) ·
[#60 original repair](60-align-first-chunk-on-typed-failure.md)

## Disposition of the frozen cases

This table records the #95 audit. [#96](96-decide-deferred-wait0-poll-settlement.md)
records the subsequent decision on #55. [#100](100-decide-deferred-auth-setup-failure.md)
settles #60 using the same materials and the pinned Durable Harness.
[#101](101-decide-deferred-stream-result-and-drop.md) records the #67
result timing and direct collector decisions and the remaining poll gap.

The four passes are ordinary no-consumption and stop-consumption, deferred
positive wait, and the result-only deferred entry at a held EOF. The
ordinary decisions cover the two setup failures, the result and public
terminal timing cases, and the six telemetry fault cases. The setup
decision names the two ordinary `stream_simple` cases.
[Setup decisions](43-match-public-models-completion.md#accepted-decisions) ·
[Ordinary lifecycle decisions](../decisions/11-telemetry-lifecycle.md#普通-models-的-resolver-进入时点) ·
[Positive wait](61-verify-deferred-positive-wait.md) ·
[Result-only entry](76-return-deferred-outcome-before-source-release.md)

| Deferred difference | Recorded decision or remaining observation |
| --- | --- |
| [#55 wait0 pending then ready](55-observe-deferred-wait0-pending-then-ready.md) | Payloads and settled poll samples match. Poll settlement at the original `fetch_deferred` return remains open. |
| [#56 result with EOF held](56-observe-deferred-result-before-source-eof.md) | #76 proves early delivery through `fetch_deferred_ready`. The original completion-oriented fetch's result timing remains open. |
| [#57 local cancel after Done](57-observe-deferred-local-cancel-after-done.md) | Both keep the ready result. Releasing the held local tail and settling the poll on cancellation remain open for this comparison. |
| [#58 local cancel before Done](58-observe-deferred-local-cancel-before-done.md) | Both deliver an aborted outcome and keep the remote job usable. Result channel, diagnostic, source exit and poll timing remain open. The #78–81 application classification work covers different inputs. |
| [#59 remote cancel then refetch](59-verify-deferred-remote-cancel-then-fetch.md) | #93 classifies the fixture outcomes. The original result and diagnostic difference is retained. Real-provider response and settlement are blocked by #94. |
| [#60 auth setup failure](60-align-first-chunk-on-typed-failure.md) | First-chunk telemetry is aligned. Result channel and diagnostic remain open. Replayed below. |
| [#62 handle expiry](62-observe-deferred-handle-expiry.md) | #91 retains provider authority by default and the caller's explicit clock check. The original difference at equality is retained. |
| [#63 route mismatch](63-compare-deferred-handle-route-mismatch.md) | #92 retains Opi's pre-auth check of provider, model and API. The original rejection-stage difference is retained. |
| [#64 unknown job ID](64-compare-deferred-unknown-job-id.md) | #93 classifies the fixture outcomes. The original result and diagnostic difference is retained. Real-provider response and settlement are blocked by #94. |
| [#67 stream and source cleanup](67-compare-deferred-stream-source-cleanup.md) | #74 aligns started successful `Done`. #77 records the started public EOF timing difference. Started `result()` timing and direct collector ownership, including early Drop with an open poll, remain in the comparison. |

The disposition follows [#66's scoped recommendation](66-review-deferred-fetch-result-and-source-cleanup.md),
the [started EOF decision](../decisions/11-telemetry-lifecycle.md#启动-deferred-的公开-eof),
the [expiry decision](91-defer-expiry-to-provider-by-default.md),
the [route decision](92-validate-deferred-routes-before-dispatch.md),
and the [real-provider gap](94-record-real-deferred-integration-gap.md).

## Replay of #60

The same registered route and saved handle enter a zero-wait fetch. At
the gate both results are pending, each resolver has entered once, each
`pi.ai.poll` child span is open and there are zero measurements.
Releasing the resolver causes zero provider fetches and zero remote
cancels. Each side then has one settled poll with `stop: error`, one
first-chunk event and duration sample, one request-duration sample and
the same zero usage and cost counters. The result on each side retains
the requested provider and model, with empty content and no handle.

| Public result | Diagnostic |
| --- | --- |
| Pi fulfills with the failed assistant message | `API key auth failed for provider lifecycle-differential: controlled-auth-setup-failure` |
| Opi returns `Err(CallFailure)` with cause `AuthFailed` | `authentication failed: controlled-auth-setup-failure` |

The poll status includes the same diagnostic as its respective message.
The comparator checks final telemetry after the recorded checkpoints.
[Fresh paired receipts](../evidence/phase5-deferred-auth-setup-failure-replay.json) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L963)

At Pi `f9a26365568127461c47a07a37dcb42c140a3ac5`, the API-key resolver
wraps the thrown failure as a `ModelsError("auth", ...)` including the
provider ID and cause text. `lazyStream` turns that rejection into the
assistant message. See `packages/ai/src/auth/resolve.ts:193–204`,
`packages/ai/src/utils/models-error.ts:5–20`,
`packages/ai/src/models.ts:950–975` and
`packages/ai/src/api/lazy.ts:4–22,74–83`. Opi forwards the resolver's typed
failure through the shared collector and gives the canonical message
the requested route.
[Opi dispatch](../../../../crates/opi-ai/src/models.rs#L1088) ·
[Opi failure collection](../../../../crates/opi-ai/src/stream.rs#L542)

The recommendation is to retain `AuthFailed` in `CallFailure` and
keep the resolver diagnostic. The caller can distinguish authentication
from local configuration and read the provider and model from the
message. Durable-style code that records a failed assistant message
can consume `CallFailure::message()`. This case verifies one resolver
attempt before dispatch and settles the SDK poll. Application retry and
settlement were outside this replay. The #43 ordinary setup decision
supports the recommendation within its own scope. #60's deferred
channel and diagnostic are pending a decision.
[Public contract](../decisions/04-public-contract.md#公开-models-的准备失败错误通道) ·
[Consumer contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Verification

- The [#89 material manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and its bound Rust executable verified before and after the replay.
  Three fresh Pi invocations are identical. The full case, including
  all four receipts and comparator output, equals the frozen case.
  Each receipt validates against stdout and SHA-256.
- The targeted evidence SHA-256 is
  `4dc285a4cbd9d0b424ba08e4e2b81cbdcc2e578e53dd863f6a54afa7f4d5b9ea`.
  Material drift and comparator mismatches are empty.
  The case remains `difference` with `strict_alignment: false`.
  The original #64 and #59 records and #93 receipts revalidate by hash.
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
  Test impact `retain` for all bound cases and focused controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
Unstaged updates touch the carried-in spec, contract index and
differential guide. The carried-in #60 issue gains a link. This issue and
the targeted replay are new untracked files.

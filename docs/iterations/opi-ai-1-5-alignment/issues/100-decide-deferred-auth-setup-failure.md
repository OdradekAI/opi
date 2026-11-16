# #100 Decide deferred authentication failure delivery

Retain Opi's `Err(CallFailure)` with `AuthFailed` and its existing
authentication diagnostic for #60. Both SDKs fail before the fetch
reaches the provider. The two diagnostics take the same retry and task
settlement path in Durable and remain distinct in persisted records.
[#60 original case](60-align-first-chunk-on-typed-failure.md) ·
[Fresh SDK and Durable receipts](../evidence/phase5-deferred-auth-setup-consumer-and-replay.json)

## Frozen SDK replay

The four new receipts match the complete #89 and #95 case. At the held
resolver each fetch is pending, with one open `pi.ai.poll`, one resolver
invocation and zero measurements. After release each side has zero
provider fetches and zero remote cancels, a settled poll with one
first-chunk event and duration sample, and the same zero token and cost
counters. The messages agree on `error`, requested provider and model,
empty content and absent handle. Each poll records its own error text.

| SDK | Public result | Diagnostic |
| --- | --- | --- |
| Pi `fetchDeferred` | Fulfilled assistant error message | `API key auth failed for provider lifecycle-differential: controlled-auth-setup-failure` |
| Opi `fetch_deferred` | `Err(CallFailure)` with cause `AuthFailed` | `authentication failed: controlled-auth-setup-failure` |

At Pi `v1.5.0`, revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
the API-key resolver wraps the thrown cause in `ModelsError("auth", …)`.
`lazyStream` converts it to an assistant error message. `fetchDeferred`
fulfills with that message. See `packages/ai/src/auth/resolve.ts:193–204`,
`packages/ai/src/utils/models-error.ts:5–20`, `packages/ai/src/api/lazy.ts:4–22,74–83`
and `packages/ai/src/models.ts:950–975`.
Opi's deferred path has propagated the resolver's typed error since
`4b3600fcdeb3ade41bf4f66249337c382630af23`. Its current collector builds
an `Error` message using the requested provider and model and the
credential-sanitized summary from the cause in this fixture.
[Resolver dispatch](../../../../crates/opi-ai/src/models.rs#L1088) ·
[Failure message](../../../../crates/opi-ai/src/stream.rs#L191) ·
[Diagnostic summary](../../../../crates/opi-ai/src/errors.rs#L95)

## Durable recording and settlement

The probe runs the pinned Durable Harness over `MemoryStorage`, with the
compiled Pi AI package and Faux. Durable submits a job, saves its handle,
and calls the real `Models.fetchDeferred` from the poll checkpoint with a
live task signal. The controlled API-key resolver fails that first poll.
One run passes Pi's message through unchanged. The other substitutes the
exact Opi diagnostic at the fetch return boundary to model the Rust
caller's use of `CallFailure::message()`. The separate Rust replay verifies
that message and its typed cause. The Durable projection retains Pi's
message shape, including its API string. Opi's synthesized message uses
an `ApiKind` fallback and leaves `wire` absent. The exact API ID remains
available on the handle. This decision covers the result channel and
diagnostic.
[Harness probe](../tools/observe_deferred_auth_failure_consumer.mjs) ·
[Opi synthesized message](../../../../crates/opi-ai/src/stream.rs#L264)

Durable runs `afterResponse`, classifies the diagnostic and appends the
assistant message to `pi.assistant`. Both original diagnostics result in
one submission, one application fetch, zero provider fetches, zero
retries and zero remote cancels. Durable records the exact respective
diagnostic in the hook, the assistant entry's `errorMessage`, the input's
`unanswered/model_error` `detail` and the failed task's `error.message`.
The Pi text names the provider and API-key method in those fields. Opi's
text names authentication and retains the resolver detail. Its canonical
message carries provider and model as fields.

The retry budget is enabled for one retry. With `503 Service Unavailable`
as the controlled resolver detail, both forms record the first failure,
retry the generation with a new job, fetch it successfully and settle
the input as `done` and the task as `completed`. The first fetch still
stops before provider dispatch. See `packages/durable/src/harness/generation.ts:323–338,758–851,1148–1154`
and `packages/ai/src/utils/retry.ts:270–275` at the pinned revision.
[#60 and 503 receipts](../evidence/phase5-deferred-auth-setup-consumer-and-replay.json)

## Decision

Keep the typed cause so Rust callers can identify the authentication
stage and distinguish it from route configuration errors. Keep the
current diagnostic because it preserves the safe resolver detail that
Durable needs to record and classify. Applications that use Durable's
policy classify the canonical message. The focused Rust test observes
`Terminal` for #60 and `Transient` for the 503 diagnostic while
`ProviderError::is_retryable()` is false on both `AuthFailed` causes.
An authentication cause alone therefore cannot substitute for the
message-based retry decision in such a caller.
[Rust contract test](../../../../crates/opi-ai/tests/deferred_contract.rs#L547) ·
[Whole-call classifier](../../../../crates/opi-ai/src/retry.rs#L1032)

Record the channel and text as intentional differences for #60. The
decision follows the current SDK and caller behavior and the value of
typed cause plus diagnostic. The 24-case raw verdict stays
`DIFFERENCES_REQUIRE_DECISION` with `strict_alignment: false`.
[#94](94-record-real-deferred-integration-gap.md) still blocks real
deferred acceptance. The original #59 and #64 differences are retained.
Next, review the remaining [#67](67-compare-deferred-stream-source-cleanup.md)
observations for started `result()` and direct collector ownership.
[Public contract](../decisions/04-public-contract.md#公开-deferred-fetch-的鉴权失败错误通道)

## Verification

- The [#89 manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and bound Rust binary verified before and after execution. Three Pi
  and one Opi receipt equal the #89 case, including the comparator
  output. Their traces and comparison also equal the original #60 case.
  Shared observations pass with zero mismatches and material drift.
- The probe runs on the same snapshot and installed lock. Twelve
  relevant Pi source and fixture files match the pinned Git revision.
  Three Durable probe invocations produce identical raw receipts. Node
  syntax passes. Missing `--snapshot` exits 2. The new evidence SHA-256
  is `887be13fc1cd00a57ffb2854a9a289a094c0f8e30303e783804512cbb21721b5`.
- Cached `test -p opi-ai --test deferred_contract --locked --offline -j4`
  passes 19 tests. Clippy on that target with `-D warnings`, rustfmt on
  the changed test, `python -B scripts/opi-doc-check.py` and
  `git diff --check` pass.

Test impact `add` for the Durable probe and the Rust auth diagnostic and
retry regression, `retain` for the four frozen SDK receipts and other
deferred regressions. Worktree handoff `committed=none-for-this-task`
and `staged=none`. The carried-in iteration, public contract, transport
docs, differential guide and Rust test are touched. This issue, consumer
probe and replay evidence are new.

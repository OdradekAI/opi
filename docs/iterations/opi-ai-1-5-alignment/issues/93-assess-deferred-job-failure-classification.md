# #93 Assess deferred job failures after dispatch

Both recorded failures end the current generation without a retry.
Keep Opi's `Err(CallFailure)` with the canonical failed assistant message
and the backend's typed cause. The scripted backend's `Config` has the
right retryability flag for these cases. Its configuration diagnostic
misidentifies a backend job state. Real adapters own their mapping from
provider responses to typed causes and diagnostics.
[#64 unknown job](64-compare-deferred-unknown-job-id.md) ·
[#59 cancelled job](59-verify-deferred-remote-cancel-then-fetch.md) ·
[Replay and consumer receipts](../evidence/phase5-deferred-job-failure-consumer-and-replay.json)

## Durable task settlement

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` resolves the
checkpoint model and awaits `fetchDeferred`. With a live task signal,
`classify` checks `stopReason`, overflow and the retry predicate on the
assistant message. A matching transient error within the retry budget
sleeps and restarts from `prepare`, which submits a new job. For a
terminal error Durable stores the assistant entry, settles the input as
`unanswered/model_error` with the diagnostic, and records the generation
task's `failed/model_error` outcome. A task abort with a retained handle
separately issues best-effort remote cancel.

The paths at the 1.5.0 pin are
`packages/durable/src/harness/generation.ts:302–337,390–415,758–849`
and `packages/ai/src/utils/retry.ts:7–104,270–275`. Pi 1.3.0 at
`71023aec1546d6d677a65669c5665c0927ce0dd0` also awaits the message and
uses the retry predicate before settling `model_error` at
`packages/durable/src/harness/generation.ts:236–244,506–533`.
[#78 transient and abort controls](78-review-deferred-failure-classification.md)

The probe runs the pinned Harness, MemoryStorage and Faux with one
retry available. It supplies the four exact normalized diagnostics from
the SDK cases as error messages on a deferred fetch. The SDK replays
below exercise the actual job maps.
[Durable probe](../tools/observe_deferred_job_failure_consumer.mjs)

| Fetch message | Submissions / fetches / remote cancels | Input | Generation task |
| --- | --- | --- | --- |
| Unknown job, either diagnostic | 1 / 1 / 0 | `unanswered/model_error` | `failed/model_error` |
| Cancelled job, either diagnostic | 1 / 1 / 0 | `unanswered/model_error` | `failed/model_error` |
| `503 Service Unavailable` control | 2 / 2 / 0 | `done` | `completed` |

For each failure, the signal remains live, both predicates are false
and the assistant history and task outcome retain the supplied diagnostic.

## Paired SDK replay

#64 changes only the job ID. Both SDKs authenticate and dispatch one
zero-wait fetch with the unknown ID. The original handle then produces
the same ready result on another explicit fetch, with one auth resolution
and one dispatch per side. Remote cancel count stays zero.

#59 sends one zero-wait fetch which returns pending, one successful
explicit cancel and one zero-wait refetch of the original handle. Cancel
forwards caller telemetry, dispatches zero fetches and creates zero SDK
spans or measurements. Both SDKs resolve auth before dispatch on the
matching route. The bound #59 driver counts dispatches. The matching
handle control from #92 separately counts one auth resolution and one
dispatch per side for cancel and refetch, with the same failed refetch
diagnostics and no preceding pending check.
[#92 auth counts](../evidence/phase5-deferred-route-authority-observations.json)

| Failure | Pi fulfilled assistant message | Opi `Err(CallFailure)` with `Config` |
| --- | --- | --- |
| #64 missing job | `Unknown faux deferred response: deferred-missing` | `invalid provider configuration: Unknown scripted deferred response: deferred-missing` |
| #59 cancelled job | `Faux deferred response was cancelled: deferred-1` | `invalid provider configuration: Scripted deferred response was cancelled: deferred-1` |

Both messages have the requested provider and model, `stop: error`, empty
content, zero usage and no returned handle. Both polls settle with
`provider_error`, their diagnostic, one first-chunk event and matching
duration and zero-valued usage samples. Faux emits an in-band error and
ends the stream with that message at the 1.5.0 pin
`packages/ai/src/providers/faux.ts:591–665`. The public fetch and cancel
entries are at `packages/ai/src/models.ts:950–987`.

## Opi result and retry

The SDK validates the local route, resolves auth once and dispatches one
fetch. A valid route's fetch result preserves the typed error from
`DeferredApi::fetch_prepared`. The early SDK `Config` for route or
expiry validation originates in `DeferredError::into_provider_error`.
The two post-dispatch causes originate in
`ScriptedDeferredBackend::scripted_error`, introduced by Opi
`4b3600fcdeb3ade41bf4f66249337c382630af23`. The generic agent diagnostic
for that cause is `provider_config_error`, so an application that displays
it as a configuration problem would mislead the user in these cases.
[Fetch dispatch](../../../../crates/opi-ai/src/models.rs#L1058) ·
[Local validation error](../../../../crates/opi-ai/src/api/contract.rs) ·
[Scripted backend](../../../../crates/opi-ai/src/test_support.rs#L855) ·
[Diagnostic mapping](../../../../crates/opi-agent/src/diagnostic.rs#L761)

The two public Rust results have `is_retryable() == false` and a canonical
message classified `Terminal` by `classify_whole_call_retry`. The SDK
leaves retry, task settlement and remote cancel to the application. A
Durable-style consumer should pass `CallFailure::message()` to the same
policy that records failed messages. Propagating `Err` past that policy
would bypass its history and settlement writes. Mapping a backend's
cancelled-job error to `ProviderError::Cancelled` would synthesize an
`Aborted` fetch message and change the stop reason. The current Opi
reopening consumer is a test fixture and settles any `Err` by recording
the error and stopping its scheduler.
[Failure message and cause](../../../../crates/opi-ai/src/stream.rs#L159) ·
[Whole-call classifier](../../../../crates/opi-ai/src/retry.rs#L1032) ·
[Reopening fixture](../../../../crates/opi-ai/tests/deferred_contract.rs#L855)

The result channel and diagnostics remain observable differences in the
raw record. The application decision agrees for the four exact messages.
The fixture's error type is retained as replay evidence. Real-provider
job responses and production application settlement remain to be
verified for six-gate acceptance.
[#94 production integration gap](94-record-real-deferred-integration-gap.md)

## Verification

- The [original-lock manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and the [24-case record](../evidence/phase5-codex-websocket-idle-timeout-differential.json)
  have zero material drift. #64 and #59 match their full bound case
  records. Each has three identical Pi receipts and one Rust receipt.
  Shared comparisons pass with `strict_alignment: false`.
- Three Durable probe receipts match. All eleven receipts validate
  against their stdout and SHA-256. The new
  [evidence](../evidence/phase5-deferred-job-failure-consumer-and-replay.json)
  has SHA-256
  `11f4cc13b3214b2ea4b17bb83cf4d1c25041db73e7838acc394d4d6126f18315`.
  Node syntax passes and a missing snapshot argument exits 2.
- `cargo test -p opi-ai --test deferred_contract --locked --offline`
  passes 18 tests in each of the default and no-default-feature runs.
  Affected-target Clippy with warnings denied, formatting, documentation
  and diff checks pass.

Test impact `add` for the five-row Durable probe. Test impact `update`
for the exact Rust diagnoses and terminal classification assertions in
the two existing regressions. Test impact `retain` for both bound drivers
and the existing local abort and reopen controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
Unstaged updates touch the carried-in Rust test target, iteration brief,
contract index, differential guide and bilingual transport guides.
Untracked updates touch the carried-in #59 and #64 issues and add the
probe, evidence and this issue.

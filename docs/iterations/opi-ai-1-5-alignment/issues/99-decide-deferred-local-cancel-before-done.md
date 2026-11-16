# #99 Decide local deferred cancellation before Done

The four fresh #58 receipts match the original-lock #89 case.
Retain Opi's typed local cancellation, diagnostic and cleanup
boundary when the caller cancels a dispatched fetch before a terminal
is cached. The paired replay also preserves the explicit refetch of
the original handle.
[#58 original case](58-observe-deferred-local-cancel-before-done.md) ·
[Fresh paired and consumer receipts](../evidence/phase5-deferred-local-cancel-before-done-replay.json)

## Result, source and poll

Each side dispatches one zero-wait fetch with the same handle. The
source has emitted `Start` and the success gate stays closed through
cancellation and refetch. Each backend sees the caller cancellation
state and emits zero successful `Done` events.

| Observation | Pi `fetchDeferred` | Opi `fetch_deferred` |
| --- | --- | --- |
| Public result | Fulfilled assistant message with `aborted` and `Request was aborted` | `Err(CallFailure)` with cause `Cancelled` and an `Aborted` message with `cancelled` |
| Source at result delivery | The cooperative source emits one abort terminal, reaches EOF and runs `finally`. Its result has yet to be read. | The SDK releases the pending source before natural EOF. |
| Poll at result delivery | Open with the first-chunk duration sample. It settles after the source result read. | Already settled with the cancellation snapshot. |

The projected cancellation messages agree on route, content, handle,
stop and usage. The final polls both have `stop: aborted` and status
`ok`, with the same parent relationship, two duration samples, four
token samples and one cost sample. The comparison excludes elapsed
duration values. Each remote cancel entry sees zero calls. An explicit
fetch with the original handle makes one more dispatch and returns
the same ready result on each side.
[Pi driver](../tools/observe_upstream.mjs#L840) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1582) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L856)

## Caller and decision

At Pi `f9a26365568127461c47a07a37dcb42c140a3ac5`, Models passes the
caller signal to the adapter. Its stream fulfills the outer result
at the first terminal. The forwarder reads the source result before
settling the poll. The scripted cancellation terminal follows Faux's
shape. See `packages/ai/src/models.ts:856–898,950–975`,
`packages/ai/src/api/lazy.ts:37–53`,
`packages/ai/src/utils/event-stream.ts:43–48,97–110` and
`packages/ai/src/providers/faux.ts:341–345,358–396,591–644`.

Opi's local token wrapper dates to AIR-043 at
`4b3600fcdeb3ade41bf4f66249337c382630af23`. It races the pending read
against the token. The shared collector drops the pending source
and builds a canonical failure from the latest partial and
`ProviderError::Cancelled`. The typed cause maps to `Aborted`.
`cancelled` is the cause's fallback diagnostic when the partial has
no diagnostic. Direct `fetch_deferred` settles an enabled poll observer
before returning when recording succeeds. This gives the caller
local source release even when the pending source cannot respond
to the token on its own, plus a cause it can match programmatically.
[Token wrapper](../../../../crates/opi-ai/src/models.rs#L1101) ·
[Failure and diagnostic](../../../../crates/opi-ai/src/stream.rs#L203) ·
[Source release and result](../../../../crates/opi-ai/src/stream.rs#L560)

Durable passes its task signal into fetch and checks it before it
classifies the returned message. The replay of the #78 Harness probe
with MemoryStorage and Faux confirms the two application outcomes.

| Durable input | Observed continuation |
| --- | --- |
| Application abort while the fetch is held | One submission and fetch, zero retries, no assistant failure entry. The input settles as `unanswered/aborted`. The task abort handler explicitly cancels the saved handle once. |
| Provider returns `aborted` with a live task signal | One submission and fetch, zero retries and zero remote cancels. Durable records the assistant entry and settles the input as `unanswered/model_error`. |

The pinned generation code sets the task outcome to `aborted` in the
abort handler and to `failed` on the terminal model error path.
See `packages/durable/src/harness/generation.ts:323–338,390–418,582–596,758–851`.
A Rust port checks the task's abort state before classifying either
fetch outcome. With a live task, it can read the canonical message
from `CallFailure::message()`. `classify_whole_call_retry` returns
`Aborted` from the stop reason before considering diagnostic text.
The task abort handler owns explicit remote cancellation. The probe
exercises the Pi application path with a controlled adapter. The
Rust tests exercise the SDK and the retry classification.
[#78 caller evidence](78-review-deferred-failure-classification.md#application-behavior) ·
[Rust retry classification](../../../../crates/opi-ai/src/retry.rs#L1032)

Record the result channel, diagnostic, source exit and poll timing
as intentional differences for #58. For this caller-triggered abort,
changing the diagnostic would leave the application's retry and
settlement decision unchanged. The decision retains the original
comparison and covers the dispatched, preterminal local cancellation
case. The #94 real integration blocker and the original #59 and #64
differences still apply.
[Public contract](../decisions/04-public-contract.md#公开-deferred-fetch-的本地取消错误通道) ·
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#deferred-结果式就绪与-poll-结算) ·
[#94 integration gap](94-record-real-deferred-integration-gap.md)

## Verification

- The [#89 manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and bound executable verified before and after replay. Three Pi
  runs and one Rust run match the complete #89 case. Their traces
  and comparison also match the original #58 record.
- The #78 Durable probe reproduces its original receipt on the same
  snapshot. The #58 raw status remains `difference` with zero
  comparator mismatches and `strict_alignment: false`. The new
  evidence SHA-256 is
  `a85ae9bc638ee345ad8f93cb2a9761413cab68fc2cd4a4928f86efa16b6eb8b7`.
- The cached `test -p opi-ai --test execution_lifecycle --test whole_call_retry --locked --offline -j4`
  passes 40 and 23 tests, respectively.
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
  Test impact `retain` for existing lifecycle and caller controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The spec, index, public contract, lifecycle decision, bilingual transport
docs and differential README are carried in and touched. The carried-in
#58 and #98 issues link the decision. This issue and replay receipt are new.

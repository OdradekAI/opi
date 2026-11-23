# Align whole-call retry classification with Durable

`retry::classify_whole_call_retry` now covers the Pi 1.5 quota and transient
rules on the canonical message from `CallFailure`. Durable's generation path
checks overflow before retrying, and the Rust classifier applies the same
veto before deciding whether a failure is transient.
[Classifier](../../../../crates/opi-ai/src/retry.rs#L1032)

## Source changes

The Rust table introduced by `3104da4c2e4ddb97ffdd809c2973b7c3cb1db1ea`
matches the pattern families from Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0`. The following changes
are present in `packages/ai/src/utils/retry.ts:7–126` at the frozen `v1.5.0`
revision `f9a26365568127461c47a07a37dcb42c140a3ac5`.

| Upstream commit | Classification change |
| --- | --- |
| `cedefc53264f521f5a55be61b746a39c6ada8d4e` | ChatGPT Business workspace usage limit vetoes retry. |
| `c719d5b08e5d9fb9bbcb1f1792281eea8b442812` | Z.ai JSON code `1308` vetoes retry. |
| `6e491cfa95b53e656e31b72f91a0839046a8da1b` | Retryable status codes require ASCII alphanumeric boundaries. |
| `698a2d23b86a5045dd54554354a5e0cb7e466795` | Upstream request failure, upstream read error, interrupted stream and HTTP/2 stream failure are transient. |
| `0fbdbd3f8ab1876ac260c0f08226a400759323b8` | Node-fetch request failure text is transient. |
| `29057ece3ef9b649b233dfc1fbcece94712f17df` | Relayed Codex CLI stream disconnect text is transient. |

The comparison uses `isRetryableAssistantError` from `retry.ts:270–275`
with the overflow gate in `packages/durable/src/harness/generation.ts:794–825`.
For example, `503` with a LiteLLM overflow diagnostic matches the standalone
Pi transient predicate and takes Durable's overflow branch. Rust reports
`Terminal` for whole-call retry and exposes the overflow predicate for
compaction. An embedded `503` in a request ID and `524` in a byte count
both take the terminal path in the new status controls.
[Cross-runtime matrix](../../../../crates/opi-ai/tests/whole_call_retry/classification.rs#L171)

## Result and HTTP consumers

The 12 public deferred observations include these decisions.

| Canonical diagnostic | Whole-call classification | Typed cause retryable |
| --- | --- | --- |
| Legacy 503 service unavailable | `Transient` | No |
| 429 quota with a typed rate-limit cause | `Terminal` | Yes |
| LiteLLM context-window or HTTP 413 overflow | `Terminal` | No |
| Provider abort with 503 text | `Aborted` | No |
| Upstream stream read failure | `Transient` | No |

The five matching #78 provider fixtures agree with the frozen Durable
generation decisions. Its returned 503 schedules a new job. Quota and
provider abort end the attempt, and eligible overflow compacts before the
next submit. The caller owns its cancellation state and the attempt budget.
[Deferred failure inputs](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1384) ·
[Frozen Durable outcomes](../evidence/phase5-deferred-failure-consumer.json)

The shared OpenAI-family 429 path reads the designated fields from flat
and nested error bodies. It checks a structured Z.ai code as the exact
string or integer `1308` and turns recognized limits into the existing
`rate limited: quota exceeded` diagnostic. The local Chat Completions and
Responses fixtures check one send for quota and two for transient 429
under the one-retry whole-call policy, both directly and through `Models`.
They check redaction of the response text, preservation of `Retry-After`,
and a separate acquisition control with zero or one admitted retry and
byte-identical payloads. Metadata and unrelated code values have negative
controls.
[HTTP mapping](../../../../crates/opi-ai/src/http.rs#L88) ·
[Loopback checks](../../../../crates/opi-ai/tests/whole_call_retry/call_and_callbacks/quota_transport.rs#L129)

## Verification

- The pinned Pi retry test target passes all 38 tests. Five frozen files
  including `package-lock.json` match Git bytes. The same 23 diagnostic
  inputs yield identical Pi decisions in three runs. Rust differs on
  14 rows before the repair and matches all 23 afterward.
- `execution_lifecycle` (40), `openai_chat_fixtures` (114),
  `openai_responses_fixtures` (83) and `whole_call_retry` (22) pass.
  The nine #80 deferred rows retain their recorded inputs and typed causes
  and add the message classification. Three new rows exercise the two
  quota forms and a gateway transient.
- Both modified targets pass all 62 tests with `--no-default-features`.
  Affected-target Clippy and rustdoc pass with warnings denied. Cargo
  format, `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
- The #80 manifest rejects the changed Rust source tree with exit 2.
  The new freeze uses the original lock with zero material blockers.
  The 24 paired lifecycle records match #80, with all shared comparisons
  and 96 trace receipts validated. The classification receipt binds the
  diagnostic matrix and public deferred observations to that snapshot
  and the executed Rust test targets.
  [Classification receipt](../evidence/phase5-deferred-retry-classification.json) ·
  [Materials](../evidence/phase5-deferred-retry-materials.json) ·
  [Replay](../evidence/phase5-deferred-retry-differential.json)
- The #80 driver copy is at
  `C:/Temp/opi-ai-lifecycle-executables/0389725b11b524fd53ea25b001a1b49f9cf4c4c94a3e72b3c63ac14add893bcf/lifecycle_differential.exe`.

At this checkpoint the shared Responses mapper reduced `response.failed`
to a generic stream failure. The follow-up traces gateway diagnostics
through local SSE fixtures and verifies which facts reach `CallFailure`
after redaction. The replay records ten deferred difference cases.
Real-provider wire behavior and six-gate acceptance remain open.
[Responses failure follow-up](82-preserve-responses-stream-failure-classification.md)

Test impact `add` for the 23-row retry matrix, `update` for the public
deferred observations and 429 wire controls, and `retain` for existing
retry policies and the paired lifecycle cases. Worktree handoff
`committed=none-for-this-task` and `staged=none`.
The retry and HTTP modules are task-owned unstaged edits. The affected
tests, bilingual guides, changelog, iteration brief and index, and replay
guide are carried-in and touched. The #81 issue and three receipts are
new untracked files. The #79 test link follows its shifted line.

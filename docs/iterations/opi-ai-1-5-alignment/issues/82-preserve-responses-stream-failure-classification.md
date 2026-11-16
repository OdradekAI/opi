# Preserve Responses stream failure classification

Completed 2026-11-12 against Pi 1.5.0
`f9a26365568127461c47a07a37dcb42c140a3ac5`. The scope is the recovery
information available from `response.failed` after a successful Responses
handoff. The fixture exercises the public Rust result and retry policy.
[Fixture](../../../../crates/opi-ai/tests/data/responses-failed.json)

## Where the decision was lost

Pi 1.3.0 `71023aec1546d6d677a65669c5665c0927ce0dd0` already formats
`response.error.code` and `response.error.message` in
`packages/ai/src/api/openai-responses-shared.ts` at line 772. Pi 1.5.0
still does so at line 802, and the gateway rules added in
`698a2d23b86a5045dd54554354a5e0cb7e466795` classify more of those
diagnostics as transient. The OpenAI and Azure adapters acquire the
response before consuming the body. The pinned Durable generation gate
checks context overflow before its retry predicate.

The Rust audit fixes `b27905a27fa4fb2329ef8851b4e8fcb2a5c2b7e2`
and `47400ee3edde41ff06ed69924139de938b949eb9` prevent a provider from
echoing credentials or request fragments through the stream error. Their
fixed summary also erased the inputs needed for recovery. The loopback
reproduction shows five missed retries, an overflow that reaches the
caller as a generic error, and a quota failure with its reason erased.

## Result contract

OpenAI Responses and Azure evaluate the nested string code and message
using the existing whole-call and overflow rules. Codex evaluates the
message, as observed through its own `CodexApiError` and public result in
the frozen adapter. A code-only gateway failure is transient in Azure
and terminal in Codex. The Codex SSE and WebSocket paths reuse the
shared mapper. Its WebSocket control verifies zero fallback sends for a
transient provider terminal.
[Mapper](../../../../crates/opi-ai/src/openai_responses_shared.rs#L723) ·
[Classifier](../../../../crates/opi-ai/src/retry.rs#L1063)

The canonical `CallFailure` carries a typed `StreamError` with fixed
text for a recognized transient, quota or overflow. A frame without
recognized diagnostic information carries the generic stream error.
Classification reads the specified error fields, with controls for
unrelated metadata, incidental status digits and an ambiguous code.
The fixed diagnostic reaches the cause, serialized assistant message
and telemetry. Pi's provider text and workspace-specific usage hint
remain differences in diagnostic presentation. Rust also retains the
provider's raw terminal status in Codex where the frozen result omits it.

The Rust test drives one prepared attempt per logical call with an
acquisition retry budget of one. Each HTTP 200 response seals the call
as `PostHandoff`. With one explicitly enabled whole-call retry, a
transient fixture sends two requests from two fresh logical calls.
Quota, overflow and ordinary terminal controls each send once.
The terminal preserves streamed text, route, projection and the
created response identity. Usage remains unknown on this failure path,
including when the failed frame or a later ignored completion carries
usage. Each attempt emits one failure and settles one request span.
[Loopback test](../../../../crates/opi-ai/tests/whole_call_retry.rs#L1211)

## Verification

- Three frozen probe runs yield the same 14 observations across
  Responses, Azure and Codex SSE. Eleven source/test/lock files match
  the Git snapshot. Pi's retry, terminal-event and usage-limit targets
  pass 54 tests. The shared Responses fixture closes the five
  retry-decision differences and preserves the quota and overflow
  diagnostics in Rust.
  [Probe](../tools/observe_responses_failed.mjs) ·
  [Classification receipt](../evidence/phase5-responses-failed-classification.json)
- `azure_openai_fixtures` (44), `execution_lifecycle` (40),
  `openai_codex_responses` (76), `openai_responses_fixtures` (83),
  `public_utilities` (53) and `whole_call_retry` (23) pass on Windows.
- The Azure, Codex, utility and whole-call targets pass 196 tests
  with `--no-default-features`. Affected-target Clippy and rustdoc pass
  with warnings denied. Cargo format, `python -B scripts/opi-doc-check.py`
  and `git diff --check` pass. The probe exits 2 with usage text when
  the snapshot argument is absent.
- The #81 material manifest rejects the changed Rust source tree
  before replay with exit 2. The original-lock freeze binds the same
  Pi snapshot to the updated Rust code with zero material blockers.
  All 24 lifecycle cases match #81, including their shared assertions.
  The audit verifies 96 trace receipts. The #81 driver is preserved
  under `C:/Temp/opi-ai-lifecycle-executables/` by its SHA-256
  `888128b5b68d4a9e84036abea238bb5db787974c3fd43e692a6acd8883efe5f3`.
  [Materials](../evidence/phase5-responses-failed-materials.json) ·
  [Lifecycle replay](../evidence/phase5-responses-failed-differential.json)

The follow-up traces the separate top-level SSE `error` through the pinned
SDK and Codex recovery path. The lifecycle suite still records ten deferred
difference cases. Live-provider wire behavior and the full acceptance gates
remain open.
[Top-level error follow-up](83-preserve-responses-sse-error-classification.md)

Test impact `add` for the shared Responses, Azure SSE and Codex
WebSocket controls, `update` for the Codex SSE redaction test, and
`retain` for existing retry, overflow, cancellation and lifecycle
controls. Worktree handoff `committed=none-for-this-task` and
`staged=none`. The shared Responses and Codex source files plus the
Azure and Codex tests are task-owned unstaged edits. The retry and
message modules, whole-call test, bilingual utility guides, changelog,
iteration brief, index, #81 issue and replay guide are carried-in and
touched. The #82 issue, shared fixture, probe and three receipts are
new untracked files.

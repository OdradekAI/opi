# Preserve top-level Responses SSE error classification

Completed 2026-11-12 against Pi 1.5.0
`f9a26365568127461c47a07a37dcb42c140a3ac5` and its original npm lock.
The fixtures exercise the independent `error` terminal through the
OpenAI, Azure and Codex Responses entries.
[Shared error frames](../../../../crates/opi-ai/tests/data/responses-error.json)

## Error selection on the wire

The locked OpenAI SDK 7.19.0 selects `data.error ?? data` for a named SSE
`error` and intercepts a top-level error envelope in a data-only frame.
It emits the selected error's message when present. Its fallback serializes
the selected error. Pi's error formatter uses that diagnostic because the
SSE error has no HTTP status. A flat data-only `type: "error"` reaches
Pi's shared mapper and includes both code and message. These paths explain
why identical JSON with a transient code and a nonempty terminal message
leads to different retry decisions depending on the SSE name.

- Locked SDK `packages/ai/node_modules/openai/src/core/streaming.ts`
  lines 138–157 and `src/core/error.ts` lines 24–57.
- Pi `packages/ai/src/api/openai-responses-shared.ts` lines 800–801,
  `packages/ai/src/utils/error-body.ts` lines 40–56 and 130–137.
- Pi Codex `packages/ai/src/api/openai-codex-responses.ts` lines
  747–781. Each field prefers a top-level string, including an empty
  string, then the nested string. The result uses the nonempty message
  or code with the Codex prefix.

The flat-error and Codex field selection already exist in Pi 1.3.0
`71023aec1546d6d677a65669c5665c0927ce0dd0` in
`packages/ai/src/api/openai-responses-shared.ts` at line 771 and
`packages/ai/src/api/openai-codex-responses.ts` at line 731.
Opi's generic diagnostic
comes from the Responses redaction boundary described in #82. The red
run shows three frames reaching a later successful completion after
the error was ignored. Other valid frames reached the caller as a
generic failure that erased the recovery inputs.

## Redacted public result

The decoder recognizes SDK error envelopes and named errors ahead of
regular OpenAI and Azure event dispatch. Data-only flat errors and Codex
use their own diagnostic shapes. The prefix is part of the classification
input. For example, an anchored 413 can match the named SDK error and
the same 413 after the flat or Codex prefix is terminal. The existing
whole-call and overflow classifiers run before the decoder emits a fixed
summary. The typed stream cause, public message and telemetry receive
only that summary.
[SDK selection](../../../../crates/opi-ai/src/api/shared/openai_responses.rs#L680) ·
[Codex selection](../../../../crates/opi-ai/src/api/shared/openai_responses.rs#L708)

Two fixture decisions differ from Pi's whole-payload JSON fallback.
A nested SDK metadata field and a Codex nested code shadowed by an
explicit empty top-level code each cause a retry in Pi. Opi terminates
both calls. These fields provide insufficient evidence to send a new
generation request. The code/type fallback remains available when it
is part of the selected SDK error. The fixed text also preserves the
quota veto and the overflow signal that an application can use for
compaction.

The loopback test uses the public Models completion and a one-retry
whole-call policy. Each transient fixture sends two requests from
separate logical calls. Each terminal, quota and overflow fixture sends
once. The failure retains partial text, response identity, projection
and unknown usage. A later completion cannot replace it. Each logical
call settles one error span. Codex SSE and Azure controls verify their
own field selection with one request each.
[Models test](../../../../crates/opi-ai/tests/openai_responses_fixtures.rs#L691) ·
[Codex test](../../../../crates/opi-ai/tests/openai_codex_responses/wire.rs#L342) ·
[Azure test](../../../../crates/opi-ai/tests/azure_openai_fixtures.rs#L1216)

## Codex WebSocket recovery

The shared field selection supplies the exact code to the existing
transport recovery guards and the safe summary to the public result
after exhaustion. The #83 loopback tests checked one pre-event reconnect,
full-input retry with a nested missing-response code, a retained
partial after public output, and two upgrades with zero SSE sends on
a repeated connection limit. The latter produced a retryable whole-call
diagnostic when the selected message describes an upstream failure.
[#83 repeated-limit baseline](../evidence/phase5-responses-error-classification.json) ·
[Partial output](../../../../crates/opi-ai/tests/openai_codex_responses/ws_protocol_regressions.rs)

The pinned Codex recovery branches at lines 356–386 and 1496–1507 show
two policy differences. Opi protects already delivered output by
allowing a full-input retry only before any public stream item.
The Pi branch can still retry after starting output. Pi also enters
SSE fallback on a repeated pre-event connection limit, while Opi ends
the call after its one reconnect. Opi's existing branch dates to
`5dc3bdbb1f975d858ce5645460bf32fb86304524`; the event and output guards
were split in `59d2430ac9f85a646cb56bab076db8b4d14fc667`.
The paired follow-up aligns the repeated pre-event limit with Pi's
bounded SSE fallback. [#84](84-fallback-after-codex-connection-limit.md)

## Verification

- Three frozen runs produce identical 18-row observations. The
  controlled comparisons restore seven transient decisions, two quota
  diagnostics and one overflow signal. The three formerly ignored
  frames each produce one terminal failure. The two differences from
  whole-payload classification are recorded in the receipt.
  [Frozen probe](../tools/observe_responses_error.mjs) ·
  [Classification receipt](../evidence/phase5-responses-error-classification.json)
- The pinned Codex reconnect and cached-continuation tests pass all
  three selected tests. The probe's missing-snapshot control exits 2.
- Windows Rust `openai_responses_fixtures` (84),
  `azure_openai_fixtures` (45), `openai_codex_responses` (79) and
  `whole_call_retry` (23) pass. The shared Responses unit test passes.
  The three provider targets pass 208 tests with
  `--no-default-features`. Affected-target Clippy and rustdoc pass
  with warnings denied. Cargo format, `python -B scripts/opi-doc-check.py`
  and `git diff --check` pass.
- The #82 material manifest rejects the changed Rust source with exit 2.
  The original-lock freeze binds the same Pi snapshot and the new Rust
  inputs with zero material blockers. All 24 case records match #82,
  including the shared assertions. The audit verifies 96 trace
  receipts. The #82 driver is preserved under
  `C:/Temp/opi-ai-lifecycle-executables/` by SHA-256
  `8e5101fc601a659a6719461149460acf0920dc2bf5efe512df198e3a0a531e2b`.
  [Materials](../evidence/phase5-responses-error-materials.json) ·
  [Lifecycle replay](../evidence/phase5-responses-error-differential.json)

Live-provider behavior, full acceptance gates and the ten recorded
deferred lifecycle differences remain open.

Test impact `add` for the shared SSE and WebSocket terminal controls,
`update` for the nested continuation recovery case and its request
assertions, and `retain` for the #82 controls and paired lifecycle cases.
Worktree handoff `committed=none-for-this-task` and `staged=none`.
The OpenAI and Azure entry sources and OpenAI Responses fixture target
are task-owned unstaged edits. The shared and Codex sources, Azure and
Codex tests, bilingual guides, changelog, brief, index, #82 issue and
replay guide are carried-in and touched. The #83 issue, fixture, probe
and three receipts are new untracked files.

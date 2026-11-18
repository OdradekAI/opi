# Recover exact Codex codes carried by response.failed

Completed 2026-11-12 against Pi 1.5.0
`f9a26365568127461c47a07a37dcb42c140a3ac5` with its original npm lock.
The nine shared rows exercise Pi's injected transport and the Rust
Codex adapter over loopback. They record request counts, cached input,
public output, raw callbacks and the final result. Counts start with
the measured call, after any cache warmup.
[Fixture](../../../../crates/opi-ai/tests/data/codex-failed-recovery.json) ·
[Pi probe](../tools/observe_codex_failed_recovery.mjs) ·
[Rust test](../../../../crates/opi-ai/tests/openai_codex_responses.rs#L2421)

## Recovery before output

Pi takes the code of `response.failed` from `response.error.code`
and creates the same typed `CodexApiError` used by the WebSocket recovery
loop. Pi 1.3.0 `71023aec1546d6d677a65669c5665c0927ce0dd0` already did
this. The earlier Opi WebSocket dispatch selected a recovery code only
for `error` frames. Its shared `response.failed` mapper supplied the
terminal summary and raw status. That split accounts for the three
baseline failures.

- Pi 1.5.0 `packages/ai/src/api/openai-codex-responses.ts` lines
  739–745 compare the exact codes. Lines 747–788 select fields by event
  shape, and lines 356–386 enforce the two recovery budgets.
- Pi 1.3.0 uses the corresponding paths at lines 723–729, 731–772
  and 342–372. Opi's frame-specific dispatch dates to
  `5dc3bdbb1f975d858ce5645460bf32fb86304524` in
  `crates/opi-ai/src/transport/codex.rs`.

Opi now takes the nested string code from a decoded `response.failed`
error and uses the existing WebSocket budgets. A limit before the
first typed provider event admits one reconnect. A second qualifying
limit enters the existing SSE fallback. A missing previous response
before any public stream item admits one full-input retry. The
terminal failure carries the final wire status and the fixed
diagnostic produced from the selected message.
[Code dispatch](../../../../crates/opi-ai/src/transport/codex.rs#L1190) ·
[Failure mapping](../../../../crates/opi-ai/src/api/shared/openai_responses.rs#L728)

| Shared case | Baseline Opi | Frozen Pi and updated Opi |
| --- | --- | --- |
| Nested limit before any provider event | 1 WS, error | 2 WS, recovered |
| Flat limit followed by a nested limit | 2 WS, error | 2 WS and 1 SSE, recovered |
| Cached nested missing response after a silent progress event | 1 WS, error | 2 WS, recovered with full input |
| Flat missing response followed by a nested missing response | 2 WS, error | 2 WS, error |
| Nested limit after a typed progress event | 1 WS, error | 1 WS, error |
| Recovery codes in unselected fields | 1 WS, error | 1 WS, error |
| Nested missing response on SSE | 1 SSE, error | 1 SSE, error |
| Completion before a nested limit frame | 1 WS, recovered | 1 WS, recovered |

The cached rejection sends `previous_response_id: resp_warm` with
one input item. The retry uses the original two-item input. The
mixed missing-response case reaches exhaustion across both shapes
and preserves `failed` plus the second frame's transient diagnostic
in Opi. The public result uses only the fixed local diagnostic.
The eight rows match in sends, callback order,
final text, response ID and stop reason. The evidence retains the
observed Start events, diagnostics and failure usage representations.

## Missing continuation after delivered text

With the same cached request, the peer delivers `partial` and then
rejects the continuation. Frozen Pi opens another socket and sends
the full input. The two text deltas appear in one successful stream,
whose result is `partialrecovered`. Opi returns one typed failure with
`partial`, `resp_partial` and raw status `failed` after the first send.
This keeps the text already delivered to the caller associated with
the failed response. Replaying the request at that point can append
content from a second generation to the same public stream. The
fixture records this result and send-count difference.
[Paired observations](../evidence/phase5-codex-failed-recovery-observations.json)

## Verification

The focused red run records the three missing recovery paths.
Three identical original-lock Pi probe runs cover all nine rows. The
missing-snapshot control exits 2. The frozen Codex Vitest result of
40 passing tests is applicable from #84 at the same source and lock.

Windows Rust passes `openai_codex_responses` (80),
`codex_native_options` (21), `openai_responses_fixtures` (84),
`azure_openai_fixtures` (45) and `whole_call_retry` (23) with
default and with no default features. The shared mapper and WebSocket
close-deadline unit controls also pass. Affected-target Clippy and
rustdoc pass with warnings denied. Cargo format, the doc checker and
`git diff --check` pass.

The #84 material manifest rejects the changed Rust source with exit
2 and zero replay cases. The new freeze binds the same original npm
lock. All 24 lifecycle cases match #84 and pass their shared
assertions. The report records zero material blockers and ten deferred
differences.
[Materials](../evidence/phase5-codex-failed-recovery-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-failed-recovery-differential.json)

The replay audit verifies 96 trace receipts. The #84 executable remains
available under `C:/Temp/opi-ai-lifecycle-executables/` by SHA-256
`5a5725bf4fad09047b3c5aaf3eff53bf5f8376650b4dda8e681d047102736895`.

The next Codex comparison is the `response.done` terminal and its
stop metadata. Real-provider behavior and the six acceptance gates
remain open.

Test impact `add` for the shared recovery fixture, Pi probe and
loopback regression, and `retain` for the existing WS, Responses
failure and lifecycle controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The WebSocket owner, shared mapper, Codex test target, bilingual
transport guides, changelog, brief, contract index, #83 and #84 issues,
and differential guide are carried-in and touched. The fixture,
frozen probe, issue and three receipts are new untracked files.

# #86 Align Codex terminal frames

Status: verified against frozen Pi and local Rust peers on SSE and WebSocket.
Real-provider acceptance remains open.

[Shared frames](../../../../crates/opi-ai/tests/data/codex-terminal.json) ·
[Pi probe](../tools/observe_codex_terminal.mjs) ·
[Rust loopback](../../../../crates/opi-ai/tests/openai_codex_responses/terminal.rs#L32) ·
[Paired results](../evidence/phase5-codex-terminal-observations.json)

## Terminal frame and result

The same 16 scenarios run on each transport. Before the change, Opi
disagreed with Pi on the stop reason in 22 of the 32 observations.
After the change, all 32 match in send count, stop reason, raw stop
reason, response ID, token usage, text or tool content, and the
reported boolean `end_turn`. The content and terminal event sequences
match after accounting for Start. These frames omit `response.created`.
Pi emits Start at handoff or the first mapped event. Opi emits Start
on `response.created`.

| Provider terminal | Baseline Opi | Updated Opi and frozen Pi |
| --- | --- | --- |
| `response.done` with status `completed` | Truncated-stream error | `Stop` or `ToolUse` |
| `response.done` with status `incomplete` and `max_output_tokens` | Truncated-stream error | `Length` |
| `response.completed` with that same incomplete status and tool content | `ToolUse` | `Length` |
| `response.incomplete` with `max_output_tokens` | `Length`, terminal ID and model lost | `Length` and terminal response ID |
| `response.incomplete` with content filtering or no reason | `Length` | Error with partial content and usage |
| `response.completed` with status `failed` | `Stop` | Error with partial content and usage |
| `response.done` with status `cancelled` | Truncated-stream error | Error with raw status `cancelled` |

Codex reads all three terminal frame names through the same mapping.
The payload must contain a response. An absent, unknown or non-string
status uses the normal completion path. A reported `queued` or
`in_progress` in one of these terminal frames also follows that path,
with its raw status retained. Only a normal completion can promote
its stop reason to `ToolUse`. The Rust result also retains the
provider's reported model. Ordinary OpenAI and Azure keep their
existing decoding scope.
[Decode](../../../../crates/opi-ai/src/api/shared/openai_responses.rs#L339) ·
[Status map](../../../../crates/opi-ai/src/api/shared/openai_responses.rs#L599)

Pi 1.5.0 revision `f9a26365568127461c47a07a37dcb42c140a3ac5` handles
the aliases in `packages/ai/src/api/openai-codex-responses.ts:791–809`.
Its shared terminal mapper at
`packages/ai/src/api/openai-responses-shared.ts:624–668,861–885` uses the
status and incomplete reason. Both behaviors are present in Pi 1.3.0
revision `71023aec1546d6d677a65669c5665c0927ce0dd0` at the corresponding
paths and lines 775–793 and 577–621,804–828. Opi's older named
`response.completed` dispatch dates to
`1833876d3f69089fd2d999d3ddc03519b2016d40`, and the separate
`Length` path for `response.incomplete` dates to
`47400ee3edde41ff06ed69924139de938b949eb9`.

## Diagnostics and resource boundaries

Pi added the optional diagnostic field `endTurn` in
`c3e7bc60a13b0a6fb80f7e8a867112ee4d10c92a`. At the 1.5.0 pin, its
production source writes the field in the Codex adapter and describes
its debugging purpose in `packages/ai/src/types.ts:603–607`.
Opi exposes `AssistantMessage::end_turn() -> Option<bool>`. Storing
the boolean as a message diagnostic preserves `true`, `false` and
absence through serialization and keeps existing Rust struct literals
source-compatible. Cached continuation uses the successful terminal's
response ID. The loopback control continues from a `response.done`
carrying `end_turn: false` and sends just the new input item on the same
socket.
[Accessor](../../../../crates/opi-ai/src/types/message.rs#L93) ·
[Cached continuation](../../../../crates/opi-ai/tests/openai_codex_responses/ws_continuation.rs#L198)

Opi uses fixed error diagnostics and keeps the raw incomplete reason.
The `incomplete.unknown_503` row exposes a retry difference. Pi's
`isRetryableAssistantError` returns true because it matches the digits
in the provider reason. Opi returns a typed stream failure with terminal
whole-call classification and retains the raw reason for inspection.
An unrecognized incomplete reason supplies no evidence of an HTTP
503 or a transient failure. Both adapters send once in each terminal
fixture.

The prepared SSE test delivers a terminal and a late failure in a
chunk while the server holds the response body open. Opi invokes the
raw callback through the first terminal, delivers one result and
releases the body. The sessionless WebSocket fixture verifies one
terminal and the same callback cutoff for that call.
[SSE cutoff](../../../../crates/opi-ai/src/api/openai_codex_responses.rs#L330) ·
[Held body test](../../../../crates/opi-ai/tests/openai_codex_responses/terminal.rs#L232)

## Verification

The baseline Rust run records 32 observations and two failing tests.
Three identical Pi probe runs use the original-lock snapshot. The
missing-snapshot invocation exits 2. The frozen Codex Vitest result
of 40 passing tests from #84 applies at the same source and lock.

Windows Rust passes `openai_codex_responses` (82),
`codex_native_options` (21), `openai_responses_fixtures` (84),
`azure_openai_fixtures` (45), `whole_call_retry` (23) and
`stream_events` (30) with default features and with no default
features. Three shared mapper unit tests and one WebSocket
close-deadline test pass. Affected-target Clippy and rustdoc pass
with warnings denied. Cargo format, the doc checker and
`git diff --check` pass.

The #85 material manifest rejects the changed Rust source with exit 2
and zero replay cases. The new manifest binds the same original
`package-lock.json`. All 24 lifecycle case records match #85 and pass
their shared assertions. The audit verifies 96 trace receipts. The
report records zero material blockers and ten deferred differences.
[Materials](../evidence/phase5-codex-terminal-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-terminal-differential.json)

The next comparison is a pooled WebSocket that receives a late
terminal from the previous request. Test whether the next request
can consume it when the driver retains already-read bytes. The
six acceptance gates and real-provider behavior remain open.

Test impact `add` for the shared fixture, Pi probe, paired Rust
terminal test, held SSE body test and wire-scoping unit control.
Test impact `update` for the cached continuation control and
`retain` for the existing Responses, retry, stream and lifecycle
controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The Codex SSE owner, shared mapper, message type, Codex test target,
bilingual transport guides, changelog, brief, contract index, prior
issue anchors and differential guide are carried-in and touched. The
diagnostic type was clean at task start. The fixture, probe, terminal
test module, issue and three evidence files are new.

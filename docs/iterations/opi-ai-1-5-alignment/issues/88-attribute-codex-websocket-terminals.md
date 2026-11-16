# #88 Attribute Codex WebSocket terminals after the next create

The loopback peer sends the previous response's terminal only after
receiving the second `response.create`. Twelve of the 36 pairs exercise
an old terminal after `response.created` has established a distinct
second ID. Both baseline Opi and frozen Pi accept the old terminal in
those pairs. Updated Opi returns the second response with its own ID,
text, eight tokens, `end_turn: true` and one `Done`.
[Shared frames](../../../../crates/opi-ai/tests/data/codex-websocket-identity.json) ·
[Pi loopback probe](../tools/observe_codex_websocket_identity.mjs) ·
[Rust loopback](../../../../crates/opi-ai/tests/openai_codex_responses/websocket_idle.rs#L363) ·
[Paired observations](../evidence/phase5-codex-websocket-identity-observations.json)

| Previous terminal arriving after the second `response.created` | Baseline Opi and frozen Pi | Updated Opi |
| --- | --- | --- |
| `response.completed` | Empty text, `resp_first`, 79 tokens | `second answer`, `resp_second`, 8 tokens |
| `response.done` after text | `second answer` with old ID, 79 tokens and `end_turn: false` | Same text with the current ID, 8 tokens and `end_turn: true` |
| `response.incomplete` | `Length` with the old ID and usage | `Stop` with the current ID and usage |
| `response.failed` with the connection-limit code after text | Failure with the second response's partial text | Successful second response |

Every pair uses one upgrade and zero SSE POSTs. The first two creates
carry the same input counts and `previous_response_id` on both sides.
The third call in the after-text case feeds the actual second message
back into the transcript. `Auto` and `WebsocketCached` send one new
item using `resp_second`; baseline Opi and Pi send it using `resp_first`.
`Websocket` sends four full input items. All three complete the third
call with the peer's `resp_third` on the same socket.

## Identity and compatibility

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` sends a create
and installs a generation's parser in
`packages/ai/src/api/openai-codex-responses.ts:1558–1569`. It calls the
raw observer before terminal dispatch at lines 766–800. In
`packages/ai/src/api/openai-responses-shared.ts:624–630,671–674`, a
terminal ID can replace the ID read from `response.created`. Pi 1.3.0
at `71023aec1546d6d677a65669c5665c0927ce0dd0` uses the same policy in
those files at lines 750–784,1542–1553 and 577–583,624–627 respectively.
The frozen Pi manifest pins `openai@7.19.0`. Its
`resources/responses/responses.d.ts:773–777` describes `Response.id`
as unique. The created, completed, incomplete and failed event types
carry the response object. Pi's Codex alias `response.done` passes that
object to the same terminal mapper.

Opi borrows the last successful ID from the actual acquired socket.
This also applies when the client sends full input. Once
`response.created` establishes a different nonempty ID, an exact match
between a terminal's nested `response.id` and that previous ID excludes
the terminal from raw callbacks, recovery and message mapping. The call
continues reading the same socket with its existing request and retry
budget. This deliberately changes the result and callbacks for a frame
that identifies the previous response. The identity rule follows the
frozen schema's ID semantics and the observed continuation corruption.
[Socket continuation](../../../../crates/opi-ai/src/openai_codex_responses/ws_transport.rs#L835) ·
[Terminal guard](../../../../crates/opi-ai/src/openai_codex_responses/ws_transport.rs#L1161)

The other 24 pairs preserve baseline Opi behavior. They cover normal
completion, repeated IDs with and without `response.created`, a new
terminal ID distinct from both earlier IDs, an unsupported top-level
`response_id`, and the current response's connection-limit failure.
The current failure preserves `second answer` as partial text. Rust
represents absent token usage as `null`, and Pi reports `0` for that
failure. The evidence retains both raw values and normalizes this one
field only for the identity comparison.

If a previous terminal arrives before a distinct `response.created`,
or arrives without a nested ID, both adapters can still consume it.
Those two injected cases return 79 tokens and lack enough identity
evidence for this guard. The pinned create and observed responses have
no echoed per-create correlation value. Live provider observations are
needed to characterize those cases further.

## Verification

The baseline Rust integration emits all 36 rows and exits 101. An
initial test expectation used zero for absent Rust failure usage and
was corrected to `null`. Comparison of the captured rows shows that the
candidate changes exactly the twelve targeted pairs. Three runs of
the frozen Node probe produce identical output. The invocation without
`--snapshot` exits 2. The #87 idle-input results and retirement controls
replay exactly in the updated Rust target.

Windows Rust passes `openai_codex_responses` (85),
`codex_native_options` (21), `ws_upgrade` (16),
`openai_responses_fixtures` (84), `azure_openai_fixtures` (45),
`whole_call_retry` (23) and `stream_events` (30) with default features
and with no default features. The `ws::tests::` unit filter passes 89
tests and the Codex close-deadline unit passes. Affected-target Clippy,
rustdoc with warnings denied, formatting and documentation checks pass.

The #87 manifest rejects the changed Rust source with exit 2 and zero
cases. The new freeze binds the same original npm lock. The replay
matches all 24 #87 lifecycle case records and verifies 96 trace
receipts. It records zero material blockers and ten deferred differences.
[Materials](../evidence/phase5-codex-websocket-identity-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-websocket-identity-differential.json)

Next, compare the configured application idle timeout when only
WebSocket control traffic continues, before and after the first provider
event. Measure the close, fallback send and final result. Real-provider
behavior and six-gate acceptance remain open.
[#89 application idle timeout comparison](89-align-codex-websocket-application-idle-timeout.md)

Test impact `add` for the shared identity fixture, frozen Node probe
and three-mode Rust regression. Test impact `update` for the shared
loopback result capture and `retain` for the idle-input, recovery,
compression, cancellation and paired lifecycle controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The WebSocket transport, idle loopback test, bilingual transport guides,
changelog, prior issue anchors, iteration brief, contract index and
differential guide are carried-in and touched. The identity fixture,
probe, issue and three evidence files are new.

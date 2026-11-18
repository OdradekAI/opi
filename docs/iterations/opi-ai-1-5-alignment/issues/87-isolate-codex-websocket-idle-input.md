# #87 Isolate Codex WebSocket idle input

Verified against frozen Pi and a local Rust peer. The comparison
covers input readable at pooled acquisition before the next create.

[Shared frames](../../../../crates/opi-ai/tests/data/codex-websocket-idle.json) ·
[Pi loopback probe](../tools/observe_codex_websocket_idle.mjs) ·
[Rust loopback](../../../../crates/opi-ai/tests/openai_codex_responses/websocket_idle.rs#L85) ·
[Paired observations](../evidence/phase5-codex-websocket-idle-observations.json)

## What the second call received

Both peers write the first terminal and its trailing frames together.
They send the second response only after receiving the second
`response.create`. The five cases run in `Auto`, `WebsocketCached`
and `Websocket`. Baseline Opi differs from Pi in 12 of 15 pairs.

| First response tail | Baseline Opi second result | Frozen Pi and updated Opi |
| --- | --- | --- |
| Empty | `resp_second`, 8 tokens, `second answer` | Same |
| Duplicate `response.completed` | `resp_first`, 79 tokens, empty text | `resp_second`, 8 tokens, `second answer` |
| `response.done` without ID | Success with no ID, 79 tokens, empty text | `resp_second`, 8 tokens, `second answer` |
| Ping followed by duplicate completion | Old terminal, Pong after the second create | New terminal, Pong before the second create |
| Late `response.failed` | One failure from the prior call's frame | `resp_second`, 8 tokens, `second answer` |

All 15 updated pairs match the first and second result, raw callback
order, control frames and create count. Each uses one socket, two
creates and zero SSE POSTs. `Auto` and `WebsocketCached` send one
incremental input item with `previous_response_id: resp_first` on
the second turn. `Websocket` sends both input items.

## Receive boundary

The Rust driver returns one decoded message at a time. It retains
bytes after that message for its next poll. Pool publication preserves
the driver across calls, so a coalesced trailing frame was still
readable by the second stream. Opi added this ownership model with
the Codex WebSocket implementation at
`5dc3bdbb1f975d858ce5645460bf32fb86304524`. The private driver was
consolidated at `7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`.

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` attaches
listeners and creates a message queue per generation in
`packages/ai/src/api/openai-codex-responses.ts:1324–1439`.
Its Codex mapper returns at the first terminal at lines 791–800.
The reused socket gets a fresh queue on the next call. Pi 1.3.0 at
`71023aec1546d6d677a65669c5665c0927ce0dd0` has the same listener
lifecycle in the corresponding file at lines 1308–1423. The probe
uses the pinned source with Node's native WebSocket client and the
locked `ws@8.21.0` as the loopback server.

Opi now checks the old socket during acquisition before building a
delta or queuing a new create. The driver polls without waiting for
more bytes and distinguishes a complete message from a partial
header, fragmented message or pending decode. A complete idle
application message is consumed by the existing frame and PMD
decoder, then dropped before callbacks, retry classification or
response mapping. A readable Ping gets its Pong before the next
create. The boundary is based on when the bytes are read. The fixture
includes a late terminal without an ID, and serial clean tests exercise
equal response IDs across turns.
[Driver read](../../../../crates/opi-ai/src/ws/io.rs#L441) ·
[Idle input handling](../../../../crates/opi-ai/src/transport/codex.rs#L606)

Opi retires the old socket when it has incomplete input or reaches
the bounded idle read allowance. A readable Close is acknowledged and
also retires the socket. Three Rust controls exercise one header byte,
peer Close and the idle message allowance. Each observes two upgrades
with one create per socket, a full-input second request, zero SSE
POSTs and the correct second result. The generation holds its pool
reservation through replacement.
[Retirement controls](../../../../crates/opi-ai/tests/openai_codex_responses/websocket_idle.rs#L232) ·
[Fragment control](../../../../crates/opi-ai/src/ws/tests/io.rs#L13)

## Verification

The baseline Rust integration records all 15 rows and fails the shared
assertion for the 12 affected pairs. Three frozen Pi loopback runs
produce identical rows. The probe invocation without its snapshot
exits 2.

Windows Rust passes `openai_codex_responses` (84),
`codex_native_options` (21), `ws_upgrade` (16),
`openai_responses_fixtures` (84), `azure_openai_fixtures` (45),
`whole_call_retry` (23) and `stream_events` (30) with default
features and with no default features. The `ws::` library filter passes
142 tests, including 98 WebSocket tests. The Codex close-deadline test,
affected-target Clippy, rustdoc, formatting and documentation checks
also pass.

The #86 manifest rejects the changed Rust source with exit 2 and zero
cases. The new freeze uses the same original npm lock. All 24
lifecycle case records match #86 and pass their shared assertions.
The audit verifies 96 trace receipts. The replay records zero material
blockers and ten deferred differences.
[Materials](../evidence/phase5-codex-websocket-idle-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-websocket-idle-differential.json)

The next case delivers the previous terminal only after the second
create, when neither adapter can assign it by the idle boundary.
Compare the response identity available at that point and after
`response.created`. Real-provider behavior and six-gate acceptance
remain open.
[#88 post-send identity comparison](88-attribute-codex-websocket-terminals.md)

Test impact `add` for the shared fixture, frozen loopback probe,
paired Rust stream and socket replacement controls, and the driver
read control. Test impact `retain` for the existing continuation,
cancellation, compression, upgrade and lifecycle controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The WebSocket transport, parent Codex test target, bilingual transport
guides, changelog, iteration brief, contract index, prior issue anchors
and differential guide are carried-in and touched. The frame decoder,
IO driver and IO unit test file were clean at task start. The fixture,
probe, loopback module, issue and three evidence files are new.

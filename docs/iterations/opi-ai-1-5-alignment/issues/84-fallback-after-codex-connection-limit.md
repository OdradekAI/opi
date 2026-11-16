# Fall back after a repeated Codex WebSocket connection limit

Completed 2026-11-12 against Pi 1.5.0
`f9a26365568127461c47a07a37dcb42c140a3ac5` with its original npm lock.
The shared fixture uses the same exact `error` frame in Pi's injected
WebSocket and Opi's loopback peer. The outer code identifies a connection
limit even when the nested code and message describe another error.
[Fixture](../../../../crates/opi-ai/tests/data/codex-connection-limit.json) ·
[Pinned probe](../tools/observe_codex_connection_limit.mjs)

## Decision

In the baseline Rust `Auto` call, two `response.create` sends each received
`websocket_connection_limit_reached` before the first typed provider
event. The call returned a redacted `StreamError` and sent zero SSE
requests. Pi 1.5 sends one SSE POST after those two rejections in each
WebSocket mode and returns a successful answer. Pi 1.3.0
`71023aec1546d6d677a65669c5665c0927ce0dd0` already had this branch.
Opi's earlier terminal path dates to `5dc3bdbb1f975d858ce5645460bf32fb86304524`
in `crates/opi-ai/src/openai_codex_responses/ws_transport.rs`. The prior
behavior changed the final result even though both provider rejections
identified the WebSocket connection limit.

Pi source paths are relative to the pinned checkout.

- `packages/ai/src/api/openai-codex-responses.ts` lines 356–386 choose one
  reconnect and then SSE before the first mapped event in Pi 1.5.0.
  The corresponding Pi 1.3.0 branch is at lines 342–372.
- Pi 1.5.0 lines 391–420 use the original request on the SSE route.
  Lines 747–781 choose the error code, 984–1003 record the session
  marker, and 1598–1607 release the rejected connection and continuation.

Opi now sends the second qualifying rejection through its existing SSE
handoff with the fixed reason `websocket_connection_limit_reached`. The
same resource generation owns the marker and the cancellable SSE leg.
Both transports use the same transformed request and callbacks. The
successful Upgrade seals the advanced acquisition budget, including when
the fallback POST ends before response headers. An HTTP peer that closes
after receiving the POST may have accepted generation. Opening a new
prepared attempt would resend that logical call. Pi defaults its SSE
retry budget to zero. In the controlled failed-POST case, explicitly
setting Pi's SSE budget to one sends a second POST. Opi's advanced
acquisition policy continues to observe `PostHandoff`.
[WS decision](../../../../crates/opi-ai/src/openai_codex_responses/ws_transport.rs#L557) ·
[SSE owner](../../../../crates/opi-ai/src/openai_codex_responses.rs#L794) ·
[Acquisition test](../../../../crates/opi-ai/tests/openai_codex_responses.rs#L4706)

## Observed outcomes

- Cold `Auto`, `Websocket`, `WebsocketCached` and a warm cached
  session each send two WS requests and one SSE POST. The normalized
  answer is `Hello from SSE` with stop reason `stop`, response ID
  `resp_sse` and eight total tokens. Both adapters observe one payload
  callback, two raw errors, the SSE response and the same SSE event
  callbacks in order. Each produces one terminal. The next call in the
  same session sends only one SSE POST.
- The warm case first rejects an increment on the reused socket with
  `previous_response_id: resp_warm`. The reconnect and SSE POST use the
  same full two-message request. Within each adapter the full WS and
  decoded SSE JSON match after removing the WS frame's `type` field.
  The two adapters encode single text inputs as a string in Opi and
  one `input_text` block in Pi. The remaining decoded fields match.
- `response.in_progress` before the limit seals recovery in both
  adapters. The Rust result retains a typed, redacted failure and
  sends no SSE request. The independent missing-continuation control
  preserves delivered text after a public delta.
- Rust cleanup cancels the active fallback SSE stream, clears the session
  marker and admits WebSocket again on the next call. Pi's injected
  sockets and Rust's loopback peer observe release of the two rejected
  connections before SSE dispatch.

The paired output records the existing Start and diagnostic differences.
Pi emits Start at SSE response handoff even when the fixture lacks
`response.created`. Opi emits it when that provider event arrives. Pi
attaches `provider_transport_failure` to the recovered call. Opi attaches
the fixed `codex_websocket_fallback` diagnostic and reports
`session_fallback` on a later sticky SSE call. The Rust messages exclude
the raw error canary.
[Start mapping](../../../../crates/opi-ai/src/openai_responses_shared.rs#L1108) ·
[Text projection](../../../../crates/opi-ai/src/openai_responses_shared.rs#L1992) ·
[Send and result test](../../../../crates/opi-ai/tests/openai_codex_responses.rs#L2697) ·
[Cleanup test](../../../../crates/opi-ai/tests/openai_codex_responses.rs#L3774) ·
[Paired observations](../evidence/phase5-codex-connection-limit-observations.json)

## Verification

Three identical frozen probes exercise seven rows. The missing-snapshot
control exits 2. The pinned Codex Vitest target passes all 40 tests.
Windows Rust `openai_codex_responses` (79), `codex_native_options` (21)
and `whole_call_retry` (23) pass with default and with no default
features. The WebSocket close-deadline unit test passes. Affected-target
Clippy and rustdoc pass with warnings denied. Cargo format, the doc
checker and `git diff --check` pass.

The #83 manifest rejects the changed Rust source with exit 2 and zero
cases. The new freeze binds the same snapshot and original
`package-lock.json`. All 24 case records match #83 and the shared
assertions pass. There are zero material blockers and ten recorded
deferred differences.
[Materials](../evidence/phase5-codex-connection-limit-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-connection-limit-differential.json)

The replay audit verifies 96 trace receipts. The #83 executable remains
available by SHA-256
`e385a5a2fb7ccb2f1b9ea758c225ec7a0970b132b2b2eec3e8a90ed2bb8bd6d4`
under `C:/Temp/opi-ai-lifecycle-executables/`.

The follow-up verifies exact recovery codes inside `response.failed`.
Real-provider behavior and the six acceptance gates remain open.
[#85](85-recover-codex-response-failed-codes.md)

Test impact `add` for the shared fixture and frozen probe, `update` for
the repeated-limit, ignored-event, acquisition and cleanup regressions,
and `retain` for the missing-continuation guards and 24 lifecycle cases.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`provider.rs` is a task-owned unstaged rustdoc edit. The WebSocket
owner, Codex tests, transport guides, changelog, brief, contract index,
#83 issue and differential guide are carried-in and touched. The
fixture, frozen probe, issue and three receipts are new untracked files.

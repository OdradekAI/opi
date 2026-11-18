# #89 Align Codex WebSocket application idle timeout

Updated Opi and frozen Pi agree on the Close, request counts, callback
order and current and next-call results in 27 loopback pairs. The peer
uses a 200 ms idle budget and sends control traffic every 40 ms. It
can produce a successful terminal after ten ticks if the socket stays
open. Baseline Opi consumes that terminal in twelve control-traffic
cases whose application waits should have expired.
[Shared fixture](../../../../crates/opi-ai/tests/data/codex-websocket-idle-timeout.json) ·
[Frozen Pi probe](../tools/observe_codex_websocket_idle_timeout.mjs) ·
[Rust loopback](../../../../crates/opi-ai/tests/openai_codex_responses/websocket_idle_timeout.rs#L193) ·
[Paired observations](../evidence/phase5-codex-websocket-idle-timeout-observations.json)

| Wait boundary | Baseline Opi | Updated Opi and Pi |
| --- | --- | --- |
| Ping, Pong or silence before any provider event | Control frames extend the wait. Silence falls back after TCP release. | Close `1000 / idle_timeout`, one full-input SSE POST, then SSE for the next call |
| Ping or silence after text, or Pong after `response.in_progress` | Control frames extend the wait. Silence returns a partial failure and the next call opens another WebSocket. | Close `1000 / idle_timeout`, one partial failure, then SSE for the next call |
| Application progress, zero timeout with Ping, or a 320 ms raw callback | WebSocket completes | WebSocket completes |

The eighteen expiry pairs across `Auto`, `WebsocketCached` and
`Websocket` each send one `response.create`. Before a provider event,
the current call sends one SSE POST with the full input. After a
provider event, the current call settles with one error and zero SSE
POSTs. All eighteen send the subsequent call by SSE with two input
items and no continuation ID. Ping receives Pong during the active
wait. The raw callback sees only application events. The after-text
failure keeps `resp_partial` and `partial answer`. The other nine
pairs complete over the first WebSocket with no SSE.

## Timer and close ownership

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` starts its
timer when the application queue is empty in
`packages/ai/src/api/openai-codex-responses.ts:1398–1425` and calls
`socket.close(1000, "idle_timeout")` at expiry. The native WebSocket
client handles the control frames. The adapter records the sticky
session marker at lines 368–386 and 995–1000, before returning the
post-start partial failure or falling back on the current call.
Pi 1.3.0 at `71023aec1546d6d677a65669c5665c0927ce0dd0` has the same
behavior at lines 1382–1409, 354–372 and 979–985 of that file. The
loopback probe uses the pinned Node native client and the locked
`ws@8.21.0` server.

Opi's initial Rust WebSocket adapter at
`5dc3bdbb1f975d858ce5645460bf32fb86304524` gave each driver read a
fresh timeout. The receive loop now keeps one absolute deadline
through Ping, Pong and pending Pong writes. Processing a current
application message, including the raw callback and public delivery,
starts a new wait. The biased select gives cancellation and an
expired deadline precedence over more IO. A discarded old terminal
from #88 keeps the current deadline. Zero disables the timer.
[Receive deadline](../../../../crates/opi-ai/src/transport/codex.rs#L1074)

Opi commits the session marker under the generation cutoff before
waiting for Close. It uses the existing 250 ms confirmation budget
for any pending Pong write, masked Close, peer Close and EOF. The
driver retains its owned frame and PMD state when the receive borrow
expires. The expired attempt consumes any frames already in flight
during Close without invoking callbacks. The native Pi client initiates
Close and lets its socket handle confirmation. Opi awaits confirmation
within the fixed local budget and records unconfirmed release for
resource cleanup. Cancellation during release still prevents a late
SSE handoff.
[Marker and cutoff](../../../../crates/opi-ai/src/transport/codex.rs#L933) ·
[Close confirmation](../../../../crates/opi-ai/src/transport/codex.rs#L970)

The post-event error keeps Opi's typed `StreamError` and gives it a
fixed idle-timeout diagnostic. The provider attempt stays sealed.
The public opt-in whole-call classifiers in both runtimes recognize
the timeout as transient. Opi's typed cause stays nonretryable for
acquisition. The comparison records each native diagnostic and
normalizes absent failure usage (`null` in Rust and `0` in Pi) only
in the shared projection.

## Verification

Baseline Rust emits all 27 rows and fails 18, including the six silent
cases with different Close or subsequent session routing. Three
frozen probes emit identical rows. The probe invocation without
`--snapshot` exits 2. The updated Rust test also reproduces the #87
idle-input and #88 post-send identity records exactly.

Windows Rust passes `openai_codex_responses` (86),
`codex_native_options` (21), `ws_upgrade` (16),
`openai_responses_fixtures` (84), `azure_openai_fixtures` (45),
`whole_call_retry` (23) and `stream_events` (30) with default and no
default features. The `ws::tests::` library filter passes 89 tests.
The three Close deadline units pass, including masked payload and
in-flight Pong, and the release bound for a partially written Pong.
Affected-target Clippy, rustdoc, formatting and documentation checks
pass.
[Close controls](../../../../crates/opi-ai/src/transport/codex.rs#L1303)

The #88 manifest rejects the source change with exit 2 and zero
cases. The fresh freeze uses the same original npm lock. The replay
matches all 24 #88 lifecycle cases, with 96 verified trace receipts,
zero material blockers and ten deferred differences.
[Materials](../evidence/phase5-codex-websocket-idle-timeout-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-websocket-idle-timeout-differential.json)

Next, compare the same-session route after the public resource cleanup.
Pi's registered Codex close callback removes socket entries. The
debug reset is the separate owner that clears the sticky marker in
the frozen source. Compare this through the public cleanup entry and
the next real loopback call. Real-provider behavior and six-gate
acceptance remain open.
[#90 public cleanup comparison](90-preserve-codex-session-cleanup-recovery.md)

Test impact `add` for the shared timeout fixture, frozen probe, Rust
loopback and two close controls. Test impact `update` for the shared
loopback peer's stop condition. Test impact `retain` for the #87/#88
controls, recovery, cancellation, PMD and paired lifecycle cases.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The WebSocket transport, parent Codex test target, bilingual guides,
changelog, prior issue anchors, iteration brief, contract index and
differential guide are carried-in and touched. The fixture, Rust
loopback, probe, issue and three evidence files are new.

# #90 Preserve Codex WebSocket recovery after session cleanup

The public cleanup comparison covers targeted and global cleanup in
`Auto`, `WebsocketCached` and `Websocket`. Each pair seeds two sticky
sessions with the #89 idle-timeout frames. The target expires before
the first application event and completes over SSE. The second session
expires after a text delta and returns the partial message. Both sessions
then send a full SSE request on their next call. A third session holds a
healthy pooled WebSocket through the cleanup boundary.
[Shared frames](../../../../crates/opi-ai/tests/data/codex-websocket-idle-timeout.json) ·
[Frozen Pi probe](../tools/observe_codex_session_fallback_cleanup.mjs) ·
[Rust regression](../../../../crates/opi-ai/tests/openai_codex_responses/session_fallback_cleanup.rs#L168) ·
[Paired observations](../evidence/phase5-codex-session-fallback-cleanup-observations.json)

| Cleanup selector | Call after cleanup | Opi | Frozen Pi |
| --- | --- | --- | --- |
| Targeted | Target, already on SSE | Fresh WebSocket | SSE |
| Targeted | Other, already on SSE | SSE | SSE |
| Targeted | Healthy control | Reused WebSocket | Reused WebSocket |
| Global | Both sessions already on SSE | Fresh WebSocket per session | SSE per session |
| Global | Healthy control | Closed, then fresh WebSocket | Closed, then fresh WebSocket |

The nine selected follow-up calls with different routes each send the
same full two-message input and session cache key, with no continuation
ID. Opi sends one create and receives the WebSocket fixture's ID and
usage. Pi sends one POST and receives the SSE fixture's ID and usage.
Both deliver one successful terminal and the same raw terminal callback.
The wire route and the returned fixture fields differ. The fixture
establishes no difference in live model text.

The targeted healthy control keeps the previous response ID and a
one-message increment in both cached modes. Explicit `Websocket` reuses
the socket with full input. Global cleanup closes that control with
code `1000` and the next call uses full input on a new socket. Pi sends
reason `debug_close` and Opi sends an empty reason. Both initial idle
closes carry `1000 / idle_timeout`. The report records the native
timeout diagnostic and missing failure usage (`null` in Opi, `0` in Pi).
Opi's awaited cleanup reports one marker and zero pooled closes for the
targeted selector, and two markers and one confirmed close for the
global selector. Pi's public call synchronously returns `undefined`
after dispatching its registered cleanup.

## Decision

Keep Opi's documented behavior. An explicit resource cleanup ends the
selected transport state, including the policy that routes that session
to SSE. A caller can reuse the same session ID and try WebSocket again
with full input. The marked session uses SSE until cleanup, and a
subsequent failure can set the marker again. Targeted cleanup
preserves the other session's route and the healthy connection. The
generation cutoff serializes marker removal with pending calls and the
cleanup report confirms the selected local releases.
[Public cleanup](../../../../crates/opi-ai/src/models.rs#L2311) ·
[Marker removal](../../../../crates/opi-ai/src/transport/codex.rs#L167) ·
[Existing transport contract](../../../../crates/opi-ai/docs/transport.md#websocket-upgrade)

Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` exports
`cleanupSessionResources` from `packages/ai/src/index.ts:33`.
`packages/ai/src/session-resources.ts:12–24` invokes the callbacks.
The Codex callback removes cached sockets and their timers.
`resetOpenAICodexWebSocketDebugStats` clears the fallback set.
The two functions are adjacent in
`packages/ai/src/api/openai-codex-responses.ts:956–982`.
Pi 1.3.0 at `71023aec1546d6d677a65669c5665c0927ce0dd0` has the same
split at lines 940–966. The probe uses the frozen built package root
and a real Codex provider with a loopback auth resolver. Its cleanup
invocations use the public package export.

Pi's history accounts for the split. Commit `4745a95898` introduced the
close helper in the then-current
`packages/ai/src/providers/openai-codex-responses.ts`.
`370fdae6fa` added sticky fallback and put marker removal in the debug
reset. `23420012ab` registered the existing close helper with public
cleanup. The reviewed source and tests give no reason to keep a transport
failure across explicit cleanup. The Codex stream test teardown calls
both helpers in `packages/ai/test/openai-codex-stream.test.ts:27–28`.
Opi's separate design dates to resource generation commit
`bfdde41d521ec284d9fcf79b0f5ec4ec480bec89`, which introduced marker removal
as part of the cleanup report. The existing bilingual transport guides
already state this contract.

Pi's session runtime awaits the old turn's abort and then calls
`dispose()` (`packages/coding-agent/src/core/agent-session-runtime.ts:167–178`).
The disposal invokes public cleanup in
`packages/coding-agent/src/core/agent-session.ts:1448`.
New sessions get new IDs. Resume loads the stored ID
(`agent-session-runtime.ts:196–221`, `session-manager.ts:1086–1097`,
`sdk.ts:427`). The runtime test resumes a prior session in the same
process in `packages/coding-agent/test/suite/agent-session-runtime.test.ts:187–202`.
That same-process reuse can encounter the retained Pi marker. The
loopback verifies the Opi SDK contract.

## Verification

Three frozen probes produce byte-identical observations. The command
without `--snapshot` exits 2. Windows tests pass with default and no
default features for `openai_codex_responses` (87), `codex_native_options`
(21), `resource_cleanup` (16) and `ws_upgrade` (16). The six new Rust
rows match across feature configurations. Clippy for the affected
test target passes with warnings denied. Cargo format, the documentation
checker and `git diff --check` pass.

Production source and lifecycle drivers retain their #89 identities.
The original-lock manifest verifies with zero drift. The existing
lifecycle replay contains 24 cases, 96 checked trace receipts, zero
material blockers and ten deferred differences. The cleanup difference
stays recorded as observable behavior outside strict alignment.
Real-provider behavior and six-gate acceptance remain open.
[Materials](../evidence/phase5-codex-websocket-idle-timeout-materials.json) ·
[Lifecycle replay](../evidence/phase5-codex-websocket-idle-timeout-differential.json)

Next, revisit the optional deferred expiry check against the current
handle producers and caller contracts. At the expiry boundary, the
existing #62 probe records different dispatch and results.
[#62 deferred handle expiry](62-observe-deferred-handle-expiry.md) ·
[#91 expiry decision](91-defer-expiry-to-provider-by-default.md)

Test impact `add` for the frozen probe and Rust public cleanup
regression. Test impact `update` for the shared test route constructor's
visibility. Test impact `retain` for the existing timeout frames,
cleanup controls and lifecycle cases.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
The parent Rust test, iteration brief, contract index, #89 issue and
differential guide are carried-in and touched. The Rust regression,
probe, issue and paired observations are new files.

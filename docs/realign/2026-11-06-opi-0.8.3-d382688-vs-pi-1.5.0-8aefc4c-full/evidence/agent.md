# Batch AGT — agent loop & harness (Dimension 5)

Measured 2026-11-06 against pi 1.5.0 `packages/agent` (source export `8aefc4c29d420c27`) and opi
`crates/opi-agent` at tree `d382688b11cee87f`. 37 contracts recorded: equivalent 1, partial 11,
missing 4, different 14, opi-only 7, unresolved 0. Biggest gaps: no nested `runToolCall` seam, no
tool progress updates, no client identity/attribution, no stream observers, event-shape divergence
(no message lifecycle for user/toolResult, completion-order tool events lost), transactional
rollback instead of pi's append-failure-transcript semantics. Uncertainties: deferred-response
continuation caller contract (both loops treat the terminal identically but caller resumption
differs); `ToolExecutionUpdate` producer existence outside this crate.

---

### AGT-AGENT-01 — stateful Agent wrapper: construction & state model
- Dimension(s): 5 (session wrapper)
- pi entry: `packages/agent/src/agent.ts:194` (class `Agent`), `:238` (constructor, `AgentOptions`), `types.ts:300` (`AgentState`: mutable `model`, `thinkingLevel`, `tools`, `messages`; readonly `isStreaming`, `streamingMessage`, `pendingToolCalls`, `errorMessage`) — public.
- Opi entry: `crates/opi-agent/src/agent.rs:543` (struct `Agent`), `:573` (`Agent::new`: `Models`, `RegisteredTool` vec, optional `ToolAuthorizer`, `model_spec`, `system`, `InferenceConfig`, `AgentLoopConfig`, `AgentHooks`), `loop_types.rs:364` (`NextTurnState`); snapshots `agent.rs:811` (`messages_snapshot`), `:821` (`state_snapshot`) — public.
- Comparison: partial
- Observable difference: pi exposes a live mutable state object (`agent.state.model = ...`, `state.tools = [...]`, `state.streamingMessage`, `state.pendingToolCalls: ReadonlySet`, `state.errorMessage`); opi exposes only full-state read snapshots plus validated `replace_state` (`agent.rs:653`). Opi has no `streamingMessage`, `pendingToolCalls`, or `errorMessage` state at all — a UI must derive those from events. Opi requires trusted tool registrations + authorizer at construction; pi accepts a plain `tools` array that can be swapped later.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Opi adds `AgentControl` (`agent.rs:494`) and `ArmedAgentRun` (`:523`) surfaces pi lacks.

### AGT-AGENT-02 — prompt() workflow & re-entrancy protection
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:380-391` (`prompt` overloads; throws "Agent is already processing a prompt" when `activeRun` set), `:419` (`normalizePromptInput`: string | message | batch + images) — public.
- Opi entry: `crates/opi-agent/src/agent.rs:663` (`prompt`), `:694` (`prompt_with_content`), `:680/:702` (`*_armed` variants), `:897` (`push_user_text`), `:906` (`validate_armed_run` → `AgentError::InvalidArmedRun`) — public.
- Comparison: partial
- Observable difference: Both accept text and text+images (`prompt_with_content`). pi's re-entrancy guard is a runtime throw; opi's is `&mut self` borrowing plus an opaque armed-generation check that rejects a stale `ArmedAgentRun` with `InvalidArmedRun` (`agent.rs:906-918`). pi `prompt` accepts a prebuilt `AgentMessage` batch (arbitrary roles incl. system prompt patches); opi only text or `InputContent` — structured prompt batches (e.g. system-message mid-session patching) are not expressible.
- Evidence basis: source inspection; test inspection (`crates/opi-agent/tests/agent_wrapper.rs`)
- Review outcome: self-reviewed
- Notes: none

### AGT-AGENT-03 — continue() semantics: queued-input fallback vs context continuation
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:393-417` (`continue()`: rejects empty/system-only transcript; non-assistant tail → `runAgentLoopContinue` with startup steering poll; assistant tail → one drained steering batch, else one follow-up batch, else throw "Cannot continue from message role: assistant"), `packages/agent/src/agent-loop.ts:80-108` (`agentLoopContinue` tail-role validation) — public.
- Opi entry: `crates/opi-agent/src/agent.rs:722` (`continue_` requires text, only checks non-empty context, then pushes a NEW user message), `:750` (`retry_last_turn` re-runs with current state, no context-shape check) — public; absence search: `grep -n "assistant" crates/opi-agent/src/agent_loop.rs crates/opi-agent/src/agent.rs` finds no tail-role validation in the loop.
- Comparison: different
- Observable difference: pi `continue()` continues from existing context without adding input and never validates a role the caller did not create; opi `continue_(text)` always appends a user message (closer to pi `prompt`). The pi-equivalent no-input continuation is opi `retry_last_turn` (documented for CredentialNeeded retry, `agent.rs:745-749`). Opi performs no assistant-tail rejection anywhere: an assistant-tail state dispatches a provider request whose last message is assistant (providers may reject downstream), and there is no queued-batch fallback from an assistant tail.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi's steering-batch fallback from an assistant tail (`agent.test.ts:1043`) has no opi counterpart.

### AGT-QUEUE-01 — steering queue semantics & drain modes
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:149` (`PendingMessageQueue`, `peek` honors mode), `:308` (`steer`), `:290` (`steeringMode` setter, default `"one-at-a-time"` `:271`), `types.ts:91-96` (`QueueMode = "all" | "one-at-a-time"`), poll points `agent-loop.ts:184` (run start), `:304` (post-turn), `:210-215` (post-prepare only when earlier poll empty) — public.
- Opi entry: `crates/opi-agent/src/agent.rs:883` (`steer(String)`), drain `agent_loop.rs:317` (first turn), `:1416` (post-turn), `:1525` (post-prepare, only when none selected), `drain_queue` `:2854` (drains ALL) — public steer, internal drain.
- Comparison: partial
- Observable difference: Injection points match (before next provider request after tool calls complete; run-start poll; re-poll after preparation when nothing was selected). Drain cardinality differs: opi steering always drains the whole queue (`drain(..).collect()`), equal to pi mode `"all"`; pi's default delivers one message per poll. Opi has no `QueueMode` selection, no `clearSteeringQueue`/`peekQueuedMessages`/`hasQueuedMessages` (`agent.ts:318/:334/:339` absent in opi). Opi announces delivery with a `QueueUpdate` event (`agent_loop.rs:349`); pi emits no such event. Opi restores undelivered drained input on failure paths (`restore_queue`, `agent_loop.rs:2845`, `fail_before_dispatch!` `:325-341`); pi loses drained-but-unemitted steering if `prepareNextTurn` throws.
- Evidence basis: source inspection; test inspection (`tests/hooks_queues.rs: steering_queue_delivered_before_next_request`, `queue_update_event_emitted`, `phase8_queue_polling_order_*`)
- Review outcome: self-reviewed
- Notes: pi `Agent.continue()`'s `skipInitialSteeringPoll` flag (`agent.ts:476-494`) has no opi analog (moot given AGT-AGENT-03).

### AGT-QUEUE-02 — follow-up queue semantics
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:313` (`followUp`), `:295` (`followUpMode`, default `"one-at-a-time"`), checked only when no tool calls and no steering (`agent-loop.ts:311-318`) — public.
- Opi entry: `crates/opi-agent/src/agent.rs:891` (`follow_up(String)`), `agent_loop.rs:1417-1422` (populated only when steering empty and (`!has_tools_pending || turn_terminate`)), `pop_follow_up` `:2864` (pops exactly one) — public follow_up, internal pop.
- Comparison: partial
- Observable difference: Delivery condition and one-at-a-time cardinality match pi's default exactly. Opi has no mode override, no `clearFollowUpQueue`/`clearAllQueues`, no peek. pi follow-up messages are full `AgentMessage`s; opi queues plain `String` text wrapped as user messages (`user_text_message`, `agent_loop.rs:2876`). Opi re-queues an undelivered popped follow-up on preparation failure (`agent_loop.rs:1505-1512`).
- Evidence basis: source inspection; test inspection (`tests/hooks_queues.rs: follow_up_queue_delivered_when_would_stop`)
- Review outcome: self-reviewed
- Notes: A terminating tool batch still allows follow-up continuation in both (`agent-loop.ts:311`, `agent_loop.rs:1414` comment).

### AGT-QUEUE-03 — completed-turn decision: queued input vs forced continuation
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:186-196` (`FinishTurn` returns `{action:"end"} | {action:"continue"} | undefined`; `continue` guarantees one more provider request even context-only), invoked `agent-loop.ts:294-306`, forced context-only turn `:320-323`; runs on error/aborted turns too (`agent-loop.ts:265-273`).
- Opi entry: `crates/opi-agent/src/hooks.rs:82-90` (`should_stop_after_turn` → bool, observes completed turn before queue polling), invoked `agent_loop.rs:1392-1414`; next turn prepared only when tool work or queued input warrants it (`agent_loop.rs:1427` + doc `:53-56` "Returning a candidate cannot itself create a turn").
- Comparison: different
- Observable difference: pi's `finishTurn {action:"continue"}` forces exactly one extra provider request with unchanged context (tested `agent-loop.test.ts:1273`); opi has no hook that can force an extra request — `should_stop_after_turn == false` merely permits normal scheduling, and a no-tool, no-queue turn ends the run. pi's `{action:"end"}` skips queue polling, matching opi `should_stop == true` (`tests/hooks_queues.rs: should_stop_prevents_queue_polling`). pi runs finishTurn on error/aborted turns; opi's hook never runs on a failed turn (hard exit before the hook, `agent_loop.rs:648/1348`).
- Evidence basis: source inspection; test inspection both sides
- Review outcome: self-reviewed
- Notes: An opi caller needing pi's forced continuation must queue a follow-up message instead — observable as an extra user message in the transcript.

### AGT-TOOL-01 — tool execution modes & event/result ordering
- Dimension(s): 5 (tool continuation)
- pi entry: `packages/agent/src/agent-loop.ts:518-538` (`executeToolCalls`: global `toolExecution` config or any per-tool `executionMode:"sequential"` forces sequential), parallel arm `:596-670` (preflight sequential, execute concurrently, `tool_execution_end` in completion order, toolResult messages in assistant source order), `types.ts:60-66` (`ToolExecutionMode`).
- Opi entry: `crates/opi-agent/src/agent_loop.rs:791-794` (any registered tool with `execution_mode() == Sequential`, unknown tools default sequential `:794-800`), sequential arm `:801-890`, parallel arm `:905-1090` (preflight all calls in source order `:930-1002`, `join_all` `:1044`, completion `complete_tool_call` in source order `:1080-1090`); per-tool mode `tool.rs:126-129`.
- Comparison: different
- Observable difference: (1) pi emits `tool_execution_end` as each parallel tool finalizes (completion order, tested `agent-loop.test.ts:668`); opi emits all `ToolExecutionEnd` events after `join_all` in assistant source order — a subscriber cannot observe interleaved completion. (2) pi exposes a global `toolExecution` config (`types.ts:333`, default parallel); opi has only the per-tool `execution_mode()` — no global toggle. (3) Opi treats an unknown tool name as sequential-batch-forcing (`unwrap_or(true)`), then rejects it in preflight ("unknown tool: {name}", `agent_loop.rs:2371-2380`); pi rejects unknown tools immediately during prepare ("Tool ${name} not found", `agent-loop.ts:730-736`) without affecting batch mode. (4) On abort mid-sequential-batch, pi stops after the in-flight call leaving remaining calls without results (`agent-loop.ts:585-587`); opi synthesizes `skipped_after_terminal_result` error entries for the remaining calls into context (`agent_loop.rs:880-893`).
- Evidence basis: source inspection; test inspection (`tests/agent_loop_semantics.rs`, `agent-loop.test.ts:668/867/949/1035`)
- Review outcome: self-reviewed
- Notes: Result-message source order is preserved in both.

### AGT-TOOL-02 — tool preflight pipeline & before/after hook contracts
- Dimension(s): 5
- pi entry: `packages/agent/src/agent-loop.ts:718-787` (`prepareToolCall`: resolve → `prepareArguments` → schema validate → `beforeToolCall(validatedArgs, assistantMessage, toolCall, context)` → block w/ reason+terminate), `:869-920` (`finalizeExecutedToolCall`: `afterToolCall` field-merge of content/details/structuredContent/usage/terminate/isError; hook throw → error result).
- Opi entry: `crates/opi-agent/src/agent_loop.rs:2355-2555` (`preflight_tool`: registry resolve → `before_tool_call` hook (BEFORE schema validation, `:2381-2385` comment) → final-parse disposition → schema validation on blocking pool → mandatory `authorize_and_verify` → authorization evidence), `:2639-2760` (`execute_prepared_tool` → `after_tool_call`), hooks `hooks.rs:41-73` (`Deny{reason}` only; `Keep | Replace(ToolResult)` full replacement).
- Comparison: different
- Observable difference: Ordering: pi hook sees validated args and the requesting assistant message; opi hook sees raw proposed args + message list, and a denial cannot set `terminate`. Opi adds an authorization stage (fail-closed without an `Allow`, stale-generation reauthorization, batch-invalidating evidence failures `agent_loop.rs:1004-1030`) that pi does not have — pi's `beforeToolCall` is the only pre-execution gate. After-execution: pi merges partial fields (structured content dropped only when content replaced without it, tested `agent-loop.test.ts:2214`); opi replaces the whole `ToolResult` or keeps it, and tool evidence retains the pre-replacement outcome (`hooks.rs:63-68`). pi's `prepareArguments` shim has no opi counterpart (opi rejects unparseable arguments with `InvalidJson` results, `agent_loop.rs:2386-2400`).
- Evidence basis: source inspection; test inspection (`tests/agent_loop_semantics.rs: phase8_hook_runs_before_schema_validation`, `phase8_hook_contract_after_replace_before_events`)
- Review outcome: self-reviewed
- Notes: Hook context shapes differ: opi passes `tool_call_id/tool_name/args/messages`; no assistant-message reference.

### AGT-TOOL-03 — length-truncated assistant message fails all tool calls
- Dimension(s): 5
- pi entry: `packages/agent/src/agent-loop.ts:275-280` + `:488-516` (`failToolCallsFromTruncatedMessage`: every call gets `tool_execution_start`, error result "Tool call \"N\" was not executed: the response hit the output token limit, so its arguments may be truncated. Re-issue the tool call with complete arguments.", `terminate: false`).
- Opi entry: `crates/opi-agent/src/agent_loop.rs:745-783` (same `StopReason::Length` guard; identical message text; `ToolNotExecutedReason::OutputLimit`; `ToolExecutionStart` + `complete_tool_call` per call; batch continues) — public behavior via events.
- Comparison: equivalent
- Observable difference: none observable to the model or event subscriber: same guard condition (terminal `length` stop reason), same model-facing error text, same non-terminating batch outcome. Opi additionally tags the durable tool result with `not_executed: OutputLimit` (`ToolNotExecutedReason`), which pi's `ToolResultMessage` cannot represent.
- Evidence basis: source inspection; test inspection (`tests/truncated_propagation.rs`, `agent-loop.test.ts:456`)
- Review outcome: self-reviewed
- Notes: none

### AGT-TOOL-04 — terminate-all batch early-stop rule
- Dimension(s): 5
- pi entry: `packages/agent/src/agent-loop.ts:700-702` (`shouldTerminateToolBatch`: every finalized result `terminate === true`), sources: tool result, blocked `beforeToolCall`, `afterToolCall` override; stops tool continuation only (`:280-282` `hasMoreToolCalls = !terminate`), steering/follow-up still polled.
- Opi entry: `crates/opi-agent/src/agent_loop.rs:1086-1090` (`all_terminate = !terminate_flags.is_empty() && all`), flags from `ToolResult.terminate` (incl. after-hook replacements, `complete_tool_call` `:2282-2316`); `turn_terminate` suppresses automatic continuation but queue input still warrants a turn (`:1413-1422`).
- Comparison: partial
- Observable difference: Batch rule and queue override match (mixed batches continue, both sides). pi allows a blocked `beforeToolCall` to contribute `terminate: true` (`types.ts:113`, tested `agent-loop.test.ts:1783`); opi's before-hook denial cannot (`hooks.rs:66` has no terminate field) — only tool results and after-hook replacements can mark termination on the opi side.
- Evidence basis: source inspection; test inspection (`tests/agent_loop_semantics.rs: phase8_hook_contract_terminal_stop_terminates_run`, `agent-loop.test.ts:1732/1841/1964`)
- Review outcome: self-reviewed
- Notes: none

### AGT-TOOL-05 — nested tool calls (`runToolCall` seam)
- Dimension(s): 5 (tool continuation)
- pi entry: `packages/agent/src/agent-loop.ts:801-829` (`runToolCall`: runs one call through the full pipeline — prepare args, validation, hooks, execution — "Tools that call other tools use this so the hooks apply to those calls too"; exported; used by the mcp-codemode example) — public.
- Opi entry: absent. Search: `grep -rn "run_tool_call|runToolCall|nested tool" crates/opi-agent/src/` → no match (only evidence `ParentCallId` correlation comment `evidence.rs:64`); `Tool::execute`/`execute_authorized` (`tool.rs:46-68`) have no caller-facing re-entry helper.
- Comparison: missing
- Observable difference: A pi tool implementation can invoke another agent tool with hooks/permissions applied by calling `runToolCall`; an opi tool cannot — there is no exported path that routes a nested call through preflight (hook + validation + authorization), so nested calls either bypass the authority pipeline or cannot be made.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Linked to H-layer: pi README "MCP and Codemode" section documents this as the integration pattern.

### AGT-TOOL-06 — deferred responses & continuation
- Dimension(s): 5 (tool continuation)
- pi entry: `packages/agent/src/types.ts` (no deferred handling); deferred is a pi-ai concept (`packages/ai/src/types.ts:489` `StopReason = ... | "deferred"`, `:539` `DeferredHandle`); the agent loop treats a `deferred` terminal as a normal done event (`agent-loop.ts:390-485` has no deferred special-case) and the run ends; resumption is caller-side via pi-ai fetch + `Agent.continue()`.
- Opi entry: `crates/opi-ai/src/deferred.rs:1-10` (DeferredHandle, fetch/cancel explicit), `stream.rs:26-32` (`StopReason::Deferred` is an Ok terminal "exactly like Stop/Length/ToolUse"); loop handling `agent_loop.rs:660-730` treats `Done` uniformly; harness-level fetch absence search: `grep -rn "Deferred" crates/opi-coding-agent/src/` → no loop-level consumer.
- Comparison: partial
- Observable difference: Both loops terminate the run on a deferred terminal and leave the handle-bearing assistant message in context (opi preserves `deferred` through event redaction, `event.rs:354-357`). Resumption differs: pi callers fetch via pi-ai `Models` deferred methods then call `agent.continue()` (validated non-assistant-tail continuation); opi has no `continue()`-without-input public operation with tail validation — `retry_last_turn` re-dispatches from any tail, and no coding-agent consumer wires `fetch_deferred` into a turn.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: "result submission" (brief item 2) on both sides is the `execute` return value; there is no separate submit API on either side.

### AGT-TOOL-07 — tool result content model
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:341-362` (`AgentToolResult`: `content`, `details`, `structuredContent?`, `usage?`, `isError?`, `terminate?`; `outputSchema` on tool `:354`), `agent-loop.ts:940-954` (`createToolResultMessage` normalizes missing content).
- Opi entry: `crates/opi-agent/src/tool.rs:88-101` (`ToolResult`: `content`, `details: Option`, `is_error`, `terminate`, `truncated`, `diagnostics: Vec<ToolDiagnostic>`); builders `tool/result.rs:20-40`.
- Comparison: different
- Observable difference: pi carries machine-readable `structuredContent` (schema-declared, not sent to the model) and per-tool `usage`; opi has neither. Opi adds `truncated` (propagated onto `ToolResultMessage.truncated` and `ToolExecutionEnd.truncated`) and tool-owned `diagnostics` lifted to the diagnostic sink and public event (`agent_loop.rs:2790-2838`), which pi cannot express. Opi `ToolResultMessage` additionally carries `not_executed: Option<ToolNotExecutedReason>`.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-ABORT-01 — cancellation: tokens, in-flight behavior
- Dimension(s): 5 (termination/abort)
- pi entry: `packages/agent/src/agent.ts:350-352` (`abort()` → AbortController), signal threaded to streamFn/tool execute/hooks; agent idles after `finishRun` (`:559`); a later `prompt()` reuses the agent (new activeRun).
- Opi entry: `crates/opi-agent/src/agent.rs:769` (`abort()` → CancellationToken cancel), `:857` (`arm_run` → `maybe_reset_cancel` resets a cancelled token for the next run, `:928-932`); loop observes cancellation via `tokio::select! { biased; cancelled() }` at every await (stream `agent_loop.rs:637-643`, preflight `:2355`, retry sleep `:1186-1198`, hooks `:379-386`, `:1392-1404`); schema compile on blocking pool discards result without claiming hard-kill (`:470-476`).
- Comparison: partial
- Observable difference: Propagation coverage is comparable (request stream, tools, hooks, waits). pi abort surfaces as an assistant message with `stopReason:"aborted"` in the transcript (see AGT-ABORT-02); opi abort surfaces as `AgentError::Cancelled` in `AgentRunResult` with a `CODE_AGENT_CANCELLED` Info diagnostic (phase-tagged, `agent_loop.rs:2900-2910`). Opi token reset is generation-guarded (`arm_run`), pi relies on run-scoped AbortControllers.
- Evidence basis: source inspection; test inspection (`tests/agent_wrapper.rs`, `e2e.test.ts:208` abort during streaming)
- Review outcome: self-reviewed
- Notes: none

### AGT-ABORT-02 — failure/abort transcript semantics: append vs transactional rollback
- Dimension(s): 5 (termination/abort)
- pi entry: `packages/agent/src/agent.ts:531-555` (`handleRunFailure`: synthesizes an assistant failure message and emits `message_start/message_end/turn_end/agent_end`; state updated via `processEvents` so the failure message and any partial/tool messages stay in `state.messages`), `agent-loop.ts:261-273` (error/aborted stop reasons are hard exits with full event sequence).
- Opi entry: `crates/opi-agent/src/agent.rs:978-1002` (`run_operation` settlement: on error the durable state rolls back to the most recent `committed_state` baseline — or the original state — EXCEPT `PartialSideEffect`/`CleanupUnknown` terminal outcomes whose terminal tool results are retained); failed-provider-call partial assistant message kept visible only when it carries content/error (`agent_loop.rs:1246-1336`); `preflight_failure` (`agent.rs:1008-1020`) emits NO events at all.
- Comparison: different
- Observable difference: After an aborted or failed run, pi's transcript contains the failure assistant message (`stopReason` aborted/error, `errorMessage`) plus everything streamed before it; opi's durable `Agent.state.context` discards the failed turn back to the last externally committed baseline (the run's `AgentRunResult.messages` still exposes the loop-boundary messages for inspection). Opi failure paths emit `AgentEnd` but no synthetic assistant message; an invalid-armed-run preflight failure emits no lifecycle events whatsoever.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Opi's design is transactional (preparation commit baselines, `commit_preparation` `agent_loop.rs:1543-1600`); pi's is append-only.

### AGT-ABORT-03 — idle/settlement & drop semantics
- Dimension(s): 5 (termination/abort)
- pi entry: `packages/agent/src/agent.ts:275-288` (`subscribe`: async listeners awaited in subscription order, included in run settlement), `:359-362` (`waitForIdle` resolves after `agent_end` listeners settle), `:505-516` (doc contract).
- Opi entry: `crates/opi-agent/src/agent.rs:53-77` (`PublicEventFanout`: synchronous `Box<dyn Fn(&AgentEvent)>` callbacks, lock released before invocation), run settlement = the `prompt`/`continue_` future itself; `impl Drop for AgentRunResult` `:484-489` (synchronous fail-closed evidence abandonment).
- Comparison: different
- Observable difference: pi subscribers can be async and delay run settlement (prompt resolves only after listeners finish); opi subscribers are synchronous and cannot extend a run. Opi adds `#[must_use]` run results whose drop performs evidence abandonment — pi has no result object and no cleanup-on-drop behavior.
- Evidence basis: source inspection; test inspection (`agent.rs:56-75` fanout test)
- Review outcome: self-reviewed
- Notes: none

### AGT-STATE-01 — transactional next-turn replacement
- Dimension(s): 5 (transactional next-turn)
- pi entry: `packages/agent/src/types.ts:170-184` (`AgentLoopTurnUpdate {context?, messages?, model?, thinkingLevel?}` — partial update, messages appended with lifecycle events), applied `agent-loop.ts:186-212`.
- Opi entry: `crates/opi-agent/src/hooks.rs:128-146` (`prepare_next_turn_transaction` returns complete `NextTurnState` or None), `agent_loop.rs:1427-1523` (warranted-only invocation, `commit_preparation` validation `:1543`: intrinsic candidate validation, selected-queue-input suffix preservation `:1562-1590`, `validate_prepared_state`, `commit_prepared_state` external-persistence settlement, rollback baseline capture `:1440-1446`).
- Comparison: partial
- Observable difference: Opi replacement is whole-state, validated (route canonical, model constraints, thinking support, finite temperature — `loop_types.rs:388-437`), and rolls back to the committed baseline on failure with undelivered queue input restored (`agent_loop.rs:1505-1512`). pi applies partial patches, appends `messages` with `message_start/end` events (opi appends selected input silently except `QueueUpdate`), and a thrown `prepareNextTurn` loses drained queue messages. Both re-poll steering after preparation when nothing was selected (`agent-loop.ts:210-215`, `agent_loop.rs:1523-1526`).
- Evidence basis: source inspection; test inspection (`tests/preparation_transaction.rs`, `agent-loop.test.ts:1503/1580`)
- Review outcome: self-reviewed
- Notes: Opi also gates dispatch on evidence health after preparation (`agent_loop.rs:397-404`).

### AGT-STATE-02 — per-request preparation hook (prepareRequest)
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:198-226` (`PrepareRequest` runs immediately before EVERY conversational provider request incl. the first; can replace context/model/thinking; does not poll queues), invoked `agent-loop.ts:224-240`.
- Opi entry: `crates/opi-agent/src/hooks.rs:147-160` (`prepare_initial_turn` — first turn only, receives selected input, must preserve it as an unchanged suffix), invoked `agent_loop.rs:361-404`; later-turn replacement is via `prepare_next_turn_transaction` (AGT-STATE-01) and inference is state-owned (`Request` built from `state.model_selection`/`state.inference`, `agent_loop.rs:479-495`).
- Comparison: partial
- Observable difference: pi can rebuild context before every request (canonical session reload pattern, README "Request preparation"); opi's equivalent runs only before the first dispatch of a run — subsequent requests reuse the committed state unless the warranted next-turn preparation replaces it, and a no-tool/no-queue turn never offers a preparation point.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-STATE-03 — mid-session context/model/tool updates
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:296-339` (mutable `model`, `thinkingLevel`, `tools`, `messages`), `agent-loop.ts:342-386` (`declareToolChanges`: transcript system messages announce tool-loadout diffs before the next request; pending system message merge), README "Agent State".
- Opi entry: `crates/opi-agent/src/agent.rs:653-661` (`replace_state`: validated idle replacement of context+model+inference), `agent.rs:543-560` (tools immutable per Agent — registry built in `new`, `authority.rs:154-190`; no setter), `system: Option<String>` fixed at construction (`loop_types.rs:357` run binding, no setter); tool defs projected from the registry into every request (`agent_loop.rs:441-450`).
- Comparison: partial
- Observable difference: Model/inference/context changes are expressible on both (opi requires whole validated state; pi mutates fields). Tool-set changes mid-session: pi swaps `state.tools` and the model sees a `toolsAdded`/`toolsRemoved` system message; opi's registry is frozen for the Agent's lifetime — changing tools requires a new Agent. System prompt: pi patches via transcript system messages; opi's is a fixed run binding (prompt text itself lives in the message list). Opi request always carries the current registry projection, so provider-visible tool defs never go stale, but there is no declaration-change announcement mechanism.
- Evidence basis: source inspection; test inspection (`agent.test.ts:164/212/247` tool-change declaration)
- Review outcome: self-reviewed
- Notes: pi `getCurrentSystemMessage`/sections-based prompt patching has no opi counterpart in this crate.

### AGT-STATE-04 — reset()
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:364-378` (`reset()`: throws while processing; keeps replayed system baseline; clears streaming flags and BOTH queues; tested `agent.test.ts:282/676`).
- Opi entry: `crates/opi-agent/src/agent.rs:653` (`replace_state`) — no queue clearing, no busy check (borrow prevents concurrent use); queues have no clear API (AGT-QUEUE-01/02).
- Comparison: different
- Observable difference: pi offers one-call reset-to-prompt-baseline with queue purge; opi callers must snapshot `state_snapshot()`, trim to their baseline, `replace_state`, and have no way to drop queued steering/follow-up text.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-EVENT-01 — event vocabulary & emission sequences
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:430-466` (`AgentEvent`: agent_start/end, turn_start/end, message_start/update/end for system+user+assistant+toolResult, tool_execution_start/update/end), sequences documented in README; user prompt emits `message_start/end` (`agent-loop.ts:126-129`), tool results emit `message_start/end` (`:956-960`).
- Opi entry: `crates/opi-agent/src/event.rs:27-114` (`AgentEvent`: same core set PLUS `QueueUpdate`, `AutoRetryStart/End`, `CompactionStart/End` (harness-emitted), `SessionPersistError`, richer `ToolExecutionEnd{result, details, truncated, is_error, diagnostics}`); loop emits `MessageStart/Update` only for assistant stream events (`agent_loop.rs:1669-1700`), `MessageEnd` only for assistant terminals (`:669/:1288`), NO message events for user prompts or tool-result messages, `MessageUpdate` not emitted for tool-call deltas (`ToolCallDelta` → Pending, `:1690`).
- Comparison: different
- Observable difference: A pi subscriber sees the user prompt and every tool-result message as `message_start`/`message_end` pairs and receives raw `assistantMessageEvent` (incl. toolcall deltas and text/thinking boundaries) inside `message_update`; an opi subscriber sees only assistant `MessageStart/Update/End`, gets tool-call argument deltas redacted to `"[REDACTED]"` in stream events (`event.rs:420-427`), and must learn about queue delivery/retries from opi-only events. pi `turn_end.toolResults` matches opi `TurnEnd.tool_results`.
- Evidence basis: source inspection; test inspection (`tests/event_public_boundary.rs`, `e2e.test.ts:221`)
- Review outcome: self-reviewed
- Notes: Opi events are serde-serializable (`#[derive(Serialize, Deserialize)]`) for JSON/NDJSON modes; pi events are in-memory objects.

### AGT-EVENT-02 — public event redaction boundary
- Dimension(s): 5
- pi entry: none (`packages/agent/src/` has no redaction; events carry raw content).
- Opi entry: `crates/opi-agent/src/event.rs:25-30` (`redacting_event_sink`), `:116-253` (`redacted_for_public`: tool args/details redacted, signatures stripped, diagnostics redacted, conversation text scrubbed only for credential patterns via `CONVERSATION_SECRET_REDACTOR` `:255-262`).
- Comparison: opi-only
- Observable difference: Opi public subscribers can never observe credentials in tool arguments/results or unreduced sensitive details; pi subscribers receive everything the model/tools produced.
- Evidence basis: source inspection; test inspection (`tests/event_public_boundary.rs`, `tests/tool_event_redaction.rs`)
- Review outcome: self-reviewed
- Notes: Implemented experimental-adjacent security boundary (no pi counterpart).

### AGT-EVENT-03 — tool progress updates (`tool_execution_update`)
- Dimension(s): 5
- pi entry: `packages/agent/src/agent-loop.ts:789-799` (`emitToolExecutionUpdate` → `tool_execution_update` events from `onUpdate` callbacks, scoped to the live `execute()` invocation), `types.ts:448`, tested `agent.test.ts:473/538`.
- Opi entry: `event.rs:63-70` defines `ToolExecutionUpdate`, but the loop always passes `None` as the update callback (`agent_loop.rs:2653` `execute_authorized(..., None)`); absence search: `grep -n "ToolExecutionUpdate" crates/opi-agent/src/agent_loop.rs` → no emission site.
- Comparison: missing
- Observable difference: pi tools stream partial results to subscribers mid-execution; opi tools accept an `on_update` parameter in the `Tool` trait signature (`tool.rs:46-52`) but the loop never wires it, so no progress event is ever emitted in this crate.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: The protocol variant exists (dead at this seam); possibly produced by other crates — not verified (see Coverage honesty).

### AGT-EVENT-04 — subscription model
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:275-288` (async listeners awaited in order, receive `(event, signal)`, unsubscribe function returned).
- Opi entry: `crates/opi-agent/src/agent.rs:832-836` (`subscribe(Box<dyn Fn(&AgentEvent)>)`, no unsubscribe handle, no signal, sync), `:805` (`emit_event` for out-of-loop events e.g. compaction).
- Comparison: different
- Observable difference: pi listeners can await, get the run's abort signal, and detach; opi callbacks are fire-and-forget sync closures with no cancellation signal and no unsubscribe.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-PROXY-01 — proxy.ts vs streaming_proxy.rs
- Dimension(s): 5
- pi entry: `packages/agent/src/proxy.ts:117-253` (`streamProxy`: an LLM stream FUNCTION that POSTs `{model, context, options}` to `${proxyUrl}/api/stream` with bearer auth, parses SSE `data:` lines, reconstructs the partial assistant message client-side, handles abort/EOF-without-terminal as error), `:255-392` (`processProxyEvent`).
- Opi entry: `crates/opi-agent/src/streaming_proxy.rs:112-135` (`StreamingProxy::run`: a JSONL command/event proxy over any `BufRead`/`Write` — reads `SdkCommand` lines, dispatches to `ProxyHandler`, writes `SdkResponse`/agent-event JSONL, bounded event channel with drop-on-full backpressure, cancellation → `proxy_cancelled`), plus `SecretRedactor` `:283-430`.
- Comparison: different
- Observable difference: The two modules share only a name. pi's proxy routes provider traffic through an auth-holding HTTP server (usable as `streamFn`); opi has no provider-proxy stream function — its Agent dispatches only through the registered `Models` collection. opi's proxy is an SDK transport shim (stdin/stdout-style JSONL) with secret redaction that pi-agent-core does not provide; the nearest pi analog lives outside `packages/agent`.
- Evidence basis: source inspection; test inspection (`proxy.test.ts`, `tests/streaming_proxy.rs`)
- Review outcome: self-reviewed
- Notes: Module-layout differences alone are not behavioral; here the behaviors themselves are disjoint.

### AGT-STREAMFN-01 — default stream function registry
- Dimension(s): 5
- pi entry: `packages/agent/src/stream-fn.ts:10-22` (`setDefaultStreamFn`/`getDefaultStreamFn`: fallback used when `streamFn` omitted; throws when unset), Agent falls back at `agent.ts:246`/`agent-loop.ts:117`.
- Opi entry: no registry; `Agent::new` requires `Arc<Models>` (`agent.rs:573-576`) and every logical call goes through `collection.prepare_call_with_policy` (`agent_loop.rs:497-520`).
- Comparison: different
- Observable difference: pi lets a host install a process-wide default so callers may omit `streamFn` (legacy compatibility, tested `agent-loop.test.ts:80`); opi has no default mechanism — the dispatch collection is a mandatory constructor dependency, making the "omit the stream function" state unrepresentable.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: The seam's purpose (host-provided dispatch) is satisfied structurally; verdict reflects mechanism divergence under the strict reproduction lens.

### AGT-IDENT-01 — client identity / provider attribution
- Dimension(s): 5
- pi entry: `packages/agent/src/agent-loop.ts:34-37` (`DEFAULT_AGENT_IDENTITY = { attribution: PI_ATTRIBUTION }`), `:440` (merged field-by-field over configured `identity`), `AgentOptions.identity` `agent.ts:118-123`, tested `agent.test.ts:1203`.
- Opi entry: absent at this seam — `Agent`/`AgentLoopContext`/`Request` construction (`agent_loop.rs:479-495`) sets no identity/attribution; `Request` exposes `extra_headers` (`opi-ai/src/provider.rs:290`) but the Agent never populates attribution; `grep -rn -i "attribution" crates/opi-agent/src/` → no match.
- Comparison: missing
- Observable difference: pi agent requests carry attribution headers naming the app (overridable per Agent, nullable to send none); opi agent-originated requests carry no Agent-level identity — any user-agent/attribution behavior is whatever opi-ai providers set internally, which is not caller-configurable through the Agent.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi-ai internal header handling belongs to the AI batch; only the Agent-seam absence is claimed here.

### AGT-OBSVR-01 — provider stream/payload observers
- Dimension(s): 5
- pi entry: `packages/agent/src/agent.ts:96-99` (`onPayload`, `onResponse`, `onProviderStreamEvent` forwarded into every stream call, tested `agent.test.ts:1155`).
- Opi entry: absent — `AgentLoopContext` (`loop_types.rs:340-374`) and `Request` construction set `callbacks: Default::default()` (`agent_loop.rs:493`); opi-ai has a `RequestCallbacks` type but the Agent exposes no observer registration.
- Comparison: missing
- Observable difference: pi callers can observe raw provider payloads, final responses, and provider-native stream events per request; opi callers see only the redacted public `AgentEvent` stream.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-APIKEY-01 — dynamic API key resolution
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:141-149` (`getApiKey(provider)` resolved per LLM call for expiring tokens, contract "must not throw"), used `agent-loop.ts:407-410`.
- Opi entry: no per-call hook; auth resolution is inside `Models::prepare_call*` (`agent_loop.rs:497-520`, collection-owned auth with typed failures `CredentialNeeded`/`CredentialRevoked`, `loop_types.rs:212-231`); retry after login via `retry_last_turn` (`agent.rs:745`).
- Comparison: different
- Observable difference: pi refreshes keys transparently between attempts via the hook; opi surfaces typed credential errors to the caller and expects an interactive login + `retry_last_turn` resumption — expiring-token transparent refresh is not expressible at the Agent seam.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: none

### AGT-RETRY-01 — provider retry behavior & events
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts` — `maxRetryDelayMs` caps provider-requested retry delays (`AgentOptions`, `agent.ts:110`); retries happen inside pi-ai streaming; the agent emits no retry events.
- Opi entry: `crates/opi-agent/src/agent_loop.rs:455-460` (acquisition policy from `AgentLoopConfig.retry.max_attempts`, `loop_types.rs:377-380`), retry loop `:1125-1210` (SDK-admitted attempts re-open the SAME prepared call; cancellable delay; `AutoRetryStart`/`AutoRetryEnd` events incl. success after retry `:1366-1380`; retry evidence records `:1199-1210`), single strict-downgrade variant rebuild `:1211-1230`, post-handoff suppression classification `:1171-1195`.
- Comparison: different
- Observable difference: Opi retries are loop-scheduled, event-visible (`AutoRetryStart{attempt,max_attempts,delay_ms,error_message}`), and sealed by an SDK acquisition owner; pi retries are invisible to the agent (inside the stream function) with only a delay cap configurable. Opi adds a strict-tool-downgrade variant path pi does not have.
- Evidence basis: source inspection; test inspection (`tests/retry_agent.rs`)
- Review outcome: self-reviewed
- Notes: none

### AGT-THINK-01 — thinking/reasoning configuration
- Dimension(s): 5
- pi entry: `packages/agent/src/types.ts:375` (`ThinkingLevel = off|minimal|low|medium|high|xhigh|max`), `AgentOptions.thinkingBudgets` (per-level token budgets, `agent.ts:114`), recorded back on the assistant message (`agent-loop.ts:434`).
- Opi entry: `crates/opi-ai/src/provider.rs:420-424` (`ThinkingConfig {enabled, budget_tokens, level}`), carried on `InferenceConfig.thinking` in `NextTurnState` (`loop_types.rs:348-356`), validated against model capability + level map (`loop_types.rs:415-429`); levels `model_info.rs:169-178` (None..Max, serde `xhigh`).
- Comparison: partial
- Observable difference: Level vocabularies match (pi "off" ≡ opi `None`). pi supports per-level budget tables; opi supports a single `budget_tokens` value — per-level budgets are not expressible. Opi rejects an unsupported level at state validation with a typed error; pi leaves level handling to pi-ai streaming.
- Evidence basis: source inspection; test inspection (`tests/thinking_integration.rs`, `tests/thinking_session.rs`)
- Review outcome: self-reviewed
- Notes: none

### AGT-ONLY-AUTHORITY — trusted tool authorization substrate
- Dimension(s): 5 (opi-only modules)
- pi entry: none (`packages/agent/src/` has no authorization concept; `beforeToolCall` is the only gate).
- Opi entry: `crates/opi-agent/src/authority.rs:32-140` (immutable `RegisteredTool` w/ `RegistrationId`, `ToolOrigin`, `CapabilityIdentity`, generation-owned schema IR), `:143-190` (`ToolRegistry`: duplicate-name rejection, insertion-ordered projection), `:236-320` (`ToolAuthorizationRequest`/`AuthorizationDecision`/`ToolAuthorizer` — Allow carries policy/permission/scope refs + evidence-health generation; Deny carries stable_code + redacted_reason; cancellation is fail-closed).
- Comparison: opi-only
- Observable difference: Every model-proposed tool call is resolved against the trusted registry, schema-validated, and must receive a fresh `Allow` matching registration/capability/evidence-generation before execution; missing/failing/stale authorizers yield zero executions with controlled denial results. pi executes any registered tool whose `beforeToolCall` hook did not block.
- Evidence basis: source inspection; test inspection (`tests/tool_authority.rs`)
- Review outcome: self-reviewed
- Notes: Implemented (not experimental); policy binding lives in opi-coding-agent.

### AGT-ONLY-EVID — evidence lifecycle & run-result ownership
- Dimension(s): 5 (opi-only modules)
- pi entry: none.
- Opi entry: `crates/opi-agent/src/evidence.rs:1-36` (opaque run/turn/call identities, versioned `EvidenceHealth`, redacted evidence values, `EvidenceSink` lifecycle, `FinalizedManifest`), loop emission `agent_loop.rs:209-303` (identity minting before effects; provider/tool/retry/compaction/diagnostic records; health advancement on emission failure), `agent.rs:92-200` (`AgentRunResult`: `terminal_outcome`, `evidence_health`, compaction begin/finish tokens, `finalize_evidence`/`abandon_evidence`, fail-closed `Drop`).
- Comparison: opi-only
- Observable difference: Opi runs can be finalized into immutable manifests or explicitly abandoned; authorization can require complete evidence; failed/uncertain side effects are classified (`PartialSideEffect`/`CleanupUnknown`) and retained in durable state. pi has no equivalent observability or side-effect-truth contract.
- Evidence basis: source inspection; test inspection (`tests/evidence_contract.rs`, `tests/evidence_runtime.rs`)
- Review outcome: self-reviewed
- Notes: Implemented.

### AGT-ONLY-VAL — schema validation dialect wrapper
- Dimension(s): 5 (opi-only modules)
- pi entry: pi validates via pi-ai `validateToolArguments` inline (`agent-loop.ts:30`, `:733`); no dedicated module.
- Opi entry: `crates/opi-agent/src/validation.rs:1-75` (`ToolSchemaIr`: compile-once admission of the trusted schema into the opi-ai dialect; normalization (optional-null deletion, limited ECMAScript coercion) then validation against the ORIGINAL schema; fail-closed on schema-admission failure; redacted failure strings).
- Comparison: opi-only
- Observable difference: Opi normalizes arguments and executes exactly the validated value; invalid schemas fail every call closed; validation runs on the blocking pool under generation-owned IR. pi's validation outcome feeds hooks directly with no normalized-value guarantee stated at this layer.
- Evidence basis: source inspection; test inspection (`tests/tool_validation.rs`)
- Review outcome: self-reviewed
- Notes: none

### AGT-ONLY-DIAG — diagnostics & sinks
- Dimension(s): 5 (opi-only modules)
- pi entry: none (pi-agent emits events only; diagnostics ride assistant messages as `AssistantMessageDiagnostic` from pi-ai).
- Opi entry: `crates/opi-agent/src/diagnostic.rs:1-16` (`Diagnostic` record: severity, snake_case code, source, message, details, action; redaction core reusing `SecretRedactor`), `diagnostic_sink.rs:23-33` (`DiagnosticSink` trait), `:36-95` (`RecordingSink` w/ clear, `NullSink`); loop observation sites for provider/tool/cancel/retry/session failures throughout `agent_loop.rs`.
- Comparison: opi-only
- Observable difference: Opi runtime failure paths record classified, redacted diagnostics to an injectable sink independent of the event stream; pi exposes failures only through events/assistant messages.
- Evidence basis: source inspection; test inspection (`tests/diagnostics.rs`, `tests/diagnostics_runtime.rs`)
- Review outcome: self-reviewed
- Notes: none

### AGT-ONLY-SDK — SDK command/response protocol
- Dimension(s): 5 (opi-only modules)
- pi entry: none in `packages/agent` (pi's RPC/SDK surface, if any, lives in other packages).
- Opi entry: `crates/opi-agent/src/sdk.rs:29-33` (`SDK_SCHEMA_VERSION = 3`), `:40-155` (`SdkCommand`: prompt/continue/steer/follow_up/abort/set_model/set_thinking_level/compact/session_info/extension_command/trace/quit with correlation ids), `:162-230` (`SdkResponse` envelope incl. `error_code`), `:234-246` (`agent_event_to_value`).
- Comparison: opi-only
- Observable difference: External embedders drive an opi agent through a versioned JSONL command protocol with structured responses; the pi-agent package exposes only the in-process class/loop API.
- Evidence basis: source inspection; test inspection (`tests/sdk_embedding.rs`)
- Review outcome: self-reviewed
- Notes: Header marks this "Unstable 0.x API" (see Layer H).

### AGT-ONLY-SESSION — durable session stack (session, session_branch, session_context, session_event, harness facade, compaction engine)
- Dimension(s): 5 (opi-only modules)
- pi entry: none in `packages/agent` (pi session persistence lives in pi-coding-agent, outside this batch's pi scope).
- Opi entry: `session.rs:1-40` (v2 fail-closed JSONL envelope: required vs ignorable-observation entries, immutable `RuntimeInputBinding` header, v1 read-only legacy, explicit non-compatibility with pi session v3 via `SESSION_FORMAT_POLICY:38-40`), crash recovery `:476-660`; `session_branch.rs:27-167` (`SessionTree` branch reconstruction, leaf-tip active branch); `session_context.rs:52-236` (deterministic context reconstruction incl. compaction drop+summary injection, branch-summary injection, metadata latest-wins, label Add/Remove, missing-parent diagnostics); `session_event.rs:29-95` (`AgentSessionEvent` session-level protocol incl. `SessionSummary`, credential events); `harness.rs:135-330` (`SessionFacade`: ordered pending-write queue with kind-priority flush, save points, abort-without-discard; `SessionRepo` trait); `compaction.rs:66-165` (`CompactionEngine`: manual/threshold/overflow triggers, 25%-tail keep split, hook-or-core heuristic (chars/4) summaries).
- Comparison: opi-only
- Observable difference: opi-agent ships durable session persistence, branch forking/reconstruction, session events, a write-queue facade, and a heuristic compaction engine inside the agent crate; the pi agent package keeps all persistence/compaction out of `packages/agent`.
- Evidence basis: source inspection; test inspection (`tests/session_storage.rs`, `session_branching.rs`, `session_context.rs`, `session_facade.rs`, `session_contract.rs`, `compaction.rs`)
- Review outcome: self-reviewed
- Notes: `SESSION_FORMAT_POLICY` explicitly disclaims pi-session-v3 compatibility. Opi compaction summary is a local heuristic; pi's compaction (pi-coding-agent) is out of scope here.

---

## Layer H — active design material encountered in scope

| Claim | Anchor | Authority wording | Observed maturity |
|---|---|---|---|
| pi-agent-core is a "Stateful agent with tool execution and event streaming" | `packages/agent/README.md:3` | README (normative-ish package doc) | implemented |
| pi low-level streams are observational; Agent class is the barrier before tool preflight | `packages/agent/README.md` ("Low-Level API") | README | implemented |
| pi `finishTurn` continue/end decision contract | `packages/agent/src/types.ts:186-196` | doc-contract on public type | implemented + tested |
| opi streaming proxy is an unstable 0.x API; consumers must pin exact versions | `crates/opi-agent/src/streaming_proxy.rs:3-5` | "Unstable 0.x API" (experimental wording) | implemented |
| opi SDK embedding surface is unstable 0.x; schema version stays 3 until stable | `crates/opi-agent/src/sdk.rs:3-5,29-33` | "Unstable 0.x API" | implemented |
| opi extension system is an unstable 0.x extension API | `crates/opi-agent/src/extension.rs:60-66` | "Unstable 0.x extension API" | implemented |
| opi session format policy: v2 envelopes, v1 read-only, NOT pi-v3 compatible | `crates/opi-agent/src/session.rs:38-40` (`SESSION_FORMAT_POLICY`, test-asserted) | normative module contract | implemented |
| opi harness D1–D4 deferred Phase-10 decisions (additive v1, branch summaries as metadata, extension custom messages via `include_in_llm_context`, no in-flight replay on resume) | `crates/opi-agent/src/harness.rs:225-262` | draft/decision record embedded in docs | partially superseded by Phase 13 v2 (implemented); D3 custom-message durable variant still "deferred" |
| opi evidence: redaction boundary and fail-visible finalization are intrinsic contracts | `crates/opi-agent/src/evidence.rs:10-36` | normative module contract | implemented |
| opi Agent Core imposes no intrinsic turn limit; `MaxTurnsExceeded` is caller-owned | `crates/opi-agent/src/agent_loop.rs:51-53`, `loop_types.rs:250-253` | normative doc comment | implemented |

---

## Caller flow trace (both sides)

pi: caller → `new Agent(options)` (`agent.ts:238`, initialState seeds leading system message `:78-95`) → `prompt()` (`:380`) → `normalizePromptInput` (`:419`) → `runWithLifecycle` (`:517`: AbortController, isStreaming=true) → `runAgentLoop` (`agent-loop.ts:111`: `declareToolChanges` → agent_start/turn_start + prompt message events) → `runLoop` (`:172`) per turn: prepareNextTurn (if turn ≥ 2) → append prepared/steering messages w/ events → `prepareRequest` → `streamAssistantResponse` (`:390`: transformContext → convertToLlm → normalizeContext → getApiKey → streamFn; partials pushed as message events) → tool calls → `executeToolCalls` (`:518`: beforeToolCall → execute(signal,onUpdate) → afterToolCall; results as toolResult message events) → `finishTurn` → turn_end → steering/follow-up polls → agent_end → `finishRun` (`agent.ts:559`) → listeners settle → `waitForIdle`/prompt resolves.

opi: caller (opi-coding-agent harness, `crates/opi-coding-agent/src/harness.rs:2959/2998/3033/3067`) → `Agent::new` (`agent.rs:573`: Models + trusted registrations + authorizer + validated model spec) → `arm_run` (`:857`) → `prompt_armed` (`:680`: armed-generation validation, push user text) → `run_with_token` → `agent_loop_with_identities` (`agent_loop.rs:205`): per turn — cancel check, steering drain (+`QueueUpdate`) → `prepare_initial_turn` + `commit_preparation` (first turn only) → `transform_context` → `convert_to_llm` → `normalize_context` (blocking pool) → `Request` build → `Models::prepare_call_with_policy` (route+auth once) → provider evidence → `'stream:` `start_attempt` → `ChatStream` events (MessageStart/Update/MessageEnd; terminal assistant message into state) → tool arms (parse → preflight: registry → before hook → schema → authorize → evidence → execute_authorized → after hook → ToolExecutionEnd + toolResult into context) → TurnEnd → on stream Err: retry admission loop (AutoRetry events) or failure-partial preservation → `should_stop_after_turn` → steering/follow-up poll → `prepare_next_turn_transaction` + commit (warranted only) → AgentEnd → `run_operation` settlement (`agent.rs:978`: state keep or committed-baseline rollback) → caller finalizes/abandons evidence on `AgentRunResult` (Drop fail-closed).

---

## Verdict tallies

| Verdict | Count | Records |
|---|---|---|
| equivalent | 1 | AGT-TOOL-03 |
| partial | 11 | AGT-AGENT-01, AGT-AGENT-02, AGT-QUEUE-01, AGT-QUEUE-02, AGT-TOOL-04, AGT-TOOL-06, AGT-ABORT-01, AGT-STATE-01, AGT-STATE-02, AGT-STATE-03, AGT-THINK-01 |
| missing | 4 | AGT-TOOL-05, AGT-EVENT-03, AGT-IDENT-01, AGT-OBSVR-01 |
| different | 14 | AGT-AGENT-03, AGT-QUEUE-03, AGT-TOOL-01, AGT-TOOL-02, AGT-TOOL-07, AGT-ABORT-02, AGT-ABORT-03, AGT-STATE-04, AGT-EVENT-01, AGT-EVENT-04, AGT-PROXY-01, AGT-STREAMFN-01, AGT-APIKEY-01, AGT-RETRY-01 |
| opi-only | 7 | AGT-EVENT-02, AGT-ONLY-AUTHORITY, AGT-ONLY-EVID, AGT-ONLY-VAL, AGT-ONLY-DIAG, AGT-ONLY-SDK, AGT-ONLY-SESSION |
| unresolved | 0 | — |

Total: 37 records.

## Coverage honesty

- pi side: read fully `agent.ts`, `agent-loop.ts`, `proxy.ts`, `stream-fn.ts`, `types.ts`, `index.ts`, `package.json`, README. Tests were inventoried by name and selectively read (`e2e.test.ts` excerpts); the full bodies of `agent.test.ts` (38K), `agent-loop.test.ts` (67K), `proxy.test.ts`, `agent-tool-typebox.test.ts` and `test/utils/` were NOT read line-by-line — behavioral claims rest on source plus test names/titles.
- opi side: read fully `agent.rs`, `agent_loop.rs`, `loop_types.rs`, `hooks.rs`, `event.rs`, `streaming_proxy.rs`, `session.rs`, `session_branch.rs`, `session_context.rs`, `session_event.rs`, `harness.rs`, `authority.rs`, `sdk.rs`, `compaction.rs`, `extension.rs`, `message.rs`, `tool.rs`, `tool/result.rs`, `validation.rs`, `diagnostic_sink.rs`, `lib.rs`. `evidence.rs` (2664 lines) and `diagnostic.rs` (1167 lines) were read at module-doc + public-API level only (grep-guided); their internal invariant tests were not read. opi-agent `tests/` (34 files) were inventoried and selectively grepped, not fully read.
- opi-coding-agent harness (`crates/opi-coding-agent/src/harness.rs`) was inspected only at the Agent-call seams needed for the caller-flow trace; its own behavior belongs to another batch. pi-coding-agent (session persistence, compaction, RPC) was NOT inspected — out of this batch's declared pi scope, so opi-only verdicts for the session stack are scoped to `packages/agent` absence, not to pi-the-product absence.
- Deferred-response resumption (AGT-TOOL-06): no live caller on either side was executed; the verdict compares source-level continuation seams only.
- `ToolExecutionUpdate` production outside `crates/opi-agent` (e.g., a coding-agent tool emitting it directly) was not exhaustively excluded; within this crate it has no producer.
- No code was executed against either tree; no builds/tests were run (read-only constraint).

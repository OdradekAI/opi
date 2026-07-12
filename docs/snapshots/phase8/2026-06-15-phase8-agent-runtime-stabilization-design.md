# Phase 8 Agent Runtime Stabilization Design

## Overview

Phase 8 stabilizes the runtime concepts in `opi-agent` and the SDK/RPC surfaces
that expose them. Phase 6 hardened alignment, and Phase 7 makes behavior
observable. Phase 8 uses that foundation to make the agent loop, hooks, event
order, tool scheduling, cancellation, compaction, and embedding contracts
predictable enough for internal packages and external embedders.

This is not a 1.0 API freeze. It is a 0.x stabilization pass: reduce ambiguity,
document invariants, add contract tests, and remove or hide surfaces that cannot
be honestly supported.

## Goals

- Define the runtime contract for agent event order, hook order, cancellation,
  queue polling, and tool scheduling.
- Clarify which `opi-agent` APIs are supported 0.x surfaces and which remain
  internal implementation details.
- Harden `ExtensionRegistry`, `AgentHooks`, `Tool`, `Agent`, and low-level
  `agent_loop` behavior with contract tests.
- Align SDK/RPC command semantics with runtime behavior.
- Use Phase 7 diagnostics and traces to make contract violations visible.
- Keep runtime abstractions Rust-native and avoid a shared `opi-types` crate.

## Non-Goals

- No stable 1.0 public API promise.
- No TypeScript extension API compatibility.
- No package ecosystem expansion.
- No new adapter kind.
- No web UI product work.
- No provider OAuth work.
- No in-core plan mode, sub-agent system, todo system, permission popup, or
  MCP runtime.
- No rewrite of the whole agent loop unless a contract test proves the current
  shape cannot satisfy the required behavior.

## Relationship to pi

Pi's agent core is explicit about event order, tool execution mode, hook
semantics, steering, follow-up, and the distinction between app messages and LLM
messages. Phase 8 preserves those runtime semantics while using Rust enums,
traits, cancellation tokens, and error types.

The goal is semantic alignment, not TypeScript API compatibility. Pi declaration
merging maps to explicit Rust message variants or extension state surfaces, not
to a dynamic type system.

## Runtime Contracts

### Agent Event Order

Document and test the order:

```text
agent_start
  -> turn_start
  -> message_start/end for input messages
  -> assistant message_start/update/end
  -> tool_execution_start/update/end
  -> tool result message_start/end
  -> turn_end
  -> should_stop_after_turn
  -> prepare_next_turn
  -> steering queue or follow-up queue polling
  -> agent_end when the run terminates
```

Contract tests should cover:

- no-tool runs;
- one tool run;
- parallel tool batches;
- sequential tool batches;
- mixed batches where one sequential tool forces sequential mode;
- tool validation failure;
- before hook block;
- after hook modification;
- cancellation during provider stream;
- cancellation during tool execution;
- compaction stop before next turn;
- `prepare_next_turn` injection before steering/follow-up polling;
- `prepare_next_turn` not running after a terminal `should_stop_after_turn`;
- steering delivered before follow-up.

### Hook Semantics

Document and test:

| Hook | Required behavior |
|---|---|
| `transform_context` | Runs before provider conversion and may alter app-level messages |
| `convert_to_llm` | Converts app messages to provider messages and filters session-only state |
| `before_tool_call` | Runs after schema validation and before tool execution; may block |
| `after_tool_call` | Runs after execution and before final tool result events |
| `should_stop_after_turn` | Runs after `turn_end` and before steering/follow-up polling |
| `prepare_next_turn` | Runs after `should_stop_after_turn` permits continuation and before steering/follow-up polling; may inject messages for the next provider request |

If an adapter or extension implements only a subset, skipped hooks must be
visible through Phase 7 trace data when verbose tracing is enabled.

### Tool Scheduling

Tool scheduling should be documented as:

- global default execution mode;
- per-tool execution mode override;
- any sequential tool in a batch forces sequential execution for the whole
  batch;
- completion events may follow completion order;
- persisted tool-result messages follow assistant source order;
- early termination only applies when every finalized tool result in the batch
  sets `terminate`.

### Cancellation

Cancellation should have a single runtime story:

- provider stream cancellation;
- tool cancellation;
- adapter best-effort cancel;
- RPC abort;
- interactive abort;
- shutdown.

The implementation may use different mechanisms internally, but the observable
contract should be consistent: cancelled work emits a terminal event or
diagnostic, no pending run remains active, and session persistence records only
finalized state.

## API Surface Review

Classify public items in `opi-agent` into:

| Category | Meaning |
|---|---|
| supported 0.x | documented, contract-tested, may still break across 0.x with changelog |
| unstable internal | public only because crate layout requires it; documentation warns users |
| candidate removal | should be hidden, moved, or removed before stronger API claims |

The review should cover:

- `Agent`;
- `agent_loop`;
- `AgentHooks`;
- `Tool`;
- `Extension` and `ExtensionRegistry`;
- `AgentEvent`;
- `AgentSessionEvent`;
- `SessionEntry`;
- `SdkCommand` and `SdkResponse`;
- streaming proxy primitives.

Phase 8 should avoid moving types between crates unless the review proves a
current crate owner is semantically wrong.

## SDK and RPC Contract

SDK and RPC commands should be mapped to runtime behavior:

| Command | Contract focus |
|---|---|
| `prompt` | accepted vs rejected when busy, event stream shape |
| `continue` | valid last-message state and error response |
| `abort` | cancellation result and idle transition |
| `steer` | queue behavior while running |
| `follow_up` | queue behavior when idle or running |
| `set_model` | busy-state rejection and capability validation |
| `set_thinking_level` | capability validation and session metadata decision |
| `compact` | compaction event order and failure response |
| `session_info` | stable metadata fields |
| `extension_command` | dispatch semantics and error shape |

Unsupported mutations while the agent is running should fail honestly with
structured errors rather than silently queuing or partially applying changes.

## Data Flow

```text
SDK/RPC command
  -> runtime state guard
  -> Agent or CodingHarness operation
  -> AgentEvent / AgentSessionEvent
  -> diagnostics and trace
  -> SDK/RPC response
```

```text
provider response
  -> agent loop
  -> assistant events
  -> tool scheduler
  -> hook chain
  -> session persistence
  -> should_stop_after_turn
  -> prepare_next_turn
  -> queue polling
```

## Error Handling

Runtime errors should be classified:

- provider error;
- tool validation error;
- tool execution error;
- hook error;
- cancellation;
- compaction error;
- session persistence error;
- invalid runtime state;
- unsupported command.

Errors that are part of normal agent operation should surface as events or
structured responses. Panics should be reserved for programmer bugs and should
not appear in normal model/tool/provider failure paths.

## Testing Strategy

| Level | Coverage |
|---|---|
| `opi-agent` unit/integration | loop event order, hook semantics, queue polling, cancellation |
| SDK/RPC contract | command acceptance, rejection, correlated responses, busy-state behavior |
| extension tests | hook composition, command dispatch, state restore/serialize |
| session tests | only finalized state persists under cancellation and errors |
| trace assertions | Phase 7 trace reflects runtime transitions |

Tests should use mock providers and mock tools. No live provider or filesystem
side effects should be required except isolated temp directories for session
tests.

## Success Criteria

Phase 8 is complete when:

1. Runtime event order, including `prepare_next_turn` and queue polling order,
   is documented and covered by contract tests.
2. Hook order and failure semantics are documented and tested.
3. Tool scheduling and termination semantics match pi where required and are
   tested for parallel, sequential, and mixed batches.
4. Cancellation has a consistent observable contract across provider, tool,
   adapter, RPC, and shutdown paths.
5. SDK/RPC command behavior is documented, versioned, and covered by tests.
6. Public `opi-agent` surfaces are classified as supported 0.x, unstable
   internal, or candidate removal.
7. Phase 7 diagnostics and traces cover runtime contract failures.
8. No ecosystem expansion or workflow-heavy feature enters core.

## Phase 9 Handoff

Phase 9 should build on the stabilized `Tool` and hook contracts to improve
built-in tool correctness. Any tool policy or output shape change must respect
the Phase 8 runtime contract and update SDK/RPC documentation when visible to
clients.

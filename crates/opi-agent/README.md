# opi-agent

[![Crates.io](https://img.shields.io/crates/v/opi-agent.svg)](https://crates.io/crates/opi-agent)
[![Docs.rs](https://docs.rs/opi-agent/badge.svg)](https://docs.rs/opi-agent)

> Provider-independent agent runtime used by [opi](https://github.com/OdradekAI/opi).

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

Provider-independent agent runtime for Rust: the agent loop, JSON-Schema tool
contract, lifecycle hooks, session storage and compaction, SDK/RPC types,
extensions, and a streaming proxy — the engine `opi-coding-agent` builds on.

```sh
cargo add opi-agent
```

## Status

Current crate version: `0.7.2`, inherited from the workspace package version.

`opi-agent` owns the agent loop and runtime primitives: tool contracts,
JSON Schema argument validation, parallel/sequential tool execution, lifecycle
hooks, event emission, steering/follow-up queues, session JSONL storage,
branch reconstruction, context compaction, SDK/RPC types, extensions, local
diagnostics, redacted trace envelopes, and streaming proxy support.

The runtime contract stays focused on the loop itself rather than adding
workflows. Tool results carry `truncated` and tool-owned structured diagnostics,
which the loop lifts into the shared diagnostic/trace system and exposes on
public `ToolExecutionEnd` events while keeping provider-facing tool-result
messages limited to LLM-visible content and failure state.
`ProviderErrorCategory` values from `opi-ai` map into redacted diagnostics and
trace records, provider-returned cancellations surface as `AgentError::Cancelled`,
and retry diagnostics distinguish exhausted retry budgets from suppression
after partial provider output. Append-only session storage carries typed
metadata entries and branch summaries while keeping `SessionHeader::version =
1`; metadata entries do not enter provider context, but branch summaries do,
through `session_context::reconstruct_context`.

It depends on `opi-ai` for provider and message types. It does not implement the
`opi` CLI, terminal UI, or built-in filesystem/shell tools; those live in
`opi-coding-agent` and `opi-tui`.

## Auth and Session-Affinity Boundary

`opi-agent` does not perform credential IO or construct OAuth providers. It
does preserve typed auth outcomes across the live loop: non-retryable
`ProviderError::CredentialNeeded`, `ProviderError::CredentialRevoked`, and
`ProviderError::AccountIdMissing { provider_id }` map to matching `AgentError`
variants and redacted diagnostics without string matching. Account-id absence
is non-retryable and distinct from revocation; product layers use the canonical
`/login <provider>` remediation. The outer interactive product may retry one pre-output pending turn exactly
once after a successful explicit login for the same provider; non-interactive
products never prompt, and revoked credentials never trigger automatic
re-login. `opi-agent` only preserves the typed outcomes and existing message
buffer; the coding-agent outer TUI owns login and retry policy.

The agent also carries an opaque `session_id` from `Agent` through
`AgentLoopContext` into every provider `Request`. It owns no persistence or
provider-specific header mapping: `opi-coding-agent` supplies the active
session id, and `opi-ai` adapters decide whether their reviewed cache-affinity
mapping consumes it.

## Core Abstractions

| Item | Purpose |
|------|---------|
| `Agent` | Stateful wrapper around the loop with prompt, continue, abort, subscribe, steering, follow-up, model switching, and tool registration helpers. |
| `Tool` | JSON Schema based tool contract with cancellable execution and optional progress updates. |
| `ExecutionMode` | Controls whether a tool can run in a parallel batch or forces sequential execution. |
| `AgentHooks` | Lifecycle hooks for context transforms, LLM conversion, tool policy/results, stopping, and next-turn preparation. |
| `AgentEvent` | Runtime event stream for lifecycle, streaming text, tool calls, queues, retries, compaction, and end state. |
| `AgentSessionEvent` | Session-level event protocol used by `opi --json`. |
| `AgentLoopConfig` | Loop limits, retry config, compaction config, and related runtime settings. |

## Loop Shape

The loop emits a fixed runtime event order. `AgentStart` fires once before the
first turn and `AgentEnd` fires once on every termination path (normal stop,
hook stop, terminate flags, cancellation, or error). Per turn (`0..max_turns`):

```text
agent_start                              # once, before turn 0
  for each turn:
    cancel check                          # cancelled -> AgentEnd, AgentError::Cancelled
    turn_start
    transform_context                     # AgentHooks::transform_context
    convert_to_llm                        # AgentHooks::convert_to_llm
    validate request capabilities         # failure -> AgentEnd, AgentError::Provider
    provider.stream(Request)
      message_start                       # assistant stream Start
      message_update                      # per text/thinking delta
      message_end                         # complete assistant message
      if tool calls are present:
        validate tool args (jsonschema)
        tool_execution_start              # per tool call
        before_tool_call                  # AgentHooks::before_tool_call (may block)
        tool.execute                      # parallel batch, or sequential if any tool is Sequential
        after_tool_call                   # AgentHooks::after_tool_call (may replace result)
        tool_execution_end                # per tool call
        turn_end                          # assistant message + tool_results
        if every result terminates -> AgentEnd, return Ok
        should_stop_after_turn            # true -> AgentEnd, return Ok (compaction stop)
      else:
        turn_end                          # assistant message, no tool_results
        should_stop_after_turn            # true -> AgentEnd, return Ok
    prepare_next_turn                     # AgentHooks::prepare_next_turn; SKIPPED after a
                                         # terminal should_stop_after_turn; may inject messages
    drain steering queue                  # non-empty -> QueueUpdate, append, next turn
    if no tools are pending:
      pop follow-up queue                 # non-empty -> QueueUpdate, append, next turn
      else -> stop
agent_end                                 # once, on termination
```

Boundaries:

- `should_stop_after_turn` runs after `turn_end` and before `prepare_next_turn`
  and any queue polling. A compaction coordinator returns `true` here to stop
  before the next turn; `prepare_next_turn` and steering/follow-up polling do
  not run after a terminal stop.
- `prepare_next_turn` runs only when `should_stop_after_turn` permits
  continuation, and before steering/follow-up polling. Injected messages are
  included in the next provider request.
- Steering is drained before follow-up. Follow-up is popped only when no tools
  are pending and the steering queue is empty.
- `CompactionEngine` is a context-size primitive; the higher-level coordinator
  that connects compaction to persisted CLI sessions lives in `opi-coding-agent`
  and stops the loop through `should_stop_after_turn`.

Retryable provider errors such as rate limits and timeouts can be retried
through `AgentLoopConfig.retry`. Retry start/end events are surfaced through
`AgentEvent`.

## Hook Semantics

`AgentHooks` customizes the loop. The six methods run in this order and have
these effects:

| Hook | Order / effect |
|------|----------------|
| `transform_context` | Runs before provider conversion; may alter app-level messages. |
| `convert_to_llm` | Converts app messages to provider messages and filters session-only state. |
| `before_tool_call` | Runs after JSON Schema argument validation and before `tool.execute`; may `Deny` to block execution (the deny reason becomes the tool error). |
| `after_tool_call` | Runs after execution and before the final `ToolExecutionEnd` event; may `Replace` the result so the replacement is what is emitted and persisted. |
| `should_stop_after_turn` | Runs after `turn_end` and before steering/follow-up polling; returning `true` stops before the next turn and skips `prepare_next_turn`. |
| `prepare_next_turn` | Runs only when `should_stop_after_turn` permits continuation, and before steering/follow-up polling; may inject messages into the next provider request. |

Extension composition: `ExtensionRegistry::wrap_hooks` runs the base
`AgentHooks` method first, then each extension in registration order. A
`Block` from an extension's `on_before_tool_call` stops the chain at the first
block; later extensions are not consulted. Extension `on_after_tool_call`
observers cannot modify the result; only the base hook can `Replace`.

When an adapter or extension implements only a subset of hooks, the skipped
hooks are recorded as `trace::TraceKind::HookSkipped` records when verbose
tracing is enabled. The runtime pushes the per-run `TraceCollector` to every
extension via `Extension::set_trace_collector` before each run (and clears it
after), so adapters that short-circuit an undeclared hook can record the skip.

## Tool Scheduling

The scheduler batches the tool calls carried by one assistant message and runs
each batch by these rules:

- The global default execution mode is `Parallel`. A tool overrides it by
  implementing `Tool::execution_mode` to return `Sequential`.
- If any tool call in a batch reports `Sequential`, the entire batch runs
  sequentially; otherwise the batch runs in parallel.
- In a sequential batch, tool calls run strictly in assistant source order:
  each call starts, executes, and finishes before the next begins.
- In a parallel batch, every `ToolExecutionStart` is emitted before any result
  is awaited, and results are gathered with `join_all`, which preserves source
  order. `ToolExecutionEnd` events are therefore emitted in source order in the
  current runtime; the contract permits completion-order emission, so observers
  must not depend on a specific end-event ordering across parallel tools.
- Persisted `ToolResult` messages follow assistant source order in both
  sequential and parallel batches, independent of completion order.
- The run terminates early only when every finalized tool result in the batch
  sets `terminate`. A single non-terminating result lets the run continue.

Argument validation runs before `before_tool_call` and before `Tool::execute`.
A validation failure is a normal runtime outcome, not a loop error: an error
`ToolResult` (`is_error = true`, `terminate = false`) is persisted and the run
continues; the hook does not run and the tool does not execute.

## Tool Results and Diagnostics

`ToolResult` is the runtime result contract shared by built-in tools, custom
tools, and extension tools:

| Field | Meaning |
|---|---|
| `content` | LLM-visible text or image output. |
| `details` | Optional structured metadata for runtime, UI, JSON/RPC, and trace boundaries. |
| `is_error` | Whether the result represents a tool failure. |
| `terminate` | Whether this result can end the run when every result in the batch also terminates. |
| `truncated` | Whether output was shortened or bounded. |
| `diagnostics` | Tool-owned structured cause records (`code`, `message`, `context`). |

The agent loop reads diagnostics after `after_tool_call`, so replacement results
can replace diagnostic context too. Each `ToolDiagnostic` is lifted into a
shared `Diagnostic` and diagnostic-linked trace record. Public events are
redacted before emission; provider requests receive only the tool result
content, `is_error`, `truncated`, and timestamp fields through
`opi_ai::message::ToolResultMessage`.

## Cancellation

Cancellation has one observable contract across every path — provider stream,
tool, adapter best-effort cancel, RPC abort, interactive abort, and shutdown.
The mechanisms differ, but the outcome is the same: cancelled work emits a
terminal event or diagnostic, no run is left pending, and session storage
records only finalized state.

In `agent_loop` a single `CancellationToken` is checked at three points each
turn — before the turn starts, during provider streaming, and during retry
backoff. When cancellation is observed the loop records an informational
`agent cancelled` diagnostic (tagged with the lifecycle phase), emits the
terminal `AgentEnd` event carrying the finalized message buffer, and returns
`Err(AgentError::Cancelled)`. Partial streaming content accumulated for an
in-flight assistant message is discarded: it is only pushed to the message
buffer when the stream's `Done` event arrives, so a cancel mid-stream writes
no partial assistant message.

Trace consumers must tolerate open turns on early provider exits. Provider
failure and provider-stream cancellation may emit `TurnStarted` without a
matching `TurnEnded`; the terminal boundary for those paths is `AgentEnd` plus
trace `RunEnded` and the linked diagnostic.

`Agent::abort` (and the harness `cancel` / `cancel_token` helpers) cancel the
active run's token; the token is reset before the next turn, so a cancelled
runtime returns to idle and accepts a new prompt. A tool that observes its
`CancellationToken` returns promptly — the process adapter tool returns
`ToolError::Cancelled` after a best-effort `cancel` message is dispatched to
the adapter child — and the result becomes a finalized error tool result, not
a hang. RPC abort, interactive abort, and shutdown all reduce to this same
token primitive, so the observable contract is uniform across embedder
boundaries.

Session persistence is append-only per finalized `AgentMessage::Llm` entry, and
a turn whose run returns `Err(AgentError::Cancelled)` is not persisted at all,
so storage can never contain a partial assistant message or a half-applied
tool result.

## Sessions and Compaction

Session storage is append-only JSONL:

- First line: `SessionHeader` (`version = 1`).
- Entries: `MessageEntry`, `CompactionEntry`, `LeafEntry`, and
  `ExtensionStateEntry` (the `SessionEntry` enum is `#[non_exhaustive]`;
  additive variants may arrive across 0.x).
- Additive typed entries on the v1 header: `SessionInfoEntry`
  (`session_info`), `ModelChangeEntry` (`model_change`),
  `ThinkingLevelChangeEntry` (`thinking_level_change`), `LabelEntry` (`label`),
  and `BranchSummaryEntry` (`branch_summary`). Metadata entries
  (`session_info`, `model_change`, `thinking_level_change`, `label`) do not
  advance the content tip and do not enter provider context; `branch_summary`
  is injected into reconstructed LLM context as a metadata-parented message by
  `session_context::reconstruct_context`, and product provider conversion
  forwards it as context when present.
- `custom_message` is deferred: opi ships no `custom_message` writer and treats
  unknown `custom_message` entries like other unknown future entries on read.
- Reader recovery distinguishes corrupt middle entries (malformed JSON or
  missing required fields, surfaced as diagnostics) from unknown future entry
  types (well-formed JSON with an unrecognized `type`, skipped and counted but
  not preserved across read+rewrite), and skips truncated trailing lines.
- `session_branch::SessionTree` reconstructs active branches from `parent_id`
  links and the latest `LeafEntry`.

Compaction primitives include threshold/manual/overflow reasons,
`CompactionEngine::should_compact`, `CompactionEngine::compact`, and
`CompactionHooks` for custom summary generation. `opi-coding-agent` owns the
higher-level coordinator that connects these primitives to persisted CLI
sessions.

## SDK and RPC Command Contract

`sdk` (`SDK_SCHEMA_VERSION = 3`) defines the unstable 0.x command set shared by
RPC JSONL mode and embedders. Each command
carries an optional `id` echoed on its response; RPC emits one `response` per
command, carrying `command`, `success`, optional `id`/`error`, optional
structured `error_code` (e.g. `unsupported_trace_request`), and optional `data`.

Structured `error_code` values are limited to runtime-contract failures:

| `error_code` | Meaning |
|---|---|
| `unsupported_trace_request` | `trace` was requested when the session has no trace sink. |
| `agent_busy` | A run is already active, or a runtime-state mutation was attempted while running. |
| `harness_unavailable` | The RPC runner has no attached `CodingHarness`. |
| `compaction_failed` | Manual compaction returned an error. |
| `extension_command_not_handled` | No registered extension handled the requested command. |

Idle capability errors from `set_model` and `set_thinking_level` remain
free-text validation failures and do not carry `error_code`.

Command-state contract (the runtime guard, not the parse layer):

| Command | Idle | While running |
|---|---|---|
| `prompt` / `continue` | accepted → spawns a run; async events follow | rejected (`agent is already running; use steer or follow_up to queue messages`) |
| `abort` | successful no-op | cancels the active run, success |
| `steer` | queued on the harness | queued on the active control handle |
| `follow_up` | queued on the harness | queued on the active control handle |
| `set_model` | validated (same provider, known model, thinking revalidation) | rejected (`cannot change model while agent is running`) |
| `set_thinking_level` | validated (`off|low|medium|high`, model supports it / budget) | rejected (`cannot change thinking level while agent is running`) |
| `compact` | manual compaction (result + diagnostic) | rejected (`cannot compact while agent is running`) |
| `session_info` | returns model / resources / session_id | rejected (`cannot query session info while agent is running`) |
| `extension_command` | dispatched to the registry (data / `not handled` / error) | rejected (`cannot dispatch extension command while agent is running`) |
| `trace` | versioned redacted envelope, or `unsupported_trace_request` | allowed (per-run snapshot) |
| `quit` | success + shutdown | success + shutdown (waits for an active run to clean up) |

- A rejected mutating command is dropped, never queued or partially applied: a
  busy `set_model` / `set_thinking_level` / `compact` leaves the running turn and
  its configuration untouched.
- `steer` and `follow_up` are the only commands that queue during a run; `steer`
  is delivered before the next provider request, `follow_up` when the agent would
  otherwise stop.
- Malformed or unknown commands fail as a structured `parse` response, not a
  silent drop.
- `abort` while running is the same observable cancellation as interactive abort
  and shutdown (see Cancellation).

## SDK, Extensions, Diagnostics, and Proxy

- `sdk` defines schema-versioned command/response types shared by RPC JSONL
  mode and embedders. `SDK_SCHEMA_VERSION` is `3`.
- `extension` provides `Extension` and `ExtensionRegistry` for lifecycle hooks,
  custom tools, custom commands, event observers, extension state, custom
  providers, and model overrides.
- `diagnostic` and `diagnostic_sink` provide typed diagnostics with redaction
  helpers for public JSON/text boundaries.
- `trace` stores a local, redacted trace envelope for the latest run when a
  caller opts in.
- `streaming_proxy` forwards JSONL commands/events over arbitrary
  `BufRead`/`Write` transports, emits a `proxy_ready` header, buffers events,
  supports cancellation, and redacts common secret patterns by default.

All SDK/RPC/proxy surfaces are unstable 0.x APIs. Clients should check schema
versions and pin exact crate versions when needed.

## API Surface Classification

`opi-agent` is a 0.x crate. Public items fall into three tiers:

| Tier | Meaning |
|---|---|
| supported 0.x | Documented and contract-tested; may still change across 0.x with a changelog entry. |
| unstable internal | Public only because crate layout requires it; documentation warns consumers to pin versions. |
| candidate removal | Should be hidden, moved, or removed before a stronger API claim. |

| Surface | Tier | Notes |
|---|---|---|
| `Agent` | supported 0.x | Stateful loop wrapper; contract-tested. |
| `agent_loop` | supported 0.x | Core async entry point; runtime event-order contract tested. |
| `AgentHooks` | supported 0.x | Six lifecycle hooks; hook-order and failure contract tested. |
| `AgentLoopConfig`, `AgentLoopContext`, `AgentError`, `AgentMessage` | supported 0.x | Required by the supported low-level `agent_loop` entry point. |
| `Tool`, `ToolDef`, `ToolResult`, `ToolError`, `ExecutionMode` | supported 0.x | JSON-Schema tool contract plus result/error/scheduling types used by embedders. |
| `AgentEvent`, `AgentEventSink` | supported 0.x | In-process runtime event stream; `AgentEvent` is `#[non_exhaustive]` because new variants may arrive across 0.x. |
| `AgentSessionEvent` | unstable internal | `opi --json` wire protocol (`NDJSON_SCHEMA_VERSION = 2`, owned by `opi-coding-agent`); `#[non_exhaustive]`. Check the schema version. |
| `SessionEntry` | unstable internal | Session JSONL storage layout; lives at `session::SessionEntry`, not re-exported at the crate root; `#[non_exhaustive]`. |
| `Extension`, `ExtensionCommand`, `ExtensionError`, `ExtensionHookResult`, `ExtensionRegistry` | unstable internal | Extension lifecycle and composition surface; the `extension` module marks it `# Unstable`. |
| `SdkCommand`, `SdkResponse`, `SDK_SCHEMA_VERSION` | unstable internal | RPC/SDK command model (`SDK_SCHEMA_VERSION = 3`); the `sdk` module marks it unstable 0.x. |
| `StreamingProxy`, `ProxyConfig`, `ProxyEvent`, `ProxyHandler`, `SecretRedactor`, `StreamingProxyError` | unstable internal | Streaming-proxy primitives; the `streaming_proxy` module marks them unstable 0.x. |
| `Diagnostic`, `DiagnosticPayload`, `RedactionMode`, `Severity`, `redact`, `redact_text`, `DiagnosticSink`, `NullSink`, `RecordingSink` | unstable internal | Diagnostic payload and sink plumbing used by runtime surfaces; current contract is redaction/schema-version behavior, not a stable API shape. |
| `FileTraceSink`, `RecordingTraceSink`, `TRACE_SCHEMA_VERSION`, `TraceCollector`, `TraceError`, `TraceKind`, `TraceRecord`, `TraceSink` | unstable internal | Local trace envelope plumbing; the `trace` module marks it unstable 0.x and carries `TRACE_SCHEMA_VERSION = 1`. |
| `AgentState` | unstable internal | Runtime state holder exposed for crate layout and harness integration; not a supported embedder contract. |
| `AgentHarness`, `Phase`, `HarnessError`, `HarnessResult`, `HarnessSnapshot`, `HarnessSession`, `HarnessRuntimeConfig`, `HarnessRuntimeConfigBuilder`, `SavePoint`, `PendingWriteQueue`, `PendingWrite`, `PendingWriteKind`, `SessionRepo`, `SessionFacade`, `JsonlHarnessSession`, `JsonlSessionRepo` | unstable internal | Generic agent-harness/session-facade orchestration seam above `Agent`; contract-tested but does not drive the loop itself yet. The `harness` module marks it unstable 0.x. |

This review found no candidate-removal crate-root re-exports. Every crate-root
`pub use` in `src/lib.rs` is named in the table above. Public modules may expose
additional items through module paths; unless those items are named as supported
0.x surfaces here, they are unstable internal 0.x APIs.

There is no stable 1.0 API promise. Stability is enforced today by
`#[non_exhaustive]` on `AgentEvent`, `AgentSessionEvent`, `SessionEntry`, and
the trace/hook result enums, and by module-level `# Unstable` / `unstable 0.x`
prose on `sdk`, `streaming_proxy`, `extension`, and `trace`. There is no
`#[doc(hidden)]` or `#[unstable]` feature gate, so embedders should pin exact
crate versions. The local trace envelope carries `TRACE_SCHEMA_VERSION = 1`.

## Non-Goals

The crate stays 0.x and the `harness` seam is internal-only. The following
remain explicitly out of scope and are not claimed:

- No stable 1.0 public API promise (surfaces stay 0.x).
- No TypeScript extension API compatibility.
- No package ecosystem expansion or package marketplace.
- No additional `opi-agent` extension-adapter kind beyond `process-jsonl`
  (`opi-extension-jsonl-v1`); `command.execute` is a separate
  `opi-coding-agent` / `opi-protocol` surface.
- No web UI product work.
- No credential store, login presenter, or vendor OAuth flow implementation in
  `opi-agent`; those remain owned by `opi-ai` contracts and
  `opi-coding-agent` product wiring.
- No in-core plan mode, sub-agent, todo, permission popup, or MCP runtime.
- No shared `opi-types` crate.
- No unjustified public type migration between crates.
- No rewrite of the whole agent loop unless a contract test proves the current
  shape cannot satisfy the required behavior.

## Public Modules

`agent`, `compaction`, `diagnostic`, `diagnostic_sink`, `event`, `extension`,
`harness`, `hooks`, `loop_types`, `message`, `sdk`, `session`, `session_branch`,
`session_context`, `session_event`, `state`, `streaming_proxy`, `tool`, `trace`,
and `validation`.

The crate root re-exports the most common runtime types, including `Agent`,
`Tool`, `ToolResult`, `ToolError`, `ExecutionMode`, `AgentHooks`, `AgentEvent`,
`AgentSessionEvent`, `AgentLoopConfig`, `SdkCommand`, `SdkResponse`, and
`ToolDef`.

## Testing Support

Use `opi_ai::test_support::MockProvider` with custom `Tool` implementations for
deterministic loop tests. Tests that touch session storage should use isolated
temporary directories.

## License

MIT. See the workspace [LICENSE](../../LICENSE).

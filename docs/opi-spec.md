# Opi Technical Specification

> Opi is a Rust reimplementation of the [pi](https://github.com/earendil-works/pi) AI agent toolkit. It preserves pi's runtime semantics while using Rust-native APIs, storage formats, and release practices.

## 0. Document Control

| Field | Value |
|---|---|
| Status | Draft |
| Spec version | 0.6-draft |
| Last updated | 2026-07-14 |
| Repository | `https://github.com/OdradekAI/opi` |
| Upstream studied | `pi` 0.80.2 at `.repo/pi-0.80.2/`; alignment is assessed by fresh `opi-realign` audits under [`docs/realign/`](realign/) |
| Current implementation | `opi` 0.7.0 workspace with Phases 1-14 implemented; Phase 14 provider/auth product paths are present, while dynamic model refresh remains substrate-only |
| Next milestone | Phase 15 safety/sandbox implementation |

This document is normative for the current design. Changes that alter public APIs, event protocols, session storage, release behavior, or phase boundaries SHOULD update this file in the same change.

The current upstream evidence baseline is `.repo/pi-0.80.2`. Alignment against
upstream is assessed by fresh `opi-realign` audits whose reports are written
under `docs/realign/`.

Normative terms:

- **MUST** means required for conformance.
- **SHOULD** means expected unless a documented reason says otherwise.
- **MAY** means optional extension behavior.

## 1. Executive Summary

Opi mirrors pi's package structure with four Rust crates:

- `opi-ai`: provider-agnostic LLM streaming.
- `opi-agent`: agent loop, stateful agent, hooks, tools, queues, and session primitives.
- `opi-tui`: terminal UI components.
- `opi-coding-agent`: the `opi` CLI binary.

The repository has completed Phases 1-14. In addition to the terminal agent,
runtime, package, provider-correctness, tooling, and session-context work,
Phase 14 now supplies OS-keychain credential storage, OAuth for Anthropic,
GitHub Copilot, and OpenAI Codex, per-stream auth resolution, request/session
affinity enrichment, capability-gated Anthropic cache markers, cache/reasoning
usage accounting, and a substrate-only dynamic model refresh API.

Opi does not claim pi package ecosystem parity and does not support npm package install, marketplace behavior, TypeScript extension live reload, provider stream interception through adapters, custom terminal UI adapter rendering, package permission policy enforcement, OAuth providers beyond the three reviewed Phase 14 profiles, image generation, or web/share flows. MCP, sub-agents, plan mode, todos, permission gates, and dynamic plugin loading should build on the substrate rather than become core features.

The central design rule:

> Preserve pi's behavior where users and integrators depend on it. Do not preserve pi's TypeScript APIs, npm extension ABI, config files, or session files by default.

## 2. Design Philosophy

| Principle | pi 0.80.2 | opi design |
|---|---|---|
| Minimal core | `CONTRIBUTING.md` and coding-agent docs keep workflow-specific features outside core | Phase 1-3 avoid MCP, dynamic plugin, sub-agent, plan-mode, todo, and background-bash scope creep |
| Layered runtime | `agentLoop` -> `Agent` -> `AgentHarness` / `AgentSession` | `agent_loop` -> `Agent` -> `Harness` / `CodingHarness` |
| Streaming first | `AssistantMessageEventStream` and agent event streams | `Stream<Item = Result<Event, Error>>` with terminal events |
| Provider agnostic | provider owns model catalog, auth, and stream behavior through `Models` | `Provider` trait, registry/collection, provider adapters, credential/OAuth contracts, and per-stream auth resolution |
| Agent vs LLM messages | `AgentMessage[] -> transformContext -> convertToLlm -> Message[]` | app messages in `opi-agent`, provider messages in `opi-ai` |
| Tool isolation | TypeBox schema at LLM boundary | typed Rust tool inputs, generated JSON Schema at the LLM boundary |
| Errors in band | provider failures become `error` stream events | provider/runtime failures surface as events, not panics |
| Append-only sessions | crash-recoverable JSONL session files | opi versioned tree JSONL inspired by pi |
| Lockstep release | all packages share a version | all crates share `workspace.package.version` |

### 2.1 Non-Goals

Opi is not a line-by-line port. Rust's enums, traits, ownership, and cancellation primitives should shape the implementation.

Opi is not API-compatible with pi. TypeScript declaration merging, `jiti` extension loading, and npm package exports do not map cleanly to Rust crates and a static binary.

Opi is not required to read pi config or pi session files in Phase 1. A migration command MAY be added later, but runtime compatibility is not assumed.

Opi is not an extensibility platform in its MVP. MCP is not a built-in core feature in the pi design; it MAY be provided later as an extension or package after the extension API is stable. Built-in sub-agents, plan mode, todo systems, background bash, permanent permission-popup workflows, WASM plugins, and subprocess plugin runtimes are outside Phase 1-3 core scope.

### 2.2 pi Design Boundaries

Pi 0.80.2 is broader than the original terminal coding harness, but it still
preserves boundaries that `opi` should keep unless a later design explicitly
chooses to depart:

- CLI/TUI remains the primary product surface.
- Core ships useful defaults, not workflow-heavy opinions.
- MCP, sub-agents, plan mode, permission gates, and todos belong in extensions, packages, or external tools rather than built-in core.
- Tool safety is primarily controlled through tool selection, visibility, containers/sandboxes, and extension hooks.
- RPC and SDK surfaces support composition without making the terminal product secondary.
- OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex, image
  generation, custom extension UI, npm/gallery workflows, and web/share flows
  need reviewed designs before entering `opi`.

## 3. Relationship to pi

Pi is the behavioral reference. The following behavior should be treated as inherited design, not incidental implementation detail.

### 3.1 Semantics Opi MUST Preserve

| Area | Required behavior | Upstream anchor |
|---|---|---|
| Agent event order | `agent_start`, `turn_start`, message events, tool events, `turn_end`, `agent_end` | `packages/agent/README.md` |
| Provider stream lifecycle | `start`, content deltas, content end events, then `done` or `error` | `packages/ai/src/types.ts` |
| Errors in stream | failures after request start are stream errors and final failed assistant messages | `StreamFunction` contract |
| Message conversion | app messages are transformed before provider conversion | `AgentMessage` / `convertToLlm` |
| Tool batching | parallel by default; any sequential tool makes the whole batch sequential | pi agent README |
| Tool result order | completion events may be completion order; persisted tool-result messages follow assistant source order | pi agent README |
| Tool termination | early stop only when every finalized result in the batch has `terminate` | pi agent README |
| Tool hooks | before hook can block; after hook replaces fields without deep merge | pi hook result types |
| `shouldStopAfterTurn` | runs after `turn_end`, before steering/follow-up polling | pi agent README |
| Steering queue | delivered after current assistant turn and tool calls, before next provider call | pi agent README and RPC docs |
| Follow-up queue | delivered only when the agent would otherwise stop | pi agent README and RPC docs |
| Session durability | append-only writes and recovery from incomplete final line | pi session manager |

### 3.2 Rust-Native Redesigns

| pi mechanism | opi replacement | Rationale |
|---|---|---|
| TypeScript unions and declaration merging | Rust enums plus explicit extension variants | exhaustive matching and safer evolution |
| TypeBox schemas | `schemars`-generated JSON Schema plus `jsonschema` validation | dynamic provider boundary, static tool code |
| dynamic provider imports | feature flags plus explicit registration | predictable binaries and cross-compilation |
| `jiti` TypeScript extensions | deferred Rust-compatible plugin story | avoids Node dependency and unstable ABI in MVP |
| pi `settings.json` / `auth.json` | TOML config and explicit credential resolution | Rust ecosystem convention and comments |
| pi session v3 | opi session v1 tree JSONL | retain branch/compaction semantics without TS-specific entries |
| custom TUI renderer | `ratatui` + `crossterm` | active Rust terminal stack |

### 3.3 Design Reference Matrix

| pi reference | Opi phase | Opi treatment |
|---|---:|---|
| package/crate layout | Phase 0 done | preserve conceptual crate boundaries |
| binary | Phase 0 placeholder, Phase 1 useful | ship `opi`, not `pi` |
| provider streaming | Phase 1 | preserve stream lifecycle and in-band errors |
| Anthropic provider | Phase 1 | first provider implementation |
| `Models` / provider-owned auth | Phase 10/14 | Rust-native collection/auth seam plus OS-keychain credentials and reviewed OAuth flows for Anthropic, GitHub Copilot, and OpenAI Codex |
| `agentLoop` / `Agent` | Phase 1 | preserve loop, hook, queue, and tool batching semantics |
| `AgentHarness` / session repo | Phase 10/13 | generic harness/session facade in `opi-agent`, coding product wrapper in `opi-coding-agent` |
| default coding tools | Phase 1 | interactive defaults are `read`, `write`, `edit`, and `bash` |
| read-only file navigation | Phase 1/3 | `read`, `grep`, `find`, and `ls` cover the pi read-only tool set; `glob` is an additional read-only convenience and core workflows must not depend on it |
| interactive TUI | Phase 1 | terminal-first user surface |
| OpenAI-compatible/OpenRouter/OpenAI/Gemini/Mistral | Phase 2 | provider contract implementations |
| sessions/resume | Phase 2 | independent opi JSONL format with pi-inspired branch and compaction semantics |
| compaction | Phase 2 | preserve compaction semantics, not pi file format |
| JSON event mode | Phase 2 | versioned opi NDJSON with pi-like event shape |
| image support | Phase 3 | preserve multimodal message behavior where providers support it |
| tool selection and safety hooks | Phase 3 | allowlists, visibility, and hooks; no permanent core permission-popup subsystem |
| RPC/SDK/extensions/skills/packages | Phase 4 | primary composition and customization path |
| MCP adapter | Phase 4+ | extension/package example after extension APIs are stable |
| custom extension UI/message renderers | future | ecosystem candidate after built-in TUI and a UI/RPC subprotocol are designed |
| image generation | future | ecosystem candidate after chat provider collection/auth is stable |

Fresh per-dimension drift audits against `.repo/pi-0.80.2` are produced under
[`docs/realign/`](realign/) by the `opi-realign` skill.

### 3.4 pi Alignment Status Vocabulary

Fresh `opi-realign` audits classify drift with this fixed status vocabulary so
that pi drift is tracked consistently:

| Status | Meaning | Required next action |
|---|---|---|
| `Full` | opi preserves the user-visible or integrator-visible pi semantics, even if the Rust implementation differs | keep contract tests and avoid accidental regression |
| `Partial` | opi implements the core idea, but product breadth, edge cases, commands, providers, or ecosystem behavior are narrower than pi | document the missing surface and decide whether a later phase closes it |
| `Intentional Divergence` | opi deliberately chooses a different Rust-native module, interface, storage format, or adapter strategy | record the reason and do not treat this as a parity bug |
| `Missing` | pi has the capability and opi does not, but the capability may still belong on the roadmap | file or link a future phase/task before claiming parity |
| `Out of Scope` | pi has the capability, but opi explicitly does not plan to carry it in core | keep the capability out of core unless a later design changes the scope |

At minimum, a fresh audit SHOULD track core semantic parity, product parity,
and ecosystem parity across agent loop semantics, generic harness ownership,
built-in tools, session format, session tree semantics, provider collection,
auth, provider catalog, OAuth/subscription login, image input, image
generation, package ecosystem, TypeScript extension compatibility (does not
claim), TUI renderer architecture, and workflow features that pi keeps out of core such as MCP,
sub-agents, plan mode, todos, permission popups, and background bash.

## 4. Current Baseline

### 4.1 Version 0.7.0

| Area | Current state |
|---|---|
| Workspace | four crates under one Cargo workspace |
| Versioning | lockstep `0.7.0` |
| Edition | Rust 2024 |
| Internal dependencies | `opi-agent -> opi-ai`, `opi-coding-agent -> opi-ai + opi-agent + opi-tui` |
| External dependencies | Rust-native async, HTTP/SSE, schema, config, TUI, search, tracing, and test stacks from workspace dependencies |
| Binary | `opi` supports interactive TUI, non-interactive text mode, `--json`, `--rpc`, session commands, `--version`, and `--help` |
| CI | `fmt`, `clippy`, `test`, `doc` |
| Release CI | six platform binary workflow |
| Extensibility | RPC JSONL, SDK types, extension API, resource/package discovery, custom provider/model registry, branch selection, streaming proxy, process-JSONL adapter hosting (`opi-extension-jsonl-v1`), and package CLI (`add/remove/list/doctor`) are implemented as unstable 0.x APIs |
| crates.io | publishable crates are quality-gated |

### 4.2 Pre-Stable API Notes

Phase 0 placeholders have been replaced, but 0.x public APIs remain unstable
unless explicitly documented otherwise. Phase 3 hardened the existing surfaces
rather than introduce broad new platform scope.

| Crate | Current surface | Next target |
|---|---|---|
| `opi-ai` | provider streaming, model registry, usage/cost, retry/backoff, custom provider/model registration | keep provider breadth extensible through registration where possible |
| `opi-agent` | agent loop, hooks, queues, tools, sessions, compaction, SDK types, extension API, streaming proxy | keep core runtime narrow and document all 0.x public surfaces as unstable |
| `opi-tui` | ratatui components, markdown/code, diff, themes, keybindings, image rendering, fuzzy pickers, branch picker | keep widgets reusable and deterministic under snapshot tests |
| `opi-coding-agent` | `clap` CLI, TOML config, built-in tools, sessions, JSON/RPC modes, resource/package discovery, branch selection | wire extensibility metadata into prompts/RPC without claiming dynamic Rust plugin loading |

### 4.3 Phase 0 Completion

Phase 0 is complete:

- four-crate workspace;
- lockstep versioning;
- placeholder modules and re-exports;
- CI gates;
- six-platform release workflow;
- `opi --version` and `opi --help`;
- GitHub Release only, crates.io deferred.

## 5. Workspace and Dependencies

### 5.1 Layout

```text
opi/
|-- Cargo.toml
|-- crates/
|   |-- opi-ai/
|   |-- opi-agent/
|   |-- opi-coding-agent/
|   `-- opi-tui/
|-- docs/
|-- .github/workflows/
`-- .claude/skills/opi-release/
```

The earlier draft's root `config/` directory is not present. Built-in themes or syntax assets should live in the owning crate until a real shared asset need appears.

### 5.2 Dependency Graph

```text
opi-ai           (no internal deps)
opi-tui          (no internal deps)
opi-agent        -> opi-ai
opi-coding-agent -> opi-ai, opi-agent, opi-tui
```

Internal dependencies MUST be declared in root `[workspace.dependencies]` and referenced by consumers with `{ workspace = true }`.

### 5.3 Crate Roles

| Crate | Type | Publish target | Role |
|---|---|---|---|
| `opi-ai` | library | crates.io after publish gates pass | provider protocols, model metadata, provider-facing messages |
| `opi-agent` | library | crates.io after publish gates pass | loop, agent, hooks, tools, queues, sessions |
| `opi-tui` | library | crates.io after publish gates pass | terminal rendering library |
| `opi-coding-agent` | binary | crates.io after publish gates pass | `opi` CLI application |

### 5.4 Why There Is No `opi-types`

Types belong to the crate that owns their semantics:

- provider-facing `Message`, `ToolDef`, `ModelInfo`, and `Usage` belong in `opi-ai`;
- runtime `AgentMessage`, `AgentEvent`, `Tool`, and `SessionEntry` belong in `opi-agent`;
- CLI config belongs in `opi-coding-agent`;
- visual state belongs in `opi-tui`.

A shared types crate would become a hub dependency. If a type crosses a crate boundary, the lower semantic owner should expose it directly. Public enums expected to grow SHOULD use `#[non_exhaustive]` before API stabilization.

### 5.5 Dependency Plan

Phase 1 dependencies SHOULD be introduced with the narrowest feature set that can
ship the MVP. Prefer explicit features, optional heavy functionality, and later
phase additions over broad defaults.

| Category | Crate | Status | Rationale |
|---|---|---|---|
| async runtime | `tokio` | present, narrow features | networking, process IO, signals, timers; avoid `features = ["full"]` unless a concrete need appears |
| serialization | `serde`, `serde_json` | present | provider/session protocols |
| library errors | `thiserror` | present | typed error handling for library crates |
| application errors | `anyhow` | Phase 1 | top-level error aggregation in `opi-coding-agent`; library crates MUST NOT use `anyhow` in public APIs |
| async traits | `async-trait` | present, keep internal or remove before API stabilization | not a target public API dependency; dyn traits use explicit boxed future/stream returns; internal non-dyn traits may use native async fn |
| HTTP/SSE | `reqwest` with `rustls-tls` | Phase 1, narrow features | provider streaming without OpenSSL; use `default-features = false` and enable only required HTTP/JSON/stream features |
| SSE parsing | hand-written line parser or `eventsource-stream` | Phase 1 | `reqwest-eventsource` is excluded (does not support POST); Anthropic uses POST-based streaming |
| streams | `futures-core`, internal stream helpers as needed | Phase 1 | public stream APIs should expose `futures-core::Stream`; keep helpers such as `futures-util` internal |
| cancellation | `tokio-util` | Phase 1 | cooperative cancellation |
| CLI | `clap` | Phase 1 | stable options and completions |
| config | `toml` | Phase 1 | human-editable config |
| TUI | `ratatui`, `crossterm` | Phase 1 | cross-platform terminal UI |
| schema | `schemars`, `jsonschema` | Phase 1, tool boundary first | typed tool schemas plus runtime validation at the model/tool boundary; avoid broad protocol validation until schemas stabilize; see §5.6 for draft compatibility |
| IDs/time | `uuid`, `time` | Phase 1 | session IDs and timestamps without `chrono`'s extra surface |
| file search | `ignore`, `globset`, `regex` | Phase 1 | gitignore-aware glob and grep behavior |
| tracing | `tracing`, `tracing-subscriber` | Phase 1/2 | observability |
| markdown/code | `pulldown-cmark`, optional `syntect` later | Phase 1/2 | basic markdown first; syntax highlighting must be optional or later so it does not threaten binary size targets |
| diff | `similar` | Phase 2 | patch visualization; do not add before a real diff view ships |

### 5.6 JSON Schema Draft Compatibility

Anthropic's Messages API accepts tool `input_schema` as a JSON Schema object
with a top-level `type: "object"` constraint. API validation errors indicate a
draft-2020-12-compatible validator, while `schemars` 0.8 generates draft-07 by
default.

For Phase 1 tool schemas (simple object + properties + required), draft-07
output should stay within the common JSON Schema subset accepted by Anthropic.
Complex schemas using features that diverged between drafts (array `items` vs
`prefixItems`, `definitions` vs `$defs`, conditional keywords) MAY be rejected.

Requirements:

- Phase 1 MUST include local fixture tests for generated built-in tool schemas,
  including validation of representative model arguments before deserialization.
- Phase 1 SHOULD include an ignored, environment-gated live Anthropic schema
  acceptance test, but default CI MUST NOT require paid credentials or network
  access.
- If incompatibilities surface, a schema post-processing step SHOULD normalize
  draft-07 output to the accepted provider subset (for example, rename
  `definitions` to `$defs` when needed).
- `schemars` 1.0 (when stable) MAY resolve this natively; until then, treat this
  as a known risk with a tested mitigation path.

## 6. Architecture

### 6.1 Layers

```text
opi-coding-agent
  CLI, built-in tools, config, prompts, tool selection, app-level session UX

CodingHarness / Harness
  session persistence, compaction, app hooks, model/thinking state, queues

Agent
  stateful runtime wrapper, subscriptions, cancellation, prompt/continue API

agent_loop
  pure LLM -> tool -> LLM loop, no persistence or UI policy

opi-ai Provider
  provider HTTP, SSE parsing, model metadata, provider-facing messages
```

`agent_loop` MUST be testable with mock providers and mock tools without disk or terminal state. `Agent` adds state, cancellation, queues, and event subscription. `Harness` composes sessions, compaction, and app hooks.

### 6.2 Harness Boundary

Pi 0.80.2 makes `AgentHarness` a central reusable orchestration layer above the
low-level loop. It owns session persistence, runtime configuration, resource
resolution, operation locking, turn snapshots, save points, and
extension-facing mutation semantics. Opi should preserve that ownership
direction in Rust without copying TypeScript APIs.

- `opi-agent` SHOULD own generic harness primitives needed by non-CLI consumers:
  phase guards, turn snapshots, save points, ordered pending session writes,
  session repo/facade traits, and generic resources/system-prompt hooks.
- `opi-coding-agent` SHOULD own coding-specific behavior: built-in file tools,
  project context, package/resource discovery, tool allowlists, CLI config,
  interactive commands, and app-level session commands.
- `CodingHarness` SHOULD be a product wrapper over generic runtime seams, not
  the only owner of reusable orchestration semantics.
- If a feature is required by both library consumers and the CLI, it belongs in
  `opi-agent`; otherwise it stays in `opi-coding-agent`.

### 6.3 Runtime Flow

```text
user input
  -> CLI parses mode and config
  -> CodingHarness loads or creates session
  -> system prompt is built from base prompt, tools, project context, summaries
  -> Agent receives prompt, steer, follow-up, or continue request
  -> agent_loop transforms AgentMessage to provider Message
  -> provider streams assistant events
  -> agent emits message updates
  -> tool calls are validated and executed
  -> tool result messages are appended in assistant source order
  -> should_stop_after_turn runs
  -> steering queue is polled
  -> follow-up queue is polled only if the agent would otherwise stop
  -> session entries are appended
  -> subscribers settle after agent_end
```

### 6.4 Boundary Rules

- Provider adapters MUST NOT execute tools.
- Tools MUST NOT call providers directly unless the tool is explicitly an integration.
- TUI components MUST consume events and snapshots; they MUST NOT own loop policy.
- Session storage MUST NOT be required for `agent_loop` tests.
- CLI shortcuts MUST NOT leak into `opi-agent` unless they describe reusable runtime behavior.

## 7. Protocols and Data Models

Opi has four related protocols. They MUST stay distinct.

| Protocol | Owner | Purpose |
|---|---|---|
| Provider stream events | `opi-ai` | normalize provider chunks into assistant deltas |
| Agent events | `opi-agent` | loop/message/tool lifecycle for UI and tests |
| Agent session events | harness / `opi-coding-agent` | queues, compaction, retry, session metadata |
| Session entries | storage | persisted records used to rebuild context |

### 7.1 Provider-Facing Messages

```rust
#[non_exhaustive]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
}

pub struct UserMessage {
    pub content: Vec<InputContent>,
    pub timestamp_ms: i64,
}

pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub api: ApiKind,
    pub provider: String,
    pub model: String,
    pub response_model: Option<String>,
    pub response_id: Option<String>,
    pub usage: Usage,
    pub stop_reason: StopReason,
    pub error_message: Option<String>,
    pub timestamp_ms: i64,
}

pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<OutputContent>,
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    pub truncated: bool,
    pub timestamp_ms: i64,
}
```

Stop reasons SHOULD stay close to pi: `stop`, `length`, `tool_use`, `error`, `aborted`.

Image content is structural at the opi protocol boundary. `InputContent::Image`
is forwarded only to models whose metadata advertises image support; known
text-only models MUST fail before the provider network call. CLI image
attachments MUST enforce a configured byte limit before reading the whole file.

`OutputContent::Image` round-trips through tool results, session JSONL, and JSON
mode as structured data. Provider request bodies MAY coerce image tool results
to a textual placeholder such as `[image: image/png]` because current provider
tool-result roles do not consistently accept binary image payloads. That
coercion is a provider-protocol limitation and MUST NOT be described as a loss
in session storage or JSON mode.

### 7.2 Agent Messages

```rust
#[non_exhaustive]
pub enum AgentMessage {
    Llm(opi_ai::Message),
    CompactionSummary(CompactionSummaryMessage),
    BranchSummary(BranchSummaryMessage),
    Custom(CustomAgentMessage),
}
```

Before each provider call:

1. `transform_context` works at `AgentMessage` level.
2. `convert_to_llm` converts to `Vec<opi_ai::Message>` and filters session/UI-only messages.

Unknown custom messages MUST NOT panic the runtime.

### 7.3 Provider Stream Events

```rust
#[non_exhaustive]
pub enum AssistantStreamEvent {
    Start { partial: AssistantMessage },
    TextStart { content_index: usize, partial: AssistantMessage },
    TextDelta { content_index: usize, delta: String, partial: AssistantMessage },
    TextEnd { content_index: usize, content: String, partial: AssistantMessage },
    ThinkingStart { content_index: usize, partial: AssistantMessage },
    ThinkingDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ThinkingEnd { content_index: usize, content: String, partial: AssistantMessage },
    ToolCallStart { content_index: usize, partial: AssistantMessage },
    ToolCallDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ToolCallEnd { content_index: usize, tool_call: ToolCall, partial: AssistantMessage },
    Done { reason: StopReason, message: AssistantMessage },
    Error { reason: StopReason, message: AssistantMessage },
}
```

Every provider stream MUST emit `Start` before deltas and terminate with exactly one `Done` or `Error`. Once a request has started, request/model/runtime failures SHOULD become `Error` events with final assistant messages instead of out-of-band failures.

### 7.4 Agent Events

```rust
#[non_exhaustive]
pub enum AgentEvent {
    AgentStart,
    AgentEnd { messages: Vec<AgentMessage> },
    TurnStart,
    TurnEnd { message: AgentMessage, tool_results: Vec<opi_ai::ToolResultMessage> },
    MessageStart { message: AgentMessage },
    MessageUpdate { message: AgentMessage, assistant_event: AssistantStreamEvent },
    MessageEnd { message: AgentMessage },
    ToolExecutionStart { tool_call_id: String, tool_name: String, args: serde_json::Value },
    ToolExecutionUpdate { tool_call_id: String, tool_name: String, args: serde_json::Value, partial_result: serde_json::Value },
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        details: Option<serde_json::Value>,
        is_error: bool,
        truncated: bool,
        diagnostics: Vec<ToolDiagnostic>,
    },
}
```

`MessageUpdate` is assistant-only. `AgentEnd` means no more loop events will be emitted, but awaited subscribers MAY still be settling.

Public agent events and persisted session messages MUST redact command/path/env/stdout/stderr-like fields in tool args, details, partial results, and diagnostics before exposure. Public assistant message snapshots and stream events also redact tool-call arguments and tool-call deltas, because those values are model-supplied tool args at the same boundary. Provider adapters consume tool result `content` plus provider-specific `is_error` handling; `details`, `diagnostics`, and `truncated` are local event/session metadata and are not sent as provider tool-result payloads.

### 7.5 Session Events

```rust
#[non_exhaustive]
pub enum AgentSessionEvent {
    Agent(AgentEvent),
    QueueUpdate { steering: Vec<String>, follow_up: Vec<String> },
    CompactionStart { reason: CompactionReason },
    CompactionEnd { reason: CompactionReason, result: Option<CompactionResult>, aborted: bool, will_retry: bool, error_message: Option<String> },
    AutoRetryStart { attempt: u32, max_attempts: u32, delay_ms: u64, error_message: String },
    AutoRetryEnd { success: bool, attempt: u32, final_error: Option<String> },
    SessionInfoChanged { session_id: String, name: Option<String> },
    ThinkingLevelChanged { level: ThinkingLevel },
}
```

When Phase 2 JSON mode is implemented, `--json` emits one JSON object per line.
The event protocol MUST include a schema version before downstream tooling
treats it as stable.

### 7.6 Queues

```rust
pub enum QueueMode {
    All,
    OneAtATime,
}
```

Steering messages are delivered after the current assistant turn and its tool calls complete, before the next provider request. Follow-up messages are delivered only when the agent has no tool calls and no steering messages and would otherwise stop. If `should_stop_after_turn` returns true, the loop exits before polling either queue.

## 8. Crate Specifications

### 8.1 `opi-ai`

`opi-ai` owns provider-facing message types, model metadata, provider registry,
credential helpers, and streaming adapters. The Phase 10 target is to deepen
this into a provider collection/auth seam inspired by `pi-ai` `Models`: provider
and model lookup, optional refresh, provider-owned auth resolution,
stream/complete dispatch, and compatibility metadata should live in `opi-ai`.
CLI config, env loading, package inputs, and product defaults remain
construction inputs owned by `opi-coding-agent`.

```rust
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn models(&self) -> &[ModelInfo];
    fn stream(&self, request: Request) -> EventStream;
    fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>>;
}

pub type EventStream =
    Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent, ProviderError>> + Send>>;
```

`stream` returns a stream handle. Cancellation is propagated through `Request::cancel` or an equivalent token. Dropping the stream SHOULD cancel the underlying HTTP request.

```rust
pub struct Request {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub thinking: ThinkingConfig,
    pub stop_sequences: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub cancel: CancellationToken,
    pub timeout: Option<std::time::Duration>,
    pub extra_headers: Vec<(String, String)>,
    pub cache_retention: CacheRetention,
    pub session_id: Option<String>,
}
```

The provider-reported usage and cost lines are:

```rust
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub cache_write_1h_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub reported: bool,
}

pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_write_cost: f64,
}
```

The optional child subsets preserve absent versus explicitly reported zero.
The weighted one-hour cache-write subset is folded into `cache_write_cost`,
reasoning remains inside `output_cost`, and totals count each parent bucket
once.

Provider priority:

| Provider | API style | Phase | Reason |
|---|---|---:|---|
| Anthropic | Messages SSE | 1 | MVP target and pi's default model family |
| OpenAI-compatible chat | SSE | 2 | broad compatibility across OpenAI-style services |
| OpenRouter | OpenAI-compatible router | 2 | fast model coverage expansion and routing diagnostics |
| OpenAI Responses | SSE | 2 | separate event mapping |
| Google Gemini | streaming generateContent | 2 | major non-OpenAI family |
| Mistral | chat SSE | 2 | provider matrix expansion |
| AWS Bedrock | response stream / SigV4 | 3 | enterprise auth complexity |
| Azure OpenAI | OpenAI-compatible | 3 | deployment-name differences |
| Google Vertex | OAuth/service account | 3 | enterprise auth complexity |

Provider expansion policy:

- Add a first-class provider only when the wire format, event model, or authentication model is materially different.
- Use configured OpenAI-compatible profiles when a provider can be expressed with base URL, API key env var, model metadata, and compatibility flags.
- Use `ProviderRegistry` model overrides for deployment-specific or fine-tuned model metadata.
- Use extension/SDK provider registration for embedders and external adapters.

OAuth remains a separate product decision. Anthropic OAuth, OpenAI Codex OAuth, and GitHub Copilot OAuth require login commands, credential storage, refresh behavior, and user-facing revocation semantics; they MUST NOT be silently added as a side effect of provider profile expansion.

Image generation is also a separate product decision. `pi-ai` 0.80.2 mirrors
chat-side provider collection/auth for image generation, but `opi` should not
add that surface before chat provider collection/auth semantics are stable.

Credential precedence:

1. explicit CLI/config override;
2. provider-specific environment variable;
3. local auth storage when implemented;
4. ambient cloud credential chain where implemented.

Bedrock credential resolution is local/offline: explicit config, AWS
environment variables, `AWS_PROFILE`, `AWS_SHARED_CREDENTIALS_FILE`,
`AWS_CONFIG_FILE`, shared credentials/config profiles, region config, and
`credential_process`. It does not perform IMDS, ECS task metadata, SSO, or
web-identity network flows.

Vertex credential resolution is intentionally scoped to a static OAuth access
token supplied through the configured environment variable, plus project and
location config. Service-account JSON parsing and Application Default
Credentials token minting are outside the current Phase 3 contract.

Secrets MUST NOT be logged, persisted in sessions, or included in diagnostics.

### 8.2 `opi-agent`

`opi-agent` is usable without the `opi` binary.

```rust
pub trait Tool: Send + Sync {
    fn definition(&self) -> opi_ai::ToolDef;

    fn execute(
        &self,
        call_id: &str,
        arguments: serde_json::Value,
        signal: CancellationToken,
        on_update: Option<UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>>;

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

pub enum ExecutionMode {
    Sequential,
    Parallel,
}

pub struct ToolDiagnostic {
    pub code: String,
    pub message: String,
    pub context: serde_json::Value,
}

pub struct ToolResult {
    pub content: Vec<opi_ai::OutputContent>,
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    pub terminate: bool,
    pub truncated: bool,
    pub diagnostics: Vec<ToolDiagnostic>,
}
```

Built-in tools SHOULD define typed Rust argument structs deriving `Deserialize` and `schemars::JsonSchema`. `ToolDef` exposes the generated JSON Schema to providers, while dynamic input from the model is validated with `jsonschema` before deserialization. `serde_json::Value` is acceptable at protocol boundaries and for diagnostics, but tool business logic should not remain Value-driven.

Argument validation happens after `ToolExecutionStart` and before `before_tool_call`. Validation failure becomes an error tool result.

`ToolResult.details` is success/audit metadata. Most built-in failure results SHOULD keep `details: None`; structured failure metadata lives in `diagnostics[].context` and is redacted at public event/session boundaries. Operation-style failures MAY keep stable operation metadata in `details`; bash operation failures use this exception for fields such as `command`, `exit_code`, `cancelled`, `timed_out`, and `truncated`, while public diagnostic context omits the raw command. Large or partial results set `truncated` consistently on `ToolResult`, `ToolResultMessage`, and `ToolExecutionEnd`.

Execution rules:

- global sequential means all calls run sequentially;
- global parallel means calls run concurrently unless any target tool is sequential;
- if any tool in a batch is sequential, the entire batch runs sequentially;
- persisted tool-result messages are ordered by assistant source order.

Hook surface:

```rust
pub trait AgentHooks: Send + Sync {
    fn transform_context(&self, messages: Vec<AgentMessage>, signal: CancellationToken)
        -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>>;

    fn convert_to_llm(&self, messages: &[AgentMessage])
        -> Result<Vec<opi_ai::Message>, AgentError>;

    fn before_tool_call(&self, ctx: BeforeToolCallContext)
        -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>>;

    fn after_tool_call(&self, ctx: AfterToolCallContext)
        -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>>;

    fn should_stop_after_turn(&self, ctx: ShouldStopAfterTurnContext)
        -> Pin<Box<dyn Future<Output = bool> + Send>>;

    fn prepare_next_turn(&self, ctx: PrepareNextTurnContext)
        -> Pin<Box<dyn Future<Output = Option<AgentLoopTurnUpdate>> + Send>>;
}
```

`after_tool_call` uses field replacement semantics and MUST NOT deep-merge `content` or `details`.

The low-level loop:

```rust
pub async fn agent_loop(
    context: AgentLoopContext,
    config: AgentLoopConfig,
    hooks: &dyn AgentHooks,
    events: AgentEventSink,
    cancel: CancellationToken,
) -> Result<Vec<AgentMessage>, AgentError>;
```

`Agent` wraps the loop with state, prompt/continue methods, abort, steering and follow-up queues, subscriber management, and idle settlement. Continuing requires the last context message to be user or tool result.

`opi_agent::Transport` was removed in Phase 4. RPC/proxy surfaces now live in `opi-coding-agent::rpc`, `opi-agent::sdk`, and `opi-agent::streaming_proxy`.

### 8.3 `opi-tui`

Phase 1 components:

| Component | Phase | Purpose |
|---|---:|---|
| `MessageList` | 1 | streaming conversation display |
| `InputEditor` | 1 | multi-line prompt input |
| `StatusBar` | 1 | model, state, token/cost summary placeholder |
| `ToolCallView` | 1 | tool call arguments and status |
| `MarkdownView` | 1 | basic markdown text |
| `CodeBlock` | 1/2 | syntax-highlighted code blocks |
| `DiffView` | 2 | edit and patch visualization |
| `SelectList` | 3 | session/model picker |

The TUI target is user-visible behavior, not renderer compatibility with pi: low flicker, responsive streaming, resize safety, Windows compatibility, and graceful degradation on small terminals.

Phase 1 should remain a minimal usable TUI: streaming messages, prompt input, status, and tool-call visibility. Themes, fuzzy pickers, rich diff views, and syntax highlighting beyond basic fenced-code presentation belong in later phases or optional features.

### 8.4 `opi-coding-agent`

The binary owns CLI parsing, config loading, provider registry construction, built-in tools, system prompt construction, session UX, tool selection, and runtime modes.

| Tool | Mode | Phase | Scope |
|---|---|---:|---|
| `read` | parallel | 1 | read file content with optional line range |
| `write` | sequential | 1 | create or replace file |
| `edit` | sequential | 1 | exact string replacement or structured patch |
| `bash` | sequential | 1 | subprocess command with timeout and streamed output |
| `glob` | parallel | 1 | additional gitignore-aware file discovery by glob pattern; not required by the pi-derived core workflow |
| `grep` | parallel | 1 | gitignore-aware regex search over file contents |
| `find` | parallel | 3 | pi-compatible file discovery alias with gitignore-aware behavior |
| `ls` | parallel | 3 | pi-compatible directory listing with bounded output |

Interactive mode SHOULD default to the pi coding tool set: `read`, `write`, `edit`, and `bash`. Non-interactive mode SHOULD default to a conservative read-only tool set: `read`, `grep`, `find`, `ls`, and `glob`. `glob` remains an additional read-only search convenience, but the core non-interactive workflow should be expressible without it. Non-interactive mutating tools require explicit opt-in through `--allow-mutating` or `defaults.allow_mutating_tools = true`, which is especially important for unattended automation and edge devices where the process may run close to deployment, storage, or device-control scripts.

Tool visibility and tool execution policy MUST agree. Opi should not advertise `write`, `edit`, or `bash` to the model in non-interactive mode unless those tools can execute under the resolved policy.

File tools MUST use explicit path policy. `write` and `edit` remain workspace-only by default. Interactive `read` MAY resolve absolute paths and workspace-external paths for pi-style usability. Non-interactive file tools remain workspace-only by default.

Interactive confirmation MAY exist in Phase 4+ as an extension-mediated safeguard, but reusable permission profiles and permission popups are not core behavior inherited from pi; richer gates should be built via tool allowlists, hooks, extensions, packages, containers, or external wrappers.

Tool selection flags SHOULD follow pi's shape before stable CLI claims:
`--tools <list>` for an allowlist, `--no-tools` to disable all tools, and
`--no-builtin-tools` once extension/custom tools exist.

CLI target:

```text
opi [OPTIONS] [PROMPT]

Options:
  -m, --model <SPEC>       Model, e.g. anthropic:claude-sonnet-4
  -c, --config <PATH>      Config file path
  -s, --system <PATH>      System prompt file
      --list-models        List available models
      --fork <SESSION_ID>  Fork a stored session into a new parented session
      --non-interactive    Single prompt mode
  -v, --verbose            Enable debug tracing
  -V, --version            Print version
  -h, --help               Print help
```

Phase 2 adds `--resume`, `--list-sessions`, and `--json` after session storage
and JSON event schemas have contract tests.
The current workspace also exposes `--fork <SESSION_ID>` for creating a new
session from the source session's active branch without rewriting the source
JSONL file.

Prompt layers:

1. base coding-agent instructions;
2. tool descriptions from `ToolDef`;
3. user system prompt file;
4. project context files, starting Phase 3: `AGENTS.md` and `CLAUDE.md` from global config and cwd ancestors, matching pi;
5. compaction summaries, starting Phase 2;
6. skills/prompt fragments, starting Phase 4.

`OPI.md` is not the default context-file name because pi and the broader coding-agent ecosystem already use `AGENTS.md` and `CLAUDE.md`. A future compatibility alias MAY be added, but it must not replace those names.

## 9. Configuration and Storage

### 9.1 Config

```toml
[defaults]
model = "anthropic:claude-sonnet-4"
max_iterations = 50
tool_timeout_ms = 30000
theme = "default"

[thinking]
enabled = true
budget_tokens = 10000

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.openai_compatible.localai]
api_key_env = "LOCALAI_API_KEY"
base_url = "https://localai.example.com"
max_tokens_field = "max_completion_tokens"

[[providers.openai_compatible.localai.models]]
id = "local-model"
display_name = "Local Model"
context_window = 128000
max_output_tokens = 4096
supports_images = true
supports_streaming = true
supports_thinking = false

[keybindings]
submit = "enter"
abort = "ctrl+c"
new_line = "shift+enter"
```

Malformed config files SHOULD fail clearly. Silent fallback is allowed for missing optional files, not invalid user config.

### 9.1.1 Configuration Precedence

Configuration values are resolved in the following priority order (highest wins):

1. CLI arguments (`--model`, `--config`, etc.)
2. Environment variables (`ANTHROPIC_API_KEY`, `OPI_MODEL`, etc.)
3. Project config file (`.opi/config.toml` in workspace root, when implemented)
4. User config file (`~/.config/opi/config.toml`)
5. Built-in defaults

Phase 1 implements this with clap (CLI args) + `std::env` (env vars) + `toml` deserialization (config file) + struct defaults. No configuration framework (figment, config-rs) is required for Phase 1. A framework MAY be introduced in later phases if configuration source complexity grows beyond what manual merging handles cleanly.

Phase 1 config loading only needs defaults, provider credentials, model
selection, timeouts, theme selection, and high-risk tool policy. Compaction,
session, and advanced keybinding settings MAY be accepted as reserved fields,
but they must not imply those Phase 2 features are active.

Phase 2 MAY add a `[compaction]` table with fields such as `enabled`,
`reserve_tokens`, and `keep_recent_tokens` after session persistence exists.

### 9.2 Directory Layout

```text
~/.config/opi/config.toml
~/.config/opi/themes/
~/.local/share/opi/sessions/
```

Windows SHOULD use `%APPDATA%\opi\` for config-like data and `%LOCALAPPDATA%\opi\` for cache-like data.

### 9.3 Session Format

The opi session format is a **Rust-native** append-only JSONL tree. It is an
independent format rather than a copy of pi's session format. The current v1
format represents a selected subset of pi's session concepts: append-only
history, parent-linked branching, compaction summaries, active leaf pointers,
and persisted extension state implemented against opi's Rust crates. It does
**not** promise pi session v3 file read/write compatibility (see 9.4).

Session persistence starts in Phase 2, not Phase 1. The target format is
append-only, versioned JSONL. The first line is a header:

```json
{"type":"session","version":1,"id":"018f...","timestamp":"2026-05-20T12:00:00Z","cwd":"/repo","parent_session":null}
```

Subsequent lines are tree entries:

```json
{"type":"message","id":"a1b2c3d4","parent_id":null,"timestamp":"2026-05-20T12:00:01Z","message":{"role":"user","content":[{"type":"text","text":"Read src/main.rs"}]}}
{"type":"message","id":"b2c3d4e5","parent_id":"a1b2c3d4","timestamp":"2026-05-20T12:00:02Z","message":{"role":"assistant","content":[{"type":"text","text":"I'll inspect it."}],"stop_reason":"tool_use"}}
{"type":"compaction","id":"c3d4e5f6","parent_id":"b2c3d4e5","timestamp":"2026-05-20T13:00:00Z","summary":"The session inspected CLI scaffolding.","first_kept_entry_id":"b2c3d4e5","tokens_before":45000,"tokens_after":8000}
```

Session entry types are separated into the v1 surface and the Phase 13
additive entries. Phase 13 keeps header **version 1**; the new metadata/context
entries are **additive** and opi reads them on v1 files. Phase 13 requires
no automatic migration as a precondition for normal resume. v1 sessions remain
readable and resumable.

| Type | Status | Purpose | LLM context |
|---|---|---|---|
| `message` | v1 | user, assistant, or tool result message | yes |
| `compaction` | v1 | summary plus first kept entry | yes |
| `leaf` | v1 | current branch pointer | no |
| `extension_state` | v1 | persisted extension state | no |
| `session_info` | Phase 13 (additive on v1) | session name and metadata | no |
| `model_change` | Phase 13 (additive on v1) | selected provider/model changed | no |
| `thinking_level_change` | Phase 13 (additive on v1) | thinking level changed | no |
| `label` | Phase 13 (additive on v1) | user marker or bookmark | no |
| `branch_summary` | Phase 13 (additive on v1) | parent branch summary used by tree/context reconstruction | yes |
| `custom_message` | deferred | extension-provided context message | configurable |

Phase 13 success criteria:

- new writes use the additive Phase 13 entries on the v1 header when those
  entries are needed (header version stays 1);
- v1 sessions remain readable and resumable;
- branch reconstruction, `--list-sessions`, resume, fork, clone, and tree views
  have deterministic behavior when labels, names, model changes, thinking
  changes, branch summaries, and custom messages are present;
- contract tests cover v1 fixtures, additive Phase 13 entry fixtures, truncated
  final lines, corrupt middle entries, unknown future entry types, active leaf
  reconstruction, and branch-summary context reconstruction.

Crash recovery MAY ignore an incomplete final line. Corrupt middle entries
(malformed JSON or missing required fields) SHOULD be reported as diagnostics
and automatic skipping of middle entries should require explicit recovery
mode. Unknown future entry types — well-formed JSON whose `type` field is not
recognized — are skipped and counted, but are not preserved across read+rewrite;
the reader splits unknown future entries from corrupt middle entries so the two
cases are distinguishable in diagnostics.

Session files are sensitive: they contain prompts, tool outputs, model and
thinking selections, and potentially leaked secrets. Local export
(`opi --export-session <id-or-path> --format markdown|json --output <file>`)
renders the active branch or full tree, applies Phase 7 redaction modes, and
may omit tool output or thinking content via include/exclude flags. Export is
local and user-controlled; it writes only the requested output file and leaves
the source session unchanged on success or failure. Interactive `/export` and
any web/share/session-publishing path are deferred to later product polish.

Session fork commands create a new session file. The new header's
`parent_session` field points at the source session ID, and the copied entries
come from the same active-branch reconstruction path used by resume. Forking
MUST NOT rewrite the source session file.

Same-file branch creation uses the append-only tree model: runtime message
entries use the current active tip as `parent_id`, compaction entries are linked
under the previous active tip, and completed turns/compactions append a `leaf`
pointer to mark the active branch. Selecting a prior branch tip and continuing
therefore creates a new sibling path without rewriting previous entries.

### 9.4 Why Not pi Session v3

The opi session JSONL is a Rust-native format that represents selected pi
session concepts **without** promising pi session v3 file compatibility. Opi
keeps pi's append-only history, branch, compaction, and session-metadata ideas
but not its file format because pi stores TypeScript-specific extension data,
opi has independent config/package plans, and accidental partial compatibility
would be misleading. Concepts opi intentionally does not carry over include
pi's TypeScript-specific extension entries, its on-disk encoding, and any
guarantee that a pi v3 session file can be opened, resumed, or appended to by
opi. A future migration command MAY translate pi v3 sessions into opi sessions,
but until then the two formats are not interchangeable.

### 9.5 Compaction

Compaction starts in Phase 2 after session storage exists.

Triggers:

- manual;
- threshold-based;
- context overflow recovery.

Results MUST record summary, `first_kept_entry_id`, tokens before/after, reason, and whether the summary came from core or a hook/extension. If compaction fails during overflow recovery, the agent MUST surface a visible error rather than silently dropping history.

## 10. CLI and Runtime Surfaces

Interactive mode is the default when stdin is a TTY. It owns terminal initialization, rendering, input editing, cancellation keys, tool-selection UX, and any extension-provided prompts.

Non-interactive mode takes one prompt from argv or stdin, streams assistant text to stdout, writes diagnostics to stderr, and exits with explicit status.

Suggested exit codes:

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | general runtime failure |
| 2 | invalid CLI usage or config |
| 3 | authentication failure |
| 4 | provider/network failure after retries |
| 5 | unrecovered tool failure |
| 130 | interrupted by user |

JSON mode is Phase 2 scope. It emits one `AgentSessionEvent` JSON object per line to stdout after the event schema has contract tests. Human-readable logs go to stderr. Phase 2 JSON mode SHOULD stay close to pi's event model but MUST include an opi schema version.

RPC mode is an early Phase 4 extensibility surface. It should use strict JSONL framing: one command per line on stdin, correlated responses by optional `id`, and async events on stdout. RPC and SDK composition should precede dynamic plugin runtimes because they match pi's process-integration model without expanding core policy. Provider breadth beyond the Phase 3 set should primarily arrive through the Phase 4 SDK, extension, and model registry path instead of adding every provider to core.

RPC `session_info` always returns `model` and `resources`. When no active
session exists, session-derived fields (`session_id`, `name`, `labels`,
`active_branch`, `thinking`, `entry_count`, `branch_summary`, `branches`,
`tree_recovery`) are absent. With an active session, `name` is `null` when no
`session_info` entry applies, `labels` is an empty array when no labels apply,
`active_branch` is absent until a content tip exists, `branch_summary` is absent
when the active branch has no summary, `tree_recovery` is absent on a clean
read, and `tree_read_error` appears only when the session file cannot be read
for tree reconstruction.

The default extension execution strategy is explicit registration, not dynamic Rust library loading. Embedders can register in-process Rust extensions through `ExtensionRegistry`; external packages should expose executable behavior through process/RPC adapters that translate package commands into SDK commands such as `extension_command`. Package/resource discovery remains metadata and resource composition unless an adapter explicitly registers executable code. The core binary MUST NOT `dlopen` arbitrary Rust crates by default, and it MUST NOT require Node/`jiti` to preserve pi's TypeScript extension mechanism.

### 10.1 Package CLI

Phase 5 adds an `opi package` subcommand group that runs before provider construction. This is a Rust-native package and process-adapter MVP, not pi package ecosystem parity:

| Command | Purpose |
|---|---|
| `opi package add <source>` | Install a package from a local directory or git source |
| `opi package remove <name>` | Uninstall a package |
| `opi package list` | List installed packages (supports `--json`) |
| `opi package doctor` | Diagnose package issues (supports `--json`) |

Packages are recorded in the global user config directory (`packages.toml` and `package-lock.toml`) or the project `.opi/` directory (`.opi/packages.toml` and `.opi/package-lock.toml`). Git package checkouts are cached under the selected scope's `package-cache/`. The lock records source path, optional git commit, cache path, and manifest hash.

`opi package add` validates the package manifest, records the declaration, and writes a lock entry. Runtime startup reads installed declarations and lock state, resolves valid packages without requiring `config.packages.paths`, starts valid adapter packages, and reports adapter startup diagnostics. `opi package doctor` validates source availability, lock consistency, manifest V2, resource containment, opi version constraints, and adapter command resolution.

Packages are trusted code. Installing a package can run adapter child processes with the same OS privileges as `opi`; Phase 5 package code is not sandboxed, and package permission declarations are not enforced by the package manager.

Phase 5 support levels:

| Capability | Status | Notes |
|---|---|---|
| local package declarations | supported | package source may be a local directory |
| git package declarations | supported | lock records commit/cache metadata when available |
| `process-jsonl` adapters | experimental | `opi-extension-jsonl-v1` is an honest 0.x protocol, not a stable 1.0 contract |
| adapter tools, commands, hooks, state, cancellation | experimental | bridged through the existing extension interface |
| npm package install | not supported | pi npm package compatibility is not claimed |
| marketplace behavior | does not exist in Phase 5 | no registry search, ratings, publishing, or marketplace update policy |
| package update/config/enable/disable | not supported in Phase 5 | may be future package-manager work |
| TypeScript extension live reload | intentionally unsupported | opi does not preserve pi's `jiti` extension ABI |
| provider stream interception through adapters | does not exist in Phase 5 | provider breadth should use existing provider modules or explicit registry/profile work |
| custom terminal UI adapter rendering | does not exist in Phase 5 | TUI extension UI requires a separate reviewed design |
| package permission policy enforcement | does not exist in Phase 5 | declarations may be metadata, but the package manager does not enforce them |

### 10.2 Process Adapters

Packages with an `[adapter]` section in their manifest run as child process adapters. The Phase 5 MVP supports the `process-jsonl` adapter kind with the `opi-extension-jsonl-v1` protocol. The behavior documented here is the **honest 0.x protocol**: it records what the implementation observes today, not a stable 1.0 contract, and may change between minor versions.

Protocol and kind are validated as a **startup-time manifest gate**, not a wire handshake. At runtime startup, `start_adapters_from_packages` only starts adapters whose manifest declares `protocol = "opi-extension-jsonl-v1"` and `kind = "process-jsonl"`. A package declaring any other value is skipped with a diagnostic that names the expected and actual protocol or kind; its static package resources still load. The `initialize` message carries the host protocol string for information, but the `capabilities` response carries no version field, so the host performs no version comparison over the wire.

Adapters are started in a **deterministic order**: ascending by `(layer_precedence, package name)`, which makes tool and hook composition reproducible across sessions.

Adapter lifecycle:

1. The harness starts the adapter child process with the configured command and args.
2. The harness sends an `initialize` message; the adapter responds with `capabilities` (tools, commands, hooks, model overrides).
3. At runtime, the harness bridges adapter capabilities into existing `Extension` trait methods: `on_command`, `on_before_tool_call`, `on_after_tool_call`, `on_event`, `serialize_state`, `restore_state`. Hooks are only dispatched to adapters that declared them in `capabilities.hooks`.
4. Adapter tools are merged into the tool set; adapter hooks are composed with `CodingAgentHooks` via `ExtensionRegistry::wrap_hooks`.
5. Ordinary registry teardown is best-effort kill-only and does not guarantee a protocol `shutdown` handshake; explicit `AdapterHost::shutdown` is the graceful protocol path.

Request/response correlation: the host owns request id generation. Each request carries an `id`; the adapter returns the same `id` on its response. Responses are matched to in-flight requests by `id`, and unsolicited messages (for example an `error` with no `id`) are ignored.

Timeouts and cancellation: the initialize handshake has a startup timeout, and each request has a per-request timeout. If the handshake times out or the adapter exits during startup, the adapter is not registered and a diagnostic is produced. If an individual request times out, it fails with a timeout error and the host remains usable. `cancel` is best-effort and carries no response; the host still enforces the local timeout. A `before_tool_call` hook that times out fails closed (the tool is blocked); an `after_tool_call` hook that times out fails open (the result stands).

Events and state: `event` is fire-and-forget; if the adapter's stdin is backpressured, the event is dropped and a diagnostic is recorded. `state_serialize` and `state_restore` round-trip adapter state for session persistence.

Shutdown and crashes: Explicit `AdapterHost::shutdown` sends a best-effort `shutdown` message, waits through a grace timeout, and kills the child if it has not exited. Ordinary registry teardown is best-effort kill-only because process adapters are held through shared registry references. If the adapter process exits after a successful handshake, pending requests fail as unavailable and the runtime adapter becomes degraded.

Adapter protocol messages: `initialize`, `capabilities`, `tool_call`, `command`, `hook`, `event`, `state_serialize`, `state_restore`, `cancel`, `shutdown`. All messages are single-line JSON over stdin/stdout with correlated `id` fields.

Adapter commands that are not routed to a registered extension are available through the RPC `extension_command` dispatch.

## 11. Cross-Cutting Runtime Concerns

### 11.1 Error Handling

| Layer | Approach |
|---|---|
| `opi-ai` | typed `ProviderError` plus stream `Error` terminal events |
| `opi-agent` | typed `AgentError`, `ToolError`, `SessionError` |
| `opi-tui` | typed terminal/render errors |
| `opi-coding-agent` | `anyhow::Result` at top level for error aggregation; mapped exit codes; library errors converted via `From` impls |

Library crates (`opi-ai`, `opi-agent`, `opi-tui`) MUST use `thiserror` for typed errors and MUST NOT expose `anyhow` in public APIs. `opi-coding-agent` MAY use `anyhow` (or `eyre`) for top-level error aggregation where typed errors add no value to the end user.

Library crates MUST avoid `unwrap` and `expect` except in tests or provably safe static initialization.

### 11.2 Cancellation and Backpressure

Cancellation uses `tokio_util::sync::CancellationToken` organized in a three-layer tree:

```text
session_token (program exit / repeated Ctrl+C)
  └── operation_token (current agent turn / first Ctrl+C)
        └── tool_token (individual tool execution / tool timeout)
```

Cancellation semantics:

- First Ctrl+C cancels `operation_token`: aborts the active provider request and any running tool executions. The agent returns to idle (ready for new input).
- Second Ctrl+C (or Ctrl+C while idle) cancels `session_token`: triggers graceful shutdown (flush pending session writes, restore terminal state, exit).
- Tool timeout cancels only the affected `tool_token`. In parallel execution mode, other tools in the batch continue. In sequential mode, the batch is abandoned after the timed-out tool.
- `Agent::abort()` cancels `operation_token` programmatically (equivalent to first Ctrl+C).
- Dropping a provider stream SHOULD cancel the underlying HTTP request via the `operation_token` or `Request::cancel` field.

Additional rules:

- Provider streams SHOULD use bounded channels to propagate backpressure.
- Tool subprocesses MUST be killed or deliberately detached on cancellation.
- Child tokens are created per-operation and per-tool; they MUST NOT outlive their parent scope.

### 11.3 Observability

Observability in opi is **local and explicit**. Shared diagnostics, the local trace envelope, and the `opi doctor` command run against local state only — they never phone home, transmit telemetry or analytics, or share sessions automatically. This is an **unstable 0.x** surface: the diagnostic codes, trace envelope shape, and `--json`/RPC event fields may change between minor versions until a later phase stabilizes them.

`tracing` spans SHOULD cover provider calls, SSE parsing, agent turns, tool execution, session append/load, compaction, and retry scheduling. Secrets and raw provider payloads MUST be redacted by default. Non-interactive CLI traces are produced only when explicitly requested via the `--trace` CLI flag. RPC sessions keep only the latest run's redacted trace envelope in local memory and expose it only when the client sends the `trace` command; no trace is persisted automatically. Trace consumers MUST tolerate a `TurnStarted` record without a matching `TurnEnded` record when cancellation, provider failure, or trace setup failure exits mid-turn.

### 11.4 Performance Targets

| Metric | Target | Verification |
|---|---:|---|
| startup to first prompt | less than 100 ms | CLI init benchmark without network |
| first token display overhead | provider delta plus less than 50 ms | mock streaming provider |
| TUI frame rate | 30 FPS target | terminal snapshot/perf fixture |
| idle memory | less than 50 MB | release smoke measurement |
| release binary size | less than 20 MB target | release artifact check |

## 12. Testing Strategy

| Level | Owner | Required coverage |
|---|---|---|
| unit | every crate | message conversion, schema validation, config parsing, path handling |
| provider contract | `opi-ai` | SSE fixtures, terminal events, error mapping |
| mock loop integration | `opi-agent` | canned provider events and mock tools |
| session round-trip | `opi-agent` | JSONL append/load, tree reconstruction, compaction |
| tool tests | `opi-coding-agent` | temp-dir file tools, command timeout/cancellation |
| CLI E2E | `opi-coding-agent` | `--help`, `--version`, non-interactive mock run, exit codes |
| TUI snapshot | `opi-tui` | deterministic render output at fixed sizes |
| JSON contract | `opi-coding-agent` | NDJSON schema and line framing |
| live provider | `opi-ai` | ignored tests gated by env vars |
| fuzz/property | selected crates | JSONL loader, provider parser, tool argument schemas |

Phase 1 MUST include a mock provider harness. Live provider tests are not sufficient because they are slow, paid, flaky, and credential-dependent.
Session round-trip, JSON contract, and session-loader fuzz/property tests become
required when the corresponding Phase 2 features are implemented.

Current CI gates:

- `cargo fmt --all --check`;
- `cargo clippy --workspace --all-targets`;
- `cargo test --workspace --all-targets`;
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.

## 13. Security and Risk

### 13.1 Threat Model

Opi runs local tools with the user's privileges. The main risks are dangerous local commands, secret leakage, sensitive session files, and credential mishandling.

### 13.2 Requirements

- API keys MUST NOT be logged or written to sessions.
- Sessions MUST be documented as sensitive.
- `bash` MUST have timeout, cwd control, environment policy, cancellation behavior, and visible command text.
- File tools MUST resolve paths deliberately and record whether paths are inside or outside the workspace.
- Path traversal MAY be allowed, but tool allowlists or extension hooks SHOULD be able to restrict it.
- Provider HTTP MUST use TLS by default.
- Phase 1 MUST include auditability for `write`, `edit`, and `bash`; mutating tools and shell execution must be visible, bounded, and explicitly controllable in non-interactive mode.
- Opi core SHOULD NOT grow a permanent permission-popup subsystem as a Phase 3 goal. Users who need environment-specific gates should use containers, read-only tool allowlists, hooks, extensions, or packages.

Structured arguments reduce shell injection risk, but invoking a shell still executes model-supplied command text. The mitigation is visibility, auditability, tool allowlists, timeout, cwd/env control, extension hooks, and careful command construction.

### 13.3 Risk Register

| Risk | Impact | Likelihood | Mitigation |
|---|---|---:|---|
| Provider API drift | high | medium | fixture tests and narrow adapters |
| Anthropic-only MVP disappoints alignment expectations | medium | medium | publish clear phase scope |
| Session schema stabilizes too early | high | medium | keep v1 unstable until contract tests pass |
| Bash tool performs destructive actions | high | high | sequential mode, visible command, timeout, tool allowlists, extension hooks |
| Secrets leak to logs/session | high | medium | redaction tests and secret types |
| Windows TUI issues | medium | medium | crossterm tests and Windows smoke checks |
| Premature crates.io publish | high | medium | gate first publish on real implementation, docs, and contract tests; defer crates.io if those gates miss 0.2.0 |
| Extension scope bloats core | medium | high | minimal-core rule |
| MCP becomes core scope creep | medium | medium | keep MCP as an extension/package example after extension API stabilizes |
| Duplicate session stacks | high | medium | explicit Harness vs CodingHarness ownership |

## 14. Release and Versioning

All crates share one workspace version.

| Version | Milestone | Publish |
|---|---|---|
| 0.1.0 | scaffolding | GitHub Release only |
| 0.2.0 | Phase 1 MVP | GitHub Release; crates.io only if publish gates pass |
| 0.3.0 | Phase 2 persistence/providers | GitHub + crates.io |
| 0.4.0 | Phase 3 production hardening | GitHub + crates.io |
| 0.5.0 workspace | Phase 4 extensibility substrate | GitHub + crates.io for publishable crates |
| 0.5.1 workspace | Phase 5 Rust-native package and process-adapter MVP | GitHub + crates.io for publishable crates |
| 0.5.2 workspace | Phase 6 alignment and reliability hardening | GitHub + crates.io for publishable crates |
| 0.5.3 workspace | Phase 7 reliability and observability hardening | GitHub + crates.io for publishable crates |
| 0.5.4 workspace | Phase 8 runtime stabilization | GitHub + crates.io for publishable crates |
| 0.6.0 workspace | Phase 9-14 roadmap realignment | GitHub-only planning/docs release |
| 0.6.1 workspace | Phase 9 pi 0.80.2 baseline evidence ledger and documentation guards | GitHub-only planning/docs release |
| 0.6.2 workspace | Phase 10 architecture deepening: provider collection/auth, generic harness, and session facade seams | GitHub + crates.io for publishable crates |
| 0.6.3 workspace | Phase 11 tooling quality: hardened built-in tools, tool-result diagnostics and metadata, public-surface redaction, and provider wire fidelity | GitHub + crates.io for publishable crates |
| 0.6.4 workspace | Phase 12 provider correctness: provider error taxonomy and safe diagnostics, OpenAI-compatible profile compatibility flags, fixture-backed provider wire coverage across all built-in families, and provider-correctness documentation guards | GitHub + crates.io for publishable crates |
| 0.6.5 workspace | Phase 13 session tree and context reconstruction: typed forward-compatible session entries, reusable context reconstruction across resume/fork/list/export, interactive session metadata commands, local redacted session export, and RPC/TUI session handoff metadata | GitHub + crates.io for publishable crates |
| 0.7.0 workspace | Interim release on top of Phase 13: NDJSON linear text deltas via `--json-compact`, provider turn counts in session summaries, the opi-document documentation skill with Artifact Truthfulness Gate and opi-eval evaluation skill, and fixes for custom chat-completions endpoint paths, runtime message timestamps, and read-tool workspace-path redaction | GitHub + crates.io for publishable crates |

The first crates.io publish is gated by quality, not by the version number alone.
It MAY happen at 0.2.0 if all published crates expose real, documented behavior
rather than placeholder public APIs, public docs build cleanly, contract tests
cover the shipped provider/tool/runtime boundaries, and the release skill's
checks pass. If those gates are not met, crates.io publishing SHOULD move to a
later 0.2.x or 0.3.0 release while GitHub binary releases continue. Because the
binary crate depends on internal library crates, those libraries should publish
together in dependency order. All 0.x public APIs are unstable unless explicitly documented
otherwise.

The release process SHOULD follow `.claude/skills/opi-release/skill.md`: pre-flight, version bump, changelog, checks, tag/draft release, crates.io publish, finalize. crates.io publishing is irreversible except yanking; rollback should use new commits and tag management, not force-pushed public history.

Release CI builds:

- `opi-linux-x64.tar.gz`;
- `opi-linux-arm64.tar.gz`;
- `opi-darwin-x64.tar.gz`;
- `opi-darwin-arm64.tar.gz`;
- `opi-windows-x64.zip`;
- `opi-windows-arm64.zip`.

`SHA256SUMS.txt` SHOULD be uploaded with release artifacts. Windows ARM64 is a
Tier 2 target and should be treated as non-blocking for Phase 1 MVP releases if
the target-specific build flakes while Tier 1 targets pass.

## 15. Implementation Roadmap

### Phase 0 - Scaffolding Baseline

Status: complete in 0.1.0.

| Task | Status |
|---|---|
| four-crate workspace | done |
| lockstep versioning | done |
| placeholder modules and re-exports | done |
| CI gates | done |
| six-platform release workflow | done |
| `opi --version` and `--help` | done |
| GitHub Release only, crates.io deferred | done |

### Phase 1 - MVP Foundation

Target: 0.2.0.

Goal: Anthropic-only coding agent with core loop, six tools, minimal safety
boundaries for mutating tools and shell execution, basic TUI, TOML config, and
mock-provider tests.

| # | Task | Crate | Definition of done |
|---|---|---|---|
| 1.0 | introduce Phase 1 dependencies | workspace | manifests include needed deps with minimal features and without unused-dep warnings |
| 1.1 | message and stream types | `opi-ai` | serialize where needed; terminal stream events tested |
| 1.2 | replace placeholder provider trait | `opi-ai` | `stream(Request)` replaces `complete` |
| 1.3 | Anthropic SSE provider | `opi-ai` | fixtures cover text, tool call, usage, error |
| 1.4 | provider registry | `opi-ai` | resolves `anthropic:model` and capabilities |
| 1.5 | tool trait and schema validation | `opi-agent` | invalid args become error tool result |
| 1.6 | `agent_loop` | `opi-agent` | mock tests cover no-tool and tool-use turns |
| 1.7 | `Agent` wrapper | `opi-agent` | prompt, continue, abort, subscribe tested |
| 1.8 | hooks and queues | `opi-agent` | before/after, should-stop, steering, follow-up tested |
| 1.9 | `read`, `write`, `edit`, `bash` | `opi-coding-agent` | temp-dir tests cover success, failure, timeout/cancellation, cwd/env reporting, and minimal confirmation policy |
| 1.10 | `glob`, `grep` | `opi-coding-agent` | tests cover ignored dirs and regex errors |
| 1.11 | system prompt construction | `opi-coding-agent` | prompt includes tool defs and system layer |
| 1.12 | TUI shell | `opi-tui` | fixed-size render snapshots |
| 1.13 | markdown/code rendering | `opi-tui` | markdown and fenced code snapshots |
| 1.14 | interactive CLI wiring | `opi-coding-agent` | runs against mock provider |
| 1.15 | non-interactive mode | `opi-coding-agent` | stdout/stderr/exit-code tests |
| 1.16 | TOML config loading | `opi-coding-agent` | missing defaults and malformed errors tested |
| 1.17 | integration harness | cross-crate | mock-provider E2E runs in CI |

Exit criteria: `opi` accepts a prompt, streams Claude output, executes
`read/write/edit/bash/glob/grep` behind the Phase 1 safety boundary, displays
results in TUI, supports non-interactive mode with explicit high-risk tool
policy, and passes mock-provider CI tests. Sessions, compaction, JSON mode, MCP,
plugins, rich diff views, and syntax-highlighted code blocks are not
Phase 1 exit criteria.

### Phase 2 - Multi-Provider and Persistence

Target: 0.3.0.

| # | Task | Crate |
|---|---|---|
| 2.1 | OpenAI-compatible chat provider | `opi-ai` |
| 2.2 | OpenRouter provider profile | `opi-ai` |
| 2.3 | OpenAI Responses provider | `opi-ai` |
| 2.4 | Google Gemini provider | `opi-ai` |
| 2.5 | Mistral provider | `opi-ai` |
| 2.6 | opi session v1 JSONL storage and contract tests | `opi-agent` |
| 2.7 | session list/resume/delete | `opi-coding-agent` |
| 2.8 | compaction | `opi-agent` / `opi-coding-agent` |
| 2.9 | thinking/reasoning support | `opi-ai` |
| 2.10 | usage and cost tracking | `opi-ai` |
| 2.11 | diff view | `opi-tui` |
| 2.12 | themes | `opi-tui` |
| 2.13 | keybindings | `opi-tui` |
| 2.14 | `--json` NDJSON mode | `opi-coding-agent` |
| 2.15 | retry/backoff/rate limits | `opi-ai` |
| 2.16 | session contract tests | `opi-agent` |

Exit criteria: sessions survive restart, multiple providers pass contract fixtures, long conversations compact before overflow, and JSON mode has schema tests.

### Phase 3 - Production Hardening

Status: complete in 0.4.0.

| # | Task | Crate |
|---|---|---|
| 3.1 | AWS Bedrock provider | `opi-ai` |
| 3.2 | Azure OpenAI provider | `opi-ai` |
| 3.3 | Google Vertex provider | `opi-ai` |
| 3.4 | image input | `opi-ai` |
| 3.5 | image tool results | `opi-agent` |
| 3.6 | terminal image rendering | `opi-tui` |
| 3.7 | `AGENTS.md` / `CLAUDE.md` context loading | `opi-coding-agent` |
| 3.8 | pi-style tool selection and safety hooks | `opi-coding-agent` |
| 3.9 | `find` / `ls` built-in file navigation tools | `opi-coding-agent` |
| 3.10 | shell completions | `opi-coding-agent` |
| 3.11 | fuzzy model/session picker | `opi-tui` |
| 3.12 | proxy support | `opi-ai` |
| 3.13 | connection pooling tuning | `opi-ai` |

Cross-platform binary releases are not listed here because release CI is already part of Phase 0.

Exit criteria: enterprise providers work, image and terminal-image flows work, project context loading matches pi, risky tools are visible and controllable through pi-style tool selection/hooks, release artifacts are repeatable, and interactive UX is robust for daily use.

### Phase 4 - Extensibility Substrate

Status: substrate implemented in the current `0.7.0` workspace.

Phase 4 is ordered so the reusable substrate lands before workflow-heavy
features. Later tasks may depend on earlier tasks, but examples must not become
core policy.

| # | Task | Crate |
|---|---|---|
| 4.1 | RPC JSONL mode with strict framing, correlated responses, async events, extension commands, and session/model/thinking/compaction commands | `opi-coding-agent` |
| 4.2 | SDK embedding surface over the same event and command model | `opi-coding-agent` / `opi-agent` |
| 4.3 | settle `opi-agent::Transport`: real RPC/proxy transport, hidden unstable API, or removal before stable public API claims | `opi-agent` |
| 4.4 | extension trait, lifecycle hooks, custom tools, custom commands, custom messages, and extension state | `opi-agent` / `opi-coding-agent` |
| 4.5 | extension/resource loading strategy for project and user resources | `opi-coding-agent` |
| 4.6 | custom provider/model registration through SDK or extensions | `opi-ai` / `opi-coding-agent` |
| 4.7 | skills, prompt fragments, themes, and packages with progressive discovery | `opi-coding-agent` |
| 4.8 | extension/package examples: permission gate, protected paths, sub-agent, plan mode, todo, MCP adapter | examples / package template |
| 4.9 | session branching UI | `opi-agent` / `opi-tui` |
| 4.10 | streaming proxy | `opi-agent` or new crate |

Exit criteria: third parties can compose and extend opi through RPC, SDK, extension APIs, discovered resources, skills, prompt fragments, themes, packages, and custom provider/model registration without patching core crates. MCP, sub-agents, plan mode, todos, and permission gates should be demonstrable as extensions or packages, not core features. The `Transport` public surface is absent; it must not be reintroduced as a stable public claim without a real implementation.

### Phase 5 - Rust-Native Package and Process-Adapter MVP

Status: implemented in the current `0.7.0` workspace.

Phase 5 adds package management and executable adapter hosting so that external packages can provide tools, commands, hooks, and events through child process adapters without patching core crates. It deliberately does not claim parity with pi's npm package ecosystem, TypeScript extension runtime, hot reload behavior, marketplace conventions, provider streaming adapters, custom TUI adapters, or package permission enforcement.

| # | Task | Crate |
|---|---|---|
| 5.1 | Package store and source model | `opi-coding-agent` |
| 5.2 | Package CLI MVP | `opi-coding-agent` |
| 5.3 | Manifest V2 compatibility with adapter and opi_version | `opi-coding-agent` |
| 5.4 | Adapter JSONL protocol types | `opi-coding-agent` |
| 5.5 | Adapter process host | `opi-coding-agent` |
| 5.6 | Adapter runtime bridge into Extension trait | `opi-coding-agent` / `opi-agent` |
| 5.7 | Harness and startup integration | `opi-coding-agent` / `opi-agent` |
| 5.8 | Runnable example adapter packages | examples / `opi-coding-agent` |
| 5.9 | Documentation, alignment, and guards | workspace |

Exit criteria: `opi package add/remove/list/doctor` works for local and git package declarations; packages with `[adapter]` sections start as child processes using `opi-extension-jsonl-v1`; adapter tools, commands, hooks, state, and cancellation bridge into the existing extension API; example packages (todo, permission-gate, protected-paths) exercise the full pipeline; documentation is truthful and guard tests reject claims about npm, marketplace, hot reload, provider streaming adapters, custom TUI adapters, package update/config/enable/disable workflows, or package permission enforcement.

### Phase 6 - Alignment and Reliability Hardening

Status: complete in the current `0.7.0` workspace.

Phase 6 tightened documentation, package/runtime integration, provider
configuration behavior, and reliability around the Phase 4-5 surfaces. It did
not change the core scope: package adapters and workflow examples remain
substrate-level extension paths, not built-in product workflows.

### Phase 7 - Reliability and Observability Hardening

Status: complete in the current `0.7.0` workspace.

Phase 7 added shared diagnostics, redaction, provider/runtime error
classification, opt-in local trace envelopes, and `opi doctor`. Observability
is local and explicit; it does not introduce telemetry, analytics, automatic
session sharing, or a stable 1.0 trace protocol.

### Phase 8 - Runtime Stabilization

Status: complete in the current `0.7.0` workspace.

Phase 8 documented and tested runtime event order, hook semantics, tool
scheduling/termination, cancellation, SDK/RPC command state, diagnostics/trace
wire behavior, and public API surface classifications. It keeps public APIs at
0.x maturity and does not add TypeScript extension API compatibility, package
ecosystem expansion, provider OAuth work, MCP core runtime work, a shared
`opi-types` crate, or a whole-loop rewrite.

### Phase 9 - pi 0.80.2 Baseline Realignment

Status: planned documentation/evidence gate.

Phase 9 updates the project baseline from the earlier studied upstream snapshot
to `.repo/pi-0.80.2` and documents the revised Phase 9-14 roadmap. It is
documentation-only: no runtime behavior changes, code migration, OAuth, image
generation, custom UI protocol, npm/gallery workflow, web/share flow, or `pi`
session compatibility promise belongs in this phase.

Exit criteria: English and Chinese normative docs name `.repo/pi-0.80.2` as the
current baseline; `opi-agent` alignment reflects the generic harness gap
honestly; and future ecosystem candidates have entry conditions. (Phase 9
originally maintained a durable alignment-matrix evidence baseline; ongoing
alignment is now done by fresh `opi-realign` audits under `docs/realign/`.)

### Phase 10 - Core Architecture Deepening

Status: in progress; initial seams landed.

Phase 10 deepens existing capabilities before breadth:

| Workstream | Owner | Purpose |
|---|---|---|
| `Models/Auth` seam | `opi-ai` | provider collection/model lookup, refresh if supported, provider-owned auth, compatibility metadata, stream/complete dispatch |
| Generic `AgentHarness` | `opi-agent` | phase guards, turn snapshots, save points, ordered pending writes, runtime config mutation semantics |
| Session repo/facade | `opi-agent` | stable durable append/load/entry-count traits and ordered read/write facade for Phase 13; list/fork stay product-owned in `opi-coding-agent` |
| Runtime hook boundaries | `opi-agent` / `opi-coding-agent` | keep current hooks narrow while preserving future provider/UI/session lifecycle paths |

Initial seams have landed across the four workstreams: `opi-ai` exposes the
published provider collection/auth seam. `opi-coding-agent` routes model
listing and model-registry construction through that seam; active runtime
provider dispatch still uses the existing `Box<dyn Provider>` path, and
collection-level dispatch adoption is deferred until a reviewed product-loop
migration. `opi-agent` exposes the published generic `AgentHarness` and session
facade seams; product turn loop adoption is deferred to a reviewed product-loop
migration, and list/fork stay product-owned in `opi-coding-agent`. The focused
regression tests cover existing behavior, and the runtime hook boundary model
is documented below.

#### Session facade boundaries

Phase 10 adds a stable `SessionFacade` / `SessionRepo` seam in `opi-agent` so
Phase 13 session-native context entries are not added through ad hoc CLI-only
paths. `SessionRepo` owns durable append/load and entry-count semantics over a
v1 session file (v1 sessions remain readable); `SessionFacade` owns ordered
read/write over the repo, with agent-emitted messages persisting before pending
extension/session writes at save points. CLI-driven session construction
(resume/fork/delete, branch selection) stays in `opi-coding-agent`; only the
durable storage seam lives in `opi-agent`. The additive entry set
(model/thinking changes, labels, branch summaries, custom entries) is deferred
to Phase 13.

#### Runtime hook boundaries

Phase 10 documents the runtime hook boundary model so `pi`'s broad TypeScript
extension surfaces (provider hooks, session lifecycle hooks, custom UI, message
renderers) do not get copied into Rust core. The narrow core loop hook contract
(`opi-agent::hooks::AgentHooks`) is contract-tested and stays in `opi-agent`;
product extensions and the process adapter do not migrate into `opi-agent`
unless a concrete non-CLI embedder needs hosting. Provider request/response
hooks and custom TUI UI/message renderers stay future ecosystem candidates with
explicit prerequisites (see Future Ecosystem Candidates below).

| Surface | Phase 10 owner | Phase 10 action |
|---|---|---|
| Core loop hooks | `opi-agent` | Contract-tested and narrow (`AgentHooks`: convert/transform/before/after/should_stop/prepare). |
| Generic harness events/results | `opi-agent` | Typed event/result reducers only where needed by generic lifecycle. |
| Coding-agent extension registry | `opi-coding-agent` / bridge to `opi-agent` | Product-specific commands, resources, and packages compose through `ExtensionRegistry`. |
| Process adapter protocol | `opi-coding-agent` | Owns `opi-extension-jsonl-v1` parsing and child-process hosting; the process adapter protocol does not migrate into `opi-agent` unless a non-CLI host needs it. |
| Provider request/response hooks | Future candidate | Deferred; prerequisites now met in design (Phase 12 correctness + Phase 7 redaction + Phase 14 Request enrichment); consumer pending. |
| Custom TUI UI / message renderer | Future candidate | Deferred until the Phase 17 built-in TUI is stable and a UI/RPC subprotocol is designed. |

Typed hook result composition is covered by contract tests: extension hooks run
after the base hook in registration order, a `Block`/`Deny` short-circuits the
chain, and the coding-agent process adapter bridges through the same
`ExtensionRegistry::wrap_hooks` composite (no bypass). Extension API docs do
not claim `pi` TypeScript extension API compatibility as current `opi` scope.

Non-goals do not claim: OAuth login, subscription auth, broad provider catalog
expansion, image generation, custom TUI extension protocol, npm/package
marketplace work, browser/web work, `pi` TypeScript API compatibility, `pi`
session file compatibility, a shared `opi-types` crate, or a whole-loop rewrite.

### Phase 11 - Tooling Quality

Status: completed in Unreleased workspace changes.

Phase 11 hardens built-in tools after Phase 10 clarifies harness/tool
scheduling boundaries. It delivered the tool-result contract, filesystem error
taxonomy, read/write/edit/bash hardening, navigation-tool bounded work and skip
diagnostics, diagnostics/trace lifting and redaction, provider `is_error`
propagation, and documentation/help guard tests. It deliberately did not add
persistent background shells or broad permission-popup systems.

### Phase 12 - Provider Correctness

Status: implemented in the current `0.7.0` workspace.

Phase 12 hardens the existing provider families and OpenAI-compatible profiles
through fixture-backed lifecycle, error, auth, image-input, thinking, usage,
retry, rate-limit, and compatibility tests, all routed through the Phase 10
provider collection/auth seam. It is not a provider breadth phase.

Implemented provider-correctness surface: the nine built-in families
(anthropic, openai chat, openai-responses, openrouter, mistral, gemini,
bedrock, azure, vertex) plus config-driven OpenAI-compatible profiles carry
per-family request/streaming/tool-call/thinking/image/usage fixtures; the
nine-class provider error taxonomy (auth, config, request, network,
rate_limit, provider, stream, capability, cancelled) maps to safe
diagnostics; OpenAI-compatible `CompatConfig` flags (`system_role_override`,
`max_tokens_field`, `tool_result_name_field`, `usage_in_stream`,
`strict_tool_schema`, `reasoning_effort`, `cache_key`,
`require_assistant_after_tool_result`) and model-level overrides honor
model-over-provider precedence; `require_assistant_after_tool_result` is
represented as compatibility metadata for legacy endpoints rather than
runtime-enforced shared-adapter behavior; `usage_in_stream` requests
`stream_options.include_usage` and preserves usage updates from any streaming
chunk; per-profile static request headers (`extra_headers`) are a separate
profile config field, not a `CompatConfig` flag; usage-side cache tokens and
provider response IDs (Anthropic `message.id`, OpenAI Chat `chatcmpl-*`,
Responses `resp_*`) round-trip into `AssistantMessage::response_id`, and
OpenAI Chat captures the ID from any chunk carrying `id`, not only role
chunks; retry, partial-output no-retry, per-family cancellation, and
per-provider proxy config are covered without live calls; provider
construction validates credentials at build time and emits auth/config
diagnostics with safe remediation.

Explicitly deferred: OpenAI Responses `previous_response_id` and server-side
session chaining (Responses requests are Chat-Completions analogues);
request-side prompt-caching breakpoints (the `cache_key` profile flag is the
available cache-affinity hint); image count/size limits beyond the existing
per-model metadata; broad provider catalog expansion.

Phase 12 non-goals (must not appear as current core behavior): OAuth login
flows; Anthropic, OpenAI Codex, and GitHub Copilot subscription auth; a broad
new first-class provider list; image generation; browser usage; a provider
streaming-adapter protocol for packages; paid live provider calls in default
tests; copying pi's provider-specific config file format. Best-effort cost
mapping keeps explicit unknown values rather than inferring false confidence:
missing usage is tracked as unknown, and session cost summaries are omitted
when any turn has unknown usage or when pricing is unknown.

### Phase 13 - Session Tree and Context Reconstruction

Status: implemented substrate plus product paths; recast from the previous
Phase 11.

Phase 13 deepens session-native context on top of the Phase 10 generic
harness/session facade semantics. It adds additive typed entries for session
metadata (`session_info`), model and thinking changes (`model_change`,
`thinking_level_change`), labels (`label`), and branch summaries
(`branch_summary`) while keeping v1 files readable and header version 1.
`custom_message` is deferred (see below). Exports are local files;
web/share/session publishing remains future ecosystem scope.

Implemented entries and their production paths:

- `session_info`, `label` — interactive `/name`, `/label`, `/unlabel`,
  `/session info`, RPC `session_info`, and `--list-sessions --json` (Phase 13.4).
- `model_change`, `thinking_level_change` — idle `set_model_validated` and
  `set_thinking_level` append through `SessionCoordinator` without advancing
  the active leaf; resume applies recorded values when compatible (Phase 13.3).
- `branch_summary` — `opi-agent::session_context::reconstruct_context` injects
  a parent branch summary into reconstructed LLM context as a metadata-parented
  message, and `opi-coding-agent` forwards that message to providers when
  present (Phase 13.2 substrate). The same-session synthesized
  `BranchSummaryMessage` currently sets `parent_session_id` to `""` and
  `entry_count` to `0` because the durable entry stores only the summary text;
  embedders must treat those fields as placeholders until cross-session summary
  injection carries richer provenance.

Explicit decisions and deferrals (each cited to the Phase 13 design):

- `branch_summary` is implemented as a context-reconstruction and provider-
  conversion substrate. Its generation UX triggers — branch switch, fork,
  manual command, extension hook — are deferred to Phase 16 Agent Intelligence;
  Phase 13 does not auto-generate branch summaries on these triggers.
- `custom_message` provider-context semantics are deferred: provider conversion
  and transcript rules for extension-provided context messages are not specified
  yet, so Phase 13 ships no `custom_message` writer and treats unknown
  `custom_message` entries like other unknown future entries on read.
- Interactive `/export` is deferred to Phase 17 TUI product polish; Phase 13
  ships the local `--export-session` CLI only.

Phase 13 handoff: session work may rely on provider-correct usage, model,
thinking, and error data through the shared `opi-ai` types, without depending
on provider-specific internals.

Phase 13 non-goals (documented as deferred, not current core):

- No vector database.
- No semantic memory service.
- No global user profile memory.
- No automatic cross-project memory injection.
- opi does not claim pi session v3 read/write file compatibility.
- No cloud sync.
- No session sharing service.
- No web UI product.
- No package ecosystem expansion.
- No provider runtime, provider auth, OAuth, or `ProviderCollection` dispatch
  refactor.

### Phase 14 - Provider & Auth

Status: implemented; pi-0.80.6 alignment complete. Historical design:
`docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`. Corrective design:
`docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.

Production startup installs a native credential store before any
credential-aware path:

| Release targets | Store crate | Native service |
|---|---|---|
| `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | `windows-native-keyring-store` | Windows Credential Manager |
| `x86_64-apple-darwin`, `aarch64-apple-darwin` | `apple-native-keyring-store` | macOS Keychain Services |
| `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `zbus-secret-service-keyring-store` | Freedesktop Secret Service |

Phase 14 promotes Provider/Auth to a production cluster. `opi-ai` owns the
IO-free `CredentialStore`, `OAuthProvider`, `LoginPresenter`, `AuthResolver`,
`WireApi`, `ModelInfo`, and `ApiMappedProvider` contracts.
`opi-coding-agent` owns native-keychain IO, environment fallback, refresh and
mutation locking, concrete login flows, provider construction, and the outer
TUI interaction policy. The store uses `fs4` locking and `secrecy` at the
boundary; there is no opi-managed plaintext credential file.

The canonical OAuth provider and keychain account ids are `anthropic`,
`github-copilot`, and `openai-codex`. The development ids `copilot` and
`codex` have no runtime alias, config alias, or keychain migration; users with
those development entries must explicitly log in again with the canonical id.

Every `ModelInfo` declares one exact `WireApi`, a matching tagged
compatibility value, capabilities, a thinking-level map, and optional pricing.
Pricing tiers are deterministic: a tier applies only when input tokens are
strictly greater than `input_tokens_above`. Public `ApiMappedProvider` exposes
one provider id and catalog, validates exactly one concrete route for each
catalog wire, and rejects unknown models, missing routes, and wire/compatibility
mismatches before network IO. All routes for one mapped provider share one
lazy `AuthResolver`.

GitHub Copilot uses the canonical `github-copilot` identity and one audited
static pi-0.80.6 catalog spanning `anthropic-messages`,
`openai-completions`, and `openai-responses`. Each route re-resolves the
current token and enterprise base URL immediately before HTTP. The static
catalog is an intentional opi divergence from live account-entitlement and
model-enable filtering: model listing reads no OAuth secret and makes no
entitlement request.

OpenAI Codex uses the canonical `openai-codex` identity and the dedicated
`openai-codex-responses` provider at
`https://chatgpt.com/backend-api/codex/responses`. It is not standard
Responses with compatibility flags. Codex requires the persisted account id
and emits its dedicated body, headers, session affinity, and SSE mapping.

`[providers.custom.<id>]` exposes the mapped-provider contract to TOML. One
provider owns one shared `api_key_env`, auth scheme, default `base_url`, proxy,
and header set. A provider `api` is the model default; model `api` and
`base_url` override provider defaults. Custom models may select only
`anthropic-messages`, `openai-completions`, or `openai-responses`;
`openai-codex-responses` is built-in-only. Thinking-map values are `true`
(identity), `false` (unsupported), or a non-empty string (wire value).
Compatibility values are wire-tagged. Models may declare base pricing and
strictly ascending threshold tiers. Unknown/disabled wires, duplicate models,
missing APIs/routes/base URLs, mismatched compatibility fields, invalid
pricing, and provider-managed authentication headers fail during config load.
The older `[providers.openai_compatible]` table remains a single-wire
Completions shorthand and lowers into the same mapped construction path.

Anthropic and OpenAI Codex Browser login use Browser PKCE with a loopback
callback and manual-code/redirect-URL fallback. GitHub Copilot Device Code and
OpenAI Codex Device Code call `present_device_code`, poll the device endpoint,
and never call `await_manual_code`. `/login openai-codex` offers Browser as the
default and Device Code as the headless option. `/login` and `/logout` run
through the production dispatcher, concrete provider registry, locked store,
and RAII terminal suspension/restoration.

After a pre-output `CredentialNeeded`, a successful explicit login for the
same provider makes the outer `run_interactive_tui` state machine retry the
same pending turn exactly once without appending a duplicate user message.
Different-provider login, cancellation, presenter/OAuth/store/terminal
failure, and mid-stream `CredentialRevoked` perform no retry. JSON, RPC, and
text modes report the canonical `provider_id` plus `/login <provider>`
remediation and fail without constructing a presenter, opening a browser, or
waiting for input.

Phase 14 also adds `opi_ai::Request` enrichment (`timeout`,
`extra_headers`, `cache_retention`, `session_id`), strict
`Usage`/`CostBreakdown` cache-and-reasoning accounting, the single nested
`ModelCapabilities` value that drives Anthropic prompt-cache markers, and the
dynamic `refresh_models` trait substrate. `cache_write_1h_tokens` is a subset
of cache writes and `reasoning_tokens` is a subset of output; both are optional
`u64` children so absent and explicitly reported zero remain distinct. The
four-line `CostBreakdown` folds weighted one-hour writes into
`cache_write_cost` and reasoning into `output_cost`, so parent buckets are
counted once. The first three Request knobs have no Phase 14 config/harness
producer; only `session_id` traverses the production harness/agent path.
Dynamic refresh has mock collection coverage but no Phase 14 production
trigger and therefore closes no product acceptance path.

Phase 14 explicitly retains eight boundaries: no opi-managed plaintext
credential file; no auto-relogin mid-stream; no per-call credential
(`apiKey`/`env`) or provider-managed auth-header override (additive
`Request::extra_headers` remains available only for non-reserved transport
headers); no `onPayload`/`onResponse` streaming hooks; no
`maxRetries`/`maxRetryDelay` on `Request`; no end-to-end `SecretString`
provider-construction migration; no OAuth providers beyond Anthropic, GitHub
Copilot, and OpenAI Codex; and no session-schema or context-reconstruction
changes. TUI changes are limited to the reviewed `/login`, `/logout`,
`CredentialNeeded` presenter, and raw/alternate-screen suspension around
login.

Only a successful, user-initiated `/login <provider>` retries a pending
interactive turn; `CredentialNeeded` never starts login automatically. JSON,
RPC, and text modes report `provider_id` plus `/login <provider>` remediation
and fail without a presenter, browser, or input prompt.

`api-map`: `implemented` by Task 14.16. The public Rust
`ApiMappedProvider` contract and `[providers.custom.<id>]` TOML contract route
one provider catalog across checked concrete wires with one shared lazy
credential source. The offline pi-0.80.6 fixtures
`github-copilot.models.json` and `openai-codex.models.json` pin catalog
provenance, while `mapped_provider_dispatches_one_catalog_across_three_wires`,
`mapped_routes_share_one_lazy_auth_resolver`,
`custom_provider_api_and_base_url_precedence`, and
`invalid_custom_provider_contracts_fail_at_load` pin mapped-provider behavior.

Phase 14 acceptance trace:

| Criterion | Owner | Production/evidence trace |
|---|---:|---|
| SC1 credential storage and probes | 14.1, 14.8, 14.14 | The cfg-gated host-selection test enters the production native-store selector and proves constructor, default-store, and guard lifecycle; async store/doctor/listing tests retain strict redacted resolver behavior. |
| SC2 OAuth product flows | 14.2, 14.9, 14.18, 14.19 | Concrete dispatcher tests cover Anthropic Browser PKCE, GitHub Copilot Device Code, and OpenAI Codex Browser/Device Code through locked persistence and exact terminal restoration. |
| SC3 live auth and session interaction | 14.2, 14.10, 14.17, 14.18, 14.20 | Factory-built provider tests prove lazy auth and revocation on every approved wire; outer `run_interactive_tui` tests prove one same-provider retry and all negative gates; text/JSON/RPC never construct a presenter. |
| SC4 Request and session affinity | 14.3 | `agent_loop_mock::session_id_reaches_every_request`, `session_runtime::phase14_session_affinity_tracks_new_resume_and_fork`, and `request_enrichment::session_affinity_wire_mappings` trace production propagation and exact positive/negative wire mappings. |
| SC5 capabilities and cache markers | 14.4, 14.11, 14.15 | `ModelInfo` carries exact wire/capability metadata, and `anthropic_cache_markers` captures capability-gated marker positions and TTL through a factory-built concrete Anthropic stream. |
| SC6 usage, metadata, and cost | 14.5, 14.12, 14.15, 14.17, 14.18 | Public contracts, pi catalog fixtures, pricing-tier tests, provider fixtures, cost tests, and session resume preserve strict subsets and deterministic model pricing without double counting. |
| SC7 dynamic refresh and api-map substrate | 14.6, 14.16 | `ApiMappedProvider` and custom TOML tests prove checked multi-wire dispatch with shared lazy auth; collection tests retain deterministic atomic refresh, which has no production trigger. |
| SC8 documentation and guards | 14.7, 14.13, 14.21 | Paired public docs, rustdoc, TUI help, runtime remediation tests, the 58-row acceptance manifest, and workspace gates pin current provider/auth truth and api-map implementation. |

### Phase 15 - Safety & Sandbox

Status: designed; implementation pending. Design:
`docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md`.

Phase 15 promotes the Safety/Sandbox cluster with a per-tool Operations seam.
It adds an OS-native subprocess-tree sandbox for `bash` (Linux seccomp +
Landlock, macOS `sandbox-exec`, Windows Job Object) as opt-in defense-in-depth —
explicitly not a security boundary — with an always-on L0 tree-kill baseline; a
per-tool `Operations` seam (`FileOperations`/`BashOperations` in
`opi-coding-agent`) layered below `PathPolicy`, giving the sandbox a home inside
local `BashOperations::exec`; and a project-trust gate (`ProjectTrustStore`,
`--trust`/`--no-trust`, `AppState::AwaitingTrust`) that gates loading of
project-local resources including adapter declarations, closing the
native-child-process blast-radius gap. opi deviates from pi by not
auto-injecting project `AGENTS.md`/`CLAUDE.md` for untrusted projects.
Non-goals: opi-self confinement, adapter strict-confinement, remote backends,
and nav-tool Operations.

### Phase 16 - Agent Intelligence

Status: designed; implementation pending. Design:
`docs/superpowers/specs/2026-07-11-phase16-agent-intelligence-design.md`.

Phase 16 promotes the Agent Intelligence cluster. It adds a production
skills/fragments runtime (`/skill:`/`/fragment:` dispatch, a pi-style
`<available_skills>` system-prompt section, recursive `ignore`-based skill
discovery, and wiring the existing `disable_model_invocation`/
`expand_fragment_body` mechanisms); LLM-driven compaction and branch-summary
(widening `CompactionHooks::generate_summary` to an async boxed `Result` future
with the first provider-backed hook impl in `opi-coding-agent`, a token-budget
`find_split_point` with `keep_recent_tokens = 20000`, and
`SessionCoordinator::move_to` plus a `ForkTarget` enum); and read-tool
inline-image (decode/resize via the `image` crate, a companion
`OutputContent::Text` label, and the `tool_result` image wire fix across
Anthropic/Gemini/Vertex/Bedrock). `opi-agent` stays skill-free; the T8 trait
widen lives in `opi-agent`, the provider-backed impl in `opi-coding-agent`.
Non-goals: skill-body argument substitution, a dedicated RPC
`SdkCommand::Skill`, `readFiles`/`modifiedFiles` entries, split-mid-turn
compaction, and EXIF re-orientation.

### Phase 17 - TUI Engine, Extension Surface, and Product Polish

Status: planned; recast from the previous Phase 14 (TUI Product Polish) and
broadened to include the TUI engine and extension surface; design in progress
via the wayfinder map.

Phase 17 lands the event-driven TUI engine (`OverlayStack`, streaming-redraw
throttle) and polishes the built-in terminal product on top of it:
model/session/branch pickers, transcript rendering, command discovery,
status/error feedback, accessibility, terminal compatibility, and image/diff
presentation. It also introduces the extension UI surface (inline renderers,
overlay/dialog, provider request/response hooks) as deferred-with-pinned-designs,
re-triggered by concrete consumers. It does not promise web UI parity, custom
extension UI, message renderer parity, or a general TUI framework. Interactive
`/export` deferred from Phase 13 lands here.

### Future Ecosystem Candidates

These capabilities are aligned with `pi` but are not scheduled phases until
their entry conditions are met.

| Candidate | Entry condition |
|---|---|
| OAuth/subscription auth beyond Phase 14 | Phase 14 lands OAuth for Anthropic, GitHub Copilot, and OpenAI Codex (credential store, redaction, doctor, session interaction, login UX, refresh, revocation); additional OAuth/subscription providers remain future pending user demand. |
| Broad provider catalog | Phase 12 provider correctness is stable; OpenAI-compatible profile quirks have a documented compatibility model. |
| Image generation | Chat-side provider collection, auth, model metadata, cost, and error semantics are stable. |
| Custom extension UI / message renderer | Phase 17 built-in TUI is stable; a separate RPC/UI subprotocol is designed. |
| npm/gallery/update/enable/disable | Package adapter lifecycle, trust/source model, diagnostics, and lock/update policy are stable. |
| Web/share/session publishing | Phase 13 export, redaction, and session sensitivity rules are stable. |
| Provider request/response adapter hooks | Core provider seam, hook ordering, redaction, and trace semantics are stable; prerequisites met in design (Phase 12 + Phase 7 + Phase 14), consumer pending. |
| `pi` session import/migration | `opi` session v2 is stable; user value is clear; normal resume remains unaffected. |

## 16. Decision Log

| # | Decision | Choice | Reason |
|---|---|---|---|
| ADR-001 | Workspace shape | four crates mirroring pi packages | preserves conceptual boundaries |
| ADR-002 | Versioning | lockstep workspace version | simplifies compatibility and release order |
| ADR-003 | No shared types crate | types live with semantic owner | avoids hub dependency |
| ADR-004 | pi compatibility | semantic alignment, not API/file compatibility | Rust-native implementation |
| ADR-005 | MVP provider | Anthropic only | first release remains testable |
| ADR-006 | Provider SDKs | direct HTTP adapters | streaming control and fewer unstable deps |
| ADR-007 | Stream protocol | start/delta/end/done/error | aligns with pi and UI partial state |
| ADR-008 | Agent layering | loop -> Agent -> Harness | testability and separation |
| ADR-009 | Agent vs LLM messages | keep separate | custom messages should not leak to providers |
| ADR-010 | Tool boundary | typed args plus generated JSON Schema | dynamic LLM boundary, typed internals, runtime validation |
| ADR-011 | Tool execution | parallel default with sequential override | matches pi and avoids races |
| ADR-012 | Session format | opi tree JSONL | branch semantics without TS format lock-in |
| ADR-013 | Config format | TOML | comments and Rust ecosystem fit |
| ADR-014 | TUI | ratatui/crossterm | cross-platform Rust terminal stack |
| ADR-015 | Extension strategy | RPC/SDK and extension API before protocol adapters | matches pi's composition model; MCP is an extension/package candidate, not a core Phase 3 feature |
| ADR-017 | Transport stub | removed from public API | avoids undocumented public surface |
| ADR-018 | crates.io timing | quality-gated first publish | publish only after placeholder APIs are hidden or replaced and release gates pass |
| ADR-019 | Tool safety | allowlists, visibility, and hooks over core permission profiles | pi explicitly avoids built-in permission popups; environment-specific gates belong in extensions/packages or external sandboxes |
| ADR-020 | Context files | `AGENTS.md` / `CLAUDE.md` before `OPI.md` | preserves pi behavior and ecosystem convention |
| ADR-021 | Current upstream baseline | `.repo/pi-0.80.2` | `pi` architecture moved materially around `Models/Auth`, `AgentHarness`, sessions, and extension UI surfaces after the earlier baseline |

## 17. Non-Functional Requirements

Tier 1 targets:

- `x86_64-unknown-linux-gnu`;
- `aarch64-unknown-linux-gnu`;
- `x86_64-apple-darwin`;
- `aarch64-apple-darwin`;
- `x86_64-pc-windows-msvc`.

Tier 2 target: `aarch64-pc-windows-msvc`.

Rustls is preferred over OpenSSL for portable binary builds.

Accessibility requirements:

- respect `NO_COLOR`;
- expose essential state in non-interactive and JSON modes;
- do not rely only on color for errors, tools, or diffs;
- provide exit codes suitable for scripts.

Maintainability requirements:

- document public APIs with examples once implemented;
- include tests before phase tasks are marked done;
- track spec/code drift in changelog or issues when unavoidable;
- split large modules by responsibility.

## 18. Future Considerations

The architecture should not preclude MCP tools, remote tool execution, streaming
proxy services, editor integrations, pi session migration, provider OAuth,
image generation, custom extension UI, web/share flows, or plugin runtimes.
These are not core Phase 1-8 requirements and should generally arrive through
RPC, SDK, extensions, packages, or later reviewed ecosystem designs after the
Phase 10-14 depth work is stable.

## 19. Glossary

| Term | Definition |
|---|---|
| Provider | LLM backend such as Anthropic, OpenAI, Gemini, or Bedrock |
| API kind | wire protocol family, such as Anthropic Messages or OpenAI Chat Completions |
| Model | provider model with capabilities and limits |
| Agent loop | pure loop that sends context, receives assistant output, executes tools, and repeats |
| Agent | stateful wrapper around the loop |
| Harness | composition layer for sessions, compaction, and app hooks |
| CodingHarness | coding-agent-specific harness |
| AgentMessage | app-level message that may include custom/session-only data |
| Message | provider-facing user/assistant/tool-result message |
| Stream event | provider-level assistant delta or terminal event |
| Agent event | runtime lifecycle/message/tool event |
| Session event | queue/compaction/retry/session event |
| Session entry | persisted JSONL tree record |
| Steering | message injected while agent is running before next provider call |
| Follow-up | message queued until agent would otherwise stop |
| Compaction | summarizing older context while preserving recent state |
| Tool | model-callable capability with JSON Schema parameters |

## 20. References

- [pi source](https://github.com/earendil-works/pi)
- `.repo/pi-0.80.2/packages/ai/README.md`
- `.repo/pi-0.80.2/packages/ai/CHANGELOG.md`
- `.repo/pi-0.80.2/packages/agent/src/index.ts`
- `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md`
- `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md`
- `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md`
- `Cargo.toml`
- `CHANGELOG.md`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.claude/skills/opi-release/skill.md`
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
- [ratatui](https://ratatui.rs/)
- [MCP specification](https://modelcontextprotocol.io/)

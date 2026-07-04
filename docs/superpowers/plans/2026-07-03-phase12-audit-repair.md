# Phase 12 Audit Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair the confirmed Phase 12 provider-correctness issues from `docs/snapshots/phase12/audit.*.md` with failing regression tests first, then production fixes, then synchronized docs and changelog.

**Architecture:** Keep changes inside the existing provider adapter, agent-loop, diagnostics, and docs boundaries. Fix identity-sensitive stream mappers by carrying provider item IDs through their existing state machines; fix retry/redaction semantics at the public boundary where events and diagnostics are emitted. Treat test-quality findings as hardening unless they protect a confirmed production bug.

**Tech Stack:** Rust 2024, Cargo workspace, `opi-ai`, `opi-agent`, `opi-coding-agent`, `wiremock`, `tokio`, existing fixture helpers.

## Global Constraints

- Do not commit unless the user explicitly asks.
- Use only targeted file staging if a commit is later requested.
- Preserve existing provider breadth policy: no OAuth, subscription auth, browser feature, image generation, or broad provider catalog expansion.
- Use workspace dependencies only.
- After non-documentation code changes, run `cargo clippy --workspace --all-targets -- -D warnings`.
- If a test file is changed, run that test target before moving on.
- Update localized documentation counterparts when changing docs with existing `.zh.md` mirrors.

---

## Verification Baseline

Current HEAD: `31ad709`.

Existing tests passed before repair, which means the confirmed issues need new edge-case tests:

- `cargo test -p opi-ai --test openai_chat_fixtures --test openai_responses_fixtures --test bedrock_fixtures --test provider_error_classes --test proxy_support`
- `cargo test -p opi-agent --test retry_agent --test trace_envelope --test diagnostics_runtime`
- `cargo test -p opi-coding-agent --test provider_factory --test phase12_provider_correctness_docs --test proxy_config --test model_listing`

Current unrelated workspace state:

- Modified: `.gitignore`
- Untracked: `docs/snapshots/phase12/audit.codex.md`, `docs/snapshots/phase12/audit.glm5.2.md`, `docs/snapshots/phase12/audit.opus4.6.md`

Do not revert or stage these unless the user explicitly says they belong to this repair.

## Confirmed Finding Classification

| Bucket | Findings | Status |
|---|---|---|
| Production bug | OpenAI Responses ignores `output_index` for function-call deltas and `output_item.done`; `Completed` fallback updates state without `ToolCallEnd` | Confirmed |
| Production bug | Bedrock HTTP stream does not flush pending `Done` when metadata is absent | Confirmed |
| Production bug | Agent retry diagnostics label a post-retry partial-output suppression as retry exhaustion | Confirmed |
| Production bug | Provider-returned `ProviderError::Cancelled` maps to `AgentError::Provider` | Confirmed |
| Runtime gap | `usage_in_stream` is parsed and threaded but has no request-body effect and drops non-terminal usage | Confirmed |
| Runtime gap | OpenAI Chat `response_id` is captured only from role chunks | Confirmed |
| Docs/runtime mismatch | `require_assistant_after_tool_result` is metadata-only in code but README says runtime-enforced | Confirmed |
| Public-boundary hardening | `CompactionEnd.error_message` and `SessionPersistError.message` bypass event redaction | Confirmed |
| Security hardening | `safe_excerpt` and `SecretRedactor` token/query coverage diverge | Confirmed, low current blast radius |
| Cost semantics | Missing provider usage becomes zero usage and `$0.0`; docs claim explicit unknown | Confirmed, invasive |
| Test quality | Network retry lacks agent-loop test; cancellation/proxy/Bedrock frame tests are shallow | Confirmed |
| Documentation/process | `CHANGELOG.md` `[Unreleased]` is empty for Phase 12; EN/ZH docs overclaim some behavior | Confirmed |

## File Structure

- Modify `crates/opi-ai/src/openai_responses.rs`: carry Responses output-item identity through parser and mapper.
- Modify `crates/opi-ai/tests/openai_responses_fixtures.rs`: add interleaved and completion-fallback fixtures.
- Modify `crates/opi-ai/src/openai_chat.rs`: make `usage_in_stream` wire-visible and carry response IDs/usage updates across non-role chunks.
- Modify `crates/opi-ai/tests/openai_chat_fixtures.rs`: add usage-in-stream body/parser tests and role-less response-id fixtures.
- Modify `crates/opi-ai/src/bedrock/mod.rs`: flush pending `Done` in HTTP path and update tool-call partials during deltas.
- Modify `crates/opi-ai/tests/bedrock_fixtures.rs`: add HTTP no-metadata terminal test and partial tool-call delta assertion.
- Modify `crates/opi-ai/src/bedrock/event_stream.rs`: keep parser panic-safe for malformed frames.
- Modify `crates/opi-agent/src/agent_loop.rs`: distinguish retry exhaustion from partial-output retry suppression and route provider cancellation.
- Modify `crates/opi-agent/src/diagnostic.rs`: add a stable diagnostic code for retry suppression after partial output.
- Modify `crates/opi-agent/src/event.rs`: redact compaction/session persist errors.
- Modify `crates/opi-agent/tests/retry_agent.rs`, `crates/opi-agent/tests/diagnostics_runtime.rs`, `crates/opi-agent/tests/trace_envelope.rs`: add retry/cancel/redaction regressions.
- Modify `crates/opi-ai/src/http.rs` and `crates/opi-agent/src/streaming_proxy.rs`: align known secret patterns and query-parameter redaction.
- Modify `crates/opi-ai/tests/provider_error_classes.rs` and `crates/opi-agent/tests/trace_envelope.rs`: assert redaction hardening.
- Modify docs: `README.md`, `README.zh.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-coding-agent/README.md`, `crates/opi-coding-agent/README.zh.md`, `docs/opi-spec.md`, `docs/opi-spec.zh.md`, `docs/pi-alignment-matrix.md`, `docs/pi-alignment-matrix.zh.md`.
- Modify `CHANGELOG.md`: add Phase 12 repair entries under `[Unreleased]`.

### Task 1: OpenAI Responses Output-Item Identity

**Files:**
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Test: `crates/opi-ai/tests/openai_responses_fixtures.rs`

**Interfaces:**
- Produces: `ResponsesEvent::OutputItemDone { output_index, item_type }`
- Produces: mapper state keyed by `output_index`
- Consumes: `AssistantStreamEvent::ToolCallStart`, `ToolCallDelta`, `ToolCallEnd`, `Done`

- [ ] **Step 1: Add failing interleaved multi-tool test**

Add this test to `crates/opi-ai/tests/openai_responses_fixtures.rs`:

```rust
#[test]
fn responses_interleaved_tool_deltas_route_by_output_index() {
    let provider = OpenAiResponsesProvider::new("key".into(), None);
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_interleave","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_a","call_id":"call_a","name":"read_file","arguments":""}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_b","call_id":"call_b","name":"bash","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_a","call_id":"call_a","delta":"{\"path\":"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_b","call_id":"call_b","delta":"{\"cmd\":"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_a","call_id":"call_a","delta":"\"a.rs\"}"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_b","call_id":"call_b","delta":"\"ls\"}"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_a","call_id":"call_a","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":1,"item":{"type":"function_call","id":"fc_b","call_id":"call_b","name":"bash","arguments":"{\"cmd\":\"ls\"}"}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_interleave","model":"gpt-4o","usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new()));
    let ended: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .collect();

    assert_eq!(ended.len(), 2);
    assert_eq!(ended[0].id, "call_a");
    assert_eq!(ended[0].arguments, "{\"path\":\"a.rs\"}");
    assert_eq!(ended[1].id, "call_b");
    assert_eq!(ended[1].arguments, "{\"cmd\":\"ls\"}");
}
```

Expected before fix: FAIL, because both deltas are appended to the last tool call.

- [ ] **Step 2: Add failing non-function done test**

Add:

```rust
#[test]
fn responses_message_output_item_done_does_not_duplicate_tool_end() {
    let provider = OpenAiResponsesProvider::new("key".into(), None);
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_mixed","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_1","call_id":"call_1","delta":"{\"path\":\"a.rs\"}"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":1,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}

event: response.output_text.delta
data: {"type":"response.output_text.delta","output_index":1,"content_index":0,"delta":"ok"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":1,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"ok"}]}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_mixed","model":"gpt-4o","usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new()));
    let tool_ends = events
        .iter()
        .filter(|event| matches!(event, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_ends, 1);
}
```

Expected before fix: FAIL, because `OutputItemDone` finalizes the last tool call again.

- [ ] **Step 3: Implement identity-aware mapper state**

In `crates/opi-ai/src/openai_responses.rs`, change the event and state model:

```rust
enum ResponsesEvent {
    // existing variants...
    FunctionCallDelta {
        output_index: usize,
        delta: String,
    },
    OutputItemDone {
        output_index: usize,
        item_type: String,
    },
    // existing variants...
}

struct ToolCallState {
    output_index: usize,
    content_index: usize,
    id: String,
    name: String,
    arguments: String,
    ended: bool,
}
```

Parse `response.output_item.done` with identity:

```rust
"response.output_item.done" => {
    let output_index = data.output_index.unwrap_or(0);
    let item_type = data
        .item
        .and_then(|item| item.r#type)
        .unwrap_or_default();
    ParsedEvent::Valid(ResponsesEvent::OutputItemDone {
        output_index,
        item_type,
    })
}
```

When handling `OutputItemAdded { output_index, item }`, store `output_index` and `content_index` in `ToolCallState`. When handling `FunctionCallDelta`, find the matching state by `output_index`. When handling `OutputItemDone`, return no events unless `item_type == "function_call"` and the matching state is not already ended.

- [ ] **Step 4: Add completion fallback test**

Add a fixture where a function-call item never receives `response.output_item.done` but does receive `response.completed`. Assert one `ToolCallEnd` before `Done`.

Expected before fix: FAIL, because current `Completed` fallback updates `partial.content` but emits no `ToolCallEnd`.

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p opi-ai --test openai_responses_fixtures responses_interleaved_tool_deltas_route_by_output_index responses_message_output_item_done_does_not_duplicate_tool_end
cargo test -p opi-ai --test openai_responses_fixtures
```

Expected after fix: all pass.

### Task 2: OpenAI Chat Usage-In-Stream and Response IDs

**Files:**
- Modify: `crates/opi-ai/src/openai_chat.rs`
- Test: `crates/opi-ai/tests/openai_chat_fixtures.rs`

**Interfaces:**
- Consumes: `CompatConfig.usage_in_stream`
- Produces: request body field `stream_options.include_usage`
- Produces: `AssistantMessage::response_id` from any chunk carrying `id`
- Produces: `AssistantMessage.usage` updated by non-terminal usage events

- [ ] **Step 1: Add failing request-body test for `usage_in_stream`**

Add:

```rust
#[test]
fn build_request_body_usage_in_stream_emits_stream_options() {
    let provider = OpenAiChatProvider::new_for_profile(
        "key".into(),
        "https://example.test".into(),
        "compat".into(),
        CompatConfig {
            usage_in_stream: true,
            ..CompatConfig::default()
        },
        vec![],
        vec![ModelInfo {
            id: "model".into(),
            display_name: "model".into(),
            context_window: 128000,
            max_output_tokens: 4096,
            supports_images: false,
            supports_streaming: true,
            supports_thinking: false,
        }],
    );

    let body = provider.build_request_body(&basic_request("compat:model"));
    assert_eq!(body["stream_options"]["include_usage"], true);
}
```

Expected before fix: FAIL, because `stream_options` is absent.

- [ ] **Step 2: Add failing content-first response-id test**

Add:

```rust
#[test]
fn content_first_chunk_id_round_trips_into_response_id() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let sse = r#"data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new()));
    let response_id = events.into_iter().find_map(|event| match event {
        AssistantStreamEvent::Done { message, .. } => message.response_id,
        _ => None,
    });

    assert_eq!(response_id.as_deref(), Some("chatcmpl-content-first"));
}
```

Expected before fix: FAIL, because `ContentDelta` cannot carry `raw.id`.

- [ ] **Step 3: Add failing non-terminal usage test**

Add:

```rust
#[test]
fn non_terminal_usage_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let sse = r#"data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":0}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":1}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new()));
    let usage = events.into_iter().find_map(|event| match event {
        AssistantStreamEvent::Done { message, .. } => Some(message.usage),
        _ => None,
    }).expect("done event");

    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.output_tokens, 1);
}
```

Expected before fix: FAIL, because usage on role/content chunks is dropped.

- [ ] **Step 4: Implement event-level id and usage updates**

Update `OpenAiChatEvent` so every non-error event can carry metadata:

```rust
pub enum OpenAiChatEvent {
    RoleDelta { role: Option<String>, model: Option<String>, id: Option<String>, usage: Option<Usage> },
    ContentDelta { content: String, id: Option<String>, usage: Option<Usage> },
    ToolCallStart { index: usize, id: String, name: String, response_id: Option<String>, usage: Option<Usage> },
    ToolCallDelta { index: usize, arguments: String, id: Option<String>, usage: Option<Usage> },
    Finish { finish_reason: String, id: Option<String>, usage: Option<Usage> },
    Error { message: Option<String> },
}
```

In `OpenAiChatMapper::process`, update shared metadata before variant-specific handling:

```rust
fn update_metadata(&mut self, id: Option<String>, usage: Option<Usage>) {
    if let Some(response_id) = id {
        self.partial.response_id = Some(response_id);
    }
    if let Some(usage) = usage {
        self.partial.usage = usage;
    }
}
```

Call `update_metadata` in `RoleDelta`, `ContentDelta`, `ToolCallStart`, `ToolCallDelta`, and `Finish`.

In `build_request_body`, emit:

```rust
if compat.usage_in_stream {
    body["stream_options"] = serde_json::json!({ "include_usage": true });
}
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p opi-ai --test openai_chat_fixtures build_request_body_usage_in_stream_emits_stream_options content_first_chunk_id_round_trips_into_response_id non_terminal_usage_chunk_updates_done_usage
cargo test -p opi-ai --test openai_chat_fixtures
```

Expected after fix: all pass.

### Task 3: Compat Metadata Honesty

**Files:**
- Modify: `crates/opi-ai/README.md`
- Modify: `crates/opi-ai/README.zh.md`
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Test: `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs`

**Interfaces:**
- Produces: docs that say `require_assistant_after_tool_result` is metadata-only unless a first-class adapter implements the legacy synthesis.
- Produces: guard test that rejects "runtime enforced" wording for that flag.

- [ ] **Step 1: Add docs guard**

In `phase12_provider_correctness_docs.rs`, extend the provider-docs test with an assertion that `crates/opi-ai/README.md` and `.zh.md` do not claim runtime enforcement for `require_assistant_after_tool_result`.

Expected before doc fix: FAIL because the README says "enforced as a runtime check".

- [ ] **Step 2: Reword EN/ZH docs**

Change the flag row to this English meaning:

```markdown
| `require_assistant_after_tool_result` | Metadata-only compatibility marker for legacy endpoints; opi does not synthesize or enforce the extra assistant turn in the shared adapter. |
```

Mirror the same claim in `crates/opi-ai/README.zh.md`, and make `docs/opi-spec.md` / `.zh.md` say the flag is represented, not runtime-enforced.

- [ ] **Step 3: Run tests**

Run:

```powershell
cargo test -p opi-coding-agent --test phase12_provider_correctness_docs
```

Expected after fix: pass.

### Task 4: Bedrock Stream Completion and Tool-Delta Partial State

**Files:**
- Modify: `crates/opi-ai/src/bedrock/mod.rs`
- Test: `crates/opi-ai/tests/bedrock_fixtures.rs`

**Interfaces:**
- Produces: HTTP `stream_http` flushes pending `Done` like fixture path.
- Produces: `ToolCallDelta.partial` reflects accumulated tool arguments during streaming.

- [ ] **Step 1: Add failing HTTP no-metadata test**

Add a wiremock test that returns Bedrock event-stream frames ending with `messageStop` and no `metadata`. Use the existing `build_bedrock_stream` helper in `crates/opi-ai/tests/bedrock_fixtures.rs`, which wraps `event_stream::build_test_frame`.

Assertion:

```rust
assert!(
    events.iter().any(|event| matches!(event, AssistantStreamEvent::Done { .. })),
    "HTTP Bedrock stream must flush pending Done when metadata is absent"
);
```

Expected before fix: FAIL, because `stream_http` only checks `mapper.saw_done`.

- [ ] **Step 2: Add failing partial-delta assertion**

In the Bedrock tool-call fixture test, assert that the `ToolCallDelta` event's `partial.content` carries the accumulated tool-call `arguments`, not only the final `ToolCallEnd`.

Expected before fix: FAIL, because `BedrockDelta::ToolUse` only updates `blocks.last_mut()`.

- [ ] **Step 3: Implement fixes**

In `stream_http`, after the byte-stream loop and before the `!mapper.saw_done` error:

```rust
if let Some(pending) = mapper.flush_pending() {
    let _ = tx.send(Ok(pending)).await;
}
```

In `BedrockDelta::ToolUse`, after appending to `partial_input`, mirror the update into `self.partial.content.last_mut()`:

```rust
if let Some(AssistantContent::ToolCall { tool_call }) = self.partial.content.last_mut() {
    if let Some(BlockState::ToolUse { partial_input, .. }) = self.blocks.last() {
        tool_call.arguments = partial_input.clone();
    }
}
```

- [ ] **Step 4: Run tests**

Run:

```powershell
cargo test -p opi-ai --test bedrock_fixtures
```

Expected after fix: pass.

### Task 5: Agent Retry and Cancellation Semantics

**Files:**
- Modify: `crates/opi-agent/src/diagnostic.rs`
- Modify: `crates/opi-agent/src/agent_loop.rs`
- Test: `crates/opi-agent/tests/retry_agent.rs`
- Test: `crates/opi-agent/tests/diagnostics_runtime.rs`
- Test: `crates/opi-agent/tests/trace_envelope.rs`

**Interfaces:**
- Produces: `CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT`
- Produces: `ProviderError::Cancelled -> AgentError::Cancelled`
- Produces: agent-loop test coverage for `ProviderError::Network` retry

- [ ] **Step 1: Add Network retry test**

Add to `retry_agent.rs`:

```rust
#[tokio::test]
async fn retry_on_network_error_then_succeed() {
    let provider = Arc::new(MockProvider::new(vec![
        MockResponse::Error(ProviderError::Network("connection reset".into())),
        MockResponse::Events(test_support::text_response("success after network retry")),
    ]));
    let events = Arc::new(Mutex::new(Vec::new()));
    let result = agent_loop(
        make_context(provider, events.clone()),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        capture_events(events.clone()),
        CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok(), "network error should retry and then succeed: {result:?}");
    assert!(events.lock().unwrap().iter().any(|e| matches!(e, AgentEvent::AutoRetryStart { .. })));
    assert!(events.lock().unwrap().iter().any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. })));
}
```

Expected before fix: PASS today, but it pins the behavior.

- [ ] **Step 2: Add failing post-retry partial suppression test**

Add:

```rust
#[tokio::test]
async fn retry_after_prior_attempt_then_partial_stream_error_is_not_exhausted() {
    let mut partial_events = test_support::text_response("partial after retry");
    partial_events.pop();

    let provider = Arc::new(MockProvider::new(vec![
        MockResponse::Error(ProviderError::RateLimited { retry_after_ms: Some(1) }),
        MockResponse::EventsThenError(
            partial_events,
            ProviderError::RateLimited { retry_after_ms: Some(1) },
        ),
    ]));

    let sink = Arc::new(RecordingDiagnosticSink::default());
    let events = Arc::new(Mutex::new(Vec::new()));
    let result = agent_loop(
        make_context_with_sink(provider, events.clone(), sink.clone()),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        capture_events(events.clone()),
        CancellationToken::new(),
    )
    .await;

    assert!(result.is_err());
    let codes: Vec<_> = sink.records().iter().map(|d| d.code).collect();
    assert!(codes.contains(&CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT));
    assert!(!codes.contains(&CODE_PROVIDER_RETRY_EXHAUSTED));
}
```

Expected before fix: FAIL, because the code emits `CODE_PROVIDER_RETRY_EXHAUSTED`.

- [ ] **Step 3: Add failing provider-cancel routing test**

Add a test where `MockProvider` returns `ProviderError::Cancelled` and assert `Err(AgentError::Cancelled)`.

Expected before fix: FAIL, because it returns `AgentError::Provider(...)`.

- [ ] **Step 4: Implement retry classification**

Add in `diagnostic.rs`:

```rust
pub const CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT: &str =
    "provider_retry_suppressed_after_partial_output";
```

In `agent_loop.rs`, before the generic `retry_attempt > 0` exhausted block, branch:

```rust
let retry_suppressed_after_partial_output =
    e.is_retryable() && stream_delivered_content && retry_attempt > 0;

if retry_suppressed_after_partial_output {
    emit_public_event(
        &events,
        AgentEvent::AutoRetryEnd {
            success: false,
            attempt: retry_attempt,
            final_error: Some(e.to_string()),
        },
    );
    observe(
        &diagnostic_sink,
        &trace,
        Diagnostic::new(
            Severity::Warning,
            CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT,
            SOURCE_PROVIDER,
            "retry suppressed after partial provider output",
        )
        .details(json!({ "attempts": retry_attempt, "max_attempts": max_attempts })),
    );
} else if retry_attempt > 0 {
    // existing retry exhausted diagnostic
}
```

In the final error mapping:

```rust
return Err(match &e {
    opi_ai::provider::ProviderError::AuthFailed(msg) => AgentError::AuthFailed(msg.clone()),
    opi_ai::provider::ProviderError::Cancelled => AgentError::Cancelled,
    _ => AgentError::Provider(e.to_string()),
});
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p opi-agent --test retry_agent
cargo test -p opi-agent --test diagnostics_runtime
cargo test -p opi-agent --test trace_envelope
```

Expected after fix: pass.

### Task 6: Public Event Redaction and Secret Pattern Alignment

**Files:**
- Modify: `crates/opi-agent/src/event.rs`
- Modify: `crates/opi-ai/src/http.rs`
- Modify: `crates/opi-agent/src/streaming_proxy.rs`
- Test: `crates/opi-agent/tests/trace_envelope.rs`
- Test: `crates/opi-ai/tests/provider_error_classes.rs`

**Interfaces:**
- Produces: public event redaction for compaction/session persistence errors.
- Produces: shared coverage for Google API keys, GitHub token prefixes, bearer tokens, credentialed URLs, and common query secret keys.

- [ ] **Step 1: Add failing public-event redaction tests**

Add to `trace_envelope.rs`:

```rust
#[test]
fn compaction_and_session_persist_errors_are_redacted_for_public() {
    let secret = "sk-ant-1234567890abcdefghijklmnopqrstuv";

    let compaction = AgentEvent::CompactionEnd {
        reason: CompactionReason::Manual,
        result: None,
        aborted: true,
        error_message: Some(format!("failed with {secret}")),
    }
    .redacted_for_public();

    match compaction {
        AgentEvent::CompactionEnd { error_message: Some(message), .. } => {
            assert!(!message.contains(secret));
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let persist = AgentEvent::SessionPersistError {
        message: format!("persist failed with {secret}"),
    }
    .redacted_for_public();

    match persist {
        AgentEvent::SessionPersistError { message } => {
            assert!(!message.contains(secret));
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
```

Expected before fix: FAIL.

- [ ] **Step 2: Add failing safe-excerpt query secret test**

Add a provider error class test that mounts a 500 body containing:

```text
https://example.test/path?api_key=opaque-secret-token&token=another-secret
```

Assert neither raw value survives in `ProviderError::ProviderSide`.

Expected before fix: FAIL.

- [ ] **Step 3: Implement event redaction**

In `AgentEvent::redacted_for_public` add explicit arms:

```rust
AgentEvent::CompactionEnd { reason, result, aborted, error_message } => AgentEvent::CompactionEnd {
    reason: *reason,
    result: result.clone(),
    aborted: *aborted,
    error_message: error_message
        .as_ref()
        .map(|message| redact_text(message, RedactionMode::Summary)),
},
AgentEvent::SessionPersistError { message } => AgentEvent::SessionPersistError {
    message: redact_text(message, RedactionMode::Summary),
},
```

- [ ] **Step 4: Implement query redaction and pattern alignment**

In `http.rs`, add a regex for common query secret keys:

```rust
static QUERY_SECRET_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn query_secret_re() -> &'static regex::Regex {
    QUERY_SECRET_RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)([?&](?:api_key|apikey|key|token|access_token|refresh_token|secret|password)=)[^&#\s]+",
        )
        .expect("valid query-secret regex")
    })
}
```

Apply it in `safe_excerpt`:

```rust
let scrubbed = query_secret_re()
    .replace_all(&scrubbed, "${1}[REDACTED]")
    .into_owned();
```

Align `SecretRedactor` value patterns with `safe_excerpt`: include `AIza[0-9A-Za-z_-]{35,}` and use a GitHub token pattern that covers `gh[pousr]_` plus `ghs_`.

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p opi-agent --test trace_envelope compaction_and_session_persist_errors_are_redacted_for_public
cargo test -p opi-ai --test provider_error_classes
```

Expected after fix: pass.

### Task 7: Bedrock Event-Stream Parser Hardening

**Files:**
- Modify: `crates/opi-ai/src/bedrock/event_stream.rs`

**Interfaces:**
- Produces: malformed binary frames never panic and leave incomplete bytes buffered or return no frames.

- [ ] **Step 1: Add adversarial parser tests**

Add tests in the existing `#[cfg(test)]` module for:

```rust
#[test]
fn parse_frames_ignores_header_length_past_total_length_without_panic() { /* bytes with headers_len > total_len */ }

#[test]
fn parse_frames_ignores_string_header_value_past_header_region_without_panic() { /* malformed header value length */ }

#[test]
fn parse_frames_ignores_garbage_shorter_than_min_frame_without_panic() { /* short garbage */ }

#[test]
fn parse_frames_ignores_bad_crc_without_panic() { /* corrupt CRC */ }
```

Expected before fix: at least one test may fail if the parser returns a frame or panics.

- [ ] **Step 2: Tighten parser checks only if tests reveal a hole**

If a test fails, add explicit bounds checks at the failing boundary. Do not redesign the parser.

- [ ] **Step 3: Run tests**

Run:

```powershell
cargo test -p opi-ai --lib bedrock::event_stream
```

Expected after fix: pass.

### Task 8: Cancellation and Proxy Test Fidelity

**Files:**
- Modify: `crates/opi-ai/tests/anthropic_fixtures.rs`
- Modify: `crates/opi-ai/tests/openai_chat_fixtures.rs`
- Modify: `crates/opi-ai/tests/openai_responses_fixtures.rs`
- Modify: `crates/opi-ai/tests/openrouter_fixtures.rs`
- Modify: `crates/opi-ai/tests/mistral_fixtures.rs`
- Modify: `crates/opi-ai/tests/gemini_fixtures.rs`
- Modify: `crates/opi-ai/tests/vertex_fixtures.rs`
- Modify: `crates/opi-ai/tests/azure_openai_fixtures.rs`
- Modify: `crates/opi-ai/tests/provider_lifecycle.rs`
- Modify: `crates/opi-ai/tests/proxy_support.rs`

**Interfaces:**
- Produces: cancellation tests that prove cancellation exits before terminal fixture completion, or honest names for tests that only prove no hang.
- Produces: one HTTP proxy routing test if practical with `reqwest::Proxy` and local wiremock.

- [ ] **Step 1: Replace one cancellation test with a deterministic slow stream**

Start with `openai_chat_fixtures.rs`. Mount a streaming body that emits `Start`/first delta and then stalls without terminal `Done`; cancel after first event; assert the stream ends within 200 ms.

- [ ] **Step 2: Generalize only after one provider proves the pattern**

Apply the same helper to the OpenAI-compatible inherited families first. If a provider cannot use slow-body wiremock cleanly, rename the current test to `stream_drains_without_hang_after_cancel` and document the limitation.

- [ ] **Step 3: Remove vacuous assertion**

In `provider_lifecycle.rs`, replace `let _ = got_terminal;` with either:

```rust
assert!(
    got_terminal || cancel.is_cancelled(),
    "stream should either terminate or observe cancellation"
);
```

or rename the test to make it a no-hang smoke test.

- [ ] **Step 4: Add one proxy routing test only if it can be local and deterministic**

Use HTTP, not HTTPS, to avoid CONNECT complexity. Configure `HttpClientBuilder::new().proxy(ProxyConfig { url: Some(proxy.uri()), no_proxy: None })`, issue a request to an HTTP origin through a provider, and assert the proxy mock received the absolute-form request. If wiremock cannot observe proxy semantics reliably, document the proxy-routing test as deferred and keep existing config tests.

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p opi-ai --test openai_chat_fixtures stream_cancellation_aborts_before_completion
cargo test -p opi-ai --test proxy_support
```

Expected after fix: pass.

### Task 9: Usage Unknown vs Zero-Cost Semantics

**Files:**
- Modify: `crates/opi-ai/src/stream.rs`
- Modify provider mappers that create `Usage`
- Modify: `crates/opi-coding-agent/src/session_coordinator.rs`
- Modify tests: `crates/opi-ai/tests/usage_cost.rs`, provider fixture tests, `crates/opi-coding-agent/tests/session_runtime.rs`

**Interfaces:**
- Produces: explicit distinction between provider-reported zero usage and missing usage.
- Produces: `cost_summary() == None` when any accumulated turn has unknown usage or unknown pricing.

- [ ] **Step 1: Add failing cost test**

In `usage_cost.rs`, add:

```rust
#[test]
fn missing_usage_is_not_reported_as_known_zero_cost() {
    let usage = Usage::unknown();
    assert!(!usage.is_reported());
}
```

In `session_runtime.rs`, add a session coordinator test where one assistant turn carries unknown usage and known pricing, then assert `coord.cost_summary().is_none()`.

Expected before fix: FAIL because no unknown state exists.

- [ ] **Step 2: Add additive usage-known flag**

Prefer an additive field over changing `AssistantMessage.usage` to `Option<Usage>`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_read_tokens: u32,
    #[serde(default)]
    pub cache_write_tokens: u32,
    #[serde(default)]
    pub reported: bool,
}

impl Default for Usage {
    fn default() -> Self {
        Self::unknown()
    }
}

impl Usage {
    pub fn unknown() -> Self {
        Self { input_tokens: 0, output_tokens: 0, cache_read_tokens: 0, cache_write_tokens: 0, reported: false }
    }

    pub fn reported(input_tokens: u32, output_tokens: u32, cache_read_tokens: u32, cache_write_tokens: u32) -> Self {
        Self { input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, reported: true }
    }

    pub fn is_reported(&self) -> bool {
        self.reported
    }
}
```

Update provider mappers to use `Usage::reported(...)` only when a provider usage object exists. Keep missing-usage fixtures passing by changing assertions to "unknown usage with zero token fields", not "known zero usage".

- [ ] **Step 3: Track unknown usage through cumulative usage**

Add to `CumulativeUsage`:

```rust
unknown_turns: u32,
```

In `accumulate`:

```rust
if !turn.reported {
    self.unknown_turns += 1;
}
```

Expose:

```rust
pub fn has_unknown_usage(&self) -> bool {
    self.unknown_turns > 0
}
```

In `SessionCoordinator::cost_summary`, return `None` when `self.usage.has_unknown_usage()`.

- [ ] **Step 4: Run tests**

Run:

```powershell
cargo test -p opi-ai --test usage_cost
cargo test -p opi-ai --test anthropic_fixtures --test openai_chat_fixtures --test openai_responses_fixtures
cargo test -p opi-coding-agent --test session_runtime
```

Expected after fix: pass.

### Task 10: Documentation, Changelog, and Guard Synchronization

**Files:**
- Modify: `CHANGELOG.md`
- Modify EN/ZH docs listed in File Structure
- Modify: `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs`

**Interfaces:**
- Produces: docs that match repaired runtime behavior and accepted deferrals.
- Produces: `[Unreleased]` entries for user-visible Phase 12 corrections.

- [ ] **Step 1: Update `CHANGELOG.md`**

Under `## [Unreleased]`, add:

```markdown
### Changed

- `opi-ai`: OpenAI-compatible streaming usage now requests usage chunks when `usage_in_stream` is enabled and preserves usage/response IDs from role-less streaming chunks.
- `opi-agent`: retry diagnostics now distinguish exhausted retry budgets from retry suppression after partial provider output.

### Fixed

- `opi-ai`: OpenAI Responses tool-call deltas and item completion now route by output item identity instead of the last observed tool call.
- `opi-ai`: Bedrock HTTP streaming now flushes a pending terminal `Done` event when metadata is absent.
- `opi-agent`: provider-returned cancellations now surface as `AgentError::Cancelled`.
- `opi-agent`: compaction and session-persistence public events redact secret-looking error text.
```

If Task 9 is deferred, do not claim usage unknown semantics are fixed.

- [ ] **Step 2: Update docs**

Update EN/ZH docs to say:

- `usage_in_stream` emits OpenAI Chat `stream_options.include_usage` and preserves usage updates from any streaming chunk.
- `require_assistant_after_tool_result` is metadata-only in the shared adapter.
- Response IDs are captured from any OpenAI Chat chunk carrying `id`, not only role chunks.
- If Task 9 is deferred, cost docs must say missing usage currently maps to zero token fields and explicit cost-unknown for missing usage remains deferred.

- [ ] **Step 3: Strengthen docs guard**

Make `phase12_provider_correctness_docs.rs` source-anchor the `CompatConfig` field count and reject docs that claim:

```text
require_assistant_after_tool_result is enforced as a runtime check
```

- [ ] **Step 4: Run docs tests**

Run:

```powershell
cargo test -p opi-coding-agent --test phase12_provider_correctness_docs
```

Expected after fix: pass.

## Final Verification Gate

After all selected tasks are complete:

```powershell
cargo fmt --all
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p opi-ai --test openai_chat_fixtures --test openai_responses_fixtures --test bedrock_fixtures --test provider_error_classes --test proxy_support --test usage_cost
cargo test -p opi-agent --test retry_agent --test diagnostics_runtime --test trace_envelope
cargo test -p opi-coding-agent --test provider_factory --test phase12_provider_correctness_docs --test proxy_config --test model_listing --test session_runtime
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Expected final state:

- All targeted tests pass.
- Workspace clippy passes with `-D warnings`.
- No docs claim behavior that is metadata-only or deferred.
- `CHANGELOG.md` has `[Unreleased]` entries for every user-visible change.
- `git status --short` shows only files intentionally changed by the repair plus pre-existing unrelated workspace changes.

## Plan Self-Review

- Spec coverage: covers Phase 12 Success Criteria 2, 5, 6, 8, and documentation/changelog fallout. Success Criteria 1/3/4 are touched only where confirmed edge bugs or test-quality gaps were found.
- Empty-step scan: no incomplete-marker steps remain. Task 8 explicitly allows deferral only after a concrete local-proxy attempt because proxy behavior tests may be impractical with existing test tools.
- Type consistency: new names are stable across tasks: `CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT`, `provider_retry_suppressed_after_partial_output`, `Usage::unknown`, `Usage::reported`, and `Usage::is_reported`.
- Scope check: this is a large but coherent repair pass. If execution time must be reduced, implement Tasks 1-6 and 10 first; Tasks 7-9 are hardening/invasive cleanup.

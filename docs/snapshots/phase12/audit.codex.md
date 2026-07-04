# Phase 12 Provider Correctness Independent Audit

Audit date: 2026-07-03

Scope: Phase 12 provider correctness changes, bounded by
`docs/snapshots/phase12/opi-impl-state.json`,
`docs/superpowers/specs/2026-06-24-phase12-provider-correctness-design.md`,
and the actual `f3a61f3..31ad709` diff.

Independence note: I did not open, search, quote, or summarize any existing
audit or review reports in the repository. The requested
`opi-impl-state.json` contains phase/task status metadata; I used it only to
identify task, commit, and touched-file scope. Findings below are based on
code, tests, config, docs, and the actual diff.

## Verification

- `cargo test -p opi-ai --test openai_chat_fixtures --test openai_responses_fixtures`
  - Result: passed, 84 tests.
- `cargo test -p opi-coding-agent --test provider_factory`
  - Result: passed, 30 tests.
- `cargo test -p opi-agent --test retry_agent`
  - Result: passed, 8 tests.

These tests do not cover the edge cases called out below. Passing the current
suite does not prove those boundaries correct.

## Findings

### P1 - OpenAI Responses tool-call stream ignores `output_index`, corrupting interleaved or mixed output items

Evidence:

- `crates/opi-ai/src/openai_responses.rs:191-195` defines
  `FunctionCallDelta { output_index, delta }`.
- `crates/opi-ai/src/openai_responses.rs:271-277` parses `output_index` for
  function-call deltas, but collapses `response.output_item.done` into
  `OutputItemDone` without `output_index` or item type.
- `crates/opi-ai/src/openai_responses.rs:434-455` ignores `output_index` and
  always appends argument deltas to `self.tool_calls.last_mut()` and the last
  tool-call content block.
- `crates/opi-ai/src/openai_responses.rs:463-494` treats any later
  `OutputItemDone` as finalizing the last tool call whenever at least one tool
  call has appeared.

Cause:

The Responses stream format carries item identity (`output_index`, `item_id`,
`call_id`) because multiple output items can exist in one response. The mapper
parses part of that identity but then discards it, so it cannot route deltas or
done events to the correct tool-call state.

Impact:

- Multiple Responses function calls with interleaved argument deltas can attach
  an earlier call's arguments to a later call.
- A text/message `response.output_item.done` after a function-call item can emit
  a duplicate `ToolCallEnd` for the last function call.
- Agent-side execution can receive the wrong tool arguments or duplicate
  provider lifecycle events. Phase 12 currently tests sequential tool-call
  fixtures, not this edge.

Suggested fix:

Track tool-call state by `output_index` or `call_id` instead of "last tool
call". Preserve `output_index` and item type on `OutputItemDone`; finalize only
the matching `function_call` item. Add fixtures for interleaved multi-tool
deltas and for a message item done after a tool-call item.

### P1 - `usage_in_stream` is parsed from config but has no effective stream behavior

Evidence:

- `crates/opi-coding-agent/src/config.rs:192-193` exposes `usage_in_stream`.
- `crates/opi-coding-agent/src/provider_factory.rs:564-576` passes it into
  `CompatConfig`.
- `crates/opi-ai/src/openai_chat.rs:580-588` stores it on `CompatConfig`.
- `crates/opi-ai/src/openai_chat.rs:810-865` builds the request body but never
  uses `compat.usage_in_stream`.
- `crates/opi-ai/src/openai_chat.rs:207-241` only carries usage into a `Finish`
  event when the chunk is terminal or has no choices.
- `crates/opi-ai/src/openai_chat.rs:249-292` returns role/content/tool events
  and drops `usage` present on non-terminal chunks.
- `crates/opi-ai/src/openai_chat.rs:502-504` updates `partial.usage` only from
  the `Finish` event.
- `crates/opi-ai/README.md:135` documents the flag as expecting usage deltas in
  the streaming response.

Cause:

The flag is treated as representational metadata. It does not request streaming
usage from the provider, and the parser does not preserve usage if it arrives
with a non-terminal role/content/tool chunk.

Impact:

Config-driven OpenAI-compatible profiles that require streaming usage support
can silently report zero or stale usage. That affects cumulative token usage,
best-effort cost display, persisted session metadata, and Phase 13 handoff
assumptions that rely on provider-correct usage data.

Suggested fix:

Define the flag's concrete wire semantics. If it means "request usage in the
stream", emit the provider-appropriate field, for example OpenAI Chat
`stream_options: { include_usage: true }`, when enabled. If it means "parse
usage whenever it appears", extend the event model or mapper state so
non-terminal chunks update usage. Add a config-through-HTTP fixture where usage
appears before the finish chunk and assert the final `Done` message carries it.

### P2 - `require_assistant_after_tool_result` is user-configurable but unused; docs and implementation disagree

Evidence:

- `crates/opi-coding-agent/src/config.rs:203` exposes
  `require_assistant_after_tool_result`.
- `crates/opi-coding-agent/src/provider_factory.rs:564-576` passes it into
  `CompatConfig`.
- `crates/opi-ai/src/openai_chat.rs:573-578` says the adapter records the flag
  as metadata and does not alter message ordering.
- `crates/opi-ai/src/openai_chat.rs:1136-1165` serializes a `ToolResult`
  directly as a `role: "tool"` message and only checks
  `tool_result_name_field`; it never checks
  `require_assistant_after_tool_result`.
- `crates/opi-ai/README.md:139` says the flag is "enforced as a runtime check",
  but `rg require_assistant_after_tool_result` shows no production use beyond
  parsing, construction, and representation tests.

Cause:

The flag exists at the config and provider-construction layers, but the runtime
does not enforce, reject, or synthesize the legacy assistant-after-tool-result
contract.

Impact:

Users can configure a profile that claims this compatibility requirement, and
the request will still be sent in the default OpenAI Chat message order. A
compatible endpoint that actually requires the extra assistant turn can reject
tool-result follow-up requests or receive a conversation shape that the config
claimed would be handled. The docs also overstate implemented behavior.

Suggested fix:

Choose one behavior and make code/docs/tests consistent:

- If this is supported, implement a runtime validation or serialization path
  for the required assistant-after-tool-result shape and add a
  factory-through-HTTP fixture.
- If this is intentionally unsupported, remove the user-facing flag or make it
  a documented hard rejection (`ProviderError::UnsupportedCapability` or config
  error) and update `README.md`, `README.zh.md`, `docs/opi-spec.md`, and guard
  tests to say the behavior is deferred or needs a first-class adapter.

### P2 - OpenAI Chat `response_id` is only captured from role chunks

Evidence:

- `crates/opi-ai/src/openai_chat.rs:172-193` includes `id` only on
  `RoleDelta`; `ContentDelta`, `ToolCallStart`, `ToolCallDelta`, and `Finish`
  cannot carry it.
- `crates/opi-ai/src/openai_chat.rs:279-285` attaches `raw.id` only when the
  chunk has `delta.role`.
- `crates/opi-ai/src/openai_chat.rs:288-292` drops `raw.id` when the first
  useful chunk is text content.
- `crates/opi-ai/src/openai_chat.rs:333-339` updates
  `partial.response_id` only in the `RoleDelta` branch.
- `crates/opi-ai/tests/openai_chat_fixtures.rs:247-255` tests only the
  role-first fixture.

Cause:

The parser assumes a role-bearing first chunk. OpenAI-compatible providers can
still include the chunk id on content, tool, finish, or usage-only chunks; those
ids are currently discarded unless the stream also includes a role delta.

Impact:

For role-less or content-first compatible streams, `AssistantMessage::response_id`
is `None` even though the provider returned an id. This weakens response-id
round-trip guarantees, session metadata, cache/session-affinity diagnostics,
and the Phase 13 dependency on provider-correct response data.

Suggested fix:

Carry `raw.id` through every parsed event variant or add a separate response-id
update path in the mapper. Update `partial.response_id` whenever an id appears,
not only on `RoleDelta`. Add fixtures for content-first, tool-first,
final-only, and usage-only OpenAI-compatible chunks that include `id`.

## Residual Test Gaps

- Existing Responses tests cover sequential multiple tool calls, but not
  interleaved `output_index` deltas or non-function `output_item.done` after a
  function call.
- Existing OpenAI Chat tests cover final usage chunks and role-first response
  ids, but not non-terminal usage deltas or role-less streams.
- Existing compat profile tests verify field representation and factory
  threading, but not runtime semantics for `usage_in_stream` or
  `require_assistant_after_tool_result`.

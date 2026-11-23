# Validate tools, replay context, and inspect requests

[简体中文](utilities.zh.md) · [SDK quick start](../README.md#quick-start)

## Validate final tool arguments

Compile the active tool's trusted JSON Schema with `schema::compile` at
registration. For a UI preview, keep a `ToolCallArgumentsStream` for each
`content_index` and append the corresponding `ToolCallDelta` fragments.
Call `snapshot()` when rendering. A snapshot can be incomplete or lag behind
the raw text. Wait for successful chat completion before considering tools
for execution.

On `StopReason::ToolUse`, validate each final `ToolCall` against the tools
active for that request. Pass the normalized object to your handler only
after validation. The app still decides whether the tool is allowed to run.
Supply a tool result with the original call ID and name for each call.
Use `ToolResultMessage::not_executed` for a call you declined. [Runnable tool loop](../examples/tool_loop.rs)

```rust
use opi_ai::message::ToolCall;
use opi_ai::schema::{CompiledSchema, ToolArgumentOutcome, validate_tool_call};
use serde_json::Value;

pub fn validated_arguments(
    call: &ToolCall,
    tools: &[(&str, &CompiledSchema)],
) -> Result<Value, Box<dyn std::error::Error>> {
    if call.arguments_error.is_some() {
        return Err("tool call has malformed final arguments".into());
    }
    let arguments = Value::Object(serde_json::from_str(&call.arguments)?);
    let ToolArgumentOutcome::Valid { normalized } =
        validate_tool_call(tools, &call.name, &arguments, None)??
    else {
        return Err("tool call arguments failed validation".into());
    };
    Ok(normalized)
}
```

This example uses `serde_json = "1"`. A parse marker, unknown tool, invalid
schema or mismatched arguments all stop execution. The validator can coerce
supported values and returns a new object while preserving the original
tool call for history.

For schema-constrained generation, set `ToolDef::constrained_sampling` to a
`ConstrainedSampling::JsonSchema` policy. `StrictPreference::Require` rejects
an unsupported route or schema. `Prefer` can fall back to ordinary tool
arguments. Validate the final call before execution in either case.

## Save and replay a conversation

Build a `TranscriptContext` with `transcript::normalize_context`. Store
assistant turns and matching tool results in order before sending the next
request. Use `append_system_change` to record later instruction or tool
changes at the point where they happened. You can serialize the canonical
`TranscriptContext` with Serde and restore it into the next
`Request::context`.

To continue with another provider, select its `ChatModelRecord` and reuse the
saved transcript. The SDK maps matching tool call/result IDs for the target
wire and can omit incompatible reasoning or collapse system messages.
Use `Request::projection.replay = ReplayProjection::Strict` to reject
lossy replay and `Request::projection.temporal = TranscriptProjection::Strict`
to require the timing of system and tool changes. Inspect the returned
message's `projection` for the applied transformations.

## Inspect requests and handle failures

Use `Request::callbacks` for the projected payload, response headers and
parsed raw events on supporting adapters. `with_on_payload` can observe or
replace the body. `with_on_response` exposes the HTTP status and headers.
`with_on_raw` sees parsed provider events before public event mapping.
Callback errors stop the call and prevent automatic retry. For request spans
and metrics, pass a host recorder through `Request::telemetry_context`.

For a failed chat call, check `CallFailure::cause()` and save
`message()` if you need the partial result. Use `message::is_context_overflow`
to decide when to shorten context. On a successful `Length` result,
`message::is_recoverable_length` checks whether generation stopped before
the requested output budget. This can inform a bounded retry after reducing
context. For failed replies, use
`retry::classify_whole_call_retry` to distinguish transient failures from
abort, quota exhaustion and context overflow. Each whole-call retry starts
a new generation.
[Cancellation and cleanup](transport.md#finish-or-cancel-a-chat-call)

# 校验工具参数、回放上下文与观察请求

[English](utilities.md) · [SDK 快速开始](../README.zh.md#快速开始)

## 校验最终工具参数

注册工具时，用 `schema::compile` 编译每个可信工具的 JSON Schema。
需要流式预览时，按 `content_index` 为每个工具维护一个
`ToolCallArgumentsStream`，追加对应的 `ToolCallDelta` 片段。
渲染时调用 `snapshot()`。预览可能不完整，也可能落后于收到的原始文本。
等聊天成功结束后，再决定是否执行工具。

收到 `StopReason::ToolUse` 后，将每个最终 `ToolCall` 与此次请求启用的工具对照
校验。校验通过后，把归一化的对象传给 handler。应用负责判断是否允许执行。
每次调用都要用原有的 ID 和工具名补一条结果。被拒绝的调用可用
`ToolResultMessage::not_executed` 标记。
[完整工具循环](../examples/tool_loop.rs)

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

示例需要 `serde_json = "1"`。解析错误标记、未知工具、无效 Schema 或参数不匹配
都会阻止执行。校验器可转换支持的值并返回新对象，原始调用可继续保存在历史中。

要在生成时约束参数，可给 `ToolDef::constrained_sampling` 设置
`ConstrainedSampling::JsonSchema`。`StrictPreference::Require` 会在路由或 Schema
不支持时拒绝请求，`Prefer` 可以回退到普通参数生成。两种模式都要校验最终调用。

## 保存和回放对话

用 `transcript::normalize_context` 构建 `TranscriptContext`。发起续轮前，按原有
顺序保存助手消息和对应的工具结果。用 `append_system_change` 在指令或工具发生变化
的位置记录变更。可用 Serde 序列化规范化的 `TranscriptContext`，再恢复为下一次的
`Request::context`。

切换供应商续聊时，选取目标供应商的 `ChatModelRecord` 并复用保存的记录。SDK 会
按目标协议映射相互配对的工具调用和结果 ID，也可能省略不兼容的推理块，或合并系统消息。
将 `Request::projection.replay` 设为 `ReplayProjection::Strict` 可拒绝有损回放。
将 `Request::projection.temporal` 设为 `TranscriptProjection::Strict` 可要求保留
系统消息和工具变更的位置。实际转换记录在返回消息的 `projection` 字段。

## 观察请求与处理失败

在支持这些功能的 adapter 上，`Request::callbacks` 可获取投影后的请求体、响应
头和解析后的原始事件。`with_on_payload` 可以观察或替换请求体。
`with_on_response` 提供 HTTP 状态和响应头。`with_on_raw` 在映射为公开
事件前接收解析后的供应商事件。回调出错会终止调用并阻止自动重试。需要记录请求
span 和指标时，将应用的观测上下文传给 `Request::telemetry_context`。

聊天失败时，通过 `CallFailure::cause()` 读取错误，需要时用 `message()` 保存
已生成的部分。用 `message::is_context_overflow` 判断是否需要缩短上下文。
成功返回 `Length` 时，用 `message::is_recoverable_length` 检查输出是否提前截断，
以便决定是否缩短上下文后重试。
失败时，可将失败消息传给 `retry::classify_whole_call_retry`，区分瞬时错误、取消、
配额耗尽或上下文溢出。
每次整次调用重试都会发起一次新的生成。[取消与清理](transport.zh.md#结束或取消聊天调用)

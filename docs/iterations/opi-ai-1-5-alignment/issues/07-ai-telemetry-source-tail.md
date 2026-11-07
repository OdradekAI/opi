# AI telemetry seam 与 source/result 尾部
Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [让普通 chat 入口独立执行](06-default-chat-execution.md)

## Scope

把 Pi AI 的领域观测合同先落成 `opi-ai` 内可独立验证的公开 seam，并修正
业务终态与 source settlement 被混为一谈的流生命周期。中立 span、metrics、
schema、no-op 和 memory recorder 仍由 `opi-telemetry` 拥有；本票只负责 AI
命名、属性、事件、UTF-16 block 计数、usage/cost 结算和状态策略。

`ChatStream` 在交付第一个业务终态后对消费者保持永久 EOF，但 `result()` 会
继续持有源并排空终态后的 source-owned 工作。projection 与 prepared attempt
保留这个排空路径；普通后台 `StartedChatStream` 继续遵守既有资源合同，在终态
转发后释放源，因此这两条入口的源生命周期差异是有意记录的实现边界。

## Acceptance

- `AiRequestTelemetry` 公开暴露 request/poll span、事件和 metrics 的 Pi 名称。
- 首事件、text/thinking/tool block、UTF-16 字符数、stop/usage/response/cost
  属性有内存 recorder 契约测试；deferred poll 不累计 token/cost；aborted 不
  自动标记 span error。
- 直接 `ChatStream::result()` 在终态后观察到 source EOF；重复终态和尾部事件
  不再交付给业务消费者。
- projection、prepared attempt、普通 ChatCall、StartedChatStream 的受影响
  生命周期回归通过；普通 `Models` chat 会自动安装 observer，但 raw
  `ChatStream`、prepared attempt 和非 chat 路径仍是手动/未覆盖边界。

## Delivery

新增 `crates/opi-ai/src/telemetry.rs` 的 AI 领域 seam 与
`tests/ai_telemetry.rs`，并更新 stream、prepared attempt、ChatCall 结果路径及
相关生命周期测试。随后在普通 `Models` ChatCall 路径安装 observer；README
明确 raw/prepared/non-chat 的手动边界，同时记录 `ChatStream::result()` 与后台
`StartedChatStream` 的尾部/释放边界。

完整检查和已知限制见[本票报告](../phase3-ai-telemetry.md)与[检查证据](../evidence/phase3-ai-telemetry-checks.json)。

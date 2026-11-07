# 普通 Models chat 自动安装 AI observer
Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [AI telemetry seam 与 source/result 尾部](07-ai-telemetry-source-tail.md)

## Scope

把 Pi `observeModelRequest` 对应的请求级观测安装到 `opi-ai` 的普通
`Models` ChatCall 执行路径。`Request::telemetry_context` 有值时，普通
`Models::{stream, complete, stream_simple, complete_simple, stream_direct,
complete_direct}` 产生一个 AI request observer，并在 normalized event 与
typed result 结算时报告 Pi 命名的 span、events 和 metrics。

raw `ChatStream::new`、prepared provider attempt 和非 chat 操作继续由调用方
显式使用 `AiRequestTelemetry`；它们不是本票的自动安装范围。observer 后端是
可失败的 Rust trait，而业务 `CallFailure` 没有遥测错误通道，因此 backend
失败只通过 host `tracing::warn!` 记录并停用该 observer，不替换或污染业务
结果。这是需要单独记录的 Rust 有意差异，不能当作严格 Pi 等价。

## Acceptance

- 普通 `Models` chat 在显式 telemetry context 下自动创建一个 `pi.ai.request`
  observer，并报告首事件、completed block 与 stop 属性。
- `None`/noop context 继续保持被动，不创建可观察 AI span，业务结果不变。
- observer 启动、事件写入或 finish 的 backend 错误不会改变 provider 事件、
  `CallFailure` 或 cancellation 语义；错误被记录并只关闭该 observer。
- tracing carrier 下的 ordinary ChatCall 仍归属于冻结 caller parent，且未
  轮询的 Models call 仍会完成其业务执行。
- raw/prepared/non-chat 路径的手动边界与 source-tail 约束继续由既有测试覆盖。

## Delivery

新增 `AiRequestObserver` 作为 `Models` 执行层适配器，在
`materialize_chat_call` 组装 request/model/api/reasoning/pricing 属性，并由
`ChatStream` 在 event、terminal failure 和 result settlement 时驱动。新增
`tests/ai_telemetry.rs` 的普通 Models 集成断言，更新
`tests/telemetry_operations.rs` 对 tracing carrier 的 span 数量/归属断言，
同步双语 README、spec、phase 报告与 contract-index。

## Limits

- 仅普通 Models ChatCall 自动安装；raw `ChatStream`、prepared attempt、图像、
  分类、deferred 等入口仍需手动或另行接入。
- 未执行真实 provider、TS/Rust 全量差分、完整 feature/platform/MSRV、性能
  或六门发布验收。

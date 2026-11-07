# 第四阶段第二个切片：普通 Models chat 自动 AI telemetry

日期：2026-11-07。实施票：[普通 Models chat 自动安装 AI observer](issues/09-ordinary-chat-ai-telemetry.md)。
本切片只闭合普通 `Models` ChatCall 的执行层接线，不代表所有 provider、入口或
Pi 1.5.0 行为已经严格等价。

## 实现

`materialize_chat_call` 现在从冻结的 `ChatModelRecord` 和 `Request` 组装
provider/model/api/reasoning/pricing 属性，并为显式 `telemetry_context` 创建
一个 `AiRequestObserver`。`ChatStream` 在每个 normalized event 上驱动 observer，
在业务 EOF 或 `ChatStream::result()` 的 source-tail 结算后完成 observer；typed
provider failure 复用同一 typed outcome。这样业务 terminal、消费者 EOF 与
source settlement 仍是三个可观察时点。

observer 是业务执行层的容错适配器：`AiRequestTelemetry` 的公开接口继续返回
backend error，普通 ChatCall 则把启动、event 或 finish 的 backend error 写入
host `tracing::warn!` 并关闭 observer。业务事件、`CallFailure`、取消和资源
生命周期不被 telemetry backend 失败改写。这一差异来自 Rust backend 的可失败
trait 与现有业务错误合同，已单独记录，不能计作严格 Pi 等价。

raw `ChatStream::new`、prepared attempt、图像、分类和 deferred 路径仍然保持
显式/手动边界。普通 Models 的 `None`/noop context 继续被动；tracing carrier
下生成的中立 `opi.telemetry.span` 仍属于冻结 caller parent。

## 验证

- `ai_telemetry`：5 passed，包括普通 Models chat 自动 request span。
- `telemetry_operations`：12 passed，包括未轮询 Models call 的冻结 parent 与
  自动 observer span 计数。
- `opi-ai` library：369 passed。
- `cargo check -p opi-ai --offline -j4`、`cargo fmt --all --check` 通过。

证据命令与边界见[检查证据](evidence/phase4-ordinary-ai-telemetry-checks.json)。

## 未完成

本票没有接入 raw/prepared/non-chat 的所有 provider 执行路径，没有运行真实服务、
TS/Rust 全量差分、跨平台/MSRV、性能或六门发布验收。observer backend failure
只验证了适配策略的代码路径；没有把任意外部 exporter 的故障注入冒充完整后端
兼容性证明。

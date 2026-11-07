# Public ChatStream telemetry constructor

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Prepared attempt observed ChatStream seam](11-prepared-ai-telemetry.md)

## Scope

为独立 provider 接入者公开 `ChatStream::new_with_telemetry`。调用方已经有
`EventStream` 时，可以传入显式 `TelemetryContext` 与冻结的
`AiRequestAttributes`，获得和普通 `Models` chat 一致的终端吸收、source-tail
排空、`CallFailure` 与 `pi.ai.request` 结算。原有 `ChatStream::new` 保持
business-only；不自动捕获 ambient tracing，也不改变 raw prepared attempt 的
retry/admission 控制职责。

## Acceptance

- 公开构造器只增加现有 `ChatStream` 的观测入口，不复制结果或 cancellation
  实现。
- 显式 context 下记录一条 `pi.ai.request`，provider/model/stop 身份来自调用方
  提供的冻结 attributes，业务结果与 source-tail 行为保持原合同。
- `None` context 仍不创建 AI observer；observer backend 故障不会替换业务结果。
- 双语 README、公共 API 表和契约索引明确无观测与显式观测两条路径。
- 外部集成测试从 crate 外部调用公开构造器，证明这不是仓库内私有 helper。

## Delivery

将既有内部 `new_with_ai_telemetry` 实现提升为公开的
`ChatStream::new_with_telemetry`，普通 chat、deferred 和 prepared observed
入口全部复用该 seam。新增 `ai_telemetry` 外部集成用例及双语接入说明。

## Limits

这是显式 opt-in 观测 seam；`ChatStream::new`、raw `start_attempt`、图像、分类
和 deferred cancel 不会因此自动安装 observer。跨 attempt 的单逻辑调用聚合、
全 provider 自动覆盖、真实服务、TS/Rust 差分、跨平台/MSRV、性能和六门发布
验收仍未执行。

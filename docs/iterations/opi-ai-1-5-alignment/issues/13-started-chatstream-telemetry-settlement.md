# Started ChatStream telemetry settlement

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Public ChatStream telemetry constructor](12-public-chatstream-telemetry.md)

## Scope

补齐公开 `ChatStream::start()` 与显式 AI telemetry 的生命周期衔接。调用方把
`ChatStream` 转为独立执行句柄后，后台执行者已经拥有请求和 observer；正常终态
必须在执行结果交付前完成同一条 `pi.ai.request` 结算。

## Acceptance

- `ChatStream::start().result()` 返回与原 `ChatStream::result()` 相同的业务终态；
- 显式 telemetry context 下仍只记录一条 `pi.ai.request`，并带有最终 `stop` 属性；
- 已经交付 terminal 后再调用 `ChatStream::start()` 也必须完成同一 observer；
- 修复只补执行者拥有的 observer 结算点，不改变 source 终态吸收、后台执行、显式
  abort 或 raw `ChatStream::new` 的其他合同；
- 集成测试从 crate 外部构造 observed `ChatStream`，覆盖独立执行路径。

## Delivery

`ChatStream::start()` 在已缓存 terminal 的快速路径，以及
`ChatStream::result_without_tail` 返回后台执行结果前，统一调用已有的
`finish_ai_telemetry()`。新增 `ai_telemetry` 集成测试，证明两条
`StartedChatStream` 路径都不会丢失 observer 终态。

## Limits

本票只覆盖正常业务终态的后台结算。显式 abort 在无业务终态时的 observer
终态、跨 attempt 聚合、raw `start_attempt` 自动观测、图像、分类、deferred cancel、
真实 provider、完整 feature/platform/MSRV、性能和六门发布验收仍未完成。

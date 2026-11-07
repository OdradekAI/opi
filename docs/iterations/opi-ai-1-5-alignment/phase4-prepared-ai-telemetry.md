# 第四阶段第四个切片：prepared attempt 的 observed ChatStream seam

日期：2026-11-07。实施票：[Prepared attempt observed ChatStream seam](issues/11-prepared-ai-telemetry.md)。

本切片没有替换既有 `PreparedProviderCall::start_attempt()` 的原始
`EventStream` 合同，而是增加 `start_attempt_with_telemetry()`。它使用冻结的
route、request、auth 和 provider 只启动一次 attempt，再通过标准 `ChatStream`
提供 result、source-tail 和可选 `pi.ai.request` observer 结算。这样高级 retry/
admission controller 仍能控制原始 attempt，独立接入者也能获得和普通 Models
chat 一致的消费合同。

## 实现观察

- provider、model、wire API 来自 `PreparedRoute`，不会重新解析目录或认证；
- thinking 与 pricing 从冻结 request/model 生成 telemetry attributes；
- 显式 `TelemetryContext` 才记录 span，observer backend 故障沿用普通 ChatStream
  的隔离策略，不改写业务结果；
- 原始 start、重试 admission、取消、资源 lease 和 credential-terminal 路径未改动。

## 验证

- `ai_telemetry`：7 passed，新增 prepared attempt `pi.ai.request` 断言；
- `cargo fmt --all --check`、`cargo check -p opi-ai --all-targets --offline -j4`、
  Clippy、rustdoc、文档检查和实际 `.crate` 外部消费者验证沿用本票证据；
- 工作区仍保留此前记录的两项无关全量失败，不能据此宣称整个 workspace 通过。

## 未完成

跨 retry attempt 的聚合观察、raw ChatStream、图像、分类、deferred cancel、完整
provider/feature 矩阵、真实服务、TS/Rust 差分、跨平台/MSRV、性能和六门发布验收
仍未执行。

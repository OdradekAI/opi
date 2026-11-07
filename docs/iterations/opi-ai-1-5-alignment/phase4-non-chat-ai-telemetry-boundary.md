# 第四阶段第十一个切片：非 chat 与 deferred cancel 的 AI telemetry 边界

日期：2026-11-07。实施票：[Non-chat and deferred-cancellation AI telemetry boundary](issues/19-non-chat-ai-telemetry-boundary.md)。

本切片把 telemetry carrier 传播和 Pi AI span 创建拆开记录。普通 chat 创建
`pi.ai.request`，deferred fetch 每次创建一个 `pi.ai.poll`；远端
`cancel_deferred`、image 和 classifier 只把显式冻结的 `TelemetryContext` 交给
adapter，不自动创建 `pi.ai.*` span。这与 Pi 1.5.0 的 `Models` 入口保持一致，
也避免把 adapter 自有观测误称为 SDK-owned AI telemetry。

## 实现观察

- `Models::cancel_deferred` 保持 best-effort remote cancel 和 typed cancellation
  结果，不创建 AI observer；
- `Models::stream_deferred` 的既有 `pi.ai.poll` 观察保持不变；
- image/classifier 的 carrier 透传测试保持不变，继续验证 context 到达 adapter；
- 双语 README 明确列出“有 carrier 但无 Pi AI span”的三个操作。

## 验证

- `ai_telemetry` 新增 deferred remote cancel + recorder 回归测试，断言没有
  `pi.ai.poll` 或 `pi.ai.request`；
- deferred fetch poll、image carrier/no-span、classifier carrier/no-span 测试继续通过；
- 文档检查和 JSON contract index 校验通过。

## 未完成

adapter-owned operation spans、真实 provider、完整 feature/platform/MSRV、TS/Rust
差分、性能和六门发布验收仍未执行。

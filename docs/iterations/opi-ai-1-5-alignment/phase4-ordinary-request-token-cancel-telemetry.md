# 第四阶段第九个切片：普通 request-token 协作式取消 telemetry

日期：2026-11-07。实施票：[Ordinary request-token cancellation telemetry](issues/17-request-token-cancel-telemetry.md)。

本切片固定普通 `Models::stream*` / `complete*` 自动 telemetry 的协作式取消边界：
调用方保留并取消 `Request::cancel` 后，provider 交付类型化
`ProviderError::Cancelled`，普通 `ChatStream` result 路径负责完成同一
`pi.ai.request` observer。

## 实现观察

- request-token 取消仍由 provider 协作，不复用 `ChatCall::abort()` 的 Tokio
  force-abort handler；
- `ProviderError::Cancelled` 沿既有 `CallFailure::from_partial` 映射为
  `StopReason::Aborted`；
- observer 以 `stop=aborted` 结算并保持一次性结束；
- force-abort 与 request-token cancellation 保持两个可观察的取消路径。

## 验证

- `ai_telemetry` 新增普通 `Models` hanging provider + request token 回归测试，
  先确认 provider 已接收调用，再取消 token，断言 typed cause、`Aborted` stop
  和唯一已结束 `pi.ai.request` span；
- 普通成功、普通显式 abort、observed `ChatStream`、prepared attempt 与
  execution lifecycle 合同继续保留。

## 未完成

raw attempt 自动观测、跨 attempt 聚合、图像、分类、deferred cancel、真实
provider、完整 feature/platform/MSRV、TS/Rust 差分、性能和六门发布验收仍未执行。

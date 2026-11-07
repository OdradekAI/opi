# 第四阶段第八个切片：普通 ChatCall 的显式 abort telemetry 结算

日期：2026-11-07。实施票：[Ordinary ChatCall explicit-abort telemetry settlement](issues/16-ordinary-chatcall-abort-telemetry.md)。

本切片关闭普通 `Models::stream*` / `complete*` 自动 telemetry 的执行边界缺口：
外层 `ChatCall::abort()` 强制取消 Tokio 任务时，内层 observed `ChatStream` 不再因为
执行任务被丢弃而遗失 `pi.ai.request` 终态。

## 实现观察

- `ChatCallDriver` 持有一个只在 inner `ChatStream` materialize 后填充的 observer slot；
- 外层 `StartedChatStream` 的 abort handler 与 inner `ChatStream` 共享同一 observer；
- handler 只有在 Tokio join 确认任务被取消后才结算，使用现有 `CallFailure` 的
  `StopReason::Aborted` 消息；
- normal completion 仍由 `ChatStream` 负责，observer 的一次性 finish 保证竞态幂等；
- setup 失败、raw stream、request-token cancellation 与非 chat 路径的边界未改变。

## 验证

- `ai_telemetry` 新增普通 `Models` hanging provider + `ChatCall::abort()` 回归测试，
  断言唯一 `pi.ai.request` 的 `stop=aborted` 与结束序列；
- 既有普通成功、observed `ChatStream` normal/abort、prepared attempt 与 execution
  lifecycle 合同继续保留。

## 未完成

raw attempt 自动观测、跨 attempt 聚合、协作式取消的独立回归、图像、分类、deferred
cancel、真实 provider、完整 feature/platform/MSRV、TS/Rust 差分、性能和六门发布验收
仍未执行。

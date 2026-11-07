# 第四阶段第七个切片：独立执行 ChatStream 的显式 abort telemetry 结算

日期：2026-11-07。实施票：[Started ChatStream explicit-abort telemetry settlement](issues/15-started-chatstream-abort-telemetry.md)。

本切片关闭 observed `ChatStream::start()` 的最后一个已知生命周期缺口：
`StartedChatStream::abort()` 强制取消 Tokio 执行任务时，observer 不能因为
`ChatStream` 被任务取消而遗失终态。

## 实现观察

- `ChatStream` 与 `StartedChatStream` 共享同一个一次性 observer cell；
- 正常终态仍由 `ChatStream::result_without_tail()` 负责；
- `StartedChatStream::abort()` 只有在 join 结果确认是 Tokio cancellation 后，才用
  现有 `CallFailure::from_partial(..., ProviderError::Cancelled)` 生成的
  `StopReason::Aborted` 消息结算 observer；
- 结束操作保持幂等，完成与 abort 的竞态不会生成第二个 span，也不会覆盖已完成的
  正常终态；
- raw `ChatStream::new`、raw `start_attempt()`、消费者 Drop 和运行时 shutdown 不改变。

## 验证

- `ai_telemetry`：12 passed，新增带 partial 的 pending source 显式 abort 回归测试，
  断言唯一 `pi.ai.request` 的 `stop=aborted` 与结束序列；
- 既有 execution lifecycle 的 abort/partial/panic 合同继续保留；
- 其余格式、check、Clippy、rustdoc、文档检查和包消费验证按受影响范围执行。

## 未完成

raw attempt 自动观测、跨 attempt 聚合、图像、分类、deferred cancel、真实 provider、
完整 feature/platform/MSRV、TS/Rust 差分、性能和六门发布验收仍未执行。

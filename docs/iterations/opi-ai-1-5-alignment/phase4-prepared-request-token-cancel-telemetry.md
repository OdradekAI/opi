# 第四阶段第十个切片：prepared observed attempt 的 request-token telemetry

日期：2026-11-07。实施票：[Prepared observed attempt request-token cancellation telemetry](issues/18-prepared-request-token-cancel-telemetry.md)。

本切片固定 `PreparedProviderCall::start_attempt_with_telemetry()` 的协作式取消
合同：该显式 helper 返回标准 `ChatStream`，保留冻结 `Request::cancel`，由 provider
交付类型化 `ProviderError::Cancelled`，并由该 attempt 的 observer 记录
`stop=aborted`。

## 实现观察

- 不改变 raw `start_attempt()` 的 `EventStream` 返回类型；
- 不隐式创建跨 retry attempt 的 logical-call aggregator；
- observed helper 的 normal `ChatStream` result 路径负责 typed failure 和 observer
  一次性结算；
- 每个 attempt 仍拥有独立 `pi.ai.request` span，whole-call 聚合由调用方负责。

## 验证

- `ai_telemetry` 新增 prepared hanging provider + request token 回归测试，
  通过后台 result task 先确认 provider 已接收调用，再取消 token，断言 typed
  cause、`Aborted` stop 和唯一已结束 request span；
- 普通 request-token、普通显式 abort、prepared retry 与 execution lifecycle
  合同继续保留。

## 未完成

raw `ChatStream::new`、raw `start_attempt` 自动观测、图像、分类、deferred cancel、
真实 provider、完整 feature/platform/MSRV、TS/Rust 差分、性能和六门发布验收仍未执行。

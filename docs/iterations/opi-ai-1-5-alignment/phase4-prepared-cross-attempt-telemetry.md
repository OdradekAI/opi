# 第四阶段第七个切片：prepared attempt 的跨 attempt telemetry 边界

日期：2026-11-07。实施票：[Prepared attempt telemetry boundary](issues/14-prepared-cross-attempt-telemetry.md)。

本切片没有改变 retry owner 或 raw `start_attempt()` 类型，而是把 observed helper
的 span 粒度固定为“每个 attempt 一条”。通过 `MockResponse::DelayedAfter` 脚本化
一次 retryable failure，再运行第二个成功 attempt，证明两个 span 都独立结算，且
终态分别保留 `error` 与 `stop`。

## 交付

- `PreparedProviderCall::start_attempt_with_telemetry()` 的 README 中英文说明补充
  每 attempt 一条 span、不自动聚合，以及需要 whole-call 视图时由 host/agent 负责。
- `ai_telemetry` 增加跨 attempt 回归，覆盖第一 attempt 失败、admission、第二
  attempt 成功和两个 span 的终态。
- spec、实施票、契约索引和证据文件同步记录该边界。

## 验证

- `ai_telemetry`：11 项通过；新增用例从 crate 外部实际调用 prepared observed helper。
- `cargo fmt --all --check`、`git diff --check`、文档检查和 JSON 解析通过。
- 其余库、生命周期、telemetry operation、check、Clippy、rustdoc 复用上一切片
  的通过结果；本切片不重新宣称完整 workspace、live provider、feature/platform/MSRV
  或六门验收通过。

## 结论

prepared observed helper 的局部契约已收束：它负责一次 attempt 的标准 stream/result
和 AI observer；跨 attempt 的逻辑聚合明确留在更高层，不再作为该公共 helper 的隐式
行为。显式 abort 无业务终态仍是独立开放边界。

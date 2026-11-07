# Prepared attempt telemetry boundary

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Prepared AI telemetry helper](11-prepared-ai-telemetry.md)

## Scope

把 prepared call 的 observed helper 与 acquisition retry owner 的边界固定下来。
`PreparedProviderCall::start_attempt_with_telemetry()` 是一次 attempt 的便利入口，
而 `PreparedProviderCall` 仍然负责顺序重试、admission、sealed state 和冻结的
route/request/auth。多次 attempt 不自动共享或合并 AI request span。

## Decision

- 每个 observed attempt 启动并结算一条独立的 `pi.ai.request` span；失败 attempt
  记录自己的终态，后续 retry 记录另一条 span。
- `start_attempt()` 继续是 raw `EventStream` seam，不隐式安装 observer。
- 跨 attempt 的 logical whole-call 聚合不属于该 helper；需要该视图的 agent 或
  host 必须在更高层拥有聚合器，并自行定义 attempt、等待、最终结果和重试预算的
  属性关系。
- 这项决定不改变 retry admission、认证只解析一次、attempt 串行和资源释放合同。

## Acceptance

- 脚本化的第一次可重试失败和第二次成功各产生一条已结算 `pi.ai.request` span；
  两条 span 的 `stop` 分别为 `error` 和 `stop`。
- 普通 prepared attempt、raw attempt、retry/backoff 和外部包消费回归保持通过。
- README、rustdoc、契约索引和证据文件都说明“不自动跨 attempt 聚合”。

## Limits

本票没有新增 logical whole-call aggregator，也没有把 raw attempt 改成 observed
类型；图像/classifier、deferred cancel、真实 provider、
完整 feature/platform/MSRV、性能和六门发布验收仍按原票开放。

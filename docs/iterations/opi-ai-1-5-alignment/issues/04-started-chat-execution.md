# 显式启动 chat 请求并分离执行与消费
Status: done
Owner: Codex for user
Blocked by: [分离纯值、公共错误与认证目录契约](03-values-errors-auth-catalog.md)

## Scope

第三阶段的首个增量切片。在 ChatCall 和 ChatStream 上提供显式 start，返回
StartedChatStream 消费句柄，由私有 execution 模块持有请求直到既有调用终态。
执行无需消费驱动；丢弃消费句柄或 result future 不取消执行。取消沿用原请求的
显式 token、timeout 和 provider 合同。

保留原构造与手动 poll 的惰性合同，包括无需 Tokio 的自定义流；start 要求当前
Tokio runtime，调度执行而不承诺同步完成 auth。暂不迁移旧入口已 poll 后的 Drop、
agent 默认入口或 raw EventStream。它们仍是已登记缺口，本票不关闭生命周期决策。
AI observer、终态后的 source.result/drain 和业务/遥测分别结算继续由后续切片处理。

## Acceptance

- 显式 start 后零消费也推进；普通 Models setup 与 advanced stream 均能使用。
- 保留句柄但不消费可完成，事件顺序与既有 event-time snapshots 保留；消费丢弃
  后不积压队列、不隐式 cancel，资源在真实源终态释放。
- 显式取消仍是 Aborted；result 中途丢弃不会取消执行。
- 正常/失败、已有部分消费、已有终态、premature EOF 和重复终态沿用单一结果。
- provider panic 不变成业务成功或被静默吞掉；等待者恢复 panic，未等待时沿用
  Tokio 的宿主 panic hook。runtime shutdown 不能被算成请求完成。
- 清楚记录缓冲的内存成本：为保留全部事件且不阻塞无消费执行，未读事件队列
  无固定上限；result 丢弃未读事件；停止消费者可 drop 句柄释放积压。
- 原惰性接口、认证/重试、冻结上下文和 prepared acquisition 合同不退化。

## Checks planned / Test impact

add：独立生命周期集成 target，验证上述新入口与公共消费者观察。
update：必要的 telemetry 公共入口回归和双语 README/rustdoc。
retain：stream、provider collection、retry、telemetry 及 agent 高级消费者回归。
delete：none。范围限定的 Clippy、doctest/rustdoc、fmt、doc-check、diff 检查。

基点 HEAD 814c95f；第二阶段基础边界 71 个路径为 carried-in 未提交工作，本票保留。
首次状态与哈希在外部 work/phase3-start/initial-state.json，后续报告区分本票触及路径。

## 交付

本票显式启动入口已实现并通过范围限定检查；见[切片报告](../phase3-started.md)及
[检查记录](../evidence/phase3-started-checks.json)。新增 15 个生命周期案例，两个既有
target 各追加一例，没有删除或更改旧断言。第三阶段、旧入口迁移和 AI 埋点仍未完成。

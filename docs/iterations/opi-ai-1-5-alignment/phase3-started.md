# 第三阶段首个切片：显式启动与独立消费

日期：2026-11-07。实施票：[显式启动 chat 请求并分离执行与消费](issues/04-started-chat-execution.md)。
本票已完成；第三阶段仍在进行，未声明 Pi 生命周期严格等价。

## 已交付

新增 `ChatCall::start()` 和 `ChatStream::start()`，返回同一个公开
`StartedChatStream`（根门面及 `stream` 路径）。私有 `execution` 模块持有原始
调用、setup、源流和任务；消费句柄仅接收事件或等待结果。已完成调用直接保留缓存。

- 未完成的调用在当前 Tokio runtime 调度，无须消费端首次 poll；这不表示方法
  返回前已同步完成 auth，也不把 JS 同步 setup 时点记作严格等价。
- 零消费、首事件后 Drop、立即 Drop 和 result future Drop 都不取消执行。
  仍由原 token、timeout、源合同或 runtime shutdown 决定结束。
- first-terminal absorption、失败 canonical partial、部分迭代后的 result、已交付
  终态缓存保持原调用行为；源资源在新句柄的终态交付前释放。
- runtime 停止未完成的执行时，typed failure 保留已生成的最新 partial，包括
  未被消费的事件和 start 前已有的 partial。源 panic 经 Tokio hook 报告，并在
  等待者恢复，不被伪装为成功。
- 未读事件使用无固定容量的 FIFO，保留顺序和 event-time snapshots；result
  首次 poll 或消费端 Drop 释放积压，owner 随后丢弃新事件而继续源执行。已入
  FIFO 的事件不会因取消被撤回，取消失败排在其后；provider 内部队列仍保留原
  取消优先规则。这是新消费入口的明确合同，不覆盖 raw receiver 的合同。

新增实现没有网络/SDK 依赖、没有自动 span、没有 exporter、没有公共状态探针。
没有改动旧调用执行函数、内置 producer 的 closed 分支或高级 retry/admission 规则。
原有第一次手动 poll 及 Drop 行为继续存在，不据此宣称已完成决策十一。

## 验证

完整命令、结果、原始日志摘要及源码身份见[检查记录](evidence/phase3-started-checks.json)。

| 范围 | 结果 |
| --- | --- |
| execution_lifecycle | 15 tests PASS：无消费者/迟读/Drop、result、显式取消、runtime shutdown、partial、panic、普通和 prepared 调用 |
| provider_lifecycle | 22 tests PASS：含真实 Anthropic 本地 HTTP 调用在消费端 Drop 后读到最终结果 |
| telemetry_operations | 12 tests PASS：含独立执行任务保留显式 frozen context，销毁 Models 不丢失调用；仍无自动 AI span |
| stream_events / provider_collection / resource_cleanup / telemetry_carrier / whole_call_retry | 188 tests PASS，保留旧惰性、错误、资源、认证和重试合同 |
| opi-ai all-targets Clippy | PASS，`-D warnings` |
| opi-ai doctest / public rustdoc | 3 doctests PASS；rustdoc `-D warnings` PASS |
| fmt / doc-check / whitespace / index 和工作树核对 | PASS |

49 项相关测试与 188 项互补回归分别报告，不累加多次重跑，也不把上游探针算作
Rust 等价案例数。既有回归执行后只改动新 start 路径的 shutdown partial 与文档，
没有改变旧惰性路径；这些新增路径已在最终 49 项运行中再次覆盖。

保留失败过程：首次编译证明 start 入口缺失；实现中一次不存在的方法名导致编译
失败，随后修复。最初 advanced 测试错误地等待成功请求再次准入，修正为断言
pending 期间仍占用、成功后 PostHandoff 封闭且无 UnknownDispatch；生产准入规则
未改。独立审查发现 runtime shutdown 丢 partial，新增用例先失败，修复后通过。

测试影响：add 为 15 个生命周期测试、1 个真实 wire 补充案例、1 个 frozen-context
补充案例；retain 为旧测试及断言；delete 为 none。现有两个 target 只追加新案例。

## 新的原版观测

新增可复现驱动 [observe_ai_telemetry.mjs](tools/observe_ai_telemetry.mjs)，直接加载
冻结的 AI `lazy.ts`、`telemetry.ts` 和 pi-telemetry 源码。唯一 package resolver 将
`@earendil-works/pi-telemetry` 定位到同一冻结快照；不替换被测实现。执行命令：

```text
node docs/iterations/opi-ai-1-5-alignment/tools/observe_ai_telemetry.mjs .pi/pi-1.5.0 docs/iterations/opi-ai-1-5-alignment/evidence/phase3-ai-telemetry-observations.json
```

Node v24.13.0 / Windows x64；五项观测断言 PASS，另验证缺参数以 exit 1 拒绝。
[原始记录](evidence/phase3-ai-telemetry-observations.json)固定驱动及八个加载源码摘要。

| 原版事实 | 对后续实现的约束 |
| --- | --- |
| 无外层消费时 setup 和 AI span 已建立；业务 result 在 terminal push 时完成，源 EOF/source.result 尚可等待 | 业务终态、源完成和遥测结束必须分别建模 |
| iterator.return 不取消协作源；后续显式 abort 使源结束，aborted span 保持 ok | 丢弃消费与取消分开；不能统一把取消标为遥测 error |
| `A😀` 与 `😀` 的 block 字符计量分别为 3、2 | 沿用 UTF-16 单元 |
| 首 terminal 的 first-chunk metric throw：业务仍 Stop，遥测最终 Error | 已锁定业务结果不被 observer 回滚 |
| onResult 的 request-duration metric throw：业务仍 Stop，span 在门控检查点 pending | 不能用统一 finally/吞错偷偷改写原版故障语义 |
| instrument factory throw：setup 尚未调用，无 span | 保留失败阶段，而非统一转成流中业务失败 |

这些是源级执行观测，不是 npm 包、Models/provider 闭包或 TS/Rust 全量差分通过。
pending 是有界门控观察，不是整个进程资源泄漏证明。原始记录中的随机 span ID 和
时间值保留，不参与跨语言同值断言。

## 状态与剩余工作

工作树基点仍是 `814c95f7ce575fb9be3df6b9d04ff6e2942706c1`；第二阶段基础边界的
71 个路径作为 carried-in 保留，本票有意触及的交集和新增路径由检查记录区分。
committed=none-for-this-task，staged=none。未提交、推送或发布。
第一阶段索引与前两张实施记录保持原对象；本票只追加一个实施映射。

本票没有完成以下事项：

- 旧普通入口首次 poll 后的 Drop 政策、agent 高级生产调用者和 raw EventStream
  迁移。用户确认的“不因消费 Drop 取消”仍是最终目标，没有批准永久保留差异。
- Pi 同步 eager/setup 时点及 first-terminal 后继续 source/source.result 的映射。
- AI schema/instrumentation、业务与遥测分别结算和 observer 故障通道。
- 队列容量预算、耐久/性能、feature 组合、完整 README、实际发布包和六门验收。

本次未运行全 workspace all-targets、agent 默认调用迁移回归、Linux/macOS、声明
MSRV 1.97、live 服务或新的实际包消费。上一阶段 sealed 包证据仍证明其旧源码快照，
不自动成为本候选的打包证明；已知 schema/spec_ledger/Vertex 全量回归问题未关闭。
下一切片继续处理默认调用入口与消费者迁移，并以本次原版时序证据设计 AI 结算。

# 第五阶段修复：普通调用的 source 尾部结算

日期：2026-11-08。工作树基点：`3f2ff874af08a82108f2326281f579808b6dd25a`。
用户确认继续后，修复[上一切片](phase5-supplemented-materials-and-gated-differential.md)
发现的普通调用 source 尾部截断。对应[实施票](issues/37-ordinary-source-tail-settlement.md)。

## 结果

**source 尾部缺口已修复；三例差分的共享观察通过，材料门仍受阻。**

- [当前材料身份](evidence/phase5-source-tail-materials.json)
- [修复后真实差分](evidence/phase5-source-tail-differential.json)
- [红绿回归与检查记录](evidence/phase5-source-tail-checks.json)

本次实际差分为 `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit **2**，不是完整对齐
通过。之前的[两例 FAIL](evidence/phase5-gated-lifecycle-differential.json)原样保留。
旧材料清单绑定旧 Rust 源码与示例，因此为修复后的候选另存清单，没有修改旧证据。

## 原因与最小修复

调用路径为 `Models::stream_simple` → `ChatCallDriver` → `ChatStream` →
`drive_resolved_call_registered` → prepared attempt → provider source。

普通调用 wrapper 在第一个业务终态后设置 `finished`，下一次 poll 直接返回 EOF，
阻断了外层已有的尾部结算。其内部 `AttemptStream` 和 projection 层本来就支持
“首终态之后只排空 source，不再更新准入状态”，但这一路径未被普通 wrapper 驱动。

修复只在 `finished` 分支排空仍持有的当前 attempt，然后返回 EOF。没有改动 retry
判断、计数或 admission，没有新增 task、取消策略、超时、公共类型或依赖。transport
错误分支本来没有保留 attempt，仍按现有方式释放，不等待错误之后的 source。

请求执行者继续独立运行：停止消费、丢弃 call/result waiter 不会中断尾部。普通调用
在 source 尾部完成后交付首终态、结算遥测和 result；迟到 delta、错误或重复终态
不能替换首个结果或触发新的 attempt。永不结束的 source 尾部保持 pending，不能
因为已见业务终态而谎称 source 已完成。

请求 token 仍依赖 source 协作；显式 `abort()` 仍停止并 join 当前执行任务。若在
尚未交付的终态尾部等待期间 abort，返回 Aborted、确认 source Drop，但不声称 EOF。
这不保证 join 自定义 provider 内部独立派生的任务。裸 `ChatStream::start()` 的
既有终态释放策略没有改变。

## 可重复红绿验证

先增加公共入口回归 `ordinary_business_terminal_waits_for_source_tail_and_settles_once`，
在 source 已产生业务终态后设置第二道屏障。修复前实际运行退出 101：

```text
business terminal must not truncate the source tail: RecvError(())
0 passed; 1 failed
```

此信号证明尾部 future 被提前 Drop，未依赖 sleep 或 scheduler 时间猜测。
修复后该测试覆盖消费/不消费 × Stop、Done(Error)、Done(Aborted)、legacy Error
八种组合，验证：

1. 尾部屏障期间 source 没有 EOF/Drop，span 未结束，终态/result 尚未交付。
2. 放行后真实 EOF/Drop 可观察；事件终态与 result 相同，之后永久 EOF。
3. 尾部 delta、Timeout 和第二个 Done 不覆盖首个结果、usage 或 span stop。

另外三个新测试覆盖尾部 pending 时显式 abort、协作 token 取消和 transport 错误
不进入尾部。独立审阅后再补一条普通入口组合回归：第一 attempt 在任何事件之前返回
可重试错误，第二 attempt 到达终态后的 gated tail；确认恰好两次 dispatch、终态在
EOF 后交付、尾部错误与重复 Done 不改变第二 attempt 的 Length 结果，也没有第三次
dispatch。两处已有 ordinary channel fixture 在 Done 后显式关闭 sender，避免将
“业务终态”当成“source 已经结束”；其他 raw-start fixture 保持原行为。示例的两个
consumer-Drop 测试增加实际 EOF 断言，Python Rust-driver 测试不再允许旧 FAIL 作为
工具成功的分支。比较器本身没有放宽。

## 检查与差分

Windows 原生工具链，复用 `E:/opi/cargo-targets/opi-72fa37a0fa044958` 及 cache lease；
运行前做 quick 状态检查，未回收既有 lease、清理缓存或创建临时 Cargo target。

| 检查 | 结果 |
| --- | --- |
| `execution_lifecycle` | 29 passed（补 retry＋tail 回归后重跑） |
| `ai_telemetry` | 20 passed |
| `retry_backoff` | 57 passed |
| `stream_events` | 29 passed |
| `lifecycle_differential` example tests | 2 passed |
| `external_sdk` | 11 passed |
| `provider_lifecycle` | 22 passed |
| `resource_cleanup` | 16 passed |
| `telemetry_operations` | 12 passed |
| library `stream::tests` | 11 passed（358 未选，不计通过） |
| Python `test_rust_driver.py` | 2 passed，包含实际三场景和未知场景负例 |
| affected-target Clippy | pass，`-D warnings` |
| fmt、文档、JSON/索引、diff 空白与材料复核 | 见检查记录 |

合计 209 个 Rust 测试与 2 个 Python 测试。未改动的 freezer、inspector、比较器和
runner 单测沿用上轮已绑定的检查，不重复运行；生产源码和示例变更后，实际 driver
编译、执行和 TS/Rust 差分均已重跑。每个 TS 场景重复三次，原始观察稳定。

三例均为 `difference` / `shared_observations: pass`：不消费、停止消费的
`source_completed` 现在两侧都是 true；setup failure 仍只比较单次 error 终止，
不声明失败注入点和错误内容等价。构造时启动时点、setup 错误通道仍是未批准差异。
`strict_alignment` 保持 false。

## 独立审阅处置

只读 reviewer 独立确认根因和最小修复；未运行编译或测试。原始报告路径及摘要保存在
检查记录中。其唯一有实质补强价值的覆盖意见是“获准 retry＋gated tail”的组合，已
增加上述回归并重跑整个 `execution_lifecycle`（29 项）及该目标 Clippy。

另外两项意见经当前工作树核验不成立：`phase5-source-tail-checks.json` 已存在，原
意见产生于并行写入期间；英中 README 与 Unreleased 的新增段落在 `git diff HEAD`
中确实存在，并非已经提交的内容。保留原始意见，同时在检查记录登记处置，不采信
审阅报告对 committed/worktree 状态的混淆。生产源码在审阅后没有再次改变，所以
其他已通过的目标及真实差分不重复运行。

## 剩余事项

1. 完整 root lock 字节仍不同于冻结版本；直接依赖匹配不能代替传递依赖身份。
   本次只在项目 `.pi` 顶部两层查找 lock/archive，没有找到原始 lock 副本；未安装、
   联网下载、覆盖现有文件或猜测还原。仍需原始材料，或具名的新基准决策。
2. 现有生成目录仍与 pinned 原始响应有九个 provider 差异（1712 vs 1738 项）。
   尚未 hydrate 或覆盖用户提供的目录；应在可追溯工作副本中完成。
3. 真正的 core＋Faux 最小 feature、更多 provider 差分及 platform/MSRV/resource
   六门验收不在本修复切片内，不能因这三例通过而关闭。

测试影响：`add` 五个 gated 生命周期测试；`update` 两处 EOF fixture、两个 example
Drop 断言和 Rust-driver 回归；`retain` 既有产品测试与全部历史证据；`delete` none。
已更新公开 rustdoc、英中 README 与 Unreleased。无提交、推送、发布或 upstream 文件
改动；共享工作树中的先前材料工具工作和 `.serena` 均保留。

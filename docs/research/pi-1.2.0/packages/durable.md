# pi-durable：持久会话、任务与工具运行时

分析日期：2026-10-16。对象为本地 Pi 1.2.0；只做静态阅读，没有安装依赖、构建、执行测试、基准、示例或模型请求。本文重新核查当前入口和关键链，不把 1.0.4 报告当成当前实现。文件身份与规模见[清单](../source-manifest.json)，资料实际审读范围见[审读记录](../documentation-reading.json)。

## 职责、入口与真实调用方

Durable 有两层权威。Session 将 conversation、不可变 entry、task、submission 及 Chord 文档变更放在同一条原子提交线上；Harness 在其上安装调度器及 generation/tool/compaction 状态机。恢复依靠已提交 checkpoint、输入和结果，不恢复 JavaScript 栈、Promise 或外部命令的进程状态。它不依赖 pi-agent-core，拥有独立的模型循环。[导出](../../../../.repo/pi-1.2.0/packages/durable/src/index.ts#L1)、[提交实现](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L690)、[规范不变量](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L46)。

根入口公开 `Harness`、`createSession`、文档/任务/entry 定义、registry、built-in tokens、`watchEvents` 和 MemoryStorage。子入口提供 `/env`、`/env/node`、`/tools`、`/codemode`、`/tool-search`、`/images` 及 Node/Cloudflare loaders、memory/JSONL/SQLite backends、`/testing`。npm 版本 1.2.0，Node 下限 22.19.0；依赖 Chord、pi-ai、pi-codemode 的 `^1.2.0`，另有 Photon 0.3.3、diff 8.0.4、TypeBox 1.3.27。镜像处理是显式导入的 WASM 能力，Node 图像入口另起 worker；不是每次导入根入口就加载图片运行时。[manifest](../../../../.repo/pi-1.2.0/packages/durable/package.json#L1)、[图片说明](../../../../.repo/pi-1.2.0/packages/durable/README.md#L149)。

实际消费者包括 pi-env 对 ExecutionEnv 与共享 conformance 的适配，以及仓库 issue triage：后者创建 Models/credentials、registry/extension、SQLite，`Harness.open()` 后恢复存储状态再 `resume()`。包内 `test/examples` 展示 print、events、subagents、plan mode 等装配。当前 coding-agent 的生产源码未导入 Durable；旧版 `src/experimental` 服务链已经不在当前树中。不能把宿主示例或库能力描述为默认终端产品已经采用的会话后端。[triage.ts:114](../../../../.repo/pi-1.2.0/scripts/issues/triage.ts#L117)、[env 适配](../../../../.repo/pi-1.2.0/packages/env/src/remote-env.ts#L354)、[示例导航](../../../../.repo/pi-1.2.0/packages/durable/README.md#L712)。

## 关键链与状态归属

```text
Conversation.submit(input) → durable submission + user/custom input entry
  → pi.generation.prepare → pi.system baseline/delta + request checkpoint
  → Models.streamSimple / fetchDeferred → committed partial → assistant entry
  → pi.tool children → tool-result entries → next generation / final settlement
Session commit → prepare drafts → Storage.commit → adopt revisions → observers
```

1. 提交先按 conversation 内 `requestId` 去重。空闲输入直接放置并启动 generation；忙时依 `steer/followUp/reject` 处理。`entry: {kind,data}` 可给输入自定义非 `pi.` kind，仍携带 user model message。只有 kind/data 的 display-only write 在忙时也立即追加并 done；若队列已有 reset，则排在它后面。其他 writes 在边界放置，失败 run 留下 inbox；新 display-only write 不会重启它。[admission](../../../../.repo/pi-1.2.0/packages/durable/src/harness/submissions.ts#L151)、[inbox 规则](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L2259)。
2. 每次 prepare 解析当前 agent、registry snapshot、settings、env 和 prompt sections；将真实 prompt/tools 的变化写为 positional system entries。一次 request 的 model、thinking、stream options、cutoff 固定，重试再 prepare。上下文由 head、edits、fork ancestry 及工具结果重排派生，原始 transcript 与 UI view 不是模型 context。[generation](../../../../.repo/pi-1.2.0/packages/durable/src/harness/generation.ts#L136)、[上下文规则](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L257)、[prompt](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L3419)。
3. 模型工具调用先确认它在该 request 的 offered tools 中。工具 task 按当前 phase 的 agent 解析实现、修复/验证 args、运行 beforeTool、持久化最终 args/replay intent，再做外部 effect，之后 hooks、限幅、结果和 usage 一起提交。replay 默认 unsafe；恢复时只有已存 policy 与当前实现都 safe 才重跑，否则输出 interrupted。外部副作用与结果提交之间仍有未知窗口，safe 声明不提供 exactly-once execution。[ToolTask](../../../../.repo/pi-1.2.0/packages/durable/src/harness/tool.ts#L109)、[恢复分支](../../../../.repo/pi-1.2.0/packages/durable/src/harness/tool.ts#L170)。

| 状态 | 持有者与寿命 |
|---|---|
| entry、submission、terminal task receipt | Storage；关闭后保留，可重开查询 |
| `pi.agent` | conversation rewindable/asOf；保存名字与选择，不保存 extension 代码 |
| `pi.provider` | conversation latest/initial；持久 UUID，fork 新建，reset/compaction 不变 |
| `pi.live`、inbox、usage | conversation 当前状态；progress 也必须提交 |
| task checkpoint/memos/task docs | checkpoint 随 phase，memos 在完成时丢弃，task docs 随 terminal 原子退休 |
| registry/settings/env factory | 宿主进程；settings 每次读取，不写入存储 |
| tracker/context/view caches | 当前进程投影；可逐出、重建，不是第二份持久权威 |

文档 scope 决定生命周期；conversation 的 latest/rewindable 与 current/initial/asOf 决定历史和 fork。读取按需迁移，不持久化；`tx.doc()` 在成功事务写入版本 base。创建/版本转换必须 base，其余由 `checkpointWhen` 决定，backend 不代猜。`document.copy` 在存储内从 pre-batch source 复制，child 独立 incarnation；未知定义仍可保留和 fork。[文档契约](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L971)、[checkpoint/fork](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L1440)。

Session 在 callback 结束后撤销 drafts、准备不可变 value/ops，持有 mutation line 直到 Storage 结算与 adopt 完成；仅成功存储后发布。table 第一次写后禁止 table read，document 仍可读写。已准入 commit 不因调用者取消中断存储提交；嵌套 Session 调用或外部等待放进 callback 会阻塞整条线。[实现](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L690)、[规则与风险](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L1577)。

## 调度、取消、恢复与失败

每个任务显式归 conversation 或同 conversation 内另一 task；owned conversation 才跨 conversation。`pending/running/waiting/completing/terminal` 不是通用队列状态别名：waiting 没有 invocation，completing 已决定 outcome，等普通 owned work 排空才 terminal。非 completed outcome/abort mark 向下级联，abort handler 从叶到根。background conversation-owned task 是普通 abort/idle 的边界；显式包括 background 的操作才跨越。持久 wait 同时检查 wait 与 ownership hold 构成的环；内存 `waitForTask()` 不进入该图。[调度 reservation](../../../../.repo/pi-1.2.0/packages/durable/src/harness/scheduler.ts#L1122)、[wait cycle](../../../../.repo/pi-1.2.0/packages/durable/src/harness/scheduler.ts#L1473)、[结构并发](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L2161)。

重开把 running 改回 pending，保留 waiting/completing 并重评估；缺定义、旧定义或迁移失败保持 blocked，可在 registry 更新后解阻。普通 abort 无可用代码时才 orphan；`abandonOnRestart` 的 restart mark 先于恢复运行，并等待缺失定义的清理代码，不直接 orphan。取消 wait 只撤销等待；关闭信号终止所有 invocation，但不写任务 outcome，重开后按 checkpoint 继续。关闭会 join 不合作的工具/handler，后者不返回时关闭也无法完成。[恢复/缺定义](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L1977)、[close](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L483)。

1.2.0 的重大变化是 fatal storage rule：任何 Storage 读取、commit、mintId 的普通异常使 Session seal 并自动 close，后续调用和 pending waits 得 `SessionFailed(cause)`，watches 得 `session_failed`。只有无持久效果的非法读取 `StorageRequestError`、调用者已取消的读取豁免；commit/mintId 从不豁免。backend 自己处理 transient retry，Session 不重试。task、tool、hook、用户 listener 的错误各自收敛；内部 scheduler/observer bookkeeping 失效则 Session 失败，因为内存可能已落后存储。[GuardedStorage](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L1054)、[发布 listener 分类](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L744)、[规范错误矩阵](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L5036)。

工具输出默认保留 head 50 KiB/2000 行；运行输出按默认 100 ms 加写入大小/100 KiB·s⁻¹ 节流，不存未提交 progress。shell 可接收 tail output window 并报告 skipped，以避免传输必然丢弃的数据；完整输出由 env spill。watch/event 最多缓冲 100 个未交付 frame/batch，溢出用最新完整 snapshot/root replacement 收敛，不能充当审计日志。文档缓存默认无观察且十分钟未用可逐出，0 立即丢弃；context cache 空闲默认十分钟。observed docs 持续 pin；在不能 unref timer 的环境只在后续活动清理。[输出契约](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L3055)、[watch](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L4469)、[cache](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L598)。

## 嵌套工具、Code mode 与扩展

`api.executeTool()` 创建 owned `pi.tool` child，沿用参数验证、ToolTask hooks、replay、usage 和独立输出限制。nested result 返回程序，不进 transcript；caller 的 task-scoped `NestedCallDoc` family 按 key 存 taskId/result，caller terminal 后退休。默认 positional key 或显式 key 使 safe caller 重跑时重附着，工具名或 args 不同则拒绝；caller settle 前 abort/join 自己遗留的 nested calls。`callers` 和 `modelTools` 分别控制可调用范围与模型暴露；deferred 工具通过 tool-search/control.addTools 按分支加载。[NestedCallDoc](../../../../.repo/pi-1.2.0/packages/durable/src/harness/tool.ts#L84)、[嵌套 API](../../../../.repo/pi-1.2.0/packages/durable/src/harness/tool.ts#L360)、[cleanup](../../../../.repo/pi-1.2.0/packages/durable/src/harness/tool.ts#L578)。

Code mode 是本版新增的完整 Durable adapter：safe、仅 model 可调的 codemode 把脚本 calls 映射成 nested calls；先提交 start/fingerprint/deadline，再把每个结果按实际交付顺序写 journal，之后才 `run.deliver()`。恢复重放相同 clock、seed、结果、issued counts；版本、fingerprint、deadline 或调用轨迹不一致则 fail closed。run/journal 是 task docs，store 是 conversation rewindable/asOf doc；成功结果与 storeWrites 同一提交。VM 确定性和日志不使外部工具天然幂等。[start 与文档](../../../../.repo/pi-1.2.0/packages/durable/src/codemode/codemode.ts#L83)、[恢复检查](../../../../.repo/pi-1.2.0/packages/durable/src/codemode/codemode.ts#L402)、[journal-before-delivery](../../../../.repo/pi-1.2.0/packages/durable/src/codemode/codemode.ts#L956)。

extension 按名称注册并原位替换；每 phase 固定 snapshot，后续 phase 可 handover/migrate，新 request 解析新实现。uninstall 不会自动释放仍被旧 invocation 使用的资源，宿主管理资源寿命。hooks 只在被 conversation 选择时生效；array selection 可以省略 guard。此处是可信扩展机制，Session 全局 ID 不是 capability。[registry/hook/reload](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L2409)、[env 信任边界](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L3179)。

## 存储与持久性边界

MemoryStorage 是参考语义，读写脱离 alias，但不持久。SQLite 一批写入一个 SQL transaction，Node adapter 使用 WAL/synchronous NORMAL，进程崩溃与断电保证不同。Cloudflare adapter 将同一 portable SQL storage 装配在 Durable Object storage 上。JSONL 先 sidecars 后 main marker，恢复裁剪 torn/unconfirmed tails，已确认缺失是 corruption；未知 append 失败 poison backend。`fsync:true` flush sidecars 后追加 marker，普通提交不显式 flush main，所以不能承诺最近 acknowledged commit 断电必存。reclaim 前才 flush authorizing main；main 不全局 compact，JSONL 历史索引也全留内存。一个存储同时只有一个 owner，包本身无跨进程锁。[SQLite](../../../../.repo/pi-1.2.0/packages/durable/src/storage/sqlite/node.ts#L180)、[JSONL commit](../../../../.repo/pi-1.2.0/packages/durable/src/storage/jsonl/storage.ts#L278)、[reclaim](../../../../.repo/pi-1.2.0/packages/durable/src/storage/jsonl/storage.ts#L508)、[精确保证](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L5130)。

## 文档时效、测试预期与静态风险

README、CHANGELOG 及 5340 行规范本次通读；本包当前仅这三份默认文本资料，没有独立 handoff/PLANNING 文件。规范多数关键机制已兑现，包括结构并发、动态 registry、nested tools、deterministic replay、fatal storage、cache retention 和后置观察。仍须按具体主张分辨：

| 文档主张 | 当前状态与证据 |
|---|---|
| `SystemMessage` 等待 PR #9548 落地 | 规范开头为历史措辞；当前 generation 已使用 positional system，不能视为未来前置条件（spec:37、3419） |
| §2.2 `SubmissionDraft` 输入 `entry?:never` | 局部陈旧；§6、README 及 submissions.ts:151 已支持 InputEntry |
| §5.1 fault write 失败可在同进程再次 reservation | 被 1.2 fatal storage rule 替代；scheduler 自己 commit 出错会失败 Session，见 §10.1 和 CHANGELOG |
| 自动 checkpoint、CRDT、多写者、强停同进程扩展 | 明确非目标，不能写作待兑现路线图（spec:5326） |
| append/fanout/cache 性能数字 | 作者历史测量与 benchmark 入口，本次没有重新测量（CHANGELOG:34起） |

静态风险主要来自明示边界：可信 revision 被外部修改会损害权威；长 callback 阻塞所有提交；取消不合作代码拖住 shutdown；in-memory waits 的环无法由 durable cycle checker 发现；nested 深度、并发与 structuredOutput 没有统一总预算；unknown side effects 需要工具自己的幂等键；JSONL 与 terminal receipt/rewindable history 可持续增长。上述不等同本次运行故障。[footguns](../../../../.repo/pi-1.2.0/packages/durable/docs/spec.md#L5182)。

测试静态预期覆盖三 backend conformance、原子回滚/恢复/fork、watch convergence、generation/tool/submission/ownership、fatal storage 和 nested/replay。重点新断言：storage read/commit 失败终结所有 waits（`harness-storage-failure.test.ts:86`）、已完成 nested call 不重复但 safe 未结算 effect 可再执行（`codemode.test.ts:207`）、按结果到达顺序恢复（:224）、sandbox 主动关闭不决定脚本 outcome（:467）。所有测试和基准均未执行；测试影响：`none`，没有新增或修改测试。[错误测试](../../../../.repo/pi-1.2.0/packages/durable/test/harness-storage-failure.test.ts#L86)、[重放测试](../../../../.repo/pi-1.2.0/packages/durable/test/codemode.test.ts#L193)。

## 1.0.4 → 1.2.0

按两版源码清单：新增 44、修改 75、删除 4、相同 84 文件；当前 203 文件，`src` 82 个 TypeScript/25159 行。计数不是逐行审计率。重点语义变化是 ToolTask v2 tagged input/result 与 v1 migration、`content`→`output`/structuredOutput、nested calls、deterministic codemode、deferred tools、images、Cloudflare SQLite、扫描顺序和 lifecycle 时间、fatal storage、缓存/调度索引保留优化。对 caller 和 persistence 的影响远超版本号调整。[发行记录](../../../../.repo/pi-1.2.0/packages/durable/CHANGELOG.md#L3)。

旧报告中的 experimental runtime/server 调用方判断不再成立；当前应以独立 issue triage 与示例、env adapter 为实证。旧 prototype 的后续 handoff 不覆盖当前 5340 行规范和新实现。这里的升级比较是清单摘要差异与当前源码重读，不声称取得完整旧新逐行 diff；当前机制的证据均直接指向本快照。

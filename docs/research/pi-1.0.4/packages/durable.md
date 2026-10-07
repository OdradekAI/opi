# pi-durable 1.0.4 源码研究

分析日期：2026-10-07。本文完整保留前版已完成的机制分析；未变化源码与文档按 SHA-256 继承，受本次变化影响的接口、调用方及关键规划重新核查。继承范围与旧内容身份见[1.0.2 历史证据](../evidence/pi-1.0.2/README.md)，具体新读范围见[文档审读记录](../documentation-reading.json)。旧源码不在现场，升级比较属于清单摘要差异、当前实现与旧结论核对，不是逐行旧新 diff。

## 1. 范围与核心判断

本报告研究本地 `pi-1.0.4` 快照的 `packages/durable`。
证据是 manifest、类型、实现、测试和实际调用点；第 15 节保留前版对 README、发行记录和设计文档的完整分析，并按实际新读范围复核原意、落实状态与时效。
所有结论来自静态阅读，未安装依赖、未运行构建、测试、基准或模型请求。
文中“测试覆盖”表示存在相应测试及断言，不代表本次执行通过。

这个包同时包含两个层次：

- Session 内核负责 conversation、不可变 entry、可变 document、task/submission 表、事务和提交后观察。
- Harness 在内核上安装调度器、generation/tool/compaction 状态机，以及 agent、inbox、live、provider、usage 文档。

它直接依赖 pi-ai 和 Chord，未依赖 pi-agent-core；它有自己的 generation/tool 循环和任务生命周期。
其可恢复性来自“下一步所需状态已经提交”，不是把进程堆、执行栈、JavaScript Promise 或外部 shell 状态持久化。
内存权威状态和观察帧在 Storage 成功之后推进；外部工具副作用与数据库提交之间仍存在不可消除的未知窗口。

包内能力与产品使用须分开：默认 CLI 的 AgentSession 路径使用 pi-agent-core；本快照的 durable 消费者主要位于 coding-agent 的 experimental 目录。
该产品包发布文件清单排除了 `dist/experimental`，且没有声明 pi-durable 运行依赖。
因此不能把这些源码中的接线描述为默认安装后已启用的会话后端。

依据：[package.json](../../../../.repo/pi-1.0.4/packages/durable/package.json)、[index.ts](../../../../.repo/pi-1.0.4/packages/durable/src/index.ts)、[coding-agent/package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[experimental/runtime.ts:125](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L125)。

## 2. 发布面、依赖与平台边界

包名 `@earendil-works/pi-durable`，版本 1.0.4，ESM，Node engine `>=22.19.0`，`sideEffects: false`。
build 使用 TypeScript；test 使用 Vitest；另有 storage 与 tool-output 基准脚本。

| export | 用途 | 平台含义 |
|---|---|---|
| `.` | Session、Harness、定义函数、类型、默认内存实现 | 核心入口 |
| `/env` | FileSystem、ExecutionEnv、Result、错误类型 | 平台中立契约 |
| `/env/node` | NodeExecutionEnv | 文件系统、进程、Node API |
| `/tools` | read/write/edit/bash、可选 PowerShell 工厂与 CodingTools 扩展 | 借助注入 env 执行 |
| `/storage/memory` | MemoryStorage | 进程内存 |
| `/storage/jsonl` | JSONL 引擎 | 借助 FileSystem |
| `/storage/jsonl/node` | Node 文件后端装配 | Node |
| `/storage/sqlite` | SQL 引擎及 database façade | driver 可注入 |
| `/storage/sqlite/node` | 内置 node:sqlite façade | Node |
| `/testing` | 存储/环境一致性与基准工具 | 适配器验证 |
| `/package.json` | 包元数据 | 直接导出 |

主要入口都有 `source`、`types`、`import`、`default` 条件，未设置 `browser` 条件。
可移植性通过入口拆分和注入实现，不是浏览器条件自动替换 Node 实现。
Node adapter 使用内置 `node:sqlite`；manifest 没有额外 SQLite 原生 npm 模块。

运行依赖为 `@earendil-works/chord ^1.0.4`、`@earendil-works/pi-ai ^1.0.4`、`diff 8.0.4`、`typebox 1.3.27`。
开发依赖为 `shx 0.4.0`、`vitest 4.1.11`。
Chord 承担严格 JSON、不可变修订、变更草稿、delta 和 replicatedState；pi-ai 承担模型及 provider 统一边界。
diff 用于文本编辑差异，TypeBox 用于工具参数声明。

依据：[package.json](../../../../.repo/pi-1.0.4/packages/durable/package.json)、[sqlite/node.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/node.ts#L1)。

## 3. 源码地图

| 文件或目录 | 责任 | 关键阅读目的 |
|---|---|---|
| `types.ts` | 内核公开协议与 Storage 契约 | ID、所有权、状态、事务写集 |
| `documents.ts` | singleton/family 定义和地址解析 | scope、历史、fork、迁移 |
| `entries.ts`、`tasks.ts` | 定义辅助与记录边界 | 数据定义与实现分离 |
| `session/session.ts` | 串行提交、缓存、生命周期 | 存储前后权威变化 |
| `session/transaction.ts` | 草稿、表写、最终验证 | 原子候选写集如何形成 |
| `session/forks.ts` | 文档 fork 复制计划 | asOf/current/initial |
| `session/observation.ts` | state source 与 watch | 观察次序、溢出、取消 |
| `storage/memory.ts` | 索引、验证、分离值 | volatile 基准实现 |
| `storage/jsonl/*` | main+sidecar 日志与恢复 | 提交标记、截尾、回收 |
| `storage/sqlite/*` | schema、数据库 façade、SQL存储 | SQL事务和版本迁移 |
| `harness/harness.ts` | Session组合及 Conversation façade | 产品可调用生命周期 |
| `harness/scheduler.ts` | 任务预留、执行、等待、恢复 | 状态机核心 |
| `harness/generation.ts` | 模型请求、重试、deferred、工具轮 | 可恢复 Agent 循环 |
| `harness/tool.ts` | 工具意图、执行、输出、恢复 | 副作用重放边界 |
| `harness/compaction.ts` | 选切点、摘要、放置 | 上下文历史投影 |
| `harness/submissions.ts`、`inbox.ts` | 接纳、去重、steering/follow-up | 输入与执行解耦 |
| `harness/context.ts`、`prompt.ts` | transcript投影和系统段 | 存储记录到模型输入 |
| `harness/registry.ts`、`agent.ts` | 扩展安装、配置、解析 | 动态代码与持久化名字 |
| `harness/view.ts`、`events.ts` | Chord视图及事件适配 | 提交后的消费协议 |
| `harness/output.ts`、`truncate.ts` | 流式有界文本 | 增量输出与截断 |
| `env/*`、`tools/*` | 环境与编码工具 | 文件/进程权限来源 |
| `testing/*`、`test/*` | conformance、故障注入、示例 | 合约验证证据 |

调度器、事务、存储引擎和 generation 都是实际复杂度中心。
只读 `Harness.open` 或 README 示例会遗漏恢复、派生阻塞、所有权收束和提交失败规则。

## 4. 领域对象与公开数据类型

### 4.1 ID、conversation、entry 与 ownership

ID 是带 TypeScript brand 的安全整数；各种记录共用一个 ID 空间，根 conversation 固定为 1。
brand 是静态区分，不是独立 UUID 格式或运行时隔离机制。

conversation 的两个关系必须分开：

- `parent` 表示 transcript 历史继承：源 conversation 和包含端点 `at`。
- `owner` 表示任务归属：某个 task 创建并监督的 conversation。

分支不会因为拥有同一 owner 就共享历史；继承历史也不自动建立任务取消关系。
entry 不可变，带 conversationId、kind、可选模型 message 数组、JSON data、提交序号及可选 byTaskId。
`head` 与 `edits` 改变模型上下文投影；它们不原地修改旧 entry。

依据：[types.ts:20](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L20)、[types.ts:286](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L286)、[types.ts:317](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L317)、[transaction.ts:358](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L358)。

### 4.2 Document 的逻辑地址、实例与历史

document 的地址由 kind、scope、owner ID、是否 family 及 key 组成。
相同逻辑地址 retire 后可重建，但会获得新的 DocumentId；旧观察者绑定旧实例，不自动转向新实例。
singleton 不需要 key；family 以 string key 和初始化 seed 生成同类文档。

| scope/history | 生命周期与回看能力 | fork策略 |
|---|---|---|
| session | 整个Session所属 | 不声明conversation fork |
| task | 某任务所属，任务最终终结时退役 | 不声明conversation fork |
| conversation/latest | 仅当前状态 | current 或 initial |
| conversation/rewindable | 可按历史提交点读取 | asOf、current 或 initial |

`snapshot` 不存在时返回 undefined，不偷偷调用 initial。
`tx.doc` 才能在事务中创建缺失实例并获得可编辑草稿。
`snapshotAsOf` 验证 entry 可见性，使用该 entry 实际来源 conversation 和 commitSeq 读取当时文档实例。

文档 checkpoint 是“以完整 base 替代一段 delta 尾部”的存储选择。
任务 checkpoint 是“下一阶段执行需要的 JSON 状态”。
两者同名，但前者优化文档恢复，后者控制业务状态机，不能互相替代。

依据：[types.ts:42](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L42)、[documents.ts:119](../../../../.repo/pi-1.0.4/packages/durable/src/documents.ts#L119)、[session.ts:162](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L162)、[session.ts:316](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L316)。

内置 `ProviderDoc` 是这里需要单独辨认的 latest 文档。它公开为 `ProviderState { sessionId: string }`，kind=`pi.provider`、version=1、scope=conversation、history=latest、fork=initial，每次变化写完整 base。`initial()` 生成 UUIDv7；这个字符串是 provider-facing conversation identity，不是数字 `ConversationId`，也不是宿主应用外层 Session ID。常规 reset、模型切换、压缩及 reopen 不会更换它；新建和 fork 则得到各自新身份。
依据：[provider.ts:6](../../../../.repo/pi-1.0.4/packages/durable/src/harness/provider.ts#L6)、[index.ts:41](../../../../.repo/pi-1.0.4/packages/durable/src/index.ts#L41)、[spec.md:737](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L737)。

已有但缺少 `pi.provider` 的旧 conversation 不在 open 时全量迁移。`ensureProviderSessionId()` 先读当前 snapshot；已有值时不写入，没有时在 Session commit 中 `tx.doc()` 初始化，保证 provider 请求开始之前完成持久化。并发补建由同一 mutation line 选出持久化赢家；不是靠每次请求临时生成 UUID，也不是 task memo。
依据：[provider.ts:26](../../../../.repo/pi-1.0.4/packages/durable/src/harness/provider.ts#L26)、[spec.md:749](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L749)。

### 4.3 Task 定义、运行状态与终结结果

TaskDefinition 包含稳定 name/version、initial、按 phase 穷尽的 handlers、abort，以及可选 migrate。
记录只保存名字、版本、输入、checkpoint 和结果，不保存 handler 闭包。
registry 中当前代码是恢复必需条件。

```mermaid
stateDiagram-v2
    [*] --> pending
    pending --> running: 提交执行预留
    running --> running: 提交下一checkpoint
    running --> waiting: 持久化等待依赖
    waiting --> pending: 等待条件满足
    running --> completing: 已有结果但普通子工作未收束
    completing --> terminal: 所有普通子工作收束
    running --> terminal: 无待收束子工作
    running --> pending: 重开或兼容实现交接
```

terminal outcome 包含 completed、failed、aborted、orphaned、faulted。
failed 表示任务业务失败；faulted 表示运行时协议被违反，例如 handler 返回但没有推进 checkpoint 或终结。
orphaned 用于无法正常执行所需实现而被终结的情况；缺失实现并不一律立即变成 orphaned。

task 可以属于 conversation 或同 conversation 的另一 task。
background 只允许 conversation-owned；它是普通 idle、取消传播和父工作收束的边界。
不能把 background 理解为线程或进程隔离。

依据：[types.ts:233](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L233)、[types.ts:264](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L264)、[types.ts:449](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L449)、[types.ts:486](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L486)。

## 5. 一次提交的逐函数路径

内核的关键调用链是 `Session.commitWith → enqueue → runCommit → Transaction.settleSuccess → Storage.commit → Transaction.adopt → publish`。

1. `commitWith` 检查 Session 可用，把回调排入同一条 promise mutation line。
2. `runCommit` 创建 Transaction，执行调用方回调；回调失败进入 settleFailure。
3. Transaction 收集表操作与 document 草稿，复制输入 JSON，避免调用方后续改动污染候选写集。
4. `settleSuccess` 先确认没有未 await 的事务异步操作；否则整个事务失败并排空尚未完成的工作。
5. 每个 Chord change 执行 prepare，草稿被撤销，产生不可变 revision 和 delta。
6. `assembleWrites` 验证所有权、终结关系、submission 合并、文档创建/退役及 fork 来源冲突。
7. checkpointWhen 在其他验证之后运行；其异常仍能回滚整个候选事务。
8. 无写集时不调用 Storage；有写集则调用 Storage.commit。
9. Storage 成功后 `adopt` 将准备好的 revision 安装到内存 tracker。
10. `publish` 通知内部同步监听器，再由各观察适配器安排外部交付。

依据：[session.ts:94](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L94)、[session.ts:404](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L404)、[transaction.ts:662](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L662)、[transaction.ts:753](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L753)。

### 5.1 读写次序与候选状态

公开表读取必须发生在首次表写之前；之后再读会抛 ReadAfterWrite。
这使回调基于一个清晰的提交前表快照作决策，避免误以为通用查询已包含尚未提交写集。
内部验证仍有专用查找；document 草稿路径也有自己的缓存和候选合并，不能笼统说“写后完全不许读取任何东西”。

创建 task/conversation 时会检查 owner 存在及状态，组装写集时再次检查最终候选 owner。
因此同一事务里把 owner 终结，再留下新子工作会被拒绝。
task 最终 terminal 的事务会退役其 task-scoped 文档，包括该事务刚创建的文档。

文档可先 retire 再在同地址创建新实例；旧实例的最终草稿内容先保存，再记录退役。
fork 所选源文档不能在同一事务里被修改，以免“复制的是哪个时刻”变得模糊。

依据：[transaction.ts:300](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L300)、[transaction.ts:509](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L509)、[transaction.ts:772](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L772)、[transaction.ts:853](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L853)、[transaction.ts:919](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L919)。

### 5.2 取消、失败与 poisoned Session

提交被接纳后，调用 Storage 使用去掉 abort signal 的 Context。
调用方取消不能把已经进入存储阶段的操作当成可撤销事务。
StorageRejected 表示已知未产生效果的拒绝，Session 可以继续使用。
其他存储异常可能发生在“已经写入但响应失败”的窗口，Session 进入 poisoned 状态，必须关闭并重新打开恢复。
存储成功而 adopt 失败同样 poison，避免内存继续冒充可靠权威。

内部 commit listeners 的契约是同步、不抛异常、不阻塞、不重新调用 Session。
这是扩展运行时的内部纪律，不是任意用户回调安全沙箱；不能由提交后发布顺序推导“每种 listener 异常都已隔离”。
close 首先封闭新操作接纳，随后完成清理、排队收尾及 storage.close。
取消 close 的等待不会取消已经开始的清理。

依据：[session.ts:350](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L350)、[session.ts:383](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L383)、[session.ts:427](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L427)、[session-documents.test.ts:440](../../../../.repo/pi-1.0.4/packages/durable/test/session-documents.test.ts#L440)。

## 6. 文档版本、分支与持久化表示

定义检查要求 version 为正安全整数；声明类型不是通用内容 schema 校验。
值通过 Chord 严格 JSON 边界进入存储，迁移输出也要通过同样检查。
读取旧版本可以在内存中 materialize 新版本，但只读不写入迁移结果。
首次以新定义打开可写事务，会以新版本 base 落盘，即使业务代码没有额外修改字段。
较新持久化版本、缺失必要迁移、scope/history/fork 语义不一致都拒绝静默解释。

checkpointWhen 接收准备好的不可变 value、ops 及此前 base 后已有的 delta 数量。
普通无变化事务不会为了 checkpoint 谓词制造写入；结构性 change 即使最后值相等也可能形成 delta。
返回 true 写 base，否则写 delta；predicate 抛错使其他已经准备的文档一并回滚。

fork 的 entry 历史主要由 parent 指针继承，文档则创建新实例并复制选定状态：

- asOf：按所选 entry 实际来源 conversation 的该次提交点选择文档。
- current：按请求 fork 的 parent conversation 当前状态选择。
- initial：不复制，后续首次 tx.doc 使用初始值。

复制依靠持久化的文档语义，可以处理当前宿主未安装定义的文档。
复制后是子 conversation 独立状态，不是指向父 document 的可变引用。
如两种复制来源映射到同一逻辑地址，拒绝模糊结果。

依据：[documents.ts:175](../../../../.repo/pi-1.0.4/packages/durable/src/documents.ts#L175)、[documents.ts:197](../../../../.repo/pi-1.0.4/packages/durable/src/documents.ts#L197)、[transaction.ts:834](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L834)、[transaction.ts:998](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L998)、[forks.ts:27](../../../../.repo/pi-1.0.4/packages/durable/src/session/forks.ts#L27)、[session-checkpoints-migrations.test.ts:196](../../../../.repo/pi-1.0.4/packages/durable/test/session-checkpoints-migrations.test.ts#L196)。

## 7. Storage 实现及其保证范围

Storage 假定上层 Session 已验证 ancestry、状态转换等业务约束。
它仍须验证原子写集、全局 ID、不可变记录、文档一致性，并向调用方返回分离的值。
它是单个 Session 所拥有的后端，不是分布式事务协调或多进程调度租约接口。

### 7.1 MemoryStorage

内存实现维护各表 Map、排序 ID 索引、task状态、submission请求及文档地址索引。
prepareCommit 先复制并验证整个写集，包括解析 document copy，再获得可以原子应用的 prepared commit。
读取不会暴露可修改内部记录；写入也不会保留调用方可变对象。
它是 conformance 的便捷后端，但进程退出即丢失数据。

依据：[types.ts:985](../../../../.repo/pi-1.0.4/packages/durable/src/types.ts#L985)、[memory.ts:219](../../../../.repo/pi-1.0.4/packages/durable/src/storage/memory.ts#L219)、[memory.ts:250](../../../../.repo/pi-1.0.4/packages/durable/src/storage/memory.ts#L250)。

### 7.2 SQLite

SQL schema 分离 records 与 document_revisions，另有全局 record_ids 和 next_id/next_seq 元数据。
tasks/submissions 的状态、归属、请求 ID 有对应索引；JSON主体保存在带 json_valid 约束的 TEXT 列。
schema migrations 是连续版本、在事务中应用，遇到数据库版本比代码新则拒绝打开。

SqliteStorage.commit 在同一数据库 transaction 中验证并写入记录、文档 revisions 和元数据。
文档 materialize 使用截至目标序号的最近 base，再按顺序应用同版本 delta。
latest 文档写新 base 或退役时可删除旧 revisions；rewindable 保留历史。
读取 record 和 revisions 也使用事务，防止混读不同修订。

Node façade 把同步 DatabaseSync 包在串行异步接口中，事务用 BEGIN IMMEDIATE。
事务失败先 ROLLBACK；如果回滚也失败，产生不同 AggregateError，不能继续声称原异常意味着无效果。
transaction callback 必须使用传入 scoped handle；在其中等待同一 parent façade 的排队操作可能死锁。

打开时配置 WAL、synchronous=NORMAL，默认 busy timeout 5秒、WAL自动 checkpoint 1000页。
close 做 WAL checkpoint(TRUNCATE) 后关闭。
这是源码配置事实，不足以宣称所有文件系统、电源故障或多进程并发都具有同等耐久性。

依据：[migrations.ts:8](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/migrations.ts#L8)、[migrations.ts:97](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/migrations.ts#L97)、[sqlite/storage.ts:162](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/storage.ts#L162)、[sqlite/storage.ts:542](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/storage.ts#L542)、[sqlite/database.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/database.ts#L1)、[sqlite/node.ts:183](../../../../.repo/pi-1.0.4/packages/durable/src/storage/sqlite/node.ts#L183)。

### 7.3 JSONL：main 是确认点，sidecar 是载荷

JSONL 后端复用 MemoryStorage 作为内存索引和候选验证器。
main.jsonl 保存提交序号与操作标记；document 内容和活跃 task checkpoint 等放到独立 sidecar。
terminal task 的最终记录进入 main，以便回收活跃 checkpoint sidecar。

普通提交顺序：

1. MemoryStorage.prepareCommit 冻结分离的候选写集。
2. 为 sidecar 生成带提交序号和 ordinal 的记录。
3. 依次 append sidecar；启用 fsync 时 flush 这些 sidecar。
4. append main 的确认标记。
5. 应用内存提交，随后尝试回收旧载荷。

**fsync 默认为 false；即使 true，也不是每次提交都 flush main。**
main 的 flush 发生在需要回收旧 sidecar 且启用 fsync 时，用来确认删除旧恢复材料之前新的主标记已被推进。
对应测试明确要求普通提交只有 append main，不包含 flush main。
因此“Storage.commit 返回”不能直接改写成“这次 main 标记已显式 fsync”。

回收可写 `.reclaim` 临时文件、可选 flush 后 rename；退役或terminal载荷可以删除。
回收是提交后尽力操作，失败可留下旧材料供以后处理，不推翻已经确认的提交。

依据：[jsonl/storage.ts:241](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L241)、[jsonl/storage.ts:277](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L277)、[jsonl/storage.ts:450](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L450)、[jsonl/storage.ts:507](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L507)、[jsonl-storage.test.ts:533](../../../../.repo/pi-1.0.4/packages/durable/test/jsonl-storage.test.ts#L533)。

恢复先读取 main 的递增提交序列，再解析 sidecar，按确认的 `(seq, ordinal)` 重建内存状态。
只存在于 sidecar、没有 main 确认的尾部可以截掉；被确认且仍必需的载荷缺失则报损坏。
已回收的 latest 历史或 terminal task 旧checkpoint不再要求存在。
未确认记录之后又出现已确认记录被视为非法顺序。
末尾不完整的一行可以截断；完整却非法 UTF-8/JSON 的行不能随意跳过。

该后端会读入日志并重建内存索引；从实现推断，启动成本与保留日志量、文档回放尾长有关。
源码没有使它成为可任意多进程共同 append 的日志服务；多写者协调不能由文件格式本身推导。

依据：[jsonl/storage.ts:534](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L534)、[jsonl/storage.ts:707](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L707)、[jsonl/storage.ts:785](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L785)、[jsonl-storage.test.ts:1060](../../../../.repo/pi-1.0.4/packages/durable/test/jsonl-storage.test.ts#L1060)。

## 8. Harness 调度、任务恢复与并发

Harness.open 安装 Session 的创建钩子、TaskScheduler、Submissions、TaskGraph 和 ConversationViews。
新 conversation 在同一事务中建立 Agent/Live/Inbox/Provider/Usage 五个基础文档，然后调用宿主的创建钩子。Provider 身份也适用于 task-owned、原始 Tx 创建与 fork；fork 为新会话重新生成 UUID，不继承父会话的 provider 身份。
root 使用保留 ID；已存在时不会重新应用首次创建选项。

openTasks 扫描 surviving nonterminal tasks，把上次留在 running 的记录改回 pending，完成必要协调，但不立即调度执行。
显式 resume 或某些提交/等待入口开启调度；这样宿主可以先安装和观察恢复状态。
close 停止调用实例并等待它们收束，不把所有任务写成 aborted；下次打开仍可从 checkpoint 继续。

依据：[harness.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/harness.ts)、[scheduler.ts:231](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L231)。

### 8.1 串行决策，线外执行

调度器在 Session mutation line 上观察、预留和提交状态，handler 在线外 await 执行。
reserve 将所有当前可运行任务在一次提交中标成 running，之后启动这些调用。
已读调度器没有通用的最大同时执行数量；事务串行不意味着模型或工具工作全都串行。
运行调用绑定独立 abort controller、结束标记和 registry snapshot。

每次阶段结束必须已经提交改变过的 checkpoint、等待或结果。
仅更新进度文档而保持同一个 checkpoint，不能证明业务阶段向前推进。
违反这个规则会 fault，防止无进展 handler 被无界重调度。

新的 registry 实现可以在阶段边界接管兼容版本；不兼容实现不会强行 reinterpret 当前阶段。
缺失定义、版本太旧或迁移失败可以形成派生 blocked 状态，保留记录等待实现恢复。
迁移失败会按定义对象缓存，替换该定义后才重新尝试。

依据：[scheduler.ts:683](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L683)、[scheduler.ts:755](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L755)、[scheduler.ts:853](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L853)、[scheduler.ts:879](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L879)。

### 8.2 等待、归属和取消

wait 在 mutation line 上检查并登记观察，避免“检查未结束，登记前恰好结束”的丢失唤醒。
等待可以 allSettled 或 failFast；后者只允许直接拥有的任务，失败会持久化其他等待任务的取消意图。
不能等待自己、owner 或不存在的任务。
代码未提供通用任意依赖图环检测；复杂 sibling wait 图仍值得专门验证。

任务已有结果但普通后代未收束时，进入 completing 而非直接 terminal。
reconcile 持续传播持久化的取消标记、撤回队列输入并推进 completing，直到稳定。
background 构成普通归属遍历的边界；显式选择包含 background 的取消另行处理。

abortTask 先持久化 abortRequested，再取消现有调用，之后运行独立 abort handler。
abort handler 自身也可能在关闭/崩溃后再次执行，必须正确处理重入。
结束实例的 runtime APIs 会拒绝迟到写入，检查不只发生在排队前，也发生在 Session 线上执行时。
但任意 handler 若忽略 signal 并永不返回，运行时不能强杀它；close/join 也可能一直等待。

依据：[scheduler.ts:274](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L274)、[scheduler.ts:441](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L441)、[scheduler.ts:980](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L980)、[scheduler.ts:1003](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L1003)、[scheduler.ts:1189](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L1189)、[harness-tasks-recovery.test.ts:147](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tasks-recovery.test.ts#L147)。

runtime.memo 在同一任务内以首次成功写入为准保存值；终结时删除执行 memos。
这可以保存阶段间稳定决策，但不能把外部副作用自动包装成 exactly-once。
registry 每阶段快照稳定；agent 在该阶段首次解析后复用；settings getter 则可以每次读取最新宿主配置。

## 9. 输入、模型请求与工具轮的完整流程

### 9.1 Submission 与 Inbox

submit 先做 durable admission，再安排 generation；接纳、放入队列、创建工作可以在一个 Session commit 中完成。
requestId 在 conversation 内去重：现有记录类型相同则返回原 submission，类型不同拒绝。
它不比较新旧 payload，因此不是内容一致性校验或全局幂等键。

运行中 input 可以 reject，或进入 steering/follow-up；write 在运行中排队。
边界先处理排队 write，再按原始 ID 顺序选择输入。
postTools 边界通常消费 steering，final 边界还消费 follow-up；reset 会改变边界及上下文起点。
one-at-a-time/all 设置决定一次选择多少输入。
比当前 head 更旧的排队 write 会变成 unanswered/stale，而非强行覆盖新上下文。

submission.abort 只撤回还 queued 的输入；已 placed 返回 already_placed，不等价于 abortTask。
wait Context 的取消只取消调用方等待，不撤销已接纳任务。

依据：[submissions.ts:56](../../../../.repo/pi-1.0.4/packages/durable/src/harness/submissions.ts#L56)、[submissions.ts:141](../../../../.repo/pi-1.0.4/packages/durable/src/harness/submissions.ts#L141)、[inbox.ts:44](../../../../.repo/pi-1.0.4/packages/durable/src/harness/inbox.ts#L44)、[inbox.ts:127](../../../../.repo/pi-1.0.4/packages/durable/src/harness/inbox.ts#L127)。

### 9.2 GenerationTask 的逐函数路线

checkpoint 区分 prepare、request、retry、poll、tools；每个阶段都保存恢复必需参数。

1. `prepare` 解析 agent/model，渲染系统段，计算是否需要压缩。
2. 提交 system entries、固定 model/thinking/options 和 context cutoff，进入 request。
3. `request` 若发现上次已提交部分回答，先将它转为 aborted assistant entry。
4. 以固定截止点重新派生 context，执行 beforeRequest hooks，确保该 conversation 的持久 provider 身份存在，再以它覆盖 `options.sessionId` 调用 models.streamSimple。
5. `streamResponse` 收集流事件，将部分 assistant 以节流方式写入 LiveDoc。
6. `classify` 区分 deferred、error、正常回答及 toolUse。
7. deferred 保存 handle 与下次 poll 时间；普通错误可保存 retry deadline；工具回答进入 startToolRound。
8. `startToolRound` 在一次提交中追加 assistant、创建工具task和live slots，父task等待子任务。
9. `finishTools` 汇总 control、处理队列边界，安排下一次 generation 或结束本次run。
10. `answer` 将回答、usage、submission结果及后续输入边界合并提交。

依据：[generation.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)。

部分消息通过 `settings.progress.partialIntervalMs` 节流，默认100ms，最多一个存储写入在途；provider 可能反复更新同一个消息对象，因此 flush 时先同步复制。
finally 停止定时器并等待在途写入，避免 terminal 之后又写入过期partial。
快速结束时不必把最后一个待flush partial单独存下，最终完整assistant会走终结提交。

恢复不是从某个 text delta 继续连接原网络流：普通 request checkpoint 会重发固定上下文。
已提交 partial 被标为 aborted，模型上下文投影排除这类失败/中断 assistant。
deferred 是不同机制：持久化handle可以恢复poll，而不是重发普通请求。
固定模型在恢复时不再可用会 no_model，不任意换另一个模型解释旧checkpoint。

依据：[generation.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)、[harness-generation-recovery.test.ts:124](../../../../.repo/pi-1.0.4/packages/durable/test/harness-generation-recovery.test.ts#L124)、[harness-generation-recovery.test.ts:200](../../../../.repo/pi-1.0.4/packages/durable/test/harness-generation-recovery.test.ts#L200)。

`request` 合并 options 时，先展开固定的 stream options，再覆盖 runtime signal 和 `ensureProviderSessionId()` 得到的 session ID，最后附上非 off 的 reasoning。因而身份属于 conversation，而非当前模型、一次 generation 或宿主提供的任意 session 字符串；依赖 provider 如何使用该字段决定 prompt-cache/session affinity 的实际效果。挂起的 deferred poll 是已有 handle 路径，不应把“generation 请求传递身份”写成恢复原有网络流。
依据：[generation.ts:194](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts#L194)、[harness-generation.test.ts:352](../../../../.repo/pi-1.0.4/packages/durable/test/harness-generation.test.ts#L352)、[harness-generation.test.ts:389](../../../../.repo/pi-1.0.4/packages/durable/test/harness-generation.test.ts#L389)。

### 9.3 重试、工具顺序与控制结果

默认 harness retry 开启，maxRetries=3、baseDelayMs=2000，持久化下一次deadline。
这是最多初次加三次重试的generation策略，与pi-ai的HTTP重试是不同层。
每次失败assistant和usage也会追加，不能将最后成功成本当全部成本。
上下文溢出走一次阻塞压缩路径，不同时套普通错误重试。

只有 stopReason 为 toolUse 且实际存在工具调用才创建工具轮。
length 中带未完整工具内容不会直接执行。
本轮允许调用的名字来自已提交请求上下文；beforeRequest 的临时替换不会自动扩充这个持久化授权集合。
任一相关工具要求 sequential 或settings指定时，整轮串行，前一子任务终结后才创建下一子任务。
并行时一起创建子任务，各结果按完成时刻提交；后续模型投影会恢复原call顺序。

只有 completed 工具outcome提供 control。
全部slot都明确 terminate 才终止自然工具续轮；单个工具终止不代表整轮终止。
控制还可添加工具或handoff；多个handoff以call顺序最后一个为准。
提交队列、reset、handoff和afterTools hooks仍参与后续run边界。

依据：[agent.ts:19](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L19)、[generation.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)。

## 10. ToolTask：意图日志、输出与外部副作用

call阶段先按当前agent找到工具，prepareArguments修复输入，校验参数，再执行beforeTool hooks。
hook可阻止或替换参数；hook之后重新校验。
最终参数和 `replay: tool.replay ?? "unsafe"` 组成 execute intent，与Live slot running状态一起提交。
随后同一次调用直接执行工具；execute阶段handler主要用于恢复已提交意图。

恢复时只有“意图记录是safe”且“当前可解析工具仍是safe”才重跑。
safe重跑先清除旧progress，使用存储参数及当前conversation环境。
工具被取消选择、移除或改成unsafe时，不再重跑；以已提交partial构造interrupted错误结果。
这里的safe是宿主声明，运行时不分析任意代码是否幂等。
内置read/write/edit/bash均未在注册中显式设safe，因此按上述缺省规则视为unsafe。

依据：[tool.ts:55](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L55)、[tool.ts:82](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L82)、[tool.ts:94](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L94)、[harness-tools-recovery.test.ts:109](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tools-recovery.test.ts#L109)、[harness-tools-recovery.test.ts:136](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tools-recovery.test.ts#L136)。

OutputBuffer对文本应用byte/line限制、head/tail策略与截断诊断。
progress更新受 `settings.progress.outputIntervalMs`（默认100ms）与100 KiB/s写入量共同节流，用有界overlap计算append/trim，并修改字符串叶子，使Chord生成较小delta；不是每次复制整个LiveDoc作为协议载荷。
details走严格JSON复制，diagnostics追加；结束后拒绝迟到的output操作。
最终结果缺少content时用已收集output，缺少details时用已有details，再执行afterTool并处理最终文本界限。
非文本content被保留，不能由文本上限推导所有图片或JSON载荷都受同一byte上限控制。

settle在同一提交中写toolResult entry、usage、slot done与task结果。
工具主动返回isError可仍是completed；执行抛异常则会形成failed并触发所拥有工作的取消收束。
因此模型看到的错误标志与任务调度失败语义不同。

依据：[tool.ts:169](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L169)、[tool.ts:285](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L285)、[tool.ts:339](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L339)、[tool.ts:372](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L372)、[tool.ts:480](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L480)。

## 11. Transcript 投影、压缩与扩展解析

context先在线上捕获tail/head边界，再在线外扫描不可变entry，避免长投影占住所有写入。
edits以较新的修改为准，head决定活动上下文范围；原始审计entry仍保留。
error、aborted和deferred assistant不进入正常模型上下文。
orderToolResults按assistant中的call顺序重排结果，缺失调用补错误result，无法匹配的结果不乱插入。

CompactionTask保存select/summarize/retry阶段及固定model、options、tail、firstKept。
阈值默认reserveTokens=16384、keepRecentTokens=20000；background阈值另留32768缓冲。
估算优先采用最近可用assistant usage，再估算其后缀，不等于精确provider tokenizer。
切点避免把工具调用与结果拆开，摘要模型输入是选择过的文本序列：工具结果最多2000字符，图片未转成完整语义描述。
摘要仍关闭 cache 及 deferred，要求正常 stop、非空文本且无 toolCall；length 摘要不直接采用。1.0.2 的摘要 options 也确保并传递同一 `pi.provider.sessionId`，覆盖展开的 stream options；但 `cacheRetention: "none"` 保持不变。Codex adapter 会因此抑制 cache/session identity，所以“传了 sessionId”不等于“摘要启用了缓存或持久连接”。

阻塞子任务可直接提交摘要；conversation-owned背景压缩通过带requestId的write submission放置。
若完成时head已经前进，该写可能stale，避免旧摘要覆盖新上下文。
压缩追加新的投影记录，不删除旧transcript。

依据：[context.ts:22](../../../../.repo/pi-1.0.4/packages/durable/src/harness/context.ts#L22)、[context.ts:58](../../../../.repo/pi-1.0.4/packages/durable/src/harness/context.ts#L58)、[context.ts:131](../../../../.repo/pi-1.0.4/packages/durable/src/harness/context.ts#L131)、[compaction.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/compaction.ts)。

这项身份接线没有把 compaction 改成“重发精确上一请求，只允许摘要且 toolChoice=none”的缓存友好方案。当前仍构造专门的摘要 system/user 消息，handoff 中该方案的 **Not in v1** 仍成立；规范也明确记录 none 与 provider 行为的关系。压缩 faux 测试关闭模拟 prompt cache，是为了让阈值/usage 断言继续专注于计量逻辑，不是实际 provider 缓存行为的实测。
依据：[compaction.ts:156](../../../../.repo/pi-1.0.4/packages/durable/src/harness/compaction.ts#L156)、[spec.md:3772](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L3772)、[pico-v5-handoff.md:729](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L729)、[harness-compaction.test.ts:143](../../../../.repo/pi-1.0.4/packages/durable/test/harness-compaction.test.ts#L143)。

Registry内置generation/tool/compaction任务；扩展不能用重复task name替换内置任务。
同名extension安装会替换原安装位置，工具及section在选中扩展顺序中解析，后面的同名项可以覆盖前面项。
wrappers随后应用；wrapper抛异常或改名会移除该目标并report，而非保留一个半成功包装结果。
AgentDoc保存扩展选择、工具选择、模型、cwd等配置名称；它是rewindable/asOf并按写入checkpoint的文档。
fork保留该时点配置；task-owned新conversation可继承owner当前agent配置；普通新conversation从空配置开始。

依据：[registry.ts:10](../../../../.repo/pi-1.0.4/packages/durable/src/harness/registry.ts#L10)、[agent.ts:41](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L41)、[agent.ts:137](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L137)、[agent.ts:158](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L158)。

## 12. Chord 接线、事件语义与实际消费者

### 12.1 从 committed document 到 replicated state

documentState在线上定位实例并绑定CommittedStateSource，然后调用Chord replicatedState建立附着状态。
source用单调本地cursor区分revision，提供初始快照及之后delta，绑定的是document实例ID。
文档退役发布root=null；重建同逻辑地址不会让旧source自动跟随。
state source与watch各有生命周期，释放最后的attachment后清理订阅。

watch采用提交后的异步串行listener，避免用户异步工作占据Session mutation line。
待交付帧上限100；溢出时用最新完整root替换尚未交付队列，而不是无限积累delta。
已进入listener的在途帧不折入reset；listener失败只结束该watch。
stop会阻止后续交付并清空待交付帧，但closed可在当前listener仍await时完成，不表示强杀或等待用户回调。
交付Context去掉原提交调用方的abort signal，避免“提交者不再等待”取消别人的观察。

依据：[session.ts:212](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L212)、[observation.ts:24](../../../../.repo/pi-1.0.4/packages/durable/src/session/observation.ts#L24)、[observation.ts:155](../../../../.repo/pi-1.0.4/packages/durable/src/session/observation.ts#L155)、[observation.ts:221](../../../../.repo/pi-1.0.4/packages/durable/src/session/observation.ts#L221)、[session-watches.test.ts:159](../../../../.repo/pi-1.0.4/packages/durable/test/session-watches.test.ts#L159)、[session-watches.test.ts:237](../../../../.repo/pi-1.0.4/packages/durable/test/session-watches.test.ts#L237)。

### 12.2 ConversationView 与 event façade

viewState/watch组合活动entries及五个内置文档：pi.agent、pi.live、pi.inbox、pi.provider、pi.usage。
自定义document不会自动加入这个总视图，应独立通过documentState/watchDoc消费。
首次挂载在Session线上hydrate并注册监听，消除初始快照与更新之间的空窗。
一次提交把document路径加前缀，再追加或切换entries，形成一个一致的view delta。

events适配器从committed changes派生message/tool/turn/run事件，不是原始provider SSE事件转发。
message_update来自已保存partial，tool_start来自已提交execute intent，tool_end先于相应result message。
completing阶段即可派生某些turn结束事件，terminal后避免重复。
慢消费者溢出时得到snapshot恢复视图；所以这是面向状态消费的事件流，不是保证每条中间事件永久可重放的审计总线。

依据：[view.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/view.ts)、[events.ts:140](../../../../.repo/pi-1.0.4/packages/durable/src/harness/events.ts#L140)、[events.ts:195](../../../../.repo/pi-1.0.4/packages/durable/src/harness/events.ts#L195)。

1.0.2 增加的是 `ConversationView.docs["pi.provider"]` 挂载：初始 hydrate 与之后的提交投影均使用包含 ProviderDoc 的 MOUNTED 列表。它不改变通用 documentState、Chord delta 或慢消费者 reset 算法；也不意味着 `AgentEvent` 的语义 snapshot 增加 provider 字段，event façade 与总文档视图是不同消费面。
依据：[view.ts:48](../../../../.repo/pi-1.0.4/packages/durable/src/harness/view.ts#L48)、[spec.md:4005](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L4005)、[harness-view.test.ts:20](../../../../.repo/pi-1.0.4/packages/durable/test/harness-view.test.ts#L20)、[events.ts:140](../../../../.repo/pi-1.0.4/packages/durable/src/harness/events.ts#L140)。

### 12.3 源码中确实存在的消费路径

experimental durable runtime创建ModelRuntime、settings和registry，安装CodingTools、系统prompt及Subagent。
它用openNodeSqliteStorage打开会话数据库，再Harness.open，root.viewState提供给UI模型。
它将commit/Chord帧通知合并到setImmediate后渲染，避免在Session线上直接做整套UI工作。
同目录ExecutionEnvs按cwd复用NodeExecutionEnv，设置代理和HTTP超时；这些是宿主装配，不是Harness自己选择系统权限。

chord-guide测试进一步展示自定义canvas/document family/task输出如何绑定service facet，并通过模拟remote transport消费。
这些是可运行测试中的扩展模式，不应写成包默认就安装canvas、diff或任意业务服务。
跨网络transport、鉴权和应用Session协议也由更上层提供；documentState本身不建立网络端口。

依据：[experimental/runtime.ts:125](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L125)、[experimental/runtime.ts:174](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L174)、[harness-setup.ts:58](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L58)、[chord-guide.test.ts:1](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L1)。

## 13. ExecutionEnv 与编码工具的权限边界

平台中立 env 使用 Result 区分预期 FileError/ExecutionError。`FileSystem.id` 表示共享文件命名空间，不能只用 cwd 区分同一文件系统上的互斥资源。NodeExecutionEnv.id 为 `node:local`；路径可指向任意宿主允许访问的位置，没有路径根沙箱。`noFollow` 只拒绝最终路径分量的 symlink，不能防止前缀换链或并发 rename；环境是受信任能力，不是安全隔离层。
依据：[env/index.ts:171](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L171)、[env/node.ts:603](../../../../.repo/pi-1.0.4/packages/durable/src/env/node.ts#L603)、[spec.md:2873](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L2873)。

1.0.3 把三组能力变成必需接口：`openBinaryReader`/`openDirReader`、`BinaryReader.scanLines`、`watch`。binary reader 持有已打开普通文件，rename 后仍读同一对象；目录 reader 逐页读取元数据并跳过已经消失/不支持的项目；`scanLines` 在环境侧完整扫描一遍，返回零基行区间的物理字节范围与解码后的 UTF-8 尺寸。它使远程 read 不必传输全文，但完整扫描仍有 O(file size) 本地 I/O。
依据：[env/index.ts:95](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L95)、[env/index.ts:140](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L140)、[env/index.ts:193](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L193)、[line-scan.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/env/line-scan.ts#L1)。

Shell.exec 同时接收 shell string 和直接 argv，`onOutput` 第三个参数提供 stdout/stderr 及可选 skipped。取消/timeout 只杀本次命令，cleanup 面向环境所有仍在运行的命令。Node Windows 使用 taskkill，其他平台先进程组 kill；输出 spill 保留完整原始数据，等待 I/O 收束后返回。共享 StreamDecoder 只去掉最开始的 BOM，后续 U+FEFF 即使位于 chunk 边界也保留。
依据：[env/index.ts:251](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L251)、[env/node.ts:274](../../../../.repo/pi-1.0.4/packages/durable/src/env/node.ts#L274)、[decode.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/env/decode.ts#L1)。

watch 的建立完成意味着初始覆盖已建立；missing target 的创建、替换、移走、重建均是变化。native 事件只触发快照重扫；Windows 与网络/FUSE 默认 polling，macOS 安装 watcher 后补重扫。默认 poll 为2秒、目录上限10,000；溢出表示应全量重读，终态 error 后不再回调。polling 可漏掉两次快照之间创建又恢复的变化，不能等同每次写入的审计日志。1.0.4 明确 permission_denied，修正重叠 recursive、symlink target、目录同路径替换及 close/rescan。Durable 核心不自动调用文件 watch，它供宿主资源加载者使用，与 Session watchDoc 的持久提交观察是不同契约。
依据：[env/index.ts:110](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L110)、[node-watch.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/env/node-watch.ts#L1)、[spec.md:2859](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L2859)、[CHANGELOG.md:5](../../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md#L5)。

read 现经打开的 reader 读取图像头/PNG chunk 头、扫描行、只取显示区间；仍不返回图片。开头 BOM 在整文件解码语义下处理，过长单行按字符边界截断，offset/limit 保持旧 slice 语义。读取前后 size 增大直接接受（支持活动日志），同尺寸且 mtime 不变也接受；缩小或同尺寸重写才重读一次，再次变化报错。**分析者推断：** size 增长只是一项接受条件，无法证明旧字节没有同时被重写，所以这是并发读取容忍策略，不是不可变文件快照保证。
依据：[read.ts:73](../../../../.repo/pi-1.0.4/packages/durable/src/tools/read.ts#L73)、[read.ts:106](../../../../.repo/pi-1.0.4/packages/durable/src/tools/read.ts#L106)、[tools.test.ts:284](../../../../.repo/pi-1.0.4/packages/durable/test/tools.test.ts#L284)。

edit/write 继续以 env.id+canonical path 做进程内合作互斥，不防外部编辑器、bash 或其他进程。CodingTools 仍只含 read/write/edit/bash；1.0.4 单独导出的 createPowerShellTool 不会自动加入该扩展。它直接 argv 启动 pwsh，spawn_error 才尝试 powershell，不把已运行脚本的非零退出当成启动失败重跑；设置 UTF-8，复用 tail、spill 和输出诊断。
依据：[file-mutation-queue.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/tools/file-mutation-queue.ts#L1)、[tools/index.ts:21](../../../../.repo/pi-1.0.4/packages/durable/src/tools/index.ts#L21)、[bash.ts:77](../../../../.repo/pi-1.0.4/packages/durable/src/tools/bash.ts#L77)、[bash.ts:148](../../../../.repo/pi-1.0.4/packages/durable/src/tools/bash.ts#L148)。

### 13.1 远程输出窗口与最终持久化

ToolExecutionApi.outputWindow 只为 tail 工具提供 `{maxBytes,maxLines,minIntervalMs,bytesPerSecond}`。bash/PowerShell 传入 env.exec；环境可以省略已被更长后缀严格排除的前缀，同一输出事件必须携带全部后缀和 decoded bytes/newlines/endsWithNewline。output(chunk, skipped) 清除旧保留段并补计丢弃数量，保证最终保留文本与诊断计数等同收到全文；head 不接受 skip。需要独立 stdout/stderr 的消费者不传 window，因为省略后的合并 chunk 用最后一个 stream 标记。
依据：[tool.ts:193](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L193)、[env/index.ts:297](../../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts#L297)、[output.ts:144](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L144)。

OutputBuffer.snapshot 保留略大于窗口的余量，避免进度提交的时机改变后续 tail 起始行；BOM 只从最初 byte output 移除，tail 起点或字符串中的 U+FEFF 不能被当新流头丢掉。Progress 至多一个 commit 在途，默认100ms但还按写入字节/100KiB/s放慢；最终 commit 收束尚未提交的 details/output。interval 使用 nullish 默认合并，显式 undefined 不会抹掉默认值。这些是采样频率，不是“崩溃最多丢100ms”。
依据：[output.ts:205](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L205)、[output.ts:260](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L260)、[agent.ts:53](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L53)、[harness-output-skip.test.ts:100](../../../../.repo/pi-1.0.4/packages/durable/test/harness-output-skip.test.ts#L100)。

新 pi-env 真正消费这组契约：RemoteExecutionEnv → Connection → Rust daemon，在远端扫描文件、执行命令、保留 spill、按窗口减少传输；模型、凭据和 Session 存储仍由本地 Harness 宿主拥有。它是新可选环境，不代表默认 coding-agent 已切换。详见[Env 报告](env.md)。

## 14. 测试矩阵、证据限度与后续阅读

前版完整阅读核心Session/Transaction/Scheduler/Generation/Tool/Compaction/Context/观察适配等实现；本次按不变摘要继承内核和存储分析，重新阅读变化的环境、工具、输出与进度接线。
恢复测试重点读具体SQLite reopen、partial保留、安全重跑、失败注入与观察生命周期断言；其他测试按机制索引。
未运行任何测试，以下不是成功率或覆盖率统计。

| 测试入口 | 具体静态证据 | 不能据此推导 |
|---|---|---|
| session-documents:440 | 不确定Storage错误poison，旧revision和publication不前进 | 真实介质故障已被穷尽 |
| session-tables:91 | 首次表写后公开读取拒绝 | SQL本身提供全部业务验证 |
| session-checkpoints-migrations:196 | predicate抛错回滚所有准备文档 | 用户migration总能成功 |
| session-checkpoints-migrations:333 | 只读迁移不写存储 | 所有旧版本自动兼容 |
| session-checkpoints-migrations:648 | asOf读取与fork跨版本历史 | 任意定义冲突可自动合并 |
| jsonl-storage:458 | append前/后/短写失败后确认状态恢复 | 完整物理断电模型 |
| jsonl-storage:533 | sidecar flush、main标记、回收次序 | 每次main均fsync |
| jsonl-storage:1139 | 完整坏行拒绝、不同于尾部截断 | 任意损坏都可无损修复 |
| harness-generation-recovery:124 | partial变aborted后重发相同上下文 | 原网络流断点续传 |
| harness-generation-recovery:164 | retry deadline跨重开保存 | provider请求只发一次 |
| harness-generation-recovery:200 | deferred handle恢复poll | 所有provider支持deferred |
| harness-tools-recovery:136 | stored/current safe真值组合 | safe声明由运行时证明 |
| harness-tools-recovery:165 | rerun采用当前cwd，取消选择后不跑 | 重跑总与第一次环境相同 |
| harness-tasks-recovery:147 | abort各阶段关闭后再执行 | 忽略signal的代码能被强停 |
| session-watches:159 | 在途帧与overflow reset分离 | 每条中间帧都永久交付 |
| session-watches:237 | stop不绑定原commit取消 | stop等待任意用户回调退出 |
| chord-guide | document、family、task状态和remote service示例 | 这些业务服务默认内置 |
| storage-conformance | 后端可复用的记录/文档/分页/原子性契约 | 未执行的新driver已获认证 |

测试依据：[session-tables.test.ts:91](../../../../.repo/pi-1.0.4/packages/durable/test/session-tables.test.ts#L91)、[session-checkpoints-migrations.test.ts:333](../../../../.repo/pi-1.0.4/packages/durable/test/session-checkpoints-migrations.test.ts#L333)、[session-checkpoints-migrations.test.ts:648](../../../../.repo/pi-1.0.4/packages/durable/test/session-checkpoints-migrations.test.ts#L648)、[jsonl-storage.test.ts:458](../../../../.repo/pi-1.0.4/packages/durable/test/jsonl-storage.test.ts#L458)、[jsonl-storage.test.ts:1139](../../../../.repo/pi-1.0.4/packages/durable/test/jsonl-storage.test.ts#L1139)、[harness-generation-recovery.test.ts:164](../../../../.repo/pi-1.0.4/packages/durable/test/harness-generation-recovery.test.ts#L164)、[harness-tools-recovery.test.ts:165](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tools-recovery.test.ts#L165)、[storage-conformance.ts:1](../../../../.repo/pi-1.0.4/packages/durable/src/testing/storage-conformance.ts#L1)。

建议继续验证的问题：

1. 在目标文件系统和掉电模型下，JSONL append/flush/reclaim与SQLite WAL NORMAL分别提供何种真实耐久性？
2. 大量历史entry、长delta尾、多conversation及大量ready任务时，恢复时间、内存和调度并发上限如何变化？
3. 宿主热替换registry与持久化迁移并发时，兼容版本的定义责任是否被应用清晰履行？
4. beforeRequest临时改变工具声明与已持久化offered集合发生差异时，应用是否有明确约束？
5. 任意任务等待图中的环、忽略取消的handler和极慢观察者，是否有宿主级诊断与处置？
6. 声明safe的工具是否真的在当前环境下可重放，其远端幂等键和side effect收据保存在哪里？
7. 实验性消费入口如何被构建和交付，哪些运行时模块只在源码开发条件下可解析？

推荐阅读顺序：types/documents → session/transaction → 一个Storage后端 → scheduler → generation/tool → submissions/inbox/context → view/events → 对应恢复测试 → experimental真实装配。
这条顺序先确定数据与提交权威，再理解模型循环，最后评估UI事件和恢复承诺。

本报告只新增研究文档，测试影响：`none`；未修改源快照、测试、manifest、缓存或运行数据。

## 15. 设计文档、规划与时效核查（2026-10-07；历史审读继承与本次复核）

### 15.1 文档清单、阅读范围与证据层级

前文主要从代码解释现状，未系统呈现 Pico5 的设计动机、分阶段替换和被否决实验。本节补齐这部分。
核查只比较本地 1.0.4 文档与源码/测试；没有访问文档链接、检出旧提交、重跑 benchmark 或执行项目测试。
“已实现”表示快照有对应机制与测试预期；文档说曾通过、曾测得，不等于本次执行确认。

| 文档 | 阅读范围 | 原本承担的角色 |
|---|---|---|
| [README.md](../../../../.repo/pi-1.0.4/packages/durable/README.md) | 本次全文复核 1–613 行 | 现行公共使用说明，明确标为实验性包；末尾链接规范、实现计划和 Chord 指南 |
| [CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md) | 本次全文复核 1–63 行 | 初始发行、provider UUID、1.0.3/1.0.4 环境破坏性变化及修复记录；没有独立未发布路线图 |
| [docs/spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md) | 本次重点重读 1–80、289–313、2655–2880、3020–3088、3270–3284、3470–3503、4630–4692；其余原机制解释继承前版，不冒称当前全文重读 | 自称 normative 的 Pico5 设计规范：records、documents、tasks、Harness、storage、观察和非目标 |
| [docs/pico-v5-handoff.md:1](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L1) | 前版全文阅读按相同摘要继承；本次重读 681–917 的最终计划/修订 | 分阶段实施计划与后补修订；顶部称 1–23 已实现，正文同时保留旧阶段说明 |
| [docs/pico-v5-chord-usage.md:1](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L1) | 前版全文阅读按相同摘要继承；本次重读 334–387 的提交与生命周期 | DocumentState、Facet/Service、remote binding、watch 生命周期的具体集成指南 |
| [docs/chord-delta-findings.md:1](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L1) | 前版全文阅读按相同摘要继承；本次重读 308–358 的历史证据/未来实验要求 | 被移除 graph tracker 实验与旧 tree/path tracker 的测量、正确性问题及后续调查要求 |

包内 Markdown 枚举只有以上六份。额外交叉阅读 Agent 1.0.0 删除记录、Chord 当前 delta 契约及相关实现/测试；没有把其他包整份设计再当成本包路线图。
这些文档不能按一个统一时态阅读：README 大体是“现在怎么用”，spec 是“应满足什么”，handoff 是“曾分几步到达”，findings 是“某轮实验为什么被拒绝”。
判定时应落到具体段落，并优先让当前源码、公共类型和测试解释现实行为。

### 15.2 spec：核心设计已落地，局部状态句过期

规范的核心动机是让存储提交成为可见状态的权威：文档 draft 可以修改，prepare 先构造候选版本，Storage 接受后才 adopt/publish；可重开的任务以持久 checkpoint 重新进入阶段，不保存 JS 调用栈。
records、conversation branch、doc incarnation、requestId admission、任务所有权和 Chord observation 因此共享提交线。
这一体系在 Session、Transaction、Scheduler 与三类后端中已有实现，不是只有规范草图。
依据：[spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[session.ts:404](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L404)、[transaction.ts:697](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L697)。

| 规范条目 | 现状与时效判断 | 依据 |
|---|---|---|
| §2.2 “Pico5 is not implemented yet”，并要求实施 package 24 时采用该 facade | **状态句已过时**。同仓 handoff 顶部已称 1–23 实现，Harness、Conversation、Submission、document API 及当前 1.0.4 包真实存在。后面的 API 形状仍有现行契约价值，不能整章作废 | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[handoff.md:10](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L10)、[harness.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/harness.ts)、[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md) |
| 开头以 PR #9548 待合入来说明 SystemMessage | **时间条件过时，设计已落实**。ai 0.86.0 记录已发布 transcript-backed system/tool changes，当前 Message union 有 SystemMessage；后文用这个 PR 名称指代消息格式仍可读作历史来源 | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[ai/CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/ai/CHANGELOG.md)、[ai/types.ts](../../../../.repo/pi-1.0.4/packages/ai/src/types.ts) |
| immutable roots、revocable drafts、prepare/adopt 和显式 doc tokens | **已实现的核心约束**。读取返回受信任 immutable revision；不是自动冻结每个对象或保存全局业务 schema registry | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[session.ts:162](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L162)、[transaction.ts:662](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L662) |
| owner/wait/completing 和逐层 abort | **已实现**。父任务可在完成请求后等待后代收敛，scheduler 负责持久状态而非任意 Promise 图；恢复是阶段重入 | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[scheduler.ts:980](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L980)、[harness-structured.test.ts:202](../../../../.repo/pi-1.0.4/packages/durable/test/harness-structured.test.ts#L202) |
| semantic events 与 task graph | **已实现的派生观察面**。events 标 experimental；由提交生成 snapshot/delta，不是 Session 内第二套持久事件 journal；task graph 展示已提交关系，调度诊断仍在 inspect | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[events.ts:140](../../../../.repo/pi-1.0.4/packages/durable/src/harness/events.ts#L140)、[task-graph.ts:62](../../../../.repo/pi-1.0.4/packages/durable/src/harness/task-graph.ts#L62) |
| §11.3 JSONL fsync | **规范比 README 摘要精确，仍有效**。默认 false；true 时先 flush sidecars 再 append main marker，普通提交不 flush main；破坏性回收前才 flush main，失败推迟回收 | [spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[storage.ts:256](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L256)、[storage.ts:294](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L294)、[storage.ts:509](../../../../.repo/pi-1.0.4/packages/durable/src/storage/jsonl/storage.ts#L509) |

从 1.0.2 继承的 provider identity 规范与代码接线一致：§2 的内置文档定义、generation、compaction 和 view 片段均描述了持久 UUID。它是已实现契约，应从“未来功能”中排除；开头仍保留的 PR 待合入与“Pico5 尚未实现”时间句没有随这次编辑清理。不能把整份 spec 判定为未落地，也不能把这些旧状态句当作 1.0.4 的实施缺口。
依据：[spec.md:737](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L737)、[spec.md:3488](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L3488)、[spec.md:3772](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L3772)、[spec.md:4005](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L4005)、[spec.md:291](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L291)。

§12 的 footguns 是当前边界说明：任意代码忽略取消会拖住 close、低层 head 改写可能使已挂载 view 无法即时补回历史、checkpoint 策略由 doc 定义承担等。
§13 则明确排除 whole-Session DOM、session-scoped rewindable docs、自动 checkpoint heuristic、自动第三方 view 挂载、CRDT/offline multiwriter、任意 Chord op 的 SQL 翻译、JSONL 全局压缩/自动损坏修复、旧 Pico compatibility 和进程内强停非合作代码。
这些不能因“目前没有”而全部列成未完成功能；有些以后可能改变，但快照没有排期或承诺。
依据：[spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)。

### 15.3 handoff：实施包次序保留了被后续阶段替换的方案

这里的 package 1–23 是**工作阶段编号**，不是仓库的 npm packages 个数。Pico5 是该套 durable 设计的阶段名；当前公共 npm 包和路径叫 `pi-durable` / `packages/durable`，不存在一个必须另行发布的 Pico5 npm 包。
handoff 顶部说 pico3 仍保留，但 Agent 1.0.0 明确记录删除 pico3 与 experimental subpath，当前文件枚举也未发现旧 pico3 实现目录。
因此该状态项已过时；文中从 `packages/agent/src/harness/...` 搬实现的路径应视为历史来源，现行接线应定位 durable。
依据：[handoff.md:1](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L1)、[handoff.md:54](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L54)、[agent/CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md)。

| 阶段与原计划 | 设计动机及当前落点 | 时效判断 |
|---|---|---|
| 1–2：numeric IDs、records/indexed scans、MemoryStorage、doc records | 建立原子提交和 detached storage ownership，避免所有权靠序列化“碰巧成立”；Session/Transaction 与 Memory backend 已存在 | 已实现基础，不是未来内存后端计划。[handoff.md:16](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L16) |
| 3–5：SQLite、JSONL sidecars/marker、reclaim | 用相同 storage contract 实现查询、重开、确认边界和可回收最新文档；现有三后端及 conformance/reopen/failure tests 对应 | 主体已落地；真实掉电与文件系统条件仍需环境验证，不能仅凭“Done”保证物理耐久性。[handoff.md:45](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L45)、[jsonl-storage.test.ts:533](../../../../.repo/pi-1.0.4/packages/durable/test/jsonl-storage.test.ts#L533) |
| 6–7：tracker、typed definitions、Tx doc access | 为失败前不推进权威版本、异步 draft 和输入类型约束服务；现在用 immutable revision/prepare/adopt，typed token 不需要中央注册 | 已实现且是当前架构核心；不是 findings 里旧 target/flush tracker 的原封延续。[handoff.md:107](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L107)、[transaction.ts:672](../../../../.repo/pi-1.0.4/packages/durable/src/session/transaction.ts#L672) |
| 8–9：checkpoint/migration/fork | 把版本变换、历史 asOf 和 fork 拷贝协议明确化；未注册类型可按存储记录复制，rewindable 与 latest 有不同策略 | 已实现；自定义 migration/checkpoint 正确性仍由定义承担。[handoff.md:189](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L189)、[session-checkpoints-migrations.test.ts:648](../../../../.repo/pi-1.0.4/packages/durable/test/session-checkpoints-migrations.test.ts#L648) |
| 10–12：Chord structural ops、documentState、bounded watches | 大数组结构变更不携带未改几何 payload；直接采用已提交 doc 帧，慢消费者超过 100 帧改 root reset | 对应实现和测试已在；10 在状态栏说明由 Chord 已满足，不能重复列为 durable 待开发算法。[handoff.md:213](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L213)、[session.ts:212](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L212)、[session-watches.test.ts:159](../../../../.repo/pi-1.0.4/packages/durable/test/session-watches.test.ts#L159) |
| 13–15：openable Harness、task runtime、首个无工具聊天 | 先连通恢复/registry/任务和一次模型请求；阶段 15 的 busy/toolUse 暂不支持是纵向里程碑限制 | 历史阶段已推进，不能当成 1.0.4 仍不会工具调用或 busy queue。[handoff.md:288](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L288)、[handoff.md:386](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L386)、[generation.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts) |
| 16：工具 intent/effect/outcome、输出 retention、pi.post-tools/after 串联 | 当时通过 post-tools 任务处理一轮收尾，工具输出需要有界并持久；工具和输出能力保留，但任务编排后被 19 改写 | **一部分已实现、一部分被明确替代**。不能在当前内置任务列表中继续加 pi.post-tools。[handoff.md:441](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L441)、[registry.ts:10](../../../../.repo/pi-1.0.4/packages/durable/src/harness/registry.ts#L10) |
| 17–18：view/inbox/usage/events、ownership/subagents | 前者从 commits 投影产品状态；后者以任务拥有 conversation 构建前台/后台协作 | 已实现主要机制与示例；未选择内置统一 subagent tool/supervisor，示例 22/23 是应用组合，不是默认装载。[handoff.md:528](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L528)、[handoff.md:577](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L577) |
| 19：structured concurrency 替换 after、pi.post-tools | generation 拥有 tool children，等待/收尾进入其 phase；子任务/拥有会话阻挡终结，以 owner 边维持收敛 | **明确取代旧方案且已实现**；文档要求删除而不保留兼容，不存在两套同等现行 API。[handoff.md:606](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L606)、[scheduler.ts:1003](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L1003) |
| 20：compaction/overflow | summary task 独立持久化、range/tail 明确、boundary 放置摘要，overflow 在重试前触发 blocking compaction | 主体已实现；“重发精确最后请求 + toolChoice none”的缓存友好摘要明确写 Not in v1，只是备选方向。[handoff.md:681](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L681)、[handoff.md:729](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L729) |
| 21：named extensions + per-conversation Agent | 用 pi.agent/configure 替换 pi.conversation.config/getters，live HarnessSettings 放运行策略，env 按调用构造；移除旧 keyed registry/batch/wrapper scopes | **替代设计已实现**；旧 13–16 示例不能直接当当前 API。CodingTools 只有 read/write/edit/bash，grep/find/ls 未移植是该阶段明确范围。[handoff.md:825](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L825)、[agent.ts:41](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L41)、[agent.ts:72](../../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L72) |
| 22–23：lifecycle audit、规范示例、task graph | 收紧 shutdown/late invocation/迁移观察；让任务所有权成为可观察图 | 文末明确 Done/Implemented，且有对应代码测试；详见下节，不能把上面的 Bugs 列表直接复制为现存缺陷。[handoff.md:850](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L850)、[handoff.md:897](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L897)、[task-graph.ts:71](../../../../.repo/pi-1.0.4/packages/durable/src/harness/task-graph.ts#L71) |

另有一个容易读错的明确冲突：13–20 前的 cross-cutting 文字要求在 Storage admission 前准备所有 ConversationView，且禁止订阅已提交 publication 重建；后面的阶段 17 改为 `subscribeCommits()`，当前 spec §9.3 和 `view.ts` 也从 durable publications 推进 mount。
因此应判定**较早约束已被后续设计替换，但旧文字未清理**；不能以这句孤立旧要求指控当前实现违反现行 view 架构，也不能把两种时序混成同一条保证。
依据：[handoff.md:270](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L270)、[handoff.md:553](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L553)、[spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[view.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/view.ts)。

handoff 的 1 GiB cat 约 0.4 s 是 throughput **目标**，不是本次或当前快照已经达到的结果；同段 TypeBox 的 RSS 数字是作者环境中的测量说明。
benchmark 实现存在、文档列有要采集的 metrics，都不能替代实际跑出的结果。没有因为目标文字存在便认定它“已满足”，也没有因本次未测便认定“未实现”。
依据：[handoff.md:502](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L502)、[handoff.md:519](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L519)。

### 15.4 阶段 22 的 Bugs 已怎样处理

| 当时发现 | 当前静态证据 | 结论 |
|---|---|---|
| invocation 结束后 now/report 仍可调用 | 两方法都检查 invocation.ended；测试 expect 两者抛 invocation has ended | **已修正对应缺口**。[scheduler.ts:1172](../../../../.repo/pi-1.0.4/packages/durable/src/harness/scheduler.ts#L1172)、[harness-tasks.test.ts:524](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tasks.test.ts#L524) |
| migration-only base 没有 ops，旧 shape observer 一直不更新 | `observedOperations` 跨版本发 root replacement；测试明确断言 migration-only base 发布给旧 watch | **已修正**；新 shape observer 可忽略同版本空 ops，不能误判再次漏发。[session.ts:553](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L553)、[session-checkpoints-migrations.test.ts:398](../../../../.repo/pi-1.0.4/packages/durable/test/session-checkpoints-migrations.test.ts#L398) |
| open 失败后用已取消 caller context close，掩盖原错误 | catch 路径 close 使用 withoutAbortSignal；测试覆盖 close 本身失败仍重抛 open 原因 | **已修正**。[harness.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/harness.ts)、[harness-lifecycle.test.ts:85](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L85) |
| close/watch 结束边界、取消 close 后是否真正 shutdown、non-cooperative handler 是否 join | seal 立即停止观察；共用 shutdown，取消只取消该等待；handler/tool/hook 忽略 signal 时测试要求 storage 保持打开直到返回 | **契约已细化并有针对性测试**，不支持强停非合作代码。[harness-lifecycle.test.ts:127](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L127)、[harness-lifecycle.test.ts:230](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L230)、[harness-lifecycle.test.ts:275](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L275) |
| progress calls、compile examples、Chord detach/withdraw 需补覆盖 | lifecycle 测试分别列只读不调度和 progress 调度；spec-usage、chord-guide 文件存在，指南测试包含 facet host 与 remote client teardown | **计划已形成实现/测试工件**，不是本次运行认证。[harness-lifecycle.test.ts:533](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L533)、[harness-lifecycle.test.ts:565](../../../../.repo/pi-1.0.4/packages/durable/test/harness-lifecycle.test.ts#L565)、[spec-usage.test.ts:1](../../../../.repo/pi-1.0.4/packages/durable/test/spec-usage.test.ts#L1)、[chord-guide.test.ts:269](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L269) |

“Done”是 handoff 自身的后补状态，上表才是本次的交叉依据。它不证明每条历史验收命令都能在任意平台通过，也不证明所有可能的生命周期 race 已穷尽。

### 15.5 Chord usage：标题旧，主要内容仍是现行集成指南

该文档开头把示例与 `test/chord-guide.test.ts` 绑定，并要求同步。Session 是提交权威；DocumentState 是已提交 doc 的 hydrated readonly adapter；ReplicatedState 的 stream sequence 不是 Session commit number。
这些术语仍与当前实现匹配，不能因为文件名含 pico-v5 就列为废弃提案。
`documentState` 在提交线上获取当前 snapshot 并注册后续 exact frames；外部 service 不需要复制整个 Session，也不赋予 replica 提交权。
依据：[chord-usage.md:1](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L1)、[chord-usage.md:28](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L28)、[session.ts:212](../../../../.repo/pi-1.0.4/packages/durable/src/session/session.ts#L212)。

| 指南场景与原意 | 当前落实 | 阅读时必须保留的边界 |
|---|---|---|
| Session-wide canvas：自定义 doc 经 facet/service 暴露给多个客户端 | 测试确实创建 CanvasDoc、绑定 service、通过 remote binding attach，验证 host dispose 撤下服务 | canvas 是示例自定义状态，不是 durable 默认 built-in；transport 由 host 提供，不默认建 socket/鉴权协议。[chord-usage.md:54](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L54)、[chord-guide.test.ts:269](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L269) |
| conversation diff family：按会话与 key 暴露 review | keyed consumer 测试存在；fork policy current/asOf 定义文档复制时点 | service 的复合 key 不是 DocumentId；latest 文档不能获得 rewindable 历史。[chord-usage.md:185](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L185)、[chord-guide.test.ts:327](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L327) |
| task-scoped output：生产者有界输出与 watch | 测试运行直到 producer retire；超过 backlog 上限 reset，退役以 null 可见 | watch.stop 不 join 任意 listener；task 终结后不能继续创建 task doc，需把永久结果另行归档。[chord-usage.md:264](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L264)、[chord-guide.test.ts:356](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L356) |
| built-in conversation view 与 commit delivery | view 挂载内置几类文档，由提交 publication 推进 | 不自动挂第三方 docs；不能把 network replica hydration、Session commit、用户回调开始看成同一时刻。[chord-usage.md:334](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L334)、[view.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/view.ts) |

指南的生命周期说明仍有阅读价值：先 detach clients、dispose facet host 及其 owned states，再 close Harness；`env.provide` 在同步 setup 注册，不能等到异步 onActivate 才提供。
文档示例测试说明接线具备可执行形式；本次未编译或运行，不能宣称文档所有示例已经在当前机器上通过。
依据：[chord-usage.md:172](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L172)、[chord-usage.md:368](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L368)。

### 15.6 delta findings：哪些结论仍有效，哪些不能当现状

调查原意是解决 large drawing 中交换、移动、持有引用与复制效率的矛盾。工作负载约 2 万 strokes × 100 points，总计约 204 万对象；作者区分 serialized size、raw heap、post-GC retained heap、采样峰值和 RSS，并要求验证身份/别名，不只 deep equality。
这个方法论仍有效，但文档自己标明其多数表格是历史测量，并且早于最终 spread-first clone 改动。
依据：[findings.md:21](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L21)、[findings.md:51](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L51)。

**被否决的 graph 方案。** 它为对象分配 ID、通过 reference writes 表达图结构，能保留 alias/cycle 和交换身份，但 import、hydration、producer metadata 与 full traversal 代价过高，所以实验实现被移除，未导出、未接入 Chord services。
文档反对的是测到的这一实现，而非数学上证明所有 graph replication 不可能。其 graph hash 只标识一次未提交实验，不意味着当前 Git 快照能恢复该实现。
因此 graph 部分是**历史决策证据**，不能列为仍待接入的半成品模块。
依据：[findings.md:3](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L3)、[findings.md:122](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L122)、[findings.md:331](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L331)。

| findings 中的发现/主张 | 当前对照 | 时效判定 |
|---|---|---|
| path operation log 的 swap 很小，但 flush 仍可能遍历几何；大量 edit 可折成大 root replacement | 当前仍是 path ops，并有结构数组 `m`、region splice 和 root fallback；但公共协议已从 mutable target/flush 改为 immutable revision + beginChange/prepare/adopt | 设计问题仍相关，**旧算法时序/数字不能直接套用**。当前成本需重测。[findings.md:54](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L54)、[delta/README.md:124](../../../../.repo/pi-1.0.4/packages/chord/src/delta/README.md#L124)、[tracker.ts:1652](../../../../.repo/pi-1.0.4/packages/chord/src/delta/tracker.ts#L1652) |
| alias/held refs/reindex 可使 producer 和 replica 分歧，copy-on-assignment 本身改变 alias 语义 | 当前 root 必须 alias-free、acyclic strict JSON；不遍历验证 trusted roots，违规仍可能分歧。draft placements 深拷贝并验证，多处放同一值成为独立容器；held handles 跟随数组重排 | **换成明确树所有权契约，而非实现任意 JS graph alias 语义**。不能写“所有 alias 缺陷修好了”，也不能拿违规 root 例子宣称实现违反自己的契约。[findings.md:95](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L95)、[delta/README.md:96](../../../../.repo/pi-1.0.4/packages/chord/src/delta/README.md#L96)、[delta-clone.test.ts:41](../../../../.repo/pi-1.0.4/packages/chord/test/delta-clone.test.ts#L41) |
| full traversal 暴露数 GiB retained heap；weak proxy cache 把该轮保留量降到约 203.54 MiB，却未解决约 2491.78 MiB 的冷读后采样 heap和慢冷读 | 当前采用 transaction overlay、WeakRef contexts、prepare/abort/adopt 清理；retention 测试覆盖 14 个情景，已不是文中描述的旧 placement/finalizer cache 结构 | **历史补丁效果与当前性能未知分开**。不能说旧 retained 数字证明当前快照仍占几 GiB，也不能说新实现已达到满意上限。[findings.md:208](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L208)、[findings.md:226](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L226)、[tracker.ts:214](../../../../.repo/pi-1.0.4/packages/chord/src/delta/tracker.ts#L214)、[retention.test.ts:5](../../../../.repo/pi-1.0.4/packages/chord/test/delta-tracker/retention.test.ts#L5) |
| spread-first clone 减小对象布局与拷贝成本，保留 null prototype | 当前 `track(root)` O(1) 保留原 root 身份、不完整 clone；placement `copyJson` 用 Object.create/own data properties，null prototype 仍被保留 | **clone 实现描述已不对应当前路径**，null-prototype 语义仍在。旧 93.924 ms/139.466 MiB 数据只属于旧 clone 比较。[findings.md:274](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L274)、[json.ts:16](../../../../.repo/pi-1.0.4/packages/chord/src/json.ts#L16)、[delta-clone.test.ts:5](../../../../.repo/pi-1.0.4/packages/chord/test/delta-clone.test.ts#L5)、[delta-clone.test.ts:17](../../../../.repo/pi-1.0.4/packages/chord/test/delta-clone.test.ts#L17) |
| 保留 benchmark 供重现，未来尝试必须覆盖读放大/复制/身份/回收 | 当前 benchmark 确有 raw/minimal proxy/current delta 三模式、独立进程、source hashes；delta 已用 beginChange/prepare/adopt | **重现实验方法仍有价值，但不是原 graph/baseline 全矩阵重现**；临时结果路径不是随快照附带的数据集。[findings.md:308](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L308)、[delta-traversal.bench.ts:53](../../../../.repo/pi-1.0.4/packages/chord/test/delta-traversal.bench.ts#L53)、[delta-traversal.bench.ts:94](../../../../.repo/pi-1.0.4/packages/chord/test/delta-traversal.bench.ts#L94) |

findings 最后的 “Requirements for a future attempt” 是下一次研究应满足的证据要求：完整 producer reads、初始化/编辑/序列化/回收分阶段、identity/reindex、import/hydration、新对象替换和多种内存指标。
它没有选定另一个必然要实现的 graph API，也没有排期；其“没有一种测得设计同时达到所有目标”应限定在该轮调查，不能跨过后来的 immutable tracker 改造无限延伸。
依据：[findings.md:347](../../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L347)。

### 15.7 README 的现行说明、局部夸大与真正剩余方向

README 对 extensions、pi.agent、settings、env、reload、compaction、task graph 已使用较新的最终表面，并补充 progress、argv 和 reader；它应作为实际入口，而不是依旧按 handoff 13–16 的旧 registry 名称接线。
“Reading images is not supported yet” 与 read 工具的 unsupported_image 分支一致，这是**明确尚未支持**，但没有版本/日期承诺；grep/find/ls 未移植和 cache-friendly compaction Not in v1 也不等于确定下一步。
依据：[README.md](../../../../.repo/pi-1.0.4/packages/durable/README.md)、[read.ts:118](../../../../.repo/pi-1.0.4/packages/durable/src/tools/read.ts#L118)、[handoff.md:729](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L729)。

README:303 把“partial/tool output at most every 100 ms”进一步解释为“崩溃最多丢失该窗口”，这个上限**不成立**。
generation 是 100 ms 节流并且只有一个 commit 在途，存储/调度可以更慢；tool output 的 Progress 更明确按上次写入量以 100 KiB/s 计费，等待至少配置的 outputIntervalMs（默认 100 ms），大输出等待可更久。
这是频率约束和减少 I/O 的取舍，不能提升为 100 ms recovery-point guarantee；JSONL 与 SQLite 还各有另外的介质耐久性条件。
依据：[README.md](../../../../.repo/pi-1.0.4/packages/durable/README.md)、[spec.md](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[generation.ts](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)、[output.ts:268](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L268)、[output.ts:319](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L319)。

README 开头“reopening picks work up where it stopped”也应按任务 checkpoint 理解：恢复会把未完成 assistant partial 归为 aborted 后重新请求；工具是否可重跑要求已保存 intent 和当前工具都声明 safe。
requestId 仅在 conversation 内去重 admission，不能把它误读为远程副作用 exactly-once。已有工具 recovery 测试覆盖 safe 组合，这些是当前实现的具体边界，不是等待未来补完的无限承诺。
依据：[README.md](../../../../.repo/pi-1.0.4/packages/durable/README.md)、[tool.ts:95](../../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts#L95)、[submissions.ts:155](../../../../.repo/pi-1.0.4/packages/durable/src/harness/submissions.ts#L155)、[harness-tools-recovery.test.ts:136](../../../../.repo/pi-1.0.4/packages/durable/test/harness-tools-recovery.test.ts#L136)。

最后，package 的能力存在不代表已替代默认产品运行时。快照内 coding-agent 的 durable 集成位于 experimental 路径；指南的自定义 canvas/diff/service、handoff 的背景 subagent 组合则属于测试与示例。
因此不能由“计划 1–23 已实现”直接推导默认 CLI 已全面切换、全部扩展和 transport 已成为默认产品功能。
依据：[experimental/runtime.ts:125](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L125)、[chord-guide.test.ts:1](../../../../.repo/pi-1.0.4/packages/durable/test/chord-guide.test.ts#L1)。

建议阅读顺序：README 确认现行 API → spec 的 invariants/ownership/storage/non-goals → handoff 19–23 掌握最终替换 → 回看 1–18 的历史原因 → Chord usage 看实际生命周期 → findings 看研究方法与被否决方案。
本轮保留的验证空白是性能目标/历史数字未重测、真实进程崩溃及掉电未实验、Node/Bun/SQLite driver 未运行、文档示例只静态对照、历史未提交 graph 实验无法从本快照复原。
以上不是新的产品路线图，也没有把明确非目标改写成缺陷；测试影响仍为 `none`。

## 16. 1.0.2 → 1.0.4：变化、文档修正与验证边界

本包163个文件中，新增8、修改26、相同129，无删除。新增环境解码、行扫描、Node watcher、环境 conformance 及4个测试文件；改动集中于环境接口、read/bash/output、progress配置和对应文档/测试。Session、Transaction、Scheduler、存储后端及三份历史设计文档内容未变，原有深度分析按旧清单继承。依据：[当前清单](../source-manifest.json)、[历史证据](../evidence/pi-1.0.2/README.md)。

| 方向 | 当前兑现与时效 |
|---|---|
| 可移植环境 | 必需 reader/scan/watch、argv、output info 都已在 Node 和新 Remote adapter 接线；这是适配器破坏性契约升级，不只是新增可选 helper |
| 远程输出减量 | Harness window → bash → env → skipped → OutputBuffer 完整接线；无 window 的消费者仍收到全部 decoded 输出，spill保持完整原始输出 |
| 进度策略 | 新 settings.progress 已实现，默认不变；README:303 仍把默认节流夸成崩溃丢失上限，已在正文限定 |
| 有界 read | 环境内一次扫描和区间传输已实现；1.0.4 接受增长文件。spec:3081“文件改变就重读一次”局部陈旧 |
| PowerShell | 工厂、argv调用和测试存在；README/计划只列CodingTools四工具仍准确，但不代表整个/tools只导出四种工具 |
| spec ToolExecutionApi | prose:2823 已描述window/skip，代码示例:2698仍只有单参数output且漏outputWindow；应以当前types/tool实现为准 |
| 历史 Pico 计划 | 阶段19–23的替代/完成判断保持；cache-friendly summary Not in v1、未移植grep/find/ls、图片read未支持仍保持；新env不等于这些方向完成 |
| 文档身份 | README/CHANGELOG重读全文，spec按明确范围复核，handoff/Chord usage/findings不变部分继承；未将枚举或哈希相等写成新全文阅读 |

依据：[spec.md:2698](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L2698)、[spec.md:2823](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L2823)、[spec.md:3072](../../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L3072)、[pico-v5-handoff.md:729](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L729)、[pico-v5-handoff.md:825](../../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L825)。

新增/变化测试静态预期包括合法skip与完整输出等价、snapshot时机不变性、read差分/BOM/随机字节/PNG远处头、日志增长、PowerShell fallback、progress undefined默认，以及Node/Remote共享env conformance的watch行为。没有运行这些测试、真实进程、原生watch、SSH或模型服务，不能给出跨OS通过结论。测试影响：`none`。
依据：[harness-output-skip.test.ts:121](../../../../.repo/pi-1.0.4/packages/durable/test/harness-output-skip.test.ts#L121)、[tools-read-differential.test.ts:156](../../../../.repo/pi-1.0.4/packages/durable/test/tools-read-differential.test.ts#L156)、[tools.test.ts:587](../../../../.repo/pi-1.0.4/packages/durable/test/tools.test.ts#L587)、[env-conformance.ts:259](../../../../.repo/pi-1.0.4/packages/durable/src/testing/env-conformance.ts#L259)。

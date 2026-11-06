# 确定遥测生命周期与失败语义
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [确定 telemetry 的归属与依赖方向](05-telemetry-boundary.md)

## Question

在已确认的中立遥测边界下，怎样把 Pi 的 span/callback/observer 契约映射为 Rust 原生生命周期？尤其是开始与首次 poll、逻辑请求和 stream handle 的寿命、取消/提前丢弃、no-op 与 ambient tracing、业务失败与遥测后端失败的关系。哪些行为按现有对齐原则保留，哪些缺少跨语言对应需要单独选择并记录为差异？

## Comments

2026-11-06，已确认 telemetry 拆分边界后继续处理本票。以下为源码核验与讨论历史；已确认的政策见 Answer。

### 可直接由已有对齐政策确定的部分

- AI schema、事件、状态、计量规则逐入口保留；aborted 不自动转成 span error，pending/deferred poll 不累计 token/cost，字符计量遵循 UTF-16。
- 业务结果与事件只有原生产/转发路径驱动，遥测不另建竞争消费者。span 生命周期覆盖 setup 和请求完成过程，返回 stream 或结束一个 poll 不等于请求结束。
- no-op 继续执行业务，只关闭本中立接口的记录，不自动静音宿主其他 tracing。当前显式 noop carrier 的全作用域静音需迁移到有名称的后端策略或差异，不能混入中立默认。
- 保留原 schema 描述数据；typed starter 不被替换成新增的强制运行时验证器。adapter 不因“遥测应尽力而为”的一般偏好自动吞掉所有错误。
- 后端上下文 handle 的 clone/drop、Rust future 的 poll/drop、逻辑请求终态和 exporter flush 是四种不同生命周期。flush/shutdown 属宿主，不能成为每次模型调用的隐式步骤。

### 原版与当前 Rust 的真实差异

路径以仓库为根；S 表示 `.pi/pi-1.5.0`。本轮只读源码，未运行行为实验。

| 观察场景 | Pi 源码事实 | 当前 Rust 源码事实 |
| --- | --- | --- |
| 创建流但未迭代 | S/packages/ai/src/models.ts:907–917 建 observer；api/lazy.ts:74–85 立即启动 setup→forward promise 链 | crates/opi-ai/src/models.rs:4957–4966,5020–5038：ChatCall 首次 poll 才 materialize/auth；未 poll drop 不启动业务 |
| 不读取或外层 iterator return/break | api/lazy.ts:44–52 独立消费 source 并结算；utils/event-stream.ts:43–57,72–84 的外层 iterator 没有 abort/finally，停止迭代不取消 producer | ChatCall/ChatStream 结构析构释放 inner；openai_chat.rs:3168–3185、anthropic.rs:2101–2114 监听 receiver closed 并释放 work；自定义 provider 依自身实现 |
| Drop 与显式取消 | 外层 iterator return 不等价于传入 AbortSignal 的取消；正在 await 的 return 也不保证立即完成 | stream.rs:967–1014 被 poll 到的显式取消关闭 rx 并形成 Cancelled partial；普通 receiver drop 不直接 cancel token。Codex 是特例：openai_codex_responses.rs:863–873 在未交付 terminal 的 receiver closed 时 owner_cancel.cancel 并 await work |
| 未观察到请求结束 | 不合作 source 不保证最终结束；不能因外层不消费而提前制造成功终态 | models.rs:3604–3613 的 AttemptStream::Drop 将 dispatched 未闭合 outcome 标 unknown 并 release acquisition，不是统一 cancel |

静态结论：当前 Rust 不能被概括为“Drop 一律只停消费”，也不能说“Drop 一律调用 cancel token”。为中立遥测定寿命前，必须由统一请求层明确每种操作的承诺。

### 遥测故障按阶段记录

上游 `packages/telemetry/README.md:125` 不承诺抑制 adapter/backend failure。必须保留以下基准异常分支，不能全局假定遥测不影响业务：

- instrument factory 在 observer 构建期间同步调用，throw 可在 Models.stream 进入 lazyStream 前失败（ai/src/telemetry.ts:128–153；telemetry/src/metrics.ts:141–143）。
- `void start(...)` 没有 rejection handler（ai/src/telemetry.ts:152–153）；rejected promise 不自动等于业务 error。
- onEvent 位于 push 之后（api/lazy.ts:45–48）。非终态 observer throw 可进入 catch 并产生 synthetic error；原 terminal 已 push 的情况下原结果已锁定，catch 的 error push 被忽略，业务结果与 observer 结果可能不同。
- onResult 位于 end 之后，observedResult 在回调前置位（api/lazy.ts:51–52,68–82）。回调 throw 不会被重试；若在 AI onResult 的 finish 前失败（telemetry.ts:181–207），用于 span settlement 的 promise 可能保持 pending。
- catch 内 observer 再 throw 没有第二层 catch，可形成未处理 rejection；内存 adapter 的 finally 只覆盖 callback 返回的 promise（telemetry/src/memory.ts:232–245），不能自动兜住后续独立 observer。

这些是已定位的异常路径与静态推导，不是声称已实测泄漏或证明全部源缺陷。后续 TS 录制需给出有界观察与宿主错误通道；Rust 的 typed adapter failure/panic 映射单列。若要统一改成故障隔离，需要明确有意差异，不在普通重构中自动批准。

### 候选所有权与验收

建议逻辑请求的执行者拥有 span 结算权；用户可持有的 context/span 只提供父关系和记录能力，不因任意 clone/drop 决定完成。普通业务终态按原规则结算一次，保留的 span handle 在结束后记录无效。具体 Rust 类型名和签名待工程结构决策，当前不为了画图增加多个公共对象。

最低契约样本：创建后未 poll；setup 失败；首事件和正常终态；显式取消；首 chunk 后停止消费/丢弃；完成后迟到记录；保留多个上下文引用；各阶段 adapter/observer 故障；no-op 下业务结果及无关宿主日志；冻结 dispatch 的跨 poll/生产者传播。轻量记录器与 tracing 适配共享中立契约，适配器专属测试只验证后端路由。

### 用户已确认的选择（保留原讨论）

Rust 对一个已经启动但尚未完成的调用，销毁最后一个用户侧消费句柄应当：

- 继续执行到请求本身结束，span 跟随请求结算；取消由显式请求操作触发。推荐此方向，接近 Pi 的停止迭代行为；需要独立执行者持有资源并按终态清理，不能因句柄消失丢失 owner。
- 默认触发取消与清理，避免失去消费方后继续请求。该方向更接近当前部分 Rust producer，但若将其映射到 Pi 停止迭代，则是有意行为差异，不能计严格等价。

此问题只讨论“已经启动”的调用，不将未 poll 的惰性 future 和已启动请求混为一谈。启动时点本身已定位为现有差异；遵循当前对齐政策的默认方案应提供与 Pi eager stream 对应的启动动作，不能把现状第一 poll 才启动自动记为等价。

## Answer

2026-11-06 用户回复“同意，继续推进”，确认：已经启动的请求，其执行与消费分离；停止迭代或释放消费句柄只结束消费，不隐式取消请求。取消由显式操作、超时或其他已声明条件触发。

由该选择及已确认的对齐原则得到以下接口约束：

1. 请求执行者持有继续完成所需的资源及遥测结算责任；消费句柄销毁不丢失执行者，也不靠遥测 handle 的最后一次 Drop 推断成功、失败或取消。执行者是逻辑职责，不要求额外公开一个新类。
2. 正常业务终态、事件转发、result 与遥测结算按原入口的时序处理；返回 stream、结束一次 poll、停止读取都不是完成。正常路径中，结算后释放请求资源；允许保留的池/缓存另按其公开生命周期管理。
3. 不新增“停止消费即取消”的统一默认策略。当前内置 producer 对 receiver-closed 的行为须迁移；自定义 provider 必须接受同一调用层契约。申请取消与完成清理分开，不能用 cancel token 已触发冒充资源已释放。
4. 与 Pi eager stream 对应的启动动作必须在公开能力映射中明确。已有 ChatCall 首次 poll 才启动不能自动判等；具体 Rust 表面可原生设计，但不得悄悄改变已纳入比较的启动时点。其签名/组织由模块结构决策处理。
5. no-op 保留业务执行、关闭中立接口记录；AI 域状态和计量沿用原入口合同。已有 tracing carrier 的作用域静音不成为中立默认。
6. 不自动新增统一吞错、重新计量、或强制所有异常 span 成功结算的行为。已定位的 observer/adapter 异常与潜在 pending 路径保留为独立基准冲突，执行验收时分别观察业务结果、telemetry 状态和宿主错误通道；安全/资源改进需另行登记差异。这不是批准未知 panic 映射或宣布这些异常已验证。

本票解决生命周期的设计政策。具体差分样本与错误通道证据由[确定验收标准的分层与发布判定](10-acceptance-policy.md)承接，接口形状和迁移由[确定 opi-ai 的模块结构与公开入口](06-module-structure.md)承接。没有行为测试结果，也没有生产实现更改。

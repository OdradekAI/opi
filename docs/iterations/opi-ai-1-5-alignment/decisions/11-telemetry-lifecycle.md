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

### 2026-11-06 源码对照（历史）

路径以仓库为根，S 表示 `.pi/pi-1.5.0`。以下为当时的源码核对。
当前普通 Models 的启动方式见下方已确认的决策。

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

2026-11-06 的决策依据是静态核对。具体差分样本与错误通道证据由
[验收政策](10-acceptance-policy.md)承接，接口形状和迁移由
[模块结构与公开入口](06-module-structure.md)承接。

### 普通 Models 的 resolver 进入时点

2026-11-11 确认保留普通 Rust chat 调用的后台调度。构造 `ChatCall` 时调度
setup，由执行任务持有请求资源与结算职责。运行中的 Tokio 执行器可在首次消费前
推进鉴权，任务的实际进入时点由执行器决定。此设计落实执行与消费分离。
[用户确认记录](04-public-contract.md#公开-models-的准备失败错误通道) · [Rust 构造入口](../../../../crates/opi-ai/src/models.rs)

登记为**有意差异**的观察是 `models-auth-setup-failure` 在公开
`streamSimple` / `stream_simple` 返回时的 resolver 进入次数。Pi 显式 API key
且 provider 注册了 API key resolver 时，`packages/ai/src/auth/resolve.ts:56–70,193–204`
在首次读取 Store 前进入 resolver。冻结轨迹中，Pi 在调用返回时计数为 1。
Rust 的 current-thread Tokio 探针在返回时为 0，消费前到达认证屏障时为 1。
多线程执行器允许任务与调用方并发运行。两侧在该故障用例中均未派发 provider。
[#43 原始轨迹](../evidence/phase5-public-models-lifecycle-differential.json)

此差异的批准范围为上述显式 key 的 resolver 入口。Store、OAuth、raw
`lazyStream` 和 observer 异常路径仍按各自的可观察契约验收。
[实施方案](../spec.md#实施前置条件与重新讨论的触发点)

### 普通 Models 的成功结果与源流 EOF

2026-11-11，用户在收到保留 Rust 结果时序的建议后回复“继续推进”。据此在
`models-result-before-source-eof` 用例保留 `ChatCall::result()` 等待执行任务
收尾的语义。普通 `Models::stream_simple` 调用产出成功 `Done` 且源流尾部有限时，
调用方收到成功结果即表示该调用持有的源流已读到 EOF 并释放。`Done` 之后的尾部耗时
也计入 `result()` 的等待时间。
[#44 决定记录](../issues/44-observe-public-result-before-source-eof.md#accepted-decision) · [Rust 调用契约](../../../../crates/opi-ai/src/models.rs)

同一用例的 EOF 屏障内，Pi `Models.streamSimple` 已兑现公开结果，其转发任务
继续读取源流，Rust 结果仍为 `Pending`。放行后两侧到达源流 EOF，最终 `stop`
和 provider/model 一致，Rust 源流已 Drop。该成功结果时点登记为第 2 条时序要求的
具名**有意差异**。公开终态的交付时点见下项。
[#44 冻结轨迹](../evidence/phase5-result-before-source-eof-differential.json)

### 普通 Models 的公开终态与源流 EOF

2026-11-11，用户在收到保留 Rust 公开 `Done` 时点的建议后回复“继续推进”。据此在
`models-terminal-and-span-before-source-eof` 的普通成功用例保留源流到达 EOF 并
释放后再交付公开 `Done`。调用方收到终态时即可依赖本次调用持有的有限源流已收尾，
与上项 `result()` 的完成语义一致。源流尾部耗时计入等待公开 `Done` 的时间。
[#45 决定记录](../issues/45-observe-public-terminal-and-span-before-source-eof.md#accepted-decision) · [Rust 执行契约](../../../../crates/opi-ai/src/models.rs)

Pi 在源流 EOF 屏障内已交付 `Done`，Rust 的公开 `next()` 仍待定。此交付时点
登记为第 2 条时序要求的具名**有意差异**。同一用例中，两侧内存记录器的请求 span
在屏障内均开放，观测到源流 EOF 后均已带 `stop` 结算。结果属性写入故障由后续用例
观测。
[#45 冻结轨迹](../evidence/phase5-terminal-and-span-before-source-eof-differential.json) · [#46 结果遥测故障](../issues/46-observe-public-result-attribute-write-failure.md)

### 成功 chat 请求的最终遥测属性写入故障

2026-11-11，用户在收到“最终属性写入失败后尝试显式结束 span，保留成功业务结果和
故障诊断”的建议后回复“继续推进”。Rust 自动 `pi.ai.request` 观察器在 chat 以
`Stop`、`Length` 或 `ToolUse` 完成且最终属性写入返回 `TelemetryError` 时，先通过
宿主 tracing 报告写入失败，再消耗 span owner 尝试结束一次。该操作直接收尾，跳过
余下的结果遥测写入。后端若返回结束错误，宿主收到第二条警告。公开调用仍交付成功
终态与结果。手动 `AiRequestTelemetry::finish` 返回原始写入错误，由调用者选择后续操作。
[#47 实施与回归](../issues/47-finish-request-span-after-result-attribute-write-failure.md)

`models-result-attribute-write-failure` 重放中，两侧各在实际源流 EOF 后写入失败一次，
都交付成功 `Done` 和匹配的 `result()`。Pi 请求 span 在有界检查点仍开放，宿主监听到
零次 `unhandledRejection`。Rust 内存后端结束 span 一次，`stop` 属性缺失，宿主收到写入
失败警告。此场景的 span 状态和宿主诊断登记为第 6 条的具名**有意差异**。
[#47 冻结轨迹](../evidence/phase5-result-attribute-write-recovery-differential.json)

### 首个非终态事件的遥测写入故障

2026-11-11，用户在收到保留 Rust 对这一次遥测写入故障的隔离建议后回复“继续推进”。
据此保留公开 Models 的首个 `Start` 上单次 `pi.ai.first_chunk` 写入返回
`TelemetryError` 时的行为。Rust 将写入失败报告给宿主，关闭该请求的观察器，
显式尝试结束 span 一次，并继续读取 provider 源流。调用方仍按 provider 的实际
终态取得结果，宿主可通过 WARN 诊断遥测故障。
[#48 决定记录](../issues/48-observe-first-chunk-event-write-failure.md#accepted-decision)

`models-first-chunk-event-write-failure` 的终态屏障内，Pi 的受控源迭代器已完成
`return()` 和 `finally`，公开 `next()` 已得到带故障标记的 `error`，其
`result()` 兑现为同一错误消息。Rust 的 `next()` 仍待定，放行后源流产出成功终态、
到达 EOF 并释放，调用方获得 `Done` 和成功结果。屏障内两个请求 span 都已结算，
Pi 记录 `stop: error`，Rust 的 `stop` 属性缺失。该用例的业务结果、源流退出、
span 属性及宿主诊断登记为第 6 条的具名**有意差异**。
[#48 冻结轨迹](../evidence/phase5-first-chunk-event-write-failure-differential.json)

### 请求时长直方图的创建故障

2026-11-11，#49 的 `models-request-duration-histogram-creation-failure` 在公开 Models
创建 `pi.ai.request.duration` 直方图时注入一次故障。故障和公开调用边界处，两侧
provider 派发均为零。Pi 同步传出原始异常，检查点内未派发 provider，内存记录器没有
`pi.ai.request` span。Rust 发出一次 WARN 并返回调用句柄，随后派发一次 provider，
源流到达 EOF 并释放，交付成功 `Done` 和匹配结果。Rust 请求 span 在工厂故障前已经
创建，包装在调用返回前已 Drop，后端 `finish` 调用为零，业务完成后的记录仍开放。
[#49 冻结轨迹](../evidence/phase5-request-duration-histogram-creation-failure-differential.json)

#50 将两项直方图和两项计数器的创建放在 span 启动前。工厂错误直接返回手动调用者，
自动观察器通过宿主 tracing 报告故障并继续业务。重放中两侧在工厂故障、公开调用边界
及最终检查点均记录零个请求 span。Rust 后续派发一次并交付成功结果，Pi 在派发前同步
抛出原始异常。
[#50 实施与回归](../issues/50-create-ai-instruments-before-starting-span.md) · [冻结轨迹](../evidence/phase5-request-duration-histogram-creation-recovery-differential.json)

2026-11-11，用户在收到保留 Rust 对这一次工厂故障的隔离建议后回复“继续推进”。
公开 Models 在派发前创建 `pi.ai.request.duration` 直方图遇到单次
`TelemetryError` 时，Rust 保留调用句柄和后续业务执行，并通过 WARN 向宿主报告
观察器启动故障。这样调用方可取得 provider 的实际结果，宿主可诊断遥测故障。
该用例的公开调用边界、后续派发、业务结果和宿主诊断登记为第 6 条的具名**有意差异**。
[#50 决定记录](../issues/50-create-ai-instruments-before-starting-span.md#accepted-decision)

### 请求 span 启动失败

2026-11-11，`models-request-span-start-failure` 在四个指标创建成功后、provider 派发前
令 `pi.ai.request` 启动失败一次。两侧均返回公开句柄，随后各派发一次，所有检查点的
请求 span 数量均为零。Pi 未等待被拒绝的启动 Promise，首个 `Start` 转发后，观察器访问
未初始化的 `span.addEvent` 抛错，关闭受控源迭代器并交付 `error`。公开 `result()`
兑现为同一错误消息。宿主在有界检查点收到注入的原始 rejection 及结果观察器访问
`span.setAttributes` 产生的另一次 rejection。Rust 记录一次原始启动 WARN，源流产出
`Done`、到达 EOF 并释放，调用方取得匹配的成功结果。
[#51 冻结轨迹](../evidence/phase5-request-span-start-failure-differential.json)

2026-11-11，用户在收到保留 Rust 对这一次请求 span 启动故障的隔离建议后回复
“继续推进”。据此在公开 Models 派发前、四个指标均创建成功后，`pi.ai.request`
启动返回单次 `TelemetryError` 时，Rust 停用该请求的 AI 观察器并通过 WARN 向宿主
报告原始故障，让调用按 provider 的实际终态完成。此用例的业务结果、源流退出及
宿主诊断登记为第 6 条的具名**有意差异**。
[#51 决定记录](../issues/51-observe-request-span-start-failure.md#accepted-decision)

### 首事件为 Done 时的首块时长记录故障

2026-11-11，`models-first-chunk-duration-record-failure` 在公开 Models 的首个源事件为
`Done` 时注入一次 `pi.ai.first_chunk.duration` 记录故障。两侧的请求 span 已写入首块事件，
故障处各有一次派发和一个源终态，源流均未到达 EOF。两侧公开流均交付唯一的成功 `Done`，
读到公开 EOF 后，`result()` 均得到与终态匹配的成功结果。
[#52 冻结轨迹](../evidence/phase5-first-chunk-duration-record-failure-differential.json)

Pi 先转发 `Done` 并锁定公开成功结果，后续记录抛错触发源迭代器 `return()` 及 `finally`。
结果观察器将合成的错误写入请求 span 和请求时长样本，span 为 `stop: error`、`provider_error`，
宿主在有界检查点监听到零次 `unhandledRejection`。Rust 报告一次 WARN 并结束请求 span，
保留首块事件及默认 `ok` 状态，缺少 `stop` 和请求时长样本。在源流 EOF 屏障内，Rust 的首个
公开 `next()` 仍待定，放行后源流到达 EOF 并释放。

2026-11-11，用户在收到保留 Rust 读完源流并通过 WARN 报告这一次遥测故障的建议后回复
“继续推进”。据此在公开 Models 的首事件为成功 `Done`、首块事件已写入时，若
`pi.ai.first_chunk.duration` 记录返回一次 `TelemetryError`，Rust 保留源流收尾、报告
原始故障并结束请求观察器。调用方收到 `Done` 时，源流已到达 EOF 并释放。此用例的终态
交付时点、源流退出、span 状态与属性、时长样本及宿主诊断登记为具名**有意差异**，涵盖
第 2 条的交付时序与第 6 条的观察器异常。
[#52 决定记录](../issues/52-observe-first-chunk-duration-record-failure.md#accepted-decision)

### 成功请求的时长记录失败

2026-11-11，`models-request-duration-record-failure` 在公开 Models 的成功终态结算时注入
一次 `pi.ai.request.duration` 记录故障。故障处两侧均已到达源流自然 EOF 并写入请求 span 的
`stop: stop` 和 `usage.input`。Rust 已释放源流，Pi 的受控源迭代器已执行 `finally` 并读取源
`result()`。两侧均交付唯一的成功 `Done`、公开 EOF 及匹配的结果。时长记录故障之后，两侧的
内存 span 在最终检查点仍开放，均只记录了首块时长样本。Rust 后端包装已 Drop，`finish` 调用
为零，宿主收到一次原始故障 WARN。Pi 宿主监听在有界检查点记录零次 `unhandledRejection`。
[#53 冻结轨迹](../evidence/phase5-request-duration-record-failure-differential.json)

2026-11-11，用户在收到复用 #47 显式收尾路径的建议后回复“继续推进”。`pi.ai.request`
以 `Stop`、`Length` 或 `ToolUse` 完成且 `pi.ai.request.duration` 记录返回 `TelemetryError`
时，Rust 自动观察器报告原始错误，保留已写入的最终属性并尝试结束 span 一次，跳过剩余计量。
结束失败会另报一条 WARN。手动 `AiRequestTelemetry::finish` 返回原始错误，由调用者决定后续操作。
[#54 实施与决定](../issues/54-finish-request-span-after-duration-record-failure.md#decision)

#54 重放中，Rust 内存后端结束 span 一次，`stop: stop` 和先前写入的用量属性保留。Pi 的 span
在有界检查点仍开放。源流释放、成功终态和匹配的结果与 #53 一致。该单次时长记录故障的 span
结算及宿主诊断登记为第 6 条的具名**有意差异**。
[#54 冻结轨迹](../evidence/phase5-request-duration-record-recovery-differential.json)

### 首项为类型化失败时的首块计量

冻结版 Pi 1.5.0 将首块定义为首个规范化流事件，覆盖派发前的 setup error 与派发后的首项
失败。Rust 自动观察器在首个规范化流项交付时记录 `pi.ai.first_chunk` 及对应时长。
首项为 `Err(CallFailure)` 时也记录，普通 chat 的准备失败与 deferred fetch 走相同计量规则。
已有 `Start` 等交付时，后续失败不增加第二个首块样本。span 仍以实际的终态结算。
[#60 源码沿革、修复及两组重放](../issues/60-align-first-chunk-on-typed-failure.md)

### 启动后的 deferred fetch 终态与收尾

启动任务缓存首个成功 `Done` 后转发事件，并继续排空源流。公开 EOF 和 `result()`
等待源流释放及已启用的 poll observer 结算。若 `abort()` 或 runtime 停止中断已缓存
结果的尾部，等待任务退出后用该结果结算 poll，结果与终态是否已读取无关。强制取消在
缓存成功结果前获胜时，等待任务退出后按类型化 `Cancelled` 结算。源流 panic 仍传播。
这条实现契约适用于 `Models::stream_deferred(...).start()`。
[#74 实施与重放](../issues/74-deliver-started-deferred-done-before-source-release.md)

### deferred 结果式就绪与 poll 结算

Tokio 上的 `Models::fetch_deferred_ready` 在首个规范化终态发送结果，包括待定、就绪和
类型化失败。接收者返回或丢弃后，原任务通过 `ChatStream::result()` 继续排空源流并结算
poll。受控 EOF 屏障内结果可用且 poll 仍开放，释放屏障后完成结算。需要等待结算的调用方
使用 `fetch_deferred`、启动后的 `result()` 或读到公开 EOF。
[#76 就绪与收尾重放](../issues/76-return-deferred-outcome-before-source-release.md)

在 #55 的两次零等待 fetch 中，派发、结果和最终 poll 计量一致。直接 `fetch_deferred`
在返回前释放 SDK 持有的源流，并在记录正常时结算已启用的 poll。将该有限源流场景的
返回时计量可见性登记为有意差异。Durable 根据返回消息安排下一次轮询或写入答案，迁移
该调用时使用 `fetch_deferred_ready`。若尾流立即结束，工作任务也可能先于调用方恢复
执行完成结算。
[#96 重放与裁决](../issues/96-decide-deferred-wait0-poll-settlement.md)

在 #56 的成功终态场景中，尾流待定时 Pi `fetchDeferred` 和 Opi `fetch_deferred_ready`
已返回结果，三条路径的 poll 均未结算。保留直接 `fetch_deferred` 等待源流释放和 poll
结算的返回边界，并把这一时序登记为有意差异。零等待只在 provider 侧执行一次状态检查，
源流仍待定时直接 fetch 可以继续等待。Durable 的受控调用可在两条尾流挂起时发起下一次
轮询并写入答案，该用法对应 Tokio 上的就绪入口。
[#97 三条路径的重放与裁决](../issues/97-decide-deferred-fetch-held-eof.md)

#57 中，成功 `Done` 已缓存且源流尾部仍待定时，Opi 的本地取消释放 SDK 持有的源流，
按缓存结果结算 poll，直接 fetch 随后返回该结果。Pi 传递取消信号，受控源流在放开 EOF
后才收尾。双方均保留就绪结果，原 handle 仍可显式 fetch，远端取消计数均为零。将本地
清理和 poll 结算时序登记为有意差异。Durable 的任务中止与显式远端取消由应用负责。
[#98 本地取消重放与裁决](../issues/98-decide-deferred-local-cancel-after-done.md)

#58 中，取消发生在缓存首个终态之前。Opi 释放仍待定的 SDK 源流，在直接 fetch 返回
`Err(CallFailure)` 前按 `Aborted` 结算已启用的 poll。Pi 的受控源流在发出取消终态后
自然结束，公开结果兑现后才读取源流结果并结算 poll。双方最终计量一致，原 handle 均可
再次 fetch。将源流退出方式和 poll 结算时序登记为有意差异。
[#99 终态前取消重放与裁决](../issues/99-decide-deferred-local-cancel-before-done.md)

### 启动 deferred 的公开 EOF

冻结版 Pi 1.5.0 的遥测测试读完 `streamDeferred` 后核对事件及结果，再单独等待 span 结算。
Durable 的业务继续点是 `fetchDeferred()` 的结果。受控尾流待定时，两侧都交付相同的成功
`Done`，Pi 此时已可返回公开 EOF。Rust 保留 `.start()` 的最后一次读取作为收尾边界，以观察
源流释放和已启用的 poll observer 结算，并在 join 时传播尾流 panic。需要立即继续业务的
流式调用者可以在循环内处理 `Done`，结果式调用者使用 `fetch_deferred_ready`。

将成功 `Done` 后的公开 EOF 时序登记为此入口的**有意差异**。证据范围是冻结版仓库中的
调用者和受控尾流。整个 `deferred-stream-source-cleanup` 比较及其中的其他观察仍保留。
[#77 调用者和时序证据](../issues/77-review-deferred-stream-eof-consumers.md)

### Deferred 的结果等待与直接丢弃

#67 中，Pi 的 `.result()` 在首个成功 `Done` 时可用，Opi 启动后的 `.result()` 等待源流
释放和已启用的 poll 结算，join 时传播尾流 panic。保留这一**有意时序差异**。Durable 在
结果就绪时安排后续轮询和写入答案，对应 `fetch_deferred_ready`。流式调用者可以在 `Done`
时继续业务，由启动任务完成尾流。

直接 collector 的所有权由调用方持有。在 `Done` 后丢弃它会立即释放源流，Pi 的公开
iterator 停止后则由转发器继续排空。保留直接释放的所有权边界和 `.start()` 的独立收尾。
直接 collector 缓存成功结果后被丢弃时，先释放源流，再按缓存结果显式结算一次已启用的
`pi.ai.poll`。尾流已待定时也适用。已通过公开 EOF 结算的 poll 在丢弃时不再结算。
[#101 重放、调用方与裁决](../issues/101-decide-deferred-stream-result-and-drop.md) ·
[#102 修复与差分](../issues/102-settle-deferred-poll-on-direct-drop.md)

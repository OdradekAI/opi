# agent：内存对话循环、工具批次与结算屏障

分析日期：2026-10-16。对象为 Pi 1.2.0 固定快照。只读分析源码、文档与测试，没有安装、构建、测试运行或 provider 调用；测试影响 `none`。

## 职责、入口与依赖

`@earendil-works/pi-agent-core` 当前是可嵌入的有状态循环：`Agent` 包装器、低层 `agentLoop/agentLoopContinue` 与 awaited sink 形式 `runAgentLoop/runAgentLoopContinue`、`runToolCall`、`streamProxy`、默认 stream 配置及其类型。根 exports 加 `./package.json` 是全部发布路径；不存在当前 harness、session repository、文件工具、技能加载、compaction 或 telemetry schema 子入口。生产依赖为 ai `^1.2.0` 与 TypeBox `1.3.27`，Node `>=22.19.0`。源码 6 文件/2519 行，测试 4 文件/3969 行。 [manifest](../../../../.repo/pi-1.2.0/packages/agent/package.json#L1)、[index.ts](../../../../.repo/pi-1.2.0/packages/agent/src/index.ts#L1)、[1.0.0 移除记录](../../../../.repo/pi-1.2.0/packages/agent/CHANGELOG.md#L23)。

`StreamFn` 是结构化协议，不强制特定 provider 或 transport。它接收 ai 的 `TranscriptContext`，必须返回 `AssistantMessageEventStream`（可以先以 promise 交付），运行失败以 stream error 与最终 assistant message 表达，不应 throw/reject。构造器类型要求 `streamFn`；运行时为旧编译调用者保留已由宿主配置的 fallback，没有自动注册全部 provider。未配置 fallback 时抛出清楚错误。 [StreamFn 契约](../../../../.repo/pi-1.2.0/packages/agent/src/types.ts#L19)、[构造器](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L230)、[stream-fn.ts](../../../../.repo/pi-1.2.0/packages/agent/src/stream-fn.ts#L1)。

主要消费者可以把 `Models.streamSimple.bind(models)` 注入 Agent，也可以注入代理函数；MCP/codemode 只是通过工具接口组合的示例，不是本包默认启用的网络或执行环境。 [README quick start](../../../../.repo/pi-1.2.0/packages/agent/README.md#L11)、[MCP/codemode 组合](../../../../.repo/pi-1.2.0/packages/agent/README.md#L506)。

## 状态归属与生命周期

`Agent` 持有可执行工具数组、模型、thinking level、完整 `AgentMessage[]`、steering/follow-up 队列，以及唯一 activeRun 的 promise/AbortController。runtime-owned streaming/pending/error 字段在公共类型上只读；给 tools/messages 重新赋值复制顶层数组，取回的数组仍可原地修改，不是深冻结。系统提示不是独立可写寄存器：leading system message 建立基线，后续 system messages 通过 content/sections/tools delta 表达变化，`state.systemPrompt` 是 replay 投影。 [状态创建](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L70)、[README state 契约](../../../../.repo/pi-1.2.0/packages/agent/README.md#L280)。

`prompt` 先拒绝重入，再把 string/图片归一成 user message，创建 activeRun，设置 isStreaming，复制当前 context/tools，运行低层循环，最后清理 transient state 并 resolve idle。事件 reducer 先更新内存，再按注册顺序 await listener，所以 assistant 的 `message_end` 处理完才进入工具 preflight；这是宿主持久化、权限检查与 UI 同步的重要屏障。`agent_end` 是最后一个循环事件，但 `prompt()/continue()/waitForIdle()` 及 isStreaming 的结束点在其 listener 全部结算之后。 [prompt](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L370)、[runWithLifecycle](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L507)、[processEvents](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L558)。

`reset` 只允许 idle 时执行，保留 replay 后的系统提示/工具声明基线，清除对话、队列与 runtime 状态；不更换模型或可执行工具。`abort()` 仅向当前 run 发 signal；工具、hooks、stream 与 listeners 必须合作结束，没有强制终止 JavaScript 或持久事务回滚。内存 transcript 的持久化/恢复由应用 listener 或外部 session 层负责，本包没有磁盘所有权。 [reset/control](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L335)。

## 请求、turn 与继续调度

关键链为 `输入事件 → 声明工具变化 → prepareRequest → transformContext → convertToLlm → normalizeContext → getApiKey → streamFunction → message_end → 工具批次 → finishTurn → turn_end → 队列/下一轮或 agent_end`。`transformContext` 工作于 AgentMessage 层，`convertToLlm` 才过滤自定义角色并输出 system/user/assistant/toolResult。模型请求没有 `context.systemPrompt/tools`，它们位于 transcript。工具可执行集合 `context.tools` 与已声明集合不同，会生成或合并 system 工具 delta，以确保 replay 可见工具与实际 loadout 对齐。 [runLoop](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L163)、[declareToolChanges](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L323)、[请求转换](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L381)。

`prepareRequest` 在每次 conversational 请求前运行，已经选定的输入先发出；可替换 context/model/thinking level，但不会额外轮询队列。`prepareNextTurn` 只在确实要继续后执行，并允许在长准备期间再收集尚未选择的 steering；`Agent` 同时支持旧 signal-only callback 与 context-aware callback，后者优先。`finishTurn` 在 finalized assistant/tool result 后、turn_end 前运行；正常响应的 `end` 在 turn_end 后立即结束，队列不消费；`continue` 保证恰好一个后续请求，自然工具或队列请求可以满足它；每次都返回 continue 会形成无限循环。error/aborted 响应也调用 finishTurn，但其返回决策不改变硬退出。 [turn hooks](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L183)、[结束与继续](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L245)、[类型](../../../../.repo/pi-1.2.0/packages/agent/src/types.ts#L134)。

队列默认 `one-at-a-time`，可选 `all`，peek 不消费。steering 在启动和完整工具批次结束后被选取；它不会跳过当前 assistant 已提出的工具。follow-up 只在没有工具继续、没有 steering 时消费。`continue()` 对空或仅 system 历史拒绝且保留队列；非 assistant 尾从既有 context 重试，assistant 尾则先借一个 steering batch，再借 follow-up batch，否则拒绝。 [队列](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L143)、[continue](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L383)、[循环队列调度](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L294)。

## 工具批次、失败与并发

每次调用先发 `tool_execution_start`，再找工具、`prepareArguments`、TypeBox validation、`beforeToolCall`；不存在、校验失败、hook throw、block 或已 abort 都成为 error outcome，不调用 execute。before hook 收到 validated args 的可变引用，之后没有再次校验，因此 hook 的修改就是实际执行输入；它是有权修改参数的宿主边界。 [prepareToolCall](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L694)、[对应测试](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L528)。

默认 parallel 先顺序完成整个批次 preflight，再 `Promise.all` 启动允许执行的工具。任一工具 `executionMode: sequential` 会使整个批次顺序执行。完成生命周期 `tool_execution_end` 以实际完成顺序发出；最终 toolResult message 与 turn_end.toolResults 保持 assistant 源顺序。parallel 没有额外并发数上限，等于该批次已允许的调用数；共享资源的串行策略需要工具/宿主明确提供。 [模式选择](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L508)、[parallel](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L586)。

execute 可流式报告 onUpdate；实现收集 sink promise 并在 execute 结算后等待它们，之后的 late callback 被忽略。execute throw 转成 error result；after hook 可逐字段覆盖 content/details/usage/isError/terminate，throw 同样变成 error；替换 content 时若没有配套 structuredContent 则丢弃旧结构结果，避免语义不一致。覆盖实现使用 nullish fallback，因此不能把 `structuredContent: null` 当作普通替换值（分析者推断：对已存在非空结构结果，该意图会被旧值回退）。`structuredContent` 供组合工具的直接 outcome 使用，当前创建的普通 transcript toolResult 不包含它。 [execute](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L821)、[finalize](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L859)、[toolResult 构造](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L930)。

1.1.0 新增的 `durationMs` 用 `performance.now()` 在 execute 前后测量并 round：不含 before/after hook；未执行调用没有 duration。工具更新 sink 的等待发生在取成功 duration 之后；该数值不是完整批次、hook 或持久化耗时。throw 的 execute 也有计时。它被复制到 outcome、tool_execution_end 与 toolResult message。 [计时与传播](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L821)、[duration 测试](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L402)。

只有当前已 finalized 的批次结果全部 `terminate: true` 才抑制自动工具 follow-up；混合批次继续，steering/follow-up 仍可触发下一轮。assistant `length` stop 的所有工具调用一律转为错误，不执行即使其残缺 JSON 可被修复且通过 schema，避免截断命令产生副作用。`runToolCall` 暴露相同 prepare/execute/finalize 机制，供嵌套工具应用 hooks，但不自动发 Agent events 或追加 transcript。 [截断保护](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L471)、[terminate](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L690)、[runToolCall](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L790)。

## 事件、失败与代理协议

`runAgentLoop` 的 awaited sink 可形成背压；低层 `agentLoop()` 返回的 EventStream 只把事件推入队列，不等消费者的异步处理，不具有 Agent listener 屏障。自定义转换/stream/队列 callback 的契约是不得 throw/reject；低层 observational wrapper 仅对 run promise 安装 success handler，违约失败可能缺少常规 terminal sequence。Agent 包装器 catch 运行异常并尝试补发 assistant failure、turn_end、agent_end；listener 若再次抛出，仍可能向 prompt 调用者传播，不能假设观察者异常永远隔离。 [observational wrapper](../../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L38)、[契约](../../../../.repo/pi-1.2.0/packages/agent/src/types.ts#L193)、[失败补序列](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L532)。

`streamProxy` POST `${proxyUrl}/api/stream`，Bearer auth 由应用提供，只序列化白名单 options；callback、AbortSignal、telemetry object 不送入 JSON。接收 `data: ` 行上的 compact 事件，在客户端重建 shared partial 与 JSON 参数。toolcall_end 的完整 metadata 覆盖临时对象。EOF 无 terminal 变成 error，abort 会 cancel reader 并撤除 listener；没有完整 SSE parser、流大小上限或 provider 重试策略。1.1.0 改为 ai 的专用 AssistantMessageEventStream，获得相同 timing 语义。 [options whitelist](../../../../.repo/pi-1.2.0/packages/agent/src/proxy.ts#L48)、[网络与 EOF](../../../../.repo/pi-1.2.0/packages/agent/src/proxy.ts#L107)、[tool metadata](../../../../.repo/pi-1.2.0/packages/agent/src/proxy.ts#L355)。

## 测试预期与证据边界

静态核对了以下测试表达的核心预期，没有执行：

| 面向行为 | 证据 |
|---|---|
| 工具计时排除 hooks、block 无计时 | [agent-loop.test.ts:402](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L402) |
| 完成顺序事件/源顺序消息、steering 等完整批次 | [agent-loop.test.ts:668](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L668)、[762](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L762) |
| prepareRequest/finishTurn 与 continuation、保留队列 | [agent-loop.test.ts:1108](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L1108)、[1404](../../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L1404) |
| 事件 listener settlement、late update 抑制、reset/reentry | [agent.test.ts:361](../../../../.repo/pi-1.2.0/packages/agent/test/agent.test.ts#L361)、[472](../../../../.repo/pi-1.2.0/packages/agent/test/agent.test.ts#L472)、[675](../../../../.repo/pi-1.2.0/packages/agent/test/agent.test.ts#L675) |
| proxy 末行无 newline、metadata 与 premature EOF | [proxy.test.ts:33](../../../../.repo/pi-1.2.0/packages/agent/test/proxy.test.ts#L33) |
| 注入 faux provider 的多轮、工具与 abort | [e2e.test.ts:186](../../../../.repo/pi-1.2.0/packages/agent/test/e2e.test.ts#L186) |

## 文档规划、历史与升级语义

全文审读 README 1–564 与 CHANGELOG 1–780。没有单独路线图。重要历史方向逐项核查：

| 文档主张/动机 | 当前状态与理由 |
|---|---|
| 0.84.0 将 v4 session/harness 与 telemetry 放入默认导出 | 已被 1.0.0 的收缩替代；CHANGELOG 23–27 与当前 index 一致，不能把旧 harness scaffold/HarnessNotImplemented 当当前包缺口 |
| prepareRequest/finishTurn 替换 shouldStopAfterTurn | 已兑现，before-request 与 before-turn_end 的时间点有实现和测试；旧 hook migration 属历史指导 |
| parallel 完成顺序与持久消息次序 | 当前源码是 completion-order end/source-order messages；0.68.0 的“全部 completion-order artifacts”陈述被 0.68.1 修正 |
| steering 跳过剩余 tools | 0.32.0 历史记录局部已替代，0.58.4 与当前 README/代码明确等当前批次结束 |
| 自定义消息、可变工具集合、宿主 stream | 已兑现，持久化、权限 UI、retry policy 与执行环境仍由应用拥有 |

README 的概念段 45–49 说 LLM 只有 user/assistant/toolResult，以及 low-level 示例 541 的 filter 仍排除 system，和当前 ai Message/本包默认 converter 不一致；照抄会丢失系统提示与工具声明。README 错误处理段 488–502 只教 throw，而当前 AgentToolResult 也支持 `isError`，MCP 示例就使用它，这是说明覆盖不足。telemetry README 仍声称 agent-core 导出 schema，也与 1.0.0 删除事实冲突。 [README 旧 filter](../../../../.repo/pi-1.2.0/packages/agent/README.md#L539)、[当前 converter](../../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L38)、[当前 MCP 示例](../../../../.repo/pi-1.2.0/packages/agent/README.md#L506)。

以旧 1.0.4 manifest 作路径/摘要基线，本包变化为 package version、CHANGELOG、agent-loop/proxy/types 及 agent/agent-loop tests。主要新语义仅执行 duration 与 proxy 专用 stream 返回类型；Agent class 与 README 内容未变。本次重读当前主链，不依据旧源码逐行比较（旧 source 未提供）。跨包 ai 的专用 stream 名义类型、classifier 扩展不意味着本包自动获得分类调度；Agent 仍执行 chat transcript。风险主要在无上限并发、listener/工具不合作取消、低层违约 callback 和 README 过滤 system 示例。阅读顺序：types → agent.ts lifecycle → agent-loop.ts 调度/工具 → proxy.ts → 对应测试。

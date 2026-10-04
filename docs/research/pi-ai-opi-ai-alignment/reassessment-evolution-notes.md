# 按“pi 的 Rust 实现”重新审视延后范围

> 非规范性研究补记，2026-10-03。按用户最新纠偏，本轮以 pi 的行为、演进意图及 Rust 所需差异为主，opi-spec 仅作参考；不再用“opi 暂无消费者”或原 core 准入门槛替代行为选择。本页只提出对既有候选决定的修订建议，不修改其他文档、运行时或既有测试。
>
> 中文新研究材料，没有既有英文 counterpart。已使用 research、codebase-design 的一手证据和职责分析方法。所列测试全部是静态阅读或断言定位，**本轮未运行**；没有模型调用、依赖安装、凭据读取或上游代码更新。

## 结论与建议变更

旧候选有几处把“opi 还没接入”作为延后的依据，导致 pi 已发布的普通路径被降为未来可选。新的前提下，应调整这些范围判断，而不是仅把报告里“有意差异”改个名称。

| 原问答／决定／要求 | 当前证据分类 | 本轮建议 |
|---|---|---|
| Q18 / D-11：remote/background catalog；AIR-002 与 scope | pi 已发布，interactive/RPC 默认在启动后刷新；SDK 创建默认先本地恢复 | 撤回“等 opi 生产消费者再考虑”的延后理由。纳入远端目录和明确刷新行为的目标调查/设计；库机制与产品调度分开，Rust 可采用不同并发实现。不是要求定时常驻轮询。RE-01。 |
| Q18 / D-11：WS continuation；scope | Codex 已发布的默认 `auto` 路径，产品明确传入，含可条件复用的 cached context | 撤回“无 opi SSE 瓶颈测量故延后”的范围依据。按 Codex 的已交付协议能力设计 Rust 对应传输/续接/回退，不将 WebSocket 抽象泛化到所有 provider。RE-02。 |
| Q18 / D-11：automatic deferred-tool loading | 需拆成已发布 adapter 机制与已发布、按配置启用的产品工具发现 | 不可整体称为未来需求。模型侧渐进声明和其具体 wire 应进入 transcript 对齐；`tool_search`/MCP 产品编排作为已实现跨包消费者记录，产品覆盖是否同批完成可另定，但不能说没有 pi 真实调用方。RE-03。 |
| Q18 / D-11：deferred responses | 已发布公共契约 + faux + durable 消费；内建远端 adapter 仍无证据 | 保留交付程度限制，撤回“只是未来提案”的可能误读。若暂不实现，应明确是已发布接口的暂缓覆盖，而非缺少 opi 消费者即否定。不可承诺远端 deferred 能力已完整。RE-04。 |
| Q18 / D-15 continuation coverage；AIR-010 后细则 | Google/Responses/Chat/Bedrock 的 continuation 数据已在 pi 普通 adapter 解析与重放 | 撤回将 encrypted/thoughtSignature/Bedrock reasoning 一概留待新需求的限制；扩展 parser→typed history→session→wire 验收范围。旧历史未知仍不能伪造。RE-05。 |
| Q18 / D-09：任何 Start 后不自动 retry；AIR-007 | opi 独立产品策略，**不是 Rust 所要求**；pi 产品默认有 bounded whole-turn recovery | 拆开低层 attempt 不隐式重播和高层显式失败回合恢复。推荐对齐后者及其失败记录、projection、事件、取消和预算；不据此偷偷改变凭据冻结或重放工具副作用。RE-06。 |
| Q14：新操作统一不自动 retry；AIR-026 | image 的默认零 retry 与 pi 接近；System One 的默认 2 HTTP retries 不同 | 作为关联策略差异交还主决策重新核对；不因 Rust `Result`/Future 语法就宣称两者等价。它独立于可见输出后的回合恢复，不能合并成一个 retry 开关。RE-06。 |

上述建议需要主文档按新前提更新目标；本页不把此前问答自动改写为新的批准。已证实的 UTF-8、终态、drop 等缺陷仍须修正；有界队列、typed errors、owned bytes 等 Rust 设计仍有独立理由。**性能收益未测量**不妨碍确认 pi 已有某项可观察能力，但也不允许虚构 Rust 对应实现更快。

## 基线与历史材料口径

继续使用 [EVO-001](design-evolution.md)：ai 包的 378 个 tracked blob 已与官方 v1.0.0 `a13d35a742c6ef8462812a28fbe1d8c8b7431c32` 比对一致。跨包消费者读取本地同快照；不把 ai 包核验扩大为全 monorepo 内容证明。

本轮重新读取本地源码与 release changelog，复用此前已核验的官方 [RFC0039](https://rfc.earendil.com/0039/)、[refresh 取消提交 fed6009](https://github.com/earendil-works/pi/commit/fed6009cc9ccfb15088bc55244120918b5fe23f2)、[transcript PR9548](https://github.com/earendil-works/pi/pull/9548) 及 [tag 后 Anthropic inline-tools 提交 b271b0a](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db)。没有本轮新的 main/latest 查询；此前固定窗口仍是 EVO 所记 `11449730c8a733953ce1bcce70e066bccaa778a5`，不能称作持续最新。

下文 changelog 链接证明的是该发布记录的明确主张；未重新访问其中每个 issue/PR，不能据编号猜测额外维护者承诺。源码和默认接线提供独立的当前行为证据。

## RE-01 目录：已进入默认产品，不是等待消费者的 substrate

**当前默认链（高置信，静态）：**

- [ModelRuntime.create:216–255](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts#L216) 默认创建 file store（无 modelsPath 时用 memory），将普通 built-in providers 包上 `withRemoteCatalog`，Radius 有自己的动态机制；默认 create refresh 的 network 只有显式 `allowModelNetwork === true` 才开，通常先恢复本地。
- [remote-catalog-provider.ts:12–22](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/remote-catalog-provider.ts#L12) 默认源 `https://pi.dev`，支持 chat/image/classifier。[`:76–166`](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/remote-catalog-provider.ts#L76) 先发布缓存，再检查网络/取消/4 小时 freshness；ETag/304、404/501、失败保留旧数据和 generation-aware publish 都有实际分支。4 小时是刷新时的 freshness gate，**不是证明每 4 小时定时轮询**。
- [main.ts:937–945](../../../.repo/pi-1.0.0/packages/coding-agent/src/main.ts#L937) 在非 offline RPC 路径启动 15 秒预算的后台 refresh；[interactive-mode.ts:1133–1143](../../../.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L1133) 在 TUI init 之后做同类刷新。不是用户必须先装实验扩展。
- [model-catalog-refresh.ts:14–47](../../../.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/model-catalog-refresh.ts#L14) 将交互模式同时出现的 refresh 合并，每个等待者有独立取消，最后一个等待者离开才 abort。另有 `/model` 与手动强制刷新消费者。

**演进意图：** 0.80.8 引入 live model catalog refresh；0.81.0 把自动网络刷新从启动初始化移到 interactive/RPC 已运行阶段；0.81.1 修 footer 等待后台请求的问题；0.82.1 用 ETag；0.84.0 修取消、旧 generation 发布和登录后刷新卡住。这是一条“新鲜目录 + 快速可用的旧快照 + 独立取消”的持续演进，不是要增加任意后台任务框架。[coding-agent CHANGELOG:1004](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L1004)、[`:965`](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L965)、[`:899`](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L899)、[`:791–795`](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L791)。

**反证/限制：** 裸 `Models` 不会自行访问 pi.dev；SDK create 不默认网络刷新，offline 会抑制该网络路径；不是所有 provider 都有同一 catalog 源；快照缺 ignored catalog 数据，所以模型完整名单未核验。本轮未发 catalog 请求。

**Rust 差异与原决策修订：** 当前 opi 独占 `&mut` refresh 能简化串行更新，是 Rust 所有权选择；它不能自动取代后台刷新、非阻塞 listing、取消独立性和过期发布保护的语义。纳入产品刷新后，应重新选择 owned snapshot/发布 owner，不能机械加 TS generation，也不能以旧独占接口为理由删除 pi 行为。源服务、持久化目录和调度属于对应产品 owner，库保留明确 refresh/缓存契约。测试设计已有 [remote-catalog-provider.test.ts:75–282](../../../.repo/pi-1.0.0/packages/coding-agent/test/remote-catalog-provider.test.ts#L75)、[model-catalog-refresh.test.ts:23–60](../../../.repo/pi-1.0.0/packages/coding-agent/test/model-catalog-refresh.test.ts#L23)；本轮未运行。

## RE-02 Codex WebSocket 与 cached continuation：默认、条件性复用

**当前调用链（高置信，静态）：** [settings-manager.ts:904–905](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/settings-manager.ts#L904) 缺省返回 `auto`；[sdk.ts:419](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts#L419) 明确交给 Agent；[agent.ts:251、476](../../../.repo/pi-1.0.0/packages/agent/src/agent.ts#L251) 默认也是 auto 并向 stream options 传递；[Codex adapter:294–307](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L294) 优先 WS，session 被标记 fallback 时改 SSE。

因此 P13 先前指出的“adapter 内省略 transport 与显式 auto 的 cached-context 检查不同”不能用于弱化正常产品结论：正常 Agent 路径明确传入 auto。[`:1518`](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L1518) 的 cached context 条件在该路径成立。

续接不是任意历史截短：[`:1425–1476`](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L1425) 比较 input 之外的请求字段及完整历史前缀；匹配才发送 previous_response_id + delta，不匹配则清 continuation 并发完整 input。session/account 资源隔离、取消、连接限额、missing previous-response 恢复各有条件；一般 transport failure 的 WS→SSE fallback 在首个 parsed event 前才允许，不能泛化为全产品回合重试规则。cleanup 见 [`:949–965`](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L949)。

**明确演进：** [ai changelog](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L1033) 与 [product changelog](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L1976) 共同记载 **0.72.0（2026-05-01）**修正 streamSimple 总走 SSE，并让 auto + cached WS 成为默认；0.71.1 同日加入 websocket-cached；0.82.0 修 grammar tool delta，0.84.0 修不同 account 共用 cached socket。这比“无性能测量所以暂不考虑”更直接说明设计意图。这里更正早期消息中的未核版本猜测；确切版本为0.72.0。

**反证/限制：** 只针对 Codex，不是所有 provider 默认 WS；SSE 明确选择、环境限制和 session fallback 仍合法；cached-context 必須逐请求判定。没有吞吐、延迟或缓存收益测量。[openai-codex-stream.test.ts:1283](../../../.repo/pi-1.0.0/packages/ai/test/openai-codex-stream.test.ts#L1283) 有 auto 转发/WS 断言，[`:1414`](../../../.repo/pi-1.0.0/packages/ai/test/openai-codex-stream.test.ts#L1414) 有 account isolation 断言；本轮未运行，带 e2e/probe 名称的其他文件不作为已通过证据。

**建议：** 将已有 Codex 协议语义列为 Rust 实现目标，明确具体 transport/session owner；`reqwest` 本身没有该行为不是延后需求的理由。可以不用 JS 模块级 cache、不新增泛用 Transport trait，但不能把“内部实现不同”写成“只支持 SSE 仍语义全部对齐”。

## RE-03 Deferred tool loading：机制与默认启用程度必须拆开

三种事实同时成立：

1. pi-ai 的渐进 tool 声明/wire 支持已发布。0.80.7 引入 cache-friendly loading；0.80.9 支持 Kimi；0.84.2 Responses 优先 message-anchored additional_tools；0.86.0 transcript 后换成一般的 system/tool 时间线。见 [ai CHANGELOG:549](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L549)、[`:497`](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L497)、[`:256`](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L256) 及 EVO-008。beta header/placeholder 的具体形式会变，渐进声明不是未来幻想。
2. [extensions/index.ts:5–12](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/index.ts#L5) 将 tool-search 注册为 CLI built-in；[tool-search/index.ts:1–14](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/tool-search/index.ts#L1) 明示 **默认 inactive**，SDK 需显式加 extension。不能写成每个默认会话都暴露 tool_search。
3. [MCP exposure](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/mcp-servers.ts#L8) 缺省是 codemode；显式 `deferred` 意味 tool_search load 后供模型直接调用。[mcp/index.ts:451–474](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/index.ts#L451) 按配置自动激活 tool_search；[tool.ts:224–248](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/tool-search/tool.ts#L224) 实际搜索并修改后续可见 loadout。它是实际产品功能，不只是示例。

0.99.0 已把 codemode/tool search/MCP 作为内建扩展发布，0.99.2 修稳定提示前缀与等待时机，1.0.0 修已加载 deferred MCP tools 在 resume/reload 丢失。[product CHANGELOG:119](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L119)、[`:68–70`](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L68)、[`:43`](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L43)。这说明生命周期/恢复是实际维护对象，不是未被接受提案。

**建议与反证：** 取消笼统的“automatic deferred-tool loading 延后”；至少将库声明转换与已经确认的完整 transcript 一并设计，保留 provider 不支持时的明确策略。BM25、MCP 连接、codemode 并不因名称相近而变成 pi-ai 的职责；跨包编排可单独安排覆盖，但依据应是本次产品范围，而非 pi 无消费者。工具发现不能自动扩大执行权限仍成立；是否采用此前 opi 更严格的 snapshot visibility gating 是独立产品决定，不是 Rust 必然。

**状态/置信度：** adapter 支持已实现、产品可选功能已发布，CLI 默认注册但不默认 active；高置信。tag 后 [b271b0a](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db) 已改 Anthropic inline tool 表达，只证明具体 wire 继续演进，不是承诺何时将来发布。

## RE-04 Deferred response：已发布契约，远端完整性仍有缺口

[ai CHANGELOG 0.84.0:346](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L346) 明确添加 deferred provider request contracts、durable handles、auth fetch/cancel dispatch 与 faux 支持。当前 [types.ts:241、500](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L241)、[Models:912–945](../../../.repo/pi-1.0.0/packages/ai/src/models.ts#L912)、[faux:526–662](../../../.repo/pi-1.0.0/packages/ai/src/providers/faux.ts#L526) 均落地。

跨包 [durable/generation.ts:228、258](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L228) 真正消费 fetch/cancel；[`classify:426`](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L426) 保存 poll checkpoint/时间。因此“只有类型”不成立；也没有依据称作尚未合并提案。

本轮重新检索 ai/src，具体 fetch/cancel 仍只有 faux；未找到 builtin 远端 API 对应实现，也未找到普通 coding-agent session 的远端 deferred 默认路径。外部 custom provider 可能实现它，检索无命中不能证明整个生态不存在。

**建议：** 对齐地图应写“已发布库契约及 durable 消费，内建远端支持未证实”。若本次要覆盖 pi-ai 公共生命周期，可用 Rust handle/enum + faux conformance表达已知契约，不宣称真实后台推理已打通；若暂缓，应明确保留这一公开能力差异及重新评估条件（具体远端契约或 durable 目标），而不是以旧规范门槛/opi消费者缺失包装成非需求。仍不建立把 deferred response、tool loading、图片生成合一的通用异步框架。置信度高于本地检索范围，远端全生态未知。

## RE-05 Continuation metadata：多项被延后的其实是普通模型继续调用

| pi 1.0.0 行为 | 解析→重放证据 | 影响及重新归类 |
|---|---|---|
| Google/Gemini/Vertex 的 text、thinking、toolCall thought signatures | [google-generative-ai:150–208](../../../.repo/pi-1.0.0/packages/ai/src/api/google-generative-ai.ts#L150)、[vertex:159–216](../../../.repo/pi-1.0.0/packages/ai/src/api/google-vertex.ts#L159)、[google-shared:116–159、230–274](../../../.repo/pi-1.0.0/packages/ai/src/api/google-shared.ts#L116) | 部分 signature 可出现在可见空 text 上；按块绑定，不可任意挪动/合并。不是只给 debug 的字段。 |
| Responses/Codex encrypted reasoning 与 item metadata | [openai-responses:372](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-responses.ts#L372)、[Codex:560](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L560)、[shared:265、534–550、692](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-responses-shared.ts#L534) | store:false 的 stateless 下一轮仍需回传；Azure 仅 terminal 出现的 encrypted_content 有明确 backfill。不是等待新远端接口的功能。 |
| Chat Completions 的 reasoning_details | [openai-completions:326–331、665–675、1305–1376](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-completions.ts#L326) | 结构化 reasoning.text/summary/encrypted 保序回放，不能等同普通 display text。该项也应进入 continuation coverage，而不只修 text/tool 混合顺序。 |
| Bedrock signature / redactedContent | [bedrock:632–688](../../../.repo/pi-1.0.0/packages/ai/src/api/bedrock-converse-stream.ts#L632)、[`:1012–1047`](../../../.repo/pi-1.0.0/packages/ai/src/api/bedrock-converse-stream.ts#L1012) | 按模型能力保留签名或 encrypted bytes，redacted 不转换为可读摘要；非 Anthropic opaque reasoning 已有修复。 |

生产消息不是只能在 tests 中构造：[AgentSession message_end:1115–1133](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1115) 将 provider 最终消息存入 session；下一轮 adapter 使用这些消息。通用 [transform-messages:94–135](../../../.repo/pi-1.0.0/packages/ai/src/api/transform-messages.ts#L94) 在同 provider/api/model 保留、跨模型剥离不可重用 metadata。置信度高于已检查链路；未重跑 session roundtrip 或 live 签名接受性。

**演进并非一次实验：** 0.80.7 修 Azure terminal encrypted backfill；0.84.0 修 Google signed-empty blocks；0.84.3 修 Chat reasoning_details 与 Bedrock redacted。[ai CHANGELOG:562](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L562)、[`:372`](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L372)、[`:232–235`](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L232)。直接测试包括 [Azure replay:83、111](../../../.repo/pi-1.0.0/packages/ai/test/azure-openai-responses-reasoning-replay.test.ts#L83)、[Bedrock redacted:137–232](../../../.repo/pi-1.0.0/packages/ai/test/bedrock-redacted-reasoning.test.ts#L137)；本轮未运行。

**建议：** D-15/AIR-010 的“其余 encrypted/thoughtSignature/完整 Bedrock parity 待新需求”应改为逐 adapter 的已交付 continuation 目标和缺口矩阵。Rust 可用闭合 enum/typed metadata 代替 pi 借字符串保存 JSON，能改善验证与所有权；把元数据全部删掉则是可观察能力损失，不是语言差异。旧 opi 持久化中已丢字段仍须标 unknown，不补造；跨模型失效处理不能直接回传所有 opaque。

已有 tag 后 Bedrock stale-thinking [69f0be6](https://github.com/earendil-works/pi/commit/69f0be6f0a6ca1bf66a2a9cf59c821b632e6bca1) 是后续具体修复信号，应与1.0.0分开作为额外兼容证据。它不支持“完整 continuation 未稳定所以全部延后”，也不支持“所有签名永远原样回传”。

## RE-06 Retry：语言机制与产品恢复政策不是一回事

pi 至少有四层：HTTP acquisition helper、Codex WS/SSE 局部恢复、公开 assistant-call helper、产品完整失败回合恢复。P08/M-06 已修正事实分类，本轮要重新评价**保留 opi 不同行为的理由**。

- [settings-manager.ts:988–1005](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/settings-manager.ts#L988) 默认 `enabled=true`、3 retries、2000 ms base delay；不是只能由试验调用方启用。
- [AgentSession:1806–1821](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1806)、[`:3713–3752`](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L3713) 在 retryable 最终 error 后，保留 raw failed attempt、通过 context edit 从 model projection 排除，再退避继续。它没有“出现过 Start/任何事件就禁止恢复”的条件；也不把 abort/context overflow/配额等任意失败全重试。
- [retryAssistantCall:185–236](../../../.repo/pi-1.0.0/packages/ai/src/utils/retry.ts#L185) 公开支持有预算的完整调用重做；[compaction.ts:614–638](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/compaction/compaction.ts#L614) 是生产消费者。0.81.1 明确发布 compaction/branch summary 的恢复与事件，见 [CHANGELOG:956、964](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L956)。
- [durable generation:401–405](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L401) 不把无 content 的 Start 当可见 partial；[`:474–494`](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L474) 也按最终 error、预算和分类安排下一 attempt。不可用 opi 的 Start 边界冒充 pi 一致契约。

**建议：** 保留低层 attempt 的事件/终态不被悄悄重播、当前 prepared request/auth 不在一次 attempt 内漂移；在更高层引入与 pi 对应的、用户可见的失败回合恢复。新逻辑调用还是现有 call 的新 attempt、凭据是否重解析、每次失败如何保留及何时清 partial，必须在主 spec 固定；Rust 所有权本身没有强制某一产品政策。失败回合恢复不等于重复执行已经完成的 tool，也不授权跨 provider/account fallback。

**反证/限制：** 不主张复制 pi 错误字符串 regex 为 Rust 公共错误模型；可用 typed classifier保持同样分类语义。自动 retry 的计费/可见重复代价真实存在，若用户希望仍采用更保守行为，应明确记录为相对 pi 的产品选择，而非默认合理 Rust 差异。既有 pi 失败/取消/事件断言仅读未跑；不能声称所有中途失败被完全安全恢复。

关联 Q14：`System One` 的 pi adapter 在 [system-one-shared.ts:220](../../../.repo/pi-1.0.0/packages/ai/src/api/system-one-shared.ts#L220) 使用 `maxRetries ?? 2`，image helper 默认0。新 SDK 两操作统一零自动 retry 是原候选额外策略，应与新的对齐原则核对，但不得把这一 HTTP acquisition 差异与已输出内容后的回合重播混在一起。

## 验证与后续使用

本轮实际执行：`rg` 调用链/默认值检索，读取所引源码与测试断言，脚本定位 changelog 最近版本标题；没有运行 runtime 测试或上游生成脚本。测试影响 **none**。

后续主文档需要将旧 Q18/D-11/AIR scope 的“不纳入”逐项替换成上述状态与具体目标。Rust 差异应说明它如何保持同一可观察行为；新产品政策要单独标识，而不混入语言适配。已发布能力可分阶段实施，但阶段安排不能重新表述成“pi 未来也许会做”。未来规划继续仅采用明确的一手计划，缺少日期/承诺就保留未知。

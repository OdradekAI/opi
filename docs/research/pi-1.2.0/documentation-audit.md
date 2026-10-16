# 文档、规划与实现时效核查

核查日期：2026-10-16，固定本地 Pi 1.2.0。本文逐项区分实现事实、文档意图、测试预期、作者的历史测量与分析者推断；没有运行 Pi 测试或抓取外部 RFC。完整文档身份与实际范围分别见[文档索引](documentation-index.md)、[审读记录](documentation-reading.json)。机器清点不是阅读证明，行号有效也不是语义正确证明。

## 覆盖与判定方法

137 份文本均有唯一审读记录（109 份完整行范围、28 份明确选段/结构审读），覆盖根/隐藏资源、全部 11 包、示例、fixture、许可证与包外 issue triage 资料。重点读取当前设计理由、状态与生命周期、否定条款及后续替代说明，再回查入口、exports、调用方和关键测试源码。长历史日志及部分参考资料保留实际选段范围，未声称全文。旧版记录不继承为本轮阅读。

判定“已实现”需要当前实现/入口支持；“部分实现”列具体缺口；“局部陈旧”只否定冲突主张；“非目标”保留作者边界；“证据不足”不等于能力不存在。删除文件、版本号和 Done 标记均不单独证明规划兑现。

## 根、隐藏资源与包外工具

| 资料与原意 | 当前状态、理由与限制 |
|---|---|
| [README](../../../.repo/pi-1.2.0/README.md#L69) 的包列表与通用平台说明 | 概览只列部分库，实际完整包集由根workspaces、11个一级manifest与清单发现。README 没列 env/mcp/codemode/evals 不表示不存在；五个private extension workspace归产品示例 |
| [CONTRIBUTING](../../../.repo/pi-1.2.0/CONTRIBUTING.md#L5) minimal core / extension-first | 文档意图与当前 Agent Core 不拥有磁盘/产品配置相符；具体 hook/工具是否可用仍以exports/产品注册为准，不从原则推出任意扩展能力 |
| [SECURITY](../../../.repo/pi-1.2.0/SECURITY.md#L6) 同一用户可写文件处于信任边界 | 当前设计边界；根文档明确要求宿主自行容器化/隔离。没有证据证明默认Agent提供文件/进程/网络逐项授权；VM执行代码也不自动约束宿主工具权限 |
| [README 的 RFC 导航](../../../.repo/pi-1.2.0/README.md#L105)、[贡献说明末尾](../../../.repo/pi-1.2.0/CONTRIBUTING.md#L99) | 站外方向存在，但本轮未取得外部RFC正文，不能声称完整覆盖上游路线图或给未来时间承诺 |
| [.pi/skills/release.md](../../../.repo/pi-1.2.0/.pi/skills/release.md#L8) lockstep、离线外目录安装/真实prompt smoke、发布恢复 | 文档意图与当前release/artifact/publish/CI链交叉核查；local-release共用tarballs、trusted publishing与公告顺序有静态接线。真实provider smoke是作者要求，本轮没有执行，不能借文档写“发布已验证” |
| [.pi/skills/add-llm-provider.md](../../../.repo/pi-1.2.0/.pi/skills/add-llm-provider.md#L13) provider接入清单 | 局部陈旧：仍指向 register-builtins.ts、旧provider直接stream函数接入方式；当前provider与API分离、Models显式装配，调用方的真实新增方式见ai报告。清单不是当前exports的证明 |
| [.pi/prompts/cl.md](../../../.repo/pi-1.2.0/.pi/prompts/cl.md#L4) 与 [AGENTS.md](../../../.repo/pi-1.2.0/AGENTS.md#L1) 开发/版本流程 | 工作流提示资源，不是产品runtime；其命令没有在本次分析执行。CONTRIBUTING要求贡献者不改changelog，maintainer发布prompt要求维护者补记录，角色语境需区分 |
| 其余 .pi prompts、interactive-testing 与各类别提示 | 均全文阅读分类。审查/安全公告/commit/push/tmux指令是研究对象，不授权本次联网或写操作，不视为产品未来规划 |
| [scripts/issues/README.md](../../../.repo/pi-1.2.0/scripts/issues/README.md#L1) Durable triage与重启恢复 | 有真实实现：triage.ts创建Models/Registry/SQLite/Harness，session doc保存结果/人工决定，每issue conversation以requestId提交，open后resume；对GitHub的决定仅本地存储。未启动该工具或核验远端同步结果 |
| [scripts/issues/prompt.md](../../../.repo/pi-1.2.0/scripts/issues/prompt.md#L10) 与15份categories | triage的分类/优先级策略与责任提示，部分提到仓库外旧产品或issues；不是已接纳需求/路线图。类别例子中的issue编号没有在本次抓取核实 |

构建发布证据补充：[package-artifacts.mjs](../../../.repo/pi-1.2.0/scripts/package-artifacts.mjs#L81)、[local-release](../../../.repo/pi-1.2.0/scripts/local-release.mjs#L95)、[publish](../../../.repo/pi-1.2.0/scripts/publish.mjs#L31)、[CI](../../../.repo/pi-1.2.0/.github/workflows/ci.yml#L15)。静态测试源码也明确限制通用consumer smoke只验证literal exports和Node ESM，不能外推wildcard/source/Cloudflare条件全部可运行。[consumer校验范围](../../../.repo/pi-1.2.0/scripts/local-package-install.mjs#L49)。

## 逐包核查

下列各组给出每包主张、实现/测试定位与状态；完整状态归属和执行链见各包独立报告。

## ai：Provider、转录与多操作目录

全文审读 README 1–1949、CHANGELOG 1–2310；后者早期历史以条目完整文本阅读，部分显示时省略重复链接目标。没有独立规划文件。结论源于当前源码与测试交叉，不以文档哈希未变推断仍然正确。

| 方向/动机与文档证据 | 当前实现/测试证据 | 状态与限制 |
|---|---|---|
| 显式 Models/Provider 与选择性 bundle，compat 最终删除：[README:1665](../../../.repo/pi-1.2.0/packages/ai/README.md#L1665)、[1831](../../../.repo/pi-1.2.0/packages/ai/README.md#L1831) | [index](../../../.repo/pi-1.2.0/packages/ai/src/index.ts#L1)、[Models](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L395)、package exports | 集合/分层已兑现；compat 仍存在，“未来删除”没有具体版本时间表，不能写成本版移除 |
| System/tool transcript 保持历史与缓存 prefix：[README:1570](../../../.repo/pi-1.2.0/packages/ai/README.md#L1570) | [replay/collapse](../../../.repo/pi-1.2.0/packages/ai/src/utils/transcript.ts#L57)、[agent 声明 delta](../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L323) | 已兑现；native/additive/collapse 路径按模型能力选择，不保证全部跨模型语义无损 |
| chat/image/classifier 共享 provider/auth/catalog：[README:790](../../../.repo/pi-1.2.0/packages/ai/README.md#L790)、[891](../../../.repo/pi-1.2.0/packages/ai/README.md#L891) | [operation guards](../../../.repo/pi-1.2.0/packages/ai/src/utils/model-operations.ts#L17)、[dispatch](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L960) | 已兑现；System One/llama-cpp 明确拒绝 classifier images，公共输入类型比具体 adapter 宽 |
| dynamic publication、防止旧 refresh 覆盖、caller cancellation：[README:1306](../../../.repo/pi-1.2.0/packages/ai/README.md#L1306) | [generation/publication](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L491)、models-runtime.test.ts | 已兑现；custom provider/store 仍须 honor signal/commit contract，取消返回不证明底层 work 停止 |
| provider-owned auth、serial refresh 与 rotated token 保全：[README:370](../../../.repo/pi-1.2.0/packages/ai/README.md#L370)、[CHANGELOG:67](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L67) | [resolve/15s refresh](../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L102) | 已兑现；store 跨进程锁由注入实现保证，caller cancel 不丢已启动 refresh 的持久结果 |
| deferred handle、fetch/cancel：[CHANGELOG:438](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L438) | [handle](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L519)、[Models](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L924)、faux-provider.test.ts | 可选机制已实现；未发现通用 durable scheduler、所有真实 adapter 的普遍支持证据，README stop list 未列 deferred |
| 1.1 response/tool duration、Decisions、多模态分类：[CHANGELOG:17](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L17) | [stream timing](../../../.repo/pi-1.2.0/packages/ai/src/utils/event-stream.ts#L91)、[Decisions](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-decisions.ts#L160)、[tests](../../../.repo/pi-1.2.0/packages/ai/test/openai-decisions.test.ts#L80) | 已兑现；duration 保留内层测量，deferred 旧结果不补计时；504 大输入阈值是作者历史观察，非本次测量 |
| 1.2 OpenRouter personalized catalog、baseUrl override：[CHANGELOG:7](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L7) | [helper](../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter.ts#L24)、[caller](../../../.repo/pi-1.2.0/packages/coding-agent/src/core/model-runtime.ts#L234)、[tests](../../../.repo/pi-1.2.0/packages/ai/test/openrouter-user-models.test.ts#L63) | 已兑现显式 helper；普通 openrouterProvider 静态，不自动个性化；401/403/404 退回原目录，image/classifier 保持 |
| strict prefer 降级：[CHANGELOG:14](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L14) | [request fallback](../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L639)、[tests](../../../.repo/pi-1.2.0/packages/ai/test/anthropic-strict-tool-fallback.test.ts#L128) | 已兑现单次 pre-response fallback，保留 require；不是 SSE 中途自动重放，也不是永久关 strict |
| Mistral headers-only timeout：[CHANGELOG:12](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L12) | [requestMistralStream](../../../.repo/pi-1.2.0/packages/ai/src/api/mistral-conversations.ts#L298) | 已兑现；固定 timeout 不再截断长 body stream，body stalls 未因此获得相同 deadline |
| pricing tiers 与负 placeholder：[CHANGELOG:44](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L44)、[13](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L13) | [pricing](../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter-api.ts#L35)、[calculateCost](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1206)、model-cost-tiers.test.ts | 已兑现；time-of-day 不在 ModelCost 能力，明确忽略，catalog estimate 不能视为账单 |

具体陈旧主张：Radius overlay（README 368）已由 replacement（radius.ts 30–42）替代；非 vision 图片 silently ignored（README 756）实际转占位（transform-messages.ts 12–57）；cross-provider `<thinking>`/same-provider-api unchanged（README 1532–1537）实际按完整模型 identity 处理、转纯文本并跳过失败 assistant（transform-messages.ts 92–116/195–202）；Azure Responses only（README 1827）被 1.0.3 两 API provider 取代（CHANGELOG 55–63、providers/azure.ts）。这些是文档修正项，不是尚未实现功能。

历史演化：0.25/0.18 的 SDK retry 开关、0.56 Mistral SDK、旧 Codex 默认 SSE、Google CLI/Antigravity、旧全局 auth/ImagesModels 均有后续替代；不拼接为当前设计。ignored model-data JSON 缺失使 catalog 数据和离线 build 不能静态完成验证；本次未下载 hydrate。

## agent：循环生命周期与宿主控制

全文审读 README 1–564、CHANGELOG 1–780，没有独立路线图。

| 方向/原意 | 当前证据与状态 |
|---|---|
| 0.84.0 v4 session/harness/telemetry 入默认 API，scaffold 未完 | [CHANGELOG:23–27](../../../.repo/pi-1.2.0/packages/agent/CHANGELOG.md#L23) 明确 1.0.0 删除；[index](../../../.repo/pi-1.2.0/packages/agent/src/index.ts#L1) 只剩 Agent/loop/proxy/types。已被收缩替代，旧 HarnessNotImplemented 不是当前包待办 |
| prepareRequest/finishTurn 替换 shouldStopAfterTurn，保留调度可预测性 | [README:124](../../../.repo/pi-1.2.0/packages/agent/README.md#L124)、[loop:219](../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L219)、[245](../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L245)、agent-loop tests 1108/1404；已兑现。error/aborted 调用 hook 但决策不改变硬退出，continue 只要求一下一轮 |
| parallel ordering 与 steering | [README:109](../../../.repo/pi-1.2.0/packages/agent/README.md#L109)、[parallel:586](../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L586)、tests 668/762；已兑现 completion-order end/source-order messages，steering 等完整批次。0.32.0 “跳过剩余 tools”、0.68.0 全 artifact completion-order 已被后续修正替代 |
| async listener 是 barrier、agent_end 非 idle | [README:200](../../../.repo/pi-1.2.0/packages/agent/README.md#L200)、[Agent:558](../../../.repo/pi-1.2.0/packages/agent/src/agent.ts#L558)、agent.test.ts 361/399；已兑现。raw agentLoop stream 不等消费者处理，是文档明确职责边界 |
| 1.1 durationMs，排除 hooks、未执行无值 | [CHANGELOG:9](../../../.repo/pi-1.2.0/packages/agent/CHANGELOG.md#L9)、[execute timing:821](../../../.repo/pi-1.2.0/packages/agent/src/agent-loop.ts#L821)、[test:402](../../../.repo/pi-1.2.0/packages/agent/test/agent-loop.test.ts#L402)；已兑现。proxy 同时换专用 ai stream class |

README 45–49 对 LLM roles 的枚举和 541 的 low-level filter 排除 system，局部陈旧；当前 default converter 包含 system（agent.ts 38–45），照抄旧例会丢提示/工具声明。README 488–502 只教 throw 表达工具失败，但 result.isError 当前也支持，属于覆盖不足；不能据此判 isError 实现错误。持久化、执行环境、权限 UI、tool scheduling policy 不属于本包，不能从 hooks 扩展点推造新路线图。

## mcp：独立客户端与可选 OAuth

全文审读 README 1–136、CHANGELOG 1–73、MIT license 1–21；无独立规划文件。

| 方向/原意 | 当前证据与状态 |
|---|---|
| 小型独立 client、无官方 SDK runtime 依赖、不接管 browser/credential file | [README:3](../../../.repo/pi-1.2.0/packages/mcp/README.md#L3)、[110](../../../.repo/pi-1.2.0/packages/mcp/README.md#L110)、manifest；已兑现。OAuth 明确改编 SDK v1.29.0 并保留 license，与“无依赖”不冲突 |
| session/GET reconnect/Last-Event-ID、progress/cancel | [README:114](../../../.repo/pi-1.2.0/packages/mcp/README.md#L114)、[client:396](../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L396)、[HTTP:377](../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L377)；已兑现，流 resume 不是非幂等 POST replay；progress 刷新 inactivity timeout |
| PKCE/DCR/shared refresh/step-up/CIMD | [flow:320](../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L320)、[446](../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L446)、oauth.test.ts；已兑现。CIMD 不持久化成 registration，scope union 防丢旧授权 |
| 1.1 signal 与 close token、1.2 issuer/late-dispatch fix | [CHANGELOG:3](../../../.repo/pi-1.2.0/packages/mcp/CHANGELOG.md#L3)、[flow issuer:383](../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L383)、[HTTP auth:287](../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L287)、[client settlement:577](../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L577)；已兑现。send signal 只阻止迟到派发，不撤回已送请求，不 abort 共享 refresh |
| structuredContent 可为任意 JSON | [CHANGELOG:7](../../../.repo/pi-1.2.0/packages/mcp/CHANGELOG.md#L7)、[content:65](../../../.repo/pi-1.2.0/packages/mcp/src/protocol/content.ts#L65)、[client tests:284](../../../.repo/pi-1.2.0/packages/mcp/test/client.test.ts#L284)；已兑现，协商版本仍2025-11-25，不代表全支持2026协议 |
| batch、legacy HTTP+SSE、server、sampling、tasks 非 initial core | [README:132](../../../.repo/pi-1.2.0/packages/mcp/README.md#L132)；当前无完整内建 surface，非目标。HTTP 接受 response array 只是容忍路径，不能等同完整 batch requests |

局部文档不足/陈旧：supported surface 未列当前 resources/list/templates/read（client.ts 300–339）；OAuth 示例仅取 code 未转发 callback.iss（README 94–104），也未关闭 callback server，不能当完整安全/清理示例。stdio whole-process-group 的跨平台意图在 Windows 实际为 taskkill /T /F（stdio.ts 17–40），非 POSIX graceful signal。

## telemetry：被动诊断与编译期 schema

全文审读 README 1–464、CHANGELOG 1–47；无独立规划文件。1.0.4→1.2.0 只有 manifest/changelog 变化，代码/测试/README 哈希相同，本次仍重读关键语义。

| 方向/原意 | 当前证据与状态 |
|---|---|
| callback settlement owns span、无 exporter/ambient context/持久业务状态 | [README:103](../../../.repo/pi-1.2.0/packages/telemetry/README.md#L103)、[118](../../../.repo/pi-1.2.0/packages/telemetry/README.md#L118)、[memory:120](../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L120)；已兑现，adapter 拥有采样、flush、exporter |
| NOOP 与参考 memory 保持业务结果、同步接纳、记录失败被动抑制 | [noop](../../../.repo/pi-1.2.0/packages/telemetry/src/noop.ts#L1)、[memory](../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L89)、[conformance](../../../.repo/pi-1.2.0/packages/telemetry/src/testing/conformance.ts#L60)；已兑现。memory 无界且无时间戳，settled 后子工作退化 NOOP但执行 callback |
| serializable schema、exact attributes、multi-schema child starter | [README:211](../../../.repo/pi-1.2.0/packages/telemetry/README.md#L211)、[index identity:71](../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L71)、[binder:324](../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L324)；编译期能力已兑现；runtime 不读 schema、不强制 parent/sensitive/value policy，是明确边界 |
| agent-core owns并导出 AI/harness schemas | [README:323](../../../.repo/pi-1.2.0/packages/telemetry/README.md#L323)、[365](../../../.repo/pi-1.2.0/packages/telemetry/README.md#L365)；局部陈旧。agent CHANGELOG 1.0.0 明确删除，当前 index/types 无导出，全 packages 符号搜索只命中这份 README。不能列成当前 API或未来计划 |

安全说明要求 context/span 不进入持久 messages/snapshots/deferred handles（README385–391），已与运行时对象性质一致；无自动脱敏机制，memory 的 Error name/message 仍由调用者的数据策略决定。CHANGELOG0.84.0功能已对应源码与测试，后续空 release headings 不意味着新功能承诺。

## durable

当前文档入口：[README](../../../.repo/pi-1.2.0/packages/durable/README.md)、[spec](../../../.repo/pi-1.2.0/packages/durable/docs/spec.md)、[CHANGELOG](../../../.repo/pi-1.2.0/packages/durable/CHANGELOG.md)；完整实现、失败与生命周期见[durable 包报告](packages/durable.md)。本节短路径相对当前 `packages/durable/`，其他包或根脚本另注。

| 方向 | 状态 | 文档/实现证据与限制 |
|---|---|---|
| committed-only authority/effect sandwich | 已实现 | docs/spec.md:46,1577；src/session/session.ts:690 prepare→Storage→adopt→publish |
| typed scopes/history/fork/checkpoint/migration | 已实现 | docs/spec.md:971-1576；src/session/transaction.ts、forks.ts；不自动sweep第三方文档、不自动checkpoint |
| structured concurrency/cancellation/wait-cycle | 已实现 | spec:1977,2161；scheduler.ts:1122,1473；memory-only wait不入持久图 |
| missing task blocks、restart abandonment | 已实现 | spec:2075-2160；scheduler.ts:1122-1237；restart mark等缺失定义而不orphan，普通显式abort可orphan |
| nested calls/result channels/deferred tools | 已实现 | spec:2796-3418；tool.ts:84,114,360,578,673；ToolTask v2，structuredOutput无统一size budget |
| deterministic codemode journal | 已实现 | spec:3588-3705；codemode/codemode.ts:83,402,610,956；先journal后deliver，mismatch/deadline/divergence终止，无外部exactly-once保证 |
| fatal storage errors / lifecycle closed | 已实现 | spec:5036；session.ts:1054；harness-storage-failure.test.ts:86；read-only StorageRequestError/取消豁免，commit/mint绝不豁免 |
| context/document/index retention | 已实现 | spec:1577、changelog:49起；session.ts:598,663与scheduler live indexes；不等同持久历史被清除 |
| observer convergence / events | 已实现/非审计日志 | spec:4371-4836；100 pending frame/batch，overflow snapshot；callbacks不join |
| SQLite/Cloudflare/JSONL | 已实现但durability有限 | spec:5103-5181；sqlite/node.ts:190 NORMAL；jsonl/storage.ts:278,508；fsync侧文件不保证main最近ack断电存活 |
| SystemMessage PR尚未land | 局部陈旧 | spec:37保留历史措辞；generation/prompt现已使用 |
| SubmissionDraft input entry?:never | 局部陈旧 | spec:303-315旧形状；spec:2291-2296、README:426起、submissions.ts:151已支持InputEntry |
| fault-write失败同进程重跑 | 已被替代的局部陈旧 | spec:1903附近旧叙述；spec:5036 fatal rule、CHANGELOG:1.2、session GuardedStorage与scheduler自己commit失败会关闭Session |
| 自动checkpoint/CRDT/multiwriter/强停扩展 | 非目标 | spec:5326-5340；不是待开发承诺 |
| 新后续handoff | 当前清单无独立文件 | README/CHANGELOG/spec是本版仅三份默认文本；旧设计/handoff不覆盖当前规范 |

## env

当前文档入口：[README](../../../.repo/pi-1.2.0/packages/env/README.md)、[protocol](../../../.repo/pi-1.2.0/packages/env/docs/protocol.md)、[semantics](../../../.repo/pi-1.2.0/packages/env/docs/semantics.md)；完整实现、失败与生命周期见[env 包报告](packages/env.md)。本节短路径相对当前 `packages/env/`，其他包或根脚本另注。

| 方向 | 状态 | 文档/实现证据与限制 |
|---|---|---|
| Node语义远端ExecutionEnv | 已实现 | README:3、semantics全文；remote-env.ts:354；结果不保证跨平台错误文案相同 |
| lazy/strict SSH/content hash deploy | 已实现 | README:20-34；ssh.ts:73,400,490；hash是内容校验不是上游签名 |
| protocol1/session handles/liveness | 已实现 | protocol:3-76；connection.ts:203,268,335；lost mutation结果未知，不重放 |
| output window/control priority/backpressure | 已实现但非总预算 | protocol:54-60,119起；output.rs:13；main.rs:345 job与PTY input为无界队列 |
| watcher reconnect+overflow | 已实现 | semantics:62-68；watch.ts:19；缺失期间事件不逐条恢复 |
| listFiles/LocalExecutionEnv/features | 已实现 | changelog:7-9、protocol:101；list-files.ts:21,80、list.rs:85；NodeExecutionEnv本身无该method |
| PTY foreground/raw/input/resize | Unix已实现 | README:53-59、protocol:134起；terminal.ts:26,93；pty.rs:145 Windows ENOTSUP，无近期交付时间证据 |
| daemon原生构建发布 | 已实现的资产管线/路径前置有限 | package build与build:daemon分开；ci.yml:42、build-binaries.yml:370准备；release.mjs:256和local-release.mjs:100链未显式准备，test.sh不透传PI_ENV_DAEMON |
| default coding-agent wiring | 未发现 | 当前packages生产及相关scripts未见导入；不把包/README当产品已启用证据 |
| 新后续handoff | 当前无独立文件 | 仅README/CHANGELOG/protocol/semantics四份默认文本 |

## chord

当前文档入口：[spec](../../../.repo/pi-1.2.0/packages/chord/docs/spec.md)、[guide](../../../.repo/pi-1.2.0/packages/chord/docs/guide.md)、[tutorial](../../../.repo/pi-1.2.0/packages/chord/docs/tutorial.md)；完整实现、失败与生命周期见[chord 包报告](packages/chord.md)。本节短路径相对当前 `packages/chord/`，其他包或根脚本另注。

README/CHANGELOG/spec261/guide750/tutorial909/delta README193均全文新读；现行guide/tutorial结尾的否定条款也已读，未将示例当部署。

| 方向 | 状态 | 文档/实现证据与限制 |
|---|---|---|
| typed hosts/contracts/objects/handles替代旧runtime | 已实现 | CHANGELOG:7-10、spec:10-73；src/index.ts、host/contract/record；旧client/protocol/server移除 |
| state/resource/hold/listener/task声明生命周期 | 已实现 | spec:74-171、guide:161-232、tutorial:208-320；record.ts:819,905,1226；非持久存储 |
| principal stamping/variants/revocation | 已实现 | spec:154、guide:317-373、tutorial:321-463；connection.ts:381,490；variant不是完整授权 |
| startup eager readiness/two-stage admission | 已实现 | spec:134-153、guide:271、tutorial:589起；host.ts:273；readiness.test.ts:197 |
| state/contract/resource三个版本 | 已实现 | spec:58,179、tutorial:614-762；reload.ts:412,514；meaning变化靠作者bump |
| two-version + migration gate + cluster votes | 已实现 | spec:172-217；reload.ts、cluster.ts:39；无持久分布式commit |
| parent retirement coordination | 部分实现 | spec:231；仍可能worker旧root到已退休server时superseded |
| persisted commit decision | 未实现方向 | spec:232；cluster.ts内存decision，server先commit，未持久化 |
| recursive multi-level participants | 未实现方向 | spec:233；cluster.ts:44只直接downstream |
| module list after coordinator restart | 未实现方向 | spec:234；cluster.ts:130依赖内存host.remoteModules |
| resumable link holds | 未实现方向 | spec:235；断连释放，新hold新record，flight effect未知 |
| structured auth refusal to client | 未实现方向 | spec:236；WebSocket closes，不携结构化失败 |
| isolated/async publication queues | 未实现方向 | spec:237；publish.ts:1全进程同步FIFO |
| automatic restart | 核心非目标/manager可提供策略 | spec:238；manager.ts:230 keepAlive/fatal，不是core自动修复 |
| isolating evicting loader | 尚未实现的明确TODO | spec:239；loader.ts:3,18,55动态query副本保留 |
| stale lock race | 已知限制 | spec:240；manager.ts:82原生socket/pipe锁，非Linux Unix probe竞争未消除 |
| listener select captured-values detection | 已知限制 | spec:241；身份或源文本比较，捕获变化不可见 |
| close over lost remote link | 已知限制 | spec:242；需要调用方bound signal，否则等reconnect |
| single state per object | 已否决 | spec:261保留named cells/derivations |
| Durable registry bridge | 应用职责/当前未附带 | spec:208明确does not ship；guide:690-718示例，不是默认CLI接线 |
| 持久化/load/save | 应用职责 | tutorial:830附近明确cells在dispose/restart后丢；linger不等于persistence（:312） |

## codemode

当前文档入口：[README](../../../.repo/pi-1.2.0/packages/codemode/README.md)、[CHANGELOG](../../../.repo/pi-1.2.0/packages/codemode/CHANGELOG.md)；完整实现、失败与生命周期见[codemode 包报告](packages/codemode.md)。本节短路径相对当前 `packages/codemode/`，其他包或根脚本另注。

| 方向 | 状态 | 文档/实现证据与限制 |
|---|---|---|
| Node worker VM/capability injection | 已实现 | README:15-61、337起；runtime/host.ts:12；工具权限由宿主 |
| Inline/Remote/Cloudflare separate Worker | 已实现 | README:104-230、examples/cloudflare；inline.ts/remote.ts；CPU同步时timer不能抢占，remote下一exchange或budget才停 |
| deterministic start/time/random/UTC/seq/fingerprint | 已实现 | README:296-325；execution.ts:530、vm.ts:23,316；不自行持久化journal |
| Durable resumable adapter | 已实现于调用包 | durable/codemode/codemode.ts:402,956；本包只提供机制 |
| output/store limits | 已实现但非总预算 | README:62-77,327-335；prelude-source常量；无统一tool concurrency/result byte cap |
| schema declarations及MCP特判删除 | 已实现/旧API已替代 | CHANGELOG:7,17；README:232；schema仅提示，不runtime validation |
| 首行options/microtask interrupt/watch bridge | 已实现 | CHANGELOG:23-29；source.ts/host.ts/vm.ts；未执行回归 |
| measured Cloudflare delay/poll speed | 历史实验 | README:122起、191起；作者October2026，不是本次测量 |
| store durability和外部effect幂等 | 宿主职责 | README:62；execution只返回storeWrites、pending calls仅signal abort不join |

## evals

当前文档入口：[README](../../../.repo/pi-1.2.0/packages/evals/README.md)；完整实现、失败与生命周期见[evals 包报告](packages/evals.md)。本节短路径相对当前 `packages/evals/`，其他包或根脚本另注。

| 方向 | 状态 | 文档/实现证据与限制 |
|---|---|---|
| host/docs区分、preplanned paired cohorts | 已实现 | README:5-80；cli.ts:24,153,171、plan.ts:21 |
| clean current-runtime consumer install | 已实现的构建说明 | README:72,84；Dockerfile:1、install-runtime.mjs；本次未运行 |
| fresh container/root-only judge/permanent UID drop | 已实现的机制 | README:84；docker.ts:91、entrypoint.ts:27、harness.ts:154；非实际隔离验收 |
| with/without docs treatment | 已实现 | README:82-90、harness transforms；依赖docs对称去除，校验真实prompt |
| exact pairs/missing telemetry withholding | 已实现 | README:105起；report.ts:249,283,359；比较测试:62 |
| resume interrupted cohort | 未见实现，不是文档承诺 | CLI每次新artifact id，顺序跑计划；没有从旧protocol恢复入口 |
| network confinement/单轮稳定性 | 明确限制 | README:90、107；provider需要network，一轮不能证明稳定 |
| coder runtime credentials/session-services升级 | 受依赖影响 | manifest ^1.2.0、harness.ts:8,317,342；同字节runner不保证旧结果成立 |
| 独立规划/handoff | 无 | README为唯一默认文本资料，未自造路线图 |

## coding-agent

本包75份默认文本都有本次实际审读记录；主要生命周期/边界文档全文阅读，较长CLI/设置/平台/协议参考及示例索引按reading.json精确选段，CHANGELOG仅1–120行近版范围。fixture按测试输入解释，不执行其指令。未把清单枚举称为全文审计；没有联网、安装、构建或执行Pi测试。

| 主张及动机 | 当前状态 | 文档/代码/测试证据与限制 |
|---|---|---|
| CLI/interactive/print/JSON/RPC/SDK复用会话机制 | 已实现 | [how-pi-works:33](../../../.repo/pi-1.2.0/packages/coding-agent/docs/how-pi-works.md#L33)；[main:948](../../../.repo/pi-1.2.0/packages/coding-agent/src/main.ts#L948)分派同runtime；[sdk:191](../../../.repo/pi-1.2.0/packages/coding-agent/src/core/sdk.ts#L191)创建AgentSession；SDK不自动复制CLI builtin工厂 |
| 原未发布server/client/plugin durable栈 | 已移除 | [CHANGELOG:63](../../../.repo/pi-1.2.0/packages/coding-agent/CHANGELOG.md#L63)；当前manifest exports仅根/rpc-entry，src/experimental只有source-resolver；生产src无Durable/env/Chord导入。独立durable/env存在不等于接回产品 |
| PI_EXPERIMENTAL first-run theme/analytics选择 | 仍存在；选择保存已实现，analytics发送未确认 | [main:674](../../../.repo/pi-1.2.0/packages/coding-agent/src/main.ts#L674)；[startup-ui:130](../../../.repo/pi-1.2.0/packages/coding-agent/src/cli/startup-ui.ts#L130)仅官方发行、无agent-dir override、settings不存在；[保存:182](../../../.repo/pi-1.2.0/packages/coding-agent/src/cli/startup-ui.ts#L182)，footer:213有xp；搜索未找到getEnableAnalytics生产读取调用方。不是旧服务栈 |
| 最小harness，不默认内置subagent/plan mode | 已兑现的边界 | [README:15](../../../.repo/pi-1.2.0/packages/coding-agent/README.md#L15)；[subagent:55](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/README.md#L55)、[plan:37](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/plan-mode/README.md#L37)为显式加载例子，前者独立进程，后者保留其他extension工具，不能充当OS沙箱 |
| 原始历史与模型context分离，压缩不删历史 | 已实现，基线已有 | [session-format:139](../../../.repo/pi-1.2.0/packages/coding-agent/docs/session-format.md#L139)；[projection:476](../../../.repo/pi-1.2.0/packages/coding-agent/src/core/session-manager.ts#L476)；[测试:36](../../../.repo/pi-1.2.0/packages/coding-agent/test/session-context-edit.test.ts#L36)验证raw仍有三message而projection只user。session-manager、格式文档与该test哈希和1.0.4相同，本次重新阅读 |
| 最终恢复/队列结束看agent_settled，启动handler期间busy | 前者沿用；后者1.2.0强化 | [extensions:62](../../../.repo/pi-1.2.0/packages/coding-agent/docs/extensions.md#L62)、[自定义触发:104](../../../.repo/pi-1.2.0/packages/coding-agent/docs/extensions.md#L104)；[_startRun:1832](../../../.repo/pi-1.2.0/packages/coding-agent/src/core/agent-session.ts#L1832)；[启动并发/abort测试:615](../../../.repo/pi-1.2.0/packages/coding-agent/test/suite/agent-session-prompt.test.ts#L615) |
| overflow/length恢复保持审计原文 | 已实现，基线已有 | [compaction:85](../../../.repo/pi-1.2.0/packages/coding-agent/docs/compaction.md#L85)；[选择:2972](../../../.repo/pi-1.2.0/packages/coding-agent/src/core/agent-session.ts#L2972)，先omit再压缩、失败保留omit但无内部retry；[compaction测试:442](../../../.repo/pi-1.2.0/packages/coding-agent/test/suite/agent-session-compaction.test.ts#L442)；不能保证所有length可恢复 |
| 配置schemas与主题启动/reload一致 | 已实现 | [settings:5](../../../.repo/pi-1.2.0/packages/coding-agent/docs/settings.md#L5)；四份schemas及类型生成脚本；[config-schemas tests:33](../../../.repo/pi-1.2.0/packages/coding-agent/test/config-schemas.test.ts#L33)。未知settings可保留，不是统一strict schema拒绝 |
| MCP后台连接不被resource listing拖住，reachesTools按需等 | 已实现 | [mcp:100](../../../.repo/pi-1.2.0/packages/coding-agent/docs/mcp.md#L100)、[exposure:183](../../../.repo/pi-1.2.0/packages/coding-agent/docs/extensions.md#L183)；[mcp/index:387](../../../.repo/pi-1.2.0/packages/coding-agent/src/extensions/mcp/index.ts#L387)。资源能力不再计数、直接工具等待最多10秒；MCP conformance仍有baseline失败/不支持2026-07-28，不称全协议验收 |
| generated images是否落盘 | 局部文档措辞不一致 | [models:185](../../../.repo/pi-1.2.0/packages/coding-agent/docs/models.md#L185)说不落盘；[codemode:209](../../../.repo/pi-1.2.0/packages/coding-agent/docs/codemode.md#L209)和tool说明image()落盘。应区分generateImages返回值与image输出动作，不能否认后者落盘 |
| trust/工作目录/QuickJS等于OS isolation | 明确不成立的边界 | [security:3](../../../.repo/pi-1.2.0/packages/coding-agent/docs/security.md#L3)、[trust:31](../../../.repo/pi-1.2.0/packages/coding-agent/docs/security.md#L31)；[codemode:7](../../../.repo/pi-1.2.0/packages/coding-agent/docs/codemode.md#L7)。受控资源信任、sessionDir先读、context独立加载与工具权限分别解释 |

未发现包内独立现行PLANNING/TODO路线图；作者意图主要来自现行行为说明、边界和发行历史。静态剩余风险：非事务JSONL append及磁盘失败后的内存/磁盘分歧；RPC无LF缓冲/命令并发无统一cap；OutputAccumulator不按WriteStream.write返回值暂停子进程；同进程extension有宿主权限。测试影响`none`，没有改产品测试。

## tui

本包5份默认文本都有本次真实记录：949行README及三个native README全文，1285行CHANGELOG只读1–155行。没有执行Node tests、native build、剪贴板操作或真实终端视觉验证。

| 主张及动机 | 当前状态 | 文档/代码/测试证据与限制 |
|---|---|---|
| 两renderer、持久组件+短存frame geometry | 已实现，基线已有 | [README:119](../../../.repo/pi-1.2.0/packages/tui/README.md#L119)；[layout:379](../../../.repo/pi-1.2.0/packages/tui/src/layout.ts#L379)、[写后提交:1779](../../../.repo/pi-1.2.0/packages/tui/src/tui-alt-screen.ts#L1779)。layout/ScrollView与基线哈希相同，调用方仍重新核查 |
| fixed dock与独立滚动，scrollbars/屏外拖选 | 已实现；旧延期已被替代 | [README:751](../../../.repo/pi-1.2.0/packages/tui/README.md#L751)；[宿主chat viewport:22](../../../.repo/pi-1.2.0/packages/coding-agent/src/modes/interactive/chat-viewport.ts#L22)；[固定dock测试:231](../../../.repo/pi-1.2.0/packages/tui/test/tui-alt-screen.test.ts#L231) |
| overscroll contain阻止传播 | 部分实现，保留静态偏差 | [routeWheel:993](../../../.repo/pi-1.2.0/packages/tui/src/tui-alt-screen.ts#L993)break后remaining仍可交未seen primary；[ScrollView:148](../../../.repo/pi-1.2.0/packages/tui/src/components/scroll-view.ts#L148)返回剩余；[test:532](../../../.repo/pi-1.2.0/packages/tui/test/tui-alt-screen.test.ts#L532)只验证chain。未运行反例 |
| overlay内独立constrained树、virtualization、active-pane键盘策略 | 历史方向，无完整当前实现/承诺证据 | [历史计划证据](evidence/pi-1.0.2/README.md#tui-plan)；[overlay:1373](../../../.repo/pi-1.2.0/packages/tui/src/tui.ts#L1373)仍render(width)合成；[layout:135](../../../.repo/pi-1.2.0/packages/tui/src/layout.ts#L135)保完整scrollContentLines。裁剪不等于virtualization；不将CSS/grid非目标补为待办 |
| focused硬件cursor不再同时画假cursor | 1.2.0已实现/公开breaking | [CHANGELOG:5](../../../.repo/pi-1.2.0/packages/tui/CHANGELOG.md#L5)；[markers:196](../../../.repo/pi-1.2.0/packages/tui/src/tui.ts#L196)、[resolve:1473](../../../.repo/pi-1.2.0/packages/tui/src/tui.ts#L1473)，TUI外consumer要strip/处理APC |
| focused overlay不吞无人处理wheel | 1.2.0已实现 | [alt input:705](../../../.repo/pi-1.2.0/packages/tui/src/tui-alt-screen.ts#L705)；[test:1866](../../../.repo/pi-1.2.0/packages/tui/test/tui-alt-screen.test.ts#L1866)。scrolling overlay需handleMouse而非只handleInput |
| fragmented terminal replies不泄漏文本/BEL | 1.2.0已实现 | [StdinBuffer:24](../../../.repo/pi-1.2.0/packages/tui/src/stdin-buffer.ts#L24)；[每个split延迟5秒测试:197](../../../.repo/pi-1.2.0/packages/tui/test/stdin-buffer.test.ts#L197)。未终止已识别reply仍可累计，未见总长度cap |
| ProgramStatus与Terminal新方法 | 1.1.0已实现；README接口示例局部陈旧 | [真实Terminal:78](../../../.repo/pi-1.2.0/packages/tui/src/terminal.ts#L78)、[status:1](../../../.repo/pi-1.2.0/packages/tui/src/program-status.ts#L1)、[test:237](../../../.repo/pi-1.2.0/packages/tui/test/terminal.test.ts#L237)；[README:760](../../../.repo/pi-1.2.0/packages/tui/README.md#L760)漏必需setProgramStatus/drainInput等 |
| 图片格式支持 | 核心机制已实现；README省略转码前提 | [README:684](../../../.repo/pi-1.2.0/packages/tui/README.md#L684)；[Image:13](../../../.repo/pi-1.2.0/packages/tui/src/components/image.ts#L13)要求Kitty非PNG同步transcoder，宿主注册；alt iTerm2主动文本降级 |
| native异步剪贴板、有界等待 | 已实现，外部ABI/预编译等同性未实测 | [native README](../../../.repo/pi-1.2.0/packages/tui/native/linux/README.md#L1)；[X11:9](../../../.repo/pi-1.2.0/packages/tui/native/linux/src/linux-platform-x11.c#L9)50MiB/2秒；[worker:1](../../../.repo/pi-1.2.0/packages/tui/native/linux/src/clipboard-worker.h#L1)3秒等待、超时busy直到后台线程归还，非强杀native阻塞 |

历史计划证据是旧报告分析文字及原文件身份，不是原始tui-plan.md；本次读二手文字并重新核对当前实现，不声称重读删除原文。两renderer写Terminal均非await drain，16ms渲染合并与main输出1Mi字符分块不是完整资源背压/virtualization保证。native `.node`只静态清点和阅读源码/构建路径，没有重建或反汇编验证。测试影响`none`，既有测试保持原样。

## 历史规划、缺口与复核结果

旧清单的14份删除文档路径由[升级报告](version-changes.md)完整保留；这包括旧Chord PLANNING、旧服务包说明和Durable handoff。它们的删除不证明所有方向兑现或撤回；当前新规范与真实调用方才支持现行判断。更早的TUI计划只保留当时分析及精确恢复对象，没有伪造丢失的原文。

独立交叉复核检查了总报告的ai/agent/MCP/telemetry边界，以及另一作者编写的Durable/env/Chord默认接线和重要否定性结论；发现并修正总升级说明把比较工具的通用legacy条件误套为旧manifest缺symlink_policy的表述。两版实际字段一致。产品报告也在复核中纠正了“实验首次设置已删除”的过强说法：PI_EXPERIMENTAL首次UI仍存在，与旧Durable服务栈的移除分开。

剩余证据边界：未抓取站外RFC、没有完整旧源码逐行diff、生成model-data不在快照、外部依赖与native二进制内部未审计、没有运行Pi测试/构建/provider/daemon/实际终端。静态审计完整覆盖包与本地文本清单，不等价于运行验收或逐行审计全部源码；最终机械核验见[验证记录](verification.md)。

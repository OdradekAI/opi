# pi-ai 与 telemetry 独立报告交叉核验

> **非规范性研究，2026-10-04。** 本次只判断固定本地快照及当前已确认候选目标，不联网、不执行模型、构建或测试、不修改原独立研究/代码/测试。两项有边界的独立复核分别回查 ai 与 telemetry；主 Agent 顺序综合。本文不是实施准入，也不把测试源码视为执行通过。新中文复核没有既有英文 counterpart；spec 的证据导航同步更新英中版本，未改变规范性行为。

## 结论与使用方法

完整阅读原 [ai.md（历史恢复）](../pi-1.0.2/evidence/pi-1.0.0/README.md#early-reports) 411 行和 [telemetry.md（历史恢复）](../pi-1.0.2/evidence/pi-1.0.0/README.md#early-reports) 167 行，对照当前 [spec-draft.md](spec-draft.md) 及 [decisions.md](decisions.md) 的 D-19–37（含前置 D-18/历史解释），并回到关键一手源码核验。**可以用这两份报告核查决策；本轮相关主张中没有发现必须重开已确认目标的新实质证据。** 它们不能单独证明全部56条目标正确或已实现：相同快照上的两份静态报告不是两次独立运行实验。

两项 ai 事实表述需要限定或纠正：实际产品 session 的 Models 路径不能由残留 compat fallback 推断；header transform 属请求选项，不属于 auth。第一项已被本目录 P02/M02/设计演进纠正，不是新发现，更不是 D-30/35 错误。telemetry 报告对通用契约、NOOP、schema 与 README 冲突的判断得到回源支持；Rust tracing 不被宣称等价于整个通用 pi-telemetry API。

后续使用顺序：报告主张 → 对应快照源码/测试断言 → 专项演进与消费者反证 → D/AIR → 未来 V。只有影响目标的实质新证据才重开决策；类型存在、文档声明、静态分析一致或提案关闭都不能跳过这条链。原独立报告仍保持纯 pi 研究，本页承载比较限定和纠正，不回写它们。

## 固定证据基线

本轮实际按原 `source-manifest.json` 重算整个快照：**1,848/1,848 文件 SHA-256 匹配，新增/缺失/差异均0**；树摘要仍为 `cc37a8fcc2c2dd97a3953c01a1e85f951984a85a0f5252111627163ce4c71173`。ai 独立复核另确认378/378匹配。这不等于验证发布 tarball、第三方 SDK 内部、运行结果或后续上游计划；先前官方 tag/blob 与历史窗口仍以 [README](README.md) 和 [TE 演进](telemetry-evolution.md) 为准，本轮没有刷新上游状态。

| 输入 | 本轮 SHA-256 |
|---|---|
| ai.md | `84b681314a9eee5df197dbf709a9204df9a4986e32a0c262b61054cfc24387af` |
| telemetry.md | `e7183c807a73d230764df35ca7472c4898fb24dddae15c5967f62f797ad8f2b4` |
| 原 source-manifest.json | `7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f` |

原报告的110/39个源码行号链接均指向现有文件且行号在范围内；这项机械检查只证明导航有效，不证明附近每个主张。当前候选在读取期间补充了导航，以下以稳定 D/AIR/V ID 为主，行号只供本轮定位，不作为目标目录约束。

状态含义：**支持**表示源事实支持目标相关职责或行为；**有意差异**表示目标明确保留/改变 pi 行为且已有决策；**遗漏**表示原报告不足以支撑完整目标，须引用补充一手证据；**冲突**表示报告表述与当前快照不一致；**证据不足**不表示能力不存在或目标自动获批。

## ai 报告与当前目标映射

下表源码路径均相对 `.repo/pi-1.0.0/packages/`；每项给出可复核符号/行号，关键冲突另展开。映射只说明证据关联，不把报告扩写成另一份 spec。

| ID | 原报告主张及定位 | 回核的一手源码/反证 | 当前 D / AIR 映射 | 判定、处置与置信度 |
|---|---|---|---|---|
| RC-AI-01 | `ai.md:9–25,29–50`：根入口/Models 实例、显式 all、compat 全局、provider≠wire；依赖与加载成本有边界 | `ai/src/index.ts:4–8,21–48` 根入口不导入目录/provider；`models.ts:384–397` 实例状态与默认 stores；`providers/all.ts:144–189` 显式 builtin；`compat.ts:1–10,102–127,217` 真实全局状态。`test/models-entry.test.ts:9–57` 设计了轻入口 faux completion，但未运行 | D-21/30/33/37；AIR-001/021/025/045/046/050/052/054 | **支持＋有意差异。** 独立 SDK 及 sole Models 有源事实支撑；Rust 无需复制 npm 子路径/全局 compat。pi root 仍导出 TypeBox（index:1–2），不能把“无目录副作用”夸大为完全无重依赖。只需沿用明确支持范围；不重开目标。高 |
| RC-AI-02 | `ai.md:81–98,243–247,355–359`：模型/操作身份、可扩展 provider/API、自定义实现与目录分离 | `models.ts:824–835,871–900` 用传入完整 model 寻 owning provider，不重新查 catalog；`:989–1046,1063–1075` 自定义实现与路由；`:393–397` 默认 memory stores；`types.ts:732–748` 公共 Context 与下层 TranscriptContext 分开 | D-21/23/30/33；AIR-001/025/045/046/049/052 | **支持＋遗漏。** 原报告未充分说明“完整 custom model 无需目录成员资格”，且 `ai.md:356` 的“调用者…提供 stores”不应解释为 stores 必填。当前目标已明确两点。闭合 operation enum/所选 wire 是候选范围，不声称复刻任意 TS 扩展字符串协议。文档补充即可。高 |
| RC-AI-03 | `ai.md:347–351`：默认 coding-agent SDK 从 compat 导入 streamSimple 并作为默认函数，进而警惕 CLI 未完成实例化 | `coding-agent/src/core/sdk.ts:36–39` 明说兼容**未提供 streamFn 的 extension/低层 loop** fallback；实际 `createAgentSession` 在 `:387–406` 显式装配 `modelRuntime.streamSimple`。`core/model-runtime.ts:24,58–59,715–744` 是实例 Models/provider 路径 | D-23/30/35；AIR-034/045/046/050 | **报告表述冲突/需限定。** compat import/fallback 确实存在，但不能据此判定默认 session 请求仍走 compat global stream。新研究 `pi-contracts-notes.md` P02 已纠正，同目录 M02/设计演进已采用正确结论；只需保留引用限定，不改变已确认目标。高 |
| RC-AI-04 | `ai.md:157–161`：Models 解析 auth、合并请求选项，最后执行“auth 的 header transform” | `ai/src/models.ts:837–868` 的类型约束为 `ProviderRequestOptions & ModelsRequestTransforms`；`:858–866` 调用 `options.transformHeaders`，随后移除这个 Models-only 字段；AI README:406–423 亦明确请求选项归属 | D-27/30/35；AIR-042/046/049，spec:269 延后 dynamic transformHeaders | **局部冲突，仅报告事实精度。** 应称“Models 请求级 header transform，在 auth/model/request headers 合并后执行”。当前 payload transform 与静态 headers 目标不因此等于已交付动态 header callback；该 callback 已显式延后，不重开。高 |
| RC-AI-05 | `ai.md:133–173`：streamSimple 的立即 handle、异步 auth/setup、provider 分发；direct adapter 可同步 throw | `models.ts:871–909` 四个入口，complete 委托 stream.result；`api/lazy.ts:46–58` 调用 setup 即启动异步工作，并封装错误；`types.ts:752–758` 明确 direct simple 的同步 auth throw；`normalizeContext` 发生在 lazy setup 外 | D-30/31/35；AIR-003/045–050 | **支持＋有意差异。** Rust 创建时捕获 owned 输入、首次 poll 才 auth/network 是已声明的 lazy 时序差异；统一 typed preflight/CallFailure 不复制 direct-vs-wrapped throw 分裂。原报告没有完整四入口使用说明，当前 SDK annex 补足；不能把“TS 立即开始”当 Rust 必须私建 executor 的依据。高 |
| RC-AI-06 | `ai.md:175–193`：全事件、live partial、done/error 决定 result，无界队列，空 end 可悬空 | `types.ts:752–783` 是规范事件注释与 union；`utils/event-stream.ts:26–88,91–104` 结果 Promise 独立于迭代，error 也 resolve assistant；`api/lazy.ts:4–21` setup 填零 usage 且总标 error | D-31/Q32（及保留 D-04）；AIR-003–005/024/047/048 | **支持＋已确认有意差异。** Rust Result 保留完整失败消息、unknown usage、immutable event-time partial、64 队列和主动 drain/cached terminal 是目标；不是 pi 相同数据结构。原报告准确暴露 EOF/无界风险，足以支持修正，不能反向要求复制悬空/零值。compact frame helper 另见 RC-AI-17。高 |
| RC-AI-07 | `ai.md:9–10,94–95,214–238`：工具只声明，partial parser≠执行校验≠服务端 strict；plain JSON/TypeBox 两路径 | `utils/validation.ts:302–349` clone→optional-null→convert→check，含 TypeBox marker 分叉；`:332` 对某些 root coercion 失败会返回原 args，是源缺口。`README.md:669–698` 是调用者自己 executeMyTool；`types.ts:715` Tool 仍非执行函数 | D-20/22/28/29/30；AIR-024/040/041/049 | **支持＋有意差异＋遗漏。** plain-JSON corpus、raw/normalized/execution、trusted hook 不复验及最终授权由后续 Agent 证据/决策闭合；ai 报告不能独自证明这些跨包政策。目标修正 root-validation 漏洞并脱敏原参数错误，不复制它。README 的 toolcall_end 即执行示例也不是 Length/Aborted 安全规则；当前完成终态后决定工具执行的限定已有确认。高 |
| RC-AI-08 | `ai.md:106–129`：shorthand 归一、named sections/工具时序、native-or-collapse、签名/ID 转换与缺 result 补偿 | `utils/transcript.ts:30–33,58–119,123–150,226` 确认；`api/transform-messages.ts:95–140,158–185,195 起` 按 provider/API/requested ID 判断、hold 中途 system、补 `No result provided`、跳过 error/aborted。没有 canonical ResultUnavailable/UnknownEffect 或权限语义 | D-19/20/28；AIR-008–013/015–020/037/038/049 | **支持＋明确扩展。** 默认 collapse 是真实行为；Strict 与 projection facts、去歧义配对及 ResultUnavailable/UnknownEffect 是目标的可审计表达。child/v3/commit barrier/compaction 需 TM/TP 的跨包证据，不能从 ai 报告直接推导。既有 D-19–28 已区分，未见重开理由。高 |
| RC-AI-09 | `ai.md:124–131`：跨模型 thinking 降级、缺图片能力占位，避免假定模态支持 | `api/transform-messages.ts:12–56` 对 nonvision user/tool image 用 placeholder，`:100–144` 保留/移除签名。原报告确实描述了当前行为，不等于推荐它 | D-19/20/23（保留 Q7）；AIR-010/013/014/038 | **有意差异。** AIR-014 要求真实发送或 preflight 拒绝，明确不把 placeholder 当支持；opaque provenance/原始 canonical 保留也是更严目标。按已确认 Q7 与后续 replay 决策记录，不把它说成 Rust 语言必然或误判原报告错误。高 |
| RC-AI-10 | `ai.md:241–273`：静态+动态目录、provider 并发/单 provider publication 串行、先持久化后内存、三种可用性不同 | `models.ts:393–397,424–431,480–637,682–732,1049–1061` 与 `models-store.ts:1 起` 支持；构造 stores 并不等于恢复外部 cache，readonly lookup 也不同于 async check/refresh | D-23/30/35；AIR-002/033/034/046 | **支持＋有意修正。** report 提醒 generation/底层 mutation 的限制；目标的单 owner commit、stale next-start 防护、404/501 tombstone 不能仅由“已有 generation 检查”声称完成。产品 remote source、ETag/4h/启动预算依 EP/上层源码，原报告未覆盖。已有目标明确，文档引用补齐。高 |
| RC-AI-11 | `ai.md:275–293`：显式 key→stored→ambient、同源 refresh、5min/15s、原子 store 与取消 | `auth/resolve.ts:95–160`；`models.ts:639–679,735–817` 区分 check/getAuth/login/logout；`auth/context.ts:23–44` 惰性 env/file methods；`credential-store.ts:14 起` 明确 mutation 顺序 | D-21/28/32/33/35；AIR-007/023/046/051/052 | **支持＋遗漏。** SDK 管理返回/效应、三条完整 OAuth 流、AWS 全来源与 canonical builtin 迁移不能靠本节概述证明，须 OA/BP/SDK annex 的一手核验。pi-specific 提前扣 expiry 与 Rust 单窗口、有界进程/完整-source失败关闭等是已记录差异；无新目标调整。高；完整依赖链本轮未重新执行/穷举 |
| RC-AI-12 | `ai.md:303–319`：Codex auto WS、会话/account 缓存、TTL/age、严格前缀 continuation、output 前 fallback、公开 cleanup | `api/openai-codex-responses.ts:294` 省略即 auto，而 `:1518` 只看原 options 决定 cached context；`:866–867,1425–1474` 参数/前缀；`session-resources.ts:1–24` 是全局同步 cleanup registry | D-23/24/35；AIR-035/036/046/050 | **支持＋源偶然的有意修正。** auto 是独立 SDK 行为，不仅产品。报告未揭露省略/显式 auto 的 delta 不一致，也未证明 cleanup 覆盖临时/connecting/overflow；当前目标已归一默认，并加强为实例 owned/awaitable cleanup。其余 live 性能/服务端 continuation 寿命仍不能由静态报告保证。高 |
| RC-AI-13 | `ai.md:321–327`：普通 provider helper 默认为0、可取消退避；并非类型注释所称 SDK 默认 | `utils/provider-retry.ts:106–123` default0；`api/openai-responses.ts:178–190` 内 SDK0、外 helper；`bedrock-converse-stream.ts:160–249` 直接构造 AWS client，未接 helper 参数；`utils/retry.ts:185–234` 与根 `index.ts:43` 还有公开整条 assistant-call retry | D-24/36；AIR-007/026/039，retry annex | **支持但范围须限定＋报告遗漏。** default0 是 helper/特定 adapter 事实，不是所有 adapter 断言；Bedrock default2 的充分证据来自已核 locked AWS/Smithy dependency，原报告本身不够。高层 helper 存在且已在后续 P08/M06 纠正遗漏；D-24/36 已分层，不重开。高 |
| RC-AI-14 | `ai.md:329–336`：成本估算、tier、推理能力向上再向下 clamp；没有完整上下文估计描述 | `models.ts:1193–1268` 计价/等级；`utils/estimate.ts:15–19,46–116` timestamp-valid positive usage anchor+后继估计，不做 same-model 检查；`api/simple-options.ts:15–48` simple context clamp/字段传递 | D-25/26/34；AIR-024/029/030/053 | **支持＋遗漏＋有意差异。** Q35 的 token heuristic/simple budget 由后续一手证据补足，报告未提不构成反证。unknown/zero 保留、独立 reported total、numeric-Claude0提前失败等不能当 pi 已有类型能力；当前 annex 明示差异/缺陷修正。没有新决策问题。高 |
| RC-AI-15 | `ai.md:96–98,243–247,359–360`：图片/分类独立类型/provider方法，deferred 可选 | `models.ts:912–980,1121–1170` 操作分发；`api/openrouter-images.ts:61–115` Chat-wire、first choice、跳过坏/URL media；`api/system-one-shared.ts:175–235` 默认retry2、先保留usage再解析所有答案；`types.ts:231–250` deferred options/handle | D-23/24/25/30；AIR-025–032/043/044/049 | **支持但不足以覆盖完整操作目标。** 首批 adapters/有界 bytes/严格 media、atomic answers、unknown usage、timeout、reopenable deferred fixture 由 SP/EP/OP 补证和确认。image 跳过坏输出/成功空图不复制，closed typed return 是 Rust 目标；未有远端 deferred adapter/其它操作不被假称全覆盖。高 |
| RC-AI-16 | `ai.md:46,338–343,362,405`：telemetry 依赖为类型/context传播，未见 AI startSpan；上层提供 instrumentation | `ai/package.json:72` production dependency；`src/types.ts:1,131–135,241–250,324–350`；`api/simple-options.ts:32`；`models.ts:837–868,871–980,1116–1170`；`test/telemetry-options.test.ts:68–165` 仅传递 identity，缺 classifier 专项 | D-37；AIR-054–056 | **支持＋报告覆盖不足。** classifier 通过共同类型与 dispatch 也能传；auth resolver 并不接收该字段。AI报告正确阻止“依赖=自动tracing”的推断，不能单独决定 Rust backend/领域 schema。D37 的 module/helpers、host-owned domain/backend、无自动 SDK spans 是已确认实现选择，不是假称通用 pi callback DSL 等价；跨包与演进详 TC/TE/TO及下表。高 |
| RC-AI-17 | `ai.md:188–193,365–411`：frame 为可选通道；测试矩阵是静态期望；环境有 key 可开启 live | `index.ts:38` 导出 frame；`types.ts:752–783` authoritative terminal；`test/models-entry.test.ts:30–57` 是源码测试设计；`test/telemetry-options.test.ts` 非真实 span；`vitest.config.ts:6 起`、`test/stream.test.ts:350 起` live gating | D-30/31/37；AIR-048/050/054–056；spec:269/369 | **支持＋已记录范围差异。** compact frames/raw provider observer/dynamic header callback 明确延后，不能由 AIR-048 全事件声称已覆盖。原报告没有把测试存在当执行，当前 acceptance 同样应保持未来式。真实服务、浏览器、bundler、第三方 backend 的缺口不自动重开产品决策。高 |

## 需要保留的关键限定

**实际调用方校正，非新架构反转。** RC-AI-03 的 [SDK fallback 注释](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts#L36) 和 [session 显式 streamFn](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts#L396) 同时存在。原报告观察到 compat 的存在是对的，推论若扩为“session 主请求仍走 global compat”就过度。本目录 [P02](pi-contracts-notes.md)、[evidence matrix](evidence-matrix.md)、[design evolution](design-evolution.md) 已限定此事实；无需回滚 D-30/35 或重新采访用户。

**返回与执行边界是对齐语义，不是复制对象形状。** [EventStream.result](../../../.repo/pi-1.0.0/packages/ai/src/utils/event-stream.ts#L86) 只返回单独的 final Promise，[AssistantMessageEventStream](../../../.repo/pi-1.0.0/packages/ai/src/utils/event-stream.ts#L91) 把 error 作为 assistant 值提取。Rust bounded stream 若也只被动等同类 future，可能卡在无人 drain 的满队列；D-31 已通过主动消费、terminal cache 和完整失败 message 闭合这个差异。`Tool`/validation 仍不执行工具，独立 SDK 用户显式追加消息；Agent 的执行权威/动态 revision 不应成为 sessionless SDK 前置条件。当前 AIR-045–050 与 spec:65 已准确分开。

**不能扩大报告的概括句。** RC-AI-13 的 helper default0 不是 Bedrock/System One 默认；RC-AI-16 的“直接 API”测试实际上含 fake `createProvider` dispatch，而非全部真实适配器/transport 已产 spans；RC-AI-15 的类型存在不证明 remote deferred 可用。目标中的独立依赖证据、真实 adapter→fake transport 验收及显式 deferred gap 保留了这些边界。

**报告之外的目标不因此无依据。** 原 AI 报告未完整研究 Agent hooks、session commit/compaction、产品目录策略或 Rust guards/Dispatch。这些要求分别依赖 TP/TM/EP/SDK/TC/TO 一手专题与人类决策，不能由 AI 报告静默替代。反过来，原报告提到一种能力，也不自动扩大已选范围；保留本轮已确认的 Rust Result、knownness、有限资源、独立 SDK、显式 trace parent 等差异，并在首次实现验收它们。

## telemetry 报告与当前目标映射

下表的报告定位指原 telemetry.md，源码仍相对 `.repo/pi-1.0.0/packages/`。全部是静态回核；测试行号指预期断言，未运行。11项均未发现需要重开Q37的证据，置信度高限于固定源码/已明示目标差异；未来运行与后端性能仍未验证。

| ID | 原报告主张及定位 | 回核的一手源码/反证 | 当前 D / AIR / V 映射 | 判定与处置 |
|---|---|---|---|---|
| RC-T-01 | `:5,15–24`：中立契约包、无 exporter/runtime dependencies | `telemetry/package.json:8–16,43–46`；`src/index.ts:14–22`；`src/testing/conformance.ts:1` | D-37；AIR-056；V-48 | **支持职责划分，不单独证成 crate 拆分。** pi 独立包有通用消费者形状；Rust 已选 tracing module 不等于否定该分层。是否拆 crate 由真实独立 consumer/契约缺口触发；testing 的 Node 依赖不推广为 root runtime 依赖 |
| RC-T-02 | `:127–131`：ai 传播 context 不等于自动 tracing | `ai/src/types.ts:132–135,324,333`；`models.ts:864–865,879–978,1116–1170`；`api/simple-options.ts:32`；`test/telemetry-options.test.ts:68–166` | D-37；AIR-054；V-46 | **支持。** 所选 chat/images/classifier/deferred 透传；classifier 有类型/分发证据但缺独立 identity 测试。不能由此宣称内置 adapters 已创建领域 spans |
| RC-T-03 | `:40,62–70,76`：同步一次 callback、Promise结果/拒绝值保持 | `telemetry/src/noop.ts:3–9`；`memory.ts:168–185`；`testing/conformance.ts:65–140` | D-37；AIR-055；V-47 | **已选语言机制差异。** Rust 同步 in_scope 与惰性 Future instrument 分开，未poll不执行；不是JS callback的相同时序。无需私建 executor；业务调用次数和结果须按Rust目标验证 |
| RC-T-04 | `:40,68,119,156`：callback结算、settled后child转NOOP、父不join子 | `memory.ts:95–98,126,141–165,176–185`；`testing/conformance.ts:220–272` | D-37；AIR-055；V-47；telemetry annex | **非等价且已明确范围。** raw tracing clone没有pi settled/inert机制；host拥有语义完成，handle返回/native close不当业务完成。当前不提供通用startSpan兼容层；若未来选择完整callback API必须重新决定生命周期 |
| RC-T-05 | `:42,72`：ok/error，显式ok后throw可保留ok | `index.ts:12`；`memory.ts:95–96,157–164`；`testing/conformance.ts:142–181` | D-37；AIR-056；V-48 | **非等价且已明确范围。** 无通用SpanStatus/last-write-wins目标；domain owner解释Result/Length/Deferred。Rust Err不能自动声称与pi记录status等价，也不能因缺省正常返回就把业务失败标成功 |
| RC-T-06 | `:76`：NOOP不读payload、共享冻结span | `noop.ts:3–20`；`test/telemetry.test.ts:156–196` | D-37；AIR-054/055；V-47 | **支持目的、机制不同。** disabled scoped Dispatch避免本次instrumentation误接ambient；不复制JS对象身份，不静默宿主独立日志/显式其他context。缺省不上传并不等于终止业务call |
| RC-T-07 | `:78–87`：原子忽略记录失败；自定义adapter仍须合规 | `memory.ts:63–68,128–165`；`testing/conformance.ts:206–218,274–312`；README:120–132 | D-37；AIR-056；V-48 | **支持。** trusted conforming backend负责被动性；不承诺tracing隔离任意panic/block。future验收保持callback/poll/send/retry次数及Result，而非只断言日志存在 |
| RC-T-08 | `:91–115`：typed schema主要编译期，元数据不验证parent或自动脱敏 | `index.ts:28–31,44–74,140–141,281–353`；`test/telemetry.test.ts:138–150` | D-37；AIR-055/056；V-47/48 | **支持Rust替代的边界。** producer掌握typed词汇、字段与结果解释，当前不移植通用schema DSL。未来需要共享schema时再选定，不把TypeScript类型工具存在当新增框架理由 |
| RC-T-09 | `:159`：使用方决定敏感内容，reference复制传入内容 | `memory.ts:71–81` 可复制Error.name/message；README:387–389是数据策略要求 | D-37；AIR-056；V-48 | **有意加强，非语言必然。** 当前默认排除自由错误文本和raw payload是已选字段策略；pi metadata/reference不替调用者脱敏，不能宣称复制pi即安全 |
| RC-T-10 | `:55,119–123,151`：memory无时钟、上限、flush；fixture处置 | `memory.ts:16–24,192–217`；`testing/conformance.ts:12–24`；`testing/types.ts:5–15`；`test/conformance.test.ts:23–46` | D-37；AIR-056；V-47/48 | **支持范围限制。** 测试recording subscriber不用无界生产buffer、数字ID/endSequence/getSpans兼容；exporter队列/flush/shutdown由host负责。无性能基准不声称更快或零开销 |
| RC-T-11 | `:129,155–167`：README与当前Agent导出矛盾，历史/第三方验证未覆盖 | README:371–383；`agent/src/index.ts:1–5`；`agent/CHANGELOG.md:7`删除声明；历史提交/提案状态见TE-03/04/08/09 | D-37；AIR-054–056；[TE演进](telemetry-evolution.md) | **报告支持现状，历史须补证。** generic契约仍有效，旧integration例子部分过时。作者拆包意图、0.84发布前已用callback、1.0删除与未合并PR不单靠该静态报告推导 |

由这两份报告能直接支持的SDK语义是：实例集合装配、消息/选项交给adapter、完整最终结果与tool声明边界，以及显式context传播和被动观测职责。调用名称之外仍须对照执行时机、有效请求、终态、失败partial、已知usage和清理。Rust Result、owned partial、有限队列/主动drain、惰性执行与native tracing已单独说明；默认安全字段、拒绝图片占位等则是有证据和确认的政策/正确性差异，不能统一归因于语言。

两份报告无法单独回答完整OAuth/AWS依赖链、产品目录/恢复/持久化、作者后续规划、所有公开pi功能覆盖或真实后端性能。继续使用已有专项证据，不因报告遗漏删掉已获确认的目标，也不因为报告列出能力而隐式新增。精确导出/协议覆盖与差分验收依然以当前spec/acceptance的范围为准。

## 检查记录与交接

实际执行：只读文件/行号读取、`rg` 定位、Python SHA-256完整manifest比对及AI子树独立比对、原报告149个源码行号链接检查。**未执行** Vitest、类型检查、build、Rust runtime 测试或任何网络请求。测试影响 `none`；本文没有修改测试，也没有改动原 `docs/research/pi-1.0.0/`。

交接结论：RC-AI-03/04 是原报告限定/精度问题；RC-AI-02/05/07/10–17 记录补证、遗漏或已确认差异；RC-T-01–11区分继承目的与非等价机制。没有把现行 opi-spec 用作否决理由，没有新增待用户猜测的目标选择。仅新建本页并补README/decisions/spec英中证据导航，56 AIR/48 V行为及确认状态不变。后续正式协调/实施准入与全部运行时验收仍未发生，候选保持DRAFT。

### 本轮最终文档检查

主Agent整合后的命令、结果与共享状态在本节记录；运行时验收不从文档检查推导。

| 实际命令/检查 | 结果与证明边界 |
|---|---|
| Python标准库：完整manifest SHA-256重算，经 `python -X utf8 -` 执行 | exit0；1,848文件及树摘要匹配，新增/缺失/差异均0；不是发布产物或运行行为证明 |
| Python标准库：原报告源码链接/行号检查 | exit0；ai110、telemetry39个引用的文件存在且行号有效；不将此当每个主张正确的证明 |
| `python scripts/opi-doc-check.py` | exit0，`opi documentation contracts: PASS` |
| `git diff --check`，Python subprocess捕获全部诊断并汇总 | exit2；仅原source-manifest的11,720条既有尾空白；该输入hash不变，全仓库检查不记为通过 |
| 逐文件 `git diff --no-index --check -- /dev/null <path>` | 本研究42文件均无whitespace诊断；新文件差异exit1正常，普通diff不会覆盖这些untracked文件 |
| Python标准库组合检查，经 `python -X utf8 -` 执行 | 42文件UTF-8/LF/final LF/无尾空白、724个本地链接目标存在、3个JSON解析；0问题 |
| 同一脚本的双语追踪/覆盖检查 | AIR-001–056、V-01–48连续唯一且索引完整；每条有owner/MUST/D/V，英中证据列相同；389行逐行AIR/D/V、61个规范词、inline code及850个数字token对应，0问题 |
| 原报告/manifest哈希及目录输入输出比对；Git HEAD/status/index读取 | 原三份输入不变；只新增本页并改README、decisions、spec英中导航，未删除文件；HEAD仍`0fa995de504bcb9854e8ee841d54d99579da1bd4`，staged为空 |

共享工作区本轮已有CHANGELOG、opi-eval的external-lock/profile/scripts/agent实现及旧manifest改动，还有无关pi-evals研究文件；全部保留。它们不是本轮写入，也不以未提交状态宣称已发布。本轮没有Git写入、外部issue/PR、依赖安装、环境凭据或live调用。

当前受检契约SHA-256（只增证据导航，不改目标行为）：

| 文件 | SHA-256 |
|---|---|
| spec-draft.md | `c22e97867f750af1bc3be6df994250769f1f4ff2167842b836260878aeddc652` |
| spec-draft.zh.md | `e056e56a3b01a3623952b44bd517f2e24196ba57d02a0af4b0c8b1d03aa6520f` |
| acceptance-matrix.md | `67f90af543209761ccef1cbf16487f84c8932fac704fe7fe2833d3cb92bfbc01` |

记录结果后重跑doc checker及组合文档检查，统计与结果保持一致。实际测试影响 **none**（add/update/delete均0）；此前已运行runtime记录仍留在原评审，当前V全部是未来设计。没有新增阻断目标决策的问题，但正式协调、实施与离线/外部SDK验收仍待执行；不将本次交叉核查登记为ready-for-agent。

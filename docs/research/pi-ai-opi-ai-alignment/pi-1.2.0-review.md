# Pi 1.2.0 对齐文档时效与候选可行性审查

> **修订后阅读提示（2026-10-16）：** 本报告正文与证据保持核查时的输入身份；“现有候选暂不通过”等结论针对修订前稿，不是修订后稿的复审结论。修订后的目标见[中文 spec](spec-draft.zh.md)／[English](spec-draft.md)及当前附件；新的独立结论与实际检查见[修订复审](pi-1.2.0-revision-review.md)。本轮没有把事实审查改写为实现验收。

> 同日后续核对见 [五问补核](pi-1.2.0-followup.md)：三类 Context/telemetry、现日官方规划、调用体验、测试处置及持续版本记录。它补充本审查的覆盖，不追改原始基线或把旧执行结果变成新运行。

> 非规范性审查，2026-10-16。对象是本目录审查前的 42 个文件、固定本地 Pi 1.2.0 快照和当前 `opi-ai`，不是产品实施或运行时验收。输入摘要、版本差异清单和共享工作区状态见 [review baseline](pi-1.2.0-review-baseline.json)。原有 `baseline.json`、D/Q 回答与探针结果保留原义。

## 核准结论与本次确认

**需要跟进调整。方向可行，但现有 `spec-draft.md` 暂不通过新目标的核准，不能原样作为实施输入。** 原因包括明确过时的契约、公开能力覆盖不足和超出包范围的完成条件；没有发现必须否决整体方案的 Rust 语言障碍。

用户本次明确选择：**仅 `opi-ai`，以 Pi 1.2.0 的公开能力和可观察语义等价为目标，接口采用 Rust 惯用设计，每项差异明确说明。** 本次没有选择修改整个 Opi 项目的定位，也没有选择仅保留旧候选的能力子集。

“等价”应检查调用方能完成的动作、输入/输出与失败、状态变化、回调、重试/取消、资源释放及可用性，不能只比较函数名或最终字符串。Rust 的 `Result`、所有权、惰性 Future、有界背压等可以构成有理由的显式差异；尚未实现的公开能力仍是缺口，写过“deferred”不代表已满足目标。

[现行规范 §2.1–2.3、GOAL-003、§3.2](../../opi-spec.md#2-mission-goals-and-non-goals)鼓励 Rust 原生设计，也没有把每个 Pi 功能列为项目义务。这不禁止本包进行能力对照；同时，研究稿不能覆盖正式规范的 PRIN-002/003、INV-001/002 或实施权限。旧稿“整个 opi 是 pi 的 Rust 实现”“opi-spec 不再否决”的概括应在新候选中改为上述包内目标。只有实际抵触的条款需要单独协调，不能预设全面重写 Mission 是本包研究的前置条件。

## 基线与时效判断

| 对象 | 本次证据 | 含义 |
|---|---|---|
| 原研究 | Pi 1.0.0；Opi `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`；后续 telemetry 记录另有固定 HEAD | README 的后续导航升级没有重新核验旧候选 |
| 当前 Opi | HEAD `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8`；workspace 0.8.3，edition 2024，MSRV 1.97 | 当前根 [manifest](../../../Cargo.toml) 是版本权威 |
| `opi-ai` 自原基线变化 | 限定 Git diff 只有两份 README 的版本号变化，`src`／`tests` 无变化 | 原实现审计大体仍适用；不能因日期旧而关闭缺口，也不把旧运行记录当本次重跑 |
| Pi 当前快照 | `@earendil-works/pi-ai` 1.2.0；整树摘要 `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10` | 本地无独立 `.git`；没有在本次另证官方 tag、npm 产物或最新远端状态 |
| Pi AI 文件差异 | 1.0.0 的 378 → 394；新增 18、同路径变化 79、删除 2、未变 297 | 按历史和当前清单的路径/SHA-256 比较；没有旧源码，不能伪称逐行 diff |
| Pi telemetry 差异 | 12 → 12；仅 `package.json`、`CHANGELOG.md` 变化，其余 10 文件摘要相同 | 中立 context 传播、不自动创建 SDK spans/exporter 的旧结论可以保留 |
| 当前源码完整性 | 重算 AI＋telemetry 406 文件摘要，差异 0 | 对照 [当前 manifest](../pi-1.2.0/source-manifest.json)，不是执行证明 |
| 旧证据链接 | 审查前 39 Markdown 的 inline-link 扫描发现 392 个 `.repo/pi-1.0.0` 链接，分布 10 文件，目标均不存在 | 这是文件路径检查，不含网页/锚点验证；不能直接替换为 1.2.0 同行号 |

## 阻塞新候选核准的发现

P1 表示阻止现稿按本次目标核准，P2 表示需要修订的覆盖/证据问题；这些级别不是对未实施代码作生产事故评级。

### R1 · P1：包内目标与旧稿范围、排除项不一致

旧 [scope、AIR-015–019、037、039、041 和完成条件](spec-draft.md)同时要求 session v3、迁移/compaction、产品 tool_search、启动刷新政策、Agent 工具授权/hook 和高层恢复。它们有各自的所有者，不能全部作为“仅 `opi-ai`”的完成条件。新版应拆出 SDK 主契约与下游集成影响，保留本包破坏性改动引发的必要调用方适配。

同时，旧稿明确排除 Mistral 原生 Conversations、其余 classifier bindings、新 OpenAI ChatGPT OAuth、动态 header transform、原生 provider-event observer 和 compact frames 等。这个选择在旧目标下是明示范围；在本次公开能力等价目标下必须逐项重审。不能沿用“没有目标问题留给实施 Agent”来跳过新覆盖清单。

普通 SDK 已经被旧稿第 63–65 行明确免除 durable ACK、session 和 Agent authorizer 前提；这是已修正的部分，应保留，不能重复报为现存耦合。

### R2 · P1：Azure 的 provider ID 与 wire 身份已变化

旧 AIR-052、SDK OAuth/builtin annex 仍要求把 builtin 迁为 provider `azure-openai-responses`，并把 Azure Chat 仅作为旧显式 profile。Pi 1.0.3 已改为 **provider `azure`**，同一 provider 提供 `azure-openai-responses` 和 `openai-completions` 两种 API，并共享 deployment-name 处理。见 [factory](../../../.repo/pi-1.2.0/packages/ai/src/providers/azure.ts#L14)、[provider ID/双 API 注册](../../../.repo/pi-1.2.0/packages/ai/src/providers/azure.ts#L44)。

修订必须区分 provider ID、API/wire ID、catalog model ID 和实际 deployment；不能把 wire 里的 `azure-openai-responses` 一并删掉。同步 AIR-052、builtin migration、V-44 和调用示例，否则照旧稿实施会立即产生新的迁移负担。

### R3 · P1：OAuth 刷新取消策略可能丢弃旋转后的 token

旧 AIR-051、refresh annex 和 V-43 仍要求 caller cancellation 终止刷新网络，commit 前取消保留旧 credential。Pi 1.0.3 修复了这个行为：[刷新契约](../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L105)明确规定 caller signal 只取消等待锁；已开始的刷新使用独立 15 秒 deadline，并在释放锁前保存结果。原因是服务器可能已经旋转 refresh token，旧 token 不再有效。

Rust 新契约应区分“调用者停止等待”与“已开始的凭据轮换完成并持久化”。保持同源/同账号约束，不自动继续已取消的生成请求；明确 refresh owner、有限任务生命周期、存储失败及 logout/replace 的串行化语义。对照 [回归测试源码](../../../.repo/pi-1.2.0/packages/ai/test/models-runtime.test.ts#L800)设计离线验收。本次没有执行该测试。

另需更新 Anthropic callback：53692 是首选端口，失败后尝试空闲 loopback 端口，再降级手贴；不是不可变的唯一 callback。见 [实际逻辑](../../../.repo/pi-1.2.0/packages/ai/src/auth/oauth/anthropic.ts#L140)。[LoginOptions.agentName](../../../.repo/pi-1.2.0/packages/ai/src/auth/types.ts#L202)也是新版公开输入，需纳入映射。

### R4 · P1：strict-prefer 降级与“payload 永远固定、回调一次”互相冲突

旧 AIR-040 主要描述本地 schema 可否转换；AIR-007/042 与 annex 又冻结 payload，要求 transform 一次。Pi 1.2.0 在特定 Anthropic `400 invalid_request_error` 的 grammar 大小、strict tool 数量、union 限制下，会在流开始前**至多降级一次**：只去掉 `prefer` 的 strict，保留 `require`，重建请求，再执行 `onPayload`。见 [请求/降级循环](../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L644)。

这不是任意 400 重试，也不是生成中重放。新稿需要区分“同一 payload 的 transport retry”和“有语义记录的 strict 降级变体”，固定次数、auth/model 不变、回调次数、诊断、取消和 require 保留规则，并修订 V-05/34/36。若保留 Rust transform 只执行一次，必须给出可实现的投影顺序及显式差异，不能同时宣称与 Pi 回调行为相同。

### R5 · P1：token 估算系数已从 4 改为 3.5

旧 estimator annex 仍写 UTF-16 code units、4800 image chars、按 message 汇总后以 `/4` 向上取整；Pi 1.1.0 改为 `ceil(chars / 3.5)`。见 [常量及文本估算](../../../.repo/pi-1.2.0/packages/ai/src/utils/estimate.ts#L15)、[message 估算](../../../.repo/pi-1.2.0/packages/ai/src/utils/estimate.ts#L46)。这直接改变 simple 的输出预算，不只是估计展示。

保留旧稿有证据的 usage anchor、known/unknown、UTF-16 和 message 粒度选择；更新公式与 V-45 的固定预期。350 个 UTF-16 units 的无 anchor 文本应估为 100 token，旧公式是 88。Rust 不应使用 UTF-8 byte length 代替该兼容语义。

### R6 · P1：classifier 新公开输入和 adapter 未进入候选

Pi 1.1.0 增加 [ClassifierContext.images](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L682)及 image modality 检查，并新增 [OpenAI Decisions adapter](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-decisions.ts#L19)。Decisions 是 `bool → predicate`、可携图像的独立协议，不能把 System One `bool → noul` 原样复用。它还具有 API-key-only availability 和特定 504 不重试规则；见 [OpenAI provider filtering](../../../.repo/pi-1.2.0/packages/ai/src/providers/openai.ts#L23)、[Decisions request](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-decisions.ts#L149)。

旧 AIR-028 及 V-24 的 System One 子集不能证明这些能力。新版需列四个 [known classifier APIs](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L79)，检查每个协议的图像支持与拒绝路径，新增相应输入/结果/usage/retry 验收，而不是只添加枚举。

### R7 · P2：公开可观察字段和默认值需要补充验收

| 新版事实 | 旧稿问题 | 新候选所需证明 |
|---|---|---|
| [AssistantMessage.durationMs](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L588)、[终态单调计时](../../../.repo/pi-1.2.0/packages/ai/src/utils/event-stream.ts#L91)；工具结果也有可选时长 | AIR-047/048 未固定该字段、direct provider 与 deferred 边界 | 同一 response 的转发/complete/result/迭代保留计时；直接 adapter 也有计时机制；已带值与较早 deferred 结果不重计。不保证覆盖外层 Models auth/loading；Rust 从创建还是首次 poll 计时需明确 |
| [samplingParamsByThinkingLevel](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L851)，按有效 effort 选层 | 旧模型/选项方案没有完整映射 | `model defaults → effective-level override → request keys`，并分别核对 native/simple 入口；见 [resolver](../../../.repo/pi-1.2.0/packages/ai/src/api/simple-options.ts#L24) |
| [pricing tier schema](../../../.repo/pi-1.2.0/packages/ai/src/providers/model-schema.ts#L33)、[按最高命中阈值计整次请求](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1206) | 不能以旧稿“billing redesign 排除”跳过计费元数据对齐 | 阈值等于/超过、input+cacheRead+cacheWrite、未知价格/usage 分开；保留现有 Rust tiers，不误报从零新增 |
| Haiku 5.5 与 Bedrock effort/replay 的更新 | 旧模型清单、thinking/wire fixtures 不覆盖新版映射 | 核对 adapter 的请求值与 stale-thinking 投影；不以“模型名能列出”作证明 |
| 新 transient/busy/upstream 和 Bedrock stalled-stream 分类 | 旧固定 retry 语料时间窗口过早 | 比对 [retry classifier](../../../.repo/pi-1.2.0/packages/ai/src/utils/retry.ts#L28)，分清 SDK acquisition retry 与宿主高层 recovery |

现有 Rust 已有 [ModelPricing/PricingTier](../../../crates/opi-ai/src/model_info.rs#L404)；需要核对字段、实际算法和 catalog 接线，不能把上游新增 release note 当成本地必然缺失。Mistral 1.2.0 将请求 timeout 限于等待 response headers，这与旧稿 image 的 body deadline 是不同操作，不能套同一结论；原生 Mistral 被旧稿排除，本次应进入公开 adapter 覆盖表重新审查。`mistral-conversations` 是历史 API 标识，当前实际请求为 [POST `/v1/chat/completions`](../../../.repo/pi-1.2.0/packages/ai/src/api/mistral-conversations.ts#L298)，不能根据名称要求实现 `/conversations`。

### R8 · P2：动态目录增加了配置 endpoint 和按 key 个性化入口

Pi 1.2.0 新增 [RefreshModelsContext.baseUrl](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L80)和显式 [personalizeOpenRouterCatalog](../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter.ts#L16)：请求有效 base URL 下的 `/models/user`，只筛 chat 并更新该 key 的限额/价格，image/classifier 不变；无 key 或 401/403/404 保留原 catalog。

这个 helper **没有自动装入** `openrouterProvider()`，不能写成“Pi 默认刷新总会探测 key 权限”。新候选应提供等价显式能力/扩展点，验证 endpoint override、chat-only 过滤、错误保留和取消。SDK 目录刷新与产品启动刷新是不同交付层。

另外，[Radius](../../../.repo/pi-1.2.0/packages/ai/src/providers/radius.ts#L26) 使用已取动态 catalog 替代基线，空列表也有权威，避免被组织禁用的模型重新出现；不能把通用 provider 的 overlay 行为推广给所有动态目录。OpenRouter 新价格解析将负的 placeholder 基价转为零，并解析 tiers；Rust 可以保留 unknown 的更准确表达，但须作为显式差异验证，不能把 placeholder 当有效负费用。见 [price ingestion](../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter-api.ts#L35)。

### R9 · P2：Anthropic transcript 的旧 wire 证据需要换基线

`pi-contracts-notes.md`、replay/transcript 附件中的旧 beta 和同名重定义会 collapse 的描述不再适合 1.2.0。Pi 1.0.1 已采用 `inline-tools-2026-09-15`，后续 `tool_addition` 按值携带声明，支持同名重定义。见 [beta](../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L195)、[inline tools](../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L1170)。

AIR-008/011 的 chronology/native-or-collapse 方向仍有效；应更新 adapter-specific fixtures，而不是推翻 canonical transcript，或把旧 beta 行号机械改到新目录。`hasToolRedefinitions()` 的 deprecated 兼容 helper 与内置 transport 实际所需也要分开。

### R10 · P2：历史评审闭合不代表新版本覆盖或执行通过

审查前两版均 389 行，56 个 AIR 唯一、48 个 V 唯一且 AIR 均有 V；对应行的 AIR/D/V/Q、inline code、数字集合一致。对错误/取消、retry、预算、投影与 telemetry 语义抽核未见实质英中漂移。这说明旧稿内部组织有价值，不说明覆盖 Pi 1.2.0 或接口已经可编译。

原 70 测试、流/路由 probe 是有固定历史输入的执行记录；本次没有重跑。旧 review 的“已关闭”保留其版本/hash 限定，不应被重开为当前问题，也不应当作新目标的签署。392 个旧源码链接应保留历史含义，通过本次新的有效源码定位补证；不能用报告恢复材料冒充旧源码已恢复。

## 新候选必须覆盖的公开能力面

以下是核准前的覆盖检查，不是另一份实施账本，也不增加 AIR/V 编号。每行必须最终给出 Pi 1.2.0 来源、Rust 公共调用、可观察语义、明确差异和机械验收；分期未完成时不能宣称整个包已等价。

当前源码可枚举 10 个 known chat API、4 个 known classifier API、1 个 known image API、42 个 builtin provider factories 和 9 类 OAuth flows。多个 factory 复用 wire，这不等于 42 套 transport；其身份、鉴权、endpoint、默认值与 availability 仍需覆盖。具体模型全集受生成数据缺失限制。专用 OpenRouter Image API、image streaming 不属于当前已交付的 Pi 1.2.0 adapter，不能凭想象加入 parity 硬要求。

| 能力面 | 对旧稿的处置 |
|---|---|
| Models、provider 注册/替换/删除/清空、chat/type/all 查询、自定义完整模型 | 保留实例化设计；补完整管理方法和自定义 provider/wire 扩展覆盖，不能只核对四个 chat 方法 |
| native/simple stream/complete、direct adapter、最终 result | 保留统一 collector，补时长和 lazy timing；TS 的错误消息终态映射为 Rust `Result` 需文档化 |
| chronology、sections、tool changes、schema/grammar、跨 provider replay、opaque thinking | 保留核心机制；以新版 adapter fixtures 更新，宿主执行/授权另列 |
| 模型 metadata、compat、sampling、effort、context estimate、usage/cost、input limits | 更新 3.5、effort-dependent sampling、pricing tiers；说明 Rust known/unknown 和整数精度差异 |
| payload/response/headers/provider-event 回调、frame helpers | payload/response 已有候选；动态 headers、raw events、frames 的旧排除需逐项重审；保密与身份保护差异不能笼统略过 |
| Chat 的各 published API 与可注册 custom adapter | 依 [KnownApi](../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L61)和 `api/*` 公开导出列协议矩阵，补 Mistral/pi-messages 等旧范围外项目的处置；复用 Chat wire 的 provider profile 也须列配置差异 |
| Images | 当前 known image API 是 `openrouter-images`；核对全部 provider bindings、输入/输出/限制，而不虚构多个 image wire 或把 terminal 工具算作包能力 |
| Classifier | System One、Cloudflare、llama-cpp、Decisions；含图像输入、可用模型过滤、结果/错误/usage 与协议特定 retry |
| Auth/login/refresh/logout、API key/AWS、OAuth metadata | 更新不可由 caller 中断的已开始 token rotation；补当前 OpenAI ChatGPT 等 [已公开 flows](../../../.repo/pi-1.2.0/packages/ai/src/auth/oauth/load.ts#L16)的覆盖，不只三条旧 flows |
| Catalog/store/refresh、configured endpoint、显式 OpenRouter personalization | 分开 SDK 机制和产品远端源/启动政策；复核按 key/账号隔离、publication 和 store 一致性 |
| transport、session resources、Codex WS/continuation、timeout/retry/cancel | 保留实例资源所有权与 await cleanup；列各 adapter 真实边界，不把通用策略强套所有协议 |
| 显式 whole-call retry helper、Codex 资源诊断 | Pi 根导出 [retryAssistantCall](../../../.repo/pi-1.2.0/packages/ai/src/utils/retry.ts#L174)，独立 SDK 可以显式使用；Models 默认不自动恢复与此并不矛盾。重试 helper 不要求产品 session。公开资源统计/关闭的观察价值另列映射，Rust 不复制 module-global registry |
| deferred submit/fetch/cancel 与可重开 caller | 保留 typed 生命周期；没有 remote 协议证据时明确缺口，不能把离线 fixture 称为远端能力已对齐 |
| 显式 telemetry context | 保留小模块与宿主 backend 设计，核验所有公开操作的传播；不扩成自动 spans/exporter |
| 根/subpath exports、轻量导入意图、catalog 资源、CLI/平台桥接 | 不复制 npm 路径或 Bun loader；逐项说明 Rust module/packaging/工具替代与平台差异，检查能否独立使用，不能从公开表面静默删除 |

## Rust 可行性与实现约束

**主体可行，当前缺的是修订后的契约和实现证明。** 下列是设计审查结论，不是已编译的新 API。

| 设计 | 建议与证明义务 |
|---|---|
| 单一 `Models` | 合并现有 collection 的真正状态，避免长期并列 facade；普通调用和高级 prepare 共用绑定/collector。当前 [Provider](../../../crates/opi-ai/src/provider.rs#L22)、[AuthResolver](../../../crates/opi-ai/src/auth.rs#L292) 已有对象安全边界 |
| owned lazy Future/Stream | 在普通方法构造时捕获 owned 输入，异步 I/O 首次 poll 开始；不能用借用 `&mut Context` 的长寿 Future 冒充 `Send + 'static`。清楚标出与 eager JS 的启动/计时差异 |
| `Result`、closed states 与 ID | `Result<AssistantMessage, CallFailure>` 保留失败 partial/usage；操作种类用 enum，provider/model 身份不能硬编码成封闭 vendor 列表。自定义 API 的扩展能力另有明确注册/验证契约，不因此造通用插件框架 |
| 有界流与 `result(self)` | 保留容量 64 和主动 drain、单个 terminal cache；drop/cancel/EOF 为可验证状态。event-time immutable partial 可以用不可变共享结构降低复制，但不能以 `Arc<Mutex<_>>` 把旧事件重新变成可变 live view；性能尚未验证 |
| refresh 与 store transaction | 当前 [CredentialStore](../../../crates/opi-ai/src/credential.rs#L202) 仅 read/write/delete/probe，产品刷新依赖 [具体 store 锁](../../../crates/opi-coding-agent/src/credential_store.rs#L1207)。仅换 `Arc<dyn CredentialStore>` 或加 Models-local mutex 不足以协调共享 store/多进程。需要 store-owned、对象安全的互斥/原子更新契约；泛型 `modify<F>` 不能直接充当 dyn trait 方法 |
| task/resource lifetime | `Drop` 发出停止信号，显式 async cleanup 提供完成屏障；已开始 token rotation 是有独立有限 owner 的完整性例外，需保存 token，不能被普通请求 drop 任意丢弃。普通生成任务不因此无限后台存活 |
| telemetry | 锁定 tracing 0.1.44 的公开 `Span::with_subscriber` 能取来源 Dispatch；保持 explicit parent、scoped poll/drop、no enter guard across await。没有必须增加独立 telemetry crate、全局 subscriber 或 exporter 的证据 |
| 依赖与验证 | 保持内向依赖，精确审查锁定 Rust 库及平台影响；测试本地 fake transport/clock/store。Rust 类型检查不证明供应商接受、资源清理、外部包消费或计费真实值 |

旧 Opi 的二次目录选模、逐 chunk lossy UTF-8、prepared terminal 后可继续 poll、tool 图片占位及缺 chronology/typed operations 等审计没有因版本号变更消失。源码定位见 [mapped dispatch](../../../crates/opi-ai/src/api_mapped.rs#L174)、[Chat chunk decode](../../../crates/opi-ai/src/openai_chat.rs#L1219)、[prepared poll](../../../crates/opi-ai/src/provider_collection.rs#L796)、[tool image placeholder](../../../crates/opi-ai/src/openai_chat.rs#L1413)。本次只是静态确认相关源码未变，未重新复现 socket/race 观察。

既有独立 SDK 已有公开 `drain_to_completion`；问题是旧双失败通道与调用组合，不是完全没有完成收集能力。既有外部 consumer/package smoke、双实例、>64 事件、三种 result 消费、tool loop 和生命周期验收设计值得保留。`opi-eval` 无 `opi-ai` 依赖，不能把其 native capture 适配器当作第二个 Rust SDK 消费者。

## 原有 42 文件的处置

“重核”表示需要新的适用性证据，不表示删除旧研究。原始确认、执行记录和固定窗口不追改；本次只在四个入口标注最新状态，正文修订留给以新范围为准的下一版候选。

| 文件 | 时效与建议 |
|---|---|
| `README.md` | 当前导航：已加本次结论；旧研究/检查段保留历史限定 |
| `spec-draft.md` | 旧 Pi 1.0.0 子集候选：已标暂不通过，按 R1–R9 修订后再核准 |
| `spec-draft.zh.md` | 同英文稿，同步状态与后续语义 |
| `acceptance-matrix.md` | 旧 56 AIR／48 V：保留原设计，更新覆盖/范围/固定预期后再用 |
| `decisions.md` | 人类确认历史：保留 D/Q；本次范围确认见本报告，不冒写新回答 |
| `baseline.json` | 旧内容身份：原样保留，不改成新 HEAD |
| `pi-contracts-notes.md` | Pi 1.0.0 事实：需要新版本补证；不得原样称 1.2.0 契约 |
| `opi-audit-notes.md` | 旧 Opi 事实：源码未变部分仍适用，执行结果不视为本次重跑 |
| `design-audit.md` | 旧综合审计：保留事实和已关闭历史，更新新版本比较 |
| `evidence-matrix.md` | 旧比较索引：双方当前基线、范围、差异重新映射 |
| `design-evolution.md` | 固定 tag/main 窗口：保留，不称当前远端路线图 |
| `decision-reassessment.md` | 旧目标纠偏历史：保留，旧 pending 不自动重开 |
| `reassessment-transcript-notes.md` | 旧 transcript 纠偏证据：保留历史，重核接入新稿的断言 |
| `reassessment-sdk-notes.md` | 旧 SDK 纠偏证据：保留历史，重核接入新稿的断言 |
| `reassessment-evolution-notes.md` | 旧演进纠偏证据：保留历史，不能替代新版本验证 |
| `transcript-target-proposal.md` | 语料和机制可复用；更新 inline tools/strict/replay 等 wire 细则 |
| `transcript-migration.md` | SDK chronology 与产品 session 交付拆分；保留必要迁移影响 |
| `replay-contracts.md` | 机制可复用；更新 metadata/serializer/thinking fixtures 与有效源码引用 |
| `sdk-target-proposal.md` | SDK/精确依赖证据：更新版本适用性、strict/预算/公开覆盖 |
| `evolution-target-proposal.md` | 库与产品联合目标：按 R1 拆层，更新刷新/transport/retry 证据 |
| `sdk-consumer-contract.md` | 使用方视角可保留；补全 1.2.0 能力与 Rust 差异清单 |
| `sdk-pi-caller-review.md` | Pi 1.0.0 caller 事实：不能作为新版本完整签署 |
| `sdk-opi-caller-review.md` | Opi 实现事实大体仍适用；旧持久化门槛已被后稿修正，不重复报错 |
| `sdk-auth-provider-migration.md` | 必须更新 Azure、refresh cancellation、callback 与 OAuth 覆盖 |
| `provider-contract-addendum.md` | 2026-10-03 外部协议观察：保留时间窗；本次未联网更新 |
| `operations-design.md` | 首轮设计与替代选项历史：保留；不能决定新版 classifier/image 覆盖 |
| `placement-options.md` | 旧归属比较：保留，按包内机制/宿主政策重新适用 |
| `telemetry-design.md` | 核心责任划分仍有效；检查新版所有操作传播 |
| `telemetry-pi-contracts.md` | telemetry 实现文件未变，核心结论保留；旧源码链接无效需新定位 |
| `telemetry-opi-audit.md` | Rust tracing 机制建议保留，不能称已接线 |
| `telemetry-evolution.md` | 旧 PR/时间窗历史：不冒称本日远端状态 |
| `telemetry-spec-review.md` | 旧 56 AIR／48 V 专项评审：保留版本/hash 证明边界 |
| `ai-telemetry-report-crosscheck.md` | 旧同快照交叉核验：保留，不能冒充本次 1.2.0 复验 |
| `review-notes.md` | 旧证据评审：保留已修正项和当时范围 |
| `spec-review.md` | 首轮候选审查历史：保留特定输入 hash |
| `spec-evidence-review.md` | 首轮 authority 审查历史：保留限定 |
| `spec-review-round-2.md` | 44 AIR／38 V 评审：不套用于新版本 |
| `sdk-spec-review.md` | 53 AIR／45 V 等历轮复核：不套用于新版本 |
| `offline-probes.md` | 旧实际观测与复现方法：保留，不称修复后的回归验证 |
| `offline-probe-result.json` | 原样保留旧执行输出 |
| `route-probe.md` | 旧实际路由反例：静态相关源码仍在，不称本次重跑 |
| `route-probe-result.json` | 原样保留旧执行输出 |

## 修订与重新核准的顺序

1. **包范围与能力清单 → verify：** 以 Pi 1.2.0 根导出、subpath、Models/Provider/options/type 和 builtin factories 为输入；每项公开能力有对齐或显式差异，跨包产品政策不混入 SDK 完成条件。
2. **契约校正 → verify：** R2–R9 分别进入双语要求、相关 annex 和唯一 acceptance matrix；特别是 Azure/provider-wire、token rotation 与 strict 降级无互相冲突的状态/次数规定。
3. **Rust 接口与消费者 → verify：** 主要调用示例、store/refresh owner、custom provider、lazy timing 和流结果可逐步解释；必要调用方迁移列清楚，未编译方案不写成已通过。
4. **文档重新核准 → verify：** 公共能力覆盖、差异理由、56 AIR 的保留/拆分/替换映射与新版验收一致；已选安全差异不隐藏尚缺能力。
5. **未来实施与验收 → verify：** 由正常实施准入决定，使用本地 fixtures、真实 adapter/fake transport 与 workspace 外 consumer。新规范核准本身不证明这些测试已经运行。

本次目标明确且可以完成一轮静态审查，未启动 wayfinder 的 issue-map 工作流、创建 tracker、分支或工单。若后续整个实施规模需要多会话决策地图，应再按其适用范围选择；这不阻止本次给出审查结论。

## 审查覆盖、检查与剩余边界

三路只读独立审查分别覆盖 Pi 契约、Opi/消费者和草案一致性，主审顺序复核高优先级源码。42 文件均已索引版本/角色；双语 spec 全文、指定 Pi/Opi 契约与 caller/迁移资料深入阅读，其他历史设计/评审按相关段落抽核。没有声称逐行重验所有历史引文，也没有审计全部第三方 SDK 内部实现。

已核对上述源码、包 manifests/exports、公开类型、相关测试源码、Cargo manifests/lock、限定 Git diff 和 406 项摘要。当前快照缺 `packages/ai/src/providers/data/` 生成数据，不能据此证明完整 bundled catalog 记录或可直接离线构建；本次未 hydrate、安装、import 或执行 Pi。

本次未运行 Rust/TS runtime tests、provider 请求、OAuth 网络或 paid/live 调用。供应商当前服务接受度、真实性能/计费、生产清理 race、外部 embedder/package 可用性均未由本审查证明。旧 70 个测试和 probes 仍只证明其历史输入。

文档检查：`python scripts/opi-doc-check.py` 为 PASS，`git diff --check` 为 exit 0；本次新增文件另行通过 UTF-8、尾空白、本地链接/源码行号范围检查，双语新增状态的标识符/数字/代码片段一致，56 AIR 保留。38 个未修改的原有文件与审查输入摘要相同。末次检查/共享 Git 状态另在 handoff 报告，避免把文档检查称为运行时验收。新增报告为中文研究，没有既有英文 counterpart；两版候选的状态同步更新。

测试影响：**none**，没有新增/更新/删除仓库测试。仅修改本目录文档/审查元数据；前一轮 Pi 1.0.4 退役和 Pi 1.2.0 报告等 carried-in 改动保持。未暂存、提交、创建分支、推送，HEAD 保持上述值。

# pi-ai / opi-ai 对齐候选 spec

> **修订草案 — 非规范性，2026-10-16。** 目标：在 `opi-ai` 中采用 Rust 惯用设计，实现固定 Pi AI 1.3.0 快照的公开能力与可观察运行时语义。这是供独立评审的文档候选，不是实施准入、ready-for-agent 批准，也不证明任何目标行为已经存在。对应英文版为 [spec-draft.md](spec-draft.md)。
>
> 本修订仅在本候选范围内取代早先的 Pi 1.0.0 子集和整项目表述。历史 D/Q 回答与执行证据继续作为历史保留。本修订不改变 Opi 的全局使命、正式规范、运行时、依赖、测试、CI 或可写会话格式。输入与评审来源记录于[revision baseline](pi-1.2.0-revision-baseline.json)；两次调查分别为[版本审查](pi-1.2.0-review.md)和[五问补核](pi-1.2.0-followup.md)。

> 本轮Rust工程修订的输入见[backend revision baseline](pi-1.2.0-backend-revision-baseline.json)，来源与精确crate identity见[source evidence](pi-1.2.0-backend-source-evidence.json)；前轮评审/verification不自动覆盖本轮改稿。

> 参数与新增限制修订见[逐项来源审计](pi-1.2.0-parameter-audit.md)及[本轮输入](pi-1.2.0-parameter-revision-baseline.json)。该审计优先于旧附件中的固定容量与拒绝规则；旧“复审关闭”不证明参数已标定或与 Pi 等价。

> Pi 1.3.0 升级：[来源／差异复核](pi-1.3.0-upgrade-review.md)固定新输入并替代已变化的 1.2.0 语义。AIR-060 / V-52 为退役身份，不是已通过门禁。Schema/WS 等 backend 缺口在当前后端附件中仍为 OPEN。

## 阅读与解释

先阅读范围、所有权和调用状态，再沿 AIR → 决策/证据 → [验收矩阵](acceptance-matrix.md)中的 V 导航。删除接口前查阅淘汰表。[README](README.md) 记录实际执行的检查；下文验收用例均为未来验证设计。

MUST 表示未来获得准入、实施本候选目标时必须履行的义务。SHOULD 允许有文档记录的例外，但必须保持所述行为和验证。MAY 表示范围内的可选行为。明确列出的证据阻碍不允许实施者猜测契约。人类确认选择的是目标，不证明实现已经完成，也不授予实施准入。

正式 `opi-spec`、AGENTS 和 CONTEXT 继续保有权威。package 级语义对齐不构成整项目移植／兼容要求，也不豁免真实消费者、状态机、依赖向内或 fail-closed 门槛。本研究不默许任何抵触规范条款的行为。User Policy、Package Trust、Capability Permission 和 Runtime Input Binding 保持当前含义；本候选不修订整个 run／会话的权威。

当前契约由本双语正文及明确引用的 [Pi 1.3.0 SDK contracts](pi-1.3.0-sdk-contracts.md)和 [Rust differences](pi-1.3.0-rust-differences.md)组成，并明确采纳[Rust后端与五项合同](pi-1.3.0-rust-backends.md)；唯一验收矩阵为 [acceptance-matrix.md](acceptance-matrix.md)。历史 TP/SP/EP/OA/D/Q 材料只在采纳且不存在冲突时提供保留的证据与详细算法。当前文本优先于旧选定子集、过时默认值，或旧稿关于全部目标决策均已闭合的说法。

共保留 67 个稳定 AIR 追踪 ID：57 项 active SDK 要求、9 项**仅宿主影响**（AIR-009、AIR-015–019、AIR-037、AIR-039、AIR-041），以及 1 项退役身份（AIR-060）。共保留 59 组 V：54 组 active SDK、4 组仅宿主项（V-17–19、V-33），以及 1 组退役身份（V-52）。退役既不是测试通过，也不产生新实施义务。混合组只含 SDK 机制和必要调用方迁移；本轮没有执行验收。

## 问题、目标与范围

历史探针已证明当前 `opi-ai` 存在路由选择、分帧、终态和 producer 清理缺陷。其日常调用组合、时间顺序、操作覆盖、SDK 可达鉴权和公开定制能力也与 Pi AI 1.3.0 不同。重命名符号或复用少量 wire fixture 不能证明调用方行为等价。

目标是一个可独立使用的 Rust SDK：调用方能够执行相同的选取／构造模型、提供 context、stream/complete、检查失败、追加工具结果、切换模型、管理 auth/catalog 和释放资源等动作，并明确语言表达与正确性差异。保留 known/unknown usage、保护秘密的 typed 错误、owned 调用状态，以及不隐式回退账户。不保证相同模型输出、JSON 字节、缓存收益、性能或账单费用。

范围包括：

- 固定能力清单中全部 10 个已发布内置 chat API、1 个 image API、4 个 classifier API、42 个内置 Provider 工厂／profile 和 9 类 OAuth flow，以及注册的自定义 Provider/API。复用 wire 不会抹去 Provider 特有的鉴权、默认值和过滤行为。
- Models/Provider 管理、按操作限定的模型选择、自定义完整记录、native/simple 四个 chat 入口、direct adapter、高级 prepare、最终结果、计时、诊断和显式清理。
- 模型输入 Context/TranscriptContext、时间顺序／section／工具变更、规范化、重放／opaque metadata、strict/grammar 投影和公开验证工具。工具仍是声明，由调用方显式执行。
- typed 图片／分类、实际支持的 classifier 图像输入、pending/deferred fetch/cancel、显式 retry helper、公开 header/payload/response/raw-event 定制，以及模型 metadata／估计／价格／输入限制。
- SDK 可达的 API-key、OAuth 和 AWS/default-source 机制；可注入 auth/store/presenter/clock/transport；catalog stores/refresh 和显式 personalization。构造不执行来源发现或外部 I/O。
- 显式 telemetry-context 传播，领域 instrumentation/backend 由宿主拥有；当前附件记录平台／package／CLI-helper 替代以及每项可观察 Rust 差异。
- 本 SDK 有界破坏性变更所必需的既有 Rust 调用方适配、独立 consumer/package 验收，以及版本跟进证据。

本 package 候选范围之外：Agent 执行／授权／hook 政策、持久会话 v3／子会话迁移／compaction、产品 tool_search/UI/启动默认值、完整 MCP/codemode 或虚拟路由产品、exporter／自动领域 span schema、尚未采用的 Responses Lite、未经验证的真实 remote-deferred 协议，以及推测性的通用 context/operation/plugin/transport 框架。宿主影响保持可追踪，但不成为 SDK 完成门禁。专用 OpenRouter Image API／图片流式处理不是 Pi 1.3.0 已交付 adapter，不凭空添加为等价要求。Native Rust 支持和 TS/Bun/browser/CLI 替代均明确限定，不静默漏掉能力。

## 角色与场景

1. 独立 SDK 调用方构造 Models 实例，注册内置或自定义 Provider，并选择或提供完整模型记录。
2. 调用方使用 native/simple stream/complete 或高级 preparation，观察到同一选定路由和最终消息／失败。
3. 调用方按时间顺序追加指令、声明和实际工具结果，序列化模型输入并切换 Provider，不虚构权威或缺失历史。
4. 流消费者获得稳定的事件时点数据和唯一最终结果，放弃工作时不泄漏库拥有的任务。
5. 调用方执行所有已发布 image/classifier 操作，包括按模型限定的图像输入，得到 typed 输出、已知计量和协议特定失败。
6. 嵌入方提供或管理鉴权，观察原子 token rotation 和 Provider 特定 availability，显式刷新 catalog，而无隐藏启动政策。
7. 持续调用 Codex 的调用方选择 transport/cache 行为，保留有效 continuation，并等待实例拥有的资源清理。
8. 调用方显式选择 whole-call recovery 或恢复 deferred handle，不自动轮询、执行工具或切换账户。
9. 宿主将自己的 tracing parent 传过 SDK 操作，不替换 subscriber、不导出数据，也不序列化 backend 对象。
10. 维护者依据固定源码身份跟进上游能力、规划和测试变化，并区分静态评审与已执行验收。

## 职责与状态所有权

| 责任方 | 职责 | 边界 |
|---|---|---|
| opi-ai Models/catalog | Provider 与按操作限定的记录、原子修订、唯一 preparation，以及冻结的 call/variant/attempt 状态 | 列表／配置不承诺远端可执行；不二次选模或回退账户 |
| opi-ai adapters/runtime | 协议编译／解析、有效 compat/options、typed 操作、计时和 owned transport/continuation 资源 | 不执行本地工具，也不担任宿主会话 writer |
| opi-ai model context/validation | Context 简写、canonical TranscriptContext、投影、schema 转换和公开验证 | 模型输入可序列化；请求视图保留 canonical 事实且不授予权威 |
| SDK auth sources/store | 可注入 AuthContext、API/OAuth/AWS 机制、按来源状态、串行 credential/metadata 事务 | AuthContext 不是模型／trace context；不在构造时发现来源、不隐式登录、不依赖具体 keychain |
| opi-ai telemetry module / 宿主 | 显式进程内 parent 加来源 Dispatch / 领域语义、subscriber、sampling、export 和 flush | 无自动 SDK 领域 spans、durable IDs、全局 backend 或上传 |
| SDK 调用方 | 追加消息／结果、选择 operation/retry/polling、保存输出并调用 cleanup | 无隐式文件、工具权威、隐藏轮询或产品 recovery |
| 既有 Agent/产品消费者 | 适配有界 SDK API 变更；保留自身正式执行／会话／权限契约 | 仅宿主 AIR 影响既不授权，也不要求新的产品／会话政策 |
| 研究／文档所有者 | 固定能力清单、差异、唯一验收映射和上游证据索引 | 无并行实施账本或自动采纳上游 |

一个 logical call 拥有捕获的 Provider/model/input 修订和一个选定 auth/source binding；普通 wire retry 复用它们。同时只有一个可执行 acquisition attempt 处于活动状态。可信 adapter 适配最多创建文档规定的有界 payload variants，每个 variant 保留不可变来源。它不切换 account/model/Provider、不把新输出拼进已交付流，也不重置 variant/send 预算。

创建 handle/Future 时无 I/O 地捕获 owned 输入。首次 poll 在等待 auth 前登记资源 generation。已知结构在 auth 前验证；合法 auth endpoint/header 材料随后决定依赖 endpoint 的 compat、最终验证和 payload 投影。每个 payload variant 执行一次 onPayload，并冻结变换后 payload，供该 variant 的普通 retry 使用。Anthropic 特定 strict-prefer 降级是有界新 variant，不是任意 invalid-input retry。功能 callback 与被动 telemetry 遵守不同失败契约。

SDK 无会话前提。它不要求 Agent authorizer、durable 事务、子会话、Promotion、宿主 v3 writer 或分发前 durable acknowledgement。durable 宿主可以按自身已准入事务契约消费冻结的 route/projection 事实；本候选既不改变该契约，也不安装第二个 writer。调用方的生成操作绝不修改 Context、执行工具或启动产品 refresh/recovery。

三种 context 责任保持独立：模型 Context/TranscriptContext（可序列化输入与规范化）、AuthContext（可注入来源访问）和 TelemetryContext（进程内观测）。构造／静态读取执行零 env/store/auth/network 工作。credential/telemetry/runtime 资源不进入模型序列化。参见 AIR-066 和当前附件。

## 要求：绑定、流与鉴权

每行包含责任方、触发条件、行为/失败及机械验收。AIR 证据的定位见导航附录。每行中的所有失败行为和不变量均属于验收范围。

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-001 | Catalog/collection/adapters；准备调用 | MUST 选择唯一有效的 Provider/请求模型/操作/wire、限制、声明能力、compat 和 effort 映射。鉴权前验证已知结构；鉴权后完成依赖 endpoint 的推断，并在 HTTP 前验证。Preflight、诊断和 adapter 使用同一绑定；退出 adapter 独立重新选择模型的路径。错误或不可执行的选择在分发前失败。 | O-002, route probe, SP-01; D-03/21/28; V-01 |
| AIR-002 | Catalog/collection；覆盖、替换或刷新 | MUST 原子发布完整的最近已知有效修订；已准备调用保留旧绑定。刷新失败保留先前有效数据。列表不承诺存在分发路由。并发/后台发布及缓存一致性遵守 AIR-033/034，不新增第二个权威来源。 | O-001/005, EP-01; D-03/23; V-01 |
| AIR-003 | Prepared/公共流；poll/EOF | 首个已交付终态 MUST 吸收、关闭内部 attempt 并永久 EOF。公共 ChatStream Done/Err(CallFailure) 是 logical-call 最终；高级 AttemptHandle 终态只属于 attempt，可重试 acquisition outcome 独立，不能转发成公共最终 error。异常 EOF 只产生一次 typed protocol failure 后 EOF。保留终态/partial message及已知 usage；旧 provider Error/transport Err 按 AIR-047 收敛，不转成功或丢消息。 | A-04/P03/06, UX-03/16; D-04/31/36; V-03/05/40 |
| AIR-004 | Stream/producer；取消竞态 | 在终止交付前观察到的取消 MUST 优先于缓冲输出，并交付唯一取消结果；已经交付的终止保持最终有效。共享调用取消阻止后续 attempt；receiver drop 结束其 attempt。验证同步后的可观测竞态，不猜测全局线程顺序。 | A-03/04; D-04 Q9; V-04/05 |
| AIR-005 | 每个生成 transport/producer；connect/send/read/backpressure/drop | MUST 保留有界 producer/队列并发及可取消背压，区分队列容量与可接受 frame/message/state 大小。现有 opi 队列计数64是内部调度选择，不是 Pi 行为或输入上限。不施加未经证明的默认字节/块数/深度 cap。合法大事件不能仅因超过队列 credit 而拒绝或死锁；独立 terminal 路径保留完整 partial/已知 usage。显式 caller 资源政策为 opt-in、冻结并 typed 报告；真实分配/backend 失败仍可观察，handoff 后不重发，也不保证 RSS。 | A-03/P07; retained D-04; V-04/05/55 |
| AIR-006 | SSE 分帧；任意合法分块 | MUST 跨任意合法 UTF-8/CRLF 分块正确 framing，增量解析并 checked 校验协议长度/CRC。Rust 或 backend 默认值不能推出新的默认 SSE/AWS/WS 容量上限。保留协议必需校验；保持各 Pi adapter 的增量 TextDecoder replacement 语义，不新增 fatal SSE UTF-8 拒绝。相关坏 JSON/protocol 失败 typed 脱敏；合法 keepalive/control 与大事件仍可接受。已配置资源政策与不可避免 backend 缺口分别记录。 | A-02/P06; D-04; V-02 |
| AIR-007 | SDK retry 所有者；auth/acquisition/variant transition | MUST 每个 logical call 只解析／冻结一次 auth，并使用唯一低层 acquisition 所有者。遵守协议默认值和显式覆盖；不叠加 transport/Agent retry。HTTP retry 复用冻结 variant，特定 strict 降级则有自己的有界 variant transition。send/wait 前观察 cancel/deadline；不在 handoff 后重放，也不回退 Provider/model/account。显式 whole-call retry 为 AIR-065，创建新的 calls。 | EP-11/12, UX-16, current SDK annex; V-05/12/33/39/57 |

## 要求：transcript、执行与重放

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-008 | Transcript；初始/变更指令或声明 | MUST 保留有序内容追加、命名 section 插入/替换/删除，以及工具添加/移除/重新定义。同名 section 替换保持位置；删除后重加则追加到末尾。Body 与 section 保持可区分。工具定义变更表示为移除+添加；执行实现 revision 归 AIR-009 宿主所有，不增加 SDK execution registry。无效/歧义名称或 schema 在安装前失败。等价状态不追加冗余变更。 | TP-01–03/P04; D-19; V-06/16 |
| AIR-009 | 仅宿主影响；Agent 执行修订 | 不属于 SDK 完成条件。SDK 按 AIR-045/049 冻结 owned 模型输入；execution registry/loadout 修订、当前政策撤销和工具批次授权仍归既有 Agent/宿主权威。本候选不选择新的 run/会话政策。 | Historical D-19/22; V-06/15/16 host portions only |
| AIR-010 | Producers/history/replay；路由相同或改变 | MUST 分别保留 provider、精确 wire、请求模型、报告模型、历史 ApiKind 和块绑定 opaque。Pi 1.3.0 在 provider/API/model 身份任一不同时丢弃全部 thinking，包括可读、signed 和 redacted 块，不把旧 reasoning 转为可见文本。同身份的 signed/redacted 空块保留，普通空白 thinking 省略。canonical 原件不变并记录投影事实；AIR-038 补充各家族 replay。 | Pi 1.3.0 transformMessages; V-07/32 |
| AIR-011 | Transcript 编译器；adapter 能力 | 默认 MUST 采用 native-or-collapse：对支持的变更类别保持时间位置，否则将当前有效 system/tools 放在请求消息序列首部。记录投影模式/受影响范围/来源前缀；不能将 collapse 称为无损，也不能改写规范历史。显式 Strict 在鉴权/网络前拒绝不支持的所请求时间保真度。单个未变更初始状态是无损的。不能用统一 native-transcript 布尔值代替 adapter fixture。 | TP-05, PR9548; D-19 Q22; V-07/16 |
| AIR-012 | Replay/Agent 恢复；工具关联 | MUST 用批次位置+来源 ID 建立关联；已关闭批次可以复用 ID。歧义重复/orphan/重复结果导致失败。可无歧义确定的缺失结果得到请求局部错误 ResultUnavailable 和 UnknownEffect，绝不是 NotExecuted、回滚或自动重新执行。由事实支持的畸形配对保留原件；wire 无法表达时使用显式失败上下文，不虚构 {} 或 null。只有 Agent 记录已知未执行的补偿。 | TP-09–11/R02–08; D-20; V-07/15 |
| AIR-013 | SDK 重放投影；失败／截断 assistant | MUST 保留 canonical Error/Aborted，同时从 wire 省略其失败请求视图和由事实支持、与之关联的未执行结果。Length 绝不授予执行许可。完整有效配对在可表达时保留；畸形／不可表达提议使用显式局部失败投影。已执行效果事实若矛盾则失败，不能令其消失。宿主 retry/执行政策不属于本要求。 | TP-09/EP-12; V-07/15 |
| AIR-014 | 图片 preflight/serializer；工具结果 | MUST 通过受支持的原生位置或已验证的替代消息位置发送真实图片，保持关联/顺序/错误；否则在分发前拒绝。MIME 占位符不算支持。共同验证媒体与模型+wire；保留现有用户图片，将私有工具细节排除在 wire 外。 | O-009/P14; D-06 Q7; V-08 |

## 宿主影响：持久化事实与迁移

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-015 | 仅宿主影响；durable 格式 | 不属于 SDK 完成条件。采用时间顺序模型输入的宿主须另行规定格式／版本／未知条目处理。本候选不选择 session v3，也不改变当前可写格式。SDK Serde 和验证仍为 AIR-049/066。 | Historical TM/D-19; V-17 host impact |
| AIR-016 | 仅宿主影响；legacy 会话迁移 | 不属于 SDK 完成条件。保留历史迁移证据；创建子会话、退出旧格式写入和用户接受 legacy 缺口须遵守所属会话契约，不是 opi-ai 前提。SDK 在消费者适配时不得改写历史 Provider/wire 事实。 | Historical TM/D-19; V-17/20 |
| AIR-017 | 仅宿主影响；durable acknowledgement | 不属于 SDK 完成条件。宿主 input/commit/cancel/projection acknowledgement 仍归既有唯一事务所有者。该所有者可消费 prepared SDK 事实；失败／取消不能使裸 SDK 要求 writer，或声称已经发生 durable dispatch。 | Historical TM-03/13; V-18 host impact |
| AIR-018 | 仅宿主影响；compaction | 不属于 SDK 完成条件。宿主 checkpoint/summary/branch/commit 政策保留正式所有者。本候选提供 canonical input/projection helpers，但不选择新 compaction 格式，也不保证压缩历史在时间上无损。 | Historical TP/D-28; V-19 host impact |
| AIR-019 | 仅宿主影响；resume/fork/export/evidence | 不属于 SDK 完成条件。Runtime Input Binding、分支重建、unknown/loss 呈现和 evidence acknowledgement 仍遵守既有宿主契约。SDK 类型保留 source/opaque/knownness；不引入新 export 协议、raw-wire 归档或子会话政策。 | Historical TM/TP; V-17/19 host impacts |
| AIR-020 | 公共 builders/adapters/调用方；输入规范化 | MUST 将初始 system/tools 简写规范化为唯一首部 transcript 状态；adapter 只消费规范化后的规范输入。便利构造可以保留，但下游独立运行时字段/读取者在同一次范围明确的破坏性变更中退出。迁移直接 adapter、mock、hook、compaction 和示例；旧会话读取者不能分发旧请求路径。 | TP-01/RT08; D-19; V-06/20 |

## 要求：compat、保留的安全保证与类型化操作

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-021 | SDK/调用方；范围明确的 0.x 迁移 | MUST 以可执行模型/鉴权/错误契约替代并淘汰 Config/ProviderConfig/config::Error/Result 和 Model/ProviderKind 外壳；SDK 构造不得依赖产品配置。保留 ApiKind/持久化消费者。迁移导出/文档/示例；删除仅测试构造器的测试，保留行为。不默认提供永久 shim。 | O-003/SP-16; D-02/08/21; V-10/20 |
| AIR-022 | Compat normalizer/config/parser；准备/迁移 | MUST 实现实际 Chat store 省略/false、依赖 reasoning 的 developer role、effort 格式/门控/off，以及 tool→user assistant bridge。使用最终 endpoint 推断，以唯一字段优先级解析，显式 false 与未设置不同。仅淘汰重复的静态 reasoning_effort；supports/bridge 名称只有在等价替代行为和调用方迁移完成后才能退出。已淘汰键在鉴权/HTTP 前报错，不能被 serde 忽略。 | SP-01–04/17/18; D-21/26; V-11/20 |
| AIR-023 | SDK Bedrock/source resolvers；选择 auth | MUST 独立于 coding-agent 提供可复用 SigV4/Bearer/profile/default-chain 和显式 custom-endpoint proxy-placeholder 来源，注入 source/HTTP/process/clock。保留来源限定的优先级、完整来源失败关闭、秘密安全和进程清理。现 opi process10s/64KiB 不是 Pi 默认，本次撤回其 SDK parity 默认地位；按固定来源机制对应，或明确 caller policy/backend 缺口。实现 full-chain/expiry fixtures；选定服务401/403后不重找身份。TUI/keychain 归宿主 adapter。 | SP-05–08 and verified AWS annex; V-09/20/55 |
| AIR-024 | SDK 参数 validator/error/usage producers；规范化或报告 | MUST 公开先 clone 再 normalize 与原 schema 全量验证，保留 raw 和 typed 脱敏失败。jsonschema0.28.3 是有条件引擎候选，不是已证明的 TypeBox 等价实现。dialect/keyword/format/ref/regex 行为须来自固定 Pi 路径及差分语料；不增加虚构 schema 容量/work/branch 拒绝、全 pattern guard 或隐式 schema HTTP/file I/O。backend 不支持的语义保持覆盖缺口，不得伪成功。Strict wire 不是本地验证或权限；usage knownness/cost 保持区分。 | TP-12–16/O-011/P17; V-12–15/35 |
| AIR-025 | Catalog/collection；操作查找 | 身份 MUST 使用封闭的 Chat/Image/Classifier 加 Provider+模型，并具有类型化操作元数据。Chat 查找只返回 chat；image/classifier 不虚构 chat 限制。共享现有 catalog/auth/preparation，不另建平行 ImagesModels/单例/通用 JSON 执行。显式 SDK 注册、内置记录和经验证的动态记录都是合法来源。错误类型/路由在鉴权/HTTP 前失败。 | SP-09/EVO-009; D-13/23; V-21 |
| AIR-026 | Typed operation runtime；generate/classify | MUST 冻结 request/model/auth 和按协议限定的 payload/options。Image 默认额外 HTTP retry0；共享 classifier helper2，保留协议排除/算法。callback/已解析结果无效不获得隐藏 retry。取消及显式或有来源的 deadline 保留唯一结果/已知计量。保持来源 timeout 范围，包括 headers 与 response body 区别；可选 caller 资源政策不变成包级默认限制。 | Current SDK contracts; V-22/50/51 |
| AIR-027 | OpenRouter image adapter；生成 | MUST 复现固定 OpenRouter 非流式 Chat image adapter 的输入、第一个 choice 解析、text/images 顺序、data URL 识别及坏 entry 处置。返回 owned 原始 encoded media/MIME 及可选 decoded bytes/status，不 URL fetch、像素解码、写文件或切换独立 API。默认不要求至少一图、container signature 匹配、四 MIME allowlist 或比 Pi 更严格的 base64。text-only/空 images 与非匹配 entry 的跳过按真实源码；更严格媒体校验仅为显式 caller policy，不称 parity。 | SP-10/SE-08; D-13/25; V-23 |
| AIR-028 | Classifier dispatch/validation；classify | MUST 通过 typed choice/bool/ordered-score 输入／结果支持全部 4 个已发布 classifier API：typesafe-system-one、cloudflare-workers-ai-system-one、llama-cpp-classify 和 openai-decisions。保留协议特定 envelope、label/score 限制、tokenizer/template/probability 算法、images、auth availability 和 failure/retry 规则。System One bool→noul 与 Decisions bool→predicate 不可互换。 | Current SDK contracts; V-24/50/55 |
| AIR-029 | 答案验证；概率/分数 | MUST 保留每个 classifier parser 真实的必需字段/类型/finite-number 校验。固定 adapter 未做的 probability/confidence[0,1]、请求 label membership、完整 probability keys、score domain、sum/argmax 或归一化，不增为统一拒绝规则。adapter 输入 label/levels 规则与 llama 概率算法分别保留。缺失/无效必需值 typed 失败，不填0；不声称 provider 数值已校准。 | SP-11/U-2; D-25 Q28; V-24 |
| AIR-030 | 操作结果；失败/成功 | MUST 区分 input/unsupported/auth/timeout/cancel/transport/http/invalid-result/resource-limit，并在业务验证失败后保留已知 usage/非秘密响应元数据。缺失/无效用量不同于 0；未报告的早期重试不是总账单。未知价格保持未知；普通诊断排除媒体/原始 body/秘密。 | SP-12/O-011; D-10/13/25; V-13/25 |
| AIR-031 | Budget/reader；编码/读取/解码前 | MUST 按来源审计区分 Pi 显式限制、协议要求、backend 约束与可选 caller policy。不得以无依据的默认 bytes/body/depth/count/总时长 cap，或统一 positive-only/no-unlimited 规则缩小 Pi 输入。各参数有意义的0/省略/sentinel分别保留；checked arithmetic 与分配失败仍 typed。显式配置的 caller policy 被捕获、检查并执行，不截断或伪成功；它不能授权新操作或目的地。有界调度不保证任意合法 retained output 的固定进程内存。 | SP-15; D-25 Q28; V-23–25 |
| AIR-032 | SDK 公开能力所有者；支持声明 | MUST 通过公共调用方路径、真实 adapter 加受控本地 transport，以及协议 fixtures，证明每项映射 operation/adapter/profile。无默认终端工具、启动请求、隐藏文件输出或权威扩张。factory/type 名称本身不是交付；共享 wire 仍需 profile-default/auth/filter 证据，单一 image wire 不是多 wire 证明。 | Capability inventory; V-21/22/26/49/50 |

## 要求：已发布路径与请求定制

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-033 | Catalog publisher/store；并发替换/刷新 | MUST 由同一个修订提交所有者负责内存和当前持久化快照。已被取代/取消的工作不能迟到发布内存或陈旧的下次启动数据；某 Provider 失败保留其先前数据，不清除其他 Provider。网络前恢复缓存，读取不被刷新阻塞。在 fetch/write/commit 屏障验证，包括不配合取消的迟到结果；只有 generation 检查不构成证明。 | EP-01/EPT-01; D-23; V-27 |
| AIR-034 | SDK catalog/store/source；显式 refresh | MUST 支持 cached restore 和显式 allow_network/force/provider 选择，构造时无 I/O。原子发布经验证、按操作限定的记录，保留用户覆盖及 Provider 特定 replacement/overlay 规则，拒绝陈旧 publication 并分别报告失败。Configured endpoint 和 OpenRouter personalization 为 AIR-064。产品 pi.dev source、启动调度／4h freshness 和 UI readiness 是宿主政策，不是 SDK 完成条件。 | Current catalog contracts; V-28/39/56 |
| AIR-035 | SDK Models/session 资源所有者；缓存/临时 WS/cleanup/drop | MUST 提供可 await 的 cleanup_session_resources(Some(id)/None)，按公共附录选择实例内精确 session/全部含 sessionless 资源。取消旧 generation 的 connecting/active/busy overflow，释放 sockets/tasks/timers/continuation/fallback/diagnostics，禁止迟到发布/retry/SSE；已交付终态保持最终。独立 Models 隔离；注入共享不转移外部所有权。Owned call guards 在 Models drop/replace 后继续持有冻结修订；最后公共 owner/guard drop 发起清理，不能把 Drop 称为已 await 完成。成功 result 可保留复用 pool；idle/max-age 遵守附录的单调时钟语义。 | EP-04/EPT-04, UX-15; D-23/30/35; V-29/42 |
| AIR-036 | Codex adapter；SDK/产品 auto transport/delta/恢复 | MUST 支持省略 SDK/产品 transport 默认 Auto 及显式 Sse/Websocket/WebsocketCached/Auto；sessionless/Disabled cache 仍可用临时 WS。先解析 effective transport：省略与显式 Auto 使用相同 delta 规则，修正 pi 局部未统一默认值的问题。Delta 要求非 input 字段相等，前一完整 input+实际 response items 是新 input 精确前缀，否则发送完整 input。只有成功终态+responseID 才发布 continuation。在公开输出前允许一次 missing-previous-response 完整重试；generation 开始前的 connection-limit 允许一次重连，两者是独立有界分支。普通 WS→SSE 只允许在首个有效 provider 事件前；abort/API/auth/protocol 失败绝不伪装成 transport 丢失。已交付输出后的错误结束该流；高层恢复独立；EOF 不是成功。 | EP-05/06/EPT-05, UX-14; D-23/24/35; V-30/39/42 |
| AIR-037 | 仅宿主影响；tool_search/loadout | 不属于 SDK 完成条件。SDK chronology/declaration/replay 机制仍为要求；产品 registrars、search 激活、pending reconnect names、runtime tool authority 和完整 MCP/codemode loading 保留各自范围。不得要求实现产品 tool_search 功能才能通过库 chronology 验收。 | Historical EP/TP; V-31 SDK declaration portion only |
| AIR-038 | Adapters/history/persistence；输出/延续 thinking | MUST 捕获并往返保存 Anthropic signed/redacted、Responses/Codex/Azure encrypted reasoning+item metadata、Google/Vertex text/thinking/tool thoughtSignature、Chat reasoning_details 及 Bedrock signatures/redacted bytes。保留 signed-empty、仅终止时出现的元数据、顺序和精确来源。同路由重放遵守实际能力；变更/未知来源或已过时的 disabled-thinking 规则以安全投影处理并记录事实。绝不将加密字节暴露为可读文本，也不虚构已丢失的旧元数据。 | EP-09/10/EPT-08/09; D-23; V-32 |
| AIR-039 | 仅宿主影响；产品／Agent recovery | 不属于 SDK 完成条件。宿主 retry 默认值、durable omission/recovery events、compaction 和 tool-batch replay 政策仍归宿主。Models 不自动执行 whole-call recovery。可独立使用的显式 SDK helper 为 AIR-065；其存在不要求 Agent 或会话。 | Historical D-24; V-33 host impact; V-57 SDK helper |
| AIR-040 | Schema/adapter constrained sampling；strict/grammar | MUST 在转换私有 wire schema 时保留可信 schema 和 per-tool intent。本地不支持的 prefer 可投影为 non-strict；require 不支持则在 dispatch 前失败。Anthropic 仅允许文档规定的 pre-response strict-limit 降级，保留 require 工具并记录 variant。Grammar function/custom IDs、wrapping/parsing 和不支持目标的投影仍按协议限定。不在 Start 后降级，strict wire 不授予权威。 | TP-14 plus 1.2.0 SDK contracts; V-34/05/36 |
| AIR-041 | 仅宿主影响；hooks/执行授权 | 不属于 SDK 完成条件。库的 raw/normalized 参数验证仍为 AIR-024/049；可信 validation 后 mutation、被拒提议的观察、最终实际值授权和零效果拒绝仍归执行工具的宿主权威。SDK 绝不安装 hooks 或执行模型工具提议。 | Historical D-20/22/28; V-35 SDK validation portion only |
| AIR-042 | SDK options/adapters；请求定制 | MUST 每个允许variant只调一次onPayload，随后验证JSON/wire/预算/受保护身份并冻结供普通retry复用。Strict降级只重建一次，不重选auth/source/model。低层retry:never防叠加；redirect/source retry按固定各路径规则，不新增blanket redirect:none。caller policy与受保护目的地/身份差异须显式。功能callback失败为终态，不被被动telemetry吞掉；不变更canonical、不授权工具、不需durable ACK。 | Current callback/variant contracts; V-36/05/34/51 |
| AIR-043 | SDK deferred dispatcher；submit/fetch/cancel | MUST 区分 Pending(handle) 与最终 assistant/error；handle 包含 provider/model/wire/token，以及适用的可选 expiry/poll hint，不包含凭据。默认 wait0 只检查一次，绝不隐式轮询。验证路由，对 unsupported/invalid/expired handle 报错；fetch 可以继续 pending。鉴权使用选定 adapter/来源；本地 fetch 取消不取消远程 job。远程取消尽力而为，不保证成功或退款。 | EP-13/EPT-12; D-23; V-37 |
| AIR-044 | Deferred 调用方 fixture；重开/检查/轮询/取消 | MUST 提供可重新打开的离线消费者：保存 handle/model/next-poll，重新打开后 fetch 而不 resubmit，pending 推进时间，只有 ready 才提交 assistant。Stop 释放本地调度器；显式尽力取消具有独立可用的取消预算，并报告失败。不新增持久化产品/crate。真实远程完成仍缺协议/账户/过期/幂等 fixture；伪实现证明不能声称支持远程 adapter。 | EP-14/EPT-13; D-23; V-38 |

## 要求：独立 SDK 调用与公共结果

补齐调用方契约，不增加另一套 routing/collection 权威。新符号是目标，不是已实施示例。

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-045 | opi-ai Models/provider；SDK 组装/调用 | MUST 将 collection 的状态/职责收敛到唯一 Models，提供直接 stream/complete/stream_simple/complete_simple。日常调用内部复用唯一 prepare/attempt/compiler；高级 prepare_call 保留同一状态机。同步 get_model(provider,id) 返回 owned ModelInfo（目标记录包含 provider_id、bare model id 和精确 wire）；完整自定义 record 在已注册 provider/wire 上也可调用，无需目录命中、不被同名目录数据覆盖。创建 call 时冻结 owned 输入/provider/model revision；无永久并列 facade/router。 | UX-01/02; D-30; V-39/42/01 |
| AIR-046 | SDK factory/auth/store；构造/读取/管理 | MUST 不依赖其他 Opi runtime，默认提供内存 stores。空 Models 不注册 provider；显式 builtin 装配包含选定 provider 的惰性默认 auth/env 来源，可被注入完全替代。构造/注册/静态目录读取零 env/auth/store/外部 I/O。check_auth/get_available* 检查配置/metadata，不刷新 OAuth；get_auth 可同源刷新；refresh 报告目录恢复/刷新；login 返回已确认 Credential，logout 只删除 store。管理附录固定 Option/Vec/typed failures/RefreshReport，来源失败不吞成空结果。Typed 每 call key/受控来源在 prepare 前选定。宣称支持的 OAuth/AWS 必须由 SDK 可达；终端/keychain 留产品。不自动登录/目录背景刷新/产品恢复/私有 executor；生成仍可在一次 auth 解析中刷新，无管理前提。 | UX-06/07/09/11–13, SP-06/08; D-21/30/35; V-39/42/43/09/12/27/28 |
| AIR-047 | ChatStream/终态 collector；complete/result/error | MUST 让 complete 与 result(self) 共用 Result<AssistantMessage,CallFailure>：Stop/Length/ToolUse/Deferred 为 Ok，Error/Aborted 为 Err。CallFailure 总含完整 canonical partial/terminal AssistantMessage 与 typed 脱敏 cause；setup 无生成内容且 usage unknown。stream 失败只交付一个 Err(CallFailure)；旧 Ok(Error) 退出。result 主动驱动剩余有界流并复用已交付 terminal 缓存，不重发/自取消。未迭代/部分迭代/至 EOF 均得到相同终态；无 terminal 按 AIR-003，无无限事件归档或被动 final future 死锁。 | UX-03, SC-04/05; D-31 Q32; V-40/03/05 |
| AIR-048 | 公共事件；生成/重建 | MUST 在可交错的 block start/delta/end 和 Done 前输出成功 Start；setup 可直接 Err，started failure 最后 Err。block index/顺序稳定，end 含权威完整内容/参数/附属元数据。partial 是 owned 不可变事件时点视图，不是 JS 共享可变 alias；先前事件不随未来输出变化。完整 text/thinking/tool、partial JSON、仅 terminal 的签名/opaque 及可选 arguments_error、provenance/usage/stop/deferred 在终态保留，不只保留最终文本。streaming partial JSON 仅为临时展示，最终参数错误不得以已解析前缀执行。 | UX-04, SC-06; D-30/31; V-41/07/32 |
| AIR-049 | SDK Context/options/validation；手工循环/模型切换 | MUST 接受初始简写和 canonical transcript，公开同一 normalize/resolve/tool-argument validation 机制。Serde 往返保留顺序/raw/opaque/provenance/knownness，不承诺 Pi JSON 字节兼容。Tools 仅作声明，caller 追加 assistant/result 后再调用；validation 不安装 hooks/权限。非空 arguments_error 往返保留且在 schema coercion 前拒绝；模型原始文本作为显式 owned 数据可取，默认错误格式仍脱敏。simple 支持 neutral effort/tool_choice(auto/none)/common options；native 支持所选 wire 的 typed 精确 options，错 wire 在 preflight typed 失败。两者接通既定 transform/observer/cancel/timeout 和同一 terminal/failure 合同；仅在 effective input/mapped options 相等的 fixtures 上比较结果。Length 不执行；Deferred 保持 pending。 | UX-05/08, SC-07–09; D-19/20/26/27/30; V-39/41/07/35/36 |
| AIR-050 | SDK package/调用方；独立使用与迁移 | MUST 验证 workspace 外 consumer，仅以 opi-ai 为 Opi runtime 依赖，覆盖公开调用、真实 adapters/本地 fake transport、双实例隔离、工具循环和离线 package 资源。说明 Tokio/Stream/Serde 要求与 native 平台限制。在有界破坏性变更中迁移旧 collection/error/当前消费者；不永久保留并行入口。TS/Bun/browser/CLI utilities 须有明确语义替代或平台差异，不照搬符号。 | SDK/Rust annexes; V-39–42/20/26/58 |
| AIR-051 | SDK OAuth/store；login/resolve/refresh | MUST 通过注入 presenter/store/source，使全部9类已发布 OAuth 可达，并保留各 flow 精确协议/callback/device/metadata/availability。caller 可取消等锁或停止等待；runtime 存活时已开始 refresh/rotation 仍由 store 拥有，成功前须确认 credential+metadata。Pi resolver15s 仅作用于 refresh 网络 signal，不覆盖 store 锁或持久化 ACK；不虚构总事务 cap。保留真实 expiry，并明确单次 Rust5min-window 差异、同源串行及 refresh 失败不回退账户。 | AIR-063 and current OAuth inventory; V-43/12/39/55 |
| AIR-052 | Built-ins/catalog/store/config 调用方；选择／迁移 | MUST 映射 canonical IDs google/google-vertex/amazon-bedrock/openai/azure 和其余所有固定 factory identity。OpenAI 默认 Responses，Chat 为显式替代；azure 拥有 azure-openai-responses 与 openai-completions，采用真实 deployment/endpoint 映射。保留历史 wire/source 事实和已经正确的 azure identity。显式可重入 config/credential-reference 迁移在 copy/ack/switch 前检测冲突；绝不合并账户、覆盖另一身份或永久保留 builtin aliases。 | Current profile/OAuth contracts; V-44/01/20/42/49 |
| AIR-053 | SDK estimator/native/simple preparation；预算 | MUST 使用最后适用的成功正 usage anchor，加后继 canonical 估计；遵循保留的 unknown/total 规则、prefix timestamp 失效，以及不限同模型／全文下界。以 UTF-16 units 和每图 4800 等效 chars，按 message 粒度使用 ceil(chars/3.5)。Simple 使用附件 4096/min1 和 adapter 公式；numeric-Claude 零值按wire实际处理，不新增 BudgetInsufficient。Native 保留精确 wire 的省略／默认／gating，不再做第二次全局 clamp。 | 1.2.0 estimator plus retained formulas; V-45/39/01/54 |

## 要求：显式 telemetry 集成

已选目标遵循 pi-ai 的传播职责，不照搬已移除的实验 Agent schemas 或未合并 exporter 提案。存在请求选项不证明已产生 traces。

| ID | 责任模块 / 触发条件 | 目标契约与失败 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-054 | opi-ai public options/Models/prepared/provider adapters；已选 SDK operation | MUST 公开可选 owned TelemetryContext，并在 native/simple stream/complete、advanced preparation、图片、分类和 deferred submit/fetch/cancel 中保留 frozen caller parent。simple mapping 与低层 retries 不替换/丢失它。缺省/显式 no-op 禁用本观测且不改变业务；无自动 SDK call/attempt spans。context 是进程局部对象，不进入 transcript/message/model/credential/catalog/session/deferred 序列化或 wire payload/headers。 | TC-01/07–09, TE-04/07/08, TO-03/08/09/16; D-37 Q37; V-46/39/42 |
| AIR-055 | opi-ai telemetry module/async handoff；显式 host context | MUST 通过 from_span 捕获匹配 parent Span 与来源 Dispatch，disabled/无来源成为 no-op；异步执行前冻结两者，不采用 poll 时 ambient，不初始化 global subscriber。在 producer tasks 及同步 Future/Stream poll/drop 范围传播该 Dispatch，不持 enter guard 跨 await。领域 owner 控制语义 status/settlement；返回 handle 或保留 Span clone 不代表业务完成。native tracing API 不宣称等同 pi 通用 callback/schema DSL。 | TC-02–06/11, TO-10–12/17–20, tracing0.1.44; D-37; V-47/42 |
| AIR-056 | Domain producers/host backend/evidence owner；观测开启/禁用/失败 | MUST 由 Agent/SDK 宿主拥有领域名字、typed/redacted fields 与终态解释，宿主拥有 backend/export/sampling/flush，既有 owner 拥有 durable evidence/diagnostics。默认观测排除原始 prompts/completions/tool data/payload/headers/credentials/自由错误文本。符合契约的 trusted backend 抑制 recording/export failure，不阻塞/重跑业务；不承诺隔离任意 subscriber panic/block 或 abort/OOM。telemetry 丢失不能供给权限、改变 Result/retry/usage 或隐去 evidence incomplete。 | TC-02/05/10, TO-04–07/12–15/19/20, CTRL-001/003; D-37; V-48/13/18 |

## 要求：完整 Pi 1.3.0 公开能力覆盖

当前 SDK 附件固定了按来源限定的能力清单；RDF 条目识别差异，绝不关闭尚未实现的能力。这些要求补充、而非替换保留的 binding/transcript/stream 规则。

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 验证 |
|---|---|---|---|
| AIR-057 | SDK public/factory/adapter 所有者；映射和调用能力 | MUST 通过明确 Rust 对应能力覆盖全部已发布 core/subpath 能力、10 个 chat API、42 个 factory profile、自定义 model/Provider/API 注册、selectors 和管理。即使共用 wire，也保留 Provider 特定 route/auth/default/filter 语义；unknown/unregistered wire 失败且不回退。每个原 export family 均有对应能力、有意差异或平台替代，不能静默留缺口。 | Current SDK inventory; V-49/58 |
| AIR-058 | Classifier/image adapters；typed 调用 | MUST 实现全部 4 个 classifier 协议、各自实际 image 支持／拒绝和 typed result/usage/failure，以及唯一已交付 image API。验证 bool 映射、llama tokenization/template/logprobs、Cloudflare envelope 和 Decisions API-key availability/504 排除。已知不支持的 operation/modality 在 dispatch 前 typed 失败。不虚构专用 image API，也不声称 live 成功。 | Current SDK operation contracts; V-50/21–26 |
| AIR-059 | SDK request hooks；定制／观察 | MUST 在适用公共路径提供支持 async 的 assembled-header transform、onPayload/onResponse 和已解析 raw-provider-event observation。固定合并顺序、per-call/variant/event 执行次数和 normalization 前观察。受保护 identity/header 修改须 typed 失败；callbacks 属功能行为，保留有界顺序／背压，绝不能作为被动 telemetry 吞掉。 | Current SDK callback contracts/RDF; V-51/36/05 |
| AIR-060 | 退役身份；Pi 1.3.0 移除 | 不再产生 SDK 交付义务。上游随实验性 consumer 移除 AssistantMessageFrame、encoder 和 reducer，不再仅为历史 parity 新增 compact-progress API。普通 stream events、不可变事件视图和最终 Result 仍按 AIR-048/047 保留；不选择新持久化格式。 | Pi 1.3.0 CHANGELOG/index/deleted source; V-52 retired, V-41 retained |
| AIR-061 | Chat response-stream 所有者；activate/finalize | MUST 提供可选 duration_ms，以单调时钟测量从 adapter response-stream activation/首次 poll 到其 terminal，包含 direct adapters。Models auth/loading 和未 poll 时间不在该 Rust 区间内。保留已有值和较早 deferred results；adapter 激活前的 setup 保持无计时。Tool duration 是可选宿主提供 metadata。序列化和转发保留已知值，不重新计算。 | Current timing/RDF contracts; V-53/40/41 |
| AIR-062 | Model/options/cost 所有者；选择 effort/载入 metadata | MUST 映射按有效 thinking-level 的 sampling，优先级为 model base < effective level < request，以及 operation input-limit metadata、compat 和 request-wide pricing tiers。对 calculateCost/helper 消费 adapter，使用 input+cache-read+cache-write 严格超过的最高阈值，应用于整次请求；OpenRouter image parseUsage 则保留 model.cost 基价、不取 tiers，这是明确源码例外；保留 1h subsets，不重复计算。保留 SDK 附件中由 response 决定的 service-tier 倍率，以及 Responses/Codex 对 default/fast 的不同处理；Azure 无此倍率。保持来源定义的 metadata 字段有效性/rates，区分 unknown 与 zero；catalog placeholder-price 处理是明确 RDF 差异。 inputLimits 是来源 metadata，不能仅因字段存在就在 SDK 增加 pre-dispatch 拒绝或 resize；实际执行归消费该政策的 owner。 calculateCost以matchedThreshold=-1起始，仅threshold>-1可选，同threshold重复保留首项；不为掩盖边界而加正数准入。 | Current metadata contracts; V-54/11/13/45 |
| AIR-063 | SDK credentials/store/OAuth 所有者；并发 auth mutation | MUST 提供对象安全、store-owned 串行 update/ACK 与 owned mutation_barrier，由 Models 委托。同一 store 跨 Models 的 cutoff/receipts 在同步构造时捕获；等待不启动 auth/logout，也不 abort 已开始 rotation。没有默认30s等待、256-key准入 cap 或 ADC60s 总预算；caller cancellation/可选 deadline 只管本次等待。诚实报告全部捕获结果/pending，保留 unknown receipts/late-write fence，明确 runtime/crash/跨进程限制；无账户 fallback/keychain/通用 shutdown 依赖。 | Current OAuth/store contracts/RDF; V-55/43/12 |
| AIR-064 | SDK catalog/store；refresh/personalize/restore | MUST 传递实际配置的 base URL，并提供显式 OpenRouter 按 key 限定的 chat filtering/limits/prices，不静默装入裸 factory。保留 nonchat 条目和文档规定的 no-key/401/403/404 行为。规定 Provider 特定 overlay/replacement/empty-list 行为和 stale-publication/account/cache 边界。Distribution schema v1 与 hydration data v6 不同；不声称已验证 pi.dev 部署，也不把 Rust crate 版本用作 Pi compatibility identity。 | Current catalog contracts/observations; V-56/27/28 |
| AIR-065 | SDK caller；显式 whole-call retry helper | MUST 通过新 logical calls 提供独立 retry classification/orchestration 和 callbacks，省略／禁用表示无额外 calls。绝不自动启用 Models/产品 recovery、重复已完成工具效果、合并 streamed output 或重置 prepared-call budget。保留 final partial/usage、cancellation、source/account 限制和协议特定 nonretry cases；不要求会话。 | Current explicit retry contracts; V-57/05 |
| AIR-066 | SDK public types/package；normalize/embed/extend | MUST 区分可序列化 model Context/TranscriptContext、可注入 AuthContext 与进程内 TelemetryContext。公开连贯 utilities、closed Chat/Image/Classifier 操作与开放 ApiId。typed options/dyn adapters 保留安全 call 所有权，不能在没有内在 Rust 必要性时虚构 catalog/API-ID 语法或 stale-handle 限制。backend 机制以来源/语料接受集为条件，逐项记录语言/平台/安全差异和未解 backend 约束，不造通用 JSON/plugin framework。 | Current SDK/RDF inventory; V-58/42/46–48 |
| AIR-067 | 文档／测试证据所有者；上游修订 | MUST 在既有 tracking/evidence 位置记录固定 Pi/Opi identities、变更的公开行为／规划状态、Rust 差异和 add/update/delete/retain 测试映射。仅在替代证据齐备后退出陈旧或 phase-freeze 断言；executed counts/commands/skip/gaps 与未来 V 设计保持区分。上游变更绝不自动授权 implementation、live、install 或竞争性进度账本。 | upstream-tracking and acceptance matrix; V-59/20/49–58 |

### 本候选采用的 Pi 1.3.0 来源变化

- 最终文本工具参数在五处变更 parser（Anthropic、Bedrock、Mistral、Chat、shared Responses）使用完整修复后的 JSON object 解析。空输入得到 {}；未完成／非 object 的最终输入得到 {} 加 arguments_error，绝不能按已解析前缀执行成功。临时展示、grammar、Google SDK object 和 custom/Pi Messages 权威输入各循原合同。\usepackage 等无效 Unicode escape 保留字面文本且不丢后续字段。V-35/41/49/58。
- Responses 的 configuration_update effort 由 supportsMidConvoEffort 加 truthy native effort 启用，并排除源码精确定义的 direct ChatGPT-sign-in 分支（apiKey 已定义含空串，不是 key truthiness）。历史 marker 要求同 provider/API/model 和已记录 providerThinkingLevel，跳过投影后空回复但保留重复 effort marker；始终追加当前 marker，即使位于 tool output 后或 effort 未改变。顶层 effort 仅在 model.reasoning 分支发出，在 sampling/onPayload 覆盖前以 medium 起始，marker／输出 metadata 保留变换前的映射值。flag 关闭、省略 effort、summary-only 和 simple off 保留原路径。Anthropic 支持 thinking-off 时禁用 thinking，不发 effort markers/output_config/block_binding 或 stamped providerThinkingLevel。V-16/32/49/54。
- Kimi Coding 改走 Chat Completions /coding/v1，保留独立 OAuth 身份；OpenCode Zen/Go generator endpoint 改为 /inference 与 /inference/go，Go 注册 Google，MiniMax 使用 Anthropic。保留显式 custom URL 的所有权，审慎迁移旧 builtin 假设。Gemini 4+ 使用源码 level-selection 分支；Xiaomi generator 意图仅 off/high 加 on/off toggle。快照缺 model catalog JSON，generator 默认与变更测试不是 hydrated record 证明。V-11/44/49/54/56。
- 同一 Chat chunk 的 reasoning/content 按 thinking 在 answer 前发出。Anthropic 与 Bedrock Claude 的 error-tool 图像以来源 caption/placeholder 放在同一 user message 的全部 tool results 后；成功结果和非 Claude Bedrock 保持原路径。显式 whole-call retry 分类补齐来源 gateway-forwarding 与 temporarily-unavailable 文本，不改变 retry 次数或自动启用 recovery。V-07/32/41/49/57。
- Telemetry source、tests 与传播 seam 不变；内部依赖精确 pin 是打包事实，不是自动 instrumentation。Rust 在创建时冻结 parent 加 Dispatch 是有意选择，Pi 持有的原 options 引用没有跨 await 不可变快照保证。Schema/WS 机制仍有条件，当前后端附件记录 OPEN 缺口与所需验收证据。V-46–48/58/59。

## 公共 SDK 契约（目标，尚未实施）

此处固定开发者使用的动作/名字，不固定文件位置。Models 收敛现有 collection 职责；prepare 保持高级机制。ModelInfo 是聊天记录，不是旧 Model 壳。目标 ModelInfo 明确添加 provider_id，id 保持 bare model id；catalog 记录与 custom 构造都必须给唯一 provider。注册时验证 provider_id 匹配拥有它的 provider，不按 id 反查或拼接推断；来源/响应模型字段保持区分。操作选择使用 AIR-025 闭合 enum/typed records，不建通用插件 trait。provider factories 可独立选择，显式 builtin 装配只列实际支持集合。

| pi 使用动作 | Rust 目标动作 | 输入/返回与责任 |
|---|---|---|
| createModels / Provider 管理 | Models::new / get_providers / get_provider / set_provider / delete_provider / clear_providers | 空内存实例，可注入 provider/auth/store/source；list/read/replace/remove/clear 由同一实例所有者负责；已捕获的旧 calls 保留其修订 |
| getModel / getModels | get_model / get_models | provider ID + model ID；同步 last-known 聊天记录，owned ModelInfo，未知 Option::None，零凭据探测 |
| *OfType / getAllModels | get_model_of_type / get_models_of_type / get_all_models | AIR-025 闭合操作选择；方法验证实际操作/wire |
| stream / complete | stream / complete | model record + Context + native CallOptions；stream 立即返回 owned ChatStream，complete 返回驱动它的 owned Future |
| streamSimple / completeSimple | stream_simple / complete_simple | 相同输入 + SimpleOptions 表达中立 effort/tool_choice；exact native 选项走原生路径，不静默 clamp |
| stream.result | ChatStream::result(self) | 消费 handle，主动 drain 至 Q32 Result；迭代为 Stream<Item=Result<AssistantStreamEvent,CallFailure>> |
| auth/refresh/login 管理 | check_auth / get_available / get_available_of_type / get_all_available / get_auth / refresh / login / logout | 显式异步；按下表区分配置检查、来源解析、目录刷新、登录/本地删除；不是生成前提 |
| 图片／classifier／deferred | generate_images / classify / stream_deferred / fetch_deferred / cancel_deferred | 同一实例的 typed 操作。Deferred submission 是返回 Deferred(handle) 的普通 generation，不虚构 submitDeferred API；stream_deferred/fetch_deferred 操作 handle。全部已发布 adapters 遵循 SDK 附件 |
| cleanupSessionResources | Models::cleanup_session_resources(Some(id)/None) | Owned 可 await 的实例资源清理；scope/取消/复用见下文；不删除 credential/catalog |
| 高级 preflight/attempt | prepare_call / PreparedCall::start_attempt | Owned AttemptHandle 至多一次 acquisition；共享下述累计 call budget/state，不套隐藏 retry loop；检查同一冻结 binding |
| request telemetryContext | options.telemetry_context | 从宿主 tracing parent 建立显式可选 TelemetryContext；所有已选请求路径同一 frozen context，无自动 exporter 或 SDK domain spans |

在创建 handle/Future 时捕获 owned 输入，不借用调用方 Context 或 mutable collection。鉴权/网络从首次 poll/await 开始：这是明确的 Rust 惰性异步时间差异，不可推迟输入冻结。使用宿主 Tokio，不建全局后台 executor。result 驱动 producer；不消费 result 而 drop 时按 AIR-004/005 结束 attempt。首次 poll 前 drop 零来源/网络工作。provider/store/资源共享必须显式；两个相同 provider ID、独立依赖的 Models 不污染全局状态。ChatStream 与 owned completion Future 为 Send + 'static，可进入宿主 Tokio task；Stream 不要求 Sync。

CallFailure.message 保留生成的 text/thinking/tool、来源、stop reason、诊断及已知 usage；cause 给出 typed 分类，不另交第二 terminal。Done.reason 等于 message.stop_reason。Err.message 为 Error/Aborted；Cancelled 映射到 Aborted，其余 setup/协议失败映射到 Error。收集不自动 recovery、执行工具或将 Length 视为执行许可；只有 caller 选择启用 AIR-065 whole-call retry，产品 recovery 政策另属一层。必要时保留版本化旧 serialized Error-event readers；当前 producers/dispatchers 退出双格式写入。

一次 call 的流程：捕获 → normalize/known preflight → 一次 auth resolution → 最终 binding/compat → 有界 per-variant transform/validation/projection 冻结 → adapter/events → 统一 collector。显式 key 在 auth 前选择并记录来源，绝不通过 payload transform 改变身份。宿主显式追加最终消息／工具结果或切换模型；SDK 不修改调用方历史。

## 工程附录：已固定的语义选择

### 管理返回、默认来源与会话资源

这是 AIR-046 的稳定返回/副作用契约；类型名称为目标，不是已编译声明。管理错误使用 typed 脱敏 ManagementFailure，不虚构 assistant；Q32 CallFailure 仍只用于生成。

| 方法 | 返回 | 工作与失败边界 |
|---|---|---|
| check_auth(provider) | Result<Option<AuthCheck>,ManagementFailure> | 读取配置/store 及来源声明的配置检查；过期 OAuth 仍 configured，不 refresh/login。Unknown/unconfigured 为 None；store/check 失败为 Err。不证明服务健康或 credential 新鲜 |
| get_available / get_available_of_type / get_all_available | Result<Vec<typed model record>,ManagementFailure> | 同一配置检查加当前 catalog/credential metadata 筛选；chat/type/all 不同，不刷新 credential/远端目录。Unknown provider 为空；真实 store/check 失败为 Err，不能吞掉 |
| get_auth(provider or model) | Result<Option<AuthResult>,ManagementFailure> | 解析选定来源，允许同源按需刷新；unknown/unconfigured 为 None。选定来源失败/不匹配为 Err，不回退 ambient/account；model 目标只合并合法静态 headers |
| refresh(options) | RefreshReport | 显式动态目录管理；allow_network 默认 true，false 仅本地恢复，无 OAuth/网络。报告 aborted 及 typed 每 provider 错误，保留成功发布；unknown/static/unconfigured 不启动网络，一个失败不抹其他成功。不是 credential-refresh 的别名 |
| login(provider,method,presenter) | Result<Credential,ManagementFailure> | 支持的 flow 仅在一次 credential/metadata store 确认后成功；unknown/unsupported/store/flow/cancel 为 typed 失败，不声称已回滚远端部分效果 |
| logout(provider) | Result<(),ManagementFailure> | 只确认删除选定本地 store 项；不远端 logout/revoke，不删除 env/catalog/history。删除失败为 Err；后续显式检查仍可能发现 env 来源 |
| credential_mutation_barrier(options) | owned Future<Result<CredentialMutationReport,ManagementFailure>> | 非变更的同store cutoff/receipt等待，同步捕获；无默认wait deadline，不新auth/logout/network，不取消已开始rotation。如实报告失败/pending与本地completion；shared-store范围按后端附件 |

Models::new 是空 providers、默认内存 stores 及惰性默认来源；set_provider 或显式 builtin 装配仍不读来源。环境只在获允许的实际 generation/get_auth/check/refresh 阶段读取。注入来源可完全替代 ambient；typed 每 call key 优先，否则 stored source 优先于 ambient，选定失败不回退。自定义来源检查可做其声明的 I/O；不能称所有异步管理为纯操作。

cleanup_session_resources 返回 owned Future<Result<(),ManagementFailure>>；有效但未匹配的 ID 成功无操作。首次 poll 确立资源 generation 截止点。Some(nonempty id) 选择此 Models namespace 中完整原始 SDK session ID；None 选择全部含 sessionless，Some(empty) typed 失败，不能按 truthiness 解释为全部。首次 poll 在任何 auth/transform/network await 前原子登记 call generation；cleanup 取消匹配的已 poll Codex auto/WS calls，含准备阶段及同一 call 正在进行的 SSE fallback，禁止迟到接入新 generation；没有这些资源的显式 SSE/其他操作及外部 stores/providers/clients 不受影响。匹配旧 calls 观察内部 logical-call cancellation，尚未交付终态时返回 Err/Aborted；caller token 取消只向内联动，cleanup 绝不取消 caller 的 parent token、另一 namespace 或后续 calls；释放 cached/temporary/connecting/busy-overflow 资源，并等待库拥有的本地 producer/connection 退出。成功完成只证明本地静止，不证明远端撤销/退款。本地清理步骤失败时仍尝试其余选定资源清理，返回 typed 脱敏失败及已知效果/未确认释放；不能把不完整清理报成功或恢复已取消 generation。

已开始 credential rotation 属于有界 credential-store 事务，不属于 Codex connection generation；连接清理不能中断它，也不能声称撤销其 credential。成功 connection cleanup 只证明选定 connection/producer 已静止。

清理可重入；截止点后的 call 和此前未 poll 的 logical-call handle 使用新资源 generation；已登记 PreparedCall 的未 poll AttemptHandle 仍属于原 generation。Drop cleanup Future 不撤销已提交的 close 标记；再次清理可等待未完成工作。已交付 terminal 保持最终；成功 result 不销毁 idle cache。Models drop/replace/remove 不取消 owned call：guard 保留冻结修订，最后公共 owner/call-guard drop 才发出资源释放信号。背景任务不能作为独立公共 owner 无限延长生命周期；Drop 发起关闭，完成屏障须 await cleanup。独立 Models 的相同 session ID 隔离；注入共享只释放本 namespace 的 lease，不关闭其他实例/外部 owner。不要求通用 cleanup registry/trait 或新的永久 shutdown 入口。

### 显式 telemetry context（Q37）

opi-ai 使用锁定 Rust tracing 生态，拥有小 telemetry module。本目标不新增 opi-telemetry crate、generic backend trait/schema engine、OTel/Sentry 依赖或默认 exporter。Agent 已依赖 opi-ai，可复用该机制，不把其领域/evidence 类型搬入库。standalone examples 说明宿主正常 tracing/runtime 依赖。

TelemetryContext::from_span(parent: tracing::Span) 从该 Span 捕获来源 Dispatch，不用后续 poll 时 ambient default；disabled/无来源是显式 noop。TelemetryContext::noop() 禁用；Clone 保留同一 parent/Dispatch，类型为 Send+Sync。宿主可主动传入 Span::current()，省略则从不捕获。非 noop 自定义 adapter instrumentation 使用显式 context/Dispatch；SDK 不替换成新的 call parent。已选 auth/management 接口不增加自动 tracing 要求；宿主可另行包围其 futures。

TelemetryContext::in_scope(FnOnce) 在捕获的 parent/Dispatch 下将同步 body 执行一次，返回前恢复调用方 context；TelemetryContext::instrument(Future) 保留其 output 与 lazy polling，在 poll/drop 使用同一 context。noop 在这些观测 helpers 内使用禁用的 scoped dispatcher，仍将业务执行一次，不创建 spans 或保留数据。这不静默宿主的独立日志或显式选择的其他 contexts。helpers 不创建领域名字/status，不超出 caller polling 驱动 Future；private Stream wrappers 使用同一 scoped 规则。

这提供 pi-ai options 接入语义，不提供源码级 pi-telemetry utility 兼容。native tracing 使用固定 callsite fields，没有通用 SpanStatus 或 callback-managed settlement。领域 instrumentation 使用自己的 typed vocabulary，显式解释 Result/Stop/Length/Deferred/failure，并将语义 scope 终结一次。捕获的外部 native parent 由宿主拥有 lifetime；SDK context 不能通过 Span clones 推断其语义完成。本候选不要求 generic startSpan wrapper 或新的 Agent span families。

tracing0.1.44 的 Future instrument/with_subscriber 不能单独覆盖 Stream 或所有 Drop 路径。执行观测时以 private synchronous wrappers 在捕获的 Dispatch 下处理 poll_next/drop 和 producer handoff；不持 thread-local enter/default guards 跨 await，不为记录而额外 poll 业务。noop 短路本 instrumentation，而不是在偶然 ambient subscriber 下创建 root。in-memory host/reference subscriber 验证 parentage 与结果保留，不用外部服务；不新增 production buffer/export API。

既有 Agent correlation IDs 由其 owner 解释，可作为已分类字段；backend span IDs 不是 durable identity。DiagnosticSink/AgentEvents/EvidenceSink 和 Session ack 保持独立。payload/response callback failure 仍遵守 AIR-042，passive telemetry 不吞掉它们。exporter setup、queue policy、flush/shutdown 是宿主职责，永不成为 Models 前提。conformance 针对可信非阻塞/nonpanic subscribers，不针对任意恶意 host code。

### 低层 chat retry 与有界请求适配

AIR-007 使用唯一 request-acquisition 所有者。`max_retries=N` 按每个 payload-variant acquisition episode 计算额外 sends：普通 HTTP acquisition 最多 1+N 次 sends。覆盖顺序为 per-call、显式 Provider profile，最后为协议默认值。Helper/Codex 默认 0；Bedrock standard 默认 2，受分类／quota 约束。Classifier/image 和原生协议遵循各自明确的附件默认值。关闭嵌套 retry；既有消费者的低层 loop 不得与此所有者叠加。本候选不选择产品高层默认值／配置。

Anthropic 特定 1.2.0 strict-prefer 拒绝可在成功 response handoff 前产生恰好一个 non-strict-preferred variant：最多 2 个 variants 和 2*(1+N) 次 acquisition sends。即使 N=0，也允许该语义降级。只有固定的 400 invalid_request_error grammar-size/tool-count/union predicates 允许降级；abort、callback failure、第二次拒绝或已开始 body 均不允许。Require 工具保持 strict，auth/model/source 保持冻结，onPayload 每个 variant 执行一次，onResponse 对接受的 response 执行一次。每个 variant 的普通 transport retry 复用其冻结 payload，不重复 callbacks。降级有独立有限 counter 和 diagnostic；它不是通用 HTTP400 retry。

每次 send/wait/variant transition 前检查共享 cancellation/deadline。SigV4 可从同一冻结 credential 更新 timestamp/signature，不能解析另一来源。成功 streaming-response handoff 封存 acquisition retry，即使尚无首个公开 content event；EOF/body/parser/callback failure 不重新进入该循环。Codex reconnect/full-input/fallback 分支仍为独立有界 AIR-036 transitions。任何低层分支都不自动重放已交付输出。

高级 start_attempt 无 I/O 地保留唯一 active lease。目标封闭 outcome 为 RetryableAcquisitionFailure(cause,retry_ready_at)、RequestAdaptationReady(variant_index,kind) 或 Final(Result<AssistantMessage,CallFailure>)。这些是目标名称，不是已编译接口。只有经分类且已观察到的 failure／允许的 adaptation 才准入另一个 attempt；caller 不能重置 counters/quota、跳过 backoff、创建第三个 variant 或复用封存 call。Attempt-scoped 中间 outcomes 不是公共 ChatStream terminals。普通 driver 在内部推进这些 outcomes，只输出唯一最终 Result。

提前／并发／封存／取消／耗尽后的 start 返回 typed 失败，零 dispatch。send 前 drop 释放 lease，不消耗 send。send 后 drop 且未观察到 retry/adaptation outcome 时，记录 UnknownEffect 并封存后续调度；不取消 caller 的 parent token。新 logical call 须显式重新 prepare；只有 AIR-065 显式 helper 可在无会话前提下编排新 calls。

| 族 | 显式启用重试时的准入与退避 |
|---|---|
| Anthropic/OpenAI Chat/Responses/Azure/Google/Vertex helper | 对其余请求失败，x-should-retry false 禁止、true 允许；否则 typed network/no-status 或 HTTP408/409/429/>=500 允许。retry-after-ms 优先于 Retry-After 秒/日期。无有效 hint 时 retry index i 从0起：min(500ms*2^i,8000ms)*(1-U[0,1)*0.25)。不能换成旧 opi1000ms/无 jitter/60s clamp |
| Codex HTTP | Typed 获取阶段 network errors 及 HTTP429/500/502/503/504 或已验证 transient response-body predicates，排除 auth/invalid/billing/quota。retry-after-ms 再 Retry-After；无 hint 为1000ms*2^i，无 jitter。保留固定 predicate fixtures，不复制 pi 可重发400/401的宽 outer catch |
| Bedrock standard | 只限 Smithy THROTTLING/TRANSIENT，按固定 error-name/trait/network-code 和 quota fixtures；裸408/409/501/505及终止性凭据失败不能仅按 status 变可重试。b=floor(U[0,1)*min(base*2^i,20000ms))，base为 transient100ms/throttling500ms；有效 server hint H 调整 delay 为 max(b,min(H_remaining,b+5000ms))。Standard quota 由每 logical call 拥有，初始500，retry cost5或 transient10，成功归还 acquired cost或1；capacity 为零禁止 retry，另一 call 从全新 quota 开始，不共享 provider-global quota |

max_retry_delay_ms 只保持 helper/Codex 来源范围：默认60000ms，非正cap只关闭server-hint上限检查（显式0仍保留hint）；坏 hint 忽略，有效过去/负 hint 为0；不是统一指数 delay clamp。撤回将其增为 Bedrock 默认拒绝：Pi未转发该 option，Smithy已有有界 hint 调整。按固定 Smithy header-selection 行为/语料，不另强制 Retry-After 胜过 x-amz。更强 caller policy 须显式，不是 Pi 默认。退避可取消/单调，测试注入 clock/RNG，不断言精确 wall-clock。

首批 AWS retry 目标为 standard 加显式 typed overrides；不在选定策略背后读取 ambient AWS_MAX_ATTEMPTS/AWS_RETRY_MODE/SMITHY_NEW_RETRIES_2026 或 shared-file retry knobs。Pi 锁定 Node SDK 会读取，且与 options.env 分离：如实记录覆盖差异，不把 scoped auth env 称为 retry-mode parity。显式不支持的 adaptive/experimental retry 请求在 preflight typed 失败，不静默用 standard。重新评估须有真实 SDK 调用方需要这些模式，及经核验 Rust 算法/权能契约；本批不增加推测性 adaptive framework/flag。


V-05 固定 per-variant override0/1/2 和总次数；strict max0 最多 2 次 sends/onPayload2，max1 最多 4 次，第三次 adaptation 为零 sends。保留固定 header/jitter/Bedrock quota 和 transient-name fixtures、retry-delay admission、冻结 auth/identity、新 SigV4 timestamps、cancellation/drop/deadline 及 handoff 后零 sends。V-34/36/51 分别验证 rebuild/callback 顺序和失败。任何 fixture 都不承诺服务端幂等或未知失败 attempt 的消费量。

### SDK OAuth、串行 credential rotation 与 builtin 迁移

AIR-051/063 覆盖 Anthropic、OpenAI Codex legacy、OpenAI ChatGPT、GitHub Copilot、OpenRouter、Kimi Coding、Meta、xAI 和 Radius。当前 SDK 附件固定每个 flow 的 browser/device/manual 行为、callback/port 政策、grant/refresh 字段、scopes、endpoint/account/model metadata、expiry 和 errors。这些必须是真实 SDK 机制，不能是空 traits 或仅委托产品登录。TUI、OS keychain 和 installation identity 持久化仍是注入的宿主选择。

Anthropic 首选 localhost:53692/callback，bind 失败时尝试空闲 loopback port，再降级手贴；保留其显式 copy-code 路径。Codex legacy 保留固定 browser/device/manual 规则，包括 bind fallback；OpenAI ChatGPT 的固定 1455 端口 bind 失败为 terminal，不采用该 fallback。通过公开 LoginOptions 保留 agentName 和所需稳定 device identity。Copilot enterprise/proxy/model/policy 行为仍按协议限定。按保留的 Rust safety 契约，已知 denied/expired device outcomes 为 terminal。

正常关闭使用 credential_mutation_barrier，连接 cleanup 范围不变。Pi auth/resolve.ts 在 store update 内仅给网络 refresh 传15s AbortSignal；这不是 lock-to-ACK 总时限。保持该网络范围，串行 login/logout/replace/refresh，credential+metadata ACK 后才成功。保存真实 service expiry，并明确保留单次 Rust5min near-expiry-window 修正；不称各 flow Pi 提前扣减时点完全一致。barrier 无虚构默认 timeout 或 receipt-count cap；caller cancellation/可选 deadline 只结束等待并报告 known/pending effects。Runtime Handle 不维持 runtime 存活；crash/shutdown/store 失败可能留下 unknown effects。不承诺远端 rollback/private runtime/keychain dependency/自动 login/额外身份 probe。

Builtin 迁移保留实际 IDs 和 wires：gemini→google、vertex→google-vertex、bedrock→amazon-bedrock；单一 openai 默认 Responses，Chat 显式选择。Canonical azure 保持 azure，提供 azure-openai-responses 和 openai-completions；deployment name 不同于 catalog ID。Responses 使用实际 v1 responses endpoint/body，Chat 使用经验证的 deployment mapping，而非重命名 Chat serializer。不得把已经正确的 azure 条目重命名为 azure-openai-responses。历史 API 事实和用户自定义 IDs 保留。

迁移先验证目标冲突，复制／确认显式选定的 credential/reference，以可重入方式切换当前消费者，并保留 source 直到 finalization 确认。不得合并账户、覆盖不同身份或增加永久 builtin aliases。未来迁移遇到冲突用户身份时需显式选择；本文不替用户选择账户。Mistral native、OpenAI ChatGPT OAuth 和其余固定的已发布 APIs/flows 均在范围内，不是延后缺口。

### 公共选项映射与当前 SDK 覆盖边界

Native 与 simple 共用终态契约，不表示相同 raw options 产生相同 wire。令 M 为显式 max，缺省时为 model.max；E 为 canonical estimate；W 为 context window。不增加统一正数/finite/整数预检；按各wire算术、0、truthiness与省略处理。W<=0 时 clamp(M)=max(1,M)，否则为 min(M,max(1,W−E−4096))；base C=clamp(M)，不再把显式 M 额外限制到 model.max。Numeric Claude（Anthropic/Bedrock）采用 common/custom B，F=clamp(min(C+B,model.max))，有效 B′=min(B,max(0,F−1024))：先加入 thinking，再做一次 context clamp。Common Chat budget fields 使用 B′=min(B,max(0,actual response ceiling−1024))，不做 Claude 的加法；B′=0 时省略 budget field，缺失 template values 按对应 fixture 处理。默认 B 为 minimal1024/low2048/medium8192/high16384；xhigh/max token budgets 映射为 high。Google/Vertex 不使用此thinking表或answer-room clamp，但simple仍先经共享4096/min1 base clamp。Native 不重复 collection-wide simple clamp：Anthropic 默认 model cap；Chat 未显式 max 时省略；普通 Responses 按 supportsMaxOutputTokens/ChatGPT Sign-in omission 对truthy max gating再设下限16；Azure Responses 独立将truthy max下限设为16，不套这些gates；两者显式0都省略字段；Codex legacy 不发送 max_output_tokens；Google 只发送显式 max；Bedrock Claude 默认 cap，其他模型只发送显式值。Pi Messages native/simple 仅转发 temperature/maxTokens/reasoning/cacheRetention/sessionId/toolChoice：simple 绕过通用 budget builder，省略 max 保持省略，显式 max 原值转发；不序列化 metadata/arbitrary sampling/thinkingBudgets/deferred。所有目标映射仍为候选，尚未实施；当前 Azure identity 为 azure，包含两个已发布 APIs。

answer room 不是低 ceiling 下无条件1024保证。撤回 B′=0 的新增 BudgetInsufficient preflight：Pi Anthropic numeric 通过 `||` 将0变1024，Bedrock通过 `??` 保留0，Chat可能省略0；分别保留这些来源映射、省略和真实失败，不做第二全局 clamp。ceiling1/1024/1025/2048 是语料输入，不是接受下限。声称协议拒绝须另有证据；源码未统一验证每个 maxTokens 为正整数。

Google预算顺序为 minimal/low/medium/high；custom thinkingBudgets 优先，Google3 level 模式不发送 numeric budget。下表是实际 wire baseline，不声称所有行有独立设计意图；GenAI/Vertex 的 flash-lite 差异保留可追踪 fixture，未来核验约束后才修订。

| Wire/model | Numeric budget / answer room |
|---|---|
| GenAI 2.5-pro；Vertex 2.5-pro | 128/2048/8192/32768；无 common 1024 answer-room clamp |
| GenAI 2.5-flash-lite | 512/2048/8192/24576；无 common clamp |
| GenAI 2.5-flash；Vertex 2.5-flash（当前包括 flash-lite 匹配） | 128/2048/8192/24576；无 common clamp |
| Google/Vertex 其他 budget 模型 | -1 表达协议的 dynamic budget，不是资源无限哨兵 |


在支持的 native/simple wires 中，per-effective-thinking-level sampling 使用 model base < effective-level entry < request keys。Input limits 和 request-wide cost tiers 遵循 AIR-062 及精确 SDK 附件，包括 OpenRouter image 的基价例外；payload 定制不能替代这些字段。

保留已确认的支持等级选择和 native exact 能力验证。以 reported anchors 估计 canonical transcript，不用于账单。Q35 撤回同模型／全文下界限制。计量 pi UTF-16 code units，每图 4800 等效 chars，以 message 粒度除以 3.5 并向上取整：user/toolResult 全部 blocks 汇总后一次取整；assistant 的全部 text/thinking/tool-name+serialized-argument chars 汇总后一次取整。System 先渲染 body+sections 再一次取整，另分别累加 JSON toolsAdded 和 toolsRemoved 的取整估计。不得对每个短 block 独立取整。最后一个非 Error/Aborted、timestamp 不早于其 prefix 的正 usage 提供 anchor：优先已知正 reported total，即使不同于 bucket sum；total 缺失/0 时，只在 input/output/cacheRead/cacheWrite parents 全部已知时使用其正和。绝不以 0 填 unknown buckets，也不重复加 children。保留 adapters 的独立 reported total；这不改变 parent/subset billing。估计 anchor 后的 messages；无 anchor 则估计全部 messages，保留 Rust 数值差异。V-45 分别覆盖 total≠sum、total=0 fallback、partial unknown 和 prefix timing，不以 normalization 隐藏。

Pi cache none 对应 Rust Disabled；SDK retention 省略时，仅对具有该默认的 wire 使用 Short，受选定 scoped-environment long override 影响。Pi Messages 保留 retention 省略态，由 backend 默认决定，除非显式 long override 生效。当前 Rust None 表示 Provider 默认，不是 pi none；任何保留的高级 native 默认都须区分。Caller headers 支持设置／移除未受保护默认项，并在 auth 后合并；identity/protected headers 仍按 AIR-042 失败。AIR-059 包含动态 assembled-header transform、onPayload/onResponse 和已解析 raw-provider-event observation。AIR-060 随 Pi 1.3.0 移除 compact frames 而退役。SDK 附件固定它们的调用顺序、protected-field 差异、失败与 per-variant/event 次数。在选定 adapter 支持时，raw observation 发生在 normalization 前；不支持路径严格按映射失败／报告，不能虚称存在通用 raw transport event。普通 progress events 与最终 settlement 仍分离，无需通用事件框架。

### 上下文与恢复投影

内容块以换行拼接；非空的按时间顺序 body 追加以空行拼接。有效 system 文本由 body 后接按插入顺序排列的非空 section 构成，以空行分隔。Null 移除 section；空字符串保留身份，但不贡献渲染文本。规范数据单独保存 section 身份，与带版本的请求时 updated/removed 包装分离。工具 name/description/schema/sampling 定义声明；execute/display 不定义声明。规范对象键比较绝不重新排序数组、section 或声明。

在目标 wire 重命名前，先按历史批次+来源 ID 解析工具关联。请求局部重命名必须为双射并保留引用，包括清洗/截断后发生碰撞的情况。待完成批次中的上下文变更保留规范位置；请求编译先用已知结果或 ResultUnavailable 关闭该批次，再投影被延迟的状态变更。这是显式顺序适配，不是虚构工具执行；不能仅因 adapter 需要这种 pi 已支持的转换就拒绝。

畸形提议有可信 NotExecuted 事实、且无法忠实表达时，在因果位置为每个受影响批次使用一个普通 user-role 失败上下文块。版本标记为 `[opi:tool-call-not-executed:v1]`；使用 JSON 转义的允许列表 `calls`，包含 `batch_call`、`tool_name`、`reason_code`。封闭 reason 集合：invalid_json、non_object_arguments、schema_validation_failed、output_limit、interrupted_before_execution、not_active。不包含原始错误/参数/策略/详情。缺失结果补偿则只说明没有可用结果且效果未知；标记文本本身不能创建任何一种可信处置。请求发起者（包括 Copilot）由规范用户所有权决定，不由合成的最后一个 User 块决定。完整有效配对保持结构化，包括 Length 未执行恢复。

混合 text/thinking/tool 内容保留各 wire 支持的顺序。Responses 保留调用之间的文本片段，并区分 item_id/call_id；Chat 保留两类子序列，并报告跨类别交错被折叠。不透明数据/签名投影与时间折叠分别记录事实。精确规范重建不能跨越 legacy-unknown，普通继续操作则可以接受该缺口；接受缺口不隐藏后续投影损失。

### Plain-JSON 规范化与采样

Rust 的公开 schema 是 JSON；不存在 TypeBox 私有标记来选择第二套类型强制转换算法。保留原始 JSON/字符串和解析状态，在规范化前复制，并单独记录有效执行。已有的精确 Rust 整数值无需人为引入 JavaScript 解析阶段的精度损失；在差分 fixture 中记录此表示差异。字符串到基本类型的转换采用有限的 ECMAScript binary64 Number/String 语义，保留 raw；这不是对任意精度字符串的无损转换。

| 规则 | 固定行为与机械语料 |
|---|---|
| 可选 null | 仅当值为 null 的已有可选属性，其独立原始子 schema 拒绝 null，且没有 string $ref 时，删除该属性；否则保留。递归处理 properties/items，不做通用 union/ref 解析。测试 required、nullable、ref、nested 和 extra null。 |
| Number/integer | null→0，bool→0/1；非空白数字字符串接受 ECMAScript 数字语法，但必须有限，integer 还必须为整数。无效转换保留原值，交由完整验证处理。包含正负号/指数/十六进制/二进制/八进制、空白、Infinity/NaN、溢出和 2^53 边界。 |
| Boolean | 仅允许 null→false、精确字符串 true/false 及数字 1/0。字符串 TRUE/1/0 和数字 2 对 boolean 仍无效。 |
| String/null | null→空字符串；number/bool 使用 ECMAScript String，不用 Debug 或对象序列化。Null 目标只转换空字符串、0、false。包含 −0、1e21、1e−7、数组和对象。 |
| Type 数组/union | 已属于允许的 JSON 类型时保留。否则按序尝试基本类型转换。先按顺序处理 allOf，再 anyOf，再 oneOf；union 优先保留已通过某个完整分支验证的值，否则为每个分支复制，选择首个转换后通过验证的分支。完整 schema 的最终验证仍执行 oneOf 唯一性约束。 |
| Objects/arrays | 只转换已有 properties、schema 值的 additionalProperties、items 和 tuple 位置。不填默认值，不删除禁止的额外字段，不将 scalar 包成 array，不将 string 解析成 object。 |
| 最终检查 | 完整验证原始可信 schema，包括 root/enum/约束；无效 root 不能通过把 raw 返回为“已验证”而逃过检查。失败类型化/脱敏，且零执行。Hook 只能在此阶段后修改；最终授权使用实际值。 |

Strict wire 转换要求 object root；递归将所有属性设为 required，将原先可选且不可 null 的字段改为可 null，设置 additionalProperties=false，保持可信 schema 不变。拒绝 pi 转换子集不支持的 references/definitions/allOf/oneOf/conditional/dependency/pattern/tuple/结构化已有 anyOf 情况；精确语料和完整关键词列表见 [TP-14](transcript-target-proposal.md)。Adapter 特有的不支持关键词参与 prefer/require 判断。未指定/false 采样不产生新要求。修订 Q31 确认 grammar：Responses/Codex 优先选择非空 lark，再选 regex；不支持 grammar 的目标使用普通 JSON 工具并记录投影。支持的目标拒绝缺少可用 variants 或无效 object schema；该 schema 必须恰有一个 required string 属性。验证 custom-call 原始文本包装和 function/custom ID/output 关联，包括仅追加 delta 和已关闭输入的修改错误。本地 JSON schema 验证不证明 grammar 正确性，也不证明调用方 variants 之间等价。

高层 effort 顺序为 off/minimal/low/medium/high/xhigh/max。Nonreasoning 支持 off；排除显式标为不支持的映射；xhigh/max 要求声明支持。请求级别可用则保持，否则先向上、再向下搜索。记录 requested/effective；native/exact 不能静默调整到支持范围内。基于预算的 adapter 通过固定 Provider fixture 应用附录可满足的回答空间公式/来源限定零值处理及实际 off/null/省略 wire 差异，不使用通用字符串 effort 字段代替。

### Compat 与鉴权

有效记录只解析一次，不从已被覆盖的记录重新导入字段。字段默认值顺序为：adapter 基线推断 → 显式 Provider profile 默认值 → 显式最终模型覆盖。同一层的显式 role 优先于自动 developer support；已有模型 role 覆盖优先于 profile role。保留未设置与 false 的区别。Chat support-store 表示发送 store:false 或省略；Responses store 是独立 wire 设置。Developer support 表示只对 reasoning 模型使用 developer，不是对所有 assistant 使用。Bridge 是请求局部 tool→user 转换，包括工具图片替代位置，带固定 assistant 标记；它不是规范 assistant reasoning。Thinking 意图/值映射/格式/门控/off 是不同职责。Provider 特有格式和推断使用 [SP-01–03](sdk-target-proposal.md) 语料，包括最终鉴权提供的 endpoint。

AWS 选定来源表和精确依赖证据见 [SDK 附录 A–D](sdk-target-proposal.md)。冻结的目标规则：

- 显式类型化来源只选择一次；互斥显式选择报错。Bearer 与 SigV4 不同时附加。Auto 区分显式/作用域内 profile 与环境 profile；显式/作用域内 profile 优先于注入的环境凭据对，单有环境 profile 不优先于完整凭据对。不完整的已选/作用域内凭据对失败，不拼接环境中的另一半；这是保留的 opi 保护，也是显式的 pi 验收差异。
- Default-chain 发现遵循已验证的 Env（有 profile 则跳过）→ 显式 SSO → INI → process → web identity → ECS 或 IMDS。INI 覆盖 role/source-profile/static/web/process/SSO/login-session，包括 config/credentials 字段覆盖及 role/MFA 选项。只有类型化、允许继续的发现失败才推进；缺失可选来源不同于选定来源的终止性/鉴权失败。SSO 无效会话/MFA 缺失/web-token IO 错误按已记录规则停止；process 发现失败可以继续。选定凭据后的 HTTP401/403 绝不启动另一轮身份搜索。
- Profile/metadata/token/process/clock 来源可注入，且独立于产品受到支持。有效且未过期的缓存凭据可以在接近五分钟过期窗口、一个同来源刷新进行时继续返回；已过期凭据要求重新解析。遵守来源报告的有效过期时间，并在来源提供时保留原始过期时间/stability 来源记录；中立 resolver 不虚构延期。所有本地等待和陈旧刷新发布都有边界及清理。文件刷新必须具有显式失效机制，不能依赖永不失效的模块 promise；不要仅因 Node 链可能如此就重复运行同一 process。
- SSO/login 刷新及获准缓存写入是真实来源行为，不能声称只是只读文件。离线验收只以伪 transport 执行 STS/OIDC/ECS/IMDS。导入时绝不发现来源。SDK DefaultChain 必须能通过公开的伪组装调用到达，不能只存在于内部测试；不复制 pi 已配置列表遗漏 default-profile/IMDS 的行为。
- 通过显式 typed `ProxyPlaceholderSigning` 保留 Pi 已发布的“不持有真实 AWS 凭据调用代理”动作：要求 caller 提供非标准 endpoint，并可显式选择 HTTP/1.1 transport。冻结的 mode/endpoint/region 使用固定非秘密 placeholder SigV4，不是无签名 `NoAuth`，也不代表已认证 AWS 账户。即使 ambient 存在真实凭据，也零读取 environment/store/profile/process/metadata/bearer 凭据；普通 AWS endpoint 拒绝此模式。这是相对原始 `AWS_BEDROCK_SKIP_AUTH`/ambient 覆盖的明确安全差异，不是延后能力。V-09/49/58 通过普通 SDK → 真实 adapter → 本地代理验证 HTTP/1.1、placeholder Authorization、secret canaries、固定身份及取消／重试；不新增通用 transport/auth 框架。

恢复鉴权使用同一选定来源/目标。允许已确认属于同一账户的凭据刷新；已知 principal/来源切换停止。凭据实质改变，却无证据标识其仍属于同一选定账户时，以 identity-unverified 错误停止自动恢复；不能静默假定，也不能额外发起身份探测网络请求。Resolver 可以提供稳定且非秘密的账户绑定；仅附加方案相同不足以证明账户相同。无论刷新是否完成，旧调用鉴权保持冻结。

对于目录 404/501，保留当前已恢复 overlay、同时使其来源失效，是观察到的 pi 行为。固定版本实现中，后续恢复是否撤下该 overlay 取决于内置 generation。AIR-034 将类型化失效 tombstone 设为权威，即使没有该 generation 也生效；这是显式一致性修正，不是对 pi 精确行为的断言。

### 操作参数与资源政策来源

只有有来源的默认值才保留其准确范围。来源审计区分 Pi 显式值、协议要求、backend 默认、现 opi 选择和已撤回 Rust 提案。可选 caller 资源政策省略时不启用；这不代表 runtime/provider 资源无限。各参数的0/省略/dynamic sentinel分别保留。本轮不另猜替代 cap。

| 项目 | 有来源的目标／修正 |
|---|---|
| 图像媒体/count/MIME | 撤回8 images、16MiB/image、32MiB total 和四 MIME 接受门槛。按固定 adapter 的 data-URL/base64/MIME 表达对应；严格媒体政策为 opt-in，不是默认 parity |
| 图像 text/request/response | 撤回64KiB text、48MiB bodies 及虚构 nonempty-input 拒绝。保留来源输入/first-choice 解析，不增统一内容 ceiling |
| 图像验证/output | 保留 text-only/空 images 成功和实际非匹配 entry 跳过；撤回无 Pi 依据的必须有图、container-signature/nonempty-byte/strict-base64 校验。不 URL fetch、pixel decode 或写文件 |
| Classifier 输入 | 撤回统一1–64 questions、1MiB/depth32。保留实际 per-adapter 输入规则，包括 llama choice2–62/score2–10 和 Decisions images128；不泛化某协议校验 |
| Classifier 响应 | 撤回2MiB 和统一 probability/confidence[0,1]、label-set/score-domain 校验。保留真实 required-field/type/finite 检查及各 adapter 算法；不新增 sum/argmax/归一化 |
| Timeouts/retry | Image600000ms 通过 OpenAI7.19.0 剩余 attempt timeout 覆盖非流式 JSON acquisition/解析；image extra retry0。共享 classifier helper extra2；native Mistral60000ms 仅 headers。保持各协议来源范围和显式 caller timeout；无包级总 deadline |
| Catalog refresh | 保留来源限定 refresh/restore/filter/publication。OpenRouter personalization15s 有来源；产品4h freshness 是宿主参考，不是 SDK deadline |
| Codex WS | 保留 Pi connect15s（显式0关闭timer）、release后idle timer5min；max-age55min在acquire检查；仅next-acquire清idle为明确Rust生命周期差异、Auto/cache 分支和 HTTP extra retry0。撤回 close<=2s 及虚构 frame/message/outgoing cap；不可避免 backend handshake/compression 限制仍为未关闭覆盖约束 |
| 显式 SDK whole-call retry | 省略/disabled 不增加 call；启用 helper 按固定 classification/count/backoff 和60s delay cap。产品 enabled/3/2s/60s 是宿主政策，不是 SDK 默认 |
| Deferred check | Pi wait0 检查一次；caller 拥有 polling。reopenable fixture 的5s 是测试输入，不是 SDK polling 默认或下限 |

### Backend 候选与五项工程合同

采纳[后端附件](pi-1.3.0-rust-backends.md)，但以[来源审计](pi-1.2.0-parameter-audit.md)为条件。保留有用的源级机制选择；撤回无依据容量/拒绝及 blanket equivalence 结论。backend 默认只证明该 backend，不能授权缩小 Pi 接受集。

| 模块 | Backend／未完成义务 |
|---|---|
| HTTP/TLS/WS | 有条件保留 reqwest0.12.28/rustls0.23.40/ring0.17.14、tokio-rustls0.26.4 与 tokio-tungstenite/tungstenite0.28.0 provided-stream 机制。来源限定 redirect/proxy/TLS/protocol 及 handshake/compression 缺口须 conformance；H1-only/none 政策不是通用 Pi 默认 |
| AWS | 保留 owned sources 和 STS/SSO/OIDC/SignIn、aws-smithy-query/xml0.62.1、xmlparser0.13.5、p2560.13.2/sha1 0.10.6 与现 hmac/sha2。撤回虚构 XML/Query/source 大小/总 send 预算；保留合法协议数据、来源 retry、完整 credential 和秘密安全 |
| ADC | 私有五家族 adapter 与受控 reqwest/ring/pki-types1.14.1 以完整来源语义为条件。撤回60s/16sends/depth4/1MiB、zero-retry、same-origin5-hop 默认；来源 retry/redirect/process/config 必须可达 |
| Schema | jsonschema0.28.3 是需 Pi TypeBox corpus/adapter 的有条件候选，不再称已完成 parity 选型。撤回强制 Draft7/unknown-format-keyword 拒绝、虚构 work/shape 配额和全 pattern guard；不引 JS runtime 或隐式 schema I/O |

#### 流调度与可接受数据

撤回默认 SSE8MiB、AWS8MiB/headers128KiB、接收 WS8MiB/message16MiB/outgoing16MiB、retained32MiB/4096 blocks/depth64、tool/opaque8MiB、queue64MiB 和 terminal-envelope256KiB。现 queue64 是内部 count bound，不是新拒绝阈值；实施背压/一个 in-progress event，以及不因大事件卡死的 handoff，不增加 retry driver。独立 single terminal 路径保留完整 partial/已知 usage，不依赖不足的 queue permits。

合法 retained output 必然随接受内容增长；不保证固定 RSS，也不静默截断。增量 framing、checked 协议长度、取消/drop/task cleanup 与 immutable shared snapshots 仍必需。显式 caller policy 测 B−1/B/B+1；省略 policy 时，更大的合法事件/输入不能新增已撤回 SDK 失败。backend size/recursion 约束须配置消除或保持明确未解覆盖缺口，不能藏为默认值。

#### Store-owned mutation completion

CredentialStore::mutation_barrier(options) 和 Models::credential_mutation_barrier(options) 返回 owned Send+'static Future 与 report/failure。同步构造捕获同 store 跨 Models 的 cutoff/active/unconfirmed receipts，按 source-generation/resource-key/revision 区分；后任务不延长。无默认30s wait/256-key cap 或 ADC60s 总事务预算。Pi resolver15s 只管网络 refresh，不使 store lock/ACK 过期。caller cancellation/可选 wait deadline 返回 pending，不取消已开始 rotation。

停止新 auth producer，await barrier 并检查报告后再关闭 Tokio。保留 unknown receipt、同 key reconcile、late-write fence 和真实 persistent/跨进程/runtime/crash 限制。回收 confirmed 且无引用记录；内部调度不得淘汰 unknown effect 或虚构资源准入拒绝。连接 cleanup 仍独立。10s refresh 加6s ACK 不是15s-total 失败；网络 refresh 超来源 deadline 仍失败并保留 known/pending effects。

#### 来源限定 transport 与 trust

只有一个显式 transport/retry owner；关闭嵌套 retry，不改来源限定的实际 sends。撤回 blanket redirect:none/仅 same-origin ADC GET<=5/16-total-send 的 parity 默认。复现固定各路径 redirect status/method/body/header stripping 和 retry；不向未授权目标泄漏 credential 或切换 auth identity。更强宿主 routing policy 须显式且记录接受差异。来源/backend 证明缺失是缺口，不是统一3xx失败。

保留 scoped proxy/NO_PROXY、TLS 验证及实例所有权，不改全局 env/provider/subscriber 或绕过。WS Upgrade 与显式 proxy-placeholder 的 H1 不表示所有 Bedrock 流量 H1。撤回 CONNECT64KiB/124-header 和 close2s SDK 政策；tungstenite handshake/feature 限制为需补救或未解覆盖的 backend 约束，不证明选型完成。cross-origin/redirect/proxy fixtures 计合法 sends 并验 credential stripping/source binding。

#### Schema 语义与执行

目标是真实 Pi plain-JSON validation/normalization 路径。TypeBox 此处不按$schema选择 dialect，忽略未知 format，支持自身已知 format registry，ref 在内存解析。不能改成强制 Draft7/三 dialect/unknown-keyword-format/cycle 拒绝，也不能默默弱化支持约束。无隐式 schema network/file lookup；URI identity 不禁止。strict wire projection 仍独立于本地验证。

撤回 schema256KiB/16384 nodes/256 targets、instance8MiB/262144 nodes/depth64/pattern4KiB、work1000000、cache128/32MiB、jobs4/queue16/errors32。fancy-regex1000000 backtracks 是 backend 默认，不是 Pi fuel 或已证安全默认。撤回 cross-product regex 预检/canonical-key 重写：可能拒绝无关分支或改变约束。候选引擎 format、ECMA regex、keyword/ref/recursion 及吞 regex error 的缺口须 branch-correct 差分证明或重选 backend；不得伪成功或 blanket fail-closed parity。内部有界调度/淘汰不能仅为 tuning 数字拒绝合法 schema；同步取消不是 hard-kill。

#### Custom API 类型与所有权 seam

保留 O 已固定的对象安全 ChatApi<O>/ImageApi<O>/ClassifierApi<O>、typed native options 和同一 ordinary/advanced driver。ApiId 保留 Pi 精确字符串 identity，不虚构 normalization/length/grammar。handle options 携带必要 API/type tag，不是实例 capability。兼容 registered adapters 的 Models 间可复用同一合法 option value；同类型替换不因 revision 单独过期。

创建 call 时冻结选定完整 record/adapter。错 operation/type 或实际 unregistered API 在 auth 前失败；后续替换/删除不改变旧 call。不施加与类型安全无关的 blanket foreign-namespace/stale-handle 拒绝。trusted Rust custom code 非 sandbox。V-01/21/39/42/49/58 验 portable compatible options、换 type 失败、旧 call 所有权，无第二 registry/driver。

## 替换与退出要求

下表穷尽本候选有意淘汰的项目。未来范围明确的 0.x 变更同步 Unreleased、消费者、schema、fixture、当前文档和中文版；标注为历史的材料保留。每行都必须满足所述退出证明；不永久保留重复实现。

| 现有职责/依赖者 | 处置及替代 | 保留/改变的行为与迁移 | 过渡、测试与退出证明 |
|---|---|---|---|
| 第二次模型查找；mapped/direct adapter、factory | 替换为 AIR-001 最终绑定 | 保留 model/wire/auth 语义；有效覆盖现在实际执行。影响 prepared adapter 契约、picker、compaction 和外部 SDK Provider | 一次范围明确的破坏性变更；无平行 router。新增捕获路由测试，更新 prepared conformance；静态证明不存在第二次独立查找，并保留回滚 |
| 独立请求 system/tools 读取者；builders/hooks/adapters/mocks | 合并为 AIR-008/020 规范化/时间序列 | 保留便利初始输入；下游唯一 transcript 加入命名变更。迁移公开 Request/message、hooks/session/compaction/示例 | Builder 仅作为规范化入口保留；所有消费者编译。新增时间线测试，更新 fixture，删除重复权威断言；证明不存在旧 adapter 路径 |
| Run 内固定 registry/loadout／按可见性执行；Agent/宿主 | 仅宿主 AIR-009 影响 | 保留当前正式执行／权威契约；SDK 接受并冻结 owned context、报告 projection | 不选择新的 run/会话政策。只适配被破坏的 SDK caller 签名；宿主 revision 变更须有自身已准入范围 |
| Deny-only hooks 和最终执行授权；宿主 | 仅宿主 AIR-041 影响；SDK 提供 AIR-024/049 normalization/validation | Raw/normalized 数据与可信 schema 无需安装 hooks 或执行工具即可获得 | 保留宿主 permission/denial 测试；声明 SDK 完成无需重设计 hook-order/authorizer |
| Config/Model/ProviderKind/error 外壳；公开导出/示例 | 通过 AIR-021 已确立的 SDK 契约删除 | 保留实际 route/auth、ApiKind 和持久化消费者。外部 0.x import 有意破坏；不依赖产品配置 | 无默认 shim。删除仅构造器测试，保留行为；编译仓库消费者，扫描当前旧导出/文档至零 |
| 静态 reasoning_effort 和重叠 compat 名称；config/parser/factory | 删除重复值；只有 AIR-022 替代完成后才合并真实行为 | 保留 store/role/gating/bridge。已淘汰键报错；迁移有效 profile/model 合并及当前 schema/README，不隐含移除会话格式 | 删除前先有新 wire 证明；同一 release 退出旧读取者。新增旧键负例 fixture，更新 wire/schema；只删除无效果测试；唯一 compat 所有者 |
| AWS 当前本地链/文档；库/产品消费者 | 修正/扩展可复用 AIR-023 resolver，保留签名 | 保留安全性；env/profile 优先级改为已选规则，添加类型化来源/完整 default-chain 覆盖。不迁为产品独占；修正有误导性的旧“无 I/O”文档 | 未明确改变时保留来源格式；新增来源/过期/伪 HTTP 测试，保留 process/SigV4；不能将仅本地能力称为完整链。不破坏性转移 SDK 功能 |
| 可写会话格式／子会话迁移；会话宿主 | 仅宿主 AIR-015–019 影响 | SDK message/opaque/knownness 变更可能需要 consumer/read compatibility；本候选不选择 v3 writer、旧格式 append 退出或子会话迁移 | 保留旧字节／readers 和当前正式宿主行为。新 durable migration 另属范围；库 Serde/consumer 测试仍适用 |
| 有损/所有权游离的流路径；SDK/Agent | 修正为 AIR-003–006 自有生命周期 | 保留有效事件/内容；改变已证明的损坏/EOF/drop 结果 | 无旧行为开关。新增对抗 fixture，保留正常语义；证明 inner/producer 释放及永久 EOF |
| 产品 recovery 默认值和 durable omission 政策；Agent/宿主 | 仅宿主 AIR-039 影响；显式 SDK helper AIR-065 | 保留宿主语义，同时公开可选 SDK new-call orchestration，不拼接输出或执行工具 | 不选择新产品 defaults/events/compaction workflow。既有 SDK-consumer 变更不得叠加低层 retry 所有者 |
| 既有 SDK 调用方拥有 acquisition retry；Agent/summary/SDK | 唯一 AIR-007 acquisition 所有者与显式 AIR-065 helper | 保留 cancellation、typed failure、冻结 auth 和如实 attempt usage；有界 caller 适配仅移除重复 acquisition scheduling | V-05/20/57 证明无叠加 sends，并验证必要 consumer 编译。本 package 候选不重设计产品 config names/defaults、whole-turn/summary recovery 或 durable policy |
| ProviderCollection 日常组装/调用；Agent/产品/SDK | 收敛为唯一 AIR-045 Models | 保留 registry/auth/catalog/prepared 状态；日常方法隐藏组合，高级 prepare 复用机制。迁移调用方/根导出/crate 双语 README/examples；改名本身不改变持久格式 | 同次 scoped 0.x break；V-39/42/20 验证消费者编译/公共路径；旧并列 collection/re-export/router 退出，无永久薄 facade |
| CompletedRequest、AssistantStreamEvent::Error 及 provider/transport 双错误；adapters/Agent/summary/mock/evidence | 替换为 AIR-047 Result/CallFailure | 保留完整失败 partial/usage/来源/typed cause；当前事件只 Err。同步 producer/drain/terminal 匹配/脱敏转发/fixtures/docs。版本化历史事件转换保留原事实 | 先 V-40/41/03/20 覆盖所有终态和 >64 收集；再迁移所有 consumers，旧 wrapper/当前 Error variant/双写路径归零。保留历史只读兼容；当前 calls/writes 全用新契约 |
| 当前产品 OAuth flows/refresh；SDK/产品消费者 | SDK 可达的 AIR-051/063 九类 flow 契约，注入宿主 adapters | 保留 keychain/TUI 所有权和 atomic ack，修正当前 rotation/expiry/callback/metadata 行为；增加其余已发布 flows | V-43/55 公共 login→fake HTTP→store→request 和 same-store concurrency/rotation faults 通过后，重复 SDK resolver/flow code 才退出 |
| 内置旧 provider ID/default wire、全局simple预算；config/store/catalog/SDK/产品 | AIR-052/053 canonical ID/真实默认adapter/统一mode estimator | 保留明确Chat adapter/profile、历史来源和unknown；迁移config/key冲突、defaultResponses、ModelInfo.provider_id、输出预算/估计helpers及docs | V-44/45先证明defaults/wire/迁移重入及预算；当前旧默认aliases/独立estimator/双clamp退出，历史只读与custom IDs保留；无静默账号覆盖 |
| 旧选定能力缺口、固定 wire/profile 清单和 phase 范围 guard | AIR-057–067 完整源码映射覆盖 | 以 Rust 对应能力保留有价值的公开 utility/state-machine 语义；不通过符号改名或收窄已发布 adapters 虚假宣称支持 | V-49–59 和 41+7 测试处置清单建立替代后，过时 presence/count/module-freeze 断言才退出；不按名称整文件删除 |

## 验收依赖与完成条件

1. 固定范围／来源清单 → 验证：固定 Pi 1.3.0 identity、全部 capability/profile/flow 行、正式权威、57 项 active SDK、9 项仅宿主与1项退役 AIR 处置，以及明确差异；无隐藏排除的已发布能力。
2. Binding/streams/typed outcome → 验证：V-01–05/39–42 的 route identity、framing、cancellation、variant budgets、>64 收集，以及独立 task/socket 释放。
3. Context/validation/replay → 验证：V-06/07/15/16/31/32/34–36 的 SDK 部分和 V-58；无需 tool executor 或 durable writer 即可验证 chronology、schema/grammar、signatures 和 projection。
4. Auth/catalog/options/resources → 验证：V-09/11–14/27–30/43–45/54–56、精确协议默认值、shared-store rotation、当前 canonical identities、pricing/sampling 和显式 cleanup。
5. 公开 operation/customization 能力面 → 验证：V-21–26/37/38/49–51/53/57，通过真实 adapters／本地受控 transport 验证全部 APIs/profiles/flows、images/classifiers/deferred/callbacks/timing 和显式 retry orchestration。
6. Telemetry/package/native 集成 → 验证：V-46–48/58 和 workspace 外 consumer/package 证明；实际 operations 中 parent+Dispatch 匹配，无隐藏全局 I/O 或 exporter，runtime/platform 替代明确声明。
7. 迁移和上游证据 → 验证：V-20/59，每项退出 assertion/symbol 均有替代证明，实际受影响 Rust consumers 编译通过，当前支持文档与 tracking 记录准确。不暗含产品／会话重设计。
8. 独立文档评审 → 验证：双语 requirements、当前附件、唯一 acceptance matrix、历史替代和全部 source/ID references 一致。文档检查通过；未决重大选择明确列出。Runtime/test 实现与正式准入仍独立。

全部 67 个 AIR IDs 和 59 组 V 保持可追踪，但只有 57 项 SDK 要求和 54 组 SDK 验收定义本 package 的未来交付。AIR-009/015–019/037/039/041 与 V-17–19/33 只保留宿主影响／历史；混合组只要求其 SDK 部分和必要 API-consumer 适配。AIR-060 与 V-52 保持退役，不计入这些有效数量。仅宿主缺口不能被静默计为 SDK 失败或完成。

差分输入为固定人工 fixtures，记录两方 source versions、实际 test entry/filters 及 executed/skip/missing counts。只归一化无关 clock/request IDs 和保留引用的双射 ID 重命名。保留字节、顺序、事件时点 partials、terminal/cancel/error、accounting knownness、callback/variant/send 次数、关联、签名和投影事实。明确 RDF cases 是差异，不是被归一化消失的失败。离线 fixtures 不证明确定性模型输出、真实服务接受、性能、账单或远端停止／退款。

本次文档工作不实现测试，也不运行 runtime suite。旧 70 个 Rust tests/probes 保留历史基线；它们不使新验收组通过。未来执行遵循实际 change-scope verification union 和唯一 matrix，包括经审查的 Pi mock-only 选择、隔离 HOME/env、禁止 credentials 触发 live，以及不静默安装／hydrate。

## 证据导航与剩余边界

| 契约 | 修改时阅读 |
|---|---|
| Rust后端选型与五项合同 | [当前后端附件](pi-1.3.0-rust-backends.md)；字节预算、mutation屏障、redirect/retry/proxy/TLS、Schema与typed custom API；选定版本不构成编译/准入证明 |
| 当前完整 capability/default/protocol/flow 映射 | [Pi 1.3.0 SDK contracts](pi-1.3.0-sdk-contracts.md)；有源码支持的精确公开行为与明确 utility/platform 替代 |
| Rust 差异、caller 影响和证明 | [RDF-01–20](pi-1.3.0-rust-differences.md)；语言／资源／正确性／平台差异与真实覆盖缺口的区分 |
| 唯一机械验收和测试迁移 | [V-01–59](acceptance-matrix.md)；54 组 SDK、4 组仅宿主影响、1 组退役，以及 41 integration+7 source-file 处置；均不是已执行验收结果 |
| 当前输入／修订和独立评审 | [参数baseline](pi-1.2.0-parameter-revision-baseline.json)、[参数复审](pi-1.2.0-parameter-revision-review.md)、[README](README.md) |
| 审查发现与补核范围 | [版本审查](pi-1.2.0-review.md)、[五问补核](pi-1.2.0-followup.md)；发现仅适用于各自固定的修订前输入 |
| 人类指示与历史选择 | [decisions](decisions.md)；当前 package 方向／范围在本候选中取代旧选定子集，不重写历史回答 |
| 保留的详细 transcript/JSON/strict/AWS 算法 | [TP](transcript-target-proposal.md)、[R replay](replay-contracts.md)、[SDK annexes A–D](sdk-target-proposal.md)；仅在不抵触当前契约／RDF 时采纳 |
| 历史跨 package／会话／产品提案 | [TM migration](transcript-migration.md)、[EP/EPT](evolution-target-proposal.md)、旧 SDK/OAuth/telemetry reviews；作为支持历史，不增加 SDK 交付范围 |
| 规划与后续上游修订 | [upstream tracking](upstream-tracking.md)；固定版本证据，区分 accepted 与 discussion/experimental/shipped/unknown，记录测试处置和明确再评估触发条件 |

文档评审后，候选仍为非规范性且尚未实施。源码映射、连贯契约和 reviewer findings 可以形成可评审提案；只有实际 runtime/consumer/conformance 证据才能证明交付。既有 D/Q 选择支持保留语义，但不能制造对每项新技术推导的批准。当前后端附件只定位机制版本；Schema/transport接受约束仍有条件，按本轮参数来源审计保留未关闭证明；未来仍须完成Cargo解析、锁定传递闭包、platform/supply-chain审查和可执行conformance。源码选型不能产生runtime证明。

获得真实 submit/fetch/cancel/expiry/account 协议时重新评估 remote deferred；核实已接受／已发布契约变化时重评 Lite；存在真实消费者及选定 schema/lifecycle/data policy 时重评 telemetry 提取／instrumentation；已发布源码／adapters 支持时重评新 operations。不因此引入推测性框架或自动采纳上游。未来范围变更和实际规范冲突需要人类权威；普通、有源码支持的实施选择不重开已定 package 意图。

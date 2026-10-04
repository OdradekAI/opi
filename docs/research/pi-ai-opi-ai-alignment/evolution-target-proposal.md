# 已落地演进能力的目标契约提案

> 非规范性候选提案，2026-10-03；不是实施准入、正式规范或 ready-for-agent。依据 [D-18及第二轮决策](decisions.md#第二轮目标决策d-18之后)：本轮以 pi 的设计意图、已落地行为与可靠演进为目标依据，opi-spec 仅作可能过时的参考。**Q26/Q27已明确确认本提案的覆盖、pi产品默认、恢复身份和分期（D-23/24）；Q22–31及修订Q31均已回答。** D-18本身不构成批准，具体确认来自这些后续回答。本文的EP保留设计推理与证据，候选要求以双语spec为准，唯一综合验收映射见[acceptance-matrix](acceptance-matrix.md)。

本文沿用 `docs/CONTEXT.md` 的 Agent Core、Reference Product、Extension Ecosystem 术语，不建立新权威或进度账本。中文研究提案没有既有英文 counterpart；双语目标契约最终由主候选同步，本文件无需复制英文研究副本。

## 使用方式和证据边界

先读 [第二轮决策](decisions.md#第二轮目标决策d-18之后)、[逐题重审](decision-reassessment.md)、[演进复审 RE-01–06](reassessment-evolution-notes.md)。本文回答其“应该实现什么、谁拥有状态、怎样证明”的后续问题，不重复完整历史。EP为本提案追踪ID，不是第二份验收权威；下文机械验收是输入设计依据，统一执行判据映射到acceptance-matrix的V-27–38及相关既有V项。证据定位集中在末尾EPT表。

基线是 pi 1.0.0（2026-10-01），本地 ai 包的 378 个 blob 已在此前核对官方 tag commit `a13d35a742c6ef8462812a28fbe1d8c8b7431c32`；不是整份 monorepo 同一性证明。后续证据仅复用 [EVO](design-evolution.md) 已核验的固定窗口 `11449730c8a733953ce1bcce70e066bccaa778a5`，不声称今天 main 最新状态。本文本轮只读本地源码/测试/CHANGELOG，未新增联网、未运行 runtime 测试。

状态含义：

- **目标清楚／已确认候选**：证据和可观察契约已形成目标，并由D-23/24及相关第二轮决策确认；仍不是已实现或正式实施准入。
- **确认后的分期**：可选tool_search及产品接线纳入，完整MCP/codemode和真实remote deferred不在本轮候选交付范围；这是明确能力差距，不是实验或不存在。
- **远端证据缺口**：公共生命周期可设计，内建远端协议未被证实；不以 opi 无消费者否定它，也不宣称已打通远端。

## 可评审的覆盖与依赖

| 覆盖层 | 条款 | 前序依赖与完成条件 |
|---|---|---|
| 公共语义与离线边界 | EP-01、07、09、10、11、13 | transcript、模型/操作身份、typed error 与 canonical terminal 的主候选先固定；各条离线公共行为验收通过。不是先构造通用任务框架。 |
| 接口实际连接与状态生命周期 | EP-02–06、08、12、14 | EP-01 先于目录产品刷新；EP-09/10 与请求序列化先于 WS delta；EP-07 先于工具发现与恢复；失败消息持久化/投影先于高层重试；EP-13 先于 deferred consumer fixture。各条同时验证生产 owner 接线和公开结果。 |
| 待协议证实的远端覆盖 | EP-14 的远端部分 | 取得具体 provider 的 submit/fetch/cancel/expiry/account 规则与离线 fixture 后才承诺 adapter；不能把 mock 通过当作远端后台推理已交付。 |

这是依赖顺序，不是允许无限期只交库接口的退出借口。若将第二层排到另一次产品工作，候选 scope 必须列出未覆盖的已发布 pi 行为及其 owner；不得继续写成“未来或实验能力”。已确认分期不要求新增 opi-durable、通用 Transport trait、全局刷新守护进程或通用异步 operation 框架。

## EP-01：目录发布及在途请求快照

- **Owner / trigger：** `opi-ai` 的 provider collection/catalog owner；provider 替换、删除、刷新、凭据所属身份变化触发新 revision。Reference Product 决定存储位置和哪些源参与刷新。
- **候选契约：** 已取得的 owned model/request snapshot 在在途调用中保持稳定；读目录不必等待网络。每个 provider 的刷新发布具有确定提交点，新 revision 生效后，旧工作不能再覆盖其内存状态或成为下次启动读取的当前存储状态。缓存恢复先于网络鉴权/请求；允许单 provider 失败而保留其他有效 snapshot，并返回按 provider 定位的错误。公开静态目录与账号专属发现结果区分；前者无需凭空建立账号缓存，后者须随实际 auth identity/revision 隔离。
- **失败与释放：** 取消/被替代工作停止发布并释放等待者；刷新失败保留最后有效目录。存储失败不能报告完整发布成功；已承诺的内存与持久化关系由同一 publisher owner 维护。Rust 可用串行 owner/事务提交或 revision 检查，不要求复制 TS callback+Map。pi 的写前/写后 generation 检查不是任意自定义 store 都能撤销已发生写入的证明。
- **机械验收：** barrier 驱动 A 刷新挂起→B 发布→A 迟到，分别暂停在 fetch、存储锁前、持久提交前后；内存与重开 store 只观察允许 revision。已准备调用同时读取旧 snapshot 并完成，新调用使用新 snapshot。注入一个 provider 错误/取消，不清空其他 provider。记录提交点前完成的旧写与提交点后的 stale write，不能把合法旧提交误判为竞态。
- **Evidence / status：** EPT-01、02；0.80.8 引入、0.84.0 明确 generation-aware publish 与取消。**目标清楚**；具体 Rust 同步实现留给设计但必须满足同一可观察验收。

## EP-02：Reference Product 的远端目录叠加

- **Owner / trigger：** `opi-coding-agent` 的 model runtime/catalog client；启动恢复、运行后的自动刷新、模型选择器以及显式 force refresh。`opi-ai` 只提供 EP-01 的刷新/发布能力，不自行选择 pi.dev。
- **候选契约：** bundled → 有效远端 cache → 现行用户配置的职责顺序保持可追踪；远端按 `(model type, model id)` 叠加，不能合并同 ID 的 chat/image/classifier。旧远端 Last-Modified 不覆盖更新的 bundled generation；不以文件 mtime 判断发布先后。请求支持 `types=chat,image,classifier`、ETag 条件验证；只有存在对应缓存 body 才发送 validator。304 更新检查时间且保留 body；未知 model type 不影响已知类型，但已知类型的畸形条目须经边界验证，不能复制 TS 强制转换。
- **失败与释放：** 404/501表示当前源不可用，清除失效validator并按overlay规则保留基线；瞬时失败保持cache/ETag。pi当次仍可显示先前恢复的overlay；有bundled generation时间时，下次恢复会依据失效metadata排除它。没有该时间的自定义wrapper不能据此宣称也会排除；fixture须分别断言当次发布和后次恢复，不混成即时清空。管理GET的retry与模型生成retry分离，每次弃用响应须释放body；offline禁止网络，force不越过offline。
- **机械验收：** fixture 覆盖 keyed/array catalog、同 ID 多 operation、unknown type、畸形已知类型、旧/新 Last-Modified、304 无缓存/有缓存、404/501、暂时错误、force、offline。可控 clock 验证 freshness gate，不创建“每四小时轮询”测试。校验产品 client→publisher→picker/SDK snapshot 的接线；无真实网络。
- **Evidence / status：** EPT-02、03；0.80.8、0.81.0、0.82.1、0.84.0、0.99.0。**已确认候选（D-23/Q26）**；服务来源及自动联网默认见确认记录EQ-1。缺少 ignored bundled 目录的完整名单不会阻止上述行为 fixture，但不能宣称完整 model inventory 已核验。

## EP-03：刷新等待者、可用性与产品关闭

- **Owner / trigger：** Reference Product model runtime 和 UI/RPC 刷新协调器；同时打开选择器、后台初始化、login/logout、session/runtime 关闭。
- **候选契约：** pi 的 interactive/RPC 在界面/服务已可用后后台更新，裸 SDK 初始化先恢复本地。并发等待同一次 interactive 全目录刷新可共享工作，一个等待者取消不取消其他等待者；最后一个离开时取消无人拥有的工作。新刷新可替代旧刷新，旧 provider/auth availability 结果不能在 logout 或新凭据发布后恢复旧可用状态。登录完成所需本地一致性与网络 freshness 分开。
- **失败与释放：** 网络失败可报告 provider 级诊断，已有模型选择不被等待网络阻塞；关闭 runtime 取消其 owned tasks 并等待释放，不留下晚到 publication。高层共享不强制所有裸 `refresh()` 都去重：pi 核心相同 provider 的新 refresh 是 supersede；这两种语义不能混成一个隐式全局行为。
- **机械验收：** 两 waiter 共一 mock fetch，取消 A 后 B 成功；全部取消后下一次能启动；login/logout 交错旧 availability 完成不得回滚新身份；启动 UI ready 信号先于 network completion；关闭后请求数、任务数、可发布 revision 均稳定。
- **Evidence / status：** EPT-01–03；0.81.0 后移启动网络、0.84.0 stale/cancel 修复、0.84.2 选择器共享刷新修复。**已确认候选（D-23/Q26）**；后台默认同已确认EQ-1。

## EP-04：Codex WebSocket 资源所有权

- **Owner / trigger：** `opi-ai` 的 Codex adapter runtime 持有连接池，Reference Product/SDK 持有 session 生命周期；明确选择 Codex WS/auto 后发起调用。
- **候选契约：** 一个 cached socket 同时最多一条生成；相同 session、账号及实际 endpoint/连接配置才允许复用。pi 1.0.0 已按 session/account 隔离；endpoint/连接配置也进入候选隔离条件，以避免其模块级 key 过窄造成复用。无 session 或 cacheRetention=none 使用一次性连接。连接忙时的临时连接也必须被 caller owner 跟踪并关闭，不能通过复用抢占在途生成。
- **失败与释放：** 连接失败、idle timeout、异常 EOF、cancel、consumer drop 或 runtime/session teardown 后，不保留不可信 continuation。会话释放同时清 sockets、timers、fallback/diagnostic 状态；pi 当前 close helper 只清连接，session 级 fallback/debug Maps 未同清，这不是应继承的持久行为。时限用单调时钟表达，不能复制 JS wall-clock 偶然行为。
- **机械验收：** 本地伪 WS server 断言同账号串行复用、跨账号/endpoint 不复用、并发忙连接不共享、sessionless/no-cache 必关闭；假时钟验证闲置和最大年龄；cancel/drop/close 后连接与 listener/task 回到基线。新建同 session ID 不继承已释放 runtime 的 fallback 状态。
- **Evidence / status：** EPT-04；0.71.1 cached、0.72.0 auto、0.80.4 age rotation、0.84.0 account 修复。**已确认候选（D-23/Q26）**，默认auto与具体时限见已确认EQ-1；实例所有权和有界资源是 Rust 实现选择，不要求模块级全局 cache。

## EP-05：Codex cached continuation 的等价条件

- **Owner / trigger：** Codex adapter 的单连接 continuation state；前一请求成功终止并收到可用 response ID，后一请求尝试复用。
- **候选契约：** 只有 input 之外的有效请求字段相同，且新 input 以前次完整 input 加前次实际 response items 为精确前缀时，发送 `previous_response_id + delta`。包含 system/tools、grammar tool 形态、effort 等的变更都影响资格；不按字符数或“最后一个 user”猜测裁剪。未满足条件则清 continuation 并发送完整上下文。`store:false` 与连接内 continuation 可并存，不能为缓存擅自打开远端存储。
- **失败与释放：** 无 response ID、取消、不完整终止或连接丢失均不发布可复用状态。`previous_response_not_found` 可进行一次有明确原因的完整上下文重试；第二次失败终止。不能复制源码对特殊错误重试缺少显式 public-output guard 的宽泛路径：若已交付内容，则由 EP-12 显式恢复，低层不把新结果悄悄接到旧 stream。
- **机械验收：** fixture 逐字段变更请求；相同前缀只发送新 tool-result/user delta，grammar function/custom call 不重复。中间插入 system/tool 修改、缺/改旧 response item、跨账号都回退完整 input。缺 continuation 一次恢复后 request 数为2；有内容后同错误不重播；不能归一化掉 input 顺序/response ID 绑定。
- **Evidence / status：** EPT-04、05；0.82.0 grammar delta 与 missing previous response 修复。**目标清楚**；输出后限制是纠正可造成重复输出的边界，不声称 pi 此分支已有完整证明。

## EP-06：WS/SSE 降级与 canonical stream

- **Owner / trigger：** Codex adapter；`auto`/WS 尝试发生传输不可用或连接失败。
- **候选契约：** SSE 明确选择直接走 SSE；auto 在可用时先 WS。普通 transport error 只在尚未交付有效 provider stream event 前允许 WS→SSE；开始后错误保留其 partial 与唯一失败 terminal。连接限额在开始前可重连一次，再按相同降级规则处理。协议错误、鉴权/配额/明确 API 失败不可伪装成 transport failure。诊断表明 configured/actual transport、失败阶段、降级原因，内容脱敏。
- **失败与释放：** abort 不触发降级；SSE/WS 都要求真实 terminal，clean close/EOF 本身不等于 success。连接状态、fallback 与 diagnostics 由 EP-04 owner 释放。pi 的 fallback 粘滞标记仅按 session；候选建议按连接身份分区，A 账号故障不静默改变 B 账号路径。
- **机械验收：** 同输入的 SSE/WS replay 在内容块、usage已知性、事件顺序、终态上等价；只归一化 clock/request ID。测试握手失败→SSE、首个事件后断开→error、不合法 JSON→protocol error、取消无重试、连接限额至多重连一次、EOF无terminal；跨账号 fallback 不污染。
- **Evidence / status：** EPT-04、05；0.73.0 fallback、0.76.0 timeout、0.80.0 connection-limit、当前源码。**已确认候选（D-23/Q26）**；产品auto默认见已确认EQ-1；继续SDK补核 UX-14/D-35 证明裸SDK也默认Auto，当前scope不是产品独占，省略/显式Auto与资源完成契约以AIR-035/036为准。不根据未测性能宣称更快。

## EP-07：已发布的渐进工具声明

- **Owner / trigger：** `opi-ai` transcript 规范化与各 API adapter；有序 system message 加/删工具或替换同名定义。
- **候选契约：** 声明变更保持原始时间位置，供支持的 provider 原生表达；OpenAI Responses 按 capability 优先 additional_tools，其次 tool-search 表达，其余按已确认 transcript 降级政策处理；Anthropic、Kimi 的已支持路径纳入逐 adapter fixture。工具搜索结果不得被误当普通文本完成后却没有更新声明。声明与可执行注册/授权分别拥有状态；D-19/Q22已撤回旧Q20的“声明之外一律拒绝执行”，执行依据绑定active/executable集合及当前授权。
- **失败与释放：** 当前目标不支持历史原生表达时必须选择主 spec 已确认的投影/拒绝政策，不由 adapter 猜测。重复名变更、remove→re-add、取消于变更提交前后保持 transcript 一致。Anthropic placeholder、beta header 和合成 tool-search wire 是版本特定 adapter 细节，不成为 provider-neutral tool 类型。
- **机械验收：** 同一 transcript 对各 adapter 捕获 request body；检查初始声明、逐步 add/remove/redefine、稳定旧前缀、最终集合及 fallback 的有意差异。产品搜索调用成功后下一请求确有新增声明，失败/取消不凭空提交。既有 terminal/工具执行授权测试保留。
- **Evidence / status：** EPT-06；0.80.7/0.80.9、0.84.2、0.86.0。**已确认候选（D-19/23）**，依赖已确认native-or-collapse与active executable/wire-visible分离；tag 后 Anthropic inline-tools 改法仅作为易变 wire 证据，不混入1.0.0基线。

## EP-08：工具发现的实际产品连接与恢复

- **确认范围说明：** D-23/Q26纳入可选tool_search及产品接线；下文MCP默认/连接行为保留为已发布pi消费者与未来接入依据。此次可用可控注册者验证同一loadout生命周期，不要求交付完整MCP/codemode；缺少MCP后端不等于库渐进声明或产品tool_search可以只留空入口。
- **Owner / trigger：** Reference Product 的 tool registry/loadout 与 tool_search 扩展；搜索未声明工具、MCP 连接完成、resume/reload 和 session shutdown。
- **候选契约：** CLI 内建注册 tool_search、默认 inactive；显式激活或 MCP exposure=deferred 自动激活，MCP 默认 exposure=codemode。搜索只从可搜索且尚未 active 的现有注册工具中选择并影响下一模型请求；空匹配不改 loadout。resume/reload 从 transcript 恢复已加载工具，尚未重连的名称保留为待解析，实际注册后恢复；开始下一 run 时不再无限保留永不注册的名称。显式减少 loadout 使旧 pending 集合作废。
- **失败与释放：** 关闭/替换 session 后迟到 MCP load/connect 不得注册到新 session，已连资源关闭；隐藏工具不因搜索变成可见，声明变更不新增执行权限。现有授权仍在执行边界检查。BM25、MCP client、codemode 不搬进 `opi-ai`；本条也不暗示已有 opi MCP 完整实现。
- **机械验收：** product fixture 用可控工具注册者证明搜索→active→transcript→wire；resume 时先恢复名称、后重连仍保留；关闭后迟到注册零影响；下一run前仍未注册则退出pending。至少一实际产品接线及同一公共行为 fixture；单纯新增库 seam 不计完成。完整 MCP/QuickJS 产品覆盖须单独列依赖，不能由这一条偷偷扩大。
- **Evidence / status：** EPT-06、07；0.99.0 产品发布、1.0.0 resume/reload 修复。**已确认候选（D-23/Q26）**；可选tool_search和产品接线纳入，完整MCP/codemode列后续差距，见已确认EQ-3。

## EP-09：opaque thinking/continuation 的捕获与持久化

- **Owner / trigger：** 每个 `opi-ai` API adapter 生成 typed content/metadata；`opi-agent`/Reference Product 的 canonical history 负责无损保留，terminal commit 后持久化。
- **候选契约：** 至少覆盖 Responses/Codex/Azure 的 encrypted reasoning/item metadata，Google/Gemini/Vertex text/thinking/toolCall 的 thoughtSignature，Chat Completions reasoning_details，Bedrock 的签名和 redacted bytes，并保留已支持 Anthropic thinking。元数据绑定原内容块与来源 provider/API/model；signed-empty text/thinking 不能按“无可见文本”删除。terminal 才出现的数据要回填同一 canonical result，不能只保存在 stream 临时状态。
- **失败与释放：** 不把 encrypted bytes 变成可读摘要、不从签名恢复隐藏推理、不因未知/旧持久化缺字段而伪造签名。解析错误按 typed adapter failure 处理，保留已知 partial。typed enum/受限 opaque payload 比 pi 的 JSON-in-string 更适合 Rust，但必须保留有意义字段及顺序。
- **机械验收：** 每族 fixture capture→terminal→序列化→重载→下一次request；包含 signed-empty、terminal-only encrypted_content、reasoning text/summary 分片合并、encrypted先于toolCall、Bedrock二进制分片。验证 bytes、块顺序与归属不变，显示层不泄漏 opaque 原文；未运行 live 签名接受性不影响离线结构完成。
- **Evidence / status：** EPT-08；0.80.7、0.84.0、0.84.3、0.84.4 的修复链。**目标清楚**，优先于“完整跨模型继续聊天”宣称。

## EP-10：跨模型重放与能力变化

- **Owner / trigger：** `opi-ai` history→target request 转换；provider/API/model 改变，或目标当前能力不允许旧 thinking 形态。
- **候选契约：** 同源同模型按具体 adapter 规则保留可用 continuation；异源 opaque/redacted 块不发送，非opaque可读 thinking 可按已确认投影转成文本，toolCall/text 的供应商签名剥离但保留其可见内容及调用因果。不能把“同 model ID”当成跨账号/endpoint 必然安全；远端若规定 identity scope，adapter 局部约束必须参与检查。原始历史不被此 request projection 破坏。
- **失败与释放：** 无法证明重放合法的数据明确标缺失/不适用；不能把丢失元数据伪装成零思考或完整parity。后续官方 Bedrock stale-thinking 修复证明“同模型永远照回签名”也不成立；目标应有能力变化 fixture，不机械复制1.0.0已知错误。
- **机械验收：** 同/跨provider、API、model的笛卡尔小矩阵；本地重放既有合法signature fixture，跨目标断言opaque不泄漏，原raw history不变；目标禁thinking而旧历史含signed reasoning时按adapter合法规则处理。明确差分只证明序列化选择，不证明远端接受所有历史签名。
- **Evidence / status：** EPT-08、09；当前 transform-messages 与固定窗口 Bedrock 修复。**目标清楚**；account/endpoint的具体远端限制只能按协议证据增加，不能创造全局兼容保证。

## EP-11：有边界的 assistant-call 恢复机制

- **Owner / trigger：** `opi-ai` 的可复用 retry classifier/显式 helper；调用方在一次 assistant-producing call 最终返回 transient error 后选择恢复。`opi-agent`/Reference Product 提供 policy，不把默认产品策略塞进所有库请求。
- **候选契约：** 按 typed error 区分 transient network/service/throttle/异常EOF 与 auth、billing/quota、invalid input、abort；上下文超限交专门 compaction 恢复。明确 initial call 不计 retry，有限 budget，指数 backoff 有 cap，取消在 sleep/prepare/stream 均终止。helper 不应绕过调用方显示完整失败attempt；它的 callbacks 至少区分 scheduled、attempt start、finished。
- **失败与释放：** 每个已scheduled恢复序列最终一次 finished；abort 成功与否不能因停止原因不是 error 就误报成功。预算耗尽返回最后实际失败；取消清 sleep/task，不能继续 produce。错误文字可用作adapter分类补充，不以复制pi regex替代Rust公共错误类型。
- **机械验收：** fake clock + scripted MockProvider：成功0retry、暂时错误2retry后成功、配额/auth/invalid/abort0retry、budget耗尽、sleep取消、stream取消、delay cap；检查回调次序与次数。compaction/summary消费者至少一项经过公开helper边界，若尚无对应产品则以fixture标明产品覆盖缺口。
- **Evidence / status：** EPT-10；0.81.1 summarization恢复、0.86.0 delay cap。**目标清楚**；已有opi失败分类和prepare契约要调整调用层级，不叠加职责相同的隐式retry入口。

## EP-12：Reference Product 的完整失败回合恢复

- **Owner / trigger：** Reference Product session/harness 持有 retry policy、恢复序列与持久化投影；一次完整 assistant attempt 已失败且分类允许恢复，包括已产生partial后失败。
- **候选契约：** 同一低层公开stream终止后不继续拼接新生成；高层可创建可区分恢复尝试。先保存failed assistant原文与已知usage，再持久标记其从下一次model projection排除，随后可取消退避、继续请求；raw历史和审计可查看失败内容，恢复不重复执行已完成tool。成功、耗尽、取消分别结束恢复序列；TUI/RPC/SDK观察相同恢复状态，不能在agent_end后误当整个用户请求完成而忽略已scheduled retry。
- **已确认身份选择：** 保留同一用户run/turn和recovery group；每次高层重做创建新的prepared logical call及其attempt身份、链接前次失败，从而正常重新解析同一目标的auth。低层同prepared call内的transport重试仍冻结request。目标provider/model不能因retry隐式改选；新解析若账号身份发生实质变化则终止本恢复序列，不自动另起不同身份调用。此映射是Rust迁移选择，pi没有可直接照搬的统一logical-call ID模型，**已由D-24/Q27确认，见EQ-2**。
- **失败与释放：** omission持久化失败则不开始重做；不能仅在内存删failed message导致重启后重新污染上下文。取消不得发success retry-end。当前pi message_end对非error的宽条件存在把aborted计为成功的风险，应由精确terminal分类验收消除。dispose取消恢复、compaction及在途请求，不遗漏finish事件或遗留任务。
- **机械验收：** partial→transient error→scheduled→durable omission→backoff→new call→success，校验raw含失败、projection不含失败、每个logical call唯一terminal与累计已知usage；重开session仍排除失败。对取消各阶段、omission存储失败、auth轮换、连续tool回合、预算耗尽做故障注入，断言已完成tool计数不增加。原“任何输出后禁retry”测试更新为低层禁止隐式拼流+高层显式恢复两组。
- **Evidence / status：** EPT-10、11；已发布默认产品行为，**已确认候选（D-24/Q27），身份/默认见EQ-1/2**。此处不自动引入virtual routing/跨provider fallback，亦不取消独立的人类live授权。

## EP-13：公开 deferred-response 生命周期

- **Owner / trigger：** `opi-ai` provider dispatch 和显式支持该能力的adapter；caller请求deferred submission，或对已有handle fetch/cancel。
- **候选契约：** 以与普通成功/失败可区分的 `Deferred(handle)` 表达本次提交/检查已结束但远端工作尚未完成；handle至少保留provider/model/API/token以及可选expiry/poll hint/conversion data。单次fetch可再次pending或返回正常最终assistant；`wait`缺省0是一次状态检查，不在库中隐式轮询。fetch/cancel走与普通请求一致的鉴权/选定adapter解析，但handle不包含凭据。Rust可用enum避免stopReason与缺handle的非法组合，不需抄三个语义重复入口。
- **失败与释放：** 不支持、handle route不匹配、无效/过期token以显式错误结束，不能静默当普通生成或自动切provider。预校验provider/API/model一致性是候选边界加固；pi目前有些校验仅在faux中，不能宣称所有adapter统一做过。expiry有声明但无内建remote证明时不编造renew/兑现规则。cancel是best-effort请求，不保证任务远端已取消；中断fetch只取消本次等待，不自动取消远端job。
- **机械验收：** 公开dispatcher + MockProvider/faux式fixture验证 submit→pending→ready/error、fetch再次pending、auth解析、unsupported、错route、cancel调用、fetch本地abort不产生remote cancel。序列化handle不含secret，pending不误发可见文本/工具执行。重复fetch ready是否稳定由具体adapter证实；不把faux的cached final当所有远端保证。
- **Evidence / status：** EPT-12；0.84.0发布公共契约及faux，真实durable调用成立。**目标清楚；远端证据缺口**。首批公共生命周期及可重开离线consumer fixture已由D-23/Q26纳入，见EQ-3。

## EP-14：deferred 的消费者恢复与远端覆盖分界

- **Owner / trigger：** 使用deferred的caller/harness持有poll schedule/checkpoint/cancel意图；`opi-ai`仅执行单次submit/fetch/cancel。已有pi durable在进程重开后恢复poll，是生产消费者事实。
- **候选契约：** 在公共契约验收中提供一个可重开的最小consumer fixture：持久保存选定model+handle+next-poll时间；重复pending时下次时间严格递增，不能busy-loop；只到ready才提交assistant结果。重开后不重做原submit，不自行换model。停止caller时取消本地调度，允许明确best-effort remote cancel并报告失败。这验证真实消费需要的契约，不要求本任务创建完整durable产品或新crate。
- **失败与释放：** 重开后model不存在明确失败；handle失效/remote取消失败不伪装ready，cancel任务有独立可用取消预算而不能直接复用已aborted token导致根本没尝试。账号绑定、远端poll期限、server-side幂等性、cancel后计费/完成竞态尚缺具体builtin协议，留到该adapter获得证据后定义。
- **机械验收：** temp store + fake clock重开fixture：submit1次、pending2次、ready1次；进程恢复只fetch；poll hint0/负值不得busy-loop；本地cancel释放scheduler，best-effort cancel失败有报告。参考pi durable重开测试但不声称已运行。远端完成条件单列协议fixture+独立授权live验证，live不作为默认离线完成条件。
- **Evidence / status：** EPT-12、13；**公共消费者语义目标清楚，内建远端adapter证据缺口**。`expiresAt`、account scope没有完整实现证据不能让Agent猜。若暂不交付真实远端，release/scope需明确“公开生命周期+离线consumer完成，远端adapter未交付”。

## 三项已获确认的取舍（保留提问来源）

| ID | 已确认选择及其代价 | 确认依据与分期边界 |
|---|---|---|
| EQ-1 | 采用已发布pi产品默认：non-offline运行后目录刷新、Codex auto/cached WS、高层retry enabled，保留offline/SSE/retry-disabled覆盖。代价是启动后网络访问、可能额外生成用量及资源生命周期。 | D-23/Q26、D-24/Q27明确确认；数值见下表，不是D-18自动批准，不新增通用配置框架。 |
| EQ-2 | 高层retry创建关联的新prepared call，保留同run/turn恢复关系并重新解析同目标auth；低层attempt冻结。 | D-24/Q27明确确认；账户实质变化终止，不隐式切账号或provider。身份映射是已选择的Rust迁移契约，不冒充pi现成logical-call模型。 |
| EQ-3 | 已知库语义与产品接线纳入，含可选tool_search、deferred公共生命周期及可重开consumer fixture；真实remote deferred等具体协议证据。 | D-23/Q26明确确认：完整MCP/codemode、真实remote deferred、其余operation adapters列后续已知差距；不创建opi-durable，也不凭fixture宣称远端已实现。 |

### 已核验并获确认的默认值与caller专属参考值

| 范围 | pi 1.0.0实际默认 | 候选建议与来源 |
|---|---|---|
| 远端目录 | pi.dev；freshness 4h；management GET attempt timeout4s、最多2次额外retry；interactive/RPC启动后台refresh预算15s | D-23/Q26确认采用这些已发布值作为可追踪初值；4h不是timer调度，management retry不是模型生成retry。EPT-02/03。 |
| Codex WS | product transport=auto；cache闲置5min、最大年龄55min；connect timeout15s；Codex HTTP maxRetries默认0 | 候选保留这些上界与明确SSE选择；不能因WS局部重连而宣称HTTP默认重试。caller timeout与connect/idle阶段分别定义。EPT-04。 |
| 高层Agent retry | enabled=true；maxRetries=3（initial不计）、base2000ms、maxAgentDelay60000ms | D-24/Q27明确确认；helper无policy则不重试。不是给全部裸SDK方法自动加三次。EPT-10/11。 |
| 工具发现 | tool_search注册inactive；MCP默认codemode，deferred配置启用tool_search | 复现可选路径而不是所有会话默认暴露；D-23/Q26已确定覆盖深度，见EQ-3。EPT-07。 |
| deferred fetch / durable poll | wait=0单次检查；durable无hint时5000ms，重复pending至少比前poll时间晚1ms | wait属于公共语义；5000ms是caller策略，可用作fixture参考，不擅自成为所有Rust调用方的默认轮询器。EPT-12/13。 |

## 不升级为目标的内容及重新评估条件

- 虚拟分类路由保持实验；只有获得明确目标与实际稳定契约后再审，不为它现在新增router框架。高层retry不等于允许跨provider fallback。
- 完整Responses Lite无已核验采纳承诺；RFC0054的additional_tools等局部机制已有实施/替代，按EP-07纳入，不能把全文都称未接受或全部搬入。
- TS模块全局cache、兼容source-mode旧build的fallback、临时placeholder名字、JSON字符串当metadata存储、非取消安全的listener/异步写竞态不是目标。Rust实现可替换，须保持已选择的可观察行为。
- 实际remote deferred adapter等待具体协议、credential scope、expiry、fetch/cancel与fixture；没有证据时持续标缺口，不能借faux或durable存在推断全部远端支持。
- 未测性能不作为删除已发布语义的理由，也不作为宣传新增WS/cache收益的依据。需要性能判断时另外取得可复现测量。

## 证据与实施导航附录

下面路径定位当前证据，不规定目标目录结构。全部测试为静态阅读，验收为未来设计；没有“这些测试已通过”的主张。

| Evidence | 源码、实际消费者和现有测试边界 |
|---|---|
| EPT-01 | [Models publisher/refresh](../../../.repo/pi-1.0.0/packages/ai/src/models.ts#L480)：provider generation、publication chain、缓存先于auth；[ModelsStore](../../../.repo/pi-1.0.0/packages/ai/src/models-store.ts#L3)；[ModelRuntime availability](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts#L334)与[refresh](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts#L839)；[providers test707](../../../.repo/pi-1.0.0/packages/ai/test/providers.test.ts#L707)证明其断言范围。 |
| EPT-02 | [remote catalog overlay](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/remote-catalog-provider.ts#L13)包含source/TTL/ETag/类型/Last-Modified/404分支；[FileModelsStore](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/models-store.ts#L46)锁+clone+reader取消；[catalog tests75–282](../../../.repo/pi-1.0.0/packages/coding-agent/test/remote-catalog-provider.test.ts#L75)；[management HTTP](../../../.repo/pi-1.0.0/packages/coding-agent/src/utils/management-http.ts#L3)含独立retry默认与body取消。 |
| EPT-03 | [ModelRuntime create](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts#L216)、[RPC启动](../../../.repo/pi-1.0.0/packages/coding-agent/src/main.ts#L937)、[interactive启动](../../../.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L1133)、[共享refresh owner](../../../.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/model-catalog-refresh.ts#L13)、[三种共享/取消断言](../../../.repo/pi-1.0.0/packages/coding-agent/test/model-catalog-refresh.test.ts#L23)。 |
| EPT-04 | [Codex constants](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L52)、[session/cache/transport选择](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L270)、[cache/cleanup](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L865)、[acquire/release](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L1153)；[AgentSession dispose](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1363)；[default auto](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/settings-manager.ts#L904)→[SDK](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts#L419)；[account isolation测试](../../../.repo/pi-1.0.0/packages/ai/test/openai-codex-stream.test.ts#L1414)。 |
| EPT-05 | [Codex恢复分支](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L301)、[WS parse/EOF](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L1307)、[prefix delta与continuation发布](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-codex-responses.ts#L1425)；[auto转发测试](../../../.repo/pi-1.0.0/packages/ai/test/openai-codex-stream.test.ts#L1283)。 |
| EPT-06 | [SystemMessage契约](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L512)、[Responses additional_tools/tool-search](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-responses-shared.ts#L179)、[Anthropic工具时间线](../../../.repo/pi-1.0.0/packages/ai/src/api/anthropic-messages.ts#L1139)、[Kimi/completions system-tool位置](../../../.repo/pi-1.0.0/packages/ai/src/api/openai-completions.ts#L1241)。完整transcript算法/测试由主候选相关条款统一，本文不另造第二套。 |
| EPT-07 | [tool_search默认](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/tool-search/index.ts#L1)、[searchAndLoad](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/tool-search/tool.ts#L191)、[MCP exposure default](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/mcp-servers.ts#L190)、[自动激活](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/index.ts#L446)、[MCP generation](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/index.ts#L954)与[shutdown](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/index.ts#L1088)；[loadout pending](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1482)、[restore/next run](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1757)、[注册后恢复](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L3541)。 |
| EPT-08 | RE-05已集中列各adapter capture→replay；补充[transform-messages](../../../.repo/pi-1.0.0/packages/ai/src/api/transform-messages.ts#L94)、[canonical持久化](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1115)；[Azure terminal replay](../../../.repo/pi-1.0.0/packages/ai/test/azure-openai-responses-reasoning-replay.test.ts#L83)、[Bedrock redacted测试](../../../.repo/pi-1.0.0/packages/ai/test/bedrock-redacted-reasoning.test.ts#L137)。 |
| EPT-09 | 固定窗口后续修复[Bedrock stale-thinking 69f0be6](https://github.com/earendil-works/pi/commit/69f0be6f0a6ca1bf66a2a9cf59c821b632e6bca1)、[Anthropic inline-tools b271b0a](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db)；transcript [PR9548](https://github.com/earendil-works/pi/pull/9548) merge `9e05370b298d0a6b8d9bc2c02e4bfae189ef1616`→0.86.0。不是未来承诺；访问/验证窗口见EVO。 |
| EPT-10 | [retry policy/helper/classifier](../../../.repo/pi-1.0.0/packages/ai/src/utils/retry.ts#L104)、[现有classifier/helper测试](../../../.repo/pi-1.0.0/packages/ai/test/retry.test.ts#L18)、[compaction真实消费者](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/compaction/compaction.ts#L614)、[产品默认](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/settings-manager.ts#L988)。 |
| EPT-11 | [post-run retry](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1806)、[raw持久化/成功事件](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1115)、[omission](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1208)、[恢复事件/取消](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L3656)、[产品事件测试](../../../.repo/pi-1.0.0/packages/coding-agent/test/suite/agent-session-retry-events.test.ts#L33)；[opi当前prepared/retry owner](../../../crates/opi-agent/src/agent_loop.rs#L460)说明迁移位置，不授权照旧保留输出后无恢复。 |
| EPT-12 | [wait/best-effort cancel](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L241)、[handle](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L500)、[Models鉴权dispatch](../../../.repo/pi-1.0.0/packages/ai/src/models.ts#L912)、[faux submit/fetch/cancel](../../../.repo/pi-1.0.0/packages/ai/src/providers/faux.ts#L526)、[public鉴权测试620](../../../.repo/pi-1.0.0/packages/ai/test/providers.test.ts#L620)和[lifecycle768](../../../.repo/pi-1.0.0/packages/ai/test/providers.test.ts#L768)。 |
| EPT-13 | [durable默认poll](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L109)、[fetch/cancel实际消费](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L223)、[partial过滤/poll checkpoint](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L398)、[ready/retry状态](../../../.repo/pi-1.0.0/packages/durable/src/harness/generation.ts#L447)、[重开测试201](../../../.repo/pi-1.0.0/packages/durable/test/harness-generation-recovery.test.ts#L201)、[poll测试270](../../../.repo/pi-1.0.0/packages/durable/test/harness-generation.test.ts#L270)。 |

版本依据由本地 [ai CHANGELOG](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md) 与 [coding-agent CHANGELOG](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md) 相应版本标题逐条定位，不以目录名、文件mtime或未打开的issue标题推断日期。0.84.4 reasoning_details分片合并（2026-08-28）、0.84.2共享catalog等待（2026-08-14）、0.84.3 catalog attempt timeout（2026-08-24）均属于已发布修复，不是规划。

## 实际检查与测试影响

本轮执行 `rg`、逐行源码读取及只读Python脚本将CHANGELOG关键词映射到最近版本标题。若候选路径不存在，使用 `rg --files` 重新定位，未将不存在路径当作能力缺失。没有运行runtime测试、fixture、网络生成脚本或live调用；上述验收均为设计。

实际文档检查：`python scripts/opi-doc-check.py` 返回 exit 0、`opi documentation contracts: PASS`；`git diff --check -- docs/research/pi-ai-opi-ai-alignment/evolution-target-proposal.md` 返回 exit 0、无输出（Git diff 不覆盖未跟踪文件，不把它当本新文件的充分检查）。补充脚本核对14个EP标题唯一、全部本地Markdown链接目标存在；未执行runtime验证。主Agent合并后仍需最终联合检查。

当前变更仅此研究文档，测试影响 **none**。未来实现测试影响建议：**add** 各EP故障注入/跨adapter/产品接线；**update** 输出后retry、签名持久化、目录刷新与工具声明旧断言；**retain** 取消/鉴权/工具授权/terminal公开边界；**delete** 仅删除被确认替换且必要行为有新覆盖的旧入口断言，不能凭目标新增就删测试。

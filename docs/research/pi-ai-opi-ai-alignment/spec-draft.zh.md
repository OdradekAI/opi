# pi-ai / opi-ai 对齐候选 spec

> **DRAFT — 非规范性。** 研究日期：2026-10-03；SDK 使用方复审：2026-10-04。这是供评审的本地候选文档，不是 `opi-spec`，不构成实施准入或 ready-for-agent 批准。本次工作不授权任何运行时实施。
> **修订后的目标：** 本次研究将 opi 定位为 pi 的 Rust 实现（D-18）。Q22–Q31（包括修订 Q31）明确取代第一轮受影响的选择；Q32–37 固定已选 SDK 范围，包括 Rust Result 和显式 telemetry 传播。当前决策为 D-19–D-37；D-01–D-17 保留早期历史。同步英文版见 [spec-draft.md](spec-draft.md)。

## 阅读与解释

先阅读范围、所有权和调用状态，再沿 AIR → 决策/证据 → [验收矩阵](acceptance-matrix.md)中的 V 导航。删除接口前查阅淘汰表。[README](README.md) 记录实际执行的检查；下文验收用例均为未来验证设计。

MUST 表示未来获得准入、实施本候选目标时必须履行的义务。SHOULD 允许有文档记录的例外，但必须保持所述行为和验证。MAY 表示范围内的可选行为。明确列出的证据阻碍不允许实施者猜测契约。人类确认选择的是目标，不证明实现已经完成，也不授予实施准入。

旧 `opi-spec` 关于能力归属和准入的理由在本轮属于可能过时的参考，不用于否决 pi 已确立的语义。正式规范协调必须记录 Rust 实现方向、多操作 SDK、动态可信修订和每次调用实际输入的来源。CONTEXT 中 User Policy、Package Trust 和 Capability Permission 的既有含义保持不变：模型内容不能授予权限。Runtime Input Binding 原先覆盖整个 run 的描述，需要与“不可变的初始来源记录，加上每次调用不可变的修订”协调；普通可信更新不要求创建子会话。本候选文档不修改任何正式文档。

## 问题、目标与范围

当前 opi 的聊天模型选择可能与实际分发不一致。合法的网络分块可能破坏 UTF-8；prepared stream 可能在终止后继续暴露事件；生产任务的清理行为因 adapter 而异。请求快照丢失 system/工具变更的时间顺序。工具图片被转成占位符。部分 compat 设置在 pi 中有实际语义，在 opi 中却尚未接通。完整 thinking 延续、SDK 操作和已发布产品路径需要一致的责任归属及离线验收。

通过 Rust 所有权、类型化状态和错误、有界队列及显式生命周期，实现 pi 已确立的可观测语义。保留 opi 有价值的保证：已知/未知 usage、保护秘密的错误、低层冻结鉴权、强制执行最终工具授权，以及保留源数据的旧会话迁移。修正已证明的缺陷，不把 pi 的实现偶然当作正确性判据。不承诺 TypeScript 符号/文件兼容、相同模型输出、缓存收益、性能或账单准确性。

已选范围：

- 现有聊天绑定、分帧、终止、取消和资源释放的正确性。
- 有序 transcript、命名 section、工具添加/移除/重新定义、默认 native-or-collapse 投影、调用间可信更新、失败/结果恢复及会话迁移。
- 有效 compat、thinking effort 选择、strict/grammar 采样，以及跨现有 adapter 家族的不透明延续数据。
- 共享的类型化图片/分类 SDK 操作：OpenRouter Chat-wire 图片；TypeSafe/OpenRouter System One 的 choice/bool/有序 score。优先服务 SDK/嵌入式调用方；不默认添加终端操作工具。
- 产品远程目录/缓存/刷新、Codex 自动 WS/延续/SSE 回退、可选 tool_search 接线及显式高层恢复。
- 公开 deferred submit/fetch/cancel 生命周期，以及可重新打开的离线消费者 fixture。这不代表已验证内置远程 adapter。
- 已选 SDK 操作间的显式可选 telemetry context，由宿主拥有 backend 与领域 instrumentation；不新增自动 SDK call/attempt schema 或 exporter。

本候选不包含：完整 MCP/codemode 产品移植、其余已发布图片/分类 adapter、独立 OpenRouter Image API、图片流式处理、缺少协议证据的真实远程 deferred adapter、实验性虚拟分类路由、尚未完整采用的 Responses Lite、通用 operation/plugin/transport 框架，以及计费策略重设计。它们属于已识别的覆盖缺口或实验，不是因为 opi 缺少消费者而被否决。重新评估条件见末节。

## 角色与场景

1. 嵌入方选择模型，在 preflight 和实际分发中观察到同一个可执行绑定。
2. 流消费者观察到唯一的终止结果，并释放被放弃的 attempt。
3. 可信宿主在调用之间修改指令或工具定义，不改变待完成调用的修订。
4. 会话用户执行迁移、恢复、分叉和压缩，不虚构历史，也不混入兄弟分支内容。
5. 工具宿主能够看到原始、规范化和实际执行的参数，并对实际执行授权。
6. SDK 调用方在绑定约束内定制请求，获得具有所有权的类型化媒体/答案及如实记录的用量。
7. 产品用户从缓存模型启动，在启动后更新目录，并能选择离线或 SSE 操作。
8. 持续调用 Codex 的调用方仅在存在语义等价的完整输入时使用缓存上下文。
9. 调用方能够区分恢复调用，或恢复 deferred 检查而不重新提交工作。
10. 嵌入方将已有 tracing parent 传过所有 SDK 调用模式，不替换其 subscriber，也不序列化 backend 对象。

## 职责与状态所有权

| 责任方 | 职责 | 边界 |
|---|---|---|
| opi-ai catalog/collection | 按操作区分的记录、原子修订、唯一 preparation、冻结调用和 attempt 租约 | 能列出不等于能执行；不隐式回退 Provider/账户 |
| opi-ai adapters/runtime | 有效 compat、wire 编译/解析、拥有 transport/cache、延续 | 不二次选择模型，不执行本地工具 |
| opi-ai transcript/validation | 时间顺序、规范化、投影、schema 转换 | 请求视图不改写规范事实，不授予工具权限 |
| SDK auth/resolver | 可注入的 API/OAuth/AWS 来源、秘密值、来源失败和过期 | 产品配置/权限决定是否使用；不在导入时发现来源 |
| opi-agent | 不可变的调用/批次执行修订、可信更新、hook、最终授权 | 已注册、active-callable、wire-visible 和已授权是不同状态 |
| 会话/产品事务所有者 | 单个分支、输入接受、checkpoint、迁移和持久化确认 | 不存在并行的上下文写入者；不虚构旧材料 |
| Reference Product 模型/会话所有者 | 远程来源/缓存、刷新协调、transport/恢复默认值、tool_search 激活 | 仅构造基础 SDK 不会访问 pi.dev |
| SDK operation/调用方 | 类型化 image/classifier/deferred 方法；调用方负责保存、阈值和轮询计划 | 不授予工具权限，不自动写文件，不隐藏轮询框架 |
| opi-ai telemetry module / Agent 或 SDK 宿主 / 宿主 backend | 显式 context 机制 / 领域 vocabulary 和 status / subscriber、export、sampling 和 flush | 无自动 SDK spans、全局初始化、durable state 替代或隐式上传 |

一个 logical call 拥有一个最终请求/模型/鉴权绑定；attempt 执行该绑定。每个 logical call 最多有一个可执行 attempt 处于活动状态。高层 recovery group 关联同一 run/turn 和意图下的新 logical call；它不修改旧调用的凭据，也不把新输出拼进旧流。已关闭任务可以在下一个 attempt 开始后完成不可执行的清理；清理最终必须释放本地资源。

Preparation 在鉴权前选择不可变的模型身份/修订和显式输入数据。鉴权可以合法提供 endpoint/header 材料。依赖 endpoint 的 compat 默认值使用最终 endpoint 解析，然后在 transport 开始前验证并冻结唯一最终绑定。此完成步骤绝不重新选择模型。SDK transform 在 payload 冻结前只运行一次。变更体现在有效投影/来源记录中；执行权限仍来自可信调用修订。

可信 context/loadout 更新先在独立位置完成组装和验证，由现有唯一事务所有者提交，再在调用边界原子安装。模型响应及其工具批次保留该调用的执行修订。当前 User Policy 仍可撤销权限。持久化的输入确认在后续鉴权/网络失败或取消后仍成立；确认前失败则保留旧状态和排队输入。仅对显式接入持久会话的调用方：最终绑定和 payload 冻结后，同一事务所有者 MUST 在首次 transport 分发前持久化确认已接受前缀、可信修订和实际投影。复用合适的现有事件/记录；不要求每次调用固定增加一对追加。此确认前失败或取消禁止分发，同时保留此前已经接受的输入。提交/取消竞态遵守既有确认规则：已确认事实保持持久化，已观察到的取消仍阻止后续分发。不持久化秘密；未观察到或因崩溃中断的分发不能记录为成功。

无会话的 SDK 调用方使用自有内存和相同的 model/message/wire 验证，不虚构会话文件、子会话或 Promotion 引用。无需事务 owner、Agent authorizer、产品配置或存储确认；AIR-015–019 仅约束持久会话集成。生成不追加 caller Context、不执行工具、不启动产品高层恢复或后台目录刷新。AIR-024/041 的执行授权仅约束执行工具的宿主/Agent，不是库聊天前提。

## 要求：绑定、流与鉴权

每行包含责任方、触发条件、行为/失败及机械验收。AIR 证据的定位见导航附录。每行中的所有失败行为和不变量均属于验收范围。

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-001 | Catalog/collection/adapters；准备调用 | MUST 选择唯一有效的 Provider/请求模型/操作/wire、限制、声明能力、compat 和 effort 映射。鉴权前验证已知结构；鉴权后完成依赖 endpoint 的推断，并在 HTTP 前验证。Preflight、诊断和 adapter 使用同一绑定；退出 adapter 独立重新选择模型的路径。错误或不可执行的选择在分发前失败。 | O-002, route probe, SP-01; D-03/21/28; V-01 |
| AIR-002 | Catalog/collection；覆盖、替换或刷新 | MUST 原子发布完整的最近已知有效修订；已准备调用保留旧绑定。刷新失败保留先前有效数据。列表不承诺存在分发路由。并发/后台发布及缓存一致性遵守 AIR-033/034，不新增第二个权威来源。 | O-001/005, EP-01; D-03/23; V-01 |
| AIR-003 | Prepared/公共流；poll/EOF | 首个已交付终态 MUST 吸收、关闭内部 attempt 并永久 EOF。公共 ChatStream Done/Err(CallFailure) 是 logical-call 最终；高级 AttemptHandle 终态只属于 attempt，可重试 acquisition outcome 独立，不能转发成公共最终 error。异常 EOF 只产生一次 typed protocol failure 后 EOF。保留终态/partial message及已知 usage；旧 provider Error/transport Err 按 AIR-047 收敛，不转成功或丢消息。 | A-04/P03/06, UX-03/16; D-04/31/36; V-03/05/40 |
| AIR-004 | Stream/producer；取消竞态 | 在终止交付前观察到的取消 MUST 优先于缓冲输出，并交付唯一取消结果；已经交付的终止保持最终有效。共享调用取消阻止后续 attempt；receiver drop 结束其 attempt。验证同步后的可观测竞态，不猜测全局线程顺序。 | A-03/04; D-04 Q9; V-04/05 |
| AIR-005 | 每个 transport/producer；连接/发送/读取/背压/drop | MUST 保持现有队列容量 64；生产任务的每次等待都响应取消和 receiver 关闭。Drop 释放空闲本地任务/HTTP 等待。每个调用同时至多一个可执行 attempt；该 attempt 终止/drop 后禁止其继续调度或输出。任务/socket 清理与租约释放分别证明；不承诺远程取消或退款。 | A-03/P07; D-04; V-04/05 |
| AIR-006 | SSE 分帧；任意合法分块 | MUST 保留原始字节，直到完整帧可被严格解码。所有 UTF-8/CRLF 分块均保持内容、顺序和终止。相关畸形 UTF-8/JSON 产生类型化脱敏错误；有效 keepalive/非内容帧仍合法。不逐块有损解码，不虚构成功。 | A-02/P06; D-04; V-02 |
| AIR-007 | Collection/retry 所有者；解析/鉴权/attempt | 低层鉴权 MUST 每个 logical call 只解析一次并保持冻结。遵守 retry 附录：adapter 默认与唯一 owner 落实显式 override，Agent 不叠加低层重试。Cancel/终止性 auth 阻止更多 attempt；只重试请求获取，不在内容交付后重播，不隐式回退 provider/model/account。保留聊天 attempt timeout 和调用方可选整体 deadline。高层恢复按 AIR-039，不因 Start 一概禁止。 | O-006, EP-11/12, UX-16; D-09/24/36; V-05/12/33/39 |

## 要求：transcript、执行与重放

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-008 | Transcript；初始/变更指令或声明 | MUST 保留有序内容追加、命名 section 插入/替换/删除，以及工具添加/移除/重新定义。同名 section 替换保持位置；删除后重加则追加到末尾。Body 与 section 保持可区分。工具定义变更表示为移除+添加；执行实现变更更新可信修订，不虚构声明变更。无效/歧义名称或 schema 在安装前失败。等价状态不追加冗余变更。 | TP-01–03/P04; D-19; V-06/16 |
| AIR-009 | Agent；可信更新/下一调用/工具批次 | MUST 在调用之间原子安装完整可信修订。每个请求及其响应工具批次保留唯一的实现/schema/来源修订。普通可信变更无需子会话。从 active-callable 上下文解析，不仅依据 wire 可见性；实际移除导致拒绝，隐藏/deferred/codemode 暴露本身不撤销合法可调用路由。历史/模型声明不能注册或授予权限。仍须执行当前授权，且当前授权可撤销权限。 | TP-06–08; D-19/22; V-06/15/16 |
| AIR-010 | Producers/history/replay；路由相同或改变 | MUST 分别保留 Provider、精确 wire、请求模型和报告模型，保留历史 ApiKind，并将不透明数据/签名绑定到所属内容块。已知兼容来源和 adapter 规则决定重放；跨来源/未知/不适用的不透明数据被省略，可读 thinking 可以转为文本并记录投影事实。规范原件保持不变；AIR-038 补齐各家族覆盖。 | R01/TP-11, EP-09/10; D-19/23; V-07 |
| AIR-011 | Transcript 编译器；adapter 能力 | 默认 MUST 采用 native-or-collapse：对支持的变更类别保持时间位置，否则将当前有效 system/tools 放在请求消息序列首部。记录投影模式/受影响范围/来源前缀；不能将 collapse 称为无损，也不能改写规范历史。显式 Strict 在鉴权/网络前拒绝不支持的所请求时间保真度。单个未变更初始状态是无损的。不能用统一 native-transcript 布尔值代替 adapter fixture。 | TP-05, PR9548; D-19 Q22; V-07/16 |
| AIR-012 | Replay/Agent 恢复；工具关联 | MUST 用批次位置+来源 ID 建立关联；已关闭批次可以复用 ID。歧义重复/orphan/重复结果导致失败。可无歧义确定的缺失结果得到请求局部错误 ResultUnavailable 和 UnknownEffect，绝不是 NotExecuted、回滚或自动重新执行。由事实支持的畸形配对保留原件；wire 无法表达时使用显式失败上下文，不虚构 {} 或 null。只有 Agent 记录已知未执行的补偿。 | TP-09–11/R02–08; D-20; V-07/15 |
| AIR-013 | Replay/Agent；失败/截断 assistant | MUST 保留规范 Error/Aborted，但从 wire 省略它们及关联的未执行结果。Length 中的提议绝不执行；完整有效提议及显式未执行结果在可表达时保持结构化。坏的/不可表达的提议采用局部失败投影。产品恢复可以省略整个失败 attempt 视图，同时记录事实。与已执行效果历史矛盾时失败，不能令该历史消失。 | TP-09/EP-12; D-20/24; V-07/15 |
| AIR-014 | 图片 preflight/serializer；工具结果 | MUST 通过受支持的原生位置或已验证的替代消息位置发送真实图片，保持关联/顺序/错误；否则在分发前拒绝。MIME 占位符不算支持。共同验证媒体与模型+wire；保留现有用户图片，将私有工具细节排除在 wire 外。 | O-009/P14; D-06 Q7; V-08 |

## 要求：持久化事实与迁移

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-015 | 会话；写入/读取新时间序列 | 新可写会话 MUST 使用 v3，按 required 分类的因果条目记录上下文变更、已知性、checkpoint、可信调用修订和投影身份。变更推进内容链。未知 required 版本/条目采用 fail-closed；中断尾部与损坏不同。旧历史缺失绝不等于已知为空。 | TM-06/08/09; D-14/19/28; V-17 |
| AIR-016 | 产品；继续 v1/v2 | 显式迁移 MUST 创建子会话，保留源字节/父引用/分支/tip，复制已验证的活动历史，将旧材料标为 legacy-unknown，只在继续时追加当前状态。触及 unknown 的精确重放失败；普通继续操作显式接受缺口。保留旧格式读取/迁移支持，不保留旧格式可写执行。这项旧格式策略不适用于普通可信更新。 | TM-07/11/12/TP-17; Q11, D-19; V-17 |
| AIR-017 | 唯一事务所有者；接受输入/状态 | MUST 验证并原子提交 context/checkpoint/leaf 和已接受前缀身份。持久化确认安装状态；后续鉴权/网络失败/取消保留已接受输入，不代表分发成功。提交前失败保留旧状态/排队后缀。最终冻结后，同一所有者 MUST 在首次分发前确认实际前缀/修订/投影；确认失败禁止 transport，同时保留此前已经接受的输入。保留提交确认和部分效果诊断；不引入竞争的上下文写入者。 | TM-03/13/TP-17; D-17 Q19; V-18 |
| AIR-018 | Compaction；压缩活动上下文 | MUST 在压缩提交时为完整有效上下文创建 checkpoint，再以 checkpoint+summary+保留的非 system 历史+后续变更重建。已经计入 checkpoint 的 delta 不再次应用。保留分支/已知性及原始审计记录。压缩有损：不能声称每个保留的旧调用都保留原始时间位置对应的 system 状态。部分提交原子失败。 | TP-04/pi checkpoint; D-28; V-19 |
| AIR-019 | 会话/产品；恢复/分叉/导出/证据 | MUST 重建唯一选定因果分支，在旧 tip 分叉时排除兄弟/后续变更。初始绑定保持不可变；即使 capture 关闭，每次调用的可信修订/前缀/投影仍通过 AIR-017 分发前确认屏障标识后续实际输入。呈现需脱敏，并显示未知/损失；完整树不是单个可重放分支。不新增原始 wire 归档或无损导出协议。 | TM-04/11/12/TP-17; D-19/28; V-17/19 |
| AIR-020 | 公共 builders/adapters/调用方；输入规范化 | MUST 将初始 system/tools 简写规范化为唯一首部 transcript 状态；adapter 只消费规范化后的规范输入。便利构造可以保留，但下游独立运行时字段/读取者在同一次范围明确的破坏性变更中退出。迁移直接 adapter、mock、hook、compaction 和示例；旧会话读取者不能分发旧请求路径。 | TP-01/RT08; D-19; V-06/20 |

## 要求：compat、保留的安全保证与类型化操作

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-021 | SDK/调用方；范围明确的 0.x 迁移 | MUST 以可执行模型/鉴权/错误契约替代并淘汰 Config/ProviderConfig/config::Error/Result 和 Model/ProviderKind 外壳；SDK 构造不得依赖产品配置。保留 ApiKind/持久化消费者。迁移导出/文档/示例；删除仅测试构造器的测试，保留行为。不默认提供永久 shim。 | O-003/SP-16; D-02/08/21; V-10/20 |
| AIR-022 | Compat normalizer/config/parser；准备/迁移 | MUST 实现实际 Chat store 省略/false、依赖 reasoning 的 developer role、effort 格式/门控/off，以及 tool→user assistant bridge。使用最终 endpoint 推断，以唯一字段优先级解析，显式 false 与未设置不同。仅淘汰重复的静态 reasoning_effort；supports/bridge 名称只有在等价替代行为和调用方迁移完成后才能退出。已淘汰键在鉴权/HTTP 前报错，不能被 serde 忽略。 | SP-01–04/17/18; D-21/26; V-11/20 |
| AIR-023 | SDK Bedrock resolver/adapter/产品；解析来源 | MUST 提供无需 coding-agent 即可复用的类型化 SigV4/Bearer/profile/default-chain 来源；产品负责配置/注入。区分配置发现与选定凭据。保留显式/作用域内 profile 相对环境 profile 的优先级、完整来源的失败关闭、秘密保护、取消、process 的 10s/64KiB 默认值/清理及非秘密来源记录。完整依赖链遵守已验证的来源/过期 fixture；仅本地来源不等于完整支持。不强制迁往产品独占层。 | SP-05–08, AWS annex; D-21/28; V-09/20 |
| AIR-024 | 执行工具的宿主/Agent 与 errors/usage producers；相关操作 | 模型参数 MUST 针对可信 schema 规范化/验证；可信的验证后修改遵守 AIR-041，最终授权绑定实际执行值。Strict wire 不等于本地验证或权限。保留类型化脱敏错误和已知/未知/零 usage；父项/子集用量只计一次。缺失成本/重试 usage 不是零，也不是完整账单。 | TP-12–16/O-011/P17; D-10/20/22; V-12–15 |
| AIR-025 | Catalog/collection；操作查找 | 身份 MUST 使用封闭的 Chat/Image/Classifier 加 Provider+模型，并具有类型化操作元数据。Chat 查找只返回 chat；image/classifier 不虚构 chat 限制。共享现有 catalog/auth/preparation，不另建平行 ImagesModels/单例/通用 JSON 执行。显式 SDK 注册、内置记录和经验证的动态记录都是合法来源。错误类型/路由在鉴权/HTTP 前失败。 | SP-09/EVO-009; D-13/23; V-21 |
| AIR-026 | Operation runtime；生成/分类 | MUST 提供类型化非流式方法，并冻结请求/模型/鉴权/payload/预算。图片默认额外 HTTP 重试 0 次，System One 为 2 次；调用方可以覆盖。每 attempt timeout 覆盖 transport/body；整体 deadline 可选，从入口开始使用单调时钟，绝不重置。只重试已验证的状态/header 类别，采用响应取消且有上限的延迟；绝不重试成功响应中的无效答案/媒体。未交付结果前，取消优先于过期，过期再优先于结果；已交付结果保持最终有效；drop 释放本地工作。不回退。 | SP-13/EP-11; D-24; V-22 |
| AIR-027 | OpenRouter image adapter；生成 | MUST 使用选定的非流式 Chat-wire，保留有序文本/图片输入（包括仅图片输入），并消费第一个响应 choice。返回按序、具有所有权的字节+MIME 及可选文本；成功必须包含有效图片。没有 choice、所消费媒体无效、仅文本/空/仅 URL 都是失败；额外 choice 不使有效的第一个 choice 失效。不得静默跳过坏图片、下载、写文件或切换独立 API。 | SP-10/SE-08; D-13/25; V-23 |
| AIR-028 | System One；分类 | MUST 支持选定的 TypeSafe/OpenRouter 公共协议、bool→noul、object state、string instructions，以及有序且唯一的 choice/score/bool criteria。返回全部请求答案的类型化结果，否则失败；忽略额外 question ID。缺少请求答案、类型/label/必需 probability 畸形或非有限值均失败。不设业务阈值，不授予工具权限。 | SP-11/SE-09; D-13/25; V-24 |
| AIR-029 | 答案验证；概率/分数 | MUST 保留 Provider confidence，确保有限 probability/confidence 位于 [0,1]、choice 属于请求 label、score 位于 [0,levels−1]。必需的 choice probabilities 与请求 label 对应。默认不增加 sum/argmax 拒绝或重新归一化。Score 要求 score+confidence，不虚构分布或均值。缺失 probability 是 unknown/error，不是零；不声称经过校准。 | SP-11/U-2; D-25 Q28; V-24 |
| AIR-030 | 操作结果；失败/成功 | MUST 区分 input/unsupported/auth/timeout/cancel/transport/http/invalid-result/resource-limit，并在业务验证失败后保留已知 usage/非秘密响应元数据。缺失/无效用量不同于 0；未报告的早期重试不是总账单。未知价格保持未知；普通诊断排除媒体/原始 body/秘密。 | SP-12/O-011; D-10/13/25; V-13/25 |
| AIR-031 | Budget/reader；编码/读取/解码前 | MUST 使用有限正数的自有预算；调用覆盖实例默认值，无 unlimited 哨兵，拒绝溢出。在过大分配/解码前执行字节/body/深度/数量限制；取消/超限中止读取，不部分成功。附录默认值是调用方可覆盖预算，不是 Provider 限制；覆盖不能启用未知 MIME/协议。不做像素解码或 URL 抓取。 | SP-15; D-25 Q28; V-23–25 |
| AIR-032 | SDK 所有者；支持声明/迁移 | MUST 经公开入口→伪鉴权→伪 transport 证明选定实际操作 adapter。两个 System One 绑定是同一协议；一个 image adapter 不证明多厂商支持。不默认添加终端工具/启动操作请求/文件输出/权限扩张。未来实施中的依赖变更需要精确锁定版本/平台评审。 | SP-09/OP02/08; D-13/25; V-21/22/26 |

## 要求：已发布路径与请求定制

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-033 | Catalog publisher/store；并发替换/刷新 | MUST 由同一个修订提交所有者负责内存和当前持久化快照。已被取代/取消的工作不能迟到发布内存或陈旧的下次启动数据；某 Provider 失败保留其先前数据，不清除其他 Provider。网络前恢复缓存，读取不被刷新阻塞。在 fetch/write/commit 屏障验证，包括不配合取消的迟到结果；只有 generation 检查不构成证明。 | EP-01/EPT-01; D-23; V-27 |
| AIR-034 | 产品 model/cache/refresh 所有者；启动/force/login/logout/关闭 | MUST 在 nonoffline 模式下于 UI/RPC 就绪后接入已验证的远端 overlay/ETag/freshness。SDK 构造始终零 store/env/外部 I/O；显式 refresh(allow_network=false) 恢复本地缓存，产品通过管理阶段调度恢复/联网刷新。保留 operation 身份、用户覆盖和更新的内置 generation。产品 waiter 共享：取消一个不取消他人，全部离开则释放。Logout/supersede/关闭阻止过期可用性/发布。304 需要缓存 body；404/501 清 validators，当次保留已恢复 overlay，下次 restore 排除失效缓存；临时失败保留可用缓存。Offline 优先于 force。 | EP-02/03/EPT-02/03, UX-13; D-23/35; V-28/39 |
| AIR-035 | SDK Models/session 资源所有者；缓存/临时 WS/cleanup/drop | MUST 提供可 await 的 cleanup_session_resources(Some(id)/None)，按公共附录选择实例内精确 session/全部含 sessionless 资源。取消旧 generation 的 connecting/active/busy overflow，释放 sockets/tasks/timers/continuation/fallback/diagnostics，禁止迟到发布/retry/SSE；已交付终态保持最终。独立 Models 隔离；注入共享不转移外部所有权。Owned call guards 在 Models drop/replace 后继续持有冻结修订；最后公共 owner/guard drop 发起清理，不能把 Drop 称为已 await 完成。成功 result 可保留复用 pool；idle/max-age 遵守附录的单调时钟语义。 | EP-04/EPT-04, UX-15; D-23/30/35; V-29/42 |
| AIR-036 | Codex adapter；SDK/产品 auto transport/delta/恢复 | MUST 支持省略 SDK/产品 transport 默认 Auto 及显式 Sse/Websocket/WebsocketCached/Auto；sessionless/Disabled cache 仍可用临时 WS。先解析 effective transport：省略与显式 Auto 使用相同 delta 规则，修正 pi 局部未统一默认值的问题。Delta 要求非 input 字段相等，前一完整 input+实际 response items 是新 input 精确前缀，否则发送完整 input。只有成功终态+responseID 才发布 continuation。在公开输出前允许一次 missing-previous-response 完整重试；generation 开始前的 connection-limit 允许一次重连，两者是独立有界分支。普通 WS→SSE 只允许在首个有效 provider 事件前；abort/API/auth/protocol 失败绝不伪装成 transport 丢失。已交付输出后的错误结束该流；高层恢复独立；EOF 不是成功。 | EP-05/06/EPT-05, UX-14; D-23/24/35; V-30/39/42 |
| AIR-037 | Transcript adapters + 产品 tool_search/loadout；添加/移除/重新定义/搜索/恢复 | MUST 将已验证的渐进声明接通到实际下一请求。产品注册 tool_search 时保持非活动；显式激活只将已注册且可搜索的工具加载到 active loadout，不新增权限。有效 deferred exposure 激活该路径。恢复/重载时保留尚未解析的注册名称，直到重连/下个 run 边界，再丢弃始终未解析的名称；显式缩减 loadout 清除过时 pending 名称。会话关闭后的迟到注册不产生影响。完整 MCP/codemode 移植不在范围内；用真实可信产品 registrar 证明接线。 | EP-07/08/TP-07; D-19/23; V-31 |
| AIR-038 | Adapters/history/persistence；输出/延续 thinking | MUST 捕获并往返保存 Anthropic signed/redacted、Responses/Codex/Azure encrypted reasoning+item metadata、Google/Vertex text/thinking/tool thoughtSignature、Chat reasoning_details 及 Bedrock signatures/redacted bytes。保留 signed-empty、仅终止时出现的元数据、顺序和精确来源。同路由重放遵守实际能力；变更/未知来源或已过时的 disabled-thinking 规则以安全投影处理并记录事实。绝不将加密字节暴露为可读文本，也不虚构已丢失的旧元数据。 | EP-09/10/EPT-08/09; D-23; V-32 |
| AIR-039 | 产品/Agent 恢复所有者；已终止且可重试的失败 turn | MUST 默认启用高层恢复，额外调用 3 次、基础退避 2s、上限 60s；允许禁用/覆盖。持久化原始失败 assistant，从请求视图省略整个失败 attempt，并发出 start/end/exhausted/aborted 恢复事件。新 prepared call 保持目标/意图及 run/turn 关联，重新解析同一来源的鉴权；实质账户变更/auth-needed/revoked 终止恢复。不重复工具批次，不隐形拼接输出。取消/总预算停止等待；只有真正成功终止才算成功，不是仅仅非 Error。低层 attempt 保持冻结。 | EP-11/12/EPT-10/11; D-24; V-33 |
| AIR-040 | Schema/adapter 采样；prefer/require JSON strict 或 grammar | MUST 保留每工具采样意图，只转换私有 wire schema，绝不转换可信执行 schema。支持经过测试的 nullable 转换/子集；prefer 在不支持时可回退，require 在分发前失败。Grammar 是不同的受约束工具契约；function/custom-call ID/结果保持关联及实际解析器行为。不支持 grammar 时回退到普通 JSON 工具，并记录投影事实；支持 grammar 的目标遇到无效 variants/schema 时在分发前失败。本地规范化/授权仍然适用。 | TP-14, EPT-05/06, P16; D-20/23/29; V-34 |
| AIR-041 | Agent args/hooks/authorizer；工具完成 | MUST 保留 raw，以 plain-JSON 规则在副本上规范化，完整验证后再运行可信修改/阻止 hook。Hook 可以变更执行值，而不重新按模型 schema 验证（Q25）。最终 authorizer 绑定实际值/注册/修订；之后的修改使授权失效。模型不能选择/伪造 hook 或授予权限。为旧 deny-only 观察者保留可区分的、针对被拒绝原始提议的只读观察；移动可变 hook 阶段时不能静默丢弃它。拒绝/取消意味着零执行，并使用有事实支持的结果。 | TP-08/12–16; D-20/22/28; V-35 |
| AIR-042 | SDK prepare/options/observer；可信定制 | MUST 将可选 payload transform 运行一次，然后验证 JSON/wire/预算，并为所有重试冻结实际 payload。身份/model/provider/endpoint/auth 变更或覆盖受保护 auth-header 均失败，并要求显式重新 prepare。非身份材料变更属于实际投影/来源记录，不改写规范数据、不授予工具权限；最终权限使用可信修订。持久化调用方通过 AIR-017 在首次分发前确认最终投影。通过保护秘密的边界观察响应；callback 失败类型化呈现，不吞掉，也不任意回退。 | SP-14/U-3; D-27 Q30; V-36 |
| AIR-043 | SDK deferred dispatcher；submit/fetch/cancel | MUST 区分 Pending(handle) 与最终 assistant/error；handle 包含 provider/model/wire/token，以及适用的可选 expiry/poll hint，不包含凭据。默认 wait0 只检查一次，绝不隐式轮询。验证路由，对 unsupported/invalid/expired handle 报错；fetch 可以继续 pending。鉴权使用选定 adapter/来源；本地 fetch 取消不取消远程 job。远程取消尽力而为，不保证成功或退款。 | EP-13/EPT-12; D-23; V-37 |
| AIR-044 | Deferred 调用方 fixture；重开/检查/轮询/取消 | MUST 提供可重新打开的离线消费者：保存 handle/model/next-poll，重新打开后 fetch 而不 resubmit，pending 推进时间，只有 ready 才提交 assistant。Stop 释放本地调度器；显式尽力取消具有独立可用的取消预算，并报告失败。不新增持久化产品/crate。真实远程完成仍缺协议/账户/过期/幂等 fixture；伪实现证明不能声称支持远程 adapter。 | EP-14/EPT-13; D-23; V-38 |

## 要求：独立 SDK 调用与公共结果

补齐调用方契约，不增加另一套 routing/collection 权威。新符号是目标，不是已实施示例。

| ID | 责任方 / 触发条件 | 目标契约与失败行为 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-045 | opi-ai Models/provider；SDK 组装/调用 | MUST 将 collection 的状态/职责收敛到唯一 Models，提供直接 stream/complete/stream_simple/complete_simple。日常调用内部复用唯一 prepare/attempt/compiler；高级 prepare_call 保留同一状态机。同步 get_model(provider,id) 返回 owned ModelInfo（目标记录包含 provider_id、bare model id 和精确 wire）；完整自定义 record 在已注册 provider/wire 上也可调用，无需目录命中、不被同名目录数据覆盖。创建 call 时冻结 owned 输入/provider/model revision；无永久并列 facade/router。 | UX-01/02; D-30; V-39/42/01 |
| AIR-046 | SDK factory/auth/store；构造/读取/管理 | MUST 不依赖其他 Opi runtime，默认提供内存 stores。空 Models 不注册 provider；显式 builtin 装配包含选定 provider 的惰性默认 auth/env 来源，可被注入完全替代。构造/注册/静态目录读取零 env/auth/store/外部 I/O。check_auth/get_available* 检查配置/metadata，不刷新 OAuth；get_auth 可同源刷新；refresh 报告目录恢复/刷新；login 返回已确认 Credential，logout 只删除 store。管理附录固定 Option/Vec/typed failures/RefreshReport，来源失败不吞成空结果。Typed 每 call key/受控来源在 prepare 前选定。宣称支持的 OAuth/AWS 必须由 SDK 可达；终端/keychain 留产品。不自动登录/目录背景刷新/产品恢复/私有 executor；生成仍可在一次 auth 解析中刷新，无管理前提。 | UX-06/07/09/11–13, SP-06/08; D-21/30/35; V-39/42/43/09/12/27/28 |
| AIR-047 | ChatStream/终态 collector；complete/result/error | MUST 让 complete 与 result(self) 共用 Result<AssistantMessage,CallFailure>：Stop/Length/ToolUse/Deferred 为 Ok，Error/Aborted 为 Err。CallFailure 总含完整 canonical partial/terminal AssistantMessage 与 typed 脱敏 cause；setup 无生成内容且 usage unknown。stream 失败只交付一个 Err(CallFailure)；旧 Ok(Error) 退出。result 主动驱动剩余有界流并复用已交付 terminal 缓存，不重发/自取消。未迭代/部分迭代/至 EOF 均得到相同终态；无 terminal 按 AIR-003，无无限事件归档或被动 final future 死锁。 | UX-03, SC-04/05; D-31 Q32; V-40/03/05 |
| AIR-048 | 公共事件；生成/重建 | MUST 在可交错的 block start/delta/end 和 Done 前输出成功 Start；setup 可直接 Err，started failure 最后 Err。block index/顺序稳定，end 含权威完整内容/参数/附属元数据。partial 是 owned 不可变事件时点视图，不是 JS 共享可变 alias；先前事件不随未来输出变化。完整 text/thinking/tool、partial JSON、仅 terminal 的签名/opaque、provenance/usage/stop/deferred 在终态保留，不只保留最终文本。 | UX-04, SC-06; D-30/31; V-41/07/32 |
| AIR-049 | SDK Context/options/validation；手动循环/换模型 | MUST 接受初始 shorthand 和 canonical transcript，公开同一 normalize/resolve/工具参数验证机制。serde 往返保留顺序/raw/opaque/来源/knownness，不保证 pi JSON 字节兼容。工具只声明；caller 追加 assistant/result 再调用，验证不安装 hook/权限。simple 支持中立 effort/tool_choice(auto/none)/公共选项；native 支持已选 wire 的 typed exact 选项，wire 不匹配提前 typed 失败。两路径都接入既定 transform/observer/cancel/timeout 和相同终态/失败契约；只有 effective 输入/映射选项相同的 fixture 才比较结果。Length 不执行；Deferred 保持 pending。 | UX-05/08, SC-07–09; D-19/20/26/27/30; V-39/41/07/35/36 |
| AIR-050 | SDK 文档/发布 owner；支持/旧路径退出 | MUST 通过 workspace 外隔离 consumer 验证以 opi-ai 为唯一 Opi runtime 依赖的公共调用、真实 adapter/假传输、双实例隔离、context/tool 循环和离线 package 导出/资源。说明常规 Rust runtime/Stream/serde 依赖。构造/静态读取无需 executor；异步工作使用宿主 Tokio，不承诺任意 executor/WASM。区分注册/可执行/adapter/产品接线；未覆盖项不宣称完整 pi 兼容。同步迁移旧 collection/complete/Error 消费者和双语 docs，不永久保留并列入口。 | UX-01–09, SDK-01–08; D-30/31; V-39–42/20/26 |
| AIR-051 | SDK OAuth/provider/store；login/resolve/refresh | MUST 让 Anthropic/Codex legacy/Copilot 完整 flow 通过注入 presenter/store 的 SDK 可达，采用 pi 协议和 provider metadata。遵守固定 OAuth 附录：JSON/state/copy-code、注册回调/手贴降级、严格 Codex refresh、Copilot enterprise/endpoint/models/policy。保存真实 expiry，只一次 5min 安全窗口；默认 refresh15s、网络可取消，commit ack 仍是权威。同源单刷新原子提交，失败不换账号来源。TUI/keychain 政策留产品；其他 OAuth 明示 gap。 | UX-06/09, OA-01–06; D-32 Q33; V-43/12/39 |
| AIR-052 | Builtin factories/catalog/config/store；选择/迁移 | MUST 使用已选 pi canonical ID/default wire：google/google-vertex/amazon-bedrock；单一 openai 默认 Responses，Chat 显式选择；内置 azure-openai-responses 必须有真实 Responses adapter，旧 Azure Chat 为显式 profile。不能只改名冒充 wire。显式迁移 config/catalog/credential 引用，不合并/覆盖账号、不永久保留默认 aliases。历史 provider/wire 事实保留；只用核验后的 request-view bridge 解释 legacy 来源。copy/ack/reference/finalization 可重入，source 保留至确认，由既有 owner 负责，不另建 ledger。 | UX-10, BP-01–04; D-33 Q34; V-44/01/20/42 |
| AIR-053 | SDK estimator/options/preparation；simple/native call | MUST 采用 pi canonical-context heuristic/simple budget：最后适用的成功正 usage anchor 加后继估计，已知正 reported total 优先，否则用完整已知 parent buckets 的正和；以 prefix 时间戳判失效，不限同模型、不取全文最大值，无 anchor 逐 message 估计。保持 unknown 与独立总数/分桶，不双加、不影响账单。simple 使用附录的精确 base/adapter 公式和 4096/min1；common-budget 家族使用可满足的 answer-room clamp，numeric Claude 为零时提前 typed 失败，不回填。native 保留各 wire 省略/默认/gating，不重复全局 simple clamp。调用方复用同一 estimator/mapping，无并列旧预算引擎。 | UX-10, simple-options/estimate; D-34 Q35; V-45/39/01 |

## 要求：显式 telemetry 集成

已选目标遵循 pi-ai 的传播职责，不照搬已移除的实验 Agent schemas 或未合并 exporter 提案。存在请求选项不证明已产生 traces。

| ID | 责任模块 / 触发条件 | 目标契约与失败 | 证据 / 决策 / 验证 |
|---|---|---|---|
| AIR-054 | opi-ai public options/Models/prepared/provider adapters；已选 SDK operation | MUST 公开可选 owned TelemetryContext，并在 native/simple stream/complete、advanced preparation、图片、分类和 deferred submit/fetch/cancel 中保留 frozen caller parent。simple mapping 与低层 retries 不替换/丢失它。缺省/显式 no-op 禁用本观测且不改变业务；无自动 SDK call/attempt spans。context 是进程局部对象，不进入 transcript/message/model/credential/catalog/session/deferred 序列化或 wire payload/headers。 | TC-01/07–09, TE-04/07/08, TO-03/08/09/16; D-37 Q37; V-46/39/42 |
| AIR-055 | opi-ai telemetry module/async handoff；显式 host context | MUST 通过 from_span 捕获匹配 parent Span 与来源 Dispatch，disabled/无来源成为 no-op；异步执行前冻结两者，不采用 poll 时 ambient，不初始化 global subscriber。在 producer tasks 及同步 Future/Stream poll/drop 范围传播该 Dispatch，不持 enter guard 跨 await。领域 owner 控制语义 status/settlement；返回 handle 或保留 Span clone 不代表业务完成。native tracing API 不宣称等同 pi 通用 callback/schema DSL。 | TC-02–06/11, TO-10–12/17–20, tracing0.1.44; D-37; V-47/42 |
| AIR-056 | Domain producers/host backend/evidence owner；观测开启/禁用/失败 | MUST 由 Agent/SDK 宿主拥有领域名字、typed/redacted fields 与终态解释，宿主拥有 backend/export/sampling/flush，既有 owner 拥有 durable evidence/diagnostics。默认观测排除原始 prompts/completions/tool data/payload/headers/credentials/自由错误文本。符合契约的 trusted backend 抑制 recording/export failure，不阻塞/重跑业务；不承诺隔离任意 subscriber panic/block 或 abort/OOM。telemetry 丢失不能供给权限、改变 Result/retry/usage 或隐去 evidence incomplete。 | TC-02/05/10, TO-04–07/12–15/19/20, CTRL-001/003; D-37; V-48/13/18 |

## 公共 SDK 契约（目标，尚未实施）

此处固定开发者使用的动作/名字，不固定文件位置。Models 收敛现有 collection 职责；prepare 保持高级机制。ModelInfo 是聊天记录，不是旧 Model 壳。目标 ModelInfo 明确添加 provider_id，id 保持 bare model id；catalog 记录与 custom 构造都必须给唯一 provider。注册时验证 provider_id 匹配拥有它的 provider，不按 id 反查或拼接推断；来源/响应模型字段保持区分。操作选择使用 AIR-025 闭合 enum/typed records，不建通用插件 trait。provider factories 可独立选择，显式 builtin 装配只列实际支持集合。

| pi 使用动作 | Rust 目标动作 | 输入/返回与责任 |
|---|---|---|
| createModels / setProvider | Models::new / set_provider | 默认内存实例，可注入 provider/auth/store/source；replace/remove 影响下一 call，不改旧 call |
| getModel / getModels | get_model / get_models | provider ID + model ID；同步 last-known 聊天记录，owned ModelInfo，未知 Option::None，零凭据探测 |
| *OfType / getAllModels | get_model_of_type / get_models_of_type / get_all_models | AIR-025 闭合操作选择；方法验证实际操作/wire |
| stream / complete | stream / complete | model record + Context + native CallOptions；stream 立即返回 owned ChatStream，complete 返回驱动它的 owned Future |
| streamSimple / completeSimple | stream_simple / complete_simple | 相同输入 + SimpleOptions 表达中立 effort/tool_choice；exact native 选项走原生路径，不静默 clamp |
| stream.result | ChatStream::result(self) | 消费 handle，主动 drain 至 Q32 Result；迭代为 Stream<Item=Result<AssistantStreamEvent,CallFailure>> |
| auth/refresh/login 管理 | check_auth / get_available / get_available_of_type / get_all_available / get_auth / refresh / login / logout | 显式异步；按下表区分配置检查、来源解析、目录刷新、登录/本地删除；不是生成前提 |
| 图片/分类/deferred | generate_images / classify / stream_deferred / fetch_deferred / cancel_deferred | 同实例 typed AIR-025–044 入口；统一命名不扩大 adapter 覆盖 |
| cleanupSessionResources | Models::cleanup_session_resources(Some(id)/None) | Owned 可 await 的实例资源清理；scope/取消/复用见下文；不删除 credential/catalog |
| 高级 preflight/attempt | prepare_call / PreparedCall::start_attempt | Owned AttemptHandle 至多一次 acquisition；共享下述累计 call budget/state，不套隐藏 retry loop；检查同一冻结 binding |
| request telemetryContext | options.telemetry_context | 从宿主 tracing parent 建立显式可选 TelemetryContext；所有已选请求路径同一 frozen context，无自动 exporter 或 SDK domain spans |

在创建 handle/Future 时捕获 owned 输入，不借用调用方 Context 或 mutable collection。鉴权/网络从首次 poll/await 开始：这是明确的 Rust 惰性异步时间差异，不可推迟输入冻结。使用宿主 Tokio，不建全局后台 executor。result 驱动 producer；不消费 result 而 drop 时按 AIR-004/005 结束 attempt。首次 poll 前 drop 零来源/网络工作。provider/store/资源共享必须显式；两个相同 provider ID、独立依赖的 Models 不污染全局状态。ChatStream 与 owned completion Future 为 Send + 'static，可进入宿主 Tokio task；Stream 不要求 Sync。

CallFailure.message 保留生成的 text/thinking/tool、来源、stop reason、诊断及已知 usage；cause 给 typed 分类，不额外交付第二 terminal。Done.reason 等于 message.stop_reason。Err.message 为 Error/Aborted；Cancelled 对应 Aborted，其余 setup/协议失败为 Error。收集不自动恢复、执行工具或将 Length 视为执行许可；仅产品 owner 启用高层恢复。必要时保留版本化旧 serialized Error-event reader；当前 producer/dispatcher 退出双格式写入。

一次调用为：捕获 → normalize/known preflight → 一次鉴权 → 最终 binding/compat/transform/projection 冻结 → durable 宿主确认或普通 SDK 内存安装 → adapter/events → 统一 collector。显式 key 在鉴权前选择并记来源，不能通过 payload transform 改身份。宿主自行追加最终消息/tool results 或换模型；SDK 不改调用方历史。

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

Models::new 是空 providers、默认内存 stores 及惰性默认来源；set_provider 或显式 builtin 装配仍不读来源。环境只在获允许的实际 generation/get_auth/check/refresh 阶段读取。注入来源可完全替代 ambient；typed 每 call key 优先，否则 stored source 优先于 ambient，选定失败不回退。自定义来源检查可做其声明的 I/O；不能称所有异步管理为纯操作。

cleanup_session_resources 返回 owned Future<Result<(),ManagementFailure>>；有效但未匹配的 ID 成功无操作。首次 poll 确立资源 generation 截止点。Some(nonempty id) 选择此 Models namespace 中完整原始 SDK session ID；None 选择全部含 sessionless，Some(empty) typed 失败，不能按 truthiness 解释为全部。首次 poll 在任何 auth/transform/network await 前原子登记 call generation；cleanup 取消匹配的已 poll Codex auto/WS calls，含准备阶段及同一 call 正在进行的 SSE fallback，禁止迟到接入新 generation；没有这些资源的显式 SSE/其他操作及外部 stores/providers/clients 不受影响。匹配旧 calls 观察内部 logical-call cancellation，尚未交付终态时返回 Err/Aborted；caller token 取消只向内联动，cleanup 绝不取消 caller 的 parent token、另一 namespace 或后续 calls；释放 cached/temporary/connecting/busy-overflow 资源，并等待库拥有的本地 producer/connection 退出。成功完成只证明本地静止，不证明远端撤销/退款。本地清理步骤失败时仍尝试其余选定资源清理，返回 typed 脱敏失败及已知效果/未确认释放；不能把不完整清理报成功或恢复已取消 generation。

清理可重入；截止点后的 call 和此前未 poll 的 logical-call handle 使用新资源 generation；已登记 PreparedCall 的未 poll AttemptHandle 仍属于原 generation。Drop cleanup Future 不撤销已提交的 close 标记；再次清理可等待未完成工作。已交付 terminal 保持最终；成功 result 不销毁 idle cache。Models drop/replace/remove 不取消 owned call：guard 保留冻结修订，最后公共 owner/call-guard drop 才发出资源释放信号。背景任务不能作为独立公共 owner 无限延长生命周期；Drop 发起关闭，完成屏障须 await cleanup。独立 Models 的相同 session ID 隔离；注入共享只释放本 namespace 的 lease，不关闭其他实例/外部 owner。不要求通用 cleanup registry/trait 或新的永久 shutdown 入口。

### 显式 telemetry context（Q37）

opi-ai 使用锁定 Rust tracing 生态，拥有小 telemetry module。本目标不新增 opi-telemetry crate、generic backend trait/schema engine、OTel/Sentry 依赖或默认 exporter。Agent 已依赖 opi-ai，可复用该机制，不把其领域/evidence 类型搬入库。standalone examples 说明宿主正常 tracing/runtime 依赖。

TelemetryContext::from_span(parent: tracing::Span) 从该 Span 捕获来源 Dispatch，不用后续 poll 时 ambient default；disabled/无来源是显式 noop。TelemetryContext::noop() 禁用；Clone 保留同一 parent/Dispatch，类型为 Send+Sync。宿主可主动传入 Span::current()，省略则从不捕获。非 noop 自定义 adapter instrumentation 使用显式 context/Dispatch；SDK 不替换成新的 call parent。已选 auth/management 接口不增加自动 tracing 要求；宿主可另行包围其 futures。

TelemetryContext::in_scope(FnOnce) 在捕获的 parent/Dispatch 下将同步 body 执行一次，返回前恢复调用方 context；TelemetryContext::instrument(Future) 保留其 output 与 lazy polling，在 poll/drop 使用同一 context。noop 在这些观测 helpers 内使用禁用的 scoped dispatcher，仍将业务执行一次，不创建 spans 或保留数据。这不静默宿主的独立日志或显式选择的其他 contexts。helpers 不创建领域名字/status，不超出 caller polling 驱动 Future；private Stream wrappers 使用同一 scoped 规则。

这提供 pi-ai options 接入语义，不提供源码级 pi-telemetry utility 兼容。native tracing 使用固定 callsite fields，没有通用 SpanStatus 或 callback-managed settlement。领域 instrumentation 使用自己的 typed vocabulary，显式解释 Result/Stop/Length/Deferred/failure，并将语义 scope 终结一次。捕获的外部 native parent 由宿主拥有 lifetime；SDK context 不能通过 Span clones 推断其语义完成。本候选不要求 generic startSpan wrapper 或新的 Agent span families。

tracing0.1.44 的 Future instrument/with_subscriber 不能单独覆盖 Stream 或所有 Drop 路径。执行观测时以 private synchronous wrappers 在捕获的 Dispatch 下处理 poll_next/drop 和 producer handoff；不持 thread-local enter/default guards 跨 await，不为记录而额外 poll 业务。noop 短路本 instrumentation，而不是在偶然 ambient subscriber 下创建 root。in-memory host/reference subscriber 验证 parentage 与结果保留，不用外部服务；不新增 production buffer/export API。

既有 Agent correlation IDs 由其 owner 解释，可作为已分类字段；backend span IDs 不是 durable identity。DiagnosticSink/AgentEvents/EvidenceSink 和 Session ack 保持独立。payload/response callback failure 仍遵守 AIR-042，passive telemetry 不吞掉它们。exporter setup、queue policy、flush/shutdown 是宿主职责，永不成为 Models 前提。conformance 针对可信非阻塞/nonpanic subscribers，不针对任意恶意 host code。

### 低层聊天重试（Q36）

AIR-007 在 opi-ai 使用唯一 request-acquisition retry owner，由公共 native/simple/prepared 路径共享。max_retries 计额外请求：显式0禁用，N最多1+N；每 call override 优先于显式 provider-profile 默认，再用选定 adapter 默认。Helper/Codex 族默认0；Bedrock standard 默认2，仍受 quota/分类约束，不是无条件3请求。关闭嵌套 transport SDK retry，淘汰 Agent 低层 request retry；AIR-039 另行创建最多3个新 logical calls。Codex 有界 WS reconnect/continuation 恢复按 AIR-036，不是通用 HTTP retry。图片/System One/远端目录次数仍按各自已选契约。

策略与同一 call binding/auth/payload 一起解析冻结；不二次解析来源或 transform。SigV4 每次 send 可从同一冻结凭据生成新 timestamp/signature，不复用过期 header bytes。Retry 只限成功 streaming-response handoff 前的请求获取；handoff 后的 body/EOF/parser/callback 失败即使尚无公开内容事件，也不重进此循环。Typed cancel/auth/permission/billing/quota/invalid-input 或无效成功响应为终止，generic catch 不能转成 network failure。每次 send/sleep 前检查 shared cancel/deadline；drop active attempt 释放等待，后续 attempt 必须符合下述显式状态转换，不能只因 active lease 已释放就重启。

高级 start_attempt 无 I/O 地占用唯一 active lease，返回 owned AttemptHandle；首次 dispatch 才消耗共享1+N预算中的一次 send。其 result 是带标签的 AttemptOutcome：RetryableAcquisitionFailure(cause,retry_ready_at)，或 Final(Result<AssistantMessage,CallFailure>)；类型名为目标，语义封闭。RetryableAcquisitionFailure 对该 attempt 是最终结果，不是 logical ChatStream 终态。普通 SDK driver 消费它、可取消地等待，再启动获准的下一个 attempt；高级 caller 显式推进同一状态，不能重置 counters/index/quota 或绕过等待。过早/并发/已封存/取消/耗尽的 start 返回 typed RetryNotReady/AlreadyActive/CallSealed/Cancelled/BudgetExhausted，零 dispatch。成功 handoff 永久关闭 acquisition retry；随后完成或失败为 Final 并封存 call。Nonretryable/exhausted acquisition failure 同样成为 Final。Drop 尚未 poll/dispatch 的 attempt 释放 lease，不消耗 send；已 dispatch 却未观察到可重试 acquisition failure 时 drop，保留 UnknownEffect 并封存后续调度。它不取消共享 logical-call/caller token。只有已观察、已分类的可重试 acquisition failure 且还有预算才进入 retry-ready。新 logical call 显式重新 prepare；旧错误不获得新预算。高级 iteration/errors 作用于 attempt；只有普通 ChatStream 发出唯一最终 Err(CallFailure)。

| 族 | 显式启用重试时的准入与退避 |
|---|---|
| Anthropic/OpenAI Chat/Responses/Azure/Google/Vertex helper | 对其余请求失败，x-should-retry false 禁止、true 允许；否则 typed network/no-status 或 HTTP408/409/429/>=500 允许。retry-after-ms 优先于 Retry-After 秒/日期。无有效 hint 时 retry index i 从0起：min(500ms*2^i,8000ms)*(1-U[0,1)*0.25)。不能换成旧 opi1000ms/无 jitter/60s clamp |
| Codex HTTP | Typed 获取阶段 network errors 及 HTTP429/500/502/503/504 或已验证 transient response-body predicates，排除 auth/invalid/billing/quota。retry-after-ms 再 Retry-After；无 hint 为1000ms*2^i，无 jitter。保留固定 predicate fixtures，不复制 pi 可重发400/401的宽 outer catch |
| Bedrock standard | 只限 Smithy THROTTLING/TRANSIENT，按固定 error-name/trait/network-code 和 quota fixtures；裸408/409/501/505及终止性凭据失败不能仅按 status 变可重试。b=floor(U[0,1)*min(base*2^i,20000ms))，base为 transient100ms/throttling500ms；有效 server hint H 调整 delay 为 max(b,min(H_remaining,b+5000ms))。Standard quota 由每 logical call 拥有，初始500，retry cost5或 transient10，成功归还 acquired cost或1；capacity 为零禁止 retry，另一 call 从全新 quota 开始，不共享 provider-global quota |

max_retry_delay_ms 保留 pi 的 server-delay 准入含义：默认60000ms；显式0关闭此检查，正有限 cap 在原始有效 server hint 超限时返回 typed RetryDelayExceeded，不再 dispatch。它不 clamp 指数 delay，也不把过长 server 指令截短成 retry 许可。Bedrock 在有界 hint 调整前检查，这是修正忽略 options；固定 Retry-After 优先于 x-amz-retry-after(毫秒)，消除 header 枚举偶然。Malformed/nonfinite hints 忽略；有效过去/负 hint 为0。Helper/Codex 的 retry-after-ms 为毫秒，Retry-After 为秒/日期，以 fixture 固定优先级及所接受 numeric/date 语法。退避使用可取消单调等待、测试注入 clock/RNG；不断言精确 wall-clock sleep。

首批 AWS retry 目标为 standard 加显式 typed overrides；不在选定策略背后读取 ambient AWS_MAX_ATTEMPTS/AWS_RETRY_MODE/SMITHY_NEW_RETRIES_2026 或 shared-file retry knobs。Pi 锁定 Node SDK 会读取，且与 options.env 分离：如实记录覆盖差异，不把 scoped auth env 称为 retry-mode parity。显式不支持的 adaptive/experimental retry 请求在 preflight typed 失败，不静默用 standard。重新评估须有真实 SDK 调用方需要这些模式，及经核验 Rust 算法/权能契约；本批不增加推测性 adaptive framework/flag。

V-05 固定 default/override0/1/2、503序列、Bedrock501/408/403/nonretry及 transient/throttling/name/trait/cause、quota耗尽/归还、helper header decisions/jitter和 Codex1000/2000/4000。Bedrock U=0.5首次 transient 时 b=50ms，10s hint 准入后产生5050ms；hint90s超默认60s失败。覆盖 cap−1/cap/cap+1/0、双 AWS hints、malformed/date/past hints、每个等待点 cancel/drop/deadline及零后续 send。Auth/transform 次数保持1；冻结 payload/identity 和新 SigV4 timestamps 一致。三种公共消费路径保留 partial/unknown usage 及相同最终 Result；fixture 不承诺远端幂等或未知失败 attempt usage。

### SDK OAuth 与内置迁移

AIR-051 仅目标 Anthropic、openai-codex legacy 与 Copilot。Anthropic 使用 JSON grant/refresh、验证 state，浏览器 localhost:53692/callback，显式 copy-code redirect 为 platform.claude.com/oauth/code/callback。Codex 浏览器 localhost:1455/auth/callback，bind 失败保留手贴，refresh_token/expires_in 严格校验，originator 有意保留 opi。Copilot 支持 enterprise domain、token 推导 proxy endpoint、账号 available-model IDs，显式 login 按 pi 条件启用 unconfigured-model policy；refresh 发现 metadata，不授予无关权限。精确字段/predicate/假 HTTP 依 pinned OA 附录，不猜远端注册。device denied/expired 保持 terminal，即使 pi 将某 status 当 pending。

保存服务原始 expiry，只一次 5min 近过期检查（与 pi 双扣的有意差异）；同源 refresh 默认15s，传递 caller cancel，通过注入 store 的 atomic update/ack 一次提交 credential/metadata。SDK memory store 与 product store 遵守同一刷新规则；SDK 不依赖具体 keychain。commit 前网络失败/取消保留旧凭据，expiry 本身不是 revoked；已开始 commit 按既有 ack 语义，不捏造回滚。不自动交互登录、远端 logout 或身份探测。

Builtin ID 迁移 gemini→google、vertex→google-vertex、bedrock→amazon-bedrock；openai-responses 合并 openai、默认 Responses 并提供显式 Chat；azure 内置变为真实 Responses 支撑的 azure-openai-responses，旧 Chat 仅显式 adapter/profile。Azure 默认 POST https://{resource}.openai.azure.com/openai/v1/responses?api-version=v1，api-key，body.model=deployment；按 pinned BP 的显式 URL/version 选项验证，不插入 /deployments/{model}/responses，也不复用 Chat payload。已存在用户自定义 provider ID 仍由 caller 拥有，不是隐式 builtin alias。迁移先验证 destination 冲突，目标 ack/引用切换前保留旧历史/source keys；可安全重试且不覆盖不同身份，仅成功显式 finalization 后删除 obsolete builtin keys。当前 config/lookup/default aliases 退出，版本化历史事实/readers 保留。当前支持表明示 Mistral native Conversations、新 OpenAI ChatGPT OAuth 及其他未选 providers/flows gap。

### 公共选项映射与当前 SDK 覆盖边界

native 与 simple 共用终态契约，不保证相同原始 options 就生成相同 wire。令 M 为 explicit max（若缺则 model.max），E 为 canonical 估计，W 为 context window。已验证 M 为正有限整数；clamp(M) 在 W<=0 时为 max(1,M)，否则 min(M,max(1,W−E−4096))；base C=clamp(M)，不能另将 explicit M 限为 model.max。numeric Claude（Anthropic/Bedrock）取 common/custom B，F=clamp(min(C+B,model.max))，有效 B′=min(B,max(0,F−1024))；这是追加 thinking 后再次 context clamp。common Chat 的预算字段取 B′=min(B,max(0,实际响应 ceiling−1024))，不做 Claude 的追加；B′=0 时省略预算字段，template 的缺失值遵对应 fixture。默认 B 为 minimal1024/low2048/medium8192/high16384，xhigh/max 的 token budget 映射 high；Google/Vertex 不套此表或 answer-room clamp。native 不重复 collection-wide simple clamp：Anthropic 默认 model cap；Chat 未显式 max 则省略；普通 Responses 的显式 max 经 supportsMaxOutputTokens 与 ChatGPT Sign-in omission gating 后 floor16；Azure Responses 的显式 max 单独 floor16，无前述 gating；Codex legacy 不发送 max_output_tokens；Google 仅显式 max；Bedrock Claude 默认 cap，其他模型仅显式。Q34 的 Azure Responses 是候选，尚未实施。

answer room 不是低 ceiling 下的无条件 1024 保证。numeric Claude 请求正 thinking 却得到 B′=0 时，在鉴权/dispatch 前返回 typed BudgetInsufficient，保存 requested/effective 预算，不自动关 thinking、不发 enabled/0，也不把 0 回填成1024。正 B′ 按上述已选 wire 映射；native exact 的已知非法值同样 typed 拒绝。pi Anthropic simple 传 0 后 wire 使用 `|| 1024` 的冲突是局部实现缺口，本候选不复制。验收固定 ceiling1/1024/1025/2048，并分别验证 zero 失败、正预算、Chat omission 与 Google 例外。

Google预算顺序为 minimal/low/medium/high；custom thinkingBudgets 优先，Google3 level 模式不发送 numeric budget。下表是实际 wire baseline，不声称所有行有独立设计意图；GenAI/Vertex 的 flash-lite 差异保留可追踪 fixture，未来核验约束后才修订。

| Wire/model | Numeric budget / answer room |
|---|---|
| GenAI 2.5-pro；Vertex 2.5-pro | 128/2048/8192/32768；无 common 1024 answer-room clamp |
| GenAI 2.5-flash-lite | 512/2048/8192/24576；无 common clamp |
| GenAI 2.5-flash；Vertex 2.5-flash（当前包括 flash-lite 匹配） | 128/2048/8192/24576；无 common clamp |
| Google/Vertex 其他 budget 模型 | -1 表达协议的 dynamic budget，不是资源无限哨兵 |


保留已确认支持等级选择与 native exact 能力验证。估计 canonical transcript，以真实 reported anchor 为依据，不用于账单。Q35 确认撤回同模型/全文下界限制。计量 pi UTF-16 code units，每图4800等效 chars；除4向上取整的粒度为 message：user/toolResult 汇总全部 blocks 后一次取整，assistant 汇总所有 text/thinking/tool name+序列化 args 字符后一次取整。system 先渲染 body+sections 再一次取整，另分别累加 JSON toolsAdded 与 toolsRemoved 的取整估计；不得按每个短 block 独立取整。最后非 Error/Aborted、时间戳不早于其前缀的正 usage 提供 anchor：已知正 reported total 优先（即使不等于分桶和）；total 缺失/0 时，仅完整已知的 input/output/cacheRead/cacheWrite parent 之正和可替代，未知桶不补0、child 不重复加。须保留 adapter 的独立 reported total；这不改 parent/subset 计费。anchor 后逐 message 估计，无 anchor 则全估计；保持 Rust 数值表示差异。V-45 单列 total≠sum、total=0 fallback、partial unknown 与 prefix 时序，不以归一化隐藏。

pi cache none 对应 Rust Disabled；省略的 SDK retention 使用 Short，受所选受控环境的 long override 影响。现 Rust None 的“provider 默认”含义不等于 pi none，不能默默映射为同义；原生高级默认保留时须明确区别。caller header 值支持 set/remove 非受保护默认项并在 auth 后合并；身份/受保护 headers 仍按 AIR-042 拒绝。当前候选提供静态 extra_headers 与 payload/response 扩展；pi transformHeaders 动态 callback、onProviderStreamEvent 原生事件 observer、compact assistant frames helper **明确延后**，不是已对齐能力。需外部 SDK consumer 的具体输入/输出与脱敏/frame终态 corpus 后重新评估；frames 的 progress 与 terminal 不能混淆，也不为它建通用事件框架。

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

高层 effort 顺序为 off/minimal/low/medium/high/xhigh/max。Nonreasoning 支持 off；排除显式标为不支持的映射；xhigh/max 要求声明支持。请求级别可用则保持，否则先向上、再向下搜索。记录 requested/effective；native/exact 不能静默调整到支持范围内。基于预算的 adapter 通过固定 Provider fixture 应用附录可满足的回答空间公式/零预算失败及实际 off/null/省略 wire 差异，不使用通用字符串 effort 字段代替。

### Compat 与鉴权

有效记录只解析一次，不从已被覆盖的记录重新导入字段。字段默认值顺序为：adapter 基线推断 → 显式 Provider profile 默认值 → 显式最终模型覆盖。同一层的显式 role 优先于自动 developer support；已有模型 role 覆盖优先于 profile role。保留未设置与 false 的区别。Chat support-store 表示发送 store:false 或省略；Responses store 是独立 wire 设置。Developer support 表示只对 reasoning 模型使用 developer，不是对所有 assistant 使用。Bridge 是请求局部 tool→user 转换，包括工具图片替代位置，带固定 assistant 标记；它不是规范 assistant reasoning。Thinking 意图/值映射/格式/门控/off 是不同职责。Provider 特有格式和推断使用 [SP-01–03](sdk-target-proposal.md) 语料，包括最终鉴权提供的 endpoint。

AWS 选定来源表和精确依赖证据见 [SDK 附录 A–D](sdk-target-proposal.md)。冻结的目标规则：

- 显式类型化来源只选择一次；互斥显式选择报错。Bearer 与 SigV4 不同时附加。Auto 区分显式/作用域内 profile 与环境 profile；显式/作用域内 profile 优先于注入的环境凭据对，单有环境 profile 不优先于完整凭据对。不完整的已选/作用域内凭据对失败，不拼接环境中的另一半；这是保留的 opi 保护，也是显式的 pi 验收差异。
- Default-chain 发现遵循已验证的 Env（有 profile 则跳过）→ 显式 SSO → INI → process → web identity → ECS 或 IMDS。INI 覆盖 role/source-profile/static/web/process/SSO/login-session，包括 config/credentials 字段覆盖及 role/MFA 选项。只有类型化、允许继续的发现失败才推进；缺失可选来源不同于选定来源的终止性/鉴权失败。SSO 无效会话/MFA 缺失/web-token IO 错误按已记录规则停止；process 发现失败可以继续。选定凭据后的 HTTP401/403 绝不启动另一轮身份搜索。
- Profile/metadata/token/process/clock 来源可注入，且独立于产品受到支持。有效且未过期的缓存凭据可以在接近五分钟过期窗口、一个同来源刷新进行时继续返回；已过期凭据要求重新解析。遵守来源报告的有效过期时间，并在来源提供时保留原始过期时间/stability 来源记录；中立 resolver 不虚构延期。所有本地等待和陈旧刷新发布都有边界及清理。文件刷新必须具有显式失效机制，不能依赖永不失效的模块 promise；不要仅因 Node 链可能如此就重复运行同一 process。
- SSO/login 刷新及获准缓存写入是真实来源行为，不能声称只是只读文件。离线验收只以伪 transport 执行 STS/OIDC/ECS/IMDS。导入时绝不发现来源。SDK DefaultChain 必须能通过公开的伪组装调用到达，不能只存在于内部测试；不复制 pi 已配置列表遗漏 default-profile/IMDS 的行为。
- 原始 SKIP_AUTH 环境模拟不在本轮类型化来源目标中。pi 的 dummy SigV4 和环境 bearer 覆盖是已记录的实现行为，不是 NoAuth。后续 proxy 模式需要显式实际签名契约及独立 resolver/header fixture；调用方现在已可提供显式选定的 SigV4 凭据/自定义 endpoint，但不能据此宣称无需鉴权。

恢复鉴权使用同一选定来源/目标。允许已确认属于同一账户的凭据刷新；已知 principal/来源切换停止。凭据实质改变，却无证据标识其仍属于同一选定账户时，以 identity-unverified 错误停止自动恢复；不能静默假定，也不能额外发起身份探测网络请求。Resolver 可以提供稳定且非秘密的账户绑定；仅附加方案相同不足以证明账户相同。无论刷新是否完成，旧调用鉴权保持冻结。

对于目录 404/501，保留当前已恢复 overlay、同时使其来源失效，是观察到的 pi 行为。固定版本实现中，后续恢复是否撤下该 overlay 取决于内置 generation。AIR-034 将类型化失效 tombstone 设为权威，即使没有该 generation 也生效；这是显式一致性修正，不是对 pi 精确行为的断言。

### 操作预算与产品默认值

以下预算均为有限正数 SDK 默认值，可由可信调用方覆盖，并按调用冻结。它们不是实测最优值或 Provider 限制。加法/编码/分配溢出均失败；预算不能增加不支持的格式。

| 项目 | 选定默认值/语义 |
|---|---|
| 每个方向的图片媒体 | 8 张图片；每张解码后 16 MiB，解码后总计 32 MiB；PNG/JPEG/GIF/WebP |
| 图片内容/body | 有序非空输入块，允许仅图片；每个方向文本 UTF-8 总计 64 KiB；编码请求和 HTTP 解码后的成功 JSON 各 48 MiB |
| 图片验证/输出 | 严格 base64、非空字节、MIME/容器签名匹配；不解码像素。只消费第一个 choice，先文本，再按其报告顺序输出图片；至少 1 张有效图片，不跳过坏的已消费条目 |
| 分类输入 | 默认 1–64 个唯一问题；choice 默认 2–255 个 label；score≥2 个有序级别，不套用 llama 的 10 上限；默认 request≤1 MiB、嵌套≤32 |
| 分类响应 | HTTP 解码后的成功 JSON 默认≤2 MiB；有限性/类型/domain/label 检查，忽略额外 question ID；默认不做 sum/argmax 检查，不要求 score 分布 |
| Timeout/retry | 图片 attempt 默认 600000ms，覆盖完整 HTTP body（修正 Node 仅计 header 的 timer）；System One 默认没有 attempt deadline；显式正数 attempt 预算覆盖。整体 deadline 可选。额外 HTTP 重试为 image0/SystemOne2；延迟上限 60000ms；状态/header 判定见 SP-13，不重试业务无效结果 |
| 远程目录 | 产品来源 pi.dev，新鲜度 4h 门槛（不是 timer）；管理 attempt 为 4s/额外重试 2 次；UI/RPC 就绪后启动刷新预算 15s。请求 operation types；只有存在缓存 body 才用 ETag 条件 GET；用户覆盖最高 |
| Codex WS | SDK/产品默认 Auto；idle5min/max-age55min 在下次 idle acquire 检查，不用 timer 中止 active generation；connect15s；显式 SSE/no-cache；HTTP 默认额外重试0次 |
| 产品恢复 | 启用；额外 logical call 3 次，指数退避基数 2s、上限 60s；显式禁用/覆盖。Auth/billing/quota/invalid-input/cancel 不属于通用暂时错误重试；overflow 由 compaction 恢复处理 |
| Deferred 检查 | wait0 一次；调用方负责轮询。可重新打开的 fixture 在没有 hint 时使用 5s，重复 pending 严格推进时间；这不创建 SDK 轮询默认值 |

## 替换与退出要求

下表穷尽本候选有意淘汰的项目。未来范围明确的 0.x 变更同步 Unreleased、消费者、schema、fixture、当前文档和中文版；标注为历史的材料保留。每行都必须满足所述退出证明；不永久保留重复实现。

| 现有职责/依赖者 | 处置及替代 | 保留/改变的行为与迁移 | 过渡、测试与退出证明 |
|---|---|---|---|
| 第二次模型查找；mapped/direct adapter、factory | 替换为 AIR-001 最终绑定 | 保留 model/wire/auth 语义；有效覆盖现在实际执行。影响 prepared adapter 契约、picker、compaction 和外部 SDK Provider | 一次范围明确的破坏性变更；无平行 router。新增捕获路由测试，更新 prepared conformance；静态证明不存在第二次独立查找，并保留回滚 |
| 独立请求 system/tools 读取者；builders/hooks/adapters/mocks | 合并为 AIR-008/020 规范化/时间序列 | 保留便利初始输入；下游唯一 transcript 加入命名变更。迁移公开 Request/message、hooks/session/compaction/示例 | Builder 仅作为规范化入口保留；所有消费者编译。新增时间线测试，更新 fixture，删除重复权威断言；证明不存在旧 adapter 路径 |
| Run 内固定 registry 及按可见性执行；Agent/产品 | 替换为 AIR-009 不可变的每调用修订 | 保留可信来源/授权；可信更新无需子会话，隐藏的活动工具仍可调用。变更 registration/loadout/binding 证据和 hook | 无 run-fixed 回退开关。新增新旧修订及撤销测试，更新来源/证据，保留权限拒绝。证明当前批次稳定，非活动工具零执行 |
| Deny-only before hook/严格参数路径；hook 实现者 | 以 AIR-041 分阶段规范化/修改扩展 | 保持无效 raw 观察可区分；增加已验证值的可变阶段及最终实际值授权。变更 hook 公共 API、工具 fixture/证据/文档 | 迁移全部实现者；不无限期保留第二执行器。新增规范化/修改 fixture；更新 hook 顺序测试，保留已拒绝效果验证。执行值与授权值相同 |
| Config/Model/ProviderKind/error 外壳；公开导出/示例 | 通过 AIR-021 已确立的 SDK 契约删除 | 保留实际 route/auth、ApiKind 和持久化消费者。外部 0.x import 有意破坏；不依赖产品配置 | 无默认 shim。删除仅构造器测试，保留行为；编译仓库消费者，扫描当前旧导出/文档至零 |
| 静态 reasoning_effort 和重叠 compat 名称；config/parser/factory | 删除重复值；只有 AIR-022 替代完成后才合并真实行为 | 保留 store/role/gating/bridge。已淘汰键报错；迁移有效 profile/model 合并及当前 schema/README，不隐含移除会话格式 | 删除前先有新 wire 证明；同一 release 退出旧读取者。新增旧键负例 fixture，更新 wire/schema；只删除无效果测试；唯一 compat 所有者 |
| AWS 当前本地链/文档；库/产品消费者 | 修正/扩展可复用 AIR-023 resolver，保留签名 | 保留安全性；env/profile 优先级改为已选规则，添加类型化来源/完整 default-chain 覆盖。不迁为产品独占；修正有误导性的旧“无 I/O”文档 | 未明确改变时保留来源格式；新增来源/过期/伪 HTTP 测试，保留 process/SigV4；不能将仅本地能力称为完整链。不破坏性转移 SDK 功能 |
| 可写 v1/v2；会话 CLI/harness | 替换为 v3，保留读取者/显式子会话迁移 | 保留旧字节/分支事实；未知保持未知。更新版本/条目/fork/export/help | 无旧追加路径。新增迁移/checkpoint 测试，更新回滚，保留损坏/中断尾部测试；证明源字节和因果分支 |
| 有损/所有权游离的流路径；SDK/Agent | 修正为 AIR-003–006 自有生命周期 | 保留有效事件/内容；改变已证明的损坏/EOF/drop 结果 | 无旧行为开关。新增对抗 fixture，保留正常语义；证明 inner/producer 释放及永久 EOF |
| Start 输出后一概禁止恢复；Agent/harness 测试 | 在高层替换为 AIR-039 | 保留低层流终止及无隐式输出重放；原始失败历史、持久化省略及新调用均可见 | 无双重高层重试策略。更新部分重试测试，新增重启/事件测试，保留账户拒绝；已完成工具只执行一次 |
| AgentLoopConfig.retry/OpiConfig.retry/summary loops 及每次 start 重置预算；Agent/harness/compaction/config/SDK | 低层调度替换为 AIR-007 唯一 call owner；summary 高层恢复用 AIR-039 同一公开机制 | 保留 typed failure、取消、低层冻结鉴权与 attempt usage 未知。公开 CallOptions max_retries/max_retry_delay_ms、产品 [model_call] 可选同名覆盖及 [recovery] enabled/max_retries/base_delay_ms/max_delay_ms 区分层级。Legacy [retry] 键显式迁移报错，不静默将 capped delay/次数解释为高层恢复 | V-05/33 分层计数/events/auth及 summary fixtures 先通过，再退出旧 Agent 低层 loop/summary 私有恢复策略/当前 retry config 路径；V-20 静态扫描当前 readers 归零。更新 schemas/help/双语 docs/config/default-source metadata。保留仍有真实消费者的纯 helpers，无永久并列策略或自动旧键 alias |
| ProviderCollection 日常组装/调用；Agent/产品/SDK | 收敛为唯一 AIR-045 Models | 保留 registry/auth/catalog/prepared 状态；日常方法隐藏组合，高级 prepare 复用机制。迁移调用方/根导出/crate 双语 README/examples；改名本身不改变持久格式 | 同次 scoped 0.x break；V-39/42/20 验证消费者编译/公共路径；旧并列 collection/re-export/router 退出，无永久薄 facade |
| CompletedRequest、AssistantStreamEvent::Error 及 provider/transport 双错误；adapters/Agent/summary/mock/evidence | 替换为 AIR-047 Result/CallFailure | 保留完整失败 partial/usage/来源/typed cause；当前事件只 Err。同步 producer/drain/terminal 匹配/脱敏转发/fixtures/docs。版本化历史事件转换保留原事实 | 先 V-40/41/03/20 覆盖所有终态和 >64 收集；再迁移所有 consumers，旧 wrapper/当前 Error variant/双写路径归零。保留历史只读兼容；当前 calls/writes 全用新契约 |
| 产品独占三 OAuth flow/刷新和旧协议；SDK/产品登录 | AIR-051 可注入完整 SDK flow/refresh | 保留 keychain/TUI 和 ack，修正 JSON/callback/Copilot metadata/expiry/deadline/cancel。具体 endpoints/PKCE/state/fixtures/锁与 store消费者迁移；新OpenAI OAuth不自动加入 | V-43 公共login→fake HTTP→store→chat/refresh全链及canary；同源并发/取消/提交故障覆盖后旧产品具体flow/重复resolver退出，产品只组装可复用机制 |
| 内置旧 provider ID/default wire、全局simple预算；config/store/catalog/SDK/产品 | AIR-052/053 canonical ID/真实默认adapter/统一mode estimator | 保留明确Chat adapter/profile、历史来源和unknown；迁移config/key冲突、defaultResponses、ModelInfo.provider_id、输出预算/估计helpers及docs | V-44/45先证明defaults/wire/迁移重入及预算；当前旧默认aliases/独立estimator/双clamp退出，历史只读与custom IDs保留；无静默账号覆盖 |

## 验收依赖与完成条件

1. 正式规范协调 → 验证：目标方向及受影响的正式条款经过正常权威流程评审；本文本身保持非规范性。
2. 绑定/流 → 验证：V-01–05 捕获路由身份、分帧及基于同步的清理。
3. Transcript/args/replay → 验证：V-06/07/15/16/34–36，单快照回归及唯一规范化/授权路径。
4. 会话迁移/压缩 → 验证：V-17–20 源保留、已接受输入的失败状态、修订、分支及有意压缩损失。
5. Compat/AWS/淘汰 → 验证：V-09–15/20 正确来源与 wire 矩阵、消费者编译及显式旧键错误。
6. SDK 操作 → 验证：V-21–26 公开入口后的真实 adapter、资源/timeout/retry/未知用量结果。
7. 已发布产品/deferred 路径 → 验证：V-27–33/37/38 实际所有者接通、cache/transport/恢复清理及重启 fixture。不据此推定完整 MCP/codemode 或未知远程 adapter。
8. 独立 SDK → 验证：V-39–45 的公共调用/结果对应、外部 consumer/package smoke、完整 OAuth、canonical 身份及 native/simple mapping。
9. 显式 telemetry → 验证：V-46–48 全部已选 options 保留 context，独立 Dispatch/异步生命周期隔离，no-op/捕获失败不改变业务或 durable evidence。
10. 独立交付评审 → 验证：每条 AIR 均有映射，每项退出都有替代和测试，英中语义一致，并通过选定仓库文档检查。不执行未获批准的实施/live 调用。

这些是依赖和完成条件，不是工单或进度账本。全部 56 条 AIR 必须映射到 V 和证据/决策；本次文档工作尚未执行运行时验收。

差分输入使用固定的人工 fixture。只归一化无关 clock/request ID 和双射工具 ID 重命名。保留 raw/effective 类型转换差异、已声明的 Rust 整数精度差异、内容字节、事件顺序、终止/取消/错误、usage 缺失/零值、关联、section 位置、投影事实和恢复 attempt。在事件发生时捕获 JS partial snapshot。不能通过归一化把 pi 缺陷变成成功，也不以模型输出作为确定性判据。离线测试不证明 Provider 服务接受、模型遵从、媒体准确性、缓存/性能收益、实际账户身份、账单准确性或外部嵌入方兼容。Live 测试需要独立授权，不属于默认完成条件。

## 证据导航与剩余边界

| 契约 | 修改时阅读 |
|---|---|
| 基线/内容身份/实际测试 | [README](README.md)、[基线](baseline.json)、[流探针](offline-probes.md)、[路由探针](route-probe.md) |
| 当前实现及 15 个主题比较 | [证据矩阵](evidence-matrix.md)、[审计](design-audit.md)、[O 笔记](opi-audit-notes.md)、[P 笔记](pi-contracts-notes.md) |
| 当前目标决策/历史 | [决策](decisions.md)、[Q1–21 重新评估](decision-reassessment.md)；D-19–34 取代受影响的历史回答 |
| Transcript/规范化/strict/grammar | [TP](transcript-target-proposal.md)、[TM 迁移](transcript-migration.md)、[R 重放](replay-contracts.md)；TP-13/14 精确语料/关键词细节 |
| Compat/AWS/操作 | [SP/SE 与 SDK 附录 A–D](sdk-target-proposal.md)、[OP](operations-design.md)、[官方协议补充](provider-contract-addendum.md) |
| Catalog/WS/延续/恢复/deferred | [EP/EPT](evolution-target-proposal.md)、[RE](reassessment-evolution-notes.md)、[设计演进](design-evolution.md)中固定的 tag/main 时间窗口 |
| 独立 SDK 调用/执行/结果 | [SDK consumer](sdk-consumer-contract.md)、[pi caller](sdk-pi-caller-review.md)、[opi caller](sdk-opi-caller-review.md) |
| SDK OAuth / builtin identities | [OAuth/provider migration](sdk-auth-provider-migration.md); OA-01–06, BP-01–04 |
| Telemetry option/module/backend/domain 区别 | [Telemetry design](telemetry-design.md)、[TC contracts](telemetry-pi-contracts.md)、[TE evolution](telemetry-evolution.md)、[TO audit](telemetry-opi-audit.md) |
| 独立 ai/telemetry 报告交叉核查 | [Report cross-check](ai-telemetry-report-crosscheck.md)：回源支持、表述纠正、已选差异及未执行验收；不是另一套目标权威 |
| 机械验收 | [V-01–48](acceptance-matrix.md)；断言是未来验证设计，不是测试通过声明 |

独立评审已完成，候选仍保持 DRAFT：正式规范协调及实施准入尚未进行。Q22–Q31（包括修订 Q31）确认修订目标；Q32–37 确认 SDK 范围。没有目标行为选择留给实施 Agent 自行推断。源码/运行时性能/live 缺口如实列出，不猜测路线图。精确 npm 依赖证据固定 AWS 和图片默认值；未来 Rust 依赖的选择/conformance 属于实施验证，不授予改变目标来源策略的权限。

当 submit/fetch/cancel/expiry/account 协议和 fixture 齐备时，重新评估被排除的远程 deferred；当固定协议范围获得选择时，重新评估其余操作 adapter；当独立产品依赖和权限经过评审时，重新评估完整 MCP/codemode。只有采用情况已验证、语义稳定时才重审实验路由/Lite。真实独立非 LLM consumer 或已证 tracing 契约缺口触发 telemetry 独立 crate 重评；自动 SDK spans/exporters 需另选 schema/lifecycle/data policy。改变这些范围边界需要显式修订目标，不能在实施期间自动跟随上游。

# 按“pi 的 Rust 实现”重新审查 Q1–Q21

> 非规范性复审，2026-10-03。用户在首轮候选形成后明确更新判断基准：本轮将 opi-spec 作为可能过时的参考，以 pi 的设计意图、实际演进和实现判断目标；Rust 差异必须说明其具体原因。
> 本文保留原回答的历史，不把此次纠偏自动当作对每个新方案的批准。下列逐题建议保存复审提出时的历史；后续Q22–31及修订Q31已经明确回答，当前选择见decisions D-19–29及重写的44 AIR，不再把历史待决表当作当前状态。只更新本研究目录，不修改正式规范或运行时。

## 结论与判断方法

**需要调整，而且涉及语义和范围，并非只改措辞。** 首轮有三个判断失误：把旧 opi core 准入规则当成 pi 新能力的否决条件；把 opi 没有接线的配置误归为没有保留价值；把更严格的拒绝、不可变装配和无重试偏好笼统归入 Rust 的合理差异。这会让候选设计成为独立的受限系统，而不是忠实承接 pi 演进的 Rust 实现。

后续采用以下区分，既不以旧规范挡住创新，也不机械复制 pi 的缺陷：

| 类别 | 判断与处置 |
|---|---|
| pi 已落地的语义或设计演进 | 以功能和可观察行为对齐为默认目标。当前 opi 缺消费者可能正是缺实现，不能循环地作为不做的理由。 |
| Rust 实现方式 | 所有权、Arc/不可变调用快照、enum、异步取消、类型化错误、编译/链接边界可以不同；说明它们如何保留行为。Rust 本身不要求拒绝 coercion、自动兼容投影或动态工具。 |
| opi 存量迁移 | 旧文件缺失的历史不能恢复；可保留原文件、版本边界和未知标记。迁移事实不等于未来所有运行都必须固定 schema 或创建 child。 |
| 独立策略或范围分期 | 默认 strict、最大资源量、SDK-only 首批范围、重试预算都属于行为选择。可保留，但必须用用户需求和代价解释，不能伪装成语言要求。 |
| 已知缺陷、实验或未接受提案 | 内容损坏、终态漏洞不值得复制；实验和提案不自动成为目标。分别标注实现状态和证据，不用“pi 有文件”代替判断。 |

pi 的职责划分也不是“全部放进 pi-ai”：例如远端目录的 URL/cache/启动刷新仍在 coding-agent，provider 的 refresh/snapshot 能力在 ai；产品与库应共同实现完整行为。

## 逐题复审

“保留”指目前没有发现应撤回该选择的证据；“拆分”指不能再整题作为一个已确认开关。“分期”不是永久排除，也不再受旧 opi 消费者门槛否决。

| 原题 | 复审 | 差异性质与建议 |
|---|---|---|
| Q1 范围/优先级 | 调整依据，保留已选能力 | 聊天正确性先处理可以是交付依赖；图片/分类和其他已落地 pi 能力不应再以 opi 当前无消费者为排除理由。pi 新语义进入目标清单，再明确分期。 |
| Q2 有边界的破坏性调整 | 保留 | 允许更换实现与收敛入口，仍须保留替代行为和迁移证明。不是按接口数量删代码。 |
| Q3 首先服务 SDK | 保留为首批范围 | 这是用户的范围选择，不是 Rust 差异。pi 有 codemode 的实际产品调用，后续接线不能被“库已有 SDK 即完整对齐”掩盖；也不等于必须新增同名终端工具。 |
| Q4 唯一 prepared model binding | 保留 | 已有离线反例证明选择与执行不一致。一次拥有完整 binding 是合理 Rust 实现，保留 pi 的实例选择、覆盖和调用一致性。 |
| Q5 完整 transcript | 保留，调整表示 | 用户当时选择对齐是正确方向。pi 有 named sections 差分、工具增删/同名重定义和 compaction checkpoint；不能用无结构全量文本快照损失这些语义。Rust 存储编码可不同。见 RA-03/04。 |
| Q6 公开表面/compat 淘汰 | **拆分并撤回部分删除建议** | 旧 Config/Model 等无职责壳可继续审退；compat 中多项在 pi 有真实 wire 行为，应补齐或合并到等价契约后再退旧键。不是“opi 没读，所以功能无用”。见 RA-01。 |
| Q7 工具图片 | 保留并补齐投影 | 真实发送或明确 unsupported 正确；须包括 pi 用协议允许的其他消息位置承载图片的路径，不能把“非原生 tool-result 图片块”直接等同不支持。 |
| Q8 AWS discovery 移产品 | **建议撤回原迁移方向** | pi-ai 的默认 AWS chain 是明确 SDK 能力。把唯一可用实现移到产品会削弱嵌入使用者。保留可复用 provider/resolver 能力，产品负责启用、来源配置和权限策略。见 RA-02。 |
| Q9 流生命周期 | 保留 | 终态、EOF、取消和本地释放修正有直接证据。owned 值和有界队列是资源/正确性取舍；不要求照搬 JS 共享 partial 或弱 EOF。高层重试另审，不能由此一并禁止。 |
| Q10 图片/分类归 opi-ai | **保留结论，撤回“破例准入”前提** | pi 0.99 已把平行 ImagesModels 合回 Models/Provider，共用身份/auth/catalog。按新定位自然以这个方向为目标；旧规范是需同步的参考，不能反过来逼选 Companion。 |
| Q11 旧会话 child 迁移 | 保留事实约束，重标政策 | 缺历史原文必须 unknown，不能补造。pi 在旧会话后续追加当前状态，并不普遍强制 child。opi 显式 child 可作为保护现有文件的迁移选择保留，但不是语言或 pi 要求。不能推广为每次工具更新都建 child。 |
| Q12 默认 strict 投影 | **建议改为 pi 式自动适配** | pi 按能力原位发送或自动 collapse，是正式发布的兼容语义。默认拒绝会令普通 Gemini/Bedrock 等历史变更无法继续。记录 projection；严格保真可作为显式策略。见 RA-03。 |
| Q13 首批协议/provider | 保留为分期，不作为完整对齐 | OpenRouter 图片与两种 System One binding 是可测首批；pi 的 llama.cpp 分类、其他已落地 adapter 应记为目标差距。tag 后变化单列，不冒充 1.0.0。 |
| Q14 新操作 timeout/retry | **拆分并建议改默认** | typed nonstream、分类完整结果、失败 usage 可保留；分类默认零 retry 与强制总 deadline 并非 Rust 要求。pi 图片零 retry，System One 默认 2 次重试；可选总体预算与既有 attempt timeout 分开。见 RA-07/10。 |
| Q15 固定 registry/schema | **建议允许可信动态更新** | pi 有真实的动态 loadout、工具注册/替换和逐轮准备。冻结每 call，而非冻结整个运行；可信 host/extension 更新应在边界原子安装，不能被模型声明自行授权。见 RA-04。 |
| Q16 失败/截断历史 | 部分保留、细化 | Error/Aborted 排除 wire 而保留审计有依据。Length 需区分可读内容、完整合法调用、截断/坏参数及结果状态，不能仅因 stop reason 一律删除全部工具 proposal。见 RA-05。 |
| Q17 缺失/失败 tool result | **建议放宽为真实的恢复投影** | 结果缺失不等于未执行。pi 合成 ResultUnavailable 语义的 error result；可只在请求视图补齐，不宣称 NotExecuted，也不自动重执行。malformed 已配对失败和有歧义的关联分别处理。见 RA-05。 |
| Q18 保留/验收/延后打包 | **必须拆开** | 现有测试边界、未知 usage、无隐式换 provider、独立 live 授权保留。远端目录、WS、deferred tool loading、continuation 和高层恢复不能再以 opi 无需求为默认排除。deferred responses 的公共生命周期与 durable 消费已交付，内建远端 adapter 尚未证实。见 RA-06/07/08。 |
| Q19 durable 输入语义 | 保留核心，审减额外约束 | provider 失败后保留已接受输入与 pi 路径相符；单一持久化 owner 有价值。但 immutable assembly、每次变化 child、双 durable append 不是 pi/Rust 必然，要与动态更新及真实证据消费者重新核对。 |
| Q20 未声明工具拒绝 | **拆分 active 与 wire-visible** | 从执行 loadout 移除后应拒绝；仅未出现在当前 wire 声明中，可能是 codemode/deferred/hidden projection，不必然不可执行。注册、active、可见、授权需分别判断。见 RA-04。 |
| Q21 固定资源边界 | **保留有界原则，重审形状和上限** | 原值是用户已选的首版预算，不是 Rust 或 pi 限制；System One 的10档、image非空prompt及固定输入顺序另有具体反证。协议硬限制与本地预算分开，保留有序blocks，再重定可覆盖预算或明确差异。 |

## 重要调整及证据

### RA-01：配置没有被 opi 使用，不证明能力可以淘汰

pi 的 `openai-completions.ts` 实际使用 supportsStore、supportsDeveloperRole、supportsReasoningEffort 和 requiresAssistantAfterToolResult：分别控制 store 字段、system/developer role、供应商推理协议以及 tool-result 后的兼容桥接。它们不是纯类型残留。原 O-004 对“opi 未读取”的静态事实可以保留；从该事实直接推出 Q6 删除，证据不足。

推荐逐项对齐行为，不保留任意旧名字：developer-role 能否统一到现有 system_role_override 需要补自动选择和 precedence 证明；reasoning map 不等于控制是否发送 reasoning_effort；Chat store 与 Responses store 不是同一个 wire；assistant-after-tool 需实际实现其兼容语义。重复的静态 reasoning_effort 值来源仍可合并到调用级 ThinkingConfig。保留 Config/Model 旧壳的退役评估，撤回这几个能力的零行为删除。完整定位与反证见 [SDK 复审](reassessment-sdk-notes.md)。

### RA-02：AWS chain 是 SDK 功能，不只是终端产品政策

pi 的 Bedrock provider、README 与 credential fixtures 明确支持默认凭据链、profile 及工作负载身份；adapter 特意处理 scoped profile 与 ambient credentials 的优先级。Rust SDK 同样可以通过显式 provider/resolver 实例提供此功能，无须引入导入副作用。

推荐保留共享机制给 opi-ai 的独立调用方；产品选择 profile/chain、提供策略及 override，必要的进程权限和资源限制不消失。是否拆内部文件不改变 SDK 可用性。本次证据不支持仅凭 §8.1/PRIN-003 强制迁到产品，也不授权现在启用真实凭据链。

保留确定且可测试的来源优先级、失败关闭、进程限额与清理保障，但不能无条件保留旧 precedence：opi 先 env 后 profile，pi 明确区分 explicit/scoped profile 优先与 ambient profile。10s/64KiB 是现有 opi 默认资源策略，不是 pi/Rust 强制值。bearer 与远端工作负载凭据链也仍是 SDK 差距，不能将本地 resolver 原样保留说成完整对齐。见 [SDK 复审](reassessment-sdk-notes.md)。

### RA-03：transcript 的目标是时间线加兼容投影

pi 0.86.0 的 changelog、`utils/transcript.ts` 和 [PR9548](https://github.com/earendil-works/pi/pull/9548) 一致：记录有序 section/tool 变化；能力足够时原位发送；否则折叠为当前 system/tools 后继续调用。此 fallback 是经过明确设计的行为，不应先当成应禁止的缺陷。它有语义/缓存代价，记录事实即可，不能谎称无损。

建议默认采用这个能力驱动策略；严格重放保真作为明确请求的不同目标。保留 canonical 历史，避免回写折叠结果。pi 的 `Context.systemPrompt/tools` 便利输入仍存在，经 normalize 变成唯一 `TranscriptContext`；AIR-020 应退出 adapter 双重状态读取，不必删除有清晰转换的公共便利入口。见 [transcript 复审](reassessment-transcript-notes.md)。

### RA-04：每次调用冻结，不等于整个运行的工具永远不变

pi 的 prepare-next-turn、loadout、注册工具与 transcript 差分共同支持在模型调用之间更新定义。Rust 可对每次调用持有不可变的定义/实现 revision，下一轮原子安装新 revision；活动调用不会看到半更新。可信更新与模型返回的声明是两个来源，后者不能自己获得执行权。

建议恢复 named sections、add/remove/redefine 的可观察语义。全量 snapshot 若采用，必须证明与 patch 重放和 provider 增量发送等价；不能为了少一个数据类型丢掉 section 身份。只有真的需要新会话身份或无法表达存量迁移时才创建 child，而不是普通动态工具更新必做。

Q20 还须把“当前 active/executable loadout”与“当前 wire declarations”分开：pi 的 hidden/deferred/codemode 路径提供反证。普通已移除的工具仍拒绝，但隐藏声明不应自动撤销仍有授权的执行能力。详见 [transcript 复审](reassessment-transcript-notes.md)。

### RA-05：恢复结果缺失，不需要猜测副作用

pi `transform-messages.ts` 补出的 error tool result 仅说明没有结果，0.69.0 将末尾未解决调用的补齐列为修复。原 Q17 把“补一个结果”误等同“声称未执行”，漏掉了中性恢复路径。

推荐在调用关联确定时允许 request-local 的 ResultUnavailable；保留 canonical 原文和 UnknownEffect，不自动执行、重试或宣告工具失败已回滚。有歧义的重复 ID/orphan 仍应诊断；合法跨 batch ID 复用不属损坏。可信 NotExecuted 仍可用于已知未执行的坏参数恢复，但不是所有恢复的唯一前提。Length 不应单靠 stop reason 批量抹掉完整合法历史；保留历史结构不授予执行 Length 提案的权力，pi Agent 仍用明确未执行结果处理它。见 [transcript 复审](reassessment-transcript-notes.md)。

### RA-06：已有产品路径不能继续被当作未来设想

远端目录：pi 的 ModelRuntime 包装 provider，先读缓存，interactive/RPC 的非 offline 路径实际触发 refresh。目录服务/cache/启动策略仍属于产品，库保持 snapshot/refresh/cancellation/stale-publication 契约。

WebSocket：pi 的产品 transport 默认 auto 并传到 Codex adapter，存在连接缓存、续传和 SSE 降级。没有 opi 性能测量不足以否定其语义/接线对齐；性能收益仍不得臆测。

Deferred tools：tool_search 在 CLI 内建但默认 inactive，MCP 的 deferred 配置会激活它；这是已发布可选功能。Deferred responses 则已有公共生命周期、faux 和 durable fetch/cancel/poll 消费；未证实的是内建远端 adapter 与普通 coding-agent 的默认远端路径，不能把整个能力称作提案。Thinking continuation：Responses encrypted reasoning、Google/Vertex thoughtSignature、Bedrock reasoning 与 Chat reasoning_details 不能因为 opi 暂时缺消费就一并排除，它们直接影响现有模型接续。

建议把上述已落地能力放回对齐目标，再按依赖排序；deferred response 的现有公共契约列为已交付差距，真实远端能力另待协议证据。虚拟分类路由仍是实验；完整 Responses Lite 未交付、无采纳承诺，RFC0054 的 additional_tools 等子机制已有落地或替代。分层、默认接线和版本定位见 [演进复审](reassessment-evolution-notes.md)。

### RA-07：低层流不可重复，不等于高层失败回合不能重试

pi 有 HTTP retry、WS 传输恢复、assistant-call utility 和产品整回合恢复四层。原 AIR-007 的输出后限制可继续约束同一公开流的隐式重放，但不应阻止有显式 retry 事件、失败历史投影、取消/预算和可区分恢复尝试的高层恢复。logical call/attempt 的具体身份映射仍须设计，不能冒充已核验的 pi 通用契约。

新操作也应区别协议：pi 图片零默认 retry；System One 默认 maxRetries=2，每 attempt 的 timeout 与 caller 的总体 deadline 不是一回事。Rust 没有要求二者都归零。推荐对齐默认重试行为并允许显式总体预算，避免强迫所有 SDK 调用者先提供另一套期限策略。见 [演进复审](reassessment-evolution-notes.md)和 [SDK 复审](reassessment-sdk-notes.md)。

### RA-08：schema normalization/coercion 是维护中的语义

根 Agent 本轮补核：`packages/ai/src/utils/validation.ts:315–339` 克隆参数后进行 optional-null normalization 与 schema-guided coercion，再校验；`test/validation.test.ts:64–202` 覆盖数字字符串、布尔值、nullable union 优先保留及拒绝非法转换。Agent `agent-loop.ts:723–728` 实际使用结果，不是仅导出的帮助函数。

演进反证：changelog 的 0.84.2 把 strict schema 的 optional→required-nullable 转换和返回 null→省略联动发布；更早的 #786、#3112、#7328 持续修复转换和 union 语义。不能把它当 TypeScript 不严格造成的偶然行为。当前 opi `validation.rs:11–20` 直接 jsonschema 校验，严格拒绝也是可选语义，但并不自动与 pi 等价。

推荐删除 AIR-024 中无条件“no implicit coercion”的目标，改为有明确规则和差分 corpus 的 normalization→validation→authorization→execution；raw arguments 保留，effective arguments 用于实际执行。Wire constrained sampling 的 prefer/require、能力 gating、strict schema 转换也应补齐，不只是保留当前 Chat/Responses 的全局 strict boolean。根证据：pi `types.ts:705–719`、`api/constrained-sampling.ts:97–143,228–249`、README:543–557；opi `message.rs:147–150`、`openai_chat.rs:1056`、`openai_responses.rs:292`。Grammar 是另一项已发布能力，不能从 JSON strict 自动推定已实现。没有建议复制根值校验或 JSON repair 中任何已确认缺陷。

### RA-09：持久化事实与额外治理机制分别处理

保留不可虚构旧 system/tools、原始审计数据及 provider 失败后已接受输入。pi 的消息事件先进入 session 再发生后续模型请求，支持后一个目标。

固定 assembly 与绑定 child 的约束源于 opi 的存量模型及首轮选择；“每 call 两次 durable append”则是首轮候选新增的 pre-call/projection/dispatch 证据设计，不是当前实现事实。当前已有的是 prepare/commit acknowledgement 和单事务 owner。前述额外限制都不是 pi/Rust 的必然要求。

重新设计时以可还原的逐 call revision、branch 与来源事实满足需求，不必把旧 INV-008 形状当唯一解。显式 legacy child 是仍可保留的安全迁移方案，不能把它混入正常动态语义。见 [transcript 复审](reassessment-transcript-notes.md)。

### RA-10：首批覆盖与资源预算不能冒充 pi 能力边界

保留 typed image/classifier 与共享 provider/auth/catalog 的方向，保留用户选择的第一批 adapter 作为实施顺序。尚未实现的 pi adapter/参数/结果形态列为差距，不以旧准入规则永久延后。操作模型数据来源也需随真实静态/动态目录对齐，不能把“调用者显式注册”作为唯一长期入口。

Q21 的有限内存原则合理；具体张数、JSON 大小、问题数量、MIME 集合、URL-only 行为和概率校验强度属于可观察限制。需逐项区分协议硬约束与本地预算。具体反证：pi image 接口接收有序 text/image blocks，候选的必需非空 prompt+references 会缩窄表达；最多10档是 llama.cpp 的单token实现限制，不能当成 System One 协议限制。pi 的 score 返回也不保证存在完整 distribution 供均值校验。

推荐保留有序输入与操作特有语义，字节/数量防护作为明确且可覆盖的 SDK 预算；无需因此新增产品配置或通用框架。此前用户确实确认过固定值，变更仍需明确决定；不能称它们因 Rust 必然成立。当前完整对照及其他选项缺口见 [SDK 复审](reassessment-sdk-notes.md)。

### RA-11：thinking effort 的拒绝与高层 clamp 也是语义差异

继续推进时补核：pi `models.ts:1215–1246` 用 off/minimal/low/medium/high/xhigh/max 的固定顺序；map 中 null 排除等级，xhigh/max须显式支持；高层 adapter 的 simple 路径对不可用请求先向更高可用等级查找，再向更低查找，非reasoning模型归 off。Chat/Responses/Codex/Google/Vertex均调用该 helper。低层原生选项的语义另行区分，不能将所有入口概括成同一种 clamp。

opi `model_info.rs:204–272` 的 Unsupported 与 `provider.rs:327–347` 会在启用 thinking 时提前拒绝；这避免隐式参数变化，但不是 Rust 类型系统要求。推荐公共便捷调用采用 pi 的支持等级选择并分别记录 requested/effective level；明确的 native/exact请求仍须验证其实际 wire 能力。预算式 provider 的 token 分配与 off wire 表达另按 adapter fixture，不把字符串 map 当完整功能。该建议尚未获新的目标确认，不覆盖原 Q18 保留的已实现映射事实。

### RA-12：compaction 的有效 checkpoint 不等于逐历史时点保真

继续细化时补核 pi `session-manager.ts:1270` 的 checkpoint 与 `buildContextEntries:499–510`：压缩记录当前完整 system/tools；重建在 summary 后保留非system历史，过滤 compaction 以前的 retained system entries，再接 compaction 以后变化。因此它保留的是压缩后的有效当前状态和后续演进，压缩本身有损；首版 AIR-018/V-19 要求“每个 retained call 边界完全等价”更强，不能继续称为 pi 对齐。

推荐 canonical审计历史仍可沿未修改的分支事实查询，给模型的压缩上下文明确使用 checkpoint 与后续事件；记录压缩边界和已知性，不把旧历史洗成已知。完整逐时点保留如仍需要，应由人明确选择其语义和成本，不能由执行 Agent 决定。精确测试与来源集中于 transcript-target-proposal。

### RA-13：trusted hook 与最终参数校验的顺序需单独决定

pi `agent-loop.ts:723–753` 先 normalize/validate，随后 beforeToolCall可修改 validatedArgs，直接执行；`agent-loop.test.ts:487` 明确断言不再次校验。模型不能借此自行注册或授权工具，但可信 hook 实际可越过原schema形状，这是公开扩展契约。opi `agent_loop.rs:2250–2284` 的现有hook观察raw clone、只返回Continue/Deny，随后校验；它目前根本不提供参数改写，不应描述成已支持 mutation后重验证。新增可信参数改写后是否复验是独立选择，不能称为 Rust 必然或 pi 当前顺序。mandatory User Policy授权仍应面对真正executed值；模型不获得改写/授权入口。

在本条提出时，Q23只询问模型输入normalization与结果恢复；后续Q25已另行确认pi hook不复验及最终授权。**不把该题答案扩大成 trusted hook 最终值的默认批准**。目标需记录 raw / normalized / hook后的executed值及适用schema revision，并另行明确是否要求hook后重验证。plain JSON Schema的转换规则与TypeBox私有元数据路径也要分开，不宣称两条路径相同。

## 对旧候选的影响与决策状态

明确保留的基础：有效模型绑定、终态/EOF/UTF-8/本地资源正确性、真实图片输入、owned 调用快照、typed 错误与脱敏、usage 缺失不等于零、无隐式换身份、离线验收和 live 独立授权。它们有直接正确性或明确调用语义依据，不只来自旧规范。

必须重审的旧要求至少包括 AIR-001/002、AIR-007–013、AIR-016–020、AIR-022–031，以及 scope/exclusions、thinking coverage、retirement 和工程附录中相关内容。AIR-021 的旧壳淘汰与 AIR-022 的 compat 能力不能再捆绑。AIR-003–006 的已证实流缺陷仍成立。

新定位直接确认的是**判断基准**，不是自动批准这里列出的每个具体默认值。原 Q6/Q8/Q12/Q14/Q15/Q16/Q17/Q18/Q20/Q21 的有关部分需按这份证据重新决定；Q11 可保留为明确的存量迁移政策，但不能再用它证明 pi 等价。未解决这些调整前，两版 spec 保持 DRAFT/重审中，不能继续宣称“所有目标无待决项”。

正式文档的 AGENTS 开头、opi-spec §2/§3.2/§8 和 Runtime Input Binding 描述仍记录旧定位；本次按用户新指令研究，旧文本不否决建议。未来正式同步的内容应由目标决定，而不是为每个 pi 创新先设计一个小例外。本次仍不编辑那些文件，也不启动实现。

## 本轮核验边界

本轮是源码/测试断言/历史与官方页面复核，没有新增运行时测试、模型调用或性能结果。此前 70 个离线测试与探针仍只证明其原有范围；旧评审通过针对旧决策前提，不证明此次调整已获批准或实现。

新增专题材料拥有独立文件边界，主 Agent 汇总冲突；三个专题的事实依据可导航，不建立第二份权威或进度账本。当前官方材料核验以固定的 1.0.0 和 design-evolution 所记 main 窗口为界，不声称掌握无依据的长期路线图。

主文经三个专题研究者交叉复核，已修正数处限定：deferred responses 的真实 durable 消费不能漏掉；完整 Lite 与已落地子机制不能混同；“新调用身份”不是已核验通用 pi 契约；AWS precedence 和候选双 durable append 必须分别与 pi 行为、opi存量事实区分。当时没有将建议当作用户再次批准；此后具体回答已记录在D-19–29，批准来源是用户实际回复。

本轮测试影响 **none**（仓库测试 add/update/delete 为0，也未重跑runtime测试）。只读文档核对覆盖本目录27文件的UTF-8/空白、419个本地链接、3个JSON以及双语候选的32个AIR和重审状态；结果无任务内错误。全仓库diff检查的旧manifest问题仍保留，具体命令与边界见README。

## 后续选择完成后的状态

Q22–31及修订Q31已全部实际回答，替代关系见decisions D-19–29。第二轮候选现为44 AIR、38组V；严格投影、固定run-wide工具、无结果一律拒绝、真实compat删除、AWS产品独占及全面零重试均已按证据调整。本文先前“待决定／暂停”表述是复审历史，不再代表当前未决项。固定1.0.0基线与tag后main窗口保持分离；没有把推断变成路线图承诺。实际新验证与第二轮评审见README和spec-review-round-2.md。

# Transcript、动态工具与历史 replay：按 pi 意图重新审视

> 非规范性研究，2026-10-03。用于重新讨论 Q5、Q11、Q12、Q15、Q16、修订 Q17、Q19、Q20 及 AIR-020；不修改这些决定、不批准实施。本文采用本轮用户纠偏后的评价前提：opi 作为 pi 的 Rust 实现，非语言差异应首先根据 pi 的意图、落地和演进判断。`opi-spec` 仅是可能过时的参考，不能作为拒绝 pi 行为的理由。

本文件只记录本专题的新证据与建议。旧 [transcript 研究](transcript-migration.md)、[replay 研究](replay-contracts.md)及候选 spec 保留此前调查和决定，不应把其“已确认”误读为本轮重新确认。研究材料按要求使用中文，没有需同步的英文 counterpart。所有本地测试证据均为静态阅读，**本专题未运行测试**。

## 结论与重新讨论范围

| 原问题 | 重审结论 | 建议调整 | 差异类别 |
|---|---|---|---|
| Q5 完整 transcript | 方向与 pi 一致；但“完整”不只是每轮最终快照 | 保留时序 system/tool 状态、分支恢复和 compaction checkpoint；重审 Q15 限制 | pi 已落地创新 |
| Q11 旧会话必须显式 child | 旧 opi 缺少原始历史确实不能恢复；强制 child 并非 pi 的迁移契约 | 保留 unknown 和源事实；继续位置追加当前状态。是否要求 child/显式接受缺口属于产品政策 | opi 存量限制 + 独立产品政策 |
| Q12 Strict 默认 | 与 pi 有意、已发布的自动 collapse 默认相反 | 默认按 adapter 能力原位表达或 collapse；可额外提供显式严格策略，但不能把它说成 Rust 必需 | 独立产品政策；不是 pi 缺陷修复 |
| Q15 完整快照 + 固定 registry + schema/实现变化 new child | 声明不授权执行值得保留；固定整个会话执行集合与 child 约束缺少 pi 依据 | 允许可信宿主动态变更工具定义/实现；每次调用冻结自己的可信 loadout；保留 pi append/sections/tool-delta 可观察语义 | Rust 生命周期要求 + opi 存量迁移限制；当前限制是独立政策 |
| Q16 失败/Length 投影 | Error/Aborted 不回放有 pi 直接依据；Length 无条件去掉 proposal 并改普通文本不是 pi 通用契约 | Error/Aborted 过滤保留；Length 不执行工具，但保留可表达的结构化 proposal/error-result；产品恢复可独立省略整次 attempt | 前者对齐；后者原决定为独立政策 |
| 修订 Q17 缺 result 仅凭可信 NotExecuted 补偿 | 将“结果缺失”与“没有执行”混淆，阻止了 pi 特意支持的中断恢复 | library request projection 可生成明确结果缺失的 error result；不改写 canonical、不声称未执行、不自动重做工具 | pi 已修复的恢复缺陷；原限制为独立政策 |
| Q19 durable commit 后保留 | 已接受输入在模型失败后保留与 pi 方向一致；全部事务/dispatch 证据协议不是 pi 必需 | 保留该外部行为和 opi 现有事务保证；逐项论证额外证据成本，不能把 immutable binding child 当 pi 要求 | 对齐行为 + opi 存量事务边界 + 独立审计政策 |
| Q20 wire 未声明必拒绝 | pi 区分当前执行集合与声明投影，确有声明隐藏但仍可执行的路径 | 按当前可信执行 loadout/调用上下文判断；不能把 wire 可见集合一概升级为授权上界 | 独立产品政策 |
| AIR-020 旧 system/tools 入口退出 | 消除 provider 下游双重权威与 pi 一致；删除一切便捷输入并不一致 | 保留初始 `systemPrompt/tools` builder/shorthand，统一 normalize 后 adapter 只接 transcript | 合理 Rust 表达差异；无需另一个 legacy runtime |

这里“建议调整”是提交主 Agent 与用户重新审定的输入，不是覆盖原决定。Rust 的所有权与并发安全不要求永久冻结工具 registry、强制 child、拒绝缺结果恢复或默认 Strict。

## RT01 — 默认 collapse 是明确设计，不能归为历史负担

**当前源码与设计意图。** `.repo/pi-1.0.0/packages/ai/src/utils/transcript.ts:104–119` 明确规定：支持中途 system 的模型保留原位消息，否则把有效当前 system/tools 放到请求首部，去掉后续 system 条目。`:220–232` 对仅支持 additions 的工具协议，在 removals/同名重新声明时发送完整当前工具集合。`packages/ai/README.md:1559–1590` 解释 native 与 fallback；`packages/ai/CHANGELOG.md:116–125` 将规范化 transcript 和 automatic collapsing 作为 0.86.0（2026-09-19）已发布行为。

**一手历史。** [官方 PR 9548](https://github.com/earendil-works/pi/pull/9548) 于 2026-09-16 合并，merge commit `9e05370b298d0a6b8d9bc2c02e4bfae189ef1616`。正文把目标解释为记录状态何时变化、恢复/分支时重建、在上游支持时保持缓存前缀；对不支持的模型明确退回旧式当前状态，接受变化时的 cache miss。它没有把“支持动态 transcript”解释为所有模型都能保持过去每个 system/tool 边界。

**反证检查。** pi 并非对任意模型宣称 native；compat 以具体 adapter/模型能力区分。`packages/ai/test/transcript-tool-changes.test.ts:82–154` 覆盖 Anthropic 支持、同名重定义 fallback 与 collapse，`:197–262` 覆盖 Responses additions/非 additions 情形。这些是离线构造和 payload 断言，不能证明供应商今天的远端接受度。Collapse 确实改变历史指令出现位置；承认这一点并不说明 pi 不知道或未有意选择它。

**原 Q 与建议。** Q12/AIR-011 默认 Strict 是更强的 opi 产品约束，不能再由“完整 transcript”或 Rust 推出。建议默认采用 pi 的 native-or-collapse，并可观测地说明所用投影；canonical 历史不因投影改变。若仍要提供 Strict，它应是调用方主动选择的策略和明确的行为差异。不能把“当前状态相同”写成“历史语义无损”，也不能让 Strict 的可选存在反向阻断默认模型切换。

**置信度：高。** 有源码、文档、发布记录、测试与合并 PR 的相互支持。尚未核验所有模型远端行为；没有找到宣布未来删除 collapse 的已核验计划。

## RT02 — system 更新具有 append、named sections 与 tool delta 语义

**当前源码。** `packages/ai/src/types.ts:512–545` 的 SystemMessage 除文本外含 named sections、toolsAdded/toolsRemoved。`utils/transcript.ts:73–101` 对正文追加，对 named sections 按名称替换/null 删除，再渲染有效 prompt；`:58–65` 按顺序先移除再增加工具；`:149–166` 将同名定义变化表示为 remove + add。它不是每条 system message 都全量 replace。

`packages/ai/test/system-message-replay.test.ts:86–123` 检查 section 更新后实际文本、shorthand 规范化、去掉 execute 字段及同名定义变化。产品 `packages/coding-agent/src/core/agent-session.ts:1679–1698` 在下一轮计算命名 prompt sections 和 tool loadout 变化；这不是只有 pi-ai 类型而没有实际调用者的实验接口。PR 9548 明确选择变化 sections 与工具 delta，完整 checkpoint 用于 compaction，而非每次更新另存完整 prompt。

**反证检查。** 一组全量快照确实可以重建每个时点的最终字符串/工具集合；这支持 Rust 内部使用 owned snapshot 或 checkpoint。可是从两个不带 section 边界的完整字符串不能唯一恢复 pi 的原始 append/section 更新及适合 native wire 的最小变化。若只发送每次完整 prompt，会改变上游追加指令和缓存前缀行为。完整快照不应不经论证就称为与 pi 等价。

**原 Q 与建议。** Q5 保留。Q15/AIR-008 中“不需要 section-patch DSL”的排除应重审：这是未调查真实产品使用后作出的范围简化，不是 Rust 语言需求。建议目标先定义可观察更新语义（追加指令、命名部分替换/删除、工具增加/删除/同名重定义），Rust 可用闭合 enum/owned 值表达，并在 compaction 生成完整有效 checkpoint。若内部保存快照，必须同时保留足够变化信息，使 native adapter 不必猜 patch；不要仅以最终字符串相同验收。

**置信度：高。** 对具体 Rust enum 布局没有作决策；没有需求要求照搬 JavaScript 对象形态或公开一个通用 patch 框架。

## RT03 — 动态可信工具并非只能选择固定 registry 子集

**源码与真实消费者。**

- `packages/agent/src/agent.ts:82–105` 允许替换 `agent.state.tools`，创建 context snapshot 时复制数组（`:460–464`）。`agent-loop.ts:171–215` 的 prepareNextTurn 可替换下一轮 context；`:323–362` 将当前 executable tools 与 transcript 声明比较，产生增加/移除。`agent/test/agent.test.ts:178–224` 验证同一 Agent 的两次 prompt 从 first 换成 second，第三次不重复声明。
- `packages/coding-agent/src/core/extensions/loader.ts:288–299` 的 registerTool 在可信 extension 中写入定义并触发 refresh；`agent-session.ts:3499–3544` 重建包裹后的工具 registry 与 active 集合。`packages/coding-agent/test/agent-session-dynamic-tools.test.ts:99` 起的测试从 session_start 注册新工具，验证真实 session registry、来源、active 集合和 prompt。
- `agent-session.ts:3612–3650` 在同一 session reload 资源、settings、API providers 与 runtime；没有因工具实现变化自动建立 child 的步骤。`:1522–1571` 的 prepareLoadout 还允许根据本轮条件覆盖面向模型的工具描述。

**后续一手演进。** [官方 commit b271b0a](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db)（2026-10-02 10:30:48Z）将 Anthropic 按名引用工具改为 inline 按值定义，移除同名重定义导致的 native 禁用，增加相应测试，并 deprecated `hasToolRedefinitions`。它是基线之后已落地实现，不是 1.0.0 当前 wire 事实。本专题独立核验了 commit/patch；发布归属沿用总研究的“该观察窗口已合并未发布”，不新增发布断言。Deprecated 注释没有给出删除版本或日期。

**反证与边界。** pi 不是“模型输出什么定义就执行什么”。直接调用仍查当前 `context.tools`，执行实现由宿主/extension 注册（`agent-loop.ts:707–775`）；历史 declaration 去掉 executable 字段（`transcript.ts:122–129`）。另外 `prepareRequest` 在声明计算之后还能替换 context（`agent-loop.ts:219–239`）；本次没有证明每种自定义 hook 组合都自动重新声明一致，不能把它当完整安全保证。

**原 Q 与建议。** Q15/AIR-009 中“整个 run 固定 implementation/schema/source，变化必须新 binding child”属于 opi 既有 assembly 模型加产品政策。建议保留“历史声明不注册执行器/不授权”，但允许可信宿主在 call 边界更新执行 loadout 与 schema。新 call 使用新版本，已准备/进行中的 call 持有自己的稳定定义和执行器；Rust 必须正确拥有其生命周期，但可用 owned/Arc snapshot，无需永久冻结会话。最终参数仍按该调用绑定的可信定义校验，当前权限策略仍由执行 owner 检查。当前 opi 不支持动态 assembly 是迁移工作量，不是拒绝 pi 方向的理由。

同名 schema 变化须在 transcript 中可辨，实现变化虽不一定改变模型声明，也应避免旧 call 被新执行器替换。是否保存实现来源 revision 属于诊断/持久化契约；是否因此强制新 child 需要独立产品理由。**置信度：高**；没有主张所有 reload 都可安全并发发生，也没有要求加入插件框架。

## RT04 — “未在 wire 声明”不等于“pi 当前不可执行”

**共同基本行为。** `agent-loop.ts:707–775` 先在当前 context.tools（或显式传给 runToolCall 的 tools）找工具；找不到时生成错误结果，不执行。被移出执行 loadout 的工具不会因为历史上存在而复活。

**真实差异。** `agent-session.ts:1511–1519` 的 callable 集合包括 active 工具和注册的 codemode/deferred 工具；`:1522–1571` 的 prepareLoadout 可保留执行器但设置 hiddenDeclarations；`:1715–1737` 的 transformContext 仅从 request view 隐藏 toolsAdded/toolsRemoved。当前执行集合与模型可见声明不完全相同。

`packages/coding-agent/test/suite/agent-session-tool-orchestration.test.ts:62–112` 具体断言 active 为 echo/run_tools、callable 为 echo/helper、请求声明只剩 run_tools，同时 nested executeTool 成功调用 helper/echo、拒绝 model-only run_tools。`:133–143` 还有隐藏声明的 echo 被 faux model 直接提出调用的路径。前一项明确证明集合不同，后一项与源码 lookup 一起反证“wire 不可见必禁止”。这些测试未运行；它们使用真实 orchestration 组装与伪模型，不是供应商远端测试。

**原 Q 与建议。** Q20 规定的 NotDeclared 统一拒绝是 opi 新产品限制。建议基础对齐定义当前可信执行集合与请求声明投影分离；执行时查绑定的可信集合和权限，不能由历史 transcript 或模型内容增加权限。需要普通产品把 active 与 visible 设为相同，可作为默认装配。暂不实施 codemode/deferred workflow 并不要求在公共契约中永久禁止这种分离。也不应反过来允许 registry 的所有项无条件被直接调用：pi 的 active/callable/exposure 有真实上下文限制。

**置信度：高。** 这不是要求撤销工具权限检查，而是纠正“声明等于授权”的范围混淆。

## RT05 — missing result 的合成表达缺失，不证明未执行

**当前源码。** `packages/ai/src/api/transform-messages.ts:158–186` 收集 pending tool calls，遇下一 user/assistant 或 transcript 结束（`:191–232`）为没有结果的 call 生成 `isError: true`、“No result provided”的 ToolResult。系统变更暂缓到该批结果之后以满足 wire 顺序，避免把中间 system message 错判为结果批次结束。它发生在 request transformation，并未把合成结果写回 canonical transcript，也没有执行工具。

**一手演进。** [官方 issue 3555](https://github.com/earendil-works/pi/issues/3555) 描述 0.68.1 会话在工具执行中被杀死，tool call 已保存但 result 未写入；已有中间 orphan 修复漏了末尾 batch，恢复后 API 拒绝请求。issue 于 2026-04-22 关闭。交付判断依据不是关闭状态，而是 `packages/ai/CHANGELOG.md:1142–1151` 的 0.69.0 修复记录、现实现的末尾 close，以及 `packages/ai/test/transform-messages-copilot-openai-to-anthropic.test.ts:136–190` 的 trailing/partial batch 断言。

**反证检查。** 工具可能已经执行或产生部分副作用，只是结果未持久化；上述 issue 正好包含这一可能性。“No result provided”没有宣称 NotExecuted。相反，当前 transform 对重复 IDs/orphan results 不做完整验证（`:205–215`），不应将这种宽松直接升格为 pi 的设计承诺。跨模型 ID normalization 的替换/截断也不自动证明无碰撞；保留一一映射与歧义检测是正确性改进。

**原 Q 与建议。** 修订 Q17/AIR-012 中“只有 Agent 凭未执行/中断事实补偿”“未知 missing result 拒绝”仍比 pi 刻意提供的恢复窄。建议把两件事拆开：

1. 对关联明确、结果缺失的 call，opi-ai 的 request projection 可以生成 typed/普通结构化 error result，准确表达 ResultUnavailable；不推断工具未执行、不将其写成 canonical 真正结果、不自动执行或自动重发工具。跨 adapter 保留 ID 关联。具体面向模型文字可使用 pi 同等信息，不能把“可能有副作用”抹掉。
2. Agent 亲自产生的 NotExecuted 仍有用，例如 schema 错误、Length、拒绝执行；它是更强事实。重复/歧义 IDs、orphan result 等无法无歧义关联的历史，可以明确拒绝或提出另有事实依据的修复，不能随意制造成功/执行事实。

opi 的 raw malformed arguments 与 pi 的 parsed arguments 不是同一存储契约。保留 raw 证据、在不能忠实表示时用可见失败上下文，仍优于偷偷 `{}`；但不能用此差异禁止所有可表达的失败 proposal/result replay。参数 salvage/coercion 的目标由主研究另审，本文不默认批准它。

**置信度：高。** issue 原因、发布修复与当前代码一致；没有核验其他所有 missing-result 场景或供应商接受度。

## RT06 — failed assistant 与 Length 要区分库投影和产品恢复

**Error/Aborted。** `transform-messages.ts:191–203` 有意跳过这两类 assistant，注释说明不完整 reasoning/tool blocks 会导致供应商拒绝，应从最后有效状态继续。`packages/agent/src/agent-loop.ts:245–255` 对它们结束 turn，不执行其工具。这支持 Q16 对 canonical 保留、wire 省略的核心方向；对人工构造的关联结果仍需防止错误掩盖。

**Length。** 同一 transform 不过滤 Length。Agent `agent-loop.ts:263–277` 对 Length 中的全部工具提案生成失败结果，不执行；`:471–502` 明确解释 salvage JSON 可能语法有效但不完整，生成“未执行、重新发出完整调用”的 error result，允许模型下一轮恢复。`agent/test/agent-loop.test.ts:415–484` 检查工具没有执行、错误包含 output token limit、随后确实进入第二次模型调用。它没有证明所有 wire 的第二请求都成功，因此 Rust 验收仍应检查真实 adapter 请求。

**产品特定恢复。** `packages/coding-agent/src/core/agent-session.ts:2883–2889,2938–2999` 在 context overflow/特定 recoverable Length 情况下一次性省略该 attempt，再 compaction/retry。`:1208–1223` 用 context-edit 条目移除投影中的 assistant 及 tool results，保留原始 session entry。这说明 pi 允许产品恢复策略改变请求 view，不说明库应把所有 Length 历史统一改成普通 User marker。

**原 Q 与建议。** Q16/AIR-013 建议改为：Error/Aborted 维持上述过滤；Length 禁止执行 proposal，但能用结构化 pair 表达时保留它和明确 error result；产品特定恢复由恢复 owner 移除整个 attempt，不能只丢 half-pair。若 opi 原始坏 JSON 无法组成供应商工具调用，可局部用可见失败上下文，而不把这个例外扩张成所有 Length 的通用规则。先前“Length 保留文本，固定排除所有 proposals/results”的策略属于独立产品政策，缺少 pi 对齐依据。

**置信度：高。** 没有主张 Error/Aborted 的任意外部孤立 result 可以静默删除；这一异常形状需要明确关联检查。

## RT07 — 旧会话 unknown 与 durable 保留不必推出强制 child

**pi 迁移意图。** PR 9548 正文明确旧会话若没有 leading system，下一次请求在继续位置追加当前 system/tools；不重写过去，也不提升 session version。当前 `packages/coding-agent/src/core/session-manager.ts:41` 仍为 version 3。该事实与 pi 初始系统状态可能在会话中间首次出现相容，不保证旧历史每次调用的原始 system/tools 可恢复。

**opi 真实限制。** `crates/opi-coding-agent/src/harness.rs:228–239,311` 保存工具/system digest；`crates/opi-agent/src/evidence.rs:510,1631` 的 binding/identity 不是原始 prompt/tool 内容仓库。旧内容不可逆恢复，unknown 区间必须真实保留。`harness.rs:2410–2446,3295` 的绑定迁移/不一致时 fork child 是当前产品路径，不是语言限制。继续复用它可以减少迁移代码，但不能因“已有 child”就把它当 pi 目标。

**Q11 建议。** 保留“不得伪造旧历史”“旧来源/分支信息可追踪”。普通 continuation 允许按 pi 追加当前可信状态，不要求重建过去。是否用新的存储版本/安全复制迁移是 opi 格式兼容问题；是否必须让用户显式创建 child 或每次接受 unknown 是产品 UX 决策。即使复制到新文件，也不必把它解释成工具 schema 每次变化都开启新语义会话。显式精确历史重建请求遇 unknown 应报告缺口；正常继续和精确重建不是同一成功条件。

**pi 已接受输入持久化。** `agent-loop.ts:171–215` 在请求前发出/接收下一轮输入事件；产品 `agent-session.ts:1074–1144` 在 message_end 保存普通消息。`session-manager.ts:1160–1187` 特别说明从首个 user message 开始落盘，使第一轮未完成时 prompt 仍在磁盘；实现以同步文件写/append 完成，不是“只有成功 assistant 才保存”。`:1191–1213` 更新 entries/leaf 并持久化。

**Q19 建议与反证。** “已接受输入在随后 auth/provider 失败或取消后仍保留”可以保留，符合 pi 方向。opi 已有 `crates/opi-agent/src/hooks.rs:211–224` 与 `agent_loop.rs:1399` 的 prepare/commit ack、`crates/opi-coding-agent/src/compaction.rs:479–640` 单事务 owner，继续整合比重建第二 owner 合理。不过 pi 的 `_appendEntry` 先改内存再写盘，没有证明跨多条 context/leaf 的全部失败原子性；不能声称二者事务机制等价。per-call projection identity、两阶段证据/dispatch fact、immutable binding child 是更强 opi 审计选择，需论证用户价值与迁移成本，不能作为采用 pi transcript 的先决条件。

**置信度：高（源码事实）；中（最小迁移方案）。** 旧 opi 格式的写入/导出全路径在本专题未重新执行验证。格式升级是否必须复制原文件要由具体 reader/writer 兼容检查决定；没有建议原地覆盖历史事实。

## RT08 — 消除下游双重权威，同时保留 Context 便利入口

`packages/ai/src/types.ts:724–748` 明确保留 `Context.systemPrompt`、`Context.tools` shorthand，公共 stream/complete 接受它们；provider-facing `TranscriptContext` 只有 messages，带 branded 类型，只有 normalizeContext 产生。`utils/transcript.ts:25–34` 将 shorthand 插入 leading system。`packages/ai/test/system-message-replay.test.ts:100–107` 覆盖这个公共行为。

因此 AIR-020 的正确退出对象是 adapter/prepare 中并列参与解释的 system/tools 权威及独立旧执行分支，不是所有方便构造一次聊天调用的入口。Rust 可保留 builder 或单一输入结构，规范化后交给只接受 canonical transcript 的下游。对同时带 shorthand 和已有 system 条目的输入，必须明确匹配 pi 的 prepend/replay 顺序或提供无歧义的受限 builder，不能让不同 adapter 各自决定优先级。便利构造无需为了“删除旧字段”无限期维持旧 transport。

**反证检查。** pi 不是保留两条 provider runtime：类型注释与 normalize 注释都强调唯一下游输入。严格的 Rust newtype 可强化此边界；删除 SDK 便利 API 并不能额外证明这种唯一性。**置信度：高。**

## 后续验收应验证的可观察行为

以下是验证设计，均未执行，也不是实施工单。优先复用现有 public Provider/Agent/session fixture 边界。

| 范围 | 必要断言 |
|---|---|
| Transcript | append 与 named section replacement/deletion 在逐时点和 compaction 前后保持语义；工具 remove/re-add/redefine 顺序明确；不要只比较最终集合 |
| Native/collapse | 同一 canonical 输入对已支持变化生成原位 wire，对不支持变化自动生成当前状态 wire；记录模式且 canonical 不变；Strict 若保留另测主动拒绝 |
| 动态可信工具 | 下一 call 能更换 schema/实现；已准备 call 的执行器/定义不被替换；历史声明不能注册实现；新旧 schema 分别校验其对应调用 |
| Active/visible | 当前执行集合外调用拒绝；可信 orchestration 隐藏声明仍按其执行上下文工作；从历史/模型制造工具不扩大权限 |
| 缺结果 | trailing/partial batch 只补缺失项，ID 关联准确；结果标为 unavailable 而非 executed/NotExecuted；不产生工具副作用，不写成 canonical 真结果 |
| 失败历史 | Error/Aborted 不发 wire；Length 工具零执行，能表达的 proposal/error-result 成对保留；产品恢复删除整 attempt view；第二次真实 adapter 请求合法 |
| 迁移/持久化 | 旧事实与 unknown 保留；当前状态只附加在 continuation 边界；已接受输入在模型失败后保留；事务失败遵守 opi 已有承诺且不伪造 dispatch |
| 入口 | shorthand normalize 后只有 transcript 下传；直接 SDK、Agent、compaction 与 mock 不留并行旧解释路径 |

## 核验记录与剩余未知

本专题实际执行：本地 `rg`/编号源码读取；`gh pr view 9548 --repo earendil-works/pi --json title,body,state,mergedAt,mergeCommit,url`；`gh issue view 3555 --repo earendil-works/pi --json title,body,state,closedAt,url`；`gh api repos/earendil-works/pi/commits/b271b0a524b29e13c0c9e748aea0d34e1597f2db` 读取元数据及 patch。访问日期均为 2026-10-03；结果支持上文所列时间、状态和变更。未调用模型、未安装依赖、未运行生成脚本或 runtime tests。测试影响：**none**。

pi 基线身份沿用总研究已核验的 v1.0.0 ai 包比对；跨包证据来自该本地快照，并不据此另行宣称所有跨包文件均与上游逐 blob 核验。未核验所有供应商远端可接受性、外部 embedder 的使用规模、pi 对所有 malformed 历史的长期承诺，也未发现永久保留/删除 collapse 或强制不可变 registry 的后续计划。上述未核验事项不能被写成上游保证。

需要人类重选的重点可收束为三组：采用 pi 默认 native-or-collapse 与更新语义；允许可信宿主同会话动态执行 loadout 而非强制 child；按 pi 区分缺失结果、Length 失败与产品恢复。工具权限仍由可信执行 owner 管理，具体 Rust 内部 enum、diff 算法和所有权载体无需逐项访谈。

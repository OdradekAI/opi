# 图片生成与分类操作的归属选项

> **后续重审：** 用户已将本轮基准明确为“pi 的 Rust 实现”，opi-spec 仅作可能过时的参考。本页源码事实保留；涉及原 Q1–Q21 的目标建议和确认状态须先读 [决策重审](decision-reassessment.md)。旧规范不能继续单独否决 pi 已落地能力。

> 非规范性研究与候选决策输入，2026-10-03。用户已确认将新的 SDK 能力纳入候选目标；**本文初稿曾比较四种归属；Q10后已选择opi-ai与有界规范修订，见D-13**。本文不修改规范、不建立 ADR、不授予实施准入。
>
> 新建中文研究材料，没有既有英文 counterpart；候选 spec 双语同步仍由 spec 文件负责。本文实际读取 `research`、`codebase-design` 技能及下列规范、术语与源码，只做只读检查。

## 当前归属结论

Q10明确选择opi-ai，D-18进一步澄清opi是pi的Rust实现。多操作目录、鉴权、prepared binding和真实adapter组成同一SDK机制；当前目标不再以旧规范的多个core消费者门槛否决已落地pi语义。下方PL选项和条款表保留首轮判断历史，其中“需先证明旧core准入才能纳入目标”已撤回。未来正式文档要协调Mission/GOAL/PRIN/§8与该定位，不为每种pi能力逐个制造小例外；本次不修改它们。SDK-only图片/分类不自动意味着默认terminal工具，也不证明尚未完成的实现或准入。

## 首轮归属推理（历史记录）

现行规范**没有把 opi-ai 永久限定为聊天，也没有规定所有 provider-neutral 操作都必须进 Independent Companion**。`CONTEXT.md` 将 Agent Core 定义为跨产品成立的 model 与 agent runtime semantics；§7.1 的运行时路由与 wire 隔离同样不含 chat-only 限制。因此“image/classifier 一定不属于 core”是过度结论。

反过来，provider-neutral、SDK 可复用、pi 已有、甚至存在两个 adapter，都**不足以单独通过 Agent Core 准入**。GOAL-002、PRIN-001 与 §8.1 要求同时证明语义必要性、删除测试、深度、真实 seam、耦合生命周期、机械验证和最小权能。当前 opi 没有已核验的图片生成或专用分类 operation/adapter/消费者；不能把此次目标确认、pi 的 codemode、测试 mock 或将来设想当成这些门槛已经通过的证据。

可保持现规范的合理路线至少有两条：在 Placement Review 中逐项补足真实证据后有条件选择 opi-ai；或将仅已证明的公共 model-call 不变量留在 core，把独立操作实现放在 core 外的适当责任层。后者也不自动要求新仓库、进程协议或新泛化 trait。若人类希望现在就把 opi-ai 的使命明确扩大为完整的多操作模型 SDK，并容许在未满足现有必要性/多 core 调用方门槛时准入，应明确修订相应规范，而非把它藏在候选 spec 的 MUST 中。

## 适用条款及其原意

| 证据/条款 | 条款原意 | 对本选择的约束；不能推导的结论 |
|---|---|---|
| [CONTEXT Agent Core](../../CONTEXT.md#product-architecture) | 产品中立的 model 和 agent runtime semantics，跨 harness/UI/product 有效 | 定义比 chat 宽，但中立性是必要条件，不是充分准入条件；不能把任意模型服务全部视作 core |
| [§2.1、GOAL-002](../../opi-spec.md#21-mission) | 小而深的 core；optional workflows 和 independently reusable capabilities 在通过全部§8门槛之前留在 core 外 | “SDK能力已纳入目标”不等于“全部准入门槛已获豁免” |
| §2.1 GOAL-001 | 跨两个以上 crate/adapter/extension 的用户可见功能需要端到端 Reference Product owner 和 assembled-product acceptance | 文字没有明确排除纯 SDK 功能。若目标坚持无产品接线的 SDK-only 交付，需明确定义本条适用方式或修订，不能自动用库单测代替。此条也不要求为了数量虚构终端功能 |
| §2.1 GOAL-003、§2.3 | Rust 设计改善正确性/状态/可测性；不匹配全部 pi 包/类型/目录 | 不复制 pi `AnyModel`、ImagesModels 迁移桥或 TS package/subpath 是合法选择 |
| [§3.1 PRIN-001](../../opi-spec.md#31-minimal-and-deep) | 删除新 core module 后，其复杂度须回到多个 core 调用方 | 用户期望两个未来 SDK方法、两个同类 mock、pi产品的两个调用点，都不能替代 opi 的真实 deletion case |
| §3.1 PRIN-002 | 新公开 seam 是 Agent state machine 内在语义，或至少两个真实 adapter/consumer 且有共享 conformance | 两个 adapter 可以证明真实 variation，不必同时有两个终端 UI；但只通过本条不自动满足 PRIN-001/§8.1 |
| §3.1 PRIN-003/004/005 | 机制低于策略、权威选择 fail closed、结论依证据 | 分类分数不得直接授予权限；图片落盘、展示、发布、任务选择等策略不应随 operation 混入 core |
| [§3.2](../../opi-spec.md#32-alignment-with-pi) | pi 是参考；新证据触发 Placement Review，human shaping 决定规范是否改变 | pi已有 provider/adapters/调用方只能说明设计可行和代价，不能算 opi真实variation或已批准方向 |
| [§4.1/PLACE-001](../../opi-spec.md#41-placement-vocabulary) | 当前 opi-ai/opi-agent 属 core；依赖向内；现表非永久 crate 拓扑 | 改模块或crate名字不会改变责任归属；core不能依赖外层workflow来证明自身完整 |
| §4.1 PLACE-002 | Agent-neutral 问题应从 Opi product crates 之外、以 Agent-neutral contract 开始 | 不应在 coding harness 内先造通用多操作SDK再反向要求core引用。条款没有明说“所有中立库都排除在Agent Core之外”，须与 core定义及§8一起读 |
| §4.1 PLACE-004/005 | experimental/flag 不能将能力偷渡core；已有代码可待证明实质危害再移动 | 新 image/classifier 不是既有代码，不能用PLACE-005豁免；feature只可用于依赖隔离证据，不能替代准入 |
| [§4.3](../../opi-spec.md#43-ecosystem-and-independence) | Opi专用workflow在ecosystem；完整Agent-neutral价值/独立artifacts/errors契约支持companion | 两个消费者且一个在Opi外，是**仓库抽离**的更高要求；不是“先建立任何companion必须已有外部消费者”的同义句。companion也不等于必须另建仓库 |
| [§7.1 INV-001/002](../../opi-spec.md#71-provider-runtime) | 模型选择必须真实绑定provider/wire，供应商细节藏在中立接口之后 | 若operation进opi-ai，这些不变量必须延伸到operation身份/错误路由；条款本身不授权增加operation种类，也不规定所有操作必须是聊天流 |
| §7.2 INV-003/004 | generic Agent turn、停止/继续、原子next request update | 图片一次性生成和分类不自动是Agent turn；不应强塞入AssistantMessage或工具续行来满足“state machine”措辞 |
| §7.3 INV-005/006 | 工具执行授权/校验先于副作用，取消/关闭/溢出/部分失败可观测 | 若operation被工具调用，必须保留工具authority层。纯SDK不因此必须增加完整tool router |
| §7.4 INV-007/008 | 进入session/evidence的行为必须遵守唯一durable truth、binding与证据契约 | SDK返回图片/分类结果不等于自动持久化session；只有选择产品接线时才扩展相应schema，不把所有结果默认为chat transcript |
| [§8.1/GATE-001/002](../../opi-spec.md#81-agent-core-admission) | 每门都过；任一失败留在core外；flag/unstable/pi parity不豁免 | 两个adapter与shared tests仅是其中一门，不是其余六门的代理 |
| [§8.2](../../opi-spec.md#82-product-and-independence-admission) | 根据问题所有权选择产品/生态/companion；新接口等触发review；跨层移动需ADR | 新操作的最终归属需要placement case。本文只是候选推理，未替人类作ADR决定 |

`Opi product crates` 在 PLACE-002 中没有另列正式词条。最稳妥的阅读是：不得用这一个短语推翻同时存在的“product-neutral model runtime”定义与全部准入门槛；也不得把它解释为对 Agent Core 的无限许可。若人类想用该短语决定所有通用SDK的唯一归属，需要澄清规范措辞，而不是由实施Agent选择一种有利解释。

## 当前 opi 证据与尚不存在的证明

[opi-ai lib.rs](../../../crates/opi-ai/src/lib.rs) 与 [Cargo.toml](../../../crates/opi-ai/Cargo.toml) 描述多 provider LLM streaming；[Provider](../../../crates/opi-ai/src/provider.rs) 只有已准备鉴权的 `stream_prepared(Request, ResolvedAuth)`。Request 带 messages/system/tools/thinking/token limit 等当前聊天调用材料。[ModelInfo/WireApi](../../../crates/opi-ai/src/model_info.rs) 的八种 wire 均为当前聊天协议，`supports_images` 表示图像输入能力，没有 operation kind、多操作目录键或专用 classifier结果契约。

[coding-agent README Boundaries](../../../crates/opi-coding-agent/README.md#boundaries) 将图片生成列为 deferred、图片支持限于输入侧。这是当前产品范围陈述，不是永不增加图片生成的规范禁止。源码检索未找到 dedicated image generation/classifier operation；普通错误“classification”以及model可写JSON，不构成专用分类能力。

目前真实 opi 调用方包括 Agent turn 与 coding-agent compaction，它们消费聊天 stream/prepared-call。它们可以证明现有 frozen auth/route、取消、错误、usage等机制有真实消费者，但**尚不能证明非聊天操作自身的必要性、独立状态机或多调用方**。使用相同 HTTP client、auth types、registry，并不等于独立发版必然更脆弱。

这是有限范围搜索结论，不是外部世界不存在 Rust SDK 用户的断言。若后续用户提供外部 embedder 的真实接口/需求，可加入 placement evidence。pi-ai 的 OpenRouter image、System One/llama.cpp classifier 和 codemode，仅作为候选形状及已知陷阱参考，不记入 opi conformance inventory。

`multiple core callers` 不必解释成两个终端界面或两个业务产品；两个真实 core adapter 若确实调用同一机制、删除后必须分别重建它，也可能支持删除测试。需要的是具体调用与复杂度归属，不是把“两个adapter存在”直接计为所有门槛通过。本研究不要求先虚构第二种产品来准入SDK。

## 全部门槛逐项评估

| 门槛 | 图片生成 | 专用分类 | 仍需的决定/证据 |
|---|---|---|---|
| Semantic necessity | 删除当前不存在的operation不破坏现有chat/model/Agent invariant；不能据此证明其永不可能成为model runtime语义 | 同样；预测choice/score/bool本身不是Agent授权或turn推进不变量 | 指明“移除operation会破坏哪项已确认 durable model invariant”，并说明为何不能在core外保持。仅把“必须支持生成/分类”新写进spec再自证不可删除，是循环论证 |
| PRIN-001 deletion / Depth and locality | 图片输入/输出媒介、生成请求、失败/取消与可能异步完成有实质复杂度，但当前无多个core调用方证据 | 类型化问题、概率/分数范围、无效结果与部分失败有复杂度，但当前无多个core调用方证据 | 列出实际调用方要重复承担的复杂度；不能以adapter数量、行数或“复用auth”代替 |
| Product neutrality | 纯生成请求与结果可能中立；默认保存目录、命名、GUI预览、发布或艺术工作流是产品策略 | 纯结构化推断可能中立；把分类结果用于自动切模型/权限/benchmark打分是不同策略 | 明确纯SDK返回契约与产品行为分开；不得把概率当authority |
| Real seam | 当前无opi真实image adapter；pi只有若干API事实不等于opi adapter已形成 | 当前无opi真实classifier adapter；“能用chat prompt模拟”不能自动算与专用API相同语义 | 选择具体、真实目标adapter/consumer和共享conformance；候选spec可规定验收需要它们，但不能记作当前已通过 |
| Intrinsic-state-machine 路径 | 普通 future 的pending/completed和取消并不自动“intrinsic to Agent state machine” | 同样；one-shot分类不因返回enum而成为Agent state machine核心 | PRIN-002的例外是内在Agent语义，不是所有异步IO都可自带状态机获得豁免；§8仍单独要求语义必要性 |
| Coupled lifecycle | 输入输出媒体与provider changes可能独立于chat演进，也可能共享真实鉴权/route规则；当前证据不足 | 类型化问答API可能独立演进；共享model ID或tokens不证明必须同版本 | 比较分离后的边界/发布成本，写出哪种独立变化会破坏已知消费者。不能仅以“同供应商”认定不可分 |
| Mechanical verification | 可设计离线payload/result/media/error/cancel/limit测试，但尚未有这项operation实现/回归证据 | 可设计schema与概率/score/choice validity、partial/malformed/error/cancel测试，尚未具备 | 与实际adapter共同实现后验证；本轮候选文档不冒充通过 |
| Minimal authority | 新network endpoint、URL下载、图片落盘、codec/SDK/native依赖可能扩大面；仅显式返回bytes且复用现有HTTP不一定扩大默认IO，但须核算 | 可仅显式调用现有HTTP，也可能引入本地模型进程/自动路由/新SDK | 单列默认与显式调用、自动跟随URL/写盘、依赖变更。feature可以减少默认依赖成本，但GATE-002禁止用它绕过任一门槛 |

因此当前结论是**core准入证据不完整**，不是“已证明必须移出core”或“用户选择两操作就是修改了规范”。候选目标可以已确认，placement仍保持未决；不能用目标确认默许一个泛化 ModelOperation trait 或新的全功能 registry。

## 保持现规范的方案

### PL-A：有条件归入 opi-ai，逐项证明，不提前豁免

保持现有 Agent Core 定义，将图片/分类作为两个独立的 candidate capability 分别做 placement case。候选spec先固定SDK可观测目标（输入/结果类型、失败/取消、鉴权所有权、错误路由），core归属标记未决；只有全部§8门槛有具体证据、GOAL-001的验收owner明确后，才允许正式spec以opi-ai为责任模块。

此路线不要求现在已有实现才能做设计；候选spec可以固定真实选定的provider协议、消费者需要和共享conformance验收设计。尚未运行的机械验证不能标成已通过，后续正式准入与交付依仓库选定workflow处理，本研究不自行裁定该流程的阶段门槛。不得用“随后总会有人用”替代真实需要。若审查后只有通用SDK使用而没有必要的core耦合，应转PL-B；不能宣布任何新model操作天生通过Semantic necessity。

主要代价是必须完成placement论证，可能发现目前目标无需core集成；收益是无需修改规范、保留真正有价值的核心语义进入空间。风险是人为制造Agent调用方以满足门槛，应明确禁止。

### PL-B：公共不变量留在 core，操作SDK留在 core 外

保留已证明的auth/route/secret handling/失败等中立契约；image/classifier的操作专用输入、结果、adapter与catalog由core外拥有。外层是否复用现有opi-ai公开契约，取决于它们是否足够中立、稳定及不会造成反向依赖；不要为了复用一处实现扩张core公共接口。

具体归属取决于真实目标：独立于Agent也有完整价值的生成/分类SDK通常符合 Independent Companion 的问题所有权；只提供Opi特定工具、展示或路由workflow的包装属于Extension Ecosystem；Reference Product只拥有必要终端交互/配置和接线，而不因实现方便承接整个通用SDK。

这不是自动建一个crate或仓库的要求。§4.1区分责任与拓扑，§4.3的仓库抽离门槛不能挪用成“现在必须找外部用户才能写候选SDK”，也不能被跳过提前独立发布。模块、crate、独立版本、进程协议是否需要，须在具体交付规格确定，不应一次预建四套基础设施。

主要代价是SDK接线可能需要明确传递auth/client/route材料，不能只靠一个默认全量collection；收益是现有chat/Agent不被不相关的媒体/分类生命周期绑住。需警惕复制secret解析和多个目录authority；不能只是把实现搬出去再做一层无意义转发。

### PL-C：仅在真实 Opi workflow 存在时，由产品/生态包装

若已确认目标实际是“工具生成图片给会话”或“插件用分类决定下一模型”，可在产品/生态层构建workflow，调用一个具体外层操作实现，使用现有工具/授权机制。该方案不自动满足当前更宽的通用SDK目标；若人类仍要求独立SDK，它只解决其中的产品接线，不是偷换范围的替代品。

不能为证明SDK有消费者而凭空增加默认图片工具、分类路由策略、模型chooser或codemode。只有真实workflow独立获准，才纳入GOAL-001 assembled acceptance、INV-005工具authority以及session/evidence要求。

## 若选择修改规范，应修改什么

这不是建议立即修改；它列出“明确把opi-ai确立为多操作模型SDK且当前不满足全部既有核心门槛”时需要人类选择的具体范围。只给§7加两种操作名、保持冲突的§3/§8不动，不足以解决authority问题。

| 可能的修改 | 为什么需要明确 | 外溢影响与应保留的限制 |
|---|---|---|
| §2.1 Mission/GOAL-001 与 GOAL-002的适用说明 | 明确是否把通用model SDK本身作为独立一级产品承诺；SDK-only验收由谁负责，是否仍需Reference Product端到端owner | 若允许SDK-only应定义SDK公开入口的conformance，而不是强迫虚构CLI功能；GOAL-002不能变为“所有AI相关能力自动core” |
| §3.1 PRIN-001 与 §8.1 Semantic necessity / Depth / Coupled lifecycle | 当前要求多个core调用方和durable invariant。若新的准入理由是“通用模型operation的一致SDK语义”，应明确是否为有界模型操作建立另一条准入路径 | 要枚举操作范围、真实使用/adapter与state所有权、删除/版本耦合理由；否则embedding、音频、搜索、排序、视频等都会引用先例无限扩张 |
| §3.1 PRIN-002 / §8.1 Real seam（仅在拟放宽时） | 若没有两个真实adapter/consumer，也没有内在Agent语义，却仍要公开统一接口，现条款不能被暗中视为已通过 | 推荐保留这条，不需要为图片/分类天然放宽；两个真实adapter仍只能证明seam，不替代其他门槛 |
| §4.1 ownership/PLACE-002 与 CONTEXT（仅实际职责改变时） | 若把opi-ai的一部分从Agent Core重新归为中立model SDK层，或澄清product crates术语，需要同步责任和依赖方向 | 不得建第二套术语；core不得反向依赖workflow。将现有crate重新归层本身是跨层移动，应有已确认ADR，而不是改表即可 |
| §7.1 INV-001/002 | 若model身份可按chat/image/classifier重复，需明确operation身份、typed dispatch、unsupported operation和错误路由；stream wording是否只作用于支持流的operation | 不要求所有结果包装成AssistantMessage，不要求全部operation支持stream，不让分类与图片改变已有chat request semantics |
| §8.1 GATE-001/002 与 Minimal authority | 若实际新增dependency/default IO，却计划作为core强制默认承诺，需明确新增准入路径如何仍受最小权能约束 | 不建议取消fail-closed与minimal authority。显式调用和可选依赖是证据，不是通过feature免审的通行证 |

若人类选择PL-A且最终全部门槛可证，则无须为了操作名称新增一个特例。若选择PL-B，也不必为了允许companion而改写Agent Core。若选择“opi-ai从此无条件拥有所有provider-neutral模型操作”，那是实质使命/准入范围修订，影响远超本次两项SDK能力，不能作为普通实施细节接受。

## 主决策所需的最小信息与验收约束

初轮访谈区分的SDK目标、场景/adapter及归属均已回答；这些材料不是当前未决事项。以D-13/18/23–25及44 AIR为目标，不沿用旧条款否决。不要把它们合并成“是否跟pi对齐”一个开关。

为做出可实施归属，至少需要：每种operation的首个真实使用场景与adapter；是否要求Agent/产品接线或仅独立SDK；多操作是否共享provider identity/auth/route与同一call冻结；结果与计费单位、部分失败/取消/资源生命周期；artifact由SDK返回还是产品保存。它们可以决定是否存在真正的共同机制，不要求现在选择全部未来功能。

任何方案都应以离线公共契约fixture检验输入/结果边界、认证不重复、unsupported operation fail closed、取消与EOF/终态、过量输出、无效响应和secret不泄露。图片输入是既有chat能力，其修正不能因图片生成placement未决而被拖住；分类workflow不能绕过INV-005取得权限。实际本轮仅阅读，未运行新测试；测试影响 **none**。

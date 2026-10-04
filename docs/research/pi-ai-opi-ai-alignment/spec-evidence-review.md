# 候选 spec：证据与 authority 独立评审

> 非规范性评审，2026-10-03。对象是英文候选 spec 的证据、范围确认与规范权威；本轮只新增本文件，不修改其他材料、运行时代码或测试。
> 首轮结论适用于下列首轮输入版本；末尾记录修订复核。Q1–Q21 及修订 Q17 的目标选择现已确认。**本评审未检查中文，不是中英一致性签署或实施准入**；正式规范修订、ADR 与实施准入仍是独立前置条件。

## 输入与方法

首轮受审 [spec-draft.md](spec-draft.md) 原始字节 SHA-256：`4592e7b0cbb236b9af2823ae2719163a848c7274ee7d8ac324af61ae16657f88`。已核算 32 个 AIR 条目，ID 均唯一。修订复核输入摘要见末尾，不把首轮文字中的 pending 当作当前目标状态。

全文读取 spec、decisions、acceptance、provider-contract-addendum；追踪 operations、replay、transcript、pi 契约、演进、综合审计及现行 opi-spec/CONTEXT 的有关条款。按 writing-for-agents 的要求检查分支指引与完成条件，不把引用数量当证据充分性。

评审者未参与英文 spec 编写，但曾编写 design-evolution 与 placement-options；对这两份来源仍属于作者证据复核，不能冒称独立作者调查。本轮没有重新查询移动的上游 main，没有执行 pi/opi runtime 测试、probe 或模型请求。

## 结论

未发现把 pi compat、虚拟模型实验、beta wire、TypeScript 模块形状或 tag 后未发布变更伪装为 opi 必须照搬的目标。也未发现以两个 OpenRouter 底层模型冒称两 image adapters，或把 pi 的消费者算作 opi 已满足的准入证据。

文稿确实审计并处置已有设计：AIR-001/020–023 和 retirement 表包含旧选择路径、Request 双 authority、旧配置/身份壳、无行为 compat、AWS discovery 的替代及退出；AIR-003–007/024 保留或修正现有生命周期、typed auth/errors、工具权限与 usage。不是只新增功能。

首轮发现三个文档问题，现均已修复，见末尾复核证据。以下保留首轮问题及当时状态作为修订依据：当时 **Q16/Q17/Q18/Q19 和工程 limits 明确 pending 本身不是缺陷**；DRAFT 是正确状态。现有目标选择已全部确认，DRAFT 保留的原因是材料非规范性及正式准入尚未完成。

## 发现

| ID / 严重度 | 受影响要求及来源 | 证据、影响与最小修补 |
|---|---|---|
| SE-001 / P2，已修复 | AIR-008/009/011、AIR-015–023、AIR-025–032 的 V-16–26；AIR-012/013 的 D-15、AIR-031 的 D-16 | 首轮 acceptance 只定义 V-01–15，decisions 只定义 D-01–14。部分专题已有 TM-V/OV 等实际验收设计，但不能让执行 Agent 猜测尚不存在的 V/D 编号等于哪条。补上准确桥接条目或直接引用已有专题 ID；pending 决策也可登记为 pending，无需先取得答案。更新后机械检查每个 AIR 的证据、决策、验收引用均可解析。 |
| SE-002 / P2，已修复 | Scope 的 `Excluded until D-11's triggers are met`；AIR-007/024 的 D-09/10/12 保留部分；spec 末尾 Q18 | 首轮 decisions D-11 明示其余延后选择尚未确认，D-09/10/12 也保留整合确认；spec 主体则把排除范围写成既定边界，只在结尾泛称 Q18 pending。继续保留当前 auth/usage/validation 的源码事实不需推翻，但候选目标及最终验收范围当时尚待统一确认。最小修补是在该 scope 段及相关要求标清受 Q18 约束的部分，或增一张精确的 pending→段落/AIR 对照；不能声称“所有 affected requirements 已 marked”却要求读者自行推断。 |
| SE-003 / P3，已修复 | AIR-014、AIR-008/009/015–020、AIR-025–032 的研究导航；evidence M-12、operations 首尾、transcript §10 | 首轮 decisions 已确认 Q7、Q10/13/14、Q15；M-12 仍写工具图片“是否修成可见或拒绝待 D-06”，operations 仍说首批 provider/归属/生命周期未确认，transcript §10 仍说 Q15 的回答将决定模型。这些是旧研究时点文字，不能作为当前状态。给相关段落明确的历史状态注并指向 owning decision，保留技术推理与未验证缺口；无需重问已回答的问题。 |

SE-001 是引用未闭合，不是声称没有任何验收设计；SE-002 是目标状态表达问题，不是要求删除保守行为。SE-003 不否定源码事实，也不要求将所有研究笔记复制成一份不断同步的决策账本。

## 首轮要求与 authority 的重点核查（历史时点）

本节的 pending/待修判断只描述首轮；当前状态由后文修订复核及 decisions 统一说明。

| 要求范围 | 判断 | 证据与限制 |
|---|---|---|
| AIR-001/002 有效 binding 与目录 | 支持 | O-002 静态数据流及纯 mock 反例、D-03/Q4 支持唯一执行选择；保留 Rust 独占刷新，未复制 TS generation。不是声称完整产品切换已运行。 |
| AIR-003–006 流与生命周期 | 支持 | A-02/03/04、D-04/Q9 支持修正；对本地 drop 的承诺不扩大为远端取消或退款。64 是当前队列容量保留，不是 pi 数值对齐。 |
| AIR-007/024 保留机制 | 证据支持，目标状态需 SE-002 | Auth 一次解析、typed 错误、输出后不自动 retry、权限校验、unknown usage 是 opi 当前有价值的差异。不得称 pi 全链同等禁止整轮 retry。 |
| AIR-008/009/011/020 transcript | 范围与主要取舍支持 | Q5/Q12/Q15 选择完整 snapshots、原子下一 call、strict 默认与显式有损投影；不照搬 pi section patch DSL。未将模型声明升级为执行 authority。 |
| AIR-010 provenance | 支持其问题及中立边界 | O-010/P12 与 replay 证据说明 requested/report model、exact wire、opaque 来源需分离；不能由旧 ApiKind 猜 exact wire。实际 serializer 仍须 fixture 证明。 |
| AIR-012/013 recovery | 正确保留 pending | Q16/17 未决已就地标注。任何普通 failure context 的来源、角色、字段与脱敏仍需最终固定；不以 `{}`/null 伪造历史。不得因本轮审查替用户选择。 |
| AIR-015/016/018/019 session | 确认方向与既有权威相容，正式修订仍需完成 | Q11/Q15 与 INV-007/008 支持保真 child、unknown、单一 prefix+binding；不能把摘要补成过去事实。pre-call ack 新顺序由 AIR-017/Q19 单列。 |
| AIR-017 durable commit | 正确保留 pending | 与当前 failure/cancel/不可变 binding 边界可能交互，spec 已标 Q19；不将候选流程图当先行实施授权。 |
| AIR-021/022 retirements | 支持 | Q6 明确清单，保留 ApiKind/有效 compat；旧 key 要报迁移错误，不能以忽略未知字段冒充退出。外部 embedder 未知仍是迁移限制。 |
| AIR-023 AWS placement | 支持，且留有正式 ADR 前置 | Q8 已选现有 product resolver；保留来源顺序、取消/限额、安全和 SigV4。没有因“库中有 OS I/O”而直接删除现有必要能力。 |
| AIR-025–032 新 operations | 候选目标支持，现行规范 gate 尚未通过 | Q10/13/14、D-13 是目标依据；OP/EVO 是参考证据。精确工程 limits 仍需确认，不能把研究者选择称为 provider 限制或测量最优值。 |

### 有界规范修订是否诚实

现行 GOAL-001 要求跨两 crate/adapter/extension 的用户功能有 Reference Product owner 与 assembled-product acceptance；GOAL-002、PRIN-001/002 及 §8.1 逐项要求 semantic necessity、deletion test、real seam、coupled lifecycle、mechanical verification 与 minimal authority。

D-13 没有声称“provider-neutral 就自动进入 core”，而是明确请求对两种 SDK 操作提出有界修订，并保留现行 gates 的正式流程。单 OpenRouter image adapter 的缺口、TypeSafe/OpenRouter 共用同一协议而非两种算法、尚无 opi 生产消费者，都未被 pi parity 掩盖。AIR-032 指定 public SDK→auth→真实 adapter 伪传输的验收，不能用 MockProvider stub 代替。

这套表述支持**提出修订**，不证明旧条款已满足，也不是实际修改完成。实施前正式修订必须明确首批单 adapter 例外的范围及剩余 gates；不能只在 §7 增加 operation 名称而省略 GOAL/PRIN/§8 的变化。spec 的 authority 前置及 D-13 已记录这个要求，本轮不再发起相同授权问题。

### pi 分类及官方后续

| 信号 | 当前候选处理 | 判断 |
|---|---|---|
| 显式 Models/provider 实例、auth/catalog 责任 | 借鉴所有权及复用责任，保留 Rust seam | 合理的行为对齐，未要求 TS 外形。 |
| compat/global 与旧 ImagesModels | 新 operation 不引入平行集合；不复制迁移入口 | 正确区分迁移负担和稳定目标；pi compat 仍有消费者，不被误称已删。 |
| chronological transcript | 选择顺序/可重放语义，使用完整 snapshots | 人类决定和源码证据共同支持，不是假借“pi 最新”自行扩大。 |
| Anthropic beta inline tools、tag 后 Bedrock修复 | 要求按具体 wire 能力 fixture，未承诺 beta 机制必交付 | 未发布 main 不冒充 1.0.0 或确定未来路线图。 |
| classifier 驱动 virtual model routing | 延后自动策略，分类只返回 typed answers | 没把实验或概率输出升级为模型选择/执行权限。 |
| 无界 EventStream、弱 EOF、missing→zero、宽泛 diagnostics | 明确保留 Rust bounded/typed/unknown 边界 | 没机械复制已知弱契约。 |
| 官方当前专用图片 API、较宽 System One 输入 | 保持已选 chat-wire/窄 SDK 输入，并披露差异 | 未把动态网页当有版本承诺；实施前重核如有实质不兼容须回到范围决定。 |

官方历史的核验仍使用 EVO 的固定 tag/main 窗口；本轮没有新的“最新”结论。未发现候选 spec 为尚未接受的 RFC、明确删除日期或性能/缓存收益作承诺。

## 前轮 R-003/R-004 复核

- **R-004 已修复。** P08 新增公开 assistant-call helper、compaction 真实消费及产品整回合重试；M-06 与 D-09 将 opi 的已交付事件禁自动 retry 明确为更保守差异。AIR-007 没再把该选择称为 pi 全链等价。本轮未重跑重试测试。
- **R-003 的原主要建议已修复，状态残留见 SE-003。** P19 已注明早期 snapshot 推荐被 Q5/Q15 覆盖；EVO-009/末段、M-03/M-05 已引用当前决定。M-12 与新增专题仍须小范围状态注；不能据先前部分修复声明全目录已同步。
- R-001/R-002 的 probe 修正未重新开启；本轮未发现新增相反证据。

## 首轮提出的后续复核完成条件

1. SE-001 引用闭合 → verify：每个 AIR 的 owner、来源、decision、验收 ID 都能解析，pending decision 也有明确影响范围。
2. SE-002/003 状态一致 → verify：研究时点建议、当前确认、预期未决分别可辨，不用末尾一句 pending 掩盖主体的相反表述。
3. 人类补答后的目标修订 → verify：只变更明确确认的条目，其余保持 DRAFT；不把本轮评审当确认。
4. 中文同步及最终改稿 → verify：32 条 AIR（或最终明确修订的集合）、失败规则、normative 前置及数值 annex 语义对应；随后独立复核修改部分。

本轮执行的是只读文本/引用/哈希检查与本评审文件写入，运行时测试影响 **none**。最终仓库文档检查由主 Agent 在合并各文档后执行；本轮不宣称候选 spec 已达到实施条件。

## 修订复核：SE-001–003 已关闭

2026-10-03 只读复核当前英文 spec、decisions、acceptance、M-12、operations、placement 与 transcript 状态说明。受审英文 spec 原始字节 SHA-256 为 `7a21c3d9f474b35c86f21cae3ae08e0c6230a18986321865509298c35f512ada`。

- **SE-001 关闭。** decisions 已定义 D-01–17，acceptance 已定义 V-01–26。机械展开 AIR 行内的单个 ID、范围和 `/` 简写后，32 条 AIR 的 D/V 缺失引用为 **0**；验收文档反向出现 AIR-001–032 全集。已读新增 V-16–26 的 owner、触发、失败及机械判据，确认它们不是空编号。实际执行验收仍属于未来实施。
- **SE-002 关闭。** decisions 记录 Q18 已确认保留／延后范围与离线边界；spec scope 就地引用 Q18，D-09/10/11/12 目标状态同步。Q16、修订 Q17、Q19、Q20、Q21 的确认也分别进入 D-15/16/17 及相关 AIR，原 pending 要求已更新。不存在因本轮关闭而替人类默认批准的选择。
- **SE-003 关闭。** M-12 明示 Q7 的发送或拒绝；operations 头部与访谈后说明将旧选项标为历史并指 D-13/16；placement 头部明确 Q10 后的 opi-ai 有界修订选择；transcript 明示 Q15/Q19/Q20 已确认、§10 只保留取舍依据。专题内保留的早期候选不能再解释为新未决事项。
- **R-003/R-004 关闭。** 原旧建议状态已由上述说明解决；P08/M-06/D-09 仍正确区分 pi 的 HTTP、传输恢复、assistant-call 与产品回合重试。没有重新出现“全 pi 输出后绝不重试”的错误归纳。

另向主 Agent 提醒 acceptance V-07 一处旧措辞“仍待调查和选择”，其下方 V-07 补充及 D-15 已提供最终目标；应将旧句改为“既有覆盖不足，目标已固定，未来执行验收待完成”。这是当前证据与未来执行的措辞同步，不是新的产品决策或架构矛盾。

本轮未发现新的实质证据／authority 矛盾。Q1–Q21 及修订 Q17 均已有确认记录；剩余正式规范修订、AWS ADR、实施准入和运行时验收不能冒充已完成。本轮**没有审查中文内容或双语一致性**，也没有运行新的 runtime 测试或 probe；测试影响 **none**。

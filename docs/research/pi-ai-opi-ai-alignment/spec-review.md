# 候选 spec 独立设计评审

状态：**非规范性独立评审，2026-10-03；不是实施准入或已通过验证声明**。

评审对象为英文 `spec-draft.md`，辅以 decisions、acceptance matrix、transcript migration、operations research 和必要 opi 源码。评审使用已实际读取的 `writing-for-agents` 技能，检查新 Agent 是否能确定状态、责任和完成条件。本评审只新增此文件，未修改候选正文、运行时或测试；未运行 runtime tests。

首轮英文稿 SHA-256：`1AB832F83C3E31BADD54345C5ADD0D6660C8F60531761165CFCB915F3F7A94DB`。
首轮 acceptance matrix SHA-256：`1E99ADA0D475C1E962610FE819D9E4C4E37D522C4C953B261A1F787EDB3545EA`。
下面保留首轮发现的证据与理由；其当前状态以复核结论为准，不表示后续版本仍存在同样问题。

## 设计复核：SR-01–05 全部关闭

复核英文稿 SHA-256：`7A21C3D9F474B35C86F21CAE3AE08E0C6230A18986321865509298C35F512ADA`。
复核 acceptance matrix SHA-256：`43435BD83A6CDFEF6BCA517611AD75B6AE62E4EEAAD0A1D1E210CB8E2BC5310C`。
主 Agent 已提供 Q16、修订 Q17、Q18–Q21 均明确确认的上下文，本轮以最新 decisions 与对应正文复核，不再次提出已决定的问题。

**本轮未发现新的实质设计阻断项。** 候选文档继续标为 DRAFT/非规范性，是约定的文档地位；正式规范修订、ADR 和实施准入尚未执行，不意味着本次研究目标未完成。关闭评审项表示目标与验收设计已闭合，不表示运行时代码已实现或测试已通过。

| 条目 | 状态 | 关闭依据与复核范围 |
|---|---|---|
| SR-01 | 已关闭 | 流程图和职责段先固定 owned pre-auth catalog/model/projection；目录变化不得重选；auth只能补充计划允许的材料。非秘密 resolved dispatch fact由同一session owner在transport前写入，失败阻止dispatch。D-17明确有效参数一起冻结；V-16/18覆盖commit期间目录变更与dispatch fact失败。 |
| SR-02 | 已关闭 | Q20/D-15明确冻结声明集合之外的模型调用按NotDeclared拒绝并保留可信NotExecuted结果；AIR-009要求注册/schema/User Policy仍为附加必要条件，不能借可见性取得权限。 |
| SR-03 | 已关闭 | 职责段与AIR-009/017覆盖initial和continuing；fresh session已知leading snapshot、legacy显式迁移；sessionless使用相同validation而无writer，新assembly只起新in-memory run；已有durable branch才由产品建child。 |
| SR-04 | 已关闭 | 已补V-16–26、D-15–17和32项AIR覆盖索引；成功/失败及真实selected adapter入口的机械判据落地。只读脚本核查ID集合和coverage index通过，详见下文。 |
| SR-05 | 已关闭 | 职责段定义terminal/drop立即禁止后续event及新transport；允许非可执行cleanup等待调度，不计入active attempts；资源收束需独立task/socket barriers证明，不以lease或机器相关毫秒阈值代替。 |

附录固定容量由Q21确认；其“研究者设计值而非供应商限制/性能最优值”的表述仍准确。两个编辑残留已反馈主 Agent 同步：acceptance V-18仍写“取决Q19”，末段仍写“当前关键范围与行为决策待确认”；它们不构成新架构选择，也不重新打开上述关闭项。

实际执行了一个只读文档结构核对：PowerShell here-string 输入 `python -`，读取 spec/acceptance/decisions，正则提取唯一 AIR requirements、V定义、D标题和双列覆盖索引；断言 AIR=001–032、V=01–26、D=01–17且coverage index精确覆盖全部AIR。命令 exit 0，输出：`PASS: 32 unique AIR requirements, 26 V definitions, 17 D definitions, complete AIR coverage index`。此检查不能证明行为正确或语义双语一致。

本轮没有执行runtime测试，也没有修改spec正文。设计复核时中文版本仍在同步；其后的最终双语复核记录如下。

## 最终双语语义复核：通过

已独立阅读中文候选全文，并在翻译更新落盘后再次读取 AIR-011、AIR-015、AIR-025，核对最终改动。最终英文 SHA-256：`6C9ACB569F044D17D009FA8137DAC5591A4FBDAA1ABAD09E5A473B2B50C278DB`；最终中文 SHA-256：`FEF82DF736106B34448F29CEC339A8EBED66CB5D07F0DB40F1C9D8741E99B93C`。

**未发现会改变实施目标、权限、失败结果或完成条件的双语语义偏差；没有新增实质评审项。** 本结论针对上述版本的候选设计表达，不是运行时实现验收。

| 核对重点 | 结论 |
|---|---|
| Initial / sessionless | 两稿都覆盖首次与继续调用的同一校验，fresh leading snapshot、legacy迁移、无session不引入writer/new child；继续调用仍先作stop/continuation判定。 |
| Pre-auth / 两次追加 | 两稿都先拥有唯一计划再提交context；auth不重选目录；非秘密dispatch fact由同一owner在transport前追加，追加失败不分发，凭据不落盘。已接受状态不伪装为鉴权或dispatch成功。 |
| Terminal / cancel / cleanup | 吸收终态、交付前可观察取消优先、共享call与单attempt drop、可执行attempt和非可执行cleanup的区分一致；都不承诺远端退款或机器相关毫秒收束。 |
| Q17失败投影 / Copilot | 可信closed disposition而非is_error/marker作为依据；Unknown/可能副作用不洗成NotExecuted；普通User投影仅存在request view，保留canonical原始参数并从canonical推导initiator；允许字段、reason codes、batch位置相同。 |
| Q20工具可见性 | 两稿均在冻结visible之外按NotDeclared拒绝，即使registry仍有实现；保留可信NotExecuted结果，schema/User Policy仍为附加必要检查。 |
| Q21固定边界 | 图片数量、单张/总bytes、prompt/text、request/body、分类题目/labels/levels、JSON大小/深度、概率/均值容差及Unknown计量含义一致；均非供应商上限或benchmark声称。 |
| AIR-025目录来源 | 中文已同步显式SDK提供/注册并验证initial operation records，不猜ID、不导入缺失pi生成数据、不新增remote discovery；复用目录/auth且chat lookup不混入其他operation。 |
| Retirement / authority | 同次有界破坏迁移、旧键显式error、旧reader仅读/迁移、新builder只建新表示、保留ApiKind/必要安全行为一致；候选不替代正式规范、ADR与准入流程。 |

两个消歧建议已落盘并复读确认：AIR-011 使用“请求消息序列首部”，不会与HTTP header混淆；AIR-015 明确“按 required 分类的因果条目”。翻译工作还报告了32行AIR的ID、D/V证据、数字、规范关键词及inline code机械一致；本评审的独立结论来自上述实际语义阅读，不将他人检查误记为自己运行。

本轮只修改本评审记录；未修改spec、代码、配置或测试，未执行runtime测试。本分项测试影响：**none**。

## 首轮评审结论（后续修订已闭合）

总体方向有证据支持。稿件保留 Rust bounded queues、trusted registry、冻结 auth、明确未知 usage，并审计和淘汰旧路径；没有将 pi 实验/compat 包袱整体搬入目标。删除表大体覆盖用户要求的八项迁移信息。

Q16/Q17/Q18/Q19 和工程数字正在等待确认是合法 DRAFT 状态。明确 pending 的行不是未经批准的 MUST；本评审不以“仍有访谈”为缺陷，也不要求立即实施正式规范修订。

本次发现四项交付前必须修复的契约/追踪缺口，以及一项可通过简短定义改善的生命周期边界。修复可以主要是文字、验收矩阵和决策记录；不要求增加新框架。

## 首轮必须修复项（全部已关闭，保留证据）

### SR-01 · P1 · AIR-001 / AIR-002 / AIR-011 / AIR-017：先提交后冻结的顺序不足以证明实际调用身份

**证据**：英文稿 `:54–55` 图中先 “Validate projection and atomic session commit”，后 “Freeze effective model + request + auth”。AIR-001 `:74` 要将 final binding 用于 preflight、diagnostics、adapter；AIR-011 `:89` 要在 auth 前验证，AIR-017 `:100` 提交 prefix/projection identity 后才允许 auth。
当前源码 `crates/opi-ai/src/provider_collection.rs:421` 在 prepare 开始选择模型，`:433–463` 还会默认/压缩 output limit；这些动作影响最后的请求。

**成立条件与影响**：实现 Agent 按图把模型选择/最终 request 冻结放到 durable ack 之后，或在 preflight 与正式 prepare 分别读取目录，则无法保证已提交 projection 的 wire/capabilities/limits 等于之后实际 dispatch。即使当前 refresh 使用独占所有权，两个阶段没有同一 owned value 的明文契约，也不能靠当前不能并发修改目录来建立长期正确性。

**反证**：AIR-001/002 已明确单一 effective model 和已有 call 保持 binding，正确意图存在；问题是 call 的稳定边界和与 session commit 的顺序未落到一个可直接执行的状态转换。

**建议修复**：将流程明确拆为：选择并持有唯一 pre-auth executable model snapshot → 以它编译 transcript projection、有效推理/容量限制 → 验证并提交对应身份 → 用该同一值 resolve auth 一次/补齐 auth-derived endpoint/header → 冻结可 dispatch 的 prepared call。后半阶段不得重新选择模型或改变已承诺的语义投影；auth-derived输入验证失败只产生失败，不改写已提交事实。无需规定新的公开类型名或 trait。

**机械判据**：在 preflight/commit 与 auth/dispatch 间切换可用目录版本（或通过测试 seam 尝试），断言实际请求与 committed identity 仍绑定旧完整值；新 call 才使用新值。覆盖 output-limit normalization，不只比较 model ID。

**处置**：必须修复。可与 Q19 的 durable acknowledgement 文本合并；不需要用户另选已确认的单一 binding 原则。置信度：高。

### SR-02 · P1 · AIR-009 / AIR-024：移出 visible declarations 后是否还可执行，尚无唯一答案

**证据**：AIR-009 `:87` 规定 visible declarations 仅选 trusted registry 的 permitted subset，AIR-024 要 final arguments 与当前 trusted schema/authorization；两者未说明 executor 是否必须检查发起该调用的 snapshot 中存在该名字。
现有 `crates/opi-agent/src/agent_loop.rs:2237` 对整个 registry 查名字，`:2282` 用 registration schema，再授权；`authority.rs:129–168` registry 与 definitions 当前一致。拆开 visibility 后，旧实现自然仍能执行未向当前模型声明但注册过的工具。

**成立条件与影响**：S0 声明工具 T，下一 call 的 S1 移除 T；模型依据旧消息仍提出 T，registry 中仍有 T，User Policy 也允许。实现者可能把“移除”解释成只隐藏文档，也可能解释成关闭当前 loadout 执行；两者都满足现有文字，但可观察行为不同。

**反证**：固定 registry 和声明不授权是正确且已确认的边界；此问题不要求动态装卸或权限扩大，也不否定 INV-005。

**建议修复**：在决策和 AIR-009 明确选择。推荐普通模型调用只允许其发起 call 冻结的 visible subset，随后仍必须通过可信 schema 与实时 authorizer；移除工具产生可观察、零执行的拒绝结果。若选择“只隐藏、不限制执行”，明确写出且提供同样的零权限扩张验证。不要把未来工具间调用/隐藏工具路由需求默认纳入本次。

**机械判据**：S0→S1 移除、S1 后模型仍提 T、late result、并行 batch 和下一 call 重新添加分别验证；当前 User Policy 拒绝始终零副作用。

**处置**：必须明确目标；这是一个真实行为选择，应由主 Agent 结合现有 Q15 回答判断是否已授权，否则只问这一窄问题。置信度：高。

### SR-03 · P2 · AIR-009 / AIR-017 / AIR-020：sessionless 与 initial preparation 的适用范围需要闭合

**证据**：AIR-009 owner 为 Agent、trigger 仅 continuing-call preparation，却无条件要求实现/schema/source变化产生 new-binding child。英文稿 `:64` 明确 sessionless SDK 不伪造 session；`crates/opi-agent/src/agent.rs:616–617` 默认 evidence/session 均 None，Agent 不拥有产品 session writer。`hooks.rs:189–197` 存在独立 initial preparation；`agent_loop.rs:1399` 对 initial selected input 的 suffix 有专门验证。

**成立条件与影响**：一个无持久化会话的 embedder 重新 trusted assembly 时，Agent 若照 AIR-009 创建 child，会违反 owner 分层和 sessionless 契约。另一方面，AIR-009 只约束 continuing，导致首次 system/tools、malformed snapshot、初次 preflight失败的原子性与 queue 消费可能被遗漏，正是通常最先调用的路径。

**反证**：AIR-017 trigger 是 pre-call，可能有意包含 initial；AIR-020 提到 initial builder。没有必要新建另一套初始化框架，只需把已有规则适用范围说明完整。

**建议修复**：初始和 continuing call 都走同一 snapshot validation/projection/installation 不变量；保留 stop/continuation 的区别。trusted reassembly 总是产生新的执行装配/run binding；**仅当已有 durable session branch 时**由产品 owner 创建新 binding child；sessionless 创建新的 in-memory run，不强迫 session I/O。已有 session 的首次 call、全新 session 的首个 leading snapshot，以及空/unknown 初始历史分别写出行为。

**机械判据**：新增首次 call 的 invalid snapshot/unsupported wire/cancel/pre-commit failure/ack后失败矩阵；sessionless reassembly不创建文件、不调用session owner；durable版本产生child且源字节不变。

**处置**：必须修复适用范围。置信度：高。

### SR-04 · P1 · AIR-008–020 / AIR-025–032：目标引用的验收与决策节点尚未落地

**证据**：评审时 acceptance matrix 只有 V-01–V-15；spec 引用 V-16–V-26。decisions 只有 D-01–D-14，而 AIR-012/013 引用 D-15，AIR-031 引用 D-16。spec `:68` 声称每行已有 mechanical verification，但新 session/projection/operation 的关键验收目标尚不可导航。此项是整合缺口，不是反对 pending 决策状态。

**影响**：新 Agent 能找到研究建议，却找不到每个已确认义务的唯一 selected inputs/observable outputs/失败分支和 completion proof；只凭主 spec 简短行会自行补齐验收。

**建议修复**：补齐所有引用节点，pending D 可以明确记录候选与未决，不需伪造已批准；V 条目必须覆盖该 AIR 的 owner、trigger、成功与失败，包括每种操作真实 selected adapter 的 public-entry fake transport 接线。添加 AIR→V 显式映射，而非只依赖行内多个范围引用。同步英文/中文 spec 的 ID 集与状态。

**机械判据**：脚本或人工枚举 spec 中 AIR、D、V、主要证据 IDs，所有目标均能解析；每 AIR 至少一个有效 V，所有 MUST 的失败分支有断言；pending不得在验收中被描述为已实现或已通过。缺少live结果不是缺陷，已明确离线范围。

**处置**：本轮文档交付前必须修复。主 Agent 已在继续编写矩阵，后续稿完成后可直接关闭本项。置信度：高。

## 首轮建议澄清（已关闭）

### SR-05 · P2 · AIR-003 / AIR-005：attempt “active” 和 producer cleanup 完成不是同一个观察点

**证据**：AIR-003 要 terminal交付后 release inner attempt；AIR-005 同时要求 drop释放 producer/HTTP等待、至多一个active attempt。当前 `provider_collection.rs:769` active slot 与 `:785` release 跟 wrapper生命周期绑定；具体 adapter 则有后台 producer。V-05 已承认 lease proof不等于inner销毁。

**问题**：如果新attempt在旧wrapper终止后即可打开，但旧producer仅已收到取消尚未实际销毁，则“至多一个active”究竟针对可继续产生用户事件的attempt，还是所有尚未结束cleanup的任务？实现者可能为追求物理零重叠引入额外异步关闭协议，也可能只测布尔lease就宣布资源契约成立。

**建议**：定义可观察状态：terminal/drop立刻禁止旧attempt再交付或发起新模型请求；已有本地等待可中断并在测试屏障上证明收束。明确新attempt是否必须等cleanup acknowledgement，或允许旧attempt仅处于不可再产生效果的cleanup阶段。若采用后者，断言“不同时存在两个可执行attempt”和“最终local cleanup完成”，不要以依赖机器负载的毫秒限时替代同步证明。

此项不要求改变已确认的取消优先级或新建公开cleanup framework；可在主 Agent已有生命周期定义足够明确时关闭。暂列可延期文字澄清，实施前仍需选定验收含义。置信度：中高。

## 检查通过的设计取向与非缺陷

- **Retirement 八要素**：七行清单已有当前依赖、理由、替代 owner、保留/改变行为、迁移范围、过渡退出、测试变化与证明；没有“只新增新入口永久保留旧路径”的授权。AIR-020 初始builder仅构造新representation，不是隐藏的legacy dispatch。
- **权限**：canonical声明与trusted schema分离，分类概率不授予权限，strict采样不替代本地validation；SR-02只补充visibility语义。
- **历史与数据**：legacy-unknown child保留原文件，gap acceptance与CurrentSnapshot选择独立，compaction保留checkpoint，presentation export不冒充lossless协议，均有证据支持。
- **数值资源边界**：queue64有当前实现依据；image/classifier数值明确标proposal且待确认。已给byte、count、nesting、numeric tolerance，32MiB媒体base64加64KiB文本可落在48MiB body上限内；无证据要求把这些值伪装成供应商上限或性能最优值。
- **能力宣称**：一条OpenRouter image adapter与两provider共用System One不被夸大为两种独立算法/adapter；future external protocol revalidation有明确范围，不允许执行Agent默换产品目标。
- **验证诚实性**：未将pi测试声明或mock数量等同真实远端能力；live验证需要另行授权，合理。spec英文/中文逐句等价未在此独立评审完成时验证，主 Agent须随同步稿检查。

## 首轮收束建议（已落实）

先修 SR-01/SR-03 与追踪节点；SR-02 如现有回答不足则做一次窄范围决策。Q19确认后，将 initial/continuing、pre-commit/ack/post-ack、sessionless/durable 四个维度的失败验收填齐。修订后主 Agent应记录这些条目的关闭依据，再运行仓库文档检查；本报告未执行 `opi-doc-check` 或runtime测试，不声称任何候选AIR已通过。

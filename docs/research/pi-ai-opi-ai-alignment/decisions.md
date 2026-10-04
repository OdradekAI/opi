# 技术决策讨论

> 非规范性候选决策。2026-10-03。**Q1–Q21 是首轮回答记录；后续用户更新了判断前提，受影响项已通过Q22–31及修订Q31完成重新选择。先读 [决策重审](decision-reassessment.md) 与 D-18。**
> D-01–17保留原证据和回答历史；其中旧规范门槛、删除、严格投影与延后建议不自动构成当前结论。
> 用户已确认研究方式、写入范围、权限边界、双语候选 spec，并在本次对话确认现有链优先且纳入图片／分类目标、允许有边界的0.x破坏性提案。后续Q6确认具体候选淘汰清单；全部确认均不授权本次实施或实际删除。
> `推荐`、离线测试通过、子 Agent 意见和提问的预选项都不是批准。

当前实际选择集中在 D-19–37；Q22–37 及修订 Q31 已回答。下面保留首轮原答，不把已替代的 Q8/Q12/Q14/Q15/Q20/Q21 当现行选择。

领域名称沿用 `docs/CONTEXT.md`。本文的 logical call 指一次准备后固定 request、route、auth 的模型调用；attempt 指其一次 wire 尝试。它们是当前代码已有语义，不建立第二份领域词典。正文只在关键决策确认后进入 to-spec；尚未确认的方案不能变成无条件 MUST。

## 首轮回答与当前选择导航

| 回答 | 人类确认的目标（仅候选设计，不授权实施） |
|---|---|
| Q1 / D-01 | 现有链优先，并纳入图片生成／分类 |
| Q2 / D-02 | 允许有边界的0.x破坏性提案；具体项目另确认 |
| Q3 / D-13 | SDK／嵌入式Agent显式调用，不新增默认终端工具 |
| Q4 / D-03 | 唯一prepared model binding |
| Q5 / D-05 | 完整transcript与持久化迁移纳入目标 |
| Q6 / D-08 | 确认旧公开壳及无行为compat字段淘汰，明确替代与旧键错误 |
| Q7 / D-06 | 工具图片真实发送；无法表达则明确拒绝 |
| Q8 / D-07 | AWS discovery迁产品resolver，保留全部已有行为 |
| Q9 / D-04 | 确认terminal吸收、异常EOF错误、取消优先、attempt drop与logical cancel区分的整组契约 |

Q10确认两种operation归入opi-ai并提出有界规范修订；Q11确认legacy-unknown child迁移；Q12确认默认严格、允许显式可追溯的current-state投影；Q13确认首批OpenRouter图片与TypeSafe/OpenRouter System One；Q14确认nonstream、整体deadline、无默认retry、分类原子结果和failure usage；Q15确认固定执行权威与动态声明快照。Q16/修订Q17确认失败历史投影，Q18确认验收与保留/延后边界，Q19确认规范澄清与commit后保留，Q20确认声明外拒绝执行，Q21确认首版资源边界。独立专题见[操作契约](operations-design.md)与[归属选项](placement-options.md)。

## D-01 对齐范围与优先级

**问题**：在同一研究里区分“必须调查”与“建议实施”。证据矩阵覆盖15主题，不等于全部入围。

**证据**：M-01–15；A-02/03/04为当前已支持路径的正确性问题；EVO-009表明image/classifier有真实pi消费者且近期整合；P15仅找到faux deferred实现；opi没有对应operation消费者证据。

| 方案 | 收益 | 主要代价 |
|---|---|---|
| 现有链优先（推荐） | 修复可观察内容与路由/生命周期错误；保持小而深Agent Core | 新operations、WS和远端目录暂不扩展 |
| 现有链+image/classifier | 同时定义多operation未来接口 | 需要opi使用场景、placement、身份/usage/错误/测试边界；更大破坏面 |
| 研究全部后再定 | 最大化决策信息 | 暂不能综合实施spec；本轮证据已可支持下一次选择 |

推荐顺序：P0 当前内容/有效route/终止/资源正确性；P1 已证明公开冗余和能力声明收敛；P2 有明确真实消费后的新operation。这里P0/P1/P2只表达建议优先级，不是工单或产品阶段。

**人类确认：现有链优先，图片生成／分类同时纳入候选目标。** Q3/Q10/Q13已固定SDK消费者、opi-ai归属与首批provider；正式规范尚未修订、准入尚未完成。规范关系：§3.2、PRIN-001/002、GATE-001/002，不因pi已有代码免除准入。新增operations的准入证据与placement另行调查；不是上游新增文件就能进入Agent Core。

## D-02 破坏性变更原则

**问题**：无当前生产职责的公开壳及无行为配置如何退出。

证据O-003/004；AGENTS允许有意scoped 0.x breaking changes，但要求未来Unreleased与同步消费者/schema/fixture/docs。推荐在每项核实替代与退出条件后允许直接淘汰；保留ApiKind等持久化使用者，避免“名称重叠即合并”。

可选兼容期只有在明确外部consumer/承诺存在时才值得：设定具体版本退出、deprecated诊断和唯一实现；不能无限转发旧入口。外部使用量未知并不能证明安全删除，也不能强制永久兼容。**人类确认允许提出有边界的破坏性淘汰**；D-08具体清单已由Q6确认。本次不改Unreleased、公开API或配置。

## D-03 有效模型绑定的唯一责任

**问题**：prepared route写入的wire/能力与adapter实际用的目录可能不同（O-002）。这影响事实正确性，不能用“字段都存在”验收。

| 方案 | 取舍 |
|---|---|
| 保持现状 | 启动active路径已有物化，但无法保证非启动override/dynamic一致，不推荐 |
| 所有route共享前物化；限制动态可执行字段 | 破坏较小，可修静态装配；需要明确禁止/拒绝无法物化的刷新，保留后续迁移成本 |
| prepare固定唯一effective binding（推荐） | preflight、route facts、wire、compat、endpoint、effort和输出限额来自同一选择；需要收窄或改动现有adapter入口 |

目标候选：registry是目录选择authority；collection负责一次准备并冻结选择结果；adapter消费该结果，不再次以不同catalog选择wire。保留registry/collection/mapped-provider职责，不新增通用模型路由trait。旧prepared call不因以后目录变化而改变；新call使用新快照。

失败候选：无可执行wire或不合法compat在auth/网络之前失败；不把可列出模型误当可执行。迁移要覆盖direct adapter tests与外部provider实现，不能新增一条binding路径后仍让默认产品走旧路径。

规范关系：落实INV-001/002，不建议修改这些规范。**确认状态：Q4已选择唯一prepared model binding**；机械验收V-01。重评触发：发现现有外部adapter无法接受有效模型而需明确过渡；不能静默保留双authority。

## D-04 流状态机与资源收束

**证据**：A-02/03/04、P03/06/07，已有70测试与隔离probe。

推荐候选契约：

- 调用者已收到的首个 terminal 是吸收态；之后永久EOF，释放attempt内部资源。
- 未见terminal而底层EOF，公开prepared stream产生一次typed StreamError再EOF；不由每个consumer独立猜成功。
- terminal尚未交付且已观察共享取消时，取消优先、丢弃缓冲输出；已交付terminal不撤回。只要求可观察点的优先级，不许宣称跨线程存在无法证明的全序。
- drop attempt结束该attempt的本地producer/HTTP等待，保留logical call在未取消、未credential-terminal时的后续合法attempt；共享token取消终止整个call。
- 有界队列的每个等待点响应取消和receiver closure；不将“断开本地socket”承诺为服务端已撤销生成或计费。
- 合法网络分块不改变文本；无效frame明确失败且脱敏。共享framing实现可私有，不以未来WS预建公开抽象。

替代方案是维持consumer守卫，但维护成本回到Agent、compaction和SDK；默认产品目前已有防御，不能夸大已发生尾事件泄漏。

超时推荐维持层级清晰：HTTP timeout是attempt预算；OAuth刷新有自己的budget；全logical-call deadline由caller策略负责。若要新增默认整体deadline，需要另行决定默认值、覆盖auth/backoff范围和到期错误；本研究不预设。

规范关系：§3.3 bounded buffers/ownership与§7.1 bounded failure；无需复制pi弱EOF或无界FIFO。**Q9已确认整组生命周期契约。** 验收V-02–05。重评触发：新adapter/transport或新增回放/恢复能力。

## D-05 完整 transcript 与持久化迁移

**证据**：P04/05/19、O-010、EVO-007/008。opi的Request由owned system/tools/messages组成，prepared call冻结，已能提供每call确定性；这不等于pi逐时点SystemMessage历史。

| 方案 | 合适场景／代价 |
|---|---|
| 保留当前snapshot，明确差异（未采纳） | 当前静态工具定义及每轮请求；无持久化迁移。明确不保留历史system/tool可见时间，也不保证pi缓存前缀行为 |
| 下一call原子替换system/tools snapshot | 用户明确需要运行中loadout变化；需冻结执行权限与模型声明一致性，变更不能影响已准备call |
| 持久化变更时间线 | 需要重放原始历史、动态工具声明和缓存前缀场景；涉及Agent、hooks、session、compaction、durable格式和wire降级 |

即使选择snapshot，也要独立决定合法history重放契约：跨route的signed/opaque thinking、tool IDs与缺失results、failed/length助手和malformed arguments。推荐不机械复制pi自动补result、coercion或历史collapse；不能把非法输入悄悄变成 `{}` 并称无损。

若选择完整transcript，需要明确：历史记录owner、声明不授权执行、同名重定义/删除顺序、旧记录迁移、adapter不支持原位变化时允许拒绝还是降级。关键项未回答时不能拟定其MUST。

规范关系：INV-002/004/005、§8.1；目标扩大需要placement与人类确认，不直接授权新框架。**Q5选择完整transcript及持久化迁移，撤回近期保留snapshot的建议。** 旧session只存system/tools摘要，不能恢复原文时间线。Q11确认保留源文件、显式创建v3 child，未知前缀不得冒充历史事实，继续需接受缺口；Q12确认默认strict拒绝无法表达历史的wire，显式current-state投影必须可追溯。Q15确认每次变更为完整system+visible tools快照，下一call原子提交；trusted registry/实现/schema/User Policy保持固定，声明只选子集；binding不改写，每call关联prefix；实现/schema/source变更重新可信装配并创建新binding child。V-06/07及后续持久化验收。重评触发：真实动态loadout/历史重放需求或可验证缓存收益。

## D-06 工具结果图片的真实语义

工具图片作为模型输入，与image generation独立。O-009确认public ToolResult可携带图像，但多wire发送MIME占位；本仓库默认内置工具未找到直接构造图片，用户附件路径则已存在。

候选：

1. 支持各wire可表达的工具图片，否则在preflight明确UnsupportedCapability（推荐）。需要wire具体验收，不假装所有provider原位同构。
2. 明确将tool image定为不支持并统一提前拒绝；改动较小，限制SDK/extension视觉工具。
3. 保留可观察降级，输出明确的降级事实；模型仍看不到图，必须由调用方选择，不能默认静默丢失。

继续保留无说明placeholder不可满足“supports_images就能看tool image”的合理期望。**Q7已确认方案1：真实发送或明确拒绝。** V-08。规范关系INV-002与fail-closed能力边界；新图像生成操作不由此附带准入。

## D-07 AWS discovery/process 的 Placement Review

**事实**：O-007，AWS helper在opi-ai读取profiles/env并执行credential_process；产品确实调用，且已有输出/超时/清理机制，不是可随意删除的死代码。

推荐保留SigV4、typed credentials、Bedrock wire在opi-ai；将本地来源顺序、profile读取与shell process policy移到Reference Product现有AuthResolver实现。优点是中立库保持机制，复用已证明seam；代价是公开exports及调用方迁移。迁移必须保持来源优先级、partial source拒绝、10s/64KiB/cleanup/provenance等已有行为，不能“移动”时省掉安全语义。

替代是保留现owner并对规范作明确解释/例外。**这与`docs/opi-spec.md` §8.1 的 product neutrality（无operating-system opinion）及PRIN-003形成张力；若选择继续将OS/profile/shell策略作为Agent Core职责，需明确是否打算修订规范及其范围。** pi把类似机制放在库里不能自行修改opi权威。

依§8.2跨层移动需ADR；当前`docs/adr/`不存在，因此只记录候选，不创未确认ADR。没有证据现在应新建Companion或crate。**Q8已确认迁到产品resolver并保留全部行为。** 本次未修改规范，不新建ADR；未来正式迁移必须履行§8.2。V-09。重评触发：两个独立外部使用者需要中立credential discovery产品。

## D-08 逐项淘汰候选及退出条件

Q6已确认旧配置／身份壳和无行为compat字段；Q4确认第二次独立路由退出；Q8确认AWS发现职责迁移。下表是已确认的候选处置，仍不授权本次改代码。

| 候选与当前职责 | 替代责任与保留行为 | 迁移范围／有意改变／退出证明 |
|---|---|---|
| `Config/ProviderConfig/Error/Result` 旧配置错误壳（O-003） | 产品OpiConfig管配置，opi-ai现有ProviderError/ResolvedAuth管运行时 | root exports/模块/文档样例/未知外部embedder；移除无接线的明文key配置面；未来Unreleased；生产及当前文档旧符号为0，现有消费者编译通过 |
| `Model/ProviderKind` 旧身份壳（O-003） | 现有ModelInfo/WireApi及registry身份 | 删除只有构造/枚举存在断言的测试；保留ApiKind和所有持久化消费者；旧path退出而模型路由fixture保留 |
| 无wire行为的compat字段（O-004） | 请求ThinkingConfig+model mapping；每个有实际wire作用的compat继续存在 | Chat reasoning_effort、require_assistant_after_tool_result、supports_store/developer_role/reasoning_effort，及Responses legacy reasoning_effort均已确认退出；产品配置解析、merging、factory、schema、README双语、fixtures同步；旧键显式报迁移错误，不能serde默默忽略 |
| 旧effective model选择路径（D-03） | 唯一prepared binding | 所有生产dispatch、adapter conformance、extension override、新ID/跨wire；禁止仅并行增加新入口；老的第二次catalog路由必须退出 |
| AWS discovery exports（D-07） | 产品AuthResolver | 迁移既有使用者与隔离credentials tests；wire/signing保留；同一来源决策只剩一个owner；过渡版本若需要必须单独确认 |

每个淘汰项的未来test impact在V-10/11中分类。对Q6清单采用有边界的0.x直接破坏性迁移，不默认兼容期；旧配置键显式migration error。新的外部兼容证据须重新讨论，不能自行留下无限期shim。不触碰released changelog、原研究与历史证据中的旧符号。

## D-09 值得保留的鉴权、重试与错误边界

推荐保留现有typed AuthScheme、per-call冻结、auth失败不自动换provider、错误producer脱敏、Agent重试policy。P09/P17说明pi实例auth可借鉴但不应降低opi边界；P08补充pi还有assistant-call utility及产品完整回合重试，因此opi输出后禁自动retry是有意更保守选择，不是宣称全pi等价。CredentialStore/OAuthProvider有真实variation，不能按接口数删除。

代价明确：凭据过期或撤销终止当前call，下次显式call重新准备；不会自动在同call换身份。需要新行为时再访谈，而不是称“修复retry”。V-12/13保留既有proof并检查本次其他迁移不回退。Q18已确认这组保留边界；这是保留现行行为的决定，不需要本次修改规范。

## D-10 Usage与成本的可信程度

推荐保留整次reported、optional子集与父桶一次计数，不跟随pi某adapter把缺失变0。明确child累计Some仅是已报告部分之和；不把估价当账单或完整成本限额。暂不将所有父桶改Option、不新建通用计费接口。若未来消费必须按完整费用限额，就需完整性元数据/计价owner单独设计和格式迁移。

证据M-11/O-011，V-14。无已测性能或财务准确性承诺。Q18已确认保留与延后；重新评估触发为预算执行者对缺报明确处理需求。

## D-11 延后能力及重新评估条件

| 能力 | 推荐延后理由 | 可操作的重新评估触发 |
|---|---|---|
| image/classifier operations（已纳入目标，不整体延后） | Q3/Q10/Q13已确认消费者、首批adapter与opi-ai归属；正式规范修订尚待未来办理 | 仅在有界规范修订与正式准入后实施；不扩大到所有模型operation |
| deferred responses | pi内建远端adapter证据不足 | 确切durable consumer、远端fetch/cancel/expiry协议与离线fixture |
| deferred tool loading | 与动态loadout/transcript互相依赖 | 完整transcript已确认；仅在实际按需工具加载workflow另获准且执行authority不因声明扩张时重评 |
| WS continuation/fallback | 无opi性能需求证据；带connection与session生命周期 | 测量到SSE瓶颈或必需能力，明确account/session资源owner、输出后不重播 |
| remote/background catalog | 当前无生产refresh trigger | 产品刷新需求、来源/缓存/过期政策与允许的网络authority |
| 新trait/crate/feature或TS同构子路径 | 无固有状态机价值或第二consumer证据 | PRIN-001/002与§8.1证据满足，性能主张有测量 |

D-01已确认图片／分类进入候选目标；Q18已确认本表其余延后边界；它们不是永久排除，依触发条件重评。

## D-12 Schema与测试边界

推荐保留最终tool参数在Agent的trusted definitions和User Policy之前校验，raw partial仅为未完成材料；不自动复制pi的类型coercion。strict wire只是供应商采样约束，不能代替本地schema/permission检查。无preview消费者就不新增partial-json公共seam。

候选spec的最高既有测试边界：ProviderCollection prepared call + MockProvider（生命周期）；具体Provider+loopback伪传输（wire）；Agent tool execution（校验/重试/无副作用）；产品factory+model switching（override）；现有session roundtrip（若持久化变化）。Q18已确认这些边界与范围匹配，避免把未经确认的新seam写成需求。验收矩阵目前是验证设计，不是test tickets。

## D-13 两种新操作的 opi-ai 归属与有界规范修订

**已确认：Q10选择opi-ai；Q13选择OpenRouter图像生成与TypeSafe/OpenRouter共用System One分类协议；Q14选择typed nonstream、显式总deadline、无默认retry、分类整体成功/失败及failure保留已知usage。** Q10第一次回答是核问pi归属，未当批准；后续明确选择才记为确认。

证据：[operations-design](operations-design.md) OP01–10、[placement-options](placement-options.md)、EVO-009。pi 0.99把平行ImagesModels并入Provider/Models且有codemode消费，支持避免重复auth/catalog的方向；不证明现行opi核心门槛已通过。首批图片只有一个真实adapter；TypeSafe/OpenRouter是两个provider，同一System One wire实现，不能称两种独立算法。

保留现规范的替代是Independent Companion责任范围，或条件性core等待证据；用户选择在候选中提出有界修订。代价：opi-ai职责与公共目录要超出chat，增加明确typed契约与协议维护；收益：共享已确定身份、鉴权冻结与模型目录，SDK无需平行集合。**不得由此推出所有provider-neutral服务都自动进入core。**

需未来正式同步修订的具体条款：

- GOAL-001：两种显式SDK操作的端到端owner为opi-ai SDK owner，验收跨public SDK→auth→真实adapter伪传输；无需为验收伪造默认终端功能。
- GOAL-002、PRIN-001与§8.1 semantic necessity/depth/coupled lifecycle：新增仅适用于本次chat/image/classifier中后两项的有界SDK理由；提交真实接口、来源、同一目录/auth所有权与版本耦合的placement case。当前缺口如实保留，不写成旧门槛已通过。
- §8.1 real seam / PRIN-002：首批图片是一个具体adapter，不能以通用ImageProvider trait或虚构consumer绕过两adapter门槛；若把单adapter能力也纳入core，正式修订须显式列出该有界首批例外。已有Provider集合可复用，新增共享公开seam仍需真实variation/conformance。
- INV-001/002：目录身份扩为operation+provider+model；chat现有lookup只返回chat；结果为各操作typed值，不强制伪装聊天流。
- GATE-001/002继续生效，依据正式修订后的条款审核；最小权能、权限不扩张、单向依赖、错误脱敏与机械验证不豁免。不得用feature flag或pi parity代替审核。

本次**不改正式规范**。该修订包是实施的authority前置条件；user确认目标不等于已完成修改、ADR或admission。跨层职责调整及其例外适合后续一个Placement ADR，不逐普通实现选择造ADR。

图片首批为prompt/reference images到owned bytes+MIME，可附text；不自动下载URL或写盘；空结果/坏媒体不成功。分类choice/bool/ordered-score，threshold/action由SDK caller拥有，概率不能授权工具。Cloudflare/llama、多模态流、server工具、图片编辑专用参数、后台操作/重试框架均不附带纳入。typed方法与现有collection共用authority；chat模型数据不填虚构数值以容纳image。

## D-14 transcript 持久化与权威

Q11/Q12/Q15已确认。逻辑transcript由opi-ai表示和验证，Agent拥有下一call原子状态，session owner保存唯一durable prefix；Reference Product负责迁移命令、binding provenance和组装。系统/工具完整快照避免为了pi同构引入section patch DSL，明确replace/clear语义；无变更不追加重复状态。工具声明不能修改trusted schema或实现。

旧格式迁移创建新版本child并保留源文件。可验证的active-branch messages继承，历史system/tools以legacy-unknown标记；当前state只从迁移边界开始生效。严格历史重放拒绝unknown，普通继续必须显式接受并保留缺口事实。新版本不默认继续追加旧格式；旧reader按INV-007拒绝新required entries。compaction需要在保留suffix前记录完整context checkpoint，不能只保留工具delta或用当前配置回填过去。

不可表达完整历史时，strict默认在auth/network前失败；显式current-state投影只影响wire，不改canonical历史，call记录projection和源prefix。它可能改变历史语义，不能作为pi等价或缓存收益证明。新的provider beta wire由实际fixture证明支持，未核验能力默认false。

Runtime Input Binding保持不可变。上下文变化是受固定可信装配约束的运行状态转换，每call引用该binding及已提交prefix；工具实现/schema/source或授权集合需要变化时，创建新assembly/new-binding child。工具策略可收紧/撤销，不能通过context更宽；热替换实现不在本次范围。新durable entry不得省略为ignorable observation。

## D-15 历史失败投影及工具可见性

证据[R01–08与RP01–11](replay-contracts.md)补足最初15主题矩阵未完全展开的真实恢复路径。坏JSON与Length后的未执行result已有Agent生产分支和测试；不能把所有坏JSON一律拒绝，也不能伪造成空对象。Gemini每轮从fc_0开始，跨已闭合batch重复字符串不是损坏。

**Q16已确认显式投影**：建议Error/Aborted只留canonical，不发wire；Length保留可读文本并提供普通未执行上下文，排除proposal/未执行result；有真实副作用的矛盾历史拒绝，不能吞掉。替代是保留当前未经stop_reason筛选的wire行为，代价是继续不完整调用链。

**原Q17与修订Q17均已答；以修订后的事实支持投影为最终选择**：最初题目发出后发现恢复路径反证，已发修订题；用户先回答原题，随后明确确认修订题；最终是有可信未执行事实的坏调用对可作普通失败上下文投影，无依据损坏仍拒绝。推荐对可信NotExecuted事实支持的坏proposal/result作普通失败上下文投影；其余关联歧义、orphan、未知缺result严格拒绝。源码只能证明当前行为，不决定投影目标；Q17修订已确认推荐方向；不再执行原题过宽的全部坏JSON拒绝规则。

工程细化：闭合batch identity+source ID建立一一关联，target wire需要时作request-local bijection，保留源IDs；NotExecuted由Agent分支写入typed disposition，is_error/自然语言/marker不构成事实来源。普通failure marker仅存在request view，既不成为canonical用户输入，也不改变Copilot请求initiator。具体字段、role、marker、允许原因和canary验收由spec AIR-012/013统一规定。

当前纳入的continuation修正：统一provider+exact wire+requested model身份；保留Anthropic已存在的signed/redacted同路由；分开Responses item_id/call_id；保留Gemini实际提供的functionCall.id；修正Chat混合Text丢失和Responses可表达的block顺序。其余encrypted/thoughtSignature/完整Bedrock reasoning parity不因本次完整system/tools transcript自动纳入；需要真实使用、parser→storage→wire fixture和协议证据才重评。缺失continuation与legacy-unknown context是两种不同缺口。

**Q20已确认声明外工具拒绝执行**：固定registry中尚存在但从本call声明移除的工具，推荐拒绝新模型调用，返回可信未执行结果；后续仍执行trusted schema与实时User Policy。替代为visibility仅提示用途、仍可调用所有获准注册工具。用户已选择前者；后者仅保留为被否决替代，不能由执行Agent再选。

这些选择落实INV-005/006，保留原始审计数据与失败恢复，但改变部分wire表示和旧容错断言。V-07/15/16验证普通恢复、关联、权限及旧serializer路径退出；不声称不同wire收到等价提示就产生同样答案。

## D-16 新操作的具体资源边界

**Q21已确认首版固定边界**。spec工程附录给固定首版范围：图片数量/解码bytes/body/text上限，分类题目/labels/levels/JSON大小与深度，以及概率和/score mean容差。推荐固定有界初值而非新增通用配置层；好处是离线可机械验证、内存有上界，代价是超出限制的合法远端请求暂不能表达。它们是研究者设计值，不是供应商限制或benchmark结果；Q21已批准候选初值。

只做base64、MIME与container signature检查，不加载像素解码器；SDK返回的不可信bytes由caller决定如何消费，不承诺图像完整可解码或视觉正确。已选explicit deadline覆盖整个新operation，复用现有transport/error安全边界；不改变chat timeout。AIR-031采用已确认附录值；为闭合请求大小，文本上限复用64 KiB、编码请求复用48 MiB，属于同一有界设计的工程细化。

## D-17 提交顺序与同一调用身份

Q15已确认binding不可变及逐call prefix；**Q19已确认**进一步固定规范澄清和已durable-ack状态的失败语义。现行CONTEXT将material inputs绑定one run，INV-008要求精确输入证据；新增动态prompt/declarations不能只展示静态digest并声称它覆盖所有实际材料。候选正式修订应明确初始trusted assembly/binding与已提交prefix/projection共同识别实际call。原header不改写；更换实现/schema/source重新装配。

独立评审SR-01促成顺序修正：先持有owned pre-auth模型计划及有效参数/投影，再提交；auth只补允许的endpoint/header材料，不重读目录换模型。已提交plan不声称auth或dispatch成功；resolved非秘密dispatch fact由同一session owner记录，失败不发HTTP。SDK无durable session时使用owned memory状态，不强制建文件或child。

Q19确认：commit前失败回滚且queue保留；ack后安装，后续auth/network失败或cancel保留accepted baseline；单一事务owner同时处理context、compaction与leaf。不再要求run失败就删除已确认接受的输入。此为可观察变化，必须更新对应failure rollback测试，不伪装纯重构。

## D-18 本轮判断基准的更正

**用户直接确认的新指令**：opi 是 pi 的 Rust 实现；opi-spec 仅作可能过时的参考。差异先调查 pi 的设计意图、实现与可靠演进依据，再区分 Rust 实现需求、存量迁移、独立政策及缺陷/实验。不能以旧准入条款或 opi 尚无消费者单独否决已落地的 pi 语义。

这是本轮研究的明确授权和方向更正，不是对所有新默认值的批量批准。Q6/Q8/Q12/Q14/Q15/Q16/Q17/Q18/Q20/Q21的有关部分需按 [逐题复审与 RA-01–10](decision-reassessment.md) 重新决定；Q11可保留历史迁移选择但不再视为pi或Rust要求。Q10归入opi-ai结论保留，旧“有限破例”论证撤回。正式规范本次不改，未来同步目标也不应反向决定本轮答案。

## 第二轮目标决策（D-18之后）

下列具体提案按用户实际回答记录；未回答不视为批准。D-01–17保留首轮历史，受影响部分以明确更新后的第二轮条目为准，不能只摘取旧段中的“已确认”。后续正文改写依赖相应目标确认，不依赖旧placement门槛同意。

| 决策／问题 | 可评审目标 | 代价／替代 | 状态与旧项影响 |
|---|---|---|---|
| D-19／Q22 | named sections、工具add/remove/redefine；自动native-or-collapse并记录，Strict显式；trusted host在call之间更新revision，冻结每call；active executable与wire-visible分离 | 扩展Agent/session来源表达；保留首轮strict及run-fixed可降低改动但形成pi语义差距 | **Q22明确确认**；替代Q12/Q15/Q20相关部分；legacy child未重新打开 |
| D-20／Q23 | request-local ResultUnavailable/UnknownEffect；Length保留合法调用、Agent记录确知未执行；raw/effective分开，schema normalization→validation→authorization→execution | 部分旧严格拒绝/容错测试改写；保留旧拒绝与无coercion会减弱pi恢复 | **Q23明确确认**；替代Q16/Q17/Q18相关部分；trusted hook改写后是否再次校验由Q25另行确认 |
| D-21／Q24 | pi真实compat补齐或等价合并；AWS独立SDK可复用resolver，产品配置启用；显式profile与ambient区分 | opi-ai承担真实provider兼容与离线链验收；原产品独占减少库I/O却削弱SDK | **Q24明确确认**；替代Q6能力误删及Q8迁移；旧Config/Model壳与重复静态值淘汰继续保留 |

| D-22／Q25 | 模型输入先normalize/validate；可信beforeToolCall可改execution，不复验模型schema；final authorizer绑定实际执行值 | 扩展旧Continue/Deny hook及观察阶段；trusted hook对超schema值负责，模型仍无授权入口 | **Q25明确确认**；raw/normalized/execution可辨，权限及拒绝保留 |
| D-23／Q26 | 远端catalog/cache/refresh、Codex auto WS/continuation/SSE、opaque thinking、渐进声明＋可选tool_search、公开deferred lifecycle＋可重开fixture；库与产品接线 | 跨Agent/session/Reference Product；完整MCP/codemode、真实remote deferred、其余operation adapters列后续差距 | **Q26明确确认**；替代原Q18整体延后；offline/SSE覆盖，tool_search inactive；pi实际默认值见EP附表 |
| D-24／Q27 | image retry0/SystemOne2，attempt timeout+可选total；产品高层恢复enabled、3次/2s/60s；关联新prepared call重新解析同目标auth | 恢复可能增用量；账户实质变化终止；lowlevel frozen且不隐式重播内容、不换provider | **Q27明确确认**；替代Q14/Q18相关策略；低层HTTP/WS/高层恢复不可重复叠加计数 |
| D-25／Q28 | 有序image blocks/firstchoice；有限可覆盖预算，无unlimited；SystemOne无10档限制/必需distribution；基础typed/finite/domain校验，extra IDs忽略，无额外sum/argmax拒绝 | 比首轮接受集更宽；预算不绕格式/协议；固定预算及严格数值是替代 | **Q28明确确认**；替代Q21相关固定及误套限制；无sum/argmax拒绝，不要求score distribution |
| D-26／Q29 | 高层effort按pi支持等级选择并记录requested/effective；native/exact显式验证 | 向上选择可能更多思考量；全拒绝是替代，非Rust必然 | **Q29明确确认**；RA-11纠正旧语言归因 |
| D-27／Q30 | trusted SDK payload transform一次；retry复用；wire/JSON/预算检查，identity与protected auth不准改；只读response observer | SDK定制新增真实入口且比pi任意改写收窄；typed-only是首批替代 | **Q30明确确认**；不得增加无限通用operation框架 |

### D-28 由已确认目标推导的技术收敛

此处是主Agent的技术判断，**不是额外伪造的人类回答**：

- compaction按pi当前有效checkpoint＋summary＋retained非system历史＋后续变化；撤回首版由研究者新增、未单独提问的逐保留旧call完全保真验收。完整原始分支事实仍可审查，压缩给模型的上下文明确有损。这落实D-18/19，而非新增更强政策。
- Rust当前公共schema是JSON Value；normalization对齐pi已测试的serialized plain-JSON路径，不复制TypeBox私有metadata分叉。已知差异明示，raw保持，effective值按固定corpus；不添加兼容模式。此为语言/依赖及真实公开seam的实现选择，落实Q23。
- pre-auth冻结选定模型身份、显式数据和原始输入；auth合法返回endpoint后完成endpoint-dependent compat默认推断，再得到唯一final binding。模型不重选，adapter不再catalog lookup。不能把未知最终endpoint的推断在auth前冒充已冻结，这是Q4/Q24实现一致性所需。
- AWS完整来源失败关闭按Q24继续保留：typed/已选scoped pair不完整不与ambient拼接。pi逐字段overlay接受集不同要如实记录；保留这一既有保护不是把它说成Rust必然。完整默认chain内部顺序仍由一手依赖调查确定，不让用户猜技术事实。

补充独立评审后的技术闭环：Q19保留已接受输入，Q30允许auth后形成最终payload，因此同一事务owner必须在final freeze后、首次dispatch前确认实际prefix/revision/projection；失败禁发但不撤销先前accepted input。复用适当既有记录，不规定每call固定双append。Q27恢复仅限同账户：凭据实变又无法证明同账户时停止自动恢复；不暗中增加identity探测网络。目录404/501下一次restore的typed tombstone在无bundled generation时也生效，是AIR-034的显式一致性修正，不冒充pi逐分支等价。

精确契约和验收输入见 [transcript target](transcript-target-proposal.md)、[SDK target](sdk-target-proposal.md)、[evolution target](evolution-target-proposal.md)。这些提案的“待确认”不覆盖本表已收到的具体回答；Q22–31及修订Q31全部收到明确回答；外部事实缺口仍需继续研究，不能冒充目标已实现。

### D-29／Q31：per-tool constrained sampling

**Q31原题确认范围，修订Q31确认正确降级，两者均已实际回答，以修订题为最终语义。** 首批JSON strict为prefer/require及私有wire schema nullable转换；grammar接Responses/Codex，支持时非空lark优先regex，不支持时回退普通JSON工具并记录实际投影。支持grammar却无可用variant或非法输入schema提前失败。本地plain-JSON参数验证、可信hook与最终authorizer独立。

原题“其他wire一律unsupported”在补核 `constrained-sampling.ts:251–283`、test:117后被撤回并重新询问，不能由原题范围确认反推该错误建议获批。grammar需要object参数、恰一个required string property；custom call/raw text与JSON wrapper、item/call IDs、tool output、增量append-only与持久化一起迁移。暂无默认新增终端tool或通用parser框架。

证据TP-14、P16、pi types:693–719、constrained-sampling:202–283与其测试。验收AIR-040/V-34；接受有限schema子集以及unsupportedgrammar回退会改变采样约束，仍须保留本地验证及投影事实。

### D-30：以独立 SDK 使用方收敛调用契约

**用户2026-10-04直接要求**：只调用pi-ai/opi-ai的开发者应理解一致的调用方式、执行流程和返回语义；opi-ai可无缝嵌入其他项目并发挥Rust优势。这是新补强目标，不用旧spec的产品准入反向决定SDK形状。

依据UX-01–09和两份SDK独立审查，主Agent技术收敛为唯一Models（合并现ProviderCollection实际状态，而非永久新增薄壳）、四chat入口及同实例typed操作；snake_case、Models::new、owned record/Context、标准Future/Stream体现Rust。内部prepare高级机制保留；普通调用不需显式prepare、事务owner、Agent authorizer或产品配置。同步目录和构造不外部I/O，管理调用明确自己的I/O边界，生成仍可在单次auth解析中做同源按需刷新；首次poll执行但创建call时冻结输入。自定义完整model不要求目录成员命中；目标ModelInfo携带provider_id和bare id，不靠目录反推。native/simple共享结果契约但保持各自默认/映射。SDK工具声明/参数验证不自动执行。

这是对用户明确意图的技术落实，**不是伪造额外人类回答**。现ModelInfo保留、旧Model壳退出；高级状态机有独立价值，不能随入口收敛删除。当前pipeline/core边界记录为正式协调候选，本次不改正式规范。V-39–42从真实外部consumer验证；source/API差异和load/性能声明区分。支持集合明确，不能把统一调用宣称为所有pi provider覆盖。详见[sdk consumer](sdk-consumer-contract.md)。

### D-31／Q32：统一 Rust Result，失败保留完整消息

**Q32已实际确认**：采用Rust Result，失败保留完整消息。目标complete与ChatStream.result(self)同为Result<AssistantMessage,CallFailure>；成功Stop/Length/ToolUse/Deferred，失败Error/Aborted。CallFailure包含完整partial/terminal AssistantMessage、已知usage、来源和typed脱敏cause，setup无生成内容且usage unknown。流失败只一个Err，再永久EOF；旧Ok(Error)+transport Err与CompletedRequest退出。Rust差异明确，不称与pi对象结构相同。

主Agent补全机械契约：result主动驱动剩余bounded stream，不依赖另一个drainer；即使terminal已被next取走仍返回缓存，绝不重发请求、自取消或另补terminal。>64事件、partial failure、setup、取消、所有stop reasons及三种消费路径验证等价。不可变event-time partial是语言所有权选择；pi共享live alias只在事件产生时作差分采样。

退出范围包括adapter producers、prepared/mock/drain、Agent正常与summary循环、evidence转发脱敏、外部Provider implementers、fixtures/docs。保留旧持久事件的版本化reader及原始审计事实，不能盲删历史Error JSON。迁移与退出由spec retirement表和V-40/41/20统一规定，没有无限兼容期。本次不修改源码或当前测试。

### D-32／Q33：三条完整 OAuth SDK 路径

**Q33已实际确认推荐**，不是单纯移动当前产品代码。首批Anthropic、openai-codex legacy、Copilot的flow与可注入内存/store/presenter/同源自动refresh由opi-ai可独立使用；TUI/keychain/CLI政策留产品。Anthropic token/refresh采用pi JSON，login带state，固定localhost:53692/callback及copy-code；Codex固定localhost:1455/auth/callback、占用时手贴降级、严格refresh字段；Copilot enterprise/token-derived endpoint/账号模型发现及显式login中的model policy启用。精确字段与失败见[sdk auth/provider migration](sdk-auth-provider-migration.md)。

已确认有意差异：保留device denied/expired终止；保存真实expiry，只一次5min刷新窗口，不复制pi双提前量；refresh默认15s，caller取消继续影响网络，进入不可中断存储事务后遵守ack事实。源码上已选来源refresh失败不转env账号；成功credential及provider metadata原子提交，失败保留原store。未核其他OAuth（包括新openai ChatGPT flow）不自动纳入；offlinemocks不能证明真实注册/远端接受。V-43覆盖完整SDK调用、网络协议、同源刷新与partial effects。

### D-33／Q34：内置 provider 身份与默认 wire

**Q34已实际确认推荐**：已支持集合canonical ID对齐pi。gemini→google、vertex→google-vertex、bedrock→amazon-bedrock；openai-responses与openai收敛同一openai identity，默认Responses，Chat保留为显式adapter/custom profile；azure的内置目标为azure-openai-responses且补齐真实Responses adapter，原Chat保留显式profile。其他同名已选providers不据名称冒充wire全覆盖，Mistral Conversations明确gap。

config/credential key/catalog/SDK lookup/productfactory/schema/help/tests/docs同步显式迁移，无永久两套default ID或隐式aliasfallback。旧sourcehistory与ApiKind/opaque来源不重写；只在验证的request projection bridge解释legacy来源。迁移必须处理destination已有不同credential/config为冲突，不能合并账户或覆盖。可重试copy/ack/reference切换、source保留至成功finalization，由现config/store owner完成，不另建ledger。旧secret不需remote重签/登录/付费调用来迁移。本次不执行迁移。V-44先证明公共id/defaultpayload再证明consumer/config/key退出。

### D-34／Q35：simple 的上下文估计与预算

**Q35已实际确认推荐**：对齐pi canonical transcript估计与simple budget。采用最后适用成功reported正usage anchor+后继文本/图/工具估计，按prefix时间戳排除已失效anchor，不限制同模型、不再max全文下界；无anchor逐message估计。仍保持usage unknown不充reported zero与已确认Rust整数表示区别；这不是账单或远端context保证。

simple mapping与native exact分开：simple默认model cap、context safety4096/min1、对应thinking budget/answer room；native保留所选adapter原生省略/默认/支持gating，不重复collection-wide clamp。跨模型anchor可能让输出room更高，是已确认近似策略选择。V-45覆盖不同输入mode、timestamps、success/error/unknown、跨模型、thinking预算和边界。现估计helper、prepare、Agentcompaction及UI消费者迁移同一算法，无双estimator或新增模式开关。

D-34 的源码精确化（技术推导，不冒充新回答）：正 reported total 优先，缺失/0 才以完整已知 parent bucket 正和作 anchor；独立 total 与计费分桶不混为一值。base 与 numeric Claude/Chat/Google 的公式、普通 Responses/Azure/Codex 的不同 max mapping 由 spec annex 固定，不为方便套全局 clamp。极小 ceiling 下 numeric Claude 的零预算提前 typed BudgetInsufficient；这是修正已复核的 simple 0 与 wire `|| 1024` 冲突，不静默降为 off。共同边界及有意差异由 V-45 验证。

### D-35：SDK 管理、默认来源与公开资源收束

**由已确认 D-30 的独立 SDK 目标与 D-04/23 的资源正确性推导，非额外人类答复。** UX-11–15 补核表明：pi 在 createModels 中提供惰性默认环境来源；checkAuth/available 不刷新 OAuth，getAuth 才可同源刷新；refresh 是目录恢复/刷新并返回 per-provider errors/aborted；login 返回已 store ack 的 Credential，logout 仅删本地 store。采用其可观察分工，明确 Option/Vec/typed 管理失败，不能把真实 store/check 失败吞为空。空 Models 不自动注册 provider；显式 builtin 装配有独立 SDK 默认来源，注入可完整替换 ambient。构造/注册/静态 lookup 零 env/store/外部 I/O，修正前稿“cache-only construction”的矛盾：持久缓存恢复由显式 refresh(allow_network=false) 执行。

Codex SDK 自身省略 transport 默认 auto，不是产品专属。sessionless/no-cache 仍可临时 WS；显式 Sse/Websocket/WebsocketCached/Auto 保持可选。pi 内部按原 options 决定 delta 导致省略与显式 auto 不同，本候选先归一 effective transport，两者遵守同一严格 prefix 条件，属于局部一致性修正。55min max-age 在下一次 idle acquire 检查，不中止正在执行的生成；5min idle/connect15s 仍用单调时钟。

pi 的公开 cleanupSessionResources 是 SDK 真需求，0.73.0 已引入；其全局同步 registry/仅缓存 socket/可能转 SSE 不作为目标。Rust 提供实例内 owned 可 await 的 cleanup_session_resources(Some(id)/None)，以首次 poll 的 generation 截止点取消旧资源 calls，覆盖 connecting、temporary、busy-overflow、本地 producer/timer/延续/降级/诊断，阻止迟到发布与 fallback。精确原始 session ID，不按空字符串 truthiness 清全部；None 含 sessionless。完成只证明库拥有的本地工作退出，不承诺服务端停止/退款。调用之后仍可建立新 generation；成功 result 可保留 idle pool。Drop Models/替换 provider 不取消已 owned calls；最后公共 owner/guard drop 触发资源关闭，后台 worker 不自行延长公共所有权。Drop 不冒充 await；注入共享只释放自己的 namespace/lease。此加强属于已确认取消/释放目标，不是声称 pi helper 已实现，也不新增通用 cleanup trait/registry。公开 owned stream/completion Future 支持 Send + 'static、宿主 Tokio；不要求 Stream Sync 或任意 executor。

替代是保留同步全局 helper，或完全隐藏 cleanup 依赖最后 Drop；前者无法给两个 SDK instance 提供作用域，后者不能给复用 Models 的宿主提供完成屏障。代价是清理需要维护资源 generation/活动 lease；复用现有 transport owner，不建立产品 SessionManager 依赖。AIR-034–036/046 和 V-28–30/39/42 固定边界；未来 public operation 新增同类 session 资源时再评估覆盖，不提前新建通用插件生命周期。现规范 §8.1 的职责/最小权能在正式协调中解释为独立 SDK 机制，本次仍不修改规范。

### D-36／Q36：按真实 adapter 默认重试，唯一 owner 落实 override

**Q36 已实际确认推荐**：对齐 adapter 默认并统一 override。Anthropic、OpenAI Chat/Responses、Azure、Google/Vertex 的 pi 外层 helper 默认 0 次额外；Codex HTTP 默认 0；Bedrock 继承锁定 AWS/Smithy standard 的 3 总 attempt 上限（最多 2 次额外，受错误分类与 retry quota 约束）。不把类型的默认 RetryConfig 或 SDK 安装存在当实际请求数证明。该证据是本地快照及先前 integrity 通过的 Node 依赖静态阅读，没有运行 HTTP 次数/取消测试。

每个 low-level call 只有一个重试 owner，公开 max_retries 计额外请求，显式 0 禁用、正数按 adapter 分类生效；不复制 pi Bedrock 忽略 maxRetries/maxRetryDelayMs 的缺口。旧 Agent 的低层 retry 不再叠加；产品 [model_call] 对应可选 SDK max_retries/max_retry_delay_ms，[recovery] 拥有 enabled/max_retries/base_delay_ms/max_delay_ms，legacy [retry] 明确迁移报错，不猜旧延迟字段的新含义；summary 使用同一高层公开恢复机制而非另一私有策略。timeout/cancel、冻结鉴权/绑定、transform 一次和 no post-content replay 保留。产品 D-24 的高层恢复仍是不同 logical calls 的策略，不能与 SDK 默认混为一层或隐形累计。精确错误分类、退避、server delay、override 及两层计数见 spec retry annex 和 V-05，静态退出由 retirement/V-20 检查。

替代是所有 SDK 默认 0，并明确偏离 Bedrock。用户选择保留真实默认；代价是 Bedrock 的可重试失败可能多发请求。Rust 有界状态/可取消计时器改进生命周期，不决定默认 retry 数。SDK 环境 source 覆盖及 shared profile 的已核事实与后续未执行的 Rust 依赖行为分开；未知 retry 模式/实验不能成为新 framework/flag 的理由。版本升级只有真实 retry 行为或调用需求变化时重新评估，不能由实施 Agent 任意更改默认。

独立继续复核收敛（D-35/36，技术推导）：首次 poll 在任何 auth/transform await 前登记资源 generation，cleanup 取消旧 matching call 含准备阶段，不能等待完再接入新代；caller token 只单向传入内部 call cancellation，cleanup 不反向取消外部共享父 token。高级 start_attempt 至多一次 acquisition、共用累计 sends/退避/quota；可重试 acquisition outcome 与唯一 public logical-call terminal 区分，handoff 后不再准入，未 dispatch drop 不收费，已 dispatch 无确证可重试失败则 UnknownEffect/封存调度但不取消 caller token。Bedrock standard quota 每 logical call 重新初始化，同 call 的高级 attempt 不重置。这是修复已选 owner/lifetime 契约的歧义，无新增产品策略或通用框架。详见 spec annex；V-05/29/42 验证。

### D-37／Q37：pi 的显式 telemetry 传播，以 Rust tracing 模块落实

**Q37 已实际确认“采用 pi 职责划分＋Rust tracing 模块”。** 证据 TC-01–09、TE-01–09、TO-01–21及[专项架构比较](telemetry-design.md)：pi-ai manifest 确实依赖 pi-telemetry，当前 src 为 type import＋request options 传播，不创建内置 AI spans；0.84 引入的中立 callback 契约仍有效，旧 Agent/harness schemas 在1.0移除，README integration 示例部分过时。10009/10085 未合并，自动关闭不等于已接受路线图。

首批在 opi-ai 内提供小 telemetry module 与显式可选 TelemetryContext，复用 tracing 的 backend-neutral 机制；不新增 opi-telemetry crate、不自带 exporter、不安装 global subscriber、不默认上传。所有已选 SDK chat native/simple/prepared、图片、分类及 deferred 操作传播同一 frozen caller parent，context 捕获匹配的 Span＋来源 Dispatch；缺省／disabled parent 是 no-op，不在异步 poll 中隐式回退 ambient。Agent／SDK 宿主拥有领域 spans/schema，当前阶段不要求恢复旧 pi schemas、不新增自动 SDK logical-call/attempt spans，也不承诺完整 tracing instrumentation 已交付。

技术落实不冒充新问题回答：公开 `TelemetryContext::from_span(parent: tracing::Span)` 从该 Span 获得匹配 Dispatch，disabled／无来源为 no-op；`TelemetryContext::noop()` 为显式 disabled，clone 保留同一 parent/dispatcher。in_scope(FnOnce) 同步执行一次，instrument(Future) 只在 caller poll/drop 恢复上下文，不创建 domain spans；noop helper 使用 disabled scoped dispatcher，业务照常执行、返回后恢复宿主 context。宿主可主动传 `Span::current()` 的结果，但库不自动捕获。库 options 接入语义对应 pi；不复制 pi-telemetry 的任意名称/属性 schema DSL 或通用 `startSpan` callback API，后者不是已选 public SDK 表面。tracing 的 native field/status/lifetime 与 pi callback settlement 不能混称等价；领域 owner 在自身范围解释 Result、关闭业务观测，backend span close 仍由 host native生命周期管理。

保留 DiagnosticSink、AgentEvents、EvidenceSink/IdentityAllocator、Session ack、脱敏错误；telemetry 是可丢失观测，不能取代 durable evidence health 或成为 dispatch/授权 gate。context/backend objects 不进入 serde、历史、模型目录、credential、deferred handle。host backend 负责被动、非阻塞、抑制 recording/export failure；任意不符合契约的 Subscriber panic/block、abort/OOM 不承诺被 tracing 自动隔离。业务 hooks 仍按原 typed failure，不与 passive telemetry 合并。

备选是自有中立 telemetry crate／trait 系统，或同时新增 SDK 标准 spans。前者增加公共生命周期、发布/依赖与生态适配；目前 tracing 加既有 Agent→AI 依赖足以承载已选契约。后者需要新的领域 schema与auth/排队/retry/body/consumer/drop 范围选择，未由当前 pi 代码或未合并提案授权。重新评估触发：真实不应依赖 LLM SDK 的独立 consumer、经过验证的 tracing 契约缺口，或明确的新 spans/backend 需求；不由实施 Agent 自动扩范围。

与 opi-spec §6.1/CTRL-001 的 explicit context／no-op／exporter外置一致；未来正式协调需解释通用 mechanism 位于opi-ai而 domain仍由Agent/宿主拥有，并保留AIR-046的独立SDK依赖限制。本次不改正式规范、AGENTS或runtime。AIR-054–056／V-46–48固定传播、Dispatch/生命周期和被动性／秘密／evidence隔离的验收；全部未实施、未执行。

## 原独立报告的交叉核查

[ai/telemetry 报告交叉核查](ai-telemetry-report-crosscheck.md)将原报告主张回到相同快照源码，再映射到已确认 D/AIR/V。原报告是非规范性静态证据导航，不是新的目标批准或运行通过证明；两份报告引用同一快照，也不构成两次独立实验。当前未发现要求重开已确认决策的实质冲突。compat fallback 与正常 ModelRuntime 路径、请求 transformHeaders 的归属需准确解释；Rust Result/owned partial/惰性执行和 native tracing 生命周期仍是明示差异，不能用“调用方式一致”掩盖。未覆盖的协议、历史、作者意图和性能问题继续由专项证据及未来验收承担。

## 进入候选 spec 的条件

当前候选用AIR-001–056表达owner、trigger、behavior/failure和验收；V-01–48全部是未来验证设计。Q22–31及修订Q31已明确回答，受影响首轮选择由D-19–37替代；双语稿和专项独立评审针对新目标。Q32–37均已实际确认；没有未回答的目标问题；正式文档协调、实现及其验收仍未进行，不构成产品准入。

本目录现阶段是调查与讨论材料，不将其登记成正式实施来源。完成研究文档检查不等于产品实施准入。

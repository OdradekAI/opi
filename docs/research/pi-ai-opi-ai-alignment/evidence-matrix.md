# 比较证据矩阵

> **后续重审：** 用户已将本轮基准明确为“pi 的 Rust 实现”，opi-spec 仅作可能过时的参考。本页源码事实保留；涉及原 Q1–Q21 的目标建议和确认状态须先读 [决策重审](decision-reassessment.md)。旧规范不能继续单独否决 pi 已落地能力。

> 非规范性研究；2026-10-03。每项是行为比较，不是实施清单。
> P 证据定位见 [pi 契约笔记](pi-contracts-notes.md)，O 证据见 [opi 独立审计](opi-audit-notes.md)，A 见 [综合审计](design-audit.md)，EVO 见 [设计演进](design-evolution.md)。这些笔记集中保存易变路径、行号和反证，本表不另造事实权威。

状态采用：语义等价／部分对齐／缺失／实现不同但语义等价／有意差异／实验能力／证据不足。一个主题可拆成多个状态；**有意差异**只指已有清晰设计边界，尚未确认的目标差异仍为建议。

规范参考：`docs/opi-spec.md` §3.2、PRIN-001/002/003、§8.1 与 INV-001/002记录原有设计和约束。D-18明确本研究以 pi 的 Rust 实现为目标，旧准入规则不否决已落地语义；未来正式文档需与已选目标协调。本表将源码事实、现行规范和新目标建议分开，源码与规范均不构成测试结果。

## M-01 身份、provider/API/wire 与模型选择

| pi 当前事实／意图 | opi 当前事实／规范 | 状态、影响与建议 |
|---|---|---|
| Provider ID 与 API 分开；Models 实例绑定 provider。model type+provider+id 为目录身份，replay 另看 API。多 operation 最近才合并。（P01、EVO-002/009） | ModelInfo/WireApi、registry provider:model、ApiMappedProvider 多 wire 已在 Copilot/custom 路径使用。effective registry model 与 adapter 自有 catalog 可能不一致。（O-001/002；INV-001/002） | **部分对齐**。保留显式路由职责，推荐修正有效模型绑定，不能仅对齐目录字段。opi 当前 chat-only 是已确认的新操作目标差距，不属于语言必然差异。高置信。 |

生产与测试：pi ModelRuntime/provider composer；opi Agent、compaction、harness、picker。opi 的 active override 测试不证明非启动 route 的 override 后 dispatch；[纯mock路由探针](route-probe.md)已重现库层wire不一致与追加ID晚失败；完整产品切换仍未运行。D-03。

## M-02 公共入口、全局状态与导入副作用

| pi | opi | 状态、影响与建议 |
|---|---|---|
| root 不注册完整 provider；compat 注册全局 API、保留旧消费者；Models 是正常 session 路径。Codex socket cache/session cleanup 仍有模块全局状态。（P01–03、07；EVO-003/010） | collection/registry/provider 为显式实例，无 pi 风格 import 注册；Config/Model/ProviderKind 旧公开壳未见仓库内生产消费。（O-003） | 实例所有权 **实现不同但语义部分等价**；`/compat` 全局入口为 **迁移机制**，不是目标；provider compat 的真实 wire 行为另见 M-09，不能一并淘汰。候选淘汰不连通旧壳，而非引入新的默认 singleton。高置信；未知外部 embedder 是迁移缺口。 |

测试边界：pi loader hook 检查模块可达性，本次未运行、不推断性能；Rust 使用公开 API 编译及真实消费者测试，不机械复制 JS 子路径。D-02/08。

## M-03 Request、消息、transcript、system/tools 与重放

| pi | opi | 状态、影响与建议 |
|---|---|---|
| SystemMessage 存有序 prompt sections/tools 增删；Agent 每轮声明差异；adapter 支持原位或 collapse。collapse 将新指令移至历史头部，不能声称完整等价。（P04/05/12；EVO-007/008） | Request 每次给当前 system/tools + messages，Message 无 system/tool-change 事件；Anthropic 有同route签名保留与跨route剥离。其他历史转换分散，非法tool JSON有 `{}` fallback。（O-010） | 当前快照请求可用，历史时间线 **缺失**；replay **部分对齐**。是否新增 transcript 涉及 Agent/session/compaction 与持久化，不能当作仅库类型更新。优先保证已支持历史输入的确定转换；Q5确认完整 transcript；Q22已替代Q15的全量无结构快照、固定整个运行及默认strict：named sections、动态trusted revision、native-or-collapse和active/wire-visible分离。见RA-03/04、D-19。 |

消费者与验证：pi Agent 和产品历史已有消费；opi Agent 逐轮构造 Request，签名 roundtrip tests 覆盖一部分。RT-04/05与 RA-05/08已补核结果缺失恢复、关联与参数 normalization 的正式实现及消费者。未来差分矩阵尚未运行；Q23已确认恢复与normalization，Q25确认可信hook不复验并最终授权；不能以旧严格偏好否决这些行为。

## M-04 流事件、增量、排序、终止与 EOF

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 公共 start/updates/done/error；partial 是共享累计引用；EventStream terminal 后忽略 push，但 end() 无result可留下 pending；Responses 严格终止，Chat 可配置 EOF推断。（P03/06） | Rust 事件携带克隆 message；Done/Error 与 typed Err 并存。Agent/drain拒绝异常EOF，但 prepared wrapper不fuse；部分SSE使用有损逐chunk UTF-8。（A-02/04） | **部分对齐**。保留语义终止与typed失败；推荐统一 prepared terminal守卫、无损framing，避免复制 pi 弱EOF分支。A-02/04有本次运行证据。 |

验收：完整事件序列、block final与最终message一致、非法终态、不完整tool、逐字节分块；差分不能将错误、丢失字符或terminal归一化掉。D-04。

## M-05 取消、超时、背压、资源与并发

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 通用FIFO无界；通用iterator退出未绑定abort；SDK/WS有局部timeout和清理；Models取消等待不等于所有外部副作用取消。（P03/07/10） | mpsc容量64；不同adapter在背压取消、receiver drop、buffer优先级上保障不同。prepared active限制一个attempt，HTTP timeout可None。（A-03/04/06） | 有界缓冲是值得保留的 **有意差异**；生命周期 **部分对齐且需修正**。不引入无界FIFO、不新增假想transport框架。Q9已确认terminal/cancel竞态与现有timeout层级，见D-04。 |

真实运行：Gemini idle drop未在500ms观察窗口关闭连接。只证该场景，不声称永久泄漏或所有adapter失败。70测试含既有cancel断言，未覆盖所有饱和队列／drop场景。

## M-06 Logical call、attempt、retry、fallback 与错误

| pi | opi | 状态、影响与建议 |
|---|---|---|
| HTTP helper默认0重试，绕过SDK不可取消退避；Codex在首次parsed输出前有受控WS→SSE恢复；不等于跨provider fallback。（P08/13） | collection一次固定request/auth/route，Agent拥有重试次数及退避；任何已交付事件后禁止自动重试；typed错误与auth terminal防止隐式换身份。（A-05、O-006） | 仅adapter transport层的输出前恢复有局部相似；pi另有assistant-call utility和产品整回合重试。同一低层流禁止隐式重播应保留；输出后高层失败回合恢复是 **缺失／当前策略差异**，不是 Rust 要求。pi产品恢复、assistant-call utility与操作 retry 已由 RA-07/RE-05补核；Q27已确认分层对齐，精确默认与身份边界见D-24及AIR-026/039。高置信。 |

2026-10-04 补核（UX-11–16）：共同 helper 默认0不是所有adapter默认；Bedrock继承锁定Smithy standard最多2额外、pi忽略其public override，Codex宽catch可复活400/401。Q36/D-36已确认真实默认＋唯一owner＋实际override，typed终止与可取消退避明确纠错；source事实/目标/未跑fixture分开。SDK管理分别是配置检查/解析/目录刷新/登录/本地删除；Codex auto是SDK默认，公开cleanup不是产品专属。Rust资源隔离/owned wait是有意加强；AWS ambient/adaptive retry knobs是已标范围差距，非已证明等价。详见 sdk-consumer-contract，当前 contract以 AIR/V 为准。

测试：prepared-call、retry-after-partial、credential needed/revoked/fallback负例；本次只实际运行前述70测试包含的子集。需未来观测 call/attempt不同身份，不能用SDK header证明attempt数。D-09。

## M-07 API key、OAuth、存储、刷新和解析时机

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 显式key优先；stored identity失败阻止ambient回退；per-provider store.modify保护refresh；默认提前5分钟、15秒超时为其具体契约。（P09、EVO-004） | AuthResolver/Store/OAuthProvider真实seam；产品刷新持mutation lock并重读、30秒默认refresh timeout；logical call冻结。AWS discovery/profile/shell helper仍在opi-ai。（O-006/007/012） | 解析/刷新 **部分对齐**；保留typed scheme与失败不换身份。AWS默认链是独立 SDK 能力；推荐保留可复用 resolver，产品配置启用与权限策略，Q24已确认完整链方向；精确锁定Node依赖源码补证见SDK annex A–D。原产品独占迁移已撤回（D-21）。既有README“无environment IO”部分过宽，不可当事实。 |

不能复制pi时间常量或为了提高并发盲拆锁。D-07原迁移方向已由D-21替代；D-09的冻结鉴权/不隐式换身份继续保留，跨层 retry 需区分高层恢复。只读查源码和mock断言；本次未读真实凭据、未执行AWS链或OAuth。

## M-08 模型目录、缓存、刷新与过期结果

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 静态离线baseline+动态overlay；Models显式refresh/generation/publication，产品remote catalog拥有URL/ETag策略。（P10、EVO-005/006） | registry overrides/dynamic/builtin；collection.refresh需要&mut、顺序collect后原子替换，失败保留旧表；所有当前内建静态、未见生产刷新触发。（O-005/002） | 静态目录已用；动态为 **库substrate**，不是已接默认产品。现有独占刷新 **实现不同**，能保护其当前互斥路径；加入异步后台产品刷新后须证明取消及过期结果不发布，不能用现有 &mut 排除新增并发情境。pi生产刷新已核实，不再是无消费者的设想。 |

本次通过 atomic rollback、replace、None清除等已有测试；不证明动态数据真实wire一致。远端缓存／后台刷新是 pi 已交付产品行为，推荐恢复为对齐目标并明确库／产品 ownership；Q26已确认目标及接线，见D-23、RA-06/RE-01和V-27/28。

## M-09 Tool schema、校验、partial JSON、strict 与能力

| pi | opi | 状态、影响与建议 |
|---|---|---|
| partial parse供展示；执行前validator含coercion；strict转换支持有限schema subset并有prefer/require。（P11/16） | ToolCall保留raw String；Agent现有deny-only hook观察raw，随后做schema/权限校验；Chat strict仅发字段；部分compat flags未读。（O-004/008） | 校验边界 **部分对齐**；pi normalization/coercion 是维护中的执行语义，推荐对齐而非归因于语言（RA-08）。strict参数存在不等于任意schema有效。保留Agent authority；未接线 flag 必须按对应真实语义补齐或合并，不能以 opi 当前无 wire 作用证明 pi 能力无价值（RA-01）。 |

未来验收在工具副作用前断言invalid args/length失败，wire fixture另验证strict。新增partial preview需真实UI消费；D-08/12。

## M-10 Thinking、签名、effort 与历史转换

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 同provider/api/model保留签名；跨route转换；effort表与clamp；tag后Bedrock继续修订回放。（P12、EVO-012） | Anthropic preserve/strip有HTTP-storage-continuation用例；thinking level map、不支持级别preflight拒绝；不同wire转换不统一。（O-010、A-05） | **部分对齐**。保留opaque与可显示文本区分；unsupported level拒绝与 pi 高层 clamp 是 **可观察策略差异**，不是 Rust 要求；见 RA-11。完整 opaque continuation 是已落地目标差距，推荐补齐同路由 parse→persist→replay，不能以 opi 缺消费者循环否决。Q26/Q29已确认完整continuation与高层等级选择，native/exact仍显式验证。 |

未知：全部签名缓存/远端接受性，不能据持久化serde通过就声称live replay可靠。验收保留requested-model alias与reported-model差异，不归一化来源身份。

## M-11 Usage、成本、cache与缺失值

| pi | opi | 状态、影响与建议 |
|---|---|---|
| 父桶number常初值0；reasoning注释说缺失≠0，但Responses实现会置0；tier/1h/service tier估价。（P12） | reported区分整次缺报；optional child、parent不重复计费、累计u64；混合缺失child总和只代表已报告部分；ModelPricing/tier供产品用。（O-011、A-06） | opi未知表示值得 **保留的有意差异**；两者都非完整账单系统。不要倒退成全零，也不在无消费证据下泛化财务framework。 |

测试设计：absent/zero、missing child、tier等值/越界、subset不双加、跨u32累计；本次未运行usage_cost。未知费用不得当可靠限额，D-10。

## M-12 图片、分类与 deferred 能力

| 能力 | pi／opi事实 | 状态与选择 |
|---|---|---|
| 用户输入图片 | pi支持转译与不支持placeholder；opi有native用户图fixture | **部分对齐**；应按URL/base64/bytes与wire分别看，不按variant判断。 |
| 工具结果图片 | pi转换链见P12；opi多数wire只发MIME占位，但vision preflight计入图片 | **部分实现**（O-009）；Q7已确认真实发送或提前拒绝，D-06。 |
| 图片生成／classification | pi统一operation且有codemode消费；opi无对应Provider操作 | **当前缺失；Q10/Q13确定归属/首批adapter；Q27/Q28替代旧重试/预算及接受集**（P14/EVO-009）；与工具图片分开。 |
| Deferred response | pi公共handle、faux实现、durable消费；未找到内建远端adapter | **公共生命周期缺失且Q26纳入；内建远端 adapter 证据不足并明确不在本批**（P15、RE-06）；已有 durable fetch/cancel/poll 消费，不能将整个能力称为提案。 |
| Deferred tool loading | pi Anthropic/Responses已转译；不同于异步推理或工具执行 | **已发布可选产品能力，opi缺失且Q26纳入渐进声明/tool_search接线**（RE-03）；CLI tool_search默认inactive，MCP deferred配置实际激活。不与异步推理合成统一框架。 |

D-01/06/11分别决定范围。opi未找到默认内置工具图片producer，SDK/extension公共ToolResult可供给图像；实验虚拟路由不能当默认模型选择策略。

## M-13 HTTP、SSE、WebSocket、代理与降级

| pi | opi | 状态、影响与建议 |
|---|---|---|
| SDK+原生fetch；CodexWS连接cache和conditional continuation；输出前才SSEfallback；proxy覆盖依运行时/adapter。（P07/13） | reqwest共享连接池，HTTP/SSE，Bedrock二进制事件；Codex为SSE；显式/env proxy。没有WS实现不能反推整体不可用。（A-02/03；crate HTTP源码） | 现有HTTP **实现不同但部分语义等价**；WS **缺失、Q26已确认目标与产品接线**；pi产品 auto 默认、连接缓存、续传和降级已核验（RE-02）。修正 framing/cleanup 是依赖，尚无性能测量不构成排除行为对齐的理由。 |

live吞吐、首token、连接复用寿命均未测。差分只验证共同支持传输的事件语义；future fallback须不重复已交付输出，D-04/11。

## M-14 日志、脱敏、诊断与 telemetry

| pi | opi | 状态、影响与建议 |
|---|---|---|
| ai manifest依赖pi-telemetry；src为type import＋所有operation options传播。通用noop/memory/callback/schema/conformance已实现，但builtin AI无自动span。0.84旧Agent schemas在1.0移除，README integration示例部分过时；exporter/Agent spans PR未合并。诊断helper截断不是脱敏、功能observer能改变执行。（TC/TE、P17） | tracing已锁定但AI无显式context；普通warn/debug日志、DiagnosticSink/AgentEvents/EvidenceSink各有真实不同消费者。RPC trace是evidence，不是tracing导出；verbose接线有文档缺口。保留typed/redacted错误、provenance和durable IDs。（TO/A-05/A-08） | 显式context **缺失→Q37确认补齐**；Rust tracing物理实现 **有意差异**，不宣称generic callback/schema API等价；保密/evidence责任保留。仅传递不算自动instrumentation，旧实验/proposals不进入target。 |

没有发现实际泄漏事件，不夸大为安全事故。前稿“无需新crate”缺少充分边界研究，本轮重新经[三方案比较](telemetry-design.md)选择opi-ai小模块（D-37），依据是tracing生态、独立SDK和既有Agent→AI依赖，不是pi包名或旧准入限制。propagation的caller parent与来源Dispatch冻结、no-op、host backend和serialization排除由AIR-054–056/V-46–48验证；memory/reference不假称生产exporter，任意subscriber panic/block不受无条件隔离。安装analytics及attribution不同路径见TC-10/TE-10，不因对齐本库而启用上传。

## M-15 测试支持、公共接口一致性与真实产品接入

| pi | opi | 状态、影响与建议 |
|---|---|---|
| faux覆盖abort/deferred/frames；普通vitest套件有按环境key启用live测试。（P18） | MockProvider、wiremock、生命周期/fixture、Agent/product消费者；类型存在型测试并不覆盖wire。（O-008/009；A-02–04） | 公共mock seam **实现不同但用途相近**。保留真实行为边界；补跨adapter不变量，不以测试数量比较质量。 |

本次结果：opi70现有测试通过；四组隔离probe特征观察成功；pi测试全部未执行。没有安装或运行pi整包test。拟新增/更新/删除测试必须与后续确认需求映射，见已选目标的验收设计，不把文档检查当runtime conformance。

## 结论置信度和缺口

高置信的已运行发现集中在A-02/03（Gemini场景）/04；其他高置信主要指源码分支和consumer追踪，不等于live完整性。O-002已有纯mock反例，完整产品交互未运行。全供应商协议、公网目录服务、外部embedder兼容、性能和费用准确性未全面验证。本研究故意保留这些缺口；不靠增加不相干全量测试或live调用来掩盖。

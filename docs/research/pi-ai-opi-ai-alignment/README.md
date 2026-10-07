# pi-ai / opi-ai 设计对齐研究

> **非规范性研究与 DRAFT 候选 spec**；初始研究/访问日期2026-10-03；SDK 调用方及 telemetry 复审2026-10-04。本轮按用户明确的“opi 是 pi 的 Rust 实现”定位判断；opi-spec 是可能过时的参考，不再否决已落地 pi 语义。
> Q22–37（含修订Q31）均已实际回答，当前目标见D-19–37。本文不修改正式规范，不构成实施准入或ready-for-agent；未修改运行时代码、测试、依赖、配置或治理文件。

> 2026-10-05 证据路径整理：独立 Pi 报告已更新至 [1.0.2 基准（历史恢复）](../pi-1.0.4/evidence/pi-1.0.2/README.md)，旧报告目录已移除。本目录既有研究仍保留各自的 1.0.0 基线、行号和执行记录；没有随导航更新而重新核验为 1.0.2。旧清单、审读记录及不同修订的全文恢复位置见 [历史证据](../pi-1.0.4/evidence/pi-1.0.0/README.md)。旧源码快照当前不在工作区，恢复历史报告不能恢复其源码链接。

> 2026-10-07 当前阅读入口：独立 Pi 报告已更新至 [1.0.4 基准](../pi-1.0.4/README.md)，1.0.2报告的精确恢复记录保存在新版的 [历史证据](../pi-1.0.4/evidence/pi-1.0.2/README.md)。本目录既有比较、候选契约和执行记录未随导航更新而重新核验为1.0.4；上面的日期记录继续保留原义，链接仅迁移到对应历史版本的恢复说明。

## 阅读顺序与完成条件

1. 基线 → verify: 本页与baseline的revision、快照内容身份、共享改动归属可复核。
2. 比较/演进 → verify: 源码事实、设计意图、发布状态、运行验证和推断分开，有一手定位。
3. 审计/决策 → verify: 每项处置有消费者、反证、替代、代价和确认来源；历史回答的替代关系可追踪。
4. 候选契约 → verify: 56条AIR有owner、trigger、行为/失败、证据/D/V；删除项有替代与退出证明。
5. 独立复核 → verify: 当前双语稿与48组V一致，发现修正后执行文档检查，不冒充运行时验收。

| 目的 | 材料 |
|---|---|
| 当前目标及完整双语契约 | [英文spec](spec-draft.md)、[中文spec](spec-draft.zh.md) |
| 人类选择与旧回答替代 | [decisions](decisions.md)、[Q1–21重审](decision-reassessment.md) |
| 15主题比较及opi审计 | [evidence matrix](evidence-matrix.md)、[design audit](design-audit.md) |
| 文档有效性、历史、tag后窗口 | [design evolution](design-evolution.md) |
| transcript/工具/strict/grammar细则 | [transcript target](transcript-target-proposal.md)、[迁移](transcript-migration.md)、[replay](replay-contracts.md) |
| compat/AWS/操作/扩展精确证据 | [SDK target与附录A–D](sdk-target-proposal.md)、[官方协议补核](provider-contract-addendum.md) |
| 目录/WS/thinking/恢复/deferred接线 | [evolution target](evolution-target-proposal.md) |
| 后续机械验收与测试处置 | [acceptance matrix](acceptance-matrix.md) |
| 当前 SDK 评审及关闭依据 | [SDK spec 复审](sdk-spec-review.md)；[第二轮评审](spec-review-round-2.md)为历史记录 |
| 开发者如何调用、执行与收取结果 | [SDK consumer](sdk-consumer-contract.md)、[pi caller](sdk-pi-caller-review.md)、[opi caller](sdk-opi-caller-review.md) |
| 三条 SDK OAuth、provider ID/wire 与迁移 | [SDK auth/provider migration](sdk-auth-provider-migration.md) |
| Telemetry 包的职责、作者意图与 Rust 方案 | [架构比较](telemetry-design.md)、[pi 契约](telemetry-pi-contracts.md)、[历史与官方讨论](telemetry-evolution.md)、[opi/Rust 审计](telemetry-opi-audit.md) |
| 本轮 telemetry 独立复核与最终检查 | [Telemetry spec 复审](telemetry-spec-review.md) |
| 用原 ai/telemetry 独立报告核查已确认决策 | [报告交叉核查](ai-telemetry-report-crosscheck.md)：支持、表述纠正、有意差异及尚待运行验证 |
| 基线/一手源码导航 | [baseline](baseline.json)、[pi契约笔记](pi-contracts-notes.md)、[opi审计笔记](opi-audit-notes.md) |
| 重审专项证据 | [transcript](reassessment-transcript-notes.md)、[SDK](reassessment-sdk-notes.md)、[evolution](reassessment-evolution-notes.md) |
| 已实际运行反例 | [流probe](offline-probes.md)、[route probe](route-probe.md) |
| 首轮历史材料 | [operations](operations-design.md)、[placement](placement-options.md)、[证据复核](review-notes.md)、[spec设计复核](spec-review.md)、[spec证据复核](spec-evidence-review.md) |

首轮32 AIR/26 V、第二轮44 AIR/38 V及 SDK 轮53 AIR/45 V评审保存研究历史，不能当作当前56 AIR/48 V稿的检查证明。重审中的旧“待确认/暂停”文字也不是当前前沿；D-19–37和当前spec表述选择。研究笔记保存事实导航，spec集中表达目标，验收矩阵集中表达证明方式，不建立另一套权威或进度账本。

## 最重要的比较结论

pi近年的变化包含逻辑革新：全局兼容入口向实例Models收敛；旧ImagesModels并入多operation集合；instructions/tools从当前快照变成chronology；真实compat、参数normalization、可信hook、远端目录、WS和恢复已在产品使用。这些不能因opi缺消费者或旧准入条款而排除。当前公开形状近期仍在演进，故对齐可观察语义并隔离wire，而非移植TypeScript文件/符号。

已经运行重现的opi缺口包括：有效目录wire与mapped dispatch不一致；SSE跨chunk UTF-8损坏；prepared terminal后仍交付事件；Gemini idle stream drop在屏障后501ms观察窗口未关闭。它们支持唯一binding、无损framing、吸收终态和producer资源清理要求，不证明所有provider失败或永久泄漏。

**保留**：显式实例、owned调用快照、容量64队列、单active attempt、typed auth/error、错误脱敏、usage未知与零分离、最终工具授权、有来源的旧会话迁移。**修正/替换**：adapter第二次选模、独立下游system/tools字段、旧snapshot-only执行、run-wide固定工具、缺结果一律拒绝、过宽的无恢复策略，以及断裂的compat/thinking/wire链。**淘汰**：不连通Config/ProviderConfig/Error/Result、Model/ProviderKind壳和无作用的重复静态effort；ApiKind及其持久化消费者保留。真实supports/bridge行为先补齐、等价合并并迁移，不能按“无读点”直接删除。

AWS完整凭据来源属于独立SDK可用机制，产品选择、注入及配置，撤回产品独占迁移。图片/classifier纳入opi-ai typed SDK；首批仅OpenRouter Chat-wire图片与TypeSafe/OpenRouter System One，不默认添加terminal操作工具。远端目录、Codex auto WS、完整opaque continuation、progressive declarations/tool_search及高层恢复纳入真实产品接线；公开deferred生命周期有可重开离线consumer，但不虚构未核验remote adapter。

pi-ai 明确依赖 pi-telemetry；前稿只记录 context 透传，漏了包、跨包边界与作者意图的完整审计，本轮已补。已发布中立 context／callback／no-op／conformance 与已删除实验 Agent schemas 必须分开。Q37 选择显式传播，以 opi-ai 内 tracing module 承载，Agent／SDK 宿主拥有领域 spans，宿主注入 backend；不自动创建 SDK spans、独立 telemetry crate 或上传。

重要有意差异已明示：有界Rust背压/owned事件；更明确的异常EOF；未知usage；selected/scoped AWS pair不拼接ambient；payload transform不能改身份；无账户证据的credential实变停止自动恢复；image timeout覆盖body；目录typed失效不依赖bundle generation。它们有具体正确性/身份依据，不能笼统称“Rust必须如此”。

## 从独立 SDK 调用方看当前目标

仅依赖 opi-ai 的开发者使用同一 Models 实例：注册 provider、选模或传入完整 ModelInfo、提供 Context，再用 stream/complete 或 simple 对应方法。高级 prepare 保留同一内部机制；不会要求调用方先启动 Agent、session、产品配置或工具授权器。ModelInfo 明确携带 provider_id；自定义完整模型不会被目录同名项替换。

| 使用面 | 已固定目标 | 有意差异或范围 |
|---|---|---|
| 调用与执行 | native/simple 四入口；一次 owned 捕获、规范化、鉴权 binding、adapter、统一 collector | Rust Future/Stream 由宿主 Tokio 驱动；创建 handle 不执行外部 I/O |
| 最终结果 | complete、直接 stream.result、迭代后 result 同一 Result；失败保留完整消息/usage/typed cause | 不复制 pi 错误仅作消息或共享 live partial；有界队列由 result 主动 drain |
| 工具循环 | 声明、校验与消息重放可独立使用，宿主显式执行/追加 | SDK 不自动执行工具、写历史或开启产品恢复 |
| 鉴权与默认身份 | 三条 OAuth 完整 SDK flow；canonical provider IDs、OpenAI/Azure 默认真实 Responses | keychain/TUI 留产品；旧 key/config 显式冲突处理和迁移，history 不改写 |
| 预算 | pi canonical anchor、UTF-16/message 粒度估计及 adapter-specific simple mapping | unknown 不补0；native 不套全局 clamp；pi numeric thinking 的零回填缺口明确修正 |
| 管理与清理 | 配置检查、来源解析、目录刷新、登录/本地删除分别返回；公开实例内 await cleanup | 构造不读取env/store；最后owner Drop与await屏障不同；不复制全局同步cleanup |
| Transport/retry | SDK Codex默认Auto；helper/Codex默认0额外、Bedrock standard2，显式override唯一owner | 省略/显式Auto统一；不叠加Agent低层retry、不复活400/401、不继承未选AWS ambient/adaptive实验 |
| Telemetry | 显式 options.telemetry_context；from_span 捕获 parent＋来源 Dispatch，noop／同步与 Future helpers | 所选操作同一父 context；不自动 SDK spans、全局 subscriber 或 exporter；领域 instrumentation 由 Agent／SDK 宿主拥有 |

独立嵌入验收使用 workspace 外仅依赖 opi-ai 及正常 tracing/runtime 依赖的 consumer，既检查公共调用，也跨真实 adapter 与本地伪传输；只编译仓库内部测试不足以宣称可集成。V-39–48 全是未来验收设计，接口尚未实施。

## pi 的设计地位与演进影响

| 地位 | 判断与选择 |
|---|---|
| 稳定核心 | provider/model/operation/wire区分，实例调用、消息/终态、真实工具执行边界；新transcript与operation已发布有消费者，API形状近期整合不等于长期冻结。 |
| 迁移机制 | `/compat`全局注册、产品ModelRegistry facade、缺type视chat保留旧消费者；区别于必须实现的provider compat行为，不新增opi singleton。 |
| 实验/提案 | 虚拟分类路由experimental；完整Responses Lite未由RFC证明交付。延后并给重新评估触发。公开deferred/faux/durable consumer已实现，远端adapter则证据不足。 |
| 历史负担 | ImagesModels旧并行设施已收敛；RFC0019被0038替代；compat退出未兑现提醒写明确替代与退出条件，但不据此称所有compat死代码。 |
| 实现偶然/已知缺口 | JS共享partial/无界FIFO、具体module缓存、Nodeheaders后timeout、dummy SigV4/ambient bearer交互；不机械复制。 |
| Telemetry 的混合地位 | 中立显式 context 是保留核心；旧 Agent/harness schemas 在1.0删除，README 的相应例子部分过时；新 exporter／Agent spans PR未合并，不能升级为已接受规划。 |

本地未找到统一pi-ai设计文档或确定路线图；通过README/CHANGELOG/注释、RFC、提交和维护者讨论重建演进。tag后Anthropic inline declarations与Bedrock修复支持“稳定chronology、局部wire适配”。已合并、已发布、提案及研究推断分别记录，不承诺时间表。官方后续窗口固定于design-evolution所记main，不声称覆盖本日全部新提交。

## 可信基线与研究范围

| 对象 | 内容身份及限制 |
|---|---|
| opi 初始基线 | HEAD `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`，commit `2026-10-03T17:29:58+08:00`；workspace0.8.2、edition2024、MSRV1.97。未提交行为不当作已发布。 |
| 初始及 SDK 轮共享状态 | staged为空；21个既有tracked unstaged及一个untracked研究文件见baseline，包含opi-eval/CHANGELOG/旧manifest。SDK 轮另见untracked `crates/opi-coding-agent/crates/`；该轮记录保留，不能当本轮最新清单。 |
| Telemetry 继续基线 | HEAD `0fa995de504bcb9854e8ee841d54d99579da1bd4`；由共享工作区其他工作推进，本任务无 Git 写入。staged为空；旧manifest modified、整个研究目录及无关 `pi-evals-opi-eval-comparison.zh.md` untracked，均保留。 |
| pi本地声明 | `@earendil-works/pi-ai`1.0.0，快照无.git；不以目录名证明tag。 |
| manifest适用性 | 按其排除规则重算1,848文件SHA-256：新增/缺失/差异均0；树摘要 `cc37a8fcc2c2dd97a3953c01a1e85f951984a85a0f5252111627163ce4c71173`；ai378文件。manifest本身carried-in未提交。 |
| 官方身份 | [v1.0.0](https://github.com/earendil-works/pi/releases/tag/v1.0.0)指向`a13d35a742c6ef8462812a28fbe1d8c8b7431c32`，ai378 Git blobs全匹配；不证明全repo/npm执行产物同一。tag后进展另记。 |
| Telemetry 后续补核 | telemetry 包12/12文件官方v1.0.0 blobs匹配；v1.0.1及main `83692682f095528f8b71652ddacff7075e36e893`固定窗口见TE，telemetry仅版本/CHANGELOG变更，不声称覆盖此后全部提交。 |
| 精确依赖补证 | SDK annex记录OpenAI Node7.19.0及AWS完整依赖。16个官方npm tarballs全部lock SHA-512匹配后仅读源码，附URL/版本/hash；未安装/import/执行包。 |

主范围为opi-ai/pi-ai；跨包只追到必要Agent、coding harness、auth/model装配、session/compaction、工具和durable消费者。枚举不等于逐行全审计；没找到消费者不等于能力不存在。原`../pi-1.0.0/`报告仅导航，矛盾在本新目录修正，不污染其独立研究。

证据分为源码事实、设计意图、实际运行、研究判断、人类明确确认和未知。测试源码仅是断言意图；文档与导出存在都不是运行证明。现行规范条款单列以便后续协调；不默改旧文本，也不以它隐去pi创新。

## 工作流适配

已实际读取并应用ask-matt、research、codebase-design、grill-with-docs及其grilling/domain-modeling、to-spec、writing-for-agents；联网一手材料用agent-reach路由，双语同步用baoyu-translate的项目技术约定，完成检查遵循verification-before-completion。独立研究及三轮专项评审有文件边界，依赖修改由主Agent顺序整合。

基线缺少`docs/agents/issue-tracker.md`及`docs/adr/`。按本次指令保留本地decisions，未发布外部issue、安装/setup、建立tracker、GLOSSARY、ADR或实施账本。CONTEXT术语继续用于User Policy/Package Trust/Capability Permission；未来正式Mission/GOAL/PRIN/§8及Runtime Input Binding需要协调已选方向，不以逐项小例外代替定位。

中文新研究没有既有英文counterpart，无需改旧双语资料；spec提供同步英语/中文。授权只覆盖本目录文档、只读检查及隔离离线验证。本次没有真实凭据、paid/live模型、生产服务、提交/推送或产品实施。

## 已实际执行的验证

| 命令/操作 | 结果与证明边界 |
|---|---|
| Git HEAD/staged/unstaged/untracked及manifest SHA-256读取 | baseline记录输入；末次共享状态复核见SDK评审，不当作已发布能力。 |
| Python标准库完整manifest重算、官方Git tree blob比较 | 1,848/1,848 manifest匹配；ai378官方blob匹配，范围如上。 |
| `python scripts/opi-cargo-cache.py status` | exit0，已有外部cache保留；无clean/per-session target。 |
| `cargo test --offline --locked -p opi-ai --test gemini_lifecycle --test provider_lifecycle --test provider_collection` | 首轮exit0：4+8+58=70pass、0failed，`E:/opi/cargo-targets/windows-direct`。仅现有mock生命周期/准备/目录断言，不证明新目标。 |
| `rustc --edition=2024`隔离流/route probes及运行 | 编译/运行exit0；完整命令/hash/stdout见两probe-result.json。证当前反例，不是修复通过；socket probe经独立复核加drop屏障重测。 |
| 官方gh/npm精确依赖读取 | SDK annex A–D记录命令与16份integrity通过；静态事实，未执行AWS链/metadata/OIDC或Node测试。 |

未运行 pi 测试、live、依赖安装、生成脚本、性能试验或 workspace 全量 runtime gate。本轮修改仅文档，未重跑此前70测试；当前全部48组V是验收设计。SDK 轮独立源码/接口、管理/资源/retry与双语复核记录集中在 [SDK spec 复审](sdk-spec-review.md)。不能借首轮70测试宣称新 runtime 已验证。该轮检查的具体统计/hash保留为历史：36文件no-index/UTF-8/LF、565本地链接、3 JSON、53 AIR/45 V和358行英中追踪/795数字核对通过；它不是 telemetry 新稿的检查结果。

Telemetry 轮三项独立复核、56 AIR/48 V双语稿、组合文档检查、共享状态和hash的结果记在 [Telemetry spec 复审](telemetry-spec-review.md)。随后对原 ai/telemetry 报告的交叉核查及本轮最终检查见 [报告交叉核查](ai-telemetry-report-crosscheck.md)；它补充事实解释与导航，不新增目标决策。全仓库diff-check的旧manifest尾空白与本研究文件检查分别报告，不用普通diff漏掉untracked文件冒充通过。

第二轮曾验证31个任务文件、539个本地链接及44 AIR/38 V，记录保留在其评审中，仅适用于当时稿。全仓库 `git diff --check` 的 carried-in manifest 尾空白问题由当前检查再次区分；本研究未跟踪文件另外逐文件 no-index 检查，不用普通 diff 的遗漏冒充通过。

## 剩余边界及测试影响

当前目标问题全部已回答。候选仍DRAFT、非规范性：正式文档协调/实施准入尚未进行，runtime实现和48组验收设计均未执行。完整MCP/codemode、真实remote deferred、其余operation adapters、新OpenAI ChatGPT OAuth/Mistral原生Conversations，以及动态header callback/raw provider-event observer/compact frames便利层及AWS adaptive/ambient retry knobs明确不在本批；telemetry 自动 SDK spans、全套 Agent schema families和exporter也未纳入。重新评估需已记录的协议/调用方/权限证据及显式范围修订。未知live接受度、性能/计费、外部embedder兼容、future Rust依赖行为不冒充已知事实，也不让实施Agent自行扩大目标。

本次测试影响：**none**（仓库测试add/update/delete均0）；首轮70个既有测试为**retain**运行记录。未来实施测试的add/update/delete/retain逐项见acceptance matrix和retirement表。

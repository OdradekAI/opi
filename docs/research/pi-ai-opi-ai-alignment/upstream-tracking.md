# Pi AI 上游版本与规划跟进记录

> 非规范性证据索引。目标：仅 `opi-ai` 对照选定 Pi 版本的公开能力和可观察语义，使用 Rust 惯用设计并明确差异。记录来源、变化、审查和证明边界；不承担实施准入、任务/阶段进度或测试完成账本。目标要求仍在双语候选，综合验收仍在 [acceptance matrix](acceptance-matrix.md)。

> 当前候选推进到本地Pi1.3.0，完整变化见[升级复核](pi-1.3.0-upgrade-review.md)。旧行保持原输入；本轮RFC/PR线上状态未刷新，不能将旧观察表当成截至1.3.0的上游最新状态。

## 版本审查记录

| 研究日期 / 比较 | 固定输入与结论 | 可复用入口 |
|---|---|---|
| 2026-10-03–04；Pi 1.0.0 | 当时选定能力子集和 D/Q 确认；56 AIR／48 V，测试执行与未来设计分开。不因此覆盖新版本 | [历史基线](baseline.json)、[决策历史](decisions.md)、[原研究导航](README.md) |
| 2026-10-16；Pi 1.0.0 → 1.2.0 | 当前 Opi HEAD `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8`；Pi tree `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10`。AI 378→394，新增18/变化79/删除2；406项 AI＋telemetry 摘要匹配。候选暂不核准，R1–R10 为修订依据 | [1.2.0 审查](pi-1.2.0-review.md)、[输入证据](pi-1.2.0-review-baseline.json)、[独立 Pi 报告](../pi-1.2.0/README.md) |
| 2026-10-16；同一 1.2.0 的五问补核 | 分清三类 Context；telemetry 主要设计仍有效、未实施。刷新官方规划状态；目录版本选择已有本地代码。当前调用体验未等价、没有差分 runner。补测试处置和持续记录；无 runtime tests/live | [五问补核](pi-1.2.0-followup.md)、[输入/清单/网页摘要](pi-1.2.0-followup-evidence.json)、[测试处置](acceptance-matrix.md#2026-10-16-测试迁移补核) |
| 2026-10-16；两轮核查后的候选修订 | 文档扩展为 67 AIR（58 SDK＋9 宿主）、59 V（55 SDK＋4 宿主）；完整公开能力及 RDF-01–20、测试迁移和版本跟进；不改变运行时。独立复审结论以链接报告为准 | [当前双语稿](spec-draft.zh.md)、[能力附件](pi-1.2.0-sdk-contracts.md)、[Rust 差异](pi-1.2.0-rust-differences.md)、[修订输入](pi-1.2.0-revision-baseline.json)、[复审](pi-1.2.0-revision-review.md) |
| 2026-10-16；同一1.2.0的Rust工程修订 | Opi输入6b81658，五项合同补齐与精确HTTP/WS/AWS/ADC/Schema后端选择；没有升级Pi/Cargo或实施，不豁免正式准入。审查/结果按本轮报告，旧核准仅针对原输入 | [工程合同](pi-1.2.0-rust-backends.md)、[本轮输入](pi-1.2.0-backend-revision-baseline.json)、[source evidence](pi-1.2.0-backend-source-evidence.json)、[本轮复审](pi-1.2.0-backend-revision-review.md) |
| 2026-10-16；同一1.2.0参数／新增拒绝复核 | 已提交c5ab42a；撤回无来源固定预算、总deadline与blanket拒绝，Schema/WS等仍OPEN；此前通过仅对冻结输入 | [参数审计](pi-1.2.0-parameter-audit.md)、[独立复审](pi-1.2.0-parameter-revision-review.md) |
| 2026-10-16；Pi1.2.0→1.3.0 | Opi输入c5ab42a；新Pi tree08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2。AI新增4/修改41/删除2、telemetry仅2 metadata；67 AIR/59 V身份中各退役1项，57/54 active SDK；所有backend gaps继续OPEN | [逐项升级与清理](pi-1.3.0-upgrade-review.md)、[来源](pi-1.3.0-source-evidence.json)、[复审](pi-1.3.0-revision-review.md)、[唯一矩阵](acceptance-matrix.md) |

本表的“暂不核准”是研究候选结论，不是线上故障或产品实施进度。旧 Pi 1.0.0 源码当前不在工作区；[历史报告恢复关系](../pi-1.2.0/evidence/pi-1.0.0/README.md)不能替代源码恢复。上游网页按访问日记录；本地快照按 manifest/hash 固定，两种身份不混用。

## 同版后端专项跟进

2026-10-16，输入提交 `a37ed70`；不升级 Pi 或 Cargo。按[专项核查](pi-1.3.0-backend-gap-audit.md)补齐七组来源、验证方法和候选补救，均保持 OPEN。[专项复审](pi-1.3.0-backend-gap-review.md)与[冻结证据](pi-1.3.0-backend-gap-evidence.json)只对应本轮文档，不重写旧验证。后续记录还须固定真实 Node/Bun runtime/build、有效 feature union、backend 公开扩展点和双端接受／失败结果；前置方案未获证明不得悄悄变成实现假设。

## 规划观察表（原证据窗口保留）

每次版本跟进更新“观察证据/状态/触发”，不把提案直接加入实施范围。来源详述在 [本次补核](pi-1.2.0-followup.md#2-未来规划已有研究需刷新状态与实施证据)，这里仅作索引。

| 观察对象 | 2026-10-16 证据状态 | 重新评估触发 |
|---|---|---|
| Responses Lite / ordered Context | RFC0054 Discussion；chronology 已交付，完整 Lite 未证；DeveloperMessage 原型不等于当前 SystemMessage | 已接受的具体协议修订或实际发布的 Lite adapter/连接语义 |
| 动态目录与版本选择 | RFC0039 Discussion；本地 distribution v1 selector/publisher 与 data v6 已实现；pi.dev 部署未核 | schema/index/兼容版本选择变更；经验证的服务合同；不能用 Rust crate 版本冒充 Pi 兼容版本 |
| 凭据相关目录 | OpenRouter helper 与产品 wrapper 已交付；Radius/个性化/通用 overlay 的空目录规则不同 | key/账号/区域 endpoint 或 logout/login 变化；缓存与晚 publication 的新隔离要求 |
| 新操作/adapter | chat/image/classifier、Decisions 与 classifier images 是当前能力 | 新 public operation/model kind、API 或实际 consumer 的新增状态机需求 |
| Deferred remote | 公共生命周期、faux 和 durable consumer 已存在；内置真实远端 adapter 未证 | submit/fetch/cancel/expiry/account/idempotence 协议与 fixtures；不以假 consumer 证明远端支持 |
| pi-messages | 当前 Radius/custom HTTP/SSE wire；非未来统一 transport 承诺 | 已发布 wire versioning 或两个真实后端需要共同 conformance |
| Telemetry spans/exporter/carrier | context 传播已交付；所查 exporter/Agent spans PR closed/unmerged；analytics 另属产品 | 明确接受、合并和发布的 instrumentation，或用户单独选择的领域/数据政策 |
| Compat 退出 | deprecated 但仍存在消费者；未核准移除版本/日期 | 上游实际删除且调用方完成迁移，或本项目显式同步破坏性变更 |
| Virtual routing | 产品可选能力已有实现；历史引入时称 experimental，不能称未实现 | 稳定公开合同改变，或用户单独选择产品路由范围；分类不授予工具权限 |
| 通用 invocation Context | Chord 有自己的调用上下文；AI 未采用其载体作为公共合同 | AI 正式签名变化及真实 operation/adapter 的共同需要；不预建万能 Context |
| 产品 analytics / 实验入口 | RFC0038 Discussion；UI/settings 部分实现，完整出站未证；0043 Published | 实验毕业/删除、明确数据/关闭/发送合同；不据此给裸 SDK 默认上报或 feature-flag 框架 |

## 下一次版本跟进模板

追加一个记录并链接独立报告/差异；不要覆盖旧基线或把历史执行结果刷新日期后复用。

```text
审查日期：
Pi 声明版本 / source 路径 / manifest / tree hash / 已验证的上游身份：
比较前驱版本 / 旧报告或恢复关系：
Opi HEAD / 相关 dirty 文件及归属：
公开能力变化 / 实际 adapter 与消费者：
Context、auth、telemetry、序列化与资源所有权变化：
未来规划：已交付 / 明确接受但未实现 / 讨论实验 / 历史替代 / 未知：
调用方动作、默认值、失败、回调、retry/cancel/cleanup 变化：
Rust 有意差异及理由 / 尚缺能力：
Rust backend版本/features/API/source hash、默认环境/代理/TLS/retry/redirect/Schema变化及upgrade guards：
测试处置：add / update / delete / retain，含替代证明与唯一矩阵定位：
实际检查：输入身份、精确命令、退出码、执行/skip/缺失；未运行项目：
候选/附录需修订处 / 人类确认来源 / 本轮核准结论：
报告与证据链接：
```

按 `固定输入 → public surface/实现/消费者/文档/测试 diff → 规划状态核对 → Rust 差异与唯一验收矩阵 → 核准与实际检查记录` 执行。报告可提示后续修订，但不自动启动实现、live、安装、提交或定时监控。产品交付进度只进入其拥有权限的既有工作流，本文件不记录百分比、任务状态或另建账本。

### 参数与新增限制的复核规则

2026-10-16：[参数审计](pi-1.2.0-parameter-audit.md)/[本轮复审](pi-1.2.0-parameter-revision-review.md)取代前轮关于固定预算和后端完全定型的结论。后续每项新增/改动默认值、zero/omission/sentinel、timeout作用域、retry/redirect、format/field拒绝、backend硬上限，都记录Pi消费路径/精确依赖/default、Rust差异及接受集正反例；无依据不得直接继承。不能以可覆盖、typed Unsupported、历史D/Q或后端默认关闭parity。运行时相关常量与metadata/注释/fixture输入分开；复杂阈值需真实语料与分配/兼容性证据。只修改版本号不完成跟进。

### 1.3.0本地规划与后端跟进增量

Responses effort markers、final arguments_error、thinking-off与投影修复已在所选源码实现，不能继续作为纯未来提案；frames则已移除，不恢复旧experimental消费者。Telemetry README的旧Agent schemas示例仍过时，源代码没有恢复，exporter／Lite／realremote-deferred未在本轮入口交付；在线RFC/PR状态待下一次明确联网核验再刷新。目录最低Pi1.2.1与distributionv1/datav6分开，pin/hydrate/build:offline是发布流程事实，缺本地catalog数据仍需补证。

后端跟进仍使用[现有附件OPEN表](pi-1.3.0-rust-backends.md#5-pi-130-后端覆盖缺口全部-open)和V-35/30/49/55/58等既有验收定位；下次同时比较真实runtime、exact依赖entry、接受集与可配置／不可配置约束。不得因版本更新、选型文档或typed拒绝便记关闭；不额外建立任务状态表或进度百分比。

## 同版 Schema／WS 最小离线差分跟进

2026-10-16，Opi输入`8cb0fd68db528ae3d3c61874458d45e46bffa29c`，Pi tree仍08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2。[执行报告](pi-1.3.0-schema-ws-offline-validation.md)、[冻结证据](pi-1.3.0-schema-ws-offline-evidence.json)、[研究资产](offline-probes/README.md)、[独立复审](pi-1.3.0-schema-ws-offline-review.md)固定这次增量，不重写前置source-only报告／执行结果。

参考运行时Node24.13.0（内置Undici7.18.2）与Bun1.3.5+1e86cebd7按exe SHA／完整versions固定，Schema TypeBox1.3.27、native jsonschema0.28.3与WS TW／TTW0.28.0各有独立锁图。Schema85／native276观察证明原生配置不足；WS60constructor／15loopback／16reader／15partial观察证明参考PMD可达、当前codec不支持。Schema需重开引擎比较或先证受控适配，WS完整组合需补救／重选，未选择新库。两组和其他五组全部OPEN。

后续版本／依赖／运行时／features变更先复用同一fixtures并另存新结果：runtime SHA／constructor／source／archive／lock／平台 → 原始输入／JS值／normalized／wire／错误／退出码 → candidate机制与已声明RDF → 原V完整验收。研究runner成功不代表等价；providerimport缺依赖、源码预期、实际错误、未执行分别报告。WSBun统一--no-install；默认自动解析诊断曾产生10文件缓存，已记录并隔离，来源／联网未知，永久排除该轮offline proof。依赖准备两个官方归档与这项副作用分别记，不宣称全轮无联网。

产品代码／Cargo／测试／CI／正式spec／ledger未改；研究harness与证据add、产品testnone。候选重新比较不授予实施或新依赖准入，不创建自动监控、任务状态表或第二实施账本。

## HTTP 上游 profile 跟进边界

当前 HTTP 合同独立采用 [RDF-22](pi-1.3.0-rust-differences.md#rdf-22-upstream-http-profile)：跟进普通 crates.io reqwest/Hyper 的实际 graph、feature union、registry checksum、head/buffer/trailer/chunk-extension边界与 typed 失败／partial/usage／取消。退役 HTTP patch-apply／namespace 专属活动义务，保留旧修正证据和通用 conformance／真实包消费者；不得因上游版本升高继承通过或重新引入隐藏 retry/身份旁路。该 HTTP 修订不改 WS；crypto 后由 RDF-23 单独修订，下列同版旧窗口不改写。完整实施与平台仍由既有任务验收。

## Crypto 上游 profile 跟进边界

用户选择按 [RDF-23](pi-1.3.0-rust-differences.md#rdf-23-upstream-crypto-profile) 采用 crates.io 原版 rustls0.23.40 + aws-lc-rs1.18.2/aws-lc-sys0.46.0，经私有 safe Rust owner 使用公开 provider/key/signing seam；取消本地 LibCtx/EVP FFI 与 third-party/pi-crypto 必建义务。允许命名的 RSA1024/多素数拒绝、无 P-521 KX 和上游集合/顺序差异，保留默认 Node roots/WebPKI、JWT alg/claims、mTLS/ALPN、显式 provider 宿主隔离和取消所有权。无本地 FFI 仍有 native C/assembly/build 准入：记录实际源码、system 替换入口、工具链与包消费者 provenance，不用 library lock 声称固定所有调用者。

[后续隔离研究摘要](offline-probes/upstream-crypto/summary.json)保留最终实际计数与局限；它不是产品代码或本次 plan 的执行。真实普通/feature-union cargo package 消费者、ADC、完整 DEL-06、平台/最终包仍由既有任务验收，不提前完成20.11/34/57。通用 conformance 保留；WS Upgrade/PMD/受控 Ratchet 完全不变。下列旧窗口的 Unicode16、HTTP patch 与 private EVP 均保留历史事实，当前选择分别由 RDF-21/22/23 取代。

## 同版设计就绪收敛窗口

2026-10-16，Opi输入仍8cb0fd6，七组按用户明确确认的“设计READY／实施验收OPEN”分层判断。[当前决定](pi-1.3.0-design-readiness.md)及[独立复审](pi-1.3.0-design-readiness-review.md)取代上节当时“尚未选择新库”的状态，不改写旧source/experimental输入、hash或测试结果。

Schema选source-defined JSON evaluator、regress算法＋Unicode16生成数据、ryu-js与独立身份U16 URL/IDNA/ICU profile；WS选ownedUpgrade＋受控Ratchet，HTTP选受控Hyper存储，TLS/source crypto选实例rustls/commonring＋private OpenSSL EVP，Auth完整source/mode owner、telemetry safe pinned carrier、catalog版本化产物合同。每次跟进同时固定runtime/Unicode/CA数据、archive/source/namespace manifest变换、算法/patch前后SHA、features/lock/构建/native/unsafe/许可；后端分发必须registry可解析，不能仅依赖consumer rootpatch或library lock。产品V全部OPEN。

独立WS复审新增的长度/算术/分配反例与纯内存补救另存新窗口；原80网络观察不移植为新patch结果。用户最新要求后只运行无socket/无请求的内存验证，若仍被平台阻止则跳过并保留未证项。正式实施、平台/包发布、真实目录身份和外部consumer仍需原准入/验收；本记录不是进度账本或自动监控。

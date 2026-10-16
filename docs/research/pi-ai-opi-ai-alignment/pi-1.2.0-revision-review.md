# Pi 1.2.0 候选修订：独立复审

> 2026-10-16；仅文档。结论：**修订后的方案在文档层面可行，独立复审发现均已修正并回读关闭；没有新增必须由用户决定的取舍。** 该结论只核准研究候选的完整性、一致性和可实现性，不是正式规范修订、实施准入、产品交付或运行等价证明。实际静态检查见[本轮检查证据](pi-1.2.0-revision-verification.json)。

## 固定范围与权威

用户已明确选择：仅 `opi-ai`，以固定 Pi 1.2.0 公开能力和可观察运行语义等价为目标，接口采用 Rust 惯用设计，每项差异明确说明；根据两轮核查修订双语正文、矩阵和相关文档，再独立复审；只改文档，暂不实施。授权及历史替代见 [D-38](decisions.md#d-38--pi-120-包内全公开能力与语义修订)。

正式 `docs/opi-spec.md`、领域权威和既有准入流程继续有效。本轮没有改变 Opi 全局定位，也没有把研究材料变成规范或实施状态。SDK 提供机制；Agent 执行授权、durable session v3／迁移／compaction、tool_search 装配、产品恢复和启动策略保持宿主责任。必要的 API 破坏性变更适配仍需在未来实际实施中同步。

输入来自[版本审查](pi-1.2.0-review.md)、[五问补核](pi-1.2.0-followup.md)与[修订前 47 文件基线](pi-1.2.0-revision-baseline.json)。Pi tree SHA-256 为 `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10`；Opi HEAD 为 `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8`。前两轮的“不通过”指向各自固定的修订前稿，没有覆写历史结论。

## 复审方式及独立性

三名新 reviewer 均未参与本轮被审文档写作；分别只读核对源码、契约和证据，不编辑仓库。主审逐项修正文档，再由原 reviewer 回读受影响条款并记录最终输入摘要。翻译者及两份附件／矩阵作者的自查没有计作独立复审。

| 独立角色 | 核对边界 | 最终结论与限制 |
|---|---|---|
| `independent_rust_scope_review` | owned／惰性调用、cleanup generations、对象安全 store-owned rotation、retry、schema／usage、telemetry parent＋Dispatch、包与宿主权威 | 两项 P2 在最终回读关闭；其余所查核心机制无新矛盾。没有执行 runtime／跨进程／平台 conformance |
| `independent_pi_capability_review` | 固定 public exports、10 chat／1 image／4 classifier、42 factories、9 OAuth、callbacks／frames／duration、metadata／cost／catalog／utilities／平台 | 四项 P2 在最终源码回读关闭；不声称穷尽 hydrated model 数据或真实服务接受度，中文由另一 reviewer 核对 |
| `independent_bilingual_evidence_review` | 完整双语意义、AIR/V scope 与追踪、41＋7 测试清单、历史执行边界、版本跟进和本地链接／渲染 | 一项 P2、两项 P3 在最终回读关闭；新修改双语一致。协议正确性由前两角色分别承担 |

重复的代理发现合并计数：共 **6 项独立 P2、2 项 P3**，均已关闭。下表不是另一份要求或实施进度账本，只记录本轮复审的发现和处置。

## 发现与处置

| ID／级别 | 发现及固定一手证据 | 修正与独立回读 |
|---|---|---|
| REV-01／P2 | 已开始 rotation 的完成保证未明确宿主 runtime 存活边界；Tokio Runtime shutdown 会终止 yielded tasks，Handle 不延长 Runtime 寿命 | AIR-051/063、OAuth 附录、RDF-10、V-12/43/55 明确：caller 停等与 store-owned 工作分离，仅在宿主 runtime 存活时继续；host 正常关闭前保持 runtime 至操作收束。shutdown/abort/timeout/remote或store failure 不保证保存、远端回滚或旧 token 有效；ACK 丢失保留已确认状态并报告未确认效果。禁止私有 runtime。Rust reviewer 最终回读关闭 |
| REV-02／P2 | 延后 SKIP_AUTH 丢失实际公开的“无真实 AWS 凭据调用代理”与 HTTP/1.1 动作；[adapter](../../../.repo/pi-1.2.0/packages/ai/src/api/bedrock-converse-stream.ts#L183)、[placeholder signing](../../../.repo/pi-1.2.0/packages/ai/src/api/bedrock-converse-stream.ts#L207)、[本地 fixture](../../../.repo/pi-1.2.0/packages/ai/test/bedrock-response-headers.test.ts#L47) | AIR-023/057/066、SDK 附件、RDF-12、V-09/49/58 增加显式 typed `ProxyPlaceholderSigning`＋非标准 endpoint＋HTTP/1.1。保留非秘密 placeholder SigV4，不称 unsigned NoAuth 或真实 AWS 身份；零真实来源／ambient bearer 探测。Pi 层显式 bearer 选择与 Node SDK ambient 漏洞分开。两位 reviewer 同意该动作对应和安全差异并回读关闭 |
| REV-03／P2 | 通用 simple/max/cache 默认覆盖 Pi Messages backend 默认；[六字段 payload](../../../.repo/pi-1.2.0/packages/ai/src/api/pi-messages.ts#L375)、[simple 透传](../../../.repo/pi-1.2.0/packages/ai/src/api/pi-messages.ts#L432)、[cache](../../../.repo/pi-1.2.0/packages/ai/src/api/pi-messages.ts#L347) | 双语选项附录、SDK 协议行、RDF-16、V-45/49 保留 max/cache 省略态、显式 max 原值和 long 环境 opt-in；仅六字段序列化，不误发 metadata/sampling/thinkingBudgets/deferred。源码 reviewer 和双语 reviewer 回读关闭 |
| REV-04／P2 | 通用 tier 计费遗漏图片 adapter 例外；[OpenRouter image parseUsage](../../../.repo/pi-1.2.0/packages/ai/src/api/openrouter-images.ts#L167)直接使用基价，不调用[共享 tier helper](../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1206) | AIR-062、SDK/RDF、V-50/54 保留基价例外；用同模型 base input=1／tier input=2 且超阈值的判别 fixture 分开 image 与 helper，不能套共享算法后声称相同。源码及双语回读关闭 |
| REV-05／P2 | serviceTier 仅记 request 选项，遗漏 response-sensitive usage.cost；[Responses](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-responses.ts#L391)、[Codex](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-codex-responses.ts#L603)、[shared parser](../../../.repo/pi-1.2.0/packages/ai/src/api/openai-responses-shared.ts#L578) | AIR-062、SDK/RDF、V-49/54 明确 Responses/Codex 的 flex/priority/fast、response default/request fallback、精确 gpt-5.5 倍率和 Azure 无倍率；保持 unknown，分 HTTP/WS 与 request/response fixtures。源码及双语回读关闭 |
| REV-06／P2 | AIR-008 active SDK MUST 仍含执行实现 revision，和 AIR-009／V-06 host-only 冲突 | 双语 AIR-008 明确执行实现 revision 属 AIR-009 宿主，不增加 SDK execution registry。双语／证据 reviewer 回读关闭 |
| REV-07／P3 | 双语退出表最后一行被空行隔开 | 删除表内空行，双语 reviewer 回读关闭 |
| REV-08／P3 | 上游跟进表新修订行被空行隔开 | 恢复连续表格，双语 reviewer 回读关闭 |

## 当前候选覆盖与测试迁移

[双语正文](spec-draft.zh.md)（[English](spec-draft.md)）有 **67 AIR＝58 SDK＋9 宿主影响**；[唯一验收矩阵](acceptance-matrix.md)有 **59 V＝55 SDK＋4 宿主影响**。host-only AIR-009/015–019/037/039/041 与 V-17/18/19/33 不冒充 SDK gate；混合项只计算 SDK 机制和必要 caller 适配。

[SDK 附件](pi-1.2.0-sdk-contracts.md)覆盖 10 chat API、1 image API、4 classifier API、42 实际 factory、9 OAuth families；还包括三类 Context、callback/header/raw、frames、duration、simple/native/defaults、sampling／pricing／input limits、catalog personalization/versioning、显式 whole-call retry、faux/deferred、custom seams、公共 utilities 与 package/native 平台对应。[RDF-01–20](pi-1.2.0-rust-differences.md)把语言表达、安全／资源、平台与未实施缺口分清；差异理由不代替能力证明。

矩阵完整保留 **41 个 integration＋7 个 cfg(test) src 文件**的未来 add/update/delete/retain 处置；有效 wire／安全／资源测试保留，过时 oracle 更新，壳／phase-count 断言仅在替代证明后删除。Pi 的 171 个 test 文件是输入清单数，不是已执行 case 数。差分工作流仍是未来方案，须隔离 HOME、auth store、环境凭据、live discovery 与 generated-data 缺失；历史 70 个离线测试和 probes 保留原输入及覆盖边界。

本次测试影响 **none**：没有修改测试／CI，没有执行 Cargo/Node tests、构建、差分 runner、live/OAuth、依赖安装或 hydration。没有因发现凭据而发起请求。缺少 generated provider data、真实 OAuth/ADC/federation 接受度、远端目录部署／bytes 与真实 remote deferred 协议依然分别是证据边界，不伪称已经证明。native 跨平台和性能收益亦未验证。 锁定 Node 依赖的部分精确行为沿用此前已记录的源码／摘要证据；旧暂存依赖叶文件本轮已不在，未重新下载或执行，不能称为本轮完整依赖源码复验。

[持续跟进记录](upstream-tracking.md)继续追加版本、固定来源、公共行为与规划状态、Rust 差异、测试处置及实际检查；不新增实施 ledger 或自动监控。Discussion RFC、closed-unmerged PR、目录本地实现与远端部署分别记录。当前无足够证据把继承的 AWS adaptive/ambient retry knobs 当成 Pi 新增的显式 SDK API；Rust 采用标准模式和 typed override 的差异仍明确，不声称完全复制 Node 依赖的环境行为。

## 静态检查及共享工作区

工作区模式检查，不审查或交付历史 branch delta。必需命令为 `python scripts/opi-doc-check.py` 和 `git diff --check`；另核对 UTF-8/LF/尾空白、公开 factory 集合、AIR/V 唯一性与 scope、双语条款及本地 link／line anchors。精确结果、命令退出码、最终输入摘要、测试清单和改动归属见[检查证据](pi-1.2.0-revision-verification.json)。本报告与新附件没有既有英文 counterpart；双语主稿共同引用同一附件，避免两套附录要求。

旧 `docs/research/pi-1.0.4` 删除、完整 `pi-1.2.0` 报告和前两轮审查是 carried-in，本次保留。已 dirty 的双语稿／矩阵／README／交叉核查为 carried-in-and-touched；其他本轮文档修改见检查证据。HEAD 不变、staged 为空、committed=none-for-this-task；没有更改 runtime、tests、Cargo、CI、正式规范或 `.opi-impl-state.json`。

## 最终复审输入摘要

以下为 reviewer 最终回读的核心输入；报告自身不参与自引用 hash。完整文件状态由检查证据记录。

| 文件 | SHA-256 |
|---|---|
| [spec-draft.md](spec-draft.md) | `499120e45437f590f44af57ac98b6278295e1a4cf0f20e1188bb5e218f5ddd5c` |
| [spec-draft.zh.md](spec-draft.zh.md) | `ea9fd69c3098e999abda9580727aa9a79f34a47e1273b192699a72828e73964f` |
| [acceptance-matrix.md](acceptance-matrix.md) | `88057b23d6366143050db72dc4603629b3d9b9d7d7cd7269ccc10a0bd6980fe8` |
| [pi-1.2.0-sdk-contracts.md](pi-1.2.0-sdk-contracts.md) | `8a93ea4b495dec0d64ed7363a04c9c2e81ed0301b70eb2b58037783c7f3253b9` |
| [pi-1.2.0-rust-differences.md](pi-1.2.0-rust-differences.md) | `5c06372f4ad92f2f6bbf8bf49b1987a4aa33fa77b15cbf356169431732904eeb` |
| [README.md](README.md) | `a18bf715160062b68b2d5f65acf9549627b052c628b4dc39cc636e21454f0375` |
| [decisions.md](decisions.md) | `5fd2ec42e4730781e13ac517f124e295811890d6c83ea3748f53efa3a775680a` |
| [upstream-tracking.md](upstream-tracking.md) | `b98fad51db481f2fbd1696ec9af76464585e57a105b810a32fb178214746b840` |

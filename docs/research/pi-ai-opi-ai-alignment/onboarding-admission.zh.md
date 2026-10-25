# SDK 上手与参数准入——P.4 审查包

[English](onboarding-admission.md) · [完整任务图与追踪](onboarding-admission-graph.md) · [P.2 结构化结果](onboarding-admission-review.json)

## Write-only 安装回执

人类已明确选择 `confirm-all` 和 **write-only**，P.4 已满足；P.5 于
`2026-10-25T07:50:28.672233+00:00` 安装确认图。确认后未改变 tasks 数组或登记来源。
下文保留确认前审查记录及输入 hash。

| 安装结果 | 验证 |
|---|---|
| 正式账本 | `.opi-impl-state.json`，SHA-256 `c06564f7501713a5108c26e17473dbb9707ea2d5fdc77fdae86431e714a6d5de` |
| 同一已审任务数组 | SHA-256 `ab7fcba57e2453cfea5aa397508168be53f0bd080e966885b9c98ae862646168`；68 项、273 场景、32 passing / 36 failing |
| 执行状态保留 | 32 个 passing 对象、每项 status/依赖/执行历史、phase-exit 历史与安装前基线相等 |
| 登记来源 | 同 16 路径，全部 CRLF-normalized pins 匹配，来源漂移为零 |
| 宿主来源行 | HOST-CFG / HOST-VALUES 仍为独立、未修改、未登记候选，不进入本 SDK 图或 spec_files |
| 临时文件清理 | 安装成功后已移除核验过的忽略草案和安装 candidate，无 ledger tmp 或 replacement backup 残留 |

原子安装使用 Windows ledger guard `Install`，传入
`-ExpectedTargetSha256 b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`。
返回 exit 0、`ledger install ok` 和上述安装 hash。对正式文件执行
`validate-plan.py .opi-impl-state.json`、ledger guard `Validate` 均 exit 0。
安装后结构化检查验证了身份、历史保留、来源 pins 和排除边界。

本 write-only 步骤修改正式账本及本双语回执/共用任务图首部；本步骤未改生产代码、
测试、父规范/登记规范或宿主候选。此前来源编辑保持未提交。HEAD 仍为
`de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`，暂存和本任务提交均为 none。
无关研究文件 SHA-256 保持
`aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824`，继续排除。
未开始任务实施、Cargo/runtime 测试、提交或发布。测试影响：**none**。

## 确认前审查快照

P.0 来源具备准入条件，P.2 对已审草案给出 **READY**。READY 仅允许展示 P.4。
该次交接时，完整图尚未得到人类确认或安装，生产代码、测试、父规范、正式账本和
Git 历史均未修改；上述回执记录了后续明确确认和安装。

用户已批准双语上手/配置设计并明确调用 `opi-implement plan`。先把已批准 SDK 含义
作为 shaping 结果物化到现有登记来源，再进入 P.0/P.2；准入 reviewer 没有修改或
发明来源含义。原设计提案仍是证据，不是新增登记规范。

## 固定输入与协调边界

| 输入 | 身份 / 处置 |
|---|---|
| Branch / HEAD | `main` / `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab` |
| 安装前正式账本字节 | SHA-256 `b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a` |
| 原 P.2 草案 | SHA-256 `773f3744b42e58ccfe70ebf786c2d574cc45c989dcda115b1bee9b151a6ea35b` |
| 记录审查元数据后最终草案 | `.opi-impl-state.draft.json`，SHA-256 `1da703413227fc72200cca429f1957f417966f4548a38ad8b092df13d852e64a` |
| 不变的已审 tasks 数组 | SHA-256 `ab7fcba57e2453cfea5aa397508168be53f0bd080e966885b9c98ae862646168`；sorted-key compact JSON、UTF-8、无末尾换行 |
| 登记来源集 | 同 16 路径，仅双语 `spec-draft` 和唯一 `acceptance-matrix` 内容/pin 变化；图附录列全部前后 pin |
| Runtime 图 | 同 68 项、32 passing / 36 failing；全部 ID、标题、依赖、运行历史及 32 个 passing 对象保留 |
| 新验收 | 7 个 ONB 判据，22 个 failing owner 各加 1 场景；场景总数 251 → 273 |

P.2 使用原 tasks 数组。之后仅向忽略草案根部 `verify_runs` 追加记录，tasks 身份和
登记来源输入均未变化，没有编辑已审任务图。该次交接时草案确认时间为 null；正式
账本的确认时间和旧来源 pins 保持原样，直到 P.4 人类 gate。

## P.0 来源准入

| 准入问题 | 来源 / 答案 |
|---|---|
| 已审且登记 | 既有 Phase 20 registry，恰 16 文件；获批 SDK 设计物化在 `spec-draft.md` / `.zh.md` 和唯一矩阵 |
| 问题/方案/范围 | 原 SDK 来源加“SDK onboarding and parameter acceptance / SDK 上手与参数验收”：普通 SDK 首次使用、手册收口、已采用请求语义 |
| 成功与退出 | ONB-01–07 映射既有 V/owner，保留 57 active AIR / 54 SDK V、独立 F.1a、host-only 排除 |
| 来源及 Rust 差异 | 向内 Pi 1.3 SDK 协议；Pi 1.4 README 仅作体验比较；Rust 类型/runtime/依赖及 RDF 不变，不增加 SDK 协议增量 |
| 放置 | 现有 SDK 和 assurance/document owners；产品 parser/命令执行留给独立 Reference Product 候选 |
| 领域与权威 | Parent AUTH/PLACE/PHASE 和 CONTEXT 不变，不增 User Policy、Package Trust、Agent Core 权威 |
| 公共测试边界 | 真实 Models/native/simple/direct 请求、同源外部 Cargo consumer、native/platform 证明、文档交接分别归现有 owner |
| Source-return 规则 | 初始化不补造缺失产品含义；已批准 shaping 修订先于 P.0，不把任意提案或递归链接加入 spec_files |

## 行级验收变更

[完整任务图](onboarding-admission-graph.md)展示全部任务的 tier、owned paths、commit type、
dependencies/拓扑序、evaluator、scenario/call-site 数、substrate、forbidden scope、
inference notes，及所有场景来源/命令、六项追踪；每个变更行还列准确 DoD 前后值和
新场景。共用附录保留正式字段原文、使用双语列名，不另译一套数据，也不是平行进度账本。

| Owner | 新增验收 / 字段影响 |
|---|---|
| 20.16 | 原 Codex owner 的 native options/frozen transport；DoD＋ONB-03＋来源 note，不增依赖 |
| 20.34 | Profile/model/call 默认到真实请求、auth endpoint 先于 compat、false/zero/省略、exchange/attempt 统计；DoD＋ONB-05＋note |
| 20.35 | Native/simple 来源/类型/单位/默认/优先级清单和共同机制请求捕获；DoD＋ONB-03＋note，behavioral tests 加已有 `request_enrichment.rs` |
| 20.36–20.49 | 逐协议/typed operation 准确参数、省略、失败和实际 wire；各加 ONB-03、DoD 补充和 note |
| 20.51 | Header 省略/设置/删除、protected identity、逐 variant callbacks；DoD＋ONB-04＋note；保留当前 adapters 与后续消费者的完成分区 |
| 20.56 | 双语手册和真实 API 示例；DoD＋ONB-01＋note；拥有将来准确 `quick_start.rs`、`quick_start_stream.rs`、`tool_loop.rs`，本任务编译 |
| 20.57 | 实际 `.crate` 的普通外部消费者执行同源示例；DoD＋ONB-02＋note；保留完整 assembled SDK/package 职责 |
| 20.58 | README 支持声称对应真实 native/MSRV/features/artifact；ONB-06＋note，原 DoD 不变 |
| 20.59 | 最终手册/示例/证据引用与实际处置；DoD＋ONB-07＋note；保留矩阵漂移 P.0/P.2/P.4、独立 F.1a |

其他字段/行保留。不提升状态、不重标 auth 证据、不删除原场景或 notes、不增删依赖。
既有四项标准 notes、8 个 shared decisions、6 个 substrate 闭包完整；新验收消费同一
公共机制和测试边界，不另增 shared decision owner。

## 两个宿主来源行：仅规划登记

独立的 [Reference Product 来源候选](../reference-product-configuration/delivery-candidate.zh.md)
有英文对应、父条款/战略映射、问题/范围/放置、固定 Pi 1.4 config 输入、非目标、
成功/退出、平台/风险/回滚、生产边界和六组可执行场景。

| 来源行 | 拟登记任务 | 依赖方向 |
|---|---|---|
| HOST-CFG | 后续 Reference Product 纵切：默认 TOML、显式 settings/models JSON bundle、直接 Pi 字段、单 resolver、旧字段拒绝、保留 authority 的配置到请求证明 | 消费已准入 SDK factories/options/callbacks |
| HOST-VALUES | 第二纵切：literal/env/转义/命令值、真实 auth/headers、逐请求解析、来源 timeout/cancel/进程树清理 | 依赖 HOST-CFG 和可复用已准入 source/process 机制 |

本次不分配正式 Phase/task 编号，不设 `20.60/61`，不加入当前可执行图或 `spec_files`。
后续选定 Phase、明确登记来源，再单独经过 P.0/P.2/P.4。宿主 parser/文件/policy 不
成为独立 SDK 前置。本登记规划无须改父规范或当前 skill registry。

Pi 1.4 字段词汇限于宿主候选；SDK 协议仍为 Pi 1.3。Client identity、空白 tool-result
等 1.4 行为单独审来源。旧 TOML 拼写不保留兼容 reader；真正 Opi-only 能力保留 typed
归属，不因改名消失。

## P.2 独立反证结果

一名 fresh-context reviewer 顺序运行五组 charter，覆盖全部十项 design/execution
lenses、原完整图、minimum-change、shared decisions、substrate 闭包。独立性为
`fresh-context-same-family`，不是跨模型。本会话无 Workflow runtime 入口，采用文档
规定的 single-reviewer fallback；未运行 `plan.workflow.js`，`wf_ref` 为 null。

结果 **READY**，无保留 design/graph finding 或 human flag。三项候选阻塞均经反证撤回：

1. SDK 图缺宿主任务：来源明确排除并要求独立登记，因此不能加入当前 SDK 前置。
2. 20.35 依赖未完成 adapters：仅证明共同机制，逐协议和组合证明归 protocol owner/20.34。
3. 20.56 只编译示例：其后继 20.57 已拥有实际包的同源公共执行证明。

[结构化结果](onboarding-admission-review.json)保留 finding、引用、撤回理由、资源和身份。
没有 finding 自动折入图；reviewer 未改文件、未跑 Cargo/provider、未确认或安装图。

## 本次实际验证

| 命令 / 检查 | 实际结果 |
|---|---|
| `python .claude/skills/opi-implement/scripts/validate-plan.py .opi-impl-state.draft.json` | Exit 0；`plan validation ok tasks=68` |
| `& .claude/skills/opi-implement/scripts/ledger-guard.ps1 -Command Validate -Path .opi-impl-state.draft.json` | Exit 0；严格 UTF-8/v2/大小/敏感值校验；仅 Validate，未 Install |
| 补充结构化 Python 检查 | ID 唯一、依赖完整/无环、tiers、behavioral path 归属、场景命令/调用点、ONB 覆盖通过；20.13/14 原测试路径括号说明按注释处理，不当路径后缀 |
| 双语/身份检查 | 7 个 SDK 和 6 个宿主判据对应，5 个宿主比较 hash、本地链接、空白通过；32 passing 对象、全部 runtime/dependency 字段与正式账本相等 |
| `python scripts/opi-doc-check.py` | Exit 0；`opi documentation contracts: PASS` |
| `git diff --check` 及新增 Markdown 空白检查 | Exit 0，无空白问题 |
| Cargo/runtime/package/live | 未运行：仅变更已批准 source/plan/evidence metadata，没有实施或修改测试 |

安装前 Worktree-only：该步骤 committed **none**、staged **none**；tracked unstaged 恰为三份
SDK 登记来源。新准入证据和双语宿主候选为 untracked。此前四份已批准提案仍为带入、
未触碰的 untracked。无关研究文件仍为 untracked 并排除，SHA-256
`aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824`。
忽略草案曾保留以继续 P.4，已在上述确认安装后移除。测试影响：**none**。

## P.4 选择历史与写入边界

具体[完整任务图](onboarding-admission-graph.md)和行级变更已可审查。Initializer 的选项为
`confirm-all`、`edit-task <id>`、`apply-rule`、`export-draft`、`import-draft`、`abort`。
任何图编辑都会使 READY 失效，确认前须重跑 P.2。

此前“同意设计”授权来源物化和规划，没有确认当时新展示的完整图。工作流 P.4 明确：
**“REFUSE to proceed until whole graph is confirmed.”** 因此等待 gate 时正式账本保持
原样、来源漂移为预期待确认状态；后续明确 `confirm-all` 和 write-only 指令已满足 gate。

Write-only 边界现已按上述回执完成：记录确认、身份复核、乐观并发安装、清理临时文件。
不会自动开始实施、暂存、提交或发布。

# Phase 19：本地原生 Agent 评测

[English](2026-09-05-phase19-local-native-agent-evaluation-design.md)

## 1. 权威与交付结果

本交付规范将已经人类评审的本地评测方向落入正式文档。英文文档拥有 Phase
含义，中文版保持相同的标识符与要求。来源登记由
[`opi-implement`](../../../.agents/skills/opi-implement/SKILL.md) 的已评审来源清单
拥有。登记不等于任务图准入、启动实现、授权 provider 调用或记录完成状态。
只有显式调用 `opi-implement plan` 才能准入来源，并将经过评审的任务图写入
`.opi-impl-state.json`。

规范性父来源为 [`docs/opi-spec.md`](../../opi-spec.md)，领域语言继续由
[`docs/CONTEXT.md`](../../CONTEXT.md) 拥有。本 Phase 派生自
**STRAT-002 — Establish independent cross-Agent Eval**，端到端负责人为
Evaluation product owner。

交付结果是一条独立的 `opi-eval` 本地使用流程：用户配置 provider，选择 Opi、pi
或两者，运行固定版本的 Terminal-Bench 2.1、Terminal-Bench 3.0 或 Datacurve
DeepSWE v1.1 任务，取得可追溯到这些 Agent 实际任务产物的原生结果。任务执行、
评分、证据和报告留在用户电脑上；模型推理可以调用用户的远程 API。验收平台为
Windows x86_64、WSL2 Linux 与本机 Docker Linux engine。

| 父条款 | 本 Phase 的责任 |
|---|---|
| `GOAL-004`、`PRIN-001`–`PRIN-003`、`PLACE-001`–`PLACE-004` | Companion 不依赖 Opi 产品 crate，也不参与 Minimal Runtime 激活。 |
| `PRIN-004`–`PRIN-005`、`CTRL-002`–`CTRL-003` | 身份或证据无效时闭合失败；保留实际生效控制并脱敏机密。 |
| `CTRL-004` | 使用已准入原生 grader 评分所选 Agent 的真实产物。 |
| `CTRL-005` | 冻结比较条件，保留选定/全量覆盖、缺失配对、排除项及失败分类。 |
| `CTRL-006` | 保持 bundle 不可变、报告可离线重建，新执行使用新身份。 |
| `CTRL-007`、`GATE-001`–`GATE-002` | 原生 runner 与任务材料留在模块外；证明 Opi/pi 与 Harbor/Pier 集成，不提前稳定公共 SDK。 |
| `PHASE-001`–`PHASE-006` | 以明确的验收、风险、回退与平台边界完成有限交付。 |

## 2. 问题、证据与优先级

现有 conformance 基础中的实验、生命周期、进程监督、封存和报告机制具有复用价值。
但 native producer 在任务 job 之外单独运行 Opi/pi，评分路径没有消费其输出，
固定 runner 在未选择 Agent 时默认使用 oracle。因此，一个 native reward 不足以
证明 Opi/pi 成绩。现有本地入口还依赖面向 fixtures 的输入和固定 scripted provider
配置。先修复评分归因，再扩大实验规模和完善便捷操作。

以下材料均为**非规范证据**，不能替代权威来源：

- [Phase18 执行缺口核查](../../research/2026-09-05-opi-eval-phase18-execution-gap.zh.md)：源码数据流和历史日志证据，没有新的动态复现或完整历史 artifact 审计。
- [Windows 可行性核查](../../research/2026-09-05-local-agent-benchmark-windows-feasibility.zh.md)：固定任务语义与已观测宿主限制；已安装不等于 engine 或任务已经可运行。
- [智谱端点核查](../../research/2026-09-05-zhipu-eval-provider-endpoints.zh.md)：端点文档证据；模型访问和工具循环兼容性仍需执行证据。
- [冻结的 Phase18 规范](../../snapshots/phase18/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md)：历史交付范围，不是本 Phase 的活动来源。
- [当前 Companion README](../../../crates/opi-eval/README.md)、[benchmark profiles](../../../crates/opi-eval/profiles/benchmarks/) 和 [external lock](../../../crates/opi-eval/external-locks/static/linux-x86_64.json)：当前实现和材料身份，不证明本文提出的行为已经实现。

Phase19 修正并补全独立 Eval 路线，不推进 Learning 或 Promotion。历史快照和
receipt 保持原样。

## 3. 范围与架构归属

选定方案扩展现有 Rust Companion CLI 与实验配置，通过真实 Opi/pi 集成调用固定
版本的外部 Harbor/Pier runner。Companion 拥有选择、配置投影、监督、证据绑定与
报告；runner 拥有任务环境创建、官方产物收集和原生 verifier 执行；各 Agent
保留自己的 provider 客户端、工具与 native 配置。

这是 Independent Companion 归属：仅评测 pi 时不需要 Opi 可执行文件即可提供
完整价值；失败不改变普通 Opi 行为；删除 Companion 会移除评测策略，而不会把它
转移到 Agent Core。Rust 类型和显式所有权保护本地契约，外部进程保持跨语言复用。
新增 trait、crate 或协议之前先复用现有适配与生命周期边界。材料获取和 runner
集成不把数据集、grader 实现、容器镜像或新的 sandbox engine 内嵌进 crate。

让用户手动运行多套上游工具并匹配输出，不能交付已经认可的本地入口。另建容器
runner 会在没有已证实需求的情况下重复上游任务语义。发布、拆仓库和稳定第三方
API 是独立决策。

非目标：

- 为这些 Linux 任务集提供原生 PowerShell 执行路径、远程 worker、托管调度、GUI
  或排行榜提交。
- 在本机完成所有资源密集的 TB3 任务；必须提供全量资源预检，真实验收使用显式
  选择的可运行子集。
- 性能 season、有统计显著性的领先声明，或必须超过 pi、取得正分的要求。
- 新的 Agent Core 证据/provider 接口、Eval 专用 LLM 客户端、额外 Agent 或任务集、
  Learning、Promotion 或结果自动激活。
- 三个 API 协议共十八次强制 trial；本 Phase 只要求首选连接的真实 provider 验收。

## 4. 本地流程与配置

构建后的程序及其声明的集成材料在独立评测目录工作。构建可以使用仓库 checkout，
运行时不需要仓库 fixtures、GitHub run/artifact 标识、workflow dispatch、上传，
或名为 `opi-eval` 的项目技能。下载公开源码和镜像仍属于普通材料获取。

| 用户操作 | 必须可观察的行为 |
|---|---|
| 诊断与校验 | 不启动模型，报告 Linux engine 可达性、架构、可用资源、缺失材料和无效配置。 |
| 准备材料 | 将固定任务包、runner 依赖、Agent 可执行文件和镜像解析到本地缓存；输出经过验证的材料身份及可处理的失败原因。不调用模型。 |
| 预览实验 | 执行前显示 subjects、连接、任务清单及选择规则、重复次数、总 trial 数、并发、限制和资源缺项。 |
| 运行 | 通过所选 native 集成执行冻结计划，显示任务、Agent、生命周期阶段、结束状态及分数/错误，并保存结构化本地结果。 |
| 恢复或替换 | 分类持久完成、取消、失败和效果未知状态。替换使用新 trial 身份，并保留旧证据与配对组规则。 |
| 校验、重新统计或重新评分 | 区分字节校验、报告重建和实际原生重新评分；三者均不启动 Agent 或 provider。 |

在语义仍准确的前提下使用现有 TOML 实验入口与 CLI。准入后的任务图确定字段和
命令的精确拼写，同步更新 help、示例和调用方，并为每个操作指定具体公共测试入口。
字节校验操作的文档不能继续称其为实际原生重新评分。真实执行由选定配置和运行
操作显式决定，凭据存在本身永远不选择真实执行。

配置包含以下语义输入：

| 对象 | 必需内容 |
|---|---|
| Subjects | 一个或多个所选 Agent 实现、可执行文件/运行时身份、隔离的 native 配置，以及声明的比较边。 |
| Connection | provider 身份、协议、base URL、精确模型 ID 与本地凭据引用；协议名称不替代 provider 身份。 |
| Common controls | 实际生效的 reasoning、采样、上下文/输出限制及来源；不支持的必要控制阻止严格比较。 |
| Task selection | corpus 来源/修订、完整任务清单、显式 full/subset 选择、任务材料身份、重复次数及适用的 seed。 |
| Execution bounds | 有限正数并发、计划 trial 数、监督限制，以及用户授权的用量/费用限制和可执行能力。 |
| Storage and capture | 本地缓存与结果位置、证据捕获策略，以及与可导出产物分离的私有凭据输入。 |

凭据来自用户选定的环境变量引用或本地私有文件。只为所选 Agent 进程解析凭据，
不读取或改写用户全局 Agent 配置。grader 不接收 provider 凭据或无关宿主文件。
机密配置不进入任务产物交接；共享环境在执行 verifier 前清除临时凭据材料，并
清理 verifier 进程环境。不能通过给 Agent 任务挂载宿主 Docker socket 或广泛的
宿主目录来实现工具访问。

首轮真实比较使用下表中用户选择的连接。其余两项为可选、独立命名的连接。两种
Agent 的三个协议投影都需要确定性的请求路径、鉴权和工具循环 fixture 覆盖；
这不等于可选连接已经通过真实服务资格验证。

| 协议 | Base URL | Native API | 首轮真实验收 |
|---|---|---|---|
| Anthropic Messages | `https://open.bigmodel.cn/api/anthropic` | `anthropic-messages` | 智谱 `glm-5.3-flash`，Opi 与 pi |
| OpenAI Chat Completions | `https://open.bigmodel.cn/api/coding/paas/v4` | `openai-completions` | 可选 |
| OpenAI Responses | `https://open.bigmodel.cn/api/v1` | `openai-responses` | 可选 |

不允许隐式回退模型、端点、协议或计费产品。native Agent 的模型选择字符串可以
语法不同，但可比性由实际请求身份与生效控制建立。缺失的服务端模型身份保留为
带原因的 unknown，不能从配置伪造。不支持的访问或控制在其所属边界失败。
有意修改配置会产生新的实验身份。金额估计、provider 报告用量和账单金额保持
区别；未知价格不能支撑保证金额上限的声明。无法强制执行的用户限制必须在执行前
拒绝或显式修订，不能忽略。

## 5. 原生执行与材料契约

上游 job 显式选择 Opi/pi 集成，并绑定实际 Agent、任务与 trial 身份。集成在任务
环境中安装/调用选定 Agent，使其文件和 shell 工具在那里工作。同一个上游 trial
提供官方产物交接和原生评分结果。先独立运行宿主 Agent，再启动未指定执行者的
另一个 runner job，不是有效集成。实际 Agent 身份缺失、为 oracle 或不匹配时，
即使所有子进程成功退出，也必须拒绝其分数。

| 任务类别 | 产物交接与 grader 边界 |
|---|---|
| TB2.1 | 遵守选定任务声明的生命周期；共享 verifier 任务在清除凭据后，对该 Agent 留下的任务环境评分。 |
| TB3 | 遵守每项任务的 verifier 隔离与 collect 声明；独立 verifier 任务只向验证环境复制声明的 Agent 产物。 |
| DeepSWE v1.1 | 遵守任务 base revision 和 collect hook；收集 Agent 已提交的 patch，在全新独立 verifier 中按声明网络策略应用。不能静默用未提交工作替代 patch。 |

这些类别描述是集成责任，不覆盖每项任务自己的配置。解析所有选定任务，保留
网络、CPU、内存、GPU、磁盘、Agent timeout 与 verifier timeout 要求。provider
访问走 runner 支持的 Agent 侧推理路径，不扩大任务或 grader 的网络权限。
不支持的生命周期或资源声明必须明确拒绝，不能近似执行后标注为原生等价。

复用当前固定的 corpus 与 Harbor/Pier 身份作为材料起点，将任务选择从当前三个
固定 smoke profile 扩展出去。每个选定任务都有经过校验的文件/摘要清单、指令、
环境、收集规则和原生 grader 身份。trial 计划冻结前，将可变镜像 tag 解析为不可变
材料身份。上游修订变更需要重新进行完整性准入并生成配置身份。本地材料校验替代
强制 GitHub upload receipt 前提，同时保留字节/来源验证并如实记录本地产生来源；
不能为本地材料伪造 GitHub 来源。

全量预检枚举固定 corpus，对照 WSL/Docker 内可见资源报告每个任务的要求，也
报告未知观测。GPU 型号、内存、磁盘不足，镜像缺失或任务语义不支持，均不能
静默缩小请求范围。用户选择可行子集后冻结明确 task ID 与选择原因。仅为报告
资源需求而做全量规划，不要求下载或启动所有任务镜像。

oracle 执行是显式选择、单独标识的诊断。它不能提供 Agent 结果、填补缺失配对或
改变评分分母。每类任务都使用可控的正确/错误产物样例，经过同一生产交接路径和
真实原生 grader，证明评分响应提交产物的变化。这些样例属于 conformance 证据，
与六次真实 provider trial 分开。

## 6. 证据、失败与报告

仅在实际集成需要时扩展现有不可变 bundle。每个分数绑定 subject/可执行文件身份、
实际连接和控制、任务与 runner 修订、实际上游 job/trial/Agent 身份、环境与资源
身份、Agent trajectory、交付产物或 patch 摘要、verifier 输入身份、原始 native
输出及解释规则。规范化不足以表达语义时保留原生产物。不超出已经证明的共同接口
强加新的 canonical trace 格式。

| 结果 | 报告处理 |
|---|---|
| 有效原生成功或 Agent 失败 | 保留原生指标和 benchmark 定义的结果，包括零分和有效任务超时。 |
| 基础设施、provider 访问、adapter、身份或无效评分失败 | 保留原因与已尝试/计划覆盖；不能计为 Agent 成功或失败。 |
| 已取消、未启动或效果未知 | 保留生命周期状态与缺失覆盖；不能编造结果或静默重跑。 |

各 benchmark 报告同时显示指标定义、分母、corpus 大小、选定任务、计划与已尝试
trial、有效评分 trial、Agent 失败、无效/基础设施失败以及未执行项。子集比率明确
标记 subset；未完成的全量实验不能声明完整 full score。任务成功与测试用例通过
比例是不同指标。三个 benchmark 的结果不平均成一个 headline。配对比较使用同一
任务/重复编号与冻结控制，显示缺失配对和差异，也不跨协议组配对。单 subject
报告不需要虚构比较边或一个缺席的 Agent。

离线字节校验检查原始 seal。报告重建消费经过验证的原生结果，在不联网、不调用
模型的情况下生成确定性输出。实际重新评分使用保存的真实 Agent 产物与固定原生
grader，在新环境中保留任务的 verifier 规则；输出引用原始 bundle 的新派生评分
记录，不改写原分数。缺少必需产物、缓存环境材料，或必需外部服务没有 verifier
允许的离线等价方式时，拒绝离线重新评分。仅校验字节不能满足这项要求。

## 7. 要求与验收场景

以下每项规范义务都有负责人和可观察验证路径。验收标识符是任务图提取的来源
条件。上述操作描述的是要求交付的行为，不是声称这些命令已经发布。

| ID | 要求 | 负责人 | 机械验证 |
|---|---|---|---|
| P19-AC-001 | CLI **MUST** 从独立本地目录完成校验、材料准备、执行和报告，不依赖仓库 fixtures 或 GitHub 执行元数据。 | Evaluation product owner | 使用声明的外部材料与本地输出目录，完成 assembled CLI 验收。 |
| P19-AC-002 | 原生结果 **MUST** 绑定实际选定 Agent 及其交付产物；缺失/错误身份或 oracle 结果 **MUST NOT** 成为 Agent 分数。 | Native integration owner | 通过生产结果读取入口注入缺失/错误 Agent、替换产物、旧 job 和 oracle 输出，逐一拒绝。 |
| P19-AC-003 | TB2.1 **MUST** 按任务原生生命周期评分真实 trial 环境。 | TB2.1 integration owner | 正确/错误 native 样例，以及两个真实 Agent 的 trace、工具效果与评分产物。 |
| P19-AC-004 | TB3 **MUST** 在任务要求时保留声明的 collection 与独立 verifier 语义。 | TB3 integration owner | Native 样例只改变声明产物，无关文件不进入 verifier；两个真实 Agent 均经过该交接路径。 |
| P19-AC-005 | DeepSWE **MUST** 在声明的全新 verifier 中评分收集的已提交 patch。 | DeepSWE integration owner | 正确/错误 patch 样例，base/patch/verifier 身份检查，以及两个真实 Agent 的 patch 到结果 trace。 |
| P19-AC-006 | Provider 投影 **MUST** 为每个 Agent 消费选定协议、端点、精确模型、凭据引用和生效控制。 | Agent integration owner | 两个 native Agent 客户端对三个协议的本地 fake server 运行；拒绝缺失凭据/控制不匹配；首选连接有真实工具循环证据。 |
| P19-AC-007 | 资源预检和选择 **MUST** 保留每项任务的要求与显式 full/subset 范围。 | Material and selection owner | 不调用模型，枚举固定版本完整清单；拒绝一项未满足的 TB3 资源要求；运行冻结的可行子集且不改变限制或范围。 |
| P19-AC-008 | 报告 **MUST** 保留原生指标含义、失败分类、分母、配对覆盖、单 subject 结果与遥测 unknown 原因。 | Report owner | 公共报告入口 fixture 覆盖有效零分、partial/full/subset、缺失配对、provider 失败和协议不匹配。 |
| P19-AC-009 | 校验、重新统计和原生重新评分 **MUST** 是独立、可离线执行的操作，保留源证据不可变。 | Evidence owner | 篡改拒绝、确定性重建、对保存产物进行新的 native regrade、缓存缺失拒绝，且 Agent/provider 调用数为零。 |
| P19-AC-010 | 凭据、监督、取消和恢复 **MUST** 遵守本地授权边界，并保留效果与证据。 | Lifecycle and privacy owner | Grader/export 中没有 canary 凭据；监督取消只清理 trial 自有进程/资源；恢复保留效果未知，替换使用新身份。 |
| P19-AC-011 | 默认测试和 CI **MUST** 保持确定性且无需凭据；本地真实验收 **MUST** 显式、独立进行。 | Test 与 Evaluation product owners | 填入 dummy 凭据的环境不启用真实测试；本地 native 检查无需 dispatch；真实验收报告有独立标识。 |
| P19-AC-012 | Phase exit **MUST** 包含至少六个真实 Agent trial：每个 benchmark 一个可行任务，Opi/pi 各使用首选连接执行一次，并完成原生评分。 | Evaluation product owner 与 local operator | 冻结运行 manifest、脱敏 provider/tool 证据、六个可归因原生结果、bundle 及可重现本地报告。 |
| P19-AC-013 | Companion **MUST** 保持独立归属，并有意记录发生变化的 0.x 契约。 | Companion owner | Cargo 依赖/包内容审查；无 Opi 的 pi-only 验收；Minimal Runtime 不变；实际变更同步到 help、schema、fixture、双语 README 与 Unreleased。 |

六次 trial 证明产品闭环，不证明统计显著性。真实 Agent 可以解题失败并得到零分，
这仍可满足执行条件。鉴权失败、没有工具循环、无效产物交接或基础设施失败，不能
替代六次中的任一次。即使需要替换 trial，失败尝试也保留在证据中。

## 8. 验证与本地验收交接

任务图准入将每项条件分配给拥有该场景的纵向交付切片，明确 CLI/测试命令及真实
生产调用点。已有 `assembled_run`、`native_driver`、Agent/benchmark conformance、
bundle、report 和 authority 测试在能够覆盖目标路径时优先复用。新增测试必须填补
行为缺口，不能仅复制已有断言并换成 Phase 名称。测试不调用付费 provider，也不
因环境中恰好存在凭据而启用真实执行。

原生产物正反例在本机使用固定外部材料运行，不调用真实模型。CI 可以运行显式
限定的 native conformance smoke，但它不是产品运行环境或用户分数的来源。已有
GitHub workflow 入口可以保留为调用同一本地路径的薄回归入口，不能拥有第二套
材料化或评分策略。

真实验收由另行授权的 local operator 在本机运行 Companion，位于禁止 provider
调用和读取真实 auth 文件的 `opi-implement` harness 之外。Harness 可以离线验证
提供的脱敏证据；本规范不豁免其限制。真实证据出现且通过验证前，`P19-AC-012`
保持未满足，不能通过 Phase exit。无需 API key 即可推进纯确定性开发。

运行前，local operator 冻结实际任务、Agent build、runner/材料摘要、模型控制、
并发、用量/时间预算与凭据引用。已经研究过的候选任务为
`openssl-selfsigned-cert`、`batched-eval-parity` 和 `abs-module-cache-flags`；
它们在本地资源/镜像检查及操作者选择完成前仍是候选。所用账号的模型访问和计费
产品资格仍是显式输入，不能仅凭端点推断。来源登记不授权使用凭据或消费额度。

本地验收交接保留命令、退出码、脱敏日志、请求/生效配置、材料与 bundle 摘要、
逐条件结果及报告路径。动态声明需要已保存产物；源码推断和旧 workflow 成功不能
关闭这些条件。运行证据留在本规范之外，使用项目工作流的证据/快照位置，以及
必要的本地私有存储。

Phase exit 还要求对最终相关状态执行一次仓库验证并集：

```sh
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

使用仓库外部 Cargo 缓存和平台 wrapper。WSL/Linux 产品路径必须有真实本地验收，
Windows/macOS 编译或托管 Linux 成功不能替代它。仅塑形文档来源时只使用文档
检查。这里的命令是后续验证要求，不表示编写本文时已经执行。

## 9. 风险阈值、回退与兼容

以下任一情况出现一次，即阻止受影响分数与 Phase exit：归因为 oracle/其他 Agent、
verifier 输入被替换或未绑定、接受篡改证据、静默回退模型/协议/资源、将无效 trial
计为 Agent 成功/失败、把子集标成 full、向 grader/export 泄露密钥，或声称取消后
仍有无界/孤儿执行。容忍数量为零。缺失必需 trial 或无法验证必需测量时，阻止
相应验收或声明，不能用其他分数抵偿。

预算、授权、来源或隐私边界遭到破坏时，停止新的受影响 trial，通过监督取消
自有执行，保留已观测结果和私有诊断证据，并停止输出无效报告。资源清理只涉及
已识别的 trial 自有容器/进程和临时凭据；保留的 bundle、用户目录及共享缓存不属于
清理目标。金额限制保留第 4 节所述强制执行能力边界。

回退会停用受影响的可选本地执行路径，或恢复之前验证过的 Companion build。
如果旧 build 存在已知评分归因缺口，它只能用于准确标注的 conformance 和字节
校验，不能恢复发布 Agent 成绩。不能同时激活相互竞争的新旧评分路径，也不能
静默切换。重新开启真实执行需要修正后的身份和显式运行范围。

已有 Opi/pi 配置、凭据、会话与普通运行行为保持不变。原始 bundle 和报告逐字节
保留；新 reader 将旧证据标识为 legacy/conformance 或 unsupported，不能仅靠
重新标注提升其分数声明。实验、材料、bundle、report 或 CLI 契约变化属于有意
限定的 0.x 变更，同步更新 schema、调用方、文档及 Unreleased 历史。不要求自动
迁移或兼容外观层。

## 10. 交付分解与准入交接

以下是来源要求的交付结果，不是已安装任务或第二套进度账本。在 `opi-implement
plan` 中派生任务 ID 和必要拆分，并在任务图中保留来源 ID。每个拆分都有独立
可观察的完成条件，且必须位于其场景负责人的依赖闭包内。

| 来源 ID | 交付结果与负责人 | 前置条件 | 验收/完成证据 |
|---|---|---|---|
| P19-D01 | 所选 Agent 的 native trial 与 TB2.1 分数绑定；native integration owner | 已有 Companion 基础 | `AC-002`、`AC-003`：CLI 到 native 的正确/错误样例，以及错误/oracle 身份拒绝。 |
| P19-D02 | 经两个真实 Agent 客户端投影用户连接；Agent integration owner | D01 的 resolved trial identity | `AC-006` 及 `AC-010` 凭据部分：隔离 native 配置的三个协议 fake-server 循环；所选真实连接留到 D08 验证。 |
| P19-D03 | TB3 声明产物交接；TB3 integration owner | D01 | `AC-004`：跨规定环境边界完成可归因原生评分。 |
| P19-D04 | DeepSWE 已提交 patch 交接；DeepSWE integration owner | D01 | `AC-005`：base/patch/pristine-verifier 来源及原生正反例。 |
| P19-D05 | 独立本地准备、全量预检与显式多任务选择；local workflow owner | D01–D04 的 configuration/task/material 契约 | `AC-001` 的准备/选择部分及 `AC-007`：经过校验的本地材料与冻结选择，不依赖 fixtures 或 GitHub receipt。 |
| P19-D06 | 原生结果报告与离线产物操作；evidence/report owner | D01–D04 | `AC-008`、`AC-009`：如实覆盖、单 subject/配对报告与真正的离线重新评分。 |
| P19-D07 | 有限恢复与 CI/产品入口对齐；lifecycle/product owner | D02、D05、D06 | `AC-010`、`AC-011`、`AC-013`：取消/凭据/独立性检查、可在本机复现的确定性关卡，以及同步的用户文档。 |
| P19-D08 | 本地验收证据验证与 Phase exit；Evaluation product owner | D02–D07 及另行授权的操作者证据 | `AC-012` 和全部先前条件：六次真实 trial、原生结果与离线重建通过验证；必需 workspace 关卡通过。 |

D01–D07 关闭各自声明的确定性/native 接线证据。D08 拥有 `AC-001` 的 assembled
真实流程、`AC-003`–`AC-006` 的真实 provider 部分及 `AC-012`；较早组件的样例
通过，不关闭这些真实产品场景。

来源准入不要求凭据或购买 provider 套餐；它们是运行时输入，不是产品设计缺项。
授权运行前冻结精确控制和任务选择，不能由实现者猜测。任何要求新增 core seam、
不同原生语义、更广授权或降低验收标准的发现，都依据 `PHASE-006` 返回人主导
塑形。Learning、更大性能研究、其他协议的真实资格验证、发布与原生 Windows
执行继续属于独立后续范围。

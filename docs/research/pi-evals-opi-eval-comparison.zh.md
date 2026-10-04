# pi-evals 与 opi-eval：独立性、设计目标与可借鉴机制

本文是非规范性研究，不是实施 spec、准入来源或进度账本。研究日期：2026-10-03。Pi 基线为本地 `.repo/pi-1.0.0/` 快照；包清单声明版本 1.0.0，但仅凭目录名不证明其对应上游 tag。这里的 Pi 结论来自静态阅读，未执行模型调用、Docker、eval 或测试。当前新建中文研究没有既有英文 counterpart；未修改先前的独立 Pi 报告。

Opi 基线为 `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73` 加当前工作区。`opi-eval` 的 README、runner、report、native capture、regrade 与测试等存在未提交修改；本文对这些文件的观察属于当前工作区，不能全部归为已发布版本。官方上游事实由主研究任务通过 GitHub 只读 API 核验，访问日期为 2026-10-03（Asia/Shanghai）。

核心结论：**pi-evals 是 Pi 产品专用评估工程；opi-eval 的规范目标是无需链接任意 Agent 运行时的独立跨 Agent 评估产品。两者应在实验与证据质量上比较，不应以相同功能数量为目标。**独立部署、版本及治理仍需与代码依赖独立分开判断。

## 1. Pi 的独立性：独立工作区包，不是 Agent 中立评估产品

`@earendil-works/pi-evals` 声明 `private: true`，以 npm scripts 提供 host eval、docs eval 和自身单元测试；没有发布给外部消费者的 `exports`、`bin` 或独立 SDK 入口。README 将其定位为 Pi coding agent 的行为评估，基础框架是 `vitest-evals`。[manifest](../../.repo/pi-1.0.0/packages/evals/package.json#L1)、[定位](../../.repo/pi-1.0.0/packages/evals/README.md#L1)

| 独立性维度 | 本地快照事实 | 判断 |
|---|---|---|
| 目录及测试组织 | 单独的 workspace package、eval 和 test scripts | 有组织边界 |
| 产品中立契约 | harness 直接使用 Pi 的 `AgentSession`、`ModelRuntime`、`SessionManager`、凭据目录与 reload | 不具备 |
| 依赖中立 | manifest 直接列出 pi-ai、pi-coding-agent；其位置是 devDependencies，但实际 runner 会 import | 不具备；不能把 devDependencies 当作运行时无依赖 |
| 构建及仓库脱离 | Docker 从仓库根构建，并调用根级打包及 consumer-install machinery | 不是拷贝包目录即可运行 |
| 被测运行隔离 | docs 各 arm 使用新的容器、home、workspace、agent/session 目录 | 有明确实验隔离，但不是与 Pi 产品解耦 |
| 底层通用性 | Vitest、vitest-evals 承担用例、harness、judge 和报告解析基础 | 通用基础来自依赖，Pi 层提供产品专用适配与实验政策 |

证据：[harness imports 与 options](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L8)、[workspace 构建](../../.repo/pi-1.0.0/packages/evals/docker/Dockerfile#L1)、[consumer 安装](../../.repo/pi-1.0.0/packages/evals/docker/install-runtime.mjs#L5)、[内部 tarball overrides](../../.repo/pi-1.0.0/scripts/coding-agent-consumer.mjs#L41)。

宿主侧 Vitest 还引用仓库根 `vitest.base.ts` 并将 coding-agent 别名指向当前源码；容器分支刻意不应用该源码 alias，改测实际打包安装的 dist。[配置](../../.repo/pi-1.0.0/packages/evals/vitest.evals.config.ts#L1)、[dist 入口校验](../../.repo/pi-1.0.0/packages/evals/docker/entrypoint.ts#L104)

**设计解释：**Pi 选择了“内部产品评估工程”，不是“支持任意 Agent 的评估底座”。这是一种针对具体问题的边界选择，不能仅因未独立发布判定其设计失败；但也不能将它直接作为独立 opi-eval 的包结构模板。

## 2. 实际解决的问题：两条评估路径

| 路径 | 问题与执行方式 | 结果含义 |
|---|---|---|
| `eval:host` | 在宿主 Vitest 运行普通 `.eval.ts`：最小模型连接 smoke，以及逐页文档与运行实现的一致性审查 | 普通 suite 结果；不产生 docs 成对 lift |
| `eval:docs` | 在 `without_docs` / `with_docs` 两种隔离环境中完成相同 Pi 操作任务 | 比较 Pi 文档资源及其 prompt 路由对任务成功率和消耗的影响 |

`npm run eval` 先 host 后 docs，`&&` 意味着 host 失败则不继续 docs。两个分支均需要显式 provider/model；容器 docs 还要求指定 provider 在 Pi `auth.json` 中有条目。[scripts](../../.repo/pi-1.0.0/packages/evals/package.json#L6)、[model selection](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L70)、[auth prerequisite](../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L67)

Host smoke 不开工具，检查 Paris、无错误、模型身份及非零 token。[smoke](../../.repo/pi-1.0.0/packages/evals/evals/smoke.eval.ts#L5)

Host 文档审查要求 Agent 阅读完整页面并追踪运行路径，仅报告明确的、用户可见的矛盾；通过专用工具提交 `match`、`mismatch` 或 `inconclusive`。最终断言只接受 `match`，因此 inconclusive 同样不会通过。这是模型辅助的调查及检查，不能当作确定性证明文档全部正确。[调查要求及断言](../../.repo/pi-1.0.0/packages/evals/evals/documentation-audit.eval.ts#L43)

Docs 当前有五个场景，各一个显式 case：

| 场景 | 判断真实完成的方式 |
|---|---|
| 新建工具扩展 | 配置后 reload，再请求实际调用 hello；检查扩展错误、工具已加载、真实工具结果及调用参数 |
| 给内置 provider 加模型 | reload 后从 ModelRuntime 读取模型字段，同时检查原有模型仍存在 |
| 加 OpenAI-compatible provider | 通过真实 ModelRuntime 对本地 fixture server 发起请求，检查凭据、请求形状、探测标记和响应 |
| 加自定义流 provider | 给出本地 API 文档，要求生成配置/扩展；reload 后验证自定义流可调用 |
| 改 TUI context footer | 装配实际 InteractiveMode 与 RecordingTerminal，用三种 context usage 输入检查进度条行为 |

证据：[扩展](../../.repo/pi-1.0.0/packages/evals/evals/extensions.docs.eval.ts#L9)、[模型](../../.repo/pi-1.0.0/packages/evals/evals/models.docs.eval.ts#L24)、[OpenAI provider](../../.repo/pi-1.0.0/packages/evals/evals/openai-provider.docs.eval.ts#L35)、[custom provider](../../.repo/pi-1.0.0/packages/evals/evals/custom-provider.docs.eval.ts#L43)、[TUI](../../.repo/pi-1.0.0/packages/evals/evals/tui.docs.eval.ts#L113)。

**问题边界：**这些用例主要评估 Agent 利用 Pi 的可配置性和扩展接口完成产品任务的能力，而非通用代码 benchmark、任意 Agent 横评、TUI 全像素回归或生产质量认证。

## 3. 因果比较与隔离：值得借鉴，也需要准确表述

两种镜像安装相同本地 workspace tarballs。control 移除 coding-agent 的 README、CHANGELOG、docs、examples；harness 同时移除 system prompt 中 `<docs>` 路由段。treatment 保留上述资源及原 prompt。[镜像差异](../../.repo/pi-1.0.0/packages/evals/docker/Dockerfile#L12)、[prompt 干预](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L494)

因此，**实际干预是“文档资源集合 + 文档发现指引”的组合**，不能从其结果单独推断“某篇文档文字的净效果”，也不能排除预训练知识影响。要分别估计文档内容与发现机制，需要另行设计实验；当前代码没有这个因子分解。

防止替代信息泄漏的措施包括：

- 两个 arm 对称移除其他内部依赖包中的 docs、examples、src、tests、README、CHANGELOG。[runtime staging](../../.repo/pi-1.0.0/packages/evals/docker/install-runtime.mjs#L31)
- 验证 image allowlist、文档在 control 缺失、treatment 存在，验证 coding-agent 从 dist 加载。[启动检查](../../.repo/pi-1.0.0/packages/evals/docker/entrypoint.ts#L26)
- evaluator 源码、fixtures 与配置由 root 持有且对普通 UID 不可读；prompt 前永久降权，还处理 Vitest transformed modules。[Docker 权限](../../.repo/pi-1.0.0/packages/evals/docker/Dockerfile#L24)、[降权](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L130)
- 每次新建 workspace/home/agent/session，清除 `PI_EVAL_*` 环境；默认只提供 read/write/edit/grep/find/ls。[环境](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L82)、[运行目录](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L272)、[工具集](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L484)

docs Docker 开启只读 root filesystem，写入集中在 tmpfs 和 artifacts；没有声明 `--network=none`，因为 provider 调用需要网络。README 明确承认 Docker 不能证明任意生成代码永不联网。扩展 reload 又可能运行生成的扩展代码，所以默认不暴露 shell 不等于完全禁止任意代码执行。[容器参数](../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L91)、[网络边界声明](../../.repo/pi-1.0.0/packages/evals/README.md#L88)、[扩展 reload 场景](../../.repo/pi-1.0.0/packages/evals/evals/extensions.docs.eval.ts#L45)

**可确认的保证**是具体环境和 evaluator 文件可见性措施，不是完整的恶意代码安全边界，也不是所有外部变量均已控制的因果证明。

## 4. 实验计划、结果完整性与证据保存

执行前在两镜像发现用例，要求 file/name cohort 完全一致，拒绝重复或歧义 case identity，再生成全部 `(evalSet, caseId, variant, model, runNumber)`。每个 case、每次 repetition 都有两 arm，即 `2 × cases × repetitions` 次运行；奇偶 repetition 交替先跑 control/treatment，实际 CLI 串行执行。[发现比对](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L106)、[plan](../../.repo/pi-1.0.0/packages/evals/src/plan.ts#L21)、[执行](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L170)

在第一 arm 执行前写入 `protocol.json` 和 `expected-runs.json`。protocol 包含镜像 ID、model、files、cases、tasks，并对 JSON 文本做 SHA-256。该 digest 标识本次协议内容，不是密码学签名，也不是整个环境可复现的证明。[protocol](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L153)

常见失败 arm 不会抹去预定分母：容器有报告就读取，无报告生成 errored observation，再继续任务。实现会忽略容器非零退出码本身并检查产物；但 Docker 启动错误、某些文件读写异常仍能使外层异常中止，不能泛称任何故障都保证跑完全组。[runTask](../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L161)、[execute](../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L23)、[observation loop](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L170)

报告解析先核对唯一 assertion、fullName、唯一 harness case、status 一致和实际模型身份。skipped/pending 在不存在 harness run 时仍保留；存在 score 但 run 有 errors 仍算 errored；0 分是有效 scored data，而非基础设施失败。[parser](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L136)、[覆盖这些情况的测试源码](../../.repo/pi-1.0.0/packages/evals/test/report.test.ts#L115)

每对 arm 必须恰好预期一次、观察一次且均 scored；缺失、额外、重复、skipped、pending、unscored、errored 都阻塞该 pair。任一 pair 阻塞，该 eval set 的 headline pass rates/lift 为 null，CLI 最终非零退出。[pair validation](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L249)、[headline gating](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L359)、[exit](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L192)

结果保存原生 Vitest JSON、归一化 observations、protocol、expected cohort 和 Pi session JSONL。原生 session 附件存在时落盘，但 parser 允许“有效评分但没有 session 附件”；因此不能将其描述为严格的证据包完整性门槛。[session 保存](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L122)、[允许缺失附件的测试](../../.repo/pi-1.0.0/packages/evals/test/report.test.ts#L204)

harness 还捕获 cleanup/dispose 失败并与原执行错误组合。但 runDiagnostics 是完成 prompt steps 后才建立；如果更早中断，不能保证已有 token/time 诊断都进入规范化 observation。session 尝试在 finally 读取，也不等于每种基础设施故障均能恢复全部证据。[diagnostics 与 finally](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L389)、[failure attachment](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L449)

## 5. 统计及评分边界

score 输入允许 `[0,1]`，但 headline pass rate 只把 `score >= 1` 算通过；lift 是 treatment pass rate 减 control pass rate。它不是平均 score 差，也不是相对百分比提升。[score validation](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L93)、[统计实现](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L379)

tokens、tools、latency、estimated cost 的配对均值只使用双方都有该 metric 的 eligible pair；另有按 variant 汇总的 operational totals，覆盖有可用指标的失败运行。缺失指标为 unavailable/null，与观测为 0 严格区分；每个指标都有实际覆盖数。质量分母和资源消耗分母可以不同，应一同解释。[paired metric](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L283)、[operational totals](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L306)

estimatedCost 仅在模型配置存在正定价时输出；全零定价不作为“已测免费”的确定证据。harness 通过 stats 采集 usage，不意味着所有 provider 的上游缺失值仍能一路保真；报告层能正确保留的是它实际收到的缺失状态。[pricing 判断](../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L391)

默认每个 variant 只跑一次。flags 包含 no-lift、negative-delta、control/treatment saturated、flaky；flaky 仅检查有效成对样本内某 case/variant 是否同时观察到满分通过与未满分，不是完整连续分数方差分析。当前 report 没有置信区间、显著性检验、功效分析、序贯检验或多重比较修正。[默认 repetition](../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L79)、[flags](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L336)、[report schema](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L43)

TUI judge 检查三个 usage fixture 的进度条及 clamping，并计算其他 status 文本的 Levenshtein retainedPercent；**retainedPercent 仅写 metadata，不参与 `valid ? 1 : 0` 得分门槛**。因此不能将该用例称为“保证其他界面内容完全不变”的视觉验收。RecordingTerminal 还 strip ANSI，因此不是像素、颜色或完整终端状态等价测试。[TUI judge](../../.repo/pi-1.0.0/packages/evals/evals/tui.docs.eval.ts#L192)、[录制边界](../../.repo/pi-1.0.0/packages/evals/evals/tui.docs.eval.ts#L52)

**统计解释：**这套报告适合定位固定 cohort 的观测差异、完整性问题和成本信号。它不直接证明文档造成稳定泛化提升，也不提供将默认单次结果推广为产品级结论所需的不确定性估计。

## 6. 可迁移的设计经验（建议，不是现行要求）

| Pi 中可确认的机制 | 对独立评估工具可借鉴的原则 | 不应直接复制的绑定 |
|---|---|---|
| 先冻结 cohort 和任务，执行后对账 | planned、observed、graded 分开，避免幸存者偏差 | docs 两种固定 variant 名称 |
| 相同任务、环境与真实安装产物 | 评估可发布系统行为，记录环境及输入证据 | 全仓 npm tarballs 与 Pi dist |
| deterministic output probes | 验证配置实际可用、工具实际执行，而不是只信模型自述 | `ModelRuntime`、Pi reload/extension 类型 |
| judgeThreshold null | 低分是质量数据，运行/评分故障另分类 | 将所有应用都固定为满分即成功 |
| 严格 pair 完整性 | 缺样必须显式阻塞、标注或采用预定义缺失策略 | 强迫所有类型实验都采用同一种 headline withholding |
| 缺失 telemetry ≠ 0 | 单独显示指标覆盖与实验质量 | 供应商计费估算当作实际账单 |
| root-owned evaluator 与降权 | 被测 Agent 应无法读取隐藏答案；把隔离效果做检查 | 把 Docker 等同完整恶意代码隔离 |
| 原生会话与规范化结果并存 | 保留原始证据用于调查和重新解释 | 评估核心依赖 Pi session 格式 |
| 任务与 runner 分离 | 场景作者专注 setup/task/oracle，基础设施拥有 identity/isolation/report | 宿主框架必须是 Vitest |

上述建议分别来自前述 plan、harness、report 和 Docker 实现。对 opi-eval 的选择应以它自身的独立产品目标及现有真实消费者为准；不能以“Pi 已有”为由自动增加 Docker、LLM judge、插件接口或统计框架。

## 7. 本地文档有效性及未来规划证据

本地 `packages/evals` 只有 README 这一份 Markdown，没有独立 CHANGELOG、设计文档或 roadmap 文件。对 README、五个 src、Docker、eval definitions、test 源码检查后，主要 runner 流程与 README 一致；上文标出了需收窄理解的“失败继续”“session artifacts”“通过率”“TUI 保留”等边界。[README](../../.repo/pi-1.0.0/packages/evals/README.md#L1)

已实现的、较有追求的方向是：以受控实验量化产品文档是否帮助 Agent 学会扩展和配置自身，并用实际 runtime probe 评分；同时让 Agent 调查文档与实现是否矛盾。[文档实验](../../.repo/pi-1.0.0/packages/evals/README.md#L72)、[模型辅助审查](../../.repo/pi-1.0.0/packages/evals/evals/documentation-audit.eval.ts#L43)

**以下只是从现有能力推导的可能方向，不是上游承诺：**自动比较文档/提示词修订、把失败轨迹反馈给文档维护、扩大自配置任务集、引入更严格统计与持续回归。现有 src/README 没有证明上游已接受这些路线，更没有证明要做跨 Agent 通用平台、自动优化闭环或公共排行榜。

官方上游调查补充了以下证据；它们与本地固定快照分开陈述，不用 PR 标题替代合并状态与正文：

调查检查了官方当前包目录、eval/evals/benchmark 相关标题及正文、相关 PR 与 issue 评论；截至查询，官方包目录也只提供 README 这一份 Markdown，未发现独立 roadmap。README 最近实质修改为 2026-09-15 的 [d7296c0](https://github.com/earendil-works/pi/commit/d7296c063b7971a7298769cfff1a167a9a16f8ed)，对应隔离 docs lift 的变更；“未找到”是本次检索范围内结论，不是保证不存在未公开规划。

| 时间与状态 | 一手证据 | 能支持的判断 |
|---|---|---|
| 2026-07-25 已合并 | [PR #7085](https://github.com/earendil-works/pi/pull/7085) | 最初明确是 private workspace package，复用 vitest-evals 与 Pi SDK，随 workspace 同步版本；支持“内部 Pi 评估”定位 |
| 2026-07-30 已合并 | [PR #7289](https://github.com/earendil-works/pi/pull/7289) | comparative Pi eval harness 讨论 seeded、重复运行、多 harness、各 harness model/prompt，以及分别展示 score lift、tokens、latency、cost；历史设计不能直接覆盖当前 docs 专用执行路径 |
| 2026-09-07 已合并 | [PR #9280](https://github.com/earendil-works/pi/pull/9280) | 增加基于实现的文档审查；模型给出结构化 verdict 仍不是形式正确性证明 |
| 2026-09-15 已合并 | [PR #9635](https://github.com/earendil-works/pi/pull/9635) | 收紧 docs lift 的容器隔离、host/docs 分流、预定 cohort、blocked pairs 等，对当前源码提供演进解释 |
| 2026-09-30 创建，截至查询仍 OPEN、非 draft、未合并 | [PR #10261](https://github.com/earendil-works/pi/pull/10261) | 提议增加 prompt-template docs-lift case、支持多 case 报告中选定目标 case、将昂贵审查改为显式 `eval:audit`，并移除冗余 oracle/test-only exports；是可审阅近程提案，不是已发布能力 |

PR #10261 中作者自报的测试结果不是本次实测。本地 `readTaskObservation` 目前要求报告只有一个 assertion/case，与该 PR 正在提出的多 case 选择行为存在时间上的区别；不能将未来修改倒填成本地现状。[本地 parser](../../.repo/pi-1.0.0/packages/evals/src/report.ts#L145)

另一个容易误读的记录是 [Issue #2002](https://github.com/earendil-works/pi/issues/2002)：它在 2026-03-09 提议 dataset splits、plugin evaluators、CI variance、quality/cost policy gates 以及 `/eval`、`/compile`、`/promote` 等内置工作流；维护者 badlogic 明确拒绝：“no, i'm sorry, this will not happen.” [维护者回复](https://github.com/earendil-works/pi/issues/2002#issuecomment-4026307302)。其关闭状态中的 COMPLETED **不代表已实现**。这只能证明该历史内置工作流提案被拒，不能推导之后的 eval 包被否定或未来不会做任何其他评估能力。

**对“富有野心的未来规划”的回答：**有依据的近期方向是加强文档驱动的自配置评估、扩展场景、控制成本并提高实验有效性。本次未找到已经承诺的跨 Agent 通用平台、托管排行榜或自动学习/晋升闭环路线图。它们可以成为使用者自己的产品设想，但不能冠以上游规划之名；`vitest-evals` 依赖框架的计划也不等于 `pi-evals` 的计划。

## 8. Opi 的目标、已实现边界与独立性缺口

现行规范明确要求 Evaluation 无 Opi crate 依赖，以冻结的 experiment identity、Agent/grader integrations、不可变 bundle、benchmark native grader 和离线报告重算为核心；最终名称、仓库和品牌仍交给 Placement Review。[规范 §6.2](../opi-spec.md#L263)

| 独立性维度 | Opi 当前证据 | 结论及尚未证明之处 |
|---|---|---|
| 代码依赖 | `Cargo.toml` 无 Opi crate 依赖；其他产品 manifests 没有反向依赖 opi-eval | 代码依赖方向已经与目标一致 |
| 被测运行时 | opi/pi adapters 通过 argv、环境、进程及保存的原生产物交互，不链接对应运行时 | 有实际进程边界，并非仅改名独立 |
| Agent 扩展 | adapter 契约是 `pub(crate)`，当前 registry 只接受 opi/pi | 是封闭的已知集成，不是开放插件 SDK；在准入前暂不稳定符合规范 |
| benchmark 接入 | 当前有 Terminal-Bench 2.1、3.0 与 DeepSWE 对应模块和 native-material 路径 | 不能说只有 mock；但有代码/fixture 不等于真实产品验收完成 |
| 离线证据 | 存在 seal verify、report 和独立 native regrade 路径 | 验证保存证据与再次运行 Agent 必须区分 |
| 分发与版本 | `publish = false`；version/edition/MSRV/dependencies 来自 workspace；README 要求仓库根运行 | 尚未证明仓库外安装、独立版本或独立发布 |
| 使命与治理 | CONTEXT 对 Independent Companion/Standalone Project 作了更强定义 | 不能仅凭一个 crate 和进程边界宣称治理与公共身份已独立 |

证据：[manifest](../../crates/opi-eval/Cargo.toml#L1)、[进程适配边界](../../crates/opi-eval/src/agent/mod.rs#L1)、[封闭 registry](../../crates/opi-eval/src/runner/experiment.rs#L3727)、[benchmark 模块](../../crates/opi-eval/src/benchmark/mod.rs#L7)、[运行前提](../../crates/opi-eval/README.md#L41)、[领域定义](../CONTEXT.md#L24)。

规范 CTRL-007 要求至少两个真实 Agent（含非 Opi）和两个 benchmark-native grader 通过共享 conformance 后，再稳定公开 seam；其形态甚至可能是 Rust trait、进程 envelope、CLI 约定或仅 conformance fixtures，不应预先锁定大型 SDK。[准入](../opi-spec.md#L269)、[CTRL-007](../opi-spec.md#L307)、[STRAT-002](../opi-spec.md#L634)

README 明确区分 hermetic fixture 与 native-material。前者证明 adapter/lifecycle/sealing/failure-path 等机制；不能冒充真实产品、provider 或官方 benchmark 环境。后者确实有运行所选 Agent 并在相同任务环境中 native grade 的实现路径；本次没有核验真实验收收据，故不声称已经达到上述稳定门槛。[模式说明](../../crates/opi-eval/README.md#L405)、[fixture 边界](../../crates/opi-eval/README.md#L73)、[另行授权的真实验收](../../crates/opi-eval/README.md#L704)

离线工作也有三个不同动作：`verify` 重读保存的字节并重算 seal 身份；`report` 从保存证据重建结果；`native regrade` 在新环境中用固定 verifier 重判保存的 Agent 产物并保存派生 grading record，不重跑 Agent，不覆盖原始 score。[verify/regrade 契约](../../crates/opi-eval/src/regrade.rs#L1)

版本对齐需要单独核验：Pi adapter 注释及 profile 仍写 v0.85.1 CLI surface，而 external lock 固定另一个具体 commit。不能因此直接断言代码过时，也不能假定本地 Pi 1.0.0 已获 conformance 覆盖；应核对 pinned executable、实际输出 schema 和用例结果。[adapter](../../crates/opi-eval/src/agent/pi.rs#L1)、[profile](../../crates/opi-eval/profiles/agents/pi.toml#L4)、[external lock](../../crates/opi-eval/external-locks/static/linux-x86_64.json#L63)

某些报告事实投影仍需检查跨 Agent 对称性：`NATIVE_FACT_SOURCES` 在 pinned hermetic profiles 中仅为 Opi 登记 usage/cost/retry/compaction 的原生产物来源，未登记产品保持 typed unknown。该事实仅限这个投影表，不表示其他 native 路径中的 Pi 所有 usage 都缺失。[范围明确的来源表](../../crates/opi-eval/src/report.rs#L56)

## 9. 两者设计目标差距与选择建议

| 问题 | pi-evals 当前选择 | opi-eval 现行目标 | 应如何比较 |
|---|---|---|---|
| 为谁服务 | Pi coding-agent 维护和文档任务 | Agent 中立、跨产品评估 | 不追求同包结构或相同功能清单 |
| 成功由谁定义 | 各 suite 的 deterministic judge/host assertions | 已准入 benchmark revision 的 native grader | 文档审查可为诊断，不能自动替代 native headline |
| 实验中心 | 固定 without/with docs 对照 | subject、edge、task、trial 的冻结实验 | 可借鉴 docs 消融作为一个 suite，不把全系统固定为二分文档实验 |
| 原生运行连接 | 在同一 Node 运行时调用 Pi SDK | 外部进程及产物集成 | Pi/Opi 专用类型留在 adapter 或外部 integration |
| 证据要求 | JSON、会话、protocol digest | 内容寻址、不可变 bundle、来源绑定、离线验证 | 不应把 Pi 较轻附件策略降格替换 Opi 已有契约 |
| 失败与质量 | blocked pairs、完整率、成本覆盖 | 还需区分 invalid benchmark、infra failure 与 Agent outcome | 共同机制可对照加强，避免建立另一套重复 runner |
| 长期边界 | 无通用独立平台承诺 | benchmark、容器、sandbox、远程调度、榜单、learning policy 在评估模块外 | “有野心”应体现证据可靠和接口中立，而非把所有平台功能收入核心 |

Opi 的 headline 规则比 Pi host audit 更严格：CTRL-004 规定 native grader 拥有 headline，LLM judge 只能是独立标记的诊断；CTRL-005 要求缺失、排除、task-integrity 与 infra 分类可见；CTRL-006 要求离线重现、真实重跑必须新 trial。[CTRL-004–007](../opi-spec.md#L299)

**建议的方向：**把 opi-eval 定义为 Agent 中立的实验与证据内核，把产品接入、benchmark 及任务 suite 当作外部或受控集成。独立发行及治理可以分阶段实现，但应明确验收，不把“无内部依赖”当成完整完成条件。

借鉴的优先顺序是：低成本编写真实任务、确定性 oracle、文档和 prompt 的明确干预、严格 pair 及 unknown 处理、保存失败成本与原始产物。Opi 已有冻结实验、sealed bundle、native grader、离线重算等机制，应先比对现有实现而不是再造一条 Pi 风格平行 pipeline。

不建议因参考 Pi 而引入以下默认目标：核心直接依赖产品 SDK、强制 Vitest/Docker、LLM verdict 进入 benchmark headline、固定满分阈值、隐去不完整样本，或把自学习/晋升控制与评估内核合并。

仍需后续证据才能决策的事项：

1. 用一个真实第三方 Agent 接入，验证“中立”不是只为 opi/pi 两个内置适配器定制；不预先承诺插件框架。
2. 用不同 native grader 与 task lifecycle 检查最小共享 seam，以及是否存在可合并的专用 runner/重复证据转换。
3. 明确独立 artifact 的构建、安装、版本、发布和 repo 外离线操作条件；与治理独立分别验收。
4. 检查 Pi 固定版本、模型/权限/预算与外部 benchmark 的实际公平性，不因同名模型或相同 trial 数就宣称公平。
5. 为单次或少量重复输出明确的不确定性提示；是否增加统计方法应由真实使用决策驱动，避免“通用统计平台”先于需求。

范围说明：本次只新建这一份中文研究材料，未修改原独立 Pi 报告、规范、代码、锁文件或进度账本，未运行真实 eval、模型或容器。文档检查由主研究任务在合并研究内容后执行；源码与测试的静态存在不等于本次实测通过。

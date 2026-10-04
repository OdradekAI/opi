# Pi 1.0.0 evals 包分析

## 1 定位与核心结论

`@earendil-works/pi-evals` 是仓库内部的行为评测包，manifest 标记 `private: true`。它以真实 Coding Agent 为受测对象，通过 `vitest-evals` 执行任务、收集会话和用量，并包含一套专门测量“提供 Pi 文档是否提高任务成功率”的成对实验流程。

最有辨识度的机制不是调用模型本身，而是实验身份、对照材料、运行隔离和缺失数据处理：每个 case 的 control/treatment 分别在独立容器中运行；先记录完整计划，再执行；任一配对不完整则保留失败信息并暂不发布该评测集合的 headline pass rate。

本报告是静态源码分析。已阅读 runner、harness、容器构造与入口、主要统计测试和代表性场景；没有构建镜像、运行模型或获得新的评测结果。[package.json:1](../../../../.repo/pi-1.0.0/packages/evals/package.json#L1)、[cli.ts:113](../../../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L113)、[report.ts:359](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L359)。

## 2 两套运行模式

| 模式 | 入口 | 范围 | 执行环境 |
|---|---|---|---|
| host eval | `eval:host` | 非 `*.docs.eval.ts` 的 eval 文件 | 本机 Vitest fork |
| 文档增益实验 | `eval:docs` → `src/cli.ts` | `*.docs.eval.ts` | 每个实验臂一个新 Docker 容器 |
| runner 单元测试 | `test` | `test/**/*.test.ts` | 普通 Vitest，测试 harness/计划/统计等代码 |

`eval` 先执行 host，再执行 docs；host 失败会通过 `&&` 阻止后一阶段。eval 配置关闭文件并行，单 worker、非 concurrent，单测试和 hook 超时都是 300 秒。harness 会改变进程环境和容器 UID，串行约束具有实际意义。[package.json:6](../../../../.repo/pi-1.0.0/packages/evals/package.json#L6)、[vitest.evals.config.ts:4](../../../../.repo/pi-1.0.0/packages/evals/vitest.evals.config.ts#L4)、[vitest.test.config.ts:1](../../../../.repo/pi-1.0.0/packages/evals/vitest.test.config.ts#L1)。

manifest 不声明生产 dependencies，所有依赖放在 devDependencies：内部 `pi-ai`、`pi-coding-agent`，以及 `vitest-evals 0.15.0`、`@vitest-evals/core 0.15.0`、`autoevals 0.3.0`、`vitest 4.1.11` 等。这与 private 工具包定位一致，不能由“没有 dependencies”推断它不需要模型和 Agent 运行时。

## 3 目录与职责地图

| 文件或目录 | 核心责任 |
|---|---|
| `src/cli.ts` | 参数检查、发现一致性、计划持久化、顺序执行、汇总 |
| `src/plan.ts` | case 身份解析和实验矩阵展开 |
| `src/docker.ts` | 镜像构建、认证来源、容器参数、单臂执行 |
| `src/harness.ts` | 将真实 AgentSession 转换为标准 harness run |
| `src/report.ts` | 解析 Vitest 结果、保存会话、配对和统计 |
| `docker/install-runtime.mjs` | 打包当前工作区并安装成独立消费者运行时 |
| `docker/Dockerfile` | 公共 runtime 与两种文档材料变体 |
| `docker/entrypoint.ts` | 检查镜像内容和评测文件访问权限，运行 Vitest |
| `evals/*.docs.eval.ts` | 文档增益场景、确定性评分 |
| `evals/configured-runtime.ts` | 在真实 ModelRuntime 中检查配置产物 |
| `evals/acme-server.ts` | 本地模拟 provider，核验实际请求 |
| `test/*` | 实验基础设施本身的离线行为测试 |

这里的单元测试与真实 eval 是不同层次：前者能证明统计和 fixture 如何工作，后者才产生模型任务表现数据。

## 4 实验数据模型与身份

case 发现结果必须拥有 `name` 和 `file`，名称格式严格为 `<eval set> > <case>`，不能多一级嵌套，也不允许 evalSet/caseId 重复。task 身份增加 variant、model 和 runNumber。[plan.ts:21](../../../../.repo/pi-1.0.0/packages/evals/src/plan.ts#L21)。

```text
DiscoveredEvalCase
  file + fullName + evalSet + caseId
            ↓ 展开
EvalTask
  + model(provider/model) + variant + runNumber
            ↓ 执行
EvalObservation
  + outcome + optional metrics + score(仅 scored)
            ↓ 精确配对
EvalComparisonReport schemaVersion=3
```

对照是 `without_docs`，实验组是 `with_docs`。对于 N 个 case 和 R 次重复，计划生成 `2 × N × R` 个 task。奇数轮先 control，偶数轮先 treatment，缓和固定执行顺序的影响；它不是随机分组，也没有由此获得统计显著性保证。[plan.ts:39](../../../../.repo/pi-1.0.0/packages/evals/src/plan.ts#L39)。

CLI 的模型参数要求成对出现：命令行一旦指定 provider 或 model，就必须同时指定另一项；不从环境补齐半组 CLI 参数。否则使用 `PI_PROVIDER` 和 `PI_MODEL`，也要求成对。重复次数必须是正的安全整数，默认 1。[cli.ts:27](../../../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L27)。

## 5 文档增益实验的完整执行链

1. `parseEvalCli` 检查模型、次数、文件和允许的 Vitest 名称过滤参数。
2. 选取 `*.docs.eval.ts`，确认文件处于包路径内，并找到对应 provider 的认证文件。
3. `buildImages()` 构建两种镜像并记录 image ID，拒绝两组指向同一个 ID。
4. 在两个镜像分别发现 case，按名称和规范化文件路径排序后核对集合一致，不要求原始 discovery 顺序一致。
5. `createTaskPlan` 生成全部 task。
6. 写入 `protocol.json`、`expected-runs.json`，为 protocol JSON 计算 SHA-256。
7. 顺序启动每个 task 的新容器，读取 native Vitest JSON；没有结果则记录 errored observation。
8. 每轮重写当前完整 `observations.jsonl`，最终输出 JSON 与文本报告。
9. 只要存在 blocked pair，就设置非零退出码。

依据：[cli.ts:106](../../../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L106)、[cli.ts:149](../../../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L149)、[docker.ts:137](../../../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L137)、[docker.ts:161](../../../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L161)。

`runTask` 不以容器退出码单独决定观察值；它检查结果文件是否存在，随后交给 report parser 判定 case、harness 和 score 是否有效。构建和 discovery 则要求命令成功。这样可在失败的测试报告中保留部分用量和会话，而不是丢弃整个实验臂。

## 6 对照材料如何构造

Docker builder 使用 Node 24 slim，在临时源码挂载上安装依赖、offline build、打包工作区 public packages，再将 Coding Agent 当作消费者安装。评测运行环境应从已安装 `dist/index.js` 加载，避免源码 alias 悄悄改变实验对象。[Dockerfile:1](../../../../.repo/pi-1.0.0/packages/evals/docker/Dockerfile#L1)、[install-runtime.mjs:15](../../../../.repo/pi-1.0.0/packages/evals/docker/install-runtime.mjs#L15)。

两组的共同处理是移除内部依赖包里的 README、CHANGELOG、docs、examples、src、test/tests，防止这些内容成为另一份操作指南。control 额外移除 Coding Agent 的 README、CHANGELOG、docs 和 examples；treatment 保留这些文件。[install-runtime.mjs:33](../../../../.repo/pi-1.0.0/packages/evals/docker/install-runtime.mjs#L33)、[Dockerfile:11](../../../../.repo/pi-1.0.0/packages/evals/docker/Dockerfile#L11)。

control 还通过隐藏的 inline extension 修改 `before_agent_start` 返回的 system prompt，只移除 `<docs>` 区块；`<rules>` 和 `<cwd>` 保留。标记不完整会报错，不静默生成未知版本的对照提示词。最后用 `verifySystemPrompt` 核对实际发送的提示词是否包含预期文档路由。[harness.ts:285](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L285)、[harness.ts:494](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L494)、[harness.test.ts:72](../../../../.repo/pi-1.0.0/packages/evals/test/harness.test.ts#L72)。

**分析判断：** 实验测量的是“Coding Agent 文档材料加提示词中的文档路由”的组合效果，并非单独测量增加若干文件的效果。这个干预范围应与结果解释保持一致。

## 7 Harness 如何驱动真实 Agent

`createPiCodingAgentHarness` 支持单个字符串，或由 prompt/reload 构成的步骤序列。输出默认是最后的 assistant 文本，也可由 output callback 检查模型生成的配置、扩展和会话，返回结构化 JSON 供 judge 使用。[harness.ts:43](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L43)、[harness.ts:471](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L471)。

一次 run 的主要资源生命周期：

```text
创建临时 root/workspace/home/agentDir
  → 保存并替换环境，移除 PI_EVAL_* 元数据
  → 将所选 provider 凭据复制到 InMemoryCredentialStore
  → ModelRuntime + session services
  → 新 SessionManager + 新 AgentSession，thinkingLevel=off
  → 检查没有加载意外扩展
  → 容器内降权
  → prompt / reload / prompt ...
  → 收集 transcript、usage、prompt hash、会话 JSONL
  → dispose session、删除临时目录、恢复环境
```

关键实现：[harness.ts:272](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L272)。fixture 文件使用相对路径播种，拒绝绝对路径和逃出 workspace 的 `..` 解析结果。[harness.ts:225](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L225)。

`promptAgent` 只接受新增 assistant message，检查 stopReason；非 stop/toolUse 视为失败，stop 而没有文本也失败。AbortSignal 在运行前检查，并在执行期间联动 `session.abort()`；清理前等待已触发的 abort Promise。[harness.ts:238](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L238)、[harness.ts:366](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L366)。

诊断包括消息和工具调用/结果、system prompt SHA-256、token 和工具次数、可能存在的估算成本及总耗时。模型价格全为零时不输出 estimatedCostUsd，以免将未知价格误写成免费。失败且已有 diagnostics 时，使用 `attachHarnessRunToError` 保留部分运行结果。[harness.ts:190](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L190)、[harness.ts:391](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L391)、[harness.ts:451](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L451)。

## 8 容器与信任边界

容器 root filesystem 只读；`/tmp` 和 Vitest 临时目录是 tmpfs，artifacts 是单独 bind mount，认证原文件只读挂载到 root-only secret 目录。启动时检查镜像目录 allowlist、受测包 dist 入口、内部依赖没有残留说明文件。[docker.ts:91](../../../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L91)、[entrypoint.ts:20](../../../../.repo/pi-1.0.0/packages/evals/docker/entrypoint.ts#L20)。

评测代码和 judge/fixture 起初由 root 读取。模型执行前，harness 保护 Vitest 转换缓存、将工作目录交给受测 UID，并永久执行 setgroups/setgid/setuid；随后探测已转换评测模块是否仍可读。这防止模型使用文件工具读取隐藏判定条件。UID 变更不恢复，解释了每臂新进程的必要性。[harness.ts:130](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L130)、[harness.ts:154](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L154)、[entrypoint.ts:93](../../../../.repo/pi-1.0.0/packages/evals/docker/entrypoint.ts#L93)。

文档 harness 默认工具为 read/write/edit/grep/find/ls，不包括 bash 和 web search；调用者可以显式扩展工具。容器仍需访问模型 provider 网络，因此这种配置不能证明生成代码绝无网络访问。认证进入进程内存后，也不能把降权解释成进程内插件与凭据之间的隔离。[harness.ts:485](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L485)、[harness.ts:523](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L523)。

host eval 不具备同等容器隔离。`documentation-audit.eval.ts` 是让模型读取实际仓库并提交判定的本机流程，它的结论质量依赖模型审阅，不等同于确定性静态证明。[documentation-audit.eval.ts:37](../../../../.repo/pi-1.0.0/packages/evals/evals/documentation-audit.eval.ts#L37)。

## 9 结果判定和统计语义

`readTaskObservation` 先检查 native Vitest JSON 中恰好一个 assertion 及正确全名；skipped/pending 在此后直接返回，不要求后面的标准 case 和 harness 资料。其他结果继续检查恰好一个标准 case、状态一致、存在 harness run、实际 provider/model 与计划一致。用量必须是非负有限数字，score 必须处于 `[0,1]`；case failed 或 run.errors 非空优先归类为 errored，即使同时存在合法分数。[report.ts:85](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L85)、[report.ts:136](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L136)、[report.ts:178](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L178)。

| 情况 | observation |
|---|---|
| 通过前置校验、无运行错误且有合法 score，包括 0 | `scored` |
| 正常完成但没有 score | `unscored` |
| skipped/todo/disabled | `skipped` |
| pending | `pending` |
| 缺结果、格式/身份/模型不匹配、run 有 errors、case failed | `errored` |

没有会话 artifact 不会自动使已经完成并有 score 的 observation 失效；有对应测试。会话是重要诊断材料，但 parser 的评分门槛与 artifact 完备性是分开的。[report.test.ts:204](../../../../.repo/pi-1.0.0/packages/evals/test/report.test.ts#L204)。

pair key 为 `[evalSet, caseId, model, runNumber]`，每个 variant 必须预期一次且观察到一次 scored 结果。重复、缺失或非 scored 均成为 blocked pair。[report.ts:209](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L209)、[report.ts:249](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L249)。

headline pass 判定使用 `score >= 1`，不是 `score > 0` 或平均分阈值。完整集合的 lift = treatment pass rate − control pass rate；输出以百分点展示。任一 pair blocked，该 evalSet 的 headline rates 和 lift 为 null，但已具备完整指标的有效 pair 仍可用于单项效率差值。[report.ts:359](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L359)。

tokens、tools、latency、estimated cost 逐项筛选双方都有该值的 pair。缺失就是 unavailable，而不是零。operationalTotals 则汇总所有观察中有值的指标，包括失败实验臂，表达实际消耗而非有效样本表现。[report.ts:283](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L283)、[report.ts:306](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L306)。

flags 包括 no-lift、negative-delta、control/treatment-saturated 和 flaky。flaky 根据同一 case/variant 多次重复是否跨越满分门槛产生；它不会反映都低于 1 的连续分数波动。没有置信区间、显著性检验或跨模型基准结论，默认单次重复无法建立稳定性。[report.ts:336](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L336)。

## 10 场景层如何避免只检查答案文本

| 场景文件 | 任务与判定特点 |
|---|---|
| `smoke.eval.ts` | 禁用工具的简单回答检查 |
| `models.docs.eval.ts` | 添加模型，检查 ModelRuntime 可见且既有模型仍保留 |
| `openai-provider.docs.eval.ts` | 配置 OpenAI-compatible provider，并通过模拟服务确认请求 |
| `custom-provider.docs.eval.ts` | 根据接口 fixture 实现自定义流式 provider |
| `extensions.docs.eval.ts` | 写扩展→reload→实际调用 hello 工具；同时检查注册、参数、工具结果和回复 |
| `tui.docs.eval.ts` | 定制上下文 footer，读取渲染结果判断效果 |
| `documentation-audit.eval.ts` | 按文档页面生成 host 审核任务，要求结构化最终提交 |

源码：[extensions.docs.eval.ts:9](../../../../.repo/pi-1.0.0/packages/evals/evals/extensions.docs.eval.ts#L9)、[configured-runtime.ts:56](../../../../.repo/pi-1.0.0/packages/evals/evals/configured-runtime.ts#L56)、[tui.docs.eval.ts:113](../../../../.repo/pi-1.0.0/packages/evals/evals/tui.docs.eval.ts#L113)。

`inspectProvider` 先刷新真实 ModelRuntime，再查配置模型并执行 completion，既检查标准化响应，也检查 fixture 服务器是否收到合法 probe。`inspectAddedModel` 另建未加自定义配置的 runtime 比较原模型列表，防止“新增成功但覆盖旧模型”误判通过。[configured-runtime.ts:65](../../../../.repo/pi-1.0.0/packages/evals/evals/configured-runtime.ts#L65)、[configured-runtime.ts:95](../../../../.repo/pi-1.0.0/packages/evals/evals/configured-runtime.ts#L95)。

## 11 产物与可追溯性

每次运行写到独立 `.eval/<timestamp>_<uuid>/`。完整产物包括 protocol、expected-runs、observations、每 task 的 Vitest JSON、分 variant 的原生会话，以及 report.json/report.txt。task 目录由完整身份元组的 SHA-256 命名，不依赖可能含路径分隔符的自然语言名称。[cli.ts:89](../../../../.repo/pi-1.0.0/packages/evals/src/cli.ts#L89)、[docker.ts:156](../../../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L156)、[report.ts:122](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L122)。

protocol digest 固定了本次实验的模型、镜像与任务计划，便于对应结果；它不意味着重新执行可获得相同模型输出，也不自动固定远端模型服务内部版本。

## 12 测试证据与阅读路线

| 测试 | 覆盖意图 |
|---|---|
| `plan.test.ts` | 身份规范、重复拒绝、交替顺序、非法模型和次数 |
| `harness.test.ts` | 模型选择、环境恢复、文档片段删除、prompt 验证、容器 gate |
| `report.test.ts` | scored/zero/unscored/error 区分、模型不符、部分失败用量、会话保存 |
| `comparison.test.ts` | 精确配对、headline withholding、缺失与零、重复观察和消耗保留 |
| `configured-runtime.test.ts` | 真实 ModelRuntime 配置读取与 fixture 请求核验 |
| `acme-server.test.ts` | 授权、格式、非 probe 请求和状态重置 |

证据：[plan.test.ts:6](../../../../.repo/pi-1.0.0/packages/evals/test/plan.test.ts#L6)、[harness.test.ts:40](../../../../.repo/pi-1.0.0/packages/evals/test/harness.test.ts#L40)、[report.test.ts:115](../../../../.repo/pi-1.0.0/packages/evals/test/report.test.ts#L115)、[comparison.test.ts:61](../../../../.repo/pi-1.0.0/packages/evals/test/comparison.test.ts#L61)、[configured-runtime.test.ts:81](../../../../.repo/pi-1.0.0/packages/evals/test/configured-runtime.test.ts#L81)、[acme-server.test.ts:90](../../../../.repo/pi-1.0.0/packages/evals/test/acme-server.test.ts#L90)。

建议先读 plan/report 确定“什么算一次有效实验”，再读 cli/docker 确定执行协议，随后读 harness 的真实 Agent 生命周期，最后逐个阅读场景。逆序只看任务提示词，容易忽略决定结果可信度的实验基础设施。

本次未验证 Docker 构建、降权机制在当前主机的实际效果、外部依赖运行时 API、模型表现及统计稳定性；本报告不发布任何文档增益数值。

## 13 设计文档、规划与时效核查（2026-10-04）

本包只有一份匹配本次文档清单口径的文档：[README.md](../../../../.repo/pi-1.0.0/packages/evals/README.md#L1)，共 129 行；本次全文核查。没有独立 `PLANNING.md`、设计交接文件或 CHANGELOG。README 是现有实验协议与使用说明，不能因为描述理想实验方法就把它当成未实现路线图。

| 文档中的设计要求 | 实现与时效结论 | 依据 |
|---|---|---|
| host eval 与 `*.docs.eval.ts` 配对实验分开 | **已实现的工作方式**，不是未来演进阶段 | [README](../../../../.repo/pi-1.0.0/packages/evals/README.md#L5)、[package scripts](../../../../.repo/pi-1.0.0/packages/evals/package.json#L1) |
| 先计划全部 `(case, variant, model, runNumber)`，再运行；按重复序号交替顺序 | **已实现的实验协议**，用于避免失败样本从分母消失及减轻固定顺序偏差 | [README](../../../../.repo/pi-1.0.0/packages/evals/README.md#L69)、[任务规划](../../../../.repo/pi-1.0.0/packages/evals/src/plan.ts#L1) |
| 两实验臂使用相同本地运行时 tarball，文档与 prompt routing 是处理差异 | **已有构建与 harness 实现**；隔离边界仍需实际容器验证 | [README](../../../../.repo/pi-1.0.0/packages/evals/README.md#L80)、[Docker 调度](../../../../.repo/pi-1.0.0/packages/evals/src/docker.ts#L1)、[harness](../../../../.repo/pi-1.0.0/packages/evals/src/harness.ts#L1) |
| 缺失、重复、errored 等实验臂阻断 headline lift；未知 telemetry 不填零 | **已实现并有测试源码约束**，不是统计增强的未来承诺 | [README](../../../../.repo/pi-1.0.0/packages/evals/README.md#L101)、[配对报告](../../../../.repo/pi-1.0.0/packages/evals/src/report.ts#L359)、[比较测试](../../../../.repo/pi-1.0.0/packages/evals/test/comparison.test.ts#L61) |
| 场景只负责 setup、模型任务与确定性评分，外层拥有隔离、身份、持久化与汇报 | **当前职责分割**；新场景的扩展入口已存在 | [README](../../../../.repo/pi-1.0.0/packages/evals/README.md#L125)、[扩展场景](../../../../.repo/pi-1.0.0/packages/evals/evals/extensions.docs.eval.ts#L9) |

README 还主动限定了结论强度：模型请求需要网络，因此 Docker 本身不能证明生成代码完全无法联网；单次重复不能建立稳定性；低分是实验数据而非基础设施故障。这些都是**当前方法的边界**，并非已承诺以后消除的缺陷。[隔离限制](../../../../.repo/pi-1.0.0/packages/evals/README.md#L88)、[稳定性](../../../../.repo/pi-1.0.0/packages/evals/README.md#L103)、[评分边界](../../../../.repo/pi-1.0.0/packages/evals/README.md#L129)。

因此，本包可从文档提炼的长期设计价值是“先固定实验协议，再运行 Agent，最后保留失败与不可用数据”。本快照不能支持“以后将增加置信区间、分布式执行、排行榜或自动扩展场景”等路线图结论；第 9 节列出的统计能力边界是分析观察，不是作者规划。本次未发现 README 中需要改判为过时的主要协议段落，也未执行 README 中的付费模型或 Docker 命令。

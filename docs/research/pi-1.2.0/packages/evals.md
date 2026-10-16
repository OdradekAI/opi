# pi-evals：行为评测与文档增益实验

分析日期：2026-10-16。本地 Pi 1.2.0 静态阅读；README 全文，runner/harness/plan/report、Docker 边界及代表测试重新核查。本次没有构建镜像、安装运行时、执行评测或请求模型，不产生新的性能、成功率或文档 lift 数据。[清单](../source-manifest.json)、[审读记录](../documentation-reading.json)。

## 定位、API 与依赖

`@earendil-works/pi-evals` 是 private 仓库工具包，没有公共 exports/bin。它把实际 coding-agent AgentSession 接入 vitest-evals，给出可重复的模型行为任务和文档 with/without 成对实验；不是 Durable 的测试器，也不是通用 provider benchmark。[package.json](../../../../.repo/pi-1.2.0/packages/evals/package.json#L1)、[harness imports](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L8)。

| 入口 | 实际职责 |
|---|---|
| `eval:host` | 本机 Vitest 的非 docs eval（smoke/documentation audit），可调用真实模型 |
| `eval:docs` → cli.ts | 构建镜像、发现一致 cohort、记录计划、每臂新容器、汇总 lift |
| `eval` | 先 host 成功再 docs，附加 flags 只给 docs |
| `test` | 评测基础设施的普通测试，不能等同 live eval 成绩 |
| harness exports | createPiCodingAgentHarness/createPiDocumentationEvalHarness，模型/环境/prompt helpers |
| plan/report exports | case/task identity、解析、paired statistics 与格式化 |

内部 devDependencies 为 pi-ai/pi-coding-agent `^1.2.0`，另有 vitest-evals/@vitest-evals/core 0.15.0、autoevals 0.3.0、Vitest 4.1.11。host/docs 配置各单 worker、不 concurrent，测试和 hook timeout 300000 ms。环境与 UID 是进程级状态，串行有实际必要。[配置](../../../../.repo/pi-1.2.0/packages/evals/vitest.evals.config.ts#L4)。

## 实验关键链与持久事实

```text
CLI provider/model/repetition/filter → current workspace tarball images
  → discover without_docs + with_docs → exact cohort equality
  → full task plan + protocol digest written before execution
  → fresh container per (case,variant,model,runNumber)
  → Vitest JSON + native Session JSONL → normalized observations
  → exact paired scoring → report.json/report.txt
```

CLI 要求 provider/model 成对出现，repetitions 为正 safe integer；仅选择 `.docs.eval.ts`，发现 case 要是 `<eval set> > <case>` 且不可重复，两个 images 的 case identity 完全一致才运行。task plan 先展开两臂×重复，奇偶 repetition 交换先后顺序减少 order bias。CLI 顺序执行整个计划；失败/缺 report 臂记录 error 后继续，最终 blocked pairs 令非零退出。没有实现中断后自动续跑同一 artifact directory。[CLI](../../../../.repo/pi-1.2.0/packages/evals/src/cli.ts#L24)、[计划](../../../../.repo/pi-1.2.0/packages/evals/src/plan.ts#L21)、[执行循环](../../../../.repo/pi-1.2.0/packages/evals/src/cli.ts#L171)。

每次 invocation 新 `.eval/<timestamp>_<uuid>`，记录 protocol（model/image IDs/files/cases/tasks/digest）、expected-runs、observations、每 task native report 与 sessions、最终 summary。task directory 用完整 identity 的 SHA-256，pair identity 不包含 variant。protocol digest 绑定实验计划，不是源码签名，也不证明各臂结果无污染。[artifact 身份](../../../../.repo/pi-1.2.0/packages/evals/src/docker.ts#L156)、[protocol 写入](../../../../.repo/pi-1.2.0/packages/evals/src/cli.ts#L153)。

harness 为每个 run 新建 home/agentDir/workspace/session，移走 PI_EVAL 元数据、隔离 HOME 等环境，先捕获 selected credential 到内存，再创建 ModelRuntime/services/AgentSession，验证只加载预期 inline extensions。按 prompt/reload steps 驱动真实 session，abort signal 调 session.abort；收最后 assistant、真实 prompt hash、tokens/toolCalls/cost/latency。finally 保存 native session artifact，dispose session、删除临时目录、恢复环境；主错误与 cleanup 错误合并。session 是测量对象，harness 生命周期负责清理。[runPiCodingAgent](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L272)、[session/abort](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L342)、[finally](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L419)。

## 隔离、权限、失败与统计

Docker builder 在 ephemeral repository mount 上 `npm ci --ignore-scripts`、offline build，再通过 consumer-install 逻辑打包当前 workspace tarballs；两个 variants 用相同运行时。without_docs 删除 coding-agent README/CHANGELOG/docs/examples，并用 inline prompt transform 去文档路由段；内部依赖 docs/src/tests 对称移除，避免备用 instructions。entrypoint 检查目录 allowlist、dist resolution、root-only evaluator 与 unprivileged 读不到；加载 runner 后，harness 清 supplementary groups/setgid/setuid 永久降权，保护转译 cache，真实工具不再能读 judges/fixtures。[Dockerfile](../../../../.repo/pi-1.2.0/packages/evals/docker/Dockerfile#L1)、[entrypoint](../../../../.repo/pi-1.2.0/packages/evals/docker/entrypoint.ts#L27)、[降权](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L154)。

容器 `--rm --read-only`，临时可执行 tmpfs、独立 /artifacts bind、credential secret read-only mount；harness 复制/读取 credential 后尽量删文件和凭据 env，provider 仍在内存使用。默认 docs 工具只有 read/write/edit/grep/find/ls，没有 shell/web-search。容器保留 provider 所需网络，Docker 和工具选择不足以证明生成代码绝无其他网络路径；这是 README 的明示限制。host eval 不具有相同容器保证。[dockerArgs](../../../../.repo/pi-1.2.0/packages/evals/src/docker.ts#L91)、[README variants](../../../../.repo/pi-1.2.0/packages/evals/README.md#L82)。

pair 只有每 variant 恰好一个 expected run、一个 observation 且都 scored 才有效；missing/duplicate/skipped/pending/unscored/errored 均阻断。只要一个 eval set 有 blocked pair，其 headline pass rates/lift 为 null，不从幸存子集估计。遥测 metric 双臂都有才算 paired delta，缺值保 unavailable；operational totals 仍能汇总可取得的 token/cost，不把失败消耗藏掉。pass threshold 为 score>=1；no-lift、negative、saturated、flaky 是描述性标记，一次 repetition 不能证明稳定。[resolvePair](../../../../.repo/pi-1.2.0/packages/evals/src/report.ts#L249)、[metrics](../../../../.repo/pi-1.2.0/packages/evals/src/report.ts#L283)、[withhold headline](../../../../.repo/pi-1.2.0/packages/evals/src/report.ts#L359)。

失败/取消要区分：模型任务得低分是观测数据，README 要求 comparative judgeThreshold:null；assertion 留给 suite invariants。runner 对容器非零退出后留下的失败/缺失 report 继续计划；发现 cohort 不同、构建/认证错误或启动 docker 子进程本身抛错则早停。模型/工具取消由 AgentSession 执行；Docker subprocess 是 spawnSync 外层，无单独容器级 wall-time timeout/并行 backpressure，Vitest 内部超时不等于父 docker 命令无法卡住。此为静态可用性限制，不是本次实际卡死。[docker execute](../../../../.repo/pi-1.2.0/packages/evals/src/docker.ts#L24)、[README write eval](../../../../.repo/pi-1.2.0/packages/evals/README.md#L109)。

## 配置扩展、测试与设计时效

harness 支持 explicit model、noTools/tools/customTools、workspace fixtures、prompt transform、output extractor；workspace 路径必须相对且不能 escape。docs factory 只允许 container sandbox，并绑定 variant/提示语校验。配置型 eval 用真实 ModelRuntime 加本地 mock provider 核查生成的 models.json；`allowModelNetwork:false` 限制 catalog refresh，不表示完整 eval 无模型请求。[harness options](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L50)、[fixture validation](../../../../.repo/pi-1.2.0/packages/evals/src/harness.ts#L225)、[configured runtime](../../../../.repo/pi-1.2.0/packages/evals/evals/configured-runtime.ts#L56)。

README 是本包唯一默认文本资料，无独立路线图/CHANGELOG/handoff。paired cohorts、fresh containers、preplanned protocol、root-only judges、dist install、prompt treatment 与 missing-data rule 均有对应实现；“一轮不能建立稳定性”“网络仍开放”是现行方法限制，不能改写为未完成承诺。扩展 eval 应保留 scenario/确定性 grading，让 runner 拥有 variant/repetition/isolation/identity/persistence；这是职责边界而非还未写的功能。

测试预期覆盖 model/env/prompt transforms、拒绝 container 外 docs harness、case identity/alternating plan、Vitest/native report extraction、blocked/duplicate/missing metrics、configured runtime 与 mock server。报告统计测试能验证缺失处理，不代表模型在任务上表现改善。全部 tests/evals 均未执行；测试影响 `none`。[harness tests](../../../../.repo/pi-1.2.0/packages/evals/test/harness.test.ts#L72)、[comparison tests](../../../../.repo/pi-1.2.0/packages/evals/test/comparison.test.ts#L62)。

静态风险/证据缺口：输出 artifacts 可含 prompt、response、生成代码和工具输出；样本量/模型版本/provider 环境决定统计解释；凭据驻留内存且网络开放；工具 hooks/默认 product 行为随 coding-agent 依赖改变；fixture 不能代表真实互联服务。本次未检查第三方 vitest-evals 内部或运行隔离效果，只据锁定依赖调用契约说明。

## 1.0.4 → 1.2.0

清单对比新增 0、修改 2、删除 0、相同 28，当前 30 文件、5 个 src TS/1446 行。主体 README/runner/统计/容器/fixtures/tests 字节相同；修改为 manifest 和 harness，后者同步新版 coding-agent 的 credential/runtime/session-services 创建接口。虽然 runner 实验结构不变，受测 coding-agent/pi-ai 已升到 1.2.0，不能继承旧成功率或把 byte-identical README 当作运行行为完全不变。旧报告仅用作导航，本次重新阅读主要链；未声称完整旧新逐行 diff。

# opi 与 pi Agent Benchmark 打榜规划

日期：2026-07-10

输入：

- [benchmark 候选文档](./2026-07-08-agent-benchmark-candidates.md)
- [现有 eval 说明](../eval/README.md)
- [现有 opi-eval skill](../../.claude/skills/opi-eval/SKILL.md)

本文把“打榜”拆成两个目标：

1. 每次 opi 大版本候选发布时，在同一批任务、同一 LLM 和同一资源约束下，重跑 opi 与 pi。
2. 在官方规则允许时，把同一套适配器产生的全量结果提交到外部榜单。

“已核验”表示来自官方仓库、官方文档或本项目源码；“建议”表示本项目应采用的工程决策。

## 执行结论

可以集成候选文档中的 benchmark，但不应把数据集、Docker 镜像和大体积轨迹复制进 opi 仓库。应集成的是：

- benchmark/agent adapter；
- 数据集、harness、容器和模型版本锁；
- mini/full profile；
- 可恢复 runner；
- 原生评分、轨迹和对比报告 schema。

第一阶段只做四条线：

| 定位 | Benchmark | 决策 |
|---|---|---|
| 主榜 | Terminal-Bench 2.1 | 立即集成，作为 terminal-first agent 首要指标 |
| 编码副榜 | SWE-bench Verified/Lite | 第二个集成；官方 Docker harness 是评分权威 |
| 安全指标 | AgentDojo | 第三阶段集成；单独报告 utility/ASR，不称为排行榜 |
| 快速诊断 | BFCL-style cases | 仅作 tool-call 回归，不作为 opi 对 pi 的 headline 分数 |

其余 benchmark 暂不进入大版本发布门禁。AppWorld、tau2/tau3 等通用 domain-tool adapter 稳定后再接；GAIA、WebArena、OSWorld 等等待浏览器、搜索或 GUI 工具面成熟。

当前 opi-eval 只有 3 个小用例，依赖 agent 流程和 LLM evaluator。建议保留为 local-smoke，另建确定性 opi-bench 控制面；外部榜只采用 benchmark 原生 grader。

## 已核验事实与关键缺口

### Terminal-Bench 与 Harbor

- Terminal-Bench 2.1 使用 Harbor 数据集 terminal-bench/terminal-bench-2-1，custom agent 通过 --agent-import-path 接入，官方命令使用 k=5，且不得修改 timeout/resource。[官方 leaderboard](https://www.tbench.ai/leaderboard/terminal-bench/2.1)
- 官方文档支持 5 个 oracle task 的 smoke、单题、全量和 custom agent；公开提交流程当前仍标记 coming soon。[官方 run guide](https://www.tbench.ai/docs/run-terminal-bench-2-1)
- Harbor 支持 BaseInstalledAgent 和外部 import path，可把 CLI agent 安装到 task container 中运行。[Harbor agent integration](https://github.com/harbor-framework/harbor/blob/main/docs/content/docs/agents/index.mdx)
- Harbor README 当前引用 v0.16.1，并说明可运行任意 agent、Terminal-Bench 和包括 SWE-bench 在内的第三方数据集。[Harbor README](https://github.com/harbor-framework/harbor)

### pi 适配器不能直接照用

- Harbor 已有 pi.py，但安装的是 @mariozechner/pi-coding-agent。[Harbor Pi adapter](https://github.com/harbor-framework/harbor/blob/main/src/harbor/agents/installed/pi.py)
- opi 的目标基线是 earendil-works/pi；其当前 package.json 使用 @earendil-works/pi-coding-agent，核验时版本为 0.80.6。[earendil-works/pi package.json](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/package.json)

因此不能把 Harbor 内置 -a pi 当成目标基线。第一版应复制其安装和日志解析思路，写项目内 EarendilPi adapter，并锁定 npm tarball integrity、版本和 git commit。

### opi 的 LLM 控制面还不完整

- opi CLI 可选 model、config、system prompt、thinking config 和 tool policy，但没有 temperature 或 max_tokens CLI 参数。[cli.rs](../../crates/opi-coding-agent/src/cli.rs)
- opi-agent 的 AgentLoopConfig 和 harness API 内部支持 temperature、max_tokens 与 thinking config。[loop_types.rs](../../crates/opi-agent/src/loop_types.rs)
- pi CLI 有 thinking level，而 opi 使用 enabled/budget tokens；同名 high 与固定 token budget 不能自动视为等价。

第一版 controlled track 可以要求双方关闭 thinking、都省略 temperature，让同一 provider 使用同一默认采样。正式全量榜前应增加 adapter/SDK 级显式控制和校验，记录 LLM control fingerprint。

### SWE-bench 必须保留官方评分路径

- SWE-bench Verified 是 500 个经人工确认可解的问题；官方 harness 使用 Docker，并建议 x86_64、约 120 GB 可用磁盘、16 GB RAM 和 8 CPU。[SWE-bench README](https://github.com/SWE-bench/SWE-bench)
- Harbor 有 SWE-bench Verified adapter，但其 README 同时记录已知排除项和与官方结果的 parity 差异。[Harbor adapter](https://github.com/harbor-framework/harbor/blob/main/adapters/swebench/README.md)
- Verified/Multilingual 自 2025-11-18 起只接收满足机构归属和开放研究出版条件的公开提交；结果目录还要求 predictions、metadata、logs 和 trajectories。[SWE-bench experiments](https://github.com/SWE-bench/experiments)

Harbor 可以负责 inference 和环境编排，但最终 model_patch 必须进入官方 swebench.harness.run_evaluation。内部报告不得把 Harbor adapter 分数标成官方 leaderboard 分数。

## 候选集成矩阵

| Benchmark | 适配度 | 项目内集成 | 大版本门禁 | 决策 |
|---|---:|---|---|---|
| Terminal-Bench 2.1 | 高 | Harbor 双 agent adapters | 主指标 | 立即做 |
| SWE-bench Verified/Lite | 高 | inference、patch extraction、官方 grader | 副指标 | 第二步做 |
| AgentDojo | 中 | domain tools/process adapter | 安全指标 | 第三步做 |
| BFCL | 中 | tool-call diagnostic | 快速回归 | 不做 headline |
| AppWorld | 中 | API/code execution adapter | 否 | 后续做 |
| tau2/tau3 | 中 | conversation/domain-tool adapter | 否 | 后续做 |
| R2E/R2E-Gym | 中 | 生成/导入 repo task | 否 | 用于 opi-native task |
| GAIA | 低 | 无 web/image 的窄子集 | 否 | 等工具面 |
| AgentBench | 中低 | 逐环境 adapter | 否 | 只再评估 OS/DB |
| WebArena/WorkArena | 低 | BrowserGym/浏览器工具 | 否 | 暂缓 |
| OSWorld/OSWorld-MCP | 低 | GUI/MCP adapter | 否 | 暂缓 |
| MLE-bench/MLAgentBench | 中低 | 长时 runner | 否 | 可选 profile |
| ToolSandbox/API-Bank/ToolBench | 中 | tool diagnostic | 否 | 只做补充 |

AgentDojo 官方结果页明确说明结果表不是 leaderboard，因为 model/defense/attack 没有完整公平组合。[AgentDojo results](https://agentdojo.spylab.ai/results/)

BFCL 公开流程要求新增 model handler 和 model config，并要求模型可公开访问；它评的是模型 function calling，不是 opi/pi 产品 harness。[BFCL contributing](https://github.com/ShishirPatil/gorilla/blob/main/berkeley-function-call-leaderboard/CONTRIBUTING.md)

tau2/tau3 要求一致模型配置、完整 domain task、优先 4+ trials，并区分 standard 与 custom scaffold；opi/pi 都应标记 custom。[tau2 submission guide](https://github.com/sierra-research/tau2-bench/blob/main/docs/leaderboard-submission.md)

## 公平对比协议

### 比较对象

主比较是产品级 agent scaffold：

- opi 使用自身 system prompt、tool schema、loop、compaction、retry 和工具实现；
- pi 使用自身对应实现；
- 用户 task、LLM、外层资源和 grader 相同。

不要强行统一双方 system prompt 和 tool schema。它们是被测 agent 的组成部分；统一后测到的是新 scaffold，不再是 opi 对 pi。

同时保留两个趋势：

1. competitive：当前 opi 对 RC 冻结日的最新稳定 earendil pi。
2. longitudinal：当前 opi 对上一大版本 opi；使用同一 benchmark season。

### Benchmark season 冻结项

benchmarks/benchmark.lock.toml 至少记录：

| 维度 | 必须记录 |
|---|---|
| opi | semver、commit、binary SHA-256、build target |
| pi | package、version、commit、tarball integrity |
| model | provider、不可变/带日期 model ID、endpoint class |
| sampling | temperature（含 omitted）、max output、thinking/reasoning、seed |
| benchmark | dataset URI、revision、task IDs |
| harness | Harbor/SWE-bench version与 commit |
| environment | image digest、OS/arch、CPU/RAM/disk |
| execution | timeout、concurrency、trial count、network policy |
| prompts/tools | prompt-template hash、tool allowlist、context policy |

禁止用 latest、sonnet、gpt-5 等可漂移 alias 做正式 season。若 provider 只提供漂移 alias，必须写入限制，并在同一 24 小时窗口交错运行双方。

模型退役时新开 season：同一个 opi/pi 版本同时跑旧模型与新模型的 overlap sample，不能把新旧模型分数直接连成一条趋势线。

### v1 可执行 LLM 控制

首个 season 使用：

- 同一 provider 和同一带日期 model ID；
- 双方关闭 thinking/reasoning；
- 双方不发送 temperature，记录 provider_default/omitted；
- 相同 task timeout、concurrency 和 trial count；
- 禁止 fallback model；
- 记录 provider 返回的实际 model/version（若 API 提供）。

全量 run 前捕获双方第一条脱敏请求，只比较控制字段，不比较 messages/tools。provider、model、sampling、reasoning 或 max-output 不一致时 fail closed。

后续 thinking track 应通过 opi SDK/harness 和 pi settings/SDK 映射到同一数值预算，单独报告，不混入 no-thinking 主趋势。

### 环境隔离

每个 task/trial 使用全新环境：

- HOME、APPDATA、XDG config 指向空目录；
- 不加载用户 AGENTS.md、CLAUDE.md、skills、packages 或 extensions；
- 只注入 benchmark 最小配置和凭据；
- opi 与 pi 从同一 task image/digest 启动；
- agent 不可读取 grader tests、oracle patch 或另一 agent 轨迹；
- task 结束后再由 grader 评分；
- secret 不进入报告、NDJSON 或上传 artifact。

### 成对运行与失败分类

每个 benchmark/task_id/trial 产生一对 opi、pi 记录。执行顺序按 block 随机化，避免固定顺序受到 provider 负载和时间漂移影响。

外层 runner 不自动“重试直到通过”。失败分为：

- agent_failure：agent crash、超时、无 patch，计入失败；
- infra_failure：容器拉取、全局 provider 故障；
- grader_failure：oracle/preflight 自身失败。

只允许重跑预注册的 infra_failure，而且必须重跑该 task 的 opi/pi 整对。

## 评分与统计

只发布 benchmark 原生指标：

- Terminal-Bench：官方 accuracy，k=5；
- SWE-bench：resolved rate；
- AgentDojo：utility、utility under attack、targeted ASR；
- BFCL diagnostic：各 category accuracy。

不把不同 benchmark 归一化后相加成“opi 总分”。综合总分会掩盖真实退化。

同任务 paired analysis：

1. 报告 opi、pi 原生均值和 percentage-point 差值。
2. 对 task-level 差值做 10,000 次 paired bootstrap，给 95% CI。
3. 二元 pass/fail 用 exact McNemar test，报告 discordant pairs。
4. 多 trial 同时给 task 均值、trial 方差和 k，不能只给最佳一次。
5. CI 跨 0 时写“未观察到明确差异”，不能宣称胜出。
6. 5-20 题 mini 只用于回归报警，不用于对外排名。

现有 opi-eval 的 LLM evaluator 可以分析 trace，但不得覆盖官方 grader，也不得参与 headline score。

## 建议项目结构

    benchmarks/
      pyproject.toml
      uv.lock
      benchmark.lock.toml
      profiles/
        terminal-smoke.toml
        release-mini.toml
        release-full.toml
      opi_bench/
        cli.py
        manifest.py
        runner.py
        compare.py
        artifact.py
      agents/
        opi_harbor.py
        earendil_pi_harbor.py
      suites/
        terminal_bench.py
        swe_bench.py
        agentdojo.py
      tests/
        test_manifest.py
        test_ndjson_normalization.py
        test_pairing.py

    docs/eval/benchmarks/
      history.jsonl
      <opi-version>/<run-id>/summary.md

大文件放 target/opi-bench/run-id 或 CI artifact/object storage，不提交：

- raw NDJSON/JSONL 和 ATIF trajectories；
- Docker logs/images；
- SWE-bench patches 与 grader logs；
- provider request captures。

仓库只提交 lock、profile、adapter、summary 和 artifact checksums。外部 benchmark 使用独立 docs/eval/benchmarks/history.jsonl，避免破坏现有 local-case history schema。

## Adapter 与命令

### Opi Harbor adapter

Opi 继承 BaseInstalledAgent：

1. 将当前 commit 的 Linux x86_64 binary 安装到 task container，校验 SHA-256。
2. 写隔离 config，关闭用户资源和 fallback。
3. 在 task workspace 运行：

    /opt/opi/bin/opi --json-compact --allow-mutating       --model <opi-provider:model> "<instruction>"

4. 捕获完整 NDJSON，转换 Harbor ATIF/metrics，保留 schema version。

先验证现有 release binary 对 Terminal-Bench task images 的兼容性；glibc 不稳定时再增加静态 musl benchmark artifact，不要未经验证修改正式 release matrix。

### Earendil Pi Harbor adapter

以 Harbor pi.py 为参考，但安装：

    npm install -g @earendil-works/pi-coding-agent@<pinned-version>
    pi --print --mode json --no-session       --provider <provider> --model <model> --thinking off "<instruction>"

校验 pi --version 与 lock 一致，保存 package integrity，禁止 @latest。

### Terminal-Bench

Oracle preflight：

    harbor run -d terminal-bench/terminal-bench-2-1 -a oracle -l 5

正式 custom-agent 形状：

    harbor run       -d terminal-bench/terminal-bench-2-1       --agent-import-path "benchmarks.agents.opi_harbor:Opi"       -m "<provider/model>" -k 5

    harbor run       -d terminal-bench/terminal-bench-2-1       --agent-import-path "benchmarks.agents.earendil_pi_harbor:EarendilPi"       -m "<provider/model>" -k 5

发布时由 uv run opi-bench run --profile release-full 生成参数，不依赖人手拼命令。

### SWE-bench

控制面创建干净 workspace、运行 agent、收集 git diff，并生成 all_preds.jsonl。最终评分调用官方 harness：

    python -m swebench.harness.run_evaluation       --dataset_name SWE-bench/SWE-bench_Verified       --predictions_path target/opi-bench/<run-id>/<agent>/all_preds.jsonl       --max_workers <pinned-workers>       --run_id <run-id>-<agent>

先用官方 gold 单题验证 grader。opi 与 pi 使用相同 instance list、official image 和 worker 数。

## 六周实施计划

### 第 1 周：实验合同

交付：Python/uv 控制面、lock schema、run/task/summary schema、model-control preflight、artifact 脱敏与 checksum。

验证：

- 同一 lock 生成字节稳定 resolved manifest；
- 缺 commit/digest/model control 时拒绝运行；
- secret/path leakage fixture 被拦截；
- fake benchmark 可断点恢复且不重复计费。

### 第 2 周：双 agent adapters

交付：Opi、EarendilPi adapters，双事件流 metrics/trajectory 转换，版本/hash/token/cost 采集。

验证：

- 同一个 trivial Harbor task 上双方都能修改文件并退出；
- container 内版本与 lock 一致；
- 空 HOME 不加载用户 context/package；
- model-control fingerprint 一致。

### 第 3 周：Terminal-Bench mini

交付：oracle smoke、10 个固定 task、10 tasks x 3 trials x 2 agents、paired comparison。

验证：

- oracle 5 题通过；
- adapter/infra error 为 0；
- 每个 task/trial 恰有一对结果；
- score、cost、duration、hash 可追溯。

### 第 4 周：Terminal-Bench full

交付：官方 2.1 full、k=5、bootstrap CI、McNemar、submission checklist。

验证：

- 不修改官方 timeout/resource；
- 可按 task/trial 恢复；
- summary 数字可追到原始 artifact；
- 报告标明 internal reproduction 或 official verified。

### 第 5 周：SWE-bench mini

交付：instance runner、patch extraction、all_preds.jsonl、20 个固定 task、官方 grader。

验证：

- gold patch 单题通过；
- 双方 patch 可被官方 harness 读取；
- 环境不暴露 oracle/grader；
- 结果含 agent/infra/grader failure 分类。

### 第 6 周：发布自动化

交付：protected manual benchmark workflow、release-mini/full profiles、benchmark history、AgentDojo/BFCL 后续 spec。

验证：

- 从 release commit 一键完成 preflight、双 agent、评分、对比和 artifact 上传；
- fork PR 不能触发，environment 需要人工批准；
- budget 超限停止新 task但保留结果；
- summary 可离线复算。

## 大版本运行制度

| 时点 | 运行内容 | 用途 |
|---|---|---|
| 普通 PR | runner unit tests、manifest、fixtures | 控制面正确性 |
| 关键 agent/tool/provider 改动 | local-smoke + 2-task paired canary | 低成本预警 |
| 大版本 RC | release-mini，双方同批重跑 | 发布门禁与 triage |
| 每个大版本发布前 | Terminal-Bench full + 固定 SWE-bench profile | 正式版本分数 |
| model/benchmark season 切换 | old/new overlap run | 保持趋势可解释 |
| 公开提交 | full 通过且政策允许 | 外部验证 |

预 1.0 阶段把每次 0.x minor release 视为“大版本”。patch release 不自动跑 full，除非改动触及 agent loop、tools、provider streaming、compaction 或 benchmark adapter。

发布门禁分两层：

1. 硬门禁：oracle/preflight 通过、0 个未解释 infra error、artifact 完整、control fingerprint 一致。
2. 分数门禁：先观察两个 release season；稳定后预注册“相对上一 opi 版本下降超过 X points 且 95% CI 不跨 0”为阻断条件。

opi 对 pi 的差值先作为发布证据，不直接阻止发布。竞争基线也会变化，不应把“必须胜过 pi”写成脆弱 CI 条件。

## 成本与运行控制

- full 前双方各跑 2 个 task pilot，用实测 token/time 外推预算。
- manifest 设置 max_cost_usd、max_wall_time、max_concurrency；超限后停止新 task并保留 checkpoint。
- 双方使用相同 concurrency、provider rate limit 和 task timeout。
- 记录 input/output/cache token、provider cost、wall time、tool calls、provider turns 和 retries。
- full artifact 至少保留两个大版本周期；公开 submission artifact 单独归档并做 SHA-256。
- SWE-bench 使用隔离 x86_64 Linux 主机或云 sandbox；普通 GitHub-hosted runner 不满足官方磁盘建议。
- workflow 只允许 protected branch/tag 和人工批准环境，避免 secrets 暴露给 fork 或不可信 task。

## 公开打榜策略

### Terminal-Bench 2.1

先产出 Harbor jobs、adapter import path、版本锁和复跑文档。官方页面说明榜单由团队复跑验证，但新 submission 流程尚未开放；不能声称内部结果已上榜。

### SWE-bench

内部可跑 Verified/Lite。公开 Verified 只有满足研究机构归属、开放实现和论文/技术报告时才提交。Lite 是否接受产品项目应在提交当日重新核对。

### AgentDojo 与 BFCL

AgentDojo 结果可以 PR 展示，但官方明确不是 leaderboard。BFCL 可以展示同一底层模型的 function-calling 诊断，不能标成 opi agent 分数。

对外报告使用三种标签：

- official-verified：官方团队复跑或榜单已收录；
- official-harness-local：官方数据和 grader 的本地复现；
- internal-mini：固定子集回归，不可与公开榜比较。

## 风险与预先决策

1. 模型漂移：使用带日期 ID；不可避免时新开 season并在 24 小时内交错跑。
2. pi 基线错位：不用 Harbor 内置旧 package adapter；固定 earendil-works/pi。
3. sampling 不等价：v1 关闭 thinking并省略 temperature；thinking track 前补 SDK 控制。
4. 环境污染：隔离 HOME/APPDATA/XDG，禁用用户 context、packages、skills 和 extensions。
5. SWE-bench grader 偏差：官方 harness 是 submission authority。
6. mini 过拟合：task IDs 预注册，不据此调 prompt；full 才作正式结论。
7. 数据污染：公开数据可能进入模型训练；分数不等于生产成功率。
8. 成本失控：pilot、预算上限、task checkpoint、相同并发。
9. 政策变化：提交前重新核验；内部复现与官方收录分开。
10. 单一总分误导：保留原生指标和置信区间，不设计 composite leaderboard。

## 第一阶段完成定义

只有同时满足以下条件，才能说“opi 已具备持续打榜能力”：

- 一条命令从 lock 运行 opi 与指定 earendil pi；
- Terminal-Bench 2.1 full 按官方资源和 k=5 完成；
- SWE-bench mini 由官方 Docker harness 评分；
- 同任务的 opi/pi 结果严格成对；
- model-control fingerprint 一致；
- 版本、容器、数据集、成本、轨迹和 grader 可追溯；
- summary 可离线复算；
- 报告区分 internal、official-harness 与 official-verified。

达到这一点后，再扩 AgentDojo、AppWorld 和 tau2/tau3。第一阶段的成功标准不是接入最多 benchmark，而是建立一条可跨 opi 大版本重复、可解释差异、也可生成官方提交物的评测管线。

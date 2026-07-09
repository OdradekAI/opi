# opi agent benchmark 打榜规划

日期：2026-07-09

关联输入：
- `docs/research/2026-07-08-agent-benchmark-candidates.md`
- `docs/eval/README.md`
- `.claude/skills/opi-eval/SKILL.md`

## 结论

opi 应该把主流 benchmark 作为“外部可复现评测配置”集成到项目里，而不是把 agent 上传到 benchmark 平台。现在主流打榜方式通常是：

1. 在 benchmark 官方 harness 中接一个 agent wrapper。
2. 在官方数据集、容器、超时和资源约束下运行。
3. 产出 predictions、patches、logs、trajectories、bundles 或 submission metadata。
4. 通过 PR、Hugging Face Space、官方表单或维护者复跑进入公开榜单。

所以 opi 的理想形态是：项目内维护 `opi-eval` 外部 benchmark adapters、版本锁、mini profiles 和报告 schema；每次大版本迭代跑内部 pinned mini suite；只有 release candidate 足够稳定时再跑 full suite 并按各榜规则提交。

## 打榜方式分型

| 类型 | 是否上传 agent | 提交物 | 适合 opi 的用途 |
|------|----------------|--------|----------------|
| Harness-local | 否。把 opi 包成 harness 可调用的 agent。 | 本地报告、trace、分数。 | 大版本回归跑分主路径。 |
| PR-to-results-repo | 通常否。提交结果目录和轨迹，必要时给复跑说明。 | `all_preds.jsonl`、logs、trajs、metadata、submission JSON。 | SWE-bench、tau-bench、AppWorld、AgentDojo 类。 |
| Maintainer-verified | 不上传 agent 二进制，但要给可复跑说明或 adapter。 | 结果和复跑入口；维护者抽样或全量复跑。 | Terminal-Bench、SWE-bench verified checkmark、tau voice。 |
| Model-handler PR | 不是 agent 榜，提交 model handler。 | model handler、model config、公开可访问 endpoint。 | BFCL；对 opi 只能作为 tool-call 诊断，不适合作为 headline agent 榜。 |
| Hosted Space / permission gate | 不上传 agent，通常上传结果或在 Space 提交。 | JSONL/结果文件；可能要申请提交权限。 | GAIA，等 opi 有 web/search/browser/image 工具后再考虑。 |

## 各候选 benchmark 的可执行判断

### Terminal-Bench

状态：第一优先级，最适合 opi 当前 terminal-first 产品面。

官方现状：
- Terminal-Bench 2.1 必须使用 Harbor 数据集 `terminal-bench/terminal-bench-2-1`，榜单页给出的 custom agent 命令是 `harbor run -d terminal-bench/terminal-bench-2-1 --agent-import-path "path.to.agent:SomeAgent" -k 5`，并注明不能修改 timeout/resource。[Terminal-Bench 2.1 leaderboard](https://www.tbench.ai/leaderboard/terminal-bench/2.1)
- Terminal-Bench 2.1 文档说明 custom agent 通过 Python import path 接入 Harbor；提交流程仍标注为 coming soon，但榜单结果由 Terminal-Bench 团队运行并验证。[Run Terminal-Bench 2.1](https://www.tbench.ai/docs/run-terminal-bench-2-1)
- Terminal-Bench leaderboards 页面说明 2.0 和 2.1 都是 live，且必须通过 Harbor 对应 dataset 运行。[Terminal-Bench leaderboards](https://www.tbench.ai/leaderboard)

opi 集成方式：
- 写一个小 Python adapter，例如 `tools/opi_eval/adapters/terminal_bench/opi_agent.py`，实现 Harbor agent interface。
- Adapter 在任务容器内调用 `target/release/opi` 或注入的 `OPI_BIN`：
  - `opi --json --allow-mutating --model <model> "<task instruction>"`
  - 工作目录使用 Harbor 提供的 task workspace。
  - 将 opi NDJSON 转成 Harbor trajectory metadata，保留原始 `output.ndjson`。
- 默认跑 `terminal-mini`：固定 5-10 个任务，验证 adapter 和回归趋势。
- RC 跑 `terminal-full`：Terminal-Bench 2.1 全量，并保存可提交 artifacts。

公开打榜：
- 先做 full run，确认没有超时/resource 修改。
- 准备 custom-agent import path、复跑说明、opi commit、model spec、成本和 trace 摘要。
- 在 Terminal-Bench 当前 submission 流程开放后提交；在此之前按榜单/Discord/HF repo 指引联系维护者复跑。

### SWE-bench Lite / Verified

状态：第一优先级，但公开 Verified 打榜有资格限制。

官方现状：
- SWE-bench 评测真实 GitHub issue，给定 codebase 和 issue 后生成 patch；评测使用 Docker 可复现 harness。[SWE-bench docs](https://www.swebench.com/SWE-bench/)
- 官方 README 给出本地评测命令：`python -m swebench.harness.run_evaluation --dataset_name princeton-nlp/SWE-bench_Lite --predictions_path <path> ...`，并提示资源需求约 120GB 磁盘、16GB RAM、8 CPU。[SWE-bench GitHub](https://github.com/swe-bench/SWE-bench)
- 公开榜单提交由 `SWE-bench/experiments` 仓库维护；Lite/Verified/Multilingual 提交需要 `all_preds.jsonl` 或 `preds.json`、`metadata.yaml`、`README.md`、`trajs/` reasoning traces 和 `logs/` evaluation artifacts。[SWE-bench experiments](https://github.com/swe-bench/experiments)
- 2025-11-18 起，Verified 和 Multilingual 只接受 academic/research institution 且 open source methods + research publication 的提交；Multimodal 仍可提交。[SWE-bench experiments policy](https://github.com/swe-bench/experiments)

opi 集成方式：
- 写 `swe-bench` runner：
  - 对每个 instance 准备 repo workspace。
  - 用 issue prompt 调 opi，让 opi 修改文件并运行测试。
  - 运行结束后收集 `git diff` 为 `model_patch`，写入 `all_preds.jsonl`。
  - 将 opi NDJSON 转成人类可读 `trajs/<instance_id>.md`。
  - 调官方 Docker harness 评分。
- 默认跑 `coding-mini`：从 Lite/Verified 中 pin 10-20 个覆盖不同 repo/失败模式的任务。
- 不要把 SWE-bench 数据和 Docker images vendored 到仓库；只提交 manifest、锁文件和 fetch/evaluate 脚本。

公开打榜：
- 如果 opi 没有 academic/research affiliation + open research publication，先不要把 Verified 公开打榜作为目标；可以跑内部 Verified mini/full，并公开自己的非官方报告。
- Lite/full 是否可被接受需按 `SWE-bench/experiments` 当时 policy 确认；2026-07-09 的硬限制至少明确覆盖 Verified 和 Multilingual。
- 如果满足条件：fork `SWE-bench/experiments`，添加 `evaluation/<split>/<date>_<opi_model>/`，运行 `analysis.get_results`，提交 PR。

### BFCL

状态：适合内部 tool-call diagnostic，不适合作为 opi 的主打榜单。

官方现状：
- BFCL 是 executable function-call evaluation，评估 LLM 调用函数/工具的能力。[BFCL README](https://github.com/ShishirPatil/gorilla/blob/main/berkeley-function-call-leaderboard/README.md)
- 公开榜新增模型需要实现 model handler、更新 model config 和 supported models，然后提 PR；模型必须公开可访问，才会进入 public-facing leaderboard。[BFCL contributing](https://github.com/ShishirPatil/gorilla/blob/main/berkeley-function-call-leaderboard/CONTRIBUTING.md)

opi 集成方式：
- 不把 BFCL 当 agent leaderboard。opi 是 harness/agent，不是单个 model endpoint。
- 可做 `tool-call-diagnostic` profile：把 BFCL-style cases 转成 opi tool schema 回归，用来测：
  - tool relevance
  - arg JSON schema validity
  - parallel/sequential tool planning
  - irrelevant-tool avoidance
  - malformed result recovery
- 只在需要宣传某个底层 model 的 function calling 时，才考虑提交 BFCL model handler；这不是 opi 本身的榜。

### AgentDojo

状态：高优先级安全评测，但官方页面明确“不作为 leaderboard”。

官方现状：
- AgentDojo 用于评测 tool-using agents 在 prompt-injection 攻击下的 utility 和 robustness；仓库提供 benchmark script。[AgentDojo GitHub](https://github.com/ethz-spylab/agentdojo)
- Results 页面明确说不是 leaderboard，因为没有对所有 model/defense/attack 做公平全组合；添加结果需要 fork 仓库跑 benchmark 并开 PR，PR 要包含模型/攻击/防御描述和实现。[AgentDojo results](https://agentdojo.spylab.ai/results/)

opi 集成方式：
- 作为 `security-mini` profile，不作为公开排名目标。
- 写 adapter，把 AgentDojo tool runtime 映射为 opi extension/custom tool surface。
- 报告里记录 `utility`、`utility_under_attack`、`targeted_asr`、policy violation 和 trace。

公开展示：
- 可以向 AgentDojo 提 PR 展示结果，但不要称为“打榜排名”。
- 更适合在 release notes 中作为 safety regression evidence。

### AppWorld

状态：第二波，适合 opi 有更稳的 API/tool adapter 后接入。

官方现状：
- AppWorld 是 9 个日常 app、457 APIs、约 100 人模拟世界上的交互式 coding/API benchmark。[AppWorld GitHub](https://github.com/StonyBrookNLP/appworld)
- AppWorld leaderboard 仓库通过 PR 接收 agent outputs；提交时要 pack `test_normal` 和 `test_challenge` 两套 experiment outputs，生成加密 `leaderboard.bundle`，并明确不要公开未加密输出。[AppWorld leaderboard](https://github.com/StonyBrookNLP/appworld-leaderboard)

opi 集成方式：
- 写 AppWorld runner，让 opi 在 task workspace 中读 API docs、写代码并执行 HTTP/API calls。
- 默认先跑 dev/mini，不要上来跑 test full。
- 等 opi extension tool adapter 边界稳定后，再把 AppWorld 放进 `general-agent` 或 `api-coding` profile。

公开打榜：
- 跑 `test_normal` 和 `test_challenge`。
- 用 `appworld pack` 生成两个 bundle。
- PR 到 `appworld-leaderboard`，只提交加密 bundle 和 metadata。

### tau-bench / tau2/tau3

状态：第二波；更偏客服/企业 tool-agent-user interaction，不是 terminal coding 主线。

官方现状：
- tau2/tau3 是多领域 customer-service agent simulation，支持 text half-duplex 和 voice full-duplex；每个 domain 有 policy、tools、tasks 和可选 user tools。[tau2-bench GitHub](https://github.com/sierra-research/tau2-bench)
- 提交要求推荐覆盖所有 text domains，配置一致，每个 domain 一份结果，全任务运行，至少 4 trials；custom scaffold 需要说明修改、引用实现，并标记 `submission_type = "custom"`。[tau-bench submission guide](https://github.com/sierra-research/tau2-bench/blob/main/docs/leaderboard-submission.md)
- 提交流程是 `tau2 submit prepare`、`tau2 submit validate`，仓库 PR 只提交 `submission.json`，trajectory files 外部托管，维护者审核后同步到 S3。[tau-bench submission guide](https://github.com/sierra-research/tau2-bench/blob/main/docs/leaderboard-submission.md)

opi 集成方式：
- 只有当 opi 有稳定 conversation harness 和 domain-tool adapter 后再接。
- opi 运行会属于 custom scaffold，因为不是默认 tau-bench prompt/control flow。
- 把轨迹格式纳入 opi-eval artifacts，不进 git。

公开打榜：
- 全 domain、全 task、4+ trials。
- `submission.json` 写清 opi scaffold、prompts、tool adapter、user simulator、cost。
- trajectory 外部托管，PR 中给链接。

### GAIA

状态：暂缓。等 opi 有官方 web/search/browser/image 工具或 package 后再接。

官方现状：
- GAIA 是 General AI Assistants benchmark，Hugging Face organization 提供 leaderboard、viewer、public results/submissions datasets。[GAIA Hugging Face org](https://huggingface.co/gaia-benchmark)
- 社区讨论显示提交可能需要 leaderboard submission access；有用户请求授权，并提到 required JSONL submission files。[GAIA leaderboard discussions](https://huggingface.co/spaces/gaia-benchmark/leaderboard/discussions)

opi 集成方式：
- 现在只适合做极小 validation subset，且限定为不需要 web/browser/image 的任务。
- 真正打榜前，必须先有可复现 search/browser/image 工具链和答案规范化。

## 推荐的 opi-eval 架构

新增外部 benchmark profile，但不把大数据集、Docker images、logs 提交进仓库。

建议目录：

```text
tools/opi-eval/
  adapters/
    terminal_bench/
    swe_bench/
    bfcl_diagnostic/
    agentdojo/
  profiles/
    local-smoke.toml
    terminal-mini.toml
    coding-mini.toml
    tool-call-diagnostic.toml
    security-mini.toml
  benchmarks.lock.toml

docs/eval/
  history.jsonl
  external/
    <run-id>.md
```

`benchmarks.lock.toml` 必须记录：

```toml
[[benchmarks]]
name = "terminal-bench"
profile = "terminal-mini"
dataset = "terminal-bench/terminal-bench-2-1"
harness = "harbor"
harness_version = "<pinned>"
task_ids = ["..."]
docker_required = true
public_submission = "custom-agent-harbor"
source_url = "https://www.tbench.ai/leaderboard/terminal-bench/2.1"
```

外部 eval report schema 在现有 `docs/eval/README.md` 基础上补字段：

| 字段 | 说明 |
|------|------|
| `benchmark.name` | Terminal-Bench / SWE-bench / AgentDojo 等 |
| `benchmark.version` | dataset/harness version 或 git commit |
| `benchmark.source_url` | 官方来源 |
| `profile` | `terminal-mini`、`coding-mini` 等 |
| `task_id` | upstream task id |
| `opi_commit` | 被测 opi commit |
| `opi_binary_sha256` | 被测二进制摘要 |
| `model` | `provider:model` |
| `tool_policy` | 是否 `--allow-mutating`、禁用哪些 tools |
| `score.native` | benchmark 原生分数 |
| `score.normalized` | `PASS` / `DEGRADED` / `FAIL` / `ERROR` |
| `cost_usd` | API 成本估计 |
| `time_ms` | wall-clock |
| `transcript_path` | 原始 opi NDJSON 或转换后的 trajectory |
| `submission_artifacts` | 公开打榜需要的输出路径 |
| `sandbox_notes` | Docker、网络、资源和隔离说明 |

## 大版本跑分流程

### 每个 PR / 日常开发

只跑便宜、确定性强的本地测试：

```powershell
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

如果改了 agent loop、tools、NDJSON/RPC、sessions、provider streaming，再手动跑现有 `opi-eval local-smoke`。

### 每个 minor/major release candidate

跑 release-mini：

1. `local-smoke`：现有 candy/tool_chain/context_retention + tool policy cases。
2. `tool-call-diagnostic`：BFCL-style 50-200 个小用例，目标 10-30 分钟内完成。
3. `terminal-mini`：Terminal-Bench 2.1 pin 5-10 题，目标覆盖 shell、repo edit、服务启动、debug。
4. `coding-mini`：SWE-bench Lite/Verified pin 10-20 题，目标覆盖 issue-to-patch。
5. `security-mini`：AgentDojo pin 小套件，覆盖外部 tool output injection。

release-mini 的合格线：
- 没有 adapter crash。
- normalized overall 不低于上一大版本同模型基线。
- 新增失败必须有 triage：agent 能力退化、模型波动、benchmark 环境变化、还是 adapter bug。
- 成本和耗时记录完整。

### 每个 public release 前

只在 release-mini 通过后跑 expensive profiles：

1. Terminal-Bench 2.1 full。
2. SWE-bench Lite full；Verified full 仅在公开提交资格和预算允许时跑。
3. AgentDojo selected full suite。
4. 如已接入，再跑 AppWorld/tau selected full。

产物：
- `docs/eval/external/<version>-<date>-<model>.md`
- `docs/eval/history.jsonl` 追加 summary
- `target/opi-eval-artifacts/<run-id>/` 保存大日志，不提交 git
- 可公开提交的 bundle/preds/trajs 另存为 release artifact

### 公开打榜触发条件

满足以下条件才提交公开榜：

- adapter 已至少连续两次 release-mini 稳定。
- full run 没有环境性大面积失败。
- benchmark policy 允许当前组织/方法提交。
- 所有公开材料不泄露密钥、私有路径、用户数据。
- README/metadata 清楚说明：
  - opi commit/version
  - model spec
  - prompts/tool policy
  - retries/best-of-k
  - cost estimate
  - hardware/cloud environment
  - 是否 modified scaffold

## 六周执行计划

### 第 1 周：评测基座

交付：
- `benchmarks.lock.toml` 草案。
- 外部 eval report schema。
- artifact 存储约定。
- `opi-eval external` 命令或 skill 流程草案。

验证：
- 能生成一个无 benchmark 的 dry-run report。
- report 中 commit、model、binary hash、cost/time 字段完整。

### 第 2-3 周：Terminal-Bench mini

交付：
- Harbor custom agent adapter。
- `terminal-mini` profile。
- 5-10 个固定 task id。
- NDJSON -> trajectory 转换。

验证：
- `harbor run -d terminal-bench/terminal-bench-2-1 --agent-import-path ... --include-task-name ...` 单题能跑通。
- mini profile 能稳定产出 summary、raw NDJSON、native score。

### 第 4-5 周：SWE-bench mini

交付：
- SWE-bench instance runner。
- opi patch extraction。
- `all_preds.jsonl` 生成。
- `trajs/<instance>.md` 转换。
- `coding-mini` profile。

验证：
- gold patch harness smoke 能跑。
- 1 个 opi-generated patch 能进入 SWE-bench harness 并产生 report。
- mini profile 产出 pass/fail 和 logs。

### 第 6 周：诊断和发布流程

交付：
- BFCL-style `tool-call-diagnostic` 内部用例。
- AgentDojo `security-mini` 适配方案或 prototype。
- release-mini 一键流程。
- public submission checklist。

验证：
- 用同一模型跑一轮完整 release-mini。
- 和 `docs/eval/history.jsonl` 的旧 schema 兼容或完成 schema migration。

## 风险和决策点

1. SWE-bench Verified 公开提交资格可能不满足。内部跑分仍有价值，但不要承诺公开上榜。
2. Terminal-Bench 2.1 submission process 仍在变化。先做 Harbor adapter 和 full-run artifacts，等流程稳定再提交。
3. Benchmark 数据不要 vendor 到 opi 仓库。只保存 task id、version、checksum、source URL 和 fetch/evaluate scripts。
4. Full run 成本高，不进默认 CI。大版本 RC 用人工触发或 nightly scheduled runner。
5. 公开榜分数容易被 benchmark policy、模型版本、资源配置影响。报告必须固定模型、commit、harness version、环境和 cost。
6. 对外宣传时区分“官方榜单分数”和“opi 内部复现实验分数”。

## 最小可执行路线

优先顺序：

1. Terminal-Bench 2.1 mini/full：这是 opi 当前最能说明产品能力的公开 agent benchmark。
2. SWE-bench Lite mini/full：证明 issue-to-patch coding agent 能力；Verified 作为内部 profile 或满足资格后提交。
3. BFCL-style diagnostic：便宜快速，作为 release-mini 的前置 guard。
4. AgentDojo security-mini：覆盖 prompt injection 和 untrusted tool output。
5. AppWorld/tau-bench：等 tool adapter/conversation harness 稳定后接入。
6. GAIA/WebArena/OSWorld/MLE-bench：暂不作为主线打榜目标。

换句话说，第一版目标不是“全榜都上”，而是把 opi-eval 做成可复现、可比较、可提交的评测管线。公开打榜只是这条管线的一个输出。


# opi-eval 的 Agent Benchmark 候选集

日期：2026-07-08

目的：识别可用于扩展 `.claude/skills/opi-eval` 的权威 agent 评测集，
补足当前小型回归用例之外的能力覆盖。这里的目标是评测 agent harness 和
terminal-first coding agent，而不是评测基础 LLM。因此最高优先级候选集应覆盖
工具选择、文件系统、shell、浏览器或 API 交互、多步状态变更，以及基于执行结果
的评分。

## 筛选标准

- 评测完整 agent loop，而不是只评测 next-token 或单次 function-call 预测。
- 有公开仓库、论文、benchmark 站点或维护中的 harness。
- 产出可回放 trace 或客观环境结果。
- 可适配 opi 的内置工具（`read`、`write`、`edit`、`bash`、`grep`、
  `find`、`ls`、`glob`），或未来的 extension tools。
- 有助于将 opi 与 Pi、Hermes Agent、OpenClaw 这类通用 agent 实现对齐比较。

## 推荐接入顺序

| 层级 | Benchmark | 为什么适合 opi-eval | 接入成本 |
|------|-----------|---------------------|----------|
| 1 | Terminal-Bench | 直接匹配 terminal-first coding agent：真实终端环境、端到端任务、容器化评分。 | 中高 |
| 1 | SWE-bench Verified / Lite | 标准软件工程 agent benchmark，使用真实 GitHub issue 和可执行测试。 | 中 |
| 1 | AgentDojo | 覆盖 prompt injection 和不可信工具输出安全性，这是工具型 agent 的真实风险。 | 中 |
| 1 | BFCL | 快速诊断可执行 function/tool-call 正确性；不是完整 agent benchmark，但对 tool-schema 回归很有价值。 | 中低 |
| 2 | AppWorld | 测试基于有状态 app 数据库的交互式编码和 API 调用。 | 中高 |
| 2 | GAIA | 有较好的通用 assistant/tool-use 信号；如果 opi 扩展 web/search/image 工具会更有用。 | 中 |
| 2 | AgentBench | 广泛的多环境 LLM-as-agent benchmark，包含 OS、DB、KG、网页购物、游戏和家庭任务。 | 中高 |
| 2 | tau-bench / tau2/tau3-bench | 测试带模拟用户和领域 API 的多轮政策遵循型 tool agents。 | 中 |
| 2 | R2E / R2E-Eval | 将仓库转成可执行编程 agent 环境；适合自定义 opi repo-level 任务。 | 高 |
| 3 | WebArena / VisualWebArena / WorkArena | 强浏览器 agent benchmark，但 opi 当前还没有一等浏览器工具。 | 高 |
| 3 | OSWorld / OSWorld-MCP | 强 computer-use/tool-invocation benchmark，但需要当前 opi 工具集之外的 GUI/MCP 集成。 | 高 |
| 3 | MLE-bench / MLAgentBench | 优秀的长程 ML engineering eval，但昂贵且慢。 | 高 |
| 3 | ToolSandbox / API-Bank / ToolBench | 有用的低层 tool-use 探针；单独看不太能代表完整 coding-agent 行为。 | 中低 |

## 一级候选

### Terminal-Bench

主要来源：
- Website: https://www.tbench.ai/
- GitHub: https://github.com/harbor-framework/terminal-bench
- Terminal-Bench 2 dataset: https://github.com/harbor-framework/terminal-bench-2

评测内容：真实终端环境中的自主任务完成能力，包括编译代码、训练模型、搭建服务、
调试，以及其他端到端命令行工作。公开站点将其描述为面向终端环境中 AI agent
的 benchmark；GitHub 仓库称其为测试真实终端环境中 AI agent 的 benchmark。
Terminal-Bench 2 使用 Harbor 在容器中运行 `terminal-bench@2.0`，任务包括
蛋白质组装、异步代码调试、安全漏洞修复等。

为什么适合 opi：这是最接近 opi 当前产品表面的外部 benchmark。opi 已经有
终端 agent、shell 执行、workspace 文件工具、JSON trace 模式和隔离 workspace。
Terminal-Bench 可以测试 agent 是否能选择命令、检查输出、从错误中恢复并完成
真实任务。

opi-eval 形态：
- 增加 Terminal-Bench adapter，在任务容器内启动 `opi --json --allow-mutating`。
- 捕获 `opi-eval` 已使用的同一套 NDJSON 信号。
- 将 benchmark 的成功/失败映射到现有 per-case report，并增加 task-level
  metadata，例如 dataset version、container image、timeout 和 score。

风险：
- 完整运行成本高且耗时。
- adapter 必须有清晰 sandbox，因为 Terminal-Bench 任务可执行任意 shell 命令。
- 版本 pinning 很重要；benchmark 版本和任务 registry 会演进。

建议：作为第一个外部集成。先从很小的 curated subset 开始，再增加 full-run 支持。

### SWE-bench Verified / Lite

主要来源：
- GitHub: https://github.com/swe-bench/SWE-bench
- Website / leaderboard: https://www.swebench.com/

评测内容：通过生成 patch 解决真实 GitHub 软件 issue，并通过仓库测试。SWE-bench
仓库说明该 benchmark 使用从 GitHub 收集的真实软件 issue，并要求模型生成 patch。
它也记录了基于 Docker 的可复现 evaluation。SWE-bench Verified 是由真实软件工程师
确认可解的 500 题 curated subset。

为什么适合 opi：opi 是 coding agent。SWE-bench 给 issue-to-patch 行为、
文件编辑、测试执行、错误恢复和最终 patch 正确性提供了被广泛认可的外部基线。

opi-eval 形态：
- 实现 agent wrapper：接收 repo 和 issue prompt，在准备好的 workspace 中运行
  opi，并产出 patch。
- 将 patch 交给 SWE-bench 的 Docker evaluation harness。
- 记录 tool-call 数、测试命令行为、retry、token 使用和最终 pass/fail。

风险：
- 经典 SWE-bench 可能更奖励 patch 生成，而不一定充分衡量交互式 agent 质量；
  需要同时分析 trace。
- Python-heavy 数据集不足以覆盖 Rust/TypeScript agent 行为。
- 完整 Verified 运行会消耗真实 API 费用；默认 smoke tier 应使用 Lite 或 pinned
  mini subset。

建议：顶级候选，但应作为独立的“expensive coding benchmark” profile，而不是默认
本地回归套件。

### AgentDojo

主要来源：
- GitHub: https://github.com/ethz-spylab/agentdojo
- Paper / OpenReview: https://openreview.net/forum?id=m1YYAQjO3w

评测内容：工具型 agent 在 prompt-injection 攻击下的 utility 和 adversarial
robustness。仓库将 AgentDojo 描述为用于评测 LLM agents 的 prompt-injection
攻击和防御的动态环境，并提供 benchmark runner。

为什么适合 opi：opi 的 agent loop 会读取工具输出、文件、类似网页的文档，以及
package/extension 数据。AgentDojo 可以系统评测 agent 是否在遵循用户目标的同时，
抵抗外部数据中嵌入的恶意指令。

opi-eval 形态：
- 增加 readonly/mutating safety benchmark profile。
- 将 opi 包装成被测 agent，并通过 adapter 暴露 benchmark tools。
- 扩展 report dimensions，增加 `utility`、`attack_success_rate` 和
  `security_policy_violation`。

风险：
- 需要 tool API adapter，而不只是 shell/file tools。
- 部分防御可能位于 prompt/tool policy，而不是 opi runtime core。

建议：高优先级，因为它测试正常 coding benchmark 覆盖不到的失败模式。

### Berkeley Function Calling Leaderboard (BFCL)

主要来源：
- GitHub: https://github.com/ShishirPatil/gorilla/tree/main/berkeley-function-call-leaderboard
- Leaderboard: https://gorilla.cs.berkeley.edu/leaderboard.html

评测内容：跨 tool-use 类别的可执行 function-call 正确性，包括多步调用、并行调用、
相关性判断和多轮场景。

为什么适合 opi：BFCL 不足以评测完整 coding agent，但它是低成本 tool-schema
fidelity 回归套件：选择正确工具、填充合法参数、避免无关调用，并处理多步/并行
工具计划。

opi-eval 形态：
- 增加 `tool-call-diagnostic` profile，通过 opi 的 tool-call path 运行 BFCL-style
  cases。
- 将 BFCL 分数归一化到现有 tool-call correctness dimension。
- 将它作为昂贵 Terminal-Bench/SWE-bench profiles 前面的快速 guard。

风险：
- 很多 BFCL 任务更偏向评测模型 function-calling 行为，而不是完整 agent runtime。
- BFCL 通过并不能证明 agent 能在真实文件系统、终端或仓库中正确行动。

建议：高优先级诊断套件，但不要作为 headline agent score。

## 二级候选

### AppWorld

主要来源：
- GitHub: https://github.com/StonyBrookNLP/appworld
- Website: https://appworld.dev/
- ACL paper: https://aclanthology.org/2024.acl-long.850/

评测内容：复杂日常自主 agent 任务，要求在模拟 app 世界中进行交互式编码和 API
调用。仓库说明每个任务都有 supervisor、instruction 和初始 app database state，
agent 必须编写包含 API 调用的代码来完成指令。

为什么适合 opi：它压力测试简单单元测试缺失的完整循环：理解任务、查看 API 文档、
写代码、调用工具、更新状态并验证结果。

opi-eval 形态：
- 在 AppWorld task workspaces 中运行 opi。
- 让 opi 通过 `bash` 编写并执行调用 AppWorld APIs 的代码。
- 使用 AppWorld 的 state-based evaluator 评分。

风险：
- 比 SWE-bench 需要更多 API/coding harness 工作。
- 可能需要 Python 环境 bootstrap 和谨慎的 dependency pinning。

建议：Terminal-Bench/SWE-bench 之后的强二阶段候选。

### GAIA

主要来源：
- Hugging Face org / leaderboard: https://huggingface.co/gaia-benchmark
- Paper: https://arxiv.org/abs/2311.12983

评测内容：需要 reasoning、multimodality、web browsing 和 tool-use proficiency 的
通用 AI assistant 任务。论文描述了 466 个真实世界问题，其中许多答案被保留用于
leaderboard。

为什么适合 opi：它是广泛的 agent benchmark，而不是 coding-only benchmark。
如果 opi 希望评测通用 assistant 行为、信息收集、tool routing 和 exact-answer
纪律，它会变得相关。

opi-eval 形态：
- 从 public validation tasks 中选择当前工具和本地 fixtures 可解决的任务开始。
- 只有在 opi 通过 extensions 支持外部 search/browser/image tools 后再加入这些工具。
- 对 exact answers 评分，并保留完整 transcripts。

风险：
- 没有 browser/search/multimodal tools 时，opi 只能运行收窄 subset。
- public validation data 可能被污染。

建议：适合作为 general-agent profile，暂不作为核心 coding-agent profile。

### AgentBench

主要来源：
- GitHub: https://github.com/THUDM/AgentBench
- OpenReview: https://openreview.net/forum?id=zAdUB0aCTQ

评测内容：跨多个环境的 LLM-as-agent 行为。仓库描述了八类环境，包括 operating
systems、databases、knowledge graphs、digital card games、lateral-thinking
puzzles、web shopping、household tasks 和 web browsing。

为什么适合 opi：AgentBench 是自主 agent 行为的广泛基线，其中 OS/database-style
任务比纯聊天 benchmark 更接近 opi 的 tool loop。它不如 Terminal-Bench 直接对齐，
因为 opi 的产品表面是 coding/terminal-first，而不是通用模拟 agent。

opi-eval 形态：
- 如果 OS 和 DB subsets 可适配当前 shell/file tools，先从这些子集开始。
- 存储 environment name、task id、action trace 和 benchmark-native reward。
- 将其作为 cross-domain agent profile，而不是主 release gate。

风险：
- 一些环境与 terminal coding agent 相关性较弱。
- 八类环境的 harness adaptation 可能不均匀。

建议：在 terminal/coding/security profiles 到位后，可作为第二阶段 broad-agent
benchmark。

### tau-bench / tau2-bench / tau3-bench

主要来源：
- tau-bench GitHub: https://github.com/sierra-research/tau-bench
- tau-bench site: https://taubench.com/
- tau2/tau3 GitHub: https://github.com/sierra-research/tau2-bench

评测内容：模拟用户与配备领域 API 工具和 policy guidelines 的 language agent
之间的动态对话。较新的 tau2/tau3 仓库增加了 domains 和 evaluation modalities。

为什么适合 opi：它评测多轮对话中的工具选择、policy adherence、database state
changes、clarification 和 recovery。

opi-eval 形态：
- 增加 domain-tool adapter layer，而不是把一切都映射到 shell。
- 根据最终 world state 和 policy compliance 评分。
- 保存 user simulator transcripts 作为 artifacts。

风险：
- customer-service domains 与 terminal coding agent 的相关性弱于 coding benchmark。
- 需要 conversation harness，而不只是 single prompt runner。

建议：当 opi 有更强 extension/tool adapters 后，适合评测 general agent maturity。

### R2E / R2E-Eval

主要来源：
- Website: https://r2e.dev/
- GitHub: https://github.com/r2e-project/r2e
- ICML paper page: https://proceedings.mlr.press/v235/jain24c.html
- R2E-Gym: https://github.com/R2E-Gym/R2E-Gym

评测内容：通过将 GitHub repositories 转成带 generated equivalence tests 的可执行
环境，评测 repository-level programming-agent 行为。该项目明确面向 static code
generation models 和 interactive programming agents。R2E-Gym 进一步提供可执行
SWE-agent environments、agent trajectories、基于 unit tests 的 reward 计算，以及
SWE-bench-compatible evaluation workflows。

为什么适合 opi：它可以为 SWE-bench 的 Python issue 集之外的仓库创建
project-specific、execution-graded coding tasks。这对 opi 自身 Rust workspace 和
private repos 都有用。

opi-eval 形态：
- 使用 R2E 生成或导入 repo environments。
- 让 opi 处理 function/method-level tasks，并通过 generated tests 评分。
- 保留一个小型 pinned local R2E-derived suite 作为回归。

风险：
- generated tests 可能有噪声；在作为 release gate 前需要人工 review。
- setup 和 generation complexity 高于固定 benchmark datasets。
- R2E-Gym images 可能很大，因此需要显式 disk 和 cache controls。

建议：适合自定义 opi-native evals，但不应作为第一个外部 benchmark 集成。

## 三级 / 专项候选

### WebArena, VisualWebArena, WorkArena

主要来源：
- WebArena GitHub: https://github.com/web-arena-x/webarena
- WebArena website: https://webarena.dev/
- VisualWebArena GitHub: https://github.com/web-arena-x/visualwebarena
- WorkArena GitHub: https://github.com/ServiceNow/workarena
- WorkArena site: https://servicenow.github.io/WorkArena/
- MiniWoB++ benchmark tasks: https://github.com/Farama-Foundation/miniwob-plusplus
- BrowserGym unified harness: https://github.com/ServiceNow/BrowserGym

评测内容：
- WebArena：在自托管真实网站中操作的自主 agent。
- VisualWebArena：需要图文理解和网站操作的多模态 web tasks。
- WorkArena：基于 ServiceNow 的企业知识工作浏览器任务。
- MiniWoB++：较小的 browser/UI-control tasks，真实度较低，但可用于确定性的
  action-loop 回归。

BrowserGym 是一个实用集成路径，因为它用 Gym-style environment API 封装了
MiniWoB、WebArena、VisualWebArena、WorkArena、AssistantBench 和其他浏览器
benchmarks。

为什么适合以后接入：这些是强浏览器 agent benchmarks，但 opi 当前内置工具集中
没有一等 browser-control tool。

建议：等 browser tooling 作为稳定 opi extension 或 built-in package 存在后再接入。
届时先用 WebArena，WorkArena 用于企业 workflow，VisualWebArena 只在 image/
multimodal observation support 稳定后使用。

### OSWorld / OSWorld-MCP

主要来源：
- OSWorld GitHub: https://github.com/xlang-ai/OSWorld
- OSWorld site: https://os-world.github.io/
- OSWorld-MCP GitHub: https://github.com/X-PLUG/OSWorld-MCP

评测内容：真实桌面环境中的 open-ended computer-use tasks。OSWorld-MCP 将这一方向
扩展为同时衡量 GUI 操作、MCP tool invocation 和 decision-making。

为什么适合以后接入：opi 有 image attachment support 和 terminal tooling，但没有
通用桌面控制 action space。如果 opi 增加 MCP/package-driven desktop 或 app tools，
OSWorld-MCP 会特别相关。

建议：持续关注；在 opi 有稳定 GUI/MCP tool surface 前不要集成。

### MLE-bench / MLAgentBench

主要来源：
- MLE-bench GitHub: https://github.com/openai/mle-bench
- MLE-bench OpenReview: https://openreview.net/forum?id=6s5uXNWGIh
- MLAgentBench GitHub: https://github.com/snap-stanford/MLAgentBench
- MLAgentBench paper: https://arxiv.org/abs/2310.03302

评测内容：
- MLE-bench：覆盖 75 个 Kaggle competitions 的 ML engineering agents，并带有
  preparation 和 grading scripts。
- MLAgentBench：端到端 ML experimentation tasks，agent 需要读写文件、执行代码、
  检查输出并改进模型。

为什么适合以后接入：它们是优秀的长程 agent benchmarks，能够压力测试 planning、
experiment management 和 shell/code use。但它们也很慢、昂贵且依赖较重。

建议：作为可选的 “expensive long-horizon” profile，而不是默认 regression gate。

### ToolSandbox, API-Bank, ToolBench

主要来源：
- ToolSandbox GitHub: https://github.com/apple/ToolSandbox
- API-Bank GitHub: https://github.com/AlibabaResearch/DAMO-ConvAI/tree/main/api-bank
- API-Bank paper: https://aclanthology.org/2023.emnlp-main.187/
- ToolBench GitHub: https://github.com/OpenBMB/ToolBench

评测内容：tool/function calling、有状态 tool-use conversations、API
planning/retrieval/calling，以及执行成功率。

为什么对 opi 是低优先级：它们对隔离 tool-call quality 有用，但很多更接近
LLM/tool-call evaluation，而不是完整 coding-agent evaluation。它们应作为补充，
而不是替代 task-environment benchmarks。

建议：借鉴任务思路和 scoring dimensions 放入 opi 的 local regression suite；完整
harness 应等更高信号的 agent benchmarks 和 BFCL-style diagnostics 到位后再集成。

## 竞品对齐笔记

Pi：
- Pi 的 README 将其定位为 agent harness 和交互式 coding-agent CLI，具备 tool
  calling 和 state management：https://github.com/earendil-works/pi
- 一个 Pi discussion 报告了 Terminal-Bench 2.0 failures，原因是每轮 32K output
  cap，包括若干任务中 zero tool calls：
  https://github.com/earendil-works/pi/discussions/1606
- 对 opi-eval 的启示：Terminal-Bench 可以暴露普通 prompt-answer eval 看不到的
  harness-level failures，尤其是 thinking-budget/tool-call interaction bugs。

Hermes Agent：
- 已检查部分中，Hermes Agent 主 README 没有显式暴露 first-party agent benchmark，
  但仓库包含围绕 `lm-evaluation-harness` 的 model benchmarking skills，主要属于
  LLM benchmarks：
  https://github.com/NousResearch/hermes-agent/blob/main/skills/mlops/evaluation/lm-evaluation-harness/SKILL.md
- Hermes issues 讨论了跟踪 tool calls、state changes、transcripts 和 outcomes 的
  agent evals：
  https://github.com/NousResearch/hermes-agent/issues/44000
- Hermes issues 也讨论了 YC-Bench 作为 long-horizon strategic agent benchmark：
  https://github.com/NousResearch/hermes-agent/issues/340
- 对 opi-eval 的启示：不要把纯 LLM-eval workflows 当作主要信号；agent 标准应坚持
  transcript/outcome-based grading。

OpenClaw：
- OpenClaw README 描述的是本地个人 assistant，具备 channels、tools、
  browser/canvas/nodes/cron/session capabilities 和 sandboxing：
  https://github.com/openclaw/openclaw
- 一个 OpenClaw issue 明确询问通过 HAL harness 定期运行 SWE-bench Verified
  evaluation：
  https://github.com/openclaw/openclaw/issues/41039
- PinchBench 是面向 OpenClaw 的 benchmark skill，包含 productivity、research、
  writing、coding 和 analysis 五类共 53 个任务：
  https://github.com/pinchbench/skill
- WildClawBench 是 OpenClaw-environment benchmark，包含 60 个端到端任务：
  https://github.com/internlm/WildClawBench
- 对 opi-eval 的启示：OpenClaw-style evals 强调 coding 之外的广泛 assistant
  workflows。对 opi 来说，除非这些仓库成为稳定且广泛采用的 benchmark standard，
  否则更适合作为自定义本地 cases 的灵感来源。

## 建议的 opi-eval 结构

保留当前小型 cases 作为 `local-smoke`。新增外部 benchmark profiles，并明确成本/
运行时间预期：

| Profile | 内容 | 默认运行？ |
|---------|------|------------|
| `local-smoke` | 当前 candy/tool_chain/context_retention，以及小型自定义 tool-policy cases。 | 是 |
| `coding-mini` | 5-20 个 pinned SWE-bench Lite/Verified 或 R2E-derived cases。 | 否 |
| `terminal-mini` | 5-10 个 pinned Terminal-Bench tasks。 | 否 |
| `tool-call-diagnostic` | BFCL-style function/tool-call cases，加本地 malformed-argument 和 irrelevant-tool cases。 | 否 |
| `security-mini` | 小型 AgentDojo suite。 | 否 |
| `general-agent` | GAIA validation subset，加 tau-bench/AppWorld tasks（当 tool adapters 存在时）。 | 否 |
| `long-horizon` | MLE-bench/MLAgentBench samples。 | 否 |

在集成外部 suites 前，report schema 应增加这些字段：

- `benchmark`：名称、版本、source URL。
- `task_id`：上游 task identifier。
- `environment`：container/image 或 harness version。
- `score`：benchmark-native score，加归一化 PASS/DEGRADED/FAIL/ERROR。
- `transcript_path`：指向 raw NDJSON/tool trace。
- `grader`：code、model、human 或 benchmark-native。
- `cost_estimate_usd` 和 `time_ms`。
- `sandbox_notes`：尤其适用于 shell/browser/API benchmarks。

## 结论

针对 opi 当前产品表面，最佳顺序是：

1. Terminal-Bench mini adapter。
2. SWE-bench Lite/Verified mini adapter。
3. BFCL-style tool-call diagnostic profile。
4. AgentDojo safety/tool-output adapter。
5. 当 domain-tool adapters 存在后，接入 AppWorld 或 tau-bench。
6. GAIA/WebArena/OSWorld/MLE-bench 仅作为可选 broader-agent profiles。

这会给 opi-eval 建立一条平衡的评测阶梯：廉价本地回归、终端自主性、真实 coding
issues、对抗性工具安全，以及更广泛的 agent workflows。

# Agent benchmark candidates for opi-eval

Date: 2026-07-08

Purpose: identify authoritative agent benchmarks that could expand
`.claude/skills/opi-eval` beyond the current small regression cases. The target
is an agent harness and terminal-first coding agent, not a base LLM, so the
highest-priority candidates exercise tool choice, filesystem/shell/browser/API
interaction, multi-step state changes, and execution-based grading.

## Selection criteria

- Evaluates the whole agent loop, not only next-token or single function-call
  prediction.
- Has a public repository, paper, benchmark site, or maintained harness.
- Produces replayable traces or objective environment outcomes.
- Can be adapted to opi's built-in tools (`read`, `write`, `edit`, `bash`,
  `grep`, `find`, `ls`, `glob`) or to future extension tools.
- Helps compare opi against Pi, Hermes Agent, and OpenClaw-style general agent
  implementations.

## Recommended adoption order

| Tier | Benchmark | Why it belongs in opi-eval | Integration cost |
|------|-----------|----------------------------|------------------|
| 1 | Terminal-Bench | Directly matches a terminal-first coding agent: real terminal environments, end-to-end tasks, containerized scoring. | Medium-high |
| 1 | SWE-bench Verified / Lite | Standard software-engineering agent benchmark using real GitHub issues and executable tests. | Medium |
| 1 | AgentDojo | Covers prompt-injection and untrusted tool-output safety, a real risk for tool agents. | Medium |
| 1 | BFCL | Fast diagnostic for executable function/tool-call correctness; not a full agent benchmark, but valuable for tool-schema regression. | Low-medium |
| 2 | AppWorld | Tests interactive coding plus API calls over stateful app databases. | Medium-high |
| 2 | GAIA | Good general assistant/tool-use signal; useful if opi grows web/search/image tools. | Medium |
| 2 | AgentBench | Broad multi-environment LLM-as-agent benchmark, including OS, DB, KG, web shopping, games, and household tasks. | Medium-high |
| 2 | tau-bench / tau2/tau3-bench | Tests multi-turn policy-following tool agents with simulated users and domain APIs. | Medium |
| 2 | R2E / R2E-Eval | Turns repositories into executable programming-agent environments; useful for custom opi repo-level tasks. | High |
| 3 | WebArena / VisualWebArena / WorkArena | Strong browser-agent benchmarks, but opi currently lacks a first-class browser tool. | High |
| 3 | OSWorld / OSWorld-MCP | Strong computer-use/tool-invocation benchmark, but needs GUI/MCP integration beyond current opi tools. | High |
| 3 | MLE-bench / MLAgentBench | Excellent long-horizon ML-engineering evals, but expensive and slow. | High |
| 3 | ToolSandbox / API-Bank / ToolBench | Useful lower-level tool-use probes; less representative of full coding-agent behavior by themselves. | Low-medium |

## Tier 1 candidates

### Terminal-Bench

Primary sources:
- Website: https://www.tbench.ai/
- GitHub: https://github.com/harbor-framework/terminal-bench
- Terminal-Bench 2 dataset: https://github.com/harbor-framework/terminal-bench-2

What it measures: autonomous task completion in real terminal environments,
including compiling code, training models, setting up services, debugging, and
other end-to-end command-line work. The public site describes it as benchmarks
for AI agents in terminal environments; the GitHub repository calls it a
benchmark for testing AI agents in real terminal environments. Terminal-Bench 2
uses Harbor to run `terminal-bench@2.0` in containers and includes tasks such
as protein assembly, async-code debugging, and security-vulnerability repair.

Why it fits opi: this is the closest external benchmark to opi's actual product
surface. opi already has a terminal agent, shell execution, workspace file
tools, JSON trace mode, and isolated workspaces. Terminal-Bench can test whether
the agent can choose commands, inspect outputs, recover from errors, and finish
real tasks.

opi-eval shape:
- Add a Terminal-Bench adapter that launches `opi --json --allow-mutating`
  inside the task container.
- Capture the same NDJSON signals already used by `opi-eval`.
- Map benchmark success/failure to the existing per-case report, while adding
  task-level metadata such as dataset version, container image, timeout, and
  score.

Risks:
- Full runs are costly and slow.
- The adapter must enforce clear sandboxing because Terminal-Bench tasks can
  execute arbitrary shell commands.
- Version pinning matters; benchmark versions and task registries evolve.

Recommendation: first external integration. Start with a tiny curated subset,
then add full-run support.

### SWE-bench Verified / Lite

Primary sources:
- GitHub: https://github.com/swe-bench/SWE-bench
- Website / leaderboard: https://www.swebench.com/

What it measures: resolving real-world GitHub software issues by generating a
patch that passes repository tests. The SWE-bench repository states that the
benchmark uses real software issues collected from GitHub and tasks a model with
generating a patch. It also documents Docker-based reproducible evaluation.
SWE-bench Verified is a curated 500-problem subset confirmed solvable by real
software engineers.

Why it fits opi: opi is a coding agent. SWE-bench gives a recognized external
baseline for issue-to-patch behavior, file editing, test execution, error
recovery, and final patch correctness.

opi-eval shape:
- Implement an agent wrapper that receives a repo and issue prompt, runs opi in
  the prepared workspace, and emits a patch.
- Feed the patch to SWE-bench's Docker evaluation harness.
- Record tool-call count, test command behavior, retries, token use, and final
  pass/fail.

Risks:
- Classic SWE-bench can reward patch generation more than interactive agent
  quality unless traces are also analyzed.
- Python-heavy dataset does not cover Rust/TypeScript agent behavior enough.
- Full Verified runs cost real API money; Lite or a pinned mini subset should
  be the default smoke tier.

Recommendation: top-tier, but run as a separate "expensive coding benchmark"
profile rather than the default local regression suite.

### AgentDojo

Primary sources:
- GitHub: https://github.com/ethz-spylab/agentdojo
- Paper / OpenReview: https://openreview.net/forum?id=m1YYAQjO3w

What it measures: utility and adversarial robustness for tool-using agents
under prompt-injection attacks. The repository describes AgentDojo as a dynamic
environment for evaluating prompt-injection attacks and defenses for LLM agents
and provides a benchmark runner.

Why it fits opi: opi's agent loop will read tool outputs, files, web-like
documents, and package/extension data. AgentDojo gives a principled way to test
whether the agent follows the user objective while resisting malicious
instructions embedded in external data.

opi-eval shape:
- Add a readonly/mutating safety benchmark profile.
- Wrap opi as the agent under test and expose the benchmark tools through an
  adapter.
- Extend report dimensions with `utility`, `attack_success_rate`, and
  `security_policy_violation`.

Risks:
- Requires tool API adaptation, not just shell/file tools.
- Some defenses may live in prompts/tool policy rather than core opi runtime.

Recommendation: high priority because it tests failure modes that normal coding
benchmarks miss.

### Berkeley Function Calling Leaderboard (BFCL)

Primary sources:
- GitHub: https://github.com/ShishirPatil/gorilla/tree/main/berkeley-function-call-leaderboard
- Leaderboard: https://gorilla.cs.berkeley.edu/leaderboard.html

What it measures: executable function-call correctness across tool-use
categories, including multi-step calls, parallel calls, relevance detection,
and multi-turn scenarios.

Why it fits opi: BFCL is not enough to evaluate a whole coding agent, but it is
a useful low-cost regression suite for tool-schema fidelity: choosing the right
tool, filling valid arguments, avoiding irrelevant calls, and handling
multi-step/parallel tool plans.

opi-eval shape:
- Add a `tool-call-diagnostic` profile that runs BFCL-style cases through opi's
  tool-call path.
- Normalize BFCL scores into the existing tool-call correctness dimension.
- Use this as a fast guard before expensive Terminal-Bench/SWE-bench profiles.

Risks:
- Many BFCL tasks evaluate model function-calling behavior more than the full
  agent runtime.
- A passing BFCL score does not prove the agent can act correctly in a real
  filesystem, terminal, or repository.

Recommendation: high-priority diagnostic suite, but never the headline agent
score.

## Tier 2 candidates

### AppWorld

Primary sources:
- GitHub: https://github.com/StonyBrookNLP/appworld
- Website: https://appworld.dev/
- ACL paper: https://aclanthology.org/2024.acl-long.850/

What it measures: complex day-to-day autonomous agent tasks requiring
interactive coding and API calls over a simulated world of apps. The repository
states that each task has a supervisor, instruction, and initial app database
state, and that the agent must write code containing API calls to complete the
instruction.

Why it fits opi: it stresses the exact loop that simple unit tests miss:
understand a task, inspect API docs, write code, call tools, update state, and
verify an outcome.

opi-eval shape:
- Run opi in AppWorld task workspaces.
- Let opi write and execute code against AppWorld APIs through `bash`.
- Grade using AppWorld's state-based evaluator.

Risks:
- More API/coding harness work than SWE-bench.
- May require Python environment bootstrapping and careful dependency pinning.

Recommendation: strong second wave after Terminal-Bench/SWE-bench.

### GAIA

Primary sources:
- Hugging Face org / leaderboard: https://huggingface.co/gaia-benchmark
- Paper: https://arxiv.org/abs/2311.12983

What it measures: general AI assistant tasks requiring reasoning, multimodality,
web browsing, and tool-use proficiency. The paper describes 466 real-world
questions with many answers held out for a leaderboard.

Why it fits opi: it is a broad agent benchmark, not a coding-only benchmark.
It becomes relevant if opi wants to evaluate general assistant behavior,
information gathering, tool routing, and exact-answer discipline.

opi-eval shape:
- Start with public validation tasks that can be solved using current tools and
  local fixtures.
- Add external search/browser/image tools only when opi supports them through
  extensions.
- Score exact answers and preserve full transcripts.

Risks:
- Without browser/search/multimodal tools, opi would need a narrowed subset.
- Public validation data can become contaminated.

Recommendation: useful as a general-agent profile, not a core coding-agent
profile yet.

### AgentBench

Primary sources:
- GitHub: https://github.com/THUDM/AgentBench
- OpenReview: https://openreview.net/forum?id=zAdUB0aCTQ

What it measures: LLM-as-agent behavior across multiple environments. The
repository describes eight environments, including operating systems,
databases, knowledge graphs, digital card games, lateral-thinking puzzles, web
shopping, household tasks, and web browsing.

Why it fits opi: AgentBench is a broad baseline for autonomous-agent behavior
and includes OS/database-style tasks that are closer to opi's tool loop than
pure chat benchmarks. It is less directly aligned than Terminal-Bench because
opi's product surface is coding/terminal-first rather than a universal
simulation agent.

opi-eval shape:
- Start with OS and DB subsets if they can be adapted to current shell/file
  tools.
- Store environment name, task id, action trace, and benchmark-native reward.
- Treat it as a cross-domain agent profile, not as the main release gate.

Risks:
- Some environments are less relevant to a terminal coding agent.
- Harness adaptation can be uneven across the eight environments.

Recommendation: useful as a second-wave broad-agent benchmark after the
terminal/coding/security profiles are in place.

### tau-bench / tau2-bench / tau3-bench

Primary sources:
- tau-bench GitHub: https://github.com/sierra-research/tau-bench
- tau-bench site: https://taubench.com/
- tau2/tau3 GitHub: https://github.com/sierra-research/tau2-bench

What it measures: dynamic conversations between a simulated user and a language
agent equipped with domain-specific API tools and policy guidelines. The newer
tau2/tau3 repository adds domains and evaluation modalities.

Why it fits opi: it evaluates tool selection, policy adherence, database state
changes, clarification, and recovery in multi-turn conversations.

opi-eval shape:
- Add a domain-tool adapter layer rather than mapping everything through shell.
- Grade final world state and policy compliance.
- Store user simulator transcripts as artifacts.

Risks:
- Customer-service domains are less aligned with a terminal coding agent.
- Requires a conversation harness, not just a single prompt runner.

Recommendation: good for general agent maturity once opi has stronger
extension/tool adapters.

### R2E / R2E-Eval

Primary sources:
- Website: https://r2e.dev/
- GitHub: https://github.com/r2e-project/r2e
- ICML paper page: https://proceedings.mlr.press/v235/jain24c.html
- R2E-Gym: https://github.com/R2E-Gym/R2E-Gym

What it measures: repository-level programming-agent behavior by converting
GitHub repositories into executable environments with generated equivalence
tests. The project explicitly targets static code generation models and
interactive programming agents. R2E-Gym extends this direction with executable
SWE-agent environments, agent trajectories, reward calculation through unit
tests, and SWE-bench-compatible evaluation workflows.

Why it fits opi: it can create project-specific, execution-graded coding tasks
for repositories beyond SWE-bench's Python issue set. This is useful for opi's
own Rust workspace and for private repos.

opi-eval shape:
- Use R2E to generate or import repo environments.
- Run opi against function/method-level tasks and grade through generated tests.
- Keep a small pinned local R2E-derived suite for regression.

Risks:
- Generated tests can be noisy; human review is needed before treating tasks as
  release gates.
- Setup and generation complexity is higher than fixed benchmark datasets.
- R2E-Gym images can be large, so disk and cache controls need to be explicit.

Recommendation: promising for custom opi-native evals, but not the first
external benchmark to integrate.

## Tier 3 / specialized candidates

### WebArena, VisualWebArena, WorkArena

Primary sources:
- WebArena GitHub: https://github.com/web-arena-x/webarena
- WebArena website: https://webarena.dev/
- VisualWebArena GitHub: https://github.com/web-arena-x/visualwebarena
- WorkArena GitHub: https://github.com/ServiceNow/workarena
- WorkArena site: https://servicenow.github.io/WorkArena/
- MiniWoB++ benchmark tasks: https://github.com/Farama-Foundation/miniwob-plusplus
- BrowserGym unified harness: https://github.com/ServiceNow/BrowserGym

What they measure:
- WebArena: autonomous agents operating in self-hosted realistic websites.
- VisualWebArena: multimodal web tasks requiring image-text understanding and
  website actions.
- WorkArena: browser-based ServiceNow knowledge-worker tasks.
- MiniWoB++: smaller browser/UI-control tasks that are less realistic but can
  be useful for deterministic action-loop regression.
BrowserGym is a practical integration path because it packages MiniWoB,
WebArena, VisualWebArena, WorkArena, AssistantBench, and other browser
benchmarks behind a Gym-style environment API.

Why they fit later: these are strong browser-agent benchmarks, but opi currently
does not expose a first-class browser-control tool in its built-in tool set.

Recommendation: defer until browser tooling exists as a stable opi extension or
built-in package. Then use WebArena first, WorkArena for enterprise workflow,
and VisualWebArena only when image/multimodal observation support is stable.

### OSWorld / OSWorld-MCP

Primary sources:
- OSWorld GitHub: https://github.com/xlang-ai/OSWorld
- OSWorld site: https://os-world.github.io/
- OSWorld-MCP GitHub: https://github.com/X-PLUG/OSWorld-MCP

What it measures: open-ended computer-use tasks in real desktop environments.
OSWorld-MCP extends the idea to jointly measure GUI operation, MCP tool
invocation, and decision-making.

Why it fits later: opi has image attachment support and terminal tooling, but
not a general desktop-control action space. OSWorld-MCP is especially relevant
if opi adds MCP/package-driven desktop or app tools.

Recommendation: monitor; do not integrate until opi has a stable GUI/MCP tool
surface to evaluate.

### MLE-bench / MLAgentBench

Primary sources:
- MLE-bench GitHub: https://github.com/openai/mle-bench
- MLE-bench OpenReview: https://openreview.net/forum?id=6s5uXNWGIh
- MLAgentBench GitHub: https://github.com/snap-stanford/MLAgentBench
- MLAgentBench paper: https://arxiv.org/abs/2310.03302

What they measure:
- MLE-bench: ML engineering agents across 75 Kaggle competitions, with
  preparation and grading scripts.
- MLAgentBench: end-to-end ML experimentation tasks where agents read/write
  files, execute code, inspect outputs, and improve models.

Why they fit later: they are excellent long-horizon agent benchmarks and stress
planning, experiment management, and shell/code use. They are also slow,
expensive, and dependency-heavy.

Recommendation: add as an optional "expensive long-horizon" profile, not as a
default regression gate.

### ToolSandbox, API-Bank, ToolBench

Primary sources:
- ToolSandbox GitHub: https://github.com/apple/ToolSandbox
- API-Bank GitHub: https://github.com/AlibabaResearch/DAMO-ConvAI/tree/main/api-bank
- API-Bank paper: https://aclanthology.org/2023.emnlp-main.187/
- ToolBench GitHub: https://github.com/OpenBMB/ToolBench

What they measure: tool/function calling, stateful tool-use conversations, API
planning/retrieval/calling, and execution success.

Why they are lower tier for opi: they are useful for isolating tool-call
quality, but many are closer to LLM/tool-call evaluation than full coding-agent
evaluation. They should complement, not replace, task-environment benchmarks.

Recommendation: borrow task ideas and scoring dimensions for opi's local
regression suite; integrate full harnesses only after higher-signal agent
benchmarks and BFCL-style diagnostics are in place.

## Competitor-alignment notes

Pi:
- Pi's README positions it as an agent harness plus interactive coding-agent
  CLI with tool calling and state management: https://github.com/earendil-works/pi
- A Pi discussion reports Terminal-Bench 2.0 failures caused by a 32K per-turn
  output cap, including zero tool calls on several tasks:
  https://github.com/earendil-works/pi/discussions/1606
- Implication for opi-eval: Terminal-Bench can expose harness-level failures
  that normal prompt-answer evals miss, especially thinking-budget/tool-call
  interaction bugs.

Hermes Agent:
- The main Hermes Agent README did not surface a first-party agent benchmark in
  the portions inspected, but the repository contains model benchmarking skills
  around `lm-evaluation-harness`, which are primarily LLM benchmarks:
  https://github.com/NousResearch/hermes-agent/blob/main/skills/mlops/evaluation/lm-evaluation-harness/SKILL.md
- Hermes issues discuss agent evals that track tool calls, state changes,
  transcripts, and outcomes:
  https://github.com/NousResearch/hermes-agent/issues/44000
- Hermes issues also discuss YC-Bench as a long-horizon strategic agent
  benchmark:
  https://github.com/NousResearch/hermes-agent/issues/340
- Implication for opi-eval: avoid copying pure LLM-eval workflows as the main
  signal; keep transcript/outcome-based grading as the agent standard.

OpenClaw:
- OpenClaw's README describes a local personal assistant with channels, tools,
  browser/canvas/nodes/cron/session capabilities, and sandboxing:
  https://github.com/openclaw/openclaw
- An OpenClaw issue explicitly asks about regular SWE-bench Verified evaluation
  through a HAL harness:
  https://github.com/openclaw/openclaw/issues/41039
- PinchBench is an OpenClaw-oriented benchmark skill with 53 tasks across
  productivity, research, writing, coding, and analysis:
  https://github.com/pinchbench/skill
- WildClawBench is an OpenClaw-environment benchmark with 60 end-to-end tasks:
  https://github.com/internlm/WildClawBench
- Implication for opi-eval: OpenClaw-style evals emphasize broad assistant
  workflows beyond coding. For opi, treat these as inspiration for custom
  local cases unless the repos become stable, widely adopted benchmark
  standards.

## Proposed opi-eval structure

Keep the current small cases as `local-smoke`. Add external benchmark profiles
with explicit cost/runtime expectations:

| Profile | Contents | Default? |
|---------|----------|----------|
| `local-smoke` | Current candy/tool_chain/context_retention plus small custom tool-policy cases. | Yes |
| `coding-mini` | 5-20 pinned SWE-bench Lite/Verified or R2E-derived cases. | No |
| `terminal-mini` | 5-10 pinned Terminal-Bench tasks. | No |
| `tool-call-diagnostic` | BFCL-style function/tool-call cases plus local malformed-argument and irrelevant-tool cases. | No |
| `security-mini` | A small AgentDojo suite. | No |
| `general-agent` | GAIA validation subset plus tau-bench/AppWorld tasks when tool adapters exist. | No |
| `long-horizon` | MLE-bench/MLAgentBench samples. | No |

The report schema should add these fields before integrating external suites:

- `benchmark`: name, version, source URL.
- `task_id`: upstream task identifier.
- `environment`: container/image or harness version.
- `score`: benchmark-native score plus normalized PASS/DEGRADED/FAIL/ERROR.
- `transcript_path`: pointer to raw NDJSON/tool trace.
- `grader`: code, model, human, or benchmark-native.
- `cost_estimate_usd` and `time_ms`.
- `sandbox_notes`: especially for shell/browser/API benchmarks.

## Bottom line

For opi's current product surface, the best sequence is:

1. Terminal-Bench mini adapter.
2. SWE-bench Lite/Verified mini adapter.
3. BFCL-style tool-call diagnostic profile.
4. AgentDojo safety/tool-output adapter.
5. AppWorld or tau-bench once domain-tool adapters exist.
6. GAIA/WebArena/OSWorld/MLE-bench only as optional broader-agent profiles.

This gives opi-eval a balanced ladder: cheap local regression, terminal
autonomy, real coding issues, adversarial tool safety, and broader agent
workflows.

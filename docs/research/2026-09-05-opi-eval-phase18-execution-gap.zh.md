# Phase 18 opi-eval 执行链与本地评测缺口核查

本文件是非规范研究证据，不是 Phase 19 实施计划、验收结论或实现进度记录。
核查日期：2026-09-05。工作区 HEAD：
`765608a55a78db47e8b41d2639c29ef8c90e0f31`。
另核对 Phase 18 native smoke 的历史候选提交
`27344e3aaf03d38eaa53c7af19c777efbe9be213`。

本文按中文讨论记录新调查，没有既有英文对应文件需要同步。

## 结论与证据边界

现有实现主要交付了实验契约、进程适配、生命周期、证据封存、离线校验和
conformance 报告基础。它尚不构成“用户配置 provider，在本机对 Opi/pi 跑
TB 2.1、TB 3.0、DeepSWE 任务集并得到可信成绩”的完整产品流程。

还存在比使用体验缺口更具体的执行链问题：提交中的 native producer 先在
宿主机运行 Opi/pi，再启动一个没有显式指定 agent 的 Harbor/Pier job。
锁定版本的 Harbor/Pier 在该调用下默认选择 oracle；先前 Opi/pi 的工作区
及输出没有接入这个 job。因而 native grader 的成功不能归因于 Opi/pi
完成了对应任务。

这里的执行链结论来自本仓库和锁定上游提交的静态代码核对，历史运行状态和
六条 reward 来自 GitHub 已有日志。本次没有运行 Opi/pi、provider、Docker
任务或 grader；没有下载、解包和重新验签完整的历史 artifact。
不能把本报告表述成一次新的动态复现或完整 artifact 审计。

## 1. 现有 CLI 的本地能力

`crates/opi-eval/README.md:44` 区分跨平台 `validate` 与仅由 Unix 验收支撑的
完整 fixture 演示。`:175` 明确默认 hermetic 模式使用确定性 helper 与本仓库
fixtures，证明适配、生命周期、封存、报告和失败路径，不证明真实模型能力。
`run --native-material` 可以读取本地 manifest，执行其中固定路径的程序；
`src/cli/run.rs:82` 和 `src/runner/material.rs:141` 没有 GitHub API 调用。

`regrade` 当前含义是离线重验已封存字节，并非重新执行官方 grader。
`report` 的固定分类是 `conformance-evidence`
（`src/report.rs:42`），README 也把真实 provider 行为评估转交给同名 workflow
skill（`:182`）。技能工作流和独立可执行模块不是同一个交付物。

## 2. 没有索要 provider 信息的直接原因

native smoke 使用 `scripts/scripted-provider.py`，其工具响应固定为调用
`bash` 写入 `answer.txt`（`:39`），随后在出现 tool result 时返回固定结束文本
（`:55`）。它不根据任务推理，也不调用付费模型。

producer 把 provider 固定为 `scripted`、model 固定为 `eval`
（`scripts/native-smoke.sh:1096`），material 中使用 dummy credentials
（`:1020`、`:1028`）。`src/runner/material.rs:43` 的 provider contract 本身
要求 script、endpoint、request_log；Opi 配置投影固定为 OpenAI Completions、
8192 context、4096 output（`src/runner/experiment.rs:853`）。

因此不询问真实凭证符合当前确定性测试目标，却不能证明用户自配置 provider 的
正式评测路径已经完成。仅把 dummy key 换成真 key，也不会修复后述评分断链。

## 3. 真实 agent binary 确实被调用，但在宿主机

`scripts/native-smoke.sh:1017` 将 manifest 的 executable 指向
`agent-opi-generic.sh` 和 `agent-pi-generic.sh`。`:1041`、`:1048` 生成的脚本
分别直接 `exec` 构建出的 Opi binary，以及 Node + pi bundle。

同一脚本确实另有每个 benchmark 的 Docker wrapper 生成函数（`:879`），但
这些 wrapper 在 `:1059` 被从 material 中移出，只放进 receipt；native runner
在 `src/runner/experiment.rs:1218` 使用的仍是 generic executable。
provider probe 注释也明确承认 agent surface 是 host-side（producer `:605`）。

所以“构建并调用了真实 Opi/pi”成立；“这两个 agent 已在官方任务容器中完成
任务”不由该调用链证明。workflow 中关于 agent-phase task containers 的注释
不能替代实际 executable 选择。

## 4. 评分没有消费 Opi/pi 的任务产物

runner 在 `src/runner/experiment.rs:1097` 创建独立的 host workspace，给 agent
传任务 instruction 和写 `answer.txt` 的附加要求（`:788`），随后另外复制原始
task package（`:1442`）。构造 grader request 时把 `answer_path` 填入
`agent_output`（`:1489`）。

但 `src/benchmark/process.rs:51` 对 `agent_output` 标了 `expect(dead_code)`。
全 crate 的引用检索只有字段定义与构造处，没有评分消费者。三个 adapter 的
`spawn_spec` 分别位于：

- `src/benchmark/terminal_bench_21.rs:746`；
- `src/benchmark/terminal_bench_30.rs:952`；
- `src/benchmark/deepswe.rs:1000`。

它们只生成 `uv run --locked harbor|pier run -p <task-dir>`，没有传入 agent
工作区、产物、patch、已有 trial，也没有指定 Opi/pi agent adapter。
producer 的 verifier wrapper（`:936`）和 oracle wrapper（`:955`）执行相同形式的
`uv "$@"`，区别是文件名、注释和被记录的用途。两者均复制最新 jobs 目录回
trace root，没有加入 agent 产物交接。

## 5. 锁定 Harbor/Pier 的默认执行者是 oracle

这不是依据最新版文档猜测。通过 GitHub Contents API 读取了仓库实际锁定的
完整提交，并检查以下代码：

| 上游提交 | 默认选择证据 | CLI 调用证据 |
| --- | --- | --- |
| Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` | [job config:385](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/models/job/config.py#L385) 默认 `[AgentConfig()]`；[trial config:166](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/models/trial/config.py#L166) 缺少 name/import_path 时设为 oracle | [main:164](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/cli/main.py#L164) 将 run 映射到 start；[jobs:1438](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/cli/jobs.py#L1438) 无显式配置时构造 JobConfig；[jobs:1481](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/cli/jobs.py#L1481) 仅显式 agent 覆盖默认值 |
| Pier `c1ebc6d145b40fae8425215e3fca528945065124` | [job config:234](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/models/job/config.py#L234) 与 [trial config:69](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/models/trial/config.py#L69) 同样默认 oracle | [main:35](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/cli/main.py#L35) 将 run 映射到 start；[jobs:601](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/cli/jobs.py#L601) 无配置时构造 JobConfig；[jobs:311](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/cli/jobs.py#L311) 的帮助文本也写明默认 oracle |

两者 oracle 都把官方 solution 上传进环境并执行，而非读取外部 Opi/pi 工作区：
[Harbor oracle:92](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/agents/oracle.py#L92)、
[Pier oracle:83](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/agents/oracle.py#L83)。

## 6. 历史上确实有 reward 数字，但不等于 agent 成绩

已读 [run 33271354427](https://github.com/OdradekAI/opi/actions/runs/33271354427)
的 metadata 和完整日志：事件是 workflow_dispatch，conclusion 为 success，
headSha 是上述 `27344e3`。日志包含三份 run-report；每份两个 trial 都显示
completed、sealed、verified，TB 2.1/TB 3.0 四条为
`known:1(harbor-result)`，DeepSWE 两条为 `known:1(pier-result)`。

通过 `git show 27344e3:<path>` 核对历史源码，generic host wrappers、移出的
Docker wrappers、未消费的 `agent_output`、未指定 agent 的 grader argv 均已存在。
故不是后来重命名脚本才引入的现象。静态执行链解释了为何固定写一个答案文本的
scripted agent 也能与官方成功 reward 同时出现在一份 outer trial receipt 中。

本次 API 读取到历史 artifact id `9720758617`，大小 `6131628911` bytes，
digest `sha256:12892746e012abc6a73142d8d968b39d68ef978872cb445f2a438f4771981539`，
未过期；upload receipt artifact id `9720758852`。未下载它们，故不声称直接检查了
archive 内每个 job config、agent trace 或封存字节。若进行下一轮正式缺陷验证，
应优先检查其中 `jobs/*/config.json` 与 per-trial result 的 agent identity，
并对当前修复路径做正确产物/错误产物的配对动态检查。

`src/benchmark/process.rs:140` 读取 `stats.evals` 的 values，未核验该 eval key 的
agent identity；artifact verifier 在 `scripts/verify-native-smoke-artifact.py:378`
检查 outer receipt 的 agent completed、verifier verified、known reward 与 digest
链，并未证明被 grader 消费的内容来自该 agent。封存能保全关联错误的记录，
不能自动补全实际的数据流。

## 7. GitHub 是验收载体，已交付路径却明显围绕它组织

native smoke workflow 是手动 dispatch，非 every-PR job，指定 ubuntu-24.04。
用它证明固定 Linux 环境下的集成兼容性合理；用户本地评测没有必然的 GitHub
依赖。Rust CLI 接收本地文件，不要求 workflow run 或 artifact id。

但 README 把 native materialization/fidelity verification 指向 GitHub workflow，
producer 绑定 workflow SHA、candidate、upload receipt。Phase 18 task 18.15 的
快照 session_notes 记录第 48 次 dispatch 才闭环，其中有脚本权限、换行摘要、
缺目录、JSON 未解析、参数遗漏等可以先在本地 Linux 排除的问题。这支持
“开发验证过度依赖远端 dispatch 循环”的判断；不支持“GitHub 是评测必需执行器”。

## 8. 本地 materialization 与 Windows 的准确边界

`scripts/materialize-external-locks.sh` 接收 candidate commit、static lock、output
directory，`:109`–`:110` 显式要求 Linux x86_64；`:162`–`:165` 给 GitHub 环境变量
提供 unknown/0 等本地默认值。因而产出 materialization 原始结果不需要 GitHub
服务器参与，但还要满足固定源码、工具链、Docker 和外部下载条件；将这些结果
作为现行“已验收 resolved lock”登记，仍受其 GitHub receipt 验收契约约束。

不能笼统说 Windows binary 完全不可运行：runner 有 Windows `.cmd` fixture
helper（`src/runner/experiment.rs:287`、`:358`），process 层有 Windows Job Object
实现（`src/process/tree.rs:38`）。本次没有发现 `run` 入口统一拒绝 Windows 的条件。
但 conformance helper 使用 `.sh`，完整演示仅获 Unix 验收；native producer 依赖
Bash、Linux 命令、Linux x86_64 pin 与 Docker。Windows 原生支持不能由 cross-compile
或 fixture compile 推断出来。Windows 主机上的 WSL2/Linux Docker 路线应作为
独立目标做本地验收，详见同日 Windows 可行性研究。

## 9. 达到用户目标需要补齐的最小闭环

下列是问题分解，不构成新的规范或任务登记：

1. 先修复评分归因：真实 agent 在任务环境内产生修改；官方 grader 消费同一次
   trial 的工作区或官方声明的 collected artifacts/patch。oracle 仅用于独立
   preflight。验收要证明错误产物失败、正确产物成功，不能只看两个进程均 exit 0。
2. 将本地运行作为主入口：在 Linux/WSL2 内完成环境检查、task 获取、构建缓存、
   agent 启动、评分和本地报告；无需 push、workflow dispatch 或 artifact upload。
3. 增加用户 provider/model 与凭证来源的明确配置，运行前显示任务数、重复次数、
   并发与预算；验证两个 agent 实际得到同等可实现的模型控制。
4. 从目前每个 benchmark 一个固定 task 扩展到明确选择的任务集；完整集、子集、
   样本数、失败、跳过和基础设施异常都保留在报告分母与标签中。
5. 保留 hermetic 单测/回归和必要 native integration smoke；正式用户成绩应由
   用户明确发起的真实 provider 本地运行产生，与 oracle/conformance 证据分开。

现有证据层、adapter 结构和进程管理有保留价值，不能据此宣称当前成绩路径已完成。
最优先的验证是评分确实随受测 agent 的产物变化，然后才是 provider 配置体验与
更大任务集运行。

## 本次核查命令与未执行项

- `git rev-parse HEAD`、`git status --short`：锁定本报告调查 HEAD；开始时工作区干净。
- `Get-Content`、`rg -n`：读取 README、workflow、producer、runner、三个 grader adapter、
  material schema、report、artifact verifier 和 Phase 18 snapshot。
- `git show 27344e3aaf03d38eaa53c7af19c777efbe9be213:<path>`：核对历史 producer、
  runner 与三个 benchmark adapter，确认关键执行链在历史提交中已存在。
- `gh api repos/<owner>/<repo>/contents/<path>?ref=<locked-commit>`：只读取得上表
  锁定 Harbor/Pier 源码；使用 `Accept: application/vnd.github.raw+json`。
- `gh run view 33271354427 --repo OdradekAI/opi --json headSha,conclusion,event,url,workflowName`
  和 `--log`：成功读取已有运行 metadata 与日志。
- `gh api repos/OdradekAI/opi/actions/runs/33271354427/artifacts`：成功读取现有 artifact
  metadata；未下载完整 artifact。

未运行 Cargo 编译或测试、provider、任务集、Docker 或远端 workflow；未编辑运行时代码、
配置、快照或实现 ledger；未创建分支、提交或发布。测试影响：`none`。

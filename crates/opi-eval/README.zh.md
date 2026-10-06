# opi-eval

> 冻结一份评测契约，运行可比较的 Agent harness trial，并离线重新验证 sealed
> evidence。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-eval` 用同一组显式模型参数、环境、任务和 trial 控制来比较多个 Agent
harness。它会在任何 Agent 启动前解析并冻结这些输入，把每次 trial 的证据写入
sealed bundle，并且只根据仍能通过验证的证据计算分数。报告也会将绑定一致的
未封存尝试保留为未经验证的诊断，不为其赋分。

本 crate 是未发布、Agent 中立的 workspace companion，不属于 `opi` 运行时，也
不会改变普通 `opi` 的行为。

## 什么时候使用

以下场景适合使用 `opi-eval`：

- 运行前确认实验定义完整，且所有关键输入都有摘要身份；
- 在相同控制条件下比较 baseline 与 candidate harness；
- 用固定的 conformance fixture 检查 Agent 或 benchmark adapter；
- 不重新运行 Agent 或 Provider，直接验证 sealed trial 证据；
- 生成规范化报告，包含通过验证的结果及明确的无效、未执行覆盖计数。

本文会反复使用以下术语：

| 术语 | 含义 |
|------|------|
| experiment | 已冻结的 subject、对比 edge、控制参数、环境和声明的 trial 集合。 |
| subject | 一份 Agent harness 配置，例如 baseline 或 candidate。 |
| edge | 实验声明的、有方向的 baseline 到 candidate 对比关系。 |
| trial | 一个 subject 在某个对比组内运行一个任务。 |
| sealed bundle | 一次已落定 trial 的内容寻址证据目录。seal 完成后，受覆盖的字节不可再修改。 |

普通编程 Agent 工作应直接使用 `opi`。下文命令用于评测 fixture 和证据工作流，
不是通用的 Agent 执行入口。

## 前置条件

- 从 workspace 根目录运行命令。
- 构建本 crate 需要 Rust 1.97 或更高版本。
- 示例实验和 fixture 来自当前仓库 checkout。
- `validate` 支持跨平台运行。本文提供的完整 `run` -> `verify` -> `report`
  流程和 fixture conformance 示例目前只由 Unix 验收测试保障；这些测试使用
  POSIX helper process。
- Hermetic fixture 模式不需要在线凭据或网络，也不会调用付费 Provider。

`opi-eval` 尚未发布，因此请通过 Cargo 调用，不要尝试从 crates.io 安装。

## 配置化 operator 与外部存储

Python operator 与唯一配置模板位于 [`operator/`](operator/)，由本 Independent
Companion 维护。在 workspace 根目录使用 Python 3.11 或更新版本创建一份显式私有 profile：

```sh
python3 crates/opi-eval/operator/evaluate.py init --config /absolute/private/eval.toml
python3 crates/opi-eval/operator/evaluate.py resolve --config /absolute/private/eval.toml
python3 crates/opi-eval/operator/evaluate.py baseline --config /absolute/private/eval.toml --bench tb21
python3 crates/opi-eval/operator/evaluate.py candidate --config /absolute/private/eval.toml --bench all
```

在这份 profile 中设置模型、连接、凭据来源、思考、重试、deadline、任务选择，以及外部
Linux workspace/cache 路径。`init` 拒绝覆盖已有文件。`resolve` 与准备不授权或启动付费模型；
准备可获取固定材料并执行本地客户端证明。审阅返回的身份、计划、路径、覆盖与实际限制。
技能接收调用者提供的 `EVAL_OPERATOR` 入口及显式 profile，不从技能安装位置发现源码或 runtime。

`baseline` 只选择已准入 pi 版本，不构建 Opi。`candidate` 每次重新构建当前 Opi 工作树，
包含影响构建的未提交输入，冻结实际源码/可执行文件并选择合格的已存 pi 快照。
缺少基线时保留独立 candidate，不自动启动 pi。获得精确冻结范围的授权后，执行返回的
family 或整组 evaluation operation：

```sh
python3 crates/opi-eval/operator/evaluate.py candidate --prepared /absolute/prepared-operation --execute
python3 crates/opi-eval/operator/evaluate.py compare --candidate /absolute/candidate/snapshot.json --out /absolute/fresh-comparison
```

`--bench all` 创建包含选定 family 计划的一次 evaluation，从首次真实 dispatch 起共用
deadline，包含 grading 与 retries；准备和后续离线报告不在真实运行计时区间。
每次 retry 使用新身份，首个有效 native 结果（包括零分）成为逻辑结果。离线比较可显式添加
`--baseline /absolute/pi/snapshot.json`，从 candidate 冻结的 workspace 解析外部目录，
核验 originals 并写独立衍生产物，不修改原始证据或分数。

外部 `storage.workspace` 包含：

```text
evaluations/YYYY/MM/<eval-id>/record.json
evaluations/YYYY/MM/<eval-id>/families/<bench>/
evaluations/YYYY/MM/<eval-id>/reports/report.json
evaluations/YYYY/MM/<eval-id>/reports/report.md
evaluations/YYYY/MM/<eval-id>/reports/report.zh.md
imports/
events/
archives/
leases/
```

Family 根保留准备包、native runs/attempts、快照、比较及准备/执行 handoff。已准备的 native evaluation 另有
`prepared-evaluation.json`、顶层 `handoff.json` 与 `result-handoff.json`；精确位置以返回引用为准。报告和目录写入全部
留在外部；`docs/eval/` 仅用于显式请求的结论导出。旧仓库索引不自动读取或写入。
操作返回 JSON 时包含 `retention` pin，指向独立的 `retention-handoff.json`，以及
`retention_status = applied|protected_excess|refused`；保留已冻结的 result handoff 与 native 结果。

独立离线比较会创建自己的外部评测记录与报告，保存两个输入及调用者指定的新输出的不可变引用，
这些引用保护仍需保留的源证据。Operator 不接管该输出树的维护；输出位于已保留证据或
operator 元数据内部时拒绝。

Native originals 是证据权威。发现过程直接读取逐评测 JSON 记录、不可变导入引用及生命周期
回执，不使用数据库或持久查询缓存。已有 `catalog.sqlite` 被忽略且不改动，不需要迁移或
reindex。只有 operator 写它的 records/events/imports；其他权限域交换不可变 bundles
和引用。发现元数据不能证明 native seal 或分数。

```sh
python3 crates/opi-eval/operator/evaluate.py list --config /absolute/private/eval.toml --action baseline --bench tb21 --limit 3
python3 crates/opi-eval/operator/evaluate.py show --config /absolute/private/eval.toml --id EVAL_ID
python3 crates/opi-eval/operator/evaluate.py import-index --config /absolute/private/eval.toml --index /absolute/legacy.jsonl
```

`list` 支持 action、benchmark、model、version、state、limit 与 offset 筛选；允许值见
`--help`。旧索引导入保留只读引用，不搬移或编辑原始证据；保留的 JSON 文件提供发现状态，
证据缺失或 drift 保持明确 gap。

新 profile 使用 `opi-eval-operator/2`；`/1` 保持可读，模型默认值不变，使用模板中的 retention 默认值，
不自动改写用户私有文件。`[baseline].max_age_days = 0` 关闭年龄筛选。唯一的
[`config.example.toml`](operator/config.example.toml) 只有一个 retention 设置：
`keep_latest_per_group = 3`。
历史结果有效性与新鲜度、版本/retirement 选择资格及存储可用性分别判断。

完成后的 native evaluation 和独立比较应用最新三次滚动 retention，按 Agent、benchmark-family
组合、模型/API、模型参数/thinking 及任务范围分组。每个 pi release 独立分桶；Opi commit、
Opi release 版本与 Companion build 不重置计数。按记录的完成时间排序，缺失时回退到创建时间/身份；
刚完成的 evaluation 保持保留。
Pin、引用、活动/可恢复任务与 cleanup/依赖不明项保持受保护；返回的 retention 结果说明额外
保留的记录与原因。Pi 仅同 release 桶内超量且不受保护的副本可被清理。滚动清理仅覆盖 operator
自有 evaluation 树；任意独立比较输出由调用者管理，共享材料、镜像及 Cargo cache 不参与。

已完成的准备失败在具备明确的分组非 live dispatch 证明时计入 retention。已停止的离线比较
在 command/stream cleanup 已确认时计入；cleanup 不明或 process 未结算时仍受保护。
即使 CLI 因异常拒绝且没有返回结果 JSON，失败摘要与 `retention-handoff.json` 仍会保存，
可直接检查这些文件。

手动保留证据维护仍须显式执行：

```sh
python3 crates/opi-eval/operator/evaluate.py pin --config /absolute/private/eval.toml --id EVAL_ID --reason "retain for review"
python3 crates/opi-eval/operator/evaluate.py retire --config /absolute/private/eval.toml --id EVAL_ID --reason "exclude from new selection"
python3 crates/opi-eval/operator/evaluate.py archive --config /absolute/private/eval.toml --id EVAL_ID --plan /absolute/fresh-plan.json
python3 crates/opi-eval/operator/evaluate.py gc --config /absolute/private/eval.toml --plan /absolute/fresh-plan.json
python3 crates/opi-eval/operator/evaluate.py apply-plan --config /absolute/private/eval.toml --plan /absolute/reviewed-plan.json
python3 crates/opi-eval/operator/evaluate.py restore --config /absolute/private/eval.toml --id EVAL_ID
```

`unpin` 与 `unretire` 使用相同 ID/reason 输入。Archive/GC 只预览绑定 inventory/digest
的计划；重复 `gc --id` 可限制选择范围。审阅计划后才显式 apply，执行前重新核验所有权、字节、
引用、lease 与 policy；inventory 改变即拒绝。选择仍受最新三次分组与相同保护约束。
共享材料、镜像及 Cargo cache 不在此维护范围内。

Restore 核验归档字节并恢复原 canonical 路径，不改写 seal 或绝对 material binding。
缺少缓存依赖时记录离线能力 gap；恢复不保证 regrade 或新的执行。运行源码归属已迁移完成，
完整独立运行包分发与 public packaging 仍为暂定。此次变更不扩大诊断 capture、pi pin 准入、
验收权限或付费执行授权。

## 快速开始：校验实验

下面的命令不会启动 Agent：

```sh
cargo run -p opi-eval -- validate \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml
```

成功后会输出一行摘要，其中包含 experiment id、schema、规范 manifest digest，
以及 subject、edge 和 trial 数量。示例会解析为 `local-paired-hermetic`，包含两个
subject、一条 edge 和两次 trial。

退出码 0 表示契约解析成功。输入无效或不完整时，命令以 1 退出，并将类型化诊断
写入 stderr。`validate` 不会创建 run root，也不会执行 trial。

## 完整 fixture 流程（Unix）

以下流程是 hermetic、fixture-grade 的本地流程。helper process 分别代替 `opi`、
`pi` 和原生 verifier；这些结果不代表真实 Agent 执行、真实 Provider 调用或官方
benchmark 环境。

先创建临时目录，但不要提前创建 run root 和报告文件：

```sh
DEMO_DIR="$(mktemp -d)"
RUN_ROOT="$DEMO_DIR/run"
REPORT_PATH="$DEMO_DIR/report.json"
```

### 0. 诊断并准备本地 material

```sh
cargo run -p opi-eval -- doctor --config experiment.toml
cargo run -p opi-eval -- prepare --config experiment.toml --out "$DEMO_DIR/material"
```

`doctor` 报告 Linux engine 可达性、架构、WSL/Docker 观测到的
CPU/内存/GPU/磁盘，以及缺失的 material；无法观测的项带原因说明，且
全程不启动任何 model。其 material 观测覆盖 `prepare` 校验的每一个
被选中的准备输入——profile 与 corpus、uv 工具 pin、runner 的
project/lock/interpreter/entrypoint、适用的 integration、scripted
provider 与 capture helper——每一项都带预期缓存位置；只有被选中的
product 才是必需项，且不下载、不执行任何内容。`doctor`、`prepare`、`plan` 与 `run` 中的每一个
观测者——engine、version、磁盘、context 与 GPU 探测——都在进程监督
器下运行，拥有有限的单次探测上限、16 KiB 捕获上限与经过验证的
自有进程树清理：停滞或溢出的观测者只会变成如实的有界拒绝原因，
绝不会造成无限等待。`prepare` 依据 pinned static lock 校验种子化的
本地缓存（`[storage].cache`），冻结所选 benchmark、subject 可执行文件
与全部被请求的 image 身份，将冻结任务的资源需求与本地观测比对
（未满足或无法观测都会整体拒绝，绝不静默收缩集合），并产出可被
`validate`/`run` 通过 `--native-material` 消费的 local-origin
native-material manifest。local origin 被如实记录，绝不伪造 GitHub
provenance：upload receipt 仍然只是 CI 的 transport claim。缓存缺失时，
若已种子化 pinned materializer 则触发受监督的获取（plan 与被捕获的
日志位于输出根下）；否则失败会点名缺失的身份及其获取方式。完整的
operator 演练见 `examples/local-evaluation/`。

选中 Opi 且需要冷构建时，必须成对提供 `--opi-source /local/git/repository`
与 `--opi-commit <full-40-hex>`。prepare 仅导出该本地 commit，验证 Cargo 锁定
输入后在导出目录构建；不会推断 HEAD、包含未提交文件或获取缺失的 Opi commit。
已验证的热缓存无需源码参数，pi-only 准备也无需构建 Opi。一次调用获取选中的
客户端、完整固定语料及锁定的 Harbor/Pier 环境。种子集合必须包含 materializer
与 `build-agent-artifacts.sh`。Agent 运行时资源和构建身份保留在已验证包中。

pi 模型数据从静态锁的 `subjects[id=pi].model_data_archive` 获取：包名和版本
必须匹配源码，并在解包和离线构建前验证精确归档 URL 与 SHA-256；不要求上游
shrinkwrap 提供 SRI。直接调用 pi 构建器时必须传入
`--static-lock /absolute/linux-x86_64.json`；本地 prepare 和 native-smoke
会显式传入各自声明的锁。仅构建 Opi 时不需要 pi 归档 pin。

本地 pi 包包含固定 bundle 的外部 chord、photon-node、jiti，以及 chord 所需的
esbuild 和 Linux x64 二进制。这些文件来自锁定的离线构建，安装钩子保持禁用。
prepare 校验包身份并拒绝越界链接或缺失的运行时文件，然后在隔离配置下执行有界的
独立版本、导入和原生 esbuild 检查，通过后才将运行包安装到缓存。

缓存中的 `corpora/<adapter>/tasks` 必须是完整、干净且版本固定的 Git 语料。
prepare 为选中任务生成 `packages/<adapter>/<task-id>` 和绑定 SHA-256 的
`profiles/<adapter>/<task-id>.toml`。material/6 分开记录完整语料的
root/commit/tasks-tree 与所选 `tasks` 映射；每项绑定 canonical ID、原生名称、
profile 和任务包。任务镜像及 Dockerfile 基础镜像必须解析为不可变摘要。已种子化
固定 materializer 时，prepare 按准入摘要拉取所选任务缺失的镜像，并在同一所选
engine 中复核后冻结；缺失或无效 pin 在拉取前拒绝。暖准备复用已验证镜像。
preview 保持只读；prepare 不构建镜像、不启动容器。

`tasks/` 的直接子项可以是固定的惰性元数据链接，例如
`tasks/README.md -> ../README.md`，但目标必须是同一固定 commit 中受跟踪的普通文件。
prepare 在写入清单前核对链接原文、目标字节和执行权限；语料摘要使用链接原文。
这些条目不会成为任务，也不会进入任务包。越界、链接链、链接祖先、任务目录内链接、
子模块及特殊文件均拒绝；运行时包和任务包仍禁止链接。相对配置/缓存路径保留配置
指定的位置，并生成绝对路径的冻结包身份。

需求记录分别保留 Agent/verifier 的 CPU、内存、存储、GPU 数量及类型、网络策略、
收集行为和阶段时限，以及固定版本默认值和来源。CPU、内存取自所选 Docker engine；
存储检查覆盖缓存、输出和 engine 存储命名空间。在 Linux/WSL 上，若所选本地 Unix
engine ID 唯一匹配一个 dockerd 命名空间，可通过有界、只读的 `wsl.exe` 观测读取
已运行 Docker Desktop 的存储。采样绑定 engine 根目录、进程和挂载身份，并再次
核验所选端点；不会启动发行版或容器。远程 engine、身份歧义、访问被拒以及失败
或超时的观测继续保留 unknown。注册 NVIDIA runtime 不能证明 GPU 容量。
必需的 unknown 在 job 或 model 启动前拒绝所选集合；显式子集预览仍保留完整清单
与排除原因。

material/4 与 run-plan/1 必须重新生成，不做隐式转换；已有密封 bundle 继续可读，
不改写其原始证据。

### 0b. 预览并冻结 selection 计划

```sh
cargo run -p opi-eval -- plan --config experiment.toml \
  --native-material "$DEMO_DIR/material/material.json" \
  --out "$DEMO_DIR/plan.json"
cargo run -p opi-eval -- run --plan "$DEMO_DIR/plan.json" \
  --root "$DEMO_DIR/run"
```

`run` 必须且只能提供 `--config` 或 `--plan` 中的一个。原生计划复用其记录的
material；显式提供的 `--native-material` 必须匹配冻结的 manifest 摘要。
原生执行无需 `--fixtures`，hermetic 执行仍要求提供。计划或 material 输入
缺失、变更时，在任何 run 副作用发生前拒绝。

`plan` 仅依据 `task.toml` 元数据枚举 pinned corpus 任务清单（不拉取
也不启动任何 image），将文档 `[selection]`（`scope=full|subset`、显式
`task_ids`、记录的 `reason`、`repetitions`、可选按任务 `seeds`）展开
为精确的 trial 集合，并把每个被选任务的声明资源需求与本地观测比对：
未满足或无法观测都会整体拒绝——集合绝不静默收缩——同时预览保留完整
清单的逐任务状态。`opi-eval-plan-preview/2` 预览 JSON 展示 subjects、connection、任务清单、
selection、trial 总数、concurrency、bounds 以及 unmet/unknown 预检
摘要；冻结的 `opi-eval-run-plan/2` 文件以 digest 绑定内嵌的实验源、
material 身份、corpus 条目与精确 trials。`[execution]` bounds 全部显
式：`concurrency`（默认 1，为上限；hermetic trial 按其并行分波执行，
native trial 保持串行）、`max_trials`（展开超出即拒绝）、
`agent_timeout_seconds`/`verifier_timeout_seconds`（按收窄合并）、
`max_output_tokens`（仅当与 pinned model control 同值时可执行）。
`max_total_tokens` 与 `max_cost` 在本构建中没有 enforcement 路径，
声明即以该原因拒绝（绝不静默忽略）。`run --plan` 重新校验 plan
digest、material 字节、实验身份与 corpus 展开后，精确执行冻结的
trials，stderr 输出 stage/settlement 进度并保存结构化结果；单主体
（pi-only 或 opi-only）实验无需人工 comparison edge。
`run --config C` 保持同一 resolve/freeze 路径——`[selection]` 文档在
运行时按同一枚举 corpus 展开。

### 0c. 离线校验 seal 并重建报告

```sh
cargo run -p opi-eval -- verify --root "$DEMO_DIR/run"
cargo run -p opi-eval -- report --root "$DEMO_DIR/run" --out "$DEMO_DIR/report.json"
```


三个互不相同的离线操作，均不启动 Agent 或 provider：
`verify --root R` 按原始 seal 逐个复核 sealed trial bundle（退出码
0 已验证 / 1 检出变更 / 2 请求被拒；不修复、不改写）；
`report --root R --out O` 重建字节稳定的规范化报告，其分数只来自已验证的
sealed 证据；`regrade --root R --native-material M --out D` 则是下文
单独描述的真正原生重评——字节校验绝不以 regrade 之名运行。
每次 run 在首个效果之前写入 run 级 `run-control.json`
（`opi-eval-run-control/2`：实验身份、manifest digest、selection、
corpus 规模、精确的 planned trials、完整的已解析实验契约与被接纳的
integrity 记录），因此即使没有任何 trial 封存，离线报告也能保留未启动的
trial：被中断或丢失 bundle 的 run 仍会报告其声明的 edge/control 配对、
原生 benchmark 身份、排除项与零 valid 分母，绝不伪造分数或跨协议配对。
sealed 控制字节会与冻结契约交叉校验；漂移会拒绝发布。遗留的 `/1` root
保持其无冻结拓扑的只读语义。
报告的结果分类消费逐 trial settlement 记录，绝不从 stderr 字符或进程
退出码推断。未封存记录必须与实验及 planned trial 的绑定一致；它们仅保留
attempted/invalid 或效果未知的覆盖信息，不提供已验证状态、原生事实或分数。
因此即使所有尝试都未封存，缺失的 seal 也保持可见。
只有已封存且通过验证的 graded trial（包括有效的原生零分与
benchmark 定义的 Agent 超时）构成指标分母，按 benchmark 各自给出
指标定义（绝不做跨 benchmark 平均），而 provider/access/身份/基础设施
/评分无效类失败保持 attempted-but-invalid，效果未知的 trial 保持
unexecuted。覆盖计数（`corpus`、`selected`、`planned`、`attempted`、
`valid`、`agent_failure`、`invalid`、`unexecuted`）、紧随
completed/incomplete 的 `full|subset` scope 标签、带缺失侧与排除
原因的同任务/同重复 comparison pair、重复控制对
（`consistent|divergent|incomplete` 并给出缺失组）以及未知遥测原因
全部保持可见：协议组边缘（同一产品的两个主体）绝不配对，单主体
报告无需人工 comparison edge。相同输入重复重建得到相同字节。被篡改的
sealed 证据和相互矛盾的控制记录阻断发布；未封存尝试保持未经验证，且不进入
指标分母。原生完成诊断识别 verified native grade，每个原生 headline 都指向
实际提供 reward 的那份已封存 trial result。

### 1. 校验

```sh
cargo run -p opi-eval -- validate \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml
```

这一步在产生任何进程副作用前冻结实验身份。命令以 0 退出后再继续。

### 2. 运行

```sh
cargo run -p opi-eval -- run \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml \
  --root "$RUN_ROOT" \
  --fixtures crates/opi-eval/tests/fixtures
```

`run` 会组装两次已声明的 trial，在进程产生副作用前记录 durable intent，然后依次
完成 trial settlement 并 seal 证据。stdout 只输出一个
`opi-eval-run-report/1` JSON 对象。退出码为 0 且包含
`"outcome":"completed"`，表示所有已声明 pair 均以可比较状态落定。

run root 必须是全新的：请使用不存在或为空、且不含以往 durable run 的路径。命令
会写入 `run-report.json`，并在 `trials/<trial-id>/` 下为每次 trial 写入 receipt
和 bundle。

### 3. 验证 sealed bundle，或离线原生重评

```sh
cargo run -p opi-eval -- verify --root "$RUN_ROOT"
cargo run -p opi-eval -- regrade \
  --root "$RUN_ROOT" \
  --native-material "$MATERIAL" \
  --out "$REGRADE_OUT"
```

`verify` 会读取每个 `trials/<trial-id>/bundle`，重新计算其身份，并检查所有受覆盖
artifact。它不会启动 Agent 或 Provider，不会修复 bundle、为变更后的字节重新
计算摘要，也不会修改 run root。

退出码为 0 且包含 `"outcome":"verified"`，表示所有 sealed bundle 仍与各自
manifest 一致。发现修改或缺失 seal 时，命令以 1 退出，并在 JSON failure 列表中
保留问题。

`regrade` 执行独立的原生重评（`P19-AC-009`）：先验证源 bundle，再检查每个 trial
保留的 Agent artifact 与 sealed verifier-input 摘要一致。它在 `--out` 下创建全新
的 trial 专属环境，通过显式 `uv run --locked --offline python -m <module>`
参数调用私有入口，直接使用固定版本的 Task、ArtifactHandler 和 Verifier API。
输出为 `opi-eval-native-regrade/2`；每个成功 trial 写入
`opi-eval-regrade-record/2`，绑定源 bundle、原始结果、恢复输入、任务包、runner、
模块、镜像、构建上下文与新原生结果摘要，原始分数和证据保持不变。

重评不启动 Agent、不解析凭据、不联系 Provider，也不构建或拉取镜像。
仅恢复已声明、保留且绑定摘要的输入；缺失共享环境最终状态时，不能用原始任务状态
替代。缓存材料缺失、输入被修改、任务或补丁基线漂移，以及无法离线满足的 verifier
网络要求均会被拒绝。取消和有限进程边界会先完成专属资源清理；清理无法确认时，
禁止成功结算和后续派发。退出码 0（`"outcome":"regraded"`）表示全部原生 trial
重评成功，1 表示逐 trial 类型化拒绝，2 表示请求被拒绝（没有原生 trial、材料不可读、
输出目录已存在，或输出在规范 run root 内部——包括经缺失父目录链或符号链接别名——
在任何 mkdir、复制或 verifier 效果之前即被拒绝，源字节保持不变）；已有输出目录
不会被覆盖。

### 4. 生成报告

```sh
cargo run -p opi-eval -- report \
  --root "$RUN_ROOT" \
  --out "$REPORT_PATH"
```

`report` 会先重新验证 sealed input，再渲染一个
`opi-eval-normalized-report/2` JSON 报告（`opi-eval-reporter/2`）。报告会输出到 stdout；如果传入
`--out`，同一份规范字节还会写入指定路径。

二元任务结果仍为 0 或 1：TB2.1 表示原生任务成功，TB3 表示完成声明产物收集后的
任务成功，DeepSWE 表示 committed patch 验证后的任务解决结果。每个指标标明
benchmark revision，仅以有效已评分 trial 为分母；零分和 benchmark 定义的
Agent 超时仍计入该分母。

每个 trial 的 `native_grader.test_case_fraction` 单独呈现原生 `passed / total`，
同时记录分子、分母及原始结果摘要。部分测试通过可以与任务零分并存。缺失计数、
零总数和无效计数均保留明确的 unknown 原因。`native_fields` 按原名保留其他
reward 字段，不推测缩写含义。`observation_coverage` 分别统计可计入的有效 trial、
已测得比例和未知项。无效 trial、corpus/subset 覆盖、计划/尝试/未执行 trial 和
配对覆盖保持独立；不跨 trial 或 benchmark family 合并测试比例。离线重建读取
现有 sealed bundle，不重写源证据；缺失的原生观测继续保持 unknown。

退出码为 0 且包含 `"outcome":"published"`，表示报告发布成功。`REPORT_PATH`
必须位于 `RUN_ROOT` 外部且尚不存在；报告输出绝不会覆盖 sealed input 或以前的
报告。

## 运行单个 conformance case

`conformance` 通过共享执行 driver 运行一个已注册的 Agent 或 benchmark adapter
case。下面的 fixture 示例仅面向 Unix：

```sh
CONFORMANCE_BASE="$(mktemp -d)"
CONFORMANCE_ROOT="$CONFORMANCE_BASE/run"

cargo run -p opi-eval -- conformance \
  --suite agent \
  --adapter opi \
  --case completed \
  --root "$CONFORMANCE_ROOT" \
  --fixtures crates/opi-eval/tests/fixtures \
  --provider crates/opi-eval/scripts/scripted-provider.py
```

命令输出一个 `opi-eval-conformance-report/1` JSON 对象。退出码为 0 且
`"met":true`，表示所选 adapter 满足该 case 的固定预期。退出码 1 表示 case 已
落定，但没有满足预期；退出码 2 表示选择或命令请求被拒绝。

支持的 suite 为 `agent` 和 `benchmark`。支持的 adapter 为 `opi`、`pi`、
`terminal-bench-2.1`、`terminal-bench-3.0` 和 `deepswe`。case id 由固定的
conformance matrix 定义；不受支持的 suite、adapter 或 case 会 fail-closed。

## Hermetic 与 native 模式

| 模式 | 输入与行为 | 能够证明什么 |
|------|------------|--------------|
| Hermetic fixture 模式 | 默认模式。使用有界的确定性 helper 和仓库内固定 fixture。 | 能证明 adapter、lifecycle、sealing、reporting 和失败路径的 conformance；不能证明真实产品或真实 Provider 的 fidelity。 |
| Native-material 模式 | 给 `validate`、`run` 或 `conformance` 添加 `--native-material <MANIFEST>`。manifest（schema `opi-eval-native-material/7`）会解析精确的 Agent executable、固定的共享 capture helper（musl 构建的 Companion，其隐藏 `__native-capture` 模式在每条流的第一个捕获落点过滤内容）、在任务环境内运行 Agent 的固定 integration module（按 runner 家族各一份：Terminal-Bench 路线用 Harbor，DeepSWE 用 Pier）、task package、verifier/oracle 入口和 scripted-provider endpoint。 | 对 `run`：每个 trial 由一个显式选择的 upstream job 在被评分的任务环境内运行所选 Agent，并对同一个 trial 评分；导入的结果必须先绑定到所选 Agent、task 与 job 身份，之后才能 seal。Terminal-Bench 3.0 走其声明的独立 verifier 交接：解析并绑定任务自身 `task.toml` 的生命周期（不支持的形态——多步任务、禁用 verifier、Harbor 路线上的 verifier collect 钩子、非 Linux 任务——在任何 verifier 启动之前拒绝）；对独立 verifier 任务，只有声明过的 artifact 进入全新 verifier——未声明、缺失或被替换的收集一律拒绝，无关的 canary 文件留在 verifier 之外。DeepSWE 走其声明的已提交补丁交接：Pier 任务的 collect 钩子提取超出任务 base revision 的已提交补丁，无网络的全新 pristine verifier 只对这份补丁评分（未提交的工作绝不可能被替换进来）；被绑定的 job 必须记录任务声明的生命周期以及 base revision 与环境镜像，nop 的空补丁照样收集并评零分，未声明的收集一律拒绝。密封 bundle 记录 verifier 输入身份（`native/verifier-input.json`：环境模式、DeepSWE 的 base revision 与镜像，加上每个声明 artifact 及其被收集字节的精确摘要）。`conformance` 只允许已注册的 native case 子集（含三个家族显式选择的 `oracle-witness`/`nop-witness` artifact witness）。 |

Agent 可携带 `runtime_package`，其中包含绝对 `path`、排序文件清单的
`manifest_sha256`，以及绑定到固定 executable 的相对 `entrypoint`。
native handoff schema 为 `opi-eval-native-handoff/4`；Harbor 和 Pier 在使用
选定 provider/model 启动前，校验并上传完整运行包。资源漂移、链接和入口路径
穿越均拒绝。对于本地准备，`cache/agents/<product>` 可以是 executable，
也可以是包含 `<product>` executable 及其资源的目录。

本地 material 还通过 `runner_project` 绑定静态 runner 提交、锁文件、已跟踪源码
字节以及已安装的 runner 可执行文件：固定的 `.venv/bin/python` 解释器与
`.venv/bin/<runner>` 控制台入口都携带精确字节身份。加载时重新校验这些身份、
已安装字节及绝对 `UV_PROJECT`、`UV_PROJECT_ENVIRONMENT` 和 `UV_PYTHON` 选择；
已安装字节缺失或被更改时，在任何 validate/plan/run 效果或冻结计划接受之前
拒绝使用该 material。material/5 及更早版本需要重新生成。准备流程对显式
oracle 诊断复用同一固定 uv executable。Pier 从 handoff 的选定
`inference_endpoint` 推导推理域名白名单，并只向 Agent 进程投影上游代理环境；
独立 verifier 不接收白名单或代理凭据。
Opi 任务进程使用隔离且不可用的会话总线地址，使固定客户端在无桌面容器中
能够通过既有的环境凭据回退路径使用所投影的连接。

上游 oracle 是显式选择的 conformance 诊断（`conformance --case oracle-witness` /
`--case nop-witness`），绝不是被 `run` 静默执行的前置条件。诊断作业选择固定
runner 注册的 `oracle` 或 `nop` 名称，不携带自定义 import path。结果绑定要求
名称精确匹配，并拒绝缺失、替换或双重选择；集成 Agent 保持固定 import 选择。

手工触发的
[external-lock materialization workflow](../../.github/workflows/opi-eval-external-lock-materialization.yml)
遵循同一委托规则：它从 Cargo compiler-artifact 输出构建候选 Companion，
从 checkout 播种声明的 cache，并把诊断性的 Terminal-Bench 2.1 准备与显式
`oracle-witness` conformance 委托给共享本地 CLI；只有两者都成功后才发布
`opi-eval-materialization-receipt/2`（以及 `opi-eval-external-lock/resolved/2`）。
回执把共享来源——Companion、诊断 config、prepare 命令/报告/material
（material/7），以及带有 `met: true`、native reward 来源与
`agent_score: false` 的 conformance 命令/报告——绑定为受限、相对、SHA 绑定的
artifact 成员，并唯一绑定 oracle 原始结果与 CTRF 字节；网络声明以
`pinned-task-policy` 为基准记录任务声明的模式，绝不从宿主设置推断离线强制。
共享 prepare 或 conformance 的拒绝会阻止任何回执发布；保留的 /1 字节只是
只读历史，当前 verifier 会拒绝它们。

请求身份来自 experiment 文档中选定的命名 connection（schema
`opi-eval-experiment/2`）：顶层 `connection` 选择一个 `[[connections]]` id，
条目携带 `provider`、`api`（`anthropic-messages` | `openai-completions` |
`openai-responses`）、`base_url`、`model`，以及互斥的
`credential_env`/`credential_file` 引用之一。不存在 provider、协议、模型或
计费产品的回退：credential 只在受监督的 Agent 启动内解析；缺失的 credential
与 required-but-unsupported 的 `[model_controls]` 控件在任何模型效果之前拒绝；
显式 native `reasoning` 支持 `off`、`minimal`、`low`、`medium`、`high`，共享预算
分别为 0、1024、2048、8192、16384 tokens。启用的深度要求数值响应上限至少比预算
多 1024 tokens。Opi 启动配置只选择请求的深度；pi 通过 `--thinking` 接收该值。
required reasoning 在 `validate`、`plan`、`run` 准入前要求真实客户端首轮及工具
续轮证据。不支持的必需设置拒绝；可选的不支持设置保留请求，并保持不可比较。
私有思考文本不予保留。远程 base URL 必须显式声明 `[execution].mode = "live"`（否则只允许回环地址）。
`[execution.caps]` 中请求的货币/用量上限会被拒绝，因为 native 路由无法强制执行。
native Agent 时间采用已提供的 `time_limit_seconds` 与 `agent_timeout_seconds`
中的较小值；`verifier_timeout_seconds` 限制 verifier 阶段。任务原有的更窄
限制仍然生效，每个 native job 还受固定 adapter profile 的外层 deadline 约束。
显式的正数 `max_output_bytes` 会在解析时被拒绝：请求的字节上限在本构建中没有
强制执行路径；省略该值时保留固定的运行时捕获保护（profile 的外层 job
stdout/stderr 上限与 native client 每条流独立的 64 MiB 工件上限）。
两个固定客户端（Opi 与 pi）由同一个 connection 对全部三种协议投影，本地
[scripted provider](scripts/scripted-provider.py) 提供确定性工具循环以及按模型
选择的 `auth-fail`、`rate-limit`、`retry`、`protocol-error` 脚本，通过
`native_protocol_client_matrix`（ignored，需本地客户端物料）验证。每个 adapter
只依据客户端自身证据分类类型化的终端 provider 原因（认证失败、限流耗尽、协议
错误）——包括进程退出码为 0 时的 JSON 错误——绝不臆测证据未知的 provider 原因。

native 准入按每个选定客户端检查 required `temperature` 与 `top_p`。
Opi 无法表达这两个控制；pi 仅在 OpenAI 协议上支持它们。不受支持的 required
数值会阻断验证、计划与执行；可选的请求数值则保留为不可比较。omitted 与 unknown
标记保持区别，均不能证明数值已被强制执行。

原生 Opi 配置省略 `[defaults].max_iterations`。当前仓库的 Opi 核心没有内置
轮数上限，产品只执行显式配置的预算；原生任务/adapter 超时与取消仍然生效。
旧 executable 可能仍有 50 轮默认值，因此需在预览中同时披露精确客户端的实测
行为与执行时限；机器 plan 尚无专门的 Agent 轮数预算字段。使用离线长循环测试
验证所选 executable 与生产配置投影：

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=/absolute/clients.json cargo test -p opi-eval --test native_driver \
  native_client_turn_budgets -- --ignored --exact --nocapture
```

提供包含 `opi` 和 `pi` executable 绝对路径的 JSON 对象。回环测试在指定数值输出控制时，
分别验证 Opi 默认配置、投影配置与固定 pi 客户端完成 51 轮工具调用及最终回答，
并确认 Opi 显式较小预算会在额外请求前停止。自定义 Opi 模型省略输出能力
时仍在 provider 请求前拒绝；配置投影不会编造该能力。检查失败会阻断该行为声明
对应的准备工作，不能静默替换所选 executable。投影字节变化需要新建控制证据、native material 和 plan
身份；旧候选与封存结果仍作为历史证据保留。

数值输出上限同时投影到模型能力字段与 Opi 的显式请求设置
`[defaults].max_output_tokens`。这**不代表**原版 Opi 0.8.2 支持该设置；
它可能静默忽略新字段。当 `max_output_tokens` 为 required 控制，或声明了
execution 输出 token 边界时，原生 `validate`、`plan` 和 `run` 要求精确客户端
的回环证据。没有证据的可选数值上限保持为不支持的比较控制。版本字符串、
声明的能力或较低的实际 token 用量，均不能替代观察到的请求上限。

使用选定实验与本地提供的 executable 生成范围明确的证据。探针仅把 endpoint
和 credential 替换为回环监听器与固定的合成认证值；绝不解析实验凭据，也不
联系实验 provider。例如 `clients.json`：

```json
{
  "opi": "/absolute/opi",
  "pi": "/absolute/pi-package/pi",
  "pi_package": "/absolute/pi-package",
  "experiment": "/absolute/experiment.toml"
}
```

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=/absolute/clients.json \
OPI_EVAL_OUTPUT_CONTROL_OUT=/absolute/new-proof-directory \
cargo test -p opi-eval --test native_driver \
  native_output_control_client_matrix -- --ignored --exact
```

输出目录必须尚不存在。原生 pi executable 按原路径启动，也保留已有的
`node`/`pi_bundle` 输入形式。若 native material 携带客户端运行包，必须提供
相应的 `opi_package` 或 `pi_package`。此测试探针覆盖实验选定的 API 和数值
上限，reasoning 为 omitted 或 unknown；显式思考设置使用下文的 native control
探针。确定性工具循环中的两个请求都必须携带精确上限；显式思考还需携带要求的
reasoning wire 字段。

启动监听器前，探针将实际执行的文件绑定到所提供 package 内的普通相对
entrypoint，拒绝路径穿越、链接和无关 package，即使 executable 字节相同。
证据同时保留 entrypoint 和 package 摘要。`node`/`pi_bundle` 形式要求
`pi_package` 同时包含解释器与 bundle；仅解释器摘要不能标识 pi 代码。

每个 `<product>/output-control.json` 固定 executable/package 身份、
provider/API/model、冻结控制、投影 config 和独立监听器日志。保留完整证据
目录，并在冻结 plan 前，在**新** native material 中设置
`agents.<product>.output_control_evidence` 为
`{"path":"/absolute/proof/<product>/output-control.json","sha256":"<file SHA-256>"}`。
该可选字段本身并不授予能力：准入校验固定字节、重新生成生产投影，并检查
两个实际 wire 上限及已投影的 reasoning 控制。必需证据缺失、被替换、不支持或
不一致时，在派发前拒绝。
不会改写已有 plan、run 或 seal。每个已准入 trial 将自包含快照封存为
`control/output-control.json`。

对于指定显式思考深度的受测对象，在冻结 native material 或 plan 前使用独立离线
探针：

```sh
python3 crates/opi-eval/scripts/native-control-proof.py \
  --companion /absolute/opi-eval --config /absolute/experiment.toml \
  --material /absolute/unfrozen-material.json --out /absolute/new-control-proof
```

输出目录必须全新。helper 仅用合成认证、隔离 home 和 IPv4 回环端点运行精确的
选定客户端包。它保留投影配置、白名单请求控制、命令退出及准入结果，丢弃客户端
stdout/stderr。两个请求都须携带请求的输出上限和支持的思考投影。只有 Companion
成功准入后才向未冻结 material 添加证据 pin；失败时其原字节保持不变。
现有 material 与 proof schema 身份不变。不要用准备 helper 改写冻结 plan 或密封
material。

CLI 只消费已经解析的 native-material manifest；它不会临时拼凑或静默替换缺失的
native input。Native materialization 由公共本地 `prepare` 路径拥有，并附带显式的
exact-client 输出控制证明；仓库的
[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml)
只是基于同一共享准备的回归调用方。
其 materialization 阶段把所有 task/image/profile/runner-material 决策委托给同一
公共本地 `prepare` 路径：已获取的外部 checkout、构建好的 agent 与仓库 pin 被送入
已准入的固定 cache，每个 benchmark 家族一次共享准备生成对应的 per-family
prepared material，而每个 CI 消费方（conformance 重跑、oracle 预检、trial）都通过
已发布的 per-family 路由表解析自己的 material——CI 不保留独立的变换或准入规则，
任何共享 prepare 拒绝都会在任何 material 发布之前停止 materialization。该阶段在
固定证据和冻结 config 前，通过同一回环探针验证 required 的 4096 token 上限。Opi
使用已定位的构建 executable；pi 通过相对路径 launcher 携带已构建的 Node 解释器、
package 元数据、发行代码与资源。探针绑定的 package 和 entrypoint 与 trial
material 声明一致。证据失败或不完整会停止 materialization；artifact verifier
仍要求 comparable pair。该阶段可能获取锁定的 Cargo 测试依赖，但客户端仅接收
隔离的合成认证值与回环 URL。真实 Provider 行为评测使用显式调用的
[opi-eval workflow](../../.agents/skills/opi-eval/SKILL.md)；其预算和证据管理与本文
hermetic CLI 示例相互独立。

## 命令参考

| 命令 | 必需输入 | 重要选项 | 输出与退出行为 |
|------|----------|----------|----------------|
| `validate` | `--config PATH` | `--native-material PATH` 会加入 native integrity identity。 | 一行摘要；解析成功为 0，无效为 1。 |
| `run` | `--config PATH` 与 `--plan PATH` 二选一，加上 `--root PATH`；hermetic 执行需 `--fixtures PATH`。 | `--recover`、`--replacement-for TRIAL`、`--canaries PATH`、`--native-material PATH`（必须匹配计划冻结的摘要）；`--behavior` 用于选择 hermetic 故障 fixture。 | 单行 JSON；完成为 0，已落定但未成功为 1，请求被拒绝为 2。 |
| `verify` | `--root PATH` | 无。 | 单行 JSON；验证成功为 0，发现变更或未 seal 为 1，命令请求被拒绝为 2。 |
| `regrade` | `--root PATH --native-material PATH --out PATH` | 无。 | 单行 JSON；全部原生 trial 完成重评为 0，逐 trial 类型化拒绝为 1，命令请求被拒绝为 2。 |
| `report` | `--root PATH` | `--out PATH`、`--canaries PATH`。 | 单行 JSON；发布成功为 0，被阻止或未通过验证为 1，命令请求被拒绝为 2。 |
| `conformance` | `--suite ID --adapter ID --case ID --root PATH --fixtures PATH --provider PATH` | `--native-material PATH`。 | 单行 JSON；满足预期为 0，未满足预期为 1，不受支持或被拒绝的请求为 2。 |

使用 `cargo run -p opi-eval -- <command> --help` 查看当前完整参数。CLI 及其格式仍是
不稳定的 0.x 契约。

## 输出与故障恢复

材料获取整次调用限时 3,600 秒，每个输出流最多保留 1 MiB。
`acquisition.log` 记录源码材料获取，`image-acquisition/acquisition.log` 记录所选
镜像获取；两者均记录退出、超时/取消、截断及自有资源清理状态。
SIGINT/SIGTERM 取消自有进程树；失败或未验证落定的调用不能发布材料。
仅清理本次调用自有的临时 staging，保留已验证的缓存条目。

完成后的 fixture run 具有以下持久结构；sealed bundle 外还可能存在其他 staging
文件：

```text
$RUN_ROOT/
├── run-report.json
└── trials/<trial-id>/
    ├── receipt.json
    └── bundle/
        ├── intent.json
        ├── manifest.json
        └── artifacts/
```

sealed bundle 一旦生成，就应视为不可变。修改任何受覆盖字节都会导致 `verify`、`regrade` 和
`report` 验证失败；这两个命令都不会修复证据或静默重算摘要。`--canaries PATH`
指定的文件每行声明一个 canary；如果 exportable content 中出现其中任意内容，
sealing 或报告发布会被阻止。native 运行中，已解析的所选连接凭据本身也会受到同样保护，
即使未指定 `--canaries`：它在任何已暂存 native 导出中的精确回显都会阻止该 trial 的
sealing，取消整个 run 以阻止后续 trial 启动，且绝不会把该密钥序列化进任何守卫列表或
artifact。native canary 违约还会取消整个 run，阻止后续 trial
启动；已观察到的证据保持私有、未密封，未派发 trial 仍保留在计划覆盖范围内。
离线比较要求已请求的 native 输出上限具备密封的 output-control 快照。缺少证明时
仍标为不受支持，旧 bundle 也如此；可变的 run report 不能提升可比性或补充缺失证明。

| 现象 | 处理方式 |
|------|----------|
| 配置被拒绝 | 先运行 `validate`，再根据 stderr 中的类型化诊断处理。系统没有隐式控制默认值或回退。 |
| run root 已包含 durable state | 不要删除或编辑 trial 证据。使用 `run --recover` 对 durable state 分类；若 trial 崩溃，则使用显式 replacement 流程。 |
| native material 被拒绝 | 通过公共本地 `prepare` 路径从已准入的固定 cache 重新生成，并刷新 exact-client 输出控制证明；不要替换为未固定的 executable 或 task package。 |
| `verify`/`regrade` 报告 `mutation-detected` 或类型化拒绝 | 保留 bundle 与保留 artifacts 以便诊断，不要修复、重写或重算摘要。 |
| `report` 被阻止 | 检查 bundle verification 与 canary failure；修正输入来源后，使用新的、位于 run root 外的 `--out` 路径。 |

`run --recover` 只读输出冻结计划中全部 trial 的分类，包括尚未创建目录的条目。
它保留原报告、control 与 bundle 的每个字节，展示已保留的结算证据，并拒绝
不一致的身份，不会重新标记已有证据。

`run --replacement-for <TRIAL_ID>` 每次调用使用同一个新 attempt token，为崩溃
trial 所在的整个对比组创建新身份。替换前会校验原计划并展开 selection；新实验
保留冻结范围、控制项及无关分组。替换实际重新执行的每一个 trial——包括保留的
无关分组——都以全新 trial ID 运行；只有目标对比组携带 attempt 后缀，因此无关
分组保留其冻结的分组身份，同时整个声明范围仍全部派发。
必须使用新输出目录，原计划与运行证据保持不变。

保留了被取代尝试的耐久 root 无法直接发布报告——对整个混合 root，`report` 会拒绝
控制漂移、未声明 trial 和 run-control 漂移，这一拒绝是有意为之。要在不改动原始
证据的前提下重构恰好一次 run-control 声明执行的报告视图，请按每次执行只读复制：

1. 保持原始混合 root 不变；绝不删除或重新标记其中被取代的尝试。
2. 为恰好一次 `run-control.json` 声明的执行创建新的 staging root，逐字节复制该
   `run-control.json` 及其 `planned_trials` 声明的恰好那些 trial 目录。
3. 对 staging 副本运行 `verify` 和 `report`，输出写入 staging root 与原始 root 之外。
4. 原始 root 中保留全部被取代的尝试，并在记录中标明 staging 副本身份；staging
   视图只是报告重构，绝不替代被保留的证据。

该 staging 流程绝不削弱校验：staging root 仍需逐字节验证通过，而整个混合 root
依旧被拒绝。

### 本地操作员验收与离线校验器

校验器要求封存的 `execution.mode = "live"`，以及首个连接
`zhipu` / `anthropic-messages` / `https://open.bigmodel.cn/api/anthropic` /
`glm-5.3-flash`。封存的终止状态、客户端流、provider 响应和工具事件必须与该声明
及保留的构建/控制证据一致。loopback 输出控制探针仅证明客户端一致性。

真实验收是另一套明确授权的本地操作员运行，发生在 WSL/Linux 上、
独立于任何开发 harness：操作员选定并冻结任务、Agent 构建、材料
摘要、模型连接控制、并发与用量/时间预算，显式接通第一个真实
凭据，并运行同一个公开 CLI。默认测试与 CI 绝不因环境中恰好存在
凭据而转为 live，任何确定性路径都不联系 provider。发生预算、
权限、来源或隐私违约时，停止受影响的 trial、取消自有执行，回滚
方式是禁用受影响的本地执行路径或恢复先前已校验的 Companion
构建——绝不重新标注旧证据。

操作员交付一个脱敏的证据根目录：包含六个可归属 trial（覆盖三个
benchmark 家族）的 manifest、冻结的 controls、source/build/
material/bundle 摘要、保留的命令退出码与 provider/tool/native
工件、Opi 专属的 `opi-artifact-audit` 范围捕获视图（与三个原始
Opi native 捕获逐字节绑定且恰好穷尽它们），以及 pi/Harbor/Pier
来源记录的缓存上游仓库身份清单。保留清单必须为每个 trial 家族
携带恰好一条 material 记录且路径唯一，并包含由各保留 material 的
`static_lock` pin 定位的已准入 static-lock 字节；pi、Harbor 与
Pier 的身份从这些已准入 lock 字节推导（来自 lock 的 subject 与
固定的 runner tools 的仓库限定提交身份），绝不依赖多个提供的
映射彼此印证，缓存清单仅作佐证。任何人都可以离线校验该证据：

```sh
python3 crates/opi-eval/scripts/verify-local-acceptance.py \
  --root <redacted-evidence-root> \
  --out <verification-report.json> \
  --companion <built opi-eval executable>
```

校验器绝不启动 Agent、读取凭据或联系 provider。它拒绝被替换、
oracle、缺失或仅 fixture 的证据，拒绝把空的一般审计成功当作验收，
要求每个封存的 control/integrity upstream identity 绑定其家族的
已准入 lock 摘要，并依据从已准入 lock 字节推导出的身份校验上游
提交（与随证据提供的缓存清单交叉核对，绝不在本 checkout 中
解析提交），且只针对随证据提供的家族 run 根调用构建出的
Companion `verify` 与 `report`。退出码 0 表示接受证据；退出码 2
输出一个类型化的拒绝原因。

验收校验器要求 Python 3.11 或更高版本。manifest 的
`controls.connection_controls` 包含 `connection`（选定的 `provider`、
`api`、`base_url`、`model`）与 `model_controls`（冻结的 resolved 对象）。
校验器将它们和 `execution_concurrency` 与每个 sealed experiment 比较，
再把封存的输出控制快照与 trial executable 身份和保留的 material 核对。
投影中的数值上限和两个独立观察到的请求上限必须一致；仅有字典形状的声明
或 seal 不足以通过。验收时绝不沿探针记录中的原始主机路径读取文件。

### 有界取消

原生 Harbor/Pier job 为 Agent 和 verifier 执行选择固定 integration 中的受监督
Docker 环境。每条命令保留自身的进程组身份；阶段取消必须等到观察到命令退出后
才能结算。无法确认终止时，停止后续派发并阻断封存。持久化 trial 配置必须保留
该环境选择和有效的阶段时限。销毁保留共享镜像与相邻资源。integration 字节变化
要求新的 material 和 plan 身份；已有源 bundle 不会被改写。

使用准备好的 material/6 显式运行无模型阶段见证：

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=/absolute/material.json \
cargo test -p opi-eval --locked --test native_driver \
  remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
```

计时派发前准备不可变任务/helper 镜像及独立 verifier 构建上下文。三个家族各需
覆盖九种受控场景和两个主体，先运行全部正向控制。Harbor 的正向 verifier 上限
为 3 秒，超时控制为 1 秒，对应延迟 4 秒的命令。Pier 对整个 verifier 生命周期
计时并重试一次：所有场景显式使用 30 秒 verifier 上限、60 秒任务 verifier
时限，verifier 超时场景的命令延迟 60 秒。Agent 上限保持 1 秒，另有任务自身
0.5 秒的更严格控制。每条被取消命令都须观察到退出；Pier verifier 超时须保留
两个不同命令身份。原生阶段时间戳、命令耗时、未达到延迟完成点和自有资源清理
分别保留证据。Pier 命令结算最多额外允许 5 秒；两主体外层观察界限分别为
Harbor 270 秒、Pier 510 秒。这些 fixture 证明取消契约，不代表真实验收。
主机墙钟回拨保留为诊断观察，耗时检查使用单调时钟。

固定的 Terminal-Bench 3.0 profile 为整个 Harbor 作业分配 18,000 秒：任务 Agent
14,400 秒、评分 900 秒、两次环境构建各 600 秒，其余准备、收集和清理预留 1,500 秒。
外层期限与任务原生的 Agent、评分器期限各自独立。修改 profile 需要新的材料与计划身份，
已有冻结计划继续保留原预算。所选原生作业达到外层期限时，归类为基础设施边界的
`native-job-timeout`，不计分；原生评分器超时仍归评分边界，两者在 settlement 与 receipt
分类中均保留 `timed_out: true`。基准允许的 Agent 超时继续保留其原生分数。

整个 run 由一个共享取消令牌管辖。向运行中的 `run` 进程发送 SIGINT 或
SIGTERM 会停止新的受影响派发、只取消在飞 trial 拥有的进程树、移除记录在
各 trial 私有 Compose 工作目录所绑定的 job 容器以及临时
凭据材料，并且仍然完成结算并输出中断报告：被取消与从未派发的 trial
保持为显式缺失覆盖，绝不伪造分数；持久的
`integration/owned-resources.json` 证据与 receipt 中的 cleanup token
精确记录清理了什么。成功的取消绝不留下孤儿执行；被 ignore 的
`native_owned_resource_cancellation` 见证在本地验证真实的 Docker 移除。
无法强制执行的必需边界在任何效果之前拒绝，且绝不以未知价格承诺
货币上限。

完整的原生 Agent stdout/stderr 输出流在现有的每个 artifact 64 MiB 上限内
逐字节封存。超出上限的尝试为 invalid 且不计分；私有原始日志仍可用于诊断，
绝不将截断前缀作为完整输出流封存。terminal-only 捕获仍省略中间输出流。

在投射配置或凭据之前，两个原生集成都在任务环境内用精确暂存的可执行文件
（包括 pi 的打包运行时）执行 `--version`，由监督机制限制为 30 秒。
私有的 `opi-eval-native-agent-terminal/3` 记录将预检、暂存 capture helper 的任务内
自检以及首个捕获落点的 capture receipt 摘要绑定到逻辑 trial、可执行文件及包、
命令和独立捕获摘要。失败或未知的预检禁止任务派发，保留
attempted、invalid 且不计分；预检退出码绝不作为任务退出码。通过预检仅证明
有边界的运行时兼容性；后续任务派发与观察到的退出仍须独立证据，有效的任务
零分、非零退出及超时保持原有语义。terminal-only 保留证明及允许的诊断。
新的派发必须使用 `/3` 且 capture 状态为 `sanitized`；旧 `/2` 字节仅供已封存
证据的离线验证、报告及仅验证器重评分读取，它们不会补造预检证明，也不会
重新运行 Agent。

私有 thinking 绝不触达第一个捕获落点：固定的 musl capture helper（随所选
Agent 一起传输，并在投射任何凭据之前完成自检）通过有边界管道派发 Agent，
在两种捕获策略下都只持久化已验证的隐私投影字节。thinking 文本、声明的私有
签名与 `redacted_thinking` 数据在 assistant 消息重复出现的所有位置（消息、
turn、AgentEnd 快照与流式 partial）被移除，私有 thinking 增量/结束内容被清空
但保留占位符与 `content_index`/`contentIndex`；模型/API/provider 身份、用量、
停止/重试/结算以及非私有工具身份保持真实，工具自有 JSON 中名为 `thinking`
的属性绝不被改写。未知的信封/事件/块字段或类型、重复 JSON 键、无效
UTF-8/JSON、不完整记录以及超过现有 64 MiB 上限的流在该记录写入之前拒绝，
且不产生 native 分数。未变化的记录保留其原始字节。Agent stderr 只接受空输出
与显式验证的 provider 诊断词汇表；任何不透明内容只保留静态计数/摘要元数据并
排除分数。helper 的 `opi-eval-native-capture/1` receipt 绑定被保留字节的摘要，
宿主在任何 terminal 完成、client message、封存或有界部分复制
（`opi-eval-partial-capture/2`）使用这些字节之前重新验证；不受信的捕获绝不
进入可导出的封存证据。

在固定的 Docker 挂载路径中，Agent 输出流和 Opi trace 直接写入该 trial 的
日志挂载，因此外层超时不依赖最终下载。移除本次运行拥有的写入进程后，
full-capture 运行保留一份私有诊断副本，限制为每文件 1 MiB、总计 16 MiB、
最多 256 个文件，并限制遍历工作量、明确记录截断计数。仅在 Docker 确认的
精确日志挂载内恢复 root 所有捕获文件的可读性，不放宽权限。符号链接和
硬链接文件不提供捕获内容。这些部分证据不提供终止事件、原生分数或 seal，
原始超时及清理失败仍然可见。

上游正常退出后也会执行清理，因为退出码为零的 job 仍可能包含失败的初始化
或销毁操作。移除前必须确认 Docker 标签、完整容器 ID 与记录的私有根目录
一致；无法确认移除完成时，禁止成功封存。`opi-eval-owned-resources/2` 记录
这些身份和结果。结算时移除临时 `handoff.json`、`config.toml` 和 `models.json`，
由 `handoff-identity.json` 保留不含秘密的启动身份。原生结果绑定采用固定
Harbor/Pier 的 `jobs/<job>/<trial>/result.json` 布局，并保留有效零分。

## 开发者指南

| 职责 | 权威实现 |
|------|----------|
| CLI 解析与退出码映射 | [`src/main.rs`](src/main.rs) |
| 实验组装与持久 lifecycle | [`src/runner/`](src/runner) |
| Agent 与 benchmark process adapter | [`src/agent/`](src/agent)、[`src/benchmark/`](src/benchmark) |
| 准入与 sealed evidence | [`src/integrity.rs`](src/integrity.rs)、[`src/bundle/`](src/bundle) |
| 离线字节校验、原生重评与规范化报告 | [`src/regrade.rs`](src/regrade.rs)、[`src/report.rs`](src/report.rs) |
| Hermetic input | [`tests/fixtures/`](tests/fixtures) |
| Assembled run 行为 | [`tests/assembled_run.rs`](tests/assembled_run.rs) |
| Regrade/report lifecycle | [`tests/end_to_end_report.rs`](tests/end_to_end_report.rs) |
| Native material 与 fidelity 检查 | [`scripts/`](scripts)、[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml) |

测试应使用本地 fixture 和确定性 Provider。测试不得调用付费 Provider，也不能因为
环境里存在凭据就自动切换为 live 模式。上文完整 fixture 流程以 Unix end-to-end
测试为验收来源。

## 稳定性与产品边界

`opi-eval` 为 `publish = false`，在任何依赖表中都不依赖 Opi crate，并且仍是未
发布的 `0.x` workspace 成员。其 CLI、schema、磁盘格式和库入口可以被有意调整；
破坏性变更记录在 [`CHANGELOG.md`](../../CHANGELOG.md) 的 Unreleased 部分。旧版
experiment 或 report 格式不隐含兼容层。

没有任何 Opi 产品链接、注册或激活本 crate。它不在 `opi` 中注册 Provider、工具、
package、command、extension、启动 hook 或默认捕获路径。普通 Opi 启动不读取或迁移评测存储。
评测命令只消费显式提供的 profile、operation 或 evidence；已有私有 profile 与封存
originals 不会被改写。

当前 crate 版本：`0.8.2`，继承自 workspace package 版本。

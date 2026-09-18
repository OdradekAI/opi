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
全程不启动任何 model。`prepare` 依据 pinned static lock 校验种子化的
本地缓存（`[storage].cache`），冻结所选 benchmark、subject 可执行文件
与全部被请求的 image 身份，将冻结任务的资源需求与本地观测比对
（未满足或无法观测都会整体拒绝，绝不静默收缩集合），并产出可被
`validate`/`run` 通过 `--native-material` 消费的 local-origin
native-material manifest。local origin 被如实记录，绝不伪造 GitHub
provenance：upload receipt 仍然只是 CI 的 transport claim。缓存缺失时，
若已种子化 pinned materializer 则触发受监督的获取（plan 与被捕获的
日志位于输出根下）；否则失败会点名缺失的身份及其获取方式。完整的
operator 演练见 `examples/local-evaluation/`。

缓存中的 `corpora/<adapter>/tasks` 必须是完整、干净且版本固定的 Git 语料。
prepare 为选中任务生成 `packages/<adapter>/<task-id>` 和绑定 SHA-256 的
`profiles/<adapter>/<task-id>.toml`。material/5 分开记录完整语料的
root/commit/tasks-tree 与所选 `tasks` 映射；每项绑定 canonical ID、原生名称、
profile 和任务包。任务镜像及 Dockerfile 基础镜像必须解析为不可变摘要，并已能在
所选 engine 的缓存中观测到。缺失身份会拒绝并提示获取方式；prepare 和 preview
不会拉取或启动任务镜像。

需求记录分别保留 Agent/verifier 的 CPU、内存、存储、GPU 数量及类型、网络策略、
收集行为和阶段时限，以及固定版本默认值和来源。CPU、内存取自所选 Docker engine；
存储检查覆盖缓存、输出和 engine 存储命名空间。无法观测远程或 Docker Desktop
engine 的存储命名空间时保留 unknown。注册 NVIDIA runtime 不能证明 GPU 容量。
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
每次 run 在首个效果之前写入 run 级 `run-control.json`（实验身份、
manifest digest、selection、corpus 规模、精确的 planned trials），
因此即使没有任何 trial 封存，离线报告也能保留未启动的 trial。
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

### 3. 重新验证 sealed bundle

```sh
cargo run -p opi-eval -- regrade --root "$RUN_ROOT"
```

`regrade` 会读取每个 `trials/<trial-id>/bundle`，重新计算其身份，并检查所有受覆盖
artifact。它不会启动 Agent 或 Provider，不会修复 bundle、为变更后的字节重新
计算摘要，也不会修改 run root。

退出码为 0 且包含 `"outcome":"verified"`，表示所有 sealed bundle 仍与各自
manifest 一致。发现修改或缺失 seal 时，命令以 1 退出，并在 JSON failure 列表中
保留问题。

### 4. 生成报告

```sh
cargo run -p opi-eval -- report \
  --root "$RUN_ROOT" \
  --out "$REPORT_PATH"
```

`report` 会先重新验证 sealed input，再渲染一个
`opi-eval-normalized-report/1` JSON 报告。报告会输出到 stdout；如果传入
`--out`，同一份规范字节还会写入指定路径。

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
| Native-material 模式 | 给 `validate`、`run` 或 `conformance` 添加 `--native-material <MANIFEST>`。manifest（schema `opi-eval-native-material/5`）会解析精确的 Agent executable、在任务环境内运行 Agent 的固定 integration module（按 runner 家族各一份：Terminal-Bench 路线用 Harbor，DeepSWE 用 Pier）、task package、verifier/oracle 入口和 scripted-provider endpoint。 | 对 `run`：每个 trial 由一个显式选择的 upstream job 在被评分的任务环境内运行所选 Agent，并对同一个 trial 评分；导入的结果必须先绑定到所选 Agent、task 与 job 身份，之后才能 seal。Terminal-Bench 3.0 走其声明的独立 verifier 交接：解析并绑定任务自身 `task.toml` 的生命周期（不支持的形态——多步任务、禁用 verifier、Harbor 路线上的 verifier collect 钩子、非 Linux 任务——在任何 verifier 启动之前拒绝）；对独立 verifier 任务，只有声明过的 artifact 进入全新 verifier——未声明、缺失或被替换的收集一律拒绝，无关的 canary 文件留在 verifier 之外。DeepSWE 走其声明的已提交补丁交接：Pier 任务的 collect 钩子提取超出任务 base revision 的已提交补丁，无网络的全新 pristine verifier 只对这份补丁评分（未提交的工作绝不可能被替换进来）；被绑定的 job 必须记录任务声明的生命周期以及 base revision 与环境镜像，nop 的空补丁照样收集并评零分，未声明的收集一律拒绝。密封 bundle 记录 verifier 输入身份（`native/verifier-input.json`：环境模式、DeepSWE 的 base revision 与镜像，加上每个声明 artifact 及其被收集字节的精确摘要）。`conformance` 只允许已注册的 native case 子集（含三个家族显式选择的 `oracle-witness`/`nop-witness` artifact witness）。 |

Agent 可携带 `runtime_package`，其中包含绝对 `path`、排序文件清单的
`manifest_sha256`，以及绑定到固定 executable 的相对 `entrypoint`。
native handoff schema 为 `opi-eval-native-handoff/3`；Harbor 和 Pier 在使用
选定 provider/model 启动前，校验并上传完整运行包。资源漂移、链接和入口路径
穿越均拒绝。对于本地准备，`cache/agents/<product>` 可以是 executable，
也可以是包含 `<product>` executable 及其资源的目录。

本地 material 还通过 `runner_project` 绑定静态 runner 提交、锁文件和已跟踪
源码字节；加载时重新校验这些身份及绝对 `UV_PROJECT`、
`UV_PROJECT_ENVIRONMENT` 和 `UV_PYTHON` 选择。准备流程对显式
oracle 诊断复用同一固定 uv executable。Pier 从 handoff 的选定
`inference_endpoint` 推导推理域名白名单，并只向 Agent 进程投影上游代理环境；
独立 verifier 不接收白名单或代理凭据。
Opi 任务进程使用隔离且不可用的会话总线地址，使固定客户端在无桌面容器中
能够通过既有的环境凭据回退路径使用所投影的连接。

上游 oracle 是显式选择的 conformance 诊断（`conformance --case oracle-witness` /
`--case nop-witness`），绝不是被 `run` 静默执行的前置条件。

请求身份来自 experiment 文档中选定的命名 connection（schema
`opi-eval-experiment/2`）：顶层 `connection` 选择一个 `[[connections]]` id，
条目携带 `provider`、`api`（`anthropic-messages` | `openai-completions` |
`openai-responses`）、`base_url`、`model`，以及互斥的
`credential_env`/`credential_file` 引用之一。不存在 provider、协议、模型或
计费产品的回退：credential 只在受监督的 Agent 启动内解析；缺失的 credential
与 required-but-unsupported 的 `[model_controls]` 控件在任何模型效果之前拒绝；
远程 base URL 必须显式声明 `[execution].mode = "live"`（否则只允许回环地址）。
`[execution.caps]` 中请求的货币/用量上限会被拒绝——native 路由无法强制执行——
而可强制的 `time_limit_seconds` 与 `max_output_bytes` 边界会连同其限制一起记录。
两个固定客户端（Opi 与 pi）由同一个 connection 对全部三种协议投影，本地
[scripted provider](scripts/scripted-provider.py) 提供确定性工具循环以及按模型
选择的 `auth-fail`、`rate-limit`、`retry`、`protocol-error` 脚本，通过
`native_protocol_client_matrix`（ignored，需本地客户端物料）验证。每个 adapter
只依据客户端自身证据分类类型化的终端 provider 原因（认证失败、限流耗尽、协议
错误）——包括进程退出码为 0 时的 JSON 错误——绝不臆测证据未知的 provider 原因。

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
相应的 `opi_package` 或 `pi_package`。此探针只覆盖实验选定的 API 和数值
上限，reasoning 为 omitted 或 unknown；它不证明 reasoning effort。确定性
工具循环中的两个请求都必须携带精确上限。

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
两个实际 wire 上限。必需证据缺失、被替换、不支持或不一致时，在派发前拒绝。
不会改写已有 plan、run 或 seal。每个已准入 trial 将自包含快照封存为
`control/output-control.json`。

CLI 只消费已经解析的 native-material manifest；它不会临时拼凑或静默替换缺失的
native input。Native materialization 和 fidelity 验证由仓库的
[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml) 负责。
其 materialization 阶段在固定 material 和冻结 config 前，通过同一回环探针
验证 required 的 4096 token 上限。Opi 使用已定位的构建 executable；pi 通过
相对路径 launcher 携带已构建的 Node 解释器、package 元数据、发行代码与资源。
探针绑定的 package 和 entrypoint 与 trial material 声明一致。证据失败或不完整
会停止 materialization；artifact verifier 仍要求 comparable pair。该阶段可能
获取锁定的 Cargo 测试依赖，但客户端仅接收隔离的合成认证值与回环 URL。
真实 Provider 行为评测使用显式调用的
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
sealing 或报告发布会被阻止。

| 现象 | 处理方式 |
|------|----------|
| 配置被拒绝 | 先运行 `validate`，再根据 stderr 中的类型化诊断处理。系统没有隐式控制默认值或回退。 |
| run root 已包含 durable state | 不要删除或编辑 trial 证据。使用 `run --recover` 对 durable state 分类；若 trial 崩溃，则使用显式 replacement 流程。 |
| native material 被拒绝 | 通过 native workflow 重新生成并解析；不要替换为未固定的 executable 或 task package。 |
| `verify`/`regrade` 报告 `mutation-detected` 或类型化拒绝 | 保留 bundle 与保留 artifacts 以便诊断，不要修复、重写或重算摘要。 |
| `report` 被阻止 | 检查 bundle verification 与 canary failure；修正输入来源后，使用新的、位于 run root 外的 `--out` 路径。 |

`run --replacement-for <TRIAL_ID>` 会为崩溃 trial 所在的整个对比组创建新身份，不会
复用失败 trial 的身份。

### 本地操作员验收与离线校验器

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
来源记录的缓存上游仓库身份清单。任何人都可以离线校验该证据：

```sh
python3 crates/opi-eval/scripts/verify-local-acceptance.py \
  --root <redacted-evidence-root> \
  --out <verification-report.json> \
  --companion <built opi-eval executable>
```

校验器绝不启动 Agent、读取凭据或联系 provider。它拒绝被替换、
oracle、缺失或仅 fixture 的证据，拒绝把空的一般审计成功当作验收，
仅凭随证据提供的缓存身份校验上游提交（绝不在本 checkout 中
解析提交），并且只针对随证据提供的家族 run 根调用构建出的
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
package、command、extension、启动 hook 或默认捕获路径。现有 session、native
evidence、本地 Eval 报告、配置、凭据和用户工件不会被本 crate 读取、改写或迁移。

当前 crate 版本：`0.8.2`，继承自 workspace package 版本。

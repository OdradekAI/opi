# opi-eval

> 冻结一份评测契约，运行可比较的 Agent harness trial，并离线重新验证 sealed
> evidence。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-eval` 用同一组显式模型参数、环境、任务和 trial 控制来比较多个 Agent
harness。它会在任何 Agent 启动前解析并冻结这些输入，把每次 trial 的证据写入
sealed bundle，并且只根据仍能通过验证的证据生成报告。

本 crate 是未发布、Agent 中立的 workspace companion，不属于 `opi` 运行时，也
不会改变普通 `opi` 的行为。

## 什么时候使用

以下场景适合使用 `opi-eval`：

- 运行前确认实验定义完整，且所有关键输入都有摘要身份；
- 在相同控制条件下比较 baseline 与 candidate harness；
- 用固定的 conformance fixture 检查 Agent 或 benchmark adapter；
- 不重新运行 Agent 或 Provider，直接验证 sealed trial 证据；
- 根据通过验证的 bundle 生成规范化、仅用于 conformance 的报告。

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
- `validate` 支持跨平台运行。本文提供的完整 `run` -> `regrade` -> `report`
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
| Native-material 模式 | 给 `validate`、`run` 或 `conformance` 添加 `--native-material <MANIFEST>`。manifest 会解析精确的 Agent executable、task package、verifier/oracle 入口和 scripted-provider endpoint。 | 能证明该 resolved material identity 所描述、且已获准的 native execution。`conformance` 只允许已注册的 native case 子集。 |

CLI 只消费已经解析的 native-material manifest；它不会临时拼凑或静默替换缺失的
native input。Native materialization 和 fidelity 验证由仓库的
[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml) 负责。
真实 Provider 行为评测使用显式调用的
[opi-eval workflow](../../.agents/skills/opi-eval/SKILL.md)；其预算和证据管理与本文
hermetic CLI 示例相互独立。

## 命令参考

| 命令 | 必需输入 | 重要选项 | 输出与退出行为 |
|------|----------|----------|----------------|
| `validate` | `--config PATH` | `--native-material PATH` 会加入 native integrity identity。 | 一行摘要；解析成功为 0，无效为 1。 |
| `run` | `--config PATH --root PATH --fixtures PATH` | `--recover`、`--replacement-for TRIAL`、`--canaries PATH`、`--native-material PATH`、`--preflight-only`；`--behavior` 用于选择 hermetic 故障 fixture。 | 单行 JSON；完成或 preflight 成功为 0，已落定但未成功为 1，请求被拒绝为 2。 |
| `regrade` | `--root PATH` | 无。 | 单行 JSON；验证成功为 0，发现变更或未 seal 为 1，命令请求被拒绝为 2。 |
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

sealed bundle 一旦生成，就应视为不可变。修改任何受覆盖字节都会导致 `regrade` 和
`report` 验证失败；这两个命令都不会修复证据或静默重算摘要。`--canaries PATH`
指定的文件每行声明一个 canary；如果 exportable content 中出现其中任意内容，
sealing 或报告发布会被阻止。

| 现象 | 处理方式 |
|------|----------|
| 配置被拒绝 | 先运行 `validate`，再根据 stderr 中的类型化诊断处理。系统没有隐式控制默认值或回退。 |
| run root 已包含 durable state | 不要删除或编辑 trial 证据。使用 `run --recover` 对 durable state 分类；若 trial 崩溃，则使用显式 replacement 流程。 |
| native material 被拒绝 | 通过 native workflow 重新生成并解析；不要替换为未固定的 executable 或 task package。 |
| `regrade` 报告 `mutation-detected` | 保留 bundle 以便诊断，不要修复、重写或重算摘要。 |
| `report` 被阻止 | 检查 bundle verification 与 canary failure；修正输入来源后，使用新的、位于 run root 外的 `--out` 路径。 |

`run --replacement-for <TRIAL_ID>` 会为崩溃 trial 所在的整个对比组创建新身份，不会
复用失败 trial 的身份。

## 开发者指南

| 职责 | 权威实现 |
|------|----------|
| CLI 解析与退出码映射 | [`src/main.rs`](src/main.rs) |
| 实验组装与持久 lifecycle | [`src/runner/`](src/runner) |
| Agent 与 benchmark process adapter | [`src/agent/`](src/agent)、[`src/benchmark/`](src/benchmark) |
| 准入与 sealed evidence | [`src/integrity.rs`](src/integrity.rs)、[`src/bundle/`](src/bundle) |
| 离线重新验证与规范化报告 | [`src/regrade.rs`](src/regrade.rs)、[`src/report.rs`](src/report.rs) |
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

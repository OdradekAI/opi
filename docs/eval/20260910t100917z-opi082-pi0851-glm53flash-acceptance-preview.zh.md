# 验收准备：Opi 0.8.2 与 pi 0.85.1

[English](20260910t100917z-opi082-pi0851-glm53flash-acceptance-preview.md)

**准备完成，等待真实执行授权。** 三份原生计划已通过材料校验和资源预检，
尚未启动付费试验。验收仍缺少授权、六份合格原生结果、隐私审查、封存校验
与离线验收检查。本文是准备报告，不是基准成绩或 Phase 完成声明。

## 证据与构建身份

- WSL Ubuntu 私有操作目录：`/home/luiz/opi-eval-runs/20260910t100917z-acceptance-opi082-pi0851`。
- 执行范围：`operator-scope.json`，SHA-256 为 `16f9a80798479e574f5f4e9b4c2278940b3dcd2ecd3e18811c0f2fd82559d553`。
- 本次尚无验收 `manifest.json`、原生标准化报告或封存运行证据。
- 被测 Opi：发布版 `v0.8.2` 的干净源码 `765608a55a78db47e8b41d2639c29ef8c90e0f31`；缓存 musl 构建 SHA-256 为 `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee`。保留的构建命令与所选二进制字节一致，`--version` 返回 `opi 0.8.2`。
- 被测 pi：官方 Linux x64 独立包 `0.85.1`，源码 `d981de1229ef899957bbe968bc8dcda02a21f477`；可执行文件 SHA-256 为 `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403`。材料与探针绑定完整运行包，`--version` 返回 `0.85.1`。
- 独立构建的 Companion：当前干净源码 `55fbc95009e6f47145196dbc539454ebcb1cf692`，包版本 `0.8.2`；可执行文件 SHA-256 为 `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`。
- 工具链：Rust `1.97.1`、Linux x86_64。构建记录、锁文件摘要和 compiler-artifact 定位记录位于 `builds/`、`commands/`。
- Harbor：`4407eb5227a2ff4f0d3f16b2eb48849382fdf276`；Pier：`c1ebc6d145b40fae8425215e3fca528945065124`。最终缓存源码及锁定虚拟环境均通过准备检查。

## 冻结范围与覆盖

每个任务由 pi、Opi 各运行一次，重复次数 1，并发 1，每个家族 `max_trials = 2`。
每份计划绑定一个已准入任务包；计划中的语料数量 1 不是完整上游基准的任务总数。

| 家族 | 所选任务 | 计划 / 已尝试 / 有效 | 原生 job 超时 | 计划摘要 |
|---|---|---|---|---|
| Terminal-Bench 2.1 | `openssl-selfsigned-cert` | 2 / 0 / 0 | 900 秒 | `02215f12563dbebbec7ca91258383613f780f878a78d8d9e3edbceb69a51aa45` |
| Terminal-Bench 3.0 | `batched-eval-parity` | 2 / 0 / 0 | 900 秒 | `62b5f4b11ddff301f8415f4c14b8e03e73e3ad156bc7f7789b8bbf81bfcfcbc6` |
| DeepSWE v1.1 | `abs-module-cache-flags` | 2 / 0 / 0 | 1800 秒 | `5a0c47671489748610b29f258c793700ad1dc9a13fc7e0d98d33372dfbf753fc` |

六次试验均未执行，三个比较对的两侧结果均缺失，没有成绩或已评分分母。
试验 ID、计划文件 SHA-256、材料摘要与探针引用保存在 `operator-scope.json`。

连接固定为 Zhipu `glm-5.3-flash`、`anthropic-messages`、
`https://open.bigmodel.cn/api/anthropic`。用户选择仓库根目录 `.env`；
目前仅识别了变量名 `OPI_BIGMODEL_API_KEY`。获得授权后，将此变量加载到
Companion 环境，不会将整份 dotenv 文件当作原始密钥。本次尚未解析真实凭据值。

提议控制项：输出上限 8192、声明的上下文窗口 131072；temperature、top-p、
reasoning 均省略。输出和上下文控制为必需项。精确客户端离线探针观察到，
两个客户端的首次请求和工具续接请求均携带 `max_tokens: 8192`。该证据仅证明
此选定值，不证明发布版 Opi 0.8.2 支持任意输出上限。上下文元数据已投影到
两端，服务容量及实际推理行为尚未观察。

原生 job 超时包含环境设置、Agent 执行和评分，六次限额相加为 7200 秒，
材料准备与清理另计。每个 job 的监督器 stdout/stderr 上限各 1 MiB。
当前原生派发不应用可配置的超时覆盖值，因此不承诺此类覆盖。没有总 token、
总费用或整体墙钟时限。因尚未真实执行，服务访问、实际用量、账单金额和
运行时长均未知。没有付费重试授权。

## 平台、材料与采集

WSL2 Ubuntu 和 Docker 默认 Linux 引擎报告 x86_64、32 个逻辑 CPU、
16611237888 字节内存。三个任务分别需要 1/1/2 CPU 和 2/4/8 GiB 内存。
所选任务的资源预检未报告不满足或未知要求；这不证明真实任务或评分已经成功。

当前引擎最初缺少所需任务/基础镜像的固定摘要，现已按静态锁摘要拉取并核对
本地别名。未经修改的 TB3 Agent/验证器、DeepSWE 验证器 Dockerfile 均构建成功；
身份与命令见 `checks/prepared-image-builds.json`、`commands/pull-*.json`。
不将静态锁中的纯来源证明/构建工具条目声称为已下载运行镜像。本次未运行
oracle 或原生评分器。

首次完整克隆本地 Harbor 因旧历史对象不可读而失败。随后仅获取固定提交，
在全新的独立浅仓库中成功检出，并通过源码/锁文件校验和离线依赖安装。
原始缓存与失败命令记录均保留。

采用完整私有采集；Agent 使用集成的隔离 `/opi-eval-agent` 目录和
`/logs/agent` 日志挂载。已检查集成的凭据目录清理及 Companion 结算清理路径，
但尚未验证本次真实执行中的清理。后续共享前必须审查原始封存字节；若封存
内容敏感，不能通过编辑或重新封存使其通过。

## 已执行验证

构建、获取、准备、校验、计划和探针的精确 argv、cwd、退出码均位于 `commands/`。
下表 `R` 指操作目录，`F` 为上表三个家族：

| 命令 | 工作目录 | 结果 |
|---|---|---|
| `cargo build --locked --release -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | Opi 仓库 | 退出 0 |
| `cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | Opi 仓库 | 退出 0；1 通过、0 失败；仅使用回环端点和合成凭据 |
| `$R/builds/opi-eval doctor --config $R/inputs/$F.toml` | `$R` | 三次均退出 0 |
| `$R/builds/opi-eval prepare --config $R/inputs/$F.toml --out $R/materials/$F` | `$R` | 三次均退出 0 |
| `$R/builds/opi-eval validate --config $R/inputs/$F.toml --native-material $R/materials/$F/material.json` | `$R` | 完整性摘要推导、固定后确认、最终复核均退出 0 |
| `$R/builds/opi-eval plan --config $R/inputs/$F.toml --native-material $R/materials/$F/material.json --out $R/plans/$F.json` | `$R` | 三次均退出 0，各 2 次试验 |

探针环境引用见 `commands/output-control.json`。尚未运行原生 `run`、`verify`、
`report`、验收校验器或 Opi 采集审计，因为本次没有试验证据。未请求原生重新评分，
不声称工作区运行时门禁通过。测试影响：**retain**；未修改测试或源码文件。
文档检查结果另行随准备交接保存。

## 清理与剩余验收工作

准备阶段清理完成，见 `cleanup-preparation.json`。删除两个空的版本探测目录；
48 个受保护文件摘要全部一致，三个家族的最终材料校验通过，检查时没有容器残留。
保留可复用镜像、runner/任务/客户端缓存、探针和命令证据供后续运行。
回收空间未知。共享缓存、历史证据均保留，构建与探针的 Cargo 租约已释放。

本次 `P19-AC-001` 真实组合流程、`P19-AC-003`–`P19-AC-006` 的真实部分及
`P19-AC-012` 均为 **missing**。准备材料和通过回环探针不能关闭这些条件。
下一步是用户授权这六次试验的精确范围，再执行原生计划、保留全部结算、
审查隐私并离线验证证据。本次未创建实现账本、最终 Phase 审计、提交或发布。

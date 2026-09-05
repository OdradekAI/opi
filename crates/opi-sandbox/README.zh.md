# opi-sandbox

[![Crates.io](https://img.shields.io/crates/v/opi-sandbox.svg)](https://crates.io/crates/opi-sandbox)
[![Docs.rs](https://docs.rs/opi-sandbox/badge.svg)](https://docs.rs/opi-sandbox)

> 独立的命令执行 restriction SDK、面向用户的 CLI 与 protocol backend。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-sandbox` 是一个独立 package，用于监督和限制单个命令进程树。你可以直接使用、
将它作为 Rust 库嵌入，或把它作为
[`command-execution-jsonl-v1`](../opi-protocol/README.zh.md) backend 启动。
它不需要也不会读取 Opi 配置、会话、package storage、trust state 或凭据。

```sh
cargo install opi-sandbox
# 作为库使用：
cargo add opi-sandbox
```

要求 Rust 1.97+（workspace MSRV；edition 2024）。

## 当前状态

当前 crate 版本：`0.8.2`，继承自 workspace package 版本。

官方发布归档面向 Linux 与 macOS。没有官方 Windows `opi-sandbox` artifact；Windows
构建只提供 L0 Job Object 监督，生产环境的 `run` 与 protocol backend 会在启动目标
之前拒绝所请求的 restriction。

## 从 `doctor` 开始

依赖 restriction 前，应先检查当前 host 的实际 posture：

```sh
opi-sandbox doctor
opi-sandbox doctor --json
```

`doctor --json` 输出稳定的 schema-version-1 对象，其中包含 `supported`、`target`、
`mechanisms`、`profiles` 与 `limitations`。即使 `supported` 为 `false`，只要诊断完成，
退出码仍为 `0`。

## 面向用户的 CLI

运行显式的程序与参数向量：

```sh
opi-sandbox run \
  --workspace /path/to/workspace \
  --profile workspace-write \
  --network deny \
  -- /bin/sh -lc 'printf "hello\n" > result.txt'
```

精确语法如下：

```text
opi-sandbox run --workspace <PATH> --profile workspace-write \
  --network <deny|allow> -- <PROGRAM> [ARGUMENTS...]
```

三个 flag 均为必填。`--` 会彻底结束选项解析；之后的值作为原生程序与参数向量传递，
而不是作为隐式 shell 字符串。面向用户的 CLI 以 workspace 作为工作目录，并继承终端
stdin 与 host 环境。每次调用都会通过 `TMPDIR`、`TMP` 与 `TEMP` 获得私有临时根。

目标的 stdout 与 stderr 按原始字节流式传递。退出码映射如下：

| 结果 | 退出码 |
|------|--------|
| 目标正常退出 | 原样返回目标退出码。 |
| Unix signal | `128 + signal`。 |
| 超时 | `124`。 |
| Ctrl-C / 协作式取消 | `130`。 |
| 不支持的平台或启动前 setup 失败 | `125`。 |
| 无效 CLI 输入、workspace 或工作目录 | `2`。 |

## 平台契约

`workspace-write` 允许读取 host 与执行程序，同时把文件系统修改限制在 canonical
workspace 和本次调用的临时根内。`network = deny` 请求启用对应平台的网络拒绝层。

| 平台 | 生产 posture |
|------|--------------|
| Linux | 仅当 Landlock 具备文件系统能力且 host architecture 受已审计的 seccomp 实现支持时可用。使用 Landlock 限制文件系统修改，使用固定的 seccomp 危险 syscall blocklist；`network = deny` 还会限制新建 socket 与 TCP，同时保留 AF_UNIX。 |
| macOS | 仅当 canonical `/usr/bin/sandbox-exec` 通过 runtime probe 时可用。使用 Seatbelt deny overlay：拒绝 workspace/临时根之外的写入；`network = deny` 会拒绝网络操作。它不提供 syscall filter，且 `sandbox-exec` 属于 legacy/experimental surface。 |
| Windows | 不支持 restriction。Job Object 只提供 L0 进程树监督；生产执行会在启动目标前被拒绝，也不发布官方 artifact。 |
| 其他平台 | 不支持；生产执行会在启动目标前被拒绝。 |

受支持平台上的原生 setup 采用 fail-closed。若无法建立所请求的 contract，目标不会被
释放执行。

## Protocol Backend

```sh
opi-sandbox backend --stdio
```

该进程使用 `command-execution-jsonl-v1`：host frame 从 stdin 输入，backend frame 从
stdout 输出，stderr 则保留为有界的带外崩溃证据。每个 backend 进程最多接受一次执行。
命令与策略输入通过 protocol frame 传递，而不是通过进程参数传递。frame、bound 与
兼容性规则详见 [`opi-protocol`](../opi-protocol/README.zh.md)。

## Library SDK

公共 SDK 使用显式输入，并且不会跨调用保存状态：

| 项目 | 用途 |
|------|------|
| `SandboxPolicy` / `Profile` / `NetworkPolicy` | 请求的 `workspace-write` 与网络 contract。 |
| `SandboxRequest` | 显式的程序、参数、workspace、cwd、timeout、环境、stdin 与取消输入。 |
| `Restriction` | 由调用方提供的平台无关 pre-spawn restriction seam。 |
| `SandboxRunner` / `SandboxRun` | 同步 setup 加一个拥有所有权、监督单个进程树的异步事件流。 |
| `SandboxEvent` | `Started`、增量 `Output`、脱敏 `Diagnostic` 与唯一的终态 `Completed` 事件。 |
| `SandboxResult` / `SandboxOutcome` | 结构化的退出/signal/timeout/cancellation 结果、清理状态与有界输出预览。 |
| `NoRestriction` | 显式的纯 L0 实现：提供进程树监督并报告 `ContractStatus::Unrestricted`，不提供原生 confinement。 |

随包提供的 CLI 与 protocol backend 会先探测 host，再选择包内的 Linux/macOS 原生
restriction。直接使用 SDK 的调用方必须显式选择 `Restriction`；通过 `NoRestriction`
构造 runner 只提供 L0 监督，不得描述为已 sandbox 或已 restricted。

完整 stdout/stderr 会通过增量 `Output` 事件和有界 backpressure 交付。终态结果对每个
stream 最多保留 1 MiB 预览，并单独报告截断。丢弃仍在运行的 `SandboxRun` 也会负责
清理：它会在所有终止路径上结束子进程树并删除本次调用的临时根；但如果操作系统无法
确认某个清理步骤，仍可能得到 `CleanupState::Unconfirmed`。

## 安全边界

- 有效 contract 是 `restricted`，绝不是 `isolated`。
- host 读取与程序执行仍然可用；这不是 host 文件、环境变量、凭据或继承文件描述符的
  机密性边界。
- restriction 只作用于目标进程树，不作用于嵌入进程或包围它的 adapter host。
- 本 package 不是容器、VM、远程执行器或多租户安全边界。目标代码以启动用户的 OS
  identity 运行。
- `NoRestriction` 有意用于自定义 SDK 组合，并报告 `unrestricted`；它绝不会静默升级为
  原生 guarantee。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。

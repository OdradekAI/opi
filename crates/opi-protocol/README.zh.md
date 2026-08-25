# opi-protocol

[![Crates.io](https://img.shields.io/crates/v/opi-protocol.svg)](https://crates.io/crates/opi-protocol)
[![Docs.rs](https://docs.rs/opi-protocol/badge.svg)](https://docs.rs/opi-protocol)

> 与具体产品无关的命令执行协议类型、有界 codec、schema 与 fixture。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-protocol` 是一个可复用的 Rust 库，适用于需要版本化
`command-execution-jsonl-v1` wire contract 的 host 与 execution backend。它不包含
进程启动器、sandbox、package manager、路由策略或权限系统。

```sh
cargo add opi-protocol
```

要求 Rust 1.97+（workspace MSRV；edition 2024）。

## 当前状态

当前 crate 版本：`0.8.1`，继承自 workspace package 版本。

Cargo crate 版本与 wire 版本相互独立。当前唯一的 wire identity 是
`command-execution-jsonl-v1`，由 [`execution::v1`][v1] 定义，并通过
`execution::v1::WIRE_IDENTITY` 暴露。

[v1]: https://odradek.ai/opi-protocol/ex/v1/

## Package 边界

本 crate 只负责协议数据与验证：

- 封闭的 host-to-backend 与 backend-to-host frame 类型；
- 原生字符串与字节 payload 表示；
- 有界 JSONL 编码与解码；
- 单次执行内的 request-id、重复 frame 与累计输出检查；
- 确定性 JSON Schema 生成，以及经过评审的 schema/fixture。

它不依赖 `opi-agent` 或 `opi-coding-agent`。进程启动、deadline 执行、进程树终止、
清理、脱敏、实时握手顺序、路由、权限与 sandbox guarantee 均由 host 和 backend
负责。[`opi-sandbox`](../opi-sandbox/README.zh.md) 是该协议的一个独立使用方；
`opi-protocol` 不依赖它。

## `command-execution-jsonl-v1`

host 通过 stdio 启动一个只执行一次的 backend。host-to-backend frame 使用 stdin，
backend-to-host frame 使用 stdout；backend 的 stderr 是带外崩溃证据，不属于协议通道。

```text
host starts backend
  -> initialize
  <- ready
  -> execute
  <- accepted
  <- started
  <- stdout | stderr | diagnostic   (zero or more)
  <- completed | failed
  -> host closes stdin
  -> backend exits
```

每个 frame 都携带同一个非空、由 host 生成的 `RequestId`。程序、参数、工作目录与环境
值使用 `NativeString`，因此可以无损往返 host 上的原生非 UTF-8 值。命令的
stdout/stderr chunk 使用基于 base64 的 `Base64Bytes`；这两种表示有意保持区分。

`initialize` 携带按偏好排序的协议列表。`select` 选择双方都支持的第一个 identity，
而不是选择数值最大的版本。runtime 只有在验证 `ready` 后才发送命令。

## 核心 API

| 项目 | 用途 |
|------|------|
| `HostToBackend` / `BackendToHost` | 两个 wire 方向使用的封闭 frame enum。 |
| `RequestId`, `ProtocolId`, `ImplementationId` | 经过验证的非空 wire identity。 |
| `select` | 按 host 偏好顺序进行 first-match 协议协商。 |
| `NativeString` / `Base64Bytes` | 无损原生命令值与二进制输出 payload。 |
| `Bounds` / `LineReader` / `encode_line` | 单 frame 大小限制与有界 JSONL codec。 |
| `Session` | 对单次执行进行有状态 request-id、重复 frame 与累计输出验证。 |
| `schema` / `schema_with_bounds` | 确定性 JSON Schema 生成。 |
| `FailureCode` / `FailurePhase` | 封闭的 wire-level 失败分类；产品级策略失败不属于本 crate。 |

codec 会执行行、配置、diagnostic 与已解码输出 chunk 的限制。`Session` 进一步限制
已解码 stdout 与 stderr 的累计大小。frame 速率/数量限制、进程 deadline 与完整状态机
转换顺序仍由 runtime 负责。

## 兼容性

`execution::v1` 的 frame 与字段集合从首次发布起冻结。新增、删除或重命名任何 `v1`
frame 或字段都会造成破坏性的 wire 变更。协议演进必须在同级 module 中使用新的
identity（例如 `command-execution-jsonl-v2`），使多个版本能够共存。

未知 frame tag，以及已知 frame 中的未知字段，均属于协议违规。`tests/snapshots/`
下经过评审的 JSON Schema snapshot 不得在缺少人工评审时通过 `INSTA_UPDATE` 更新。
与语言无关的有效和无效 fixture 位于 `tests/fixtures/`。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。

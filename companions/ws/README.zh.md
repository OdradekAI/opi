# opi-ws — 中立自有 WebSocket 客户端底层

`opi-ws` 是仓库的中立 WebSocket 客户端协议状态 Independent Companion
（RDF-24，任务 20.15）。它拥有严格 RFC 6455 帧解码、消息装配、
permessage-deflate（RFC 7692）、显式客户端 Close 交换与异步 IO 状态机。
它刻意不依赖任何 Opi crate、不是根 workspace 成员、不关心 HTTP Upgrade、
凭据、代理、TLS、Provider、Agent 或终端：调用方提供已完成 Upgrade 的
`AsyncRead + AsyncWrite` 流与已验证的协商参数。产品侧严格 Upgrade 桥位于
`opi-ai/src/ws`；真实消费者是 Codex WebSocket provider（20.16），不是本 crate。

- 状态：companion 底层，`publish = false`；registry identity 与发布需另行授权。
- 语言：Rust，edition 2024，MSRV 1.97。
- 许可：MIT（上游压缩 identity 与受支持的 feature 组合见 `LICENSE` 与
  `NOTICE`）。

## 模块一览

| 模块 | 合同 |
|---|---|
| `frame` | 严格客户端帧解码（保留 opcode/RSV、分片、控制帧形状、最短长度编码、64 位高位）与消息装配；存储按实际到达字节增长，类型化 fallible 预留。 |
| `pmd` | 自有 permessage-deflate：tail 前 `SyncPoint` 证明、恰好一次 `00 00 FF FF` 注入、`Z_BLOCK` 边界/`BFINAL`/`ResetKeep` 处理、context takeover、每 poll 有界工作量、显式取消；encoder 是普通安全 `flate2` 客户端。 |
| `io` | 独占连接 owner：owned pending read/write/decode 任务、显式 Close 状态机、调用方提供 mask、背压、终态失败吸收、只丢弃借用的取消。 |

## 接口合同

- 一个 `Driver` 拥有整条连接。`next()` 返回完整消息（含
  `Ping`/`Pong`/`Close`），并在借用 future 被丢弃（取消）后保留部分
  帧、PMD 与 UTF-8 状态——被取消的借用可恢复同一 owned 操作。
- `Driver::new(stream, Option<PmdParameters>)` 按字段名接收双向
  context-takeover 标志及窗口。server 参数配置接收，client 参数配置发送；
  `None` 禁用 PMD，RFC 中省略的窗口参数映射为 15。固定 encoder 后端在
  内部将请求的窗口 8 归一化为 9；wire 窗口由下述 Upgrade 测试独立检查。
- 出站帧用调用方提供的新鲜 mask 排队；没有隐藏 mask、响应、timer、
  重连或重试。
- `finish_close` 完成显式 Close：要求双帧、写 shutdown 与对端传输 EOF，
  保留 in-flight 应用事件（有界），唯一 deadline 是调用方的绝对
  `tokio::time::Instant`。
- 异常 EOF/协议/Close 超时失败以类型化错误出现一次，丢弃保留的接收事件，
  随后接收永久 EOF。成功完成 Close 后仍可交付其保留事件。
- 存储随实际接收字节以 `try_reserve*` 类型化失败增长；从不按声明长度
  投机预分配，没有消息大小或迭代上限（内部 8 KiB/quantum 调度值是
  协作边界，不是限额）。

## 上游压缩与 unsafe 边界

固定依赖图为未修改的 `libz-rs-sys = 0.6.8`（关 default features，开
`std` + `rust-allocator`）、`zlib-rs 0.6.8`、`flate2 = 1.1.10`（关
default features，`zlib-rs`）。decoder 调用 doc(hidden) 公开导出的
`inflateSyncPoint` / `inflateResetKeep`（zlib-rs `__internal-api`）。本
crate 唯一的 unsafe 边界在 `src/pmd.rs` 内有完整注释：唯一 boxed
`z_stream`、无 clone/allocator/callback 泄漏、每次调用前后重绑定借用
指针、恰好一次 `inflateEnd`、以及刻意的 `Send`-无-`Sync` 实现。受支持
的 feature 组合与升级时的上游审计义务见 `NOTICE`。

## Feature 图

| 图 | Encoder 后端 | Decoder 后端 |
|---|---|---|
| default（普通） | flate2 `zlib-rs` | 直接 `libz-rs-sys` Rust 路径 |
| `union-default` | flate2 `zlib-rs`，合并 default features | 直接 `libz-rs-sys` Rust 路径 |
| `union-c-zlib` | flate2 `zlib`（原生 C `libz-sys` 构建） | 直接 `libz-rs-sys` Rust 路径 |

flate2 1.1.10 优先选择 C 后端，其次是 `zlib-rs`，两者均未选定时才使用
miniz_oxide。default 组合将 miniz_oxide 加入依赖图，但仍使用 `zlib-rs`
encoder。

## 测试

`cargo test` 运行移植的研究测试组（`frame`、`pmd`、`pmd_async`、
`pmd_red`、`replay`、`io`、`close`、`gaps` 共 76 个函数）以及
`tests/boundary.rs` 的边界重放：从 `fixtures/boundary-seeds.json` 重新
生成固定的 72,413 条确定性 PMD 语料，并逐条对照
`fixtures/boundary-expectations.json` 中的记录分类（超过 64 字节的输出
按 SHA-256 比较）。fixtures 由封存研究 oracle 提取
（`extract-boundary-fixtures.py` 记录来源）。请在
`--features union-default` 与 `--features union-c-zlib` 下运行同一测试组
以覆盖调用方 feature 组合。

根 `opi-ai` 测试
`upgrade_window8_wire_has_independent_decode_and_distance_proof` 在普通、
default、C-zlib 三图通过公开 `opi_ai::ws::upgrade` / `HttpClient` 路径
连接 loopback 对端。它在双方向协商窗口 8，使用独立 RFC 1951 decoder
审查多条消息及有/无 context takeover 两组。审查重建原始字节，将实际
wire distance 与 256 字节窗口对照，并包含 distance 257 的反控制：窗口
9 接受，窗口 8 拒绝。

一个 idle-read `cfg(test)` 测试注册真实 Pending read waker，在连接 owner
内 select 逻辑取消，随后丢弃 `Driver`，观察 ACK 前 transport 与 PMD
释放及无晚输出。只丢弃接收借用 future 仍可恢复；此 owner 取消测试没有
新增生产取消 API。

这些是局部协议与所有权检查。完整 Codex WebSocket、continuation、
pool/cleanup 接入及原生平台/MSRV 验收仍由后续任务承担。

## 维护边界

出现以下情况时重开设计评审而非静默适配：上游隐藏 API 消失或语义变化、
要求任何 unsafe/feature/安全变更、接受集会缩小、或第二个生产消费者需要
不同接口。升级 libz-rs-sys/zlib-rs/flate2 时必须记录解析的
source/checksum/features，并重跑三图测试组、边界语料、
Send/Close/取消/公平性探针与打包证明。

# Pi 1.3.0：WS 设计就绪的后端决策

> 2026-10-16。研究与隔离原型；不修改产品源码、根 Cargo、CI 或实施账本。这里的 **设计 READY** 表示候选、可表达机制和有限适配范围已经确定；产品 provider、平台和生命周期验收仍 **OPEN**。本附件没有既有英文 counterpart；双语 spec 应同步引用同一决策。原始差分与源码身份见 [资产清单](offline-probes/readiness-ws/assets.json)、[最终结果](offline-probes/readiness-ws/results.json)、[摘要](offline-probes/readiness-ws/summary.json)。

## 1. 明确选择

选择 **自有严格 HTTP Upgrade + `ratchet_core` / `ratchet_ext` / `ratchet_deflate` 1.2.1 的 PMD codec + 公共 extension 适配 + 一个覆盖 PMD／RFC 长度与存储增长的受控源码补丁**。不再以 TTW/TW 0.28.0 为完整 WS 后端，也不采用 Ratchet 的固定 32 header 握手或未经适配的 decoder。

这项决定不要求另造 WS frame parser。自有 Upgrade 消费上层已经完成 proxy/TLS 策略的异步流，将 HTTP head 与 tail 分离；经严格协商后调用公开 `WebSocket::from_upgraded`。帧、mask、fragment、control、PMD、Close 交给选定 codec；薄适配只恢复每条消息的压缩标记生命周期，源码补丁在原 codec 内约束 PMD 的 RSV1 所在帧，并修复 RFC 长度、checked arithmetic 与按实际到达字节增长的存储路径。后续在产品内落实这些合同；不能以 Unsupported、SSE 最终成功或新增大小限制替代。

| 候选 | 固定事实与结果 | 决定 |
|---|---|---|
| TTW/TW 0.28.0 | 上轮真实 Node/Bun 都协商并解码 PMD；该 codec 没有 PMD，partial 接口也不能补上 | 淘汰为完整后端；历史结果保留 |
| soketto 0.8.1 | 发布源码 handshake 32 slots；Deflate client 要求服务端确认 `server_no_context_takeover`。普通 `permessage-deflate` 响应已是参考运行时的有效子集；不能把这种要求变成 Opi 策略。该候选仅做源码比较，未运行其完整客户端 | 不选；不是宣称其所有 PMD 路径不可用 |
| Ratchet 1.2.1 受控组合 | 公共 extension/provider/from-upgraded seam；真实 PMD/context/window/fragment 与动态握手 tail 均已运行。原库也被多项反例击中，下面的适配和 patch 均为选择的必要组成 | 选定该有限受控组合；不声称原库可直接满足全部合同 |

来源：[soketto 0.8.1 发布源码](https://docs.rs/crate/soketto/0.8.1/source/src/extension/deflate.rs)、[Ratchet core 1.2.1 发布源码](https://docs.rs/crate/ratchet_core/1.2.1/source/src/ws.rs)、[Ratchet extension 1.2.1 发布源码](https://docs.rs/crate/ratchet_ext/1.2.1/source/src/lib.rs)。各文件 SHA 与行数在 [source-pins](offline-probes/readiness-ws/source-pins.json)，归档身份在 [retrieval](offline-probes/readiness-ws/retrieval.json)。

## 2. 固定输入、运行边界与真实成本

- 参考仍是 Node **24.13.0**，SHA `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`；Bun **1.3.5+1e86cebd7**，SHA `0025a8bc31f136745d102212ddb192a1876c721b71deeb43123d18375fcf030c`。全部 Bun 子进程使用 `--no-install`。只用 `127.0.0.1`、固定假 Authorization/canary；子进程环境白名单不继承真实 tokens/proxies/NODE_OPTIONS。
- Rust **1.99.1**、Windows MSVC；tokio **1.52.3**、Ratchet 三包 **1.2.1**、flate2 **1.1.10**、libz-sys **1.1.30**；研究锁图为 **93 个 registry packages + 一项受控 path patch + harness**。没有把该研究锁图等同于产品锁图或建议整体升级其余依赖。[Cargo.lock](offline-probes/readiness-ws/rust/Cargo.lock) 与 [features](offline-probes/readiness-ws/cargo-features.txt) 固定实际版本。
- `ratchet_core`/`ratchet_ext` 的 default features 为空；不引入 `ratchet_rs` 自带的 tracing-subscriber，不开 split/fixture/TLS。`ratchet_deflate` 强制 `flate2/zlib`，因此**包含 native C、libz-sys build.rs、cc/pkg-config/vcpkg 构建依赖与 FFI**。本机实际静态编译其归档内的 zlib；不是 pure Rust 或已经证明的三平台可移植性。系统动态 zlib 探测差异必须在产品构建策略中固定，Windows/Linux/macOS、MSRV 和产品 feature union 继续验收 OPEN。
- 依赖准备明确联网：官方 crate metadata/归档及 Cargo index/fetch，保留 [准备日志](offline-probes/readiness-ws/cargo-fetch-prepare.txt)；93 个 registry archive 的 checksum 均与研究 lock 核对，[dependency-pins](offline-probes/readiness-ws/dependency-pins.json)。只有准备完成后的 `cargo build --locked --offline` 与 loopback 测试属于离线。使用已有 `E:/opi/cargo-targets/windows-direct`，没有清理缓存或新建每会话 target。
- 官方仓库在查询时均未归档，Ratchet `pushed_at=2026-06-04`，soketto `pushed_at=2026-02-10`，详见各 maintenance JSON。这只说明仓库活动；具体 Ratchet 1.2.1 的质量判断来自本轮反例/修复，不能用活动日期证明成熟性。

## 3. 已执行的相同 wire 输入

先前六行 patch 的历史 **20 cases × 4 clients = 80 observations**：Node/Bun Pi 构造形状、带受控 frame patch 的 Ratchet 原握手、带 patch/PMD adapter 的自有握手。均记录命令、退出码、stdout/stderr、请求、协商、帧摘要、客户端发帧与解码；原握手这个控制保留 decoder 的原行为。不是完整 Pi/opi provider。

| 输入 | 真实观察与结论 |
|---|---|
| 普通 PMD、首帧 tail、fragment + 中间 Ping | Node/Bun 与选定组合收到准确消息；不能在返回 Ping/Pong 时清掉累积的 message buffer。该清理曾导致早期 harness 假错误，已纠正并重跑 |
| 两消息 context takeover / 双方 no-context | 选定组合分别收到两条；独立对第二条 takeover 压缩体单独解码失败，证明语料实际使用了历史字典，未用两个独立压缩消息伪装 takeover |
| window bits 8 / 9 / 15 | Node 与选定组合收到两条；Bun 在 8 的这一合法语料已 open/send、但收到零条，实际差异保留。9/15 两参考与原生均成功；不能把 Bun 的这个异常变成 Rust 限制 |
| 双向发帧 | Rust 发端实际 RSV1 压缩，server 解码得到完整 **24,023 字节**；Node 本轮 send 保持未压缩，Bun 在部分较大 PMD send 中压缩。逐消息是否压缩属于内部选择，不能据 wire 字节不同宣称公开消息不同 |
| 125 headers / 66,000 字符 header value | Bun 和 owned Upgrade 稳定成功；Ratchet 自带握手拒绝 125 headers。Node 拒绝 66,000 value。动态 header 容器消除了候选的 32 slots 限制；采样点不是新 SDK 上限 |
| invalid Accept / unknown extension / unsolicited subprotocol / invalid PMD16 / duplicate PMD parameter | owned 在 Upgrade 阶段拒绝；不会把 `negotiate_client=Ok(None)` 当作“响应宣告 PMD 但可以继续”的成功。Node 有两类子进程 exit1，原始结果保留，不能要求 Rust 仿效进程异常 |
| 压缩消息后跟未压缩 fragmented message + Ping | Node/Bun 两条；未经 message-state 适配的 Ratchet 只有一条后 inflate 失败。public decoder wrapper 恢复两条。见 [未适配失败](offline-probes/readiness-ws/results-unadapted-mixed-failure.json) 与最终相同 case |
| 协商 PMD 后 control/continuation 设置 RSV1 | 原 codec 漏拒，PMD-only 六行源码 patch 在原有 frame header 解析位置拒绝，最终结果有 `UnknownExtension`。这是明确的协议修正，不能复制参考实现可能的宽松接受 |
| Connection token list | owned 按 token 解析成功，两个参考在该采样拒绝；选择遵守合法 Upgrade token 语义，不新增对这个合法值的拒绝 |
| partial header / partial payload 时取消 read 后销毁整个连接 | 两条额外观察均 exit0、对端约 0.5 秒看到 EOF。**没有证明 cancel 后可以继续读或归还连接池**；库明确声明 read/write 不具 cancel safety |

这 80 条及另 2 条销毁观察没有被写成产品验收 PASS，也没有记为后续 R01 扩大补丁的执行结果。最初一次 patch 文件使用本机默认编码写出，导致编译失败；旧 shell 链没有及时停止，随后旧 binary 的一轮输出已废弃。修正 UTF-8、成功构建后，最终 80 条完整重跑；[排除记录](offline-probes/readiness-ws/excluded-exploration.json) 与 [编译失败日志](offline-probes/readiness-ws/cargo-build-failed-encoding.txt) 保留，复现入口对 build 失败立即退出。

## 4. window8：不是因配置数字而重新选择

Ratchet 把 zlib 初始化的 8 提升到 9，不能仅凭该数字断言违反协商。固定 libz-sys 1.1.30 所带 zlib 的 `deflate.h` 定义 `MAX_DIST = w_size - MIN_LOOKAHEAD`，`MIN_LOOKAHEAD = 258 + 3 + 1`；9 bit 发端的最大引用距离因此为 **250 ≤ 256**。这是**该固定 zlib 实现**的上界，不是对其他 compressor/backend 的保证。

本轮还实际发送大于 8 bit 窗口很多的消息：Rust 客户端 RSV1 payload 用独立 `zlib.decompressobj(-8)` 解码完全匹配，独立 RFC1951 bitstream 扫描检查距离；服务端两个响应的 max distance 分别为 21/148，客户端该大输入的 compressed Huffman blocks 没有 distance 引用。详见 [window8-audit](offline-probes/readiness-ws/window8-audit.json) 和 [独立扫描器](offline-probes/readiness-ws/deflate-distance.py)。没有缩减 PMD offer、禁用窗口8、提高 SDK 限制或借 SSE 掩盖差异。

## 5. 必须落实的设计合同

1. **Upgrade owner**：使用已受 scoped transport/proxy/TLS 处理的流；URI 与用户 header 经 typed validation 后编码。随机新 nonce，严格核对 101/HTTP版本/Accept、Upgrade/Connection tokens、singleton 与重名冲突、只允许已提议的 protocol/extension。对 PMD 参数进行精确 token、重复、值与 offer 一致性验证；不能依赖 Ratchet 宽松 `starts_with` 分支。合法 redirect 走统一 transport redirect 策略，跨 origin 不泄露认证。研究 parser 只是可表达性原型，不是已交付的完整 HTTP 安全实现。
2. **无新增候选预算**：动态 head/header slots；head 与 tail 的大小分别计算，tail 按原顺序只交一次。Ratchet `max_message_size` 由默认 64 MiB 改为 `usize::MAX` 去掉 backend 人为 cap；仍遵守 WS/RFC 长度规则和实际内存分配失败。没有从实验 timers/125/66,000/24,023 推导 Opi 预算；默认是否限制只依 Pi 公开合同与另行明确的调用方配置。
3. **PMD owner**：选定 provider 正常协商 context/window；public wrapper 在新 Text/Binary 起始帧记录 RSV1，只对当前压缩消息委托 decoder，FIN 清除自身标记。保留每连接 encoder/decoder context；Ping/Pong 不能抹掉 partial message；控制与 continuation RSV1 由 [受控补丁](offline-probes/readiness-ws/ratchet-core-pmd.patch) 在原解析点验证。
4. **取消与池**：连接 owner 持有原始流、codec、部分消息及压缩 context；每条消息完成前保留读取状态。取消途中 read/write 就使该连接不可复用并 drop 整个 owner，禁止 cancel 后继续读、把半帧 codec 归还池、或跨连接重用压缩 state。正常 terminal 路径按 continuation/池合同决定可复用性；只有完整、安全边界才能归池。超时与调用方取消共享该销毁规则，不能新增隐式重连/重发。
5. **维护边界**：private patch 现包含 `protocol/frame.rs`、`framed/mod.rs`、`errors.rs` 三个 runtime 文件的 RFC 长度、存储增长与 PMD-only RSV 修复（另有纯测试模块／hook 和两条 upstream test-helper rustdoc），准确差分及各文件前后 SHA 见 [R01 纯内存资产](offline-probes/readiness-ws/memory-only/README.md)；PMD RSV 规则仍只用于 PMD-only codec；不能默默用作支持任意其他 extension 的通用 Ratchet fork。记录官方 archive checksum、原/后文件 SHA、diff、feature graph。公开 PMD wrapper 留在 adapter 内部。升级时先重新跑未适配反例与修复正反例，再决定删除 adapter/patch；不能以仅能 apply 为升级通过。优先回馈上游并在有等价维护版本时移除 patch；本任务不发上游消息或创建外部 fork。

这项设计选择承担一个有限的受控维护成本：HTTP Upgrade owner、本地薄 PMD adapter、上述有限的三文件 codec patch；不选择修改 TLS/proxy、重写帧解析器或压缩引擎。若未来需要新的 extension，必须重新审查 PMD-only patch 的适用边界。

## 6. 关闭的是设计阻塞，保留的是实施验收

后端缺失 PMD、固定 head slots、不可表达的 message-state 修复、取消后复用策略已给出具体选择、公开机制或已运行的受控修复；可以按本附件将 WS **设计阻塞标记为已决**。V-02/30/49/58 仍需要产品公共 provider、Auto/explicit fallback、continuation/pool/cleanup、完整分片/压缩负例、scoped proxy/TLS、产品锁图、MSRV 与三平台结果；它们是 **实施/覆盖 OPEN**，不是本轮已经通过的交付。

步骤与检查：`固定源码/归档 → verify: pins/member byte comparison`；`隔离 build → verify: locked/offline exit0`；`相同 loopback wire → verify: raw result / 收发字节 / 异常退出`；`适配与 patch → verify: 旧反例失败与修复后正反控制`；`复现入口 → verify: 离线准备成功、错误 source pin 在写 workspace 前 exit1`。研究增加测试资产 **add**；产品测试 **none**。


## 7. 独立复审 R01：只在内存修复长度与存储边界

独立复审发现旧 patch 在 `817f8000000000000000` 触发 capacity overflow，在 `817fffffffffffffffff` 触发 header+payload 整数溢出，且接受非最短 126 编码。原始结果原样保留为 [历史反例](offline-probes/readiness-ws/memory-only/review-r01-historical-results.json)。它们否定“只把 cap 改为 usize::MAX 就足够”的旧结论。

按用户新约束，本次后续**没有任何请求、下载、socket、连接或 loopback**，只用已有缓存归档、内存 AsyncRead/Cursor/Vec 和 `cargo test --locked --offline`。新选择的具体补救是：

- 在原 `FrameHeader` 解析处拒绝 64-bit length 最高位、非最短 126/127 编码；使用 `usize::try_from` 处理平台表示，不截断。
- header+payload 与聚合 message 长度使用 checked 运算；超出缓冲表示返回 typed `StorageError::Capacity`，不 panic，不把合法但当前机器不能表示的数据伪装成新 SDK 配额。
- 删除按对端声明的全部剩余 payload 提前 `resize` 的逻辑。读取使用小块局部数组，仅在字节真实到达之后增长持久缓冲；8192 是内部 read 粒度，不是 frame/message 上限。合法 1 TiB 声明只有 10 字节 head 时停在 Pending，未预留整个 payload；随后 EOF 返回 typed IO。
- `BytesMut` 1.11.1 无公开 `try_reserve`。bulk 增长使用 `Vec::try_reserve_exact` 与公开所有权转换；为减少复制的 preferred capacity 失败时回退精确 required capacity。真实 bulk reservation failure 保留 `TryReserveError` typed cause，物理表示失败单独分类。共享 tail、累计碎片与空消息仍有内存回归。

最终 **14 个测试通过**：13 个内部边界单测，另一个 public codec 单测遍历 12 个 PMD／context／window／碎片／RSV 的冻结内存 fixtures。两个此前 panic 的输入现在得到 typed 错误；非最短编码拒绝；tail 次数与顺序正确；原 message-state adapter 与 RSV 补丁保持。纯内存入口还对复制到 TEMP 的旧 80-row summarizer 执行正负 fixture；不改历史 JSON，不运行旧网络脚本。真实输出见 [memory-test](offline-probes/readiness-ws/memory-only/memory-test.txt)，复现入口见 [verify.py](offline-probes/readiness-ws/memory-only/verify.py)，多文件身份和最小 diff 见 [source-files](offline-probes/readiness-ws/memory-only/source-files.json)、[patch](offline-probes/readiness-ws/memory-only/ratchet-core-memory.patch)。

typed 分配合同有明确边界：`Bytes::from(Vec)` 可能分配小型 shared metadata、BytesMut split 和 PMD/native 代码仍有 infallible allocation；本次不保证全进程 OOM 可恢复，不捕获 abort 后假称错误。产品若要求所有分配均 fallible，需要另立更宽的存储／压缩实现合同；本研究只证明大块 read/message 存储增长和表示错误路径。三平台、产品锁图、完整 provider 以及新补丁的网络差分仍 **OPEN**；没有用旧 80 条冒充新补丁已跑相同网络验收。

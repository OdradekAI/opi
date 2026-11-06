# Rust 生态复用与测试边界核查（研究证据，不是已批准设计）

核查日期：2026-11-06。仅只读研究；未修改 opi 仓库、依赖、测试、分支或产品 README；未运行付费 provider。本地基线：`D:/Luiz/Odradek/opi`，HEAD `90f953d5ee288c165dc295bfdfb7cb7a05d7cecd`；Pi 行为参照 `.pi/pi-1.5.0/packages/ai/src/`，其完整源身份由上游契约研究负责确认。本报告对应 `decisions/03-rust-ecosystem.md`。

## 结论

1. “Rust 没有成熟 HTTP/WebSocket/AWS SDK，因此必须自研整层”不成立。HTTP 已在复用 reqwest；tokio-tungstenite 提供成熟异步 WebSocket；AWS 官方 Rust SDK 提供 Bedrock ConverseStream、凭据、签名和事件流组件。[reqwest 文档](https://docs.rs/reqwest/0.13.5/reqwest/)、[tokio-tungstenite](https://github.com/snapview/tokio-tungstenite/blob/76e10f08c488eb0032f136c7c56a437cd30164c2/README.md)、[Bedrock 实际生成代码](https://github.com/awslabs/aws-sdk-rust/blob/02e59e21fea3bbbab13fb05416a6286035789c5c/sdk/bedrockruntime/src/client/converse_stream.rs)。
2. “crate 存在，所以整个 opi 私有实现可以直接删除”同样不成立。tungstenite 仍不支持 permessage-deflate；默认连接 helper 不继承 reqwest 代理/TLS；AWS Rust 默认凭据链和同步 interceptor 不能直接等价于 Pi 的 profile 优先、异步 hook、原始流事件形状和错误处理。下文给出具体边界。
3. 推荐决策规则：通用协议优先复用；opi 拥有 Pi 可观察语义的薄适配；只有已接受的契约无法由公开 API 满足时才保留最小自研机制，并给出反例、覆盖和退出条件。依赖替换不是本轮实施项；先批准兼容范围，再做有明确退出条件的技术验证。
4. **旧 npm `ws` 前提已不能直接沿用。** 现有实现和现有测试不自动成为新要求。尤其 PMD 的严格边界、异常分配、Close 与 EOF 等当前私有契约，必须与 Pi 外部可观察要求分开。Pi 的 Codex 源码 `api/openai-codex-responses.ts:1011–1041` 选择 Bun 或 `globalThis.WebSocket`，此处未显式配置 PMD。不能以历史 npm `ws` 的 API 或默认压缩行为作为当前 1.5.0 硬验收，也不能从当前 opi 的 PMD 代码反推出 Pi 要求精确复刻这些内部机制；实际运行时的协商行为仍需指定运行时版本后实测。

## 外部来源身份

| 来源 | 本次读取身份 | 用途 |
| --- | --- | --- |
| snapview/tungstenite-rs | `2f20dc53e2a8d09bba02970d7ba15c5f33fbc17a`，2026-10-23 | README、WebSocketConfig、fuzz 配置 |
| snapview/tokio-tungstenite | `76e10f08c488eb0032f136c7c56a437cd30164c2`，2026-10-24；README 推荐 0.30 | 连接 helper、原始流接入与取消契约 |
| seanmonstar/reqwest | `8e1c8061e5dd936762dcef08a64ef2cff584b295`，2026-10-19；公开文档 0.13.5 | 实际 proxy/upgrade 测试与私有测试 |
| awslabs/aws-sdk-rust | `02e59e21fea3bbbab13fb05416a6286035789c5c`，2026-11-05；BedrockRuntime 1.153.0 | 生成客户端、配置、依赖与 MSRV |
| tokio-rs/tokio | `695a07dd5e5baae82eb8b6931afa61029cd9b854`，2026-11-04 | 私有 Loom 测试与公共 oneshot 测试 |
| serde-rs/json | `8c535a260ce561eaab4111aafc29038eb9f77ff3`，2026-10-16 | compile-fail、分组测试和特殊源码导入 |

这些是核查快照，不是要求项目升级到最新版本。opi 当前 workspace 声明 reqwest 0.12；迁移到 0.13 应单列行为与成本验证，不能混入结构整理。

## HTTP：保持复用，收缩 opi 策略边界

reqwest 提供异步 Client、连接池、请求体、代理、TLS、redirect policy。官方建议重复使用 Client；默认重定向 10 次，系统代理默认读取 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY 等变量，显式代理和 `no_proxy()` 可覆盖。它也提供原始 HTTP upgrade：实际测试发送 101，调用 `Response::upgrade()`，再双向读写字节。[功能与默认值](https://docs.rs/reqwest/0.13.5/reqwest/)、[真实 upgrade 测试](https://github.com/seanmonstar/reqwest/blob/8e1c8061e5dd936762dcef08a64ef2cff584b295/tests/upgrade.rs)。

本地 `crates/opi-ai/src/http.rs` 已包装 reqwest，并包含自己的重定向默认值（20）、错误读取、取消和池配置；`src/ws.rs` 使用专用 HTTP/1.1 upgrade client、冻结代理与 TLS、拒绝 redirect。它们不是重写 HTTP 协议。建议继续保留一个清晰的传输 owner，但把 provider 错误分类放到其实际所属边界；不要因为文件名叫 `http` 就把所有错误语义归到通用传输。

需由 opi 继续证明的不是“reqwest 能发 HTTP”，而是选用配置在真实调用图中生效：scoped env 与全局 env 优先序、NO_PROXY 匹配、HTTP 与 HTTPS CONNECT 路径、auth header 跨源保护、H1 upgrade、请求 timeout 与取消、response hook 先后、有限错误体处理。依赖的测试覆盖不了包装层传错选项或绕过包装层的错误。

## WebSocket：官方库有能力，PMD 是实质缺口

官方 tungstenite README 明确声明当前不支持 permessage-deflate；并声明覆盖内部单元测试和 Autobahn。其实际配置默认最大消息 64 MiB、最大 frame 16 MiB、read buffer 128 KiB，均须与选定契约比较，不能默认等价。[固定版本 README](https://github.com/snapview/tungstenite-rs/blob/2f20dc53e2a8d09bba02970d7ba15c5f33fbc17a/README.md)、[WebSocketConfig](https://github.com/snapview/tungstenite-rs/blob/2f20dc53e2a8d09bba02970d7ba15c5f33fbc17a/src/protocol/mod.rs#L40)。搜索可能命中 `phantom-tungstenite` 的压缩支持；那是另一发行身份，不能当作官方 tungstenite 已支持的证据。

tokio-tungstenite 的 `connect_async` 通过域名直接 `TcpStream::connect`，再 TLS/handshake；这段 helper 没有接入 reqwest 的代理或 opi 的冻结 TLS 配置。但其 `WebSocketStream::from_raw_socket` / `from_partially_read` 接受自定义 `AsyncRead + AsyncWrite` 流，因此“使用它就无法保留代理”也不成立：可以先由既有传输完成连接/upgrade，再交给 codec。[连接代码](https://github.com/snapview/tokio-tungstenite/blob/76e10f08c488eb0032f136c7c56a437cd30164c2/src/connect.rs)、[原始流接入与取消说明](https://github.com/snapview/tokio-tungstenite/blob/76e10f08c488eb0032f136c7c56a437cd30164c2/src/lib.rs#L185)。

该文档明确 read-side `StreamExt::next` 被取消后不丢消息；Sink 发送侧没有文档化的取消安全保证。这比“异步库应该支持取消”更具体，但仍不足证明当前 opi 的所有发送/关闭契约。

本地 `src/ws.rs`、`src/ws/frame.rs`、`src/ws/pmd.rs`、`src/ws/io.rs` 与 README 的 Private protocol ownership 说明当前自行拥有：严格 upgrade/扩展协商；frame/fragment/control/UTF-8；双向 context takeover 和 window；PMD 尾部及 reset；已接收字节驱动的 fallible 分配；借用 future 取消后的部分进度；带 deadline 的 Close 双帧、shutdown、peer EOF；终止错误只出现一次。`pmd.rs` 实际使用 `libz_rs_sys`、自有 Decoder、取消标记和 TryReserveError；`io.rs` 实际保存 Pending 并定义 CloseTimeout/PeerEofBeforeCloseSent 等错误。这些是可见实现事实，不是自动被采纳的目标。

建议的候选判定：

| 路径 | 适用条件 | 必须先验证 |
| --- | --- | --- |
| 官方 tokio-tungstenite + 薄包装 | 批准的契约允许不协商 PMD，或 PMD 不在支持范围 | 真实代理/TLS upgrade、消息/控制帧、取消发送、错误映射、Close、pool reuse、Codex fallback |
| 具备 PMD 的其他成熟库或窄上游扩展 | PMD 是硬要求，公开接口可完整承载 | 协商与方向窗口、context takeover、分片、畸形压缩流、取消、维护/平台/许可证；不能仅查看 feature 名 |
| 暂留最小私有 codec | 硬要求被明确接受，候选在上述反例失败，包装无法补足 | 限定 owner、独立 oracle、fuzz/协议验收、退出条件；不顺势维护新通用 WebSocket 产品 |

若借用上游 framing 仍要改私有字段、复制 frame parser 或旁路上游状态机才能加压缩，这已经不是“薄包装”；需要按 fork/自研维护成本重新比较。现有 PMD 足以说明无条件替换不成立，不足说明永远应自研。

## AWS：优先验证官方 SDK；必要时复用更低层组件

AWS 的实际 `sdk/bedrockruntime/src/client/converse_stream.rs` 提供 fluent builder、消息/工具/附加模型字段以及 `EventReceiver<ConverseStreamOutput, ConverseStreamOutputError>`，失败返回 `SdkError<ConverseStreamError>`。当前 Cargo.toml 是 1.153.0、rust-version 1.94.1，依赖 aws-sigv4、aws-smithy-eventstream 和 async/runtime 等官方组件。[操作源码](https://github.com/awslabs/aws-sdk-rust/blob/02e59e21fea3bbbab13fb05416a6286035789c5c/sdk/bedrockruntime/src/client/converse_stream.rs)、[manifest](https://github.com/awslabs/aws-sdk-rust/blob/02e59e21fea3bbbab13fb05416a6286035789c5c/sdk/bedrockruntime/Cargo.toml)。当前生成配置还直接提供 `bearer_token` / `bearer_token_resolver` 和 `auth_scheme_preference`，不是只支持 SigV4。[配置源码 L363 起](https://github.com/awslabs/aws-sdk-rust/blob/02e59e21fea3bbbab13fb05416a6286035789c5c/sdk/bedrockruntime/src/config.rs#L363)。

官方 HTTP 默认基于 hyper/rustls/aws-lc-rs，可替换 HTTP client；通过 aws-config 注入的 client 同时用于服务和凭据 provider。官方文档显示 standard/adaptive retry 和可配置最大尝试次数；aws-config 默认链是 env → shared config → web identity → ECS/general HTTP → IMDS，外层缓存刷新。[HTTP 配置](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/http.html)、[重试配置](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/retries.html)、[凭据链](https://docs.rs/aws-config/1.13.0/aws_config/default_provider/credentials/struct.DefaultCredentialsChain.html)。不要将这些默认策略直接当成 Pi 行为。

本地 `src/bedrock/mod.rs`、`event_stream.rs`、`sigv4.rs`、`retry.rs` 自行负责服务 wire/signature/retry，`src/auth/aws/` 自行负责来源选择、缓存、STS、SSO、process、metadata、web identity、login；manifest 只复用了 aws-smithy-query/xml 等局部组件。这是下一轮最值得按责任而非文件逐块审查的维护面。

需要逐项解决的具体差异：

- **凭据优先序。** Pi `api/bedrock-converse-stream.ts:167–173` 明确显式 profile/scoped AWS_PROFILE 胜过 ambient access keys；Rust 默认链先读 env。可选择构造特定 provider 而非整体重写，但必须验证 profile、scoped env、无 credential 混用与 refresh/cancel。当前 opi BuiltinAwsConfig 还支持注入 source HTTP/process，不能被不可控的全局发现替代。
- **代理/HTTP 与 endpoint。** Pi `:180–255` 有 ARN region 优先、standard/custom endpoint 规则、HTTP 代理、force HTTP1、skip-auth 的 dummy credential 签名与 bearer 分支。官方支持自定义 HTTP 是扩展点证据，不是这些策略已自动对齐；服务请求和凭据请求可能需要不同 scope，须明确配置归属。
- **异步 callback。** Pi payload callback 在 command 创建前 await；response middleware 保存 gateway headers 并 await 回调；每个 SDK 流事件在转换前 await `onProviderStreamEvent(item)`。AWS Rust Intercept hooks 返回同步 `Result<(), BoxError>`，不是 Future。可在 send 外 await payload/事件回调，或用自定义 HTTP owner 保存响应头并正确安排 await；不能仅接一个 interceptor 就声称四类 async hook 全覆盖。[官方 interceptor 指南](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/interceptors.html)、[trait 签名](https://docs.rs/aws-smithy-runtime-api/1.19.2/aws_smithy_runtime_api/client/interceptors/trait.Intercept.html)。
- **raw-event 语义。** Pi callback 收 SDK union item；本地 README 和 `bedrock/mod.rs:985` 的 raw observer 使用 `{type, ...payload}`。正式兼容层需明确是 provider SDK event、wire frame 还是规范化事件，不能把它们混称 raw。若要求保留未知 JSON 字段，需检查生成反序列化是否丢弃；本次未证明它能保留。
- **错误、重试与部分输出。** Pi `:304–326` 只在特定 ValidationException/strict-tool 条件且未取消时走 strict fallback；`:403` 起专门映射错误名称供上层判断；stream exception 与请求失败是两种边界。本地 callback failure 为不可重试的 CallbackPhase。要避免 SDK acquisition retry 与 opi whole-call retry 相乘，并证明 response 被观察/开始输出以后不会重放。SDK 有 typed errors 不等于已有 Pi 错误分类。

推荐先评估完整官方 SDK 的服务、签名、eventstream 与凭据 provider 复用，opi 仅拥有 Pi message/option/event/error/生命周期适配。如果生成模型或回调拦截位置确实无法满足已经批准的 wire contract，再退到官方 `aws-sigv4` 与 `aws-smithy-eventstream` 复用，保留自己的调用编排；二者官方分别公开 HTTP/event stream 签名和 frame 编解码。[aws-sigv4](https://docs.rs/aws-sigv4/1.6.4/aws_sigv4/)、[aws-smithy-eventstream](https://docs.rs/aws-smithy-eventstream/0.61.6/aws_smithy_eventstream/)。不要因完整 SDK 某一层不适配就连签名算法、CRC/frame codec 一起复制。

## 成熟项目的实际测试布局

Cargo 官方定义：unit tests 与目标同 crate，可访问私有接口；integration tests 是链接 library 公共 API 的独立 target；每个根 tests 文件会编译为独立二进制，过多目标可考虑按主题子模块合并；共享 helper 可放 `tests/common/mod.rs`。没有“文件长就必须 integration”或“跨两个私有模块就必须公共”的规则。[Cargo layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)、[Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#tests)。

| 读取的真实案例 | 观察 | 对 opi 的启示 |
| --- | --- | --- |
| reqwest `src/proxy.rs:809` 与 `tests/proxy.rs`、`tests/upgrade.rs` | 私有模块有 cfg(test) tests；公共 Client 由本地服务器验证 URI/headers/auth/upgrade；proxy env 测试用锁串行保护 | 私有解析与真实传输验收互补；环境变量测试需隔离，优先注入环境而非全部全局锁 |
| Tokio `tokio/src/sync/tests/loom_oneshot.rs` 与 `tokio/tests/sync_oneshot.rs` | 内部 Loom 遍历 send/close/receive 交错；公共测试直接检查 Pending、wake、drop、Send trait | 自有资源 owner/缓存竞态才值得针对性模型测试；不要在 opi 重写测试 Tokio channel 自身 |
| serde_json `tests/compiletest.rs`、`tests/lexical.rs` | trybuild compile-fail 单列；lexical 目标用多个子模块，甚至 `#[path = "../src/lexical/mod.rs"]` 导入源码 | 成熟项目没有统一神圣目录；不能照搬特殊源码复制路径，它不证明打包后的公共 API 正确。opi 当前 tests README 明确禁止用此法暴露私有机制，应保留该边界 |
| tungstenite `fuzz/Cargo.toml` | 独立 fuzz workspace 有 parse_frame_header、read_message_server、read_message_client targets | fuzz 应按解析入口和角色归属；若 opi 保留 codec，就承担自己的恶意输入面；复用后重点 fuzz 自有适配和状态交接 |

固定源码： [reqwest 私有测试](https://github.com/seanmonstar/reqwest/blob/8e1c8061e5dd936762dcef08a64ef2cff584b295/src/proxy.rs#L809)、[reqwest proxy](https://github.com/seanmonstar/reqwest/blob/8e1c8061e5dd936762dcef08a64ef2cff584b295/tests/proxy.rs)、[reqwest upgrade](https://github.com/seanmonstar/reqwest/blob/8e1c8061e5dd936762dcef08a64ef2cff584b295/tests/upgrade.rs)、[Tokio Loom](https://github.com/tokio-rs/tokio/blob/695a07dd5e5baae82eb8b6931afa61029cd9b854/tokio/src/sync/tests/loom_oneshot.rs)、[Tokio 公共测试](https://github.com/tokio-rs/tokio/blob/695a07dd5e5baae82eb8b6931afa61029cd9b854/tokio/tests/sync_oneshot.rs)、[serde compiletest](https://github.com/serde-rs/json/blob/8c535a260ce561eaab4111aafc29038eb9f77ff3/tests/compiletest.rs)、[serde lexical](https://github.com/serde-rs/json/blob/8c535a260ce561eaab4111aafc29038eb9f77ff3/tests/lexical.rs)、[tungstenite fuzz targets](https://github.com/snapview/tungstenite-rs/blob/2f20dc53e2a8d09bba02970d7ba15c5f33fbc17a/fuzz/Cargo.toml)。

## 建议测试归属与去重标准

| 层 | opi 应拥有的证据 | 不应据此宣称 |
| --- | --- | --- |
| crate 内 unit | 自己的 message/option/event/error 映射、parser 增量边界、callback freeze、资源状态机与取消清理 | 一个私有 parser 通过就等于整个 provider 通过 |
| 公共 integration | 从真实公共入口走真实 adapter 到本地 HTTP/WS peer；请求字节、回调顺序、错误/取消/代理/TLS、部分输出、continuation | stub provider success 等于真实生产路径 |
| conformance | 固定 Pi 1.5.0 oracle/fixtures，经过同一用例输入与行为输出比较；声明有意差异；独立 consumer 从打包 crate 接入 | Rust 与自己生成的 fixture 相同就等于 Pi 相同 |
| fuzz/property | 自有不可信输入 parser、适配边界、事件序列、取消/恢复状态；发现样本晋升确定性回归 | 上游 fuzz 存在就覆盖 opi 配置与 wrapper |
| doc-test/examples | 外部接入语法、错误处理、stream/callback 生命周期；离线例子运行，真实网络例子 no_run 编译 | no_run 能证明网络调用或 provider 成功 |
| live smoke（另获授权） | 选定平台/provider 的真实鉴权、服务版本与互通 | 几次 live success 覆盖全部失败/取消/重试语义 |

rustdoc 的 `no_run` 会编译而不执行，`ignore` 连编译保证也放弃，`compile_fail` 可证明被拒绝的用法；README 可通过 `include_str!` 纳入 doctest。建议依此建立独立库接入例子，而非把长篇设计历史保留为用户文档。[官方 doctest](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)。

删除测试的依据应是相同契约、相同边界、相同反例由新 owner 完整覆盖，并能追到替代测试；不能只看测试名相似、assert 重复、上游已有测试。保留低层精确反例和高层“真实公共调用确实到达该机制”的少量组装测试通常合理。用统一 fixture helper 消除生成器重复，避免把断言和实现共享成同一 oracle。

## 方案下一步与未验证项

待“确定独立接入与行为对齐的公共契约”“确定测试归属、差分证据与去重规则”“确定验收标准的分层与发布判定”确定公共兼容与验收等级后，建议安排两个有边界的验证工作包（本轮不实施）：(1) 选定 WebSocket 候选，跑 proxy+TLS+upgrade、是否协商 PMD、取消发送/关闭、Codex fallback 与 reuse 的最小反例；(2) 用 AWS 官方 SDK 走本地可控 transport，验证 profile 优先、bearer/dummy auth、gateway headers、async hooks、unknown event 字段、strict fallback 与 retry owner。每包交付通过/失败矩阵和可删除的本地责任清单，再决定依赖迁移。

本次没有编译候选依赖，没有全平台构建、性能/二进制体积测量、真实 AWS 调用、完整 credential provider 逐项核验，也没有证明其他 PMD 库可替代。当前依赖版本和 MSRV 是一手 manifest 事实，不构成完整 feature graph/MSRV 通过证明。源码阅读发现旧 README 有历史残留句（例如“codec no production caller yet”后文又说 proof exists），因此本报告以实际源码与可观察契约为判断入口，不拿文档中的完成宣称充当运行证据。

测试影响：`none`（研究与规划，无测试增删改）。仓库写入：`none`；本文件由研究代理写到 Codex 工作目录，根代理负责合并到唯一决策票。

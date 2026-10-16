# Pi 1.3.0 WS 最小离线差分结果

候选结论：**TW/TTW 0.28.0 不能原样保留为已定案的完整 WS 后端**。实际参考构造器支持 PMD，而该 codec 不具备该能力；握手也存在稳定接受集差分。需要继续验证具备压缩能力的受维护后端／有限上游改动，尚未选择替代库。仅自有握手接 `from_partially_read` 不能补齐 PMD。WS 组继续 **OPEN**，不是产品验收通过。

## 固定来源与层次

Pi snapshot 1.3.0 tree `08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2`。执行层为 Pi 源码相同的 `new globalThis.WebSocket(url,{headers})` 调用形状、标准单参数构造器，以及直接 TTW `client_async`；未运行 Pi 完整公开 provider 或 opi 公开接口。

- Node v24.13.0，`D:/nvm/v24.13.0/node.exe` SHA256 `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`；实际 global 为内置 Undici 7.18.2，不是 Pi npm lock 的其他 undici。Pi package 只声明 Node >=22.19.0，这里选择一个精确实验 runtime，不泛化为所有 Pi runtime。
- Bun **1.3.5+1e86cebd7**，`D:/nvm/node_global/node_modules/bun/bin/bun.exe` SHA256 `0025a8bc31f136745d102212ddb192a1876c721b71deeb43123d18375fcf030c`。`process.version=v24.3.0` 是 Node 兼容值，不能冒充 Bun 版本。Bun probe 未执行 Pi 的专用 proxy wrapper。
- TW/TTW 0.28.0，从冻结发布归档完整安全解出、全部 60 个文件 member 逐字节核对，无源码改动；TEMP manifest 的 path patch 明示，Cargo.lock 固定 54 个 package。Rust 1.99.1 Windows MSVC。共享 target `E:/opi/cargo-targets/opi-8abcf38f9871653e`，没有会话 target、clean 或产品 Cargo 改动。`cargo-features.txt` 为真实图，未启 TLS。

## 实际结果

正式受控运行：60 个 Node/Bun constructor rows、15 个 Rust loopback rows、16 个确定性 supplied-reader rows、15 个 public partial-codec rows。正式 Bun 子进程全部 `--no-install`，只使用 node 内置模块，环境仅 SystemRoot/PATH/TEMP，服务只绑定 127.0.0.1；凭据和 header canary 均为实验假值。harness 的 1500/4000ms timeout 是实验 containment，不是 opi 默认预算。

| case | Node 24.13.0 | Bun 1.3.5 | TTW/TW 0.28.0 | 可下结论 |
|---|---|---|---|---|
| baseline 101+首帧同一次 server write | message，exit0 | message，exit0 | message，exit0；确定性 reader 也成功 | canary 路径与基本 tail 机制可用；server write 次数不是 read 次数 |
| Pi 形状 headers | fake Authorization/canary 均到达 | 同左 | fake headers 均到达 | 此 Node build 的第二参数 overload 实际可用；标准单参数均无 canary |
| Connection `keep-alive, Upgrade` | 握手前拒绝 | 握手前拒绝 | `MissingConnectionUpgradeHeader` | 此 pinned 参考也拒绝，不能单凭 native 拒绝断言对齐差分；其他 runtime 未证 |
| total 124 headers | 已输出 message，之后进程 `3221226505` | message，exit0 | message，exit0 | Node 是 delivered-then-process-failed，不能称稳定通过 |
| total 125 headers | 已输出 message，之后同类崩溃 | **message，exit0** | **TooManyHeaders** | Bun 与 native 稳定接受集差分；无须依赖 Node 崩溃样例 |
| total 140 headers | 已输出 message，之后同类崩溃 | 握手前拒绝 | TooManyHeaders | 不泛化 Bun header count 上限；只报告已测点 |
| header value 16000/20000/66000 | 16000 成功；20000/66000 拒绝 | 三者成功 | 16000/20000 成功；66000 AttackAttempt | 只固定已测点，不推导 Node 精确默认上限；66000 对 Bun 存在 native 额外限制 |
| text tail 70000 | 成功 | 成功 | 成功 | 未证明单纯 70000 尾帧会触发握手 attack budget；总 reader 计数含 frame 阶段 |
| invalid Accept | 拒绝 | **仍交付 message** | SecWebSocketAcceptKeyMismatch | Bun 意外宽松不是 Rust 必须复制的合同；保留 fail-closed |
| unrequested `x-unrequested` extension | 拒绝 | 仍交付 message | **仍交付 message** | native extension 验证缺口实际可达；应 fail-closed，Bun 宽松不消除问题 |
| unsolicited subprotocol | 进程 exit1，内部 `includes(null)` TypeError，无 JSON 结果 | 拒绝 | ServerSentSubProtocolNoneRequested | Node 意外崩溃不是正常 rejection 或可模仿合同 |
| PMD response + 普通 text | 自己 offer PMD，协商后成功 | 同左 | **自己未 offer**，仍接受 PMD response 并交付普通 text | native 对未协商 extension 的宽松需修复；不是成功协商压缩 |
| PMD response + RSV1 deflate text | 自己 offer，实际解压成功 | 同左 | 未 offer；frame NonZeroReservedBits | native 正确拒绝未协商压缩本身不是 bug。结合无 PMD feature/机制、partial codec 仍拒绝，证明该候选不能提供参考已证压缩能力 |
| RSV1 无 extension | open 后拒绝 frame，零 message | 同左 | NonZeroReservedBits | 正反例均保留 |
| supplied reader 每次 1 byte | 未套用该私有 reader | 未套用 | 第 65 个成功 read、65 bytes 即 AttackAttempt | 确定性机制证据；不能说 TCP 包数或已跑双端 SDK 同样切片 |

上述 Node header crash 共 6 行（两种构造形状×3），subprotocol crash 共 2 行；正式 constructor 60 行中 exit0=52、exit1=2、3221226505=6。原始 stderr 和每个子进程返回码均保留。第一个未显式 no-install 的纯 builtin WS run 也得到相同主要结果，作为历史诊断，不替代正式受控结果。

`from_partially_read` 在完全**跳过 HTTP 验证**后能读取 125 headers／66000 header 对应 tail；baseline 与 70000 case 第二次 next 为 EOF 协议错误而非重复 text。它只证明接入机制和本样例 tail 一次，绝不证明完整 owned handshake：invalid Accept 也会“成功”正说明不能将跳过验证当方案。该入口依然拒绝压缩帧。>512 read 独立路径、coalesced tail 恰好跨 attack 阈值等没有新增运行证明。

## 准备、失败和副作用

首次 `cargo --offline` 因 data-encoding2.11.0 未缓存而 exit101；父任务随后仅为依赖准备获取 data-encoding2.11.0/utf-8 0.7.6 的精确官方归档并核 lock checksum，sha1 0.10.6 用已有归档。第一次 build 还发现旧 TTW 目录只选择性解出了源码，compat/handshake 缺失；完整解出原归档后 build --locked --offline exit0。两项都是环境／准备失败，不能算后端行为失败。

另有一次 Bun provider module import **没有 `--no-install`**：默认解析真实产生 workspace-root `node_modules/.bun-cache`，含 partial-json0.1.7。它的网络来源未被监听，不能保证来自已有离线缓存。父任务已把生成目录原封隔离到 TEMP，并记录 inventory/hash、移除 workspace 副作用。此诊断标为 **auto-resolution-uncontrolled**，不计受控离线或公开 provider 证明。历史命令和输出在 `runtime-and-provider-imports.json`；修正后的显式 no-install import 与 Node 均因缺 partial-json 报错，见 `public-provider-no-install-imports.json`。不能宣称整个任务完全未联网，或只有两项准备网络请求。

## 后续保留 OPEN

完整 Pi provider 的 WS/SSE fallback 与 wire、完整 PMD 参数／fragment/control/context takeover、owned handshake 的全部协议验证／取消／tail、连接缓存恢复与调度生命周期、产品 Cargo／proxy/TLS／native platform 都未完成。无新 SDK bytes/count/CPU/时长默认预算，无产品代码、Cargo、测试或 CI 实施。

## 重跑

用 `prepare-ws-source.py --archive-dir <frozen TW/TTW archive dir> --output <new TEMP scratch>` 复制自有 harness 并核归档。Cargo.lock 的 registry 依赖须已在 Cargo cache；脚本无下载器。设置 `CARGO_TARGET_DIR` 为既有外部共享 cache，执行 `cargo build --locked --offline --manifest-path <scratch>/rust/Cargo.toml`，再运行 `node <scratch>/run-ws-probe.mjs <exact-node> <exact-bun>`、`node <scratch>/run-ws-probe.mjs --rust <built-probe-exe>` 与 `<built-probe-exe> --synthetic <scratch>/synthetic-cases.json`。最后两步的 fixture 由首个 runner 从固定 case 定义生成；没有第三方 binary/source/target 被纳入持久资产。

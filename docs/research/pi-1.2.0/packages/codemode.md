# pi-codemode：显式工具能力与确定性 VM

分析日期：2026-10-16。研究本地 Pi 1.2.0，README、CHANGELOG 和 Cloudflare 示例说明全文阅读；源码重点覆盖共享 execution、worker/inline/remote launchers、VM 确定性与调用方。本次未安装、构建、执行脚本/测试、部署 Cloudflare 或调用 provider。[清单](../source-manifest.json)、[审读记录](../documentation-reading.json)。

## 职责、API 与真实使用

pi-codemode 在 QuickJS/WASM VM 运行模型写出的 async-function body，唯一外部能力是宿主注入的 tools/globals。没有 Node process/require、fetch、timers、modules 或 WebAssembly globals；eval/Function 仍只能生成同一 VM 里的代码。宿主函数在宿主执行，权限与副作用由注入方管理。库没有 Pi 内部依赖，唯一运行依赖 quickjs-wasi 3.6.2，ESM/Node >=22.19.0。[manifest](../../../../.repo/pi-1.2.0/packages/codemode/package.json#L1)、[README 环境](../../../../.repo/pi-1.2.0/packages/codemode/README.md#L49)。

| API/入口 | 职责 |
|---|---|
| `CodemodeSandbox` 根入口 | 每次 execution 一个 Node/Bun worker 与新 VM |
| `CodemodeSandboxBase` | 工具表、共享 execution/结果/取消；execute 与 deterministic start |
| `/portable` 的 Inline/Remote sandboxes | 无 Node imports，选择调用线程或 RPC 另一 isolate |
| `/cloudflare` | deployable CodemodeSandboxDurableObject 与预编译 WASM loader |
| `/source` | parseCodemodeSource、首行 options、Lark grammar |
| `/declarations` | JSON Schema→TypeScript/tool declarations；不执行参数验证 |
| `/worker` | 可供 bundler 单独构建的 worker entry，导入即运行 worker bridge |

根入口也 reexport inline/remote，但引入 Node worker/wasm loader；需要无 Node 环境时应以 portable 分支的源码边界理解，不可从全包“portable”推导根 entry 浏览器安全。[根 exports](../../../../.repo/pi-1.2.0/packages/codemode/src/index.ts#L1)、[portable](../../../../.repo/pi-1.2.0/packages/codemode/src/portable.ts#L1)。

真实使用有三条：coding-agent 内置 codemode extension 把 Agent tools/result rendering/policy 接入 sandbox；agent 的 mcp-codemode 示例把 MCP 工具映射进去并可通过 runToolCall 走 hooks；Durable 新 `/codemode` adapter 用 deterministic start 与任务日志恢复。直接调用工具 execute 会绕过 agent hooks，库不自动替调用者补齐这些 policy。[coding-agent execute](../../../../.repo/pi-1.2.0/packages/coding-agent/src/extensions/codemode/execute.ts#L1)、[Agent 示例](../../../../.repo/pi-1.2.0/packages/agent/examples/mcp-codemode/tools.ts#L1)、[Durable adapter](../../../../.repo/pi-1.2.0/packages/durable/src/codemode/codemode.ts#L303)。

## execution 链与资源归属

```text
execute(code, options) → snapshot tool table + serialize store
  → shared Execution arms deadline/signal → launcher creates VM channel
  → VM prelude tool promise → host handles/awaits injected execute
  → JSON result → VM resumes → output/done
  → first outcome wins → abort pending calls → stop channel → resolve result
```

CodemodeSandboxBase 保存 tools/globals/defaults 和 running executions；每次 execute 新 VM，绝不自行持久化 store。tools 原名也有安全 identifier alias，ALL_TOOLS 只有 name/description。tool args/result 经 JSON round-trip；throw 变脚本 Error。globals 可 namespace/spread、自定义 declaration，不进入 result.calls。`text`/console/image 构造有序 output；image 只接受合法 base64 data 或 image block，依据签名识别 PNG/JPEG/GIF/WebP。脚本 return、exit、parse/runtime error、deadline、abort、transport crash 竞争同一个 terminal outcome。[共享执行](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L188)、[tool bridge](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L380)。

Node launcher 加载/编译缓存 WASM，再建 worker；SharedArrayBuffer interrupt flag 配合 worker.terminate，覆盖 Bun 无法立刻杀掉 spinning wasm 的情况；worker error/exit 归 sandbox error。Node --watch 的 watch:import/watch:require 消息被忽略。关闭 sandbox 拒绝新执行，abort 所有正在执行的 VM。tool 的 AbortSignal 会置位，但执行器不 await 这些 host tools 自己返回，忽略取消的工具副作用可能继续；这与 VM 已停止是两件事。[worker launcher](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/host.ts#L12)、[finish](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L410)、[close](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L569)。

结果错误分 script/timeout/aborted/sandbox，script 失败以 `{ok:false}` 返回，不抛到宿主；调用已 close sandbox 等 API misuse 仍可 reject/throw。output 失败时也保留，storeWrites 只成功时返回。返回时尚 pending 的 calls 为 cancelled，late replies 不改变 outcome。[结果收敛](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L410)、[README results](../../../../.repo/pi-1.2.0/packages/codemode/README.md#L327)。

## 边界、并发、背压与限额

execute 默认 timeout 300000 ms；Infinity 关闭 deadline，VM 堆 limit 与 process 总内存不同。无 pending host calls 却等待永远不会 resolve 的 promise 会失败。输出是完成后集中返回的内存列表：16 Mi characters（含 base64）、100000 items；超过后即使脚本 catch RangeError 仍失败。store 单值 256 Ki JSON characters、总 1 Mi characters，load 返回副本、store(undefined) 删除，失败无 writes。字符单位不是 bytes/tokens。[默认与 finish](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L49)、[prelude constants](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/prelude-source.ts#L1)、[store 说明](../../../../.repo/pi-1.2.0/packages/codemode/README.md#L62)。

宿主没有每脚本 tool concurrency、args/result 总字节或 call 数量统一预算。脚本 Promise.all 可以同时产生许多外部调用；result.calls 持有全部工具记录到 execution 结束。remote output 以 1000 items/1 Mi chars 分批 exchange，避免每行一次 RPC，这只是批处理而非统一 admission/backpressure。[pending/calls](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L192)、[remote flush](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/remote.ts#L88)。

## 跨运行时执行与确定性恢复契约

Inline launcher 同线程，CPU 计算期间 event loop timer/abort 不运行；QuickJS interrupt polls 的总预算才是该时段的硬终止机制。默认 100000 polls，跨整个 execution（含多次 tool waits）累积；stack guard 为 256 KiB。poll 次数不等于稳定时长，README 的约 2 s 本机/10 s Cloudflare 是作者 October 2026 测量，非本次基准。[inline.ts](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/inline.ts#L1)。

RemoteSandbox 保留 tools、calls、deadline、abort 在 caller，将 VM 放在另一个 isolate；remote.execute 一次 RPC，通过 exchange callbacks 返回 output/calls，再把 host result 送回。caller timeout/abort 可立刻结束本地 execution，但远端同步计算要到下一次 exchange 或 interrupt budget 才停。RPC 失败或未给 terminal message 就返回归 sandbox error。Cloudflare remote end 应是单独 Worker，session 相同 worker/dynamic worker 的隔离不足以避免阻塞；可以每 session 一个 object 或每 execution 一个。remote caps budget/heap，class 本身不使用 storage。[remote protocol](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/remote.ts#L1)、[server caps](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/remote.ts#L104)、[Cloudflare example](../../../../.repo/pi-1.2.0/packages/codemode/examples/cloudflare/README.md#L1)。

`sandbox.start()` 是新增 host-controlled deterministic run：声明自己的 tools/globals，host onCall 决定 effect，`deliver(seq,now,id,outcome)` 逐个编号交付。VM 虚拟时钟不倒退、seed 固定 randomness、UTC、去除 WeakRef/FinalizationRegistry；结果即使 transport 乱序也按 seq 应用。fingerprint 包括 quickjs/quickjs-wasi versions、prelude hash、deterministic version、stack/heap/budget；onIdle 给每次处理后累计 issued count。相同 start+seed+表+store+有序 results，且 fingerprint 相同才保证相同 calls/output。[start](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/execution.ts#L530)、[VM 设计与 fingerprint](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/vm.ts#L23)、[fingerprint build](../../../../.repo/pi-1.2.0/packages/codemode/src/runtime/vm.ts#L316)。

库提供 deterministic replay 的机制，不提供 journal、持久 store、外部 exactly-once、recovery admission 或 fingerprint compatibility policy；Durable 的 adapter 才在交付前持久化日志并检查 divergence。`start()` 默认无 deadline，宿主需提供自己的 policy。这是职责分工，不能把 start 本身称作持久执行。[README deterministic](../../../../.repo/pi-1.2.0/packages/codemode/README.md#L296)、[Durable journal](../../../../.repo/pi-1.2.0/packages/durable/src/codemode/codemode.ts#L956)。

source options 的 max_output_tokens/timeout_ms 只由 parser 提取，sandbox 不执行它们；caller 选择是否应用。解析严格限制 options 必须第一行、字段已知、有效 JSON、非空代码；空白行后 options 也拒绝。schema declarations 只是提示文本，runtime validation 必须调用者提供。[source.ts](../../../../.repo/pi-1.2.0/packages/codemode/src/source.ts#L1)、[README declarations](../../../../.repo/pi-1.2.0/packages/codemode/README.md#L232)。

## 设计资料、测试与静态风险

三份文本均本次全文阅读。未发现独立规划/handoff。README 的 Cloudflare placement/interrupt rationale 已落实为 Inline/Remote 与独立 Worker 示例；“工具是唯一能力”和“store 不自行持久化”为现行边界；deterministic section 已落实为 start/VM protocol。1.2.0 删除 MCP_TYPESCRIPT_PREAMBLE/mcpStructuredContentSchema 和 MCP output schema 特判，不再把 arbitrary output schema 当 CallToolResult<T>；调用方须按自己的 schema 表述。没有复用旧版仅 Node worker 的结论。[CHANGELOG](../../../../.repo/pi-1.2.0/packages/codemode/CHANGELOG.md#L3)。

测试预期包括冻结 builtins/畸形 bridge、timeouts/microtask spin/heap、source options、declaration schema、Inline/Remote、deterministic time/random/UTC/seq/fingerprint/issued counts。`portable.test` 检查 remote caps、batching/abort/failure；`deterministic.test` 对 worker 与 remote 两个 backend 作一致调用结果与乱序交付，remote 的测试端内部使用 inline VM。未执行测试；不证明真实 Cloudflare/Bun 环境通过。测试影响 `none`。[portable tests](../../../../.repo/pi-1.2.0/packages/codemode/test/portable.test.ts#L37)、[deterministic tests](../../../../.repo/pi-1.2.0/packages/codemode/test/deterministic.test.ts#L108)。

分析者风险判断：trusted host 工具可读写任意授权资源，VM 的有限能力不能限制已注入工具；host tools 忽略 abort 可晚发生 effect；工具高并发与大结果没有统一预算；inline/remote 计算停止有不同延迟，不能按 Node 的 wall deadline 推断；fingerprint 不同必须由持久宿主处理，升级不保证可无缝继续旧脚本。quickjs-wasi 内部与其二进制未做源码审计。

## 1.0.4 → 1.2.0

清单对比新增 14、修改 16、删除 0、相同 3；当前 33 文件、17 生产 TS/3014 行。共享 Execution 从 worker 宿主抽出，新增 VM、inline/remote/Cloudflare、portable、deterministic start 与测试；1.1 加 console 标记，1.2 加 schema constraint tags、首行 options 校验、Node watch 噪声处理、promise-jobs interrupt，并移除 MCP 专用 declaration helpers。quickjs-wasi 版本未变，宿主路径与调用者仍有重大变化。旧报告仅导航；比较是哈希差异与当前源码复核，不声称完整旧新逐行 diff。

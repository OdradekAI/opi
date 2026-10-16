# mcp：独立客户端、传输恢复与 OAuth 状态

分析日期：2026-10-16。本报告分析 Pi 1.2.0 当前源码与文本，测试仅静态审读；没有安装、构建、执行测试、启动 server 或访问网络/provider。测试影响 `none`。

## 职责、发布与真实接线

`@earendil-works/pi-mcp` 是独立 MCP client，不依赖其他 Pi 包或官方 MCP SDK。生产依赖只有 `cross-spawn 7.0.6`，用于跨平台 stdio 进程启动。Node engine `>=22.19.0`；exports 为根、`./oauth`、`./testing`，带 source 条件，`sideEffects:false`。发布 dist/README/CHANGELOG/LICENSES。源码 18 个 TypeScript 文件、3277 行，5 个测试文件、1567 行。 [manifest](../../../../.repo/pi-1.2.0/packages/mcp/package.json#L1)、[公开导出](../../../../.repo/pi-1.2.0/packages/mcp/src/index.ts#L1)。

结构分为 transport-neutral client（correlation、初始化、超时、取消、server requests）、`protocol`（JSON-RPC、工具/资源/内容类型）、`transports`（stdio、Streamable HTTP、in-memory）、OAuth discovery/flow/provider/callback，以及 testing pair。工具与模型适配是应用层组合：README 的 AgentTool wrapper 明确转交 signal、内容与 isError，agent 的 mcp-codemode 示例还可以通过 runToolCall 复用 hooks；本包不调度 agent、不决定工具权限、不打开浏览器、不选择持久 credential 文件。 [README 组合边界](../../../../.repo/pi-1.2.0/packages/mcp/README.md#L30)、[OAuth 存储边界](../../../../.repo/pi-1.2.0/packages/mcp/README.md#L110)。

## 协议与内容契约

默认协商版本仍为 `2025-11-25`，接受 `2025-06-18`、`2025-03-26`、`2024-11-05`。1.2.0 放宽 `structuredContent` 为任意 JSON（包括 array/null），是对较新内容规则的兼容，不是已把协商版本升到 2026。 [版本常量](../../../../.repo/pi-1.2.0/packages/mcp/src/protocol/types.ts#L4)、[内容类型](../../../../.repo/pi-1.2.0/packages/mcp/src/protocol/content.ts#L65)、[升级说明](../../../../.repo/pi-1.2.0/packages/mcp/CHANGELOG.md#L3)。

client 提供 generic request/notify、ping、分页 tools/list、tools/call、resources/list 与 templates/list 的整表/单页访问及 resources/read；也可设置 server request handler 与通用通知 listener。默认 server ping/roots/list 可用，roots 是静态数组或异步函数。分页每页独立 request timeout，最多 1000 页并检测重复 cursor；空串/null cursor 归一为结束；资源缺 name 时用 URI/uriTemplate 代替。 [helpers](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L93)、[public methods](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L292)、[listAll](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L356)。

`callTool` 的 result 外壳验证较窄：要求 object，content 若存在必须 array，省略时补 `[]`，不做全部 content block schema 或 structuredContent 深度校验。JSON transport 本身限制输入为 JSON；类型 unknown 并不自动验证应用手写对象。`toLlmContent` 保留 text/image，unwrap embedded text/image resource，audio/链接/其他 binary 转文本占位；只有无 content block 且 structuredContent 非 undefined 时才序列化它，null 会成为 `"null"`。annotations/_meta 不进入模型内容。MCP 工具级 `isError` 与 JSON-RPC error 是两种错误通道。 [result 验证](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L144)、[toLlmContent](../../../../.repo/pi-1.2.0/packages/mcp/src/protocol/content.ts#L79)。

## Client 生命周期与请求取消

状态为 `idle → connecting → connected → closed`，没有 closed→reconnect 路径；重连需新建 client/transport。connect 先安装 transport listeners 并 start，再发 initialize、校验 serverInfo/capabilities/版本，向 transport 设置协商版本、发送 initialized，最后进入 connected。失败 best-effort close。connect/initialize 不能按 MCP 通知取消，但仍有本地 timeout。 [state](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L38)、[connect](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L204)。

每个 request 分配 id、pending entry、timer、可选 progressToken、外部 abort listener 与专用 sendController。默认 timeout 30,000ms；非有限或 `<=0` 关闭 timer。progress 只为带 onProgress 的请求关联，匹配通知会重新装同一个 timeout，所以是 inactivity timeout 而非总体 deadline；持续 progress 可无限延长。result、abort、timeout、send failure、close 任一路结算都会移除 pending/timer/listener，并 abort sendController。除 initialize 外，取消/超时还 best-effort 发送 notifications/cancelled；这只是通知，不证明服务器已经撤销副作用。 [requestInternal](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L396)、[progress/timeout](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L536)、[结算](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L561)。

1.2.0 的 send signal 契约专门封闭“请求已超时/取消，OAuth refresh 后却重新 POST”窗口：transport 不得在 sender 已结算后再派发或重试。它不撤回已经送出的请求，也不要求停止共享 auth recovery。HTTP 在等待 token 后、每次 fetch 前检查该 signal；stdio 没有等待鉴权/重试阶段，直接写 stdin。 [transport send options](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/transport.ts#L9)、[HTTP dispatch check](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L287)。

server requests 使用各自 AbortController，notifications/cancelled 可取消 handler；close abort 所有 incoming 并拒绝所有 pending，onClose 只发一次。transport error 默认只报告，直到 transport close 才统一拒绝 pending；未知 response id 报 error。通知 listener 的 throw 被转交 error listener，但 error listener 本身没有统一 try/catch 隔离，应用应保证其不抛出。 [server handler](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L492)、[close/error](../../../../.repo/pi-1.2.0/packages/mcp/src/client.ts#L592)。

## stdio：进程所有权、framing 与清理

StdioTransport 用 cross-spawn 运行配置的 command/args/cwd，默认继承环境并覆盖 env；`inheritEnv:false` 可提供独立环境。stdin/stdout 是 newline JSON-RPC，stdin.write callback 代表写入结算。接收端按 byte buffer 找 LF、支持 CRLF/空行，默认每条 16MiB，超限丢该记录或清空未终止缓冲并报告 error；默认只保留最后 64KiB stderr，可选 inherit 或 onStderr。它没有内容级权限模型、出站大小限制或 provider 语义。 [start/send](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/stdio.ts#L90)、[输入限制](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/stdio.ts#L181)、[共享上限](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/transport.ts#L3)。

关闭先 end stdin，等待最多 `min(500ms,closeTimeoutMs)` grace 后 SIGTERM，再过 closeTimeoutMs（默认 2000ms）SIGKILL，直到 child close。POSIX 使用独立 process group 与 host exit hook；Windows 使用 `taskkill /T /F` 隐藏窗口终止进程树，故无法按 POSIX 的 graceful signal 逐级兑现。README 的“whole process group”是跨平台意图，具体机制不同。 [killProcessTree](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/stdio.ts#L17)、[close](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/stdio.ts#L152)。

## Streamable HTTP：会话、重连与真实上限

start 仅标记生命周期，不立即联网。send POST JSON-RPC，携 accept/content-type，附 configured headers、session id、protocol version 与 Bearer token。request 的 JSON body 回复被逐条 parse，SSE body 交异步 consumer；notification/response 的回复 body 被丢弃。initialize 得到的 Mcp-Session-Id 被保留，initialized 后默认启动 server-to-client GET；405 表示无 GET stream。session 404 变成 McpSessionExpiredError，没有透明重新 initialize。 [send](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L221)、[headers/session](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L319)。

SSE parser 支持 chunk、CRLF、comment、multiline data、id/retry，包括仅 id 的 resume priming；默认 event data 与未终止行上限 16MiB。response stream 在尚未收到该 request response 且已有 event id 时以 GET+Last-Event-ID 恢复；无 cursor 或恢复耗尽，只把该请求变成 protocol error。GET stream 自己重连，默认 1000ms 指数增长到 30,000ms，连续失败最多 5 次；server retry 值优先且不受该指数上限钳制，收到事件或长时间健康连接可重置失败预算。只重连 dropped stream，不重新 POST 非幂等工具请求。 [parser](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L41)、[response resume](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L377)、[GET loop](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L422)。

可重试的是 408/429/5xx、TypeError 与部分网络 code；auth/session/protocol 错误不会按该策略恢复。401 或 insufficient_scope 403 仅交一次 auth recovery 后重试；fetch 调用不带 receiver，以兼容 worker 平台。close 先 abort transport controller，再用最后一次请求的 token 做最多 1000ms 的 best-effort DELETE，不调用可能 refresh 的 token()。 [authorizedFetch](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L268)、[retry classifier](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L478)。

限制必须区分：16MiB 是 stdio/SSE framing 上限，HTTP `response.json()` 无预读大小上限；错误体先完整 `response.text()` 再 slice 8KiB（字符切片，常量名不能证明只读取 8KiB）。pending 数量、整表 items 总量没有内存配额。请求 send signal 仅防派发，不传给已经开始的 fetch/response resume；本地 pending 结束后流任务可继续到 terminal/transport close。 [JSON 回复](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L254)、[错误体](../../../../.repo/pi-1.2.0/packages/mcp/src/transports/streamable-http.ts#L339)。

## OAuth：发现、持久状态与安全边界

OAuth 代码明确改编自 MIT MCP TypeScript SDK v1.29.0，移除 SDK/Zod 依赖，用 WebCrypto PKCE；许可证完整保留。它是源码改编，不是运行时依赖 SDK。flow discovery 获取 protected resource 与 authorization-server metadata，验证 issuer；可配置 metadata URL 明确信任该配置，不缓存此 override。安全 endpoint 要求 HTTPS（loopback 例外）。 [来源](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L1)、[license](../../../../.repo/pi-1.2.0/packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt#L1)、[discovery](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/discovery.ts#L58)。

runFlow 先载入 discovery/有效 resource/scope，再选 configured/stored client information 或 CIMD；没有 client 时 DCR，自动推导 native/web application_type（显式配置优先）。CIMD 只作为 client_id 与 redirect URI，不存成注册信息。code exchange 必须先核 `iss`：存在 iss 却没有 metadata issuer 也拒绝；server 要求 iss 而响应缺失同样拒绝。该 1.2.0 fail-closed 修复发生在 token exchange 之前。 [client 与 issuer](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L320)、[DCR application_type](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L246)。

有 refresh token 时尝试刷新；取消或 insecure endpoint 不降级为新登录，部分 OAuth grant/client 错误由 authorizeMcp 失效对应状态后最多重跑一次。`signal` 贯穿 discovery、registration、token 请求，没有默认请求 deadline，需要调用者合成 timeout。`adaptOAuthProvider` 以一个 inFlight promise 合并并发 401；若被拒 token 已被另一请求替换，直接复用新 token。step-up 跳过 refresh，把已获 scope 与新 challenge scope 合并后重新授权。该共享 recovery 故意不绑定单个 tools/call 的取消 signal。 [refresh/retry](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L401)、[adapter](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/flow.ts#L446)。

默认 McpOAuthProvider 用可注入 state store，内存实现 structuredClone；状态包含 exact serverUrl、client、tokens、expiresAt、verifier、state 与 discovery。所有 update 通过 promise chain 串行，别的 server URL 的存量状态被忽略；这不是跨进程事务锁。tokensExpireAt 被记录但 token() 直接返回 access token，常规 refresh 在 401 recovery 触发。存储写链若 rejected，后续 update 没有 catch 恢复（分析者推断：同一 provider 实例后续写也会被先前失败阻断）。 [provider state](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/provider.ts#L10)、[写入与 URL 绑定](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/provider.ts#L152)。

callback server 默认 127.0.0.1 随机端口、/callback、5 分钟；按 state 和可选 expected path 匹配，转交 code/state/iss；close 拒绝 pending 并关闭 listener。callback 验 state/path，authorizeMcp 验 issuer，二者不是彼此替代。README 示例只解构 code，没有把 callback.iss 传给 authorizeMcp，也没有在示例结尾关闭 callback，因此不能把示例当完整的 issuer/资源清理流程。 [callback](../../../../.repo/pi-1.2.0/packages/mcp/src/oauth/callback.ts#L64)、[README 示例](../../../../.repo/pi-1.2.0/packages/mcp/README.md#L94)。

## 测试预期、文档时效与升级

本次静态核对 client 初始化/分页/resource/structuredContent、超时续期、取消，HTTP auth recovery 后取消与 close token，以及 OAuth issuer/abort 新增用例；没有执行。测试使用 local HTTP fixture、in-memory pair 与子进程 fixture，不能由此宣称实际第三方 server 互通已验收。 [client tests](../../../../.repo/pi-1.2.0/packages/mcp/test/client.test.ts#L70)、[取消 recovery 测试](../../../../.repo/pi-1.2.0/packages/mcp/test/streamable-http.test.ts#L346)、[issuer 与 abort 测试](../../../../.repo/pi-1.2.0/packages/mcp/test/oauth.test.ts#L491)、[stdio fixture](../../../../.repo/pi-1.2.0/packages/mcp/test/stdio.test.ts#L9)。

README 1–136、CHANGELOG 1–73 与 license 1–21 均全文阅读。没有独立路线图；重要主张核查如下：

| 文档方向 | 当前判定 |
|---|---|
| 独立小 client、不引 SDK、不接管 browser/存储 | 已兑现，模块依赖与注入接口一致 |
| session、GET reconnect、response Last-Event-ID、progress/cancellation | 已兑现，限制是流恢复而非请求 exactly-once 或自动新 session |
| PKCE/DCR/refresh/step-up | 已兑现，含 shared refresh 与 scope union；README 对 CIMD/issuer/path 的覆盖不足 |
| batch、legacy HTTP+SSE、server、sampling、tasks 不属 initial core | 非目标；generic handler 可扩展但不等于内建完整支持。HTTP 接受 response JSON array 是容忍路径，不等于 batch request API |
| supported surface 主要列 tools | 局部文档不全；当前 resources/list/templates/read 已存在，不是待实现规划 |

相较 1.0.4 有效旧 manifest，当前本包的主要升级为：1.1.0 OAuth flow cancellation 与 close 复用 token；1.2.0 structuredContent unknown、issuer 无 metadata 时拒绝、request settlement 防止迟到派发。相关 exports/transport 接口/测试同步变化；旧源码目录缺失，比较证据为路径/摘要、旧报告导航与当前发行条目/实现，不声称完成旧新逐行 diff。历史 DCR/CIMD 已在 1.0.4 之前出现，不能重复列成新 1.2.0 功能。阅读顺序：protocol → client → transport contract/stdio/HTTP → oauth provider/flow/discovery/callback → 本地测试。

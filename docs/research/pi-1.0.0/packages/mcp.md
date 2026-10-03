# mcp：独立 MCP 客户端、stdio/HTTP 传输与 OAuth

## 范围与结论

本文以 `pi-1.0.0` 本地快照为一手来源，进行静态源码研究。
未执行测试、build、子进程、OAuth 浏览器流程或真实服务/provider 请求。
测试章节记录测试实现意图；外部 MCP 实现兼容性未在本次验证。

`@earendil-works/pi-mcp` 是可独立使用的 Model Context Protocol 客户端。
它实现 JSON-RPC 请求/通知、MCP initialize、工具与资源方法、stdio、Streamable HTTP 和可选 OAuth。
它不依赖 pi-protocol、pi-client、pi-server 或 AI/agent 包。
因此 MCP、稳定 coding-agent JSONL RPC、新 CBOR Session 服务协议是三个不同语义边界。
本包没有 MCP server 产品实现，也不自动把远端工具安装进 agent；后者由 coding-agent 扩展完成。
依据：[package.json:1](../../../../.repo/pi-1.0.0/packages/mcp/package.json#L1)、[index.ts:1](../../../../.repo/pi-1.0.0/packages/mcp/src/index.ts#L1)。

## Manifest、导出和源码来源

| 项目 | 事实 |
|---|---|
| 名称/版本 | `@earendil-works/pi-mcp` / `1.0.0` |
| 模块 | ESM，`sideEffects:false` |
| Node 下限 | `>=22.19.0` |
| 运行依赖 | 只有 `cross-spawn:7.0.6` |
| 根入口 | McpClient、JSON-RPC/MCP 类型、内容转换、stdio/HTTP、AuthProvider |
| `./oauth` | discovery、授权/刷新/注册、provider store、回调 server、错误 |
| `./testing` | 内存 transport pair |
| exports 条件 | source、types、import |
| 发布文件 | dist、README、CHANGELOG、LICENSES |

OAuth discovery/flow/types 明确标注从官方 TypeScript SDK v1.29.0 改编，去掉 SDK/Zod 依赖。
PKCE 使用 WebCrypto；授权服务器 issuer 校验是本地实现明确加强的行为。
相应许可证随包发布，不能将其误写成全量依赖官方 MCP SDK。
依据：[discovery.ts:1](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/discovery.ts#L1)、[flow.ts:1](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L1)。

## 文件地图

| 文件/目录 | 责任 |
|---|---|
| `src/client.ts` | initialize、pending、progress、cancel、server requests、分页 |
| `src/protocol/jsonrpc.ts` | wire shape guards 与错误类型 |
| `src/protocol/types.ts` | MCP 版本、能力、工具/资源 DTO |
| `src/protocol/content.ts` | content blocks 与 LLM text/image 转换 |
| `src/transports/transport.ts` | transport 生命周期和事件基类 |
| `src/transports/stdio.ts` | 子进程、JSONL、stderr、进程树关闭 |
| `src/transports/streamable-http.ts` | POST、GET SSE、session、恢复与认证回调 |
| `src/oauth/discovery.ts` | protected resource、OAuth/OIDC metadata、issuer/resource 检查 |
| `src/oauth/flow.ts` | PKCE、注册、token exchange/refresh、step-up 与 adapter |
| `src/oauth/provider.ts` | exact-server 状态、store 与写入串行化 |
| `src/oauth/callback.ts` | 默认 loopback 浏览器回调接收 |
| `test/*` | 内存、stdio fixture、本地 HTTP/OAuth server 测试 |

## Wire 及版本策略

最新声明版本是 `2025-11-25`。
还接受 `2025-06-18`、`2025-03-26`、`2024-11-05`，以支持服务器选回其较旧版本。
这只是快照支持集合，不是对当前网络规范版本的在线判断。
依据：[types.ts:4](../../../../.repo/pi-1.0.0/packages/mcp/src/protocol/types.ts#L4)。

JSON-RPC message 是三类联合：request `{jsonrpc:"2.0",id,method,params?}`、notification 无 id、response 为 result 或 error。
id 允许 string 或有限 number；不要求整数，null 不接受。
parseJsonRpcMessage 是结构验证器，没有 protocol 包那种 additionalProperties:false 的完全封闭 schema。
标准错误码为 parseError、invalidRequest、methodNotFound、invalidParams、internalError。
McpError 保留 code/data；另有 connection closed、timeout 和 AbortError。
依据：[jsonrpc.ts:1](../../../../.repo/pi-1.0.0/packages/mcp/src/protocol/jsonrpc.ts#L1)、[jsonrpc.ts:89](../../../../.repo/pi-1.0.0/packages/mcp/src/protocol/jsonrpc.ts#L89)。

McpTransport 的 send 接受已结构化 JsonRpcMessage，而不是裸 bytes。
传输实现拥有序列化与消息边界：stdio 是 newline-delimited JSON，HTTP 是 POST JSON 和 SSE JSON data。
transport errors 与 terminal close 分开，前者不必关闭客户端。
事件基类仅保证 emitClose 至多一次；其 error listener 回调没有普遍的异常隔离。
依据：[transport.ts:9](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/transport.ts#L9)。

## McpClient 公开接口

构造 options 包含 name/version/title、capabilities、首选 protocolVersion、requestTimeoutMs、roots。
options 外层冻结；不是所有嵌套配置都深冻结。
内置 request handler 有 ping；配置 roots 后增加 roots/list，roots 可为异步函数。

| API | 行为 |
|---|---|
| connect(transport) | start → initialize → initialized notification |
| request/notify | 泛型低层扩展方法 |
| setRequestHandler | 注册 server→client 请求处理器，返回注销函数 |
| onNotification/onError/onClose | 事件监听与注销 |
| ping | 空结果探测 |
| listTools | 读取所有工具页 |
| listResources/listResourceTemplates | 自动全部分页 |
| listResourcesPage/listResourceTemplatesPage | 单页控制 |
| readResource/callTool | 结果结构检查和兼容性归一化 |
| close | 关闭 client 与 transport |

connectionState、serverInfo、serverCapabilities、instructions、protocolVersion 可读取。
能力信息被保留，但 convenience methods 不自动阻止未声明能力的方法调用。
调用方可根据能力选择是否发 tools/list 等请求。
依据：[client.ts:171](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L171)、[client.ts:250](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L250)。

## Initialize 与对象状态机

状态为 idle → connecting → connected → closed。
connect 仅允许 idle，因此关闭后必须建立新 McpClient，不能对原对象 reconnect。
连接前安装三个 transport listener，再 await start。
initialize 包含版本、clientInfo、capabilities；有 roots 但无显式 roots capability 时补 `{}`。
验证结果基本结构、选中版本后保存 server metadata，并调用 transport.setProtocolVersion。
发送 `notifications/initialized` 成功后才公开 connected。
任意异常会 close 并重新抛出。
依据：[client.ts:202](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L202)。

```text
idle
  → install transport listeners
  → start transport
  → initialize RPC
  → validate supported protocol + save server info
  → transport.setProtocolVersion
  → notifications/initialized
  → connected
failure / close → closed（不可重用）
```

## 请求、超时、进度和取消

request 分配递增数字 id，注册 pending 后发送。
默认超时 30 秒，per-request 覆盖 client 默认。
非有限值或 <=0 使计时器不启用；源码没有更严格的 timer option 范围验证。
传入 onProgress 时在 params._meta 加 progressToken，并保留已有 metadata。
匹配的 progress 每次都会重新启动超时，因此超时是“最近进度后的空闲时间”，不是总执行时长上限。
依据：[client.ts:394](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L394)、[client.ts:531](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L531)。

AbortSignal 或 timeout 会删除 pending、progress 映射和 listener，再拒绝 Promise。
普通请求同时发 `notifications/cancelled`；initialize 明确不能发送取消通知。
send 失败拒绝该请求，不自动关闭 client。
取消后的迟到响应只 emitError “unknown request”，不会像 pi-client 那样视为连接致命错误。
单请求 signal 没有直接连接到每次 HTTP fetch 的 AbortSignal；远端停止依赖协议通知，整个 transport close 才 abort 共享 fetch controller。
依据：[client.ts:548](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L548)、[client.ts:476](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L476)。

服务器反向请求通过 setRequestHandler 分派，不支持的方法回复 -32601。
每个 incoming id 有 AbortController，收到 cancelled 或 client close 会 abort。
handler 正常返回 null/undefined 会用 `{}` 响应。
普通 handler 异常的 message 会作为 internalError 发送，不能假设像 pi-server 那样统一隐藏内部详情。
依据：[client.ts:487](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L487)。

transport error 仅报告；只有 transport close/显式 close 才拒绝全部 pending 并 abort incoming。
markClosed 保证 close listeners 一次；close 会先解除 transport listeners 再关 transport。
应用级 onError listener 本身抛错没有在 emitError 中捕获，需由注册者保证诊断不会再抛。
依据：[client.ts:586](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L586)。

## 工具、资源、分页和内容适配

listAll 最多 1,000 页，以 Set 拒绝重复 nextCursor。
null 或空字符串 cursor 按“结束”兼容；没有跨页去重条目，也没有总条目数限制。
每页复用请求 options，所以超时按单个请求计算。
tool 条目只硬性检查 name 和对象 inputSchema；不执行 JSON Schema 校验。
resource/template 缺 name 时分别回退 uri/uriTemplate。
依据：[client.ts:91](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L91)、[client.ts:354](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L354)。

readResource 检查 contents 数组，每项有 uri 且 text/blob 至少一个字符串。
callTool 允许只给 structuredContent，自动补 content:[]；structuredContent 要为对象。
content 中每个 block 不在此完整验证所有字段，保留未来/服务端扩展的空间，也意味着消费者仍需处理不完整内容。
工具 annotations 是返回的提示数据，本包不把 readOnlyHint 当强制授权规则。
依据：[client.ts:128](../../../../.repo/pi-1.0.0/packages/mcp/src/client.ts#L128)。

toLlmContent 输出只包含 text 与 base64 image：

- text/image 直接转对应 shape。
- embedded text/image resource 提取内容。
- audio、其他 binary resource 输出简短占位文本。
- resource_link 输出 `name: uri`，不自动下载。
- 无 content blocks 但有 structuredContent 时输出格式化 JSON 文本。

该 shape 与 AI 包匹配，但没有运行依赖；不承担资源读取权限、图像验证或媒体解码。
依据：[content.ts:72](../../../../.repo/pi-1.0.0/packages/mcp/src/protocol/content.ts#L72)。

## stdio：进程与 JSONL 生命周期

StdioTransport 支持 command/args/cwd/env、inheritEnv、stderr 策略、大小上限和关闭时间。
默认继承 process.env，再覆盖配置 env；`inheritEnv:false` 才只使用显式 env。
cross-spawn 处理跨平台可执行文件，Windows 隐藏窗口，Unix detached 建独立进程组。
因此启动 stdio MCP 是真实代码执行能力，不是解析一个纯数据文件。
依据：[stdio.ts:55](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L55)、[stdio.ts:90](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L90)。

stdout 以 Buffer 累积，按 LF 拆行，兼容末尾 CR，跳过空白行。
每行 JSON.parse 后 parseJsonRpcMessage；坏行 emitError，继续处理后续行。
默认单消息 16 MiB；完整超长行跳过，未遇 newline 的超长缓冲直接清空并报告。
退出时仍有非空残留会报告 incomplete message。
stderr 默认保存最后 64 KiB，也可 inherit；onStderr 可另行接收日志。
send 以 JSON.stringify + newline 写 stdin，等待 write callback；未设置对称的出站字节上限或 pending count 上限。
依据：[stdio.ts:143](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L143)、[stdio.ts:181](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L181)。

close 先关闭 stdin，最多 min(500ms,closeTimeoutMs) 后 SIGTERM，再过默认 2 秒 SIGKILL。
Unix 对负 pid 发送组信号，避免 npx/uvx 等 wrapper 留下后代；宿主 exit hook 也清理登记的进程组。
Windows 使用 taskkill `/T /F` 处理进程树，不能等同于 Unix 优雅信号语义。
stubborn-server 测试在 Windows 跳过，所以不可声称两平台树终止均受同一测试证明。
依据：[stdio.ts:17](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L17)、[stdio.ts:152](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/stdio.ts#L152)。

## Streamable HTTP：POST、session 与 GET

start 仅标记启用，不立即网络探测。
每次 send POST JSON，Accept 同时允许 application/json 与 text/event-stream。
成功响应捕获 Mcp-Session-Id；后续请求带 session header 和 initialize 协商后的 MCP-Protocol-Version。
notification/response 的 HTTP body 丢弃；initialized notification 成功后才启动后台 GET SSE。
请求若只获 202/204 而无 JSON-RPC 结果，立即报错，不无限等候。
JSON body 若是数组会逐条分派，否则作为单消息；SSE response 则交异步读取循环。
依据：[streamable-http.ts:223](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L223)。

HTTP 401 为 McpAuthRequiredError；已有 session 下的 404 为 McpSessionExpiredError；其他错误为 McpHttpError。
close abort 全部共享网络任务；若有 session，尝试带认证 DELETE，独立 1 秒截止，失败忽略后 emitClose。
本包不会在 session expired 后自动新建 McpClient/重新 initialize。
依据：[streamable-http.ts:256](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L256)、[streamable-http.ts:318](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L318)。

### SSE 解析和重连

consumeSseStream 支持分块 UTF-8、LF/CRLF、注释、event、id、retry 和多行 data。
事件 id 即使没有 data 也更新 cursor，支持 resumption priming。
含 NUL 的 id 忽略；retry 必须是十进制数字串。
空白行 dispatch，EOF 也处理残留事件；finally 释放 reader lock。
data 累计长度和未完成行都受默认 16 MiB 上限保护。
依据：[streamable-http.ts:35](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L35)。

request response SSE 与后台 GET SSE 有不同恢复范围：

- response stream 在收到匹配 response 后完成；未回答且已有 event id 时用 GET + Last-Event-ID 恢复。
- 无 event id 的断流只让原请求失败，通过合成 JSON-RPC internalError 回给客户端。
- GET stream 默认启动，405 表示服务端不提供它，直接结束且不关闭整个 client。
- 临时 HTTP 408/429/5xx、TypeError 网络异常或 E*/UND_ERR* code 可重试；认证、session、协议错误不重试。
- 默认首次 1 秒、指数增加到 30 秒、连续失败最多 5 次；server retry 字段直接覆盖 delay。
- 成功收到事件/足够长的健康 GET 流会重置连续失败计数。

这不是重新执行原始工具 POST，所以与盲目 replay mutation 不同。
后台 GET 最终失败只 emitError，其他请求仍可通过 POST 工作。
依据：[streamable-http.ts:361](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L361)、[streamable-http.ts:408](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L408)。

### HTTP 上限的真实边界

maxMessageBytes 用于 SSE event；普通 JSON 响应直接 response.json，并未先按该值有界读取。
HTTP 错误体是 response.text 完整读完后 slice 到 8 Ki 字符左右，error message 再裁成 500 字符。
因此这些常量不是网络读取峰值内存的硬上限；变量名 BYTES 也不改变 JS slice 按字符单位的事实。
OAuth metadata/token body 同样没有包内流式读取上限。
依据：[streamable-http.ts:243](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L243)、[streamable-http.ts:318](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L318)。

## 认证 adapter 与 OAuth 状态

HTTP AuthProvider 只有 token() 与可选 onUnauthorized(context)。
一次 GET/POST 遇 401 或 insufficient_scope 的 403 时，最多调用一次认证回调并重发一次。
context 包含被拒绝的 token，供并发刷新判断；原 response body 在 finally 丢弃。
任意 403 不会都触发登录。
依据：[auth-provider.ts:3](../../../../.repo/pi-1.0.0/packages/mcp/src/auth-provider.ts#L3)、[streamable-http.ts:281](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts#L281)。

McpOAuthProvider 为一个规范化后 exact server URL 保存 client info、tokens、expiresAt、verifier、state、discovery。
store 默认内存并 structuredClone；应用可注入持久 store，本包不写磁盘文件也不自带钥匙串。
载入其他 server URL 的状态会忽略，防止跨服务器复用凭据。
写入串行到 writes Promise，读先 await writes；一次 store save reject 会影响之后这条 Promise 链，没有独立恢复 catch。
保存 expiry 只是元数据，adaptOAuthProvider.token 不据此主动提前刷新。
依据：[provider.ts:42](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/provider.ts#L42)、[provider.ts:138](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/provider.ts#L138)。

## OAuth discovery、授权与信任边界

WWW-Authenticate parser 接受 Bearer/DPoP 起始 scheme，提取 resource_metadata、scope、error。
这不代表实现了 DPoP proof；HTTP adapter 实际发送 Bearer token。
protected-resource discovery 先试路径对应 well-known，4xx/502 时可退回 origin well-known。
授权服务器按 OAuth metadata、两种 OIDC 路径顺序查找。
默认校验 metadata issuer 与候选 issuer 相等，容忍末尾斜杠差异。
配置的 authorizationServerMetadataUrl 被显式视作可信覆盖，不套同样 issuer 比对。
依据：[discovery.ts:39](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/discovery.ts#L39)、[discovery.ts:102](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/discovery.ts#L102)。

resource 参数要求 metadata.resource 与 MCP URL 同 origin，且其 path 是请求路径的目录前缀。
discovery 可使用 cached metadata；配置 metadata URL 时故意绕过 cache。
未取到 protected-resource metadata 的某些错误会回退 origin discovery，TypeError 网络失败继续抛出。
安全判断不能仅凭一次 discovery 失败就推断整体拒绝连接。
依据：[discovery.ts:126](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/discovery.ts#L126)、[discovery.ts:172](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/discovery.ts#L172)。

runFlow 的主要分支：

1. discovery/cache → 选择 resource/scope → 获取 clientInformation。
2. 无 client：可用 client metadata document ID，否则动态注册并保存。
3. 有 authorizationCode：校验 RFC9207 iss（若 metadata 要求或实际提供），使用已存 PKCE verifier 换 token。
4. 否则有 refresh token 且未 skipRefresh：先刷新，保留未轮换的 refresh token 和省略的旧 scope。
5. 无可用刷新结果：生成 32 字节随机 verifier、SHA-256 S256 challenge，存 verifier 后 onRedirect。
6. 返回 AUTHORIZED 或 REDIRECT；授权浏览器行为由 provider 回调决定。

PKCE/response type 不受服务器支持时提前拒绝。
scope 包含 offline_access 时授权 URL 加 prompt=consent。
token endpoint 和配置 metadata URL 经过 HTTPS/loopback 检查；不能外推所有 discovery/registration/authorization URL 都执行相同检查。
特别是 registerClient/startAuthorization 未调用同一 secureEndpoint helper，URL parser 也只显式排除部分危险 scheme。
依据：[flow.ts:95](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L95)、[flow.ts:143](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L143)、[flow.ts:281](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L281)。

authorizeMcp 对 invalid_client/unauthorized_client 清 all，对 invalid_grant 清 tokens，再重跑一次。
adaptOAuthProvider 以 inFlight 合并并发 401 刷新；迟到的旧 token 401 若当前 token 已不同，直接让 HTTP 重试。
insufficient_scope 跳过 refresh，把已授予 scope 与 challenge scope 合并后走授权，保留旧 grant 直到新授权成功。
依据：[flow.ts:385](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L385)、[flow.ts:407](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/flow.ts#L407)。

OAuthCallbackServer 默认监听 127.0.0.1 随机端口、/callback、等待 5 分钟。
state 必须对应 pending entry；收到后一次性消费，缺 code 或 OAuth error 拒绝对应等待。
iss 传给调用者进一步验证，callback server 自己不进行 token exchange。
自定义 host/redirectHost/renderPage 都是受信任配置；HTML renderer 需自行正确转义消息。
依据：[callback.ts:61](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/callback.ts#L61)、[callback.ts:117](../../../../.repo/pi-1.0.0/packages/mcp/src/oauth/callback.ts#L117)。

## 实际产品集成：稳定扩展，而非新 Session 协议

coding-agent builtInExtensions 注册可替换的 mcp extension。
扩展读取配置、建立连接、注册工具/资源、处理用户登录和展示状态；本包只提供通用客户端能力。
runtime 根据 serverCapabilities 决定是否 tools/list/resources 查询，并监听 list_changed 刷新目录。
依据：[extensions/index.ts:7](../../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/index.ts#L7)、[runtime.ts:371](../../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/runtime.ts#L371)。

恢复策略位于该 runtime：只读调用遇暂时 HTTP 错误可重试一次，工具调用不因普通临时失败自动重放。
已建立 session 的 404 被解释为请求未执行，创建新 session 后允许一次重试。
transport close 后丢弃 client，下一次调用再连接；手动 reconnect 同样创建新 client。
这些行为不能归因于 McpClient 内置 reconnect API，因为不存在该 API。
依据：[runtime.ts:286](../../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/runtime.ts#L286)、[runtime.ts:431](../../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/mcp/runtime.ts#L431)。

## 测试矩阵：仅静态阅读

| 文件/位置 | 验证意图 |
|---|---|
| `client.test.ts:71` | initialize 完整顺序、metadata 与能力 |
| `client.test.ts:94` | tools 分页及 DTO 保留；resources 读写边界 |
| `client.test.ts:185` | progress 续时；abort/timeout 发取消 |
| `client.test.ts:228` | transport error 不终止 pending |
| `client.test.ts:251` | 较旧协议版本兼容、缺 content 默认值 |
| `client.test.ts:283` | initialize timeout 不发 cancelled |
| `stdio.test.ts:9` | Node fixture 完整 JSONL initialize/tools/stderr |
| `stdio.test.ts:30` | 非 Windows stubborn server 及后代终止 |
| `streamable-http.test.ts:92` | 分块 CRLF、多行 data、注释、ID、未终止事件上限 |
| `streamable-http.test.ts:158` | response stream 失败仅影响一个请求 |
| `streamable-http.test.ts:188` | initialized 后 GET，恢复才带 Last-Event-ID |
| `streamable-http.test.ts:208` | 带 event ID 的响应恢复和 GET 重连 |
| `streamable-http.test.ts:306` | 401/insufficient-scope 403 传 rejected token |
| `oauth.test.ts:94` | discovery、注册、PKCE、401 refresh |
| `oauth.test.ts:280` | rotating refresh token 的并发合并与迟到 401 |
| `oauth.test.ts:349` | scope step-up 合并旧权限且暂留旧 grant |
| `oauth.test.ts:388` | exact server URL 凭据隔离、issuer 拒绝 |
| `oauth.test.ts:469` | code exchange iss 匹配/缺失规则 |
| `content.test.ts:5` | 内容转换与 structuredContent fallback |

依据：[client.test.ts:185](../../../../.repo/pi-1.0.0/packages/mcp/test/client.test.ts#L185)、[stdio.test.ts:30](../../../../.repo/pi-1.0.0/packages/mcp/test/stdio.test.ts#L30)、[streamable-http.test.ts:188](../../../../.repo/pi-1.0.0/packages/mcp/test/streamable-http.test.ts#L188)、[oauth.test.ts:280](../../../../.repo/pi-1.0.0/packages/mcp/test/oauth.test.ts#L280)。

内存 transport pair 用 structuredClone + microtask 交付并联动关闭，用于不启动网络的 client 行为验证。
它没有 stdio/HTTP 的序列化约束，不应单独当跨 transport conformance 证明。
依据：[in-memory.ts:19](../../../../.repo/pi-1.0.0/packages/mcp/src/transports/in-memory.ts#L19)。

## 限制、待验证与阅读顺序

没有证据表明本次验证了任意第三方服务、所有版本特性、全部 OAuth 部署或 Windows 进程树清理。
generic request 可调用额外方法，不代表本包为 prompts、sampling、elicitation、tasks 都提供完整 convenience API。
没有工具 inputSchema/outputSchema 执行验证、远端权限决策、通用重放去重、HTTP 总响应上限或请求数配额。
SSE 恢复、HTTP session 重建、工具调用重试必须按其各自实现层区分。

阅读顺序建议：types/jsonrpc → McpClient initialize/request/cancel → stdio → HTTP send/SSE loops。
之后读 OAuth discovery/runFlow/provider/callback，最后对照测试与 coding-agent MCP runtime，确认策略归属。

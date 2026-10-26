# opi-ai

[![Crates.io](https://img.shields.io/crates/v/opi-ai.svg)](https://crates.io/crates/opi-ai)
[![Docs.rs](https://docs.rs/opi-ai/badge.svg)](https://docs.rs/opi-ai)

> [opi](https://github.com/OdradekAI/opi) 使用的 Provider 无关 LLM API。

[English](README.md) | [opi workspace](../../README.zh.md)

Rust 的 Provider 无关 LLM API：一个 `Request` 与流式事件模型，由内置 Provider
family、checked multi-wire provider 和配置驱动的 profile 共享。

```sh
cargo add opi-ai
```

## 当前状态

当前 crate 版本是 `0.8.3`，继承自 workspace 包版本。

`opi-ai` 负责模型/Provider 层：请求和消息类型、流式事件、模型元数据、Provider
注册、HTTP/代理连接、重试辅助、图片内容、用量累计、尽力而为的费用辅助，以及供
`opi-agent` 诊断层使用的 Provider 侧错误分类。它不实现 Agent 主循环、会话、
package 加载或内置编程工具；这些能力分别位于 `opi-agent` 和 `opi-coding-agent`。

`opi-ai` 还暴露一个 unstable-0.x 的模型/鉴权 seam：`models`
模块（`Models`）是唯一的 driver，在 `ProviderRegistry` 之上叠加 Provider 侧鉴权契约、
OpenAI-compatible 兼容性元数据，以及全部聊天调用入口。无 IO 的凭据与 OAuth
trait 位于本 crate；可复用的本地 AWS 凭据来源链（scoped/ambient 环境变量、
显式 SSO 与 login_session 段、共享配置 INI 叠加（assume-role 与 web identity）、
`credential_process`、以及远端 ECS/IMDS 来源段，经锁定的 STS Query/XML codec、
SSO OIDC 与 SignIn ES256 DPoP 流程、单飞来源缓存交换）位于 `auth::aws`；
ADC 来源与全部九种 OAuth 流由 SDK 拥有。coding agent 拥有 keychain backend、
终端 presenter 与宿主注册策略。

## Provider

| 模块 | Provider id | 后端 |
|------|-------------|------|
| `anthropic` | `anthropic` | Anthropic Messages streaming |
| `openai_chat` | `openai` | OpenAI Chat Completions streaming |
| `openai_responses` | `openai-responses` | OpenAI Responses streaming |
| `openai_codex_responses` | `openai-codex` | subscription-specific OpenAI Codex Responses streaming |
| `api_mapped` | 调用方定义 | 把一个 Provider identity/catalog 路由到 checked 具体 wire |
| `openrouter` | `openrouter` | OpenAI-compatible OpenRouter profile |
| `mistral` | `mistral` | OpenAI-compatible Mistral profile |
| `gemini` | `gemini` | Gemini `streamGenerateContent?alt=sse` |
| `bedrock` | `bedrock` | AWS Bedrock Converse streaming，支持 SigV4 或 Bearer 认证 |
| `azure_openai` | `azure` | Azure OpenAI deployment 专用 Chat Completions |
| `vertex` | `vertex` | Google Vertex AI Gemini streaming |

内置模型列表刻意保持有限，主要用于能力校验和模型列表输出。站点专用模型、
fine-tuned 模型和 deployment 应通过 registry override 或配置的
OpenAI-compatible profile 加入。

## 核心 API

| 项 | 作用 |
|----|------|
| `Provider` | 后端 trait，包含 `id`、`models` 和 `stream_prepared`（唯一分发入口，经 `Models::prepare_call` 携已解析认证调用）。 |
| `Request` / `CacheRetention` | Provider 请求：模型、消息、工具、token 限制、thinking 配置、metadata、取消、timeout、额外 header、cache retention 和会话亲和。 |
| `Message` | 面向 Provider 的 user、assistant 和 tool-result 消息。 |
| `InputContent` / `OutputContent` | 文本与图片内容块。 |
| `ToolResultMessage` | 工具结果 content、details、错误/截断/时间元数据，以及可信执行边界写入的可选 typed `not_executed` 事实；历史缺省表示效果未知，普通错误文本不能证明未执行。 |
| `AssistantStreamEvent` | Provider 无关流式事件，覆盖 start、text、thinking、tool call 与 done。带内 `Error` 终结符是只读的历史格式，仅为反序列化历史保存事件而保留；当前 producer 不再产生它。 |
| `ChatStream` | 一个 logical call 事件流的公开 owner：迭代为 `Stream<Item = Result<AssistantStreamEvent, CallFailure>>`，首个终结符被吸收为永久 EOF，`result(self)` 主动驱动到唯一最终结果。 |
| `CallFailure` | 一个 logical call 的唯一最终失败：规范化 partial/terminal `AssistantMessage`（内容、出处、诊断、已知 usage；setup 失败没有生成内容）加一个已脱敏的 typed cause。 |
| `drain_to_completion` | 复用 `ChatStream::result` seam 的收集器：done 终结符返回 `Ok(AssistantMessage)`，一切失败（含提前 EOF）恰好一次 `Err(CallFailure)`。 |
| `WireApi` / `ModelInfo` / `ModelCapabilities` | 精确请求 wire identity，以及模型能力、thinking、compatibility 与 pricing 元数据。 |
| `ApiMappedProvider` | 通过经校验的 `WireApi -> Provider` route map 派发一个公开 Provider identity/catalog。 |
| `ProviderError` / `ProviderErrorCategory` | Provider 失败分类：auth、config、request、network、rate_limit、provider、stream、capability 和 cancelled（超时归为 network）。 |
| `ProviderErrorSummary` | 封闭且可安全脱敏的 Provider error 文本。公共构造仅限 `redacted()`、`authentication_rejected()` 与 `from_untrusted(...)`；不会保留任意 Provider payload。 |
| `ProviderRegistry` | 解析 `provider:model`、注册自定义 Provider、叠加模型覆盖。 |
| `Models` / `AuthDescriptor` / `AuthCheck` | unstable-0.x 唯一 driver，位于 `ProviderRegistry` 之上：Provider+模型查找、脱敏鉴权状态、OpenAI-compatible 兼容性元数据、派发与原子动态目录 refresh。普通 native/simple/direct 入口与 advanced `prepare_call` 共用同一 compiler、鉴权解析、attempt seam 与 collector。 |
| `ChatApi` / `ApiId` / `ChatApiHandle` / `NativeOptions` | 类型化自定义 chat adapter 注册，携带精确 API 身份与固定 options 类型；由 handle 创建的 options 携带 API/类型标签，可在具备兼容注册的实例间移植。 |
| `ToolCall::arguments_error` | 文本 tool 参数的最终解析结果标记（AIR-048）：空输入归一为 `{}`，可修复的非法转义保留字面文本，不完整/非对象最终文本保留自有 raw 并携带该错误，绝不以 partial-prefix 成功执行；缺少该字段的历史记录仍可反序列化。 |
| `ChatModelRecord` / `SimpleOptions` / `ChatCall` | 调用方持有的完整 chat record（provider id、裸模型 id、精确 wire），无需目录成员即可调用；中性 simple options；构造时捕获输入、首次 poll 才启动鉴权的惰性普通调用 owner。 |
| `PreparedProviderCall` / `PreparedRoute` | `Models::prepare_call` 返回的不透明 logical-call 状态，以及公开的已脱敏 route facts。`route`、`auth_provenance` 与 `start_attempt` 是 prepared-call 生命周期 API。 |
| `CredentialStore` / `Credential` / `CredentialSource` | 无 IO、object-safe 的凭据持久化与已脱敏三态探测契约。 |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | 九种 SDK OAuth 家族的 boxed-future 契约、只读 flow 元数据、逐家请求身份转换与注入的登录 UX。 |
| `AuthResolver` / `ResolvedAuth` | 由 driver 按调用解析的鉴权契约；attempt 开始前会冻结一次准备结果。 |
| `ApiKind` | crate 根枚举，标注 assistant 消息携带的后端家族（`Anthropic`、`OpenAi`、`Google`、`Mistral`）。 |
| `HttpClient` | 共享 `reqwest` client，支持连接池和显式/环境变量代理。 |
| `ws` | 公开严格动态 WebSocket Upgrade 桥（`upgrade`、`WsUpgradeRequest`、`Negotiation`、`fresh_nonce`/`fresh_mask`）：请求头、nonce/mask 熵与严格的 101 校验。Frame/PMD/Close/IO 模块为私有；Upgrade 返回原始升级流与协商 metadata。 |
| `retry` | 采集重试唯一 owner（`AcquisitionPolicy`、`RetryAdmission`、`AcquisitionSeal`），以及产品侧重试配置、指数退避和 `Retry-After` 解析。 |
| `Usage` / `CumulativeUsage` | token 累计和费用辅助。 |
| `test_support::MockProvider` | 供下游测试使用的确定性 mock provider。 |

## 凭据、Request 扩充与 Refresh

`CredentialStore` 将 backend 错误与条目缺失区分开，`CredentialSource` 则在不暴露
秘密的前提下报告 present、absent 或 backend-unavailable 状态。
`AuthDescriptor::StoreCredential` 只携带 key 与 display source。
`OAuthProvider`、`LoginPresenter` 和 `AuthResolver` 是 object-safe boxed-future
seam。`auth::oauth` 提供 Anthropic、Codex legacy、ChatGPT direct、GitHub
Copilot、OpenRouter、Kimi Coding、Meta、xAI 与 Radius 流。`opi-coding-agent`
提供 OS-keychain store 和终端 presenter；SDK 流调用注入的 presenter，不启动浏览器。

`WireApi` 为每个 `ModelInfo` 指定一个精确请求 wire；公开
`ApiMappedProvider` 暴露一个 provider identity 与 catalog，并在派发前校验其
`WireApi -> Provider` route。一个 mapped provider 的所有 route 共享一个惰性
`AuthResolver`；provider/model 元数据会在网络 IO 前选择 route。未知 model、缺少
route 与 wire/compatibility 不匹配都会成为类型化、不可重试的失败。

GitHub Copilot 把一个静态 catalog 路由到 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses；OpenAI Codex 使用专用 Responses provider，而不是标准 Responses 兼容标志。
collection 在 `prepare_call` 中、任何 attempt 之前只解析一次 `AuthResolver`，并返回
`PreparedProviderCall`。其 `route()` 只暴露 `PreparedRoute`，`auth_provenance()` 暴露
非 secret provenance，每次 `start_attempt()` 都复用已冻结的 route、request、鉴权和
cancellation token。attempt 必须串行：重叠 stream 返回
`ModelsError::AttemptAlreadyActive`，取消返回 `CallCancelled`，先前 attempt 若报告
credential-needed/revoked，则 prepared call 以 `CredentialTerminated` 关闭。凭据缺失与撤销分别成为显式且不可重试的
`ProviderError::CredentialNeeded` 和 `ProviderError::CredentialRevoked`。
类型化的逐调用 API-key、Bearer 与 SigV4 覆盖互斥，并在访问来源前优先；
`extra_headers` 会拒绝 Provider 管理的鉴权 header。

### 惰性 AuthContext 与管理结果（AIR-063/AIR-066）

来源访问是独立的 context 职责：object-safe 的 `AuthContext`（`env`、
`file_exists`、`process_snapshot`）拥有来源访问，不承载模型或 telemetry 数据。
`AmbientAuthContext` 是默认的惰性实现，`Models::set_auth_context` 注入后
整体替换 ambient——注入来源不会被绕回进程环境。构造、注册与静态目录
查找执行零 env/store/network 访问；`AuthContextError` 让非 Unicode 环境字节
与失败的存在性探测保持类型化，而不是空结果。选定的 native helper 接收
自有 `AuthProcessSnapshot`（精确 env map 与 cwd）；注入的 context 默认给 helper
空环境，必要变量必须显式提供。Ambient context 仅在选中 helper 时捕获进程环境，
有效的 ADC output cache 不探进程快照。命令按冻结 argv、cwd、env 直接执行；
超时或等待者 drop 会触发进程清理与收割。

在此之上，`Models::{check_auth, get_available, get_auth, logout}` 是管理
结果。`check_auth` 报告配置状态而不刷新——已过期的存储 OAuth 凭据仍是
已配置——并将不可达的 store backend（含探测快照）以类型化的
`ModelsError::CredentialStore` 错误呈现，绝不静默折叠为 `Ok(None)`。
`get_available` 用同一检查过滤注册表。注册的 OAuth route 还遵守持久化的
`available_model_ids`：`Some([])` 是账号的权威空目录，`None` 保留注册目录；
检查不刷新该元数据。`get_auth` 按需解析被选中的来源；
显式 `AuthResolutionOverrides::api_key` 无需触碰任何来源即获胜，route
resolver 的 `CredentialNeeded` 报告未配置，其他被选来源失败则以
`ModelsError::AuthResolution` 传播且绝不尝试回退来源。`logout` 按调用接收
凭据 store，且只确认这一次本地删除：不 unset 环境变量、不撤销远端会话、
不动其他 provider 的条目。仅 descriptor 的存储凭据解析不刷新；通过
`register_oauth_route` 注册的 OAuth route 在 `get_auth` 与准备调用时使用
live resolver，包括必要的刷新。

### Store 拥有的 mutation、ACK 与 mutation barrier（AIR-063 / P20-STORE）

凭据 mutation（login、logout、replace、refresh）由共享 store 拥有，
而不是提交它的 waiter。`CredentialStore` 新增 `modify` 与
`mutation_barrier`：`modify` 提交一个串行化的 read-modify-write
（操作看到当前凭据，并拿到用于失败后重读的 slot view），结算为非机密
的 `MutationReceipt`——即 ACK——记录带 per-provider revision 的已应用
写入、类型化失败与 unknown 丢失窗口。丢弃等待绝不会取消已开始的
mutation：宿主 runtime 存活期间它照常完成，第二个 waiter 重读已提交
的新 token 而不重复操作，per-provider FIFO 将 login/logout/replace/
refresh 在同一 store 上串行化（生产 keychain 桥在每个 slot 全程持有
跨进程锁，保留宿主锁策略）。

`CredentialMutationOp::{Write, Delete}` 是无条件操作，无需解码旧值即可
替换或清除损坏条目；`Update(CredentialUpdateOp)` 则先严格读取当前值再执行
回调。现有 `write_mutation`、`delete_mutation` helper 构造无条件操作。
没有当前 Tokio runtime 时，队列保留 pending 提交；在 runtime 内 poll ACK
或正常提交操作才将它们交给分离的 store owner，barrier 不会启动这些操作。
runtime 丢失可能留下无 ACK 的 pending、未确认效果；worker panic 记录为 `Unknown`。

`mutation_barrier` 同步捕获 store 级 cutoff——共享同一 store 的所有
`Models` 实例看到相同 generation。其 report 列出全部被捕获的
mutation（pending、settled、failed、unknown），并将 cutoff 之后的
新提交作为 fenced late write 报告；它从不触发 auth，也从不中止进行中的
rotation。没有默认 wait/receipt/admission 上限：调用方自带的界限只停止
等待。参考实现 `InMemoryCredentialStore` 内嵌该队列；无共享状态的
store 继承 inline 默认实现。revision 与 receipt 是进程内视图；
跨进程串行化仍由 store 自身的锁策略负责。

### 采集重试唯一 owner（AIR-007）

`start_attempt` 预留 lease；adapter 及其 producer 只在第一次 poll 时启动。
丢弃未 poll 的 lease 不发送请求；HTTP 后端的 protocol-NACK retry 已关闭，
额外 send 必须经过 SDK admission。

`prepare_call_with_policy` 为每个逻辑调用冻结一份 `AcquisitionPolicy`
（普通 `prepare_call` 冻结 provider 的协议默认：helper/Codex wire 为零额外
send，Bedrock 为其冻结的 AWS Smithy 配置）。prepared call 是唯一的采集重试 owner：它对观测到的
attempt 失败进行分类、维护每 payload 变体的 send 计数（`max_retries` 按变体计额外 send，
普通 episode 至多 `1 + max_retries` 次）、计算 source-qualified 等待并封存状态。可重试失败且
预算仍有余量时，`retry_admission()` 返回 1 起始的 `retry_index` 与 `delay_ms`（服务器 hint
优先，受 `max_retry_delay_ms` 约束；否则 `min(500ms * 2^i, 8000ms)` 乘以 `[0.75, 1.0)`
jitter 因子）。截止前 `start_attempt` 以类型化的 `ModelsError::RetryNotReady` 拒绝分发，
封存后返回 `CallSealed`，attempt 流仍在活动时返回 `AttemptAlreadyActive`。lease 预留与
封存/readiness 检查使用同一个同步边界。封存状态具有粘性，可通过 `acquisition_seal()`
观测：成功的流式响应交接记录为 `PostHandoff`。OpenAI Chat 即使在首个 SSE event 前发生
已接受响应的 body 失败，也保留这一事实；body/EOF/parser 失败不能重新开启采集重试。
没有更早传输观测的 adapter 仍由首个交付事件建立交接。已分发但未观测到结局即被 drop 的
attempt 记录为
`UnknownDispatch`；预算耗尽记录为 `Exhausted`；未交付任何事件的终局记录为 `Finished`。
因此成功不再重开 prepared call：重试按 admission 调度，新逻辑调用需重新 prepare。主循环消费方
读取 admission、可取消地等待并再次进入 `start_attempt`；无法重置计数、跳过等待或在冻结预算之外叠加 send。

`prepare_call` 在鉴权前把省略的 `Request.max_tokens` 解析为所选 registry model 的
`max_output_tokens` 容量，包含 model override 和刷新后的 catalog。显式 request 上限
仍作为上限。若已启用 thinking 的 budget 大于或等于该默认输出上限，preparation 会在
鉴权和派发前返回类型化 capability 错误。每次 attempt 都复用解析后的上限。
隐式与显式上限都会按估算的剩余上下文收缩，预留 4096 token 安全余量，最少保留一个
输出 token。无法装入收缩后上限的 thinking 会缩小，尽可能留出 1024 token 回答空间。
非 adaptive Anthropic 按 thinking 等级解析省略的预算；无法同时容纳最小 thinking
预算和回答空间的上限会在鉴权前被拒绝。adaptive effort 不使用该数字预算下限。
`context` 模块估算消息、system 文本和工具 schema；最后一个适用的正向成功 usage
（非 Error／Aborted、时间戳不早于前缀、不限模型身份）充当锚点，只估算锚点之后的
消息，不再有同模型／全文下界。直接调用 provider adapter 仍保留原有默认行为；
这是每个 request 的输出上限，与产品的轮数预算相互独立。

`ProviderError::AccountIdMissing { provider_id }` 是独立且不可重试的情形：凭据存在，
但缺少所选 wire 要求的 account identity。产品层通过显式 `/login <provider>` 修复；
它不表示凭据已撤销。

具备能力的 Anthropic 内置模型会在 system prompt、最后一段 user text、最后一段
assistant text 和最后一个 tool definition 上发出 `cache_control`。Long retention
会增加 `ttl: "1h"`；short/default ephemeral retention 不带 TTL，显式禁用以及
未知/自定义模型不发出 marker。

`Request` 新增 `timeout`、`extra_headers`、`CacheRetention` 和 `session_id`。前三项是
公开的 request-to-wire 基底；只有 `session_id` 在 coding harness 中具有生产生成方。
session-affinity 行为在直连 Responses 与自定义/proxy route 之间不同，详见下文；
Anthropic 则使用模型能力门控的 cache marker。`ModelInfo` 包含现有的嵌套
`ModelCapabilities`；未知/自定义模型默认关闭 cache 支持。

`Usage::cache_write_1h_tokens` 与 `Usage::reasoning_tokens` 是可选 `u64` 子集，
可区分缺失与显式上报的零。`CostBreakdown` 只有四行：`input_cost`、`output_cost`、
`cache_read_cost` 和 `cache_write_cost`。加权的一小时 write 子集折算进
`cache_write_cost`，reasoning 仍计入 `output_cost`，费用与总 token 只计一次父 bucket。
`Provider::refresh_models` 与 `Models::refresh` 实现确定性的原子目录替换，
但仅为基底、无生产触发。

## 图片支持

图片输入使用 `InputContent::Image` 表示。支持的媒体类型是 PNG、JPEG、GIF 和
WebP。所选模型支持图片时，Provider 会把图片序列化为各自的原生 wire 格式。

`validate_request_capabilities` 会在发起网络请求前拒绝已知纯文本模型。Bedrock
通过 Converse 支持 byte/base64 图片源，但会在本地拒绝 URL 图片，因为 Bedrock
Converse 需要图片 bytes。

## 工具结果错误语义

失败工具结果在每个受支持的 provider wire 上都保持可区分：

| Provider family | 失败信号 |
|-----------------|----------|
| Anthropic | `tool_result` content block 上的原生 `is_error: true`。 |
| AWS Bedrock | 原生 `toolResult.status = "error"`。 |
| Gemini / Vertex | `functionResponse.response` 对象内部的 `error: true`。 |
| OpenAI Chat / Azure / OpenRouter / Mistral | 工具输出字符串前缀 `[tool_error] `。 |
| OpenAI Responses | `function_call_output.output` 字符串前缀 `[tool_error] `。 |

成功的（`is_error = false`）工具结果 body 保持修复前的 wire 形状。
`ToolResultMessage::details` 用于 opi 运行时/UI/会话边界；provider 请求 body 使用
LLM 可见内容和 provider 专用失败信号。

## Provider 行为矩阵

Anthropic thinking 内容保留不透明的 `signature`，
`AssistantContent::RedactedThinking` 保留加密的 `data`。这两个字段会经过消息与
会话序列化完整保存，仅向同一个 Anthropic provider 和请求模型原样回传。
HTTP 响应的 `model` 保留请求模型，`response_model` 保存服务端报告的名称，
因此模型别名可以继续使用自己的签名历史。切换路由时，带签名的可读 thinking 转成文本，
加密块被省略。会话导出始终省略这两个不透明字段，即使包含可读 thinking。
已有无签名消息仍可读取，`signature` 为 `None`。
服务端省略模型名时，`response_model: Some("")` 明确保留未知证据，
不会回退为请求别名。

按 family 列出能力与元数据行为。“yes” 表示 adapter 在 wire 上实现了该行为；
“inherited” 表示继承共享的 OpenAI Chat 或 Gemini 路径；“—” 表示不产生。

| Family | Thinking/reasoning | 图片输入 | Cache tokens（用量侧） | Response ID |
|--------|--------------------|----------|------------------------|-------------|
| `anthropic` | thinking blocks | PNG/JPEG/GIF/WebP | `cache_read` / `cache_creation` | `message.id` |
| `openai_chat` | `request.thinking` + 所选 `ModelInfo::thinking_level_map` | PNG/JPEG/GIF/WebP | `cached_tokens` | `chatcmpl-*`（`id`） |
| `openai_responses` | `request.thinking` + 所选 `ModelInfo::thinking_level_map` | PNG/JPEG/GIF/WebP | `cached_tokens` | `resp_*`（`id`） |
| `openrouter` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `mistral` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `gemini` | thinking | PNG/JPEG/GIF/WebP | `cached_content` | — |
| `bedrock` | `supports_thinking` 模型声明 thinking，但 Converse 流的 `reasoningContent` 块不会被解析为 thinking（parser 限制） | 仅 byte/base64；拒绝 URL 图片 | `cache_read` / `cache_write` | — |
| `azure_openai` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `vertex` | inherited | PNG/JPEG/GIF/WebP | `cached_content` | — |

已捕获的 response ID（`message.id`、`chatcmpl-*`、`resp_*`）会回写到
`AssistantMessage::response_id`；没有 provider response ID 的 family 保持为
`None`。纯文本模型会在发起网络请求前通过 `validate_request_capabilities` 拒绝图片
输入。

## OpenAI-Compatible Profile

兼容的 OpenAI 风格服务在缺少实质 wire/auth/能力差异时保持配置驱动
（config-driven）。这是 provider breadth 的首选路径；新增 first-class provider 模块
是保留给这些实质差异的非默认步骤。

`CompatConfig` 携带按 profile 的兼容性标志：

| 标志 | 作用 |
|------|------|
| `system_role_override` | 将 system prompt 渲染为 `developer`（或其他角色）而非 `system`。 |
| `max_tokens_field` | 输出上限的请求字段名（`max_tokens` 还是 `max_completion_tokens`）。 |
| `tool_result_name_field` | 在工具结果消息上回显工具名。 |
| `usage_in_stream` | 请求 `stream_options.include_usage`，并保留来自任意流式 chunk 的 usage 更新。 |
| `strict_tool_schema` | 发出 strict JSON-schema 工具定义。 |
| `reasoning_effort` | 仅保留为遗留兼容性/profile 元数据；wire reasoning 由 `request.thinking` 与所选 `ModelInfo::thinking_level_map` 决定。 |
| `cache_key` | 发送 provider 的 prompt-cache key（cache-affinity 提示）。 |
| `send_session_affinity_headers` | 把请求 `session_id` 映射为兼容的 `session_id`、`x-client-request-id` 与 `x-session-affinity` header；默认关闭。 |
| `require_assistant_after_tool_result` | 面向遗留端点的纯兼容性元数据标记；opi 不会在共享适配器中合成或强制额外的 assistant 轮次。 |
| `chat_completions_path` | 相对 `base_url` 的 chat completions 端点路径（默认 `/v1/chat/completions`）；当 provider 的 base URL 已包含 API 前缀时设置（如 BigModel `/api/paas/v4/...`）。 |

`ModelCompatOverride` 在 profile 默认值之上叠加模型级的 `system_role_override` 和
`max_tokens_field` 覆盖（model 优先于 provider）。静态的按 profile 请求 header
（`extra_headers`，用于会话亲和 / 路由）是独立的 profile 配置字段，经
`OpenAiChatProvider` 构造传入，不是 `CompatConfig` 标志。

OpenAI Responses 原生语义（`ResponsesConfig`）：`store` 和 `strict_tools`
已实现。静态 `reasoning_effort` 仅保留为遗留兼容性元数据；`request.thinking`
与所选 `ModelInfo::thinking_level_map` 是 Chat 和 Responses wire 输出的权威来源。
`previous_response_id` 刻意缺省——Responses 请求按
Chat-Completions 类比构造，因此服务端响应链未被接入。

## 缓存、Response ID 与会话亲和

只要 provider 提供，用量侧 cache token 就会被归一化（Anthropic
`cache_read`/`cache_creation`、OpenAI Chat/Responses `cached_tokens`、Gemini
`cached_content`、Bedrock `cache_read`/`cache_write`）。具备能力的 Anthropic 内置模型会按
`CacheRetention` 在 system prompt、最后的 user/assistant text block 和最后的 tool
definition 上发出请求侧 prompt-caching 断点。兼容 OpenAI 风格的 profile 使用独立的
`cache_key` cache-affinity 提示。

Provider response ID 被捕获并回写到 `AssistantMessage::response_id`（Anthropic
`message.id`、OpenAI Chat `chatcmpl-*`、OpenAI Responses `resp_*`）；其中
OpenAI Chat 会从任何携带 `id` 的 chunk 捕获 response ID，而不只是在 role chunk 中
捕获；其它 family 保持为 `None`。

会话亲和刻意受限。存在有效 session 时，直连 OpenAI Responses 会自动派生
`prompt_cache_key` 和新的 `x-client-request-id`；`send_session_id_header` 只门控
`session_id`。自定义/proxy affinity 默认关闭，必须显式 opt-in。
`previous_response_id` 被推迟（见 OpenAI-Compatible Profile），兼容 profile 可携带静态
`extra_headers` 用于路由/会话钉扎。不存在服务端会话链。

## WebSocket Upgrade（RDF-24）

`ws::upgrade` 在共享的 source-qualified `HttpClient` 上执行严格动态 Upgrade
（显式代理/TLS profile；重定向必须按 client 关闭：对带有重定向策略的 client
发起 Upgrade 会类型化失败，而不是静默跟随）。经典 Upgrade 使用该 client 的私有
HTTP/1.1 传输，保留同一份冻结的 trust、mTLS 身份与代理配置，不重新发现来源；普通请求
保持原有 H2/H1 配置。该桥拥有请求形状（`Connection:
Upgrade`、`Upgrade: websocket`、`Sec-WebSocket-Version: 13`、来自固定平台熵源的新鲜
16 字节 `Sec-WebSocket-Key`、已提议的 subprotocol 与可选的 `permessage-deflate`
offer）以及 101 响应的严格校验：状态与 HTTP 版本、精确的 `Sec-WebSocket-Accept`
（钉住 RFC 6455 已知向量）、重复/缺失 singleton 规则、按 token 的
`Connection`/`Upgrade` 匹配、只接受已提议的 subprotocol，以及精确的
permessage-deflate 参数校验（无重复条目、无未知参数、值域合法、不含 offer 未提议
的参数、client 窗口不高于提议上限）。任何违规都会在流分离前以类型化的
`WsUpgradeError` 失败。

协议状态机位于 SDK 内部私有的 `src/ws/frame.rs`、`pmd.rs`、`io.rs` 模块。
没有公开 codec API、test-only export 或 Cargo feature，也没有独立的包、版本与
发布身份。公开 Upgrade 结果仍为原始 `reqwest::Upgraded` 流与协商 metadata，
Upgrade 不构造私有 codec。`Negotiation::driver_parameters` 返回
`Option<NegotiatedPmd>`，保留两个方向各自的 context-takeover 标志与窗口。
私有 codec 当前尚无生产调用方。仅物理合并不能证明 Models → 真实 Codex →
Upgrade → codec（20.16）、continuation/pool/cleanup（20.17）、最终组装 SDK
包（20.57）或原生平台/MSRV（20.58）已验收；这些仍保持 OPEN。

### 私有协议所有权

| 模块 | 合同 |
|---|---|
| `ws::frame` | 严格客户端帧解码（保留 opcode/RSV、分片、控制帧形状、最短长度编码、64 位高位）及消息装配；存储按实际到达字节增长，类型化 fallible 预留。 |
| `ws::pmd` | 自有 permessage-deflate：tail 前 `SyncPoint` 证明、恰好一次 `00 00 FF FF` 注入、`Z_BLOCK`/`BFINAL`/`ResetKeep` 处理、context takeover 与每 poll 有界工作量；encoder 是普通安全 `flate2` 客户端。 |
| `ws::io` | 独占连接 owner，持有 pending read/write/decode 任务、显式 Close、调用方提供的 mask、背压与终态失败吸收。 |

一个私有 `Driver` 拥有完整连接状态。借用的 `next()` future 返回完整消息
（含 Ping/Pong/Close）；丢弃借用保留部分 frame/PMD/UTF-8 状态，可恢复同一
owned 操作。Owner 的逻辑取消/abort/Drop 则释放资源并抑制晚输出；idle owner
必须注册真实取消唤醒。取消借用不授权连接归池。协议层不发现 auth、不创建
timer 或 mask 熵、不重连/重试、不决定 provider/resource policy。

双向 PMD 参数按 server 值配置接收、client 值配置发送。`None` 禁用 PMD；RFC
省略的窗口参数映射为 15。固定 encoder 内部将请求窗口 8 归一化为 9；测试
独立核对真实 wire distance 是否满足协商的 256 字节窗口，不以配置数字为证明。

显式 Close 要求双帧、写 shutdown 与对端 transport EOF，保留部分应用写，区分
已写字节和 pending flush，保留有界 in-flight 事件，deadline 仅为调用方显式
绝对 `tokio::time::Instant`。异常 EOF/协议/Close 超时只报一次类型化错误，丢弃
保留的接收事件，随后接收永久 EOF；成功 Close 仍可交付保留事件。存储随实际
接收字节增长，以 `try_reserve*` 类型化失败，从不按声明 payload 投机分配。
IO/inflate/UTF-8/continuation 内部工作量 quota 提供协作让出，不是消息大小或
迭代上限。分配、单次 backend 调用与同步出站编码不可硬抢占。

### 压缩与维护边界

未修改的依赖图为 `libz-rs-sys = 0.6.8`（关 defaults，开 `std` +
`rust-allocator`）、实际解析 `zlib-rs 0.6.8` 与 `flate2 = 1.1.10`（关 defaults，
开 `zlib-rs`）。decoder 使用 doc(hidden) 公开的 `inflateSyncPoint` /
`inflateResetKeep` 及传递 zlib-rs `__internal-api`。窄范围本地 unsafe 边界在
`src/ws/pmd.rs` 内说明：唯一 boxed `z_stream`、无 clone/自定义 allocator/
callback/raw handle 泄漏、调用前后指针重绑定、恰好一次 `inflateEnd`，以及
刻意的 `Send` 而非 `Sync`。没有本地 extern 声明或动态 system zlib 选择。
压缩许可证及 source/feature/native 审查义务见 [NOTICE](NOTICE)。

| Caller 图 | Encoder 后端 | Decoder 后端 |
|---|---|---|
| ordinary | flate2 `zlib-rs` | 直接 `libz-rs-sys` Rust 路径 |
| `flate2/default` | flate2 `zlib-rs`，合并 default features | 直接 `libz-rs-sys` Rust 路径 |
| `flate2/zlib` | flate2 `zlib`（原生 C `libz-sys` 构建） | 直接 `libz-rs-sys` Rust 路径 |

flate2 1.1.10 优先 C 后端，其次 zlib-rs，最后 miniz_oxide。default union 只向
图中加入 miniz_oxide，并不选用其 encoder。C-zlib native source/build/link 输入
须另行审查。allocator 冲突、gz/gzprintf、nightly、export-symbols/prefix 与其他
未审 union 不继承支持。隐藏 API/unsafe/feature/安全变化、接受集缩小或新的
生产消费者均须重审。依赖升级须记录实际 source/checksum/features，并重跑
三图语料、Send/Close/取消/公平性及打包检查。

### 局部协议检查

私有协议测试位于 `src/ws/tests`，原始 fixture 位于 `src/ws/tests/fixtures`。
边界重放重新生成固定 72,413 条 PMD 语料并核对记录分类（超过 64 字节的输出
用 SHA-256）；提取来源与 oracle 身份随 fixture 保留。三图运行同一组测试：

```sh
cargo test -p opi-ai --lib ws::
cargo test -p opi-ai --lib ws:: --features flate2/default
cargo test -p opi-ai --lib ws:: --features flate2/zlib
```

公开 Upgrade、协商与 WSS 传输合同位于 `tests/ws_upgrade.rs`，通过调用方同样使用的
公开 `HttpClient`/Upgrade 接口验证。运行 `cargo test -p opi-ai --test ws_upgrade`；
SDK 包验证会在三图中分别运行该集成目标和私有测试。两层仅共享 `tests/support`
下的脚本化 peer helper；WSS 与 TLS 集成测试使用同一份合成 TLS fixture。

私有 loopback 测试将公开 Upgrade/`HttpClient` 与 codec 组装，包含双向窗口 8
协商、有/无 context takeover 的多消息、独立 RFC 1951 decoder 及 distance-257
反控制。它们验证 `cfg(test)` 接线，不证明生产 Codex 路径或外部公开 codec。
idle cancellation 测试等待真实 Pending read waker，在 owner 内取消，并在
ACK 前观察 transport/PMD 释放，没有新增生产取消 API。

## 代理

`HttpClient` 携带共享的 `reqwest` 连接池，支持显式按 provider 的代理配置
（provider profile 的 `proxy.url` 和 `proxy.no_proxy`）以及环境变量回退
（`HTTPS_PROXY` > `HTTP_PROXY` > `NO_PROXY`）。代理 URL 中的代理凭据在任何诊断
展示前都会被脱敏。代理传输语义（经代理重试、取消）由重试/代理覆盖范围负责。

## 尽力而为的费用

费用映射是 best-effort。错误的置信比显式未知更糟：当 provider 用量缺失时，
`Usage::unknown()` 和 `CumulativeUsage` 会把该轮次明确标记为未知，而不是当作已知零用量。
因此，只要任一轮 usage 未知，或模型定价缺失，面向 session 的费用汇总就应省略。费用绝不
阻塞成功的流。

## 私有 Schema 方言引擎

`schema` 实现固定 TypeBox JSON 方言（由 schema readiness 附件选定）：固定
TypeBox 1.3.27 `Compile` 路径在受信 JSON schema 上的求值语义——全部 42 个
JSON 机制、21 个默认字符串 format、结构化关键字 guard（guard 不满足的关键字
为 inert，绝不是错误）、JavaScript 属性键枚举顺序（integer-index 键升序在
前，随后为插入顺序）以及完整的 lexical/resource/dynamic/recursive 引用
frame。求值运行在显式 work stack 上（schema 深度不会耗尽 Rust 调用栈），
区分 productive 递归与 no-progress 递归（后者为 typed 失败），没有隐式
I/O 或 source-free 的规模/fuel 上限。结构选中的 schema 边选择 regex 编译；
annotation 对象只有被引用为 schema 时才参与验证。引用目标在编译后的 plan 中
冻结，包括实例中缺失字段在编译时产生的资源影响。只有证明重复的编译前缀在
后续各轮保持相同的 URI 判断与函数缓存命中行为，admission 才返回 `NoProgress`；
不剪枝 active 节点，也不设置迭代次数上限。null 引用目标使用来源的 false
回退；根 null 及根或引用目标中的 number/string 以 `InvalidSchema` 失败，JSON
数组仍保持 inert。regex 与 URL 使用 crates.io 上游 `regress 0.12.0` 和
`url 2.5.8`。Unicode 属性、脚本、大小写映射和 IDNA 数据随受支持的依赖
profile，不承诺固定 Unicode 版本。允许的数据差异与升级检查见
[RDF-21](../../docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md#rdf-21-upstream-data-profile)。
workspace lock 不固定所有库消费者的图。Opi 关闭 regress 默认 features，
启用 `std`、`utf16`、`prohibit-unsafe`；普通外部消费者也验证 feature union。
数据差异不放宽 schema guard、admission/ref 顺序、normalization、授权或
零 I/O 合同。完整 conformance 与包／平台验收仍保持 OPEN。

上游 profile 审查（regress、`url`、`idna`、`idna_adapter`、ICU4X `icu_*`、
`unicode-ident` 及其 `yoke`／`zerofrom`／`zerovec` 支撑 crate）：workspace
解析为 ICU data 2.2.0 与 `displaydoc 0.2.5`，普通 consumer 证明解析为
ICU data 2.3.0 与 `displaydoc 0.2.7`。`icu_normalizer_data` 和
`icu_properties_data` 都携带读取 `ICU4X_DATA_DIR` 的 `build.rs`；其
`src/lib.rs` 可包含该目录的 `mod.rs`，替代内嵌数据；已定义的空值也可
选择此路径并使构建失败。`compiled_data` 不关闭这个构建期输入。证明
runner 在创建 scratch 或启动任何子进程前拒绝所有已定义值，包括空值。
该保证仅适用于此 harness；普通 Cargo consumer 必须管理自己的构建环境。
这不改变 Schema 运行时的零 I/O 合同。

解析闭包还包含 `proc-macro2`、`quote`、`serde`、`serde_core` 的构建脚本；
proc-macro 包含 `displaydoc`、`serde_derive` 和 `yoke-derive`、
`zerofrom-derive`、`zerovec-derive` 伴随 crate。这些构建输入需要来源与
feature 审查；Schema 数据 profile 不引入新的原生 C/C++ 代码（`ring`
仍是既有的 rustls 传输依赖）。

regress 的 `prohibit-unsafe` 是上游实现分支选择 feature，不是
`forbid(unsafe_code)` 或 `deny(unsafe_code)` 编译规则。多处使用 `cfg!`
选择实现，且 regress 0.12.0 的 `src/unicode.rs::uppercase` 在该 feature
开启时仍使用 `unsafe` `get_unchecked`。Opi 调用 safe API，但该 feature
不证明依赖树无 unsafe；升级时仍须按精确源码和有效 features 审查上游
剩余 unsafe 及其不变量。家族 `rust-version` 要求最高为 1.88（consumer
侧 ICU4X 2.3.x），低于 workspace MSRV 1.97；这是 manifest 审查，不是
原生平台或 MSRV 构建结果。许可证为 MIT OR Apache-2.0（regress、`url`、
`idna`、`idna_adapter`）、Unicode-3.0（ICU4X 家族及支撑 crate）与
`(MIT OR Apache-2.0) AND Unicode-3.0`（`unicode-ident`）；`displaydoc` 及
`serde`／`proc-macro2`／`quote`／`syn` 工具链为 MIT OR Apache-2.0。解析版本、
registry 来源与校验和记录在 `Cargo.lock`，两个 caller profile 的普通外部
consumer 实际解析图与有效 features 记录在 package-consumer 证明证据
（`scripts/pi-sdk-conformance/evidence/upstream-package-proof.json`）。

## 公开工具参数 Normalizer／Validator

`schema::validate_tool_arguments`（以及按名字查找的
`schema::validate_tool_call`）是构建在方言引擎上的 SDK 接缝：传入自有的
`arguments_error` 标记会在任何 coercion 之前拒绝；调用方的 raw 值绝不被改写；
normalize 删除“独立编译的子 schema 拒绝 null”的可选 `null` 属性，应用固定 Pi
coercion 语料（allOf 顺序折叠、anyOf/oneOf 原值优先再 clone-per-arm、按
`type` 数组顺序的类型联合 primitive coercion，使用精确 ECMAScript
`Number`／`String` 转换——`Number("0x10")` 为 16，`String(1e21)` 经固定的
ryu-js 1.0.2 得到 `"1e+21"`），最后用原 schema 检查 normalize 后的根值（上游
在 coerced 根失败时返回未验证 raw 的行为被有意不复制——RDF-09）。Invalid 结果
只携带脱敏失败信息（实例指针、keyword、来自 schema 的原因）；精确 JSON 整数
保留精确十进制数字，不复制 JavaScript 解析期的精度丢失（RDF-09）。TypeBox
`Value.Convert` 对纯 JSON 方言 schema 是 no-op（其分发读取非 JSON 的 `~kind`
标记），JSON schema 中的 `~kind` 标记保持 inert——有记录的方言边界。

## 规范 Replay 投影

`replay` 把规范时序转换为请求局部的 wire 视图（AIR-010/012/013/014，
任务 20.7）。`project_replay(&[Message], &ReplayRoute, ReplayProjection)` 按
批次位置＋source ID 校验工具关联：已闭合批次可以复用 ID，但批内重复 call
ID、重复 result、孤儿 result 或 result／call 名称不匹配都是类型化
`ReplayError`，在任何请求构造之前（授权与网络之前）失败。无歧义的缺失
result 以请求局部错误收尾，声明结果不可得且执行效果未知——
绝不声称“未执行”，也绝不重新执行；只有可信执行边界记录已知的未执行。
若失败 assistant 的调用视图被省略，则用脱敏的普通 user 上下文保留结果缺失
事实，不伪造工具调用或结果。
继续身份是精确 wire＋provider＋请求模型（`AssistantMessage.wire`，serde
默认缺省：legacy 未知绝不视为同身份）。跨身份丢弃全部 thinking——可读、
signed 与 redacted 一视同仁——绝不把推理转成可见文本（Pi 1.3 语义）；同
身份保留 redacted 数据与 signed 块（含 signed 空块），丢弃空白未签名
thinking。失败回合（`Error`／`Aborted`）及其关联的未执行 result 作为闭合
单元一并省略，`Length` 回合保留可表达的对；length 绝不授予执行。畸形
proposal 在字符串参数 wire（Chat、Responses）上原样发送原始文本，在对象
参数 wire（Anthropic、Bedrock、Google）上替换为显式具名失败上下文——绝不
`{}`、`null` 或伪造调用。wire ID 请求局部合法化（字符集／长度，稳定双射，
call／result 同步；canonical 不变）。图像在能力路由上保留关联、顺序与失败
标志（Anthropic 错误 result 的图像拆到回合结果之后）；`details` 绝不进
wire，URL 源原样透传不下载。`ReplayProjection::Strict` 用类型化错误拒绝同
批有损转换而不是记录事实。六个 provider adapter 在构造请求体开头消费该投
影（`build_request_body` 现在返回 `Result`）；Responses 解析器打包
`call_id|fc_item_id` 支持同身份双 ID 往返，序列化器还原两者，外来历史丢弃
item id。固定参考：
`.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts`；逐 wire 加密推理
parity（AIR-038）与 effort 戳（V-32）由后续任务收尾。

## 规范 Context／Transcript 时序

`transcript` 拥有规范的请求时序（AIR-008/011/020）：公共 `Context` 简写
（`system_prompt`、`messages`、`tools`）经 `transcript::normalize_context`
恰好一次折叠为首条 `Message::System`，该函数返回 typed `Result<TranscriptContext, TranscriptError>`——未归一化的简写无法到达 provider 代码。后续 system 消息修改运行态：
`content` 追加，具名 `sections` 保序替换（`None` 删除、删除后重加移末尾），
`tools_added`／`tools_removed` 按值变更工具集（重定义=先删后加；相等状态不
记录冗余变更）。重放（`get_current_system_message`／`get_current_tools`）
解析当前 prompt 与工具。投影为 native-or-collapse：接受对话中 system 消息
的传输保持原位，其余折叠为单个重放头——始终返回新值，绝不改写规范时序。
显式 `TranscriptProjection::Strict` 在任何请求构造之前拒绝传输无法按时序
放置的时序保真度。声明安装（初始归一化、内嵌 system 消息、反序列化及
`TranscriptContext::append_system_change`）先
校验名称（非空、批次内唯一）并用私有方言引擎编译 input schema，失败以类型
化错误在消息进入 transcript 之前发生。`Request` 以
`context: TranscriptContext` 携带输入，不再有独立的 `system`／`tools` 运行时
字段；每个 adapter 在构造请求体开头解析 transcript（固定的 TypeScript 参考
是 `.repo/pi-1.3.0/packages/ai/src/utils/transcript.ts`；上游已废弃的
`hasToolRedefinitions` 不移植）。

## 私有上游 Crypto Profile

私有 `crypto` 模块（不属于公开 API）基于未修改的 crates.io crypto 基线
（`rustls` 0.23.40 + `aws-lc-rs` provider：`aws-lc-rs` 1.18.2 /
`aws-lc-sys` 0.46.0）负责 key 准入与签名，且仅使用其公开 safe Rust
接口（`KeyProvider::load_private_key`、`SigningKey::algorithm`/
`choose_scheme`、`Signer::sign`）。它是后续 TLS/mTLS 宿主接线与真实 ADC
调用方的 substrate；没有本地 FFI、crypto fork、私有 crypto namespace
或公开 crypto API。

准入 profile 接受未加密双素数 RSA-2048..8192（PKCS#1/PKCS#8、PEM/DER，
含 e=3）与 P-521（PKCS#8/SEC1）。RSA-1024、多素数 RSA、加密/畸形输入、
仅公钥输入及 profile 外算法一律为 typed、脱敏的拒绝——绝不假成功、
降级或重新发现身份。AWS-LC provider 按实例显式构造（绝不安装进程级
全局 provider）；reqwest rustls-tls 图中传递的 `ring` 共存不变，现有
HMAC/SHA/SigV4 签名不变。

原生准入说明：`aws-lc-sys` 将 vendored AWS-LC C 源码构建为静态库
（标准证据使用 `AWS_LC_SYS_USE_SYSTEM=0`；system directory/bindings、
version-skip 与 target override 均属未审查替换）。各平台构建需要 C
工具链；允许原生进程级一次 CPU 初始化与 OS entropy，不进行全局
provider/config/env/subscriber 变更，不做环境凭据发现。基线为非
FIPS；未测试的 feature union 与平台不声明支持。同步原生签名不可被
硬抢占：取消仅在操作间观察，owned key/资源保留至阻塞 worker join，
迟到结果被抑制而非发布。

## 源限定 TLS（RDF-23）

`HttpClientBuilder::tls(TlsOptions)` 是源限定 TLS 配置的唯一公开入口。
`TlsOptions` 携带 trust 选择（打包的固定 Node 参考默认 CA bundle、
同 bundle 加显式 extra roots、显式 roots 整体替换、或对宿主系统根
存储的显式 opt-in）、可选的 owned mTLS 客户端身份，以及 H2 优先 /
仅 HTTP1 的 ALPN 偏好。所有输入在构建客户端时验证并冻结——先于任何
网络 I/O——失败为 typed、脱敏（`HttpClientBuildError::Tls` /
`TlsConfigError` / `TlsKeyError`）；客户端身份密钥经由上方私有 crypto
owner 准入（RSA-1024、多素数 RSA、加密或畸形密钥在准入处 typed 拒绝）。

没有显式 TLS override 的客户端使用同一套默认选项：固定 Node 根、无客户端
身份、H2 优先 ALPN 和固定实例 provider，不继承进程全局 provider 或后端默认根。

TLS profile 本身固定、不可选：显式实例 AWS-LC provider，TLS 1.3 + 1.2，
WebPKI 链/主机名/时间验证始终开启，声明密钥交换组顺序
`X25519MLKEM768, X25519, secp256r1, secp384r1`（有意不提供 P-521 密钥
交换；仅 P-521 的对端握手失败且零业务 HTTP）。不暴露 backend 选择器、
generic crypto trait、原始 verifier、旁路或测试专用签名 API；不安装
进程级全局 provider 或配置——在宿主预装 `ring` provider 下实例路径
行为不变。

固定默认信任 bundle 是 Node v22.19.0 运行时内置 CA 集合，打包在
`opi-ai` 内，可通过 `scripts/extract-node-root-store.py` 复现（150 张
证书，bundle sha256
`12f4cd646968b72866b445b2850df3ac9e245031d15cdfa76d8ab51bde6f7c24`）。
它不是 webpki-roots、OS 存储或环境发现；宿主根仅为逐客户端显式
opt-in。

包消费者证明：`scripts/pi-sdk-conformance/package_slice.py --family
crypto --offline` 对 `opi-ai` 打包、在工作区外解包，并以普通与
caller feature-union 两个消费者走完本公开路径（trust 模式、mTLS、
ALPN、typed 负例、宿主 provider 无关性），记录解析出的 rustls/aws-lc
版本、registry 校验和与原生构建溯源。真实 ADC 接线仍归后续 owner；
本条只收口传输机制。

## 源限定 HTTP 采集 Profile（RDF-22）

`HttpClient` 在未修改的 crates.io `reqwest 0.12.28` / `Hyper 1.9.0` 图之上
持有源限定的 HTTP 发送策略。`HttpClientBuilder::redirects(RedirectPolicy)`
是显式重定向规则：`RedirectPolicy::Limited(n)`（默认 10，由 `opi-ai` 显式
持有而不是跟随后端默认值）限定一次逻辑请求的合法重定向发送——原始请求加
至多 `n` 跳——超限在采集阶段以类型化脱敏原因 `Network("too many
redirects")` 失败且不再发送；`RedirectPolicy::None` 将 3xx 响应作为普通
provider 状态分类交还给调用方。跨源跳转（主机或端口变化）在下一跳发送前
剥离受保护头（`authorization`、`cookie`、`cookie2`、`proxy-authorization`、
`www-authenticate`）；同源跳转保留它们。代理仅显式使用：后端的
`HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` 环境探测被禁用，由显式
`ProxyConfig` 的作用域 `no_proxy` 列表决定旁路；`https://` 代理以
外层 TLS -> CONNECT -> 内层 TLS 的方式到达目标，无自定义传输二次包装。
客户端不存在任何重试、退避或隐藏发送：取消会停止尝试，接受后被截断的
body 恰好产生一个终止性流错误，服务端观测到的发送数等于合法计数。

钉定上游图的实测接受范围（由 `tests/http_acceptance.rs` 经真实
`Models -> OpenAiChatProvider -> HttpClient` 回环路径、按逐字节与合并两种
投递方式验证）：HTTP/1 响应头经 8 KiB 初始读缓冲解析并自适应增长（自适应
上限之内无字节上限；300 KiB 响应头被接受），消息头最多 100 个字段，
chunked body 累计 chunk 扩展字节至多 16 KiB（超出后 body 以类型化的
接受后 `StreamError` 失败）。trailer fixture 等待 HTTP EOF：99 个字段及累计
16,383 字节被接受，100 个字段或 16,384 字节及更大 fixture 以类型化的接受后
`StreamError` 失败。畸形及截断 trailer 也会失败，不再被更早的 SSE 终态隐藏。
这些是上游 parser 边界，不存在 SDK 侧
body/事件上限、分配提示或放宽的 content-length/transfer-encoding/EOF
校验。失败保持可证明的阶段：响应前问题在采集阶段表现为
`Network`/`Timeout`；接受后的 body 问题保留 `StreamError` 或原有 `Timeout`
原因，以及 canonical partial 和已知 usage。已迁移的 Chat 路径即使在首个公共事件前
发生已接受响应的失败，也会封存采集重试。

打包消费者证明：`scripts/pi-sdk-conformance/package_slice.py --family http
--offline` 将 `opi-ai` 打包、在工作区之外解包，并以普通与调用方特性并集
两种消费者经由这条公开路径运行（带精确发送计数与受保护头剥离的重定向链、
类型化的重定向超限与采集失败、带作用域旁路的代理路由、单终止 EOF），
记录解析出的 reqwest/hyper/hyper-util/http 版本与 registry 校验和。

## 离线内置 Model Catalog（DEL-10）

`catalog` 模块把一份真实、带版本的 model catalog 作为离线 crate 资源嵌入：
`resources/catalog/builtin/catalog.json` 与本地 Pi 树
（`.repo/pi-1.3.0/nix/model-catalog.json`）所固定 revision 的发布产物逐字节一致，
`manifest.json` 记录交付输入溯源——来源 URL 与 revision、artifact SHA-256、
Pi 包/本地来源关联、data schema 版本 6、bundle schema 版本、完整 provider
列表与逐操作条目计数，以及 Pi 自有 validator/hydrator 与 opi packer 的摘要。
Bundle 由显式交付输入工具 `scripts/pi-catalog/pack.py` 生成（`--check` 校验）；
任何路径都不会从测试、defaults 或 generator 运行合成 catalog 数据，真实
artifact 缺失是阻断性输入而不是重建触发条件。

`Catalog::builtin()` 每进程解析并校验一次内嵌字节——无网络 I/O、无账户探测、
不运行 generator——重新校验 artifact 摘要、bundle/data schema 版本、
provider 集合、每个条目的数据合同（identity、模态、能力、cost）、每 provider
的 `type:id` 唯一性与结构摘要。任一不匹配都是 typed、fail-closed 的
`CatalogError`；不存在部分可用的 catalog。按操作限定的 getter
（`models(provider, operation)`、`model(provider, operation, id)`）公开
chat/image/classifier 条目的 typed 核心字段与完整原始条目，供后续
store/refresh 与 profile/catalog 集成 owner 消费。`CatalogHandle` 持有活动
revision：`try_install` 完整校验候选后原子换入，因此篡改、过期、错误版本、
缺 provider、重复 identity 或违反合同的候选被拒绝时，最后已知 revision
保持可读，已持有的 `Arc<Catalog>` 永不被重新绑定。显式完整 custom 记录
（`ChatModelRecord`）独立于任何 catalog 状态保持可用。Image 条目仅是用于
选择/计价的 catalog 元数据；SDK 自身的 image 操作路径归 typed image API
任务所有。

## 动态 Catalog Store 与 Refresh（V-27/AIR-002/AIR-033）

`ModelsStore`（`catalog` 模块）是动态 catalog 的唯一 revision-commit owner：
`RefreshGuard::commit` 先持久化快照（临时文件 + 原子 rename，distribution
schema v1 JSON 加 `opi` revision 头），再返回确切被确认的 map，
`Models::refresh` 随后才把它应用到 registry——内存与磁盘要么一起发布，
要么都不发布。在新 revision 之前创建的 guard 被拒绝为 `Superseded`，
不触碰已发布状态；被取消的 guard 不落盘；重开 store 时先于任何网络
工作恢复最后已知 revision。持久化文件采用 Pi 的 *distribution* schema
（`models.schema.json`），与内置 bundle 内嵌的 hydration 数据（v6）是
不同 artifact；opi 特有保真度（streaming、cache 开关、计价分层、精确
thinking 映射、wire compat 载荷）经显式 `x-opi` 扩展往返，纯 v1 文档
以封闭默认值解码。

`Models::refresh(allow_network)` 是唯一刷新入口：`refresh(false)` 不做
任何 OAuth/网络工作，仅恢复 store 缓存（`RefreshStatus::OfflineRestored`）；
`refresh(true)` 按确定性排序顺序抓取已注册 provider，再按注册顺序抓取
已注册 catalog source。结果按 provider 隔离：成功者发布，失败者保留
该 provider 的旧数据并以脱敏错误上报（`RefreshReport` 中的
`RefreshOutcome::{Published,Failed,Restored}`），不会抹掉其他 provider 的
catalog；`Ok(None)` 让 provider 回退到内置模型。`refresh_with` 观察
`CancellationToken`——被取消的轮次不抓取、不持久化、不发布任何内容。
已 prepare 的调用在刷新后保持冻结绑定。`DynamicCatalogSource` 是
宿主自定义 catalog source 的公开接缝；`HttpCatalogSource` 经池化连接
抓取单份 distribution-schema-v1 文档，失败全部脱敏。SDK 不注册任何
source、不附加任何 store、没有 pi.dev/默认启动抓取策略。

## 公共 header 合并、scoped env 与可取消等待（P20-S53-6，任务 20.53.6）

`provider_headers::merge_provider_header_sources`、新 `provider_env` 模块与新
`cancel` 模块补齐锁定 `utils/headers.ts`、`utils/provider-env.ts`、
`utils/sleep.ts`、`utils/abort.ts` 工具族：

- **`merge_provider_header_sources(&[&[(String, Option<String>)]])
  -> Option<Vec<(String, String)>>`**：多来源 case-insensitive 合并。后源胜出；
  胜者移至末尾并保留原始大小写（锁定 Map delete+set 顺序）；`None` 删除且不
  插入；空结果返回 `None`。属未验证的配置归一化，输出送
  `ProviderHeaders::try_new`；`merge_request` 的 fail-closed 覆盖拒绝不变。
- **`provider_env::provider_env_value(name, scoped)`**：scoped 注入值优先于
  ambient `std::env::var`，忠实复刻锁定的 `||` 短路——scoped 空串回退
  ambient，ambient 空值产出 `None`。仅惰性按名读取：构造期零访问、零全局
  缓存/变更；不新增代理/身份策略层（属主仍是 `http` 与 `auth`）；无 Bun
  `/proc/self/environ` 回退。
- **`cancel::{operation_token, sleep_cancel, race_with_cancel,
  Cancelled}`**：复用 `CancellationToken` 的可取消等待。
  `operation_token(Some(&parent))` 返回单向子 token（父取消传子，反向不传）；
  `sleep_cancel(ms, &token)` 预取消不启计时器，取消/就绪/丢弃均释放计时器；
  `race_with_cancel(future, &token)` 返回操作输出或 `Cancelled`，预取消立即失
  败且不轮询操作，取消时丢弃被弃操作 future（Rust 原生的弃约处理）。测试以
  暂停的 Tokio 时钟与 Drop 哨兵保证确定性。无新依赖；不建执行器或全局计时
  器权威。

## 公共 shortHash 与 UUIDv7 身份工具（P20-S53-4，任务 20.53.4）

`opi_ai::identity::{short_hash, UuidV7Generator, UuidV7Error}` 补齐锁定
`utils/hash.ts` 与 `utils/uuid.ts` 工具族：

- **`short_hash(&str) -> String`**（锁定 `shortHash`）：兼容标识（会话种子、
  `fc_` item id）的逐位移植——UTF-16 code unit（代理对计两单元）、32 位回绕
  乘、顺序终局混合（各流自移 16、他移 13，第二段混合使用已更新的 `h1`）、
  无符号 base36 先 `h2` 后 `h1` 输出，fixture 以严格 `ToInt32`/`ToUint32`
  语义预计算锁定。文档明示非密码学、非持久身份承诺；source-qualified
  crypto hash 的精确编码仍由其属主模块持有（`auth/oauth` PKCE、`auth/aws`
  签名）。
- **`UuidV7Generator`**（锁定 `uuidv7`）：持有单调时间戳下限与 41 位序列。
  `generate(now_ms)` 时钟回退不倒退（有效时间 = `max(now, last)`）；
  `generate_at(ts)` 原样保留 0..2^48-1 整数时间戳且不推进下限；越界返回
  类型化 `UuidV7Error::InvalidTimestamp`；序列首用随机字节播种、递增至
  `UuidV7Error::SequenceExhausted`（测试经
  `test_support::force_sequence_exhausted` 确定性触发）。字节布局逐位照锁定
  源（版本 `0x7`、变体 `0b10`、序列散布 b6..b11、保随机 LSB），8-4-4-4-12
  小写 hex。

## 公共 overflow/Length 判别与类型化诊断（P20-S53-3，任务 20.53.3）

`opi_ai::message::{is_context_overflow, is_recoverable_length}` 与
`opi_ai::provider::{extract_diagnostic_error, DiagnosticErrorInfo}` 补齐锁定
`utils/overflow.ts` 与 `utils/diagnostics.ts` 工具族：

- **`is_context_overflow(message, context_window)`**（锁定
  `isContextOverflow`）：错误型溢出先过锁定非溢出排除（throttling /
  rate-limit 类消息永不判为溢出）再匹配锁定 provider 模式集；Cerebras
  bodyless `400/413 (no body)` 形状额外要求 `provider == "cerebras"`。
  给定窗口时还检测静默溢出（`Stop` 且 `input + cache_read > window`）与
  零输出 Length 截断溢出（`Length` 且 `output == 0` 且
  `input + cache_read >= 窗口的 99%`）。两个用量分支都要求已知用量
  （`Usage::reported`）且窗口大于零；unknown 用量绝不转为零。
- **`is_recoverable_length(message, desired_max_output)`**（锁定
  `isRecoverableLength`）：`Length` 停止且低于原始期望输出上限（仅已知
  用量）时为真——调用方可据此做一次有界 compact-and-retry。两个谓词均为
  纯函数，绝不触发 compaction 或 retry 策略。
- **`extract_diagnostic_error(&ProviderError)`**（锁定
  `extractDiagnosticError`）：返回可序列化的
  `DiagnosticErrorInfo { name, message, http_status, error_name }`——变体
  名、secret-safe 展示消息（细节载体为 `ProviderErrorSummary`）、类型化
  `ServiceFault` HTTP 状态与 wire 错误名。原始 provider 载荷永不进入
  summary；无 stack 字段（Rust 无执行栈对应物）。

## 公共 JSON repair 与可移植 schema 工具（P20-S53-2，任务 20.53.2）

`opi_ai::message::{repair_json, parse_streaming_json,
finalize_tool_arguments, string_enum}` 补齐锁定 `utils/json-parse.ts` 与
`typebox-helpers.ts` 工具族：

- **`repair_json`** 修复畸形 JSON 字符串字面量：串内原始控制字符转为合法
  转义（`\b`/`\f`/`\n`/`\r`/`\t` 或 `\u00xx`）；非法转义前的反斜杠加倍，
  使模型发送的文本保持字面；无四位十六进制的 `\u` 同样处理；合法转义原样
  保留。
- **`parse_streaming_json`** 是 display 专用部分解析器，遵循锁定级联序：
  空输入→`{}`；完整文档（经转义修复）权威解析；否则依次对原始文本、修复
  文本做前缀补全（闭合悬挂引号/括号、悬挂 key/value→`null`、截断数字与
  字面量保守补全、忽略首个完整顶层值后的垃圾）；最终兜底为合法空对象。
  永不失败，也永不凌驾终态结果。
- **`finalize_tool_arguments`** 是完成 tool call 的公共 AIR-048 契约：空输入
  终化为 `{}`；完整对象保留其 owned raw 文本；仅转义差异返回修复文本；
  其余保留 raw 文本并携带 redacted-safe `arguments_error`（绝不回显模型
  文本）。终态 object/`arguments_error` 权威，raw 保留。
- **`string_enum`** 构造可移植 `{"type": "string", "enum": [...]}` schema，
  可选 `description`/`default`（空串省略）——刻意不用 `anyOf`/`const`，无
  TypeBox 符号或 JavaScript ABI；返回值可直接用于 `ToolDef::input_schema`。

## 公共 text/system 渲染工具（P20-S53-1，任务 20.53.1）

`opi_ai::message::{content_text, assistant_content_text,
output_content_text}` 按块序抽取并拼接消息内容枚举中的**可见文本**块，
以单个换行连接。图像、工具调用与 thinking（明文与 redacted 两种形态）
绝不出现在渲染结果中。这些 helper 是只读投影：借用内容，绝不改动
canonical 输入。

它们与 transcript 所属的 system 渲染器（`opi_ai::transcript::
system_message_text` 与 `render_system_message_update`）共同构成锁定
`utils/text.ts` 工具族：初始渲染为正文加插入序存活 sections、以空行连接
（`null` 值删除 section；空值保留身份但不贡献渲染文本）；更新渲染按名框
架化每处变更（`Updated system prompt section "n":\n\n…` / `Removed
system prompt section "n".`）。编年重放
（`get_current_system_message` / `get_current_system_prompt`）保留
initial/current 区分：content 追加以空行连接，section 替换保位，删除即移
除，删后重加移至末尾。

## 显式 whole-call retry helper（AIR-065/V-57，任务 20.52）

`opi_ai::retry_assistant_call` 以有界 whole-call 重试包裹调用方拥有的
`produce` 闭包。闭包每次调用都是一个**新 logical call**——helper 绝不重派旧
调用、绝不改动 transcript、绝不重执行工具、绝不重置 prepared call 预算。省
略 policy（或 `enabled: false`）即恰好一次调用。

- 分类（`WholeCallRetryPolicy` + `classify_whole_call_retry`）遵循锁定源的模
  式表：quota/billing/usage-limit 耗尽**先否决**，其后才是暂态族——供应商负
  载与 429/5xx/520/524 状态、网关转发（"unable to forward upstream"、
  "service temporarily unavailable"）、网络/传输失败、流提前结束、流中显式
  重试指引、gRPC `ResourceExhausted`、订阅用量抖动。
- 有效停止（Stop/Length/ToolUse——含未来的 Deferred 变体）与 abort 绝不重
  试；预算耗尽返回最终错误消息。
- 回调顺序精确：`on_retry_scheduled` → 可取消的指数退避（调用方给基数，
  delay 默认 cap 60s）→ `on_retry_attempt_start` → 新调用；
  `on_retry_finished` 在循环结束恰好一次，且仅当确已 schedule 过重试。回调
  失败以 typed `WholeCallRetryError::Callback` 停止循环。
- 退避期间取消返回重标为 `Aborted` 的失败消息——错误文本剔除、部分内容与
  usage 保留。
- helper 自身绝不切换 provider/model/auth（那是调用方工厂的事），也绝不接
  产品恢复；host 重试政策仍属 host。

## 受限采样：strict wire 投影与有界降级变体（AIR-040/V-34，任务 20.11）

`ToolDef.constrained_sampling` 携带逐工具意图——`json_schema` 的
`prefer`/`require`，或带 lark/regex 变体的 `grammar`——适配器在构体时解析：

- strict 投影（`constrained_sampling::make_strict_json_schema`）转换的是
  **私有 wire schema**：所有属性变为 required，可选非空属性包裹 nullable，
  不支持形态（引用、对象/数组 union、tuple、schema 值
  `additionalProperties`、逐 wire 拒绝的关键字如 Anthropic 的数值边界与白名
  单外 format）为 typed 失败。转换使用克隆；trusted 声明永不改动。
- wire 或 schema 不支持 strict 时 `prefer` 投影非严格；`require` 在任何
  派发之前 typed 失败，包括尚无 strict 投影的 wire（Chat 含
  Mistral/OpenRouter、Bedrock、Gemini/Vertex、Azure）——require 约束绝不
  静默删除。返回的显式 null 按**原** trusted schema 归一。
- Anthropic 恰好允许一次 source-qualified strict-limit 降级：携带文档化
  grammar 大小/strict 工具数/union 标记之一的 400
  `invalid_request_error`、且请求确实派发了 prefer-strict 工具时，产出不可
  重试的 `ProviderError::StrictToolLimits`，acquisition 状态机允许一个重
  建变体——只移除 prefer-strict、require 保留、每变体预算 ≤1+N（总计
  ≤2×(1+N)，N=0 仍两次 send，无任何重置路径）。重建在新冻结槽重跑
  `onPayload` 与 post-validation，复用冻结的 headers/auth/model（header
  transform 现为每逻辑调用一次），并在消息上记录一次
  `anthropic_strict_tools_fallback` 诊断。其他 400、回调错误、取消与
  Start 后失败一律不分支。
- Responses/Codex 将 grammar 工具投影为 `custom` 工具，lark 优先 regex；
  不支持目标退普通 JSON function 工具并在消息上记录 loss，支持目标无可用
  variant/schema 则派发前失败。流式将 `custom_tool_call` 原始输入增量封装
  为 `{"<property>":"…"}`（单调增长、单次 close、违规 typed 失败），原始输
  入仍是权威最终参数；replay 以 `ctc_` id 还原
  `custom_tool_call(_output)` 项（function call 保持 `fc_` 前缀）。

## 功能请求回调（AIR-059/V-51 普通切片，任务 20.51）

`callbacks::RequestCallbacks`（随 `Request.callbacks` 携带，默认为空）附加四类
观察者，在真实直连适配器内按其源定义的时机触发——绝不从归一化 `EventStream`
反推，也绝不叠加第二个驱动：

| 回调 | 时机 | 合同 |
|---|---|---|
| `with_header_transform` | 每次派发一次，作用于合并后的每调用集合 | 输出整体替换该集合；非保护项的省略/设置/删除受支持。保护性 identity/auth/signing 名单（保留字列表加 `amz-sdk-*`）绝不进入变换，输出重校验——触碰即派发前 typed 失败。 |
| `with_on_payload` | 采样/compat/wire 投影之后，作用于最终请求体 | `Ok(None)` 保持原体；`Ok(Some(v))` 校验后替换：JSON object、model 身份不变、输出预算不得上调。每个 payload variant 只运行一次——普通获取重试按字节复用冻结体（`ORDINARY_PAYLOAD_VARIANT`）。 |
| `with_on_response` | 响应存在时一次，携带 status 与 headers，先于任何 body 处理或错误映射 | 从未产生响应的失败（连接错误、超时）不通知。 |
| `with_on_raw` | 每个已解析 provider wire 事件，严格先于其归一化发射，按 wire 顺序 | 只读；失败以 typed 错误终止当前调用并保留已交付的 partial。 |

适配器覆盖：Anthropic、OpenAI Chat（含 Mistral 与 OpenRouter profile）、
OpenAI Responses（含 Azure）、Codex 与 Bedrock 接线全部四类（Bedrock raw 事件
为已解析 smithy 帧 `{type, ...payload}`；Smithy 签名头受变换保护）。Google
wire（Gemini/Vertex）支持 payload 与 header-transform 两类，对 response/raw
观察者在任何派发之前以 typed `UnsupportedCapability` 拒绝——两个源事实
（无 HTTP response hook、无 raw 支持）共用一个 fail-closed 门。回调失败是
调用的功能失败：`ProviderError::CallbackPhase`，已脱敏、永不重试、不被遥测
吞掉、不触发新调用。回调是运行时函数值，不是 JSON 配置；本切片只收普通
路径——strict variant 交互（20.11）、新引入的操作 API（20.43–50）与完整
AIR-059/V-51 收口（20.57）是独立切片。

## 估算器、Sampling 与计价元数据（V-45/AIR-053/AIR-062，任务 20.35）

`context` 估算器按每 token 3.5 个 UTF-16 码元（`ceil(chars * 2 / 7)`）与每图
4800 等效字符计算，并在消息粒度向上取整：user／toolResult 消息整块聚合取整一次，
assistant 消息对 text/thinking/tool-call 字符聚合取整一次，system 消息对正文加
sections 取整一次，added／removed 工具列表的序列化 JSON 各自单独取整。最后一个
适用的正向成功 usage 充当锚点（不限模型身份、非 Error／Aborted、时间戳不早于前
缀）；只估算锚点之后的消息，没有同模型／全文下界。独立上报的正向
`Usage::total_tokens` 即使与桶和不同也优先；显式 0 或缺失时仅当父桶已上报才回退到
桶和。未知与零处处区分，子集永不重复相加。

| 参数 | 单位／公式 | 省略 → 默认 | 零／null／未知 | 优先级 |
|---|---|---|---|---|
| 文本估算 | `ceil(utf16 / 3.5)` | 空 → 0 | — | — |
| 图片估算 | 每图 4800 字符 | — | — | — |
| usage 锚点 | total 或桶和 | 无锚 → 内容估算 | total 0 → 已上报则取桶和；未知 → 跳过 | total > 桶和 |
| simple 上限 | `clamp(M)`：窗口 ≤ 0 时 `max(1,M)`，否则 `min(M, max(1, W−E−4096))` | `None` → 模型默认输出 | 显式 `M` 不再按模型 max 二次封顶 | request < 窗口算术 |
| 数字 Claude thinking 预算 | 等级表 1024/2048/8192/16384（xhigh/max → high），再夹取留出回答空间 | 省略预算 → 等级默认 | 低于 1024 的预算在鉴权前拒绝 | 等级默认 < 显式预算 |
| sampling 参数 | 按键合并的 JSON 映射 | 空映射 → 不合并 | 缺失键保持缺失 | 模型基座 < 生效等级 < request |
| temperature | 具名字段 | `None` → 按适配器门控省略 | NaN 省略 | sampling 映射覆盖具名字段（最后合并） |
| 计价 tier | 以 input+cacheRead+cacheWrite 选择，严格大于，取最高，相等重复取首个 | 无 tier → 基础费率 | 构造时拒绝 0 阈值 | 命中费率适用于整个请求 |
| 1h 缓存写入 | 子集按 2× 输入费率 | `None` → 全部短缓存 | 显式 0 有效，与缺失区分 | 子集，永不重复计费 |
| service tier | flex 0.5×、priority/fast 2×（`gpt-5.5`：2.5×）、default 1× | 缺失 → 1× | 未识别的 wire 值保持缺失 | 作用于每个成本分量 |

仅 OpenAI 兼容适配器（Chat、Responses）合并解析后的 sampling 映射，且作为 body 的
最后一步，使模型与 request 的 sampling 覆盖具名请求字段；其他适配器忽略它。
Responses 适配器把响应中的 `service_tier` 解析进 `Usage`。OpenRouter 图片
`parse_usage` 是显式源例外：图片用量保留基础模型费率，不做 tier 选择。opi 没有
Pi-Messages wire，其 native/simple 直传条款（只转发 temperature/maxTokens、不走
公共预算构建器）不适用，没有绕过共享机制的路径。元数据本身永远不会产生新的派发前限制或缩放；重复的静态 `reasoning_effort` compat 键
仍然解析，只有在 AIR-022 替换证明之后才会开始硬报错。

## 非目标

以下是明确的非目标，不得作为当前核心行为出现：

- 上列九种之外的 OAuth 家族。
- 完整 provider/profile catalog、Radius pi-messages 生成，以及可用性检查中的
  自动目录请求。
- 凭据撤销后的自动重新登录。
- 经任意 `extra_headers` 注入鉴权。
- Provider payload/response 流式 hook。
- 大范围新增 first-class provider 列表（兼容 provider 保持为 config-driven
  profile）。
- 图像生成（图片支持仅为输入侧）。
- SDK 拥有的浏览器自动化；打开授权 URL 归宿主负责。
- 面向 package 的 provider 流式 adapter 协议。
- 默认测试中的付费实时 provider 调用（实时测试保持 `#[ignore]` 门控）。
- 复制 pi 的 provider 专用配置文件格式。

## 最小示例

```rust
// Cargo.toml 依赖：opi-ai、secrecy、tokio（features "macros"、"rt-multi-thread"）、
// tokio-util、futures-util。
use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::{
    AuthProvenanceSource, AuthScheme, CompatMetadata, Models,
    StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = AnthropicProvider::new(None);
    let resolver = Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        SecretString::from(std::env::var("ANTHROPIC_API_KEY")?),
    ));
    let mut models = Models::new();
    models.register_route(
        Box::new(provider),
        resolver,
        AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".into(),
        },
        CompatMetadata::default(),
    )?;

    let model = "anthropic:claude-sonnet-4-5-20250514";
    let request = Request {
        model: model.into(),
        system: Some("回答要简洁。".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "你好".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let prepared = models.prepare_call(model, request).await?;
    let mut stream = prepared.start_attempt()?;
    while let Some(event) = stream.next().await {
        println!("{:?}", event?);
    }
    Ok(())
}
```

## 测试分层与 SDK 包

私有/公开测试目录、契约 target 子模块、共享 fixture 和定向命令见[测试组织](tests/README.zh.md)。

WS、TLS 和 crypto 在 `opi-ai` 内遵循相同的测试边界：

| 层次 | 现有覆盖 | 定向命令 |
| --- | --- | --- |
| 私有状态机 | `src/ws/tests` 下的 WS framing/PMD/Close/IO | `cargo test -p opi-ai --lib ws::` |
| 私有 key/signing 所有权 | `src/crypto` 内的 key 准入、独立验证和 worker 取消 | `cargo test -p opi-ai --lib crypto::` |
| 公开传输接线 | 严格 Upgrade/WSS，以及 Models → Chat → HttpClient → loopback TLS | `cargo test -p opi-ai --test ws_upgrade --test tls_contract` |
| SDK 包 | 实际 `opi-ai.crate`、解包后测试和工作区外公共消费者 | `scripts/pi-sdk-conformance/package_slice.py --family <ws\|crypto\|http> --offline ...` |

模块测试就近覆盖私有状态；公共集成测试使用受支持的调用方接口，测试 helper 不暴露
私有 codec 或 signing API。三个模块共享 SDK 的包、版本和 workspace 检查。
打包 family 选择的是场景和依赖图，不代表分别分发的模块。解包后的私有测试、包内
公共 Upgrade 测试和外部公共消费者分别记录；回归计数用于确认测试组完整执行，
不能作为产品验收结论。

私有 WS codec 当前仍没有生产 Codex 调用方。其测试与公开 raw Upgrade 合同不能
证明真实 Models → Codex → Upgrade → codec 路径、continuation/pool 行为或完整
cleanup 验收。

## 模块

`provider`、`message`、`stream`、`model_info`、`api_mapped`、`registry`、
`models`、`provider_headers`、`auth`、`credential`、`catalog`、`http`、`retry`、`model`、
`anthropic`、`openai_chat`、`openai_responses`、`openai_codex_responses`、
`openrouter`、`mistral`、`gemini`、`bedrock`、`azure_openai`、`vertex`、
`config`、`time` 和 `test_support`。

## 许可证

MIT。详见包内 [LICENSE](LICENSE) 与压缩 [NOTICE](NOTICE)。

### AWS Smithy V2 重试模式（Bedrock）

Bedrock 生成发送按锁定的 AWS 模式重试（`bedrock::retry`）：`standard` 与
`adaptive` 是仅有的已知模式（未知 `retry_mode` 是类型化配置失败，绝不
Unsupported 或静默 standard），Smithy2026 常量是构造期配置上的显式冻结分支。
Scoped 输入按来源优先级（环境、共享配置 profile、standard/legacy/3 总次数默认）
解析 `AWS_RETRY_MODE`/`retry_mode` 与 `AWS_MAX_ATTEMPTS`/`max_attempts`，
绝不采样 ambient 进程状态。分类完全精确：429 与限流错误名为 THROTTLING；
`$retryable` trait（`ModelNotReadyException`）、瞬态名单、传输超时/网络失败
以及 500/502/503/504 为 TRANSIENT；其余裸 5xx 为 SERVER_ERROR，客户端错误
（408 的 `ModelTimeoutException`、409、403、无 trait 的 400）为 CLIENT_ERROR
——没有整段 `>=500` 重试，也不套 helper 的 60 秒 hint 上限。每次调用的配额从
500 token 起步（成本 5/10，2026 常量为 5/14；成功返还），
`floor(u * min(base * 2^count, 20s))` 全抖动退避基于 100/500ms（2026 为
50/1000ms）基数，服务器 hint（`Retry-After` 秒/日期与 `x-amz-retry-after`
毫秒）按 `max(delay, min(hint, delay + 5s))` 有界调整。adaptive 模式追加
CUBIC 客户端限速器（半秒桶 EWMA 实测速率、限流乘性削减、立方恢复），
其 send-token 等待并入同一可取消的 readiness 截止。每次发送在线上标记
`amz-sdk-invocation-id`（每逻辑调用一个 uuid）与
`amz-sdk-request: attempt=N; max=M` 头，并以新时间戳重签 SigV4，而冻结的
凭据与 payload 保持字节一致；来源交换发送与生成发送分开计数，配额从不
跨调用。

### ADC 原生来源与 Vertex 双模式 wire（P20-S21-1）

Vertex 认证按锁定的 ADC 家族解析（`auth::adc`）：`service_account`
文件以 RS256 JWT 断言（仅 alg 头、`exp = iat + 3600`）在 canonical
Google token 端点交换；`authorized_user` 文件以四个 form 字段刷新并
独立保留 `quota_project_id`（随生成请求发送
`x-goog-user-project`）；`external_account_authorized_user` 文件以
HTTP Basic 客户端认证在其经校验的 `token_url` 或默认 STS oauthtoken
端点刷新，返回的新 refresh token 在 owner 内原地替换、默认不写回
ADC 文件。显式选择的 direct metadata 来源以
`Metadata-Flavor: Google` 双向校验提供 instance token，超时按冻结的
residency（resident：无超时，否则 3000ms；超时按实测语料为单发送
终局失败）。交换在可注入 transport 上运行固定 Gaxios 重试配置
（允许 POST、额外 3 次、no-response 2 次、状态 1xx/408/429/5xx、
100/500/1500ms 延迟），绝不进入生成重试 owner。token POST 拒绝 redirect；
来源 GET 至多跟随 20 次 redirect，跨 origin 移除敏感 header。
显式 `GOOGLE_APPLICATION_CREDENTIALS` 文件读/解析失败即终局；
well-known 文件（Windows `%APPDATA%\gcloud\...`、Unix
`$HOME/.config/gcloud/...`）缺失表示未配置——绝不触发 metadata 或
gcloud 发现。`VertexProvider` 同时服务 API-key Express
（`x-goog-api-key`、无 project/location 的 collection URL）与 Bearer
ADC 路由（需要 project/location，`global` 固定共享域名）；产品路由
依次优先存储凭据/`GOOGLE_CLOUD_API_KEY`、配置的 access-token 环境变
量、ADC 链。

### ADC external-account 联邦与可执行 supplier（P20-S21-2）

`auth::adc` 同时拥有 `external_account` 联邦家族。file/url supplier 保留
subject token 原始文本（不 trim）或命名的 JSON 字段；AWS（`aws1`）来源
解析 region 与安全凭据（环境变量优先，其次 security-credentials 端点，
可选 IMDSv2 session token；每条 IMDS 腿单发送），并以 SigV4 签名序列化的
`GetCallerIdentity` 请求 —— `x-goog-cloud-target-resource` 按上游在签名
之后合并。certificate 来源解析其配置（显式 location >
`GOOGLE_API_CERTIFICATE_CONFIG` > well-known gcloud 路径），校验 leaf 与
chain，按精确 leaf 顺序规则构造 base64(DER) JSON 链 token，并经冻结
TLS profile 准入的**源作用域 mTLS transport** 交换。executable supplier
在任何 cache 读取与 spawn **之前**检查
`GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1`，将命令解析为直引号感知
argv（绝不进 shell），合并 stdout/stderr，校验响应模式（token-type URN、
配置 output_file 时的 `expiration_time`、过期），malformed cache 为终局、
过期 cache 为 miss，使用选定 helper 的精确环境/cwd 快照，超时或调用方
drop 时结束并收割 Unix process group 或 Windows Job，且绝不改写 output_file。helper 失败的 code/message 为
闭合 `ProviderErrorSummary`，Display、Debug 与安全摘要不保留任意 helper 输出。
RFC8693 STS
交换默认 `https://sts.{universe_domain}/v1/token`，confidential 客户端用
HTTP Basic，无客户端认证时 workforce user project 进入 `options` form
字段。溯源经 `AdcCredentialSource::ExternalAccount` 非机密分类供应者；
`impersonated_service_account` 经下述 source/target 机制组合。AWS subject
supplier 遵守自身 ADC metadata/环境变量合同，不使用 Bedrock profile/SSO/process 链。

### ADC 模拟、递归组合与分键缓存（P20-S21-3）

`auth::adc::impersonation` 拥有 `impersonated_service_account` 家族：
`source_credentials` 经同一 adapter 递归解析（不新增深度上限），target
principal 从 `service_account_impersonation_url` 提取（越界/无法提取为
typed 失败），文件的原始 URL 绝不作为 wire 目的地 —— IAM
`generateAccessToken` 请求按有效 universe domain 重建（显式
`universe_domain` 与 source 不一致时在任何网络腿之前 typed 失败）。body
保留 delegates、target scope 数组与 `"{seconds}s"` lifetime；响应
`expireTime` 按 RFC3339 解析；source 与 target 身份在 `Impersonated`
溯源中保持分离，文件 quota project 经 `x-goog-user-project` 随生成请求。
`external_account` 文件的可选模拟 URL 在 STS 交换后激活（强制
cloud-platform scope），并把联邦 source 包进同一 `Impersonated` 溯源。
`auth::adc::cache` 按钉定 key 语法（config digest 加 principal/target、
delegates、audience、scopes、project、quota、endpoint、universe、
TLS/mTLS identity）集中 owner 的 token 存储：单 flight 获取、一次 5 分钟
eager 窗口、短 TTL token 服务当前调用且不循环刷新、失效后完成的晚
refresh 经 generation fencing 绝不发布。无稳定 wire principal 的来源为
identity-unverified（`stable_identity() == None`）：automatic whole-call
recovery 不得跨 source 变更复用；provider 普通重试继续复用冻结的
per-call token。可能轮转的 ADC 获取由共享 mutation queue 拥有：等待者 drop
不丢弃已开始的刷新；来源替换对晚 token 和轮转的 refresh 材料统一设 fence。
barrier 观察来源操作与 ACK，不启动鉴权。产品与 keychain store 共用该队列。

### SDK OAuth（Anthropic）与 store-ACK 的 `Models::login`（P20-S23）

`auth::oauth` 拥有钉定 pi 1.3.0 wire 形状的 Anthropic 登录机制：token
请求为 JSON（`Content-Type`/`Accept: application/json`），在总登录预算内
施加 30 秒单请求上限；authorize URL 携带精确 scope、S256 PKCE 与独立的
随机 state；回调阶梯首选 `127.0.0.1:53692` 加 `/callback` 路径（redirect
URI 写 `localhost`），失败退到空闲 loopback 端口，最后退到纯手贴。第二种
登录方式 copy-code 面向托管 `oauth/code/callback` 页面，接受手贴的
`code`、`code#state`、query 串或重定向 URL。方式选择、展示与 copy-code 等待
共用配置的总预算和同一 cancellation waiter。token 端点与流程失败保持全
遮蔽的产品摘要——闭合 OAuth 错误类也不外显，上游 body 无法回显。过期时
间按真实服务过期存储；唯一的 5 分钟近过期校正保留在
`OAuthCredential::needs_refresh`。`Models::register_oauth_flow` +
`Models::login(provider, presenter)` 以注入的 presenter 与附着的凭据
store 运行流程：只有等到 store ACK 之后才报告成功，写失败按类型浮出，
未知 mutation 结果保持诚实的 pending。不发生自动浏览器启动、keychain
读取、环境回退或远程撤销。`opi-coding-agent` 只保留宿主策略（TUI
presenter、终端挂起、keychain store）并注册 SDK 流。

### SDK OAuth（OpenAI Codex legacy）（P20-S24）

`auth::oauth::codex` 拥有钉定 pi 1.3.0 `openai-codex.ts` 的 Codex 登录机制：
token 交换与刷新为 form 编码；浏览器路径使用与 Codex CLI 共享的固定
`http://localhost:1455/auth/callback` redirect URI——端口被占用时直接退到
纯手贴且 redirect URI 不变，绝不换空闲端口（`bind_fallback` 二选一：
Anthropic 空闲端口阶梯或 Codex 手贴阶梯）；authorize URL 携带生产 scope、
S256 PKCE、独立 state 与 `id_token_add_organizations`/
`codex_cli_simplified_flow`，`originator` 由
`CodexOAuthProvider::with_agent_name` 捕获（默认 `opi`）。设备码路径钉定 OpenAI
`deviceauth` 端点：JSON 请求体、403/404 pending 怪癖回退、结构化
pending/slow_down 码与 RFC 8628 +5 秒退避（增量与上限仍由宿主套件钉
定）、`access_denied`/`expired_token` 先于状态回退的类型化分类、15 分钟
设备预算，以及用服务器签发的 `authorization_code` + `code_verifier` 面向
托管设备 redirect URI 的交换。刷新按 pi `readTokenResponse` 严格
（`RefreshFieldPolicy::Strict`：三个字段全必需），并从访问令牌 JWT 重新
提取 ChatGPT account id（登录与刷新一致的 `AccountIdMissing` 类型化失
败）。endpoint（`base_url`）随凭据穿过刷新。`Models::login` 以与
Anthropic 相同的 store-ACK 语义注册 `openai-codex`。`opi-coding-agent`
在编译期切换：产品 Codex provider、设备轮询与 account id 提取被删除，
改用 SDK 机制（注册表与测试经 re-export）。

### SDK OAuth（OpenAI ChatGPT direct）（P20-S25）

`auth::oauth::chatgpt` 钉定 pi 1.3.0 `openai-chatgpt.ts`：每次登录以
`dynamic_agent_client` 注册动态客户端，回调与手贴 URL 都必须携带
OpenAI 签发的 client id——form 交换与其后所有刷新复用该 ID，凭据持久
化它（`OAuthCredential`/`Credential::OAuthToken` 新增非密可选
`client_id`），缺失时刷新以类型化的“重新登录”失败。authorize URL 携带
生产 scope（含 `resource.invoke chatgpt.tokens.use.direct`）、S256
PKCE、独立 state 与 nonce、RFC 8707 `resource` 参数、`agent_name_hint`
（默认 `opi`）与安装的稳定 `ext_agent_host_id=urn:uuid:<小写>`——缺失
或畸形 UUID 在任何绑定/HTTP 之前类型化失败。重定向固定为
`http://127.0.0.1:1455/auth/callback`：绑定失败是类型化终端错误（新
`CallbackBindFallback::Required`，绝不套 Codex 手贴阶梯或空闲端口）；
绑定成功时服务器与严格校验的手贴输入竞速——手贴必须是同 origin/path
的完整 URL、匹配 state 与签发 client id（裸 code 或异源均类型化；
`error` 参数为类型化拒绝且不回显提供商文本）。token 响应按 pi
`credentialFromTokenResponse` 严格校验：access/refresh/scope 非空、
scope 含 direct 标志、`expires_in` 正有限、login 要求 `id_token` 存在
（绝不解码）；存储真实过期，唯一 5 分钟近过期校正在
`needs_refresh`。`run_pkce_login` 重构为共享的
`run_pkce_login_with` 缝隙（流提供输入解析与交换；Anthropic/Codex
经基础适配器行为逐字节不变）。`Models::login` 以相同 store-ACK 语义注
册 `openai-chatgpt`；落库恒为 OAuth 类型，API-key-only 消费者
（Decisions classifier）绝不能把它当 API key 解析。宿主信封持久化签发的
`client_id` 与校验后的 `scopes` 列表，save/reopen 与刷新后保留 OAuth 类型。

### SDK OAuth（GitHub Copilot）（P20-S26）

`auth::oauth::copilot` 钉定 pi 1.3.0 `github-copilot.ts`。流程以新的
`LoginPresenter::prompt_text` 询问 GitHub Enterprise URL/域名开始
（空=github.com；非空且不可解析在任何 HTTP 之前类型化失败），并按 pi
`getUrls` 从规范化 hostname 派生全部三个端点。GitHub 设备码流使用固定
client `Iv1.b507a08c87ecfe98` + `read:user`，严格校验响应、要求
`verification_uri` 为 http(s) URL、首轮轮询前等待一个 interval、
`slow_down` 优先服务端 interval（否则 +5s），`access_denied`/
`expired_token` 为类型化终端吊销，未知码闭合。Copilot token 交换严格
（401/403 吊销）；请求 base URL 每次请求从 token 的 `proxy-ep=` 提示派生
（`proxy.`→`api.`），回退企业域与 individual 默认——绝不存储。模型元数据
以限界 429 重试发现（login 2 次/5s，refresh 单次）并按 pi picker/policy
语义解析（enabled-policy 回退仅 individual 端点；宿主注入的静态目录白名
单决定 login 可对哪些 unconfigured 模型以 `{"state":"enabled"}`
chat-policy POST 激活，先 `notify_progress`）。策略激活仅限 login：
refresh 重换存储的 GitHub token、重发现元数据、绝不触碰策略。凭据新增非
密 `enterprise_domain` 与 `available_model_ids`；宿主信封与 token、其他 OAuth
元数据原子持久化这两个字段。存真实服务过期并保留唯一 5 分钟
`needs_refresh` 校正，GitHub
token 留在 refresh 槽。产品 `CopilotOAuthProvider` 协议复制品删除；宿主
注册 SDK 流并以静态目录为策略白名单，请求 base 从 token 派生（legacy
存储 `base_url` 仍优先，保证切换前的凭据不换端点）。

### SDK OAuth（OpenRouter）（P20-S27）

`auth::oauth::openrouter` 钉定 pi 1.3.0 `openrouter.ts`。OpenRouter 用
授权码换永久 API key：authorize URL 只带 `callback_url` + S256 PKCE，
无 `state`、无 `client_id`——CSRF 防线是每次登录随机生成的
`/oauth/callback/` UUID 路由，共享 loopback 处理器现对全部 PKCE 流校验
该路由（错误路由或方法：404 页并继续等待）。临时端口回调与手动粘贴
（完整 redirect URL、`code=` 片段或裸 code）竞速；交换为对
`/api/v1/auth/keys` 的单次 JSON POST（30s 期限），响应必须携带非空
`key`。凭据以 key 为 access、refresh 槽为空、过期为永久哨兵
（9999-12-31——pi 的 `MAX_SAFE_INTEGER` 映射为可表示的最后一刻），
因此 `needs_refresh` 恒为 false、`refresh` 为零网络恒等操作，且流绝不
发起 personalization 请求。共享 runner 新增 `AuthorizeStyle` 与可选的
绑定后进度模板；既有流行为不变。

### SDK OAuth（Kimi Coding 设备流）（P20-S28）

`auth::oauth::kimi_coding` 钉定 pi 1.3.0 `kimi-coding.ts` 与共享
`device-code.ts` 轮询状态机。对 `auth.kimi.com` 的 RFC 8628：一次 form
POST 以固定 client id 启动设备授权（两个 verification URI 都必须为
http(s)；interval/expires 默认 5s/15min），presenter 收到 user code 与
**完整** verification URI；form 编码的 token 轮询先等一个 interval、下限
1s、`slow_down` 采纳服务器 interval 否则 +5s，设备期限耗尽时类型化收尾
（遇过 slow_down 的原因可区分）。`authorization_pending` 继续轮询；
`expired_token`/`access_denied` 为终态撤销；成功必须是严格的
access/refresh/expires_in 三元组并按真实期限存储。每请求 30s 期限；
失败体永不回显。`refresh` 对 429/5xx/传输失败至多重试三次（1s/2s/4s
退避）；401/403 或 `invalid_grant` 为类型化撤销凭据，绝无账号回退。
env 的 OAuth host 覆盖归宿主政策，经构造注入。

### SDK OAuth（Meta 设备流与 Model API key 铸造）（P20-S29）

`auth::oauth::meta` 钉定 pi 1.3.0 `meta.ts` 与共享 `device-code.ts` 轮询
状态机。对 `auth.meta.com` 的 RFC 8628 得到的 identity token 不直接用于
推理，而是在 `api.meta.ai/muse-code/key` 铸造 Model API key（Bearer
identity、`{}` body、`x-api-version: 1.0.0`）。凭据以铸造 key 为
`access`、identity token 为 `refresh`、固定 24 小时期限存储，因此
`refresh` 用同一 identity 重铸——绝无 refresh_token 授权端点、绝无重试
环。铸钥 401/403（登录或刷新）为类型化撤销会话，须重新登录。
verification URI 取可信的 `verification_uri_complete` 回退可信的
`verification_uri`；interval/expires 默认 5s/15min；轮询首询前等待、
1s 下限、slow_down 采纳服务器 interval、设备期限类型化超时。错误码
读取不分状态（与 pi 的 token 检查后 switch 一致）。失败体永不回显；
唯一契约允许的例外是无 key 铸造响应中的可信 http(s) `action_url`
进入 presenter 原因。

### SDK OAuth（xAI 设备流与单遍刷新轮转）（P20-S30）

`auth::oauth::xai` 钉定 pi 1.3.0 `xai.ts`。对 `auth.x.ai` 的 RFC 8628：
form 体携带固定 client id、xAI scope 与 `referrer=pi`。verification URI
仅接受 https；不可信的 `verification_uri_complete` 直接类型化失败而非
回退 plain（pi 先校验再取 `complete ?? plain`）；设备授权要求正有限
`expires_in`，`interval` 缺省 5s。轮询 ok 先验（成功体须含非空 access
与 refresh token），RFC 8628 错误码——含 xAI 特有
`authorization_denied` 拼写——仅在非 ok 应答上映射，并遵循共享状态机
（首询等待、1s 下限、slow_down 服务器 interval 或 +5s、设备期限）。
token `expires_in` 缺省 3600s。刷新为单遍：响应缺 `refresh_token`
保留旧值（sdk-contracts 行 146 fixture），401/403 撤销重登，无任何
重试。存储期限为真实服务期限，5 分钟近过期修正在 `needs_refresh`
单点。失败体永不回显；仅状态与固定错误类。

### SDK OAuth（Radius 网关流与九族组装验收）（P20-S31）

`auth::oauth::radius` 钉定 pi 1.3.0 `radius.ts`。gateway 构造期固定
（缺省 https、去尾斜杠）。`with_identity(id, name)` 捕获 flow 自身的身份与
显示名，同时保留该 gateway；既有构造器默认 `radius` / `Radius`。
Models 注册键仍由调用方独立选择。方法选择经 `select_login_method` 镜像 pi
prompt（browser 推荐）。浏览器模先 `GET {gateway}/v1/oauth` 发现授权端点
（设备模绝不先 discovery），复用共享 PKCE 骨架：固定回调
`http://127.0.0.1:1456/oauth/callback`、Required bind（占用即失败）、
`pi-gateway` client、`gateway offline_access` scope 与 `handoff=url`。
浏览器路径仅接受 callback，绝不启动手动输入 reader。设备模 POST `/v1/oauth/device` 后走共享 RFC 8628 状态机；
verification URI 直通（无 scheme 校验，异于 xAI），denial 仅 RFC 单拼写。
非 ok token 应答按错误码分类；token 响应每种 grant 都要求完整三元组；
刷新为共享 Strict 单遍（无 §6 保留），401/403 撤销重登。存储期限为真实
服务期限（pi 写期 60s skew 不复现，5 分钟修正单点 `needs_refresh`）。
组装测试在单 `Models` 注册全部九族、跨两个 `Models` 共享一个 store，
证明按 id 的 ACK 隔离与跨源 late-write 轮转隔离。


### 将 SDK OAuth 接到实际请求

先附着 `CredentialStore`，再调用 `Models::register_oauth_route(provider,
flow, compat)`；注册不访问来源。该入口将 provider 与登录流绑定到
`auth::store_oauth::StoreOAuthResolver`；`register_oauth_flow` 仍供纯登录注册。
`Models::login` 与产品 `/login` helper 共用 SDK `login_to_store`，完整转发
presenter 操作，凭据与元数据写入 ACK 后才报告成功。`get_auth` 与普通调用
解析选定的存储身份，必要时经同一流刷新、等待持久化，再冻结返回的请求身份。
`OAuthProvider::to_auth` 无 I/O；Copilot 从已确认的 token 派生请求端点。
刷新失败不重新发现其他账号或来源。OAuth 刷新网络的 15 秒上限不包含
queue/lock 等待和持久化 ACK。

离线验收沿真实家族流，经公开 login、store ACK 与 `get_auth` 到已有的
Anthropic、Codex、Chat Completions 与 Responses adapter，包括 Copilot
token 选择的 HTTPS 端点。Radius 覆盖到 gateway 绑定的登录/刷新与公开鉴权；
其 pi-messages 生成 adapter 与动态目录仍归后续任务。有限但不可表示的来源
时长类型化失败；各流保留各自字段要求、缺省与轮询限制。OpenRouter 永久 key
及 Anthropic/xAI 缺失 refresh 字段的行为仍各自独立。

产品构造器现在接收拥有共享队列的 store：`build_harness_collection` 最后增加
可选 `Arc<dyn CredentialStore>`；`CodingHarnessBuilder::credential_store`
附着具体 keychain store；
`NonInteractiveRunner::new_with_resume_runtime_packages_and_auth` 与
`RpcRunner::new_with_runtime_packages_and_auth` 在 `auth_resolver` 后接收
可选 keychain store。生产与 resolver、AWS 来源缓存、ADC owner 使用同一个
store；`None` 用于无 store 的测试组装。

AWS 嵌套 role profile 保留完整 static pair 优先，再沿现有
web-identity/process/SSO/login 来源链，从内到外 AssumeRole。SSO cache
区分 region/account/role/session、endpoint 与来源 revision。AWS 轮转必须
具备自有依赖和共享 mutation queue；缺少 owner 时在轮转前类型化失败。
旋转后的来源材料与真实过期保留在内存，即使标准缓存文件的 best-effort 保存
失败。receipt 确认来源内存更新，不宣称文件持久化成功。失效 fence 拒绝晚
内存/文件写；fence 失败不表示远端轮转已回滚。login-session 普通刷新失败中
原有可 Continue 的分类保持不变。


`AdcNativeOwner` 可直接作为 `AuthResolver` 注册。其 `check_configured`
通过同一选定 context 检查环境变量与文件存在性，不读/解析凭据内容、不刷新
令牌，也不联系 metadata。文件缺失表示未配置，存在性探测失败按类型浮出；
存在但畸形的文件在实际解析校验前仍算已配置。可选的
`AuthResolver::check_configuration` hook 供 `check_auth` 与可用性检查使用，
opaque resolver 保留其来源分类检查。产品 Vertex checker 先保留存储的
Express/API-key 与配置 Bearer 的优先级，再检查同一个 ADC owner。


存储 OAuth 解析在串行 mutation slot 内首次选择当前凭据，包含 fresh-token
读取，避免等待锁时保留先于 logout/replace 的旧身份。替换附着的 store 或
upsert helper 绑定的 OAuth flow 会将新调用重绑到该 store/flow；已捕获的
调用保留其冻结绑定。显式由调用方拥有的 `register_route` resolver 保留自身归属。


Windows helper 以 suspended 状态启动，先加入自有 kill-on-close Job，再恢复执行。
关闭 owner 会结束其后代进程，即使最初 helper parent 已退出。必要的
`windows-sys` 调用封装在私有 Windows owner，使用 `OwnedHandle`，不添加
unsafe `Send` 实现。Windows-only 的 `windows-sys` 0.61.2 依赖在已有
Job/ToolHelp/Threading feature 集上增加 `Win32_Security`，不新增包版本或
build script。Unix process group 与 AWS 现有 10 秒/64-KiB 宿主策略保持不变；
本变更不宣称完整 native-platform 验收。


`OAuthProvider::{name, is_subscription, login_label}` 可经 `dyn OAuthProvider`
读取非密 flow 元数据，不执行鉴权或来源 I/O。自定义实现默认 `name = id()`，
其余两个可选字段为 `None`。九种 SDK 流保留固定 Pi 的显示值及 optional 区别：
OpenRouter 与 Radius 不声明 subscription flag，没有 login label 的流不合成
通用 label。Radius 在方式选择、请求溯源和类型化取消/撤销中使用捕获的自身身份，
刷新仍绑定配置的 gateway 与存储的 account 元数据。fresh 与 refreshed
Radius 请求鉴权均忽略凭据 `base_url`，路由权威始终属于该 gateway。不新增通用 flow inventory
或完整 provider/profile 枚举。

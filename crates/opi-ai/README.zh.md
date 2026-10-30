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
| `azure_openai` | `azure` | Azure OpenAI deployment 专用 Chat **与** Responses 双 wire（任务 20.38） |
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
| `Models` / `AuthDescriptor` / `AuthCheck` | unstable-0.x 唯一 driver，位于 `ProviderRegistry` 之上：Provider+模型查找、脱敏鉴权状态、OpenAI-compatible 兼容性元数据、派发与原子动态目录 refresh。native/simple/direct 与高级入口共用校验及调用生命周期；native options 保留适配器预算语义，simple/高级准备采用共享输出预算。 |
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

`prepare_call` 与中性 simple 调用在鉴权前把省略的 `Request.max_tokens` 解析为所选
完整 model record 的 `max_output_tokens` 容量，包含适用的 model override 和刷新后的
catalog。显式 request 上限
仍作为上限。若已启用 thinking 的 budget 大于或等于该默认输出上限，preparation 会在
鉴权和派发前返回类型化 capability 错误。每次 attempt 都复用解析后的上限。
隐式与显式上限都会按估算的剩余上下文收缩，预留 4096 token 安全余量，最少保留一个
输出 token。无法装入收缩后上限的 thinking 会缩小，尽可能留出 1024 token 回答空间。
非 adaptive Anthropic 按 thinking 等级解析省略的预算；无法同时容纳最小 thinking
预算和回答空间的上限会在鉴权前被拒绝。adaptive effort 不使用该数字预算下限。
`context` 模块估算原始 canonical 消息编年，包含后续 System 变更及工具声明；最后一个适用的正向成功 usage
（非 Error／Aborted、时间戳不早于前缀、不限模型身份）充当锚点，只估算锚点之后的
消息，不再有同模型／全文下界。类型化 native `ChatApi` 调用仍校验 route 与能力，
但保留 native 的省略及显式上限，不应用 simple 预算默认值或夹取。直接调用 provider
adapter 保留其 wire 默认值。这些是每个 request 的输出上限，与产品的轮数预算相互独立。

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

`CompatConfig`（任务 20.36，AIR-022/V-11）应用单一归一化规则：每个策略
字段都是叠加在适配器冻结 provider 推断（按 provider id 或 base-URL host 推断
`store`/`instruction_role`/`thinking_format` 与字段默认值）之上的显式值；
显式配置逐字段胜出，模型级覆盖再胜过 profile：

| 标志 | 作用 |
|------|------|
| `store` | `Omit`/`SendFalse` store 字段策略；标准 OpenAI 基线发送 `store: false`，非标准端点省略该字段。 |
| `instruction_role` | `Auto`/`System`/`DeveloperForReasoning`/`Developer` system 消息角色策略；基线仅在标准端点对 reasoning 模型使用 `developer`。 |
| `thinking_format` | 十一种已声明 reasoning wire 格式之一（见下）。 |
| `supports_reasoning_effort` | 端点是否接受标准 effort 字段；`false` 不禁用格式自身的开关。 |
| `usage_in_stream` | 请求 `stream_options.include_usage`；推断基线启用，显式 `Some(false)` 关闭。 |
| `max_tokens_field` | 输出上限字段名覆盖（标准端点 `max_completion_tokens`，冻结的 legacy 兼容集 `max_tokens`）。 |
| `tool_result_name_field` | 在工具结果消息上回显工具名。 |
| `strict_tool_schema` | 发出 strict JSON-schema 工具定义。 |
| `cache_key` | 发送 provider 的 prompt-cache key（cache-affinity 提示）。 |
| `send_session_affinity_headers` | 把请求 `session_id` 映射为兼容的会话亲和 header（OpenRouter 推断启用）。 |
| `require_assistant_after_tool_result` | 在工具结果与下一条 user 消息之间插入固定 assistant 桥接；assistant 工具调用轮次用 `content: ""` 而非 `null`。 |
| `requires_thinking_as_text` | 将 assistant thinking 以纯文本部分回放。 |
| `requires_reasoning_content_on_assistant_messages` | 为回放的 assistant 轮次标记空 `reasoning_content`（DeepSeek 推断）。 |
| `zai_tool_stream` / `vllm_priority` | 仅显式配置的 `tool_stream` 与 `priority` 附加项；从不推断。 |
| `thinking_token_budget_field` / `supports_thinking_token_budget` | 顶层 reasoning 预算字段（vLLM `thinking_token_budget`、Qwen/DashScope/SGLang `thinking_budget`、llama.cpp `thinking_budget_tokens`；alias 选择 vLLM 字段）。 |
| `chat_template_kwargs` / `chat_template_args` | `$var` 模板值（`thinking.enabled`、`thinking.effort`、`thinking.budget`；无 effort 时 `omit_when_off` 丢弃该键）。 |
| `chat_completions_path` | 相对 `base_url` 的 chat completions 端点路径（默认 `/v1/chat/completions`）。 |

已退役键 `system_role_override`、`supports_store`、`supports_developer_role`
与静态 `reasoning_effort` 字符串在每个配置位置（catalog `x-opi.compat`、产品
TOML profile 与模型 compat）都会报出显式迁移错误——绝不静默忽略。请求
thinking 意图与所选 `ModelInfo::thinking_level_map` 是 reasoning wire 输出的
唯一来源。

工具结果按锁定来源投影：文本（或 `(see attached image)` / `(no tool output)`
占位符）承载于 `role: "tool"` 消息；当模型接受图片时，图像部分承载于一条尾随
的 user 消息（标签 `Attached image(s) from tool result:`）；错误结果保留确定的
`[tool_error]` 前缀。

### 已声明 thinking 格式（on/off/null/省略）

`openai` 发送 `reasoning_effort`（on 用映射值，off 用模型声明的 off 值）；
`openrouter` 发送 `reasoning: {effort}`；`deepseek` 发送
`thinking: {type: enabled|disabled}`（off 不可映射的模型从不发送关闭标记）；
`together` 发送 `reasoning: {enabled}`；`baseten` 发送 `chat_template_args`
外加 off effort 值；`zai` 发送带 `clear_thinking` 的
`thinking: {type: enabled|disabled}`；`qwen` 发送 `enable_thinking`；
`qwen-chat-template` 发送固定 `chat_template_kwargs`；`chat-template` 解析配置的
`$var` 值；`string-thinking` 发送 `thinking` 字符串；`ant-ling` 仅在设置了
effort 时发送 `reasoning: {effort}`。

### Native 与 simple 选项（ONB-03）

| 选项 | 接受类型 / 单位 | 省略 · 默认 | 优先级 · wire 字段 · 类型化拒绝 |
|------|----------------|-------------|----------------------------------|
| `tool_choice`（native） | `OpenAiToolChoice`：`None`/`Auto`/`Required`/具名函数 | 省略则字段缺席 | native 选项独占该字段；wire `tool_choice`（字符串或 `{type: function, function: {name}}`）；simple 入口仅接受中性 `Auto`/`None`。 |
| `reasoning_effort`（native） | `ThinkingLevel`：`minimal`/`low`/`medium`/`high`/`xhigh`/`max` | 省略 = 无 effort 意图；off 按所声明格式的 off 投影 | native effort 取代中性 `Request::thinking`；wire 字段随 `thinking_format`；不支持的级别在 auth 前以类型化 `Config` 预检失败。 |
| `thinking_budgets`（native） | 每级别 token 数（`u64`） | 省略的级别使用来源默认 1024/2048/8192/16384（`xhigh`/`max` 映射为 `high`） | 仅经配置的预算字段发送，并夹取以保证响应上限下至少剩余 1024 token；小于等于零的预算省略。 |
| 中性 effort（simple） | `Request::thinking` 级别 | 默认禁用 | simple 入口把不支持的级别夹取到最近的受支持级别（先向上再向下）；native 入口保留精确级别。 |
| `store` | — | 推断基线 | 见 compat 表；显式配置胜出。 |

`ModelCompatOverride` 在 profile 默认值之上叠加模型级的 `instruction_role`、
`store`、`thinking_format` 与 `max_tokens_field` 覆盖（model 优先于
provider）。静态的按 profile 请求 header（`extra_headers`，用于会话亲和 /
路由）是独立的 profile 配置字段，经 `OpenAiChatProvider` 构造传入，不是
`CompatConfig` 标志。

OpenAI Responses 原生语义（任务 20.37，`OpenAiResponsesOptions`）：

| 选项 | 接受的类型/单位 | 省略 · 默认 · null · false/零 | 优先级 · wire · 类型化拒绝 |
|------|----------------|-------------------------------|----------------------------|
| `reasoning_effort`（native） | `ThinkingLevel`（`minimal`..`max`） | 省略 = 无 native effort；wire `reasoning.effort` 取自模型 `thinking_level_map` | native effort 取代中性 `Request::thinking`；运行 effort 同时请求 `include: ["reasoning.encrypted_content"]`。不支持的级别在 auth 前以类型化 `Config` 预检失败；simple 条目改为就近收敛。 |
| off / 关闭（两个条目） | — | 支持推理的模型发送 `reasoning: {effort: "none"}`（映射的 off 投影；`github-copilot` 路由跳过） | off 只发送 effort——无 summary、无 `include`。 |
| `reasoning_summary`（native） | `ResponsesReasoningSummary`：`Auto`/`Detailed`/`Concise`/`Null` | 省略**与**显式 `Null` 都发送 wire 默认 `"auto"`（源 `\|\|` 语义）；仅有 summary 而无 effort 时 effort 默认 `medium` | wire `reasoning.summary`。 |
| `service_tier`（native） | `ResponsesServiceTier`：`auto`/`default`/`flex`/`priority`/`fast` | 省略则字段缺失 | wire `service_tier`，字符串原样。响应观察到的 tier 进入 `Usage::service_tier`；本 wire 不计算分层成本倍率。 |
| `tool_choice`（native） | `ResponsesToolChoice`：`None`/`Auto`/`Required`/具名函数 | 省略则字段缺失 | wire `tool_choice`（字符串或 `{type: "function", name}`）；simple 条目只接受中性 `Auto`/`None`。 |
| `store` | — | 除非 catalog/profile 显式钉住其他值，恒为 `false`（源无条件发送 `store:false`） | catalog compat > provider config > 默认 `false`。 |
| `max_output_tokens` | `Request::max_tokens`（token） | 省略或零则字段缺失 | 正值以最小 16 发送；simple 调用在此 wire 映射前先解析共享预算。 |
| 加密推理 / 回放 | 持久化 thinking `signature` | 未签名 thinking 绝不捏造 item | 运行 effort 时请求 `include: ["reasoning.encrypted_content"]`；reasoning item 以序列化 wire item 作为签名持久化，回放时（仅同身份回合）原样推回 `input`；`xai` 推理模型恒请求加密内容；终态输出回填补齐逐帧缺失的签名。 |
| 会话中 effort 戳 | catalog compat `supportsMidConvoEffort`（默认 `false`）+ 回放回合的 `provider_thinking_level` | 默认关闭；未记录级别的回合无戳 | 每个记录了级别的早期同模型回复前插入一个 `configuration_update`，末尾一个携带当前 effort；请求 `reasoning.effort` 恒为 `medium`；回复记录自身级别供下一回合使用。 |
| 会话缓存 | `Request::session_id` + `cache_retention` | `Disabled` 全部抑制；否则发送截断（64 字符）的 `prompt_cache_key` | `cache_retention: Long` 追加 `prompt_cache_retention: "24h"`。显式 prompt-cache 模式（`prompt_cache_options`）未采纳——无 opi catalog 消费者。 |

静态 `reasoning_effort` 仅保留为遗留兼容性元数据；`request.thinking`
与所选 `ModelInfo::thinking_level_map` 是 Chat 和 Responses wire 输出的权威来源。
`previous_response_id` 刻意缺省——Responses 请求按
Chat-Completions 类比构造，因此服务端响应链未被接入。

### Azure OpenAI 双 wire（任务 20.38）

Azure 同时保留两条 wire，并按钉住的源 `azure-openai-config.ts` 显式映射
deployment/resource/api-version：

| 映射 | 优先级 | wire 效果 |
|---|---|---|
| Base URL | 显式 `azure_base_url` → 资源名（`https://{resource}.openai.azure.com`）→ 模型 `baseUrl` → 类型化 `Config` 失败 | 携带 `/openai[/v1][/responses]` 路径的 Azure 主机规范化为资源根；非 Azure 主机保留路径 |
| API 版本 | 显式 `azure_api_version` → 默认 `v1` | 每个请求携带 `?api-version=…`（取代旧默认 `2024-06-01`） |
| Deployment | 显式 `azure_deployment_name` → provider `modelId=deployment` 映射 → 模型 id | 同时进入 URL 路径 `/openai/deployments/{d}/…` **和** 请求体 `model`；catalog id 不变 |

Responses wire（`WireApi::AzureOpenAiResponses`、`azure-openai-responses`
记录、经 `Models::register_chat_api` 注册的公开 `AzureOpenAIResponsesChatApi`）
复用完整的 Responses 参数装配——`store: false`、off-effort
`{"effort":"none"}`、summary 默认 `auto`、加密推理 include/签名/回放、
16-token 输出下限——运行在 Azure 传输（`api-key` 头、deployment URL、
azure 状态映射、终态 duration、冻结 telemetry producer）之上。
`AzureOpenAIOptions` 携带 native 清单：`reasoning_effort`（类型化预检；
simple 条目就近收敛）、`reasoning_summary`、`tool_choice` 与四个 Azure
映射字段。Azure 刻意不含 `service_tier`（源缺省；本 wire 无 Responses
服务层级倍率）。Azure 不接会话中 effort 戳（源 azure `buildParams` 从不
传递）。Chat wire 保持既有行为（记录默认走 OpenAI-completions 投影），
两条 wire 都只接受 `api-key` 头认证。

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

`ws::upgrade` 在共享的 source-qualified `HttpClient` 上按其显式代理/TLS profile
执行严格动态 Upgrade。经典 Upgrade 使用该 client 禁用重定向的私有 HTTP/1.1 传输，
保留同一份冻结的 trust、mTLS 身份与代理配置，不重新发现来源；普通请求保留其
已配置的重定向策略与 H2/H1 行为。该桥拥有请求形状（`Connection:
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
标志（Anthropic 错误 result 的图像拆到该轮全部工具结果之后，并与其同处
一条 user 消息——任务 20.39）；`details` 绝不进 wire，URL 源原样透传不
下载。`ReplayProjection::Strict` 用类型化错误拒绝同
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
最低 reader 版本、provider 集合、每个条目的数据合同（identity、模态、能力、cost）、每 provider
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
不同 artifact；opi 特有保真度（streaming、cache 开关、保持原序的计价分层，含零阈值及重复项、
base 与 thinking 等级的 sampling 映射、精确 thinking 映射、含 strict-tool 支持的
wire compat 载荷）经显式 `x-opi` 扩展往返，纯 v1 文档
以封闭默认值解码。

`Models::refresh(allow_network)` 是唯一刷新入口：`refresh(false)` 不做
任何 OAuth/网络工作，仅恢复 store 缓存（`RefreshStatus::OfflineRestored`）；
`refresh(true)` 按确定性排序顺序抓取已注册 provider，再按注册顺序抓取
已注册 catalog source。结果按 provider 隔离：成功者发布，失败者保留
该 provider 的旧数据并以脱敏错误上报（`RefreshReport` 中的
`RefreshOutcome::{Published,Failed,Restored}`），不会抹掉其他 provider 的
catalog。未注册显式 catalog source 时，native provider 的 `Ok(None)` 恢复其内置
catalog；存在显式 source 时，该 native 未发布结果保留最后已知 source catalog。
权威的 `Ok(Some([]))` 仍会将其替换为空 catalog。`refresh_with` 观察
`CancellationToken`——提交前观察到取消即保留先前发布，包括 source 在观察取消的
同次 poll 中返回成功的情况。无 store 时，最终 token 检查是同步内存提交的起点。
已 prepare 的调用在刷新后保持冻结绑定。`DynamicCatalogSource` 是
宿主自定义 catalog source 的公开接缝；`HttpCatalogSource` 经池化连接
抓取单份 distribution-schema-v1 文档，失败全部脱敏。SDK 不注册任何
source、不附加任何 store、没有 pi.dev/默认启动抓取策略。

## Telemetry 载体：冻结 parent Span 与 Dispatch（任务 20.54，DEL-12）

`telemetry` 提供进程本地 `TelemetryContext` 载体——P20-TELEMETRY 共享决策
owner；closure test 位于 `tests/telemetry_carrier.rs`。

- **`TelemetryContext::from_span(span)`** 通过 `Span::with_subscriber` 冻结 parent
  `tracing::Span` 与其自身 `Dispatch` 的配对。即使创建或 poll 时另有当前
  subscriber，配对仍由源 span 决定。disabled span 退化为显式
  `noop()` 态。`Clone` 保留同一配对；上下文 `Send + Sync`。
- **`in_scope(FnOnce)`** 在冻结配对下（dispatch 设为段默认、parent 仅段内
  enter）恰好执行一次同步 body，并在返回前恢复 caller 上下文。
- **`instrument(Future) -> Instrumented<F>`** 是惰性 `Unpin` 包装器（boxed
  inner future，零 `unsafe`）：每次 poll 与 drop（含未 poll 的 drop）运行同
  一 scoped 段。无 enter guard 跨 await；每次外层 poll 恰好 poll 一次内层；
  `Pending` 与输出保真。
- **`noop()`** 在 no-op dispatcher 下运行同样段落：业务仍恰好执行一次，段内
  观测被显式静默而不泄漏到 ambient dispatch。包装段外的独立宿主日志仍使用自身
  subscriber。

`Request::telemetry_context: Option<TelemetryContext>` 显式将该配对带入真实 Codex
producer，覆盖其异步回调与 SSE 回退。省略时不隐式捕获 ambient 上下文。载体不序列化、
不上 wire，不安装 global subscriber、exporter、采样政策或自动 SDK root span。
全 SDK 操作传播仍归后续工作（20.55）；真实 Codex 路径另由
`tests/codex_telemetry.rs` 覆盖。

## 实例 Resource Generation／Pool 清理／Await 释放（任务 20.17）

Codex WebSocket 族现在为每次调用注册一个 resource generation，并通过公共
`Models::cleanup_session_resources` 入口暴露按 session 的清理
（`codex_resources` 模块；测试位于 `tests/openai_codex_responses.rs` 与
`tests/resource_cleanup.rs`）。

- **注册：** 调用首次 poll 时、在任何 auth、transform 或 network await 之前，
  原子注册 generation。纯 `Sse` 偏好调用不注册。即使 cache retention 为
  `Disabled`，清理身份仍是原始完整 session id。阶段
  （`prepared`/`connecting`/`busy`/`temporary`/`active-fallback`）覆盖同一调用
  的 SSE 回退。
- **Cutoff 语义：** 返回的 cleanup Future 自有选定的 owner 与 session，在其
  **首次 poll** 建立所有 cutoff；这不同于 credential mutation barrier 在构造时
  同步捕获 cutoff。`Some(非空 id)` 选择匹配的旧 generation 与池化连接；`None`
  选择全部，含 sessionless；未匹配 id 为保留 idle 复用的 no-op；`Some("")` 为
  零副作用的 typed 失败。cutoff 后禁止迟到挂接、发布、重连及转入 SSE。此后才首次
  poll 的调用获得新 generation；已登记的 `PreparedProviderCall` 保持旧 generation，
  即使 attempt 从未 poll。
- **释放：** cleanup 等待库拥有的 producer、连接及有界关闭。调用方保留但暂停的
  preparation/auth Future 无须再次 poll，cleanup 即可完成；其下一次 poll 会看到
  已锁存取消。丢弃已 poll 的 cleanup Future 不撤销关闭标记，再次 cleanup 可继续等待
  剩余退出。Drop 只发起释放，不宣称 await barrier 完成。被移除的连接发送 Close 并
  flush 后有界等待确认；某一步失败仍尝试其余释放，以
  `CleanupError::Incomplete { report }` 返回已确认计数及未确认效果。已交付终态保持
  最终性，协议释放可继续进行。
- **隔离：** `SessionResourceOwner::new_namespace` 为每个 `Models` 注册建立独立
  namespace，即使两个实例包装同一 provider/client 且使用同一完整 session id。
  调用方父 token、凭据、catalog 及独立 store-owned rotation 保持原样。owned call
  guard 在 `Models` drop、remove 或 replace 后仍持有资源；成功调用可保留 idle 缓存连接。
- **分发：** 持有资源的 provider 公开 `Provider::session_resources`，并经
  `stream_resource_attempt` 转交已登记 lease。`Provider::stream_model_resource_attempt`
  还携带调用方冻结的完整 record 与 simple options；默认实现转交 resource 与 attempt
  所有权。类型化 `ChatApi` 注册提供 `session_resources`、`stream_resource_prepared`
  以及携带所选 record 的 `stream_model_resource_prepared`。默认的
  `validate_native_request` 与 `simple_options_for` 钩子保留既有自定义适配器行为；
  消费元数据的适配器覆盖它们以校验 native 字段并映射其使用的中性选项。
  无资源默认实现不产生效果。
  `validate_simple_options` 默认拒绝显式中性选项；只有 simple 映射确实消费某个
  选项时，适配器才接纳该选项。
  `Models` 尝试所有 owner 的释放并聚合结果。

## 真实 Codex WebSocket 传输：Auto／Delta／续传（任务 20.16，DEL-07）

Codex adapter 现持有经严格 Upgrade 桥进入私有 frame/PMD/Close owner 的真实
WebSocket 生产路径（`openai_codex_responses/ws_transport.rs`；测试位于
`tests/openai_codex_responses.rs`）。

`openai_codex_responses::CodexChatApi::new(Arc<OpenAiCodexResponsesProvider>,
ModelInfo) -> Result<Self, ProviderError>` 经 `ChatApi<CodexOptions>` 注册真实
适配器。将适配器、resolver 与 auth source 传给 `Models::register_chat_api`，用
`handle.options(opts)` 创建带标签的 options，再调用
`Models::stream(record, request, options)`。non-exhaustive 的 `CodexOptions` 以
`Default::default()` 构造后设置公开字段。native 默认值保留原生 reasoning 的省略/off
映射；中性 simple 调用则映射 `Request::thinking`。两个入口都消费本次调用所选并冻结的
完整 `ChatModelRecord`（包括 thinking 映射），而非构造时的示例 record 或同 ID 的
catalog 项。native Codex 不消费中性 `Request::thinking`，包括中性入口不支持的级别；
公共结构校验与所选模型校验仍在 auth 前执行。options 按调用冻结，不修改已注册
provider，也不以 `sampling_params` 承载 native 参数。

| 输入 | 类型／单位 | 省略、零或 null | 优先级与 wire 字段 |
|---|---|---|---|
| `reasoning_effort` | `CodexReasoningEffort` 枚举 | 省略采用受支持模型的 off 映射；`Some(None)` 显式请求 off | native effort 采用模型映射或 native 拼写，独立于中性 thinking；`reasoning.effort` |
| `reasoning_summary` | `CodexReasoningSummary` 枚举 | 序列化 effort 时，省略或 `Null` 变为 `auto`；否则不使用 | native summary；`reasoning.summary` |
| `service_tier` | `CodexServiceTier` 枚举 | 省略保持缺失；`Some(Null)` 发送 JSON null | 显式 native 值；`service_tier` |
| `text_verbosity` | `CodexTextVerbosity` 枚举 | 省略 → `low` | 显式 native 值覆盖默认；`text.verbosity` |
| `tool_choice` | `CodexToolChoice` 枚举 | 省略 → `auto`；枚举 `None` 发送 `"none"` | 显式 native 值覆盖默认；`tool_choice` |
| `SimpleOptions::tool_choice` | `Option<SimpleToolChoice>`（`Auto` / `None`） | 省略 → Codex `auto` | Codex 中性映射至 `tool_choice`；当前不支持此项的 route 在 auth 前拒绝显式值 |
| `Request::temperature` | 可选无量纲数值 | 省略或非有限值 → 缺失；零有效 | request 值；`temperature` |
| `websocket_connect_timeout` | 可选 `Duration` | 省略 → 15 秒；零禁用连接计时器 | 按调用冻结的本地连接预算；独立于 `Request::timeout`，不序列化 |
| `Request::max_tokens` | 可选 token 数 | Codex 不发送输出上限 wire 字段，包含显式零 | simple 准备保留共享预算语义；native 调用在 wire 映射前保留 request |

封闭 native 枚举拒绝无效序列化拼写。无效完整 record 或错误 wire 以 typed 配置错误
失败；注册、options 身份或类型不匹配在 auth/网络工作前失败。公共 transport、cache、
取消与 timeout 输入仍在 `Request` 上。

- **传输解析：** `Request::transport`
  （`TransportPreference::{Auto, Sse, Websocket, WebsocketCached}`，默认
  `Auto`）先行解析；省略与显式 `Auto` 遵循同一 delta 规则。专用 HTTP/1.1 Upgrade
  client 禁用重定向，并保留 `HttpClient` 冻结的 TLS 身份、信任与 proxy 输入。
  普通 HTTP 保留其已配置的重定向策略；该策略不会强制 `Auto` 转为 SSE。
- **真实链路：** `response.create` 以带掩码的 text 帧发送；事件经与 SSE 完
  全相同的分类器解码，两种传输发布一致的公开事件；临时连接完成显式客户
  端 Close；无终态的 EOF 绝不算成功；`Request::timeout` 即 WebSocket 空闲
  超时（零禁用该计时器）；取消覆盖活跃与空闲两态。下次 acquire 淘汰释放后
  空闲满 5 分钟或连接年龄满 55 分钟的连接及其续传状态；不打断 busy 调用。
- **Delta／续传**（Auto/WebsocketCached 且有 session）：仅当非 input 字段
  未变且“上次完整 input＋实际响应 items”构成新 input 的精确前缀时发送
  `previous_response_id`＋delta input，否则发送完整 input。仅成功终态且带
  response id 才发布续传。无 session 或 `Disabled` 保留策略使用临时连接。
- **有界分支：** `previous_response_not_found` 在公开输出前恰允许一次完整
  input 重试；`websocket_connection_limit_reached` 在生成开始前恰允许一次
  重连；WS→SSE 回退仅在首个有效 provider 事件之前的传输类失败上触发并按
  session 粘滞。abort/API/auth/协议失败绝不伪装为传输丢失；已交付输出后的
  错误终止该流并保全 partial。有效且非空的 provider event type 即封闭回退和
  重连，即使 mapper 未发布公开事件。合法 SSE 回退保留
  `codex_websocket_fallback` 诊断及原有 sampling 诊断，仅含固定原因，无 raw 错误文本。
- **Upgrade 桥：** `WsUpgradeRequest::headers` 增加调用方握手头（握手保留
  名被拒绝）。
- **打包：** `ws` conformance family 新增真实链 consumer 检查（外部
  `Models::stream_simple` -> Codex -> Upgrade -> 私有 codec，本地脚本
  peer），三图运行并单独报告。

## MockProvider 多操作脚本测试支撑（任务 20.53.8，DEL-11）

`test_support` 按钉定 `providers/faux.ts` 为既有 `MockProvider` chat 脚本补齐
三种形态——不建第二 mock driver，公共 API 不变：

- **`MockResponse::DelayedAfter { pending_attempts, retry_after_ms,
  events }`**（钉定 `pendingFetches`）：前 `pending_attempts` 次 prepared
  attempt 各产出携带 `retry_after_ms` 的
  `ProviderError::RateLimited`（钉定 `pollAfterMs` 的 chat 级对应物，复用既有
  retry 机制而非计时器）；下一次 attempt 产出脚本 events。
  `pending_attempts: 0` 与 `Events` 完全一致。单个变体值跨这些 attempt，
  对应钉定的按条目递减：首个条目在倒计时期间保持队首，其余按序消费。
- **`MockResponse::HangingUntilCancelled`**：不产出任何事件直到请求的
  cancellation token 触发，随后以 `ProviderError::Cancelled` 终止；预取消立即
  失败。为钉定 signal-aborted-to-error 在 attempt 边界的形态，以
  `futures_util::stream::once` 叠加 `CancellationToken::cancelled` 构造——无计
  时器、无新依赖。
- **`blocks_response(Vec<AssistantContent>)`**：有序多块/tool
  fixture——text 块发 `TextDelta`、tool-call 块发 `ToolCallEnd`，
  `content_index` 与块位置一致；含任意 tool call 则 `Done` 用
  `StopReason::ToolUse`，否则 `Stop`。每个 partial 仅包含已发出的 block 前缀，
  先前快照保持独立。按契约对
  `Thinking`/`RedactedThinking` 块 panic。既有
  `Events`/`Error`/`EventsThenError` 语义与 `stream_prepared`
  队列断言不变；deferred record 及其脚本随 20.45-50 交付。

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
  空输入→`{}`；完整文档（经转义修复）用于显示；否则依次对原始文本、修复
  文本做前缀补全（闭合悬挂引号/括号、悬挂 key/value→`null`、截断数字与
  字面量保守补全、忽略首个完整顶层值后的垃圾）；最终兜底为合法空对象。
  永不失败，也永不凌驾终态结果。
- **`finalize_tool_arguments`** 是完成 tool call 的公共 AIR-048 契约：空输入
  终化为 `{}`；完整对象保留其 owned raw 文本；仅修复非法转义或字符串原始控制字符后
  成为完整对象时返回修复文本；
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
- Chat/Responses 的 HTTP 429 从指定 JSON error 字段保留 quota/billing/usage-limit
  分类，以固定安全消息标签表达。typed `RateLimited` 原因与 retry hint 不变，
  acquisition 保持其独立 429 发送策略；不暴露 raw 错误体。
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

## Anthropic Messages native 选项与 inline-tools 时序（任务 20.39）

公开 direct adapter `anthropic::AnthropicMessagesChatApi` 经
`Models::register_chat_api` 注册（api id `anthropic-messages`），携带类型化
`AnthropicMessagesOptions`——钉住的 pi 1.3.0 native 清单：

| 选项 | wire 效果 |
|---|---|
| `thinking_enabled: Option<bool>` | 三态：省略沿用 neutral 请求意图；`Some(false)` 在 reasoning 模型上投影类型化 `{"type":"disabled"}` 块；`Some(true)` 启用 native thinking |
| `thinking_budget_tokens` | 启用的数值 thinking 遵循源 `\|\| 1024` 语义：缺失**或为 0** 发送 native 默认 1024；非零预算原样透传 |
| `thinking_display` | `summarized`（启用默认，adaptive 与数值皆然）或 `detailed`；启用时恒发送 |
| `effort` | 仅 adaptive thinking：类型化预检拒绝不支持的级别，wire 收到 `output_config.effort`；数值 thinking 忽略 |
| `interleaved_thinking` | 默认 `true`（启用的非 adaptive thinking 附带 beta `interleaved-thinking-2025-05-14`）；`Some(false)` 省略；adaptive 模型从不发送 |
| `tool_choice` | `auto`／`any`／`none` 投影 `{"type": ...}`；`Named` 投影 `{"type":"tool","name":...}` |

neutral／simple 条目只映射工具选择；neutral thinking 保留其级别预算策略
（1024/2048/8192/16384），与 native 无预算默认 1024 保持区分。按 annex 行
165 撤回 sub-1024 类型化预检：钳到 0 的预算流向 wire，由 `|| 1024` 兜底。

inline-tools 时序（`inline-tools-2026-09-15`，compat
`supports_mid_convo_system_messages`＋`supports_mid_convo_tool_changes`，默认
均 false）：请求级工具表保持初始活动集（缓存断点在末项）加 deferred 占位符
`__pi_deferred_placeholder__`（`defer_loading: true`）；后续 system 消息保持
native，按值携带 `tool_addition` 定义——同名重定义是新增，绝不删除——以及
真正下线名称的 `tool_removal` 引用。system 更新挂起到下一条 assistant 消息
（或结尾）之前，保证 `tool_result` 仍紧随其 `tool_use`。

错误回合图像迁移遵循钉住的 `splitErrorToolResultImages` 形状：错误
`tool_result` 保留其文本并以 `(image attached after the tool results)` 占位
文本块替换每个图像段；标签加延迟图像跟随该轮**全部**工具结果之后，同处一
条 user 消息。anthropic producer 现运行于显式冻结的 `TelemetryContext`
（P20-TELEMETRY）。

## Google Generative AI native 选项与 Gemini 3 wire 规则（任务 20.40）

公开 direct adapter `gemini::GoogleGenerativeAiChatApi` 经
`Models::register_chat_api` 注册（api id `google-generative-ai`），携带类型化
`GoogleGenerativeAiOptions`——钉住的 pi 1.3.0 native 清单：

| 选项 | wire 效果 |
|---|---|
| `tool_choice: auto/none/any` | 投影 `toolConfig.functionCallingConfig.mode`（Google wire 无 named 工具模式）；有工具时 `none`/`any` 优先于 strict 模式，`auto` 让位于 `VALIDATED`（当有工具走了 strict） |
| `thinking.enabled: false` | 禁用配置：budget 模式模型发 `thinkingBudget: 0`；level 模式模型在 thinking map 提供关闭映射时用该 level，否则 `0` |
| `thinking.enabled: true` + `level` | `includeThoughts: true` + `thinkingLevel`（MINIMAL/LOW/MEDIUM/HIGH）；level 优先于同时给出的 budget |
| `thinking.enabled: true` + `budget_tokens` | `includeThoughts: true` + `thinkingBudget`，源值原样透传——`-1` 是动态默认、`0` 为禁用，绝不在本地重新解释 |
| （省略 `thinking`） | native 省略完全不发送 `thinkingConfig` |

neutral／simple 条目只映射工具选择；neutral 请求意图保持 pi `streamSimple`
策略：钳位级别，level 模式模型（`gemini-3(.x)-pro/flash`、`gemini-4+`、两个
`*-latest` 别名、Gemma 4）投影 `thinkingLevel`，budget 模式模型用 2.5 家族
表（pro 128/2048/8192/32768，flash 128/2048/8192/24576，flash-lite
512/2048/8192/24576；显式预算优先；其余 budget 模式模型回退动态 `-1`），
不可解析的级别映射在派发前类型化失败。neutral 显式关闭在 reasoning 模型上
发送禁用配置（内置 catalog 已将 2.5 系列标记为 reasoning）。

工具声明经 `parametersJsonSchema` 携带原 JSON Schema（绝不走遗留 OpenAPI
`parameters` 投影）；Gemini 3+ 模型经共享受限采样引擎投影 prefer/require
strict 并选择 `VALIDATED` 函数调用，Gemini 2 保持 Require 类型化失败与
Prefer 静默降级。Gemini 3+（以及 Google API 后面的 claude/gpt-oss）工具调用
与响应在 wire 上携带规范化 id（`[a-zA-Z0-9_-]`，64 字符）；Gemini 2 保持仅
名称的 function 调用／响应。

thought 签名（base64，仅 same-provider/same-model）回放在 text、thinking 与
functionCall part 上；仅带签名的空 part 得以保留。成功工具结果用 `output`
响应键，失败用带文本值的 `error` 键，连续 function 响应共用一条 user 消息，
工具结果图像在 Gemini 3+ 嵌入 `functionResponse.parts`，更早模型获得独立的
`Tool result image:` user 轮。流式侧捕获首个非空 `responseId`，thought part
（`thought: true`）作为 thinking 块流式输出并按块保留签名，未签名或重复的
工具调用 id 本地再生为 `name_N`，usage 计 `input = prompt - cached`、
`output = candidates + thoughts` 且 `reasoning = thoughts`（缺失的 thoughts
记为零），未映射的 finish 原因携带原始原因类型化失败
（`Provider stopped with: <raw>`）。Google producer 运行于显式冻结的
`TelemetryContext`（P20-TELEMETRY）。

## pi-messages 直接适配器（Radius 网关，任务 20.44）

公共直接适配器 `pi_messages::PiMessagesChatApi` 经 `Models::
register_chat_api` 注册（api id `pi-messages`，provider id `radius`），
携带类型化的 `PiMessagesOptions`。wire 是 pi 自身的消息协议：向
`<base>/messages`（去尾斜杠；`debug` 时附 `?debug=1`）单次 POST
`{ model, context, options }`，响应为序列化 assistant 消息事件的 SSE
流加一个 `done`/`error` 终结事件。它不是 Chat 序列化器：`context` 字
段是规范 chronology 投影成的 pi camelCase wire JSON（system 段落与工
具声明由 `normalize_context` 折入、assistant 文本/思考签名、redacted
thinking、参数为解析对象的工具调用、`toolResult` 消息），且会话中
system 消息原生保留——后端原生理解 pi 的 chronology。

`options` 对象是精确的六字段白名单——`temperature`、`maxTokens`、
`reasoning`、`cacheRetention`、`sessionId`、`toolChoice`——省略保持省
略：不补 `auto`、不注默认 maxTokens、不做 context 钳位。`reasoning`
逐字携带请求的 `ThinkingLevel`（`None` 映射为 wire 串 `"off"`）；
`toolChoice` 支持 `auto`/`none`/`required`/具名函数；`cacheRetention`
与请求值一一映射（`Disabled` 映 `none`），仅遗留的
`PI_CACHE_RETENTION=long` 环境项在未设时生效。simple 入口（pi
`streamSimple` 透传）不钳位地转发请求的思考级别——模型记录的级别
映射从不改写它——且只映射中性的工具选择。

流式事件 typed 且 fail-closed 校验：内容索引允许稀疏（未 start 的洞
保持为空）、无 start 的 delta、未知事件类型、坏帧或带非成功原因的
`done` 都以 `Invalid pi-messages streaming event: …` 失败；`data:
[DONE]` 被忽略；CRLF 输入与无空行结尾的尾帧均可解析。终结 `done` 事
件携带完整 usage 族（cache-write-1h、reasoning、total）、`responseId`、
`providerThinkingLevel` 与服务端重写的 `pi_messages_rewrite` 诊断；终
结 `error` 事件带 `errorMessage` 类型化失败（`aborted` 映射
`Cancelled`）；无终结事件的流失败。HTTP 错误格式
`"{status} {statusText}: {error.message ?? body} ({code})"`（优先 JSON
error 体，原始体兜底）；429 保持可重试身份。回调按钉住顺序触发——
payload、response（先于成功检查）、随后每个原始解析事件先于规范化。
opi 策划的基线目录（一条）覆盖目录到来之前的窗口；pi 生成的
`data/radius.json` 基线未随钉住源提供，配置网关的动态目录在注册时整
体替换基线。

## 内置 Provider 工厂层：42 个 canonical profile（任务 20.34）

`providers::builtin_provider_profiles` 携带 pi 1.3.0 能力清单中逐字提
取的 42 条 profile 表：canonical id（`google`、`google-vertex`、
`amazon-bedrock` 等）、显示名、默认 base URL、env API-key 名、auth
scheme、OAuth 可用性与端点语义（Azure 部署、Vertex location 模板、
Bedrock 区域、动态 radius）。旧的 opi id（`gemini`、`vertex`、
`bedrock`、`openai-responses`）不是别名——表从不注册它们。
`provider_collection::register_builtin` 把该表与内置目录转成基于已完
成 wire 适配器的注册 provider：每 profile 一个 mapped provider（Azure
与 google-vertex 受配置门控并给出类型化跳过原因，bedrock 以 AWS 链
描述符 lookup-only 注册），canonical-route 包装器把固定 id 适配器重发
布为 canonical id（含前缀敏感适配器所需的 model-spec 前缀重写），目
录条目连同分层计价转换进 `ModelInfo::pricing`，classifier/image 协议
经类型化缝以目录记录注册。环境 key 在派发时经注入的 `AuthContext`
解析——构造不读环境、存储或网络。

## OpenAI Decisions 适配器与四协议分类器闭环（任务 20.49）

第四个真实分类器协议：`api::decisions::OpenAiDecisionsApi`（api id
`openai-decisions`，provider `openai`）服务 OpenAI 的 Decisions API——一次
`POST <base>/decisions`，携带 `{model, input, questions}`。state 以 JSON
**字符串**上送；有图时 input 变为一条 user 消息：state 作 `input_text`，
图作 `input_image` data URL 依次随后（至多 128 张，请求前类型化拒绝）。
问题映射到 Decisions 类型：`choice` 保留选项（空描述不上线，与其他 opi
criteria 映射一样按字母序）、`score` 保留层级、`bool` 变为 `predicate`
并把真/假语义并入 instructions——System One 的 `noul` 与 Decisions 的
`predicate` 不可互换。答案以数组返回并按 name 匹配；拒答、缺答、形状
错型与非 finite 必填数均为类型化错误，且保留已计费的 usage 与按目录分
层的成本（仅做 source 必需的校验——不加任何通用概率范围/求和/argmax/
标签成员规则）。该路由只接受 OpenAI API key：空 key 在任何请求前类型
化失败。api.openai.com 前方的 Cloudflare 在请求超过约五秒时以 HTML 页
应答 504；重发同一输入只会再次超时，因此 **504 永不重试**，失败消息
解释网关限制而不透出页面——其余状态保持共享分类器重试策略（默认 2 次
额外重试、Retry-After 感知封顶延迟、逐 attempt body-inclusive 超时）。
curated 的 `gpt-6-luna` 记录记载被测模型及其 272K token 以上的长上下文
计价分层。`temperature` 在 pi 选项类型上存在但 Decisions 请求从不读
取，故类型化选项只携带重试旋钮。至此分类器缝完成四协议证明：
typesafe、Cloudflare、llama.cpp 与 Decisions 在同一个 `Models` 中并列注
册，经同一公共 `classify` 派发应答，同时保持各自的 wire 形状。

## llama.cpp 标签 token 读出分类（任务 20.48）

第三个真实分类器协议完全不需要专用分类服务：
`api::llama::LlamaCppClassifyApi`（api id `llama-cpp-classify`，provider id
`llama.cpp`）以 llama.cpp `llama-server` 服务的普通聊天模型做分类。模型
从不生成答案——每个问题渲染为一条聊天提示词，把候选答案列在单 token
标签下（choice 至多 62 项用字母与数字，score 至多 10 级用数字，bool 用
`Yes`/`No`），答案从这些标签 token 的**预采样下一 token 对数概率**读出：
对标签 logprob 做 softmax（除以类型化 `temperature`，默认 1；该值永不上
线——completion 恒携带 `temperature: 0`）。一个问题走三个端点：
`/tokenize` 解析各标签 token id（先换行前缀法，回退标签单独分词；词汇表
拆分的标签与共享 token 均为类型化错误）；`/apply-template` 以模型自身
聊天模板渲染并禁用思考（开放的 `<think>` 块立即闭合）；`/completion`
读取 top `n_probs`，缺标签时把读出深度从 `max(256, 每标签 16)` 逐级升至
4096、32768——始终未上榜的标签是错误，绝不发明零概率。答案保持共享语
义：bool 概率、score 期望值配 TypeSafe 置信度 `(n·peak−1)/(n−1)` 截断到
`[0,1]`、choice argmax 配逐选项原样分布。服务器根 URL 会剥掉一个
OpenAI 兼容的 `/v1` 后缀；每个请求携带模型 id；可选 Bearer 密钥仅在解析
出秘密时附上。llama-server 不报告 token 用量，结果亦不携带。选项数、正
有限温度、图像输入与回调族都在**首个请求之前**类型化拒绝；重试/超时策
略与共享分类器传输一致（默认每端点 2 次额外重试、逐 attempt
body-inclusive 超时、Retry-After 感知延迟）——基于共享策略助手实现自有
循环，因为带逐端点观察面的三端点读出与单请求的 System One 内核是不同
的传输形状。已解析的标签 token 按（服务器根、模型、标签）在适配器生命
期内缓存。

## Cloudflare Workers AI System One 适配器与多模型注册（任务 20.47）

第二个真实分类器协议驱动同一条共用 conformance：
`api::cloudflare::CloudflareWorkersAiSystemOneApi`（api id
`cloudflare-workers-ai-system-one`，provider id `cloudflare-workers-ai`）
以自有协议形状经 Cloudflare REST 端点服务 System One 模型，**传输内核自
`api::system_one` 参数化共享**（pi `SystemOneTransport` 形状：URL、请求
信封与响应解包不同；守卫、payload 冻结、重试策略、逐 attempt
body-inclusive 超时、计费保留与 tiered 目录计价完全一致）。单次
`POST <base>/run` 携带 `{model, input: {state, questions}}` 信封——完整
模型 id 在 `model`，公开 `bool` 问题仍以 `noul` 上线。账户级端点携带
`{CLOUDFLARE_ACCOUNT_ID}` 占位符，由解析出的鉴权账户身份物化；缺省时占
位符原样保留（pi `resolveCloudflareModel` 回退），请求不会到达真实端点。
响应为 Cloudflare API 信封两种形状：第三方 run 记录以
`{success, result: {state, result: {answers, usage}}}` 包裹输出且
`state` 必须为 `"Completed"`（否则
`"Cloudflare Workers AI run did not complete (state: {state})"`）；
Cloudflare 自托管模型直接返回 `{result: {answers, usage}}`；精确的
`success: false` 将信封 `errors[].message` 字符串聚合为
`"Cloudflare Workers AI error: m1; m2"`，无可用消息回退
`"Cloudflare Workers AI request failed"`。`temperature` 仍是永不上线的
类型化选项；图像输入在任何请求之前拒绝；HTTP 失败格式化为
`"Cloudflare Workers AI error ({status}): {body}"`。

一个 provider 服务多个分类器模型，因此注册现在持有一个记录**集合**：
`Models::register_classifier_api` 以适配器完整记录播种，新增的
`Models::register_classifier_model_record` 追加（或按 provider/model id
替换），`Models::classifier_model` 扫描集合；API 无注册的记录是类型化
`ModelsError::ClassifierApiNotRegistered` 拒绝。curated 基线交付三条钉住
身份——`typesafe/jev`（64000 token 上下文窗口自同底座模型的 TypeSafe
fixture 跨宿主沿用）、`@cf/cloudflare/clef`（$0.24/M 输入）与
`@cf/cloudflare/clef-flash`（$0.09/M 输入）；输出价与自托管模型的上下文
窗口无钉住证据（pi 生成的 `data/cloudflare-workers-ai.json` 未随钉住源
提供），保持生成器的 `0` 回退。注册方整体替换集合。

## 分类器操作与 TypeSafe System One 直接适配器（任务 20.46）

fixed-`O` 分类器操作与 chat／image seam 并列：object-safe 的
`api::ClassifierApi<O>` trait 镜像 `ImageApi`（精确 `ApiId`、provider
身份、完整记录、类型化选项、prepared 派发）——**一条共用 conformance 经
此 seam 覆盖全部真实分类器协议**；公开的 `classifier` 契约承载类型化
`ClassifierContext`（JSON state 对象、有序 questions、可选 images）、
`ClassifierAnswer` 变体、`ClassifierResult`（provider 失败永不以 `Err` 返
回——一律走 `stop_reason = Error`/`Aborted`，携带类型化 `ProviderError`
与展示消息；已计费请求在后续答案解析失败时保留其 usage）以及
operation-qualified 的 `ClassifierModelRecord`，其目录定价为 **含 tiers 的
`ModelPricing`**。`Models::register_classifier_api` 以解析器与来源注册适
配器；`Models::classifier_model` 执行按操作限定的选择（chat 模型与未知
id 在**任何鉴权解析之前**类型化失败）；`Models::classify` /
`classify_native` 是普通与类型化 native 两个入口；
`ClassifierApiHandle::options` 生成的值在兼容的重新注册间保持有效。

唯一的真实适配器 `api::system_one::TypesafeSystemOneApi`（api id
`typesafe-system-one`，provider id `typesafe`）复刻钉住的 pi 模块：向
`<base>/systemone`（尾斜杠剥除）单次 POST `{model, state, questions}`，
仅 Bearer 鉴权；公开的 **`bool` 问题与答案双向映射为 wire 层的
`noul`**（choice/score 保持原形）；图像输入在任何请求之前拒绝
（`"System One API does not support image input"`，与记录模态无关）；
带 api 不匹配守卫。答案解析仅校验源所要求的部分——record 形状、wire
类型一致、字符串 `choice`、有限数——**没有**通用的概率范围/求和/
argmax/标签成员规则；每个已知问题都必须有类型匹配的答案，未知答案
id 被忽略。usage `{input_tokens, output_tokens}` 保留源容忍度（非数或
非正 token 计为 0；两字段皆缺则结果不带 usage），并按记录目录 **chat 式
tiered 计价**（区别于图像路径的基础价例外）。

传输遵循源：**无默认超时**（调用方超时 body-inclusive 且**每次重试
attempt 新鲜**，超时报 `"Request timed out after {ms}ms"` 并像任何无状
态失败一样可重试），**默认 2 额外重试**并采用 SDK 重试策略
（`x-should-retry` 覆盖；408/409/429/5xx/无状态可重试；
`retry-after-ms`/`retry-after` 生效且有 60 秒服务端延迟上限——超上限请
求立即失败并保留源消息格式），重试复用冻结 payload，取消映射为
`Aborted`，HTTP 失败格式化为 `"System One API error ({status}): {body}"`。
`temperature` 是 System One 无法应用的类型化选项：永不上线。payload 观
察器（替换或保留、每次逻辑调用一次、跨重试冻结）与 response 观察器
（status + headers，先于 body 处理）受支持；raw-event 与 header-transform
族在派发前类型化拒绝。冻结的 `TelemetryContext` 透传到请求路径。
curated 基线记录（`typesafe/jev-latest`，64000 token 上下文，文本输入，
注册前零价）在注册时整体替换；pi 生成的 `data/typesafe.json` 未随钉住
源提供。

## 图像操作与 OpenRouter 图像直接适配器（任务 20.45）

fixed-`O` 图像操作落在 `api`/`images`：object-safe 的
`api::ImageApi<O>` trait 镜像 `ChatApi`（精确 `ApiId`、provider 身份、
完整记录、类型化选项、prepared 派发），公开的 `images` 类型承载有序的
`ImagesContext`/`ImagesOutput`、`ImagesResult`（provider 失败永不以
`Err` 返回——一律走 `stop_reason = Error`/`Aborted`，携带类型化
`ProviderError` 与展示消息）以及 operation-qualified 的
`ImageModelRecord`。`Models::register_image_api` 以解析器与来源注册适配
器；`Models::image_model` 执行按操作限定的选择（chat 模型或未知 id 在
**任何鉴权解析之前**类型化失败）；`Models::generate_images` /
`generate_images_native` 是普通与类型化 native 两个入口；
`ImageApiHandle::options` 生成的值在兼容的重新注册间保持有效。

唯一的真实适配器 `api::openrouter_images::OpenRouterImagesApi`
（api id `openrouter-images`，provider id `openrouter`，`O =
OpenRouterImagesOptions`，承载重试策略）复刻钉住的 pi 模块：向
`<base>/chat/completions`（尾斜杠剥除）单次非流式 Chat-wire POST
`{model, messages, stream: false, modalities}`（记录声明文本输出时为
`["image", "text"]`，否则 `["image"]`），仅 Bearer 鉴权；文本输入序列化
为 text 部分，图像输入为 `data:<mime>;base64,<data>` URL——无四 MIME/
数量/字节/签名门槛，无 URL 抓取、写盘、像素解码或流式变体。解析保持
**仅首 choice、文本先于图像**：非空字符串 content 成为首个文本输出，
随后 `message.images` 中匹配 `data:<mime>;base64,<data>` 的条目成为图
像输出并保留 encoded 原值；非 data URL、畸形条目与后续 choice 跳过，
纯文本/空图像/零 choice 响应仍是成功。Usage 保留 provider 拆分
（有写入时 `cache_read = reported_cached - cache_write`，`input` 扣除两
个缓存桶），计价仅用记录的**基础价四率**（图像路径的 base-cost 例外
——tiers 永不生效；curated 基线条目的未知价格保持生成器的 `0` 回退，
pi 生成的 `data/openrouter.json` 未随钉住源提供，注册方整体替换记
录）。

传输遵循源：**默认 600000 ms 的逐 attempt 超时覆盖整个 body 读取与解
析**（区别于 Mistral 仅 headers 的 60 秒截止），显式超时必须为正
（`Some(zero)` 是派发前类型化拒绝），**默认 0 额外重试**并采用 SDK 重
试策略（`x-should-retry` 覆盖；408/409/429/5xx/无状态可重试；
`retry-after-ms`/`retry-after` 生效且有 60 秒服务端延迟上限——超上限请
求立即失败并保留源消息格式），重试复用冻结 payload，取消映射为
`Aborted`，HTTP 失败格式化为 `"<status>: <error.message | body>"`，
401/403 → `AuthFailed`、429 → `RateLimited`、408/504 → `Timeout`。回
调：payload 观察器（替换或保留、每次逻辑调用一次、跨重试冻结）与
response 观察器（status + headers，先于 body 处理）受支持；raw-event
与 header-transform 族在派发前类型化拒绝（ONB-04）。冻结的
`TelemetryContext`（P20-TELEMETRY）透传到请求路径，无自动 span。

## Mistral Conversations 直接适配器与 native 选项（任务 20.43）

公共直接适配器 `mistral::MistralConversationsChatApi` 经
`Models::register_chat_api` 注册（api id `mistral-conversations`，
provider id `mistral`），携带类型化的 `MistralOptions`。API 名称保留，实际
POST `{base}/v1/chat/completions`；既有 OpenAI 兼容 profile
（`mistral::mistral_provider`）不动，仍是合法 profile。

| 选项 | wire 效果 |
|---|---|
| `tool_choice: auto/none/any/required/named` | `tool_choice`（named 为 `{"type":"function","function":{"name":...}}`）；省略时成员不出现 |
| `reasoning_effort: none/low/medium/high/max` | 逐字转发 `reasoning_effort` |
| `prompt_mode: reasoning` | `prompt_mode: "reasoning"` |

中性入口仅映射 tool choice，reasoning 分支由模型记录决定：thinking level
map 带显式映射的模型发送 `reasoning_effort`（钳位后取最近支持级别；关闭
thinking 时投影 `none` 级映射；缺省回退 `high`），无映射的 reasoning 模型
发送 `prompt_mode: "reasoning"`，非 reasoning 模型两者皆省略。采样只投影
`temperature` 与 `max_tokens`——stop、`top_p` 与 penalty 家族不进此 wire
（pi 载荷构造器不设其他成员）。工具默认投影 `strict: true` 并携带转换后
schema。

Prompt 缓存随会话：有 session id 且 retention 非 `Disabled` 时，body 携带
`prompt_cache_key`、headers 携带 `x-affinity`（显式 `x-affinity` 头优先，
仅抑制自动头）。响应时限（默认 60s，可用请求 timeout 覆盖）**只覆盖等
response headers 的时间**——body 停顿交给 HTTP 客户端空闲处理，长流不会被
固定 deadline 切断。

回放把工具调用 id 规范为 9 个字母数字（strip、恰好 9 位直用、否则
SHA-256 派生、冲突重试），assistant 调用与配对 tool result 同时应用。消
息转换将会话中 system 消息折叠为合并后的首提示、丢弃空白文本、assistant
thinking 以 `thinking` 块回放并带 `prefix: false`、纯单文本 user 轮保持纯
字符串、图像模型发送 `image_url` data URI（否则 `(image omitted: model
does not support images)`）、tool result 以 `[tool error]` 前缀与
`(see attached image)`/`(no tool output)` 占位族渲染。流式侧保留首个非空
chunk `id` 作 response id、拆分缓存 prompt tokens（`input = prompt -
cached`、`cache_read = cached`、`cache_write = 0`、total 回退求和）、跳过
空 content delta（GLM 式拆块问题）、按 index 累积工具调用（wire 发
`"null"` 占位时派生 id）、映射 `stop`/`length`/`model_length`/
`tool_calls`，未映射 stop 与流内 `error` stop 类型化失败并携带原始原因
（可重试服务端形态为 `Provider stopped with: error (server error)`），
无 finish reason 的流以类型化失败收尾。HTTP 失败格式
`Mistral API error (status): body`（4000 字符截断先于 crate 级 sanitized
摘要上限）；429 保持可重试限流身份。

## Bedrock Converse Stream 直接适配器与 native 选项（任务 20.42）

公共直接适配器 `bedrock::BedrockConverseStreamChatApi` 经
`Models::register_chat_api` 注册（api id `bedrock-converse-stream`，
provider id `bedrock`），携带类型化的 `BedrockOptions`。AWS 传输层——SigV4
凭证链、bearer token、region/profile 优先级、Smithy retry、eventstream+CRC
解析器与显式 proxy-placeholder 签名模式——继承自既有交付，不变；适配器在其
上补齐 pi 内容引擎。

| 选项 | wire 效果 |
|---|---|
| `tool_choice: auto/any/none/named` | `toolConfig.toolChoice`（`{"auto":{}}`、`{"any":{}}`、`{"tool":{"name":...}}`）；`none` 整个省略 `toolConfig`，工具不随其他成员发送 |
| `reasoning: <级别>` | Claude 预算模型：`additionalModelRequestFields.thinking = {type: enabled, budget_tokens}`，取自定义 `thinking_budgets` 覆盖或默认表（1024/2048/8192/16384；`xhigh`/`max` 钳到 `high`）；adaptive 模型（Opus 4.6+/4.7/4.8/5、Sonnet 4.6/5、Haiku 5、Fable 5）：`thinking = {type: adaptive, display, block_binding}` + `output_config.effort`（能力模型原生 `xhigh`，thinking map 的显式映射优先） |
| `interleaved_thinking: bool`（默认 true） | 预算制 Claude 模型附 `interleaved-thinking-2025-05-14` beta |
| `thinking_display: summarized/omitted` | `thinking.display`（默认 `summarized`）；**GovCloud** 目标（`us-gov-*` 派发 region 或 `us-gov.`/`arn:aws-us-gov:` 模型 id）省略 `display` 与 binding-controls beta，直至 GovCloud Converse schema 追平 |
| `request_metadata` | 逐字转发为 `requestMetadata`；AWS 限制仅存在于 pi 注释，校验归 SDK（无本地 preflight） |
| （非 Claude） | gpt-oss 模型取平面 `reasoning_effort`（low/medium/high）；GPT 模型取嵌套 `reasoning.effort`；其他家族不发送额外字段 |

`region`/`profile`/`bearerToken` 是 resolver 持有的输入（20.20 AWS 链：显
式/scoped/env 密钥、profile、web identity、ECS 相对 URI、IMDS、SSO），不在
本结构体上重复。

strict 工具跟随 `BedrockConverseStreamCompat.supports_strict_tools`
（`WireCompat::BedrockConverseStream` 变为元组变体）：strict toolSpec 携带
转换后的 schema 与 `"strict": true`；flag 为 false 时 `require` 依旧在派发
前类型化失败。Cache points 附着于 system 提示与最后一条 user 消息（含工具
回合）——默认保留策略保持 pi 的 `short` 默认，`Long` 加一小时 `ttl`，
`Disabled` 全部丢弃。消息转换在 Bedrock 要求非空内容处发送 `<empty>`、丢
弃空白文本、thinking 以 `reasoningContent.reasoningText` 回放（带签名的
Claude 块；无签名 Claude 回退纯文本；非 Claude 永不携带 signature 成员）、
redacted reasoning 以不透明 `redactedContent` 字节回放（非 base64 载荷丢弃
该块）、连续 tool result 合并为一条 user 消息、Claude 错误工具图像移至本
回合全部结果之后并带说明文字。流式侧把 `reasoningContent` delta 映射为
thinking 块（signature 累积、redacted 载荷缓冲），把
`model_context_window_exceeded` 映射为 `length`，未映射 stop 原因类型化失
败并携带原始原因（`Provider stopped with: <raw>`），一小时的
`cacheDetails` 求和进 `cache_write_1h_tokens`。producer 运行于显式冻结的
`TelemetryContext`。

## Google Vertex 直接适配器与 native 选项（任务 20.41）

公共直接适配器 `vertex::GoogleVertexChatApi` 经 `Models::register_chat_api`
注册（api id `google-vertex`，provider id `vertex`），携带类型化的
`GoogleVertexOptions`。tool-choice 与 thinking 意图复用任务 20.40 记录的
Google 形态（无 named 工具模式；`budget_tokens` 原样保留 `-1` 动态与 `0`
禁用；native 省略完全不发 `thinkingConfig`），另有两项 Vertex 专属扩展：

| 选项 | wire 效果 |
|---|---|
| `project` / `location` | 重路由 ADC URL（`.../v1/projects/{p}/locations/{l}/publishers/google/models/{model}:streamGenerateContent`）；Express API-key 路由不带二者，此选项在其下为惰性 |
| （Bearer 鉴权缺 project 或 location） | 任何网络工作之前类型化 setup 失败——绝不静默产出退化 URL |

路由由鉴权持有且独立于 Gemini key 身份：API key 选择 Express 模式
（无项目前缀的 `aiplatform.googleapis.com` collection 端点 +
`x-goog-api-key`）；Bearer token 走 ADC project/location URL，凭证来源
持有 quota project 时附加 `x-goog-user-project`。自定义 base URL 绝不丢
project/location 资源前缀（前缀总是重建），`location = "global"` 钉住共享
域名。内容引擎、流式解析、usage 计量（`output = candidates + thoughts`、
`reasoning = thoughts`）、strict 工具处理与 thought 签名规则继承自共享
Google 引擎（任务 20.40）。

每 wire 一处 neutral 策略差异：vertex 预算表没有专用 flash-lite 条目，pi
的子串匹配把 `2.5-flash-lite` 路由进 flash 表（minimal 128；
generative-ai wire 的专用表映射 minimal 512）。vertex producer 运行于显式
冻结的 `TelemetryContext`。

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
  `anthropic_strict_tools_fallback` 诊断。普通 `Models` 与直连 Anthropic 调用
  采用与高级 attempt 相同的 acquisition owner；转发 wrapper 继承该 owner，不能
  创建第二份重试预算。成功响应头在 response 回调或 body 解析前即封闭 acquisition，
  即使尚无公开事件。其他 400、回调错误、取消与 handoff 后失败一律不分支；
  `require` 绝不降级。
- Responses/Codex 将 grammar 工具投影为 `custom` 工具，lark 优先 regex；
  不支持目标退普通 JSON function 工具并在消息上记录 loss，支持目标无可用
  variant/schema 则派发前失败。流式将 `custom_tool_call` 原始输入增量封装
  为 `{"<property>":"…"}`（单调增长、单次 close、违规 typed 失败），累积的原始
  输入构成权威最终参数，包含仅终态携带的 input；矛盾的终态 input 以 typed 失败结束。
  replay 以 `ctc_` id 还原
  `custom_tool_call(_output)` 项（function call 保持 `fc_` 前缀）。

grammar 输入属性名按 schema 的 literal key 查找，包括 `/` 与 `~`；包装保留精确
名称，trusted schema 不变。

## 功能请求回调（AIR-059/V-51 普通切片，任务 20.51）

`callbacks::RequestCallbacks`（随 `Request.callbacks` 携带，默认为空）附加四类
观察者，在真实直连适配器内按其源定义的时机触发——绝不从归一化 `EventStream`
反推，也绝不叠加第二个驱动：

回调定义可共享及克隆，header/body 冻结状态属于单次 logical call。独立调用各自持有
状态；acquisition 重试与 strict variant 保持同一调用 owner。每个同步 builder 都有
异步对应：`with_async_header_transform`、`with_async_on_payload`、
`with_async_on_response` 与 `with_async_on_raw`。适配器在发送或归一化对应数据前按序
await；取消可结束 pending 回调而不再发送。

| 回调 | 时机 | 合同 |
|---|---|---|
| `with_header_transform` | 每个 logical call 一次，作用于组装后的每调用集合 | 输出整体替换该集合；非保护项的省略/设置/删除受支持。已存在的受保护 identity/auth/signing 项可原样返回；删除、修改或引入受保护项在派发前 typed 失败。适配器拥有的凭据不进入变换。 |
| `with_on_payload` | 采样/compat/wire 投影之后，作用于最终请求体 | `Ok(None)` 保持原体；`Ok(Some(v))` 校验后替换：JSON object、model 身份不变、输出预算不得上调。Chat/Responses/Codex 还在派发前检查 streaming wire shape，保留未知扩展及合法 content/tool 变更。每个 payload variant 只运行一次——普通获取重试按字节复用冻结体（`ORDINARY_PAYLOAD_VARIANT`）。 |
| `with_on_response` | 在适配器来源定义的响应观测点触发，携带 status 与 headers，先于 body 处理 | Codex SSE 也在分类前观测非成功状态；未产生响应的失败不通知。成功响应 handoff 封闭普通 acquisition retry。 |
| `with_on_raw` | 每个已解析 provider wire 事件，严格先于其归一化发射，按 wire 顺序 | 只读；失败以 typed 错误终止当前调用并保留已交付的 partial。 |

适配器覆盖：Anthropic、OpenAI Chat（含 Mistral 与 OpenRouter profile）、
OpenAI Responses（含 Azure）、Codex 与 Bedrock 接线全部四类（Bedrock raw 事件
为已解析 smithy 帧 `{type, ...payload}`；Smithy 签名头受变换保护）。Google
wire（Gemini/Vertex）支持 payload、header-transform 及归一化前的已解析 raw
观测；其 HTTP response hook 不可用，附加 response 观察者会在派发前以 typed
`UnsupportedCapability` 拒绝。回调失败是
调用的功能失败：`ProviderError::CallbackPhase`，已脱敏、永不重试、不被遥测
吞掉、不触发新调用。回调是运行时函数值，不是 JSON 配置；本切片包含普通调用及
strict variant。新操作 API（20.43–50）与完整 AIR-059/V-51 收口（20.57）保留各自
独立验收边界。

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
| 计价 tier | 以 input+cacheRead+cacheWrite 选择，严格大于，取最高，相等重复取首个 | 无 tier → 基础费率 | 零阈值有效；保持原序及重复项 | 命中费率适用于整个请求 |
| 1h 缓存写入 | 子集按 2× 输入费率 | `None` → 全部短缓存 | 显式 0 有效，与缺失区分 | 子集，永不重复计费 |
| service tier | flex 0.5×、priority/fast 2×（`gpt-5.5`：2.5×）、default 1× | 缺失 → 1× | 未识别的 wire 值保持缺失 | 作用于每个成本分量 |

仅 OpenAI 兼容适配器（Chat、Responses）合并解析后的 sampling 映射，且作为 body 的
最后一步，使模型与 request 的 sampling 覆盖具名请求字段；其他适配器忽略它。
Responses 适配器把响应中的 `service_tier` 解析进 `Usage`。OpenRouter 图片
`parse_usage` 是显式源例外：图片用量保留基础模型费率，不做 tier 选择。opi 没有
Pi-Messages wire，因此该 wire 的 native/simple 直传条款不适用。native `ChatApi`
调用遵循前述适配器预算语义。元数据本身永远不会产生新的派发前限制或缩放；重复的静态 `reasoning_effort` compat 键
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

## Deferred 响应（任务 20.50）

Deferred 提交就是普通生成（AIR-043）：具备能力的后端返回一条
`StopReason::Deferred`、内容为空并携带 `deferred` handle 的 assistant
消息，而不是最终内容——不存在 `submitDeferred` API。`DeferredHandle`
携带 source 拥有的身份（`provider`/`model_id`/`api`/`id`）、可选的过期
与轮询提示、可选的 provider 转换数据；它绝不携带凭据，并按 pi 形态的
camelCase（`expiresAt`、`pollAfterMs`、`data`）序列化。

获取与取消是针对该 handle 的显式操作：

- `Models::stream_deferred(handle, call) -> EventStream` 先校验 handle
  （身份，然后按调用方注入的 `now_ms` 校验过期），再经后端注册的
  resolver 解析鉴权，最后恰好派发一次 fetch。派发前失败以单个类型化
  `Err` 流项呈现，由 `ChatStream` 转换为规范 `CallFailure`（含类型化
  `CredentialNeeded` 原因）。
- `Models::fetch_deferred(handle, call)` 驱动该流至单一最终结果：仍
  pending 的结果是 `Ok`，返回消息携带 `StopReason::Deferred` 与 handle；
  ready 的结果交付最终 assistant 消息。`wait_ms = 0`（pi wait0）执行一次
  状态检查；无论 `poll_after_ms` 为何，SDK 绝不隐式轮询。
- `Models::cancel_deferred(handle, call)` 是尽力而为的远端取消：失败会
  报告，从不保证成功。取消 fetch 调用的本地令牌只中止本地尝试
  （`ProviderError::Cancelled`；首次 poll 前取消则零后端工作），绝不触及
  远端任务——两条取消路径相互独立（AIR-044）。

后端经 `Models::register_deferred_api` 注册（类型化 `DeferredApi` trait：
`api_id`/`provider_id`/`model_id`/`default_options` 加上
`fetch_prepared`/`cancel_prepared`），镜像 image/classifier 的按操作限定
注册表；同一精确 `ApiId` 的 upsert 整体替换注册。
`Models::validate_deferred_handle` 暴露类型化的鉴权前校验（装箱 detail 的
`RouteMismatch`、`Expired`、`Unsupported`）。fetch/cancel 调用可将可选的
冻结 `TelemetryContext` 直达后端（AIR-054）；提交复用 chat 请求既有的
载体。本任务不声称任何真实远端 deferred 协议：发布的接缝服务于脚本化与
自定义后端，`test_support::ScriptedDeferredBackend` 与
`MockProvider::with_deferred` 提供确定性的 pi-faux 脚本（pending 倒计时、
取消记录、可选强制取消失败），`tests/deferred_contract.rs` 中的可重开
consumer fixture 在伪造单调时钟下不重提交地演练这些行为。

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

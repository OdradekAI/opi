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
共享配置 INI 叠加（assume-role 与 web identity）、`credential_process`、以及远端 ECS/IMDS 来源段，
经锁定的 STS Query/XML codec 交换）位于 `auth::aws`；
keychain、环境变量、
登录 presenter 和 refresh 实现仍位于 `opi-coding-agent`。

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
| `Models` / `AuthDescriptor` / `AuthStatus` | unstable-0.x 唯一 driver，位于 `ProviderRegistry` 之上：Provider+模型查找、脱敏鉴权状态、OpenAI-compatible 兼容性元数据、派发与原子动态目录 refresh。普通 native/simple/direct 入口与 advanced `prepare_call` 共用同一 compiler、鉴权解析、attempt seam 与 collector。 |
| `ChatApi` / `ApiId` / `ChatApiHandle` / `NativeOptions` | 类型化自定义 chat adapter 注册，携带精确 API 身份与固定 options 类型；由 handle 创建的 options 携带 API/类型标签，可在具备兼容注册的实例间移植。 |
| `ToolCall::arguments_error` | 文本 tool 参数的最终解析结果标记（AIR-048）：空输入归一为 `{}`，可修复的非法转义保留字面文本，不完整/非对象最终文本保留自有 raw 并携带该错误，绝不以 partial-prefix 成功执行；缺少该字段的历史记录仍可反序列化。 |
| `ChatModelRecord` / `SimpleOptions` / `ChatCall` | 调用方持有的完整 chat record（provider id、裸模型 id、精确 wire），无需目录成员即可调用；中性 simple options；构造时捕获输入、首次 poll 才启动鉴权的惰性普通调用 owner。 |
| `PreparedProviderCall` / `PreparedRoute` | `Models::prepare_call` 返回的不透明 logical-call 状态，以及公开的已脱敏 route facts。`route`、`auth_provenance` 与 `start_attempt` 是 prepared-call 生命周期 API。 |
| `CredentialStore` / `Credential` / `CredentialSource` | 无 IO、object-safe 的凭据持久化与已脱敏三态探测契约。 |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | 与 flow 无关的 boxed-future OAuth 契约；具体 flow 位于 `opi-coding-agent`。 |
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
seam。`opi-coding-agent` 提供 OS-keychain store，以及获批的 Anthropic、GitHub
Copilot 和 OpenAI Codex 登录 flow；`opi-ai` 不执行 keychain、环境变量或 presenter IO。

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
按调用凭据仍不在范围内：`extra_headers` 会拒绝 Provider 管理的鉴权 header。

### 惰性 AuthContext 与管理结果（AIR-063/AIR-066）

来源访问是独立的 context 职责：object-safe 的 `AuthContext`（`env`、
`file_exists`）只回答"环境说了什么"，不承载模型或 telemetry 数据。
`AmbientAuthContext` 是默认的惰性实现，`Models::set_auth_context` 注入后
整体替换 ambient——注入来源不会被绕回进程环境。构造、注册与静态目录
查找执行零 env/store/network 访问；`AuthContextError` 让非 Unicode 环境字节
与失败的存在性探测保持类型化，而不是空结果。

在此之上，`Models::{check_auth, get_available, get_auth, logout}` 是管理
结果。`check_auth` 报告配置状态而不刷新——已过期的存储 OAuth 凭据仍是
已配置——并将不可达的 store backend（含探测快照）以类型化的
`ModelsError::CredentialStore` 错误呈现，绝不静默折叠为 `Ok(None)`。
`get_available` 用同一检查过滤注册表。`get_auth` 按需解析被选中的来源；
显式 `AuthResolutionOverrides::api_key` 无需触碰任何来源即获胜，route
resolver 的 `CredentialNeeded` 报告未配置，其他被选来源失败则以
`ModelsError::AuthResolution` 传播且绝不尝试回退来源。`logout` 按调用接收
凭据 store，且只确认这一次本地删除：不 unset 环境变量、不撤销远端会话、
不动其他 provider 的条目。管理路径中的存储 OAuth 凭据解析不刷新；刷新
属于 live route/flow owner。

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
（`prepare_call` 保持默认零额外 send）。prepared call 是唯一的采集重试 owner：它对观测到的
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
`context` 模块估算消息、system 文本和工具 schema，并使用适用的同模型 usage 加新增
消息计算当前占用，而非会话累计用量。直接调用 provider adapter 仍保留原有默认行为；
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

## 非目标

以下是明确的非目标，不得作为当前核心行为出现：

- Anthropic、GitHub Copilot 与 OpenAI Codex 之外的 OAuth provider。
- 经审计的静态 pi-0.80.6 GitHub Copilot/OpenAI Codex snapshot 之外的
  Provider catalog，包括在线 entitlement filtering。
- 凭据撤销后的自动重新登录。
- 按调用 API key、env 或鉴权 header 覆盖。
- Provider payload/response 流式 hook。
- 大范围新增 first-class provider 列表（兼容 provider 保持为 config-driven
  profile）。
- 图像生成（图片支持仅为输入侧）。
- 已获批的 Anthropic 与 OpenAI Codex OAuth 登录流程之外的浏览器自动化。
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
`models`、`provider_headers`、`auth`、`credential`、`http`、`retry`、`model`、
`anthropic`、`openai_chat`、`openai_responses`、`openai_codex_responses`、
`openrouter`、`mistral`、`gemini`、`bedrock`、`azure_openai`、`vertex`、
`config`、`time` 和 `test_support`。

## 许可证

MIT。详见包内 [LICENSE](LICENSE) 与压缩 [NOTICE](NOTICE)。

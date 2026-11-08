# Provider、Catalog、认证与刷新参考

本专题集中整理首次 `opi-ai` 调用之后最常用的 Provider、Catalog、认证和刷新
合同。主 [README](../README.zh.md) 保留任务导向的接入路径和简明 Core API
表；本文档承载详细生命周期与能力矩阵。

专题遵循 crate 的所有权边界：`opi-ai` 拥有 Provider 路由、Catalog 校验与发布、
注入式凭据合同、OAuth 元数据和刷新编排；宿主拥有 keychain 存储、登录展示以及
exporter/runtime 策略。以下内容描述当前 Rust 原生行为，不表示自动登录或启动时
联网策略。


## 工厂、Catalog 与认证

- 免手写路由的注册：内置 42-profile 工厂
  （`providers::builtin_provider_profiles`，任务 20.34）注册完成的适配器
  与 catalog——见「Builtin Provider Factory Layer: 42 Canonical Profiles」。
- 模型记录：`Models::get_model` 解析 registry 元数据；离线内置 catalog 与
  动态 `ModelsStore` 刷新见「Offline Bundled Model Catalog」与
  「Dynamic Catalog Store and Refresh」。
- 认证：注入式 `AuthResolver`（如快速开始所示）、`CredentialStore` 持久化、
  九个 OAuth 家族与独立 AWS 凭据源链位于 `auth` / `auth::aws`；宿主
  keychain 与终端登录策略保留在 `opi-coding-agent`
  （「Credentials, Request Enrichment, and Refresh」）。

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

## 凭据、Request 扩充与 Refresh

`Models::new()` 与 `Models::from_registry(...)` 各自创建独立的
`InMemoryCredentialStore`。构造不需要 runtime，也不主动发现外部来源或访问
环境、文件、网络。登录凭据仅属于该实例；调用方可用 `set_credential_store`
显式接入共享 store 并替换默认存储。本地内存 ACK 不表示凭据可跨重启持久化。

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
cancellation token。`start_attempt_with_telemetry()` 会把一个 attempt 包装为
`ChatStream` 并结算显式 AI observer；需要自己管理 retry admission 和 stream
归一化时继续使用原始入口。attempt 必须串行：重叠 stream 返回
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
descriptor 配置检查优先使用默认存储中的实际凭据；只有隐式内存存储返回
`Ok(None)` 时才可查询经 `set_probe` 注入、仅含脱敏标签的快照。显式接入的
store 始终具有权威性，包括缺失和错误。probe 元数据不提供认证，也不授权
派发请求。
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
可区分缺失与显式上报的零。`CostBreakdown` 包含四个 catalog 分量
`input_cost`、`output_cost`、`cache_read_cost`、`cache_write_cost`，以及
`total` 与 `total_from_provider`。加权的一小时 write 子集折算进
`cache_write_cost`，reasoning 仍计入 `output_cost`，费用与总 token 只计一次父 bucket。
`catalog_total_cost()` 始终求四个 catalog 分量；`total_cost()` 只有在
`total_from_provider` 为真时才使用 provider 上报的 `total`。
`Usage::apply_catalog_cost` 会重新计算 catalog 估算并清除 provider 覆盖。

计价 tier 遵循 Pi 的边界规则：以 `input + cache_read + cache_write` 选择
`input_tokens_above` 严格小于该值的最高 tier，选中的费率作用于整个请求。
一小时 cache-write 子集按 input 费率的两倍计费，剩余写入使用短期费率，绝不重复计算。
OpenRouter 计费字段只在 OpenRouter 路由上解释：非 BYOK 使用其 charged `cost`，
BYOK 加上 provider 上报的 upstream 总额，缺少总额时回退到 prompt 加 completion。
provider 计费数据无效或不完整时保留 catalog 计价。
`AssistantMessage::raw_stop_reason: Option<String>` 保存 provider 已上报的原始
终止事实，与归一化的 `stop_reason` 独立。旧记录缺失该字段时读为 `None`，
序列化省略 `None`；现有 assistant-message 结构体字面量补上
`raw_stop_reason: None`。
`Provider::refresh_models` 与 `Models::refresh` 实现确定性的原子目录替换，
但仅为基底、无生产触发。

## Provider 行为矩阵

Anthropic thinking 内容保留不透明的 `signature`，
`AssistantContent::RedactedThinking` 保留加密的 `data`。这两个字段会经过消息与
会话序列化完整保存，仅向同一个 wire、Anthropic provider 和请求模型原样回传。
HTTP 响应的 `model` 保留请求模型，`response_model` 保存服务端报告的名称，
因此模型别名可以继续使用自己的签名历史。跨身份时丢弃全部 thinking 块，
包括 signed 与 redacted 块，绝不转成可见文本。会话导出始终省略这两个不透明字段，
即使包含可读 thinking。
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
| `strict_tool_schema` | 声明 strict 能力；Require/Prefer 工具使用共享 strict 转换器，无约束工具在支持的 wire 上发送 `strict: false`。 |
| `cache_key` | 发送 provider 的 prompt-cache key（cache-affinity 提示）。 |
| `send_session_affinity_headers` | 把请求 `session_id` 映射为兼容的会话亲和 header（OpenRouter 推断启用）。 |
| `require_assistant_after_tool_result` | 在工具结果与下一条 user 消息之间插入固定 assistant 桥接；assistant 工具调用轮次用 `content: ""` 而非 `null`。 |
| `requires_thinking_as_text` | 将 assistant thinking 以纯文本部分回放。 |
| `requires_reasoning_content_on_assistant_messages` | 为回放的 assistant 轮次标记空 `reasoning_content`（DeepSeek 推断）。 |
| `zai_tool_stream` / `vllm_priority` | 仅显式配置的 `tool_stream` 与 `priority` 附加项；从不推断。 |
| `thinking_token_budget_field` / `supports_thinking_token_budget` | 顶层 reasoning 预算字段（vLLM `thinking_token_budget`、Qwen/DashScope/SGLang `thinking_budget`、llama.cpp `thinking_budget_tokens`；alias 选择 vLLM 字段）。 |
| `chat_template_kwargs` / `chat_template_args` | `$var` 模板值（`thinking.enabled`、`thinking.effort`、`thinking.budget`；无 effort 时 `omit_when_off` 丢弃该键）。 |
| `chat_completions_path` | 非默认路径覆盖 suffix。显式 model API base 追加 `/chat/completions`，仅构造器提供的 root 追加 `/v1/chat/completions`。 |

Chat compat 还提供 `supports_finish_reason`、`supports_grammar_tools`、
`open_router_routing`、`vercel_gateway_routing`、`cache_control_format`、
`session_affinity_format` 与 `supports_long_cache_retention`。路由值保留显式
空数组/对象、零价格及 sort partition 的显式 null。`vllm_priority` 为
`Option<serde_json::Number>`；模板映射为 `Option<BTreeMap<...>>`，`None`
继承，`Some(empty)` 清空。catalog codec 通过完整 `x-opi.compat` 快照保留
这些值。`supports_finish_reason: Some(false)` 允许合法流 EOF 关闭已积累的
文本/工具调用，推断结束原因但不编造原始原因。grammar 能力选择 custom 工具
声明、流式输入与回放。Anthropic 缓存标记及长期保留遵循选定能力；Chat 亲和 header
独立于 prompt 缓存并使用完整 session ID。

`OpenAiResponsesSourceCompat` 是普通 Responses、Azure Responses 与 Codex
Responses 唯一的 presence-aware 来源控制所有者。存储元数据中缺失的可选项
保留 `None`，具体 wire 自行解析默认值。普通 Responses 的 strict 默认 false，
Azure/Codex 默认 true；Codex 无约束工具保留 `strict: null`。developer role
默认 true，grammar、时序工具 additions/search、显式缓存与对话中 effort 默认
false。仅普通 Responses 消费 max-output 能力、显式缓存模式与对话中 effort；
其长期缓存能力默认 true。Azure 保留原生最大 token 行为，Codex 省略该字段。
精确匹配的 OpenAI direct signin 调用在准备时冻结来源限定的 auth 分类，
并抑制普通 Responses 的 max/temperature/cache/effort 分支。

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
| `service_tier`（native） | `ResponsesServiceTier`：`auto`/`default`/`flex`/`priority`/`fast` | 省略则字段缺失 | wire `service_tier`，字符串原样。响应观察到的 tier 进入 `Usage::service_tier`；有效计费 tier 单独保留，供按 wire 限定的费用计算使用。 |
| `tool_choice`（native） | `ResponsesToolChoice`：`None`/`Auto`/`Required`/具名函数 | 省略则字段缺失 | wire `tool_choice`（字符串或 `{type: "function", name}`）；simple 条目只接受中性 `Auto`/`None`。 |
| `store` | — | 除非 catalog/profile 显式钉住其他值，恒为 `false`（源无条件发送 `store:false`） | catalog compat > provider config > 默认 `false`。 |
| `max_output_tokens` | `Request::max_tokens`（token） | 省略或零则字段缺失 | 正值以最小 16 发送；simple 调用在此 wire 映射前先解析共享预算。 |
| 加密推理 / 回放 | 持久化 thinking `signature` | 未签名 thinking 绝不捏造 item | 运行 effort 时请求 `include: ["reasoning.encrypted_content"]`；reasoning item 以序列化 wire item 作为签名持久化，回放时（仅同身份回合）原样推回 `input`；`xai` 推理模型恒请求加密内容；终态输出回填补齐逐帧缺失的签名。 |
| 会话中 effort 戳 | catalog compat `supportsMidConvoEffort`（默认 `false`）+ 回放回合的 `provider_thinking_level` | 默认关闭；未记录级别的回合无戳 | 每个记录了级别的早期同模型回复前插入一个 `configuration_update`，末尾一个携带当前 effort；请求 `reasoning.effort` 恒为 `medium`；回复记录自身级别供下一回合使用。 |
| 会话缓存 | `Request::session_id` + `cache_retention` | `Disabled` 全部抑制；否则发送截断（64 字符）的 `prompt_cache_key` | `cache_retention: Long` 追加 `prompt_cache_retention: "24h"`。显式 prompt-cache 模式（`prompt_cache_options`）未采纳——无 opi catalog 消费者。 |

已退役的静态 `reasoning_effort` 兼容性键会被拒绝。`request.thinking`
与所选 `ModelInfo::thinking_level_map` 是 Chat 和 Responses wire 输出的权威来源。
`previous_response_id` 刻意缺省——Responses 请求按
Chat-Completions 类比构造，因此服务端响应链未被接入。

显式 Responses model API base 默认追加 `/responses`，仅构造器提供的 root
默认追加 `/v1/responses`；非默认 compatibility path 保持显式覆盖。

### Azure OpenAI 双 wire（任务 20.38）

Azure 同时保留两条 wire，并按钉住的源 `azure-openai-config.ts` 显式映射
deployment/resource/api-version：

| 映射 | 优先级 | wire 效果 |
|---|---|---|
| Base URL | 显式 `azure_base_url` → 所选 base 配置 → 资源名 → 模型 `baseUrl` → 类型化 `Config` 失败 | 标准 Azure 资源主机的源 Chat/Responses 使用 `/openai/v1`；custom API base 保留路径，Responses 仅在尚未包含 `/responses` 时追加它。 |
| API 版本 | 显式 `azure_api_version` → 所选环境配置 → `v1` | Responses 携带 `?api-version=…`；builtin Chat 不带此 query，legacy direct Chat 保留 versioned deployment URL。 |
| Deployment | 显式 `azure_deployment_name` → provider `modelId=deployment` 映射 → 模型 id | 源 wire 将 deployment 放入 body `model`，保留 catalog 身份；只有 legacy direct Chat 同时放入 URL。 |

Responses wire（`WireApi::AzureOpenAiResponses`、`azure-openai-responses`
记录、经 `Models::register_chat_api` 注册的公开 `AzureOpenAIResponsesChatApi`）
复用完整的 Responses 参数装配——`store: false`、off-effort
`{"effort":"none"}`、summary 默认 `auto`、加密推理 include/签名/回放、
16-token 输出下限——运行在 Azure 传输（`api-key` 头、API-base `/responses` URL、
azure 状态映射、终态 duration、冻结 telemetry producer）之上。
`AzureOpenAIOptions` 携带 native 清单：`reasoning_effort`（类型化预检；
simple 条目就近收敛）、`reasoning_summary`、`tool_choice` 与四个 Azure
映射字段。Azure 刻意不含 `service_tier`（源缺省；本 wire 无 Responses
服务层级倍率）。Azure 不接会话中 effort 戳（源 azure `buildParams` 从不
传递）。Builtin Azure Chat 使用 `openai-completions`，在 API base 追加
`/chat/completions` 并使用 Bearer 认证。独立的 legacy direct Chat 保留
`/openai/deployments/{deployment}/chat/completions` 和 `api-key`。两条源 wire 的
provider 均为 `azure`，endpoint、deployment 和 API-version 在 acquisition 前冻结一次。

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


`ModelsStore::invalidate_source(id)` 显式撤下该来源发布的记录并持久化失效标记。
旧 guard 此后不能发布；离线 `refresh` 无需认证或网络即可观察已附加共享 store
的后续 revision。该来源新的成功发布清除标记；SDK 不默认安装按 HTTP 状态失效的政策。

`catalog::personalize_openrouter_catalog` 接收 owned 目录记录、显式 base URL/key、
HTTP client 与取消 token，返回独立记录：chat 的过滤、限制和价格按 key 生效，
image/classifier 保持不变，真实 bundled artifact 不被改写。缺 key 以及 HTTP
401/403/404 保留输入。来源规定的 15 秒 timeout 包含 body 获取；发布和调度属于调用方。
`CatalogModel::to_chat_model_info` 可将返回的 chat 记录交给普通 SDK 调用。
未知费用为 `None`，明确免费的零费率为 `Some`。来源元数据保留有限负数占位值
和 tier 顺序，计费时严格验证实际选中的费率；caller 的严格价格构造器仍保持验证。


`Credential::ApiKey(ApiKeyCredential)` 保存可省略的 key 和 owned provider env
（`BTreeMap<String, SecretString>`）。缺失字段按该 provider 的来源合同合并，
显式空值保持区别；`ResolvedAuth::env` 冻结所选配置交给 adapter，Debug 不显示值。
受保护 keychain envelope 支持 v2 scoped credential 并严格读取旧 v1，产品配置文件格式不变。
Anthropic federation 只在选择时检查 rule、organization、identity-file 配置，
实际 adapter 首次执行才读取身份文件并向所选 model base 交换 token；缓存与刷新遵循来源，
401 使下一次 acquisition 失效，不隐式重发当前 generation。
取消单次等待后，owner 驱动的交换继续完成；最后一个 owner 释放时中止交换。
后续调用不会重新起算旧 token 的有效期。

`ModelsStoreEntry` 持有三类 `CatalogRecord`，以及可省略的 `last_modified`、
`checked_at`（Unix 毫秒）和原样 `etag`；`None` 与零有区别。动态来源返回按 provider
分组的 entries，model ID 在各 operation 内唯一；恢复/刷新在持久化 ACK 后统一发布三类记录。
Headers 保留有序 set/remove，input limits 仅为元数据；image pricing tiers 保真保存，
image 费用仍按 base rates 计算。

`ModelsStore`（`catalog` 模块）是动态 catalog 的唯一 revision-commit owner：
`RefreshGuard::commit` 先持久化快照（临时文件 + 原子 rename，distribution
schema v1 JSON 加 `opi` revision 头），再返回确切被确认的 map，
`Models::refresh` 随后才把它应用到 registry——内存与磁盘要么一起发布，
要么都不发布。在新 revision 之前创建的 guard 被拒绝为 `Superseded`，
不触碰已发布状态；被取消的 guard 不落盘；重开 store 时先于任何网络
工作恢复最后已知 revision。持久化文件采用 Pi 的 *distribution* schema
（`models.schema.json`），与内置 bundle 内嵌的 hydration 数据（v6）是
不同 artifact；opi 特有保真度（streaming、cache 开关、保持原序的计价分层，含负数／小数阈值及重复项、
base 与 thinking 等级的 sampling 映射、精确 thinking 映射、含 strict-tool 支持的
wire compat 载荷）经显式 `x-opi` 扩展往返，纯 v1 文档
以封闭默认值解码。

`Models::provider_ids` 返回已注册 chat、image、classifier 和 deferred owner
的有序并集。`delete_provider` 和 `clear_providers` 仅移除当前模型 provider 的绑定；
共享凭据／catalog store 和独立注册的纯认证 flow 保持原有调用方所有权。
已捕获的调用保留原 adapter 修订，显式资源清理仍能访问退役的 resource owner。
`get_models` 仍只列 chat；`get_model_of_type`、`get_models_of_type` 和
`get_all_models` 使用封闭的 `CatalogOperation`／`CatalogRecord`，不探测凭据。
异步 `get_available_of_type` 和 `get_all_available` 检查各 operation 已有的
注册配置与账户限制，不解析 builtin generation 认证。

`Models::refresh(ModelsRefreshOptions)` 是唯一刷新入口。默认允许全部当前
provider 的网络工作，不指定强制刷新。`allow_network: false` 仅恢复选中的缓存，
不做 OAuth／网络工作（`RefreshStatus::OfflineRestored`）。`providers: None`
选择全部当前 owner；`Some(empty_set)` 和未知 ID 不执行 source 工作。
`force: None`、`Some(false)`、`Some(true)` 仅在网络阶段原样传给选中的 source，
其新鲜度含义由 source 决定。native provider 按 ID 排序执行，catalog source
随后按注册顺序执行。`Provider::refresh_models` 接收 `CatalogRefreshContext`；
每个 `DynamicCatalogSource` 声明 `provider_ids`，只接收当前选中 owner 的交集。
返回未声明 provider 会使该 source 失败；声明范围内但未选中的记录不能改变其现有发布。
`HttpCatalogSource::new(url, provider_ids)` 声明所有权而不抓取；网络调用仅抓取
一次固定文档，不虚构 force header 或 query 参数。

结果仍按 provider 隔离（`RefreshReport` 中的
`RefreshOutcome::{Published,Failed,Restored}`）：成功者发布，失败者保留旧数据。
native `Ok(None)` 恢复内置回退，除非该 provider 注册的显式 source 仍拥有其
最后已知 catalog；权威空发布保持为空。子集刷新基于完整共享 store 快照合并，
保留未选中的持久化 owner 与 publisher，仅向当前实例应用选中的已确认记录。
即使先前子集已观察到同一 store revision，后续离线子集仍会恢复其缓存。
删除 owner 后不能暴露缓存；显式重新注册允许恢复兼容记录。
`refresh_with(options, &token)` 保持取消检查、store ACK 和旧 revision guard。
被取代或取消的工作不能迟到发布；source 失效不能从未选中的旧内存复活数据。
SDK 不注册动态目录 source、不附加持久化 `ModelsStore`，没有 pi.dev／默认启动
抓取策略。本地内存凭据存储与目录持久化相互独立。

## 内置 Provider 工厂层：42 个 canonical profile（任务 20.34）

`providers::builtin_provider_profiles` 携带 pi 1.3.0 能力清单中逐字提
取的 42 条 profile 表：canonical id（`google`、`google-vertex`、
`amazon-bedrock` 等）、显示名、默认 base URL、env API-key 名、auth
scheme、OAuth 可用性与端点语义（Azure 部署、Vertex location 模板、
Bedrock 区域、动态 radius）。旧的 opi id（`gemini`、`vertex`、
`bedrock`、`openai-responses`）不是别名——表从不注册它们。
provider 显示名包括 Amazon Bedrock、Anthropic 与 Google Vertex AI；凭证标签
作为独立元数据保留。
`provider_collection::register_builtin` 把该表与内置目录转成基于已完
成 wire 适配器的注册 provider：每 profile 一个 mapped provider（Azure
与 google-vertex 受配置门控并给出类型化跳过原因，bedrock 以 AWS 链
描述符 lookup-only 注册），canonical-route 包装器把固定 id 适配器重发
布为 canonical id（含前缀敏感适配器所需的 model-spec 前缀重写），目
录条目连同分层计价转换进 `ModelInfo::pricing`，classifier/image 协议
经类型化缝以目录记录注册。环境 key 在派发时经注入的 `AuthContext`
解析——构造不读环境、存储或网络。

## 公共入口

主要接入点包括 `Models`、`ProviderRegistry`、`Provider`、`AuthResolver`、
`CredentialStore`、`Catalog`、`ModelsStore`、`Models::refresh` 和
`ModelsRefreshOptions`。完整符号清单见主 README 的 Core API 表；本专题各节
承载生命周期约束以及 Provider/Catalog 选择规则。

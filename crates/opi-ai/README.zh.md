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

当前 crate 版本是 `0.7.0`，继承自 workspace 包版本。

`opi-ai` 负责模型/Provider 层：请求和消息类型、流式事件、模型元数据、Provider
注册、HTTP/代理连接、重试辅助、图片内容、用量累计、尽力而为的费用辅助，以及供
`opi-agent` 诊断层使用的 Provider 侧错误分类。它不实现 Agent 主循环、会话、
package 加载或内置编程工具；这些能力分别位于 `opi-agent` 和 `opi-coding-agent`。

`opi-ai` 还暴露一个 unstable-0.x 的模型/鉴权 seam：`provider_collection`
模块（`ProviderCollection`）在 `ProviderRegistry` 之上叠加 Provider 侧鉴权契约、
OpenAI-compatible 兼容性元数据，以及 stream/complete 派发。无 IO 的凭据与 OAuth
trait 位于本 crate；keychain、环境变量、登录 presenter 和 refresh 实现仍位于
`opi-coding-agent`。

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
| `bedrock` | `bedrock` | AWS Bedrock Converse streaming，使用 SigV4 签名 |
| `azure_openai` | `azure` | Azure OpenAI deployment 专用 Chat Completions |
| `vertex` | `vertex` | Google Vertex AI Gemini streaming |

内置模型列表刻意保持有限，主要用于能力校验和模型列表输出。站点专用模型、
fine-tuned 模型和 deployment 应通过 registry override 或配置的
OpenAI-compatible profile 加入。

## 核心 API

| 项 | 作用 |
|----|------|
| `Provider` | 后端 trait，包含 `id`、`models` 和 `stream(Request)`。 |
| `Request` / `CacheRetention` | Provider 请求：模型、消息、工具、token 限制、thinking 配置、metadata、取消、timeout、额外 header、cache retention 和会话亲和。 |
| `Message` | 面向 Provider 的 user、assistant 和 tool-result 消息。 |
| `InputContent` / `OutputContent` | 文本与图片内容块。 |
| `ToolResultMessage` | 面向 Provider 的工具结果消息：content、可选 details、`is_error`、`truncated` 和时间戳元数据。 |
| `AssistantStreamEvent` | Provider 无关流式事件，覆盖 start、text、thinking、tool call、done 和 error。 |
| `WireApi` / `ModelInfo` / `ModelCapabilities` | 精确请求 wire identity，以及模型能力、thinking、compatibility 与 pricing 元数据。 |
| `ApiMappedProvider` | 通过经校验的 `WireApi -> Provider` route map 派发一个公开 Provider identity/catalog。 |
| `ProviderError` / `ProviderErrorCategory` | Provider 失败分类：auth、config、request、network、rate_limit、provider、stream、capability 和 cancelled（超时归为 network）。 |
| `ProviderRegistry` | 解析 `provider:model`、注册自定义 Provider、叠加模型覆盖。 |
| `ProviderCollection` / `AuthDescriptor` / `AuthStatus` | unstable-0.x 模型/鉴权 seam，位于 `ProviderRegistry` 之上：Provider+模型查找、脱敏鉴权状态、OpenAI-compatible 兼容性元数据、派发与原子动态目录 refresh。 |
| `CredentialStore` / `Credential` / `CredentialSource` | 无 IO、object-safe 的凭据持久化与已脱敏三态探测契约。 |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | 与 flow 无关的 boxed-future OAuth 契约；具体 flow 位于 `opi-coding-agent`。 |
| `AuthResolver` / `ResolvedAuth` | 在 Provider HTTP 前使用的按 stream 鉴权解析契约。 |
| `ApiKind` | crate 根枚举，标注 assistant 消息携带的后端家族（`Anthropic`、`OpenAi`、`Google`、`Mistral`）。 |
| `HttpClient` | 共享 `reqwest` client，支持连接池和显式/环境变量代理。 |
| `retry` | 重试配置、指数退避和 `Retry-After` 解析。 |
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
每个 route 都在返回的 stream 内、紧邻 HTTP 之前解析
`AuthResolver`。凭据缺失与撤销分别成为显式且不可重试的
`ProviderError::CredentialNeeded` 和 `ProviderError::CredentialRevoked`。
按调用凭据仍不在范围内：`extra_headers` 会拒绝 Provider 管理的鉴权 header。

`ProviderError::AccountIdMissing { provider_id }` 是独立且不可重试的情形：凭据存在，
但缺少所选 wire 要求的 account identity。产品层通过显式 `/login <provider>` 修复；
它不表示凭据已撤销。

具备能力的 Anthropic 内置模型会在 system prompt、最后一段 user text、最后一段
assistant text 和最后一个 tool definition 上发出 `cache_control`。Long retention
会增加 `ttl: "1h"`；short/default ephemeral retention 不带 TTL，显式禁用以及
未知/自定义模型不发出 marker。

`Request` 新增 `timeout`、`extra_headers`、`CacheRetention` 和 `session_id`。前三项是
公开的 request-to-wire 基底；本阶段只有 `session_id` 在 coding harness 中具有生产生成方。
session-affinity 行为在直连 Responses 与自定义/proxy route 之间不同，详见下文；
Anthropic 则使用模型能力门控的 cache marker。`ModelInfo` 包含现有的嵌套
`ModelCapabilities`；未知/自定义模型默认关闭 cache 支持。

`Usage::cache_write_1h_tokens` 与 `Usage::reasoning_tokens` 是可选 `u64` 子集，
可区分缺失与显式上报的零。`CostBreakdown` 只有四行：`input_cost`、`output_cost`、
`cache_read_cost` 和 `cache_write_cost`。加权的一小时 write 子集折算进
`cache_write_cost`，reasoning 仍计入 `output_cost`，费用与总 token 只计一次父 bucket。
`Provider::refresh_models` 与 `ProviderCollection::refresh` 实现确定性的原子目录替换，
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
// Cargo.toml 依赖：opi-ai、tokio（features "macros"、"rt-multi-thread"）、
// tokio-util、futures-util。
use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{Provider, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = AnthropicProvider::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        None,
    );

    let request = Request {
        model: "claude-sonnet-4-5-20250514".into(),
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
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
    };

    let mut stream = provider.stream(request);
    while let Some(event) = stream.next().await {
        println!("{:?}", event?);
    }
    Ok(())
}
```

## 模块

`provider`、`message`、`stream`、`model_info`、`api_mapped`、`registry`、
`provider_collection`、`auth`、`credential`、`http`、`retry`、`model`、
`anthropic`、`openai_chat`、`openai_responses`、`openai_codex_responses`、
`openrouter`、`mistral`、`gemini`、`bedrock`、`azure_openai`、`vertex`、
`config`、`time` 和 `test_support`。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。

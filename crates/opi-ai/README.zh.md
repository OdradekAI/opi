# opi-ai

[![Crates.io](https://img.shields.io/crates/v/opi-ai.svg)](https://crates.io/crates/opi-ai)
[![Docs.rs](https://docs.rs/opi-ai/badge.svg)](https://docs.rs/opi-ai)

> 面向 Rust 的 Provider 无关 LLM API。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-ai` 在内置 Provider、checked multi-wire adapter 和配置驱动 profile 之间提供
统一的 request、message 与 streaming 模型。它是 library crate，不拥有 Agent 主循环、
session、package loading 或 coding-agent 工具。

当前 workspace version `0.8.3`。

## 使用指南

新接入可以先沿着下面的短路径进行。后续链接是能力和参考页面；第一次成功调用不需要
阅读每个 Provider 的实现细节。

| 目标 | 从这里开始 |
| --- | --- |
| 在外部 Cargo 项目安装 | [安装](#安装) |
| 选择默认或核心 feature profile | [Feature profile](#feature-profile) |
| 发起一次普通请求 | [快速开始](#快速开始) |
| 消费增量并取得最终消息 | [流式与最终结果](#流式与最终结果) |
| 构造 Context、工具与续轮 | [Context、工具与续轮](#context工具与续轮) |
| 注册模型并选择 Provider | [Provider](#provider)；[Provider、Catalog、认证与刷新参考](docs/provider-auth.zh.md) |
| 使用 transport、utility 或 Provider 细节 | [Transport](docs/transport.zh.md)；[Utilities](docs/utilities.zh.md)；[Provider Wire 与操作参考](docs/providers.zh.md) |
| 无凭据测试 | [Faux Provider 测试](#faux-provider-测试)；[测试与包消费](docs/testing.zh.md) |
| 理解错误、取消、telemetry 与 deferred | [错误处理](#错误处理)；[Telemetry：显式上下文与可选 tracing](#telemetry显式上下文与可选-tracing)；[Deferred 响应](#deferred-响应) |

## 安装

```sh
cargo add opi-ai
```

复制三个 onboarding 示例的外部 consumer 可以使用这份完整清单：

```toml
[dependencies]
opi-ai = "0.8.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
futures-util = "0.3"
serde_json = "1"
```

这个项目只依赖已发布的 `opi-ai` 与普通公开 runtime/serialization crate，不要求
`opi-agent`、workspace、仓库路径或隐式全局初始化。

## Feature profile

`catalog-bundled`、`http-transport` 与 `telemetry-tracing` 可独立选择。
`tls-backend`、`cloud-auth`、`websocket-compression`、`protocol-adapters` 与
`oauth-store` 也可独立选择。为保持兼容，`builtins` 包含 catalog、共享
HTTP transport、provider protocol adapters 和 provider OAuth flow，默认配置仍
保留原有完整 surface。

Provider protocol surface 还提供独立的 source owner：
`provider-anthropic`、`provider-azure-openai`、`provider-bedrock`、
`provider-gemini`、`provider-mistral`、`provider-openai-chat`、
`provider-openai-codex-responses`、`provider-openai-responses`、
`provider-openrouter`、`provider-pi-messages` 与 `provider-vertex`。
`provider-protocol-common` 与 `provider-openrouter-usage` 是内部共享 owner；
直接调用方应选择实际使用的 provider feature。

| Profile | Cargo 写法 | 可用范围 |
| --- | --- | --- |
| 默认 | `opi-ai = "0.8.3"` | 核心、内置适配器/认证、内嵌 catalog 和 tracing carrier。 |
| 最小 | `default-features = false` | 核心值、Models/自定义 API、凭据 Store/自定义 OAuth、中立 telemetry、工具与脚本化 Faux 测试支持。 |
| 仅内嵌 catalog | `default-features = false, features = ["catalog-bundled"]` | 核心 catalog 值与经过校验的 opi 内嵌 catalog，不启用网络适配器或内置认证。 |
| 仅共享 HTTP transport | `default-features = false, features = ["http-transport"]` | 公共 `HttpClient`、代理/重定向策略和 source-scoped TLS wiring，不启用具体 provider protocol 或内置认证。 |
| 仅 TLS backend | `default-features = false, features = ["tls-backend"]` | 固定 rustls/AWS-LC/native-certs backend 与公共 source-scoped `opi_ai::tls` 配置，不启用 reqwest 或 provider transport。 |
| 仅云认证 | `default-features = false, features = ["cloud-auth"]` | AWS source chain/STS、GCP ADC、共享 SigV4 与固定云 codec，并传递选择共享 HTTP/TLS，不启用 provider wire adapter 或 compression。 |
| 仅 WebSocket/compression | `default-features = false, features = ["websocket-compression"]` | 公共严格 Upgrade bridge 与私有 RFC 6455/permessage-deflate owner，并传递选择共享 HTTP/TLS，不启用 provider adapter。 |
| 仅 Provider protocol adapters | `default-features = false, features = ["protocol-adapters"]` | 所有 provider wire owner 与共享协议依赖的兼容性组合；不启用内嵌 catalog 或 provider collection/auth factory。 |
| 单个 Provider protocol | `default-features = false, features = ["provider-anthropic"]`（替换为对应 provider owner） | 单个 provider wire/source owner 及其声明的共享协议依赖；完整的十一项 owner 见能力矩阵。 |
| 仅 Provider OAuth/store | `default-features = false, features = ["oauth-store"]` | Provider 自有 OAuth flow 与 Store-backed login/refresh 接线，并传递选择共享 HTTP transport；中立 credential、mutation、Store 和 OAuth 合同仍在 core。 |
| 内置后端无 tracing | `default-features = false, features = ["builtins"]` | 内置网络适配器、云认证与 catalog，不启用可选 tracing carrier。 |

```toml
opi-ai = { version = "0.8.3", default-features = false }
```

最小配置的依赖图排除 reqwest、TLS、云签名/协议 codec 和 WebSocket 压缩后端；
保留 Tokio、HTTP 值类型，以及现有 `MockProvider`/`ScriptedDeferredBackend`，不声称
完整 Pi Faux 对等。`http-transport` 拥有 `HttpClient`、代理/重定向合同和
source-scoped TLS/crypto wiring，但不启用具体 provider protocol 模块。
`tls-backend` 拥有固定的 rustls/AWS-LC/native-certs backend 与公共
source-scoped `opi_ai::tls` 配置，不选择 reqwest；`http-transport` 为了
`HttpClient` 传递选择 `tls-backend`，同时保留原有 `opi_ai::http` TLS re-export。
`protocol-adapters` 是覆盖所有 provider wire owner 的兼容性组合，并传递选择
`http-transport`、`cloud-auth` 与 `websocket-compression`，但不启用内嵌 catalog。中立的 `auth::store_oauth` 合同和
credential value 保留在 core。`oauth-store` 拥有具体 provider OAuth flow 与
Store-backed login/refresh bridge，并传递选择 `http-transport`；内置
OAuth/AWS/Vertex credential factory 及 provider collection 因注册内嵌 catalog
和 provider credential source，仍属于 `builtins`。
`cloud-auth` 拥有协议无关的 AWS source chain/STS、GCP ADC、共享 SigV4 与云
codec 闭包，并传递选择共享 HTTP/TLS，不选择具体 provider wire 或 compression。
`Catalog::from_bytes` 与自定义 catalog source 仍可用；
`Catalog::builtin` 要求 `catalog-bundled`，HTTP catalog source 和 provider
collection 要求 `builtins`，具体 Provider 模块要求对应的 `provider-*` feature
（或兼容性组合 `protocol-adapters`/`builtins`）。关闭 feature 时，归档中仍可能包含内嵌 catalog 数据文件。

已有 no-default 网络消费者须增加 `features = ["builtins"]`；只使用固定 source-scoped
TLS 配置的调用方可选择 `features = ["tls-backend"]`，需要 `HttpClient` 的调用方
可选择 `features = ["http-transport"]`；只需要公共 WebSocket Upgrade/PMD bridge
的调用方可选择 `features = ["websocket-compression"]`，它会传递选择共享 HTTP/TLS。
需要自行提供 provider wire 但使用 AWS/GCP source resolution 或 signing 的调用方可选择
`features = ["cloud-auth"]`，它会传递选择共享 HTTP/TLS，不启用 provider protocol/compression。
直接提供自身 model/auth/catalog
接线的 protocol 调用方可选择 `features = ["protocol-adapters"]`；provider
collection 与内置 OAuth/AWS/Vertex credential factory 仍要求 `builtins`；自行
提供 provider registry 的调用方可选择 `features = ["oauth-store"]` 使用 SDK
OAuth flow 和 Store-backed refresh bridge。只使用一个具体 wire 的调用方可从上面的
owner 列表选择对应的 `provider-*` feature；`provider-vertex`/`provider-bedrock`
保留传递选择 cloud-auth，`provider-openai-codex-responses` 保留 WebSocket/compression
依赖。三个 onboarding 示例要求 `builtins`。最小消费者若需要
`TelemetryContext::from_span`，可独立开启 `telemetry-tracing`。exporter、sampling、
初始化和 shutdown 仍由宿主拥有。配对/高阶组合覆盖和原生 macOS 仍在 capability V
矩阵中分开计量。当前已有 55 个 provider-owner 配对、2 个选定高阶 provider
组合、2 个共享 owner 配对和 Windows all-features 通过。完整 capability V 仍有
未枚举的高阶组合、原生 macOS、Deferred 真实生产证据和 performance/fuzz 预算待闭合。
当前 receipt 位于 `docs/iterations/opi-ai-1-5-alignment/evidence/`。

## API 总览

公共 surface 围绕调用方持有的值和一个 execution driver 组织：

| 目标 | 公共入口 | 责任 |
| --- | --- | --- |
| 注册并选择模型 | `Models` / `ProviderRegistry` / `register_builtin` | 构造 catalog 或显式 route，构造阶段不读取认证。 |
| 描述一次调用 | `Request` / `ChatModelRecord` | 携带规范化 Context、模型身份、限制、取消和可选 telemetry。 |
| 完成一次调用 | `Models::complete_simple` | 执行到最终 `AssistantMessage` 或类型化 `CallFailure`。 |
| 消费 stream | `ChatCall` / `AssistantStreamEvent` / `Models::stream_simple` | 观察增量、终态、取消和最终消息。 |
| 构造 Context 与工具 | `normalize_context` / `ToolDef` / `transcript` | 将 system text、message 和 tool declaration 归一为规范时序。 |
| 观察执行 | `TelemetryContext` / `telemetry-tracing` | 注入显式 context，并选择 `opi-telemetry` tracing carrier。 |

精确签名、trait bounds 和完整字段说明以
[rustdoc](https://docs.rs/opi-ai) 为准。

## 参考索引

第一次成功调用后可以使用下面的包内专题：

| 能力 | 参考 |
| --- | --- |
| Provider 选择、catalog、认证和 refresh | [Provider、Catalog、认证与刷新参考](docs/provider-auth.zh.md) |
| 估算、pricing tier、cache pricing 和上报成本 | [费用与计价参考](docs/cost-pricing.zh.md) |
| Proxy、TLS、WebSocket、deferred transport 和 cleanup | [Transport、协议与资源生命周期](docs/transport.zh.md) |
| Schema、transcript、replay、identity、retry 和 callback | [公共工具与数据契约](docs/utilities.zh.md) |
| Provider 专属 wire 与 operation | [Provider Wire 与操作参考](docs/providers.zh.md) |
| Faux、fixture 所有权和包证明 | [测试与包消费](docs/testing.zh.md) |

## 快速开始

一次完整的普通调用使用注入的 `AuthContext`、调用方持有的 `ChatModelRecord`、规范
transcript 归一化和标准 `complete_simple` 结果契约。以下是
`examples/quick_start.rs` 的逐字源码：

```rust
//! Complete ordinary quick start: register lazy builtins, select a model,
//! and drive one chat call to its single final message through the neutral entry.
//!
//! Compile: `cargo check --example quick_start`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example quick_start`.

use std::future::Future;
use std::sync::Arc;

use opi_ai::CallFailure;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript;
use opi_ai::{AmbientAuthContext, AuthContext, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

/// Construct and statically select without reading authentication sources.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<(Models, ChatModelRecord), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    Ok((models, ChatModelRecord::new("anthropic", info)))
}

/// Schedule the owned call on Tokio; setup does not wait for result consumption.
pub(crate) fn start(
    models: &Models,
    record: ChatModelRecord,
) -> Result<
    impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<>,
    Box<dyn std::error::Error>,
> {
    // Canonical transcript input: the system prompt and tool
    // declarations fold into one leading system message at normalization.
    let context = transcript::normalize_context(transcript::Context {
        system_prompt: Some("You are concise.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "In one short sentence, say hello.".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })?;

    let request = Request {
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5".into(),
        context,
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
    };

    Ok(models.complete_simple(record, request, SimpleOptions::default()))
}

pub(crate) async fn finish(
    call: impl Future<Output = Result<AssistantMessage, CallFailure>>,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    let message = call
        .await
        .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;
    for content in &message.content {
        if let AssistantContent::Text { text, .. } = content {
            println!("{text}");
        }
    }
    Ok(message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, record) = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = start(&models, record)?;
    finish(call).await?;
    Ok(())
}
```

构造阶段不读取产品配置或访问 Provider。认证通过选定 route 注入；调用返回一个
`Ok(AssistantMessage)` 或类型化的 `Err(CallFailure)`。

## 流式与最终结果

`Models::stream_simple` 返回 `ChatCall`。构造时调度执行，consumer 控制 event poll。
`TextDelta` 等 partial event 之后会交付一个终态 `Done` event 或一个
`Err(CallFailure)`。`ChatCall::result()` 等待同一个最终结果。普通聊天调用在准备阶段
合成错误消息时，会从冻结的 `ChatModelRecord` 取得 provider 和 model ID。

每个 partial 都是该事件独立拥有的快照。进度展示读取最新事件的 partial。保留旧事件时，
其内容和用量保持在原事件时点。工具预览为每个工具保留一个
`streaming_json::ToolCallArgumentsStream`，逐条追加原始 `ToolCallDelta`，在渲染时请求
独立快照。详见[预览与最终校验契约](docs/utilities.zh.md#tool-参数归一化与校验)。

OpenAI Chat 的一次 completion 使用一个文本块。该块的文本增量之间可以收到工具增量。
按事件顺序更新进度，并用 `content_index` 将增量归到各自的块。completion 关闭内容块时，
`TextEnd` 携带该块的完整正文。

收到业务终态后，普通调用会继续驱动 Provider source 到 EOF，再交付终态、结算请求
telemetry 并完成 result。尾部事件不能替换首个结果或触发另一次 attempt。停止消费或
丢弃 call 不会停止这项工作。不结束的 source 尾部会使调用保持 pending：请求 token
取消需要 source 配合，`ChatCall::abort()` 则停止并等待执行任务退出；这不代表等待
自定义 Provider 内部独立启动的所有任务。

以下是 `examples/quick_start_stream.rs` 的逐字源码：

```rust
//! Streaming quick start: iterate the neutral entry's provider-neutral
//! events as they arrive, then read the terminal event's final message.
//!
//! Compile: `cargo check --example quick_start_stream`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example quick_start_stream`.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript;
use opi_ai::{AmbientAuthContext, AuthContext, ChatCall, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

/// Construct and statically select without reading authentication sources.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<(Models, ChatModelRecord), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    Ok((models, ChatModelRecord::new("anthropic", info)))
}

/// Schedule the ChatCall on Tokio before consuming its events.
pub(crate) fn start(
    models: &Models,
    record: ChatModelRecord,
) -> Result<ChatCall, Box<dyn std::error::Error>> {
    let context = transcript::normalize_context(transcript::Context {
        system_prompt: Some("You are concise.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Count from one to five.".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })?;

    let request = Request {
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5".into(),
        context,
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
    };

    Ok(models.stream_simple(record, request, SimpleOptions::default()))
}

pub(crate) async fn finish(
    mut call: ChatCall,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    // Done carries the final message; after it the call stays at EOF.
    let mut final_message = None;
    while let Some(event) = call.next().await {
        match event.map_err(|failure| format!("call failed: {:?}", failure.cause()))? {
            AssistantStreamEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantStreamEvent::Done { message, .. } => {
                println!();
                eprintln!("-- stop reason: {:?}", message.stop_reason);
                final_message = Some(message);
            }
            _ => {}
        }
    }
    final_message.ok_or_else(|| "stream ended without a final message".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, record) = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = start(&models, record)?;
    finish(call).await?;
    Ok(())
}
```

启动调用与消费调用是两个操作。丢弃 consumer 不会隐式取消底层执行；需要取消时使用
request 的 cancellation token。传输和 cleanup 所有权见[Transport、协议与资源生命周期](docs/transport.zh.md)。

## Context、工具与续轮

`transcript::normalize_context` 生成所有 adapter 共用的规范时序。system text、message
和 tool declaration 只归一化一次；续轮会携带 assistant tool call 与 `ToolResultMessage`
结果重放完整时序。

调用方执行工具并追加结果。畸形最终参数通过 `ToolNotExecutedReason` 记录为未执行；SDK
不会猜测 partial call。以下是 `examples/tool_loop.rs` 的逐字源码：

```rust
//! Agent-owned tool loop: declare a tool through the canonical transcript,
//! execute each admitted tool call, close it with a `ToolResultMessage`, and
//! continue until the assistant produces final text.
//!
//! Compile: `cargo check --example tool_loop`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example tool_loop`.

use std::sync::Arc;

use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall, ToolDef,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript::{self, TranscriptContext};
use opi_ai::{AmbientAuthContext, AuthContext, ChatCall, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

const MODEL_SPEC: &str = "anthropic:claude-sonnet-4-5";

struct ToolOutcome {
    text: String,
    is_error: bool,
    not_executed: Option<ToolNotExecutedReason>,
}

/// The caller is the trusted execution boundary: a malformed final
/// argument text is recorded as not executed, never guessed into a call.
fn execute_weather_tool(call: &ToolCall) -> ToolOutcome {
    if call.arguments_error.is_some() {
        return ToolOutcome {
            text: "tool arguments were not a complete JSON object".into(),
            is_error: true,
            not_executed: Some(ToolNotExecutedReason::NonObjectArguments),
        };
    }
    let arguments: serde_json::Value =
        serde_json::from_str(&call.arguments).unwrap_or_else(|_| serde_json::json!({}));
    let city = arguments.get("city").and_then(|value| value.as_str());
    let report = match city {
        Some("Tokyo") => "22 C, clear".to_string(),
        Some(other) => format!("no data for {other}"),
        None => "missing required field: city".to_string(),
    };
    ToolOutcome {
        text: report,
        is_error: city.is_none(),
        not_executed: None,
    }
}

fn request_for(context: TranscriptContext) -> Request {
    Request {
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: MODEL_SPEC.into(),
        context,
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
    }
}

pub(crate) struct ToolLoop {
    pub(crate) models: Models,
    record: ChatModelRecord,
    history: Vec<Message>,
    tools: Vec<ToolDef>,
}

/// Construct and select statically; the first round is built separately.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<ToolLoop, Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Tools are declared on the transcript context; normalization folds them
    // into the leading system message.
    let tools = vec![ToolDef {
        name: "get_weather".into(),
        description: "Current weather for one city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
        }),
        constrained_sampling: None,
    }];

    let history: Vec<Message> = vec![Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "What is the weather in Tokyo? Use the tool.".into(),
        }],
        timestamp_ms: 0,
    })];

    Ok(ToolLoop {
        models,
        record,
        history,
        tools,
    })
}

impl ToolLoop {
    /// Freeze this round and schedule its authentication and execution on Tokio.
    pub(crate) fn start_round(&self) -> Result<ChatCall, Box<dyn std::error::Error>> {
        let context = transcript::normalize_context(transcript::Context {
            system_prompt: Some("You are a concise assistant.".into()),
            messages: self.history.clone(),
            tools: self.tools.clone(),
        })?;
        Ok(self.models.stream_simple(
            self.record.clone(),
            request_for(context),
            SimpleOptions::default(),
        ))
    }

    pub(crate) async fn finish(
        mut self,
        mut call: ChatCall,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        // Each round replays the full canonical chronology, including tool results.
        for round in 0..8 {
            let assistant = call
                .result()
                .await
                .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;
            let tool_calls: Vec<ToolCall> = assistant
                .content
                .iter()
                .filter_map(|content| match content {
                    AssistantContent::ToolCall { tool_call } => Some(tool_call.clone()),
                    _ => None,
                })
                .collect();
            if tool_calls.is_empty() {
                for content in &assistant.content {
                    if let AssistantContent::Text { text, .. } = content {
                        println!("{text}");
                    }
                }
                return Ok(assistant);
            }
            self.history.push(Message::Assistant(assistant));
            for tool_call in tool_calls {
                let outcome = execute_weather_tool(&tool_call);
                self.history.push(Message::ToolResult(ToolResultMessage {
                    tool_call_id: tool_call.id,
                    tool_name: tool_call.name,
                    content: vec![OutputContent::Text { text: outcome.text }],
                    details: None,
                    is_error: outcome.is_error,
                    not_executed: outcome.not_executed,
                    truncated: false,
                    timestamp_ms: 0,
                }));
            }
            if round == 7 {
                break;
            }
            call = self.start_round()?;
        }
        Err("tool loop did not converge".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = state.start_round()?;
    state.finish(call).await?;
    Ok(())
}
```

Schema、replay、transcript、header 和 callback 细节见[公共工具与数据契约](docs/utilities.zh.md)。

## Provider

`Models` 与 `ProviderRegistry` 选择模型或显式 Provider route。`register_builtin` 安装已校验
的内置 profile 与 catalog，不代表登录、网络访问或 ambient 配置读取。完整的 Provider、Catalog、认证参考见[Provider、Catalog、认证与刷新参考](docs/provider-auth.zh.md)。凭据、OAuth、Request 扩充和 Refresh 生命周期维护在该专题。

Provider 矩阵、compatibility profile、图像、分类器、native option 和 direct adapter 见
[Provider Wire 与操作参考](docs/providers.zh.md)。

## 图片支持

图像输入使用带顺序的 `InputContent::Image`，携带显式 media type 和 source metadata。
图像生成是独立 operation，拥有自己的 model record、context、output 和类型化 result。
route 限制见[Provider Wire 与操作参考](docs/providers.zh.md)。

## 分类器

Classification 是独立 operation 和 catalog 类型。公共结果映射为 `ClassifierOutput`；
缺失、畸形、拒答和非 finite answer 都是类型化失败。Decisions、System One 和 llama.cpp
使用各自的 wire envelope。

## 思考与推理

simple 路径携带 `ThinkingLevel`；Provider 有额外 reasoning 控制时通过类型化 option 暴露。
当 Provider 区分省略、零和 false 时，省略保持省略。thinking 与 redacted-thinking block
保持顺序；route 无法保留时不会变成可见文本。

Stop reason 归一到 `Stop`、`Length`、`ToolUse`、`Error`、`Aborted`、`Deferred` 和
`Pending`。Provider raw reason 只通过类型化 failure 或 adapter diagnostic 暴露。

## 错误处理

Provider、protocol、认证、schema、callback 和 cancellation failure 通过类型化的
`CallFailure`/`ProviderError` cause 表达。错误 summary 会限长并脱敏，不回显 response body
或 secret。未知模型、不支持的 option、非法 tool/schema 组合会在成功 Provider event 发布前失败。

取消是显式且可观察的。abort、timeout、Provider terminal error 和停止消费是不同结果。
重试由 logical call 或 Provider adapter 拥有；显式 abort 和 callback failure 不会触发重发。
用量和 best-effort pricing 详见[费用与计价参考](docs/cost-pricing.zh.md)。

## 尽力而为的费用

费用是 best-effort。未知用量或缺失定价得到未知费用；明确的免费费率保留为显式零值。
完整的估算器、sampling、tier 选择、cache pricing、service tier 和 provider 上报成本规则见
[费用与计价参考](docs/cost-pricing.zh.md)。

## Telemetry：显式上下文与可选 tracing

宿主需要 AI request 观测时，在 `Request` 上传入显式 `TelemetryContext`。默认
`telemetry-tracing` feature 提供可选 tracing carrier；`default-features = false` 保留中立
context 和 recorder。exporter、sampling、初始化和 flush 由宿主持有。

普通 chat request 产生 `pi.ai.request`；deferred fetch 产生 `pi.ai.poll`。Pending 和
Deferred poll 不累计 token 或 cost。

自动观察器在首次交付规范化流项时记录 `pi.ai.first_chunk` 与
`pi.ai.first_chunk.duration`。首项为类型化失败时也记录，包括 provider 派发前的准备失败。

AI 观察器先创建所需指标，再启动 span。指标工厂返回错误时，自动观察器通过宿主 tracing
报告错误，业务调用继续执行。手动调用 `AiRequestTelemetry::start` 会直接收到后端错误。

当 `pi.ai.request` 以 `Stop`、`Length` 或 `ToolUse` 完成时，若最终属性写入或请求时长
记录失败，自动观察器通过宿主 tracing 报告该错误，并尝试结束请求 span 一次。已写入的属性
保留，剩余的结果指标跳过。结束操作若返回错误，会单独发出一条警告。调用仍交付成功的模型结果。

raw stream 和 prepared raw attempt 仍可在无隐式
ambient context 下使用。完整生命周期见[Transport、协议与资源生命周期](docs/transport.zh.md)。

## 自定义 Provider

自定义 Provider 可以实现 `Provider`、`ChatApi`、image、classifier 或 deferred operation
契约，并注册到 `Models`。`CompatConfig` 是显式 profile 覆盖，不能新增未支持的 wire 能力
或替换 Provider 管理的认证。direct adapter option 携带自己的 `ApiId` 和类型化身份。

操作边界见[Provider Wire 与操作参考](docs/providers.zh.md)，精确 trait 签名以 rustdoc 为准。

## Faux Provider 测试

需要无凭据的确定性测试时，使用 `test_support::MockProvider` 和 Faux 注册 helper，显式脚本化
response、stream、tool call、pending 和 failure。Faux 测试证明调用方行为和生命周期接线，不
证明真实 Provider 的服务可用性或认证。

测试分层和外部包规则见[测试与包消费](docs/testing.zh.md)。

## Deferred 响应

Deferred submit 可能返回带 `StopReason::Deferred`、空 content 和类型化 handle 的消息。
有效的 fetch 使用 `wait_ms = 0` 时，请求后端进行一次状态检查。
`Models::fetch_deferred` 会返回仍待定的 `StopReason::Deferred` 及 handle、就绪消息或类型化失败。
收到成功终态后，它会把 fetch 流读到 EOF 再返回。在 Tokio runtime 内迁移 Pi 的结果式
`Models.fetchDeferred` 调用时，可等待 `Models::fetch_deferred_ready`。它返回首个规范化结果，
任务继续排空源流并结算 poll。待定响应仍带有 handle，供调用方发起下次 fetch。
从 Pi 的 `Models.streamDeferred` 迁移时，
在 Tokio runtime 内调用 `Models::stream_deferred(...).start()`。任务独立于事件读取推进
fetch，丢弃消费句柄后仍继续执行。成功 `Done` 可在收尾前交付，公开 EOF 和 `result()`
等到源流释放及 poll 结算。`cancel_deferred` 是 best-effort 远端取消，与本地
fetch 取消独立。`poll_after_ms` 是 Provider 提供给调用方的调度提示。

确定性的 Faux route 不代表 startup fetch、全局 deferred worker 或真实远端服务。
详见[Transport、协议与资源生命周期](docs/transport.zh.md)。

## 迁移

低层 mapper 调用方现在使用 `anthropic::AnthropicUsageDelta`，而非 `Usage`，构造
`AnthropicEvent::MessageDelta.usage`。已上报的 token 字段使用 `Some(value)`，包括
清零的 `Some(0)`；`None` 保留原值，不变字段可通过 `..Default::default()` 省略。

公共 Rust surface 处于 unstable 0.x。优先使用当前的 `Models`、`Request`、`ChatCall`、
`ProviderRegistry`、`AuthContext`、`TelemetryContext` 和类型化 operation 契约。从内部
workspace caller 迁移时，用显式 Provider 注册和注入认证替代隐式全局 setup；用已发布 crate
和包内专题链接替代 workspace-only path。

## 开发

包内 examples 是 canonical onboarding 程序，并在普通外部 consumer 中执行。维护者应使用
[测试与包消费](docs/testing.zh.md)；需要精确类型时使用
[rustdoc](https://docs.rs/opi-ai)。transport、utility 和 Provider 细节可从[参考索引](#参考索引)
进入。

## 许可证

MIT。见随包的 [LICENSE](LICENSE) 和 [NOTICE](NOTICE)。

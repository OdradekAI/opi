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

当前 `opi-ai` 只公开一个 feature：`telemetry-tracing`：

| Profile | Cargo 写法 | 增加或移除 |
| --- | --- | --- |
| 默认 | `default` / `telemetry-tracing` | 开启 `opi-telemetry` 的可选 tracing carrier，以及 `TelemetryContext::from_span`。 |
| 核心 context | `default-features = false` | 保留中立 context、recorder 和 AI request 类型，但不启用可选 tracing carrier。 |

需要核心 profile 的消费者可以使用：

```toml
opi-ai = { version = "0.8.3", default-features = false }
```

`default-features = false` 不会移除内置 Provider 或网络依赖，只改变可选的
`opi-telemetry` tracing carrier；Provider 注册、catalog、请求生命周期和显式认证保持不变。
exporter、sampling、初始化和 shutdown 仍由宿主拥有。

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
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
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
`TextDelta` 等 partial event 之后会有一个终态 `Done`、`Error` 或 `Aborted`；
`ChatCall::result()` 等待相同的最终 `AssistantMessage` 或 `CallFailure`。
`Pending` 不是终态。

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
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
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
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
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
Deferred poll 不累计 token 或 cost。raw stream 和 prepared raw attempt 仍可在无隐式
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
`Models::fetch_deferred` 将后续 `ChatStream` 推进到一个 ready result 或类型化 failure；
`cancel_deferred` 是 best-effort 远端取消，与本地 fetch 取消独立。`poll_after_ms` 是 Provider
hint，不是隐式 sleep policy。

确定性的 Faux route 不代表 startup fetch、全局 deferred worker 或真实远端服务。
详见[Transport、协议与资源生命周期](docs/transport.zh.md)。

## 迁移

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

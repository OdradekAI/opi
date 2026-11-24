# opi-ai

[English](README.md)

`opi-ai` 是可独立引入的 Rust SDK，支持聊天、流式输出、工具调用、图像生成和结构化分类。应用可以用同一个 `Models` 切换供应商，自行保存对话历史，无需安装 opi CLI 或引入 agent crate。

[安装](#安装) · [快速开始](#快速开始) · [供应商与模型](#供应商与模型) · [流式输出](#流式输出) · [工具调用](#工具调用与续轮) · [图像与分类](#图像与分类)

## 安装

使用 Rust 1.97 或更高版本。若要在可能包含未发布变更的本地 `opi` 仓库上运行本指南，可在仓库的**同级目录**创建独立项目：

```sh
cargo new hello-opi
cd hello-opi
```

在 `Cargo.toml` 中加入以下依赖。如果仓库在其他位置，调整 `path`。

```toml
[dependencies]
opi-ai = { path = "../opi/crates/opi-ai" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

使用已发布的 `0.8.4` 时，可将路径依赖替换为下面这一行。已发布的 `0.8.3` 使用旧 API。

```toml
opi-ai = "0.8.4"
```

## 快速开始

在运行应用的环境中设置 `ANTHROPIC_API_KEY`，将下面的程序保存为 `src/main.rs`，运行 `cargo run` 即可打印模型回复。如果使用 OpenAI，把传给 `ask` 的供应商和模型改为 `"openai", "gpt-4o-mini"`，并设置 `OPENAI_API_KEY`。

```rust
use std::error::Error;
use std::sync::Arc;

use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage, assistant_content_text};
use opi_ai::provider::Request;
use opi_ai::providers::collection::register_builtin;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{AmbientAuthContext, ChatModelRecord, Models};

type AppResult<T> = Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    let mut models = Models::new();
    register_builtin(
        &mut models, Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()), &Default::default(),
    )?;

    let reply = ask(&models, "anthropic", "claude-sonnet-4-5", "Say hello.").await?;
    println!("{}", assistant_content_text(&reply.content));
    Ok(())
}

pub async fn ask(
    models: &Models, provider: &str, id: &str, prompt: &str,
) -> AppResult<AssistantMessage> {
    let info = models.get_model(provider, id).ok_or("unknown chat model")?;
    let record = ChatModelRecord::new(provider, info);
    let context = normalize_context(Context {
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: prompt.into() }],
            timestamp_ms: opi_ai::time::now_ms(),
        })],
        ..Default::default()
    })?;
    let request = Request {
        model: format!("{provider}:{id}"),
        context,
        max_tokens: Some(256),
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
        projection: Default::default(),
        transport: Default::default(),
    };
    Ok(models.complete_simple(record, request, Default::default())
        .await.map_err(|failure| failure.into_parts().1)?)
}
```

应用启动时注册一次内置供应商，后续请求复用 `Models`。模型查询可以离线完成，首次调用时才解析该供应商的凭据。需要自行注入认证环境或 HTTP 客户端时，可参考[完整调用示例](examples/quick_start.rs)。

## 供应商与模型

[内置目录](resources/catalog/builtin/manifest.json)覆盖 **42 个供应商、1,712 条按操作区分的模型记录**。选择模型时需要供应商、操作类型和模型 ID。`get_model` 查询聊天模型，`image_model` 查询图像生成模型，`classifier_model` 查询分类模型。

| 供应商 | 操作 | 模型 ID 示例 | 密钥环境变量 |
| --- | --- | --- | --- |
| `anthropic` | 聊天 | `claude-sonnet-4-5` | `ANTHROPIC_API_KEY` |
| `openai` | 聊天 | `gpt-4o-mini` | `OPENAI_API_KEY` |
| `google` | 聊天 | `gemini-2.5-flash` | `GEMINI_API_KEY` |
| `deepseek` | 聊天 | `deepseek-v4-pro` | `DEEPSEEK_API_KEY` |
| `openrouter` | 图像生成 | `black-forest-labs/flux.2-pro` | `OPENROUTER_API_KEY` |
| `typesafe` | 分类 | `jev-latest` | `TYPESAFE_API_KEY` |

要在没有密钥的机器上查看**全部**供应商、操作类型和模型 ID，可把这个函数加入快速开始示例，并将 `main` 中调用 `ask` 和打印回复的两行替换为 `list_models(&models);`。

```rust
pub fn list_models(models: &Models) {
    for provider in models.provider_ids() {
        for model in models.get_all_models(Some(provider)) {
            println!("{provider}  {}  {}", model.operation().as_str(), model.id());
        }
    }
}
```

用 `models.get_models(Some("anthropic"))` 可查看该供应商的聊天模型。每条记录的 `capabilities` 包含 `supports_images`、`context_window` 等字段。用 `models.get_available(Some("anthropic")).await?` 可按本机的凭据配置筛选，具体模型的访问权限由远端服务决定。Azure 还需要部署端点，Bedrock 使用 AWS 凭据，Vertex 支持 Google Cloud 认证。OAuth 登录、凭据存储、自定义端点和 Radius 目录刷新的用法见[供应商与认证参考](docs/provider-auth.zh.md)。

## 流式输出

在依赖中加入 `futures-util = "0.3"`。把 `ask` 末尾调用 `complete_simple` 的整个表达式替换为 `stream_reply(models, record, request).await`。如果只想逐段显示文本，可删去 `main` 最后的 `println!`。

```rust
pub async fn stream_reply(
    models: &Models, record: ChatModelRecord, request: Request,
) -> AppResult<AssistantMessage> {
    use std::io::{self, Write};
    use futures_util::StreamExt;
    use opi_ai::AssistantStreamEvent;

    let mut call = models.stream_simple(record, request, Default::default());
    while let Some(event) = call.next().await {
        match event.map_err(|failure| failure.into_parts().1)? {
            AssistantStreamEvent::TextDelta { delta, .. } => {
                print!("{delta}");
                io::stdout().flush()?;
            }
            AssistantStreamEvent::Done { message, .. } => return Ok(message),
            _ => {}
        }
    }
    Err("stream ended without a final message".into())
}
```

`TextDelta` 携带新增文本，`Done` 携带最终消息，包括 `stop_reason` 和 `usage`。调用失败时，`CallFailure::cause()` 返回具体错误，`message()` 可取回已生成的部分消息。需要取消时，在发起调用前克隆 `request.cancel`，然后对克隆的令牌调用 `cancel()`。另见[完整流式示例](examples/quick_start_stream.rs)。

## 工具调用与续轮

添加依赖 `serde_json = "1"`，用 JSON Schema 声明工具。在快速开始示例的 `Context` 中设置 `tools: vec![weather_tool()]`，并让模型查询东京的天气。

```rust
pub fn weather_tool() -> opi_ai::message::ToolDef {
    opi_ai::message::ToolDef {
        name: "get_weather".into(),
        description: "Look up the weather for one city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        }),
        constrained_sampling: None,
    }
}
```

1. 从成功返回的助手消息中取出 `AssistantContent::ToolCall`，把完整的助手消息保存在历史中。
2. 对照声明核验工具名称和最终参数，再由应用执行相应函数。流式传来的参数片段可用于展示进度。
3. 每个调用对应一条 `Message::ToolResult`，填回原来的 `tool_call_id` 和 `tool_name`。将更新后的历史传给 `normalize_context` 后再请求模型，直到得到不含工具调用的回复。

[完整工具循环](examples/tool_loop.rs) 展示了两轮请求以及天气结果如何回传。若直接把这三个链接指向的示例文件之一用作 `src/main.rs`，还需添加 `tokio-util = "0.7"`。保存对话或将历史交接给另一家供应商的用法见[上下文与回放](docs/utilities.zh.md)。

## 图像与分类

要让聊天模型看图，选择 `capabilities.supports_images` 为真的聊天模型，在用户文本旁加入 `InputContent::Image`，并填写媒体类型与图片来源。

**生成图像。** 将下面的函数加入快速开始示例，设置 `OPENROUTER_API_KEY`，将 `main` 中聊天和打印回复的两行替换为 `let blocks = draw(&models).await?;`。返回值按顺序保留文本和图片。`ImagesOutput::Image` 包含 MIME 类型与 base64 数据，应用可自行解码或保存。

```rust
pub async fn draw(models: &Models) -> AppResult<Vec<opi_ai::images::ImagesOutput>> {
    use opi_ai::images::{ImagesContext, ImagesInput};

    let model = models.image_model("openrouter", "black-forest-labs/flux.2-pro")?;
    let context = ImagesContext {
        input: vec![ImagesInput::Text { text: "A red kite above a beach".into() }],
    };
    let result = models.generate_images(model, context, Default::default()).await;
    if let Some(error) = result.error { return Err(error.into()); }
    Ok(result.output)
}
```

**对结构化数据分类。** 设置 `TYPESAFE_API_KEY`，添加依赖 `serde_json = "1"`，将 `main` 中聊天和打印回复的两行替换为 `classify_build(&models).await?;`。用声明问题时的 ID 读取答案。图像生成和分类都通过结果的 `error` 字段返回供应商错误。内置图像生成使用 OpenRouter，TypeSafe 提供分类服务。

```rust
pub async fn classify_build(models: &Models) -> AppResult<()> {
    use opi_ai::classifier::{ClassifierAnswer, ClassifierContext, ClassifierQuestion};

    let model = models.classifier_model("typesafe", "jev-latest")?;
    let question = ClassifierQuestion::Bool {
        instructions: "Did the tests pass?".into(),
        true_criteria: "Every test passed.".into(),
        false_criteria: "At least one test failed.".into(),
    };
    let context = ClassifierContext {
        state: serde_json::json!({ "passed": 48, "failed": 0 }),
        questions: [("passing".into(), question)].into_iter().collect(),
        images: None,
    };
    let result = models.classify(model, context, Default::default()).await;
    if let Some(error) = result.error { return Err(error.into()); }
    let Some(ClassifierAnswer::Bool { probability }) = result.answers.get("passing") else {
        return Err("expected a boolean answer for passing".into());
    };
    println!("Probability that the tests passed: {probability:.2}");
    Ok(())
}
```

## Cargo 功能选项

| 应用需要的能力 | Cargo 配置 |
| --- | --- |
| 全部内置供应商、目录、OAuth 和 tracing 支持 | 默认配置 |
| 内置供应商，关闭 tracing 适配 | `default-features = false, features = ["builtins"]` |
| 用 `Catalog::builtin()` 离线读取目录 | `default-features = false, features = ["catalog-bundled"]` |
| 核心类型、自定义注册和脚本化测试 | `default-features = false` |

## 专题参考

| 任务 | 文档 |
| --- | --- |
| 配置凭据、选择供应商或刷新模型目录 | [接入供应商](docs/provider-auth.zh.md) |
| 接入兼容端点，使用原生选项或图像输入 | [选择 API 与接入端点](docs/providers.zh.md) |
| 校验工具参数、回放对话或观察请求 | [校验工具参数与回放上下文](docs/utilities.zh.md) |
| 读取用量并显示费用估算 | [读取用量与估算费用](docs/cost-pricing.zh.md) |
| 配置代理或 TLS、取消调用或查询延迟响应 | [HTTP、流式调用与延迟响应](docs/transport.zh.md) |
| 使用模拟模型响应测试应用 | [离线测试 SDK 接入](docs/testing.zh.md) |

给 `Request::telemetry_context` 传入应用的遥测上下文即可观测调用。要查看当前 checkout 的 Rust 类型签名，可在 opi 仓库根目录运行 `cargo doc -p opi-ai --no-deps --open`。

## 许可证

[MIT](LICENSE) · [NOTICE](NOTICE)

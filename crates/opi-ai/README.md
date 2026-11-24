# opi-ai

[Simplified Chinese](README.zh.md)

Use `opi-ai` in your Rust application to stream chat, call tools, generate images and classify structured data. One `Models` collection lets you switch providers while keeping the conversation in your application. You can depend on the SDK independently of the opi CLI and agent crates.

[Installation](#installation) · [Quick start](#quick-start) · [Providers and models](#providers-and-models) · [Streaming](#streaming) · [Tools](#context-tools-and-continuation) · [Images and classification](#images-and-classification)

## Installation

Use Rust 1.97 or later. To run this guide against a checkout that may contain unreleased changes, create a binary project **next to** your checkout of `opi`:

```sh
cargo new hello-opi
cd hello-opi
```

Add these entries to `Cargo.toml`. Adjust the path if your checkout is elsewhere.

```toml
[dependencies]
opi-ai = { path = "../opi/crates/opi-ai" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For the published `0.8.4` crate, replace the path dependency with the following line. The published `0.8.3` crate has an earlier API.

```toml
opi-ai = "0.8.4"
```

## Quick start

Set `ANTHROPIC_API_KEY` in the environment of your application. Save this program as `src/main.rs` and run `cargo run`. It prints the assistant’s reply. For OpenAI, change the provider and model passed to `ask` to `"openai", "gpt-4o-mini"` and set `OPENAI_API_KEY`.

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

Register the built-ins once and reuse the collection. Model lookup works offline. The first call resolves the selected provider’s credentials. For an example with separately injectable authentication and HTTP transport, see [quick_start.rs](examples/quick_start.rs).

## Providers and models

The [bundled catalog](resources/catalog/builtin/manifest.json) contains **42 providers and 1,712 operation-specific model entries**. A model ID belongs to a provider and an operation. For example, `get_model` selects a chat model, `image_model` selects an image generator and `classifier_model` selects a classifier.

| Provider | Operation | Example model ID | API key |
| --- | --- | --- | --- |
| `anthropic` | Chat | `claude-sonnet-4-5` | `ANTHROPIC_API_KEY` |
| `openai` | Chat | `gpt-4o-mini` | `OPENAI_API_KEY` |
| `google` | Chat | `gemini-2.5-flash` | `GEMINI_API_KEY` |
| `deepseek` | Chat | `deepseek-v4-pro` | `DEEPSEEK_API_KEY` |
| `openrouter` | Image generation | `black-forest-labs/flux.2-pro` | `OPENROUTER_API_KEY` |
| `typesafe` | Classification | `jev-latest` | `TYPESAFE_API_KEY` |

To list **every** registered provider, operation and ID without credentials, add this function to the quick start. Replace the `ask` and `println!` lines in `main` with `list_models(&models);`.

```rust
pub fn list_models(models: &Models) {
    for provider in models.provider_ids() {
        for model in models.get_all_models(Some(provider)) {
            println!("{provider}  {}  {}", model.operation().as_str(), model.id());
        }
    }
}
```

Use `models.get_models(Some("anthropic"))` to browse chat capabilities such as `supports_images` and `context_window` on each model’s `capabilities`. To filter by configured credentials, use `models.get_available(Some("anthropic")).await?`. The remote service determines account access to each model. Azure also needs a deployment endpoint, Bedrock uses AWS credentials, and Vertex supports Google Cloud authentication. For OAuth login, credential stores, custom endpoints and Radius catalog refresh, see the [provider and auth reference](docs/provider-auth.md).

## Streaming

Add `futures-util = "0.3"` to your dependencies. In `ask`, replace the final `complete_simple` expression with `stream_reply(models, record, request).await`. Remove the final `println!` from `main` if you only want the streamed text.

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

`TextDelta` carries new text. `Done` carries the final message, including `stop_reason` and `usage`. A `CallFailure` gives you the typed cause via `cause()` and the partial message via `message()`. To cancel a request, clone `request.cancel` before dispatch and call `cancel()` on the clone. See the [complete streaming example](examples/quick_start_stream.rs).

## Context, tools, and continuation

Add `serde_json = "1"` and declare a tool with JSON Schema. In the quick start’s `Context`, set `tools: vec![weather_tool()]` and ask for the weather in Tokyo.

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

1. Read the successful assistant message and collect its `AssistantContent::ToolCall` blocks. Keep the complete assistant message in history.
2. Check each tool name and its final arguments against your declaration, then run the handler in your application. Streamed argument fragments are for previews.
3. Append a `Message::ToolResult` for each call using the original `tool_call_id` and `tool_name`. Normalize the updated history and send it to the model. Stop when the assistant returns a reply without a tool call.

The [runnable tool loop](examples/tool_loop.rs) demonstrates both requests and the weather result. To use any of the three linked example files as your own `src/main.rs`, also add `tokio-util = "0.7"`. For saving transcripts and handing a conversation to another provider, see [context and replay](docs/utilities.md).

## Images and classification

For vision chat, include `InputContent::Image` with its media type and source alongside the user’s text. Select a chat model advertising `capabilities.supports_images`.

**Generate an image.** Add the following function to the quick start and set `OPENROUTER_API_KEY`. In `main`, replace the chat call and its `println!` with `let blocks = draw(&models).await?;`. The result holds ordered text and image blocks. An `ImagesOutput::Image` contains a MIME type and base64 data for your application to decode or save.

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

**Classify structured data.** With `TYPESAFE_API_KEY` and `serde_json = "1"`, replace the chat call and its `println!` in `main` with `classify_build(&models).await?;`. The answer is keyed by your question ID. Both operations return provider failures in the result’s `error` field. Built-in image generation runs through OpenRouter. TypeSafe serves classifiers.

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

## Feature profiles

| Your application needs | Cargo features |
| --- | --- |
| All built-in providers and catalog, OAuth and tracing support | Default |
| Built-in providers without the tracing carrier | `default-features = false, features = ["builtins"]` |
| Offline catalog lookup with `Catalog::builtin()` | `default-features = false, features = ["catalog-bundled"]` |
| Core types, custom registration and scripted tests | `default-features = false` |

## Reference

| Task | Where to look |
| --- | --- |
| Set credentials, choose providers, or refresh a catalog | [Connect a provider](docs/provider-auth.md) |
| Connect a compatible endpoint or use native options and image input | [Choose an API or connect an endpoint](docs/providers.md) |
| Validate tool arguments, replay a conversation, or inspect requests | [Validate tools and replay context](docs/utilities.md) |
| Interpret reported usage and display a cost estimate | [Read usage and estimate cost](docs/cost-pricing.md) |
| Configure proxies or TLS, cancel a call, or fetch a deferred response | [HTTP, streaming, and deferred responses](docs/transport.md) |
| Test an integration with scripted model responses | [Test an SDK integration offline](docs/testing.md) |

Set `Request::telemetry_context` to observe a call with your host’s telemetry recorder. To browse type signatures for this checkout, run `cargo doc -p opi-ai --no-deps --open` from the opi repository root.

## License

[MIT](LICENSE) · [NOTICE](NOTICE)

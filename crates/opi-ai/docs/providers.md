# Choose an API or connect an endpoint

[简体中文](providers.zh.md) · [SDK quick start](../README.md#quick-start)

Use the selected model's wire API. A provider ID can serve several APIs.
For example, GitHub Copilot routes by model across Chat Completions, Responses
and Anthropic Messages. To inspect a chat model, read `ModelInfo::wire_api`,
`capabilities`, `base_url` and `compat`. Start with [provider setup](provider-auth.md).

## Connect a compatible endpoint

Reuse the Chat Completions adapter when your server accepts OpenAI-style
chat requests. For example, call `local_models("http://127.0.0.1:11434/v1", id, key)?`,
where `id` matches a model served by your endpoint. Copy the context and
output limits from your server configuration. Use the server's API key or an
application-chosen placeholder on a keyless local server. Then call the
[quick start's `ask`](../README.md#quick-start) with the returned collection,
provider `"local"` and that model ID.

```rust
use std::sync::Arc;

use opi_ai::http::HttpClient;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ModelCapabilities, ModelInfo, Models,
    RegistrationError, StaticAuthResolver, WireApi,
};

pub fn local_models(
    api_base: &str,
    model_id: &str,
    api_key: String,
) -> Result<Models, RegistrationError> {
    let model = ModelInfo::new(
        model_id,
        model_id,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(32_768, 4_096).with_streaming(true),
    )
    .with_base_url(api_base);
    let provider = OpenAiChatProvider::for_route(
        Some(api_base.into()),
        "local".into(),
        Default::default(),
        vec![model],
        Arc::new(HttpClient::new()),
    );
    let resolver = StaticAuthResolver::new(AuthScheme::Bearer, api_key.into());

    let mut models = Models::new();
    models.register_route(
        Box::new(provider),
        Arc::new(resolver),
        AuthProvenanceSource::Static,
        Default::default(),
    )?;
    Ok(models)
}
```

An explicit model API base ending in `/v1` gets the `/chat/completions`
suffix. If your server uses a different path or field name, set the record's
`WireCompat::OpenAiCompletions` through `ModelInfo::with_compat`. Inspect
`chat_completions_path`, `max_tokens_field`, `usage_in_stream` and
`supports_finish_reason` for common compatibility issues. Use
`Request::extra_headers` for app headers and configure authentication
through the resolver.

## Use reasoning and native options

Set `Request::thinking` with `enabled: true` and the desired `ThinkingLevel`.
Use the model's `thinking_level_map` to present supported levels. The simple
entry uses the model's level mapping and can select an adjacent supported
level. For example, a Gemini or Vertex model that requires thinking uses its
lowest supported discrete level when thinking is disabled.

Simple calls resolve an omitted `Request::max_tokens` from the model and
reduce the ceiling to fit the estimated context remaining. Pi Messages
routes such as Radius keep their own limits, as do native and direct calls. For Chat Completions and Responses
adapters that support `Request::sampling_params`, per-key overrides apply
in order from model defaults to effective thinking level to request
parameters. An empty map keeps inherited keys.

To request prompt caching, set `Request::cache_retention` to `Short` or
`Long` on models that support the desired retention. Use `Disabled` to
suppress cache and session-affinity hints, or `None` to keep provider defaults.
For routes that use session affinity, reuse the same `Request::session_id`
across calls in the session. [Usage and cache pricing](cost-pricing.md)

For API-specific options, register a `ChatApi` with `Models::register_chat_api`.
Keep the complete `ChatModelRecord` for that route and create options with the
returned handle's `options(value)`. Pass them to `Models::stream` or
`Models::complete`. The request must match the registered API and options type.
Use `Models::register_route` with a provider and a resolver when you want that
provider's models to appear in catalog lookup. An app can also register its
own image, classifier or deferred adapter.

Enable `builtins` for the bundled collection, or select a `provider-*` feature
when wiring a single adapter yourself. [Cargo feature declarations](../Cargo.toml)

## Select the operation

| Task | Entry point and input |
| --- | --- |
| Chat with an image | Select a chat model with `capabilities.supports_images` and add `InputContent::Image` to the user message. Bedrock Converse needs bytes or base64 input. |
| Generate an image | Select with `Models::image_model` and call `generate_images`. Built-in image generation uses OpenRouter. Read image bytes from the base64 data in `ImagesOutput::Image`. |
| Classify data | Select with `Models::classifier_model` and call `classify` with state and typed questions. Read each `ClassifierAnswer` from `ClassifierResult::answers` by question ID. |
| Retrieve a deferred response | Keep the handle supplied by a registered deferred backend. [Fetch and cancel deferred responses](transport.md#deferred-transport-semantics) |

Check the `error` field of both `ImagesResult` and `ClassifierResult`.
Chat, image generation and classification each have a distinct model
catalog. Examples are in the [SDK README](../README.md#images-and-classification).

For classification with images, start by checking the record's declared
input capabilities and limits. Add image data to `ClassifierContext::images`.
TypeSafe System One accepts PNG or JPEG images. Cloudflare Workers AI Clef
accepts up to four PNG, JPEG or WebP images. OpenRouter System One uses the
selected model's modalities. OpenAI Decisions accepts at most 128 images and
requires an API key.

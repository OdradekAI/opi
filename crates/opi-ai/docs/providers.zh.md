# 选择 API 与接入端点

[English](providers.md) · [SDK 快速开始](../README.zh.md#快速开始)

以所选模型的 API 协议为准。同一个供应商 ID 可以使用多个协议，例如 GitHub Copilot
按模型选择 Chat Completions、Responses 或 Anthropic Messages。查看聊天模型的
`ModelInfo::wire_api`、`capabilities`、`base_url` 和 `compat` 可了解它使用的协议
和能力。先完成[供应商配置](provider-auth.zh.md)。

## 接入兼容端点

如果服务接受 OpenAI 风格的聊天请求，可以复用 Chat Completions adapter。
例如传入 `local_models("http://127.0.0.1:11434/v1", id, key)?`，其中 `id`
是服务实际提供的模型 ID。上下文和输出上限以服务配置为准。传入服务的 API key，
无需密钥的本地服务可以使用占位值。将返回的集合、供应商 `"local"` 和模型 ID
传给[快速开始中的 `ask`](../README.zh.md#快速开始)。

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

显式给模型设置以 `/v1` 结尾的 API base 时，SDK 会追加 `/chat/completions`。
如果服务使用不同的路径或字段名，用 `ModelInfo::with_compat` 设置
`WireCompat::OpenAiCompletions`。常用排查字段包括 `chat_completions_path`、
`max_tokens_field`、`usage_in_stream` 和 `supports_finish_reason`。
应用自定义请求头通过 `Request::extra_headers` 传入，认证由 resolver 配置。

## 使用推理和原生选项

在 `Request::thinking` 中设置 `enabled: true` 和所需的 `ThinkingLevel`。
可用级别由模型的 `thinking_level_map` 提供。简单调用按模型映射选择级别，
必要时调整到相邻的可用级别。例如某些 Gemini 或 Vertex 模型必须开启推理，
关闭推理的请求会使用模型支持的最低级别。

简单调用在省略 `Request::max_tokens` 时使用模型的输出上限，再按估算的剩余上下文
收紧上限。Radius 等 Pi Messages 路由、原生调用和直接调用使用对应 adapter 的预算规则。支持
`Request::sampling_params` 的 Chat Completions 和 Responses adapter 逐键合并参数，
依次应用模型默认值、实际推理级别的覆盖值和请求参数。空 map 会保留继承的键。

模型支持相应的提示缓存时，可将 `Request::cache_retention` 设为 `Short` 或 `Long`。
`Disabled` 禁止发送缓存和会话亲和性提示，`None` 沿用供应商默认行为。使用会话
亲和性的路由应在同一会话的多次调用中复用 `Request::session_id`。[用量与缓存计价](cost-pricing.zh.md)

需要 API 专属选项时，用 `Models::register_chat_api` 注册 `ChatApi`，保留该路由的
完整 `ChatModelRecord`，用注册返回的 handle 调用 `options(value)`，然后传给
`Models::stream` 或 `Models::complete`。请求的 API 和选项类型须与注册一致。
如果希望通过目录查到供应商的模型，用 `Models::register_route` 注册供应商及其 resolver。
应用也可以注册自己的图像生成、分类或延迟响应 adapter。

使用内置集合时启用 `builtins`，自行接入单个 adapter 时可选择对应的 `provider-*`
feature。[Cargo 功能选项声明](../Cargo.toml)

## 选择操作类型

| 任务 | 入口与输入 |
| --- | --- |
| 让聊天模型读图 | 选择 `capabilities.supports_images` 为真的聊天模型，将 `InputContent::Image` 放入用户消息。Bedrock Converse 需要字节或 base64 图像。 |
| 生成图像 | 用 `Models::image_model` 选模型，再调用 `generate_images`。内置图像生成使用 OpenRouter。`ImagesOutput::Image` 的数据以 base64 返回，解码可得到图像字节。 |
| 分类 | 用 `Models::classifier_model` 选模型，将状态和类型化问题传给 `classify`。按问题 ID 从 `ClassifierResult::answers` 读取 `ClassifierAnswer`。 |
| 读取延迟响应 | 保存已注册的延迟响应后端返回的 handle。[查询与取消延迟响应](transport.zh.md#deferred-transport-语义) |

图像生成和分类分别检查 `ImagesResult` 和 `ClassifierResult` 的 `error` 字段。
聊天、图像生成和分类各有自己的模型目录。[SDK README 示例](../README.zh.md#图像与分类)

分类时如需携带图像，先查模型记录声明的输入能力和限制，在
`ClassifierContext::images` 传入图像。TypeSafe System One 支持 PNG 或 JPEG。
Cloudflare Workers AI Clef 支持最多四张 PNG、JPEG 或 WebP。OpenRouter System One
的格式取决于所选模型。OpenAI Decisions 最多接受 128 张图像，需要 API key。

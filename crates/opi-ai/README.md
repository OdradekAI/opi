# opi-ai

[![Crates.io](https://img.shields.io/crates/v/opi-ai.svg)](https://crates.io/crates/opi-ai)
[![Docs.rs](https://docs.rs/opi-ai/badge.svg)](https://docs.rs/opi-ai)

> Provider-neutral LLM API used by [opi](https://github.com/OdradekAI/opi).

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

## Status

Current crate version: `0.6.4`, inherited from the workspace package version.

`opi-ai` owns the model/provider layer: request and message types, streaming
events, model metadata, provider registration, HTTP/proxy plumbing, retry
helpers, image content, usage accumulation, best-effort cost helpers, and the
provider-side error taxonomy consumed by `opi-agent` diagnostics. It does not
implement an agent loop, sessions, package loading, or built-in coding tools;
those live in `opi-agent` and `opi-coding-agent`.

`opi-ai` also exposes an unstable-0.x models/auth seam (Phase 10): the
`provider_collection` module (`ProviderCollection`) wraps `ProviderRegistry`
with a provider-side auth contract (`AuthDescriptor` / `AuthStatus`),
OpenAI-compatible compatibility metadata, and stream/complete dispatch. OAuth
and subscription auth are explicit non-goals.

The workspace package version is `0.6.4`; the checkout may also contain
unreleased Phase 13 session-integration changes. Provider-correctness work from
Phase 12 tightens existing provider families rather than adding provider
breadth: request/stream/error fixtures cover all built-in families,
`ProviderError::category` exposes the nine documented classes,
OpenAI-compatible profiles have tested `CompatConfig` and
`ModelCompatOverride` behavior, cache tokens and provider response IDs
round-trip where available, and missing usage remains explicit `unknown usage`
instead of known-zero usage.

The Phase 11 tool-result fix remains part of the provider contract:
`ToolResultMessage::is_error` stays visible to provider wire converters.
Providers with native error fields use them; OpenAI-family wire formats that
lack one use a deterministic text marker. This is a correctness fix for
existing providers, not a provider-breadth phase.

## Providers

| Module | Provider id | Backend |
|--------|-------------|---------|
| `anthropic` | `anthropic` | Anthropic Messages streaming |
| `openai_chat` | `openai` | OpenAI Chat Completions streaming |
| `openai_responses` | `openai-responses` | OpenAI Responses streaming |
| `openrouter` | `openrouter` | OpenAI-compatible OpenRouter profile |
| `mistral` | `mistral` | OpenAI-compatible Mistral profile |
| `gemini` | `gemini` | Gemini `streamGenerateContent?alt=sse` |
| `bedrock` | `bedrock` | AWS Bedrock Converse streaming with SigV4 signing |
| `azure_openai` | `azure` | Azure OpenAI deployment-specific Chat Completions |
| `vertex` | `vertex` | Google Vertex AI Gemini streaming |

Built-in model lists are finite and intended for capability checks and model
listing. Site-specific models, fine-tuned models, and deployments should be
added through registry overrides or configured OpenAI-compatible profiles.

## Core API

| Item | Purpose |
|------|---------|
| `Provider` | Backend trait with `id`, `models`, and `stream(Request)`. |
| `Request` | Provider request: model, messages, tools, token limits, thinking config, metadata, cancellation. |
| `Message` | Provider-facing user, assistant, and tool-result messages. |
| `InputContent` / `OutputContent` | Text and image content blocks. |
| `ToolResultMessage` | Provider-facing tool-result message: content, optional details, `is_error`, `truncated`, and timestamp metadata. |
| `AssistantStreamEvent` | Provider-neutral stream events for start, text, thinking, tool calls, done, and error. |
| `ModelInfo` | Model metadata: context window, output limit, image, streaming, and thinking support. |
| `ProviderError` / `ProviderErrorCategory` | Provider failure taxonomy: auth, config, request, network, rate_limit, provider, stream, capability, and cancelled (timeouts classify as network). |
| `ProviderRegistry` | Resolves `provider:model`, registers custom providers, and layers model overrides. |
| `ProviderCollection` / `AuthDescriptor` / `AuthStatus` | Unstable-0.x models/auth seam above `ProviderRegistry`: provider+model lookup, redacted auth resolution, OpenAI-compatible compat metadata, and stream/complete dispatch. No OAuth/subscription auth. |
| `ApiKind` | Crate-root enum tagging the backend family (`Anthropic`, `OpenAi`, `Google`, `Mistral`) carried on assistant messages. |
| `HttpClient` | Shared `reqwest` client with pooling and explicit/env proxy support. |
| `retry` | Retry config, exponential backoff, and `Retry-After` parsing. |
| `Usage` / `CumulativeUsage` | Token accumulation and cost helpers. |
| `test_support::MockProvider` | Deterministic mock provider for downstream tests. |

## Image Support

Image input is represented by `InputContent::Image`. Supported media types are
PNG, JPEG, GIF, and WebP. Providers serialize images to their native wire
format when the selected model supports images.

`validate_request_capabilities` rejects known text-only models before a network
call. Bedrock supports byte/base64 image sources through Converse, but URL
images are rejected locally because Bedrock Converse expects image bytes.

## Tool Result Error Semantics

Failed tool results remain distinguishable on every supported provider wire:

| Provider family | Failure signal |
|-----------------|----------------|
| Anthropic | Native `is_error: true` on the `tool_result` content block. |
| AWS Bedrock | Native `toolResult.status = "error"`. |
| Gemini / Vertex | `error: true` inside the `functionResponse.response` object. |
| OpenAI Chat / Azure / OpenRouter / Mistral | `[tool_error] ` prefix on the tool-output string. |
| OpenAI Responses | `[tool_error] ` prefix on the `function_call_output.output` string. |

Successful (`is_error = false`) tool-result bodies keep their pre-fix wire
shape. `ToolResultMessage::details` is for opi runtime/UI/session boundaries;
provider request bodies use the LLM-visible content plus the provider-specific
failure signal.

## Provider Behavior Matrix

Per-family capability and metadata behavior. "yes" means the adapter implements
the behavior on the wire; "inherited" means it inherits the shared OpenAI Chat
or Gemini path; "—" means not produced.

| Family | Thinking/reasoning | Image input | Cache tokens (usage) | Response ID |
|--------|--------------------|-------------|----------------------|-------------|
| `anthropic` | thinking blocks | PNG/JPEG/GIF/WebP | `cache_read` / `cache_creation` | `message.id` |
| `openai_chat` | `reasoning_effort` via compat profile | PNG/JPEG/GIF/WebP | `cached_tokens` | `chatcmpl-*` (`id`) |
| `openai_responses` | `reasoning_effort` (`ResponsesConfig`) | PNG/JPEG/GIF/WebP | `cached_tokens` | `resp_*` (`id`) |
| `openrouter` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `mistral` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `gemini` | thinking | PNG/JPEG/GIF/WebP | `cached_content` | — |
| `bedrock` | `supports_thinking` models advertise thinking, but Converse-stream `reasoningContent` blocks are not parsed as thinking (parser limitation) | byte/base64 only; URL images rejected | `cache_read` / `cache_write` | — |
| `azure_openai` | inherited | PNG/JPEG/GIF/WebP | inherited | `chatcmpl-*` |
| `vertex` | inherited | PNG/JPEG/GIF/WebP | `cached_content` | — |

Captured response IDs (`message.id`, `chatcmpl-*`, `resp_*`) round-trip into
`AssistantMessage::response_id`; families without a provider response ID leave
it `None`. Text-only models reject image input through
`validate_request_capabilities` before a network call.

## OpenAI-Compatible Profiles

Compatible OpenAI-style services stay config-driven unless a material
wire/auth/capability difference requires a first-class adapter. This is the
preferred path for provider breadth; adding a new first-class provider module
requires a task-graph update and is a Phase 12 non-goal by default.

`CompatConfig` carries the per-profile compatibility flags:

| Flag | Effect |
|------|--------|
| `system_role_override` | Render the system prompt as `developer` (or another role) instead of `system`. |
| `max_tokens_field` | Request field name for the output cap (`max_tokens` vs `max_completion_tokens`). |
| `tool_result_name_field` | Echo the tool name on the tool-result message. |
| `usage_in_stream` | Request `stream_options.include_usage` and preserve usage updates from any streaming chunk. |
| `strict_tool_schema` | Emit strict JSON-schema tool definitions. |
| `reasoning_effort` | Send a reasoning-effort hint for models that support it. |
| `cache_key` | Send the provider's prompt-cache key (cache-affinity hint). |
| `require_assistant_after_tool_result` | Metadata-only compatibility marker for legacy endpoints; opi does not synthesize or enforce the extra assistant turn in the shared adapter. |

`ModelCompatOverride` layers model-level overrides for `system_role_override`
and `max_tokens_field` on top of the profile defaults (model wins over
provider). Static per-profile request headers (`extra_headers`, used for
session affinity / routing) are a separate profile config field threaded
through `OpenAiChatProvider` construction, not a `CompatConfig` flag.

OpenAI Responses native semantics (`ResponsesConfig`): `store`,
`reasoning_effort`, and `strict_tools` are implemented.
`previous_response_id` is intentionally absent — Responses requests are built
as Chat-Completions analogues, so server-side response chaining is not wired.

## Cache, Response IDs, and Session Affinity

Usage-side cache tokens are normalized where the provider supplies them
(Anthropic `cache_read`/`cache_creation`, OpenAI Chat/Responses
`cached_tokens`, Gemini `cached_content`, Bedrock `cache_read`/`cache_write`).
Request-side prompt-caching breakpoints (for example Anthropic
`cache_control`) are not emitted by opi; the `cache_key` profile flag is the
available cache-affinity hint.

Provider response IDs are captured and round-tripped into
`AssistantMessage::response_id` (Anthropic `message.id`, OpenAI Chat
`chatcmpl-*`, OpenAI Responses `resp_*`); OpenAI Chat captures the ID from any
chunk carrying `id`, not only role chunks, and other families leave it `None`.

Session affinity is intentionally limited: `previous_response_id` is deferred
(see OpenAI-Compatible Profiles), and compatible profiles may carry static
`extra_headers` for routing/session pinning. There is no server-side session
chain.

## Proxy

`HttpClient` carries shared `reqwest` pooling with explicit per-provider proxy
config (provider profile `proxy.url` and `proxy.no_proxy`) and environment
fallback (`HTTPS_PROXY` > `HTTP_PROXY` > `NO_PROXY`). Proxy credentials in a
proxy URL are redacted before any diagnostic display. Proxy transport semantics
(retry-through-proxy, cancellation) are owned by the Phase 12 retry/proxy
coverage.

## Best-Effort Cost

Cost mapping is best-effort. Incorrect confidence is worse than explicit
unknown values: when provider usage is absent, `Usage::unknown()` and
`CumulativeUsage` keep the turn explicitly unknown instead of treating it as
known-zero usage. Session-facing cost summaries should therefore be omitted
when any turn has unknown usage or when pricing is absent. Cost never blocks a
successful stream.

## Phase 12 Non-Goals

Phase 12 is a provider-*correctness* phase, not a breadth phase. The following
are explicit non-goals and must not appear as current core behavior:

- OAuth login flows.
- Anthropic subscription auth.
- OpenAI Codex subscription auth.
- GitHub Copilot auth.
- A broad new first-class provider list (compatible providers stay
  config-driven profiles).
- Image generation (image support is input-only).
- Browser usage.
- Provider streaming-adapter protocol for packages.
- Paid live provider calls in default tests (live tests stay `#[ignore]`-gated).
- Copying pi's provider-specific config file format.

## Phase 13 Session Integration

Phase 13 session work relies on provider-correct usage, model, thinking, cache,
response ID, cancellation, and error data through shared `opi-ai` types. It does
not add provider families or require callers to depend on provider-specific
internals.

## Minimal Example

```rust
use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{Provider, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let provider = AnthropicProvider::new(
    std::env::var("ANTHROPIC_API_KEY")?,
    None,
);

let request = Request {
    model: "claude-sonnet-4-5-20250514".into(),
    system: Some("You are concise.".into()),
    messages: vec![Message::User(UserMessage {
        content: vec![InputContent::Text { text: "Hi".into() }],
        timestamp_ms: 0,
    })],
    tools: vec![],
    max_tokens: Some(1024),
    temperature: None,
    thinking: ThinkingConfig::default(),
    stop_sequences: vec![],
    metadata: None,
    cancel: CancellationToken::new(),
};

let mut stream = provider.stream(request);
while let Some(event) = stream.next().await {
    println!("{:?}", event?);
}
# Ok(()) }
```

## Modules

`provider`, `message`, `stream`, `registry`, `provider_collection`, `http`,
`retry`, `model`, `anthropic`, `openai_chat`, `openai_responses`, `openrouter`,
`mistral`, `gemini`, `bedrock`, `azure_openai`, `vertex`, `config`, and
`test_support`.

## License

MIT. See the workspace [LICENSE](../../LICENSE).

# opi-ai

[![Crates.io](https://img.shields.io/crates/v/opi-ai.svg)](https://crates.io/crates/opi-ai)
[![Docs.rs](https://docs.rs/opi-ai/badge.svg)](https://docs.rs/opi-ai)

> Provider-neutral LLM API used by [opi](https://github.com/OdradekAI/opi).

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

Provider-neutral LLM API for Rust: one `Request` and streaming-event model
shared across built-in provider families, a checked multi-wire provider, and
config-driven profiles.

```sh
cargo add opi-ai
```

## Status

Current crate version: `0.8.0`, inherited from the workspace package version.

`opi-ai` owns the model/provider layer: request and message types, streaming
events, model metadata, provider registration, HTTP/proxy plumbing, retry
helpers, image content, usage accumulation, best-effort cost helpers, and the
provider-side error taxonomy consumed by `opi-agent` diagnostics. It does not
implement an agent loop, sessions, package loading, or built-in coding tools;
those live in `opi-agent` and `opi-coding-agent`.

`opi-ai` also exposes an unstable-0.x models/auth seam: the
`provider_collection` module (`ProviderCollection`) wraps `ProviderRegistry`
with provider-side auth contracts, OpenAI-compatible compatibility metadata,
and stream/complete dispatch. IO-free credential and OAuth traits live here;
keychain, environment, login-presenter, and refresh implementations remain in
`opi-coding-agent`.

## Providers

| Module | Provider id | Backend |
|--------|-------------|---------|
| `anthropic` | `anthropic` | Anthropic Messages streaming |
| `openai_chat` | `openai` | OpenAI Chat Completions streaming |
| `openai_responses` | `openai-responses` | OpenAI Responses streaming |
| `openai_codex_responses` | `openai-codex` | Subscription-specific OpenAI Codex Responses streaming |
| `api_mapped` | caller-defined | One provider identity/catalog routed across checked concrete wires |
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
| `Provider` | Backend trait with `id`, `models`, and `stream_prepared` (the sole dispatch entry, reached through `ProviderCollection::prepare_call` with already-resolved auth). |
| `Request` / `CacheRetention` | Provider request: model, messages, tools, token limits, thinking config, metadata, cancellation, timeout, extra headers, cache retention, and session affinity. |
| `Message` | Provider-facing user, assistant, and tool-result messages. |
| `InputContent` / `OutputContent` | Text and image content blocks. |
| `ToolResultMessage` | Provider-facing tool-result message: content, optional details, `is_error`, `truncated`, and timestamp metadata. |
| `AssistantStreamEvent` | Provider-neutral stream events for start, text, thinking, tool calls, done, and error. |
| `WireApi` / `ModelInfo` / `ModelCapabilities` | Exact request-wire identity plus model capability, thinking, compatibility, and pricing metadata. |
| `ApiMappedProvider` | One public provider identity/catalog dispatched through a validated `WireApi -> Provider` route map. |
| `ProviderError` / `ProviderErrorCategory` | Provider failure taxonomy: auth, config, request, network, rate_limit, provider, stream, capability, and cancelled (timeouts classify as network). |
| `ProviderRegistry` | Resolves `provider:model`, registers custom providers, and layers model overrides. |
| `ProviderCollection` / `AuthDescriptor` / `AuthStatus` | Unstable-0.x models/auth seam above `ProviderRegistry`: provider+model lookup, redacted auth state, OpenAI-compatible compat metadata, dispatch, and atomic dynamic-catalog refresh. |
| `CredentialStore` / `Credential` / `CredentialSource` | IO-free, object-safe credential persistence and redacted three-state probe contracts. |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | Flow-independent boxed-future OAuth contracts; concrete flows live in `opi-coding-agent`. |
| `AuthResolver` / `ResolvedAuth` | Collection-owned per-call auth resolution contract; one prepared result is frozen before an attempt starts. |
| `ApiKind` | Crate-root enum tagging the backend family (`Anthropic`, `OpenAi`, `Google`, `Mistral`) carried on assistant messages. |
| `HttpClient` | Shared `reqwest` client with pooling and explicit/env proxy support. |
| `retry` | Retry config, exponential backoff, and `Retry-After` parsing. |
| `Usage` / `CumulativeUsage` | Token accumulation and cost helpers. |
| `test_support::MockProvider` | Deterministic mock provider for downstream tests. |

## Credentials, Request Enrichment, and Refresh

`CredentialStore` keeps backend errors distinct from missing entries, while
`CredentialSource` reports present, absent, or backend-unavailable state
without exposing a secret. `AuthDescriptor::StoreCredential` carries only a
key and display source. `OAuthProvider`, `LoginPresenter`, and `AuthResolver`
are object-safe boxed-future seams. `opi-coding-agent` supplies the OS-keychain
store and the approved Anthropic, GitHub Copilot, and OpenAI Codex login flows;
`opi-ai` performs no keychain, environment, or presenter IO.

`WireApi` gives every `ModelInfo` one exact request wire, while public
`ApiMappedProvider` exposes one provider identity and catalog and validates
its `WireApi -> Provider` routes before dispatch. One mapped provider shares
one lazy `AuthResolver` across all routes; provider/model metadata chooses the
route before network IO. Unknown models, missing routes, and wire/compatibility
mismatches are typed non-retryable failures.

GitHub Copilot routes one static catalog through Anthropic Messages, OpenAI
Completions/Chat, and OpenAI Responses; OpenAI Codex uses its dedicated
Responses provider rather than standard Responses compatibility flags. The
collection resolves `AuthResolver` once in `prepare_call`, before any attempt,
and every `start_attempt` reuses that frozen authentication. Missing and revoked
credentials surface as explicit, non-retryable
`ProviderError::CredentialNeeded` and `ProviderError::CredentialRevoked`
variants. Per-call credentials remain out of scope: `extra_headers` rejects
provider-managed auth headers.

`ProviderError::AccountIdMissing { provider_id }` is the separate,
non-retryable case where a credential exists but the selected wire requires
account identity absent from it. Product layers remediate it with an explicit
`/login <provider>`; it is not credential revocation.

Capable built-in Anthropic models emit `cache_control` on the system prompt,
final user text, final assistant text, and final tool definition. Long
retention adds `ttl: "1h"`; short/default ephemeral retention omits the TTL,
while explicit disablement and unknown/custom models emit no markers.

`Request` adds `timeout`, `extra_headers`, `CacheRetention`, and `session_id`.
The first three are public request-to-wire substrate; only `session_id` has a
production producer in the coding harness. Session-affinity behavior
differs between direct Responses and custom/proxy routes as detailed below,
while Anthropic uses model-gated cache markers. `ModelInfo` contains the
existing nested `ModelCapabilities`; unknown/custom models default cache
support off.

`Usage::cache_write_1h_tokens` and `Usage::reasoning_tokens` are optional
`u64` child subsets, preserving absent versus explicitly reported zero.
`CostBreakdown` has four lines: `input_cost`, `output_cost`,
`cache_read_cost`, and `cache_write_cost`. The weighted one-hour write subset
is folded into `cache_write_cost`, reasoning remains in `output_cost`, and
cost and total-token calculation count the parent buckets once.
`Provider::refresh_models` and
`ProviderCollection::refresh` implement deterministic atomic catalog
replacement, but remain substrate-only with no production trigger.

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
| `openai_chat` | `request.thinking` + selected `ModelInfo::thinking_level_map` | PNG/JPEG/GIF/WebP | `cached_tokens` | `chatcmpl-*` (`id`) |
| `openai_responses` | `request.thinking` + selected `ModelInfo::thinking_level_map` | PNG/JPEG/GIF/WebP | `cached_tokens` | `resp_*` (`id`) |
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
is a non-default step reserved for those material differences.

`CompatConfig` carries the per-profile compatibility flags:

| Flag | Effect |
|------|--------|
| `system_role_override` | Render the system prompt as `developer` (or another role) instead of `system`. |
| `max_tokens_field` | Request field name for the output cap (`max_tokens` vs `max_completion_tokens`). |
| `tool_result_name_field` | Echo the tool name on the tool-result message. |
| `usage_in_stream` | Request `stream_options.include_usage` and preserve usage updates from any streaming chunk. |
| `strict_tool_schema` | Emit strict JSON-schema tool definitions. |
| `reasoning_effort` | Legacy compatibility/profile metadata; wire reasoning comes from `request.thinking` and the selected `ModelInfo::thinking_level_map`. |
| `cache_key` | Send the provider's prompt-cache key (cache-affinity hint). |
| `send_session_affinity_headers` | Map a request `session_id` to the compatible `session_id`, `x-client-request-id`, and `x-session-affinity` headers; disabled by default. |
| `require_assistant_after_tool_result` | Metadata-only compatibility marker for legacy endpoints; opi does not synthesize or enforce the extra assistant turn in the shared adapter. |
| `chat_completions_path` | Chat completions endpoint path relative to `base_url` (default `/v1/chat/completions`); set for providers whose base URL already includes an API prefix (e.g. BigModel `/api/paas/v4/...`). |

`ModelCompatOverride` layers model-level overrides for `system_role_override`
and `max_tokens_field` on top of the profile defaults (model wins over
provider). Static per-profile request headers (`extra_headers`, used for
session affinity / routing) are a separate profile config field threaded
through `OpenAiChatProvider` construction, not a `CompatConfig` flag.

OpenAI Responses native semantics (`ResponsesConfig`): `store` and
`strict_tools` are implemented. Static `reasoning_effort` remains legacy
compatibility metadata; `request.thinking` plus the selected
`ModelInfo::thinking_level_map` is authoritative for Chat and Responses wire
output.
`previous_response_id` is intentionally absent — Responses requests are built
as Chat-Completions analogues, so server-side response chaining is not wired.

## Cache, Response IDs, and Session Affinity

Usage-side cache tokens are normalized where the provider supplies them
(Anthropic `cache_read`/`cache_creation`, OpenAI Chat/Responses
`cached_tokens`, Gemini `cached_content`, Bedrock `cache_read`/`cache_write`).
For capable built-in Anthropic models, request-side prompt-caching breakpoints
are emitted on the system prompt, final user/assistant text blocks, and final
tool definition according to `CacheRetention`. Compatible OpenAI-style
profiles use the separate `cache_key` cache-affinity hint.

Provider response IDs are captured and round-tripped into
`AssistantMessage::response_id` (Anthropic `message.id`, OpenAI Chat
`chatcmpl-*`, OpenAI Responses `resp_*`); OpenAI Chat captures the ID from any
chunk carrying `id`, not only role chunks, and other families leave it `None`.

Session affinity is intentionally limited. For an effective session, direct
OpenAI Responses automatically derives `prompt_cache_key` and a fresh
`x-client-request-id`; `send_session_id_header` gates only `session_id`.
Custom/proxy affinity remains disabled by default and requires explicit
opt-in. `previous_response_id` is deferred (see OpenAI-Compatible Profiles),
and compatible profiles may carry static `extra_headers` for routing/session
pinning. There is no server-side session chain.

## Proxy

`HttpClient` carries shared `reqwest` pooling with explicit per-provider proxy
config (provider profile `proxy.url` and `proxy.no_proxy`) and environment
fallback (`HTTPS_PROXY` > `HTTP_PROXY` > `NO_PROXY`). Proxy credentials in a
proxy URL are redacted before any diagnostic display. Proxy transport semantics
(retry-through-proxy, cancellation) are part of the retry/proxy coverage.

## Best-Effort Cost

Cost mapping is best-effort. Incorrect confidence is worse than explicit
unknown values: when provider usage is absent, `Usage::unknown()` and
`CumulativeUsage` keep the turn explicitly unknown instead of treating it as
known-zero usage. Session-facing cost summaries should therefore be omitted
when any turn has unknown usage or when pricing is absent. Cost never blocks a
successful stream.

## Non-Goals

The following are explicit non-goals and must not appear as current core
behavior:

- OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex.
- Provider catalogs beyond the audited static pi-0.80.6 GitHub Copilot and
  OpenAI Codex snapshots, including live entitlement filtering.
- Automatic re-login after credential revocation.
- Per-call API-key/env/auth-header override.
- Provider payload/response streaming hooks.
- A broad new first-class provider list (compatible providers stay
  config-driven profiles).
- Image generation (image support is input-only).
- Browser automation outside the approved Anthropic and OpenAI Codex OAuth login flows.
- Provider streaming-adapter protocol for packages.
- Paid live provider calls in default tests (live tests stay `#[ignore]`-gated).
- Copying pi's provider-specific config file format.

## Minimal Example

```rust
// Cargo.toml deps: opi-ai, secrecy, tokio (features "macros", "rt-multi-thread"),
// tokio-util, futures-util.
use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::{
    AuthProvenanceSource, AuthScheme, CompatMetadata, ProviderCollection,
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
    let mut collection = ProviderCollection::new();
    collection.register_route(
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let prepared = collection.prepare_call(model, request).await?;
    let mut stream = prepared.start_attempt()?;
    while let Some(event) = stream.next().await {
        println!("{:?}", event?);
    }
    Ok(())
}
```

## Modules

`provider`, `message`, `stream`, `model_info`, `api_mapped`, `registry`,
`provider_collection`, `provider_headers`, `auth`, `credential`, `http`, `retry`, `model`,
`anthropic`, `openai_chat`, `openai_responses`, `openai_codex_responses`,
`openrouter`, `mistral`, `gemini`, `bedrock`, `azure_openai`, `vertex`,
`config`, `time`, and `test_support`.

## License

MIT. See the workspace [LICENSE](../../LICENSE).

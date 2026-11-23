# Provider Wire and Operation Reference

This page describes current provider-specific behavior after registration and
authentication have been selected. Use the [Provider, Catalog, Auth and Refresh
reference](provider-auth.md) for factory, catalog, credential and OAuth rules;
use [Transport, Protocols and Resource Lifecycle](transport.md) for shared HTTP,
TLS, WebSocket and cleanup behavior.

## Native Chat Adapters

Native adapters preserve provider-specific options and wire shapes when a
compatibility profile cannot express the behavior. The public registration
seam is `Models::register_chat_api`; typed options carry their `ApiId` and
remain valid across compatible registrations.

Anthropic Messages retains tool chronology, cache controls, thinking/signature
blocks, inline tool changes and the provider's error-result image placement.
Google Generative AI and Vertex preserve their content parts, safety settings,
thinking levels, usage presence and finish-reason mapping. Bedrock Converse
uses the selected AWS or bearer authentication path and its event-stream
termination. Mistral, Codex and pi-messages keep their own request envelope
and streaming terminal events rather than being forced through a generic chat
serializer.

## Amazon Bedrock

The `amazon-bedrock` builtin accepts the selected Converse model ID from the
catalog, including regional inference profiles. Streaming sends
`POST /model/{id}/converse-stream` to the selected runtime endpoint. Tools
that request JSON Schema constraints use the selected model's strict-tool
capability. When strict mode is selected, the request sends the converted
schema and `toolSpec.strict: true`. Adaptive Claude models use the catalog
effort mapping in `additionalModelRequestFields.output_config.effort`.

The standalone factory and coding-agent use the same reviewed Bedrock model
list as the builtin catalog. The standalone route resolves standard AWS
endpoints through its endpoint configuration and authentication region.
Choose model IDs from the current catalog when configuring defaults.

In `Models` calls, a pre-stream `ValidationException` with a recognized
strict-tool limit and an enabled `Prefer` tool allows one request rebuild.
The SDK sends the original schema for each `Prefer` tool and
keeps `Require` and ordinary tools as projected. The rebuilt payload runs
its own payload callback and uses the same resolved authentication.
The result includes the `bedrock_strict_tools_fallback` diagnostic.

## GitHub Copilot

The SDK and coding-agent use the embedded Copilot chat catalog. The selected
record routes each request to `/responses`, `/v1/messages` or
`/chat/completions`. OAuth supplies the account's base URL. The adapters derive
`X-Initiator`, `Openai-Intent` and `Copilot-Vision-Request` from the
request. SDK availability checks intersect the catalog with the account's
allowed model IDs when the stored OAuth credential includes them.

## OpenAI Responses

The SDK builtin, standalone Responses factory and coding-agent use the same
OpenAI chat catalog. The OpenAI endpoint receives `POST /v1/responses`.
The SDK builtin exposes OpenAI Decisions separately as a classifier.

The selected model determines how tools introduced after the first turn
reach the provider. GPT-5.4, GPT-5.4 Mini, GPT-5.4 Pro and GPT-5.5 send
the current set in top-level `tools`. They support strict JSON Schema tools,
grammar tools and mid-conversation instructions. Models such as
`gpt-6.1-terra` support inline `additional_tools` for additions.

## OpenAI Codex

The SDK and coding-agent select chat models from the embedded Codex catalog.
The dedicated adapter sends requests to `/codex/responses` on the selected
model's base URL. Stored OAuth supplies Bearer authentication and the ChatGPT
account ID. The simple entry uses the selected record's reasoning map. For
`gpt-6.1-terra`, Off, Minimal and Max send `none`, `low` and `max`.

## OpenCode Zen and Go

OpenCode Zen (`opencode`) and OpenCode Go (`opencode-go`) take their endpoints
from the selected catalog record. Chat Completions and Responses share each
provider's OpenAI base. Messages and Gemini use separate protocol bases. Chat
requests can carry `X-OpenCode-Session`, and the simple entry point selects a
supported reasoning level for the model. Zen's two Jev classifier models use
the System One endpoint with Bearer authentication.

## Baseten

The builtin `baseten` route uses the 14 chat records in the fixed Pi 1.5
catalog. Set `BASETEN_API_KEY` for Bearer authentication. Chat requests use
`POST /v1/chat/completions` on `https://inference.baseten.co/v1` and include
`stream_options.include_usage` when the catalog enables streaming usage.

Baseten models use the catalog's `thinkingLevelMap` and compatibility profile.
OpenAI-format records send `reasoning_effort`, while models with the Baseten
format send `chat_template_args.enable_thinking` and the mapped effort. A
level that is unavailable in a model's map is clamped by the neutral simple
entry according to the shared thinking-level policy. The builtin list follows
the fixed Pi release catalog, so retired IDs are rejected before dispatch.

## xAI

The builtin `xai` route exposes the four xAI Responses records from the fixed
Pi 1.5 catalog and sends Bearer authentication to `/v1/responses`. The
`grok-4.3` and `grok-4.5` records map XHigh to `reasoning.effort=xhigh`.
xAI reasoning requests include `reasoning.encrypted_content` so the provider's
encrypted reasoning can be preserved.

## Fireworks

The builtin `fireworks` route exposes the 29 Fireworks chat records from the
fixed Pi 1.5 catalog. Anthropic Messages records post to
`/inference/v1/messages`, while OpenAI Completions records post to
`/inference/v1/chat/completions`. Fireworks Anthropic requests use the
provider's API-key header and OpenAI-compatible requests use Bearer
authentication.

The `qwen-max-latest` record uses adaptive thinking and maps XHigh to
`output_config.effort=xhigh`. Records that enable session affinity propagate
the request session through the provider's affinity headers.

## TypeSafe

The builtin `typesafe` route exposes the single `jev-latest` classifier record
from the fixed Pi 1.5 catalog. Set `TYPESAFE_API_KEY` for Bearer
authentication. Text classification posts to `/v1/systemone` on the selected
record's base URL and maps public boolean questions to `noul`.

The catalog declares PNG and JPEG image input through TypeSafe's `assets`
map. Image requests place `<image-N>` asset names before the original state and
send `TypeSafe-Danger-Zone-Images: true`. Text-only requests keep the original
state and omit both the asset map and the opt-in header. Unsupported image
formats are rejected before dispatch.

## Radius

The builtin `radius` route exposes the 28 Radius Pi Messages records from the
fixed Pi 1.5 catalog. The default gateway uses the bundled baseline. An
authenticated `GET /v1/config` with Bearer replaces that baseline with the
gateway-owned catalog, and chat requests use Bearer `POST /v1/messages` with
Pi Messages SSE. The four retired IDs are rejected before dispatch.

## Together

The builtin together route exposes the 15 Together OpenAI Completions records
from the fixed Pi 1.5 catalog. Bearer authentication is sent to
POST /v1/chat/completions. Qwen3.7-Max enables reasoning and uses the
published max_tokens limit. The five retired IDs are rejected before dispatch.

## Hugging Face

The builtin `huggingface` route exposes the 78 Hugging Face OpenAI Completions
records from the fixed Pi 1.5 catalog. Bearer authentication is sent to
`/v1/chat/completions`. DeepSeek-R1-0528 uses the published
`max_completion_tokens` limit and sends `reasoning_effort=high` when reasoning
is enabled. The three retired IDs are rejected before dispatch.

## Vercel AI Gateway

The SDK builtin `vercel-ai-gateway` exposes 303 chat models and 14 text
classifiers from the bundled catalog. Set `AI_GATEWAY_API_KEY` for both
operations. Chat sends `POST /v1/messages?beta=true` with `x-api-key`.
Messages beta features are sent in the `anthropic-beta` header.
Classification sends `POST /typesafe/v1/systemone` with Bearer authentication.
Select classifier records through `Models::classifier_model`.

Simple chat calls send `thinking.type=disabled` when reasoning is off on a
reasoning model that supports Off. For `openai/gpt-6.1-terra`, XHigh uses a
16,384-token thinking budget subject to the context and output ceilings.
`moonshotai/kimi-k3.1-pro` advertises a 131,072-token output limit.

## Image Operation

Image input uses ordered `InputContent::Image` values with explicit media type
and source metadata. Image generation is a separate operation with its own
`ImageModelRecord`, `ImagesContext`, `ImagesOutput` and typed result. A provider
failure is returned as a typed result failure; it is not silently converted to
chat content. The OpenRouter image adapter applies the shared timeout, retry,
callback and cancellation boundaries.

## Classification Operation

Import a classifier adapter from the module matching its wire API. Consumers
upgrading from the previous public paths need these import changes.

| Wire API | Previous module under `opi_ai::api` | Current module |
| --- | --- | --- |
| `openai-decisions` | `decisions` | `openai_decisions` |
| `cloudflare-workers-ai-system-one` | `cloudflare` | `cloudflare_workers_ai_system_one` |
| `llama-cpp-classify` | `llama` | `llama_cpp_classify` |

Classification is a separate operation and model catalog type. The public
classifier adapters preserve the provider's answer shape while mapping it to
`ClassifierOutput`; missing, malformed, refused and non-finite answers are
reported as typed failures. The Decisions, System One and llama.cpp routes use
separate wire envelopes and do not share undocumented assumptions about score,
choice or boolean semantics.

For TypeSafe System One, the catalog declares PNG and JPEG image input as
`assets`. The adapter places `<image-N>` asset names before the original state,
sends `TypeSafe-Danger-Zone-Images: true` only for image requests, and rejects
unsupported formats before dispatch.

Cloudflare Workers AI System One records use the account-scoped `/run` endpoint
with Bearer authentication. Clef records declare `images` and send up to four
PNG, JPEG or WebP images as `input.images` data URLs inside Cloudflare's
`{model, input}` envelope. Jev records keep the catalog-declared `assets`
format. Cloudflare routes do not send TypeSafe's image opt-in header.

For OpenRouter System One, pass images in `ClassifierContext::images` and
select a classifier model that declares image input. The adapter places the
images in order as `image_url` data URLs before the original JSON state and
posts to `<base>/systemone`. The OpenRouter provider ID selects this format
with a configured proxy endpoint. Text-only models reject images before
dispatch. [OpenRouter System One API](https://openrouter.ai/docs/api/api-reference/systemone/submit-a-system-one-request)

## Reasoning and Native Options

The simple chat path carries a `ThinkingLevel`; native adapters may expose
provider-specific reasoning options through their typed options. Omitted values
remain omitted where the provider distinguishes omission from zero or false.
Streaming thinking and redacted-thinking blocks retain their order and are not
promoted to visible text when the route cannot preserve them.

Gemini and Vertex models with discrete thinking levels may disallow Off.
When thinking is disabled for such a model, the request uses its lowest
supported level as the sole field in `thinkingConfig`. A default simple call to
OpenCode Go's `gemini-4-argon`, for example, sends
`thinkingConfig: {"thinkingLevel":"LOW"}`.

Stop reasons are normalized to the public vocabulary (`Stop`, `Length`,
`ToolUse`, `Error`, `Aborted`, `Deferred`, `Pending`), while provider raw
reasons remain available only through the typed failure or adapter diagnostics.

## OpenAI-Compatible Provider Profiles

`CompatConfig` is an explicit override layered over the adapter's provider
inference. It controls instruction role, thinking format, store behavior,
extra headers and the fields documented by the selected profile. A profile
cannot add an unsupported wire capability, increase an output budget or replace
provider-managed authentication headers.

Azure keeps deployment/resource/api-version mapping explicit for both Chat and
Responses wires. OpenRouter, Mistral, Together, Qwen, xAI and other compatible
profiles retain their source-specific fields and omission rules. Use the
provider/auth topic for registration and the exact rustdoc options for code.

## Moonshot AI CN

The builtin moonshotai-cn route uses the six fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from MOONSHOT_API_KEY.
Requests post to /v1/chat/completions. Kimi K3.1 Pro and HighSpeed use the
catalog max_tokens field and map High to reasoning_effort=high. Model
selection is closed to the published six-record catalog.

## Moonshot AI

The builtin `moonshotai` route uses the six fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `MOONSHOT_API_KEY`. Requests post to
`/v1/chat/completions`. Kimi K3.1 Pro and HighSpeed map High to
`reasoning_effort=high`. The K3.1 Pro record uses the published input price 2,
output price 10, cache-read price 0.2 and 1048576-token context window.
Model selection is closed to the published six-record catalog.

## Qwen Token Plan

The builtin `qwen-token-plan` route uses the 21 fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from `QWEN_TOKEN_PLAN_API_KEY`.
Requests post to `/compatible-mode/v1/chat/completions`. The published
`deepseek-v4.1-flash-1017` record has a 1000000-token context window. Its
requests use `max_completion_tokens`, `enable_thinking=true` and the mapped
`reasoning_effort=high` when High reasoning is selected. Model selection is
closed to the published 21-record catalog.

## Qwen Token Plan CN

The builtin `qwen-token-plan-cn` route uses the 21 fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from
`QWEN_TOKEN_PLAN_CN_API_KEY`. Requests post to
`/compatible-mode/v1/chat/completions` with the same published thinking and
output-limit behavior. Model selection is closed to the published 21-record
catalog.

## Kimi Coding

The builtin `kimi-coding` route uses the six fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `KIMI_API_KEY`. Requests post to
`/coding/v1/chat/completions`. Kimi K3.1 Pro and HighSpeed map High to
`reasoning_effort=high`. The `k3-256k` record uses the published input price
3, output price 15, cache-read price 0.3 and 262144-token context window.
Model selection is closed to the published six-record catalog.

## Z.AI Coding CN

The builtin `zai-coding-cn` route uses the four fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from
`ZAI_CODING_CN_API_KEY`. Requests post to
`/api/coding/paas/v4/chat/completions` and use the catalog `max_tokens`
field. The published `glm-4.6v` record has a 128000-token context window and
0.05 cache-read price. Model selection is closed to the published four-record
catalog.

## Xiaomi

The builtin `xiaomi` route uses the three fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `XIAOMI_API_KEY`. Requests post to
`/v1/chat/completions`, use `max_completion_tokens`, and project enabled
reasoning as `thinking.type=enabled`. The published MiMo 2.6 records have a
1048576-token context window. Model selection is closed to the published
three-record catalog.

## Xiaomi Token Plan AMS

The builtin `xiaomi-token-plan-ams` route uses the two fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from
`XIAOMI_TOKEN_PLAN_AMS_API_KEY`. Requests post to `/v1/chat/completions`, use
`max_completion_tokens`, and project enabled reasoning as
`thinking.type=enabled`. The catalog records use a 1048576-token context
window and zero prices. Model selection is closed to the published two-record
catalog.

## Xiaomi Token Plan CN

The builtin `xiaomi-token-plan-cn` route uses the two fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from
`XIAOMI_TOKEN_PLAN_CN_API_KEY`. Requests post to `/v1/chat/completions` with
the same output-limit and DeepSeek thinking projection. Model selection is
closed to the published two-record catalog.

## Xiaomi Token Plan SGP

The builtin `xiaomi-token-plan-sgp` route uses the two fixed Pi 1.5 OpenAI
Completions records and Bearer authentication from
`XIAOMI_TOKEN_PLAN_SGP_API_KEY`. Requests post to `/v1/chat/completions` with
the same output-limit and DeepSeek thinking projection. Model selection is
closed to the published two-record catalog.

## MiniMax CN

The builtin minimax-cn route uses the four fixed Pi 1.5 Anthropic Messages
records and API-key authentication from MINIMAX_CN_API_KEY. Requests post to
/anthropic/v1/messages and use the x-api-key header. MiniMax-M3.1-Flash uses
adaptive thinking and maps XHigh to output_config.effort=xhigh. Model
selection is closed to the published four-record catalog.

## MiniMax

The builtin `minimax` route uses the four fixed Pi 1.5 Anthropic Messages
records and API-key authentication from `MINIMAX_API_KEY`. Requests post to
`/anthropic/v1/messages` with `x-api-key` and `anthropic-version=2023-06-01`.
`MiniMax-M3.1-Flash` uses adaptive thinking and maps XHigh to
`output_config.effort=xhigh`. `MiniMax-M3` exposes the published 524288-token
output limit. Model selection is closed to the published four-record catalog.

## NVIDIA

The builtin `nvidia` route uses the 21 fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `NVIDIA_API_KEY`. Requests post to
`/v1/chat/completions` with the catalog `NVCF-POLL-SECONDS: 3600` header.
NVIDIA records use `max_tokens` and do not send `reasoning_effort`. The
DeepSeek V4.1 Flash record maps High to its `thinking` envelope, while the
published Nemotron records without a thinking map omit that envelope. Model
selection is closed to the published 21-record catalog.

## Cerebras

The builtin `cerebras` route uses the two fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `CEREBRAS_API_KEY`. Requests post to
`/v1/chat/completions`. Cerebras uses the standard `max_completion_tokens`
output-limit field and `reasoning_effort` mappings. The published catalog also
declares `gpt-oss-120b` cache-read pricing of 0.35. Model selection is closed
to the published two-record catalog.

## Groq

The builtin `groq` route uses the seven fixed Pi 1.5 OpenAI Completions
records and Bearer authentication from `GROQ_API_KEY`. Requests post to
`/openai/v1/chat/completions`. `openai/gpt-oss-safeguard-20b` uses the
published cache-read price of 0.0375, and `qwen/qwen3.8-27b` has a
131072-token context window. Model selection is closed to the published
seven-record catalog.

## Cloudflare AI Gateway

The builtin cloudflare-ai-gateway route uses the 59 fixed Pi 1.5 records.
The catalog contains Anthropic Messages, OpenAI Completions and OpenAI
Responses records. Cloudflare resolves the API key, account ID and gateway ID
from CLOUDFLARE_API_KEY, CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_GATEWAY_ID,
then sends cf-aig-authorization.

Anthropic passthrough requests use /anthropic/v1/messages, Workers AI
OpenAI-compatible requests use /compat/chat/completions, and OpenAI
Responses requests use /openai/responses. gpt-6.1-terra maps XHigh to
reasoning.effort=xhigh, the DeepSeek Workers AI record uses max_tokens,
and claude-sonnet-5-5 uses adaptive thinking with XHigh effort.

## Direct Adapters

Direct adapters expose a typed public API when the operation's envelope or
response cannot be represented by generic chat. They include pi-messages,
OpenAI Decisions, System One, llama.cpp classification, OpenRouter images and
the provider-specific Vertex/Bedrock/Mistral routes. Each adapter freezes its
model/base/auth identity before the first request and uses the shared callback,
timeout, cancellation and typed-error boundaries.

## Provider-Specific Limits

Provider catalogs are finite and do not imply automatic registration, login or
network access. Construction is source-free; authentication and catalog refresh
happen only at the documented call or refresh boundary. Unsupported options,
unknown models, invalid tool/schema combinations, unavailable operations and
provider protocol failures fail before a successful event is published.

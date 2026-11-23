# Provider, Catalog, Auth and Refresh reference

This topic reference collects the public Provider, Catalog, authentication and
refresh contracts that are easy to need after the first `opi-ai` call. The main
[README](../README.md) keeps task-oriented onboarding and the compact Core API
table; this document contains the detailed lifecycle and capability matrices.

The topic follows the same ownership boundary as the crate: `opi-ai` owns
provider routing, catalog validation/publication, injected credential contracts,
OAuth metadata and refresh orchestration; hosts own keychain storage, login
presentation and exporter/runtime policy. The sections below describe the
current Rust-native behavior and do not imply an automatic login or startup
network policy.

Feature ownership: `oauth-store` independently enables the concrete SDK OAuth
flows and their store-backed login/refresh bridge, while the neutral
`OAuthProvider`, `OAuthCredential`, `CredentialStore`, mutation and
`auth::store_oauth` contracts remain in the core profile. The provider
collection and AWS/ADC/Vertex credential-source factories remain behind
`builtins`.

The independent `cloud-auth` feature owns protocol-independent AWS source
chains/STS exchanges, GCP ADC families, shared SigV4 signing and the pinned
query/XML/DPoP codec closure. It transitively selects shared HTTP/TLS and does
not select provider wire adapters, the bundled catalog or WebSocket compression;
`protocol-adapters` selects it transitively for Bedrock and Vertex callers.


Protocol ownership: `protocol-adapters` remains the compatibility group, while
these concrete owners can be selected independently: `provider-anthropic`,
`provider-azure-openai`, `provider-bedrock`, `provider-gemini`,
`provider-mistral`, `provider-openai-chat`, `provider-openai-codex-responses`,
`provider-openai-responses`, `provider-openrouter`, `provider-pi-messages` and
`provider-vertex`. `provider-protocol-common` and `provider-openrouter-usage`
are internal shared owners. Bedrock and Vertex transitively retain `cloud-auth`;
Codex Responses transitively retains `websocket-compression`; `builtins` keeps
the historical provider collection and credential factories.


## Factories, Catalog, and Auth

- Registration without hand-written routes: the bundled 42-profile factory
  (`providers::builtin_provider_profiles`) registers the
  completed adapters and catalog - see "Builtin Provider Factory Layer:
  42 Canonical Profiles".
- Model records: `Models::get_model` resolves registry metadata; the offline
  bundled catalog and dynamic `ModelsStore` refresh are described in
  "Offline Bundled Model Catalog" and "Dynamic Catalog Store and Refresh".
- Auth: injected `AuthResolver`s (as in the quick start), `CredentialStore`
  persistence, the nine OAuth families, and the standalone AWS
  credential-source chain live in `auth` / `auth::aws`; host keychain and
  terminal login policy stay in `opi-coding-agent` ("Credentials, Request
  Enrichment, and Refresh").

## Providers

| Module | Provider id | Backend |
|--------|-------------|---------|
| `providers::anthropic` + `api::anthropic` | `anthropic` | Anthropic Messages streaming |
| `providers::openai` + `api::openai_completions` | `openai` | OpenAI Chat Completions streaming |
| `providers::openai_responses` + `api::openai_responses` | `openai-responses` | OpenAI Responses streaming |
| `providers::openai_codex` + `api::openai_codex_responses` | `openai-codex` | Subscription-specific OpenAI Codex Responses streaming |
| `providers::mapped` | caller-defined | One provider identity/catalog routed across checked concrete wires |
| `providers::openrouter` | `openrouter` | OpenAI-compatible OpenRouter profile |
| `providers::mistral` + `api::mistral` | `mistral` | OpenAI-compatible Mistral profile |
| `providers::gemini` + `api::gemini` | `google` via `google_provider`, `gemini` via `GeminiProvider::new` | Gemini `streamGenerateContent?alt=sse` |
| `providers::bedrock` + `api::bedrock` | `bedrock` | AWS Bedrock Converse streaming with SigV4 or Bearer authentication |
| `providers::azure_openai` + `api::azure_openai` | `azure` | Azure OpenAI deployment-specific Chat **and** Responses wires |
| `providers::vertex` + `api::vertex` | `google-vertex` via `google_vertex_provider`, `vertex` via `VertexProvider::new` or `express` | Google Vertex AI Gemini streaming |

Built-in model lists are finite and intended for capability checks and model
listing. Site-specific models, fine-tuned models, and deployments should be
added through registry overrides or configured OpenAI-compatible profiles.

The standalone Gemini factories and the SDK's built-in Google route draw from
the same reviewed catalog. Direct dispatch returns `UnknownModel` for an ID
absent from the provider's model list. Use `with_models` to bind explicit custom
records. A factory host or proxy prefix supplies the default `/v1beta` endpoint,
and a complete endpoint on a custom record takes precedence.

Vertex's standalone and built-in routes use the same reviewed 13-model catalog.
API keys use the Express publisher collection with `x-goog-api-key`. Bearer
tokens use the project and location in the resource path. With the SDK builtin,
`BuiltinVertexConfig::base_url` supplies the generation host or proxy prefix
for either mode. A complete endpoint in an explicit SDK model record selects
the publisher collection and can supply its own API version.

## Credentials, Request Enrichment, and Refresh

`Models::new()` and `Models::from_registry(...)` each create an independent
`InMemoryCredentialStore`. Construction needs no runtime and performs no
external source discovery or environment/file/network access. Logins stay local
to that instance unless the caller attaches a shared store with
`set_credential_store`; that explicit store replaces the default. A local memory
ACK does not imply persistence across restarts.

`CredentialStore` keeps backend errors distinct from missing entries, while
`CredentialSource` reports present, absent, or backend-unavailable state
without exposing a secret. `AuthDescriptor::StoreCredential` carries only a
key and display source. `OAuthProvider`, `LoginPresenter`, and `AuthResolver`
are object-safe boxed-future seams. `auth::oauth` supplies Anthropic, Codex
legacy, ChatGPT direct, GitHub Copilot, OpenRouter, Kimi Coding, Meta, xAI and
Radius flows. `opi-coding-agent` supplies the OS-keychain store and terminal
presenter; SDK flows use the injected presenter and never launch a browser.

`WireApi` gives every `ModelInfo` one exact request wire, while public
`providers::mapped::ApiMappedProvider` exposes one provider identity and catalog and validates
its `WireApi -> Provider` routes before dispatch. One mapped provider shares
one lazy `AuthResolver` across all routes; provider/model metadata chooses the
route before network IO. Unknown models, missing routes, and wire/compatibility
mismatches are typed non-retryable failures.

### Lazy AuthContext and management results

Source access is a separate context responsibility: the object-safe
`AuthContext` (`env`, `file_exists`, `process_snapshot`) owns source access,
never model or telemetry data. `AmbientAuthContext` is the
default lazy implementation, and `Models::set_auth_context` replaces it
entirely — an injected source is never bypassed back to the process
environment. Construction, registration, and static catalog lookup perform
zero environment/store/network access; `AuthContextError` keeps non-unicode
environment bytes and failed existence probes typed instead of empty. A
selected native helper receives an owned `AuthProcessSnapshot` (exact env map
and cwd); an injected context defaults to an empty helper environment and
must supply required variables explicitly. Ambient context captures its
process environment only when that helper is selected, and a valid ADC output
cache needs no process snapshot. Commands execute directly with frozen argv,
cwd and env; timeout or waiter drop triggers process cleanup and reaping.

On top of that context, `Models::{check_auth, get_available, get_auth,
logout}` are the management results. `check_auth` reports configuration
without refreshing — an expired stored OAuth credential is still configured —
and surfaces unreachable store backends as typed `ModelsError::CredentialStore`
errors (probe snapshots included), never as silent `Ok(None)`.
For descriptor configuration checks, actual credentials in the default store
win. Only `Ok(None)` from that implicit memory store may consult an injected
`set_probe` snapshot containing redacted labels. An explicitly attached store
remains authoritative, including absence and errors. Probe metadata never
supplies authentication or authorizes dispatch. `get_available`
filters the registry by the same check. A registered OAuth route also honors
stored `available_model_ids`: `Some([])` is an authoritative empty account
catalog, while `None` keeps the registered catalog; checking never refreshes
that metadata. `get_auth` resolves the selected
source on demand; an explicit `AuthResolutionOverrides::api_key` wins over
every source without touching any, a route resolver's `CredentialNeeded`
reports unconfigured, and any other selected-source failure propagates as
`ModelsError::AuthResolution` with no fallback source tried. `logout` takes
the credential store per call and acknowledges exactly one local deletion:
no environment variable is unset, no remote session is revoked, and other
providers' entries stay untouched. Descriptor-only stored credentials resolve
without refresh. An OAuth route registered with `register_oauth_route` uses
its live resolver for `get_auth` and preparation, including required refresh.

GitHub Copilot routes one static catalog through Anthropic Messages, OpenAI
Completions/Chat, and OpenAI Responses; OpenAI Codex uses its dedicated
Responses provider rather than standard Responses compatibility flags. The
collection resolves `AuthResolver` once in `prepare_call`, before any attempt,
and returns `PreparedProviderCall`. Its `route()` exposes only `PreparedRoute`,
`auth_provenance()` exposes non-secret provenance, and every `start_attempt()`
reuses the frozen route, request, authentication, and cancellation token.
`start_attempt_with_telemetry()` wraps one such attempt in `ChatStream` and
settles an explicit AI observer; use the raw method when the caller owns retry
admission and stream normalization.
Attempts are sequential: an overlapping stream returns
`ModelsError::AttemptAlreadyActive`, cancellation returns `CallCancelled`,
and a prior credential-needed/revoked attempt closes the prepared call with
`CredentialTerminated`. Missing and revoked credentials surface as explicit, non-retryable
`ProviderError::CredentialNeeded` and `ProviderError::CredentialRevoked`
variants. Typed per-call API-key, Bearer and SigV4 overrides are mutually
exclusive and win before source access; `extra_headers` rejects provider-managed
auth headers.

### Store-owned mutations, ACKs, and the mutation barrier

Credential mutations (login, logout, replace, refresh) are owned by the
shared store, not by the waiter that submitted them. `CredentialStore`
gains `modify` and `mutation_barrier`: `modify` submits a serialized
read-modify-write (the operation sees the current credential plus a slot
view for post-failure re-reads) and settles into a non-secret
`MutationReceipt` — the ACK — recording applied writes with per-provider
revisions, typed failures, and unknown loss windows. Dropping the wait
never cancels a started mutation: it completes while the host runtime
lives, a second waiter re-reads the committed token instead of repeating
the operation, and per-provider FIFO serializes login/logout/replace/
refresh on one store (the production keychain bridge holds its
cross-process lock across each slot, preserving the host locking policy).

`CredentialMutationOp::{Write, Delete}` are unconditional and can replace or
remove corrupt entries without decoding them. `Update(CredentialUpdateOp)`
strictly reads the current value before running the callback. The existing
`write_mutation` and `delete_mutation` helpers construct unconditional operations.
Without a current Tokio runtime, the queue retains submissions as pending;
an ACK poll or normal submission inside a runtime hands them to its detached
owner. Barriers never start these operations. Runtime loss can leave pending,
unconfirmed effects without an ACK; a worker panic records `Unknown`.

`mutation_barrier` captures the store-wide cutoff synchronously — every
`Models` instance sharing one store observes the same generations. Its
report lists every captured mutation (pending, settled, failed, unknown)
plus post-cutoff admissions as fenced late writes; it never starts auth
and never aborts a running rotation. There are no default wait, receipt,
or admission caps: caller-provided bounds stop the wait only. The
reference `InMemoryCredentialStore` embeds the queue; stores without
shared state inherit an inline default. Revisions and receipts are an
in-process view; cross-process serialization stays the store's own lock
policy.

### Acquisition retry owner

`prepare_call_with_policy` freezes an `AcquisitionPolicy` per logical call
(ordinary `prepare_call` freezes the provider's protocol default: zero extra
sends for helper/Codex wires, the frozen AWS Smithy configuration for
Bedrock). The prepared call is
the one acquisition-retry owner: it classifies observed attempt failures,
keeps the per-payload-variant send counters (`max_retries` counts extra sends
per variant, so an ordinary episode performs at most `1 + max_retries`
sends), computes the source-qualified wait, and seals states. After a
retryable failure with budget remaining, `retry_admission()` returns the
1-based `retry_index` and `delay_ms` (server hints first, bounded by
`max_retry_delay_ms`; otherwise `min(500ms * 2^i, 8000ms)` scaled by a
`[0.75, 1.0)` jitter factor). `start_attempt` refuses dispatch with a typed
`ModelsError::RetryNotReady` before that deadline elapses, with
`CallSealed` once the call is sealed, and with `AttemptAlreadyActive` while
an attempt stream is still live. Lease reservation and the sealed/readiness
checks share one synchronization boundary. Seals are sticky and observable via
`acquisition_seal()`: a successful streaming-response handoff is
`PostHandoff`. OpenAI Chat preserves this fact even when its accepted response
fails before the first SSE event; body/EOF/parser failures cannot reopen
acquisition. The first delivered event also establishes handoff for adapters
without an earlier transport observation. A dispatched attempt dropped without
an observed outcome records
`UnknownDispatch`, a spent budget records `Exhausted`, and a terminal outcome
without any delivered event records `Finished`. Success therefore no longer
re-opens a prepared call: schedule retries through the admission, and
re-prepare for a new logical call. The main-loop consumer reads the
admission, waits cancellably, and re-enters `start_attempt`; it cannot reset
counters, skip the wait, or stack sends beyond the frozen budget.

`start_attempt` reserves the lease; the adapter and its producer start only on
the first poll. Dropping an unpolled lease sends nothing. The HTTP backend's
protocol-NACK retries are disabled; extra sends require SDK admission.

Except for the Pi Messages passthrough, before authentication `prepare_call` and
neutral simple calls resolve an omitted
`Request.max_tokens` to the selected complete model record's
`max_output_tokens` capacity, including applicable overrides and refreshed
catalogs. An explicit request limit remains a ceiling.
If enabled thinking has a budget at or above that default output limit,
preparation fails with a typed capability error before auth or dispatch. Both
implicit and explicit ceilings are then reduced to fit the estimated remaining
context after a 4096-token reserve. The estimate of available space has a
one-token floor. An explicit zero survives that clamp for the adapter to
interpret. Thinking that no longer fits is reduced to leave 1024 answer tokens
where possible. The non-adaptive Anthropic wire resolves an omitted thinking
budget from its level before wire projection. A zero native budget uses the
source default of 1024. Nonzero native budgets pass through. Adaptive effort
does not use a numeric minimum. The
`context` module estimates the original canonical message chronology, including
later System changes and tool declarations; the last
applicable positive successful usage (non-error, non-aborted, timestamp not
older than its prefix, any model identity) anchors the estimate and only
messages after the anchor are estimated, with no same-model or full-text
lower bound. Every attempt reuses the resolved limit. `Models::stream` and
`Models::stream_direct` validate the route and typed options, then pass the
caller's output limit to the adapter for wire projection. OpenAI Chat omits
the field for an unset native or direct limit. An explicit zero also omits
the field, including through `Models::stream_simple`. A positive limit uses
the field selected by Chat compatibility. Direct provider adapter calls
apply their wire defaults. These limits apply per request, independently of
any product turn-count budget.

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
`AssistantMessage::raw_stop_reason: Option<String>` retains a reported provider
termination fact independently of normalized `stop_reason`. Historical records
read the missing field as `None`, which is omitted when serialized. Existing
assistant-message literals add `raw_stop_reason: None`.
`CostBreakdown` has four catalog component lines: `input_cost`, `output_cost`,
`cache_read_cost`, and `cache_write_cost`, plus `total` and
`total_from_provider`. The weighted one-hour write subset is folded into
`cache_write_cost`, reasoning remains in `output_cost`, and cost and
total-token calculation count the parent buckets once. `catalog_total_cost()`
always sums the four catalog components; `total_cost()` uses the provider
reported `total` only when `total_from_provider` is true. `Usage::apply_catalog_cost`
recomputes the catalog estimate and clears a provider override.

Pricing tiers follow Pi's exact boundary rule: select the highest tier whose
`input_tokens_above` is strictly less than `input + cache_read + cache_write`;
the selected rates apply to the whole request. A one-hour cache-write subset is
charged at twice the input rate, while the remaining writes use the short-cache
rate and are never double-counted. OpenRouter billing fields are interpreted
only on an OpenRouter route: non-BYOK uses its charged `cost`, and BYOK adds the
reported upstream aggregate or its prompt-plus-completion fallback. Invalid or
incomplete provider accounting leaves catalog pricing intact.
All HTTP adapters decode streaming bytes through one incremental
replacement decoder: multi-byte UTF-8 split across network chunks survives
byte-exactly, each maximal invalid subsequence becomes one U+FFFD (no fatal
SSE UTF-8 rejection), valid keepalive comments and large lawful events beyond
the withdrawn size caps stay accepted, and AWS event-stream framing keeps its
prelude/CRC validation and checked arithmetic.
`Provider::refresh_models` and
`Models::refresh` implement deterministic atomic catalog
replacement, but remain substrate-only with no production trigger.

## Provider Behavior Matrix

Anthropic thinking content retains its opaque `signature`, and
`AssistantContent::RedactedThinking` retains encrypted `data`. These fields
survive message/session serialization and are replayed unchanged only on the
same wire, Anthropic provider and requested model. HTTP responses keep that requested
model in `model` and the service-reported name in `response_model`, so aliases
can continue their own signed history. A different identity drops all thinking
blocks, including signed and redacted blocks, without converting them to visible
text. Session exports omit both opaque
fields, even when readable thinking is included. Existing unsigned messages
remain readable with `signature: None`.
When the service omits the model name, `response_model: Some("")` preserves
explicitly unknown evidence instead of falling back to the requested alias.

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
| `bedrock` | ConverseStream parses `reasoningContent` into thinking events, preserves text signatures and retains redacted payloads as opaque thinking blocks | byte/base64 only; URL images rejected | `cache_read` / `cache_write` | — |
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

`CompatConfig` applies the single normalization
rule: every policy field is an explicit value layered over the adapter's
frozen provider inference (`store`/`instruction_role`/`thinking_format` and
field defaults are inferred per provider id or base-URL host; explicit
config wins per field, then model-level overrides win over the profile):

| Flag | Effect |
|------|--------|
| `store` | `Omit`/`SendFalse` store-field policy; the standard OpenAI baseline sends `store: false`, non-standard endpoints omit the field. |
| `instruction_role` | `Auto`/`System`/`DeveloperForReasoning`/`Developer` system-message role policy; the baseline uses `developer` only for reasoning models on standard endpoints. |
| `thinking_format` | One of the eleven declared reasoning wire formats (see below). |
| `supports_reasoning_effort` | Whether the endpoint accepts the standard effort field; `false` never disables a format's own toggle. |
| `usage_in_stream` | Request `stream_options.include_usage`; the inference baseline enables it, explicit `Some(false)` disables. |
| `max_tokens_field` | Output-limit field name override (`max_completion_tokens` on standard endpoints, `max_tokens` on the frozen legacy-compatible set). |
| `tool_result_name_field` | Echo the tool name on the tool-result message. |
| `strict_tool_schema` | Advertise strict capability; Require/Prefer tools use the shared strict converter, while unconstrained tools send `strict: false` on capable wires. |
| `cache_key` | Send the provider's prompt-cache key (cache-affinity hint). |
| `send_session_affinity_headers` | Map a request `session_id` to the compatible session-affinity headers (inferred for OpenRouter). |
| `require_assistant_after_tool_result` | Insert the fixed assistant bridge between a tool result and the next user message; assistant tool-call turns use `content: ""` instead of `null`. |
| `requires_thinking_as_text` | Replay assistant thinking as plain text parts. |
| `requires_reasoning_content_on_assistant_messages` | Mark replayed assistant turns with an empty `reasoning_content` (inferred for DeepSeek). |
| `zai_tool_stream` / `vllm_priority` | Explicit-only `tool_stream` and `priority` extras; never inferred. |
| `thinking_token_budget_field` / `supports_thinking_token_budget` | Top-level reasoning budget field (`thinking_token_budget` vLLM, `thinking_budget` Qwen/DashScope/SGLang, `thinking_budget_tokens` llama.cpp; the alias selects vLLM's). |
| `chat_template_kwargs` / `chat_template_args` | `$var` template values (`thinking.enabled`, `thinking.effort`, `thinking.budget`; `omit_when_off` drops the key when no effort is set). |
| `chat_completions_path` | A non-default path overrides the suffix. An explicit model API base uses `/chat/completions`; a constructor-only root uses `/v1/chat/completions`. |

Chat compat also exposes `supports_finish_reason`, `supports_grammar_tools`,
`open_router_routing`, `vercel_gateway_routing`, `cache_control_format`,
`session_affinity_format` and `supports_long_cache_retention`. Routing values
retain supplied empty arrays/objects, zero prices and an explicit null sort
partition. `vllm_priority` is `Option<serde_json::Number>`; template maps are
`Option<BTreeMap<...>>`, where `None` inherits and `Some(empty)` clears.
The catalog codec retains these values through its complete `x-opi.compat`
snapshot. With `supports_finish_reason: Some(false)`, lawful stream EOF closes
accumulated text/tool calls with an inferred reason and no invented raw reason.
Grammar capability selects custom tool declarations, streamed inputs and replay.
Anthropic cache markers and long retention follow the selected capabilities;
Chat affinity headers use the full session ID independently of prompt caching.

`OpenAiResponsesSourceCompat` is the single presence-aware source owner for
ordinary Responses, Azure Responses and Codex Responses. Missing optional
controls stay `None` in stored metadata; each wire resolves its own defaults.
Strict defaults to false for ordinary Responses and true for Azure/Codex;
Codex keeps unconstrained `strict: null`. Developer role defaults true, while
grammar, temporal additions/search, explicit cache and mid-conversation effort
default false. Ordinary Responses alone consumes the max-output capability,
explicit cache mode and mid-conversation effort. Its long-cache capability
defaults true; Azure retains native maximum-token behavior and Codex omits it.
Exact direct OpenAI signin calls freeze their source-qualified auth classification
at preparation and suppress the ordinary max/temperature/cache/effort branches.

The retired keys `system_role_override`, `supports_store`,
`supports_developer_role` and the static `reasoning_effort` string fail with
explicit migration errors at every config position (catalog `x-opi.compat`,
product TOML profile and model compat) — they are never silently ignored.
Request thinking intent and the selected `ModelInfo::thinking_level_map` are
the only sources of reasoning wire output.

Tool results put text (or the `(see attached image)` and `(no tool output)`
placeholders) in the `role: "tool"` message,
and image parts ride one trailing user message labeled
`Attached image(s) from tool result:` when the model accepts images; error
results keep the deterministic `[tool_error]` prefix.

### Declared thinking formats (on/off/null/omission)

`openai` sends `reasoning_effort` (on with the mapped value, off with the
model's declared off value); `openrouter` sends `reasoning: {effort}`; `deepseek`
sends `thinking: {type: enabled|disabled}` (a model whose off is unmappable
never sends the disable marker); `together` sends `reasoning: {enabled}`;
`baseten` sends `chat_template_args` plus an off effort value; `zai` sends
`thinking: {type: enabled|disabled}` with `clear_thinking`; `qwen` sends
`enable_thinking`; `qwen-chat-template` sends fixed `chat_template_kwargs`;
`chat-template` resolves configured `$var` values; `string-thinking` sends the
`thinking` string; `ant-ling` sends `reasoning: {effort}` only when an effort
is set.

### Native and simple options

| Option | Accepted type / unit | Omission · default | Precedence · wire field · typed rejection |
|--------|----------------------|--------------------|--------------------------------------------|
| `tool_choice` (native) | `OpenAiToolChoice`: `None`/`Auto`/`Required`/named function | Omitted field absent | Native options own the field; wire `tool_choice` (string or `{type: function, function: {name}}`); the simple entry admits only neutral `Auto`/`None`. |
| `reasoning_effort` (native) | `ThinkingLevel`: `minimal`/`low`/`medium`/`high`/`xhigh`/`max` | Omitted = no effort intent; off follows the declared format's off projection | Native effort replaces the neutral `Request::thinking`; wire field per `thinking_format`; an unsupported level fails typed `Config` preflight before auth. |
| `thinking_budgets` (native) | tokens per level (`u64`) | Omitted levels use the source defaults 1024/2048/8192/16384 (`xhigh`/`max` map to `high`'s) | Sent only through the configured budget field, clamped so at least 1024 tokens remain under the response ceiling; zero-or-below budgets are omitted. |
| Neutral effort (simple) | `Request::thinking` level | Disabled by default | The simple entry clamps an unsupported level to the nearest supported level (higher first, then lower); native entries retain exact levels. |
| `store` | — | Inference baseline | See the compat table; explicit config wins. |

`ModelCompatOverride` layers model-level `instruction_role`, `store`,
`thinking_format` and `max_tokens_field` overrides over the profile defaults
(model wins over provider). Static per-profile request headers
(`extra_headers`, used for session affinity / routing) are a separate profile
config field threaded through `OpenAiChatProvider` construction, not a
`CompatConfig` flag.

OpenAI Responses native semantics (`OpenAiResponsesOptions`):

| Option | Accepted type / unit | Omission · default · null · false/zero | Precedence · wire · typed rejection |
|--------|----------------------|-----------------------------------------|--------------------------------------|
| `reasoning_effort` (native) | `ThinkingLevel` (`minimal`..`max`) | Omitted = no native effort; wire `reasoning.effort` from the model's `thinking_level_map` | Native effort replaces the neutral `Request::thinking`; running an effort also requests `include: ["reasoning.encrypted_content"]`. An unsupported level fails typed `Config` preflight before auth; the simple entry clamps instead. |
| Off / disabled (both entries) | — | Reasoning-capable models send `reasoning: {effort: "none"}` (the map's off projection; `github-copilot` routes skip it) | Off sends effort only — no summary, no `include`. |
| `reasoning_summary` (native) | `ResponsesReasoningSummary`: `Auto`/`Detailed`/`Concise`/`Null` | Omitted **and** explicit `Null` both send the wire default `"auto"` (source `\|\|` semantics); a summary without an effort defaults the effort to `medium` | Wire `reasoning.summary`. |
| `service_tier` (native) | `ResponsesServiceTier`: `auto`/`default`/`flex`/`priority`/`fast` | Omitted field absent | Wire `service_tier`, exact string. The observed response tier rides `Usage::service_tier`; the effective billing tier is retained separately for wire-qualified cost calculation. |
| `tool_choice` (native) | `ResponsesToolChoice`: `None`/`Auto`/`Required`/named function | Omitted field absent | Wire `tool_choice` (string or `{type: "function", name}`); the simple entry admits only neutral `Auto`/`None`. |
| `store` | — | Always `false` unless a catalog/profile value pins another value (source sends `store:false` unconditionally) | Catalog compat > provider config > default `false`. |
| `max_output_tokens` | `Request::max_tokens` (tokens) | Omitted or zero omits the field | A positive value is sent with a floor of 16; simple calls resolve their shared budget before this wire mapping. |
| Encrypted reasoning / replay | persisted thinking `signature` | Unsigned thinking never fabricates an item | Running an effort requests `include: ["reasoning.encrypted_content"]`; reasoning items persist their serialized wire item as the signature and replay pushes it back into `input` (same-identity turns only); `xai` reasoning models always request encrypted content; terminal-output backfill patches signatures that per-item frames omitted. |
| Mid-conversation effort | catalog compat `supportsMidConvoEffort` (default `false`) + `provider_thinking_level` on replayed turns | Off by default; turns without a recorded level get no stamp | One `configuration_update` item before each earlier same-model reply that recorded a level, one with the current effort at the end; the request `reasoning.effort` stays fixed at `medium`; the reply records its level for the next turn. |
| Session cache | `Request::session_id` + `cache_retention` | `Disabled` suppresses; otherwise the clamped (64-char) `prompt_cache_key` | `cache_retention: Long` adds `prompt_cache_retention: "24h"`. Explicit prompt-cache mode (`prompt_cache_options`) is not adopted — no opi catalog consumer. |

Retired static `reasoning_effort` compatibility keys are rejected.
`request.thinking` plus the selected `ModelInfo::thinking_level_map` controls
Chat and Responses output. An explicit Responses model API base appends
`/responses` by default; a constructor-only root appends `/v1/responses`.
A non-default compatibility path remains an explicit override.
`previous_response_id` is intentionally absent — Responses requests are built
as Chat-Completions analogues, so server-side response chaining is not wired.

### Azure OpenAI dual wires

SDK builtin registration and coding-agent use the same Azure chat catalog and
`AzureCatalogProvider` route. Each selected record determines the protocol,
model limits and request capabilities. The standalone factory is available
with `provider-azure-openai`.

| Mapping | Precedence | Wire effect |
|---|---|---|
| Base URL | native `azure_base_url` or CLI `endpoint` → `AZURE_OPENAI_BASE_URL` → `AZURE_OPENAI_RESOURCE_NAME` → model `baseUrl` → typed `Config` failure | Catalog Chat and Responses use `/openai/v1` for a standard Azure resource host. Custom API bases keep their path. |
| API version | native `azure_api_version` or CLI `api_version` → `AZURE_OPENAI_API_VERSION` → `v1` | Responses sends `?api-version=…`. |
| Deployment | native `azure_deployment_name` → `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` → catalog model ID | Comma-separated `model-id=deployment` pairs set body `model`. Lookup and results use the catalog ID. |

The Responses wire (`WireApi::AzureOpenAiResponses`, `azure-openai-responses`
records, public `AzureOpenAIResponsesChatApi` registered through
`Models::register_chat_api`) reuses the exact Responses parameter assembly —
`store: false`, catalog effort mapping, summary default `auto`,
encrypted-reasoning include/signature/replay, the 16-token output floor —
under Azure transport (`api-key` header, API-base `/responses` URL, azure status
mapping, terminal duration, frozen telemetry producer). `AzureOpenAIOptions`
carries the native inventory: `reasoning_effort` (typed preflight; the simple
entry clamps), `reasoning_summary`, `tool_choice` and the four Azure mapping
fields. `service_tier` is deliberately absent on Azure (source omission; no
Responses service-tier multiplier). Mid-conversation effort stamps are not
wired for Azure (the source's azure `buildParams` never passes them).
Builtin Azure Chat uses `openai-completions`, appends `/chat/completions` to its
API base, and authenticates with Bearer syntax. The separate legacy direct
Chat surface retains `/openai/deployments/{deployment}/chat/completions` and
`api-key`. Both source wires keep provider identity `azure`; endpoint,
deployment and API-version values freeze once before acquisition attempts.

## Offline Bundled Model Catalog

The `catalog` module embeds opi's model data as offline crate resources.
Edit the model lists in [`resources/catalog/providers/`](../resources/catalog/providers).
Each filename identifies the provider. The pair `(type, id)` identifies a record
within that list. A chat model and an image model can share an ID.

From the repository root, build a candidate in a fresh directory and review
the generated files before copying them to `crates/opi-ai/resources/catalog/builtin`.
Use `check` to verify that the committed resources match the source lists.

```sh
cargo run -p opi-ai --no-default-features --example build_catalog -- build crates/opi-ai/resources/catalog/providers catalog-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- check crates/opi-ai/resources/catalog/providers crates/opi-ai/resources/catalog/builtin
```

The builder rejects duplicate JSON keys, unresolved builtin routes, incompatible
protocol fields and invalid typed model values. Use a top-level `x-<name>` for
opaque metadata. The persistence codec reserves `x-opi`. A capability change
also needs a fixture at the requesting adapter. The public bundle consists of
`catalog.json` and `manifest.json`. The builder also derives
`openrouter-profile.json` from selected Chat Completions records in
`providers/openrouter.json`. Removing a record from that source drops it
from the direct factory and the coding-agent listing on the next build.
The direct factory applies its configured endpoint to every selected record.
Selecting only `provider-openrouter` loads the small resource.
The v2 manifest records the opi format, the artifact and structure digests, and
the provider list. `Catalog::builtin()` validates the embedded pair once per
process. Consumers get a content-derived revision and counts by operation.
Use `scripts/pi-catalog/pack.py` for a read-only comparison. To follow a Pi
release, use the last upstream snapshot adopted for that provider, the target
release snapshot, and a trusted revision pin for each. The optional offline
sync tool merges those snapshots with the current opi source.

```sh
python scripts/pi-catalog/sync.py --provider mistral --old-artifact old-catalog.json --old-pin old-pin.json --new-artifact new-catalog.json --new-pin new-pin.json --opi-sources crates/opi-ai/resources/catalog/providers --output-dir sync-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- build sync-candidate/providers bundle-candidate
```

Upstream additions, removals and field changes apply by default. Apply a
retirement to every built-in entry point for that provider and operation.
Keeping an upstream-retired ID or adding a fallback requires the project
owner's explicit approval before implementation. Local
models, local deletions and independent field edits survive subsequent
upgrades. Objects merge by field and arrays are ordered values. Exit 2
reports conflicting identities and field paths to stdout and produces no
candidate. Exit 1 reports invalid input or output. On success, review the
JSON report, apply the selected provider file and generated resources, run
`check` and the affected listing or wire tests, and record the adopted
upstream digest for the next comparison. Normal crate builds use the checked-in resources.

`Catalog::from_bytes` still reads v1 bundles. Code that used
`provenance().pi_package`, `source_url`, `local_origin` or `generator` can
read those values from `CatalogOrigin::PiPublished` through `origin`. New
bundles from the opi builder use `CatalogOrigin::Opi`.

```rust
use opi_ai::catalog::{Catalog, CatalogOrigin};

fn legacy_pi_package(catalog: &Catalog) -> Option<&str> {
    if let CatalogOrigin::PiPublished { pi_package, .. } = &catalog.provenance().origin {
        Some(pi_package)
    } else {
        None
    }
}
```

Operation-qualified getters (`models(provider, operation)` and
`model(provider, operation, id)`) expose typed core fields and the complete
raw record. `CatalogHandle::try_install` validates the candidate before an
atomic swap. On error the last-known revision stays readable. Already-held
`Arc<Catalog>` values refer to the revision originally returned.

## Dynamic Catalog Store and Refresh


`ModelsStore::invalidate_source(id)` explicitly withdraws the records published
by that source and persists a tombstone. Old guards cannot publish afterward;
An offline `refresh` observes subsequent revisions of the attached shared store
without authentication or network work. A new successful source publication
clears its tombstone. This does not install an HTTP-status invalidation policy.

`catalog::personalize_openrouter_catalog` takes owned catalog records, an explicit
base URL/key, HTTP client and cancellation token. It returns separate records:
chat filtering/limits/prices are key-specific, image/classifier records stay
unchanged, and the authenticated bundled artifact is never rewritten. Missing
keys and HTTP 401/403/404 preserve the input. The 15-second source timeout covers
body acquisition; callers own publication and scheduling. `CatalogModel::
to_chat_model_info` projects a returned chat entry for an ordinary SDK call.
Unknown billed costs are `None`; known free rates remain `Some` with zero values.
Source metadata retains finite negative placeholders and tier order, while billing
validates the selected rates. Strict caller pricing constructors retain validation.


`Credential::ApiKey(ApiKeyCredential)` stores an optional key and an owned
provider environment (`BTreeMap<String, SecretString>`). Missing fields permit
the provider's documented source merge; present empty values remain distinct.
`ResolvedAuth::env` freezes selected configuration for the adapter. Debug output
omits environment values. The protected keychain envelope supports v2 scoped
credentials and strict legacy v1 reads; this does not change product config files.
Anthropic federation selects configured rule/organization/identity-file fields
without reading the identity file. The actual adapter exchanges at the selected
model base on first execution and retains its source cache/refresh behavior;
401 invalidates the next acquisition without resending the current generation.
Cancelling one wait leaves the owner-driven exchange running; releasing its
last owner aborts it. A later caller cannot restart an old token's expiry clock.

`ModelsStoreEntry` owns `Vec<CatalogRecord>` for chat, image and classifier,
plus optional `last_modified`/`checked_at` Unix milliseconds and verbatim `etag`.
`None` and zero differ. Dynamic sources return provider-keyed entries; model IDs
are unique within each operation. Restore and refresh publish all operations
after persistence ACK. Headers retain ordered set/remove operations; input limits
are metadata, and image pricing tiers are preserved while image fees use base rates.

`ModelsStore` (`catalog` module) is the single revision-commit owner for
dynamic catalogs: a `RefreshGuard::commit` persists the snapshot first
(atomic temp-file rename, distribution-schema-v1 JSON plus the `opi`
revision header) and returns exactly the acknowledged map, which
`Models::refresh` then applies to the registry — so memory and disk publish
together or not at all. Guards created before a newer revision are rejected
as `Superseded` without touching published state; cancelled guards commit
nothing; reopening a store restores the last-known revision before any
network work. The persisted file is the Pi *distribution* schema
(`models.schema.json`), a different artifact from the hydration data (v6)
embedded in the builtin bundle; opi-only fidelity (streaming, cache flags,
pricing tiers in their original order (including negative/fractional thresholds
and duplicates),
sampling maps at base and thinking levels, exact thinking maps, and wire compat
payloads including strict-tool support) round-trips
through an explicit `x-opi` extension, and plain v1 documents decode with
closed defaults.

`Models::provider_ids` lists the sorted union of registered chat, image,
classifier and deferred owners. `delete_provider` and `clear_providers` remove
only live model-provider bindings; shared credentials/catalog stores and
independently registered auth-only flows remain owned by their callers.
Captured calls retain their adapter revisions and explicit resource cleanup
still reaches retired resource owners. `get_models` stays chat-only;
`get_model_of_type`, `get_models_of_type` and `get_all_models` use the closed
`CatalogOperation`/`CatalogRecord` types without probing credentials. The async
`get_available_of_type` and `get_all_available` check each operation's existing
registration configuration and account restrictions, without resolving
builtin generation authentication.

`Models::refresh(ModelsRefreshOptions)` is the only refresh entry. Defaults
permit network work for all active providers with no forced-refresh choice.
Set `allow_network: false` to restore selected cached records without
OAuth/network work (`RefreshStatus::OfflineRestored`). `providers: None`
selects every active owner; `Some(empty_set)` and unknown IDs perform no source
work. `force: None`, `Some(false)` and `Some(true)` are forwarded unchanged only
to the selected network source, which owns their freshness meaning.
Native providers run in sorted ID order, then catalog sources in registration
order. `Provider::refresh_models` receives `CatalogRefreshContext`; each
`DynamicCatalogSource` declares `provider_ids` and receives only the active
selected intersection. Returning an undeclared provider fails that source;
declared but unselected records cannot change their existing publication.
`HttpCatalogSource::new(url, provider_ids)` declares ownership without fetching;
its network call fetches the fixed document once without inventing force
headers or query parameters.

Outcomes remain per-provider (`RefreshOutcome::{Published,Failed,Restored}` in
`RefreshReport`): successes publish and failures retain prior data. Native
`Ok(None)` restores builtin fallback unless the provider's registered explicit
source still owns its last-known catalog; authoritative empty publications
remain empty. Subset refresh merges over the complete shared-store snapshot,
preserves unselected persisted owners and publishers, and applies only selected
acknowledged records to this instance. Offline subset restores work even when
a previous subset already observed that store revision. Deleted owners cannot
expose cached records; explicit re-registration permits compatible restore.
`refresh_with(options, &token)` observes cancellation and the same store ACK
and stale-revision guard. Superseded or cancelled work cannot publish late;
source invalidation cannot resurrect unselected stale memory. Hosts attach
`ModelsStore` explicitly and register custom catalog sources as needed.
Construction and lookup do no catalog I/O.

### Radius gateway catalog

Builtin Radius starts with opi's bundled chat catalog for the default
gateway. For a custom gateway in `BuiltinOverrides::base_urls`, it starts
empty. An offline refresh restores a previous builtin catalog publication
from the attached store when the configured gateway origin matches. Clones
of one `ModelsStore` share its publication revision.

A network-enabled refresh of `radius` resolves the route's current API key
or OAuth token and requests `GET /v1/config` at that gateway. The
gateway's advertised chat base URL must have the same origin. A valid catalog
replaces the prior catalog in full, including an empty model list. Missing
credentials and refresh failures retain the last-known catalog. Network
refresh checks the gateway on each call.

## Builtin Provider Factory Layer: 42 Canonical Profiles

`providers::builtin_provider_profiles` exposes 42 builtin profiles with canonical ids
(`google`, `google-vertex`, `amazon-bedrock`, …), display names, default
base URLs, env API-key names, auth schemes, OAuth availability and endpoint
semantics (Azure deployments, Vertex location templates, Bedrock regions
and Radius gateway URLs). Legacy opi ids (`gemini`, `vertex`, `bedrock`,
`openai-responses`) are not aliases — the table never registers them.
Provider display names include Amazon Bedrock, Anthropic and Google Vertex AI;
credential labels remain separate metadata.
`providers::collection::register_builtin` turns the table plus the bundled
catalog into registered providers over the completed wire adapters. Multi-wire
profiles use a mapped provider. Radius keeps its Pi Messages route available
while its catalog loads. A canonical-route wrapper republishes fixed-id adapters under
canonical ids (including the model-spec prefix rewrite prefix-sensitive
adapters need), catalog entries converted with their tiered pricing into
`ModelInfo::pricing`, and the classifier/image protocols registered through
the typed seams with catalog records. Environment keys resolve via the
injected `AuthContext` when a call or an authenticated refresh needs them.

## Public entry points

The main integration points are `Models`, `ProviderRegistry`, `Provider`,
`AuthResolver`, `CredentialStore`, `Catalog`, `ModelsStore`,
`Models::refresh`, and `ModelsRefreshOptions`. Use the root README's Core API
table for the complete symbol inventory and the topic sections above for
lifecycle constraints and provider/catalog selection rules.

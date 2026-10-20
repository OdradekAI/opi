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

Current crate version: `0.8.3`, inherited from the workspace package version.

`opi-ai` owns the model/provider layer: request and message types, streaming
events, model metadata, provider registration, HTTP/proxy plumbing, retry
helpers, image content, usage accumulation, best-effort cost helpers, and the
provider-side error taxonomy consumed by `opi-agent` diagnostics. It does not
implement an agent loop, sessions, package loading, or built-in coding tools;
those live in `opi-agent` and `opi-coding-agent`.

`opi-ai` also exposes an unstable-0.x models/auth seam: the
`models` module (`Models`) is the single driver wrapping `ProviderRegistry`
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
| `Provider` | Backend trait with `id`, `models`, and `stream_prepared` (the sole dispatch entry, reached through `Models::prepare_call` with already-resolved auth). |
| `Request` / `CacheRetention` | Provider request: model, messages, tools, token limits, thinking config, metadata, cancellation, timeout, extra headers, cache retention, and session affinity. |
| `Message` | Provider-facing user, assistant, and tool-result messages. |
| `InputContent` / `OutputContent` | Text and image content blocks. |
| `ToolResultMessage` | Tool-result content, details, error/truncation/time metadata and optional typed `not_executed` facts from the trusted execution boundary. Historical absence means unknown effect; ordinary error text never proves non-execution. |
| `AssistantStreamEvent` | Provider-neutral stream events for start, text, thinking, tool calls, and done. The in-band `Error` terminal is a read-only legacy format retained for deserializing historical saved events; current producers never emit it. |
| `ChatStream` | The public owner of one logical call's event stream: iteration is `Stream<Item = Result<AssistantStreamEvent, CallFailure>>` with first-terminal absorption into permanent EOF, and `result(self)` actively drains to the single final outcome. |
| `CallFailure` | The single final failure of one logical call: the canonical partial/terminal `AssistantMessage` (content, provenance, diagnostics, known usage; setup failures have none) plus a typed redacted cause. |
| `drain_to_completion` | Collector sharing the `ChatStream::result` seam: `Ok(AssistantMessage)` for done terminals, one `Err(CallFailure)` for every failure, including premature EOF. |
| `WireApi` / `ModelInfo` / `ModelCapabilities` | Exact request-wire identity plus model capability, thinking, compatibility, and pricing metadata. |
| `ApiMappedProvider` | One public provider identity/catalog dispatched through a validated `WireApi -> Provider` route map. |
| `ProviderError` / `ProviderErrorCategory` | Provider failure taxonomy: auth, config, request, network, rate_limit, provider, stream, capability, and cancelled (timeouts classify as network). |
| `ProviderErrorSummary` | Closed redaction-safe provider-error text. Public construction is limited to `redacted()`, `authentication_rejected()`, and `from_untrusted(...)`; arbitrary provider payloads are not retained. |
| `ProviderRegistry` | Resolves `provider:model`, registers custom providers, and layers model overrides. |
| `Models` / `AuthDescriptor` / `AuthStatus` | Unstable-0.x single driver above `ProviderRegistry`: provider+model lookup, redacted auth state, OpenAI-compatible compat metadata, dispatch, and atomic dynamic-catalog refresh. Ordinary native/simple/direct entries and advanced `prepare_call` share one compiler, auth resolution, attempt seam, and collector. |
| `ChatApi` / `ApiId` / `ChatApiHandle` / `NativeOptions` | Typed custom chat adapter registration with its exact API identity and fixed options type; handle-created options carry the API/type tag and stay portable across instances with compatible registrations. |
| `ToolCall::arguments_error` | Optional final-parse outcome for textual tool arguments (AIR-048): empty input finalizes to `{}`, repairable invalid escapes stay literal, and incomplete/non-object finals keep the owned raw text plus this error, never a partial-prefix success. Historical records without the field keep deserializing. |
| `ChatModelRecord` / `SimpleOptions` / `ChatCall` | Complete caller-owned chat record (provider id, bare model id, exact wire) usable without catalog membership; neutral simple options; the lazy ordinary-call owner that captures input at construction and starts auth on first poll. |
| `PreparedProviderCall` / `PreparedRoute` | Opaque logical-call state returned by `Models::prepare_call`, plus its public redacted route facts. `route`, `auth_provenance`, and `start_attempt` are the prepared-call lifecycle APIs. |
| `CredentialStore` / `Credential` / `CredentialSource` | IO-free, object-safe credential persistence and redacted three-state probe contracts. |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | Flow-independent boxed-future OAuth contracts; concrete flows live in `opi-coding-agent`. |
| `AuthResolver` / `ResolvedAuth` | Driver-owned per-call auth resolution contract; one prepared result is frozen before an attempt starts. |
| `ApiKind` | Crate-root enum tagging the backend family (`Anthropic`, `OpenAi`, `Google`, `Mistral`) carried on assistant messages. |
| `HttpClient` | Shared `reqwest` client with pooling and explicit/env proxy support. |
| `ws` | Strict dynamic WebSocket Upgrade bridge (`upgrade`, `WsUpgradeRequest`, `Negotiation`, `fresh_nonce`/`fresh_mask`): request headers, nonce/mask entropy, and strict 101 validation; the neutral frame/PMD/Close/IO state machine is the `opi-ws` companion. |
| `retry` | The acquisition-retry owner (`AcquisitionPolicy`, `RetryAdmission`, `AcquisitionSeal`) plus the product-facing retry config, exponential backoff, and `Retry-After` parsing. |
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

### Lazy AuthContext and management results (AIR-063/AIR-066)

Source access is a separate context responsibility: the object-safe
`AuthContext` (`env`, `file_exists`) answers only "what does the ambient
environment say", never model or telemetry data. `AmbientAuthContext` is the
default lazy implementation, and `Models::set_auth_context` replaces it
entirely — an injected source is never bypassed back to the process
environment. Construction, registration, and static catalog lookup perform
zero environment/store/network access; `AuthContextError` keeps non-unicode
environment bytes and failed existence probes typed instead of empty.

On top of that context, `Models::{check_auth, get_available, get_auth,
logout}` are the management results. `check_auth` reports configuration
without refreshing — an expired stored OAuth credential is still configured —
and surfaces unreachable store backends as typed `ModelsError::CredentialStore`
errors (probe snapshots included), never as silent `Ok(None)`. `get_available`
filters the registry by the same check. `get_auth` resolves the selected
source on demand; an explicit `AuthResolutionOverrides::api_key` wins over
every source without touching any, a route resolver's `CredentialNeeded`
reports unconfigured, and any other selected-source failure propagates as
`ModelsError::AuthResolution` with no fallback source tried. `logout` takes
the credential store per call and acknowledges exactly one local deletion:
no environment variable is unset, no remote session is revoked, and other
providers' entries stay untouched. Stored OAuth credentials resolve without
refresh in the management path; refresh belongs to the live route/flow owners.

GitHub Copilot routes one static catalog through Anthropic Messages, OpenAI
Completions/Chat, and OpenAI Responses; OpenAI Codex uses its dedicated
Responses provider rather than standard Responses compatibility flags. The
collection resolves `AuthResolver` once in `prepare_call`, before any attempt,
and returns `PreparedProviderCall`. Its `route()` exposes only `PreparedRoute`,
`auth_provenance()` exposes non-secret provenance, and every `start_attempt()`
reuses the frozen route, request, authentication, and cancellation token.
Attempts are sequential: an overlapping stream returns
`ModelsError::AttemptAlreadyActive`, cancellation returns `CallCancelled`,
and a prior credential-needed/revoked attempt closes the prepared call with
`CredentialTerminated`. Missing and revoked credentials surface as explicit, non-retryable
`ProviderError::CredentialNeeded` and `ProviderError::CredentialRevoked`
variants. Per-call credentials remain out of scope: `extra_headers` rejects
provider-managed auth headers.

### Acquisition retry owner (AIR-007)

`prepare_call_with_policy` freezes an `AcquisitionPolicy` per logical call
(`prepare_call` keeps the default of zero extra sends). The prepared call is
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
an attempt stream is still live. Seals are sticky and observable via
`acquisition_seal()`: the first delivered stream event is the
streaming-response handoff (`PostHandoff`; later body/EOF/parser failures
are final), a dispatched attempt dropped without an observed outcome records
`UnknownDispatch`, a spent budget records `Exhausted`, and a terminal outcome
without any delivered event records `Finished`. Success therefore no longer
re-opens a prepared call: schedule retries through the admission, and
re-prepare for a new logical call. The main-loop consumer reads the
admission, waits cancellably, and re-enters `start_attempt`; it cannot reset
counters, skip the wait, or stack sends beyond the frozen budget.

Before authentication, `prepare_call` resolves an omitted `Request.max_tokens`
to the selected registry model's `max_output_tokens` capacity, including model
overrides and refreshed catalogs. An explicit request limit remains a ceiling.
If enabled thinking has a budget at or above that default output limit,
preparation fails with a typed capability error before auth or dispatch. Both
implicit and explicit ceilings are then reduced to fit estimated remaining
context with 4096 safety tokens and a minimum output of one token. Thinking that
no longer fits is reduced to leave 1024 answer tokens where possible. The
non-adaptive Anthropic wire resolves an omitted thinking budget from its level
and rejects ceilings that cannot fit its minimum thinking and answer room
before authentication; adaptive effort does not use a numeric minimum. The
`context` module estimates messages, system text and tool schemas; applicable
same-model usage plus new messages replaces cumulative session usage. Every
attempt reuses the resolved limit. Direct provider adapter calls retain their
existing default behavior; this is a per-request output limit, independent of
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
`CostBreakdown` has four lines: `input_cost`, `output_cost`,
`cache_read_cost`, and `cache_write_cost`. The weighted one-hour write subset
is folded into `cache_write_cost`, reasoning remains in `output_cost`, and
cost and total-token calculation count the parent buckets once.
All HTTP adapters decode streaming bytes through one incremental
replacement decoder: multi-byte UTF-8 split across network chunks survives
byte-exactly, each maximal invalid subsequence becomes one U+FFFD (no fatal
SSE UTF-8 rejection), valid keepalive comments and large lawful events beyond
the withdrawn size caps stay accepted, and AWS event-stream framing keeps its
prelude/CRC validation and checked arithmetic.
`Provider::refresh_models` and
`Models::refresh` implement deterministic atomic catalog
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

Anthropic thinking content retains its opaque `signature`, and
`AssistantContent::RedactedThinking` retains encrypted `data`. These fields
survive message/session serialization and are replayed unchanged only on the
same Anthropic provider and requested model. HTTP responses keep that requested
model in `model` and the service-reported name in `response_model`, so aliases
can continue their own signed history. A different route receives signed
thinking as readable text and omits encrypted blocks. Session exports omit both opaque
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

## WebSocket Upgrade (RDF-24)

`ws::upgrade` performs the strict dynamic Upgrade over the shared,
source-qualified `HttpClient` (explicit proxy/TLS profile, redirects must be
disabled per client: an upgrade request against a client with a redirect
policy fails typed instead of silently following). The bridge owns the
request shape (`Connection: Upgrade`, `Upgrade: websocket`,
`Sec-WebSocket-Version: 13`, a fresh 16-byte `Sec-WebSocket-Key` from the
pinned platform entropy, offered subprotocols and an optional
`permessage-deflate` offer) and strict validation of the 101 response:
status and HTTP version, exact `Sec-WebSocket-Accept` (RFC 6455 known
vector pinned), duplicate/absent singleton rules, token-wise
`Connection`/`Upgrade` matching, offered-only subprotocols, and exact
permessage-deflate parameter validation (no duplicate entries, no unknown
parameters, value ranges, no parameters the offer did not include, no
client window above the offered ceiling). Any violation is a typed
`WsUpgradeError` before the stream detaches.

The neutral protocol state machine is deliberately outside this crate: the
`opi-ws` Independent Companion (`companions/ws`, wired here as a
path-only development dependency) owns strict RFC 6455 frames, message
assembly, permessage-deflate with exactly-once `00 00 FF FF` tail injection,
the explicit Close exchange and the async IO owner. `Negotiation::
driver_parameters` hands the validated `(pmd, server_no_context_takeover)`
facts to `opi_ws::io::Driver::from` the detached `reqwest::Upgraded` stream;
the published package resolves the companion by registry identity once that
publication is authorized. The real provider consumer (Codex WebSocket,
20.16) and platform/package acceptance (20.57/20.58) remain open.

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

## Private Schema Dialect Engine

`schema` implements the fixed private TypeBox JSON dialect selected by the
schema readiness annex: the evaluation semantics of the pinned TypeBox 1.3.27
`Compile` path over trusted JSON schemas — all 42 JSON mechanisms, the 21
default string formats, structural keyword guards (a keyword whose guard fails
is inert, never an error), JavaScript property-key enumeration order
(integer-index keys ascending, then insertion order) and full
lexical/resource/dynamic/recursive reference frames. Evaluation runs on an
explicit work stack (schema depth cannot exhaust the Rust call stack),
distinguishes productive recursion from no-progress recursion as a typed
failure, and carries no implicit I/O or source-free size/fuel limits.
Regex admission follows source-selected schema edges; annotation objects are
not schemas unless reached as reference targets. Reference targets are frozen
in the compiled plan, including resource effects from fields absent in the
instance. Admission reports `NoProgress` only after proving that a repeated
compiler prefix's URI decisions and function-cache membership remain invariant;
it does not prune active nodes or impose an iteration limit. Null reference
targets use the source's false fallback; root null and root/referenced numbers
or strings fail with `InvalidSchema`, while JSON arrays remain inert.
Regex and URL operations use upstream crates.io `regress 0.12.0` and `url 2.5.8`.
Unicode properties, scripts, case mapping and IDNA data follow the supported
dependency profile, rather than a fixed Unicode version. The permitted data
differences and upgrade checks are recorded in
[RDF-21](../../docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md#rdf-21-upstream-data-profile).
The workspace lock does not fix every library consumer's graph. Opi disables
regress default features and enables `std`, `utf16`, `prohibit-unsafe`; ordinary
external consumers also exercise feature union. These data differences do not
relax schema guards, admission/ref order, normalization, authorization or the
no-I/O contract. Full conformance and package/platform acceptance remain open.

Upstream profile review (regress, `url`, `idna`, `idna_adapter`, ICU4X
`icu_*`, `unicode-ident`, and their `yoke`/`zerofrom`/`zerovec` support
crates): the workspace resolves ICU data 2.2.0 and `displaydoc 0.2.5`; the
ordinary-consumer proof resolves ICU data 2.3.0 and `displaydoc 0.2.7`.
Both `icu_normalizer_data` and `icu_properties_data` ship a `build.rs` that
checks `ICU4X_DATA_DIR`. Their `src/lib.rs` can include that directory's
`mod.rs` instead of bundled data; a defined empty value can also select this
path and fail the build. `compiled_data` does not disable this build-time
input. The proof runner rejects every defined value, including empty, before
creating scratch data or launching any subprocess. That guarantee applies to
the harness; ordinary Cargo consumers must control their own build environment.
It does not change the Schema runtime's no-I/O contract.

The resolved closure also contains `proc-macro2`, `quote`, `serde` and
`serde_core` build scripts. Its proc-macros include `displaydoc`, `serde_derive`
and the `yoke-derive`, `zerofrom-derive`, `zerovec-derive` companions. These
build inputs require source and feature review; no new native C/C++ code enters
through the Schema data profile (`ring` remains the pre-existing rustls
transport dependency).

regress is compiled with `prohibit-unsafe`, an upstream implementation-selection
feature, not a `forbid(unsafe_code)` or `deny(unsafe_code)` compilation rule.
Several sites use `cfg!` to select implementations, and regress 0.12.0's
`src/unicode.rs::uppercase` still uses `unsafe` `get_unchecked` when this
feature is enabled. Opi calls safe APIs, but the feature does not establish an
unsafe-free dependency tree: remaining upstream unsafe and its invariants must
be reviewed against the exact source and effective features on upgrades.
Family `rust-version` requirements peak at 1.88 (ICU4X 2.3.x on the consumer
side), within the workspace MSRV 1.97; this is a manifest review, not a native
platform or MSRV build result.
Licenses are MIT OR Apache-2.0 (regress, `url`, `idna`, `idna_adapter`),
Unicode-3.0 (ICU4X family and support crates), and
`(MIT OR Apache-2.0) AND Unicode-3.0` (`unicode-ident`); `displaydoc` and the
`serde`/`proc-macro2`/`quote`/`syn` tooling use MIT OR Apache-2.0. Resolved versions,
registry sources and checksums are recorded in `Cargo.lock`, and the actual
ordinary-consumer resolution and effective features of both caller profiles
in the package-consumer proof evidence
(`scripts/pi-sdk-conformance/evidence/upstream-package-proof.json`).

## Public Tool-Argument Normalizer/Validator

`schema::validate_tool_arguments` (and the name-lookup
`schema::validate_tool_call`) is the SDK-owned seam over the dialect engine:
an owned `arguments_error` marker rejects before any coercion; the caller's
raw value is never mutated; normalization deletes optional `null` properties
whose independently compiled sub-schema rejects `null`, applies the pinned Pi
coercion corpus (allOf fold, anyOf/oneOf value-first then clone-per-arm,
type-union primitive coercion in `type`-array order with exact ECMAScript
`Number`/`String` conversions — `Number("0x10")` is 16, `String(1e21)` is
`"1e+21"` through the pinned ryu-js 1.0.2), and finally checks the
normalized root against the original schema (the upstream's return of
unvalidated raw on a failing coerced root is deliberately not replicated —
RDF-09). Invalid outcomes carry redacted failures only (instance pointer,
keyword, schema-derived reason); precise JSON integers keep their exact
decimal digits instead of replicating JavaScript's parse-time precision loss
(RDF-09). TypeBox `Value.Convert` is a no-op for plain JSON dialect schemas
(its dispatch reads the non-JSON `~kind` marker), and `~kind` markers in
JSON schemas stay inert — a documented dialect boundary.

## Canonical Replay Projection

`replay` turns a canonical chronology into the request-local wire view
(AIR-010/012/013/014, task 20.7). `project_replay(&[Message], &ReplayRoute,
ReplayProjection)` validates tool association by batch position plus source
ID: a closed batch may reuse an ID, but a duplicate call ID, duplicate
result, orphan result or result/call name mismatch inside a batch is a
typed `ReplayError` raised before any request is built (before
authorization or network). An unambiguous missing result closes
request-locally with an error stating the result is unavailable and the
execution effect is unknown - never "not executed", never re-executed;
only a trusted execution boundary records known non-execution. When the
failed assistant's call view is omitted, a sanitized ordinary user context
retains the missing-result fact without fabricating a tool call or result.
Continuation identity is the exact wire plus provider plus requested model
(`AssistantMessage.wire`, serde-defaulted: legacy-unknown is never
same-identity). Cross-identity turns drop every thinking block - readable,
signed and redacted alike - and never convert reasoning into visible text
(Pi 1.3 semantics); same identity keeps redacted data and signed blocks,
including signed-empty ones, and drops blank unsigned thinking. Failed
turns (`Error`/`Aborted`) and their linked non-execution results are
omitted as a closed unit while `Length` turns keep their expressible
pairs; length never grants execution. Malformed proposals ride their raw
text on string-argument wires (Chat, Responses) and become explicit named
failure contexts on object-argument wires (Anthropic, Bedrock, Google) -
never `{}`, `null` or a fabricated call. Wire IDs are legalized
request-locally (charset/length, stable bijective, synchronized across
call/result; canonical untouched). Images keep association, order and
failure flag on capable routes (Anthropic error-result images split after
the turn's results); `details` never serialize and URL sources pass
through undownloaded. `ReplayProjection::Strict` rejects the same lossy
transforms with a typed error instead of recording facts. All six provider
adapters consume the projection at the head of request construction
(`build_request_body` now returns `Result`); the Responses parser packs
`call_id|fc_item_id` for same-identity dual-ID round-trip and the
serializer restores both, dropping the item id for foreign history. Pinned
reference: `.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts`;
per-wire encrypted-reasoning parity (AIR-038) and effort stamps (V-32)
close with later tasks.

## Canonical Context/Transcript Chronology

`transcript` owns the canonical request chronology (AIR-008/011/020): the
public `Context` shorthand (`system_prompt`, `messages`, `tools`) folds
exactly once into a leading `Message::System` entry through
`transcript::normalize_context`, returning a typed
`Result<TranscriptContext, TranscriptError>` — a raw shorthand cannot reach provider code. Later
system messages change the running state: `content` appends, named
`sections` replace in order (`None` deletes, delete+readd appends), and
`tools_added`/`tools_removed` change the tool set by value (a redefinition
is a removal followed by an addition; equal states record no redundant
change). Replay (`get_current_system_message`/`get_current_tools`) resolves
the current prompt and tools. Projection is native-or-collapse: transports
that accept mid-conversation system messages keep them in place, everything
else collapses to one replayed head — always a fresh value; the canonical
chronology is never rewritten. Explicit `TranscriptProjection::Strict`
rejects a transcript whose temporal placement the transport cannot honor
before any request is built. Declaration installation
(initial normalization, embedded system messages, deserialization and
`TranscriptContext::append_system_change`) validates names (non-empty,
unique per batch) and compiles input schemas under the private dialect
engine, failing with a typed error before the message enters the
transcript. `Request` carries `context: TranscriptContext` instead of
independent `system`/`tools` runtime fields; every adapter resolves the
transcript at the head of request construction (the pinned TypeScript
reference is `.repo/pi-1.3.0/packages/ai/src/utils/transcript.ts`; the
upstream-deprecated `hasToolRedefinitions` is not ported).

## Private Upstream Crypto Profile

A private `crypto` module (not part of the public API) owns key admission and
signing on the unmodified crates.io crypto baseline: `rustls` 0.23.40 with
the `aws-lc-rs` provider (`aws-lc-rs` 1.18.2 / `aws-lc-sys` 0.46.0), used
only through their public safe Rust seams (`KeyProvider::load_private_key`,
`SigningKey::algorithm`/`choose_scheme`, `Signer::sign`). It is substrate for
the TLS/mTLS host wiring and the real ADC callers; there is no local FFI,
crypto fork, private crypto namespace, or public crypto API.

The admission profile accepts unencrypted two-prime RSA-2048..8192 keys
(PKCS#1/PKCS#8, PEM/DER, including e=3) and P-521 keys (PKCS#8/SEC1).
RSA-1024, multi-prime RSA, encrypted/malformed inputs, public-key-only
inputs, and algorithms outside the profile are typed, redacted rejections —
never false success, downgrade, or identity rediscovery. The AWS-LC provider
is instantiated explicitly per owner (never installed process-globally);
transitive `ring` from the reqwest rustls-tls graph coexists, and existing
HMAC/SHA/SigV4 signing is unchanged.

Native admission notes: `aws-lc-sys` builds the vendored AWS-LC C sources
into a static library (standard evidence uses `AWS_LC_SYS_USE_SYSTEM=0`;
system directory/bindings, version-skip, and target overrides are unreviewed
replacements). The build needs a C toolchain on each platform; native
process-once CPU initialization and OS entropy are used without global
provider/config/env/subscriber mutation or ambient credential discovery.
The baseline is non-FIPS; untested feature unions and platforms are not
declared supported. Synchronous native signing is not hard-preemptible:
cancellation is observed between operations, owned keys/resources are
retained until the blocking worker joins, and late results are suppressed
rather than published.

## Source-Scoped TLS (RDF-23)

`HttpClientBuilder::tls(TlsOptions)` is the one public entry for source-
scoped TLS configuration. `TlsOptions` carries the trust selection
(packaged pinned Node reference default CA bundle, the same bundle plus
explicit extra roots, explicit roots replacing it, or an explicit opt-in
to the host system root store), an optional owned mutual-TLS client
identity, and the H2-preferred / HTTP/1.1-only ALPN preference. Everything
is validated and frozen when the client is built — before any network I/O —
with typed redacted failures (`HttpClientBuildError::Tls` /
`TlsConfigError` / `TlsKeyError`); client-identity keys are admitted through
the private crypto owner above (RSA-1024, multi-prime RSA, encrypted or
malformed keys fail typed at admission).

The TLS profile itself is fixed, not caller-selectable: an explicit
instance AWS-LC provider with TLS 1.3 + 1.2, WebPKI chain/hostname/time
verification always on, and the declared key-exchange group order
`X25519MLKEM768, X25519, secp256r1, secp384r1` (P-521 key exchange is
deliberately omitted; a P-521-only peer fails the handshake with zero
business HTTP). There is no backend selector, generic crypto trait, raw
verifier, bypass, or test-only signing API, and no process-global provider
or configuration is installed — the instance path works unchanged under a
host-preinstalled `ring` provider.

The pinned default trust bundle is the Node v22.19.0 runtime's compiled-in
CA set, packaged inside `opi-ai` and reproducible via
`scripts/extract-node-root-store.py` (150 certificates, bundle sha256
`12f4cd646968b72866b445b2850df3ac9e245031d15cdfa76d8ab51bde6f7c24`). It is
not webpki-roots, the OS store, or ambient discovery; host roots are an
explicit per-client opt-in only.

Packaged-consumer proof: `scripts/pi-sdk-conformance/package_slice.py
--family crypto --offline` packages `opi-ai`, extracts the artifact outside
the workspace, and runs an ordinary plus caller feature-union consumer
through this exact public path (trust modes, mTLS, ALPN, typed negatives,
host-provider independence), recording the resolved rustls/aws-lc
versions, registry checksums and native build provenance. Real ADC wiring
stays with its own owner; this closes only the transport mechanism.

## Source-Scoped HTTP Acquisition Profile (RDF-22)

`HttpClient` owns the source-qualified HTTP send policy over the unmodified
crates.io `reqwest 0.12.28` / `Hyper 1.9.0` graph. `HttpClientBuilder::redirects
(RedirectPolicy)` is the explicit redirect rule: `RedirectPolicy::Limited(n)`
(default 10, owned by `opi-ai` rather than silently tracking the backend)
bounds lawful redirect sends per logical request — one original request plus
at most `n` hops — and exceeding the bound fails acquisition with the typed
redacted cause `Network("too many redirects")` and no further sends;
`RedirectPolicy::None` returns the 3xx response to the caller as ordinary
provider status classification. Cross-origin hops (host or port change)
strip protected headers (`authorization`, `cookie`, `cookie2`,
`proxy-authorization`, `www-authenticate`) before the next send; same-origin
hops preserve them. Proxy use is explicit-only: ambient
`HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` environment detection by the backend is
disabled, and the scoped `no_proxy` list of an explicit `ProxyConfig` decides
bypassing; `https://` proxies are supported as outer TLS -> CONNECT ->
inner TLS to the target with no custom-transport double-wrap. There is no
retry, backoff or hidden-send behavior anywhere in the client: cancellation
stops the attempt, an accepted-then-truncated body produces exactly one
terminal stream error, and the server-observed send count equals the lawful
count.

The recorded acceptance range of the pinned upstream graph (exercised by
`tests/http_acceptance.rs` through the real `Models -> OpenAiChatProvider ->
HttpClient` loopback path, with bytewise and coalesced delivery): HTTP/1
response heads are parsed through an 8 KiB initial read buffer that grows
adaptively (no byte cap is enforced below the adaptive maximum; 300 KiB
heads are accepted), message heads are capped at 100 header fields, and
chunked bodies allow at most 16 KiB of cumulative chunk-extension bytes
(above that the body fails as a typed post-acceptance `StreamError`).
Trailer sections are accepted below, at and above the upstream internal
constants (no artificial trailer cap is enforced through this surface), and
no SDK-side body/event caps, allocation hints or loosened
content-length/transfer-encoding/EOF validation exist. Failures keep their
provable stage: pre-response problems surface as `Network`/`Timeout` at
acquisition, post-acceptance body problems as `StreamError` (wrapped with
partial assistant state when content was already delivered).

Packaged-consumer proof: `scripts/pi-sdk-conformance/package_slice.py
--family http --offline` packages `opi-ai`, extracts the artifact outside
the workspace, and runs an ordinary plus caller feature-union consumer
through this exact public path (redirect chains with exact send counts and
protected-header stripping, typed redirect-limit and acquisition failures,
proxy routing with scoped bypass, single-terminal EOF), recording the
resolved reqwest/hyper/hyper-util/http versions and registry checksums.

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
    AuthProvenanceSource, AuthScheme, CompatMetadata, Models,
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
    let mut models = Models::new();
    models.register_route(
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

    let prepared = models.prepare_call(model, request).await?;
    let mut stream = prepared.start_attempt()?;
    while let Some(event) = stream.next().await {
        println!("{:?}", event?);
    }
    Ok(())
}
```

## Modules

`provider`, `message`, `stream`, `model_info`, `api_mapped`, `registry`,
`models`, `provider_headers`, `auth`, `credential`, `http`, `retry`, `model`,
`anthropic`, `openai_chat`, `openai_responses`, `openai_codex_responses`,
`openrouter`, `mistral`, `gemini`, `bedrock`, `azure_openai`, `vertex`,
`config`, `time`, and `test_support`.

## License

MIT. See the workspace [LICENSE](../../LICENSE).

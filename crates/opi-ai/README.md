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
the reusable local AWS credential-source chain (scoped/ambient environment,
explicit SSO and login-session segments, shared-config INI overlay with
assume-role and web identity, `credential_process`, and the remote ECS/IMDS
segment, exchanging through the pinned STS Query/XML codecs, the SSO OIDC and
SignIn ES256 DPoP flows, and the single-flight source cache) lives in
`auth::aws`;
ADC sources and all nine OAuth flows are SDK-owned. The coding agent owns
the keychain backend, terminal presenter and host registration policy.

## Installation

```sh
cargo add opi-ai
```

The examples below additionally use `tokio` (features `macros` and
`rt-multi-thread`), `tokio-util`, `secrecy`, `futures-util`, and
`serde_json`.

## Quick Start

One complete ordinary call: register a route with injected auth, resolve the
caller-owned model record, normalize the transcript, and drive the neutral
entry to its single final message. This is the exact source of
`examples/quick_start.rs`; run it with
`ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start`.

```rust
//! Complete ordinary quick start: register the Anthropic route with injected
//! auth, resolve the caller-owned model record, and drive one chat call to
//! its single final message through the neutral entry.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start`.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::transcript;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SDK construction never reads product configuration: auth is injected
    // per route (AIR-021).
    let mut models = Models::new();
    models.register_route(
        Box::new(AnthropicProvider::new(None)),
        Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(std::env::var("ANTHROPIC_API_KEY")?),
        )),
        AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".into(),
        },
        Default::default(),
    )?;

    // Caller-owned record: provider id, bare model id, and the exact wire
    // from the registry's model metadata - valid without catalog membership.
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5-20250514")
        .ok_or("claude-sonnet-4-5-20250514 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Canonical transcript input (AIR-020): the system prompt and tool
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
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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

    // Neutral entry: construction freezes the owned call, authentication
    // starts on the first poll, and `complete_simple` drains to the single
    // final outcome (`Ok(AssistantMessage)` or one `Err(CallFailure)`).
    let message = models
        .complete_simple(record, request, SimpleOptions::default())
        .await
        .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;

    for content in &message.content {
        if let AssistantContent::Text { text, .. } = content {
            println!("{text}");
        }
    }
    Ok(())
}
```

Key contracts: SDK construction never reads product configuration - auth is
injected per route (AIR-021); `ChatModelRecord` is caller-owned and valid
without catalog membership (AIR-045); `normalize_context` folds the system
prompt and tool declarations into the canonical chronology (AIR-020);
`complete_simple` returns one `Ok(AssistantMessage)` or one
`Err(CallFailure)`.

## Streaming and the Final Result

`stream_simple` returns a lazy `ChatCall` - a
`Stream<Item = Result<AssistantStreamEvent, CallFailure>>` whose terminal
absorbs into permanent EOF. The `Done` event carries the final
`AssistantMessage`. This is the exact source of
`examples/quick_start_stream.rs`.

```rust
//! Streaming quick start: iterate the neutral entry's provider-neutral
//! events as they arrive, then read the terminal event's final message.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start_stream`.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.register_route(
        Box::new(AnthropicProvider::new(None)),
        Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(std::env::var("ANTHROPIC_API_KEY")?),
        )),
        AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".into(),
        },
        Default::default(),
    )?;

    let info = models
        .get_model("anthropic", "claude-sonnet-4-5-20250514")
        .ok_or("claude-sonnet-4-5-20250514 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

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
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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

    // `stream_simple` returns a lazy `ChatCall`: it is a
    // `Stream<Item = Result<AssistantStreamEvent, CallFailure>>` whose
    // terminal absorbs into permanent EOF. `Done` carries the final
    // `AssistantMessage`; one `Err` item is the single final failure.
    let mut call = models.stream_simple(record, request, SimpleOptions::default());
    while let Some(event) = call.next().await {
        match event.map_err(|failure| format!("call failed: {:?}", failure.cause()))? {
            AssistantStreamEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantStreamEvent::Done { message, .. } => {
                println!();
                eprintln!("-- stop reason: {:?}", message.stop_reason);
            }
            _ => {}
        }
    }
    Ok(())
}
```

`ChatCall::result(self)` (and `Models::complete_simple`) actively drains the
same stream to the single final outcome.

## Context, Tools, and Continuation

Tools are declared on `transcript::Context` and fold into the leading system
message; every continuation round replays the full canonical chronology with
assistant tool-call turns and their `ToolResultMessage` results. The caller
is the trusted execution boundary: a malformed final argument text is
recorded as not executed (`ToolNotExecutedReason::NonObjectArguments`),
never guessed into a call (AIR-048). This is the exact source of
`examples/tool_loop.rs`.

```rust
//! Agent-owned tool loop: declare a tool through the canonical transcript,
//! execute each admitted tool call, close it with a `ToolResultMessage`, and
//! continue until the assistant produces final text.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example tool_loop`.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolCall, ToolDef,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::transcript::{self, TranscriptContext};
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

const MODEL_SPEC: &str = "anthropic:claude-sonnet-4-5-20250514";

struct ToolOutcome {
    text: String,
    is_error: bool,
    not_executed: Option<ToolNotExecutedReason>,
}

/// The caller is the trusted execution boundary (AIR-048): a malformed final
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.register_route(
        Box::new(AnthropicProvider::new(None)),
        Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(std::env::var("ANTHROPIC_API_KEY")?),
        )),
        AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".into(),
        },
        Default::default(),
    )?;

    let info = models
        .get_model("anthropic", "claude-sonnet-4-5-20250514")
        .ok_or("claude-sonnet-4-5-20250514 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Tools are declared on the transcript context; normalization folds them
    // into the leading system message (AIR-020).
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

    let mut history: Vec<Message> = vec![Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "What is the weather in Tokyo? Use the tool.".into(),
        }],
        timestamp_ms: 0,
    })];

    // Continuation: every round replays the full canonical chronology
    // (assistant tool-call turns and their tool results included).
    for _round in 0..8 {
        let context = transcript::normalize_context(transcript::Context {
            system_prompt: Some("You are a concise assistant.".into()),
            messages: history.clone(),
            tools: tools.clone(),
        })?;
        let assistant = models
            .complete_simple(
                record.clone(),
                request_for(context),
                SimpleOptions::default(),
            )
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
            return Ok(());
        }

        history.push(Message::Assistant(assistant));
        for tool_call in tool_calls {
            let outcome = execute_weather_tool(&tool_call);
            history.push(Message::ToolResult(ToolResultMessage {
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
    }

    Err("tool loop did not converge".into())
}
```

## Factories, Catalog, and Auth

- Registration without hand-written routes: the bundled 42-profile factory
  (`providers::builtin_provider_profiles`, task 20.34) registers the
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

## Options, Errors, Cancellation, and Usage

- Neutral `SimpleOptions` overrides (tool choice, temperature, output limit,
  timeout, stop sequences) versus typed `NativeOptions<O>` per adapter - see
  "Native and simple options (ONB-03)".
- Failures: one typed `CallFailure` whose redacted `ProviderError` cause and
  taxonomy are described in "Core API" (`ProviderError`,
  `ProviderErrorCategory`, `ProviderErrorSummary`).
- Cancellation is `Request::cancel` (a `CancellationToken`, distinct from
  `Request::timeout`); token usage and cost live on `Usage` /
  `CumulativeUsage`.

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
| `bedrock` | `bedrock` | AWS Bedrock Converse streaming with SigV4 or Bearer authentication |
| `azure_openai` | `azure` | Azure OpenAI deployment-specific Chat **and** Responses wires (task 20.38) |
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
| `Models` / `AuthDescriptor` / `AuthCheck` | Unstable-0.x single driver above `ProviderRegistry`: provider+model lookup, redacted auth state, OpenAI-compatible compat metadata, dispatch, and atomic dynamic-catalog refresh. Native/simple/direct and advanced entries share validation and the call lifecycle; native options retain adapter budget semantics, while simple/advanced preparation applies the shared output budget. |
| `ChatApi` / `ApiId` / `ChatApiHandle` / `NativeOptions` | Typed custom chat adapter registration with its exact API identity and fixed options type; handle-created options carry the API/type tag and stay portable across instances with compatible registrations. |
| `ToolCall::arguments_error` | Optional final-parse outcome for textual tool arguments (AIR-048): empty input finalizes to `{}`, repairable invalid escapes stay literal, and incomplete/non-object finals keep the owned raw text plus this error, never a partial-prefix success. Historical records without the field keep deserializing. |
| `ChatModelRecord` / `SimpleOptions` / `ChatCall` | Complete caller-owned chat record (provider id, bare model id, exact wire) usable without catalog membership; neutral simple options; the lazy ordinary-call owner that captures input at construction and starts auth on first poll. |
| `PreparedProviderCall` / `PreparedRoute` | Opaque logical-call state returned by `Models::prepare_call`, plus its public redacted route facts. `route`, `auth_provenance`, and `start_attempt` are the prepared-call lifecycle APIs. |
| `CredentialStore` / `Credential` / `CredentialSource` | IO-free, object-safe credential persistence and redacted three-state probe contracts. |
| `OAuthProvider` / `OAuthCredential` / `LoginPresenter` | Boxed-future contracts for the nine SDK OAuth families, read-only flow metadata, provider-specific request identity conversion, and injected login UX. |
| `AuthResolver` / `ResolvedAuth` | Driver-owned per-call auth resolution contract; one prepared result is frozen before an attempt starts. |
| `ApiKind` | Crate-root enum tagging the backend family (`Anthropic`, `OpenAi`, `Google`, `Mistral`) carried on assistant messages. |
| `HttpClient` | Shared `reqwest` client with pooling and explicit/env proxy support. |
| `ws` | Public strict dynamic WebSocket Upgrade bridge (`upgrade`, `WsUpgradeRequest`, `Negotiation`, `fresh_nonce`/`fresh_mask`): request headers, nonce/mask entropy, and strict 101 validation. Frame/PMD/Close/IO modules are private; Upgrade returns a raw upgraded stream and negotiation metadata. |
| `retry` | The acquisition-retry owner (`AcquisitionPolicy`, `RetryAdmission`, `AcquisitionSeal`) plus the product-facing retry config, exponential backoff, and `Retry-After` parsing. |
| `Usage` / `CumulativeUsage` | Token accumulation and cost helpers. |
| `test_support::MockProvider` | Deterministic mock provider for downstream tests. |

## Credentials, Request Enrichment, and Refresh

`CredentialStore` keeps backend errors distinct from missing entries, while
`CredentialSource` reports present, absent, or backend-unavailable state
without exposing a secret. `AuthDescriptor::StoreCredential` carries only a
key and display source. `OAuthProvider`, `LoginPresenter`, and `AuthResolver`
are object-safe boxed-future seams. `auth::oauth` supplies Anthropic, Codex
legacy, ChatGPT direct, GitHub Copilot, OpenRouter, Kimi Coding, Meta, xAI and
Radius flows. `opi-coding-agent` supplies the OS-keychain store and terminal
presenter; SDK flows use the injected presenter and never launch a browser.

`WireApi` gives every `ModelInfo` one exact request wire, while public
`ApiMappedProvider` exposes one provider identity and catalog and validates
its `WireApi -> Provider` routes before dispatch. One mapped provider shares
one lazy `AuthResolver` across all routes; provider/model metadata chooses the
route before network IO. Unknown models, missing routes, and wire/compatibility
mismatches are typed non-retryable failures.

### Lazy AuthContext and management results (AIR-063/AIR-066)

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
errors (probe snapshots included), never as silent `Ok(None)`. `get_available`
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
Attempts are sequential: an overlapping stream returns
`ModelsError::AttemptAlreadyActive`, cancellation returns `CallCancelled`,
and a prior credential-needed/revoked attempt closes the prepared call with
`CredentialTerminated`. Missing and revoked credentials surface as explicit, non-retryable
`ProviderError::CredentialNeeded` and `ProviderError::CredentialRevoked`
variants. Typed per-call API-key, Bearer and SigV4 overrides are mutually
exclusive and win before source access; `extra_headers` rejects provider-managed
auth headers.

### Store-owned mutations, ACKs, and the mutation barrier (AIR-063 / P20-STORE)

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

### Acquisition retry owner (AIR-007)

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
implicit and explicit ceilings are then reduced to fit estimated remaining
context with 4096 safety tokens and a minimum output of one token. Thinking that
no longer fits is reduced to leave 1024 answer tokens where possible. The
non-adaptive Anthropic wire resolves an omitted thinking budget from its level
before wire projection; a zero native budget uses the source default of 1024,
while nonzero native budgets pass through. Adaptive effort does not use a numeric minimum. The
`context` module estimates the original canonical message chronology, including
later System changes and tool declarations; the last
applicable positive successful usage (non-error, non-aborted, timestamp not
older than its prefix, any model identity) anchors the estimate and only
messages after the anchor are estimated, with no same-model or full-text
lower bound. Every attempt reuses the resolved limit. Typed native `ChatApi`
calls still validate route and capabilities, but preserve native omission and
explicit limits instead of applying the simple budget default or clamp. Direct
provider adapter calls retain their wire defaults. These are per-request output
limits, independent of any product turn-count budget.

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

`CompatConfig` (task 20.36, AIR-022/V-11) applies the single normalization
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
| `strict_tool_schema` | Emit strict JSON-schema tool definitions. |
| `cache_key` | Send the provider's prompt-cache key (cache-affinity hint). |
| `send_session_affinity_headers` | Map a request `session_id` to the compatible session-affinity headers (inferred for OpenRouter). |
| `require_assistant_after_tool_result` | Insert the fixed assistant bridge between a tool result and the next user message; assistant tool-call turns use `content: ""` instead of `null`. |
| `requires_thinking_as_text` | Replay assistant thinking as plain text parts. |
| `requires_reasoning_content_on_assistant_messages` | Mark replayed assistant turns with an empty `reasoning_content` (inferred for DeepSeek). |
| `zai_tool_stream` / `vllm_priority` | Explicit-only `tool_stream` and `priority` extras; never inferred. |
| `thinking_token_budget_field` / `supports_thinking_token_budget` | Top-level reasoning budget field (`thinking_token_budget` vLLM, `thinking_budget` Qwen/DashScope/SGLang, `thinking_budget_tokens` llama.cpp; the alias selects vLLM's). |
| `chat_template_kwargs` / `chat_template_args` | `$var` template values (`thinking.enabled`, `thinking.effort`, `thinking.budget`; `omit_when_off` drops the key when no effort is set). |
| `chat_completions_path` | A non-default path overrides the suffix. An explicit model API base uses `/chat/completions`; a constructor-only root uses `/v1/chat/completions`. |

The retired keys `system_role_override`, `supports_store`,
`supports_developer_role` and the static `reasoning_effort` string fail with
explicit migration errors at every config position (catalog `x-opi.compat`,
product TOML profile and model compat) — they are never silently ignored.
Request thinking intent and the selected `ModelInfo::thinking_level_map` are
the only sources of reasoning wire output.

Tool results project per the pinned source: text (or the `(see attached
image)` / `(no tool output)` placeholders) rides the `role: "tool"` message,
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

### Native and simple options (ONB-03)

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

OpenAI Responses native semantics (task 20.37, `OpenAiResponsesOptions`):

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

### Azure OpenAI dual wires (task 20.38)

Azure keeps both wires with explicit deployment/resource/api-version
mapping (pinned source `azure-openai-config.ts`):

| Mapping | Precedence | Wire effect |
|---|---|---|
| Base URL | explicit `azure_base_url` → selected base configuration → resource name → model `baseUrl` → typed `Config` failure | Source Chat/Responses use `/openai/v1` for a standard Azure resource host. Custom API bases keep their path; Responses appends `/responses` only if absent. |
| API version | explicit `azure_api_version` → selected environment configuration → `v1` | Responses sends `?api-version=…`. Builtin Chat uses the OpenAI Chat path without this query; legacy direct Chat retains its versioned deployment URL. |
| Deployment | explicit `azure_deployment_name` → provider `modelId=deployment` map → model id | Source wires send the deployment in body `model`, preserving catalog identity. Only legacy direct Chat also puts it in its URL. |

The Responses wire (`WireApi::AzureOpenAiResponses`, `azure-openai-responses`
records, public `AzureOpenAIResponsesChatApi` registered through
`Models::register_chat_api`) reuses the exact Responses parameter assembly —
`store: false`, off-effort `{"effort":"none"}`, summary default `auto`,
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
source-qualified `HttpClient` with its explicit proxy/TLS profile. Classic
Upgrade uses the client's private HTTP/1.1 transport with redirects disabled,
retaining the same frozen trust, mTLS identity and proxy configuration without
source rediscovery. Ordinary requests retain their configured redirect policy
and H2/H1 behavior. The bridge owns the
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

The protocol state machine lives in private `src/ws/frame.rs`, `pmd.rs` and
`io.rs` modules inside this SDK. It has no public codec API, test-only export
or Cargo feature, and no independent package/version/release identity.
The public Upgrade result remains a raw `reqwest::Upgraded` stream plus
negotiation metadata; Upgrade does not construct the private codec.
`Negotiation::driver_parameters` returns `Option<NegotiatedPmd>` retaining
both directions' context-takeover flags and windows. The private codec has
no production caller yet. Physical consolidation alone does not establish
Models → real Codex → Upgrade → codec behavior (20.16), continuation/pool/
cleanup (20.17), or final assembled SDK package acceptance (20.57); those
proofs now exist. Native platform/MSRV acceptance (20.58) executed on Linux
x86_64 (rustc exactly 1.97.1, the declared MSRV) and Windows
x86_64-pc-windows-msvc with byte-identical packaged file sets; macOS remains
an explicit CI claim pending an owning host.

### Private protocol ownership

| Module | Contract |
|---|---|
| `ws::frame` | Strict client frame decoding (reserved opcodes/RSV, fragmentation, control-frame shape, shortest length encodings, 64-bit high bit) and message assembly with actual-received-byte, typed fallible storage growth. |
| `ws::pmd` | Owned permessage-deflate: pre-tail `SyncPoint` proof, exactly-once `00 00 FF FF` tail injection, `Z_BLOCK`/`BFINAL`/`ResetKeep` handling, context takeover and bounded per-poll work; the encoder is an ordinary safe `flate2` client. |
| `ws::io` | Exclusive connection owner with owned pending read/write/decode jobs, explicit Close, caller-supplied masks, backpressure and terminal failure absorption. |

One private `Driver` owns all connection state. Borrowed `next()` futures
return complete messages (including Ping/Pong/Close); dropping a borrow
preserves partial frame/PMD/UTF-8 state so the same owned operation can
resume. Owner cancellation/abort/Drop instead releases resources and
suppresses late output; an idle owner must register a real cancellation
wakeup. A cancelled borrow does not authorize returning a connection to a
pool. The protocol layer does not discover auth, create timers or mask
entropy, reconnect, retry, or make provider/resource-policy decisions.

Directional PMD parameters configure reception from server values and
sending from client values. `None` disables PMD; absent RFC window
parameters map to 15. The pinned encoder normalizes requested window 8 to
9 internally; tests independently verify actual wire distances against the
negotiated 256-byte window rather than treating configuration digits as
proof.

Explicit Close requires both frames, write shutdown and peer transport EOF.
It preserves partial application writes, distinguishes bytes written from
pending flush, retains bounded in-flight events and uses only the caller's
absolute `tokio::time::Instant` deadline. Abnormal EOF/protocol/Close-timeout
errors occur once, discard retained receive events and then leave permanent
receive EOF; successful Close still permits delivery of its retained event.
Storage grows from received bytes with typed `try_reserve*` failures, never
from speculative declared-payload allocation. Internal IO/inflate/UTF-8/
continuation work quanta provide cooperative yielding, not message-size or
iteration limits. Allocation, single backend calls and synchronous outbound
encoding are not hard-preemptible.

### Compression and maintenance boundaries

The unmodified graph is `libz-rs-sys = 0.6.8` (defaults off, `std` +
`rust-allocator`), resolved `zlib-rs 0.6.8`, and `flate2 = 1.1.10` (defaults
off, `zlib-rs`). The decoder uses doc(hidden) public `inflateSyncPoint` /
`inflateResetKeep` exports and the transitive zlib-rs `__internal-api`.
The narrow local unsafe boundary is documented in `src/ws/pmd.rs`: a unique
boxed `z_stream`, no clone/custom allocator/callback/raw-handle escape,
pointer rebinding around calls, exactly-once `inflateEnd`, and deliberate
`Send` without `Sync`. It neither adds local extern declarations nor
selects dynamic system zlib. See [NOTICE](NOTICE) for compression licenses
and source/feature/native audit obligations.

| Caller graph | Encoder backend | Decoder backend |
|---|---|---|
| ordinary | flate2 `zlib-rs` | direct `libz-rs-sys` Rust path |
| `flate2/default` | flate2 `zlib-rs` with default features unified | direct `libz-rs-sys` Rust path |
| `flate2/zlib` | flate2 `zlib` (native C `libz-sys` build) | direct `libz-rs-sys` Rust path |

flate2 1.1.10 prioritizes C backends, then zlib-rs, then miniz_oxide. The
default union adds miniz_oxide without selecting its encoder. C-zlib native
source/build/link inputs require separate review. Allocator conflicts,
gz/gzprintf, nightly, export-symbols/prefix and other unreviewed unions do
not inherit support. Reopen review for hidden-API/unsafe/feature/security
changes, a shrinking accepted-input set or a new production consumer.
Dependency upgrades require actual resolved source/checksum/features plus
the three-graph corpus, Send/Close/cancellation/fairness and package checks.

### Local protocol checks

Private protocol tests live under `src/ws/tests`, with the original
fixtures under `src/ws/tests/fixtures`. The boundary replay regenerates the fixed
72,413-case PMD corpus and checks recorded classifications (SHA-256 for
outputs longer than 64 bytes); extraction provenance and oracle identities
remain with those fixtures. Run the same tests in all three graphs:

```sh
cargo test -p opi-ai --lib ws::
cargo test -p opi-ai --lib ws:: --features flate2/default
cargo test -p opi-ai --lib ws:: --features flate2/zlib
```

Public Upgrade, negotiation and WSS transport contracts live in
`tests/ws_upgrade.rs`, using the same public `HttpClient`/Upgrade interface as
callers. Run `cargo test -p opi-ai --test ws_upgrade`; the packaged SDK checks
run this integration target alongside the private tests in all three graphs.
Both layers share only a scripted peer helper under `tests/support`; WSS and
TLS integration tests use the same synthetic TLS fixtures.

Private loopback tests combine public Upgrade/`HttpClient` with the codec,
including two-direction window-8 negotiation, multiple messages with/without
context takeover, an independent RFC 1951 decoder and a distance-257 negative
control. They test wiring under `cfg(test)`; they do not prove a production
Codex path or an external public codec. The idle-cancellation test waits for
a real Pending read waker, cancels in the owner and observes transport/PMD
release before acknowledgement, without adding a production cancellation API.

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
no-I/O contract. Full schema conformance closure stays with the
P20-GAP-SCHEMA scenario and its recorded mechanism owners; package/platform
acceptance evidence lives in
[phase20-delivery-evidence](../../docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md)
(Linux and Windows executed, macOS pending an owning host).

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
all of the turn's tool results inside one user message, task 20.39);
`details` never serialize and URL sources pass
through undownloaded. `ReplayProjection::Strict` rejects the same lossy
transforms with a typed error instead of recording facts. Provider
adapters consume the projection at the head of request construction
(`build_request_body` now returns `Result`); the Responses parser packs
`call_id|fc_item_id` for same-identity dual-ID round-trip and the
serializer restores both, dropping the item id for foreign history. Pinned
reference: `.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts`.
Loopback regressions capture real Anthropic and Codex terminal output,
serialize/reload it, then send same-identity, changed-model and foreign-wire
requests while keeping canonical signed/redacted or encrypted data intact.
These checks complement, and do not replace, the final SDK package and
platform matrix; the matrix's executed evidence is recorded in
[phase20-delivery-evidence](../../docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md).

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

Clients without an explicit TLS override use these same default options:
pinned Node roots, no identity, H2-preferred ALPN and the fixed instance
provider. They never inherit a process-global provider or backend default roots.

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
Trailer fixtures wait for HTTP EOF: 99 fields and 16,383 cumulative trailer
bytes are accepted, while 100 fields or 16,384 bytes and larger fixtures fail
with a typed post-acceptance `StreamError`. Malformed and truncated trailers
also fail instead of being hidden behind an earlier SSE terminal. These are
upstream parser boundaries; no SDK-side body/event caps, allocation hints or loosened
content-length/transfer-encoding/EOF validation exist. Failures keep their
provable stage: pre-response problems surface as `Network`/`Timeout` at
acquisition; post-acceptance body problems retain `StreamError` or the original
`Timeout` cause with canonical partial state and known usage. The migrated
Chat path seals acquisition even when an accepted response fails before its
first public event.

Packaged-consumer proof: `scripts/pi-sdk-conformance/package_slice.py
--family http --offline` packages `opi-ai`, extracts the artifact outside
the workspace, and runs an ordinary plus caller feature-union consumer
through this exact public path (redirect chains with exact send counts and
protected-header stripping, typed redirect-limit and acquisition failures,
proxy routing with scoped bypass, single-terminal EOF), recording the
resolved reqwest/hyper/hyper-util/http versions and registry checksums.

## Offline Bundled Model Catalog (DEL-10)

The `catalog` module embeds one authentic, versioned model catalog as offline
crate resources: `resources/catalog/builtin/catalog.json` is byte-for-byte
the artifact published at the exact revision pinned by the local Pi tree
(`.repo/pi-1.3.0/nix/model-catalog.json`), and `manifest.json` records the
delivery-input provenance — source URL and revision, artifact SHA-256,
Pi package/local-origin association, data schema version 6, bundle schema
version, the full provider list with per-operation entry counts, and the
digests of Pi's own validator/hydrator plus the opi packer. The bundle is
produced by the explicit delivery-input tool `scripts/pi-catalog/pack.py`
(verify with `--check`); nothing ever synthesizes catalog data from tests,
defaults, or a generator run, and a missing authentic artifact is a blocking
input rather than a reconstruction trigger.

`Catalog::builtin()` parses and validates the embedded bytes once per
process — no network I/O, account probing, or generator execution —
re-verifying the artifact digest, bundle/data schema and minimum-reader versions, provider set,
every entry's data contract (identity, modalities, capabilities, cost),
`type:id` uniqueness per provider, and the structure digest. Any mismatch
is a typed, fail-closed `CatalogError`; no partial catalog exists.
Operation-qualified getters (`models(provider, operation)`,
`model(provider, operation, id)`) expose chat/image/classifier entries with
typed core fields plus the complete raw entry, ready for the store/refresh
and profile/catalog integration owners. `CatalogHandle` owns the active
revision: `try_install` validates a candidate completely and swaps
atomically, so tampered, stale, wrong-version, missing-provider, duplicate,
or contract-violating candidates are rejected while the last-known revision
stays readable, and already-held `Arc<Catalog>` values are never rebound.
Explicit complete custom records (`ChatModelRecord`) remain usable
independently of any catalog state. Image entries are catalog metadata for
selection/pricing only; the SDK's own image operation paths are owned by the
typed image API task.

## Dynamic Catalog Store and Refresh (V-27/AIR-002/AIR-033)


`ModelsStore::invalidate_source(id)` explicitly withdraws the records published
by that source and persists a tombstone. Old guards cannot publish afterward;
`refresh(false)` observes subsequent revisions of the attached shared store
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

`Models::refresh(allow_network)` is the only refresh entry: `refresh(false)`
performs no OAuth/network work and restores the store cache
(`RefreshStatus::OfflineRestored`); `refresh(true)` fetches registered
providers in deterministic sorted order, then registered catalog sources in
registration order. Outcomes are per-provider: successes publish, failures
retain that provider's prior data and report a redacted error
(`RefreshOutcome::{Published,Failed,Restored}` in `RefreshReport`) without
erasing other providers' catalogs. Without explicit catalog sources, a native
provider's `Ok(None)` restores its builtin catalog. With an explicit source,
that absence of a native publication retains the last-known source catalog;
an authoritative `Ok(Some([]))` still replaces it with an empty catalog.
`refresh_with` observes a `CancellationToken` — cancellation observed before
commit retains the prior publication, including a source returning success in
the poll that observes cancellation. Without a store, the final token check
starts the synchronous memory commit. Already-prepared
calls keep their frozen bindings across refreshes. `DynamicCatalogSource`
is the public seam for host-plugged catalog sources; `HttpCatalogSource`
fetches one distribution-schema-v1 document over HTTP with pooled
connections and redacted failures. The SDK registers no source, attaches
no store, and has no pi.dev/default startup fetch policy.

## Telemetry Carrier: Frozen Parent Span plus Dispatch (task 20.54, DEL-12)

`telemetry` provides the process-local `TelemetryContext` carrier - the
P20-TELEMETRY shared-decision owner; the closure test lives at
`tests/telemetry_carrier.rs`.

- **`TelemetryContext::from_span(span)`** freezes the parent
  `tracing::Span` together with its own `Dispatch`, obtained through
  `Span::with_subscriber`. The source span determines the pair even when
  another subscriber is current at creation or poll time. A
  disabled span degrades to the explicit `noop()` state. `Clone`
  retains the same pair; the context is `Send + Sync`.
- **`in_scope(FnOnce)`** runs the synchronous body exactly once under
  the frozen pair (dispatch installed as the section default, parent
  entered for the section only) and restores the caller context before
  returning.
- **`instrument(Future) -> Instrumented<F>`** is a lazy `Unpin` wrapper
  (boxed inner future, zero `unsafe`) whose every poll and whose drop -
  including an unpolled drop - run the same scoped section. No enter
  guard crosses an await; each outer poll polls the wrapped future
  exactly once; `Pending` and the output are preserved.
- **`noop()`** runs the same sections under a no-op dispatcher:
  business work still executes exactly once while section observation
  is explicitly silenced instead of leaking to ambient dispatch. Independent
  host logging outside the wrapped section keeps its own subscriber.

`Request::telemetry_context: Option<TelemetryContext>` explicitly carries this
pair into the real Codex producer, including its async callbacks and SSE
fallback. Omission captures no implicit ambient context. No carrier is
serialized or placed on the wire, and no global subscriber, exporter, sampling
policy or automatic SDK root span is installed. Propagation is closed across
all SDK operations (task 20.55): the four chat entries (native/simple/direct
and the advanced `prepare_call` path including acquisition retry and the
strict-variant rebuild), image generation, classifier calls and deferred
submit/fetch/cancel observe the same frozen pair through their producers and
scoped poll/drop sections, with default/`noop` carriers staying passive and
business results unchanged. The assembled all-operation conformance suite
lives at `tests/telemetry_operations.rs`; the Codex producer path is
additionally covered by `tests/codex_telemetry.rs`.

## Instance Resource Generation, Pool Cleanup and Awaited Release (task 20.17)

The Codex WebSocket family now registers one resource generation per call
and exposes session-scoped cleanup through the public
`Models::cleanup_session_resources` entry (`codex_resources` module;
tests at `tests/openai_codex_responses.rs` and `tests/resource_cleanup.rs`).

- **Registration:** the call's first poll atomically registers its generation
  before any auth, transform or network await. Pure `Sse`-preference calls stay
  unregistered. The original complete session id remains the cleanup identity
  even with `Disabled` cache retention. Phases (`prepared`/`connecting`/`busy`/
  `temporary`/`active-fallback`) include the same call's SSE fallback.
- **Cutoff semantics:** the returned cleanup Future owns its selected owners
  and session and establishes all cutoffs on its **first poll**. This differs
  from the credential mutation barrier's synchronous construction cutoff.
  `Some(nonempty id)` selects matching old generations and pooled connections;
  `None` selects everything, including sessionless work; an unmatched id is a
  no-op retaining idle reuse; `Some("")` is a typed zero-effect failure. Late
  attachment, publication, reconnect and entry into SSE are forbidden. Calls
  first polled after the cutoff get a new generation; an already registered
  `PreparedProviderCall` keeps its old generation even for an unpolled attempt.
- **Release:** cleanup awaits library-owned producers, connections and bounded
  closes. A retained, dormant caller preparation/auth Future needs no further
  poll for cleanup to finish; its next poll sees latched cancellation. Dropping
  a polled cleanup Future does not undo its close marks, and another cleanup can
  await the remaining exits. Drop initiates release without claiming an await
  barrier. Removed connections send Close and flush before bounded confirmation;
  one failure still allows the other releases and returns
  `CleanupError::Incomplete { report }` with confirmed counts and unconfirmed
  effects. A delivered terminal stays final while protocol release continues.
- **Isolation:** `SessionResourceOwner::new_namespace` gives each `Models`
  registration an independent namespace, including when two instances wrap the
  same provider/client and full session id. Caller parent tokens, credentials,
  catalogs and independent store-owned rotations remain untouched. Owned call
  guards retain their resources across `Models` drop, removal or replacement;
  successful calls may retain an idle cached connection.
- **Dispatch:** resource-owning providers expose `Provider::session_resources`
  and forward the registered lease through `stream_resource_attempt`.
  `Provider::stream_model_resource_attempt` also carries the caller's frozen
  complete record and simple options; its default forwards resource and attempt
  ownership. Typed `ChatApi` registrations expose `session_resources` and
  `stream_resource_prepared`, with `stream_model_resource_prepared` for the
  selected record. The default `validate_native_request` and `simple_options_for`
  hooks preserve existing custom adapters; metadata-aware adapters override
  them to validate native fields and map consumed neutral options. Resource-less
  defaults remain inert.
  `validate_simple_options` rejects explicit neutral options by default;
  adapters admit an option only when their simple mapper consumes it.
  `Models` attempts every owner's release and aggregates the results.

## Real Codex WebSocket Transport: Auto/Delta/Continuation (task 20.16, DEL-07)

The Codex adapter now owns the real WebSocket production path through the
strict Upgrade bridge into the private frame/PMD/Close owner
(`openai_codex_responses/ws_transport.rs`; tests at
`tests/openai_codex_responses.rs`).

`openai_codex_responses::CodexChatApi::new(Arc<OpenAiCodexResponsesProvider>,
ModelInfo) -> Result<Self, ProviderError>` registers the real adapter through
`ChatApi<CodexOptions>`. Pass the adapter, resolver and auth source to
`Models::register_chat_api`, create tagged options with `handle.options(opts)`,
then call `Models::stream(record, request, options)`. Construct the
non-exhaustive `CodexOptions` with `Default::default()` and set its public fields.
Native defaults preserve the native reasoning omission/off mapping; neutral
simple calls map `Request::thinking` instead. Both entries consume the frozen
complete `ChatModelRecord` selected for the call, including its thinking map,
rather than the constructor's exemplar or a same-ID catalog entry. Native Codex
does not consume neutral `Request::thinking`, including unsupported neutral
levels; common structural and selected-model validation still runs before auth.
Options are frozen per call and
do not mutate the registered provider or use `sampling_params` as a carrier.

| Input | Type / unit | Omission, zero or null | Precedence and wire field |
|---|---|---|---|
| `reasoning_effort` | `CodexReasoningEffort` enum | Omitted uses the supported model's off mapping; `Some(None)` explicitly requests off | Native effort uses the model's mapping or native spelling, independently of neutral thinking; `reasoning.effort` |
| `reasoning_summary` | `CodexReasoningSummary` enum | Omitted or `Null` becomes `auto` when an effort is serialized; otherwise unused | Native summary; `reasoning.summary` |
| `service_tier` | `CodexServiceTier` enum | Omitted stays absent; `Some(Null)` sends JSON null | Explicit native value; `service_tier` |
| `text_verbosity` | `CodexTextVerbosity` enum | Omitted → `low` | Explicit native value overrides the default; `text.verbosity` |
| `tool_choice` | `CodexToolChoice` enum | Omitted → `auto`; enum `None` sends `"none"` | Explicit native value overrides the default; `tool_choice` |
| `websocket_connect_timeout` | optional `Duration` | Omitted → 15 seconds; zero disables the connect timer | Frozen local connection budget; independent of `Request::timeout`, never serialized |
| `SimpleOptions::tool_choice` | `Option<SimpleToolChoice>` (`Auto` / `None`) | Omitted → Codex `auto` | Neutral Codex mapping to `tool_choice`; currently unsupported routes reject an explicit value before auth |
| `Request::temperature` | optional unitless number | Omitted or non-finite → absent; zero is valid | Request value; `temperature` |
| `Request::max_tokens` | optional token count | Codex omits an output-limit wire field, including explicit zero | Simple preparation retains its shared budget semantics; native calls preserve the request before wire mapping |

Closed native enums reject invalid serialized spellings. Invalid complete
records or wrong wires fail with typed configuration errors; registration and
options identity/type mismatches fail before auth or network work. Common
transport, cache, cancellation and timeout inputs remain on `Request`.

- **Transport resolution:** `Request::transport`
  (`TransportPreference::{Auto, Sse, Websocket, WebsocketCached}`, default
  `Auto`) resolves first; omitted and explicit `Auto` follow the same delta
  rules. The dedicated HTTP/1.1 Upgrade client disables redirects and retains
  the `HttpClient`'s frozen TLS identity, trust and proxy inputs. Ordinary HTTP
  keeps its configured redirect policy; that policy does not force `Auto` to SSE.
- **Real chain:** `response.create` travels as a masked text frame; events
  decode through the exact SSE classifier, so both transports publish
  identical public events; explicit client Close completes temporary
  connections; EOF without a terminal is never success; `Request::timeout`
  is the WebSocket idle timeout (zero disables its timer); cancellation is active
  and idle. The next acquire evicts connections idle for five minutes after
  release or aged 55 minutes, dropping their continuation; busy calls continue.
- **Delta/continuation** (Auto/WebsocketCached with a session):
  `previous_response_id` + delta input are sent only when the non-input
  fields are unchanged and the previous full input plus the actual response
  items form an exact prefix of the new input; otherwise the full input.
  Continuation publishes only after a successful terminal with a response
  id. Sessionless or `Disabled` cache retention uses temporary connections.
- **Bounded branches:** exactly one full-input retry on
  `previous_response_not_found` before public output; exactly one reconnect
  on `websocket_connection_limit_reached` before generation starts; the
  WS->SSE fallback fires only for transport-class failures before the first
  effective provider event and is sticky per session. Abort/API/auth/
  protocol failures never masquerade as transport loss; an output-delivered
  error ends that stream with the partial preserved. A valid nonempty provider
  event type seals fallback/reconnect even when the mapper emits no public event.
  Legal SSE fallback retains a `codex_websocket_fallback` diagnostic alongside
  existing sampling diagnostics, with a fixed reason and no raw error text.
- **Upgrade bridge:** `WsUpgradeRequest::headers` adds caller handshake
  headers (handshake-managed names are reserved and rejected).
- **Packaging:** the `ws` conformance family now includes a real-chain
  consumer check (external `Models::stream_simple` -> Codex -> Upgrade ->
  private codec against a local scripted peer) in all three feature graphs.

## Scripted Multi-Operation MockProvider Support (task 20.53.8, DEL-11)

`test_support` extends the existing `MockProvider` chat scripts with the
three shapes pinned to `providers/faux.ts` - no second mock driver, no
public API change:

- **`MockResponse::DelayedAfter { pending_attempts, retry_after_ms,
  events }`** (pinned `pendingFetches`): the first `pending_attempts`
  prepared attempts each fail with `ProviderError::RateLimited`
  carrying `retry_after_ms` (the chat-level analog of the pinned
  `pollAfterMs` hint, reusing the existing retry mechanism instead of
  timers); the next attempt yields the events.
  `pending_attempts: 0` behaves exactly like `Events`. One variant value
  spans those attempts, mirroring the pinned per-entry countdown.
- **`MockResponse::HangingUntilCancelled`**: yields no events until the
  request's cancellation token fires, then terminates with
  `ProviderError::Cancelled`; a pre-cancelled request fails immediately.
  The attempt-boundary form of the pinned signal-aborted-to-error
  behavior, built with `futures_util::stream::once` over
  `CancellationToken::cancelled` - no timers, no new dependency.
- **`blocks_response(Vec<AssistantContent>)`**: the ordered
  multi-block/tool fixture - `TextDelta` for text blocks and
  `ToolCallEnd` for tool calls, `content_index` matching block position,
  `Done` with `StopReason::ToolUse` when any tool call is present else
  `Stop`. Each partial contains only the emitted block prefix; earlier snapshots
  remain independent. Panics for `Thinking`/`RedactedThinking` blocks by contract.
  Existing `Events`/`Error`/`EventsThenError` semantics and the
  `stream_prepared` queue-assertion are unchanged; deferred records and
  their scripts stay with 20.45-50.

## Public Header Merge, Scoped Env, and Cancellable Waits (P20-S53-6, task 20.53.6)

`provider_headers::merge_provider_header_sources`, the new
`provider_env` module, and the new `cancel` module complete the pinned
`utils/headers.ts`, `utils/provider-env.ts`, `utils/sleep.ts`, and
`utils/abort.ts` families:

- **`merge_provider_header_sources(&[&[(String, Option<String>)]])
  -> Option<Vec<(String, String)>>`**: multi-source case-insensitive
  merge. Later sources win; a winner re-inserts at the end preserving
  its original case (the pinned Map delete+set ordering); `None`
  deletes the name without inserting; an empty result is `None`. This
  is unvalidated config normalization feeding
  `ProviderHeaders::try_new`; `merge_request` keeps its fail-closed
  override rejection.
- **`provider_env::provider_env_value(name, scoped)`**: scoped
  injected values precede the ambient `std::env::var` lookup, faithful
  to the pinned `||` chain - an empty scoped value falls back to
  ambient and an empty ambient value yields `None`. Lazy per-name
  reads only: no construction-time access, no global caching or
  mutation, no proxy/identity policy layer (owners stay `http` and
  `auth`), and no Bun `/proc/self/environ` fallback.
- **`cancel::{operation_token, sleep_cancel, race_with_cancel,
  Cancelled}`**: cancellable waits reusing `CancellationToken`.
  `operation_token(Some(&parent))` returns a one-way child token
  (parent cancellation reaches the child, never the reverse);
  `sleep_cancel(ms, &token)` arms no timer when pre-cancelled and
  releases the timer on cancel/ready/drop; `race_with_cancel(future,
  &token)` returns the operation output or `Cancelled`, failing
  immediately on a pre-cancelled token without polling the operation,
  and drops the abandoned operation future on cancel (the Rust-native
  abandoned-promise handling). Tests use the paused Tokio clock and
  drop sentinels for determinism. No new dependency; no executor or
  global timer authority.

## Public shortHash and UUIDv7 Identity Utilities (P20-S53-4, task 20.53.4)

`opi_ai::identity::{short_hash, UuidV7Generator, UuidV7Error}` complete the
pinned `utils/hash.ts` and `utils/uuid.ts` families:

- **`short_hash(&str) -> String`** (pinned `shortHash`): exact algorithm
  port for compat identifiers (session seeds, `fc_` item ids) - UTF-16
  code units (surrogate pairs contribute two units), 32-bit wrapping
  multiplies, the sequential final mix (each stream shifts itself by 16
  and the other by 13, the second mix using the already-updated `h1`),
  and unsigned base36 output `h2` then `h1`, locked by fixtures computed
  with strict `ToInt32`/`ToUint32` discipline. Documented
  non-cryptographic and not a durable identity promise;
  source-qualified crypto hashes keep their exact encodings in their
  owning modules (`auth/oauth` PKCE, `auth/aws` signing).
- **`UuidV7Generator`** (pinned `uuidv7`): owns the monotonic timestamp
  floor and 41-bit sequence. `generate(now_ms)` never regresses on clock
  rollback (effective timestamp is `max(now, last)`); `generate_at(ts)`
  preserves an explicit integer timestamp in `0..=2^48-1` exactly without
  advancing the floor; out-of-range timestamps fail with the typed
  `UuidV7Error::InvalidTimestamp`; the sequence is seeded from random
  bytes on first use and increments until `UuidV7Error::SequenceExhausted`
  (deterministically reachable in tests via
  `test_support::force_sequence_exhausted`). Layout follows the pinned
  source bit for bit (version `0x7`, variant `0b10`, sequence spread
  across bytes 6-11, random LSB preserved) in 8-4-4-4-12 lowercase hex.

## Public Overflow/Length Predicates and Typed Diagnostics (P20-S53-3, task 20.53.3)

`opi_ai::message::{is_context_overflow, is_recoverable_length}` and
`opi_ai::provider::{extract_diagnostic_error, DiagnosticErrorInfo}`
complete the pinned `utils/overflow.ts` and `utils/diagnostics.ts`
families:

- **`is_context_overflow(message, context_window)`** (pinned
  `isContextOverflow`): error-based overflow matches the pinned provider
  pattern set only after the pinned non-overflow exclusions (throttling /
  rate-limit style messages never classify as overflow); the bodyless
  Cerebras `400/413 (no body)` shape additionally requires
  `provider == "cerebras"`. With a caller-supplied window it also detects
  silent overflow (`Stop` with `input + cache_read > window`) and
  zero-output Length-stop overflow (`Length` with `output == 0` and
  `input + cache_read >= 99% of window`). Both usage-based branches
  require known usage (`Usage::reported`) and a window greater than zero;
  unknown usage is never converted to zero tokens.
- **`is_recoverable_length(message, desired_max_output)`** (pinned
  `isRecoverableLength`): true when a `Length` stop ended below the
  original desired output limit (known usage only) - the signal a caller
  may use for one bounded compact-and-retry attempt. Both predicates are
  pure: they never invoke compaction or retry policy.
- **`extract_diagnostic_error(&ProviderError)`** (pinned
  `extractDiagnosticError`): returns a serializable
  `DiagnosticErrorInfo { name, message, http_status, error_name }` - the
  variant name, the secret-safe display message (detail carriers are
  `ProviderErrorSummary`), and the typed `ServiceFault` HTTP status and
  wire error name. Raw provider payloads never enter the summary, and
  there is no stack field (no Rust execution-stack analog).

## Public JSON Repair and Portable Schema Utilities (P20-S53-2, task 20.53.2)

`opi_ai::message::{repair_json, parse_streaming_json,
finalize_tool_arguments, string_enum}` complete the pinned
`utils/json-parse.ts` and `typebox-helpers.ts` families:

- **`repair_json`** repairs malformed JSON string literals: raw control
  characters inside strings become valid escapes (`\b`/`\f`/`\n`/`\r`/`\t`
  or `\u00xx`), a backslash before an invalid escape is doubled so the
  model-sent text stays literal, `\u` without four hex digits keeps that
  treatment, legal escapes pass through untouched.
- **`parse_streaming_json`** is a display-only partial parser with the
  exact pinned cascade order: empty input -> `{}`; a complete document
  parses (after escape repair) for display; otherwise the raw
  text, then the repaired text, complete as a partial prefix (dangling
  quotes/brackets are closed, dangling keys/values become `null`,
  truncated numbers and literals complete conservatively, junk after the
  first complete top-level value is ignored); finally a valid empty
  object. It never fails and never overrides final results.
- **`finalize_tool_arguments`** is the public AIR-048 contract for a
  finished tool call: empty input finalizes to `{}`; a complete object
  keeps its owned raw text; a complete object repaired only for invalid escapes
  or raw string control characters returns the repaired text; anything else
  keeps the raw text and a redacted-safe
  `arguments_error` that never echoes the model-sent text. The final
  object/`arguments_error` remains authoritative and raw text is retained.
- **`string_enum`** builds a portable `{"type": "string", "enum": [...]}`
  schema with optional `description`/`default` (empty strings omitted) -
  deliberately no `anyOf`/`const` and no TypeBox symbols or JavaScript
  ABI; the value is ready for `ToolDef::input_schema`.

## Public Text/System Rendering Utilities (P20-S53-1, task 20.53.1)

`opi_ai::message::{content_text, assistant_content_text,
output_content_text}` extract and join the **visible text** blocks of the
message content enums in exact block order, separated by single newlines.
Images, tool calls and thinking — in both plain and redacted forms — are
never exposed by these renderings. The helpers are read-only projections:
they borrow content and never mutate canonical input.

They complete the pinned `utils/text.ts` family together with the
transcript-owned system renderers (`opi_ai::transcript::system_message_text`
and `render_system_message_update`): the initial render is the body followed
by insertion-ordered live sections joined with blank lines (a `null` value
removes a section; an empty value keeps its identity but contributes no
rendered text), and the update render frames each change by name
(`Updated system prompt section "n":\n\n…` / `Removed system prompt section
"n".`). Replaying the chronology (`get_current_system_message` /
`get_current_system_prompt`) preserves the initial/current distinction:
content appends join with a blank line, a section replacement keeps its
position, a delete removes it, and a delete-then-readd moves it to the end.

## Explicit Whole-Call Retry Helper (AIR-065/V-57, task 20.52)

`opi_ai::retry_assistant_call` wraps a caller-owned `produce` closure with
bounded whole-call retry on transient failures. Each invocation of the
closure is a **new logical call** — the helper never re-dispatches an old
one, never mutates transcripts, never re-executes tools and never resets a
prepared call's budget. Omitting the policy (or `enabled: false`) means
exactly one call.

- Classification (`WholeCallRetryPolicy` + `classify_whole_call_retry`)
  follows the pinned source pattern lists: quota/billing/usage-limit
  exhaustion vetoes first, then the transient families — provider load and
  429/5xx/520/524 statuses, gateway forwarding ("unable to forward
  upstream", "service temporarily unavailable"), network/transport
  failures, premature stream endings, explicit mid-stream retry guidance,
  gRPC `ResourceExhausted`, subscription usage hiccups.
- Chat/Responses HTTP 429 errors retain quota/billing/usage-limit classification
  from the designated JSON error fields as a fixed safe message label. The typed
  `RateLimited` cause and retry hint remain unchanged; acquisition retains its
  own 429 send policy. Raw error bodies are never exposed.
- Valid stops (Stop/Length/ToolUse — and any future Deferred variant) and
  aborts never retry; an exhausted budget returns the final error message.
- Callback order is exact: `on_retry_scheduled` → cancellable exponential
  backoff (caller-supplied base, delay capped at 60s by default) →
  `on_retry_attempt_start` → the new call; `on_retry_finished` fires
  exactly once at loop end, only when a retry was scheduled. A failing
  callback stops the loop with the typed `WholeCallRetryError::Callback`.
- Cancellation during the backoff returns the failed message re-stamped
  `Aborted` — error text dropped, partial content and usage preserved.
- The helper itself never switches provider/model/auth (that is the
  caller's factory) and never wires product recovery; host retry policy
  stays with the host.

## Constrained Sampling: Strict Wire Projection and the Bounded Downgrade Variant (AIR-040/V-34, task 20.11)

`ToolDef.constrained_sampling` carries the per-tool intent — `json_schema`
with `prefer`/`require`, or `grammar` with lark/regex variants — and the
adapters resolve it at body-build time:

- The strict projection (`constrained_sampling::make_strict_json_schema`)
  converts a **private wire schema**: every property becomes required,
  optional non-nullable properties wrap nullable, and unsupported shapes
  (references, unions of objects/arrays, tuples, schema-valued
  `additionalProperties`, per-wire rejected keywords such as Anthropic's
  numeric bounds and off-whitelist formats) are typed failures. The
  conversion clones; the trusted declaration never changes.
- `prefer` projects non-strict whenever the wire or the schema cannot
  support strict mode; `require` fails typed before any dispatch, including
  on wires without strict projection (Chat incl. Mistral/OpenRouter,
  Bedrock, Gemini/Vertex, Azure) — a require constraint is never silently
  dropped. Returned explicit nulls normalize back onto the original
  trusted schema.
- Anthropic admits exactly one source-qualified strict-limit downgrade: a
  400 `invalid_request_error` carrying one of the documented
  grammar-size/strict-tool-count/union markers, on a request that actually
  dispatched a prefer-strict tool, becomes the non-retryable
  `ProviderError::StrictToolLimits`, and the acquisition state machine
  admits one rebuilt variant — prefer-strict removed, require kept, fresh
  per-variant budget (≤1+N sends each, total ≤2×(1+N), N=0 still two sends,
  nothing resets the counters). The rebuild re-runs `onPayload` and
  post-validation under its own freeze slot, reuses the frozen
  headers/auth/model (the header transform now runs once per logical call),
  and records one `anthropic_strict_tools_fallback` diagnostic on the
  message. Ordinary `Models` and direct Anthropic calls use the same
  acquisition owner as advanced attempts; forwarding wrappers inherit it and
  cannot create a second retry budget. Successful response headers seal
  acquisition before response callbacks or body parsing, even if no public
  event has been emitted. Other 400s, callback errors, cancellation and
  post-handoff failures never branch; `require` never downgrades.
- Responses/Codex project grammar tools as `custom` tools, lark preferred
  over regex; unsupported targets fall back to a plain JSON function tool
  with the loss recorded on the message, and a supported target without a
  usable variant or schema shape fails before dispatch. Streaming wraps the
  raw `custom_tool_call` input incrementally into `{"<property>":"…"}`
  (monotonic growth, single close, typed failure on violation) while the
  accumulated raw input supplies the authoritative final arguments, including
  terminal-only input; a contradictory terminal input fails typed. Replay restores
  `custom_tool_call(_output)` items with `ctc_` ids (`fc_` stays the
  function-call prefix).

Grammar input property names are literal schema keys, including `/` and `~`;
the wrapper retains the exact name and leaves the trusted schema unchanged.

## Functional Request Callbacks (AIR-059/V-51 ordinary slice, task 20.51)

`callbacks::RequestCallbacks` (carried on `Request.callbacks`, default empty)
attaches four observer families. They fire inside the real direct adapters at
their source-defined timing — never reconstructed from a normalized
`EventStream`, and never by stacking a second driver:

Callback definitions may be shared and cloned, while header/body freezes belong
to one logical call. Independent calls get independent state; acquisition
retries and strict variants keep the same call owner. Each synchronous builder
also has an async counterpart: `with_async_header_transform`,
`with_async_on_payload`, `with_async_on_response` and `with_async_on_raw`.
Adapters await them in order before sending or normalizing the corresponding
data, and cancellation can end a pending callback without another send.

| Callback | Timing | Contract |
|---|---|---|
| `with_header_transform` | Once per logical call, on the assembled per-call set | Output replaces the set; omission/set/removal of non-protected entries is supported. Protected identity/auth/signing entries already present may be returned unchanged; removing, changing or introducing them fails typed before dispatch. Adapter-owned credentials remain outside the transform. |
| `with_on_payload` | After sampling/compat/wire projection, on the final body | `Ok(None)` keeps the body; `Ok(Some(v))` replaces it after validation: JSON object, unchanged model identity, output budget never raised. Chat/Responses/Codex also validate the streaming wire shape before dispatch, preserving unknown extensions and legal content/tool changes. Runs once per payload variant — an ordinary acquisition retry reuses the frozen body byte-for-byte (`ORDINARY_PAYLOAD_VARIANT`). |
| `with_on_response` | At the adapter's source-defined response observation point, with status and headers; streaming observes the source response boundary, image/classifier after complete JSON acquisition | Codex SSE also observes non-success statuses before classification. Failures without a response are not notified. Successful response handoff seals ordinary acquisition retry. |
| `with_on_raw` | Per parsed provider wire event, strictly before its normalized emission, in wire order | Read-only; a failure ends the current call with the typed error while preserving the delivered partial. |

Adapter coverage: Anthropic, OpenAI Chat (including the Mistral and OpenRouter
profiles), OpenAI Responses (including Azure), Codex, and Bedrock wire all four
families (Bedrock raw events are the parsed smithy frames as
`{type, ...payload}`; the Smithy signing headers are transform-protected).
Google wires (Gemini/Vertex) support payload, header transforms and parsed raw
observation before normalization. Their unavailable HTTP response hook rejects
an attached response observer with typed `UnsupportedCapability` before dispatch.
A callback failure is a functional failure of the call:
`ProviderError::CallbackPhase`, redacted, never retryable, never swallowed by
telemetry, never triggering a new call. Callbacks are runtime function values,
not JSON configuration. This slice includes ordinary calls and strict variants;
new operation APIs (20.43–50) and full AIR-059/V-51 closure (20.57) retain their
separate acceptance boundaries.

## Estimator, Sampling and Pricing Metadata (V-45/AIR-053/AIR-062, task 20.35)

The `context` estimator uses 3.5 UTF-16 code units per token
(`ceil(chars * 2 / 7)`) and 4,800 equivalent chars per image, rounding up at
message granularity: each user or tool-result message rounds once over its
blocks, each assistant message rounds once over text/thinking/tool-call
characters, and a system message rounds its body plus sections once with the
serialized added/removed tool lists rounded separately. The last applicable
positive successful usage anchors the estimate (any model identity,
non-error, non-aborted, timestamp not older than its prefix); only messages
after the anchor are estimated, and there is no same-model or full-text
lower bound. An independently reported positive `Usage::total_tokens` wins
even when it differs from the bucket sum; an explicit zero or absent total
falls back to the bucket sum only when the parent buckets are reported.
Unknown stays distinct from zero everywhere; child subsets are never added
twice.

| Parameter | Unit / formula | Omission → default | Zero / null / unknown | Precedence |
|---|---|---|---|---|
| text estimate | `ceil(utf16 / 3.5)` | empty → 0 | — | — |
| image estimate | 4800 chars per image | — | — | — |
| usage anchor | total or bucket sum | none → content estimate | total 0 → sum if reported; unknown → skip | total > sum |
| simple max tokens | `clamp(M)`: `max(1,M)` if window ≤ 0, else `min(M, max(1, W−E−4096))` | `None` → model default output | explicit `M` is not re-capped at the model max | request < window arithmetic |
| numeric-Claude thinking budget | level table 1024/2048/8192/16384 (xhigh/max → high), then clamped to leave answer room | omitted budget → level default | native enabled-without-budget uses 1024; simple budgets keep their wire-specific zero behavior | level default < explicit budget |
| sampling params | per-key JSON map | empty map → no merge | keys absent stay absent | model base < effective level < request |
| temperature | named field | `None` → omitted per adapter gating | NaN omitted | sampling map overrides the named field (merged last) |
| pricing tiers | selection on input+cacheRead+cacheWrite, strict `>`, highest wins, first of equal duplicates | no tiers → base rates | finite negative/fractional thresholds retained; only thresholds `> -1` can match | tier rates apply to the whole request |
| 1h cache write | subset at 2× input rate | `None` → all short-cache | explicit 0 valid, distinct from absent | subset, never double counted |
| service tier | Responses: flex 0.5×, priority/fast 2× (`gpt-5.5`: 2.5×); Codex fast and all Azure tiers: 1× | effective request/response fallback follows the selected wire | unknown tier → unknown cost on tier-sensitive wires | applies to every cost component |

Only OpenAI-compatible adapters (Chat, Responses) merge the resolved
sampling map, as the last body step so model and request sampling override
named request fields; other adapters ignore it. `Usage::service_tier` retains
the observed response category; `effective_service_tier` retains the selected
wire's request/response billing rule. `calculate_cost_tiered(usage, pricing,
model_id, wire)` returns `None` for unknown usage, invalid pricing or an unknown
tier on a tier-sensitive wire. OpenRouter images use base rates without tiers.
`PricingTier::input_tokens_above` uses `serde_json::Number`: exact integers and
finite fractional values. Integer Rust literals use `100_000_u64.into()`;
fractions use `Number::from_f64`. Existing integer catalog snapshots remain
readable without rounding. Thresholds and `u64` token comparisons stay exact
above `2^53`.
Native calls preserve their adapter budget semantics; Pi Messages simple calls
forward their own whitelist. `ModelInfo::input_limits` is metadata, never an
automatic rejection or resize policy. Retired static `reasoning_effort` compat
keys fail with a migration error.

## Non-Goals

The following are explicit non-goals and must not appear as current core
behavior:

- OAuth families beyond the nine listed above.
- Complete provider/profile catalogs, Radius pi-messages generation, or
  automatic catalog fetching during availability checks.
- Automatic re-login after credential revocation.
- Injecting authentication through arbitrary `extra_headers`.
- Provider payload/response streaming hooks.
- A broad new first-class provider list (compatible providers stay
  config-driven profiles).
- Image generation (image support is input-only).
- SDK-owned browser automation; hosts own opening presented authorization URLs.
- Provider streaming-adapter protocol for packages.
- Paid live provider calls in default tests (live tests stay `#[ignore]`-gated).
- Copying pi's provider-specific config file format.

## Minimal Example

Moved into [Quick Start](#quick-start); the three compiling examples live in
`examples/` (`quick_start.rs`, `quick_start_stream.rs`, `tool_loop.rs`) and
compile with `cargo check -p opi-ai --examples`.

## Test layers and SDK packages

See [test organization](tests/README.md) for private/public directories,
contract-target submodules, shared fixtures and focused commands.

WS, TLS and crypto follow the same testing boundaries inside `opi-ai`:

| Layer | Existing coverage | Focused command |
| --- | --- | --- |
| Private state machines | WS framing/PMD/Close/IO under `src/ws/tests` | `cargo test -p opi-ai --lib ws::` |
| Private key/signing ownership | Key admission, independent verification and worker cancellation in `src/crypto` | `cargo test -p opi-ai --lib crypto::` |
| Public transport integration | Strict Upgrade/WSS and Models → Chat → HttpClient → loopback TLS | `cargo test -p opi-ai --test ws_upgrade --test tls_contract` |
| SDK package | Actual `opi-ai.crate`, extracted tests and workspace-external public consumers | `scripts/pi-sdk-conformance/package_slice.py --family <ws\|crypto\|http> --offline ...` |

Module tests remain close to the private state they exercise. Public integration
tests use supported caller interfaces; test helpers do not expose a private
codec or signing API. All three modules share the SDK's package, version and
workspace checks. Package families select scenarios and dependency graphs,
not separately distributed modules. Extracted private tests, packaged public
Upgrade tests and external public consumers are recorded separately. Their
regression counts establish battery completeness, not product acceptance.

The private WS codec still has no production Codex caller. Its tests and the
public raw Upgrade contract do not establish the real Models → Codex →
Upgrade → codec path, continuation/pool behavior or complete cleanup acceptance.

## Modules

`provider`, `message`, `stream`, `model_info`, `api_mapped`, `registry`,
`models`, `provider_headers`, `auth`, `credential`, `catalog`, `http`, `retry`, `model`,
`anthropic`, `openai_chat`, `openai_responses`, `openai_codex_responses`,
`openrouter`, `mistral`, `gemini`, `bedrock`, `azure_openai`, `vertex`,
`config`, `time`, and `test_support`.


## Anthropic Messages Native Options and Inline-Tools Chronology (task 20.39)

The public direct adapter `anthropic::AnthropicMessagesChatApi` registers
through `Models::register_chat_api` (api id `anthropic-messages`) with typed
`AnthropicMessagesOptions` — the pinned pi 1.3.0 native inventory:

| Option | Wire effect |
|---|---|
| `thinking_enabled: Option<bool>` | Tri-state: omitted keeps the neutral request intent; `Some(false)` projects the typed `{"type":"disabled"}` block on reasoning models; `Some(true)` enables native thinking |
| `thinking_budget_tokens` | Source `\|\| 1024` semantics on enabled numeric thinking: missing **or zero** sends the native default 1024; nonzero budgets pass through |
| `thinking_display` | `summarized` (the enabled default, both adaptive and numeric) or `detailed`; always sent when thinking is enabled |
| `effort` | Adaptive thinking only: typed preflight rejects unsupported levels, the wire gets `output_config.effort`; numeric thinking ignores it |
| `interleaved_thinking` | Defaults to `true` (beta `interleaved-thinking-2025-05-14` on enabled non-adaptive thinking); `Some(false)` omits it; adaptive models never send it |
| `tool_choice` | `auto` / `any` / `none` project `{"type": ...}`; `Named` projects `{"type":"tool","name":...}` |

The neutral/simple entry maps only the tool choice; neutral thinking keeps
its level-budget policy (1024/2048/8192/16384 by level), which stays
distinct from the native no-budget default. Per annex row 165 the sub-1024
typed preflight is withdrawn: clamped zero budgets flow to the wire where
`|| 1024` applies.

Inline-tools chronology (`inline-tools-2026-09-15`, compat
`supports_mid_convo_system_messages` + `supports_mid_convo_tool_changes`,
both default false): the request-level tool list stays the initial active
set (cache breakpoint on its last entry) plus the deferred placeholder
`__pi_deferred_placeholder__` (`defer_loading: true`); later system
messages stay native and carry by-value `tool_addition` definitions — a
same-name redefinition is an addition, never a removal — plus
`tool_removal` references for genuinely withdrawn names. System updates
are held back until the next assistant message (or the end) so a
`tool_result` still immediately follows its `tool_use`.

Error-turn image relocation follows the pinned
`splitErrorToolResultImages` shape: the error `tool_result` keeps its text
with a `(image attached after the tool results)` placeholder text block in
place of each image run, and the caption plus deferred images follow all of
the turn's tool results **in the same user message**. The anthropic
producer now runs under the explicit frozen `TelemetryContext`
(P20-TELEMETRY).


## Google Generative AI Native Options and Gemini 3 Wire Rules (task 20.40)

The public direct adapter `gemini::GoogleGenerativeAiChatApi` registers
through `Models::register_chat_api` (api id `google-generative-ai`) with
typed `GoogleGenerativeAiOptions` — the pinned pi 1.3.0 native inventory:

| Option | Wire effect |
|---|---|
| `tool_choice: auto/none/any` | Projects `toolConfig.functionCallingConfig.mode` (the Google wire has no named-tool mode); with tools present, `none`/`any` win over strict mode, `auto` yields to `VALIDATED` when a tool went strict |
| `thinking.enabled: false` | Disabled config: budget-mode models send `thinkingBudget: 0`; level-mode models use the mapped off level when the thinking map provides one, else `0` |
| `thinking.enabled: true` + `level` | `includeThoughts: true` + `thinkingLevel` (MINIMAL/LOW/MEDIUM/HIGH); a level wins over a simultaneous budget |
| `thinking.enabled: true` + `budget_tokens` | `includeThoughts: true` + `thinkingBudget` with the source values untouched — `-1` is the dynamic default and `0` disables; neither is reinterpreted locally |
| (omitted `thinking`) | Native omission sends no `thinkingConfig` at all |

The neutral/simple entry maps only the tool choice; the neutral request
intent keeps the pi `streamSimple` policy: clamped level, level-mode models
(`gemini-3(.x)-pro/flash`, `gemini-4+`, the two `*-latest` aliases, Gemma 4)
project `thinkingLevel`, budget-mode models use the 2.5 family tables
(pro 128/2048/8192/32768, flash 128/2048/8192/24576, flash-lite
512/2048/8192/24576; an explicit budget wins; other budget-mode models fall
back to the dynamic `-1`) and an unresolvable level mapping fails typed
before dispatch. Neutral explicit off sends the disabled config on
reasoning models (the builtin catalog marks the 2.5 models reasoning).

Tool declarations ride the original JSON Schema through
`parametersJsonSchema` (never the legacy OpenAPI `parameters` projection);
Gemini 3+ models project prefer/require strict via the shared constrained
sampling engine and select `VALIDATED` function calling, while Gemini 2
keeps the typed Require failure and silent Prefer downgrade. Gemini 3+ (and
claude/gpt-oss behind Google APIs) tool calls and responses carry normalized
ids (`[a-zA-Z0-9_-]`, 64 chars) on the wire; Gemini 2 keeps name-only
function calls/responses.

Thought signatures (base64, same-provider/same-model only) replay on text,
thinking and functionCall parts; a signature-only empty part survives.
Success tool results use the `output` response key, failures the `error`
key with the text value, consecutive function responses share one user
turn, and tool-result images nest inside `functionResponse.parts` on
Gemini 3+ while earlier models get a separate `Tool result image:` user
turn. Streaming captures the first non-empty `responseId`, thought parts
(`thought: true`) stream as thinking blocks with per-block signature
retention, unsigned or duplicate tool call ids regenerate locally as
`name_N`, usage counts `input = prompt - cached`, `output = candidates +
thoughts` with `reasoning = thoughts` (absent thoughts leave the optional
reasoning subset `None`; explicit zero remains `Some(0)`). Usage fields and
`totalTokenCount` retain their independent presence; a reported total is not
reconstructed from the buckets. A usage-only frame after `finishReason` updates
the same terminal result. Duration is measured from the adapter response
stream's first poll to terminal, excluding Models auth/loading; unpolled
streams start no generation work. These usage and timing rules also apply to
Vertex. Unmapped finish reasons fail typed carrying the raw reason
(`Provider stopped with: <raw>`). The Google producer runs under the
explicit frozen `TelemetryContext` (P20-TELEMETRY).

## pi-messages Direct Adapter (Radius Gateway, task 20.44)

The public direct adapter `pi_messages::PiMessagesChatApi` registers through
`Models::register_chat_api` (api id `pi-messages`, provider id `radius`)
with typed `PiMessagesOptions`. The wire is pi's own message protocol: one
POST of `{ model, context, options }` to `<base>/messages` (trailing
slashes stripped, `?debug=1` when `debug` is set) answered by an SSE stream
of serialized assistant-message events plus a terminal `done`/`error`
event. It is not a Chat serializer: the `context` field is the canonical
chronology projected into pi's camelCase wire JSON (system sections and
tool declarations folded by `normalize_context`, assistant text/thinking
signatures, redacted thinking, tool calls with parsed argument objects,
`toolResult` messages), and mid-conversation system messages stay in place
because the backend natively understands pi's chronology.

The `options` object is an exact six-field whitelist — `temperature`,
`maxTokens`, `reasoning`, `cacheRetention`, `sessionId`, `toolChoice` — and
omissions stay omissions: no `auto` fill-in, no default maxTokens, no
context clamp. `reasoning` carries the requested `ThinkingLevel` verbatim
(`None` maps to the wire string `"off"`); `toolChoice` supports
`auto`/`none`/`required`/named-function; `cacheRetention` maps the request
value one-to-one (`Disabled` to `none`) with only the legacy
`PI_CACHE_RETENTION=long` ambient opt-in when unset. The simple entry
(pi `streamSimple` passthrough) forwards the request's thinking level
unclamped — the model record's level map never rewrites it — and maps only
the neutral tool choice.

Streaming validates events typed and fail-closed: content indices may be
sparse (unstarted holes stay empty), a delta without its start, an unknown
event type, a malformed frame or a `done` with a non-success reason all
fail with `Invalid pi-messages streaming event: …`; `data: [DONE]` is
ignored; CRLF input and a trailing frame without a blank line both parse.
Terminal `done` events carry the full usage family (cache-write-1h,
reasoning, total), `responseId`, `providerThinkingLevel` and the
`pi_messages_rewrite` diagnostic for server-side rewrites; terminal
`error` events fail typed with `errorMessage` (`aborted` maps to
`Cancelled`); a stream that ends without a terminal event fails. HTTP
errors format `"{status} {statusText}: {error.message ?? body} ({code})"`
with the JSON error body preferred and the raw body as fallback; 429 keeps
its retryable identity. Callbacks fire in the pinned order — payload,
response (before the success check), then each raw parsed event before
normalization. The opi-curated baseline catalog (one entry) covers the
pre-catalog window; pi's generated `data/radius.json` baseline is absent
from the pinned source and a configured gateway's dynamic catalog replaces
the baseline at registration time.

## Builtin Provider Factory Layer: 42 Canonical Profiles (task 20.34)

`providers::builtin_provider_profiles` carries the 42-entry source-qualified
profile table from the pi 1.3.0 capability inventory: canonical ids
(`google`, `google-vertex`, `amazon-bedrock`, …), display names, default
base URLs, env API-key names, auth schemes, OAuth availability and endpoint
semantics (Azure deployments, Vertex location templates, Bedrock regions,
dynamic radius). Legacy opi ids (`gemini`, `vertex`, `bedrock`,
`openai-responses`) are not aliases — the table never registers them.
Provider display names include Amazon Bedrock, Anthropic and Google Vertex AI;
credential labels remain separate metadata.
`provider_collection::register_builtin` turns the table plus the bundled
catalog into registered providers over the completed wire adapters: one
mapped provider per profile (Azure and google-vertex stay config-gated with
typed skip reasons, bedrock registers lookup-only behind its AWS-chain
descriptor), a canonical-route wrapper republishing fixed-id adapters under
canonical ids (including the model-spec prefix rewrite prefix-sensitive
adapters need), catalog entries converted with their tiered pricing into
`ModelInfo::pricing`, and the classifier/image protocols registered through
the typed seams with catalog records. Environment keys resolve at dispatch
through an injected `AuthContext` — construction reads no environment,
store, or network.

## OpenAI Decisions Adapter and Four-Protocol Classifier Closure (task 20.49)

The fourth real classifier protocol: `api::decisions::OpenAiDecisionsApi`
(api id `openai-decisions`, provider `openai`) serves OpenAI's Decisions
API — one `POST <base>/decisions` with `{model, input, questions}`. The
state rides as a JSON *string*; with images the input becomes one user
message with the state as `input_text` followed by the images as
`input_image` data URLs (at most 128, rejected typed before any request).
Questions map to Decisions types: `choice` keeps its options (empty
descriptions stay off the wire, alphabetical like every opi criteria
map), `score` keeps its levels and `bool` becomes a `predicate` whose
criteria meanings join the instructions — a System One `noul` and a
Decisions `predicate` are not interchangeable. Answers come back as an
array matched by name; refusals, missing answers, mistyped shapes and
non-finite required numbers are typed errors that clear the entire answer set
while keeping billed usage and its catalog-tiered cost (source-required
validations only — no universal probability range, sum, argmax or label-membership rule). The
route accepts only OpenAI API keys: an empty key fails typed before any
request. Cloudflare in front of api.openai.com answers 504 with an HTML
page when a request runs past about five seconds; retrying the same input
hits it again, so **504 never retries** and the failure explains the
gateway limit instead of surfacing the page. Network, timeout and eligible HTTP
failures keep the shared classifier retry policy (two extra retries by default,
Retry-After-aware capped delays, fresh per-attempt body-inclusive
timeouts). The curated `gpt-6-luna` record documents the tested model
with its long-context pricing tier above 272K tokens. `temperature`
exists on the pi options type but the Decisions request never reads it,
so the typed options carry the retry knobs only. With this adapter the
classifier seam closes its four-protocol proof: typesafe, Cloudflare,
llama.cpp and Decisions register side by side in one `Models` and answer
through the same public `classify` dispatch while keeping their own wire
shapes.

## llama.cpp Classification by Label-Token Readout (task 20.48)

The third real classifier protocol needs no dedicated classification
service at all: `api::llama::LlamaCppClassifyApi` (api id
`llama-cpp-classify`, provider id `llama.cpp`) classifies with an ordinary
chat model served by llama.cpp's `llama-server`. The model never generates
an answer — each question becomes one chat prompt that lists the possible
answers under single-token labels (letters `A`–`Z`/`a`–`z`/digits for up to
62 choice options, digits `0`–`9` for up to 10 score levels, `Yes`/`No`
for bool), and the answer is read from the **pre-sampling next-token
log-probabilities** of those label tokens: a softmax over the label
log-probabilities (divided by the typed `temperature`, default 1, which
never reaches the wire — the completion always carries `temperature: 0`).
Three endpoints drive one question: `/tokenize` resolves each label's
token id (newline-prefixed first, the label alone as fallback; a label the
vocabulary splits is a typed error, as are shared tokens), `/apply-template`
renders the model's own chat template with thinking disabled (an open
`<think>` block is closed at once), and `/completion` reads the top
`n_probs` tokens, escalating the readout depth from `max(256, 16 per
label)` through 4096 to 32768 when a label is missing — a label never
ranked is an error, never an invented zero. Answers keep the shared
semantics: bool probability, score expected value with TypeSafe's
confidence `(n·peak−1)/(n−1)` clamped to `[0,1]`, choice argmax with the
verbatim per-option distribution. The server root strips one
OpenAI-compatible `/v1` suffix from the base URL; every request carries
the model id; the optional Bearer key attaches only when a secret
resolved. llama-server reports no token usage, so results carry none.
Option counts, the positive-finite temperature, image input and the
callback families all reject typed **before the first request**, and the
retry/timeout policy matches the shared classifier transport (two extra
retries per endpoint by default, fresh per-attempt body-inclusive
timeouts, Retry-After-aware delays) — implemented against the shared
policy helpers with its own loop, because a three-endpoint readout with a
per-endpoint observer surface is a different transport shape from the
single-request System One core. Resolved label tokens are cached per
server root, model and label for the adapter's lifetime.

## Cloudflare Workers AI System One Adapter and Multi-Model Registration (task 20.47)

The second real classifier protocol drives the same shared conformance: the
`api::cloudflare::CloudflareWorkersAiSystemOneApi` (api id
`cloudflare-workers-ai-system-one`, provider id `cloudflare-workers-ai`)
serves System One models through Cloudflare's REST endpoint with its own
protocol shapes and **one shared transport core parameterized out of
`api::system_one`** (the pi `SystemOneTransport` shape: URL, request
envelope and response unwrap differ; guards, payload freeze, the retry
policy, per-attempt body-inclusive timeouts, billed-usage retention and
tiered catalog pricing are identical). One `POST <base>/run` carries the
`{model, input: {state, questions}}` envelope — the full model id rides
`model`, public `bool` questions still ride `noul`. The account-scoped
endpoint carries a `{CLOUDFLARE_ACCOUNT_ID}` placeholder that materializes
from the resolved auth's account identity; when absent the placeholder
stays literal (the pi `resolveCloudflareModel` fallback) and the request
never reaches a real endpoint. Responses ride Cloudflare's API envelope in
two shapes: third-party run records wrap the output as
`{success, result: {state, result: {answers, usage}}}` with `state`
required to be `"Completed"` (otherwise `"Cloudflare Workers AI run did
not complete (state: {state})"`), while Cloudflare-hosted models return
`{result: {answers, usage}}` directly; an exact `success: false` aggregates
the envelope's `errors[].message` strings into `"Cloudflare Workers AI
error: m1; m2"` with `"Cloudflare Workers AI request failed"` as the
fallback. `temperature` stays a typed option the wire never carries; image
input rejects before any request; HTTP failures format
`"Cloudflare Workers AI error ({status}): {body}"`.

One provider serves many classifier models, so registrations now hold a
record **set**: `Models::register_classifier_api` seeds it with the
adapter's complete record, the new `Models::register_classifier_model_record`
appends (or replaces, keyed by provider/model id), and
`Models::classifier_model` scans the set; a record whose API has no
registration is a typed `ModelsError::ClassifierApiNotRegistered`
rejection. The curated baseline ships the three pinned identities —
`typesafe/jev` (the 64000-token context window carries over from the same
underlying model's TypeSafe fixture), `@cf/cloudflare/clef` ($0.24/M input)
and `@cf/cloudflare/clef-flash` ($0.09/M input); output pricing and the
hosted models' context windows have no pinned evidence (pi's generated
`data/cloudflare-workers-ai.json` is absent from the pinned source) and
stay at the generator's `0` fallback. Registrars replace the set wholesale.

## Classifier Operation and TypeSafe System One Direct Adapter (task 20.46)

The fixed-`O` classifier operation ships beside the chat and image seams:
the object-safe `api::ClassifierApi<O>` trait mirrors `ImageApi` (exact
`ApiId`, provider identity, complete record, typed options, prepared
dispatch) — **one shared conformance spans every real classifier protocol**
through this seam — and the public `classifier` contracts carry the typed
`ClassifierContext` (JSON state object, ordered questions, optional images),
`ClassifierAnswer` variants, the `ClassifierResult` (provider failures never
reject — they ride `stop_reason = Error`/`Aborted` with the typed
`ProviderError` and a display message; a billed request keeps its reported
usage when later answer parsing fails), and the operation-qualified
`ClassifierModelRecord` with **catalog `ModelPricing` including tiers**.
`Models::register_classifier_api` registers an adapter with its resolver and
provenance source, `Models::classifier_model` performs operation-qualified
selection (chat models and unknown ids fail typed *before* any auth
resolution), and `Models::classify`/`classify_native` are the ordinary and
typed-native drivers; a `ClassifierApiHandle::options` value stays valid
across compatible re-registrations.

The one real adapter, `api::system_one::TypesafeSystemOneApi` (api id
`typesafe-system-one`, provider id `typesafe`), reproduces the pinned pi
module: one POST `{model, state, questions}` to `<base>/systemone`
(trailing slashes stripped), Bearer-only auth, public **`bool` questions and
answers mapped to the wire-level `noul` type** on both sides (choice/score
keep their shape), image input rejected before any request (`"System One API
does not support image input"`, regardless of the record's modalities), and
an api-mismatch guard. Answer parsing validates only what the source
requires — record shape, wire-type agreement, a string `choice`, finite
numbers — with **no** universal probability-range/sum/argmax/label-membership
rule; every known question must have a type-matching answer, unknown answer
ids are ignored. Usage `{input_tokens, output_tokens}` retains known counts;
missing or invalid fields do not become reliable zero and incomplete parent
buckets remain marked unreported. It prices **chat-style through the
record's catalog tiers** (unlike the image path's base-rate exception).

Transport follows the source: **no default timeout** (a caller timeout is
body-inclusive and **fresh for every retry attempt**, reported as
`"Request timed out after {ms}ms"` and retryable like any no-status
failure), **two extra retries by default** with the SDK retry policy
(`x-should-retry` override, 408/409/429/5xx plus network/timeout retryable,
`retry-after-ms`/`retry-after` honoured with the 60 s server-delay cap;
above-cap requests fail immediately with the source message), retries reuse
the frozen payload, cancellation maps to `Aborted`, and HTTP failures
format `"System One API error ({status}): {body}"`. Across all four classifier
protocols, JSON syntax, callback and answer-validation failures do not trigger
low-level acquisition retries; Decisions also excludes HTTP 504. `temperature` is a
typed option System One cannot apply: it never reaches the wire. The
payload observer (replace-or-keep, once per logical call, frozen across
retries) and the response observer (status + headers after complete JSON decoding, outside acquisition retry and its timeout) are supported; raw-event and header-transform families reject
typed before dispatch. The frozen `TelemetryContext` passes through to the
request path. The curated baseline record (`typesafe/jev-latest`, 64000-token
context, text input, zero pricing until registered) replaces wholesale at
registration; pi's generated `data/typesafe.json` is absent from the pinned
source.

## Image Operation and OpenRouter Image Direct Adapter (task 20.45)

The fixed-`O` image operation ships in `api`/`images`: the object-safe
`api::ImageApi<O>` trait mirrors `ChatApi` (exact `ApiId`, provider
identity, complete record, typed options, prepared dispatch), public
`images` types carry the ordered `ImagesContext`/`ImagesOutput`, the
`ImagesResult` (provider failures never reject — they ride
`stop_reason = Error`/`Aborted` with the typed `ProviderError` and a display
message), and the operation-qualified `ImageModelRecord`. `Models::
register_image_api` registers an adapter with its resolver and provenance
source, `Models::image_model` performs operation-qualified selection (a chat
model or unknown id fails typed *before* any auth resolution), and
`Models::generate_images`/`generate_images_native` are the ordinary and
typed-native drivers; an `ImageApiHandle::options` value stays valid across
compatible re-registrations.

The one real adapter, `api::openrouter_images::OpenRouterImagesApi`
(api id `openrouter-images`, provider id `openrouter`, `O =
OpenRouterImagesOptions` with the retry policy), reproduces the pinned pi
module: one nonstream Chat-wire POST `{model, messages, stream: false,
modalities}` to `<base>/chat/completions` (trailing slashes stripped;
`modalities` is `["image", "text"]` when the record declares text output,
else `["image"]`), Bearer-only auth, text inputs serialized as text parts
and image inputs as `data:<mime>;base64,<data>` URLs — no four-MIME/count/
bytes/signature gates, no URL fetch, file write, pixel decode or streaming
variant. Parsing keeps the **first choice only, text before images**:
non-empty string content becomes the leading text output, then each
`message.images` entry whose URL matches `data:<mime>;base64,<data>` becomes
an image output preserving the encoded original; non-data URLs, malformed
entries and later choices are skipped, and text-only, empty-image and
zero-choice responses stay successes. Usage keeps the provider split
(`cache_read = reported_cached - cache_write` when writes exist, `input`
excludes both cache buckets) and pricing uses the record's **base rates
only** (the image path's base-cost exception — tiers never apply; the
curated baseline entry's unknown pricing stays the generator's `0`
fallback, and pi's generated `data/openrouter.json` is absent from the
pinned source, so a registrar replaces the record wholesale).

Transport follows the source: a **600000 ms default per-attempt timeout
covering the whole body read and parse** (distinct from Mistral's
headers-only 60 s deadline), an explicit timeout must be positive
(`Some(zero)` is a typed pre-dispatch rejection), **zero extra retries by
default** with the SDK retry policy (`x-should-retry` override,
408/409/429/5xx/no-status retryable, `retry-after-ms`/`retry-after`
honoured with the 60 s server-delay cap — above-cap requests fail
immediately with the source message), retries reuse the frozen payload,
cancellation maps to `Aborted`, and HTTP failures format
`"<status>: <error.message | body>"` with 401/403 → `AuthFailed`, 429 →
`RateLimited`, 408/504 → `Timeout`. Callbacks: the payload observer
(replace-or-keep, once per logical call, frozen across retries) and the
response observer (status + headers after complete JSON decoding, outside acquisition retry and its timeout) are supported;
raw-event and header-transform families are rejected typed before dispatch
(ONB-04). The frozen `TelemetryContext` (P20-TELEMETRY) passes through to
the request path without automatic spans.

## Mistral Conversations Direct Adapter and Native Options (task 20.43)

The public direct adapter `mistral::MistralConversationsChatApi` registers
through `Models::register_chat_api` (api id `mistral-conversations`,
provider id `mistral`) with typed `MistralOptions`. The API keeps its name
but posts `{base}/v1/chat/completions`; the pre-existing OpenAI-compatible
profile (`mistral::mistral_provider`) stays untouched as a legal profile.

| Option | Wire effect |
|---|---|
| `tool_choice: auto/none/any/required/named` | `tool_choice` (`{"type":"function","function":{"name":...}}` for named); omitted options leave the member off the wire |
| `reasoning_effort: none/low/medium/high/max` | `reasoning_effort` verbatim |
| `prompt_mode: reasoning` | `prompt_mode: "reasoning"` |

The neutral entry maps tool choice only, then the model record resolves the
reasoning branch: a model whose thinking level map carries explicit entries
sends `reasoning_effort` (the nearest supported level after clamping; the
`none`-level entry projects when thinking is disabled; missing entries fall
back to `high`), a mapless reasoning model sends `prompt_mode: "reasoning"`,
and non-reasoning models send neither. Sampling projects `temperature` and
`max_tokens` only — stop sequences, `top_p` and the penalty family stay off
this wire (pi's payload builder sets nothing else). Tools project
`strict: true` by default with the converted schema.

Prompt caching rides the session: with a session id and a non-`Disabled`
retention, the body carries `prompt_cache_key` and the headers carry
`x-affinity` (an explicit `x-affinity` header wins and only suppresses the
auto header). The response deadline (default 60s, overridable via the
request timeout) covers **only the wait for response headers** — body
stalls stay with the HTTP client's idle handling, so long streams are never
cut by a fixed deadline.

Replay normalizes tool-call ids to nine alphanumeric characters (strip,
exact-length passthrough, SHA-256-derived otherwise, collision retries) on
both the assistant call and the paired tool result. Message conversion
collapses mid-conversation system messages to the merged leading prompt,
drops blank text, replays assistant thinking as `thinking` chunks with
`prefix: false`, keeps single text-only user turns as plain strings, sends
`image_url` data URIs on image models (`(image omitted: model does not
support images)` otherwise), and renders tool results with the
`[tool error]` prefix and the `(see attached image)`/`(no tool output)`
placeholder family. Streaming keeps the first non-empty chunk `id` as the
response id, splits cached prompt tokens (`input = prompt - cached`,
`cache_read = cached`, `cache_write = 0`, total falls back to the sum),
skips empty content deltas (GLM-style block splitting), accumulates tool
calls by index with derived ids when the wire sends the `"null"`
placeholder, maps `stop`/`length`/`model_length`/`tool_calls` stops, fails
unmapped stops and the in-stream `error` stop typed carrying the raw reason
(`Provider stopped with: error (server error)` for the retryable server
form), and fails a stream that ends without a finish reason. HTTP failures
format `Mistral API error (status): body` (4000-char truncation before the
crate-wide sanitized-summary cap); 429 keeps its retryable rate-limit
identity.

## Bedrock Converse Stream Direct Adapter and Native Options (task 20.42)

The public direct adapter `bedrock::BedrockConverseStreamChatApi` registers
through `Models::register_chat_api` (api id `bedrock-converse-stream`,
provider id `bedrock`) with typed `BedrockOptions`. The AWS transport layer
— the SigV4 credential chain, bearer tokens, region/profile precedence,
Smithy retry, the eventstream+CRC parser and the explicit proxy-placeholder
signing mode — is inherited unchanged from the earlier delivery; the
adapter adds the pi content engine on top.

| Option | Wire effect |
|---|---|
| `tool_choice: auto/any/none/named` | `toolConfig.toolChoice` (`{"auto":{}}`, `{"any":{}}`, `{"tool":{"name":...}}`); `none` drops the whole `toolConfig`, so the tools ride no other member |
| `reasoning: <level>` | Claude budget models: `additionalModelRequestFields.thinking = {type: enabled, budget_tokens}` from the custom `thinking_budgets` override or the default table (1024/2048/8192/16384; `xhigh`/`max` clamp to `high`); adaptive models (Opus 4.6+/4.7/4.8/5, Sonnet 4.6/5, Haiku 5, Fable 5): `thinking = {type: adaptive, display, block_binding}` + `output_config.effort` (native `xhigh` on capable models, the thinking map's explicit mappings override) |
| `interleaved_thinking: bool` (default true) | The `interleaved-thinking-2025-05-14` beta on budget-based Claude models |
| `thinking_display: summarized/omitted` | `thinking.display` (default `summarized`); **GovCloud** targets (a `us-gov-*` dispatch region or a `us-gov.`/`arn:aws-us-gov:` model id) omit `display` and the binding-controls beta until the GovCloud Converse schema catches up |
| `request_metadata` | Rides verbatim as `requestMetadata`; the AWS limits live in pi comments only and the SDK owns validation (no local preflight) |
| (non-Claude) | gpt-oss models take a flat `reasoning_effort` (low/medium/high); GPT models take nested `reasoning.effort`; other families send no additional fields |

`region`/`profile`/`bearerToken` are resolver-owned inputs (the task 20.20
AWS chain: explicit/scoped/env keys, profiles, web identity, ECS/ECS
relative URIs, IMDS, SSO), not options on this struct.

Strict tools follow `BedrockConverseStreamCompat.supports_strict_tools`
(`WireCompat::BedrockConverseStream` became a tuple variant): a strict tool
spec carries the converted schema plus `"strict": true`; `require` still
fails typed before dispatch when the flag is false. Cache points ride the
system prompt and the last user message (including tool turns) on the
supported Claude families — default retention keeps pi's `short` default,
`Long` adds the one-hour `ttl`, `Disabled` drops every point. Message
conversion sends `<empty>` where Bedrock requires non-blank content, drops
blank text, replays thinking as `reasoningContent.reasoningText` (signed
Claude blocks; unsigned Claude falls back to plain text; non-Claude never
carries the signature member), replays redacted reasoning as opaque
`redactedContent` bytes (a non-base64 payload drops the block), merges
consecutive tool results into one user message, and moves Claude error-tool
images after the turn's results with a caption. Streaming maps
`reasoningContent` deltas to thinking blocks (signature accumulation,
redacted payload buffering), maps `model_context_window_exceeded` to
`length`, fails unmapped stop reasons typed carrying the raw reason
(`Provider stopped with: <raw>`), and sums one-hour `cacheDetails` into
`cache_write_1h_tokens`. The producer runs under the explicit frozen
`TelemetryContext`.

## Google Vertex Direct Adapter and Native Options (task 20.41)

The public direct adapter `vertex::GoogleVertexChatApi` registers through
`Models::register_chat_api` (api id `google-vertex`, provider id `vertex`)
with typed `GoogleVertexOptions`. The tool-choice and thinking intents share
the Google shapes documented for task 20.40 (no named-tool mode;
`budget_tokens` keeps `-1` dynamic and `0` disable untouched; native
omission sends no `thinkingConfig`), with two Vertex-own extensions:

| Option | Wire effect |
|---|---|
| `project` / `location` | Re-route the ADC URL (`.../v1/projects/{p}/locations/{l}/publishers/google/models/{model}:streamGenerateContent`); inert under the Express API-key route, which carries neither |
| (Bearer auth without project or location) | Typed setup failure before any network work — never a silently degraded URL |

Routing is auth-owned and independent of the Gemini key identity: an API key
selects Express mode (`x-goog-api-key` on the project-less
`aiplatform.googleapis.com` collection endpoint), a Bearer token rides the
ADC project/location URL plus the ADC file's `x-goog-user-project` quota
header when the credential source owns one. A custom base URL never drops
the project/location resource prefix (the prefix is always rebuilt), and
`location = "global"` pins the shared domain. The content engine, streaming
parse, independent usage/total accounting and activation-based duration,
strict tool handling and thought-signature rules
are inherited from the shared Google engine (task 20.40).

One neutral policy differs per wire: the vertex budget table has no
dedicated flash-lite entry, and pi's substring match routes
`2.5-flash-lite` through the flash table (minimal 128; the generative-ai
wire's dedicated table maps minimal 512). The vertex producer runs under
the explicit frozen `TelemetryContext`.

## Deferred Responses (task 20.50)

Deferred submission is a normal generation (AIR-043): a capable backend
returns an assistant message with `StopReason::Deferred`, empty content and
a `deferred` handle instead of final content — no `submitDeferred` API
exists. `DeferredHandle` carries source-owned identity
(`provider`/`model_id`/`api`/`id`), optional expiry and poll advice, and
optional provider conversion data; it never carries a credential and
serializes pi-shaped camelCase (`expiresAt`, `pollAfterMs`, `data`).

Fetch and cancellation are explicit operations on that handle:

- `Models::stream_deferred(handle, call) -> EventStream` validates the
  handle (identity, then expiry against the caller-injected `now_ms`),
  resolves authentication through the backend's registered resolver, and
  dispatches exactly one fetch. Pre-dispatch failures surface as one typed
  `Err` stream item that `ChatStream` converts into the canonical
  `CallFailure` (typed `CredentialNeeded` causes included).
- `Models::fetch_deferred(handle, call)` drives that stream to the single
  final outcome: still-pending results are `Ok` with
  `StopReason::Deferred` and the handle on the returned message; ready
  results deliver the final assistant message. `wait_ms = 0` (pi wait0)
  performs one status check; the SDK never polls implicitly regardless of
  `poll_after_ms`.
- `Models::cancel_deferred(handle, call)` is a best-effort remote cancel:
  failures are reported, never guaranteed. Cancelling the fetch call's
  local token aborts only the local attempt (`ProviderError::Cancelled`,
  zero backend work when cancelled before the first poll) and never
  reaches the remote job — the two cancellation paths are independent
  (AIR-044).

Backends register through `Models::register_deferred_api` (typed
`DeferredApi` trait: `api_id`/`provider_id`/`model_id`/`default_options`
plus `fetch_prepared`/`cancel_prepared`); a deferred upsert with the same exact `ApiId`
replaces the registration wholesale. `Models::validate_deferred_handle`
exposes the typed pre-auth validation (`RouteMismatch` boxed detail,
`Expired`, `Unsupported`). Fetch and cancel calls carry an optional frozen
`TelemetryContext` straight to the backend (AIR-054); submission reuses the
chat request's existing carrier. No real remote deferred protocol is
claimed: the published seam serves scripted and custom backends, and
`test_support::ScriptedDeferredBackend` plus
`MockProvider::with_deferred` provide the deterministic pi-faux scripting
(pending-fetch countdown, cancel recording, optional forced cancel
failure) that the reopenable consumer fixture in
`tests/deferred_contract.rs` exercises without resubmission under a fake
monotonic clock.

## License

MIT. See the packaged [LICENSE](LICENSE) and compression [NOTICE](NOTICE).

### AWS Smithy V2 retry modes (Bedrock)

Bedrock generation sends retry through the pinned AWS modes
(`bedrock::retry`): `standard` and `adaptive` are the two known modes (an
unknown `retry_mode` is a typed configuration failure, never `Unsupported`
and never silently standard), and the Smithy2026 constants are an explicit
frozen branch on the construction-time configuration. Scoped inputs resolve
`AWS_RETRY_MODE`/`retry_mode` and `AWS_MAX_ATTEMPTS`/`max_attempts` with the
source's precedence (environment, then shared-config profile, then the
standard/legacy/3-total default) and never sample ambient process state.
Classification is exact: 429 and the throttling error names are THROTTLING;
the `$retryable` trait (`ModelNotReadyException`), the transient names,
transport timeouts/network failures, and statuses 500/502/503/504 are
TRANSIENT; bare other 5xx are SERVER_ERROR and client faults
(`ModelTimeoutException` on 408, 409, 403, 400 without a trait) are
CLIENT_ERROR — no blanket `>=500` retry and no helper 60-second hint cap.
The per-call quota starts at 500 tokens (costs 5/10, or 5/14 with 2026
constants; success refunds), `floor(u * min(base * 2^count, 20s))` full
jitter runs on the 100/500ms (50/1000ms in 2026) bases, and server hints
(`Retry-After` seconds/dates and `x-amz-retry-after` milliseconds) adjust
with the bounded `max(delay, min(hint, delay + 5s))` rule. Adaptive mode
adds the CUBIC client-side rate limiter (measured-rate EWMA over half-second
buckets, multiplicative cut on throttling, cubic recovery) whose send-token
wait folds into the same cancellable readiness deadline. Each send stamps
the wire `amz-sdk-invocation-id` (one uuid per logical call) and
`amz-sdk-request: attempt=N; max=M` headers and re-signs SigV4 with a fresh
timestamp while the frozen credential and payload stay byte-identical;
source-exchange sends and generation sends are counted separately and the
quota never spans calls.

### ADC native sources and the Vertex dual-mode wire (P20-S21-1)

Vertex authentication resolves through the pinned ADC families
(`auth::adc`): a `service_account` file exchanges an RS256 JWT assertion
(alg-only header, `exp = iat + 3600`) at the canonical Google token
endpoint; an `authorized_user` file refreshes with its four form fields
and keeps `quota_project_id` separately (sent as `x-goog-user-project`);
an `external_account_authorized_user` file refreshes with HTTP Basic
client auth at its validated `token_url` or the default STS oauthtoken
endpoint, and a rotated refresh token replaces the owner's copy in place
without rewriting the ADC file. The explicitly selected direct metadata
source serves the instance token with the `Metadata-Flavor: Google`
exchange and the frozen residency timeout (resident: none, otherwise
3000 ms; a timeout is a single-send terminal failure per the measured
corpus). Exchanges run the fixed Gaxios retry configuration (POST
admitted, 3 extra retries, 2 no-response retries, statuses 1xx/408/429/
5xx, 100/500/1500 ms delays) through an injectable transport and never
enter the generation retry owner. Token POSTs refuse redirects; source GETs
follow at most twenty redirects and drop sensitive headers across origins. An explicit
`GOOGLE_APPLICATION_CREDENTIALS` file is terminal on read/parse failure;
a missing well-known file (`%APPDATA%\gcloud\...` on Windows,
`$HOME/.config/gcloud/...` elsewhere) means not configured — never
metadata or gcloud discovery. `VertexProvider` serves API-key Express
(`x-goog-api-key` on the project/location-less collection URL) and
Bearer ADC routes (project/location required, `global` pins the shared
domain); the product route prefers a stored credential or
`GOOGLE_CLOUD_API_KEY`, then the configured access-token env, then the
ADC chain.

### ADC external-account federation and the executable supplier (P20-S21-2)

`auth::adc` also owns the `external_account` federation families. File and
URL suppliers keep the raw subject-token text (no trim) or the named JSON
field; an AWS (`aws1`) source resolves region and security credentials
(environment first, else the security-credentials endpoint with an optional
IMDSv2 session token; every IMDS leg is a single send) and signs a
serialized `GetCallerIdentity` request — `x-goog-cloud-target-resource` is
merged after signing, as upstream. The certificate source resolves its
configuration (explicit location > `GOOGLE_API_CERTIFICATE_CONFIG` >
well-known gcloud path), validates leaf and chain, builds the base64(DER)
JSON chain token with the exact leaf-order rules, and exchanges over a
source-scoped mTLS transport admitted through the frozen TLS profile. The
executable supplier checks `GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1`
before the output-file cache and any spawn, parses the command into a
direct quote-aware argv, merges stdout/stderr, validates the response
schema (token-type URNs, `expiration_time` when an output file is
configured, expiry), treats a malformed cache as terminal and a stale one
as a miss, uses the selected helper's exact environment/cwd snapshot,
terminates and reaps its Unix process group or Windows Job on timeout or
caller drop, and never rewrites the output file. Helper failure code/message fields are closed `ProviderErrorSummary`
values; arbitrary helper output is absent from Display, Debug and safe summaries. The RFC8693 STS exchange defaults to
`https://sts.{universe_domain}/v1/token`, uses HTTP Basic for confidential
clients, and flows a workforce user project into the `options` form field
without client authentication. Provenance classifies the supplier through
`AdcCredentialSource::ExternalAccount`; impersonation composes through the
source/target mechanism below. The AWS subject supplier uses its ADC-specific
metadata/environment contract rather than the Bedrock profile/SSO/process chain.

### ADC impersonation, recursive composition and the keyed cache (P20-S21-3)

`auth::adc::impersonation` owns the `impersonated_service_account` family:
`source_credentials` resolves recursively through the same adapter with no
new depth cap, the target principal is extracted from the
`service_account_impersonation_url` (bounded, typed failure otherwise), and
the raw URL never becomes the wire destination — the IAM
`generateAccessToken` request is reconstructed from the effective universe
domain (an explicit `universe_domain` disagreeing with the source's fails
typed before any leg). The body keeps delegates, the target scope array and
`"{seconds}s"` lifetime; the response `expireTime` is RFC3339; source and
target identities stay separate in the `Impersonated` provenance with the
file's quota project riding `x-goog-user-project`. An `external_account`
file's optional impersonation URL activates after the STS exchange with the
forced cloud-platform scope, wrapping the federation source in the same
`Impersonated` provenance. `auth::adc::cache` centralizes the owner's token
store under the pinned key grammar (config digest plus principal/target,
delegates, audience, scopes, project, quota, endpoint, universe,
TLS/mTLS identity) with single-flight acquisitions, one 5-minute eager
window, short-TTL current-call service without refresh loops, and
generation fencing so a refresh completing after an invalidation never
publishes. Sources without stable wire principals are identity-unverified
(`stable_identity() == None`): an automatic whole-call recovery must not
reuse them across a source change; ordinary provider retries keep the
frozen per-call token. Rotating ADC acquisitions are owned by the shared
mutation queue: dropping a waiter does not discard a started refresh; source
replacement fences late tokens and rotated refresh material together. A
barrier observes the source operation and ACK without starting auth. The
product shares this queue with its keychain store.

### SDK OAuth (Anthropic) and store-acknowledged `Models::login` (P20-S23)

`auth::oauth` owns the Anthropic login mechanism pinned to pi 1.3.0's wire
shapes: token requests are JSON (`Content-Type`/`Accept: application/json`)
with a 30-second per-request deadline inside the total login budget, the
authorize URL carries the exact scopes plus S256 PKCE and an independent
random state, and the callback ladder prefers `127.0.0.1:53692` with the
`/callback` path (redirect URI names `localhost`), falls back to a free
loopback port, and finally to paste-back only. A second login method,
copy-code, authorizes against the hosted `oauth/code/callback` page and
accepts a pasted `code`, `code#state`, query, or redirect URL. Selection,
presentation and copy-code waiting share the configured total budget and
cancellation waiter. Token-endpoint
and flow failures keep the fully redacted product summaries — the closed
OAuth error class never surfaces and upstream bodies cannot echo back.
Expiry is stored as the real service expiry; the single five-minute
near-expiry correction stays `OAuthCredential::needs_refresh`.
`Models::register_oauth_flow` + `Models::login(provider, presenter)` run a
flow against an injected presenter and the attached credential store:
success is reported only after the awaited store ACK, failed writes surface
typed, and unknown mutation outcomes stay honestly pending. No automatic
browser launch, keychain read, environment fallback, or remote revocation
happens. `opi-coding-agent` keeps only host policy (TUI presenter, terminal
suspension, keychain store) and registers the SDK flow.

### SDK OAuth (OpenAI Codex legacy) (P20-S24)

`auth::oauth::codex` owns the Codex login mechanism pinned to pi 1.3.0's
`openai-codex.ts`: form-encoded token exchange and refresh, and the
fixed `http://localhost:1455/auth/callback` redirect URI shared with the
Codex CLI — a failed bind falls back to paste-back only with that same URI,
never a free port (`bind_fallback` selects the Anthropic free-port ladder or
the Codex manual-only ladder). The browser authorize URL carries the
production scopes, S256 PKCE, an independent state, and the
`id_token_add_organizations`/`codex_cli_simplified_flow` flags and an
`originator` captured by `CodexOAuthProvider::with_agent_name` (default `opi`). The device-code path pins OpenAI's `deviceauth` endpoints: JSON
bodies, the 403/404 pending quirk, structured pending/slow_down codes with
the RFC 8628 +5s backoff, typed denied/expired classification, the bounded
15-minute budget, and the exchange of the server-issued
`authorization_code` + `code_verifier` against the hosted device redirect
URI. Refresh is strict per pi `readTokenResponse` (all three token fields
required) and re-extracts the ChatGPT account id from the access-token JWT
(typed `AccountIdMissing` on login and refresh alike). The endpoint
(`base_url`) rides the credential through refresh. `Models::login`
registers `openai-codex` with the same store-ACK semantics as Anthropic.
`opi-coding-agent` cut over compiling: the product Codex protocol duplicate
was deleted in favor of the SDK mechanism.

### SDK OAuth (OpenAI ChatGPT direct) (P20-S25)

`auth::oauth::chatgpt` is pinned to pi 1.3.0's `openai-chatgpt.ts`: every
login registers a dynamic client (`dynamic_agent_client`) and every
callback or pasted URL must carry OpenAI's issued client id — the form
exchange and all later refreshes reuse it, the credential persists it
(`OAuthCredential`/`Credential::OAuthToken` gained a non-secret optional
`client_id`), and a stored credential without it fails typed with a
reconnect error. The authorize URL carries the production scopes
(`resource.invoke chatgpt.tokens.use.direct`), S256 PKCE, an independent
state and nonce, the RFC 8707 `resource` parameter, `agent_name_hint`
(default `opi`), and the installation's stable
`ext_agent_host_id=urn:uuid:<lowercase>` — a missing or malformed device
UUID is a typed failure before any bind or HTTP traffic. The redirect is
the fixed `http://127.0.0.1:1455/auth/callback`: a failed bind is a typed
terminal error (the new `CallbackBindFallback::Required`; never the Codex
paste-back ladder, never a free port), while the bound server races a
strictly validated manual input that must be the full redirect URL with
the same origin and path, matching state, and the issued client id (a bare
code or a foreign origin is typed; an `error` parameter is a typed denial
whose provider text is never echoed). The token response is validated per
pi `credentialFromTokenResponse` (non-empty access/refresh/scope, the
direct grant flag, positive finite `expires_in`, and a present, never
parsed `id_token` on login); the stored expiry is the real service expiry
with the single five-minute correction at `needs_refresh`.
`run_pkce_login` was refactored into the shared `run_pkce_login_with`
seam (Anthropic/Codex behavior byte-identical through the base adapters).
`Models::login` registers `openai-chatgpt` with the same store-ACK
semantics; the persisted envelope is always the OAuth kind, so
API-key-only consumers (the Decisions classifier) can never resolve this
subscription credential as an API key. Host call sites map the new field
through the host envelope: issued `client_id` and the validated `scopes`
list survive save/reopen and refresh without changing the OAuth credential kind.

### SDK OAuth (GitHub Copilot) (P20-S26)

`auth::oauth::copilot` is pinned to pi 1.3.0's `github-copilot.ts`. The
flow starts with the new `LoginPresenter::prompt_text` asking for a
GitHub Enterprise URL/domain (blank = `github.com`; a non-empty
unparseable answer fails typed before any HTTP) and derives all three
endpoints from the normalized hostname like pi `getUrls`. The GitHub
device-code flow uses the fixed client `Iv1.b507a08c87ecfe98` with
`read:user`, strictly validates the response, requires an http(s)
`verification_uri`, waits one interval before the first poll, honors the
server interval on `slow_down` (else +5s), and treats
`access_denied`/`expired_token` as typed terminal revocations with
unknown codes closed. The Copilot token exchange is strict (401/403
revoked) and the request base URL is derived per request from the
token's `proxy-ep=` hint, then the enterprise domain, then the
individual default — never stored. Models metadata is discovered with
bounded 429 retries (login 2/5s, refresh a single attempt) and parsed
with pi's picker/policy semantics (the enabled-policy fallback only on
the individual endpoint; the static-catalog whitelist — injected by the
host — gates which unconfigured models login enables via
`{"state":"enabled"}` chat-policy POSTs after a `notify_progress`).
Policy activation is login-only: refresh re-exchanges the stored GitHub
token and re-discovers the metadata without touching policy.
Credentials gained non-secret `enterprise_domain` and
`available_model_ids` fields; the host envelope persists both atomically
with the tokens and other OAuth metadata. They store the real service expiry with the single
five-minute `needs_refresh` correction, and keep the GitHub token in
the refresh slot. The product's `CopilotOAuthProvider` protocol
duplicate is deleted; the host registers the SDK provider with its
static catalog as the policy whitelist and derives the Copilot request
base from the token (a legacy stored `base_url` still wins so
pre-cutover credentials never change endpoints).

### SDK OAuth (OpenRouter) (P20-S27)

`auth::oauth::openrouter` is pinned to pi 1.3.0's `openrouter.ts`.
OpenRouter trades an authorization code for a permanent API key: the
authorize URL carries `callback_url` + S256 PKCE with no `state` and no
`client_id` - the CSRF defense is the fresh random `/oauth/callback/`
UUID route each login, which the shared loopback handler now validates
for every PKCE flow (wrong route or method: 404 page, keep waiting). The
ephemeral-loopback callback races a manual paste of the redirect URL, a
`code=` fragment, or the bare code; the exchange is one JSON POST to
`/api/v1/auth/keys` (30-second deadline) that must carry a non-empty
`key`. Credentials keep the key as the access token with an empty
refresh slot and the permanent sentinel expiry (9999-12-31 - pi's
`MAX_SAFE_INTEGER` mapped to the last representable instant), so
`needs_refresh` is always false, `refresh` is the identity with zero
network I/O, and the flow never issues the personalization request.
The shared runner gained `AuthorizeStyle` and an optional post-bind
progress template; existing flows are unchanged in behavior.

### SDK OAuth (Kimi Coding device flow) (P20-S28)

`auth::oauth::kimi_coding` is pinned to pi 1.3.0 `kimi-coding.ts` plus the
shared `device-code.ts` polling state machine. RFC 8628 against
`auth.kimi.com`: a form POST starts device authorization with the fixed
client id (both verification URIs must be http(s); interval/expires default
5s/15min), the presenter gets the user code with the COMPLETE verification
URI, and form-encoded token polls wait one interval first, floor at 1s,
honor a server `slow_down` interval or add 5s, and end typed at the device
deadline (the after-slow-down reason is distinct). `authorization_pending`
keeps polling; `expired_token`/`access_denied` are terminal revoked grants;
success must be the strict access/refresh/expires_in triple stored as the
real expiry. Every request has the 30s deadline; failure bodies are never
echoed. `refresh` retries 429/5xx/transport up to three times (1s/2s/4s
backoff); 401/403 or `invalid_grant` is a typed revoked credential with no
account fallback. The env OAuth-host override is host policy via
construction.

### SDK OAuth (Meta device flow and Model API key mint) (P20-S29)

`auth::oauth::meta` is pinned to pi 1.3.0 `meta.ts` plus the shared
`device-code.ts` polling machine. RFC 8628 against `auth.meta.com` yields an
identity token that never authenticates inference; it is exchanged for a
Model API key at `api.meta.ai/muse-code/key` (Bearer identity, `{}` body,
`x-api-version: 1.0.0`). The credential stores the minted key as `access`
and the identity token as `refresh` with a fixed 24-hour expiry, so
`refresh` re-mints with the same identity - never a refresh_token grant and
never a retry loop. A mint 401/403 (login or refresh) is a typed revoked
session for relogin. The verification URI is the trusted
`verification_uri_complete` falling back to the trusted `verification_uri`;
interval/expires default to 5s/15min; polling waits before the first poll,
floors at 1s, honors slow_down intervals, and times out typed at the device
deadline. The body error code is read regardless of status, matching pi's
switch after its token check. Failure bodies are never echoed; the
sanctioned exception is a trusted http(s) setup `action_url` from a keyless
mint response reaching the presenter reason.

### SDK OAuth (xAI device flow and single-pass refresh rotation) (P20-S30)

`auth::oauth::xai` is pinned to pi 1.3.0 `xai.ts`. RFC 8628 against
`auth.x.ai` with form bodies carrying the fixed client id, the xAI scope,
and `referrer=pi`. Verification URIs are https-only; an untrusted
`verification_uri_complete` fails login rather than falling back to the
plain URI (pi validates before choosing `complete ?? plain`), and the
device authorization requires a positive finite `expires_in` while
`interval` defaults to 5s. Polling branches on ok first (success bodies
need non-empty access and refresh tokens) and maps the RFC 8628 error
codes - including xAI's `authorization_denied` spelling - only on non-ok
answers, under the shared machine (first-poll wait, 1s floor, slow_down
server interval or +5s, device deadline). Token `expires_in` defaults to
3600s. Refresh is a single pass: an omitted `refresh_token` preserves the
stored one (the sdk-contracts row 146 fixture), 401/403 revokes for
relogin, and nothing retries. The stored expiry is the real service
lifetime with the single five-minute correction in `needs_refresh`.
Failure bodies are never echoed; statuses and fixed classes are.

### SDK OAuth (Radius gateway flow and nine-family assembled conformance) (P20-S31)

`auth::oauth::radius` is pinned to pi 1.3.0 `radius.ts`. The gateway is
fixed at construction (https-defaulted, trailing slashes stripped).
`with_identity(id, name)` captures the flow's own identity and display name
while retaining that gateway; the existing constructors default to
`radius` / `Radius`. Models registration keys remain independently chosen
by the caller. The method choice mirrors pi's prompt through `select_login_method` with
browser recommended. Browser mode discovers `GET {gateway}/v1/oauth` for
the authorization endpoint (device mode never discovers first) and runs
the shared PKCE skeleton with the fixed callback
`http://127.0.0.1:1456/oauth/callback` under a required bind, the
`pi-gateway` client, `gateway offline_access` scope, and `handoff=url`. The
browser path accepts only its callback and never starts a manual-input reader.
Device mode posts `/v1/oauth/device` and follows the shared RFC 8628
machine; the verification URI passes through verbatim (no scheme check,
unlike xAI) and denial keeps the single RFC spelling. Non-ok token
answers classify by error code; token responses require the full triple
on every grant; refresh is the shared strict single pass (no §6
preservation) with 401/403 revoked. The stored expiry is the real
service lifetime (pi's 60s write skew not reproduced; the five-minute
correction stays in `needs_refresh`). The assembled test registers all
nine delivered families on one `Models`, shares one store across two
`Models`, and proves per-id ACK isolation and late-write rotation
isolation across sources.


### Connecting SDK OAuth to a request

Attach a `CredentialStore`, then call `Models::register_oauth_route(provider,
flow, compat)`. Registration performs no source I/O. It binds the provider
and login flow to `auth::store_oauth::StoreOAuthResolver`;
`register_oauth_flow` remains available for login-only registration.
`Models::login` and the product `/login` helper share SDK `login_to_store`: all
presenter operations pass through, and success follows the credential-plus-
metadata write ACK. `get_auth` and ordinary calls resolve the selected stored
identity, refresh through the same flow when required, await persistence, and
freeze the returned request identity. `OAuthProvider::to_auth` is I/O-free;
Copilot derives its request endpoint from that acknowledged token. No refresh
failure discovers a different account or source. The 15-second OAuth refresh
network bound excludes queue/lock waiting and persistence ACK.

Offline acceptance follows the real family flows through public login, store
ACK and `get_auth` into the existing Anthropic, Codex, Chat Completions and
Responses adapters, including Copilot's token-selected HTTPS endpoint. Radius
coverage ends at gateway-bound login/refresh and public authentication; its
pi-messages generation adapter and dynamic catalog remain later work. Finite
but unrepresentable source durations fail typed; each flow keeps its own
field requirements, defaults and polling limits. OpenRouter's permanent key
and Anthropic/xAI omitted-refresh behavior remain distinct.

Product constructors now accept the store that owns this shared queue:
`build_harness_collection` takes a final optional `Arc<dyn CredentialStore>`;
`CodingHarnessBuilder::credential_store` attaches the concrete keychain store;
`NonInteractiveRunner::new_with_resume_runtime_packages_and_auth` and
`RpcRunner::new_with_runtime_packages_and_auth` take an optional keychain store
after `auth_resolver`. Production passes the same store used by its resolver,
AWS source cache and ADC owner; `None` supports store-free test assemblies.

AWS nested role profiles retain static-pair precedence, then resolve the
existing web-identity/process/SSO/login source chain before assuming roles from
inside out. SSO caches separate region/account/role/session, endpoint and
source revision. AWS rotations require owned dependencies and the shared
mutation queue; missing ownership fails typed before rotation. Rotated source
material and actual expiry survive best-effort standard-cache file saves.
Receipts acknowledge the source-memory update, not durable file success.
Invalidation fences late memory/file writes; a failed fence does not claim
that the remote rotation was rolled back. Existing login-session Continue
classification is retained for its eligible ordinary refresh failures.


`AdcNativeOwner` implements `AuthResolver` directly. Its `check_configured`
uses the same selected context for environment/file-existence checks without
reading or parsing credential contents, refreshing a token or contacting
metadata. A missing file is unconfigured; a failed existence probe is typed;
a present malformed file is configured until actual resolution validates it.
`AuthResolver::check_configuration` is an optional hook used by `check_auth`
and availability; opaque resolvers retain their source-classification check.
The product Vertex checker preserves stored Express/API-key and configured
Bearer precedence before checking its ADC owner.


Store-backed OAuth resolution selects the current credential inside its
serialized mutation slot, including fresh-token reads, so a lock wait cannot
retain an identity selected before a preceding logout or replacement.
Replacing the attached store or upserting a helper-bound OAuth flow rebinds
new calls to that store/flow; already captured calls retain their frozen
binding. Explicit caller-owned `register_route` resolvers keep their own
ownership.


Windows helpers start suspended, join an owned kill-on-close Job, and resume
only after assignment. Closing that owner also terminates descendants after
the original helper parent has exited. The private Windows owner contains the
required `windows-sys` calls and uses `OwnedHandle` rather than an unsafe
`Send` implementation. The Windows-only `windows-sys` 0.61.2 dependency adds
`Win32_Security` to the existing Job/ToolHelp/Threading feature set, without a
new package version or build script. Unix process groups and the existing AWS
10-second/64-KiB host policy remain unchanged; this change does not claim
complete native-platform acceptance.


`OAuthProvider::{name, is_subscription, login_label}` expose non-secret flow
metadata through `dyn OAuthProvider` without auth or source I/O. Custom
implementations default to `name = id()` and `None` for both optional fields.
The nine SDK flows retain the fixed Pi display values and optional-field
distinctions: OpenRouter and Radius do not declare a subscription flag, and
flows without a login label do not receive a synthesized one. Radius uses
its captured identity for method selection, request provenance and typed
cancellation/revocation while refresh remains bound to its configured gateway
and stored account metadata. Fresh and refreshed Radius request auth both
ignore credential `base_url`; routing authority stays with that gateway. This adds no generic flow inventory or complete
provider/profile enumeration.

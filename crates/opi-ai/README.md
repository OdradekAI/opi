# opi-ai

[![Crates.io](https://img.shields.io/crates/v/opi-ai.svg)](https://crates.io/crates/opi-ai)
[![Docs.rs](https://docs.rs/opi-ai/badge.svg)](https://docs.rs/opi-ai)

> Provider-neutral LLM API for Rust.

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

`opi-ai` provides one request, message and streaming model across built-in
provider families, checked multi-wire adapters and config-driven profiles. It
is a library crate: it does not own an agent loop, sessions, package loading or
coding-agent tools.

Current workspace version `0.8.3`.

## Guide

Use the short path for a new integration. The later links are capability and
reference pages; they keep the first successful call independent from the
implementation detail of every provider.

| Goal | Start here |
| --- | --- |
| Install in an external Cargo project | [Installation](#installation) |
| Choose the default or core feature profile | [Feature profiles](#feature-profiles) |
| Make one ordinary request | [Quick Start](#quick-start) |
| Consume deltas and a final message | [Streaming and the Final Result](#streaming-and-the-final-result) |
| Build context, tools and a continuation | [Context, Tools, and Continuation](#context-tools-and-continuation) |
| Register a model and choose a provider | [Providers](#providers); [Provider, Catalog, Auth and Refresh reference](docs/provider-auth.md) |
| Use transport, utility or provider-specific detail | [Transport](docs/transport.md); [Utilities](docs/utilities.md); [Provider Wire and Operation Reference](docs/providers.md) |
| Test without credentials | [Faux Provider for Tests](#faux-provider-for-tests); [Testing and Package Consumption](docs/testing.md) |
| Understand errors, cancellation, telemetry and deferred calls | [Error Handling](#error-handling); [Telemetry: Explicit Context and Optional Tracing](#telemetry-explicit-context-and-optional-tracing); [Deferred Responses](#deferred-responses) |

## Installation

```sh
cargo add opi-ai
```

For a project that copies all three onboarding examples, use this complete
external consumer manifest:

```toml
[dependencies]
opi-ai = "0.8.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
futures-util = "0.3"
serde_json = "1"
```

The package depends only on `opi-ai` and ordinary public runtime/serialization
crates. It does not require `opi-agent`, the workspace, a repository path or
implicit global initialization.

## Feature profiles

`opi-ai` currently exposes one public feature, `telemetry-tracing`:

| Profile | Cargo spelling | Adds or removes |
| --- | --- | --- |
| Default | `default` / `telemetry-tracing` | Enables `opi-telemetry`'s optional tracing carrier and `TelemetryContext::from_span`. |
| Core context | `default-features = false` | Keeps the backend-neutral context, recorder and AI request types without the optional tracing carrier. |

Consumers that want the core profile can use:

```toml
opi-ai = { version = "0.8.3", default-features = false }
```

`default-features = false` does not remove built-in providers or network
dependencies. It changes only the optional `opi-telemetry` tracing carrier;
provider registration, catalog, request lifecycle and explicit authentication
remain the same. Exporter setup, sampling, initialization and shutdown remain
host-owned.

## API at a glance

The public surface is organized around caller-owned values and one execution
driver:

| Goal | Public entry points | Responsibility |
| --- | --- | --- |
| Register and select models | `Models` / `ProviderRegistry` / `register_builtin` | Build a catalog or explicit route without reading authentication during construction. |
| Describe one call | `Request` / `ChatModelRecord` | Carry normalized context, model identity, limits, cancellation and optional telemetry. |
| Complete one call | `Models::complete_simple` | Drive execution to one final `AssistantMessage` or typed `CallFailure`. |
| Consume a stream | `ChatCall` / `AssistantStreamEvent` / `Models::stream_simple` | Observe deltas, terminal state, cancellation and the final message. |
| Build context and tools | `normalize_context` / `ToolDef` / `transcript` | Normalize system text, messages and tool declarations into canonical chronology. |
| Observe execution | `TelemetryContext` / `telemetry-tracing` | Inject explicit context and opt into the `opi-telemetry` tracing carrier. |

Exact signatures, trait bounds and exhaustive field descriptions belong to
[rustdoc](https://docs.rs/opi-ai).

## Reference map

Use these links after the first successful call. Each topic is package-local and
ships with the crate.

| Capability | Reference |
| --- | --- |
| Provider selection, catalog, authentication and refresh | [Provider, Catalog, Auth and Refresh reference](docs/provider-auth.md) |
| Estimation, pricing tiers, cache pricing and reported cost | [Cost and pricing reference](docs/cost-pricing.md) |
| Proxy, TLS, WebSocket, deferred transport and cleanup | [Transport, Protocols and Resource Lifecycle](docs/transport.md) |
| Schema, transcript, replay, identity, retry and callbacks | [Public Utilities and Data Contracts](docs/utilities.md) |
| Provider-specific wire and operation behavior | [Provider Wire and Operation Reference](docs/providers.md) |
| Faux, fixture ownership and package proof | [Testing and Package Consumption](docs/testing.md) |

## Quick Start

One complete ordinary call uses an injected `AuthContext`, a caller-owned
`ChatModelRecord`, canonical transcript normalization and the normal
`complete_simple` result contract. This is the exact source of
`examples/quick_start.rs`:

```rust
//! Complete ordinary quick start: register lazy builtins, select a model,
//! and drive one chat call to its single final message through the neutral entry.
//!
//! Compile: `cargo check --example quick_start`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example quick_start`.

use std::future::Future;
use std::sync::Arc;

use opi_ai::CallFailure;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript;
use opi_ai::{AmbientAuthContext, AuthContext, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

/// Construct and statically select without reading authentication sources.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<(Models, ChatModelRecord), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    Ok((models, ChatModelRecord::new("anthropic", info)))
}

/// Schedule the owned call on Tokio; setup does not wait for result consumption.
pub(crate) fn start(
    models: &Models,
    record: ChatModelRecord,
) -> Result<
    impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<>,
    Box<dyn std::error::Error>,
> {
    // Canonical transcript input: the system prompt and tool
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
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5".into(),
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

    Ok(models.complete_simple(record, request, SimpleOptions::default()))
}

pub(crate) async fn finish(
    call: impl Future<Output = Result<AssistantMessage, CallFailure>>,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    let message = call
        .await
        .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;
    for content in &message.content {
        if let AssistantContent::Text { text, .. } = content {
            println!("{text}");
        }
    }
    Ok(message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, record) = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = start(&models, record)?;
    finish(call).await?;
    Ok(())
}
```

The constructor does not read product configuration or contact a provider.
Authentication is injected through the selected route; the call returns one
`Ok(AssistantMessage)` or one typed `Err(CallFailure)`.

## Streaming and the Final Result

`Models::stream_simple` returns a `ChatCall`. The call schedules execution at
construction, while the consumer controls event polling. `TextDelta` and other
partial events are followed by one terminal `Done`, `Error` or `Aborted` event;
`ChatCall::result()` waits for the same final `AssistantMessage` or
`CallFailure`. `Pending` is not a terminal result.

After a business terminal, ordinary calls drain the provider source to EOF before
forwarding the terminal, settling request telemetry and completing the result.
Tail events cannot replace the first outcome or trigger another attempt. Stopping
consumption or dropping the call does not stop this work. An unending source tail
keeps the call pending: request-token cancellation requires source cooperation,
while `ChatCall::abort()` stops and joins the execution task. This does not join
independent tasks spawned inside a custom provider.

The exact streaming onboarding program is `examples/quick_start_stream.rs`:

```rust
//! Streaming quick start: iterate the neutral entry's provider-neutral
//! events as they arrive, then read the terminal event's final message.
//!
//! Compile: `cargo check --example quick_start_stream`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example quick_start_stream`.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript;
use opi_ai::{AmbientAuthContext, AuthContext, ChatCall, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

/// Construct and statically select without reading authentication sources.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<(Models, ChatModelRecord), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    Ok((models, ChatModelRecord::new("anthropic", info)))
}

/// Schedule the ChatCall on Tokio before consuming its events.
pub(crate) fn start(
    models: &Models,
    record: ChatModelRecord,
) -> Result<ChatCall, Box<dyn std::error::Error>> {
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
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5".into(),
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

    Ok(models.stream_simple(record, request, SimpleOptions::default()))
}

pub(crate) async fn finish(
    mut call: ChatCall,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    // Done carries the final message; after it the call stays at EOF.
    let mut final_message = None;
    while let Some(event) = call.next().await {
        match event.map_err(|failure| format!("call failed: {:?}", failure.cause()))? {
            AssistantStreamEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantStreamEvent::Done { message, .. } => {
                println!();
                eprintln!("-- stop reason: {:?}", message.stop_reason);
                final_message = Some(message);
            }
            _ => {}
        }
    }
    final_message.ok_or_else(|| "stream ended without a final message".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, record) = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = start(&models, record)?;
    finish(call).await?;
    Ok(())
}
```

Starting a call and consuming it are separate operations. Dropping a consumer
does not implicitly cancel the underlying execution; use the request's
cancellation token when cancellation is intended. Transport and cleanup
ownership are described in [Transport, Protocols and Resource Lifecycle](docs/transport.md).

## Context, Tools, and Continuation

`transcript::normalize_context` produces the canonical chronology used by every
adapter. System text, messages and tool declarations are normalized once;
continuation rounds replay the full chronology with assistant tool calls and
`ToolResultMessage` results.

The caller executes tools and appends their results. A malformed final argument
is recorded as not executed through `ToolNotExecutedReason`; the SDK does not
guess a partial call. This is the exact source of `examples/tool_loop.rs`:

```rust
//! Agent-owned tool loop: declare a tool through the canonical transcript,
//! execute each admitted tool call, close it with a `ToolResultMessage`, and
//! continue until the assistant produces final text.
//!
//! Compile: `cargo check --example tool_loop`.
//! Run: `ANTHROPIC_API_KEY=... cargo run --example tool_loop`.

use std::sync::Arc;

use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall, ToolDef,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript::{self, TranscriptContext};
use opi_ai::{AmbientAuthContext, AuthContext, ChatCall, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

const MODEL_SPEC: &str = "anthropic:claude-sonnet-4-5";

struct ToolOutcome {
    text: String,
    is_error: bool,
    not_executed: Option<ToolNotExecutedReason>,
}

/// The caller is the trusted execution boundary: a malformed final
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
        projection: Default::default(),
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

pub(crate) struct ToolLoop {
    pub(crate) models: Models,
    record: ChatModelRecord,
    history: Vec<Message>,
    tools: Vec<ToolDef>,
}

/// Construct and select statically; the first round is built separately.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<ToolLoop, Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Tools are declared on the transcript context; normalization folds them
    // into the leading system message.
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

    let history: Vec<Message> = vec![Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "What is the weather in Tokyo? Use the tool.".into(),
        }],
        timestamp_ms: 0,
    })];

    Ok(ToolLoop {
        models,
        record,
        history,
        tools,
    })
}

impl ToolLoop {
    /// Freeze this round and schedule its authentication and execution on Tokio.
    pub(crate) fn start_round(&self) -> Result<ChatCall, Box<dyn std::error::Error>> {
        let context = transcript::normalize_context(transcript::Context {
            system_prompt: Some("You are a concise assistant.".into()),
            messages: self.history.clone(),
            tools: self.tools.clone(),
        })?;
        Ok(self.models.stream_simple(
            self.record.clone(),
            request_for(context),
            SimpleOptions::default(),
        ))
    }

    pub(crate) async fn finish(
        mut self,
        mut call: ChatCall,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        // Each round replays the full canonical chronology, including tool results.
        for round in 0..8 {
            let assistant = call
                .result()
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
                return Ok(assistant);
            }
            self.history.push(Message::Assistant(assistant));
            for tool_call in tool_calls {
                let outcome = execute_weather_tool(&tool_call);
                self.history.push(Message::ToolResult(ToolResultMessage {
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
            if round == 7 {
                break;
            }
            call = self.start_round()?;
        }
        Err("tool loop did not converge".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = state.start_round()?;
    state.finish(call).await?;
    Ok(())
}
```

Schema, replay, transcript, header and callback details are in [Public Utilities
and Data Contracts](docs/utilities.md).

## Providers

`Models` and `ProviderRegistry` select a model or an explicit provider route.
`register_builtin` installs the validated built-in profiles and catalog; it does
not imply login, network access or ambient configuration reads. The detailed Provider, Catalog, and authentication reference is in [Provider, Catalog, Auth and Refresh reference](docs/provider-auth.md). The credential, OAuth, request-enrichment, and refresh lifecycle is maintained there as well.

The provider matrix, compatibility profiles, images, classification, native
options and direct adapters are in [Provider Wire and Operation Reference](docs/providers.md).

## Image Support

Image input uses ordered `InputContent::Image` values with explicit media type
and source metadata. Image generation is a separate operation with its own
model record, context, output and typed result. See [Provider Wire and Operation
Reference](docs/providers.md) for route-specific limits.

## Classification

Classification is a separate operation and catalog type. The public result
maps provider answers to `ClassifierOutput`; missing, malformed, refused and
non-finite answers are typed failures. Decisions, System One and llama.cpp use
separate wire envelopes.

## Thinking/Reasoning

The simple path carries `ThinkingLevel`; native adapters expose typed options
when a provider has additional reasoning controls. Omitted values remain
omitted when the provider distinguishes omission from zero or false. Thinking
and redacted-thinking blocks keep their order and are not promoted to visible
text when the route cannot preserve them.

Stop reasons normalize to `Stop`, `Length`, `ToolUse`, `Error`, `Aborted`,
`Deferred` and `Pending`. Provider raw reasons remain available through typed
failures or adapter diagnostics.

## Error Handling

Provider, protocol, authentication, schema, callback and cancellation failures
are represented by typed `CallFailure`/`ProviderError` causes. Error summaries
are bounded and redacted; response bodies and secrets are not echoed. Unknown
models, unsupported options and invalid tool/schema combinations fail before a
successful provider event is published.

Cancellation is explicit and observable. An abort, a timeout, a provider
terminal error and stopping consumption are distinct outcomes. Retry policy is
owned by the logical call or provider adapter; it never turns an explicit abort
or callback failure into a resend. Usage and best-effort pricing details are in
[Cost and pricing reference](docs/cost-pricing.md).

## Best-Effort Cost

Cost is best-effort. Unknown usage or missing pricing yields an unknown cost;
known free rates remain an explicit zero. The detailed estimator, sampling, tier selection,
cache pricing, service-tier and provider-reported-cost rules are in the [Cost and
pricing reference](docs/cost-pricing.md).

## Telemetry: Explicit Context and Optional Tracing

Pass an explicit `TelemetryContext` on `Request` when the host wants AI request
observation. The default `telemetry-tracing` feature supplies the optional
tracing carrier; `default-features = false` keeps the neutral context and
recorder types. The host owns exporter, sampling, initialization and flush.

Ordinary chat requests produce `pi.ai.request`; deferred fetch produces
`pi.ai.poll`. Pending and deferred polls do not accumulate token or cost data.
Raw streams and prepared raw attempts remain available without implicit ambient
context. Full lifecycle and resource rules are in [Transport, Protocols and
Resource Lifecycle](docs/transport.md).

## Custom Providers

A custom provider can implement the typed `Provider`, `ChatApi`, image,
classifier or deferred operation contracts and register it with `Models`.
`CompatConfig` is an explicit profile override; it cannot add an unsupported
wire capability or replace provider-managed authentication. Direct adapter
options carry their `ApiId` and typed option identity.

Use [Provider Wire and Operation Reference](docs/providers.md) for current
operation boundaries and rustdoc for exact trait signatures.

## Faux Provider for Tests

Use `test_support::MockProvider` and the Faux registration helpers for
credential-free deterministic tests. Script response, stream, tool-call,
pending and failure cases explicitly. Faux tests prove caller behavior and
lifecycle wiring, not a real provider's availability or authentication.

The test-layer and external-package rules are in [Testing and Package
Consumption](docs/testing.md).

## Deferred Responses

A deferred submit may return `StopReason::Deferred`, empty content and a typed
handle. `Models::fetch_deferred` drives a later `ChatStream` to one ready result
or one typed failure; `cancel_deferred` is best-effort remote cancellation and
is independent from local fetch cancellation. `poll_after_ms` is a provider
hint, not an implicit sleep policy.

No startup fetch, global deferred worker or real remote service is implied by
the deterministic Faux route. See [Transport, Protocols and Resource Lifecycle](docs/transport.md).

## Migration

The public Rust surface is unstable 0.x. Prefer the current `Models`, `Request`,
`ChatCall`, `ProviderRegistry`, `AuthContext`, `TelemetryContext` and typed
operation contracts. When moving from an internal workspace caller, replace
implicit global setup with explicit provider registration and injected auth;
replace workspace-only paths with the published crate and package-local topic
links.

## Development

The package examples are the canonical onboarding programs and are executed
from an ordinary external consumer. Maintainers should use [Testing and Package
Consumption](docs/testing.md), while users needing exact types should use
[rustdoc](https://docs.rs/opi-ai). Transport, utilities and provider details
remain available through the package-local references in the [Reference map](#reference-map).

## License

MIT. See the packaged [LICENSE](LICENSE) and [NOTICE](NOTICE).

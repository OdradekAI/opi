# Phase 12 Provider Correctness Design

Historical note: this design was originally drafted as Phase 10. After the
`.repo/pi-0.80.2` baseline review, Phase 10 became core architecture
deepening. Provider correctness is now Phase 12 and should test through the
Phase 10 `Models/Auth` seam where that seam exists.

## Overview

Phase 12 improves correctness for the provider adapters that already exist in
opi. The phase focuses on wire-format fidelity, streaming lifecycle, tool call
conversion, image handling, thinking/reasoning support, usage and cost mapping,
retry behavior, proxy handling, OpenAI-compatible profile compatibility, and
provider error classification.

This is not a provider breadth phase. Adding many new providers, OAuth flows,
or subscription login support belongs to a later ecosystem or product phase.

## Goals

- Strengthen fixture coverage for every existing provider family.
- Normalize provider error taxonomy and diagnostic mapping.
- Verify streaming lifecycle events across text, thinking, tool calls, usage,
  finish reasons, cancellation, and errors.
- Confirm image input and image output behavior only where existing provider
  support claims it.
- Improve auth and endpoint diagnostics without adding new credential stores.
- Keep OpenAI-compatible provider breadth config-driven where the wire protocol
  does not require a first-class adapter.
- Deepen OpenAI-compatible profile correctness for role mapping, token fields,
  usage-in-streaming, strict/tool-result behavior, cache control, response IDs,
  and session affinity.
- Make provider behavior visible through Phase 7 diagnostics and traces.

## Non-Goals

- No OAuth login flows.
- No Anthropic subscription auth.
- No OpenAI Codex subscription auth.
- No GitHub Copilot auth.
- No broad new first-class provider list.
- No image generation feature.
- No browser usage feature.
- No provider streaming adapter protocol for packages.
- No paid live provider calls in default tests.
- No provider-specific config file format copied from pi.

## Relationship to pi

Pi supports a broad provider catalog and multiple auth strategies. Opi should
preserve the provider-agnostic runtime model but should not chase catalog width
before correctness. Rust's advantage here is precise fixture tests and explicit
error types.

First-class providers should be added only when:

- the wire protocol materially differs;
- auth semantics materially differ and are approved by a product design;
- model capability mapping cannot be represented by an existing profile.

OpenAI-compatible services should remain config-driven profiles unless they
need special streaming, tool, image, or auth behavior.

This includes compatibility flags rather than one-off adapters for provider
differences such as developer-role support, reasoning effort, usage in
streaming, strict tool schema mode, token field names, tool-result name
requirements, assistant-after-tool-result requirements, cache control format,
thinking format, response IDs, and session affinity headers.

## Architecture

| Module | Crate | Responsibility |
|---|---|---|
| Provider trait | `opi-ai` | Stable request/stream/capability contract |
| Provider adapters | `opi-ai` | Wire-specific serialization, streaming, usage, and errors |
| Provider collection / registry | `opi-ai` | Model/provider resolution, compatibility metadata, and provider-owned auth semantics from Phase 10 |
| Provider construction | `opi-coding-agent` | Config, env, proxy, packages, and product runtime wiring into the `opi-ai` seam |
| Diagnostics | `opi-ai`, `opi-coding-agent` | Safe provider error and config reporting |
| Tests | `opi-ai/tests`, `opi-coding-agent/tests` | Fixture and wiring coverage |

No new crate is needed.

## Provider Contract Areas

### Streaming Lifecycle

Each provider should be tested for:

- stream start;
- text delta;
- text end;
- thinking start/delta/end where supported;
- tool call start/delta/end where supported;
- usage update;
- finish reason;
- stream done;
- stream error after partial output;
- cancellation.

The provider adapter should map protocol-specific events into the shared
`opi-ai` stream model without losing enough information for the agent runtime
to make correct decisions.

### Tool Calls

Fixture tests should cover:

- no tool calls;
- one complete tool call;
- multiple tool calls;
- streamed or chunked arguments;
- malformed JSON arguments;
- provider-specific tool call IDs;
- tool result conversion back to provider messages.

Invalid tool arguments should become runtime tool validation errors, not
provider panics.

### Thinking and Reasoning

For providers that support thinking/reasoning:

- validate request fields;
- validate budget/max-token interactions;
- preserve thinking content in assistant content where expected;
- expose unsupported thinking through capability diagnostics.

For providers that do not support thinking, capability checks should reject or
clamp requests before the provider call where possible.

### OpenAI-Compatible Profile Correctness

Config-driven OpenAI-compatible profiles should have fixture coverage for:

- system versus developer role mapping;
- reasoning effort and provider-specific thinking formats;
- `store` and strict tool schema support;
- usage chunks in streaming responses;
- `max_tokens` versus `max_completion_tokens` request fields;
- tool-result name requirements;
- assistant-after-tool-result requirements;
- cache control formatting;
- response ID extraction and propagation where the provider returns one;
- session affinity headers where a compatible provider requires them;
- provider-level and model-level override precedence;
- unsupported capability diagnostics that fail before a bad request where
  possible.

If a profile requires behavior outside these flags, Phase 12 should document
why it needs a first-class adapter or defer it as future provider breadth.

### Images

For providers that support image input:

- verify MIME handling;
- verify base64 serialization;
- verify image size/count limits where configured;
- verify unsupported image diagnostics.

Image generation is not part of Phase 12.

### Usage and Cost

Normalize:

- input tokens;
- output tokens;
- cache read/write tokens where provider supplies them;
- total tokens;
- best-effort cost mapping;
- missing usage behavior;
- provider response IDs when supplied;
- cache retention or session-affinity metadata when supplied.

Cost should remain best-effort. Incorrect confidence is worse than explicit
unknown values.

### Retry and Rate Limits

Provider retries should be fixture-tested for:

- retryable HTTP status codes;
- retry-after headers where supported;
- stream errors that should not retry after partial content;
- non-retryable auth and validation failures;
- cancellation during backoff.

Retry events should feed Phase 7 diagnostics and traces.

## Error Taxonomy

Provider errors should classify into:

| Class | Examples |
|---|---|
| auth | missing API key, invalid token, expired credentials |
| config | invalid endpoint, unsupported model, bad profile |
| request | schema/validation error before request |
| network | DNS, TLS, proxy, timeout |
| rate_limit | retryable or terminal rate limits |
| provider | provider-side 4xx/5xx with safe body excerpt |
| stream | malformed SSE/event stream, unexpected terminal event |
| capability | unsupported image, tool, or thinking request |
| cancelled | user or runtime cancellation |

Do not log secrets from headers, URLs, environment variables, or response
bodies.

## Data Flow

```text
config/model selection
  -> provider registry
  -> provider request
  -> wire adapter
  -> provider stream events
  -> shared stream events
  -> agent loop
  -> diagnostics and trace
```

## Testing Strategy

| Level | Coverage |
|---|---|
| fixture | request body, stream parsing, error parsing per provider |
| lifecycle | provider trait contract for start/delta/end/done/error |
| wiring | config env/proxy/model registry in `opi-coding-agent` |
| retry | retryable/non-retryable cases and cancellation during backoff |
| capability | image/tool/thinking support and rejection paths |
| compat profile | OpenAI-compatible flags, request fields, streaming usage, cache/session behavior, and tool-result constraints |
| docs guard | docs do not claim OAuth, subscription auth, or unsupported provider breadth |

Live provider tests remain ignored and environment-gated.

## Documentation Updates

Update provider docs to clarify:

- supported providers and protocols;
- auth method per provider;
- OpenAI-compatible profile policy;
- OpenAI-compatible profile flags, cache control, response IDs, and session
  affinity behavior;
- thinking/image/tool support by provider where known;
- proxy behavior;
- cost is best-effort;
- OAuth, subscription auth, image generation, and broad provider catalog
  expansion are deferred product decisions.

## Success Criteria

Phase 12 is complete when:

1. Every existing provider has fixture coverage for request serialization,
   streaming lifecycle, and error mapping.
2. Provider errors map into a documented taxonomy.
3. Tool call conversion is tested for providers that support tools.
4. Thinking and image capability checks are tested and documented.
5. Retry behavior is covered without live network calls.
6. Config-driven OpenAI-compatible provider profiles have fixture coverage for
   compatibility flags, cache/session behavior, and tool-result constraints.
7. Config-driven OpenAI-compatible provider profiles remain the preferred path
   for compatible provider breadth.
8. Phase 7 diagnostics and traces include provider error class and safe
   metadata.
9. No OAuth, subscription auth, image generation, or broad provider catalog
   expansion is added.

## Phase 13 Handoff

Phase 13 should rely on provider-correct usage, model, thinking, and error data
when deepening sessions. Session metadata should record what users need to
understand past work without depending on provider-specific internals.

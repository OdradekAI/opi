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

## Image Operation

Image input uses ordered `InputContent::Image` values with explicit media type
and source metadata. Image generation is a separate operation with its own
`ImageModelRecord`, `ImagesContext`, `ImagesOutput` and typed result. A provider
failure is returned as a typed result failure; it is not silently converted to
chat content. The OpenRouter image adapter applies the shared timeout, retry,
callback and cancellation boundaries.

## Classification Operation

Classification is a separate operation and model catalog type. The public
classifier adapters preserve the provider's answer shape while mapping it to
`ClassifierOutput`; missing, malformed, refused and non-finite answers are
reported as typed failures. The Decisions, System One and llama.cpp routes use
separate wire envelopes and do not share undocumented assumptions about score,
choice or boolean semantics.

## Reasoning and Native Options

The simple chat path carries a `ThinkingLevel`; native adapters may expose
provider-specific reasoning options through their typed options. Omitted values
remain omitted where the provider distinguishes omission from zero or false.
Streaming thinking and redacted-thinking blocks retain their order and are not
promoted to visible text when the route cannot preserve them.

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

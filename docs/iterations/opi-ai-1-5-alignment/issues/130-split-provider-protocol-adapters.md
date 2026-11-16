# #130 Split provider protocol adapters

Status: completed for the current boundary; finer per-provider and backend
splits remain open.

## Decision

`opi-ai` now exposes `protocol-adapters` as an independently selectable Cargo
feature. It owns the built-in provider wire modules (Anthropic, Azure/OpenAI,
Bedrock, Gemini, Mistral, OpenRouter, Pi Messages, Vertex and Codex Responses)
and the protocol-only dependency closure: shared HTTP transport, rustls/AWS-LC
certificate wiring, AWS signing/XML/query codecs, P-256/SHA-1 helpers and
WebSocket compression support. `builtins` transitively selects
`catalog-bundled` and `protocol-adapters`, so default and explicit-builtins
callers retain their previous surface.

The bundled catalog remains a separate `catalog-bundled` owner. Direct protocol
callers can use `protocol-adapters` without the catalog and can provide their
own model, authentication and catalog wiring. `provider_collection` and its
registration helpers remain builtins-only because they register the bundled
catalog and provider-owned factories.

`auth::store_oauth` and neutral credential/value contracts stay in the core.
Anthropic federation and the source-owned base-URL helper remain available to
direct adapters. Built-in OAuth, AWS, ADC and Vertex credential-source
factories remain behind `builtins`; this prevents protocol-only callers from
silently acquiring platform credential discovery or provider collection state.

## Evidence

- Protocol-only native contract tests cover direct provider modules, neutral
  OAuth/store types and WebSocket helpers. Warnings-denied Clippy is run with
  the protocol feature and test targets.
- The external package runner consumes the actual `opi-ai` archive with
  `default-features = false, features = ["protocol-adapters"]`, verifies the
  exact feature union (`http-transport`, `protocol-adapters`), rejects the
  bundled catalog package, checks the protocol dependency owners and executes
  the direct adapter/auth-store/WebSocket calls outside the workspace.
- Default and explicit-builtins HTTP compatibility and the existing builtins
  HTTP acceptance suite remain green. A repository caller inventory records
  direct adapter, provider collection, auth/OAuth-store and WebSocket users so
  the feature boundary is not inferred from an empty external-reference scan.
- The frozen Pi 1.5.0 pairings and prior catalog/HTTP receipts remain
  immutable. This issue adds a new capability-V revision and keeps the
  protocol package receipt and native logs as current evidence.

Evidence: [capability V v4](../evidence/phase8-capability-v-20261115-v4.json),
[combined native/package/caller receipt](../evidence/phase8-protocol-adapters-20261115.json),
and [current package receipt](../evidence/phase8-protocol-adapters-package-20261115-v3.json).

## Deliberate scope and remaining work

This split does not claim a complete capability-granular V. Per-provider
features, TLS backend selection, WebSocket/compression ownership, and provider
OAuth/store integration still have no independent Cargo owners. macOS native
verification remains deferred because no runner is available. These are the
next capability slices; no provider behavior change is implied by this feature
boundary.

Test impact: add the protocol feature/profile, package runner and caller
inventory; update feature documentation and capability mapping; delete none;
retain all historical Pi, adapter/observer, catalog and HTTP receipts.

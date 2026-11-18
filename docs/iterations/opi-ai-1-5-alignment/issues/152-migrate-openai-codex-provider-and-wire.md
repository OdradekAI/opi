# #152 Split the OpenAI Codex profile from the Responses wire adapter

Status: implemented; canonical namespaces enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the legacy `src/openai_codex_responses.rs` owner into explicit canonical
namespaces without retaining a compatibility module:

- `opi_ai::providers::openai_codex` owns the Codex provider identity, default
  endpoint, model set, shared HTTP client and provider construction state.
- `opi_ai::api::openai_codex_responses` owns the Codex request projection,
  native options, SSE/WebSocket transport, resource cleanup, stream mapping,
  status classification and typed `CodexChatApi`.

The provider-scoped WebSocket pool remains an implementation detail of the
canonical API adapter. `codex_resources` continues to reference that opaque
transport state through the API namespace because leases are shared by the
Codex resource lifecycle and the wire transport.

## Ownership boundary

The Provider profile contains route identity and construction state. Wire
serialization, native option projection, HTTP/WebSocket execution, stream
mapping, error classification, deferred/resource cleanup and `ChatApi`
implementation live in the API module. The migration must preserve Codex
authentication, account-id handling, session affinity, WebSocket pooling,
SSE fallback, retry classification and telemetry behavior.

## Acceptance

1. `OpenAiCodexResponsesProvider` lives under
   `src/providers/openai_codex.rs` and remains the provider route type.
2. `CodexOptions`, native enums, `CodexChatApi`, request/stream code and
   WebSocket transport live under `src/api/openai_codex_responses.rs` and its
   canonical private submodules.
3. `src/openai_codex_responses.rs` and the old
   `opi_ai::openai_codex_responses` module are deleted; no compatibility shim
   remains.
4. `codex_resources`, provider collection, coding-agent consumers, tests,
   conformance scripts and active documentation use canonical profile/API
   paths.
5. Codex HTTP/WebSocket/native-options/resource tests, provider protocol
   tests, coding-agent consumers and an external consumer built from the
   actual crate archive pass.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The implementation creates `providers::openai_codex` and
`api::openai_codex_responses`, moves the native and WebSocket submodules below
the API namespace, updates the shared resource lifecycle and removes the root
module. The old root module and private submodules are absent; no compatibility
shim remains.

Verification completed:

- `cargo check -p opi-ai --locked --no-default-features --features provider-openai-codex-responses --lib`: PASS.
- Codex/shared integration set (`openai_codex_responses`, native options,
  telemetry, resource cleanup, service tier, capture/replay, OAuth/request
  enrichment/auth, transcript contract, external SDK and provider collection):
  367 passed, 0 failed.
- `opi-coding-agent` `provider_factory`: 58 passed, 0 failed.
- `cargo clippy -p opi-ai --locked --all-targets --features builtins -- -D warnings -A clippy::too_many_arguments`: PASS.
- Provider protocol conformance: 10 passed, 0 failed; canonical caller scan
  reports 33 Codex files and 151 matches under the canonical owner modules.
- Documentation, formatting, whitespace and Python compilation checks: PASS.
- External consumer receipt: [issue-152-openai-codex-consumer-20261117-r1.json](../evidence/issue-152-openai-codex-consumer-20261117-r1.json).
  The actual `opi-ai-0.8.3.crate` built and ran with the exact Codex feature
  closure (`http-transport`, `provider-openai-codex-responses`,
  `provider-openai-responses-shared`, `provider-protocol-common`, `tls-backend`,
  `websocket-compression`).

## Test impact

`add`: canonical profile/API assertions, root deletion coverage and external
consumer receipt; `update`: module declarations, resource lifecycle, callers,
tests, scripts and active docs; `delete`: obsolete root module and old private
submodules; `retain`: Codex wire behavior, shared response/resource
infrastructure and historical snapshots/evidence as audit records.

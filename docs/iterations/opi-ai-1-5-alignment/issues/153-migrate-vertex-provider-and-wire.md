# #153 Split the Vertex profile from the Gemini-compatible wire adapter

Status: implemented; canonical namespaces enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the legacy `src/vertex.rs` owner into explicit canonical namespaces
without retaining a compatibility module:

- `opi_ai::providers::vertex` owns Vertex identity, project/location routing,
  endpoint defaults, model catalog, shared HTTP client and construction state.
- `opi_ai::api::vertex` owns the Gemini-compatible request projection, Vertex
  URL/auth wire, SSE transport and mapping, status classification and typed
  `GoogleVertexChatApi`.

Vertex continues to reuse the canonical Gemini body engine and SSE event
mapper. That reuse is a wire implementation dependency; it does not make
Vertex a second public root module or merge the two provider profiles.

## Ownership boundary

The Provider profile contains route identity, project/location and Express
mode, model records, client and the delegated Gemini construction state used
by native options. Wire URL construction, auth headers, request serialization, HTTP streaming, SSE event
mapping, error classification and `Provider`/`ChatApi` implementations live in
the API module. Existing ADC, API-key Express, quota-project, model override,
telemetry and callback behavior must remain unchanged.

## Acceptance

1. `VertexProvider` and `model_catalog` live under
   `src/providers/vertex.rs`.
2. `GoogleVertexOptions`, `GoogleVertexChatApi`, request/stream/status code and
   Vertex wire helpers live under `src/api/vertex.rs`.
3. `src/vertex.rs` and the `opi_ai::vertex` module are deleted; no compatibility
   shim remains.
4. Provider collection, coding-agent consumers, tests, conformance scripts and
   active documentation use canonical profile/API paths.
5. Vertex fixture/lifecycle/auth/profile tests, coding-agent consumers and an
   external consumer built from the actual crate archive pass.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The implementation creates `providers::vertex` and `api::vertex`, moves the
Vertex profile construction state away from the Gemini-compatible wire code,
updates canonical callers and removes the root module. The canonical API keeps
the existing Gemini body/event engine as an internal wire dependency; no root
module or compatibility shim remains.

- Minimal owner check with `provider-vertex`: PASS.
- Vertex/Gemini integration set (`vertex_fixtures`, `gemini_lifecycle`, model
  metadata, ADC ownership, error classes, protocol adapter/profile,
  per-request auth, tool-result wire and transcript contract): 194 passed,
  0 failed.
- `opi-coding-agent` Vertex wiring and provider factory: 73 passed, 0 failed.
- Provider collection regression: 111 passed, 0 failed.
- Provider protocol conformance: 11 passed, 0 failed; canonical caller scan
  reports 23 Vertex files and 59 matches, with four canonical Gemini/Vertex
  shared-owner files.
- `cargo clippy -p opi-ai --locked --all-targets --features builtins -- -D warnings -A clippy::too_many_arguments`: PASS.
- Documentation, formatting, whitespace and Python compilation checks: PASS.
- External consumer receipt: [issue-153-vertex-consumer-20261117-r2.json](../evidence/issue-153-vertex-consumer-20261117-r2.json).
  The actual `opi-ai-0.8.3.crate` built and ran with the exact Vertex feature
  closure (`cloud-auth`, `http-transport`, `provider-gemini`,
  `provider-protocol-common`, `provider-vertex`, `tls-backend`).
- The broader `protocol-adapters` external consumer also built and ran after
  its feature-union guard was aligned with the current Cargo compatibility
  group: [issue-153-protocol-adapters-consumer-20261117-r3.json](../evidence/issue-153-protocol-adapters-consumer-20261117-r3.json).

## Test impact

`add`: canonical profile/API assertions, root deletion coverage and external
consumer receipt; `update`: module declarations, callers, tests, scripts and
docs, including the public module-set baseline; `delete`: obsolete root
module; `retain`: Gemini shared body/SSE engine, Vertex wire behavior and
historical snapshots/evidence as audit records.

# #151 Split the OpenAI Responses Provider profile from the wire adapter

Status: implemented; canonical namespaces enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the legacy `src/openai_responses.rs` owner into explicit canonical
namespaces without retaining a compatibility module:

- `opi_ai::providers::openai_responses` owns the OpenAI Responses identity,
  endpoint, model catalog, compatibility profile, shared HTTP client and
  per-dispatch state.
- `opi_ai::api::openai_responses` owns request projection, SSE/background
  decoding, HTTP transport, status/error mapping, deferred operations and the
  typed native Chat API.

The shared Responses mapper remains a crate-private, wire-neutral implementation
detail in `src/api/shared/openai_responses.rs`; Azure and Codex adapters reuse it
through the canonical API/shared namespace.

## Ownership boundary

The Provider profile contains identity and configuration state only. Wire
serialization, request execution, stream/event mapping, deferred operation
handling and the `Provider`/`ChatApi` implementations live in the API module.
No auth, catalog, retry, stream terminal or telemetry behavior changes are
intended.

## Acceptance

1. `OpenAiResponsesProvider`, `ResponsesConfig`,
   `ResponsesAffinityPolicy` and `model_catalog` live under
   `src/providers/openai_responses.rs`.
2. Responses request/stream/deferred code, native options and
   `OpenAiResponsesChatApi` live under `src/api/openai_responses.rs`.
3. `src/openai_responses.rs` and the `opi_ai::openai_responses` module are
   deleted; no compatibility shim remains.
4. Azure, Codex, provider collection, coding-agent consumers, tests, scripts
   and active documentation use canonical profile/API paths.
5. Responses fixture/lifecycle/affinity tests, provider protocol tests,
   coding-agent consumers and an external consumer built from the actual crate
   archive pass.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The implementation creates `providers::openai_responses` and
`api::openai_responses`, updates the shared Azure/Codex/provider-collection
callers and removes the root module.

- Minimal owner check with `provider-openai-responses`: PASS.
- Focused Responses and shared regression tests: PASS — 190 tests across
  fixtures, lifecycle, request enrichment, service-tier cost, wire metadata,
  capture/replay and the profile contract; `external_sdk` 11 tests and
  `provider_collection` 111 tests also pass.
- `opi-coding-agent` provider-factory consumer: PASS — 58 tests.
- Provider protocol Python harness: PASS — 9 tests. Documentation checker,
  `cargo fmt --all -- --check`, Python bytecode compilation and whitespace
  check: PASS. `opi-ai` clippy with `-D warnings` (allowing the repository's
  existing `too_many_arguments` lint): PASS.
- Canonical caller scan must report both profile/API owners and no active
  `opi_ai::openai_responses`, `crate::openai_responses` or root-file
  dependency. PASS — owner scan reports 41 files and 115 matches for the
  Responses owner, with five shared Responses callers; the strict old-path
  scan only finds the negative deletion assertion and this migration record.
- External consumer receipt: PASS —
  [issue-151-openai-responses-consumer-20261117-r1.json](../evidence/issue-151-openai-responses-consumer-20261117-r1.json)
  packages the actual `opi-ai` archive and builds/runs both canonical
  namespaces with the exact `provider-openai-responses` feature closure.

## Test impact

`add`: canonical profile/API assertions and external consumer receipt;  
`update`: module declarations, shared callers, tests, scripts and docs;  
`delete`: obsolete root module/file;  
`retain`: shared Responses mapper and historical snapshots/evidence as audit
records. No commit or stage is performed.

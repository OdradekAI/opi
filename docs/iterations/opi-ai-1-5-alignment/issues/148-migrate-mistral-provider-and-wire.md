# #148 Split the Mistral Provider profile from the conversations wire adapter

Status: implemented; canonical namespaces enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move Mistral profile construction and the native `mistral-conversations` wire
implementation out of the legacy root module without retaining
`opi_ai::mistral` as a compatibility module. The canonical paths are:

- `opi_ai::providers::mistral` for the OpenAI-compatible Mistral profile,
  catalog, native conversations provider state, endpoint and client;
- `opi_ai::api::mistral` for Mistral native options, transcript projection,
  SSE/event parsing and mapping, status/error handling, streaming and the
  typed `MistralConversationsChatApi` adapter.

The ordinary Mistral profile continues to delegate to the shared
`openai_chat` wire. The native conversations route keeps its Mistral-specific
wire behavior in the new API module while reusing the profile state.

## Ownership boundary

- The Provider profile owns Mistral identity, default endpoint, model catalog,
  shared HTTP client, model records and per-dispatch native option state.
- The wire adapter owns tool/reasoning/prompt-mode projection, prompt-cache
  affinity, tool-call id normalization, SSE framing/event mapping, HTTP status
  mapping, stream lifecycle and the typed Chat adapter.
- The shared OpenAI Chat adapter remains the owner of ordinary compatible
  Mistral requests; the native conversations adapter owns only the
  `WireApi::MistralConversations` route.
- No request, auth, catalog, retry, stream terminal, telemetry or lifecycle
  behavior changes are intended.

## Acceptance

1. Mistral profile construction and catalog live under
   `src/providers/mistral.rs`.
2. The native Mistral conversations wire implementation and options live under
   `src/api/mistral.rs`.
3. The obsolete `src/mistral.rs` file and `opi_ai::mistral` module are removed;
   no compatibility shim is retained.
4. All active callers, tests, docs and package probes use canonical profile/API
   paths.
5. Mistral fixtures, lifecycle/terminal/tool/replay/telemetry tests,
   coding-agent wiring, minimal feature compilation and an external consumer
   built from the actual crate archive pass.
6. Formatting, documentation and task-owned whitespace checks pass.

## Work and verification

- Add this issue before changing module ownership.
- Keep profile state and ordinary compatible construction in
  `providers::mistral`; keep native wire behavior in `api::mistral`.
- Update provider collection, internal callers, tests and package probes to
  canonical namespaces; delete the obsolete root module rather than retaining
  a compatibility path.
- Record an external consumer proof built from the actual crate archive.

Historical snapshots, the frozen contract index and prior evidence may retain
`src/mistral.rs` as audit records; active source and current migration
documentation must not depend on that path.

## Completed implementation

- `src/providers/mistral.rs` now owns the ordinary OpenAI-compatible Mistral
  profile and catalog plus native conversations profile state: identity,
  endpoint, client, models and native options.
- `src/api/mistral.rs` now owns native Mistral options, transcript projection,
  prompt-cache affinity, tool-call normalization, SSE/event mapping,
  status/error mapping, streaming and the typed Chat adapter.
- Ordinary Mistral requests continue through the shared `openai_chat` adapter;
  the native conversations route is implemented by the canonical Mistral API
  adapter.
- `src/mistral.rs` and the `opi_ai::mistral` module were deleted. Provider
  collection, internal callers, tests, docs and package probes use the
  canonical `providers::mistral` and `api::mistral` namespaces.
- Historical snapshots, the frozen contract index and prior evidence may retain
  `src/mistral.rs` as audit records; active source and current migration
  documentation do not depend on that path.

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo check -p opi-ai --locked --no-default-features --features
  provider-mistral --lib`: PASS.
- `cargo test -p opi-ai --locked --no-default-features --features
  provider-mistral --test provider_mistral_profile`: 1 passed.
- Mistral wire coverage with builtins: `mistral_fixtures`: 55 passed;
  `mistral_repeated_finish`: 4 passed.
- Shared regression coverage: `chat_pi_messages`: 42 passed;
  `model_wire_metadata`: 24 passed; `vertex_fixtures`: 73 passed.
- `cargo clippy -p opi-ai --locked --all-targets -- -D warnings
  -A clippy::too_many_arguments`: PASS.
- `cargo test -p opi-coding-agent --locked --test provider_factory`: 58
  passed.
- Provider protocol Python harness: 6 passed; modified scripts compile;
  `python scripts/opi-doc-check.py`: PASS; task-owned diff whitespace check:
  PASS (with the repository's existing CRLF convention handled by
  `core.whitespace=cr-at-eol`).
- Canonical source scan finds no active `opi_ai::mistral`, `crate::mistral` or
  root-file references; the only remaining matches are this brief's migration
  record and the negative package-probe assertion.
- External consumer receipt:
  `evidence/issue-148-mistral-package-consumer-20261117-r2.json` reports
  package lock, metadata, build and run all passing against the canonical API
  and Provider paths from the actual crate archive after the root module was
  deleted.

The repository's broader protocol-adapters package matrix retains its existing
feature-union limitation; this owner-specific Mistral proof passes
independently.

## Test impact

`add`: canonical Provider/API profile assertions, canonical-only external
consumer source and the migration receipt; `update`: module declarations,
internal imports, current docs and tests; `delete`: obsolete root module/file;
`retain`: historical snapshots and evidence as audit records.

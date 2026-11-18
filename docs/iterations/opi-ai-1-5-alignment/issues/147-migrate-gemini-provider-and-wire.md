# #147 Split the Gemini Provider profile from the Google wire adapter

Status: implemented; canonical namespaces enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the Gemini service profile and Google Generative AI wire implementation
out of the legacy root module without retaining `opi_ai::gemini` as a
compatibility module. The canonical paths are:

- `opi_ai::providers::gemini` for provider identity, endpoint, model catalog,
  shared client, wire selection and dispatch configuration;
- `opi_ai::api::gemini` for SSE/event parsing and mapping, request projection,
  status/error handling, streaming, native Google options and the typed Chat
  adapter.

## Ownership boundary

- The Provider profile owns Gemini route identity, base URL, model catalog,
  shared HTTP client, Vertex wire selection and per-dispatch native options.
- The wire adapter owns Gemini SSE framing, raw event types and mapping,
  request-body serialization, tool/thinking projection, HTTP transport and
  status mapping, streaming activation, and `GoogleGenerativeAiChatApi`.
- Vertex reuses these canonical API/profile symbols while retaining its own
  Vertex endpoint and authentication transport.
- No request, auth, catalog, retry, stream terminal, telemetry or lifecycle
  behavior changes are intended.

## Acceptance

1. The profile implementation and catalog live under
   `src/providers/gemini.rs`.
2. The Google Generative AI wire implementation and native options live under
   `src/api/gemini.rs`.
3. The obsolete `src/gemini.rs` file and `opi_ai::gemini` module are removed;
   no compatibility shim is retained.
4. All active in-repository callers, tests, docs and package probes use
   canonical profile/API paths.
5. Gemini and Vertex fixtures, lifecycle/auth/telemetry/replay/tool tests,
   coding-agent wiring, minimal feature compilation and an external consumer
   built from the actual crate archive pass.
6. Formatting, documentation and task-owned whitespace checks pass.

## Work and verification

- Add this issue before changing module ownership.
- Split with the smallest behavior-preserving delegation; keep route/profile
  state in `providers::gemini` and wire behavior in `api::gemini`.
- Update Vertex, internal callers, tests and package probes to canonical paths;
  delete the obsolete root module rather than retaining a shim.
- Record an external consumer proof built from the actual crate archive.

## Completed implementation

- `src/providers/gemini.rs` now owns the Gemini Provider profile, catalog,
  endpoint, client, wire selection and native-option state.
- `src/api/gemini.rs` now owns the Google Generative AI wire adapter, including
  SSE/event mapping, request projection, native options, status/error mapping,
  streaming and the typed Chat adapter.
- `src/gemini.rs` and the `opi_ai::gemini` module were deleted. Vertex,
  internal callers, tests and package probes now use the canonical namespaces.
- Historical snapshots and prior evidence may retain `src/gemini.rs` as audit
  records; active source and current documentation do not depend on that path.

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo check -p opi-ai --locked --no-default-features --features
  provider-gemini --lib`: PASS.
- `cargo test -p opi-ai --locked --no-default-features --features
  provider-gemini --test provider_gemini_profile`: 1 passed.
- `cargo test -p opi-ai --locked --features builtins --test gemini_fixtures`:
  71 passed; `vertex_fixtures`: 73 passed; `model_wire_metadata`: 24 passed.
- Gemini lifecycle and terminal behavior: `gemini_lifecycle`: 19 passed;
  `gemini_repeated_finish`: 3 passed.
- `cargo clippy -p opi-ai --locked --all-targets -- -D warnings
  -A clippy::too_many_arguments`: PASS.
- `cargo test -p opi-coding-agent --locked --test provider_factory`: 58 passed.
- Provider protocol Python harness: 6 passed; modified scripts compile;
  `python scripts/opi-doc-check.py`: PASS; task-owned diff whitespace check:
  PASS (with the repository's existing CRLF convention handled by
  `core.whitespace=cr-at-eol`).
- External consumer receipt:
  `evidence/issue-147-gemini-package-consumer-20261117-r1.json` reports
  package lock, metadata, build and run all passing against the canonical API
  and Provider paths.

The repository's broader protocol-adapters package matrix retains its existing
feature-union limitation; this owner-specific Gemini proof passes independently.

## Test impact

`add`: canonical Provider/API profile assertions, canonical-only external
consumer source and the migration receipt; `update`: module declarations,
internal imports, current docs and tests; `delete`: obsolete root module/file;
`retain`: historical snapshots and evidence as audit records.

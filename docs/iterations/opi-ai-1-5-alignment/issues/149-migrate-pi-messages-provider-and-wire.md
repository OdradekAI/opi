# #149 Split the pi-messages Provider profile from the wire adapter

Status: implemented; canonical namespaces enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move Radius/pi-messages profile construction and the native `pi-messages` wire
implementation out of the legacy root module without retaining
`opi_ai::pi_messages` as a compatibility module. The canonical paths are:

- `opi_ai::providers::pi_messages` for the Radius identity, default endpoint,
  model baseline, HTTP client and per-dispatch profile state;
- `opi_ai::api::pi_messages` for native options, cache-retention projection,
  canonical chronology serialization, SSE/event mapping, status/error
  handling, streaming and the typed `PiMessagesChatApi` adapter.

## Ownership boundary

- The Provider profile owns endpoint selection, route identity, advertised
  model records, shared HTTP client and cloned native dispatch options.
- The wire adapter owns pi's option whitelist, context projection, streaming
  terminal state, callback ordering, typed failures and complete-Chat seam.
- No request, auth, catalog, retry, stream terminal, telemetry or lifecycle
  behavior changes are intended.

## Acceptance

1. The profile and baseline catalog live under `src/providers/pi_messages.rs`.
2. The native wire implementation and options live under
   `src/api/pi_messages.rs`.
3. The obsolete `src/pi_messages.rs` file and `opi_ai::pi_messages` module are
   removed; no compatibility shim is retained.
4. Active callers, tests, documentation and package probes use canonical
   profile/API paths.
5. Pi Messages terminal, fixture, external SDK, provider profile and shared
   regression tests pass, along with an external consumer built from the actual
   crate archive.
6. Formatting, documentation and task-owned whitespace checks pass.

Historical snapshots, the frozen contract index and prior evidence may retain
`src/pi_messages.rs` as audit records; active source and current migration
documentation must not depend on that path.

## Implementation and verification

The implementation keeps `PiMessagesProvider` in `providers::pi_messages` and
implements its wire-facing methods and `Provider` behavior in
`api::pi_messages`. Provider collection, integration tests and package probes
now import the canonical namespaces. The legacy root module is deleted.

- `cargo fmt --all -- --check`: PASS.
- `cargo check -p opi-ai --locked --no-default-features --features
  provider-pi-messages --lib`: PASS.
- `cargo test -p opi-ai --locked --no-default-features --features
  provider-pi-messages --test provider_pi_messages_profile`: 1 passed.
- Builtins Pi Messages coverage: `chat_pi_messages`: 42 passed;
  `pi_messages_terminal`: 2 passed.
- Shared consumer regressions: `external_sdk`: 11 passed;
  `provider_collection`: 111 passed; `vertex_fixtures`: 73 passed.
- Coding-agent provider factory: 58 passed.
- `cargo clippy -p opi-ai --locked --all-targets -- -D warnings
  -A clippy::too_many_arguments`: PASS.
- Provider protocol Python harness: 7 passed; modified scripts compile;
  `python scripts/opi-doc-check.py`: PASS; task-owned diff whitespace check:
  PASS.
- Canonical source scan finds no active `opi_ai::pi_messages`,
  `crate::pi_messages` or root-file references. Remaining matches are the
  issue brief's migration record and the negative package-probe assertion.
- External consumer receipt:
  `evidence/issue-149-pi-messages-package-consumer-20261117-r1.json` reports
  package lock, metadata, clippy and terminal tests passing in both
  builtins/no-tracing and defaults profiles against the actual crate archive.
- Provider/API namespace consumer receipt:
  `evidence/issue-149-pi-messages-profile-consumer-20261117-r1.json` reports
  the canonical `api::pi_messages` options and
  `providers::pi_messages` profile compiling and running from the extracted
  package with the exact `provider-pi-messages` feature union.

## Test impact

`add`: canonical Provider/API assertions and the migration consumer receipt;
`update`: module declarations, internal imports, tests, scripts and docs;
`delete`: obsolete root module/file; `retain`: historical snapshots and prior
evidence as audit records. No commit or stage was performed.

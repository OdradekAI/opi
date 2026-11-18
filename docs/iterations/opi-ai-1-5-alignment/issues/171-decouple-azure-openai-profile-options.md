# #171 Decouple Azure OpenAI profile from native wire options

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)  
Precedent: [#170 Anthropic profile options](170-decouple-anthropic-profile-options.md)

## Evidence

Before this slice, `providers::azure_openai::AzureOpenAIProvider` imported
`api::azure_openai::AzureOpenAIOptions`, stored `azure_native`, and retained
`active_record` dispatch state. The API module also implemented the wire
transport directly on the profile. A native Chat API call therefore cloned a
profile, mutated request-scoped state on that clone, and sent it through the
same type used for the registered route.

The profile's stable ownership is endpoint/resource/deployment resolution,
API-version policy, model catalog, OpenAI compatibility configuration and the
shared HTTP client. Native Responses options, selected record state, request
serialization, SSE mapping and transport lifecycle belong to the wire adapter.

## Implemented seam

`api::azure_openai::AzureOpenAIWireAdapter` now owns per-call
`AzureOpenAIOptions`, the selected model record and the Azure Chat/Responses
wire implementation. Neutral Provider dispatch constructs a neutral adapter;
simple and typed Responses dispatch construct a fresh adapter with explicit
options. The registered `AzureOpenAIProvider` no longer carries mutable
native or selected-record state.

The API module retains small forwarding methods on the profile for existing
request-fixture callers, while the Provider implementation delegates through
the adapter. These methods do not add state or a compatibility module. The
old root `src/azure_openai.rs` path remains absent.

## Acceptance

- `providers/azure_openai.rs` contains no native option field and no direct
  `crate::api`/`api::` dependency.
- `api::azure_openai::AzureOpenAIWireAdapter` is the canonical owner of
  request-scoped options, dual-wire dispatch state, body construction, SSE
  mapping and streaming.
- Registered Azure profiles remain immutable route/catalog descriptions during
  native Chat API and Responses dispatch.
- Azure Chat, Responses, endpoint/deployment, callback, auth, cancellation,
  lifecycle and compatibility tests pass.
- Structural conformance checks and an independent `.crate` consumer prove
  the canonical profile and adapter paths.
- No root-module compatibility shim or duplicate provider module is added.

## Validation

Observed on 2026-11-18:

- `cargo test -p opi-ai --locked --features builtins --quiet --test
  azure_openai_fixtures --test provider_azure_openai_profile --test
  external_sdk --test api_mapped_provider` passed (46 + 1 + 11 + 17 tests).
- `cargo check -p opi-ai --locked --features builtins` and
  `cargo fmt --all -- --check` passed.
- The Azure feature-closure Clippy check passed with the repository's existing
  `openai_responses.rs:699` `too_many_arguments` lint explicitly allowed; the
  ordinary `-D warnings` run remains blocked only by that pre-existing
  unrelated warning.
- `python -B -m unittest discover -s scripts/pi-sdk-conformance -p
  'test_provider_protocol_package.py' -q` passed (35 tests); the protocol
  adapter suite passed (6 tests).
- The independent `provider-azure-openai` consumer built and ran from the
  extracted `.crate`; evidence is recorded in
  `C:\Temp\opi-azure-profile-20261118-r1\evidence.json`.
- The independent protocol-adapters consumer built and ran from the extracted
  crate; evidence is recorded in
  `C:\Temp\opi-canonical-contract-20261118-r4\evidence.json`.
- `python scripts/opi-doc-check.py`, `git diff --check` and the final format
  check passed.

## Test impact

`add`: Azure profile/adapter structural assertions and canonical adapter
consumer reference.  
`update`: Azure fixture helpers construct the canonical wire adapter for body
and SSE tests; protocol package checks include the Azure profile seam.  
`delete`: profile-owned native option and selected-record state, plus the old
root module if present.  
`retain`: Azure request JSON, dual-wire Responses routing, auth, callbacks,
retries, cancellation, compatibility and lifecycle behavior.

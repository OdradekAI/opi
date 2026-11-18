# #169 Decouple OpenAI Codex profile from native wire options

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)  
Precedent: [#168 OpenAI Responses profile options](168-decouple-openai-responses-profile-options.md)

## Evidence

`providers::openai_codex::OpenAiCodexResponsesProvider` still stored
`native_options: Option<CodexOptions>` and held the WebSocket pool type from
`api::openai_codex_responses::ws_transport`. That made the provider profile
depend on the wire adapter and caused native dispatch to mutate a cloned
profile before sending a request. The profile should own identity, endpoint,
model records, HTTP client and transport-resource state; Codex request options
and WebSocket/SSE mapping belong to the API and transport owners.

## Implemented seam

`CodexOptions` is now passed explicitly to the Codex wire adapter. The ordinary
Provider path passes `None`; simple and typed native Chat API paths construct a
private per-call configured adapter that keeps the native options out of the
registered provider. The body compiler, service-tier projection and WebSocket
connect timeout all consume that explicit value.

The pooled Codex WebSocket state moved to canonical `transport::codex`. This
keeps the provider profile independent of `api::*` while leaving the existing
transport state machine, session leases, fallback markers and cleanup barrier
unchanged. The old `api/openai_codex_responses/ws_transport.rs` path was
removed; no compatibility shim was added.

## Acceptance

- `providers/openai_codex.rs` contains no `native_options` field and no API
  layer dependency.
- Codex native options remain owned by the API adapter and are supplied per
  call without mutating a registered provider profile.
- The WebSocket transport state has one canonical transport owner and the old
  API submodule path is absent.
- Codex native, SSE/WebSocket lifecycle, profile and external SDK tests pass.
- The extracted `.crate` consumer proves the provider feature closure and the
  canonical profile/transport layout.
- No profile shim or duplicate root provider module is introduced.

Observed on 2026-11-18:

- `cargo check -p opi-ai --locked --features builtins` passed.
- `cargo check --workspace --locked` passed.
- The focused Codex profile test passed (1 test).
- `codex_native_options`: 21 passed.
- `openai_codex_responses`: 87 passed.
- `external_sdk`: 11 passed.
- The provider protocol package suite passed (31 Python tests), including the
  no-`native_options`, no-API-dependency and canonical transport assertions.
- The protocol-adapters package suite passed (5 Python tests), including the
  canonical transport and provider-profile assertions.
- The independent `provider-openai-codex-responses` consumer built and ran
  from the extracted `.crate`; the recorded verdict was `PASS` in
  `C:\Temp\opi-openai-codex-profile-20261118-r1\evidence.json`.
- The independent protocol-adapters consumer built and ran from the extracted
  crate; the recorded verdict was `PASS` in
  `C:\Temp\opi-canonical-contract-20261118-r2\evidence.json`.
- The WebSocket lifecycle regression found during the move was traced to a
  dropped resource lease in the trait wrapper and fixed before these results;
  the baseline comparison confirmed the failure was migration-caused.

## Test impact

`update`: Codex native-options, OpenAI Codex Responses lifecycle, provider
profile and package-consumer structural tests.  
`delete`: the provider-owned native option field, its mutation path and the old
API-owned WebSocket transport module path.  
`retain`: request JSON, authentication, retries, callbacks, SSE/WebSocket
fallback, resource leases, continuation and cleanup behavior.

# #170 Decouple Anthropic profile from native wire options

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)  
Precedent: [#169 OpenAI Codex profile options](169-decouple-openai-codex-profile-options.md)

## Evidence

Before this slice, `providers::anthropic::AnthropicProvider` imported
`api::anthropic::AnthropicMessagesOptions`, retained
`native_options: Option<AnthropicMessagesOptions>`, and exposed a mutating
`with_native_options` construction path. The API module also attached the
wire compiler, SSE mapper, HTTP stream and native dispatch directly to the
profile type. That made the route profile own request-scoped wire policy and
prevented a clean provider/profile seam.

## Implemented seam

`api::anthropic::AnthropicMessagesWireAdapter` now owns the copied route
context needed by one wire dispatch plus the explicit native options for that
dispatch. The ordinary Provider entry constructs a neutral adapter; the
neutral/simple and typed Chat API entries construct a per-call adapter with
their explicit options. A registered `AnthropicProvider` is never mutated by
native dispatch.

The pure validated `ProviderHeaders` value moved to `types::headers`; the API
shared module retains only wire-specific merge policy and re-exports that
value. This removes the Anthropic profile's dependency on the API layer while
preserving the public header configuration surface. The profile now owns route
identity, endpoint, catalog, auth policy, client, federation cache and
validated configured headers. The adapter owns request projection, native
options, serialization, SSE mapping and streaming.

## Acceptance

- `providers/anthropic.rs` contains no `native_options` field, no
  `with_native_options` method and no `crate::api` dependency.
- `api::anthropic::AnthropicMessagesWireAdapter` is the canonical wire owner;
  native options are supplied per call and never stored on a registered
  profile.
- `ProviderHeaders` remains a pure validated value under `types::headers`;
  wire merge behavior remains under `api::shared::headers`.
- Existing Anthropic request, SSE, OAuth, native Chat API, retry and lifecycle
  behavior remains covered by the focused tests.
- Structural conformance checks and an independent `.crate` consumer prove the
  canonical profile and adapter paths.
- No root-module compatibility shim or duplicate provider implementation is
  introduced.

## Observed validation

On 2026-11-18:

- `cargo test -p opi-ai --locked --features builtins --test
  anthropic_fixtures --test anthropic_thinking_roundtrip --test
  provider_anthropic_profile --test external_sdk` passed (155 + 13 + 1 + 11
  tests).
- The adjacent header/collection/auth/request suite passed (17 + 12 + 111 +
  54 + 45 tests).
- `cargo check --workspace --locked` and `cargo fmt --all -- --check` passed.
- `python scripts/opi-doc-check.py` passed.
- The provider conformance unit suite passed (33 tests); the protocol-adapters
  suite passed (6 tests).
- The independent Anthropic `.crate` consumer passed from the extracted
  archive; evidence is recorded in
  `C:\Temp\opi-anthropic-profile-20261118-r1\evidence.json`.
- The independent protocol-adapters `.crate` consumer passed; evidence is
  recorded in `C:\Temp\opi-canonical-contract-20261118-r3\evidence.json`.

The structure phase remains open globally:
the API adapters still consume concrete profile route context, remaining
profiles still need evidence-backed migration, and `models.rs` still needs its
private owner split.

## Test impact

`add`: Anthropic profile/adapter structural assertions and canonical adapter
consumer reference.  
`update`: Anthropic native fixture helpers now construct the wire adapter
explicitly; header value ownership tests continue through the same public
facade.  
`delete`: profile-owned native option state and its mutating constructor path.
  
`retain`: Anthropic wire fixtures, OAuth and lifecycle behavior, route header
validation and external consumer coverage.

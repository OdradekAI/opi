# #163 Freeze ownership of the remaining crate-root modules

Status: root-module audit complete; cross-layer ownership remains open  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Close the canonical-only structure pass by reviewing every remaining active
module declared from `crates/opi-ai/src/lib.rs`. A root module is retained only
when it owns a public cross-provider contract, a crate-wide lifecycle seam, a
transport/backend boundary, or a private substrate that is consumed by those
owners. Provider profiles and compositions remain under `providers`; provider
wire adapters and neutral wire helpers remain under `api` and `api::shared`.

This audit is an ownership decision. It does not move a module merely because
its file is small or because another namespace exists. Any later migration must
have a separate issue with caller evidence, behavior tests, documentation and
archive-consumer verification. No compatibility module is introduced.

## Audit result

| Active root module | Responsibility evidence | Decision |
| --- | --- | --- |
| `api` | Object-safe image/classifier contracts and the API-adapter namespace | Retain as the API namespace and operation-contract owner |
| `auth` | Per-request auth resolution, credential/store contracts and OAuth coordination | Retain as the auth domain owner |
| `callbacks` | Public request callback contract and dispatch timing | Retain as a public cross-provider contract |
| `cancel` | Public cancellation-token and cancellable-wait semantics | Retain as a cross-provider lifecycle utility |
| `catalog` | Bundled catalog, dynamic source and store contracts | Retain as the catalog/data owner |
| `classifier` | Public classifier context, result and model-record types | Retain as an operation contract |
| `codex_resources` | Public resource-generation, lease and session-cleanup contract consumed by `Models`, `Provider` and multiple adapters | Retain as a cross-provider lifecycle contract; its Codex-origin name does not make it a wire adapter |
| `constrained_sampling` | Public grammar/schema sampling options and validation | Retain as a public semantic contract |
| `context` | Token/context occupancy estimates used by models and coding-agent compaction | Retain as shared request semantics |
| `deferred` | Public submit/fetch/cancel handle and lifecycle contracts | Retain as a public operation contract |
| `errors` | Shared provider failures, diagnostics and classification | Retain as the error domain owner |
| `execution` | Private independently executing chat-call owner used by `stream` and `Models` | Retain as a private lifecycle implementation |
| `http` | Public pooled HTTP client, proxy and redirect policy | Retain as the transport owner |
| `identity` | Public compatibility hashes and UUID identity utilities | Retain as a cross-provider value contract |
| `images` | Public image context, result and model-record types | Retain as an operation contract |
| `models` | Public Models front door, route selection and call lifecycle | Retain as the primary facade and lifecycle owner |
| `provider` | Provider trait, request contract and dispatch seam | Retain as the provider contract owner |
| `providers` | Provider profile table and provider composition namespace | Retain as the canonical provider namespace |
| `registry` | Runtime provider registration and `provider:model` resolution | Retain as the registry/composition contract |
| `replay` | Canonical transcript-to-wire replay projection | Retain as cross-provider request semantics |
| `retry` | Acquisition admission, backoff and provider retry policy | Retain as the retry domain owner |
| `schema` | Private TypeBox-compatible schema engine used by public tool contracts | Retain as the schema substrate |
| `stream` | Public assistant events, chat streams and terminal/result semantics | Retain as the stream contract and execution facade |
| `streaming_json` | Public caller-owned tool-argument preview stream | Retain as a public wire-independent contract |
| `telemetry` | AI operation telemetry context bridge to `opi-telemetry` | Retain as the telemetry integration boundary |
| `test_support` | Hidden deterministic mock-provider and fixture utilities | Retain as a test-only support boundary |
| `time` | Public wall-clock helper used by agents, models, streams and adapters | Retain at the crate root; it is cross-domain and is not an API wire helper |
| `tls` | Source-scoped TLS configuration for the HTTP transport | Retain as the TLS backend owner |
| `transcript` | Public context normalization and chronology projection | Retain as the transcript domain owner |
| `types` | Deterministic values, metadata and validation shared by all domains | Retain as the value/type domain owner |
| `ws` | Public HTTP Upgrade bridge and private WebSocket transport state | Retain as the WebSocket transport owner |
| `crypto` | Private TLS/key-admission substrate | Retain as a private backend substrate |

The audit found no remaining root module whose implementation is solely a
provider profile, provider composition, provider wire adapter, or neutral
wire helper. In particular, `time` and `codex_resources` are intentionally
not moved to `api::shared`: both are consumed by public lifecycle and agent
surfaces outside API adapters. `execution` is also intentionally kept private
because it owns the chat execution state that `stream` exposes.

## Canonical structure after the audit

- `api::*`: provider wire adapters and operation-specific API contracts.
- `api::shared::*`: protocol-neutral wire seams such as headers, environment
  lookup, incremental text decoding, endpoint joining, OpenAI Responses
  mapping and OpenRouter usage accounting.
- `providers::*`: provider profiles, built-in collection, route composition and
  provider-facing profile data.
- crate root: public domain contracts, transport, lifecycle and cross-provider
  semantics listed above.

The only root-level public facade retained from the preceding slices is the
selected `ProviderHeaders` type re-export. The old `provider_headers` module
path was deleted; this facade is an intentional public value import, not a
second implementation path.

## Scope correction

This audit only settled retention of active `lib.rs` modules. It did not prove
that `api`, `providers` and `models` are decoupled internally. The follow-up
[#164](164-audit-unresolved-ownership-boundaries.md) records the remaining
evidence: operation contracts still live in `models`/`deferred`, API adapters
still import concrete provider profiles, provider profiles still store native
API options, and `models.rs` still owns protocol-specific policy and multiple
unrelated orchestration domains. The structural phase therefore remains open
for those evidence-backed migrations.

The real Deferred acceptance gate in
[#143](143-connect-real-deferred-production-route.md) and the remaining
platform, capability-V, resource, performance and fuzz evidence remain
parallel work; they do not close #164.

## Verification

The audit was checked against the active `lib.rs`, `api.rs`, `providers/mod.rs`,
production references, public tests and external-consumer paths. The latest
canonical endpoint slice still passes its focused tests, conformance, rustdoc,
Clippy, documentation and archive-consumer checks; this issue adds no runtime
code and therefore requires no new Cargo behavior test.

## Test impact

`add`: this ownership matrix and the structural decision record.  
`update`: iteration/spec navigation and the module-structure decision.  
`delete`: none.  
`retain`: all existing public-contract, lifecycle, provider and external
consumer tests.

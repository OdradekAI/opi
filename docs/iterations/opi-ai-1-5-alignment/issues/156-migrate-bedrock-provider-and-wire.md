# #156 Migrate Bedrock provider and wire to canonical namespaces

Status: implemented; canonical-only namespaces enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining `src/bedrock` root module and separate the Bedrock
provider profile from its Converse wire adapter:

- `opi_ai::providers::bedrock` owns route identity, endpoint configuration,
  model catalog, proxy-placeholder authentication construction and immutable
  retry/client state.
- `opi_ai::api::bedrock` owns Converse request projection, endpoint/auth wire
  selection, SigV4 sends, event-stream decoding, status mapping, native
  options and the direct `ChatApi`.
- `opi_ai::auth::aws::sigv4` remains the canonical shared AWS signing owner;
  the old `bedrock::sigv4` re-export is removed.
- Smithy retry helpers remain under `opi_ai::retry::smithy`; no provider retry
  re-export is retained.

## Ownership decision

Bedrock is both a provider profile and a concrete wire protocol. Its endpoint,
model and route configuration must not be mixed with request serialization or
stream parsing. The provider profile therefore owns only immutable construction
state and provider identity. The API adapter owns Converse framing and
transport, while AWS SigV4 is reused from the existing cloud-auth owner. This
keeps the dependency direction explicit and gives callers one canonical path
for each concern without preserving a root compatibility shim.

## Acceptance

1. `src/bedrock/mod.rs`, `src/bedrock/event_stream.rs` and the obsolete
   `src/bedrock/sigv4.rs` path are deleted.
2. `providers::bedrock` and `api::bedrock` are independently declared under
   their canonical namespaces, and all crate, test, script and consumer
   references use those paths.
3. Event framing uses `api::bedrock::event_stream`; AWS signing uses
   `auth::aws::sigv4`; retry helpers use `retry::smithy`.
4. Existing Bedrock fixtures, lifecycle, replay, metadata, direct Chat API,
   model-wire and protocol-profile tests continue to pass.
5. A consumer built from the actual `.crate` archive can use both the provider
   profile and API-native options under their canonical paths.
6. Conformance inventory and active documentation record the split; no
   compatibility shim or active old-root dependency remains.

## Implementation and verification

The former 2,595-line root module is now split between
[`src/providers/bedrock.rs`](../../../../crates/opi-ai/src/providers/bedrock.rs),
[`src/api/bedrock.rs`](../../../../crates/opi-ai/src/api/bedrock.rs) and
[`src/api/bedrock/event_stream.rs`](../../../../crates/opi-ai/src/api/bedrock/event_stream.rs).
Provider-specific callers use `providers::bedrock`; wire callers use
`api::bedrock`; shared signing and retry paths no longer have Bedrock aliases.

- Bedrock owner `cargo check` with `provider-bedrock`: PASS.
- Bedrock fixture, pending-metadata, provider-profile, protocol-profile and
  model-wire tests: 207 passed, 0 failed.
- External consumer from the actual package archive: [provider-bedrock receipt](../evidence/issue-156-bedrock-package-consumer-20261117-r1.json),
  PASS. The consumer instantiates the provider profile and references the
  canonical API native options.
- The complete `protocol-adapters` external consumer from the actual archive:
  [receipt](../evidence/issue-156-protocol-adapters-consumer-20261117-r1.json),
  PASS.
- Canonical namespace conformance tests: PASS; root-module deletion and
  caller-path assertions are active in the package test suite.

The full built-in test target compilation, warnings-denied Clippy,
documentation checker, formatting and whitespace checks all pass. The caller
inventory scans 1,273 repository files and records 27 Bedrock owner files plus
the four canonical profile/wire ownership entries. Historical contract-index
and earlier issue snapshots may retain the old path as audit evidence; active
source and this issue use canonical-only paths.

## Test impact

`add`: root deletion and canonical namespace assertions, Bedrock external
consumer receipt and caller inventory entry; `update`: module declarations,
all Bedrock callers, conformance runners and active migration documentation;
`delete`: obsolete root Bedrock module and SigV4/retry re-export paths;
`retain`: Bedrock request, stream, auth, retry and direct Chat behavior.

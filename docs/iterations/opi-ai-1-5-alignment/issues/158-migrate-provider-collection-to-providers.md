# #158 Move builtin provider assembly to the canonical providers namespace

Status: implemented; canonical-only namespace enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Place the builtin provider assembly module beside the provider profiles and
remove the remaining provider-owned root module:

- `opi_ai::providers::collection` owns builtin profile registration, route
  composition, auth resolver wiring, and catalog-backed provider assembly;
- `opi_ai::api::*` continues to own wire adapters and `opi_ai::api::shared`
  continues to own protocol-neutral mappers;
- there is no `opi_ai::provider_collection` compatibility shim or duplicate
  implementation path.

## Ownership decision

`provider_collection.rs` is a deep provider composition module. Its public
interface is the builtin registration report/configuration plus
`canonical_route`; its implementation combines provider profiles, catalog
records, auth sources, typed classifier/image seams, and already-constructed
wire adapters. It does not own a wire protocol. The canonical seam is therefore
`providers::collection`, while `providers::{profile}` remains the profile owner
and `api::{adapter}` remains the wire owner.

Keeping the module at the crate root made provider assembly appear peer to
transport, catalog, and API contracts and left a second namespace family after
the provider migrations. Moving the module preserves its interface and
behavior while giving maintainers locality: provider registration changes now
live under one provider-owned namespace.

## Acceptance

1. `src/provider_collection.rs` is deleted and the implementation lives only at
   `src/providers/collection.rs`.
2. `lib.rs`, coding-agent callers, tests, examples, README files, provider-auth
   docs, and conformance scripts use `providers::collection`.
3. No root module declaration or root compatibility re-export remains.
4. Builtin registration, route dispatch, auth laziness, catalog mapping,
   classifier/image registration, and existing error behavior remain covered by
   the existing provider-collection tests.
5. An external consumer built from the actual `.crate` archive resolves and
   executes the canonical collection path through the packaged README flows.

## Implementation and verification

The former root module was moved verbatim to
[`src/providers/collection.rs`](../../../../crates/opi-ai/src/providers/collection.rs).
`providers::mod` declares it under the `builtins` feature; the root declaration
was removed. All active callers now use `opi_ai::providers::collection`.

The README external consumer exercises `BuiltinOverrides` from the extracted
archive and runs all three controlled onboarding flows. The archive receipt is
[`issue-158-readme-consumer-20261117-r3.json`](../evidence/issue-158-readme-consumer-20261117-r3.json)
(`PASS`).

Verification for this slice also passed: the builtins library check; all 111
provider-collection tests; all 11 external-sdk tests; all 58 coding-agent
provider-factory tests;
warnings-denied Clippy for `opi-ai` and `opi-coding-agent`; the canonical-only
Python conformance suite (21 tests); README contract tests (31 tests); package
documentation, formatting and whitespace checks; and the caller inventory,
which records `providers/collection.rs` as the collection owner.

## Test impact

`add`: canonical namespace/root-deletion assertions and archive consumer
source coverage;  
`update`: provider collection callers, docs, examples, caller inventory and
conformance tests;  
`delete`: `src/provider_collection.rs`, root module declaration and root
compatibility path;  
`retain`: registration, route composition, auth resolution, catalog and
classifier/image behavior tests.

# #157 Move mapped provider composition to the canonical providers namespace

Status: implemented; canonical-only namespace enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining provider-composition root module and make the ownership
boundary explicit:

- `opi_ai::providers::mapped` owns `ApiMappedProvider` and `ApiMapError`;
- the module combines one provider identity/catalog with checked concrete wire
  routes and remains independent of any single wire adapter;
- wire implementations remain under `opi_ai::api::*`, while concrete provider
  profiles remain under `opi_ai::providers::*`;
- no `opi_ai::api_mapped` module or root-level compatibility re-export remains.

## Ownership decision

`ApiMappedProvider` is a provider composition object. It validates a complete
provider catalog and delegates each model to the route selected by `WireApi`.
It does not serialize a wire protocol, so placing it beside the API trait or a
specific adapter would invert the dependency direction. `providers::mapped`
is the narrow canonical owner: it depends on the provider contract and model
metadata, while concrete routes continue to depend on their API adapters.

## Acceptance

1. `src/api_mapped.rs` is deleted and `src/providers/mapped.rs` is the only
   implementation path.
2. `lib.rs`, `provider_collection`, `opi-coding-agent`, tests and docs use
   `providers::mapped::{ApiMappedProvider, ApiMapError}`.
3. The old `opi_ai::api_mapped` module and root-level type re-exports are not
   retained as shims.
4. Existing mapped-provider and provider-collection behavior remains covered;
   the public-module baseline records the canonical namespace change.
5. An external consumer built from the actual `.crate` archive resolves and
   uses the canonical mapped-provider path.

## Implementation and verification

The former root module was moved verbatim to
[`src/providers/mapped.rs`](../../../../crates/opi-ai/src/providers/mapped.rs).
`providers::mod` declares it as the canonical composition owner. The root
`api_mapped` declaration and `ApiMappedProvider`/`ApiMapError` re-export were
removed; all in-repo callers now import through `providers::mapped`.

The full protocol-adapters package consumer also references the canonical
mapped provider path, so the archive check covers both provider profiles and
provider composition. The actual archive receipt is
[`issue-157-protocol-adapters-consumer-20261117-r1.json`](../evidence/issue-157-protocol-adapters-consumer-20261117-r1.json)
(`PASS`). Historical phase snapshots may still mention the old
path as audit material; active source, tests and docs do not depend on it.

Verification completed:

- the `provider-openrouter` canonical owner library check passed;
- mapped-provider, provider-collection and public-module regression tests passed
  (`17 + 111 + 2`);
- coding-agent mapped-catalog and provider-factory tests passed (`3 + 58`);
- warnings-denied Clippy for `opi-ai` and `opi-coding-agent`, formatting,
  documentation and whitespace checks passed;
- provider conformance tests passed (`19`), and the caller inventory records
  the three mapped-composition owners.

## Test impact

`add`: canonical namespace assertions and archive-consumer source coverage;  
`update`: module declaration, mapped-provider tests, provider factory,
provider collection, docs and public-module baseline;  
`delete`: `src/api_mapped.rs`, root module declaration and root compatibility
re-exports;  
`retain`: mapped route validation, dispatch, catalog replacement, refresh and
error semantics.

# #179 Extract the Models auth owner

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`AuthOwner` now owns route auth descriptors, resolver/source maps, OAuth flow
bindings, builtin auth factories/checks, ambient auth context and credential
store state. The `Models` facade keeps the same public registration and auth
methods while delegating these mutable source-ownership fields to the private
owner.

Together with #178, two private owners now cover registry/resource state and
auth/source state. Catalog and operation/prepared execution owners remain.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib`: 374 passed.
- Architecture report now exposes `models_owner_modules=2`.

## Test impact

`update`: Models constructors and auth/catalog management now initialize and
use the private auth owner.

`delete`: flat auth, resolver, OAuth and credential-store ownership from
`Models`.

`retain`: auth ownership, OAuth, credential-store, catalog, prepared-call and
provider lifecycle coverage.

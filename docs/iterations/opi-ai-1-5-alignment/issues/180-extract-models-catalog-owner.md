# #180 Extract the Models catalog owner

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`CatalogOwner` now owns the persisted models store, dynamic catalog sources,
publisher provenance, acknowledged catalog entries and publication generation.
Refresh rounds keep their snapshot and publication fencing state together while
the public `Models` catalog methods remain source-compatible.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `catalog_personalization`: 5 passed.
- The architecture report now exposes `models_owner_modules=3`.

## Test impact

`update`: Models refresh, store and dynamic catalog state now initialize and
use the private catalog owner.

`delete`: flat catalog store, source, publisher and publication-generation
ownership from `Models`.

`retain`: catalog refresh, personalization, provider projection and
publication fencing coverage.

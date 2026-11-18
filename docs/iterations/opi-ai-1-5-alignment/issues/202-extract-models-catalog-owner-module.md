# #202 Extract the Models catalog owner into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`CatalogOwner` now lives in `crates/opi-ai/src/models/catalog_owner.rs`. The
module owns the models store, dynamic catalog sources, publisher retention,
dynamic entries and publication generation. `Models` keeps the stable facade
and uses a crate-private re-export.

The physical migration preserves refresh ordering, publication generation
invalidation and catalog personalization behavior. It does not change the
catalog interface or add a forwarding layer.

## Structure receipt

- `models_owner_modules=4` total owner definitions.
- `models_owner_module_files=3` independent owner module files.
- `models_owner_definitions_in_facade=1` remaining owner definition in
  `models.rs`.
- `models_lines=5293`, down from 5314 after the auth slice.
- `models_prepared_execution_owner_files=0`.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test catalog_personalization --locked --offline --features builtins --quiet`: 5 passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: architecture expectations now require three independent owner module
files and one owner definition remaining in the facade.

`retain`: catalog refresh, source registration, publication and personalization
regression tests.

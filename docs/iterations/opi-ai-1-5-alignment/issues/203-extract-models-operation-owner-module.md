# #203 Extract the Models operation owner into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`OperationOwner` now lives in `crates/opi-ai/src/models/operation_owner.rs`.
The module owns chat, image, classifier and deferred registration maps. The
four registration record types remain in `models.rs` as crate-private data
contracts because the facade still performs the dispatch orchestration.

`Models` has no owner definition left in its facade file. It keeps the public
registration and dispatch interface unchanged and reaches the operation maps
through a crate-private owner re-export.

## Structure receipt

- `models_owner_modules=4` total owner definitions.
- `models_owner_module_files=4` independent owner module files.
- `models_owner_definitions_in_facade=0`.
- `models_lines=5273`, down from 5293 after the catalog slice.
- `models_prepared_execution_owner_files=0`, with prepared execution isolated
  in the next slice.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test catalog_operations --locked --offline --features builtins --quiet`: passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: registration record types are crate-private so the physical owner
module can expose its maps without widening the public facade.

`retain`: chat, image, classifier and deferred registration and dispatch tests.

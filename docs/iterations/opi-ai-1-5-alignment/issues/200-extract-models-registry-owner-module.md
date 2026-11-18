# #200 Extract the Models registry owner into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`RegistryOwner` now lives in `crates/opi-ai/src/models/registry_owner.rs`.
The module owns the provider registry, session-resource namespaces, cleanup
owner list and registration gate. `Models` keeps the stable facade and uses a
crate-private re-export, so registration and lookup behavior stay unchanged.

This slice changes physical ownership only. The owner still exposes its
crate-private state to the facade while later slices can deepen the internal
interface without changing public callers.

## Structure receipt

- `models_owner_modules=4` total owner definitions.
- `models_owner_module_files=1` independent owner module file.
- `models_owner_definitions_in_facade=3` remaining owner definitions in
  `models.rs`.
- `models_lines=5350`, down from 5383 before the physical split.
- `models_prepared_execution_owner_files=0`, because prepared execution is
  still in the facade for the next slice.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib --quiet`: 374 passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: architecture checks now distinguish total owner definitions from
owners physically extracted into `models/*.rs`.

`retain`: registry, resource cleanup, registration and facade regression tests.

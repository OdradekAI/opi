# #201 Extract the Models auth owner into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`AuthOwner` now lives in `crates/opi-ai/src/models/auth_owner.rs`. The module
owns auth descriptors, resolver and source maps, OAuth bindings, builtin auth
factories and checks, ambient auth context, and credential-store state.
`Models` keeps the stable facade and uses a crate-private re-export.

The migration is physical and behavior-preserving. The owner still exposes
crate-private state to the facade, which keeps this step narrow and leaves a
clear seam for a later method-level deepening if the callers justify it.

## Structure receipt

- `models_owner_modules=4` total owner definitions.
- `models_owner_module_files=2` independent owner module files.
- `models_owner_definitions_in_facade=2` remaining owner definitions in
  `models.rs`.
- `models_lines=5314`, down from 5350 after the registry slice.
- `models_prepared_execution_owner_files=0`.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib --quiet`: 374 passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: architecture expectations now require two independent owner module
files and two owner definitions remaining in the facade.

`retain`: auth resolution, OAuth, credential-store and builtin auth regression
tests.

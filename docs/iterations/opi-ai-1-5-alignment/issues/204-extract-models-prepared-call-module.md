# #204 Extract the Models prepared execution owner into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`PreparedProviderCall`, `PreparedRoute`, the strict-variant admission value and
the attempt stream now live in `crates/opi-ai/src/models/prepared_call.rs`.
Prepared execution owns frozen route, request, auth, acquisition, retry and
attempt lifecycle state behind a crate-private constructor seam.

`Models` builds prepared calls through `PreparedProviderCall::new` and uses a
controlled cancellation-token accessor while the prepared module retains all
execution fields privately. Public route, provenance, retry and attempt
methods remain source-compatible.

## Structure receipt

- `models_owner_modules=4` total owner definitions.
- `models_owner_module_files=4` independent owner module files.
- `models_owner_definitions_in_facade=0`.
- `models_prepared_execution_owner=1`.
- `models_prepared_execution_owner_files=1`.
- `models_lines=4895`, down from 5273 after operation-owner extraction.
- Architecture violations remain empty.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test execution_lifecycle --locked --offline --features builtins --quiet`: 41 passed.
- `cargo test -p opi-ai --test tls_contract --locked --offline --features builtins --quiet`: 25 passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: the architecture test now verifies that prepared execution is
physically outside `models.rs` and that the facade falls below 5000 lines.

`retain`: prepared-call retry, cancellation, telemetry, TLS and execution
lifecycle coverage.

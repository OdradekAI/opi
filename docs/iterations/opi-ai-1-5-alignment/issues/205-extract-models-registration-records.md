# #205 Extract Models registration records into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The five registration records now live in
`crates/opi-ai/src/models/registration.rs`.

- `ImageApiRegistration`
- `ClassifierApiRegistration`
- `DeferredApiRegistration`
- `ChatApiRegistration`
- `ChatApiEntry`

The module also owns the typed dispatch adapters, native option handles and
the catalog-invisible `ChatApiRouteProvider`. `Models` retains registration
method orchestration through a crate-private import seam, while public handle
and native-option paths remain unchanged.

`FrozenChatBinding` remains with the ChatCall orchestration slice because it is
a call-creation snapshot that captures one registration entry, route and auth
plan.

## Structure receipt

- `models_registration_records=5`.
- `models_registration_record_files=1`.
- `models_registration_records_in_facade=0`.
- `models_chat_call_files=0`.
- `models_chat_call_orchestration_in_facade=8`.
- `models_lines=4366`, down from 4895.
- Architecture violations remain empty.

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test catalog_operations --locked --offline --features builtins --quiet`: 27 passed.
- `cargo test -p opi-ai --test deferred_contract --locked --offline --features builtins --quiet` passed.
- `cargo test -p opi-ai --test custom_provider_registration --locked --offline --features builtins --quiet` passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: architecture checks now measure registration record placement and
ChatCall orchestration placement independently.

`retain`: image, classifier, deferred and custom chat registration coverage.

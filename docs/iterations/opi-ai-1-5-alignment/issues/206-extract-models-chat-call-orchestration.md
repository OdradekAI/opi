# #206 Extract Models ChatCall orchestration into a physical module

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The complete ordinary chat-call orchestration now lives in
`crates/opi-ai/src/models/chat_call.rs`.

- frozen chat binding and route snapshot
- auth plan selection
- `ChatCall` public execution handle
- `ChatCallDriver` lifecycle state machine
- setup and materialization future
- resource-retaining stream wrapper
- binding telemetry attributes and auth resolution

`Models` keeps binding assembly and the existing public entry methods. The
module still uses the shared request compiler, route auth resolver, prepared
execution owner and `StartedChatStream`, so scheduling, cancellation,
buffering, telemetry and terminal behavior remain on the existing seams.

## Structure receipt

- `models_registration_record_files=1`.
- `models_registration_records_in_facade=0`.
- `models_chat_call_files=1`.
- `models_chat_call_orchestration_in_facade=0`.
- `models_chat_call_orchestration=8`.
- `models_lines=3897`, down from 4366.
- Architecture violations remain empty.

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test execution_lifecycle --locked --offline --features builtins --quiet`: 41 passed.
- `cargo test -p opi-ai --test resource_cleanup --locked --offline --features builtins --quiet` passed.
- `cargo test -p opi-ai --test ai_telemetry --locked --offline --features builtins --quiet` passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: architecture checks now verify that all eight ChatCall orchestration
definitions live outside the facade.

`retain`: ordinary call scheduling, cancellation, resource cleanup and
telemetry regression coverage.

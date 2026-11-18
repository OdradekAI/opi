# #194 Move neutral thinking preparation to the provider contract

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The neutral `Models::stream_simple` entry no longer checks
`WireApi::PiMessages`. `Provider::prepare_simple_request` now owns neutral
thinking-level normalization. Ordinary providers use the default nearest-level
clamp. Pi route providers and Pi Chat API registrations override the hook to
preserve the requested level for adapter-owned wire validation.

Mapped providers and erased Chat API routes forward the hook to their selected
route. This keeps the behavior stable for both registered providers and typed
Chat API registrations.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `models_protocol_branches` | 8 | 7 |
| `models_owner_modules` | 4 | 4 |
| `models_lines` | 5,424 | 5,426 |

The line count increased because the lower contract and route forwarding are
explicit. Ownership moved out of the Models facade, which is the acceptance
criterion for this slice.

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test chat_pi_messages --locked --offline --features provider-pi-messages --quiet` passed, 42 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `models_protocol_branches=7`.

## Test impact

`update`: Pi neutral/native thinking and Chat API route coverage.

`delete`: Pi wire branch from `Models::stream_simple`.

`retain`: ordinary simple-request clamping, mapped routes, typed Chat API
registrations and Pi adapter fallback behavior.

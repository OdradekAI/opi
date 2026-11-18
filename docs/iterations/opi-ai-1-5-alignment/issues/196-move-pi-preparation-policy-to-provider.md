# #196 Move Pi shared-preparation policy to providers

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The Pi-specific early return in `compile_prepared_request` is now a provider
capability. `Provider::skips_shared_request_preparation` is forwarded through
mapped providers and erased Chat API routes. Pi provider and Chat API adapters
own the bypass, while Models keeps the shared preparation state machine.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `models_protocol_branches` | 6 | 5 |
| `models_owner_modules` | 4 | 4 |
| `api_provider_edges` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test chat_pi_messages --locked --offline --features provider-pi-messages --quiet` passed, 42 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `models_protocol_branches=5`.

## Test impact

`update`: Pi direct, mapped and typed Chat API preparation paths.

`delete`: Pi wire branch from `compile_prepared_request`.

`retain`: shared preparation validation, Pi native fallback and lifecycle tests.

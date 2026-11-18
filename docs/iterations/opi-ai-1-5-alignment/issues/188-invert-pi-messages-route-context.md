# #188 Invert the Pi Messages route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`PiMessagesRouteContext` now owns Radius endpoint, catalog and injected HTTP
client state under `provider::routes`. `providers::pi_messages::PiMessagesProvider`
remains a canonical type alias with the existing default catalog and client
configuration helpers.

The Pi Messages API adapter, body builder, prepared dispatch and direct Chat
API implement against `PiMessagesRouteContext`. Native options remain owned by
the adapter.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 7 | 6 |
| `api_route_context_edges` | 5 | 6 |
| `provider_to_provider_edges` | 1 | 1 |
| `profile_native_state` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-pi-messages` passed.
- `cargo test -p opi-ai --test chat_pi_messages --locked --offline --features provider-pi-messages --quiet` passed, 42 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=6` and `api_provider_edges=6`.

## Test impact

`update`: Pi Messages chat regression and architecture metrics.

`delete`: concrete Pi Messages API/profile dependency.

`retain`: Radius endpoint, model catalog, native options, body, stream,
terminal, auth and lifecycle coverage.

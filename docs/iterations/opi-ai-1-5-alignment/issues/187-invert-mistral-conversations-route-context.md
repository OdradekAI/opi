# #187 Invert the Mistral Conversations route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`MistralRouteContext` now owns native Conversations identity, endpoint,
catalog and injected HTTP state under `provider::routes`. The profile module
keeps `MistralConversationsProvider` as a canonical type alias and preserves
its constructors and built-in client binding.

The Mistral native API adapter, body builder, prepared dispatch and direct Chat
API implement against `MistralRouteContext`. Native options remain adapter
owned and ordinary OpenAI-compatible Mistral traffic continues to use the
shared `OpenAiRouteContext`.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 8 | 7 |
| `api_route_context_edges` | 4 | 5 |
| `provider_to_provider_edges` | 1 | 1 |
| `profile_native_state` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-mistral` passed.
- `cargo test -p opi-ai --test mistral_fixtures --locked --offline --features provider-mistral --quiet` passed, 55 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=5` and `api_provider_edges=7`.

## Test impact

`update`: Mistral Conversations fixture coverage and architecture metrics.

`delete`: concrete Mistral Conversations API/profile dependency.

`retain`: native options, request body, stream lifecycle, finish handling,
auth, callbacks and direct Chat API coverage.

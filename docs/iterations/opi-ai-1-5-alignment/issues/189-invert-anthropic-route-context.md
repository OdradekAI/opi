# #189 Invert the Anthropic Messages route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`AnthropicRouteContext` now owns Anthropic identity, endpoint, catalog, headers,
auth-invalid policy, OAuth/Copilot route flags, injected HTTP client and the
federation cache under `provider::routes`. `providers::anthropic::AnthropicProvider`
remains a canonical type alias with the existing constructors and route helpers.

The Anthropic Messages API adapter, body builder, prepared dispatch and direct
Chat API implement against `AnthropicRouteContext`. Native options and wire
state remain adapter-owned.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 6 | 5 |
| `api_route_context_edges` | 6 | 7 |
| `provider_to_provider_edges` | 1 | 1 |
| `profile_native_state` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-anthropic` passed.
- `cargo test -p opi-ai --test anthropic_fixtures --locked --offline --features provider-anthropic --quiet` passed, 155 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=7` and `api_provider_edges=5`.

## Test impact

`update`: Anthropic fixture coverage and architecture metrics.

`delete`: concrete Anthropic API/profile dependency.

`retain`: Messages body, tool projection, OAuth/federation, Copilot headers,
SSE, callbacks, retries, lifecycle and direct Chat API coverage.

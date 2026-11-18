# #184 Invert the Bedrock adapter/profile route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`BedrockRouteContext` now owns Bedrock identity, endpoint, model catalog,
standard and placeholder HTTP clients, and Smithy retry configuration under the
lower `provider::routes` contract. `providers::bedrock::BedrockProvider` remains
a canonical type alias and continues to expose the existing constructors and
credential helpers.

The Bedrock API adapter, body builder, prepared dispatch and direct Chat API
implement against `BedrockRouteContext`. Standard endpoint detection and
placeholder endpoint validation are lower route-contract helpers, with the
provider module retaining public re-exports for existing callers.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 12 | 9 |
| `api_route_context_edges` | 2 | 3 |
| `provider_to_provider_edges` | 3 | 3 |
| `profile_native_state` | 0 | 0 |

The three removed edges include the direct profile import and the two
placeholder validation calls. The remaining API/profile edges are unrelated
protocol adapters tracked by #172.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-bedrock` passed.
- `cargo test -p opi-ai --test bedrock_fixtures --locked --offline --features provider-bedrock --quiet` passed, 179 tests.
- The architecture report exposes `api_route_context_edges=3` and `api_provider_edges=9`.

## Test impact

`update`: Bedrock fixture coverage and architecture boundary metrics.

`delete`: concrete Bedrock API/profile dependency and API-owned endpoint helper calls.

`retain`: SigV4, bearer, placeholder signing, retry, endpoint policy, body,
streaming, callback, resource and direct Chat API coverage.

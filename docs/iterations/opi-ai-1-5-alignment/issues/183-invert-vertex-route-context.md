# #183 Invert the Vertex adapter/profile route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`VertexRouteContext` now owns Vertex identity, project/location routing,
endpoint, catalog, Express mode and injected HTTP state under the lower
`provider::routes` contract. Its Gemini body-engine input is the neutral
`GeminiRouteContext`, so the Vertex profile no longer imports a concrete Gemini
provider profile.

`providers::vertex::VertexProvider` remains a canonical type alias. The Vertex
API adapter and direct Chat API implement against `VertexRouteContext`, while
the existing constructors and fixture callers retain their source path.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 13 | 12 |
| `api_route_context_edges` | 1 | 2 |
| `provider_to_provider_edges` | 4 | 3 |
| `profile_native_state` | 0 | 0 |

The remaining three provider-to-provider edges are OpenAI-compatible route
composition and are tracked separately from this adapter/profile seam.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-vertex` passed.
- `cargo test -p opi-ai --test vertex_fixtures --locked --offline --features provider-vertex` passed, 73 tests.
- `cargo fmt --all -- --check` passed before the documentation-only receipt update.
- The architecture report exposes `api_route_context_edges=2` and `provider_to_provider_edges=3`.

## Test impact

`update`: Vertex fixture coverage and architecture boundary metrics.

`delete`: concrete Vertex API/profile and Vertex→Gemini profile dependencies.

`retain`: Vertex URL/auth, Express/ADC, native options, body reuse, streaming,
capture/replay and direct Chat API coverage.

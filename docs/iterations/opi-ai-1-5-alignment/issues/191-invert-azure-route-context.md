# #191 Invert the Azure dual-wire route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`AzureRouteContext` now owns Azure endpoint, API version, deployment map,
model records, injected HTTP client and the neutral OpenAI Chat route context.
The provider module keeps `AzureOpenAIProvider` as a canonical type alias and
retains the existing URL/version/deployment constructors.

The Azure API adapter now consumes `AzureRouteContext` and the neutral
`OpenAiResponsesRouteContext` for its dual-wire path. Azure no longer imports
the concrete OpenAI Chat or Responses profiles, and the provider layer has no
concrete provider-to-provider edge.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 4 | 2 |
| `api_route_context_edges` | 8 | 9 |
| `provider_to_provider_edges` | 1 | 0 |
| `profile_native_state` | 0 | 0 |

The two remaining API/profile edges belong to the OpenAI Codex Responses
adapter and its native subadapter.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-azure-openai` passed.
- `cargo test -p opi-ai --test azure_openai_fixtures --locked --offline --features provider-azure-openai --quiet` passed, 46 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=9`, `api_provider_edges=2` and `provider_to_provider_edges=0`.

## Test impact

`update`: Azure dual-wire fixture coverage and architecture metrics.

`delete`: concrete Azure API/profile, Azure→OpenAI Chat and Azure→OpenAI Responses dependencies.

`retain`: endpoint/version/deployment resolution, Chat/Responses dispatch,
auth, callbacks, lifecycle and direct Chat API coverage.

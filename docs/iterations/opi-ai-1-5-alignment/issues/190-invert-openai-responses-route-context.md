# #190 Invert the OpenAI Responses route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`OpenAiResponsesRouteContext` now owns Responses identity, endpoint, catalog,
compatibility configuration, headers, affinity/auth policy, Copilot route
flags, injected HTTP client and prepared sign-in classification under
`provider::routes`. `providers::openai_responses::OpenAiResponsesProvider`
remains a canonical type alias with its construction and policy helpers.

The Responses API adapter, deferred transport, body compiler and direct Chat
API implement against `OpenAiResponsesRouteContext`. Azure can now consume the
same neutral Responses context without importing the concrete Responses
profile.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 5 | 4 |
| `api_route_context_edges` | 7 | 8 |
| `provider_to_provider_edges` | 1 | 1 |
| `profile_native_state` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-openai-responses` passed.
- `cargo test -p opi-ai --test openai_responses_fixtures --locked --offline --features provider-openai-responses --quiet` passed, 84 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=8` and `api_provider_edges=4`.

## Test impact

`update`: Responses fixture coverage and architecture metrics.

`delete`: concrete Responses API/profile dependency.

`retain`: Responses body, SSE/deferred, affinity, auth, callbacks, lifecycle
and direct Chat API coverage.

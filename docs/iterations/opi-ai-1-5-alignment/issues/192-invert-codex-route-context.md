# #192 Invert the Codex Responses route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`OpenAiCodexRouteContext` now owns Codex endpoint, model catalog, injected HTTP
client and pooled WebSocket resource state under `provider::routes`.
`providers::openai_codex::OpenAiCodexResponsesProvider` remains a canonical type
alias and preserves the existing factory path and provider identity constant.

Both the Codex Responses adapter and its native-options subadapter now consume
the neutral route context. The API layer has no concrete provider imports.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 2 | 0 |
| `api_route_context_edges` | 9 | 11 |
| `provider_to_provider_edges` | 0 | 0 |
| `profile_native_state` | 0 | 0 |

This closes the API adapter/profile inversion gate. The remaining architecture
violations are stream/execution mutual ownership and Models protocol branches.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-openai-codex-responses` passed.
- `cargo test -p opi-ai --test openai_codex_responses --locked --offline --features provider-openai-codex-responses --quiet` passed, 87 tests.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib --quiet` passed, 374 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_provider_edges=0`, `api_route_context_edges=11` and `provider_to_provider_edges=0`.

## Test impact

`update`: Codex Responses fixture coverage and architecture metrics.

`delete`: concrete Codex profile dependency from both adapters.

`retain`: native options, managed headers, WebSocket resource ownership,
Responses body, SSE, cancellation, auth and direct Chat API coverage.

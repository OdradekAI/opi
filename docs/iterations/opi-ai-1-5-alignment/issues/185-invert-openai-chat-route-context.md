# #185 Invert the OpenAI Chat adapter/profile route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`OpenAiRouteContext` now owns OpenAI-compatible route identity, endpoint,
catalog, compatibility policy, model overrides, headers, auth-invalid policy,
cache flags and injected HTTP client state under `provider::routes`.
`providers::openai::OpenAiChatProvider` remains a canonical type alias and
keeps its constructors and public compatibility aliases.

The OpenAI Chat API adapter, compatibility resolution, body builder, prepared
dispatch and direct Chat API implement against `OpenAiRouteContext`. The API
module no longer imports the concrete OpenAI profile.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 9 | 8 |
| `api_route_context_edges` | 3 | 4 |
| `provider_to_provider_edges` | 3 | 3 |
| `profile_native_state` | 0 | 0 |

The route context is shared by OpenAI-compatible profiles. Mistral, OpenRouter
and Azure profile-to-profile construction remains as the next composition
slice.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-openai-chat` passed.
- `cargo test -p opi-ai --test openai_chat_fixtures --locked --offline --features provider-openai-chat --quiet` passed, 116 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `api_route_context_edges=4` and `api_provider_edges=8`.

## Test impact

`update`: OpenAI Chat fixture coverage and architecture boundary metrics.

`delete`: concrete OpenAI API/profile dependency.

`retain`: compatibility inference, body projection, SSE, auth, callbacks,
cache/session headers, lifecycle and direct Chat API coverage.

# #186 Share OpenAI route context across compatible profiles

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

Mistral and OpenRouter now construct `provider::routes::OpenAiRouteContext`
directly. Their profile modules no longer import or construct the concrete
`providers::openai::OpenAiChatProvider` profile. The OpenAI Chat wire remains the
single operation owner, while profile modules supply identity, endpoint,
headers, compatibility defaults and model catalogs through the neutral context.

Azure remains the final concrete OpenAI profile-to-profile edge because its
dual Chat/Responses route still needs the Responses route context migration.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 8 | 8 |
| `api_route_context_edges` | 4 | 4 |
| `provider_to_provider_edges` | 3 | 1 |
| `profile_native_state` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-mistral` passed.
- `cargo check -p opi-ai --locked --offline --features provider-openrouter` passed.
- `cargo test -p opi-ai --test provider_mistral_profile --locked --offline --features provider-mistral --quiet` passed, 1 test.
- `cargo test -p opi-ai --test provider_openrouter_profile --locked --offline --features provider-openrouter --quiet` passed, 1 test.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `provider_to_provider_edges=1`.

## Test impact

`update`: Mistral and OpenRouter profile conformance plus architecture metrics.

`delete`: concrete OpenAI profile construction from Mistral and OpenRouter.

`retain`: OpenAI-compatible body, catalog, auth and route behavior coverage.

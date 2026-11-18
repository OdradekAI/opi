# #195 Move authenticated model admission to providers

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`Models::get_available` no longer checks `WireApi::OpenAiResponses` for
`responses_only` credentials. The lower `Provider` contract now exposes
`model_allowed_after_auth`. `ApiMappedProvider` applies its route admission
policy, while Models retains only availability and account-model intersection.

This keeps authentication-derived protocol restrictions next to the provider
route graph and works for mapped providers without adding protocol branches to
the facade.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `models_protocol_branches` | 7 | 6 |
| `models_owner_modules` | 4 | 4 |
| `api_provider_edges` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test auth_contracts --locked --offline --features builtins --quiet` passed, 108 tests.
- `cargo test -p opi-ai --test oauth_store_integration --locked --offline --features builtins --quiet` passed, 10 tests.
- The architecture report exposes `models_protocol_branches=6`.

## Test impact

`update`: authenticated availability and OAuth account-model filtering tests.

`delete`: Responses-only wire comparison from the Models facade.

`retain`: route admission, provider catalog order and account model filtering.

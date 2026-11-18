# #207 Close the G3 API contract seam

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`api::contract` now consumes `EventStream`, `ModelInfo`, `ProviderError`,
`Request` and request validation through `provider::operation` re-exports.
The lower operation module owns the public bridge, while `provider.rs` keeps
its existing compatibility surface and does not import API contracts.

The remaining `types` and `errors` coupling is recorded in #172 as shared
contract-bundle coupling. This slice closes the direct API-contract/provider
implementation edge without claiming a broader purity rewrite.

## Structure receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `api_contract_provider_edges` | 1 | 0 |
| `provider_contract_api_edges` | 0 | 0 |
| architecture violations | G3 direct contract edge | none |

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test chat_pi_messages --locked --offline --features builtins --quiet`: 42 passed.
- `cargo test -p opi-ai --test deferred_contract --locked --offline --features builtins --quiet`: 19 passed.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.
- Enforced architecture report has no violations.

## Test impact

`update`: the architecture report adds a direct API-contract/provider edge
metric and treats lower operation re-exports as the allowed seam.

`retain`: ChatApi, DeferredApi, request validation and provider contract tests.

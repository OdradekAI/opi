# #197 Move wire thinking budget policy to providers

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`compile_prepared_request` now owns only shared output/context ceilings and
delegates thinking budget preparation to `Provider::prepare_thinking_request`.
The default capability preserves the reviewed Anthropic and non-adaptive
Bedrock numeric budget behavior, and provider implementations can override it
when a wire needs different semantics.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `models_protocol_branches` | 5 | 2 |
| `models_owner_modules` | 4 | 4 |
| `api_provider_edges` | 0 | 0 |

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --test anthropic_fixtures --locked --offline --features provider-anthropic --quiet` passed, 155 tests.
- `cargo test -p opi-ai --test bedrock_fixtures --locked --offline --features provider-bedrock --quiet` passed, 179 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report exposes `models_protocol_branches=2`.

## Test impact

`update`: Anthropic and Bedrock thinking budget regression coverage.

`delete`: Anthropic/Bedrock wire branches from Models preparation.

`retain`: shared ceilings, numeric thinking budgets, adaptive Bedrock effort,
Claude combined ceilings and provider wire fixtures.

# #182 Invert the Gemini adapter/profile route context

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`GeminiRouteContext` now lives under the lower `provider::routes` contract and
owns Gemini identity, endpoint, model catalog, wire selection and injected HTTP
client state. `providers::gemini::GeminiProvider` remains a source-compatible
canonical type alias with its constructors and catalog helpers.

The Gemini API adapter, body builder, prepared dispatch and direct Chat API now
implement against `GeminiRouteContext`. The adapter has no concrete
`crate::providers::*` import. Existing callers using `GeminiProvider` continue
to resolve the same methods through the canonical alias.

## Structural receipt

The architecture report adds `api_route_context_edges` as the positive seam
metric. This slice removes one concrete API/profile edge:

| Metric | Before | After |
| --- | ---: | ---: |
| `api_provider_edges` | 14 | 13 |
| `api_route_context_edges` | 0 | 1 |
| `profile_native_state` | 0 | 0 |

The remaining API/profile edges are tracked by #172 and are migrated in later
protocol slices. The route context count measures real lower-contract imports,
not a renamed profile import.

## Verification

- `cargo check -p opi-ai --locked --offline --features provider-gemini` passed.
- `cargo test -p opi-ai --test gemini_fixtures --locked --offline --features provider-gemini` passed, 71 tests.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed.

## Test impact

`update`: Gemini fixture coverage and architecture boundary metrics.

`delete`: concrete Gemini profile dependency from the API adapter.

`retain`: direct Gemini body, streaming, callback, auth, cancellation and
typed Chat API coverage.

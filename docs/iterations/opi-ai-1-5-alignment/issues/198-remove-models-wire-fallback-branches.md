# #198 Remove Models wire fallback branches

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The final `WireApi` branches in Models were fallback route metadata, not
provider policy. `Provider::default_wire_api` now owns the fallback identity,
while the prepared route resolves its wire from the selected record, catalog
or provider capability in that order.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `models_protocol_branches` | 2 | 0 |
| `models_owner_modules` | 4 | 4 |
| `api_provider_edges` | 0 | 0 |
| architecture violations | Models protocol branches | none |

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib --quiet` passed, 374 tests.
- `cargo fmt --all -- --check` passed.
- The architecture report now reports no violations.

## Test impact

`update`: prepared route fallback and full Models regression coverage.

`delete`: final `WireApi` fallback comparisons from Models.

`retain`: provider default wire identity, selected record routing and all owner
facade behavior.

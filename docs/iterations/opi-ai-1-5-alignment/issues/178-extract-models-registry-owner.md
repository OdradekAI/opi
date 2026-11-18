# #178 Extract the Models registry owner

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`RegistryOwner` now owns `ProviderRegistry`, provider resource namespaces, the
all-owner cleanup list and the registration gate. `Models` remains the public
facade and keeps its existing methods and signatures, while registration,
lookup and prepared-call code reach the private owner explicitly.

This is the first G5 owner slice. Auth, catalog, operation registration and
prepared execution remain the next private owners.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `cargo test -p opi-ai --locked --offline --features builtins --lib`: 374 passed.
- The architecture report now exposes `models_owner_modules=1` and keeps
  `models_lines=5370` as a diagnostic rather than a completion gate.

## Test impact

`update`: Models construction and registry/resource access now initialize and
use the private registry owner.

`delete`: the flat registry/resource field ownership from `Models`.

`retain`: all Models registration, lookup, auth, catalog, prepared-call,
resource cleanup and provider fixture coverage.

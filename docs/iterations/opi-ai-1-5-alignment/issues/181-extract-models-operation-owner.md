# #181 Extract the Models operation and prepared execution owners

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`OperationOwner` now owns chat, image, classifier and deferred API
registrations. The existing `PreparedProviderCall` remains the prepared
execution owner for frozen route, auth, request, retry and attempt state.
Registration maps and prepared execution state are therefore explicit private
owners behind the stable `Models` facade.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `catalog_personalization`: 5 passed after the owner migration.
- The architecture report now exposes `models_owner_modules=4` and
  `models_prepared_execution_owner=1`.

## Test impact

`update`: all four adapter registration maps initialize through
`OperationOwner`.

`delete`: flat operation registration ownership from `Models`.

`retain`: chat/image/classifier/deferred registration, prepared-call retry,
auth freezing and execution lifecycle coverage.

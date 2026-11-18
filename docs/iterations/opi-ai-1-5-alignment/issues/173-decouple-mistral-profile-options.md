# #173 Decouple Mistral profile from native wire options

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Goal

Keep `MistralConversationsProvider` responsible for route identity, endpoint,
catalog and transport state. Keep `MistralOptions` and request-scoped wire
state in the canonical Mistral API adapter.

## Implemented seam

`MistralConversationsWireAdapter` now carries optional native options for one
dispatch. `MistralConversationsProvider` has no API-native option field and no
provider-owned `with_native_options` state mutation. The public helper path
returns the wire adapter, while ordinary Provider dispatch uses the profile's
neutral path with no native options.

The API adapter supplies the selected options directly to request-body
projection and stream execution. Existing fixture callers continue to exercise
the same wire path through the adapter type.

## Verification

- `cargo fmt --all` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `mistral_fixtures`: 55 passed.
- `mistral_repeated_finish`: 4 passed.
- The architecture report dropped profile native-state references from 18 to 14
  and identifies four remaining profile families.

## Test impact

`update`: Mistral fixture helpers now receive the canonical wire adapter when
native options are required.

`delete`: Mistral profile-owned native option storage and mutating state.

`retain`: Mistral request serialization, streaming, callbacks, retries,
telemetry, repeated terminal and lifecycle assertions.

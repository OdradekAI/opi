# #177 Decouple Vertex profile from native wire options

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`VertexWireAdapter` now owns per-dispatch `GoogleVertexOptions`, including
project and location overrides. The Vertex profile retains route identity,
endpoint, catalog and client state. The adapter binds the cloned route snapshot
and delegates body projection to the parameterized Gemini engine without
storing native options on the registered profile.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `vertex_fixtures`: 73 passed.
- The architecture report reached `profile_native_state=0`, closing G1.

## Test impact

`update`: Vertex native body helpers accept the canonical wire adapter.

`delete`: profile-owned Vertex native option storage and mutation.

`retain`: Vertex Express/ADC routing, project/location overrides, Gemini body
projection, SSE lifecycle, callbacks, cancellation, signatures, telemetry,
resource and direct Chat coverage.

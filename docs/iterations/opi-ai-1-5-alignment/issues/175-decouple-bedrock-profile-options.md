# #175 Decouple Bedrock profile from native wire options

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`BedrockWireAdapter` now owns per-dispatch `BedrockOptions`. The Bedrock
profile retains endpoint, model catalog, clients and Smithy retry policy. Both
neutral model dispatch and the direct Converse Chat API bind options on the
short-lived adapter, while request body projection and retry headers continue
to use the same provider transport owner.

## Verification

- `cargo fmt --all` passed.
- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `bedrock_fixtures`: 179 passed.
- The architecture report dropped profile native-state references from 10 to 6
  and identifies Gemini and Vertex as the remaining profile families.

## Test impact

`update`: Bedrock body fixture helpers accept the adapter-owned body builder.

`delete`: profile-owned Bedrock native option storage and mutation.

`retain`: Converse body projection, SigV4/Bearer routing, retry headers,
stream lifecycle, callbacks, cancellation, resource and direct Chat coverage.

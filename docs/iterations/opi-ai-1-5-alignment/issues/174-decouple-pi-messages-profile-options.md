# #174 Decouple Pi Messages profile from native wire options

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`PiMessagesWireAdapter` now owns per-dispatch `PiMessagesOptions`. The Radius
profile retains endpoint, catalog and HTTP client state. Native fixture and
Chat API paths use the adapter while ordinary profile dispatch remains neutral.

## Verification

- `cargo fmt --all` passed.
- `chat_pi_messages`: 42 passed.
- `pi_messages_terminal`: 2 passed.
- The architecture report dropped profile native-state references from 14 to 10
  and identifies Bedrock, Gemini and Vertex as the remaining families.

## Test impact

`update`: Pi Messages fixture helpers accept the canonical wire adapter when
native options are required.

`delete`: profile-owned Pi Messages native option storage and mutation.

`retain`: Pi Messages wire, catalog, callback, cancellation and terminal
metadata coverage.

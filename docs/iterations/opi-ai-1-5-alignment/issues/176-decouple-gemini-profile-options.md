# #176 Decouple Gemini profile from native wire options

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

`GeminiWireAdapter` now owns per-dispatch `GoogleGenerativeAiOptions`. The
Gemini profile retains endpoint, catalog, client and wire identity. Neutral
dispatch and the direct Google Generative AI Chat API bind options on the
adapter, while the shared body and stream engine remains reusable by Vertex.

Vertex temporarily keeps its own native option value while it consumes the
new option-parameterized Gemini body engine. The follow-up Vertex slice moves
that value into a Vertex wire adapter and removes the remaining profile state.

## Verification

- `cargo check -p opi-ai --locked --offline --features builtins` passed.
- `gemini_fixtures`: 71 passed.
- `vertex_fixtures`: 73 passed, covering the transitional shared engine path.
- The architecture report shows no Gemini native state. The remaining
  `profile_native_state=6` references are all in Vertex.

## Test impact

`update`: Gemini native body helpers accept the canonical wire adapter.

`delete`: Gemini profile-owned native option storage and mutation.

`retain`: Gemini and Vertex SSE mapping, authentication, callbacks,
cancellation, signatures, thinking, tool choice, direct Chat and lifecycle
coverage.

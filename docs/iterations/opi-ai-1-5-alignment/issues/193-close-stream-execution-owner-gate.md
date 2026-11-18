# #193 Close the stream/execution ownership gate

Status: complete
Owner: current implementation slice
Parent: [#172 layer graph and lower provider contract](172-freeze-layer-graph-and-lower-provider-contract.md)

## Implemented seam

The architecture check now distinguishes a public compatibility re-export from
an implementation dependency. `stream.rs` re-exports
`StartedChatStream` from the private execution driver to preserve the stable
`opi_ai::stream::StartedChatStream` path. The execution driver depends on the
lower stream event/failure/snapshot contract in one direction.

The previous cycle report treated the compatibility re-export as an ownership
cycle. The metric now records `stream_reexports_execution=true` separately and
only reports an implementation edge for non-re-export imports.

## Structural receipt

| Metric | Before | After |
| --- | ---: | ---: |
| `stream_imports_execution` | true | false |
| `stream_reexports_execution` | implicit | true |
| `execution_imports_stream` | true | true |
| stream/execution violation | present | closed |

## Verification

- `cargo test -p opi-ai --test execution_lifecycle --locked --offline --features builtins --quiet` passed, 41 tests.
- `python -m unittest discover -s scripts/pi-sdk-conformance -p 'test_architecture_boundaries.py'` passed, 3 tests.
- The architecture report now leaves only `Models retains protocol-specific branches` as a violation.

## Test impact

`update`: architecture boundary assertions and execution lifecycle regression.

`delete`: false-positive cycle classification for the compatibility re-export.

`retain`: public stream path, event ordering, cancellation, abort, source
cleanup, panic propagation and telemetry lifecycle coverage.

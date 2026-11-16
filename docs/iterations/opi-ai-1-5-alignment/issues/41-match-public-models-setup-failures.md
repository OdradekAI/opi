# Pair public Models setup failure observations

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [rebuild from the frozen npm lock](40-rebuild-frozen-lock-candidate.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` plus the #39 and #40 worktree changes

## Compared entry points

On 2026-11-11 the probes used the #40 candidate at
`C:/Temp/opi-ai-frozen-lock-20261111`, from Pi 1.5.0 frozen source manifest
`959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072`.
The Pi setup probe imports `packages/ai/dist/models.js`, the public
`./models` export. Its `streamSimple` passes an async setup wrapper to
`lazyStream` at `packages/ai/src/models.ts:929–940`. The Rust probe uses
`Models::stream_simple` and public `register_route` from
`crates/opi-ai/src/models.rs`.

- `models-missing-provider` requests `missing-provider:mock-model` on two
  empty collections. Both fail route lookup before authentication or dispatch.
- `models-auth-setup-failure` registers the same provider/model identity and
  fails the registered auth resolver after releasing a named barrier. The Pi
  probe uses a synthetic explicit API key to reach `ApiKeyAuth.resolve` via
  `packages/ai/src/auth/resolve.ts:56–67`. Rust uses the registered `AuthResolver`.
  The barrier is reached before either driver polls the public stream, and
  the dispatch counters stay at zero.

The two source-completion cases continue to observe their original gates.
Their construction counters refer to the raw Pi setup callback and Rust
provider dispatch. The public auth case measures resolver entry on both sides.
The standalone `lazy-setup-failure-phases` probe still observes synchronous
throw escaping `packages/ai/src/api/lazy.ts:74–85` and rejected setup yielding
one error terminal. Its fault remains separate from the public Models traces.

## Observed behavior

Both public faults yielded one error and then EOF, and the final outcome
agreed with the failure delivered by the stream. The Pi test compares message
identity and the Rust test compares `CallFailure` values. Both retain the fault
marker in the diagnostic. The missing route yields Rust `Config` and the
gated auth failure yields `AuthFailed`.

| Observable boundary | Pi 1.5.0 | Rust |
| --- | --- | --- |
| Auth resolver entries when the call returns | One on the explicit-key path | Zero on the current-thread Tokio probe, then one before consumer polling |
| Failure delivery | `error` event followed by a fulfilled `result()` containing the error message | Stream item `Err(CallFailure)` and final `Err(CallFailure)` |
| `provider` and `model` in the failure message | Requested provider and `mock-model` | Empty for both tested setup failures |

The timing evidence describes this resolver path and the current-thread
schedule. A multithreaded executor can run scheduled work concurrently with
construction. Aligning an inline before-return guarantee would require moving
user resolver work into the constructing call. The existing typed result
also has a documented consumer contract. Both differences still require a
decision under [public compatibility](../decisions/04-public-contract.md) and
[lifecycle](../decisions/11-telemetry-lifecycle.md).

The missing message identity can be addressed independently. The frozen
request record already supplies the requested provider and model when the
setup driver creates the synthetic failure. The narrow follow-up is to carry
those values into the pre-dispatch failure message while retaining the typed
cause and scheduling contract.

## Verification

The new material manifest matches #40's upstream source, lock, dist and
dependency inspection. The four changed probe inputs have new hashes.
The runner recorded 12 TypeScript invocations and four Rust invocations.
All four shared comparisons passed and `blocked_gates` is empty. The overall
verdict is `DIFFERENCES_REQUIRE_DECISION` / exit 0.

- [Material manifest](../evidence/phase5-matched-models-setup-materials.json), SHA-256 `4cdb6c86d1d9231d2129bbb0f6dee427653dc69bc3fc199e901e839b8e58381f`.
- [Replay commands and raw traces](../evidence/phase5-matched-models-setup-differential.json), SHA-256 `57a3e5f30c706261ca60f9cf068fb901b097a58ad23ad85f6c74c16858b1143c`.
- `python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p 'test_*.py' -v` passed 18 tests, including actual Rust probes and negative traces.
- Cached `test -p opi-ai --example lifecycle_differential --offline -j4` passed both Drop tests. Cached `clippy -p opi-ai --example lifecycle_differential --offline -j4 -- -D warnings` and `cargo fmt --all --check` passed.
- The raw lazy probe passed. The TypeScript CLI rejected an unknown case with exit 1.

Test impact `add` for paired public faults and comparator negatives, `update`
for the drivers and fixtures, `delete` for the current runner's mixed-layer
setup comparison, and `retain` for the original raw probe, source-completion
and Drop tests, and historical evidence.

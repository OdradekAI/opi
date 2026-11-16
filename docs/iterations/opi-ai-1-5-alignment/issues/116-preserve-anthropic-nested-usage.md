# #116 Preserve Anthropic nested usage details

Status: complete for the nested Anthropic usage slice; full Pi alignment remains open.

Continuation of #115. The frozen Pi 1.5.0 source reads
`cache_creation.ephemeral_1h_input_tokens` and
`output_tokens_details.thinking_tokens` from both `message_start` and
`message_delta`. The Rust adapter previously only recognized the legacy flat
1-hour field and discarded the reasoning subset.

## Contract and implementation

`RawUsage` now accepts the nested cache and reasoning objects while retaining
the flat `cache_creation_input_tokens_1h` alias for older proxies. Nested data
takes precedence when present. The public `AnthropicUsageDelta` carries an
optional `reasoning_tokens` subset alongside the existing cache subset:

- an absent or null nested field leaves the accumulated fact unchanged;
- an explicit zero clears the subset;
- `cache_creation.ephemeral_1h_input_tokens` must not exceed merged
  `cache_creation_input_tokens`;
- `output_tokens_details.thinking_tokens` must not exceed merged
  `output_tokens`; and
- invalid updates fail atomically, preserving the previous partial.

The shared `Usage` shape already persists both subsets. The coding-agent
aggregator and `SessionCoordinator` therefore retain the corrected values
through `MessageEnd`, disk reconstruction and `open_existing` without a new
serialization format.

## Verification

The [materials receipt](../evidence/phase6-anthropic-nested-materials.json)
binds the frozen Pi snapshot, current adapter/test sources and raw probe logs.
The [paired receipt](../evidence/phase6-anthropic-nested-differential.json)
compares Pi simple/direct routes for success and wire-error against the Rust
adapter trace. Both produce `[100, 20, 0, 200, 75, 6]` for input, output,
cache-read, cache-write, cache-write-1h and reasoning tokens. The low-level
mapper intentionally has no `ModelInfo`, so its cost is zero; the actual
Agent route/session regression covers catalog cost calculation separately.
The raw mapper also retains an accepted stop on a later wire error; the
canonical ChatStream/Agent failure path still reports `error`.

The [actual package receipt](../evidence/phase6-anthropic-nested-package.json)
executes the current SDK archive and companion telemetry archive in both
`builtins` without tracing and default profiles. It runs the existing five
Anthropic usage tests plus the nested trace from the extracted package. The
package source is checked against the candidate archive; no workspace source
patch substitutes for the archive.

The [final checks receipt](../evidence/phase6-anthropic-nested-checks.json)
binds the full workspace run, doctests, rustdoc, targeted caller tests,
documentation checks, cache status and unchanged historical evidence.

The targeted adapter suite covers 154 Anthropic fixture tests and the five
usage/observer tests. The actual coding-agent test covers success and wire
failure, corrected nested subsets, catalog cost and session save/read/recovery.
Invalid cache and reasoning subset updates are asserted to be atomic.

Test impact: `add` nested adapter/trace, paired Pi probe/comparator, package
consumer and nested Agent/session assertions; `update` Anthropic raw usage
mapping and the existing usage recovery fixture; `retain` all previous flat
usage, terminal, observer, numeric, preview, cancellation and EOF evidence;
`delete=none`.

The low-level cost and wire-error observations above are bounded ownership
differences, not regressions. No previous evidence or approved difference
contract was rewritten. The next priority is the remaining adapter/observer
contracts and public helper callers, followed by full configuration V,
MSRV/native-platform, budget and real Deferred acceptance.

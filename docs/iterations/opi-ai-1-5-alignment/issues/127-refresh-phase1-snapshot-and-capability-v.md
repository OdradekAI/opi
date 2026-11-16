# #127 Refresh the phase-one current snapshot and map capability-granular V

Status: current snapshot verified; capability-granular V is inventoried with an explicit open scope.

The frozen phase-one baseline and source inventory remain historical receipts. The
current worktree has since split Rust modules, added adapter/observer regressions,
changed manifests and lockfiles, and added tests. Running the historical verifier
against that tree therefore reports source and line-evidence drift by design.

## Current verifier boundary

`verify_phase1.py` now has an explicit `--mode current` path. It verifies a fresh
`freeze_baseline.py` capture, the current raw `source_inventory.py` output, exact
file sets and content hashes, parser versions, syntax/re-export status, and
declaration/entrypoint uniqueness. The default mode still verifies the original
phase-one contract index, ownership links, line evidence and Pi observations.
Neither mode rewrites the other mode's receipts.

The current capture is recorded in [the current verification receipt](../evidence/phase1-current-verification-20261115.json); the raw current baseline and
source inventory are retained beside it. The historical verifier failure is kept as
a separately hashed log so the old drift remains auditable rather than silently
reclassified.

## Capability V

[The capability matrix](../evidence/phase8-capability-v-20261115.json) names the
current `core`, `telemetry-tracing`, and `builtins` feature rows and maps the
capabilities hidden inside `builtins`: protocol adapters, HTTP, cloud auth/codecs,
catalog, TLS/crypto, WebSocket/compression and provider OAuth/store. Core and
tracing have independent passing rows on the native Linux/Windows and archive
consumer receipts. The builtins-backed rows are `COARSE_ONLY`; the matrix does
not relabel them as independent single, pair or higher-order configurations.

The declared all-features row remains a manifest-level pass from #126. Complete
capability V requires feature ownership decomposition, then independent pair and
higher-order checks with fresh native and package evidence. Native macOS remains
blocked by the user's recorded lack of an environment.

The current Windows candidate was rerun against the live worktree with the
persistent Cargo target: `core_profile` passed 3 tests with no defaults,
`core_profile` passed 4 tests with only `telemetry-tracing`, and the coarse
`builtins` `platform_delivery` profile passed 9 tests. Raw logs and hashes are
bound in the matrix's `current_validation` section. All four declared metadata
profiles (`core`, `core-tracing`, `builtins-no-tracing`, and `defaults`) also
exit successfully in the current manifest; their metadata hashes are retained
separately from runtime/package evidence.

The next implementation order is to split the bundled catalog/data owner first,
then the shared HTTP/protocol owner, followed by cloud auth/OAuth-store and the
TLS plus WebSocket/compression owners. Each new feature boundary must repeat the
current native profile and external archive consumer checks before it can enter
the independent or pair rows.

Test impact: `add` current snapshot verifier mode, current hashes, and the
capability matrix; `update` none in product runtime; `delete` none; `retain` all
frozen phase-one, Pi differential, adapter/observer, native platform and package
receipts.

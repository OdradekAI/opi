# Validate supplemented materials and strengthen lifecycle observations

Status: done
Owner: Codex for user
Parent: [opi-ai alignment specification](../spec.md)

## Scope and acceptance

After the user supplied installed dependencies, dist and generated model data,
record their actual identity without replacing the original source baseline or
assuming that existence implies parity. Strengthen the three lifecycle probes
with a terminal-release barrier, actual public results, source EOF/Drop
observations, negative comparator cases and immutable evidence outputs.

This is an acceptance-tooling slice, not a production lifecycle repair, feature
implementation, upstream reinstall/hydration or release. A tool that correctly
records a product failure passes its tooling check; the product comparison stays
failed. No difference is approved by a comparator classification.

## Outcome

- Actual dependency resolution includes nested OpenAI/proxy packages and local
  schema/telemetry packages. All ten selected direct runtime dependencies match
  the frozen declarations/lock excerpts.
- Dist inventories and source-map/output completeness are recorded, not merely
  directory presence. These are not reproducible-build or npm consumer proofs.
- Current root lock differs from the frozen hash. Original full-lock bytes were
  not recovered, so transitive equality remains unproved.
- Current generated model data passes internal validation but differs from the
  pinned response in nine providers: 1712 pinned versus 1738 current entries.
- The gated no-consumption and stop-consumption cases expose an ordinary-provider
  source-tail mismatch: `drive_resolved_call_registered` stops polling the
  internal attempt after its business terminal. Source Drop occurs, source EOF
  does not. Both comparisons are `FAIL`, not accepted Rust-native differences.
- Setup failure compares only a single error terminal. Its distinct injection
  points and error channels remain visible and unapproved.

Report, exact identities and checks: [supplemented-materials report](../phase5-supplemented-materials-and-gated-differential.md).

## Test impact and boundary

`add`: material inspector, drift/negative-path tests, runner verdict tests and
explicit consumer-Drop source probes. `update`: freezer, comparator and Rust
trace tests. `retain`: historical evidence, original upstream probe and all
product regression tests. `delete`: none.

Product source, upstream files, manifests and lockfiles remain unchanged.
The original phase-one rows remain frozen; current observations are appended to
the existing contract index. Complete provider/features/platform/MSRV, resource
budgets and six-gate acceptance remain open.

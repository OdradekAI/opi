# Audit proof obligations

Use this reference to turn registered Phase requirements into falsifiable
evidence obligations before reviewing implementation details.

## Requirement matrix

Create one row per mandatory requirement:

| Requirement | Source | Observable behavior | Production surfaces | Test/fixture evidence | Check | State |
|---|---|---|---|---|---|---|
| R1 | `<path:section>` | ... | ... | ... | ... | `met` / `partially-met` / `not-met` / `not-assessable` |

Do not merge requirements merely because one change implemented them together.
Every row must be decidable from cited evidence. `not-assessable` means the
auditor could not obtain sufficient evidence, not that the requirement passes.

## Review axes

Run these lenses against the complete current surfaces named by the matrix:

- **Standards**: repository rules, Rust correctness, dependency direction,
  failure behavior, public contract consistency, and documentation lockstep.
- **Spec**: each registered requirement and observable acceptance behavior.
- **Security/authority**: validation, permissions, process boundaries, secrets,
  durable inputs, and fail-closed behavior.
- **Invariants/integration**: state transitions, adapters, persistence,
  protocol/schema consumers, CLI/TUI/model-visible output, and recovery paths.
- **Test quality**: positive, invalid-input, boundary, integration, fixture, and
  conformance coverage at the owning seam.
- **Residuals**: placeholders, compatibility remnants, dead branches, duplicated
  mechanisms, unused public surface, and unresolved TODOs in Phase scope.

## Test anti-vacuity

For every test used as acceptance evidence, establish:

1. the assertion observes the required behavior rather than only success or
   object construction;
2. the test reaches the production path, not a parallel helper or mock-only
   implementation;
3. its input distinguishes the required behavior from the previous/default
   behavior;
4. invalid or boundary cases exercise the claimed validation/failure boundary;
5. fixtures and snapshots contain the field or state whose preservation is
   claimed;
6. skipped, ignored, platform-gated, or feature-gated tests are not counted as
   universal evidence without a stated limitation.

A command that passes without a discriminating assertion is weak evidence and
cannot by itself mark a mandatory requirement `met`.

## Blocker/Major refutation

Before publishing a Blocker or Major, attempt to refute it:

- search every production caller and alternate implementation path;
- check exact locked dependency behavior when the claim depends on it;
- inspect tests and fixtures for counterexamples;
- reproduce the defect or show a static invariant violation;
- distinguish an absent requirement from an intentionally registered deferral.

Record the strongest counter-evidence and why it fails to refute the claim.
If the claim cannot survive this Blocker/Major refutation, downgrade, rewrite,
or remove it.

## Minimum-change conformance matrix

Audit every admitted implementation task at current committed `audit_head`.
The matrix is requirement evidence, not a new finding axis.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|

Verify the recorded `reuse_search`, `surface_necessity`, and
`simplification_ceiling` against current code. For any introduced public seam,
record `production_consumers`, `nonproduction_consumers`, `net_deletion`, and
`residual_glue`. Use `conforming`, `drifted`, `triggered`, `not-recorded`, or
`not-assessable` as status.

- `conforming`: evidence still supports the recorded minimum-change decision.
- `drifted`: current code no longer matches the trace.
- `triggered`: a recorded revisit condition now holds.
- `not-recorded`: legacy work has no required trace; inspect current evidence
  directly and do not invent historical intent.
- `not-assessable`: required evidence is unavailable or contaminated.

Route any actionable defect through the existing `standards`, `spec`,
`security`, `test-quality`, `invariants`, `integration`, or `residuals` axis.

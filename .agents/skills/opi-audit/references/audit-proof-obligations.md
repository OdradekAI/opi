# Audit Proof Obligations

Turn the latest committed registered requirements into falsifiable evidence
obligations before inspecting implementation.

## Requirement records

Create one record per independently decidable requirement in the current
member's `audit.<reviewer-id>.<model-id>.requirements.jsonl` sidecar using the
shared audit-set contract. Seal these fields first:

- `audit_run_id`, requirement `id`, and `mandatory`;
- exact criterion path, SHA-256, and citation;
- observable behavior.

Then populate `production_surfaces`, `test_evidence`, `checks`, `state`, and
reciprocal `finding_ids` from current `audit_head` evidence. Do not merge
requirements merely because one implementation change addressed them together.

`not-assessable` means sufficient current evidence is unavailable or
contaminated. It never means pass. Each non-met mandatory record must link a
finding that explains the gap.

## Review axes

Apply these lenses to the complete current surfaces named by the records:

- **Standards**: repository rules, Rust correctness, dependency direction,
  failure behavior, public contract consistency, and documentation lockstep.
- **Spec**: each latest committed registered requirement and its observable
  acceptance behavior.
- **Security/authority**: validation, permissions, process boundaries, secrets,
  durable inputs, and fail-closed behavior.
- **Invariants/integration**: state transitions, adapters, persistence,
  protocol/schema consumers, CLI/TUI/model-visible output, and recovery paths.
- **Test quality**: positive, invalid-input, boundary, integration, fixture,
  and conformance coverage at the owning seam.
- **Residuals**: placeholders, compatibility remnants, dead branches,
  duplicated mechanisms, unused public surface, and unresolved TODOs in scope.

## Test anti-vacuity

For every test used as acceptance evidence, establish that:

1. its assertion observes the required behavior rather than only success or
   construction;
2. it reaches the production path, not a parallel helper or mock-only path;
3. its input distinguishes required behavior from previous/default behavior;
4. invalid and boundary cases exercise the claimed failure boundary;
5. fixtures and snapshots contain the field or state claimed as preserved;
6. skipped, ignored, platform-gated, or feature-gated tests are not treated as
   universal evidence without a limitation.

A passing command without a discriminating assertion is insufficient to mark a
mandatory requirement `met`.

## Blocker/Major refutation

Before publishing a Blocker or Major:

- search current production callers and alternate paths;
- verify exact locked dependency behavior when relevant;
- inspect current tests and fixtures for counterexamples;
- reproduce the defect or show a static invariant violation;
- distinguish a missing requirement from a currently registered deferral.

Record the strongest current counter-evidence and why it does not refute the
claim. Downgrade, rewrite, or remove claims that do not survive this check.
Historical audit conclusions are not counter-evidence.

## Minimum-change conformance

Audit every admitted implementation task at current `audit_head`. Verify the
recorded `reuse_search`, `surface_necessity`, and `simplification_ceiling`
against current code. For introduced public seams, record
`production_consumers`, `nonproduction_consumers`, `net_deletion`, and
`residual_glue`.

Use `conforming`, `drifted`, `triggered`, `not-recorded`, or `not-assessable`.
`not-recorded` requires direct current inspection; never invent historical
intent. Route actionable defects through the normal finding axes instead of
creating a separate historical axis.

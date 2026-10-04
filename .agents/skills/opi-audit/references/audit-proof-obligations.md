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

## Raw-proof preflight

After sealing obligations and before implementation inspection, identify the
required proof class: fixture, native, live/operator, or platform. A fixture
pass establishes only the fixture path; tool availability and credentials
establish readiness, not acceptance. Use the optional `evidence_root` only for
operator-supplied redacted raw proof, never to import audit/remediation summaries.
Keep raw ledgers and sibling conclusions outside tool output. Inventory paths
and SHA-256 first, then read only selected proof files; project any mixed
metadata to raw execution facts before context exposure.

For each required proof, verify:

- source/build identity, locked dependencies, exact client/version, material
  identity, applicable platform/features, and original proof seals;
- exact command, original process exit, executed case/trial identities and
  counts, and the raw observations that establish the criterion;
- applicability to `audit_head`: matching relevant source/dependency bytes and
  required execution conditions, rather than an old PASS or closure claim.

Missing, mismatched, truncated, or inapplicable proof leaves its criterion
`not-assessable` with a reciprocal finding when mandatory. Record any separately
proven part without calling the whole requirement `met`. Continue independent
inspection; rerun only within existing execution authorization and available
materials. Neither `evidence_root` nor missing proof authorizes paid/live calls.
Store paths, hashes, applicability and limitations in existing `test_evidence`
and `checks` fields; do not add a new ledger or sidecar schema.

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

## Boundary proof

For applicable security/authority and resource boundaries, trace each input
from admission through its production consumer. Record the following in the
existing evidence fields, selecting cases from the actual contract and parser:

1. **Validation/consumer interpretation**: compare the guard's interpretation
   with the actual locked parser or downstream consumer. Exercise relevant
   alternate forms such as URL fragments/userinfo/IPv6, canonical path aliases,
   and numeric type/extreme/overflow cases. Include an ordinary accepted input
   and an invalid input that distinguishes the boundary. A helper's rejection
   alone does not establish the consumer's destination or authority.
2. **Limits before work**: trace accepted input -> allocation/expansion/loop or
   resource acquisition -> first external effect. Verify the bound precedes
   the work it constrains in every ingress branch, including explicit values
   and values derived from selections. Guard presence or a small over-limit
   test does not prove bounded pre-validation work.
3. **Bounded isolated probes**: when static evidence is insufficient, use an
   isolated subprocess with enforced timeout and, for allocation cases, a
   memory cap; capture bounded output and use temporary/local fake materials.
   Record the input, actual consumer, first costly work/effect, bound location,
   and observed rejection or effect in `checks.observed`. If isolation cannot
   be enforced, retain a static proof or explicit limitation. Never probe an
   extreme allocation in the reviewer process or let a parser probe contact
   remote services, real credentials, or paid providers.

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

Before using a filtered test command, establish that its filter names current
tests and that ignored selection matches their annotations. Afterward, record
actual passed/failed/ignored/filtered counts and executed identities. A run with zero
executed tests is not behavioral acceptance, even with exit 0; a compile or
empty doctest check may establish compilation only. For Rust libtest logs, use
[the read-only execution checker](test-execution-check.md). Other runners need
equivalent runner-native execution evidence. The checker validates selection
and counts, not assertions, source provenance, or native/operator fidelity.

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

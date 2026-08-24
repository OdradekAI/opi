# Current Finding Verification and Closure Matrix

Use the strict union of findings from every member of the validated active
`audit.index.json`. Preserve source records unchanged; do not load history or
derive lineage. The same textual ID in two run IDs is two source keys.

## Criterion matrix

Align requirement evidence by exact criterion identity
`(path, sha256, citation)` when possible. Each reviewer/model column retains
its original requirement record and state; unaligned criteria remain separate
rows. The matrix exposes disagreement but never overwrites it with consensus.
The fail-dominant indexed verdict remains the gate.

## Current-set verification

For every `(audit_run_id, id)`:

1. reproduce or statically prove the current claim at `remediation_head`;
2. search current counter-evidence and alternate paths;
3. assign `Confirmed`, `Partially confirmed`, `Cannot confirm`, or `Refuted`;
4. retain source severity and record evidence/rationale for any final change;
5. define `closure_key`, `family_key`, decision, and exact disposition.

`Cannot confirm` is not `Refuted`. Findings from different indexed
reviewer/model runs remain separate evidence. Coverage is descriptive, never a
vote. Schedule a shared repair at the highest source severity while retaining
every original severity and rationale.

When current claims conflict, keep them separate until current code, tests,
specifications, or runtime evidence resolves the conflict. `Refuted` and final
severity changes require that independent evidence; another reviewer's
contrary conclusion is insufficient. Do not appeal to which model, title, or
conclusion appeared earlier.

When rejecting a source identity, state all three boundaries explicitly: the
current run/digest controls admission, title similarity cannot substitute for
identity, and no older source is consulted. No history or recurrence
classification enters that decision.

## Closure clustering

Two findings may share a closure batch only when all are true:

1. they have one falsifiable closure predicate;
2. one root change is expected to satisfy it for every member;
3. one red-before/green-after evidence pair discriminates closure;
4. they can be reviewed and reverted as one bounded change.

Shared files, family keys, severity, or suggested refactors are insufficient.
Use `family_key` to describe adjacent current paths without merging their
closure predicates.

## Coverage

Every current finding appears exactly once:

- in one planned fix; or
- in exclusions as Refuted, Cannot confirm, Info/no-action, Returned to
  shaping, or Deferred by a currently registered source.

The matrix must contain no source outside the current index. Each disposition
binds its owning member's `audit_run_id` and `findings_sha256`.

| Batch | Closure key/predicate | Family | Current finding IDs | Current reviewer coverage | Verification | Decision |
|---|---|---|---|---|---|---|
| B1 | ... | ... | ... | ... | ... | ... |

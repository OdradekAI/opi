# Current Finding Verification and Closure Matrix

Use only the findings from the validated active audit set. Preserve source
records unchanged; do not load older runs or derive lineage.

## Current-set verification

For every `(audit_run_id, id)`:

1. reproduce or statically prove the current claim at `remediation_head`;
2. search current counter-evidence and alternate paths;
3. assign `Confirmed`, `Partially confirmed`, `Cannot confirm`, or `Refuted`;
4. retain source severity and record evidence/rationale for any final change;
5. define `closure_key`, `family_key`, decision, and exact disposition.

`Cannot confirm` is not `Refuted`. Findings from multiple reviewers inside one
deliberately combined current run remain separate evidence. Coverage is
descriptive, never a vote.

When current claims conflict, keep them separate until current code, tests,
specifications, or runtime evidence resolves the conflict. Do not appeal to
which model, title, or conclusion appeared earlier.

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

The matrix must contain no source outside the current `audit_run_id` and
`findings_sha256`.

| Batch | Closure key/predicate | Family | Current finding IDs | Current reviewer coverage | Verification | Decision |
|---|---|---|---|---|---|---|
| B1 | ... | ... | ... | ... | ... | ... |

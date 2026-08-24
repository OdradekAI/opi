# Remediation Execution Protocol

Use only for `mode=apply` after explicit approval of the current fixed plan and
its exact digest.

## Admission

Before editing:

1. validate `remediation.plan.md` and its dispositions;
2. compare the emitted `plan_sha256` byte-for-byte with the invocation;
3. verify the exact `Audit index SHA-256` still matches the live index and
   every indexed member still validates;
4. verify current committed HEAD equals the full `Remediation head`;
5. reproduce staged, unstaged, and untracked baseline and stop on overlapping
   unowned changes;
6. verify `READY-FOR-APPLY`, no unresolved decision, complete strict-union
   coverage, and observed behavioral red-before.

Approval provenance is the current apply invocation, fixed plan path, plan
digest, active index digest, and task context. Any index, member, sidecar, or
membership byte change invalidates approval. Do not treat a different digest,
older task, blanket consent, or file existence as approval.

Do not revise approved product meaning while applying. New evidence that
changes public API, durable format, dependencies, specification, authority, or
planned scope returns to `mode=plan`.

## Closure-batch loop

For each dependency-ordered batch:

1. confirm its closure predicate and owned path set;
2. make only the planned minimum change;
3. run its green-after check;
4. run the missing part of its verification union;
5. record exact paths, commands/results, and `Closed` or `Not closed`.

Proceed only after the prior batch passes. On unexplained failure, endpoint
drift, or unowned overlap, stop and preserve evidence. Do not roll back user
work, broaden scope silently, or call the batch closed.

## Bounded verification-blocking incidental repair

When a recorded verification command exposes a new blocking defect:

1. prove the defect is directly required for the current batch to become green;
2. prove it remains inside the causal/owned surface;
3. confirm it changes no public API, durable format, dependency graph,
   registered specification, authority boundary, manifest/lockfile,
   implementation ledger, or public schema;
4. observe a focused red-before;
5. make the minimum repair and record `I<N>` in result dispositions;
6. observe focused green-after and resume the approved verification union.

If any condition fails, stop and create a new plan. Report non-blocking
observations without fixing them. A prose note is never a substitute for the
incidental machine record.

## Result contract

Write fixed `remediation.result.md` and
`remediation.result.dispositions.jsonl`. The report header contains:

```markdown
**Status**: COMPLETE
**Audit index SHA-256**: `<exact active index digest>`
**Plan SHA-256**: `<approved digest>`
**Changed paths**: ["path/owned/by/planned-or-incidental-record"]
```

Copy every planned finding disposition exactly except for observed
green-after and final remediation status. Add accepted incidentals as separate
records. The Changed paths JSON array equals the union from all result records.
Validate:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase<N>/assurance/remediation.result.md
```

## Materialization handoff

Do not modify the plan retroactively and do not commit automatically. A fresh
audit or reviewer re-run may be requested only after fixes plus the current
live set are committed, the assurance directory is clean, external evidence is
resolved, and any owning-workflow return is complete. `opi-remediate` never
invokes another explicit-only skill itself and never grants a Phase PASS.

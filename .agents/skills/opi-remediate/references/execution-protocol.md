# Remediation Execution Protocol

Use only for `mode=apply` after explicit approval of the current fixed plan and
its exact digest.

## Admission

Before editing:

1. validate `remediation.plan.md` and its dispositions;
2. compare the emitted `plan_sha256` byte-for-byte with the invocation;
3. verify the exact `Audit index SHA-256` still matches the live index and
   every indexed member still validates;
4. verify the committed source boundary from `Remediation head` under the
   shared disposition contract's plan-only commit rule;
5. reproduce staged, unstaged, and untracked baseline and stop on overlapping
   unowned changes;
6. verify `READY-FOR-APPLY`, no unresolved decision or remaining obligation,
   complete strict-union coverage, and observed behavioral red-before.

Run `validate_assurance_artifact.py apply` with the invocation's exact
`--plan-sha256` using the shared disposition contract's command. It checks
steps 1–4 and 6 under the assurance lock; step 5 remains an ownership check.
An unchanged fixed plan may be committed before apply without rewriting its
head or asking for the same approval again. Record both the source baseline
and current HEAD; apply admission itself neither commits nor starts a batch.
Capture the external baseline receipt described below before editing. A baseline
capture repeats admission; it does not grant authority beyond the invocation.

Approval provenance is the current apply invocation, fixed plan path, plan
digest, active index digest, and task context. Any index, member, sidecar, or
membership byte change invalidates approval. Do not treat a different digest,
older task, blanket consent, or file existence as approval.

Do not revise approved product meaning while applying. New evidence that
changes public API, durable format, dependencies, specification, authority, or
planned scope returns to `mode=plan`.

## Closure-batch loop

For each dependency-ordered batch:

1. confirm its prerequisites passed, closure predicate and owned path set;
2. make only the planned minimum change;
3. run its green-after check;
4. run the missing part of its verification union;
5. record exact paths, commands/results, and `Closed` or `Not closed`.

Use the criterion's required evidence class and actual production consumer.
Record test identities and executed counts, original exits and relevant
source/build/client/material identities. Retain the original execution state
when reusing evidence, including dirty-source package metadata; separately prove
current applicability. A matching log hash, compilation pass or fixture path
does not establish a stronger behavioral or packaging claim.

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
**Outcome**: APPLIED
**Audit index SHA-256**: `<exact active index digest>`
**Plan SHA-256**: `<approved digest>`
**Changed paths**: ["path/owned/by/planned-or-incidental-record"]
```

Copy every planned finding disposition exactly except for observed
green-after and final remediation status. Add accepted incidentals as separate
records. The Changed paths JSON array equals the union from all result records.
Preserve no-action dispositions and their remaining obligations. `COMPLETE`
reports completion of this result, not a Phase conformance verdict. The planning
branch's `NO-CHANGE` outcome is defined only by the shared disposition contract.
Validate:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase<N>/assurance/remediation.result.md
```

## Source and materialization proof

For an applied result, run the commands below from the repository root.
Receipts are immutable evidence written to new files outside the repository.
They bind the plan/index identities,
baseline carried-in state and tested source bytes; they are not a progress
ledger and do not authenticate test execution.

Before production edits, after apply admission:

```text
python .agents/skills/_shared/scripts/remediation_source_proof.py baseline docs/snapshots/phase<N>/assurance/remediation.plan.md --plan-sha256 <approved-digest> --output <external-baseline.json>
```

Immediately before the final verification union, capture its input:

```text
python .agents/skills/_shared/scripts/remediation_source_proof.py tested <external-baseline.json> --baseline-sha256 <baseline-digest> --output <external-tested.json>
```

Run the required checks and retain their exact commands, exits, executed counts,
raw proof references and applicability limitations in the result. Then validate
the fixed result and compare it with the actual source state:

```text
python .agents/skills/_shared/scripts/remediation_source_proof.py verify docs/snapshots/phase<N>/assurance/remediation.result.md --baseline <external-baseline.json> --baseline-sha256 <baseline-digest> --tested <external-tested.json> --tested-sha256 <tested-digest> --stage worktree
```

This compares actual task changes with attributed result paths, confirms tested
input is unchanged at verification observations, and preserves ordinary carried-in worktree,
index and untracked content. Two artifact groups are separate from the source
delta: the four fixed plan/result outputs, which may be generated or changed
under their contracts, and `frozen_artifacts`, the baseline's exact index plus
each indexed member's four files. Frozen artifacts retain their raw bytes and
modes and pass full set validation at every stage; they may be materialized with
the repair even when initially uncommitted. Capture does not authorize audit
changes or automatically include history or unindexed files. It covers tracked
files and non-ignored untracked files without locking concurrent editors.
Ignored inputs, external clients,
materials, platform/features and actual test execution still require their
declared evidence checks. Do not replace their identities with the receipt hash.
Existing symlinks are captured as link text without following their targets;
changed symlinks and unsupported modes such as submodules fail closed. Index and
commit proof also reject external Git clean filters instead of executing them.

On mismatch, preserve the evidence and resolve ownership or rerun invalidated
checks within the approved scope. A new tested receipt belongs to a new
verification execution, not a relabeling of an earlier pass. Never relax a
snapshot to make altered source appear tested.

Only after explicit user authorization to commit, stage task-owned paths, the
required fixed outputs and any uncommitted bound frozen audit artifacts, then
repeat `verify` with `--stage index`. After the
authorized commit, repeat with `--stage commit --commit <full-commit-SHA>`.
Index proof covers the entire index: unrelated staged paths, including
carried-in staged work, block it. The helper preserves that state and never
unstages files or supplies a partial-commit mode. Both stages require all four
fixed outputs and bound frozen audit artifacts to match their current bytes;
unchanged artifacts already committed need not be restaged. Commit proof requires
the current HEAD's sole parent to equal the baseline execution HEAD.
Record the commit and verified content association in the handoff; the helper
does not stage, commit or push. Keep any original dirty-package VCS metadata
unchanged even when its relevant source is proved equal to the final commit.

## Materialization handoff

Do not modify the plan retroactively and do not commit automatically. A fresh
audit or reviewer re-run may be requested only after fixes plus the current
live set are committed, the assurance directory is clean, external evidence is
resolved, and any owning-workflow return is complete. `opi-remediate` never
invokes another explicit-only skill itself and never grants a Phase PASS.

Include a ledger-owner handoff, or `none` with a concrete reason no maintenance
is indicated. Identify affected tasks as direct repair, linked revalidation or
retained; give the repair commit or exact uncommitted source receipt, applicable
evidence references, still-open obligations, and the proposed maintenance reason.
The owning workflow decides and performs any ledger update. Preserve original
delivery/start commits, `Opi-*` identities, source pins, DoD, dependencies and
prior evidence when proposing follow-up notes. If evidence challenges an existing
status, report that conflict for the owner rather than silently preserving or
promoting it. `opi-remediate` writes neither the ledger nor product specifications
and never invokes the owning skill automatically.

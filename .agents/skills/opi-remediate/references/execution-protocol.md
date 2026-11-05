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
6. verify `READY-FOR-APPLY`, no unresolved decision or remaining obligation
   for an indexed finding, complete strict-union coverage, and observed
   behavioral red-before.

Carry independent ledger maintenance to the
[materialization handoff](#materialization-handoff). Use the shared
[obligation rule](../../_shared/references/remediation-disposition-contract.md#finding-obligations-and-owner-handoff)
if a source-pin discrepancy affects the admitted closure evidence.

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
5. record exact paths, commands/results, and whether its assigned predicate
   passed.

Keep one result disposition per source finding. Its planned `closure_batches`
remain unchanged; mark the finding `Closed` only after every listed batch
predicate and their source-level conjunction are proved. Record those batch
outcomes and the conjunction in observed green-after evidence. A successful
batch alone cannot close a compound finding.

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

### CI checkpoints (only when required by the approved plan)

Use this branch when a bounded, explicitly authorized commit/push must precede
execution of a required CI check. Keep the original baseline for the entire
apply. Before local verification of each candidate, run `tested` as below.
For a later candidate, add `--checkpoint <previous-commit-receipt>` and
`--checkpoint-sha256 <previous-commit-receipt-digest>` to `tested`.

1. Finish the planned local checks for the candidate. Stage only currently
   implemented planned source changes. Keep the fixed outputs and frozen audit
   artifacts at their parent Git entries; the baseline still binds the exact
   plan and live audit bytes locally. Verify the entire index before committing:

   ```text
   python .agents/skills/_shared/scripts/remediation_source_proof.py checkpoint <external-baseline.json> --baseline-sha256 <baseline-digest> --tested <external-tested.json> --tested-sha256 <tested-digest> --stage index --output <external-candidate.json>
   ```

2. After the authorized commit, bind its exact tree and sole parent to the
   checked candidate before pushing it:

   ```text
   python .agents/skills/_shared/scripts/remediation_source_proof.py checkpoint <external-baseline.json> --baseline-sha256 <baseline-digest> --tested <external-tested.json> --tested-sha256 <tested-digest> --stage commit --candidate <external-candidate.json> --candidate-sha256 <candidate-digest> --commit <full-commit-SHA> --output <external-checkpoint.json>
   ```

3. Publish only the verified commit to the authorized target. Preserve the
   receipts and digests, CI checkout SHA, and original execution evidence.
   Verify the CI checkout is that exact commit. A source checkpoint receipt
   proves content and ancestry; it supplies no observed green or finding closure.

Checkpoint proof accepts a subset of planned repairs with the exact tested
source. Staged assurance artifacts and unplanned paths (including incidental
paths outside the plan) block the checkpoint. If CI needs any of those paths,
return to planning. The complete fixed outputs and frozen audit artifacts are
required for final delivery proof. No receipt grants
commit or push authorization or allows a merge, skipped commit or new baseline.
After a checkpoint, `tested` and final verification check each linked receipt,
parent and tree back to the original execution HEAD. A fresh `apply` admission
still allows only plan-pair commits after `Remediation head`; checkpoint receipts
continue an already admitted apply, they do not relax fresh admission.

### Final result and delivery

Immediately before the final verification union, capture its input. When CI
checkpoints exist, supply the last commit receipt and digest to `tested` as above:

```text
python .agents/skills/_shared/scripts/remediation_source_proof.py tested <external-baseline.json> --baseline-sha256 <baseline-digest> --output <external-tested.json>
```

Run the required checks and retain their exact commands, exits, executed counts,
raw proof references and applicability limitations in the result. Then validate
the fixed result and compare it with the actual source state:

```text
python .agents/skills/_shared/scripts/remediation_source_proof.py verify docs/snapshots/phase<N>/assurance/remediation.result.md --baseline <external-baseline.json> --baseline-sha256 <baseline-digest> --tested <external-tested.json> --tested-sha256 <tested-digest> --stage worktree
```

This compares cumulative task changes from the original baseline with attributed result paths, confirms tested
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

Only after explicit user authorization for final delivery, stage remaining task-owned paths, the
required fixed outputs and any uncommitted bound frozen audit artifacts, then
repeat `verify` with `--stage index`. After the
authorized commit, repeat with `--stage commit --commit <full-commit-SHA>`.
Index proof covers the entire index: unrelated staged paths, including
carried-in staged work, block it. The helper preserves that state and never
unstages files or supplies a partial-commit mode. Both stages require all four
fixed outputs and bound frozen audit artifacts to match their current bytes;
unchanged paths already in verified checkpoints need not be restaged. Commit
proof requires the current HEAD's sole parent to be the last verified checkpoint,
or the baseline execution HEAD when there are none. A result-only final commit
is valid when all repaired source is already in those checkpoints. A final
commit with changed build inputs requires its own applicable test evidence.
Record which exact checkpoint each CI run tested and prove applicability to
the final inputs; never relabel CI evidence with the final commit SHA.
Record the commit and verified content association in the handoff; the helper
does not stage, commit or push. Keep any original dirty-package VCS metadata
unchanged even when its relevant source is proved equal to the final commit.

## Materialization handoff

Do not modify the plan retroactively and do not commit automatically. Before
requesting a fresh audit or reviewer re-run, require committed fixes and a
committed current live set, a clean assurance directory, and resolution of
external evidence and any owner return required by the indexed findings or
the next audit's admission. `opi-remediate` never invokes another explicit-only
skill itself and never grants a Phase PASS.

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

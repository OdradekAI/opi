---
name: opi-remediate
disable-model-invocation: true
description: Verify immutable assurance findings, plan closure, or apply an approved remediation plan.
---

# Opi Remediate

Turn normalized audit/eval findings into independently reviewable closure
evidence. The workflow preserves source identity, distinguishes recurring
defects from new findings, and separates planning authority from execution.

## Invocation contract

Require `mode=plan | apply`.

For `mode=plan`, require `phase=<N>` and one or more `sources=<path,...>` unless
the Phase snapshot unambiguously identifies them. Optional `scope=<area>` may
order verification but cannot hide a source finding.

For `mode=apply`, require `plan=<immutable-plan-path>`. Do not infer approval
from the existence of a plan, a previous conversation, or blanket consent.

Outputs under `docs/snapshots/phase<N>/` are immutable:

- `remediation.<head7>.<round>.plan.md`
- `remediation.<head7>.<round>.plan.dispositions.jsonl`
- after application or a verified no-change outcome,
  `remediation.<head7>.<round>.result.md` and
  `remediation.<head7>.<round>.result.dispositions.jsonl`

`<head7>` is the first seven characters of the full remediation head. Derive
`<round>` as the next free `rN` among immutable artifacts for that Phase and
head, starting at `r1`; never fill a deleted or historical gap.

Never overwrite source findings, a prior plan, dispositions, or result.

## Shared contracts

Always read:

- `../_shared/references/finding-contract.md`
- `../_shared/references/remediation-disposition-contract.md`
- `../_shared/references/change-scope-and-check-selection.md`
- `references/cross-reference-matrix.md`

In `mode=plan`, also read `references/remediation-plan-template.md`. Read
`references/execution-protocol.md` only for `mode=apply`; this keeps execution
mechanics out of the planning context.

## `mode=plan`

### 1. Freeze the verification endpoint

Record the full committed remediation head and inventory committed, staged,
unstaged, and untracked paths. The shared change-scope reference selects the
verification union only; normalized findings and derived layers own remediation
scope. Verify committed evidence. Isolate overlapping dirty paths with
`git show <head>:<path>` or stop when that cannot be done safely.

Run all plan-stage checks from an isolated checkout of `remediation_head`.
Write only the immutable plan artifacts to the
original worktree. Apply mode operates in the original worktree only after its
separate exact-plan and dirty-baseline admission checks.

Reject mutable source names for new artifacts. Legacy narrative findings may
be ingested only as degraded input with their missing fields recorded.

### 2. Verify every source finding

Preserve the source `(source_path, id)`, severity, evidence, model identity, and
independence. Against current committed code and tests, assign `Confirmed`,
`Partially confirmed`, `Cannot confirm`, or `Refuted`. A final severity change
requires evidence and rationale; it never rewrites the source record.

Compare current records with prior immutable result dispositions:

```text
python .agents/skills/_shared/scripts/compare_finding_lineage.py --current <verified-findings.jsonl> --history <prior-result-dispositions.jsonl>...
```

Classify each as `new`, `recurrent-same-defect`,
`recurrent-adjacent-path`, `regression`, or `carried-forward-deferred`.
Recurrence is based on closure identity and prior disposition, not similar prose.

### 3. Derive closure batches and decisions

Give each actionable finding a falsifiable `closure_key` and broader
`family_key`. Cluster findings into one closure batch only when one root change
and one closure predicate prove all members closed. Similar paths, themes, or
recommendations are insufficient.

Record the exact decision for every confirmed or partially confirmed finding.
If the change requires new product meaning, authority, scope, or a contested
tradeoff, list the unresolved decision and stop at `DRAFT-UNRESOLVED`. Never
turn ambiguity into an implementation choice.

For each behavioral fix, define and observe a red-before check before any
production edit. If current behavior cannot make the check fail, revise the
claim or explain why the change is non-behavioral. Define the corresponding
green-after check and the verification union. When an unresolved product
decision prevents even defining the discriminating behavior, a draft may mark
both checks `pending:D<N>` with `observed: not-run`; that artifact cannot become
`READY-FOR-APPLY`. After the decision, create a new round and observe red-before.

### 4. Seal the plan and dispositions

Create the immutable plan and plan-stage dispositions together. Use
`READY-FOR-APPLY` only when every source has a disposition, every actionable
item has a decision and closure proof, all required red-before observations are
recorded, and `Unresolved decisions` is `none`.

Validate:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions <path>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan <path>
```

If all findings are refuted, informational, or already closed, emit the
plan-stage dispositions plus an immutable result and result-stage dispositions,
then stop; do not fabricate a fix item or request apply approval.

Present the user with the plan status, verified lineage summary, closure
batches, unresolved decisions, exact verification commands, and output paths.

## `mode=apply`

Follow `references/execution-protocol.md`.

Refuse execution unless the named plan is `READY-FOR-APPLY`, its validator
passes, the current committed HEAD matches its exact remediation head, no
unresolved decision remains, and the user explicitly approved that exact plan.

Apply one closure batch at a time. Preserve the dirty-worktree baseline, make
only planned changes, run the recorded green-after check and deduplicated
verification union, and stop on unexplained failure or endpoint drift. Write a
new immutable result and result-stage dispositions; do not mutate the plan or
its plan-stage dispositions.

After all batches pass, stop for the user's normal commit/materialization gate.
Once the fixes and immutable result are committed, request a fresh independent
`$opi-audit` invocation at that new committed endpoint. `opi-remediate` does not
silently invoke another explicit-only Opi skill. Remediation may report a batch
`Closed`; only that later audit may establish Phase conformance.

## Completion criterion

Planning completes when every source finding has a validated immutable
disposition and the outcome is either a valid `DRAFT-UNRESOLVED`, a valid
`READY-FOR-APPLY`, or a verified no-change result. Application completes only
when every approved closure batch passes its checks, the immutable result is
written, carried-in changes remain untouched, and the fresh audit requirement
is reported or fulfilled.

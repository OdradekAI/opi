---
name: opi-remediate
disable-model-invocation: true
description: Plan or apply finding closure against exact Opi audit-set and plan digests.
---

# Opi Remediate

Close the current live audit set with reviewable evidence. Planning and applying
are separate branches; apply requires the exact `audit.index.json` and approved
plan digests.

## Invocation and inputs

Require one of:

```text
$opi-remediate mode=plan phase=<N>
$opi-remediate mode=apply phase=<N> plan_sha256=<64 lowercase hex>
```

Validate the complete indexed active audit set before consuming semantic inputs
in either branch. Use `audit.index.json` and every indexed member's metadata,
requirements and findings sidecar. Findings are the strict union of indexed
`(audit_run_id, id)` keys; reports cannot substitute for machine records and no
peer is canonical.

Apply approval is the current explicit invocation plus the validator-emitted
`plan_sha256`. The shared disposition contract owns the fixed outputs:

```text
remediation.plan.dispositions.jsonl
remediation.result.dispositions.jsonl
```

It also owns paired Markdown paths, schemas, replacement and materialization.

## Common references

Read before branching:

- `../_shared/references/audit-set-contract.md`
- `../_shared/references/finding-contract.md`
- `../_shared/references/remediation-disposition-contract.md`

Bind the exact active index SHA-256 and each member's full `audit_run_id` and
`findings_sha256`. Keep history, prior conclusions, eval artifacts, lineage and
alternate source/plan paths outside context except for the shared contract's
validated indexed advisory appendix and its exact candidate raw proofs. That
exception supplies inspection leads, never finding or closure authority.

## `mode=plan`

### Plan-only references

Load these only for planning:

- `../_shared/references/change-scope-and-check-selection.md`
- `references/cross-reference-matrix.md`
- `references/remediation-plan-template.md`

The change-scope contract inventories normalized findings and derived layers
from the strict finding union and selects their verification union; it does not
expand ownership beyond this live audit set.

### 1. Admit the current set and baseline

Run `validate_assurance_artifact.py audit-set` against the assurance directory.
It recovers interrupted installations under lock and emits the exact index
digest for plan/result headers.

Record full committed `remediation_head` and the staged, unstaged, and untracked
path inventory. Use the shared disposition contract's source-baseline rule
when the user commits the fixed plan; do not reseal solely for plan-only commits.
Inspect committed code and run plan-stage checks in a unique
temporary `git archive` export; do not use a Git worktree. Refuse overlap with
uncommitted fixed remediation outputs. A clean tracked group may be replaced
without reading its earlier content.

### 2. Verify and dispose every current finding

Use the cross-reference matrix to verify every source key against current
committed evidence and assign its disposition, falsifiable closure key and
family key. Preserve source identities/evidence; textual ID duplicates remain
separate. Resolve disagreement by evidence and schedule at the highest source
severity.

After current-set validation, an indexed report's validated historical appendix
may guide regression checks and exact raw-proof lookup under the shared
contract. Reverify applicability at `remediation_head`; an old `Closed` or a
matching log hash cannot replace current evidence. Do not search history to
reconstruct missing appendices, and do not add historical disposition fields.
Reports without appendices remain fully consumable. A later appendix changes
the index binding and invalidates an already approved plan.

One closure batch requires one root change and one predicate proving every
member closed. New product meaning, public contract, authority, scope or
contested trade-off remains `DRAFT-UNRESOLVED` for the owning authority.

Behavioral fixes require discriminating red-before evidence before production
edits and a matching green-after check. Shared evidence may close identical
closure keys; every source key still needs exactly one disposition.

### 3. Seal and validate the fixed plan

Use the plan template. `READY-FOR-APPLY` requires exact strict-union coverage,
complete closure decisions/proofs, required red-before evidence, no unresolved
decision and exact active-index binding. Otherwise produce a validated
`DRAFT-UNRESOLVED`.

Run the dispositions validator, then:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase<N>/assurance/remediation.plan.md
```

Present the emitted digest and wait for a new apply invocation containing that
exact digest. When no code change is admissible, use the no-change result path
defined by the disposition contract.

## `mode=apply`

### Apply-only reference

Load `references/execution-protocol.md` only for apply. It owns admission,
closure batches, incidental repairs, result validation and materialization.
Proceed through the approved batches without renewed permission inside that
scope. Changed index/member bytes, plan digest, source drift after the remediation
head or overlapping unowned paths fail admission before production edits. The
shared disposition contract admits committed fixed plans without a HEAD-reseal loop.

## Completion criterion

Planning completes with a validated fixed `DRAFT-UNRESOLVED`, a validated fixed
`READY-FOR-APPLY` plus digest, or a validated no-change result. Apply completes
only when the execution protocol validates exact result coverage and evidence,
preserves carried-in work, and reports the materialization boundary without
declaring the Phase conformant or invoking another skill.

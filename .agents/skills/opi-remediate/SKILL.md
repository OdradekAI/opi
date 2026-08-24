---
name: opi-remediate
disable-model-invocation: true
description: Plan or apply closure for every finding in the current live indexed Opi audit set, bound to its exact active index digest.
---

# Opi Remediate

Turn the current live indexed audit set into independently reviewable closure
evidence. Plan and apply are separate branches bound to the same exact
`audit.index.json` digest, fixed plan, and approved plan digest.

## Invocation and inputs

Require one of:

```text
$opi-remediate mode=plan phase=<N>
$opi-remediate mode=apply phase=<N> plan_sha256=<64 lowercase hex>
```

Validate the complete indexed active audit set before either branch. Once
valid, consume `audit.index.json` plus every indexed member metadata,
requirements, and findings sidecar. Reports establish integrity but are not a
substitute for machine records. Findings are the strict union of all indexed
`(audit_run_id, id)` source keys; no peer is canonical.

Apply approval is the current explicit invocation plus the exact
validator-emitted `plan_sha256`. The shared remediation disposition contract
owns these fixed machine outputs:

```text
remediation.plan.dispositions.jsonl
remediation.result.dispositions.jsonl
```

Their paired Markdown paths, record schemas, replacement rules, and
materialization boundary remain in the shared contracts.

## Common references

Read before branching:

- `../_shared/references/audit-set-contract.md`
- `../_shared/references/finding-contract.md`
- `../_shared/references/remediation-disposition-contract.md`

Use the exact active index SHA-256 and every member's full `audit_run_id` and
`findings_sha256`. Keep history runs, prior task conclusions, eval artifacts,
recurrence/lineage data, and alternate source or plan paths outside the
context.

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

Run `validate_assurance_artifact.py audit-set` against the assurance directory
before consuming semantic inputs. The validator recovers interrupted
installations under the assurance lock, validates every indexed group, and
recomputes the exact `audit.index.json` digest used by plan/result headers.

Record full committed `remediation_head` and the staged, unstaged, and untracked
path inventory. Inspect committed code and run plan-stage checks in a unique
temporary `git archive` export; do not use a Git worktree. Refuse overlap with
uncommitted fixed remediation outputs. A clean tracked group may be replaced
without reading its earlier content.

### 2. Verify and dispose every current finding

Preserve every source identity and evidence, then assign `Confirmed`,
`Partially confirmed`, `Cannot confirm`, or `Refuted` from current committed
evidence. Give each finding one falsifiable closure key, family key, and exact
plan disposition. Duplicate textual IDs in different runs remain separate.
Resolve conflicting current claims by evidence, not reviewer vote; schedule at
the highest source severity without rewriting any source record.

Cluster findings into one closure batch only when one root change and one
closure predicate prove all members closed. Record decisions explicitly. A
change requiring new product meaning, public contract, authority, scope, or a
contested trade-off remains `DRAFT-UNRESOLVED` for the owning authority.

Each behavioral fix needs a discriminating red-before observation before
production edits and a matching green-after check. One fix and evidence pair
may close several source keys only when their closure key is identical, but
every current finding still appears in exactly one disposition.

### 3. Seal and validate the fixed plan

Write the plan and plan dispositions using the plan template. Use
`READY-FOR-APPLY` only with exact strict-union coverage, complete decisions and
closure proofs, all required red-before evidence, no unresolved decision, and
plan headers bound to the exact active index digest. Otherwise
publish a validated `DRAFT-UNRESOLVED`.

Run the dispositions validator, then:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase<N>/assurance/remediation.plan.md
```

Present the emitted digest and wait for a new apply invocation containing that
exact digest. When no code change is admissible, use the no-change result path
defined by the disposition contract.

## `mode=apply`

### Apply-only reference

Load `references/execution-protocol.md` only for apply and follow it in full.
It owns plan admission, dependency-ordered closure batches, incidental-repair
bounds, result validation, and the materialization handoff. Apply remains bound
to the exact index digest, approved plan digest, exact
remediation head, and non-overlapping path inventory. Any membership, member,
or index-byte change invalidates approval before production edits.

## Completion criterion

Planning completes with a validated fixed `DRAFT-UNRESOLVED`, a validated fixed
`READY-FOR-APPLY` plus digest, or a validated no-change result. Apply completes
only when the execution protocol validates exact result coverage and evidence,
preserves carried-in work, and reports the materialization boundary without
declaring the Phase conformant or invoking another skill.

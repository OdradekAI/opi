---
name: opi-remediate
disable-model-invocation: true
description: Verify and remediate only the current Opi active audit set, binding plan/apply to exact run and content digests. Use only when the user explicitly invokes opi-remediate.
---

# Opi Remediate

Turn the current active audit findings into independently reviewable closure
evidence. The workflow has no historical ingestion or lineage pass: one audit
run, one findings digest, one fixed plan, and one fixed result.

## Invocation contract

Require one of:

```text
$opi-remediate mode=plan phase=<N>
$opi-remediate mode=apply phase=<N> plan_sha256=<64 lowercase hex>
```

The source is always:

```text
docs/snapshots/phase<N>/assurance/audit.meta.json
docs/snapshots/phase<N>/assurance/audit.findings.jsonl
```

Do not accept `sources=`, a historical default, an eval artifact, a round, or
an alternate plan path. Apply approval is the current explicit invocation plus
the exact validator-emitted plan digest; blanket or earlier approval is not
sufficient.

The only outputs are:

```text
docs/snapshots/phase<N>/assurance/remediation.plan.md
docs/snapshots/phase<N>/assurance/remediation.plan.dispositions.jsonl
docs/snapshots/phase<N>/assurance/remediation.result.md
docs/snapshots/phase<N>/assurance/remediation.result.dispositions.jsonl
```

Git history preserves superseded sets after materialization. Do not create
timestamped siblings or a separate remediation ledger.

## Shared contracts

Always read:

- `../_shared/references/audit-set-contract.md`
- `../_shared/references/finding-contract.md`
- `../_shared/references/remediation-disposition-contract.md`
- `../_shared/references/change-scope-and-check-selection.md`
- `references/cross-reference-matrix.md`

Use the change-scope contract only to inventory normalized findings and derived layers
and to select the verification union; it does not expand remediation
ownership beyond the current active audit set.

In `mode=plan`, also read `references/remediation-plan-template.md`. Read
`references/execution-protocol.md` only in `mode=apply`.

## History boundary

Read the current four audit files only. Never read, search, compare, or import:

- earlier audit/remediation files or their Git history;
- prior fixed plan/result content from another run;
- eval findings as remediation sources;
- conclusions from a previous task or scratch ledger.

`closure_key` and `family_key` group findings inside the current set only.
Historical `lineage`, `prior_occurrences`, recurrence, regression, and consensus
classification are forbidden.

## `mode=plan`

### 1. Admit exactly one current audit set

Run `audit-set` against the fixed directory. Read `audit_run_id` and
`findings_sha256` from validated `audit.meta.json`; recompute the findings hash
and refuse on mismatch. These values identify every plan disposition source.

Record the full committed `remediation_head` and staged, unstaged, and untracked
path inventory. Inspect committed code and run plan-stage checks in a unique
temporary `git archive` export of `remediation_head`; do not use a Git worktree.
The live worktree receives only the fixed plan outputs.

If prior fixed remediation files have uncommitted path state, refuse rather
than overwrite them. A tracked clean prior remediation group may be replaced
without reading its content because Git preserves it.

### 2. Verify every current finding

Preserve the source run ID, findings digest, finding ID, evidence, model
identity, and reported severity. Against current committed code/tests, assign
`Confirmed`, `Partially confirmed`, `Cannot confirm`, or `Refuted`. Any final
severity change requires current evidence and rationale.

Give every finding a falsifiable `closure_key` and `family_key`. Resolve
conflicting current claims through evidence, not voting. Every current finding
must appear exactly once in a fix or explicit exclusion; no older or extra
source identity is admissible.

### 3. Derive closure batches and decisions

Cluster findings only when one root change and one closure predicate prove all
members closed. Shared files, family keys, wording, or severity are not enough.

Record the exact decision for every confirmed or partially confirmed finding.
When a change needs new product meaning, public contract, authority, scope, or
a contested trade-off, record the decision and stop at `DRAFT-UNRESOLVED`.
Never infer product authority from an audit finding.

For each behavioral fix, observe a discriminating red-before before production
edits and define its green-after. If the behavior cannot be made red, revise or
refute the claim. A draft may use `pending:D<N>` only while the named unresolved
decision prevents a concrete check; it cannot be ready for apply.

### 4. Seal and validate the fixed plan

Write the fixed plan and plan dispositions together. Use
`READY-FOR-APPLY` only when every current finding has one disposition, every
actionable item has a decision and closure proof, all required red-before
observations exist, and unresolved decisions are `none`.

Run:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase<N>/assurance/remediation.plan.dispositions.jsonl
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase<N>/assurance/remediation.plan.md
```

The second command emits `plan_sha256`. Present status, current finding
coverage, closure batches, unresolved decisions, exact checks, fixed paths, and
the digest. Do not apply until the user invokes apply with that digest.

If all findings are refuted, Info/no-action, deferred by a registered current
source, or returned to shaping, write and validate the fixed no-change result;
do not fabricate a fix.

## `mode=apply`

Follow `references/execution-protocol.md`.

Validate the fixed plan and compare its emitted digest to the invocation
exactly. Refuse unless status is `READY-FOR-APPLY`, current committed HEAD is
the exact remediation head, unresolved decisions are none, and dirty changes do
not overlap owned paths.

Apply one closure batch at a time. Make planned changes only, except for the
bounded verification-blocking incidental repair defined in the shared
disposition contract. Record every actual changed path in a planned or
incidental result disposition; narrative-only corrections are invalid.

Validate the fixed result with `validate_assurance_artifact.py result`.
Remediation may report a batch `Closed`, but cannot
declare the Phase conformant.

After success, stop at materialization. A later audit is allowed only after the
fixes and complete active assurance set are committed together, the assurance
directory is clean, required external evidence is resolved, and any owning
workflow return is complete. Never commit or invoke `opi-audit` automatically.

## Completion criterion

Planning completes with a validated fixed `DRAFT-UNRESOLVED`, validated fixed
`READY-FOR-APPLY` plus digest, or validated no-change result. Apply completes
only when every planned batch and accepted incidental repair has red/green
evidence, result coverage is exact, carried-in work is untouched, and the
materialization requirement is explicit.

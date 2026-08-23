# Fixed Remediation Plan Template

Output `remediation.plan.md` and
`remediation.plan.dispositions.jsonl` under the current Phase `assurance/`
directory. The JSONL contract is
`../../_shared/references/remediation-disposition-contract.md`.

```markdown
# Phase <N> Remediation Plan

**Status**: DRAFT-UNRESOLVED | READY-FOR-APPLY
**Audit run ID**: `<exact audit.meta.json value>`
**Findings SHA-256**: `<exact audit.meta.json value>`
**Remediation head**: `<full committed SHA>`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: <staged/unstaged/untracked inventory>
**Unresolved decisions**: none | D<N>, ...

## Current Finding Verification

| Finding ID | Verification | Final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| ... | ... | ... | ... | ... | ... |

## Unresolved Decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| ... | ... | ... | ... | ... |

Use `none` when ready. A draft may stop before fix design when a decision
determines the closure predicate; do not add placeholder fixes.

## Closure Batches

### Batch B1: <one behavioral closure>

**Closure predicate**: <one falsifiable outcome>
**Dependencies**: <earlier batches or none>
**Verification union**: <deduplicated checks>

#### Fix B1.1: <short title>

- **Finding source**: <current audit_run_id + findings_sha256 + finding ID>
- **Decision**: <exact decision>
- **Verification status**: Confirmed | Partially confirmed
- **File(s)**: `<paths>`
- **Change kind**: behavioral | test-only | documentation | metadata
- **Change**: <minimum bounded change>
- **Closure predicate**: <observable outcome>
- **Red-before**: <command and observed failure, or concrete N/A reason>
- **Green-after**: <same discriminating check and expected pass>

## Final Verification

    <deduplicated verification union>

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| ... | Refuted / Cannot confirm / Info-no-action / Returned to shaping / Deferred by registered source | ... |
```

Every finding ID must appear in a fix or exclusion and in exactly one plan
disposition. After validation, present the emitted `plan_sha256`; never write
that digest into the plan itself because it hashes the exact plan bytes.

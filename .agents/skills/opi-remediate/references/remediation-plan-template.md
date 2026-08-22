# Remediation plan template

Output a new `remediation.<head7>.<round>.plan.md` and its sibling dispositions.
Never reuse `remediation-plan.md` for a new run.

The header contract is `Status: DRAFT-UNRESOLVED | READY-FOR-APPLY`.

```markdown
# Phase <N> Remediation Plan

**Status**: DRAFT-UNRESOLVED | READY-FOR-APPLY
**Verification target**: committed `<full SHA>`
**Round**: <immutable round identity>
**Finding sources**: <immutable source paths>
**Disposition artifact**: `remediation.<head7>.<round>.plan.dispositions.jsonl`
**Dirty-worktree baseline**: <staged/unstaged/untracked inventory>
**Unresolved decisions**: none | D<N>, ...

## Lineage and verification summary

| Source finding | Verification | Final severity + rationale | Lineage | Closure batch | Decision |
|---|---|---|---|---|---|
| ... | ... | ... | ... | ... | ... |

## Unresolved decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| ... | ... | ... | ... | ... |

Use `none` when the plan is `READY-FOR-APPLY`.
A `DRAFT-UNRESOLVED` may stop here before fix design when the missing decision
determines the closure predicate. Do not add placeholder fix items to satisfy a
template.

## Closure batches

### Batch B1: <one behavioral closure>

**Closure predicate**: <one falsifiable outcome shared by every batch member>
**Dependencies**: <earlier batches or none>
**Verification union**: <deduplicated checks>

#### Fix B1.1: <short title>

- **Finding source**: <immutable source_path + id>
- **Lineage**: <kind + prior evidence>
- **Decision**: D<N>
- **Verification status**: Confirmed | Partially confirmed
- **File(s)**: `<path>`
- **Change kind**: behavioral | test-only | documentation | metadata
- **Change**: <minimum bounded change>
- **Closure predicate**: <observable outcome>
- **Red-before**: <command and observed failure, or concrete N/A reason>
- **Green-after**: <same discriminating check and expected pass>

## Final verification

    <deduplicated verification union>

## Exclusions

| Source finding | Disposition | Evidence/authority |
|---|---|---|
| ... | Refuted / Cannot confirm / Info-no-action / Returned to shaping / Deferred by registered source | ... |
```

Layer batches by the live workspace dependency graph when dependencies require
it, but do not merge independent closure predicates merely to share a layer.
Every source finding must appear either in a fix item or exclusions.

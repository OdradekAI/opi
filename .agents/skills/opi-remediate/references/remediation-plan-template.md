# Fixed Remediation Plan Template

Output `remediation.plan.md` and
`remediation.plan.dispositions.jsonl` under the current Phase `assurance/`
directory. The JSONL contract is
`../../_shared/references/remediation-disposition-contract.md`.

```markdown
# Phase <N> Remediation Plan

**Status**: DRAFT-UNRESOLVED | READY-FOR-APPLY
**Audit index SHA-256**: `<exact raw-byte digest>`
**Remediation head**: `<full committed SHA>`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: <staged/unstaged/untracked inventory>
**Unresolved decisions**: none | D<N>, ...

## Current Finding Verification

| Source run / Finding ID | Verification and counter-evidence | Source and final severity + rationale | Closure key/family | Closure batches | Decision / remaining obligations |
|---|---|---|---|---|---|
| ... | ... | ... | ... | ... | ... |

If an indexed report has a validated historical appendix, cite useful leads in
the current evidence rationale only after independently checking their raw
proofs and applicability at `remediation_head`. Do not add old source rows or
historical disposition fields. An absent appendix requires no reconstruction.

## Unresolved Decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| ... | ... | ... | ... | ... |

Use `none` when ready. A draft may stop before fix design when a decision
determines the closure predicate; do not add placeholder fixes.

## Planning Evidence Work

| Question | Source/build inputs | Positive / negative controls | Distinguishing observation | Dependent decision |
|---|---|---|---|---|
| ... | ... | ... | ... | ... |

Use `none` when no additional probe is needed. Complete this work before sealing
an executable plan. Carry any residual proof or authority gap into the source
disposition's `remaining_obligations`; any such gap keeps the whole plan draft.

## Closure Batches

### Batch B1: <one behavioral closure>

**Root change**: <one bounded root>
**Closure predicate**: <one falsifiable outcome>
**Dependencies**: []
**Verification union**: <deduplicated checks>
**Contract rehearsal**: <record below, or concrete reason the batch preserves its contracts>

#### Fix B1.1: <short title>

- **Finding source(s)**: <audit_run_id> / <owning findings_sha256> / <finding ID>
- **Assigned portion**: <whole finding, or this batch's obligation for each compound finding>
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

For each compound finding, identify its ordered `closure_batches` and the check
that proves the source closure predicate as their conjunction.

## Exclusions

| Source run / Finding ID | Disposition | Refuted claim / retained observation | Remaining obligation and authority |
|---|---|---|---|
| ... | Refuted / Cannot confirm / Info-no-action / Returned to shaping / Deferred by registered source | ... | ... |
```

Use a JSON array such as `["B1"]` for dependent batches and declare batches in
topological order. Their IDs must match the union of disposition
`closure_batches` exactly; each disposition's array follows that order. For
timing-sensitive behavior, the closure predicate identifies event
order, exact counts/budgets and forbidden late actions under the matrix's proof
rules. A no-change plan has no batch or Fix headings; use the shared contract's
no-change result branch without inventing a modification.

Every source key in the indexed strict union appears in exactly one plan
disposition. A fix appears in every batch named by its non-empty
`closure_batches`; an exclusion has `closure_batches: []`. A compound finding
retains its single source identity and requires every assigned batch predicate
plus their conjunction for closure. Similar IDs never merge. After
validation, present the emitted `plan_sha256`; never write that digest into the
plan itself because it hashes the exact plan bytes. Any active-index byte change
invalidates the plan binding and its approval, including a later advisory
`append-history` installation. Complete available historical enrichment before
planning; do not exempt report-only changes from the index binding.

`Remediation head` records the source evidence baseline. If the user commits
the fixed plan pair afterward, retain that baseline and use the shared
disposition contract's apply admission check; plan-only commits need no reseal.

## Contract rehearsal

Rehearse changes to public APIs, accepted inputs, observable errors or behavior,
wire or durable formats, and other contracts relied on by callers. Keep all
trial source and fixture edits in the isolated export. For each affected batch:

1. Trace the changed contract through production and downstream test callers,
   including reverse dependencies, fixtures and package consumers as applicable.
   Select checks for the affected seams and record the inspected scope.
2. Run the selected tests against the baseline and the minimum contract change
   with existing downstream expectations. Bind the committed source and the
   trial diff. Record exact commands, exits, executed tests and counts, and raw
   proof references.
   Separate expected contract migrations from unexpected failures.
3. Rehearse each necessary caller or fixture migration. Preserve the assertion's
   intent, then rerun the affected checks on the candidate with those migrations.
   Keep the plan draft for unexplained failures or required checks without proof.
4. Compare the export delta and the required path inventory with the batch's
   `File(s)` and its assigned dispositions' `changed_paths`. For a compound
   disposition, check the union of its assigned batches. Include every required
   migration in the approved checkpoint path set when CI needs it. Recheck the
   affected trial and path inventory after a change to the proposed contract.

Record the commands, observations and paths under `Contract rehearsal` or link
their exact receipts there. Treat trial passes as planning evidence. The actual
apply still owes its own green-after, verification union and source proof.

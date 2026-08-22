# Remediation execution protocol

Use only for `mode=apply` after explicit approval of one immutable plan.

## Admission

Before editing:

1. validate the plan and dispositions;
2. verify the current committed HEAD is the exact remediation head named by the
   plan; a prefix match is insufficient for execution;
3. reproduce and record the dirty-worktree baseline, stopping on overlapping
   unowned changes;
4. verify `Status` is `READY-FOR-APPLY`, unresolved decisions are `none`, and
   approval names this exact plan path;
5. rerun each behavioral batch's red-before check when environment drift could
   change the observation.

Capture approval provenance for the result: exact plan path, approval time, and
the user message/task context that granted it. If exact-plan approval cannot be
identified, refuse before editing.

Do not revise the approved decision while applying. New evidence that changes
scope or meaning returns the work to `mode=plan` in a new immutable round.

## Closure-batch loop

For each dependency-ordered closure batch:

1. confirm its closure predicate and owned file set;
2. apply only the planned minimum change;
3. run the batch green-after check;
4. run the missing portion of its verification union;
5. record changed paths, command output, resulting SHA/worktree state, and
   `Closed` or `Not closed` in the result.

Proceed only after the prior batch passes. On any unexplained failure, endpoint
drift, or unowned overlap, stop and preserve evidence. Do not impose an
arbitrary retry count, roll back user work, broaden scope, or call the batch
closed.

Select focused crate tests for local behavior, conformance/fixture checks for
protocol or durable formats, documentation checks for docs/skills/metadata, and
workspace gates only for cross-crate semantics. Deduplicate the union; do not
rerun an unchanged passing gate without a stated reason.

## Result and assurance handoff

Write `remediation.<head7>.<round>.result.md` and
`remediation.<head7>.<round>.result.dispositions.jsonl` as new immutable
artifacts. They must name the approved plan, exact starting endpoint,
approval provenance, dirty-worktree baseline, every closure batch outcome,
changed paths, exact commands/results, test impact, remaining exclusions, and
resulting endpoint.

Do not edit the plan or retroactively change its dispositions. If actual
evidence differs, write the new fact in the result and, when needed, a new
disposition artifact in a later round.

After successful application, stop for the user's ordinary commit/materialize
gate; this workflow never commits automatically. Only after the fixes and
result artifacts exist at a new committed endpoint, request a fresh independent audit
through an explicit new `$opi-audit` invocation. The result may say
remediation checks passed; it must not grant itself a Phase `PASS`.

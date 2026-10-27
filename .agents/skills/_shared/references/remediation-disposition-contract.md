# Remediation Disposition Contract

`opi-remediate` consumes `audit.index.json` and every indexed member of the
current live audit set. Findings are a strict union keyed by
`(audit_run_id, id)`. It records one decision for every source key. Historical
records and earlier remediation conclusions are not source or closure authority.
The only advisory input exception is a validated indexed report appendix and its
exact raw proofs under the
[audit-set contract](audit-set-contract.md#post-audit-historical-reference).

The fixed files are:

```text
remediation.plan.md
remediation.plan.dispositions.jsonl
remediation.result.md
remediation.result.dispositions.jsonl
```

## Current finding disposition

Plan and result stages use this record shape:

```json
{
  "record_kind": "finding-disposition",
  "source": {
    "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z",
    "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "id": "P17-AUD-001"
  },
  "verified_at": "236c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "verification_status": "Confirmed",
  "remaining_obligations": [],
  "final_severity": "Major",
  "final_severity_rationale": "The mandatory durable binding is absent.",
  "closure_key": "session.runtime-binding",
  "family_key": "session.durability",
  "decision": "fix:persist-runtime-binding",
  "closure_batch": "B1",
  "change_kind": "behavioral",
  "changed_paths": ["crates/opi-agent/src/session.rs"],
  "red_before": {
    "command": "cargo test -p opi-agent --test session_contract binding",
    "expected": "FAIL because the binding is absent",
    "observed": "FAIL: binding was None"
  },
  "green_after": {
    "command": "cargo test -p opi-agent --test session_contract binding",
    "expected": "PASS",
    "observed": "not-run"
  }
}
```

`source.audit_run_id` identifies one indexed member and
`source.findings_sha256` matches that owning member metadata exactly.
`(audit_run_id, id)` identifies the source finding. Every indexed finding
appears exactly once in a fix or explicit exclusion; extra or older source
identities are invalid. Findings with the same textual ID or repair remain
separate source records and require separate dispositions, although they may
share a closure batch and verification evidence.

Reviewer disagreement is not a vote. Among dependency-ready batches, the highest
source severity controls scheduling priority while every source severity and rationale remains
unchanged. A final severity change or `Refuted` disposition requires independent
current evidence; another peer's contrary conclusion is insufficient.

`closure_key` names one falsifiable behavior. `family_key` clusters adjacent
findings only inside the current set. Historical `lineage`,
`prior_occurrences`, and `prior_disposition` fields are forbidden.
An appendix may suggest verification work, but cannot assign a disposition,
severity, closure batch or new source key. Independently verify any candidate raw
proof and its applicability at `remediation_head`; record the resulting current
evidence through the existing fields. Reports without appendices remain valid,
and consumers do not reconstruct missing historical comparisons.

`verification_status` is `Confirmed`, `Partially confirmed`, `Cannot confirm`,
or `Refuted`. `change_kind` is `behavioral`, `test-only`, `documentation`, or
`metadata`. Behavioral changes require observed red-before. Non-behavioral
changes require either `red_before` or a concrete
`red_before_not_applicable` reason.

`verified_at` equals the plan's `Remediation head`. It identifies the evidence
baseline; result records preserve it rather than relabeling an earlier execution
with a later repair commit. Current applicability and the final source state
require their own evidence under the execution protocol.

`remaining_obligations` is a required array of non-empty strings, or `[]` when
this disposition has no known residual obligation. Record missing proof and
unresolved source applicability or precedence here, with the affected criterion
and the evidence or authority needed. Refuting a finding's claim does not mark
its linked requirements `met` or remove their proof obligations. Any non-empty
array keeps the entire plan `DRAFT-UNRESOLVED`; exclusions do not bypass this
gate. New evidence that changes an approved disposition requires a new plan.

Use `fix:<decision>` only for `Confirmed` or `Partially confirmed` findings,
with a non-null closure batch and non-empty changed paths. An exclusion uses
`no-action:<reason>`, `closure_batch: null`, `changed_paths: []`, and
`change_kind: metadata`: that metadata is the disposition, not a product edit.
Its result status is `Refuted`, `Cannot confirm`, `Info/No action`, `Returned to
shaping`, or `Deferred by registered source`, never `Closed` or `Not closed`.
If the result uses `Refuted` or `Cannot confirm`, it must equal the plan's
`verification_status`; these evidence conclusions cannot be interchanged.
The other three statuses record an administrative disposition without replacing
the verification conclusion. Keep the original source record unchanged.
In the decision and rationale, identify the exact
claim refuted, strongest counter-evidence, surviving observations, and the
resulting limits on permissible changes. A draft may use `pending:<question>`
while that decision is still being investigated.

At plan stage, `green_after.observed` is `not-run`, and no
`remediation_status` is allowed. At result stage, green-after is observed and
`remediation_status` is `Closed`, `Not closed`, `Deferred by registered
source`, `Returned to shaping`, `Info/No action`, `Refuted`, or `Cannot
confirm`.

Older plans missing required fields must be regenerated and validated before
use; the resulting bytes have a new plan digest and require new apply approval.
Adding fields to an approved plan does not preserve its approval.

## Evidence work and closure dependencies

Resolve evidence questions during planning before sealing a repair decision.
For each probe, record its question, source/build inputs, positive and negative
controls, distinguishing observation, and dependent decision. Static proof and
external local probes need no fictitious production edit. Record actual commands
and observations; a compiler or fixture-setup failure is not a behavioral red.
Evidence work cannot become an apply-time conditional choice of product meaning.

Each planned closure batch has one `### Batch <id>: <title>` heading and one
`**Dependencies**` JSON array of earlier batch IDs (`[]` for none). These headings
and the non-null disposition `closure_batch` values must name exactly the same
batches. IDs and dependency entries are unique; unknown dependencies, self-edges,
cycles and a declaration before its prerequisites are invalid. Execute in that
topological order and require each prerequisite to pass before continuing.
This does not admit a subset of an unresolved plan.

## Plan approval identity

`remediation.plan.md` records the exact `Audit index SHA-256` of the current
live index, `Remediation head`, the fixed disposition filename, status, and
unresolved decisions. The index digest is the complete approval binding: it
covers membership, revision, every member head, and every member digest, so
any installed, replaced, or edited member invalidates the plan. The validator
computes:

```text
sha256(remediation.plan.md exact bytes + NUL + remediation.plan.dispositions.jsonl exact bytes)
```

It prints `plan: PASS plan_sha256=<digest>`. Apply requires explicit user
approval of the fixed plan path and that digest. Any plan byte or active-index
byte change invalidates the approval. Appending advisory history changes the
member report digest and index even though its independent core is unchanged;
an already approved plan therefore becomes stale. Do not waive this binding for
an appendix or retroactively alter the approved plan.

`Remediation head` is the committed source evidence baseline, not the commit
that stores the finished plan. Apply requires it to be a committed ancestor
of current HEAD. Every intervening commit, including each merge-parent diff,
may touch only this Phase's `remediation.plan.md` and
`remediation.plan.dispositions.jsonl`. These plan-only commits do not require
resealing an unchanged plan or new approval. Changed plan bytes still require
their new exact digest; changes to any other path require a new plan, even if
a later commit restores the original bytes. Other Phases' plans, result files,
audit artifacts, specifications, ledgers and source are outside this exception.

Run the locked admission check before production edits:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py apply docs/snapshots/phase<N>/assurance/remediation.plan.md --plan-sha256 <approved-digest>
```

It validates the fixed plan, live set, ready status, exact approved digest and
committed source boundary. The execution protocol still owns carried-in path
inventory and overlapping unowned-change admission. Planning and applying do
not grant commit authorization; commit only on an explicit user request.

## No-change result

The planning branch may finish without a repair or apply invocation when every
current finding has an explicit `no-action:` disposition. Keep the fixed plan
`DRAFT-UNRESOLVED`; it is not an executable plan, including when all claims have
been refuted and `Unresolved decisions` is `none`. Validate the plan and bind its
emitted digest in the fixed result. Use:

```markdown
**Status**: COMPLETE
**Outcome**: NO-CHANGE
**Audit index SHA-256**: `<exact active index digest>`
**Plan SHA-256**: `<validated draft digest>`
**Changed paths**: []
```

There are no closure batches, Fix items, changed source paths or incidental
records. Preserve exact strict-union coverage, source identities and all residual
obligations. Copy the plan dispositions, filling each `green_after.observed` with
the actual verification observation and the matching no-action result status.
For static verification, name the actual inspection command and its observation;
use a concrete `red_before_not_applicable` reason instead of a fabricated test.

`COMPLETE` describes the completed disposition report. It grants no apply
permission, requirement closure or Phase verdict. Missing evidence and authority
decisions remain explicit, including when the original implementation-defect
claim is refuted. Compare final Git state with the recorded planning inventory:
only this Phase's fixed plan/result outputs may be newly changed; preserve all
carried-in work. If any repair was made, this branch is unavailable.

The ordinary result uses `**Outcome**: APPLIED` and requires the approved
`READY-FOR-APPLY` plan. A draft with some fix decisions cannot produce either an
applied result or a no-change result; retain the validated draft and its gaps.

## Bounded incidental repair

An apply run may add an incidental result record only when a recorded
verification command exposes a new defect that blocks the approved batch:

```json
{
  "record_kind": "incidental-repair",
  "id": "I1",
  "trigger_batch": "B1",
  "blocking_check": "cargo test --workspace --all-targets",
  "scope_rationale": "The collision blocks the approved B1 workspace gate.",
  "guardrails": {
    "required_for_green": true,
    "within_causal_surface": true,
    "changes_public_api": false,
    "changes_durable_format": false,
    "changes_dependency_graph": false,
    "changes_spec_or_authority": false
  },
  "changed_paths": ["crates/opi-agent/src/worker.rs"],
  "red_before": {
    "command": "cargo test --workspace --all-targets",
    "expected": "FAIL",
    "observed": "FAIL: helper name collision"
  },
  "green_after": {
    "command": "cargo test --workspace --all-targets",
    "expected": "PASS",
    "observed": "PASS"
  },
  "remediation_status": "Closed"
}
```

All bounded incidental repair guardrails are literal: `required_for_green` and
`within_causal_surface` are `true`, while all protected `changes_*` fields are
`false`. An incidental repair cannot change a public API, durable format,
dependency graph, registered specification, authority boundary, Cargo
manifest/lockfile, implementation ledger, or public schema. It needs its own
observed FAIL/PASS. If any guardrail fails, stop and create a new plan.
Non-blocking observations are reported but not fixed.

`remediation.result.md` records `Outcome`, the active `Audit index SHA-256`,
`Plan SHA-256`, and a JSON array in `Changed paths`. The array must equal the
union of paths attributed by planned and accepted incidental result records.
Narrative-only corrections fail validation. Result dispositions cover exactly
the plan source-key set. The `Changed paths` value is machine-checked, not
narrative metadata. This agreement between records is not a comparison with Git.
For an applied result, the
[execution protocol](../../opi-remediate/references/execution-protocol.md#source-and-materialization-proof)
also requires actual-delta and tested-input proof before completion.

Validate with:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase<N>/assurance/remediation.plan.dispositions.jsonl
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase<N>/assurance/remediation.plan.md
python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase<N>/assurance/remediation.result.md
```

A later audit or re-run is admitted only after fixes and the current live set
are materialized by being committed and the assurance directory is clean.

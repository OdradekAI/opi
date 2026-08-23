# Remediation Disposition Contract

`opi-remediate` consumes only the current active audit set. It records one
decision for every current finding without consulting earlier audit or
remediation artifacts.

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
    "audit_run_id": "phase17-136c380-20260824t010203z",
    "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    "id": "P17-AUD-001"
  },
  "verified_at": "236c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "verification_status": "Confirmed",
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

`source.audit_run_id` and `source.findings_sha256` match `audit.meta.json`
exactly. `(audit_run_id, id)` identifies the source finding. Every current
finding appears exactly once in a fix or explicit exclusion; extra or older
source identities are invalid.

`closure_key` names one falsifiable behavior. `family_key` clusters adjacent
findings only inside the current set. Historical `lineage`,
`prior_occurrences`, and `prior_disposition` fields are forbidden.

`verification_status` is `Confirmed`, `Partially confirmed`, `Cannot confirm`,
or `Refuted`. `change_kind` is `behavioral`, `test-only`, `documentation`, or
`metadata`. Behavioral changes require observed red-before. Non-behavioral
changes require either `red_before` or a concrete
`red_before_not_applicable` reason.

At plan stage, `green_after.observed` is `not-run`, and no
`remediation_status` is allowed. At result stage, green-after is observed and
`remediation_status` is `Closed`, `Not closed`, `Deferred by registered
source`, `Returned to shaping`, `Info/No action`, `Refuted`, or `Cannot
confirm`.

## Plan approval identity

`remediation.plan.md` records `Audit run ID`, `Findings SHA-256`, `Remediation
head`, the fixed disposition filename, status, and unresolved decisions. The
validator computes:

```text
sha256(remediation.plan.md exact bytes + NUL + remediation.plan.dispositions.jsonl exact bytes)
```

It prints `plan: PASS plan_sha256=<digest>`. Apply requires explicit user
approval of the fixed plan path and that digest. Any byte change invalidates
the approval.

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

All bounded incidental repair guardrails are literal. An incidental repair cannot change a public API,
durable format, dependency graph, registered specification, authority boundary,
Cargo manifest/lockfile, implementation ledger, or public schema. It needs its
own observed FAIL/PASS. If any guardrail fails, stop and create a new plan.
Non-blocking observations are reported but not fixed.

`remediation.result.md` records the exact `Audit run ID`, `Findings SHA-256`,
approved `Plan SHA-256`, and a JSON array in `Changed paths`. The array must
equal the union of paths attributed by planned and accepted incidental result
records. Narrative-only corrections fail validation.

Validate with:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase<N>/assurance/remediation.plan.dispositions.jsonl
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase<N>/assurance/remediation.plan.md
python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase<N>/assurance/remediation.result.md
```

A later audit is admitted only after fixes and the complete active set are
materialized by being committed together and the assurance directory is clean.

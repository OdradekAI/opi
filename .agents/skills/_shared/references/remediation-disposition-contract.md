# Remediation disposition contract

`opi-remediate` records its verification, lineage, decision, and closure proof
without changing the source audit or eval finding. Records are JSON Lines in
two immutable stages:

- `remediation.<head7+>.<round>.plan.dispositions.jsonl` records verification,
  lineage, decisions, observed red-before, and the planned green-after;
- `remediation.<head7+>.<round>.result.dispositions.jsonl` copies the plan
  disposition identity and adds observed green-after plus final remediation
  status. It never overwrites the plan-stage record.

```json
{
  "source": {"source_path": "docs/snapshots/phase17/audit.codex.136c380.run1.md", "id": "P17-CODEX-SPEC-001"},
  "verified_at": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "verification_status": "Confirmed",
  "final_severity": "Major",
  "final_severity_rationale": "The mandatory durable binding is absent.",
  "closure_key": "session.runtime-binding",
  "family_key": "session.durability",
  "lineage": {
    "kind": "carried-forward-deferred",
    "prior_occurrences": [],
    "prior_disposition": "Deferred by registered source"
  },
  "decision": "D6",
  "closure_batch": "B2",
  "change_kind": "behavioral",
  "red_before": {"command": "cargo test ...", "expected": "FAIL", "observed": "FAIL"},
  "green_after": {"command": "cargo test ...", "expected": "PASS", "observed": "not-run"}
}
```

## Field rules

- `source` is the immutable `(source_path, id)` from the producer artifact.
- `verified_at` is the full committed HEAD used for remediation verification.
- `verification_status` is `Confirmed`, `Partially confirmed`,
  `Cannot confirm`, or `Refuted`.
- `closure_key` names one falsifiable behavior that one root fix can close.
- `family_key` connects adjacent paths without merging them into one fix.
- `lineage.kind` is `new`, `recurrent-same-defect`,
  `recurrent-adjacent-path`, `regression`, or `carried-forward-deferred`.
- `decision` refers to the immutable remediation plan's decision record.
- `closure_batch` groups only fixes that share one closure predicate and can be
  reviewed independently.
- `change_kind` is `behavioral`, `test-only`, `documentation`, or `metadata`.
- Behavioral fixes require `red_before` and `green_after` records. The plan
  stage observes red-before and records the expected green-after; the result
  stage records the observed green-after. Other change kinds require either
  `red_before` or
  `red_before_not_applicable` with a concrete reason.
- A plan-stage record uses `green_after.observed: "not-run"` and cannot claim a
  `remediation_status`.
- A `DRAFT-UNRESOLVED` record still assigns a defect-level `closure_key` and
  `family_key`; these describe the falsifiable claim, not the chosen solution.
  Use `decision: "pending:D<N>"`, `closure_batch: null`, and a `green_after`
  whose command/expected fields name the pending decision when that decision
  prevents a concrete check. If the decision also prevents a discriminating
  red check, use the same pending form for `red_before` with
  `observed: "not-run"`; a later decided round must replace the placeholder with
  observed red-before before it can be ready. For a verified no-action
  disposition, keep the claim keys, use `closure_batch: null`, and record
  concrete red/green N/A reasons rather than omitting fields.
- A result-stage record requires observed green-after and a
  `remediation_status`: `Closed`, `Not closed`, `Deferred by registered source`,
  `Returned to shaping`, `Info/No action`, `Refuted`, or `Cannot confirm`.
  A remediate run may say `Closed`; only a later independent audit can declare
  the Phase requirement satisfied.

Validate records with:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions <path>
```

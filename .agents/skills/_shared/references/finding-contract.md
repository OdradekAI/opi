# Normalized Finding Contract

Audit and runtime-eval findings share severity, evidence, and confidence
vocabulary. Only findings in the current audit set are admissible to
`opi-remediate`; eval findings remain evaluation evidence and do not become an
implicit remediation source.

## Current audit findings

Write each peer's audit findings as JSON Lines to
`docs/snapshots/phase<N>/assurance/audit.<reviewer-id>.<model-id>.findings.jsonl`.
The human report is the same stem with the `.md` suffix. A finding uses this
shape:

```json
{
  "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z",
  "id": "P17-AUD-001",
  "source_kind": "audit",
  "source_path": "docs/snapshots/phase17/assurance/audit.codex.gpt56.md",
  "source_model": "gpt-5.6",
  "observed_at": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "independence": "fresh-context-same-family",
  "axis": "spec",
  "severity": "Major",
  "conformance_effect": "blocks",
  "title": "Durable session binding is absent",
  "claim": "New sessions do not persist the required runtime binding.",
  "evidence": [
    {
      "location": "crates/opi-agent/src/session.rs:42",
      "detail": "The serialized header has no runtime binding."
    }
  ],
  "requirement_ids": ["P17-A1"],
  "criterion_source": "docs/opi-spec.md#INV-007",
  "reproduction": ["cargo test -p opi-agent --test session_contract"],
  "confidence": "high",
  "status": "unverified"
}
```

Audit finding identity is `(audit_run_id, id)`. `source_path` is traceability,
not identity. IDs need only be unique inside one audit run; the same textual ID
in two indexed runs remains two findings in the remediation strict union.

## Field rules

- `audit_run_id` matches the owning suffixed member metadata exactly.
- `source_kind` is `audit`; `source_path` names that member's suffixed report.
- `source_model` equals `reviewer_model_id` from the owning metadata. Never
  infer or invent it.
- `observed_at` is the full committed `audit_head` inspected by the run.
- `independence` is `independent-family`, `fresh-context-same-family`, or
  `unknown`. A new context on the same model family is not independent-family.
- `axis` is `standards`, `spec`, `security`, `test-quality`, `invariants`,
  `integration`, `residuals`, or `runtime-fidelity`.
- `conformance_effect` is `blocks` or `advisory`. Blocker and Major findings
  always block. Advisory findings may be Minor or Info only.
- `requirement_ids` is non-empty and reciprocal with the linked requirement
  records. A blocking finding cannot link a requirement whose state is `met`.
- `claim` is falsifiable; `evidence` cites observations; `reproduction` names a
  runnable check or a concrete case.
- `criterion_source` is `null` only when no normative criterion applies.
- `status` is `unverified` at publication. Remediation records its own
  verification separately.

Validate the current audit sidecar with:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py findings docs/snapshots/phase<N>/assurance/audit.codex.gpt56.findings.jsonl
```

A complete audit must validate the whole set with the `audit-set` command; the
standalone command is diagnostic only.

## Runtime eval findings

`opi-eval` may reuse the axis, severity, evidence, reproduction, confidence,
and independence fields. Its producer-owned case/run identity remains outside
the active audit set. A later independent audit may reproduce eval evidence,
but remediation never ingests an eval artifact or silently converts it into an
audit finding.

## Severity scale

| Tier | Meaning |
|---|---|
| **Blocker** | Cannot ship safely: normal-path data loss, credential/user-data exposure, crash or panic on expected input, or a common-path deadlock/infinite loop. |
| **Major** | Incorrect behavior or a significant gap that must be fixed before the next phase: wrong output for valid input, silent edge-case corruption, material spec deviation, cascading error handling failure, or a critical-path test gap. |
| **Minor** | Quality or completeness gap without incorrect behavior: a non-critical test gap, documentation drift, duplicate logic, naming inconsistency, or an unsynchronized localized counterpart. |
| **Info** | Improvement or future consideration rather than a defect: performance opportunity, API ergonomics, scale observation, or a documented design trade-off. |

Normalize foreign labels while retaining the original label in narrative
evidence: P0/Critical to Blocker, P1/High to Major, P2/Medium/Warning to Minor,
and P3/Low/Note to Info. Map unfamiliar labels by described impact rather than
spelling.

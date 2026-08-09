# Normalized Finding Contract

Audit and runtime eval artifacts use this interchange when their findings may
enter `opi-remediate`. Narrative reports remain human-readable; each actionable
finding also carries these fields.

```yaml
id: <source-stable identifier>
source_kind: audit | eval
source_path: <repo-relative artifact path>
source_model: <reported reviewer/evaluator identity>
independence: independent-family | fresh-context-same-family | unknown
axis: standards | spec | security | test-quality | invariants | integration | residuals | runtime-fidelity
severity: Blocker | Major | Minor | Info
title: <concise title>
claim: <falsifiable problem statement>
evidence:
  - location: <file:line, trace event, artifact, or command>
    detail: <observed evidence>
criterion_source: <spec/rule citation or null>
reproduction:
  - <command or eval case>
confidence: high | medium | low
status: unverified
```

## Field rules

- `id` is stable within `source_path`. Consumers identify the source finding by
  `(source_path, id)`; they do not assume IDs are globally unique.
- `source_kind` distinguishes static/code review evidence from runtime fidelity
  evidence. It does not imply either source is more trustworthy.
- `source_model` reports the identity claimed by the producer. Never invent a
  model ID.
- `independence` reports the actual relationship to the implementation or model
  under evaluation. A fresh context on the same family is degraded independence,
  not `independent-family`.
- `axis` preserves Standards and Spec as separate Matt review axes. Opi-specific
  audit dimensions and eval use the remaining values.
- `severity` uses the four-tier scale below. Foreign labels are normalized at
  ingestion while preserving the original label in the narrative report.
- `claim` must be independently checkable. Recommendations belong outside the
  claim.
- `evidence` cites observed facts, not reviewer confidence or conclusions.
- `criterion_source` is `null` only when no normative criterion applies.
- `reproduction` may name an eval case when a direct command is unavailable.
- `status` is always `unverified` at production time.

## Remediation ownership

`opi-remediate` preserves every source field unchanged and records its own
verification status separately as `Confirmed`, `Partially confirmed`, `Cannot
confirm`, or `Refuted`. Consensus clustering may select a candidate severity,
but it never silently reranks an individual source finding. Any final severity
change is recorded with code/trace evidence and rationale.

Malformed finding blocks remain visible in the source report but are not
silently repaired. Remediation reports the missing fields and asks for a source
correction or treats the narrative as an explicitly degraded legacy input.

## Severity scale

| Tier | Meaning |
|---|---|
| **Blocker** | Cannot ship safely: normal-path data loss, credential/user-data exposure, crash or panic on expected input, or a common-path deadlock/infinite loop. |
| **Major** | Incorrect behavior or a significant gap that must be fixed before the next phase: wrong output for valid input, silent edge-case corruption, material spec deviation, cascading error handling failure, or a critical-path test gap. |
| **Minor** | Quality or completeness gap without incorrect behavior: a non-critical test gap, documentation drift, duplicate logic, naming inconsistency, or an unsynchronized localized counterpart. |
| **Info** | Improvement or future consideration rather than a defect: performance opportunity, API ergonomics, scale observation, or documented design trade-off. |

Normalize foreign labels as follows while retaining the producer's original
label in narrative evidence:

| Canonical | Common foreign labels |
|---|---|
| Blocker | P0, Critical |
| Major | P1, High |
| Minor | P2, Medium, Warning |
| Info | P3, Low, Note |

When a label is unfamiliar, map by described impact rather than spelling:
security/data-loss/crash to Blocker; wrong behavior/spec deviation to Major;
quality/test/doc gap to Minor; suggestion/style/future work to Info. A healthy
review usually has few Blockers; do not inflate severity to create urgency.

# Audit framework: layers, drift, evidence

## Layer rule

The report has two layers, kept strictly separate.

- **Layer A — objective.** Current state of each project, cited, and the raw
  differences. This is the body. No classification, no priorities, no roadmap
  phases, no "matrix says".
- **Layer B — judgment.** Drift classification and recommendations. This is an
  appendix, produced only when asked. It never frames Layer A.

Why the split: a measured difference is a fact about today's code; a
classification is an opinion about intent. Mixing them ages the report badly
(the classification goes stale as code changes) and confuses "where are we" with
"what should we do". Measure facts; offer opinions separately, on request.

## Drift taxonomy (Layer B)

| Level | Meaning |
|---|---|
| Aligned | Current behavior matches target semantics or accepted design intent. |
| Intentional divergence | Difference is justified by language, runtime, product scope, or an explicit non-goal. |
| Partial | A seam or subset exists but does not yet cover target semantics. |
| Missing | Target capability exists and is relevant; the current project lacks it. |
| Overreach | The current project adds target-adjacent scope that is not justified. |
| Risk | The implementation is in the wrong layer or could block future alignment. |

Distinguish "not implemented yet" (Partial) from "implemented in the wrong
layer" (Risk).

## Verification outcomes

Every delta carries one:

- `confirmed` — re-checked against source on both sides; solid.
- `refuted` — the claimed gap or parity does not hold; drop or footnote.
- `refined` — directionally right but overstated or imprecise; correct it.
- `added` — a real difference the measurer missed; fold in.

## Evidence discipline

- Cite `file:line` for every claim, or state `absent: searched <paths>`. Silence
  is not absence.
- Mark inference separately from documented evidence.
- Examples, demos, and package samples are not core product behavior.
- Check the changelog before claiming a capability is current.
- Separate ecosystem parity (breadth the target has) from core semantic
  alignment (whether the shared behavior is correct).

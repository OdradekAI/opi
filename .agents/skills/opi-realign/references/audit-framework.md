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

This five-status vocabulary is a non-normative Layer B classification owned by
this audit framework. It must not be cited as product authority or written back
into `docs/opi-spec.md` without human shaping:

| Status | Meaning |
|---|---|
| `Full` | opi preserves the user/integrator-visible target semantics, even if the Rust implementation differs. |
| `Partial` | opi implements the core idea, but breadth/edges/commands/providers/ecosystem are narrower than the target. |
| `Intentional Divergence` | opi deliberately chooses a different Rust-native module/interface/format/adapter strategy. |
| `Missing` | The target has the capability and opi does not, but it may still belong on the roadmap. |
| `Out of Scope` | The target has the capability, but opi explicitly does not plan to carry it in core. |

Two optional **judgment sub-flags** annotate a primary status when useful — they
are not separate statuses:

| Sub-flag | Annotates | Meaning |
|---|---|---|
| `overreach` | any status | The current project adds target-adjacent scope that is not justified. |
| `risk` | `Partial` / `Missing` | The implementation is in the wrong layer or could block future alignment. |

Distinguish "not implemented yet" (`Partial`) from "implemented in the wrong
layer" (`risk` on `Partial`).

## Verification outcomes

Every delta carries one:

- `confirmed` — re-checked against source on both sides; solid.
- `refuted` — the claimed gap or parity does not hold; drop or footnote.
- `refined` — directionally right but overstated or imprecise; correct it.
- `added` — a real difference the measurer missed; fold in.

## Evidence discipline

- Record the exact opi and pi commit SHAs and verify pi's remote identity.
  "Latest" is a request to resolve a revision, not a stable evidence label.
- Cite `file:line` for every claim, or state `absent: searched <paths>`. Silence
  is not absence.
- Mark inference separately from documented evidence.
- Examples, demos, and package samples are not core product behavior.
- Check the changelog before claiming a capability is current.
- Separate ecosystem parity (breadth the target has) from core semantic
  alignment (whether the shared behavior is correct).
- Keep outward proposals out of the delta ledger. Capabilities not grounded in
  pi are inputs to `opi-research`, not inward alignment findings.

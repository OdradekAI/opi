# Audit framework: layers, drift, evidence

## Layer rule

The report has three layers, kept strictly separate.

- **Layer A — objective.** Current state of each project, cited, and the raw
  differences. This is the body. No classification, no priorities, no roadmap
  phases, no "matrix says".
- **Layer H — target design horizon.** Active design intent found inside the
  pinned pi tree, its stated authority/status, and its observed implementation
  maturity. This follows Layer A and is evidence, not an implementation claim,
  Opi requirement, or priority.
- **Layer B — judgment.** Drift classification and recommendations. This is an
  appendix, produced only when asked. It never frames Layer A or Layer H.

Why the split: current implementation, documented target intent, and Opi
adoption judgment answer different questions. Mixing them makes designed-only
behavior look shipped or turns upstream intent into an Opi commitment. Measure
current facts in Layer A, preserve target direction in Layer H, and offer Opi
judgment separately on request.

## Target-design evidence (Layer H)

Record two independent fields for each design item:

- **Design authority/status** quotes or paraphrases the artifact's own boundary,
  such as normative, explanatory target, explanatory proposal, explicitly
  informative, open question, or explicit non-goal. These labels may be
  combined when the source does so.
- **Observed maturity** compares that item with current pi implementation
  evidence. Assign one status:

| Status | Meaning |
|---|---|
| `implemented` | Current pi source and focused tests directly realize the cited target contract. |
| `partial` | Current pi source realizes a meaningful subset while cited target behavior remains absent or incomplete. |
| `scaffold` | The public shape exists, but primary operation paths are explicitly unimplemented or placeholder-only. |
| `designed-only` | The cited target or proposal has no implementation in the stated search scope. Its authority is recorded separately. |
| `not-assessed` | No implementation comparison is meaningful for this row, normally because it records an open question, informative sketch, or non-goal. |

Treat authority as evidence, not inference. Cite the artifact's own normative,
informative, draft, or open wording. A repository location or document title
alone does not make every sentence normative. Generic TODOs, issue chatter,
superseded proposals, and external roadmaps are not Layer H unless the pinned
tree explicitly adopts them as active design evidence.

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

Every retained Layer A delta and Layer H item carries one:

- `confirmed` — re-checked against all evidence applicable to the item's
  layer; Layer A requires current source on both sides, while a Layer H
  `not-assessed` item requires its cited design authority/status and boundary.
- `refuted` — the claimed gap or parity does not hold; drop or footnote.
- `refined` — directionally right but overstated or imprecise; correct it.
- `added` — a real difference the measurer missed; fold in.

## Evidence discipline

- Record the exact opi and pi commit SHAs and verify pi's remote identity.
  "Latest" is a request to resolve a revision, not a stable evidence label.
- Cite `file:line` for every claim, or state `absent: searched <paths>`. Silence
  is not absence.
- Mark inference separately from documented evidence.
- For Layer H, cite the design claim and its authority/status. Maturity labels
  other than `not-assessed` also require current source, test, changelog, or
  explicit absence-search evidence. A `not-assessed` row may use `N/A` for
  current evidence and states that no Opi parity conclusion follows.
- Examples, demos, and package samples are not core product behavior.
- Check the changelog before claiming a capability is current.
- Separate ecosystem parity (breadth the target has) from core semantic
  alignment (whether the shared behavior is correct).
- Keep outward proposals out of the delta ledger. Capabilities not grounded in
  pi are inputs to `opi-research`, not inward alignment findings.

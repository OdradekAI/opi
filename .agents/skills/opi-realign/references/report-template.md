# Realign report template

Write to `docs/realign/YYYY-MM-DD-opi-vs-pi-<revision>.md`, using a short pi
commit prefix or immutable tag in the filename.

Layer A records current objective differences. Layer H separately records
active target-design evidence found in the pinned pi tree. Layer B (judgment)
is an appendix produced only when requested. See `audit-framework.md` for the
taxonomies.

## Header

- Title: `opi vs pi <revision> - objective differences`.
- Measurement date.
- opi root, version, and exact commit SHA.
- pi root, version when declared, exact commit SHA, and verified remote URL.
- Scope and dimension count.
- Method: both sides read from source; every retained delta adversarially
  checked; any non-independent verification limitation stated.
- Framing: `Layer A has no phases, baselines, or roadmap framing; Layer H
  separately records pinned target-design evidence and maturity.`

Never leave `latest`, a moving branch name, or only a semantic version as the
target identity.

## Body: Layer A

Start with a short `How to read` block: project columns are cited facts;
`Difference` is factual; absence names searched paths; Layer H is target intent,
not shipped behavior or an Opi requirement; judgment appears only in the
appendix.

For each dimension in `dimensions.md` order:

1. `**opi:**` one or two sentences, followed by key facts with one `file:line`
   anchor per fact.
2. `**pi:**` the same shape.
3. A table with `Item | opi | pi | Difference | Verification outcome`.

Allowed outcomes are `confirmed`, `refuted`, `refined`, and `added`. Refuted
items are normally omitted from the main table and retained as method notes.

### Layer A cleanliness gate

Search the objective body for roadmap language such as
`Phase [0-9]`, `roadmap`, `matrix says`, `should`, `needs to`, and `planned`.
Remove judgment leaks. A source symbol actually named `Phase` is allowed when
clearly cited as code.

## Layer H: target design horizon

Include this section when the pinned pi tree contains active design material
that describes in-scope behavior beyond current implementation. For a full
audit, state the searched design paths even when no qualifying artifact exists.

Start with the target document's title, location, and self-declared authority.
For each affected dimension, use:

`Item | pi target design | Design authority/status | pi current evidence | Observed maturity | opi current evidence | Objective horizon difference | Verification outcome`

Use only the maturity statuses and verification outcomes in
`audit-framework.md`. Cite design authority/status from the artifact itself.
Every maturity label except `not-assessed` needs current source, test,
changelog, or explicit absence-search evidence. A `not-assessed` row may use
`N/A` for current evidence and must state that no Opi parity conclusion follows.
Layer H may describe build order or future direction because it is isolated
from Layer A; it must not recommend adoption or assign Opi priority.

## Appendix A: drift classification (optional)

Only when requested, add:

`Dimension | Item | Classification | Evidence note`

Use only the primary statuses and optional sub-flags in `audit-framework.md`.
Do not add priorities unless the user separately asked for prioritization.

## Appendix B: method and verification

Record the Layer A dimension count and outcome tally, the Layer H maturity and
outcome tallies, refuted-item notes, unavailable evidence, and every dimension
or horizon item that lacked an independent verifier.

## Chat summary

Report the highest-signal current deltas, grouped as `pi ahead` and `opi ahead`,
then summarize the target-design horizon with maturity labels. State the exact
pi revision, disclose any single-pass dimensions or horizon items, link the
report, and route outward opportunities to `opi-research`.

# Realign report template

Write to `docs/realign/YYYY-MM-DD-opi-vs-pi-<revision>.md`, using a short pi
commit prefix or immutable tag in the filename.

The body is Layer A (objective). Layer B (judgment) is an appendix produced
only when requested. See `audit-framework.md` for the taxonomy.

## Header

- Title: `opi vs pi <revision> - objective differences`.
- Measurement date.
- opi root, version, and exact commit SHA.
- pi root, version when declared, exact commit SHA, and verified remote URL.
- Scope and dimension count.
- Method: both sides read from source; every retained delta adversarially
  checked; any non-independent verification limitation stated.
- Framing: `No phases, baselines, or roadmap framing.`

Never leave `latest`, a moving branch name, or only a semantic version as the
target identity.

## Body: Layer A

Start with a short `How to read` block: project columns are cited facts;
`Difference` is factual; absence names searched paths; judgment appears only in
the appendix.

For each dimension in `dimensions.md` order:

1. `**opi:**` one or two sentences, followed by key facts with one `file:line`
   anchor per fact.
2. `**pi:**` the same shape.
3. A table with `Item | opi | pi | Difference | Verification outcome`.

Allowed outcomes are `confirmed`, `refuted`, `refined`, and `added`. Refuted
items are normally omitted from the main table and retained as method notes.

### Body cleanliness gate

Search the objective body for roadmap language such as
`Phase [0-9]`, `roadmap`, `matrix says`, `should`, `needs to`, and `planned`.
Remove judgment leaks. A source symbol actually named `Phase` is allowed when
clearly cited as code.

## Appendix A: drift classification (optional)

Only when requested, add:

`Dimension | Item | Classification | Evidence note`

Use only the primary statuses and optional sub-flags in `audit-framework.md`.
Do not add priorities unless the user separately asked for prioritization.

## Appendix B: method and verification

Record the dimension count, outcome tally, refuted-delta notes, unavailable
evidence, and every dimension that lacked an independent verifier.

## Chat summary

Report the highest-signal inward deltas, grouped as `pi ahead` and `opi ahead`,
without phase/roadmap framing. State the exact pi revision, disclose any
single-pass dimensions, link the report, and route outward opportunities to
`opi-research`.

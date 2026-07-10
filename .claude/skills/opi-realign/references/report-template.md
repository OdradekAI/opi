# Report template

File path: `docs/realign/YYYY-MM-DD-<current>-vs-<target>.md`
(for example `docs/realign/2026-07-10-opi-vs-pi-0.80.2.md`).

The body is Layer A (objective). Layer B (judgment) is an appendix, produced
only when asked. See `audit-framework.md` for the layer rule and taxonomy.

## Body (Layer A)

Header:

- Title: `<current> vs <target> — Objective Differences`.
- One line: fresh measurement, date, current project + version vs target +
  version, read from source with file:line anchors. State "No phases, no
  baseline, no roadmap framing."
- Source roots for both projects (path, language, crates/packages).
- Method line: number of dimensions measured; that each delta was adversarially
  verified (refute-on-gap); note any dimension re-measured from source.

A short "How to read" block: columns are factual current state with citations;
"Difference" is one line, factual, no judgment; absence is stated with where
searched; drift classification lives in the appendix.

Then one section per dimension, in the `dimensions.md` order. For each:

- `**<current>:**` 1–3 sentence summary, then `Key facts:` as a bulleted list,
  one `file:line` anchor per fact.
- `**<target>:**` same shape.
- A difference table with columns `Item | <current> | <target> | Difference`.

Body-cleanliness gate: after writing, grep the body for
`Phase [0-9]|roadmap|matrix says|should|needs to|planned` and remove any hit
that crept in. A `Phase` type/enum name (e.g. the harness `Phase` enum) is fine;
a numbered roadmap phase is not. The body reports state, not intent.

## Appendix A — Drift classification (Layer B, only if asked)

A table with columns `Dimension | Item | Classification | Note`, classification
drawn from the `audit-framework.md` taxonomy.

## Appendix B — Method & verification

State the dimension count and the outcome tally (see `audit-framework.md`).
List any refuted deltas as footnotes. Flag any dimension that was
single-sourced (re-measured from source rather than adversarially verified) and
why.

## Chat summary

Lead with nothing about phases or the prior audit. Give the highest-signal
deltas, one line each and dimension-prefixed, grouped as "where the target is
ahead" and "where the current project is ahead". Note any single-sourced
dimension. Point at the report file. Mention spot-checks only if you performed
them.

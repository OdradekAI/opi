---
name: opi-realign
description: Fresh, objective delta audit of this project against a target/reference project.
disable-model-invocation: true
---

# Opi Realign

A realign audit is **fresh**: it measures current source on both projects and
produces a **delta ledger** of objective, cited differences. *Fresh* means
today's code is the only input that counts — a completed phase, a prior audit,
or any baseline doc are irrelevant to the measurement; quote them only for
`file:line` anchors or recorded non-goals. Judgment (drift classification,
priorities) is a separate appendix, never the frame.

## Inputs

- `target=<path>` — required. The reference/upstream project to compare against.
- `current=<path>` — optional, defaults to cwd.
- `scope=<text>` — optional. A named slice (dimensions, packages, or surfaces)
  instead of the full audit.
- Labels optional.

Treat `@path`, quoted, Windows, and POSIX paths as valid.

## Process

1. **Scope.** Confirm current and target paths and the scope (full or a named
   slice). State assumptions that affect the outcome.
   *Done when:* both paths resolve and the scope is stated.

2. **Measure fresh, both sides, per dimension.** For every dimension in
   `references/dimensions.md` (or the chosen slice): read current source on BOTH
   projects and record each side's state with a `file:line` anchor. State absence
   explicitly (`absent: searched <paths>`), never by silence.
   *Done when:* every in-scope dimension has a cited current-state entry for both projects.

3. **Write the deltas.** For each dimension, write the objective differences —
   current state | target state | raw difference, one line each, factual, no
   judgment language.
   *Done when:* every consequential difference is a delta, and no delta contains phase/roadmap/plan/should language.

4. **Verify each delta adversarially.** For every delta claiming the current
   project *lacks* a capability, hunt the current source for it before accepting
   (refute-on-gap); for every "has", confirm it is real and not overstated. Give
   each delta an outcome (defined in `references/audit-framework.md`). Drop or
   footnote refuted deltas; fold in refined and added ones.
   *Done when:* every delta carries an outcome. See `references/audit-framework.md`.

5. **Render the ledger (Layer A).** Write the report under `docs/realign/`
   (filename pattern and template in `references/report-template.md`). The body
   is pure objective state: dimension sections, per-project facts with anchors, a
   difference table.
   *Done when:* the body passes the cleanliness gate defined in
   `references/report-template.md` (no numbered-phase / roadmap / baseline /
   plan language leaks in).

6. **Judgment appendix (Layer B) — only if asked.** Add drift classification
   and/or recommendations as a clearly separated appendix, so it never frames
   the body. Keep recommendations as proposals for the user to action.
   *Done when:* Layer B sits in an appendix, not interleaved into Layer A.

For a full audit, fan out one measurer + one verifier per dimension (see
`references/dimensions.md`); the verifier red-teams the measurer's gaps.

Summarize the highest-signal deltas in chat and point at the report file.

## Guardrails

- Stay fresh: never frame the audit relative to a prior audit, baseline,
  phase, or roadmap.
- Keep judgment (classification, priorities) out of the objective body.
- Cite `file:line` for every claim, or state absence with the search performed.
- Do not claim API, config, package, or file-format compatibility unless
  evidence proves it.
- Target breadth is not automatically desirable; prefer strengthening existing
  seams.
- Do not recommend copying target-language architecture when it conflicts with
  current-language ownership, dependency, concurrency, packaging, or testing
  norms. See `references/language-porting.md`.
- Reports under `docs/realign/` are generated, non-normative artifacts. Do not
  edit source, `opi-spec.md`, READMEs, or roadmaps; state findings and let the
  user action them.
- Do not commit unless asked.

---
name: opi-realign
description: Audit opi against pinned pi implementation and target design, preserving Rust-native semantic alignment.
disable-model-invocation: true
---

# Opi Realign

Realign produces a cited **inward alignment** report against an exact
`earendil-works/pi` revision: current implementation differences (Layer A),
pinned target-design evidence (Layer H), and judgment only when requested
(Layer B).

A fresh audit uses only artifacts present in the pinned trees. Prior audits,
implementation phases, external roadmaps, and baseline documents do not define
the result. Active implementation specifications and architecture or design
documents inside the pinned pi tree may define Layer H; keep their target intent
separate from current implementation evidence.

## Inputs

- `target=<path>`: required local pi checkout.
- `target-revision=<commit-or-tag>`: required. Resolve and record its exact
  commit SHA.
- `current=<path>`: optional; defaults to the current opi checkout.
- `scope=<text>`: optional named dimensions/packages/surfaces; otherwise full.

Accept quoted, `@path`, Windows, and POSIX paths.

If the user requests "latest", fetch the configured pi upstream read-only,
resolve the default-branch tip, and record that SHA. Verify the remote identity.
If this would replace a revision the user explicitly named, stop for
confirmation. Never use `latest` as an evidence label in the report.

## Process

Read [`references/audit-framework.md`](references/audit-framework.md) for layer,
evidence, maturity, and verification rules. For full audits, use the bounded
measurement/verifier batches in
[`references/dimensions.md#execution`](references/dimensions.md#execution).

1. **Pin and scope.** Resolve both roots, both commit SHAs, pi's remote identity,
   and the exact scope. State material assumptions.
   *Done when:* paths, revisions, remote, and scope are explicit.

2. **Measure current state from source.** For every in-scope dimension in
   [`references/dimensions.md`](references/dimensions.md), record current opi and
   pi state with `file:line` anchors or `absent: searched <paths>`.
   *Done when:* every dimension has cited evidence for both projects.

3. **Map the target-design horizon.** Inspect in-scope implementation
   specifications, architecture/design documents, RFCs, and explicitly
   forward-looking sections in the pinned pi tree. Record each document's
   stated authority/status, then test its design claims against current pi
   source, tests, and changelog. Include explicit non-goals, informative futures,
   and open questions without promoting them to contracts.
   *Done when:* every consequential candidate is represented with design and
   implementation anchors or is explicitly ruled out as inactive evidence.

4. **Write objective current deltas.** Record opi state, pi state, and the raw
   difference for each consequential item, without judgment or roadmap framing.
   *Done when:* each difference is factual and independently understandable.

5. **Verify adversarially.** A verifier independent from the measurer hunts opi
   source for every claimed gap, checks every parity claim for overstatement,
   and verifies each Layer H authority/status and maturity label against the
   pinned design artifact plus current pi implementation evidence. Assign an
   outcome; drop or footnote refuted items and correct refined ones.
   *Done when:* every retained Layer A and Layer H item has an outcome.

6. **Render Layers A and H.** Write the objective report under `docs/realign/`
   using [`references/report-template.md`](references/report-template.md).
   Complete when every in-scope dimension and consequential horizon item is
   accounted for, limitations are explicit, and the Layer A cleanliness gate
   passes.

7. **Add Layer B only when asked.** Drift classification and recommendations
   belong in a separate appendix. They never frame or interleave the objective
   body. Classification is not prioritization; priorities are optional and
   require an explicit request.

## Inward boundary

- A pi target-design document is inward evidence, not an Opi requirement or
  implementation priority. Route adoption decisions to human-led shaping.
- A pi capability is not automatically an opi core task. Preserve the design
  idea, then prefer plugin/package placement unless a missing core seam is
  evidenced.
- A capability pi lacks or implements poorly for opi's goals belongs to
  `opi-research`. Do not smuggle outward ecosystem exploration into realign.
- Target breadth is not automatically desirable. Distinguish semantic
  alignment from ecosystem breadth.
- Do not recommend copying target-language architecture when it conflicts with
  Rust-native ownership, dependencies, concurrency, packaging, or tests. Apply
  `references/language-porting.md`.

## Guardrails

- Do not claim compatibility without direct evidence.
- Reports are generated, non-normative artifacts. Do not edit product source,
  `docs/opi-spec.md`, READMEs, or roadmaps while running this skill.
- Do not commit unless asked.

Use the report template's chat-summary contract to hand off current deltas,
target-design horizons, and verification limitations with a report link.

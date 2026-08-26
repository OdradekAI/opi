---
name: opi-realign
description: Audit opi inward against an exact earendil-works/pi revision, covering current implementation and authority-scoped target-design horizons while preserving pi design lineage through Rust-native implementation choices.
disable-model-invocation: true
---

# Opi Realign

Realign is the **inward alignment** workflow. It measures opi against an exact
revision of `earendil-works/pi` and produces a cited delta ledger. It asks
whether opi still preserves pi's design ideas and visible semantics while
expressing them through Rust ownership, dependency, concurrency, packaging,
and testing norms.

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

1. **Pin and scope.** Resolve both roots, both commit SHAs, pi's remote identity,
   and the exact scope. State material assumptions.
   *Done when:* paths, revisions, remote, and scope are explicit.

2. **Measure current state from source.** For every in-scope dimension in
   `references/dimensions.md`, record current opi and pi state with `file:line`
   anchors. Express absence as `absent: searched <paths>`, never as silence.
   *Done when:* every dimension has cited evidence for both projects.

3. **Map the target-design horizon.** Inspect in-scope implementation
   specifications, architecture/design documents, RFCs, and explicitly
   forward-looking sections in the pinned pi tree. Record each document's
   stated authority/status, then test its design claims against current pi
   source, tests, and changelog using the Layer H maturity vocabulary in
   `references/audit-framework.md`. Include explicit non-goals, informative
   futures, and open questions without promoting them to contracts.
   *Done when:* every consequential candidate is represented with design and
   implementation anchors or is explicitly ruled out as inactive evidence.

4. **Write objective current deltas.** For each consequential item, record opi state,
   pi state, and the raw difference. Exclude judgment, phases, plans, and
   recommendations.
   *Done when:* each difference is factual and independently understandable.

5. **Verify adversarially.** A verifier independent from the measurer hunts opi
   source for every claimed gap, checks every parity claim for overstatement,
   and verifies each Layer H authority/status and maturity label against the
   pinned design artifact plus current pi implementation evidence. Assign the outcome
   from `references/audit-framework.md`; drop or footnote refuted items and
   correct refined ones.
   *Done when:* every retained Layer A and Layer H item has an outcome.

6. **Render Layers A and H.** Write the objective report under `docs/realign/`
   using `references/report-template.md`. Layer A records current state; Layer H
   records pinned target design and observed maturity. Include exact revisions
   and pass the Layer A cleanliness gate.

7. **Add Layer B only when asked.** Drift classification and recommendations
   belong in a separate appendix. They never frame or interleave the objective
   body. Classification is not prioritization; priorities are optional and
   require an explicit request.

For a full audit, process dimensions in bounded batches. Use no more than the
currently available worker slots minus one so the coordinator remains free.
Measurement and verification are separate passes; they need not be resident at
the same time. If independent workers are unavailable, measure and then run a
fresh, explicitly labeled verifier pass without pretending it was independent.

## Inward boundary

- Compare pi's current behavior and documented target architecture to opi's
  current Rust implementation, preserving the evidence boundary between them.
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

- Cite `file:line` for every claim or state the exact absence search.
- Check the changelog before making version/currentness claims.
- Derive Layer H only from artifacts inside the pinned target tree. Record
  self-declared authority and label explanatory, informative, open, and
  non-goal material accordingly.
- Do not claim compatibility without direct evidence.
- Reports are generated, non-normative artifacts. Do not edit product source,
  `docs/opi-spec.md`, READMEs, or roadmaps while running this skill.
- Do not commit unless asked.

Summarize current deltas and target-design horizons separately, then link the
report. Route outward questions to `opi-research` and design decisions to direct
shaping rather than turning the audit into an implementation plan.

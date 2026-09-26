---
name: opi-research
description: Research capabilities beyond pi against primary sources, assessing Rust feasibility and Opi placement.
disable-model-invocation: true
---

# Opi Research

Investigate an outward capability question that pi does not answer, implements
poorly for opi's needs, or deliberately leaves to its surrounding ecosystem.
This is distinct from `opi-realign`, which tracks opi's inward design lineage
against a concrete pi revision.

## Input

Establish a bounded question, target users, and decision the evidence must
inform from the request and existing context. Ask only when missing information
would materially change the research scope.

## Subskill

Open and invoke Matt `research` for background fact gathering, primary-source
evidence, claim-level citations, and the Markdown artifact. Supply the bounded
question and the Opi evidence contract below.

Write the report under:

```text
docs/research/YYYY-MM-DD-<topic>.md
```

Match an existing English/Chinese counterpart convention when the research area
already has one. Otherwise a single research artifact is sufficient.

## Opi evidence contract

The report must cover:

1. **Question** — the capability or decision being investigated.
2. **Relationship to pi** — why pi is absent, insufficient, or unsuitable as
   the sole reference. Link exact pi evidence when relevant.
3. **Primary-source findings** — official specifications, documentation, source
   code, or first-party APIs supporting each material claim.
4. **Alternatives** — viable approaches and their tradeoffs.
5. **Rust feasibility** — crate/platform constraints, safety implications, and
   implementation risks.
6. **Existing extension fit** — whether current packages, resources, lifecycle
   hooks, custom tools/providers, or process adapters can express the feature.
7. **Smallest missing core seam** — only when the feature cannot be expressed
   through existing extension points.
8. **Placement candidates** — use the Agent Core, Reference Product,
   Extension Ecosystem, and Independent Companion boundaries in
   `docs/opi-spec.md` Chapters 4 and 8. Opi-specific optional capabilities
   default to plugins/packages; Agent-neutral capabilities begin as Independent
   Companions. A missing core seam still requires the core admission gates.
9. **Unresolved decisions** — product and architecture choices the evidence
   cannot settle.
10. **Limitations and non-findings** — unavailable sources, uncertainty, and
    claims the research could not support.

Complete when every item above has cited findings or an explicit limitation.
Link the report and summarize the decision-relevant findings and unresolved
choices. Placement remains a recommendation. Human-led shaping may use Matt
`wayfinder` or Matt `grill-with-docs`; recommend an exact invocation when useful,
but these user-invoked skills require the user's request.

## Boundaries

The report is non-normative outward evidence. This workflow does not authorize
specification or supplemental-design edits, implementation tickets or ledger
tasks, product decisions, implementation, commits, pushes, or publication.
Keep inward pi alignment in `opi-realign`.

---
name: opi-research
description: Research capabilities beyond or poorly served by pi, using Matt research against primary sources and evaluating Rust feasibility plus plugin-first placement for the opi ecosystem.
disable-model-invocation: true
---

# Opi Research

Investigate an outward capability question that pi does not answer, implements
poorly for opi's needs, or deliberately leaves to its surrounding ecosystem.
This is distinct from `opi-realign`, which tracks opi's inward design lineage
against a concrete pi revision.

## Input

Require a bounded research question. If the user supplies only a feature name,
clarify the capability, target users, and decision the evidence must inform
before dispatching research.

## Subskill

Open and invoke Matt `research`. It owns background fact gathering, primary
source preference, claim-level citations, and the Markdown research artifact.
Do not replace it with a secondary web summary.

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
8. **Placement candidates** — Minimal Runtime, core extension seam, official
   plugin/package, or external example. Optional, provider-specific,
   experimental, and non-pi capabilities default toward plugin/package form.
9. **Unresolved decisions** — product and architecture choices the evidence
   cannot settle.
10. **Limitations and non-findings** — unavailable sources, uncertainty, and
    claims the research could not support.

Placement is a recommendation, not an approved product decision. Feed the
report into direct human deliberation, Matt `wayfinder`, or Matt
`grill-with-docs` as appropriate. The Matt shaping skills are user-invoked:
recommend the exact explicit invocation and stop rather than claiming research
invoked them.

## Boundaries

Do not:

- modify `docs/opi-spec.md` or a supplemental design;
- create implementation tickets or `.opi-impl-state.json` tasks;
- select the product direction on the user's behalf;
- treat every interesting external capability as core work;
- merge this workflow into `opi-realign`;
- implement, commit, push, or publish the researched capability.

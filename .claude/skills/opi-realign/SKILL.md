---
name: opi-realign
description: Audit opi inward against an exact earendil-works/pi revision while preserving pi design lineage through Rust-native implementation choices.
disable-model-invocation: true
---

# Opi Realign

Realign is the **inward alignment** workflow. It measures opi against an exact
revision of `earendil-works/pi` and produces a cited delta ledger. It asks
whether opi still preserves pi's design ideas and visible semantics while
expressing them through Rust ownership, dependency, concurrency, packaging,
and testing norms.

A fresh audit uses today's source only. Prior audits, implementation phases,
roadmaps, and baseline documents do not define the result. They may be cited
only for explicit non-goals or source anchors.

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

2. **Measure both sides from source.** For every in-scope dimension in
   `references/dimensions.md`, record current opi and pi state with `file:line`
   anchors. Express absence as `absent: searched <paths>`, never as silence.
   *Done when:* every dimension has cited evidence for both projects.

3. **Write objective deltas.** For each consequential item, record opi state,
   pi state, and the raw difference. Exclude judgment, phases, plans, and
   recommendations.
   *Done when:* each difference is factual and independently understandable.

4. **Verify adversarially.** A verifier independent from the measurer hunts opi
   source for every claimed gap and checks every parity claim for overstatement.
   Assign the outcome from `references/audit-framework.md`; drop or footnote
   refuted deltas and correct refined ones.
   *Done when:* every retained delta has an outcome.

5. **Render Layer A.** Write the objective report under `docs/realign/` using
   `references/report-template.md`. Include exact revisions and pass its body
   cleanliness gate.

6. **Add Layer B only when asked.** Drift classification and recommendations
   belong in a separate appendix. They never frame or interleave the objective
   body. Classification is not prioritization; priorities are optional and
   require an explicit request.

For a full audit, process dimensions in bounded batches. Use no more than the
currently available worker slots minus one so the coordinator remains free.
Measurement and verification are separate passes; they need not be resident at
the same time. If independent workers are unavailable, measure and then run a
fresh, explicitly labeled verifier pass without pretending it was independent.

## Inward boundary

- Compare pi's current design and behavior to opi's Rust implementation.
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
- Do not claim compatibility without direct evidence.
- Reports are generated, non-normative artifacts. Do not edit product source,
  `docs/opi-spec.md`, READMEs, or roadmaps while running this skill.
- Do not commit unless asked.

Summarize the highest-signal inward deltas and link the report. Route outward
questions to `opi-research` and design decisions to direct shaping rather than
turning the audit into an implementation plan.

# Opi Spec OpenClaw Evidence Optimization Design

## Purpose

Carefully strengthen `docs/opi-spec.md` using the durable findings in
`docs/research/2026-08-11-openclaw-agent-practices-opi-spec.md` without changing
Opi's existing mission, product placement doctrine, Agent Core admission model,
or authority hierarchy.

The revision closes proven semantic gaps and adjusts the evolution route. It
does not copy OpenClaw product architecture, turn research findings into
implementation status, or admit experimental capabilities into Agent Core.

## Chosen approach

Use a minimal semantic-closure revision:

- retain the existing eleven chapters and sixty-one stable clause identifiers;
- strengthen existing `CTRL`, `INV`, and `STRAT` clauses instead of adding a new
  governance layer;
- add only durable requirements that remain valid across products, harnesses,
  providers, package formats, and evaluation implementations;
- keep volatile mechanisms, field inventories, and acceptance details in Phase
  delivery specifications and authoritative implementation contracts; and
- preserve equivalent English and Chinese normative specifications.

Two alternatives were rejected:

- leaving the research entirely non-normative would preserve unresolved
  authority and Package Trust ambiguity; and
- importing every P0 recommendation would prematurely freeze experimental Eval,
  memory, and proactive-Agent mechanisms into the long-term specification.

## Normative changes

### 1. Resolved execution and evidence provenance

Strengthen the existing evidence and artifact clauses so a finalized run can be
explained offline as the result of a resolved execution, not merely a model
name and final outcome.

The durable contract will require evidence to retain:

- harness, runtime, and adapter identity, version, and configuration digest;
- requested and actual provider, model, wire, authentication profile, and
  fallback outcome;
- context and compaction policy identity when they materially affect the run;
- effective User Policy and Active Snapshot binding;
- trigger provenance for non-interactive work; and
- cost origin and the reason for unavailable measurements.

These details belong in evidence and snapshot contracts. Agent Core continues
to expose only the minimum product-neutral correlation and finalized-artifact
references required to assemble them. Exporters, reports, hosted stores, and
evaluation policy remain outside Agent Core.

### 2. Benchmark integrity

Preserve native graders as the source of headline outcomes while making clear
that a native grader does not prove the benchmark itself is valid.

Evaluation admission and reporting will distinguish:

- valid Agent outcomes;
- broken or unsatisfiable tasks;
- ambiguous or prompt/test-misaligned tasks;
- infrastructure and resource failures; and
- excluded or retired benchmark material.

Coverage and exclusion reasons remain visible. Invalid or infrastructure-failed
trials do not silently become Agent failures, successful trials, or zero-valued
measurements. Detailed audit procedures remain Phase and evaluation-product
contracts rather than permanent benchmark inventories in the parent spec.

### 3. Model-visible data cannot create authority

Strengthen the existing tool-side-effect invariant. Content originating from a
tool, retrieval adapter, channel, memory item, extension package, or another
Agent remains untrusted for authority even after it enters model-visible
context.

Effective permission, capability, schema, and scope are derived only from User
Policy and trusted runtime state before a side effect. A model, classifier,
prompt label, or risk score may deny, mark risk, or escalate to Human Authority;
it cannot grant `Permitted`, weaken policy, or widen scope.

This is a runtime invariant with negative-path verification, not merely an
evidence field or prompt-engineering recommendation.

### 4. Package Trust identity

Retain the existing five independent lifecycle states: Installed, Trusted,
Enabled, Selected, and Permitted. Make Package Trust precise enough to support
that state model:

- Package Trust is bound to an exact immutable package artifact digest;
- the trust record includes the package's declared capability footprint for
  user review and diagnostics;
- a changed artifact or expanded footprint is a new trust object and cannot
  inherit Trusted automatically;
- affected Capability Permission does not automatically carry to the new or
  expanded object; and
- signatures, scans, registry provenance, and reviews are evidence, not sources
  of Trust, Enablement, selection, or permission.

This strengthening is necessary because name-level or publisher-level trust
would allow arbitrary future code to inherit authorization from a previously
reviewed artifact. It does not require a mandatory signature infrastructure,
package registry, public-key hierarchy, or Agent Core dependency.

Package Trust remains Reference Product policy. Declared capability footprints
remain metadata and do not claim operating-system enforcement. Extension code
continues to run with the launching user's operating-system authority unless a
separate execution or sandbox mechanism enforces stronger restrictions.

### 5. C1 memory and skill evidence

Strengthen the existing Continual Learning route without choosing a memory
implementation. A C1 memory or skill candidate will retain its source episode,
permission snapshot, ownership, expiry, contradiction, and withdrawal state.
Evaluation compares it with no-memory/no-learning and ordinary-context
baselines and covers retention, negative transfer, and action-coupled safety.

The existing route remains unchanged: episodic memory precedes reusable skills,
shadow precedes activation, the Candidate Producer cannot select its own
cohorts or thresholds, and activation is not automatic by default.

Markdown storage, embeddings, hybrid retrieval, MMR, dreaming, and consolidation
algorithms remain experimental implementation choices outside the parent spec.

## Evolution-route changes

The strategic ordering remains Agent Core semantics, independent Eval,
measurable capability, C1 learning, C2 behavior candidates, and finally
Controlled Self-Iteration.

Within that ordering:

- `STRAT-001` explicitly prioritizes resolved-execution provenance and
  authority validation before side effects alongside provider dispatch and
  next-turn replacement;
- `STRAT-002` includes benchmark integrity, failure classification, coverage,
  and retirement before Learning or Promotion depends on Eval claims;
- `STRAT-004` adds source-bound C1 evidence and ordinary-context/no-memory
  controls; and
- proactive triggers, Gateway/control-plane behavior, and multi-Agent
  orchestration remain parallel Reference Product, Extension Ecosystem, or
  Independent Companion experiments until real consumers, shared conformance,
  and frozen evaluation evidence justify a Placement Review.

No Gateway, heartbeat, scheduler, A2A protocol, multi-Agent coordinator, memory
store, or influence graph is admitted into Agent Core by this revision.

## Files and synchronization

The implementation revision will modify:

- `docs/opi-spec.md` as the normative English source;
- `docs/opi-spec.zh.md` as its identifier-preserving equivalent Chinese source;
  and
- `scripts/opi-doc-check.py` only if a synchronization check must be strengthened
  without changing the sixty-one-clause identity contract.

No new domain term is required. `docs/CONTEXT.md` remains unchanged unless the
actual normative wording exposes a semantic ambiguity that cannot be expressed
using its current definitions of Active Snapshot, User Policy, Capability
Permission, Permission Scope, and Package Trust.

## Verification

The completed revision must demonstrate:

1. identical chapter order and stable clause identifiers in both languages;
2. no new progress, release, implementation-status, or volatile inventory text;
3. every strengthened `MUST` retains an owner and mechanical verification route;
4. Package Trust remains Reference Product policy and does not imply sandboxing;
5. research evidence does not override the parent authority chain;
6. the strategic route does not bypass Agent Core, Eval, Learning, or Promotion
   prerequisites; and
7. documentation contract and focused test commands pass.

The implementation diff will be reviewed clause by clause against the research
document and the pre-change specification. Unrelated dirty-worktree changes are
out of scope and will be preserved.

# Comparison and transfer guide

Use the relevant input lenses while framing scope, the comparison dimensions
while tracing counterparts, and the transfer standard before recommending change.
The report contract governs the final Opi-facing synthesis, not the subskill's
source-research notes.

## Input lenses

These are lenses, not separate workflow modes. Combine them for mixed material;
expand them when source evidence reveals another decision-relevant dimension.

| Input | Investigate | Opi-facing answer |
|---|---|---|
| Research paper, experiment, or technical talk | Exact mechanism, assumptions, baselines/ablations, workloads, reproducibility, negative results, and scope of demonstrated effects | Which current mechanism could benefit, is already aligned, or lies outside the experiment's applicability? A toy result is not production proof. |
| Another Agent framework or product | Comparable user journeys, actual feature reachability, defaults, APIs, composition, state/lifecycle, operational model, and maturity | What product and semantic gaps exist in either direction? Which mechanism is useful without importing the other framework's entire architecture? |
| Future technical or product design | Proposed problem, assumptions, competing routes, prerequisites, uncertainty, and reversible decision points | Which route fits pi's design and current Opi, what would need to become true, and what evidence would distinguish routes? A proposal is not shipped capability. |
| Communication protocol or interoperability standard | Participants, addressing, message schema/versioning, ordering, delivery/replay/deduplication, shared state, cancellation, backpressure, failure/recovery, trust, and transports | Where do Opi's current process/tool/session/event mechanisms match or differ? A shared transport alone does not establish equivalent communication semantics. |
| Memory, learning, planning, or orchestration method | State ownership/lifetime, scheduling, context flow, provenance, invalidation, isolation, evaluation, and feedback loops | Is the value in a reusable mechanism, optional workflow, or integration? Distinguish retrieval from learning and coordination from a new core requirement. |
| Provider, SDK, tool, library, or systems technique | Caller contract, exact versions/features, platform support, Rust ownership/error/concurrency model, native/unsafe boundaries, licensing, and maintenance | Is there a smaller implementation or integration improvement at an existing Opi call site? A crate's existence is not integration proof. |
| Benchmark, performance claim, security incident, or failure analysis | Workload/threat model, control conditions, denominators, missing/failing cases, resource costs, affected versions, and mitigations | Does Opi share the preconditions or failure path? What concrete test could confirm risk or benefit without treating a score or incident as universally applicable? |
| Ecosystem, deployment, or developer-experience practice | Installation, configuration, extension discovery, debugging, portability, support, and operational burden | Which end-to-end user friction is real in current Opi, and can existing interfaces solve it without increasing mandatory complexity? |

An author's name or the word "latest" does not identify a unique source. Resolve
the intended work and its date/version; ask when ambiguity changes the question.
Secondary commentary can locate a primary source, but is not a substitute for it.

## Comparison dimensions

Use a problem-first three-way map: **external finding → pi intent/capability →
current Opi implementation → consequence**. Cite each side independently; an
unavailable side remains unknown. Cover all material surfaces within scope, not
every unrelated feature of every repository.

For relevant dimensions, trace:

- **Product:** user, problem, entry point, discoverability/configuration, defaults,
  complete workflow, outputs, and practical adoption/operation costs.
- **Contract and mechanism:** callers, parameters/defaults, state ownership,
  lifecycle/order, persistence, protocol/data flow, composition, extension seams,
  cancellation, resource bounds, errors, recovery, and observability.
- **Engineering:** module depth, dependency direction, actual reuse/consumers,
  Rust-specific ownership and concurrency, platform/build/release constraints,
  maintenance, security/privacy, authority, and supply-chain exposure.
- **Evidence:** implemented vs declared behavior, experimental conditions,
  counterexamples, unsupported generalization, conflicting sources, and checks
  actually executed vs read or proposed.

A same-named feature may differ end to end; a differently named composition may
already solve the problem. Trace public reachability before concluding either.
Record documentation/code disagreements rather than choosing whichever supports
an attractive recommendation. Opi's absence is not automatically a defect, and
external absence is not automatically Opi superiority.

## Transfer standard

Evaluate each material candidate against every criterion below. Use qualitative,
evidence-backed reasoning rather than an invented numeric score or a quota of
recommendations. Include retaining current behavior and reuse/composition as
alternatives to change.

| Criterion | Evidence needed |
|---|---|
| Real relevance | A concrete Opi user problem, mechanism weakness, credible future need, or independently supported alignment with the research; identify the affected journey/call site or bounded absence search. |
| Mechanism and evidence validity | Explain what causes the claimed effect and under which assumptions. Identify demonstrated results, inference, counterevidence, and missing proof; fame, recency, or author claims alone do not qualify. |
| Pi design fit | Cite relevant official/package design intent and explain compatibility or tension, including module depth, extensibility, composition, and policy placement where applicable. Neither pi's current feature list nor a slogan settles the issue. |
| Rust and current-system feasibility | Identify actual integration points, existing behavior to reuse/preserve, version/platform constraints, ownership/concurrency implications, dependencies, and unresolved engineering risks. |
| Net value over the alternatives | Compare likely user/correctness/capability benefit with complexity, runtime/resource costs, maintenance, authority/security/privacy, and migration burden. State when the value is still only a hypothesis. |
| Smallest falsifiable next step | Give the smallest relevant offline probe or conditional experiment, observable success and failure criteria, prerequisites, and a reversible adoption path. Proposing a check is not claiming it passed or authorizing its execution. |

Give each candidate one reasoned disposition:

- **Learn/adapt:** evidence supports a useful, pi-compatible direction at an
  identified Opi location. Explain the mechanism to borrow, what not to copy,
  uncertainties, and the smallest validation. This is a recommendation, not
  approval or proof that improvement has already occurred.
- **Retain/aligned:** current Opi already solves the relevant problem or follows
  the supported mechanism. Cite the equivalence and its limits; avoid churn for
  terminology or framework parity. Research compatibility alone is not measured
  performance validation.
- **Watch/insufficient evidence:** potential relevance exists, but a named source,
  applicability, feasibility, benefit, or baseline gap prevents a recommendation.
  Identify the evidence or concrete trigger that would justify revisiting it.
- **Not recommended:** the mechanism is inapplicable, duplicates adequate behavior,
  has negative net value, or conflicts with the pi design baseline. State why,
  acknowledge any useful part, and name any condition that could change the result.

A technically interesting departure from pi may be discussed as a trade-off, but
is not an adoption recommendation under this baseline. It requires a separate,
explicit user decision to revisit direction. Opi planning documents are neither
support for nor a veto against any of these dispositions.

## Report contract

Organize the report for the actual question; the following coverage is required,
not a demand for identical headings or a fixed number of opportunities.

1. **Decision summary:** what Opi can learn, should retain, should not copy, and
   cannot yet decide. Say explicitly if no change is justified or research is
   incomplete. Separate coverage/completion from evidence sufficient for a choice.
2. **Scope and identities:** source/question, users/journeys, Opi working-tree
   baseline, pi documentation and snapshot identities, online/snapshot differences,
   exclusions, access limitations, and delegated research artifact links.
3. **External findings:** mechanisms, assumptions, evidence/maturity, alternatives,
   contradictory or negative findings, and claim-level primary-source citations.
4. **Three-way capability map:** for every material item, external behavior, pi
   intent/implementation, Opi entry point and source anchors, present/partial/
   absent-within-search/unresolved status, meaningful differences, and consequences.
   For non-matches, record the bounded search; never invent a corresponding feature.
5. **Transfer analysis:** apply the standard and give each material candidate a
   disposition. For improvements, identify the module/interface/caller or product
   workflow, what changes and stays unchanged, expected benefit, costs/risks,
   dependencies, and the smallest way to test the hypothesis. Explain rejections
   and supported alignment as carefully as positive advice.
6. **Conditional future directions:** order worthwhile options by evidenced value
   and prerequisites, not an existing roadmap. Explain technical/product route
   alternatives, near-term small steps vs longer-term hypotheses, validation and
   revisit triggers. These are references for human iteration, not admitted tasks,
   a release promise, or a competing implementation plan/ledger.
7. **Review and limits:** decisive claim rechecks, bounded absence searches,
   actual executed checks and results, checks not run, delegation/verifier identity
   or self-review, inaccessible evidence, residual uncertainty, and next evidence.

Distinguish **source fact**, **inference**, **recommendation**, and **executed
result** wherever readers could confuse them. Documentation/test source establishes
what is described or tested, not that a check passed in this investigation.
Performance or causal improvement claims require applicable measured evidence.
The final chat handoff links the report and leads with Opi-specific conclusions,
not a recap of the external author's abstract.

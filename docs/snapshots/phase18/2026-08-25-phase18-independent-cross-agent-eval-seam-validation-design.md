# Phase 18: Independent Cross-Agent Eval Seam Validation

## Status and authority

This document is the human-shaped candidate Phase 18 delivery specification.
It derives one finite delivery from the durable direction in
[`docs/opi-spec.md`](../../opi-spec.md). It becomes a registered supplemental
source only when an explicit `opi-implement plan` invocation maps it into the
implementation ledger and the human graph gate confirms that mapping. Until
then it is a candidate specification, not implementation status or
delivery authority.

The normative parent remains `docs/opi-spec.md`. Domain language remains owned
by [`docs/CONTEXT.md`](../../CONTEXT.md). Current product, CLI, protocol, and
artifact facts remain owned by source, crate documentation, generated help,
schemas, fixtures, manifests, and the completed Phase 17 snapshot. Only
`opi-implement` may write `.opi-impl-state.json` or register this source.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-AUTH-001 | Phase 18 **MUST** implement the parent clauses cited below without lowering, reinterpreting, or bypassing their gates. | Phase lead | Admission mapping and clause-by-clause design review. |
| P18-AUTH-002 | Phase 18 **MUST** derive from `STRAT-002 — Establish independent cross-Agent Eval`. A different strategic goal **MUST** require an explicit parent-route revision before implementation continues. | Phase shaper | Source mapping to `STRAT-002` and blocked-handoff review. |
| P18-AUTH-003 | Research, realignment material, pi Harness v2, the earlier local Eval design, and implementation observations **MUST** remain evidence; none **MUST** override this Phase or the parent specification. | Phase shaper | Source-classification review. |
| P18-AUTH-004 | Implementation status, task state, dates, completion claims, benchmark scores, and seam-admission claims **MUST NOT** be written into this document or `docs/opi-spec.md`. | Implementation workflow owner | Documentation diff review and `scripts/opi-doc-check.py`. |
| P18-AUTH-005 | Any implementation discovery that requires a broader Agent Core seam, a lower evidence gate, or a different authority boundary **MUST** stop delivery for explicit shaping rather than enter as an incidental task. | Phase lead | Blocked handoff and route-revision review. |

## Parent-clause traceability

Phase 18 addresses the currently highest-priority admissible unmet goal,
`STRAT-002 — Establish independent cross-Agent Eval`. Phase 17 completed the
minimum Agent Core evidence prerequisite; Phase 18 consumes that evidence from
outside the runtime and does not reopen `STRAT-001`.

| Parent clause | Phase 18 responsibility |
|---|---|
| `GOAL-001` | Name one end-to-end owner for the assembled Eval product and accept the complete Agent-to-grader-to-report path rather than isolated components. |
| `GOAL-004` | Leave the Opi Minimal Runtime useful and behaviorally unchanged when Eval is absent, unconfigured, or failed. |
| `PRIN-001`–`PRIN-003` | Keep Eval outside Agent Core, avoid speculative facades, and place product orchestration above process-neutral mechanisms. |
| `PRIN-004` | Fail closed at manifest, adapter, evidence, pairing, integrity, grader, and report boundaries. |
| `PRIN-005` | Bind every outcome or efficiency claim to immutable, reproducible evidence. |
| `PLACE-001`–`PLACE-004` | Incubate Eval as an Agent-neutral Independent Companion with no Opi crate dependency and no experimental feature-flag path into core. |
| `CAP-003` and `CAP-006` | Use completed Agent Execution evidence as the entry condition; do not claim Continual Learning or Promotion readiness from an incomplete Eval seam. |
| `CTRL-001`–`CTRL-003` | Consume stable Opi correlation, finalized artifacts, resolved-execution provenance, and pre-export redaction without adding an exporter to Agent Core. |
| `CTRL-004` | Make admitted benchmark-native graders the sole headline outcome authority. |
| `CTRL-005` | Pair baseline and candidate by task and trial under one frozen manifest, with complete coverage and failure classification. |
| `CTRL-006` | Recompute, regrade, and render reports offline from immutable bundles; assign a new identity to every real execution. |
| `CTRL-007` | Keep datasets, grader implementations, images, sandboxes, schedulers, exporters, and leaderboards outside the Eval module; freeze only seams proved by two real Agents and two native graders. |
| `CTRL-008`–`CTRL-010` | Keep evidence generation separate from candidate generation, promotion, and Human Authority; Eval failure changes no active runtime. |
| `INV-006` | Preserve visible cancellation, timeout, partial, queue, adapter, and grader failures rather than converting them to success. |
| `INV-007`–`INV-008` | Consume the validated session branch and Runtime Input Binding represented by finalized Opi evidence without creating a second session truth. |
| `GATE-001`–`GATE-002` | Reject pi parity, unstable labels, or a feature flag as substitutes for placement and conformance evidence. |
| `PHASE-001`–`PHASE-006` | Keep the delivery finite, placed, risk-bounded, testable, reversible, and subordinate to its parent route. |

## Evidence and current implementation state

### Completed Phase 17 prerequisite

The implementation ledger records Phase 17 as complete with all 70 criteria
accepted and the three-platform CI evidence preserved in
[`docs/snapshots/phase17/opi-impl-state.json`](../../snapshots/phase17/opi-impl-state.json).
The delivered runtime now exposes:

- stable run, turn, call, parent, and sequence identities;
- a storage-neutral evidence lifecycle with no-op and in-memory core adapters;
- a Reference Product file adapter that writes `evidence.jsonl` and publishes a
  finalized `manifest.json` only on successful finalization;
- resolved route, authentication provenance, User Policy, Runtime Input
  Binding, session, usage, artifact, and unknown-with-reason facts; and
- explicit opt-in capture through the real `opi` process.

The current evidence contract is intentionally writer-oriented and 0.x. It is
not a cross-Agent RunBundle reader, benchmark manifest, final-workspace
snapshot, or grader contract. Phase 18 may build a private, fail-closed Opi
importer around the current product artifacts, but it does not promote the
current on-disk shape to a public cross-Agent protocol.

### pi Harness v2 evidence boundary

The current inward evidence is
[`docs/realign/2026-08-25-opi-vs-pi-v0.84.3.md`](../../realign/2026-08-25-opi-vs-pi-v0.84.3.md).
Its Layer H correctly distinguishes pi's target design from shipped behavior:

- the session/storage work is partial;
- the public `AgentHarness` is a compile-complete scaffold whose operational
  methods remain unavailable;
- the build transition and a shared search-index sink are designed only; and
- Harness v2 explicitly leaves open several repair, overflow, and persistence
  questions.

The primary
[`AgentHarness implementation specification`](../../../.repo/pi-0.84.3/packages/agent/docs/harness.md)
declares only its literal `must` and `must not` clauses normative for pi. Its
broader system model remains high-value design evidence rather than an Opi
requirement.

Phase 18 adopts five lessons at the Eval control-plane boundary:

1. stable intent, effect, settlement, and terminal identities are distinct;
2. a durable intent without settlement represents an effect-unknown interval;
3. terminal outcome and finalized artifacts need an authoritative completion
   predicate rather than inference from a process-local event;
4. failed and retried calls retain usage and provenance; and
5. parallel work forms a causal graph, not necessarily one total event order.

Phase 18 does not adopt pi's `AgentHarness`, named lanes, register namespaces,
session formats, operation-state union, search service, remote protocol,
manual-drive API, or storage backends. The current pi coding-agent process, not
the Harness v2 scaffold, is the real external integration.

### Existing Eval evidence

The approved
[`Opi Local Eval Foundation Design`](../../superpowers/specs/2026-08-11-opi-local-eval-foundation-design.md)
remains useful evidence for sealed bundles, deterministic grading, bounded
capture, unknown measurements, non-compensatory verdicts, and case review. Its
chosen implementation route deepens the project-local `opi-eval` skill and
explicitly excludes a cross-Agent Companion, so it does not satisfy this Phase.

The
[`Agent benchmark plan`](../../research/2026-07-10-opi-agent-benchmark-plan.zh.md)
provides non-normative evidence for Opi/pi pairing, model-control fingerprints,
Terminal-Bench/Harbor, benchmark-native verification, failure classification,
environment isolation, and full-season operations. Phase 18 takes only the
minimum seam-validation slice. Full benchmark seasons, statistical comparison,
release gates, and public submissions remain later shaping.

The pinned pi
[`packages/evals`](../../../.repo/pi-0.84.3/packages/evals/README.md) package is
additional inward implementation evidence. Its useful mechanisms are stable
harness names, explicit provider/model selection, isolated temporary project
and Agent directories, canonical input-plus-repetition grouping, one baseline
against multiple declared candidates, incomplete-observation diagnostics, and
native session attachments. Phase 18 may reuse those semantics in its
conformance fixtures and pairing design, but it does not depend on
`vitest-evals`, link pi's `AgentSession`, consume user-global authentication, or
adopt model-backed judges as benchmark authority. pi eval artifacts are mutable
run-local JSONL/attachments and do not supply Phase 18 bundle sealing,
effect-unknown recovery, benchmark task packages, or native-verifier
provenance.

The [GLM-5.3 release and evaluation notes](https://z.ai/blog/glm-5.3) are
outward evidence for the desired long-range coverage catalog and for keeping
model controls distinct from Agent harness identity. The page's reported
Terminal Bench 2.1 and 3.0 runs use Claude Code, while DeepSWE uses
mini-swe-agent. Phase 18 does not copy those fixed-harness configurations as its
evaluated subject. It freezes one model configuration and varies the real Agent
harness so that Opi, pi, and later admitted harnesses can be compared.

The current repository has no independent Eval library or CLI, no
machine-readable cross-Agent experiment contract, no native-grader integration,
no content-addressed RunBundle, and no offline paired report recomputation. The
existing `opi-eval` skill remains an explicit, credential-consuming
provider-fidelity workflow rather than this product.

## Outcome

After Phase 18, one unpublished Independent Companion prototype can evaluate
the real Opi and earendil pi coding-agent processes through one N-harness-capable
process contract, run three pinned benchmark revisions from their complete
official task packages, seal content-addressed run bundles, and reproduce the
same paired outcome-first report without executing either Agent again.

The delivery validates a seam; it does not claim a benchmark win. Its
conformance matrix determines the smallest common contract supported by:

- Opi and earendil pi as the first two real Agent harness integrations;
- Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1 as three pinned
  official task-package and native-verifier integrations; and
- hermetic negative fixtures for missing evidence, mutation, cancellation,
  failure ownership, integrity adjudication, and incomplete pairs.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-OUT-001 | One frozen experiment **MUST** execute or import exactly identified Opi and pi trials through an N-harness-capable contract without linking either runtime into the Eval process. | Evaluation product owner | Dependency audit and real-process Agent integration conformance. |
| P18-OUT-002 | Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1 **MUST** each grade sealed Agent outputs through a pinned complete official task package and native verifier, and their native results **MUST** remain the headline outcome authority. | Benchmark integration owner | Three-revision native smoke plus shared benchmark conformance. |
| P18-OUT-003 | Every accepted trial **MUST** produce or reference an immutable, content-addressed bundle that retains resolved experiment, Agent, environment, trajectory, final-workspace, grader, failure, and artifact provenance. | Evidence Producer | Bundle schema, digest, mutation, and provenance tests. |
| P18-OUT-004 | A paired report **MUST** be reproducible from sealed bundles without running an Agent, and a new real execution **MUST** receive a new trial identity. | Evaluation orchestrator | Offline recompute/regrade/render and identity-reuse negative tests. |
| P18-OUT-005 | The conformance result **MUST** identify the minimum shared seam actually proved by both Agents, all three benchmark revisions, and at least two independently owned native-verifier contracts; unproved package, type, process, trajectory, and span choices **MUST** remain provisional. | Placement Review participants | Phase 18 integration matrix and seam-evidence review. |
| P18-OUT-006 | Opi without the Companion **MUST** retain the same Minimal Runtime behavior, default I/O, capture state, User Policy, and public CLI semantics. | Reference Product owner | Before/after Minimal Runtime acceptance and dependency/call-site review. |

## Non-goals

Phase 18 excludes:

- implementing pi Harness v2, an Opi `AgentHarness`, named lanes, multi-lane
  queues, session search/indexing, or a new session repository;
- implementing Claude Code, Codex, OpenCode, or another post-Phase-18 harness
  adapter, even though the common contract and roadmap admit them later;
- adopting pi's experimental protocol/client/server stack, remote multi-session
  hosting, a browser transport, or a persistent Eval RPC service;
- expanding `opi-agent`, `opi-ai`, `opi-coding-agent`, `opi-protocol`,
  `opi-sandbox`, or `opi-tui` with Eval-specific public types;
- declaring Opi's Phase 17 evidence JSONL, pi JSON output, ATIF, or a
  supplemental span graph to be the canonical cross-Agent format before
  conformance;
- complete Terminal-Bench 2.1/3.0 or DeepSWE v1.1 performance seasons, `k=5`,
  `avg@3`, leaderboard-aligned runs, statistical significance, or an
  Opi-versus-pi superiority claim;
- public leaderboard submission, official-verified claims, release gating,
  hosted scheduling, remote workers, dashboards, exporters, or artifact
  services;
- vendoring benchmark datasets, native graders, container images, sandboxes,
  oracle patches, or large trajectories into the Eval module;
- AgentDojo, BFCL, AppWorld, browser, GUI, MCP, or general domain-tool
  benchmark integrations;
- an Eval-authored LLM judge, a judge-mediated Phase 18 benchmark, evaluator
  calibration, a composite score, best-trial selection, or replacement of
  native grader authority; future official judge-mediated methods remain
  governed by `P18-RDM-004`;
- Continual Learning, Candidate Production, Promotion, Active Snapshots,
  behavioral activation, automatic remediation, or source modification;
- default telemetry, default evidence capture, background upload, private raw
  Chain-of-Thought capture, or broader runtime authority; and
- publication, repository extraction, final product naming, final brand, or a
  stable third-party SDK.

## Considered approaches and decision

Three scopes were considered during shaping:

| Approach | Result |
|---|---|
| Deepen the project-local `opi-eval` skill or specify contracts with mocks only | Rejected for Phase 18. It cannot satisfy the two-real-Agent and three-native-benchmark-revision seam-admission evidence. |
| Validate a two-Agent by three-benchmark-revision smoke seam | Selected. It is the smallest closed loop that satisfies `STRAT-002`, proves dataset-first reuse, and keeps public shapes provisional. |
| Build the complete benchmark control plane and full seasons | Deferred. It combines seam validation with scheduling, operations, statistics, release policy, and submission work. |

The selected design proves the boundary first. A later Phase may use its
evidence to shape full seasons, release comparison, or a Placement Review, but
Phase 18 does not pre-authorize those outcomes.

## Architecture placement case

| Capability | Placement | Why it belongs there | Deletion test and seam evidence |
|---|---|---|---|
| Experiment resolution, comparison edges, immutable bundles, offline reports | Independent Companion prototype | These solve an Agent-neutral evaluation problem and have value without Opi. | Removing the Companion leaves Opi unchanged but removes cross-harness comparison and reproducibility. Opi and pi provide the first two real consumers. |
| Agent harness process integrations | Companion-owned provisional adapters | They translate each product's stable process surfaces and native artifacts without linking runtimes. | Opi and pi share one N-harness-capable conformance contract while retaining product-specific launch and evidence rules. |
| Benchmark task-package and verifier integrations | Companion-owned provisional adapters over external artifacts/tools | Dataset resolution, environment execution, verifier invocation, and provenance are Agent-neutral; benchmark implementations remain external. | Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1 provide three real revision consumers and shared conformance. |
| Phase 17 evidence production | Existing `opi-agent` mechanism and `opi-coding-agent` file adapter | Correlation, redaction, Runtime Input Binding, and finalization are intrinsic runtime facts already completed in Phase 17. | Phase 18 consumes them. Duplicating or widening the core seam would fail the deletion and placement tests. |
| Opi/pi CLI, configuration, tools, and sessions | Their existing Reference Products | Product prompts, tools, session formats, and output protocols are part of the evaluated scaffold. | The Companion invokes the products as processes and does not normalize away their product identity. |
| Datasets, graders, containers, sandboxes, provider endpoints | External benchmark/runtime owners | These have independent authority, distribution, cost, platform, and security lifecycles. | The Companion records exact identity and invokes them; it neither embeds nor republishes them. |
| Project-local `opi-eval` skill | Extension workflow | It remains an explicit Opi provider-fidelity workflow, not the Agent-neutral product owner. | Removing the skill does not remove Companion operation or evidence interpretation. |

The implementation hypothesis is one unpublished Rust workspace package with a
library and CLI, provisionally located at `crates/opi-eval` and marked
`publish = false`. The package name and path support implementation and tests;
they do not decide the final product name, repository, brand, or public API.
The library may expose only the minimum provisional Rust entry seam needed by
its same-package CLI and integration tests. That Rust-visible entry remains
unpublished, is not a stable SDK or compatibility promise, and does not make
the package's internal experiment, adapter, bundle, or report types public.
The package has no dependency on any `opi-*` crate. No existing workspace crate
depends on it, and it is not added to `[workspace.dependencies]` without a real
consumer.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-PLC-001 | The Eval prototype **MUST** build and test without depending on any Opi crate or linking Opi/pi runtime code. | Workspace and Evaluation product owners | `cargo tree`/metadata dependency audit and process-only integration review. |
| P18-PLC-002 | No existing Opi crate or standard binary **MUST** depend on or activate the Eval prototype. | Workspace maintainers | Reverse-dependency and production-call-site scan. |
| P18-PLC-003 | Agent and benchmark/verifier integrations **MUST** remain replaceable process adapters whose failures cannot fall through to a different Agent, benchmark revision, verifier, or local implementation. | Integration owners | Selected-adapter failure and no-fallback tests. |
| P18-PLC-004 | Benchmark datasets, native grader implementations, images, sandboxes, schedulers, exporters, and leaderboards **MUST NOT** be packaged in the Eval module. | Evaluation product owner | Package-content, dependency, and artifact-size review. |
| P18-PLC-005 | The existing project-local `opi-eval` skill **MUST NOT** own, register, or become the only executable interface to the Companion. | Workflow and Evaluation product owners | Source ownership and call-graph review. |
| P18-PLC-006 | Publication, extraction, branding, or a stable third-party API **MUST** require a post-conformance Placement Review and **MUST NOT** be implied by Phase exit. | Product maintainers | Manifest publication flags and Phase-exit wording review. |

## End-to-end control model

```text
Human-reviewed benchmark revision + task set
                    │
                    ▼
resolved experiment manifest
  ├── exact Agent identities and roles
  ├── exact model/control/environment projection
  ├── task/trial pair identities
  ├── exact grader and benchmark identities
  ├── budgets, timeout, cancellation, and capture policy
  └── integrity and exclusion policy
                    │
                    ▼
durable trial intent
  ├── Opi process adapter ──► native NDJSON + Phase 17 evidence
  └── pi process adapter  ──► native JSON events + product facts
                    │
                    ▼
settled Agent record + final-workspace artifact
                    │
                    ▼
native grader adapter
  ├── Terminal-Bench 2.1 task package / verifier
  ├── Terminal-Bench 3.0 task package / separate verifier
  └── DeepSWE v1.1 task package / pristine verifier
                    │
                    ▼
sealed content-addressed RunBundle
                    │
                    ├──► offline native regrade
                    └──► offline paired outcome-first report
```

The resolved experiment is immutable before the first Agent process starts.
Product-level comparison preserves each Agent's own system prompt, tool
schemas, loop, compaction, retries, and tool implementations. Shared controls
cover the benchmark task, provider/model identity, permitted sampling values,
resource limits, environment class, network policy, timeout, trial count, and
grader. Product scaffold identity remains different and is recorded rather
than forced equal.

## Provisional package and seam discipline

The package uses internal interfaces to make the Phase executable, but no type
or command name is a durable public promise until the complete Phase 18
integration matrix proves it. The expected provisional CLI operations are:

```text
opi-eval validate <experiment>
opi-eval run <experiment>
opi-eval regrade <sealed-bundle>
opi-eval report <sealed-bundle>
opi-eval conformance <profile>
```

The implementation may rename internal traits, envelopes, or modules while the
Phase is active. Shared interfaces use semantic execution roles; `Adapter` is
reserved for the concrete Opi, pi, and benchmark-revision implementations that
satisfy those interfaces. Phase exit records an artifact-derived seam-evidence
matrix with three sets:

- fields and behaviors required by the two real Agent harness integrations and
  all three benchmark revisions;
- adapter-private fields required by only one product or benchmark revision; and
- rejected or still-provisional hypotheses.

`AgentAdapter`, `BenchmarkAdapter`, Rust trait objects, a JSON process envelope,
ATIF, the span graph, and the directory layout are descriptive hypotheses in
this document. The admitted result may prove that the stable boundary is a
smaller process envelope, a CLI convention, only schemas and conformance
fixtures, or a combination.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-SEAM-001 | Before the complete Phase 18 integration matrix passes, every package, trait, process, trajectory, span, and directory-layout choice **MUST** be marked provisional and unpublished. | Interface owner | Rustdoc/README/manifest wording and publication-flag review. |
| P18-SEAM-002 | Shared conformance **MUST** test observable behavior and artifact meaning rather than require Opi and pi to expose identical internal events, sessions, prompts, or tools. | Conformance owner | Cross-adapter fixture and expectation review. |
| P18-SEAM-003 | After the required native smoke completes, Phase exit **MUST** derive the final matrix from that artifact and retain only fields and behaviors proved necessary by both real Agent integrations or by all three benchmark-revision integrations spanning at least two independently owned native-verifier contracts; adapter-private facts **MUST** remain namespaced or native artifacts. | Placement Review participants | Artifact-derived seam-evidence matrix and schema-diff review. |
| P18-SEAM-004 | A missing shared field **MUST** remain `unknown` with a typed reason when the owning acceptance criterion permits it; it **MUST NOT** be fabricated from another Agent's vocabulary. | Evidence contract owner | Asymmetric Opi/pi telemetry fixtures. |
| P18-SEAM-005 | Provisional Eval terms **MUST NOT** be added to `docs/CONTEXT.md` as durable domain language merely because the prototype uses them. | Domain maintainers | Glossary diff review and post-conformance placement record. |

## Resolved experiment identity and pairing

One resolved experiment freezes all material controls before execution:

```text
ResolvedExperiment
├── schema identity
├── experiment id and manifest digest
├── benchmark
│   ├── name, revision, dataset reference, and integrity-record digest
│   └── task ids and grader identity
├── comparison set
│   ├── subject ids
│   ├── product, version, source/package digest, adapter digest
│   ├── harness/runtime/native-output identity
│   └── directed edges: baseline subject → candidate subject
├── shared model controls
│   ├── provider, exact model, endpoint class
│   ├── temperature/max-output/reasoning values or explicit omitted/unknown
│   └── fallback prohibited
├── environment
│   ├── platform, architecture, container/image digest, cwd policy
│   ├── projected environment/config/resource identity
│   └── network, tools, timeout, concurrency, and budget
└── trials and pairs
    ├── trial id, subject id, task id, and trial group
    └── edge id, pair id, candidate trial id, and baseline trial id
```

The comparison set supports any positive number of subjects and any explicit
directed comparison graph. Each edge still has exactly one baseline subject and
one candidate subject so it preserves `CTRL-005` pairing semantics. Phase 18
exercises two subjects, Opi and pi, and at least one directed edge; adding
Claude Code, Codex, OpenCode, or another harness later does not require a new
experiment, trial, bundle, or report shape.

The comparison fingerprint includes every control whose difference can change
outcome interpretation. Strict same-model comparison freezes provider,
endpoint class, exact model, reasoning/sampling values, context/output limits,
timeout, resource, environment, and grader controls where the harness exposes
them. Opi/pi product prompts, editing tools, loop, compaction, retry behavior,
and native harness integrations are subject identity, not shared controls. A
required control that one harness cannot express makes that edge
non-comparable; a mismatch inside one pair invalidates that pair rather than
producing a performance comparison.

Resolved experiment identity owns pre-dispatch schema, subject, edge, and
control resolution. Pair completeness, control mismatch, and comparability are
owned separately after trial facts exist. The pairing path consumes the frozen
resolved identity and cannot mutate or silently reinterpret it; reports consume
the pairing result rather than recreating either decision.

Every real execution has a unique trial identity. Regrading and rendering are
new derived artifact identities over the same sealed trial; they are not new
Agent trials. Re-executing after failure, interruption, or user request creates
a new trial and a new pair group. No path overwrites or quietly replaces the
prior trial.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-EXP-001 | The resolved experiment and all material control identities **MUST** be frozen and digest-addressed before the first Agent process starts. | Evaluation orchestrator | Canonicalization, pre-dispatch write-order, and mutation tests. |
| P18-EXP-002 | Every reported comparison **MUST** contain exactly one candidate and one baseline trial for the same benchmark revision, task, trial group, and control fingerprint. | Pairing owner | Missing, duplicate, mismatched, and valid-pair fixtures. |
| P18-EXP-003 | Product prompt, tool, loop, compaction, retry, and harness identity **MUST** remain attributable to the evaluated Agent and **MUST NOT** be silently normalized into a synthetic common scaffold. | Agent integration owner | Resolved-manifest and adapter-argv/config snapshot tests. |
| P18-EXP-004 | Provider, exact model, sampling/reasoning, timeout, resource, environment, or grader control mismatch **MUST** invalidate comparison before a paired claim is rendered. | Evaluation orchestrator | Control-fingerprint negative matrix. |
| P18-EXP-005 | Every real execution **MUST** receive a fresh trial identity; retry, resume, replacement, and re-run paths **MUST NOT** reuse an earlier trial identity. | Evaluation orchestrator | Crash/retry/resume identity tests. |
| P18-EXP-006 | Missing pairs and incomplete control coverage **MUST** remain visible in coverage and **MUST NOT** be removed from the denominator silently. | Report owner | Coverage and exclusion report fixtures. |
| P18-EXP-007 | The experiment contract **MUST** support N identified harness subjects and explicit directed comparison edges without hard-coding Opi/pi, while Phase 18 **MUST** prove that contract with real Opi and pi adapters. | Experiment contract owner | Three-subject schema fixture plus two-subject real-process conformance. |
| P18-EXP-008 | A subject that cannot express a required shared model control **MUST** be marked non-comparable on the affected edge and **MUST NOT** contribute to a strict same-model claim. | Pairing owner | Unsupported reasoning/sampling/context control fixtures. |

## Trial durability and effect uncertainty

The Eval runner applies the useful Harness v2 intent/settlement distinction at
its own process boundary without adopting pi's runtime implementation:

```text
planned
  → intent-published
  → process-effect-pending
  → settled
  → sealed
  → graded
  → reported
```

`Intent-published` reserves the trial, pair, artifact, and expected output
identities before process launch. `Process-effect-pending` means the Agent may
have consumed provider credits or changed its isolated workspace. `Settled`
records the observed exit/cancellation/timeout, native logs, final workspace,
and any finalized native evidence. `Sealed` makes the artifact set immutable.

A crash after durable intent but before settlement is `effect-unknown`. The
runner does not infer that the Agent never started, does not mark the trial as
success, and does not reuse its identity. Policy may authorize a replacement
execution as a new paired trial group. The Phase does not claim exactly-once
provider calls, tool effects, grader effects, or external billing.

The Eval module remains single-writer for one bundle. Atomic publication makes
either the staging state or the complete sealed manifest visible. It does not
introduce a general workflow scheduler, shared mutable database, multi-writer
session, or replicated state machine.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-DUR-001 | Trial intent **MUST** durably reserve trial, pair, and artifact identities before the Agent process can start. | Runner owner | Instrumented write/process-start ordering test. |
| P18-DUR-002 | A durable intent without settlement **MUST** be classified as effect-unknown and **MUST NOT** be inferred as not-started, successful, or safely retryable under the same identity. | Runner owner | Crash-at-each-boundary fixtures. |
| P18-DUR-003 | Settlement **MUST** retain the actual observed process outcome, cancellation/timeout owner, partial logs, final-workspace state, and native evidence completeness. | Agent integration owner | Kill, timeout, partial-write, and incomplete-evidence tests. |
| P18-DUR-004 | Sealing **MUST** publish one complete immutable manifest atomically; a partial artifact set **MUST NOT** appear as sealed. | Bundle owner | Torn-write, failed-rename, and reopen validation tests. |
| P18-DUR-005 | Phase 18 **MUST NOT** claim exactly-once external effects, provider-stream resumption, multi-writer bundles, replication, or a durable scheduling service. | Evaluation product owner | Public documentation and source-surface review. |

## Agent harness process integrations

### Shared semantic contract

The provisional Agent integration receives a frozen trial intent and produces
one settled Agent record. The semantic boundary covers:

- exact adapter and Agent identity plus capability declaration;
- bounded prompt/task input and isolated workspace;
- explicit argv, cwd, allowed environment projection, configuration, and
  resource identity;
- timeout, cancellation, process-tree cleanup, and output-size limits;
- native stdout, stderr, session/evidence artifacts, and exit status;
- authoritative terminal/completion predicate;
- final-workspace artifact or patch;
- usage, call, retry, compaction, and timing facts with measurement origin and
  unknown reason; and
- typed Agent, adapter, configuration, provider, cancellation, timeout, and
  evidence failures.

The adapter may expose capabilities such as tool events, evidence manifest,
session identity, compaction, or cost. A missing capability remains visible;
the shared contract does not require every Agent to implement pi Harness v2 or
Opi's Phase 17 evidence vocabulary.

### Opi integration

The Opi adapter launches the real built `opi` binary in one-shot JSON mode with
explicit `--trace` capture. Each trial receives:

- a fresh task workspace and fresh trace root;
- isolated home, application-data, config, and session directories;
- explicit `--no-trust` plus explicit benchmark configuration;
- an exact `provider:model` selection and no fallback;
- a reviewed tool profile and mutating opt-in only where the task requires it;
  and
- a deterministic local provider for default conformance and native smoke, so
  repository tests consume no paid provider or credential.

The adapter validates the NDJSON schema and requires exactly one completed
trace child in the fresh trace root. It copies `evidence.jsonl` and the
finalized `manifest.json` as native artifacts, validates completeness
fail-closed, and associates them with the trial. Final workspace capture is
owned by the Companion; Opi's product manifest is not misrepresented as a
benchmark RunBundle.

Opi's current evidence records are serialize-oriented and do not define a
public reader. The importer is private to the provisional adapter and accepts
only the exact schema identities covered by fixtures. Unknown required Opi
evidence fails the Opi integration; an optional unsupported fact becomes
unknown only when the conformance contract declares it non-authoritative.

### pi integration

The pi adapter launches the pinned earendil coding-agent process rather than
the incomplete Harness v2 scaffold. It records the exact package version,
source revision, package integrity, executable digest, and adapter digest. The
one-shot integration uses pi's JSON output with:

- an isolated `PI_CODING_AGENT_DIR` and home/application-data projection;
- no persistent product session;
- explicit provider/model and thinking/sampling controls;
- explicit project/resource/trust behavior;
- a reviewed tool profile; and
- the same deterministic local provider used by Opi conformance.

The adapter treats pi's documented terminal message and Agent-end events plus
process exit as native product facts. It captures native JSON events,
stdout/stderr, final workspace, and exact configuration identity. pi lacks an
Opi-equivalent finalized evidence manifest, so absent fine-grained correlation,
usage, or cost remains unknown with source-specific reasons. Harness v2 target
events, lanes, operation registers, or settlement claims are not fabricated
from the scaffold.

### Shared conformance

Both real Agent processes run the same conformance scenarios with their own
product prompts and tools. Default automated scenarios use a local scripted
model endpoint and isolated temporary workspaces. Saved fixtures may test
parsing and negative paths, but mock executables alone cannot satisfy the real
integration criterion.

The shared contract is intentionally harness-neutral. Claude Code and Codex are
the next named admission candidates because they expose real coding-agent
processes used in the target comparison workflow; OpenCode and other harnesses
may follow the same gate. They are roadmap entries, not Phase 18 deliverables.
A later adapter must pass the same process, isolation, identity, failure,
workspace, unknown-telemetry, and same-model-control conformance without adding
a harness-specific branch to the experiment, bundle, or report core.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-AGT-001 | Opi and pi **MUST** each be invoked as a real external process with exact version, executable/package, adapter, argv, cwd, environment, and configuration identity. | Agent integration owners | Real-binary/package conformance and resolved-manifest snapshots. |
| P18-AGT-002 | Both Agent integrations **MUST** pass one shared conformance suite covering success, Agent failure, invalid output, timeout, cancellation, bounded output, final workspace, and unknown telemetry. | Conformance owner | Shared test binary against both real adapters. |
| P18-AGT-003 | Agent adapters **MUST** fail closed on an unsupported required native schema or missing authoritative completion predicate and **MUST NOT** fall back to another Agent or parser. | Agent integration owners | Schema-version, missing-terminal, and selected-adapter failure tests. |
| P18-AGT-004 | Opi conformance **MUST** validate complete Phase 17 evidence when capture is configured; incomplete or missing finalized evidence **MUST NOT** be represented as complete. | Opi adapter owner | Fresh-trace-root, corrupt/missing manifest, and completeness tests. |
| P18-AGT-005 | pi conformance **MUST** use the current coding-agent process and **MUST NOT** claim operational Harness v2, durable lane, or settlement semantics absent from v0.84.3. | pi adapter owner | Adapter source/argv review and native-output fixture assertions. |
| P18-AGT-006 | Default tests and required Phase-exit conformance **MUST NOT** call a paid/live provider, require provider credentials, or load user-global/project resources. | Test owner | Local scripted-provider, environment-isolation, and secret-free CI review. |
| P18-AGT-007 | Agent-specific telemetry outside the proved common projection **MUST** remain in namespaced native artifacts and **MUST NOT** be dropped merely to make the Agents look identical. | Evidence contract owner | Asymmetric native-artifact and normalized-projection tests. |
| P18-AGT-008 | A future Claude Code, Codex, OpenCode, or other harness adapter **MUST** pass the same harness conformance and **MUST NOT** require a harness-specific experiment, bundle, pairing, or report format. | Future adapter owner | Post-Phase admission suite and schema-diff review. |
| P18-AGT-009 | Phase 18 **MUST NOT** represent the named future harness adapters as implemented, supported, or required for Phase exit. | Phase lead | Acceptance and public-surface review. |

## Benchmark task-package and native-verifier integrations

Phase 18 is dataset-first. It does not integrate the Harbor Hub product,
leaderboard, hosted upload, authentication, dashboard, or a mandatory cloud
sandbox. It resolves complete pinned official task packages and executes their
native task/verifier contracts through provisional local adapters.

The three required revision integrations are:

1. [Terminal-Bench 2.1](https://github.com/harbor-framework/terminal-bench-2-1),
   using a pinned official task revision and Harbor task/verifier contract;
2. [Terminal-Bench 3.0](https://github.com/harbor-framework/terminal-bench-3),
   using a pinned tagged dataset revision, official task environment, declared
   artifacts, and separate verifier container; and
3. [DeepSWE v1.1](https://github.com/datacurve-ai/deep-swe), using a pinned
   v1.1 task revision, patch/commit collection, and pristine separate-verifier
   grading through the admitted Pier/Harbor-compatible native path.

“Dataset” means the complete executable task package, not only prompts, task
ids, or expected answers. For every admitted task it includes the instruction,
task manifest, environment/image or compose inputs, resource and timeout
declarations, allowed artifact/patch collection, native tests/verifier image,
and required task metadata. Oracle/reference solutions are used only for
integrity preflight and are never exposed to the evaluated Agent environment.

The Companion may invoke an exact pinned Harbor or Pier executable as an
external task-protocol implementation. That use does not grant hosted service
authority and does not make Harbor/Pier the Eval product owner. Implementing a
second partial task runner is not a Phase goal; the implementation plan selects
the smallest pinned native execution path that preserves each revision's
official isolation, artifact, and verifier semantics.

A provisional benchmark adapter resolves the task package and external
tool/image, validates exact identity, supplies the isolated Agent output,
supervises native verification, and captures:

- benchmark name, admitted revision, task-package digest, and task id;
- environment, artifact-collection, native verifier, executable/image, and
  adapter identities;
- exact input/output artifacts and environment identity;
- structured invocation, bounded stdout/stderr, exit status, timeout, and
  cancellation;
- raw native verifier output and its digest;
- parsed native metrics without renaming their authority; and
- task, verifier, adapter, and infrastructure failure classification.

Shared benchmark conformance uses upstream-native oracle/gold preflight plus
negative fixtures. The Linux native-smoke profile invokes every pinned real
task/verifier path. Recorded verifier output alone covers parser determinism but
cannot replace actual task-package/native-verifier smoke.

The assembled Phase smoke uses at least one pre-registered task from each of
the three revisions and one strictly paired Opi/pi trial group for each task.
It is labelled conformance evidence, not a public benchmark score, Z.ai score
reproduction, or performance season. Official leaderboard comparison requires
the separately admitted rollout count, aggregation, model parameters, resource
policy, and submission rules; Phase smoke does not imply them.

For Phase 18 smoke success, every selected upstream oracle preflight must pass,
and every Opi/pi native verifier must run to completion with authoritative
output and complete provenance. An Opi/pi native reward of zero is a valid
integration result under that contract, but it is not Agent task success and
must remain zero in the native and normalized evidence. Phase 18 does not
require reward one from the scripted Agents and therefore does not admit a
task-specific solution or leak oracle material into their provider responses.

Using Harbor-compatible task transport for all three revisions does not reduce
the parent `CTRL-007` gate. The selected tasks must collectively exercise at
least two independently owned native-verifier contracts—for example, an
admitted Terminal-Bench task verifier and an admitted DeepSWE functional
verifier—even when one pinned external runner supervises both. A single fake
grader, shared parser fixture, or Eval-authored scoring implementation presented
under three revision labels does not satisfy the gate.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-BMK-001 | Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1 **MUST** each resolve a complete pinned official task package and invoke its native verifier semantics. | Benchmark integration owners | Three native task/oracle/Agent smoke profiles and provenance assertions. |
| P18-BMK-002 | Prompt text, task ids, expected answers, or recorded scores alone **MUST NOT** satisfy benchmark integration; environment, resources, artifacts, verifier, and task metadata **MUST** be present and validated. | Benchmark integrity owner | Incomplete-package negative matrix. |
| P18-BMK-003 | Terminal-Bench 3.0 and DeepSWE v1.1 **MUST** grade only declared artifacts or patches in their required fresh verifier environment; hidden tests, oracle material, and reference solutions **MUST NOT** enter the Agent environment. | Benchmark integration owners | Canary-oracle, artifact-boundary, and separate-container tests. |
| P18-BMK-004 | Harbor Hub login, upload, public submission, leaderboard access, dashboard access, and a cloud sandbox **MUST NOT** be required for local Phase 18 evaluation. | Evaluation product owner | Offline/local smoke and network-call audit. |
| P18-BMK-005 | All three revision integrations **MUST** pass one shared conformance suite covering identity, task-package closure, bounded invocation, native output, parse failure, non-zero exit, timeout, cancellation, isolation, and immutable result capture. | Conformance owner | Shared benchmark conformance against all three revisions. |
| P18-BMK-006 | A selected task runner or verifier failure **MUST NOT** fall back to another revision, grader, cached score, heuristic, or LLM judgment. | Benchmark integration owners | Selected-verifier failure and no-fallback tests. |
| P18-BMK-007 | Native metrics **MUST** retain their benchmark-defined names, values, denominator, rollout/aggregation method, and provenance; the report **MUST NOT** normalize different benchmarks into one composite score. | Report owner | Native-result fixtures and report-schema tests. |
| P18-BMK-008 | Task packages, verifier implementations, images, oracles, and sandbox material **MUST** remain externally resolved artifacts rather than Eval package content. | Evaluation product owner | Package-content and resolved-artifact review. |
| P18-BMK-009 | A recorded-output parser fixture **MUST NOT** be represented as proof that a real official task package or native verifier was executed. | Phase lead | Native-smoke artifact and Phase-exit evidence review. |
| P18-BMK-010 | A Phase 18 conformance smoke **MUST NOT** be labelled leaderboard-compatible unless it separately satisfies and records the official revision's complete rollout, aggregation, parameter, resource, and submission protocol. | Phase lead | Report classification and official-protocol diff. |
| P18-BMK-011 | The three required revision integrations **MUST** collectively exercise at least two independently owned native-verifier contracts and **MUST NOT** satisfy `CTRL-007` by relabelling one fake, fixture-only, or Eval-authored grader. | Phase lead and benchmark integration owners | Native-verifier ownership/provenance matrix and real-smoke artifacts. |

## Post-Phase Eval coverage roadmap

This section records the intended Eval coverage destination without adding the
later entries to Phase 18 exit. The catalog is the comparison table and methods
published in the
[GLM-5.3 release notes](https://z.ai/blog/glm-5.3), observed on 2026-08-26.
It contains 16 named benchmark entries across Coding, Cyber, and Agentic work,
plus the separately described private Z.ai Code Bench.

| Coverage wave | GLM-5.3 benchmark entries | Planning status and admission boundary |
|---|---|---|
| Phase 18 coding foundation | Terminal Bench 2.1; Terminal Bench 3.0; DeepSWE v1.1 | Required now as complete pinned task-package/native-verifier integrations. The Eval varies Agent harness under one model rather than copying Z.ai's fixed Claude Code or mini-swe-agent subject. |
| Remaining coding | NL2Repo; ProgramBench (Almost Solved); FrontierSWE; SWE-Marathon v1.1; PostTrainBench | Named post-Phase candidates. Each needs separately shaped task access, integrity, native-grader, anti-cheat, rollout, and aggregation contracts. |
| Cyber | CyberGym; ExploitGym (2h / 6h); ExploitBench | Named post-Phase candidates. Admission additionally requires explicit high-risk environment, network allowlist, resource-budget, exploit-artifact, and Human Authority review. |
| Agentic | Toolathlon Verified; AutomationBench v1.0.6; Agents' Last Exam (ALE-CLI); HLE w/ Tools; GDPval-AA v2 | Named post-Phase candidates. Official-service and judge-mediated methods require exact service/judge identity, evidence export, replay limits, and authority shaping before headline use. |
| Private evidence | Z.ai Code Bench | Not admitted. It remains a coverage reference until the tasks, grader/checklists, access terms, revision identity, and reproducible evidence path are available to Opi Eval. |

The roadmap separates four identities that published model tables often bind
together:

- the exact model and provider controls;
- the Agent harness, including system prompt, tools, loop, and compaction;
- the benchmark task/environment/verifier protocol; and
- the rollout, aggregation, exclusion, and publication method.

Opi Eval's target experiment fixes the first and third/fourth identities and
varies the second. A Z.ai score instead reflects the exact harness and method
reported by Z.ai. The two are different experiment classes even when they use
the same model and benchmark. A report can compare them only after proving the
full published method projection; shared benchmark names alone are
insufficient.

Some later entries use an official hosted service or an official LLM judge.
Such a benchmark is not rejected solely for that reason, but Phase 18 does not
pre-admit it. A future shaping decision must establish whether the exact
benchmark-owned service or judge satisfies `CTRL-004`, how its version, prompt,
model, sampling, inputs, outputs, and availability are pinned, and what can be
recomputed offline. An Eval-authored substitute, cached score, or silent
failure fallback never becomes the native authority.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-RDM-001 | The Phase 18 planning record **MUST** retain all 16 named GLM-5.3 comparison-table benchmark entries, the private Z.ai Code Bench status, their coverage wave, and their admission boundary. | Evaluation product owner | Catalog completeness review against the dated official release evidence. |
| P18-RDM-002 | Experiment, harness, bundle, benchmark-result, and report shapes **MUST NOT** hard-code two Agents or the three Phase 18 benchmark revisions; later admitted adapters **MUST** fit the same core shapes or trigger explicit reshaping. | Interface owner | Three-subject and fourth-benchmark schema fixtures plus schema-diff review. |
| P18-RDM-003 | A post-Phase benchmark entry **MUST NOT** be represented as admitted until its exact task access, revision, environment, native authority, integrity, rollout, aggregation, and evidence contracts pass a separately approved gate. | Future benchmark owner | Post-Phase admission record and conformance suite. |
| P18-RDM-004 | A hosted-service or LLM-judge benchmark **MUST** require explicit authority and reproducibility shaping; an Eval-authored replacement, cached result, heuristic, or silent fallback **MUST NOT** stand in for the benchmark-owned method. | Human Authority and future benchmark owner | Source-ownership, judge/service identity, and failure-path review. |
| P18-RDM-005 | Z.ai Code Bench **MUST** remain not-admitted while its private tasks, grader/checklists, access terms, revision identity, or evidence path cannot satisfy the ordinary benchmark gate. | Benchmark integrity owner | Access and reproducibility review. |
| P18-RDM-006 | Same-model cross-harness reports **MUST** distinguish their experiment class from Z.ai's published cross-model/fixed-harness methods and **MUST NOT** claim score reproduction from a shared benchmark name alone. | Report owner | Method-projection diff and report wording tests. |

## Benchmark revision integrity

Native graders own outcome semantics, not benchmark integrity. Phase 18
introduces a small immutable integrity record for each measured revision. It
binds:

- benchmark and dataset revision;
- native grader and environment identity;
- upstream task identity and source digest/reference;
- oracle/gold preflight result where provided upstream;
- admitted, retired, or not-admitted revision status;
- per-task validity classification and reviewed reason; and
- every excluded or infrastructure-invalid trial.

Per-task classifications distinguish:

- valid Agent outcome;
- broken or unsatisfiable task;
- ambiguous requirement;
- prompt/test mismatch;
- infrastructure failure; and
- grader failure.

Only the first class enters Agent success/failure. A human-reviewed integrity
record, upstream oracle, or deterministic native preflight supplies
adjudication; the evaluated Agent and an LLM diagnostic cannot admit their own
task, alter the grader, or hide an exclusion.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-INT-001 | A benchmark revision **MUST** be admitted by an immutable integrity record before its native outcome can appear as a headline result. | Benchmark integrity owner | Admission-state and missing-record tests. |
| P18-INT-002 | Task validity, exclusion, infrastructure failure, and grader failure **MUST** remain distinct from Agent success/failure and visible in coverage. | Benchmark integrity and report owners | Failure-classification and denominator fixtures. |
| P18-INT-003 | The evaluated Agent, its adapter, and any LLM diagnostic **MUST NOT** admit or retire a benchmark revision, alter a task classification, or choose the headline grader. | Human Authority and integrity owner | Negative authorization and source-ownership review. |
| P18-INT-004 | Reclassifying a task or benchmark revision **MUST** create a new integrity-record identity and **MUST NOT** rewrite an older bundle or report. | Benchmark integrity owner | Reclassification immutability tests. |
| P18-INT-005 | Every exclusion and missing trial **MUST** retain a stable reason and remain traceable from report coverage to its source record. | Report owner | Bidirectional coverage/provenance tests. |

## Content-addressed RunBundle

The bundle is a sealed artifact graph, not a shared mutable database. Its
semantic contents include:

- resolved experiment and integrity-record digests;
- pair, task, trial, Agent, adapter, model-control, environment, and budget
  identity;
- native Agent stdout/stderr/events/session/evidence artifacts;
- the bounded final workspace snapshot or patch and its manifest;
- the validated common trajectory hypothesis and causal span hypothesis;
- process outcomes, failure ownership, cancellation, timeout, and
  effect-unknown state;
- all measurements with origin, coverage, and unknown reason;
- native grader input, output, logs, identity, metrics, and provenance; and
- a content-addressed artifact manifest covering every retained byte.

Raw artifacts use normalized workspace-relative logical paths and immutable
digests. Machine-local absolute paths are either excluded or stored only in a
classified local-only native artifact. The bundle manifest never follows
symlinks outside the staging root. Sealing validates every digest, required
artifact, media/sensitivity classification, path, size bound, and relationship
before atomic publication.

Regrading writes a new derived grade artifact addressed by bundle, grader, and
grader-adapter identity. Rendering writes a new derived report artifact.
Neither operation edits the sealed bundle or an earlier derived result.

`opi-eval report` recomputes normalized comparison and coverage views from the
sealed bundle and the selected immutable derived grades before rendering. Phase
18 does not add a separate public `recompute` command; recomputation remains an
effect-free internal step of the report path and is verified directly.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-BND-001 | Every sealed bundle **MUST** be content-addressed and **MUST** cover every retained input, native artifact, final-workspace artifact, trajectory/span artifact, and grader artifact in one validated manifest. | Bundle owner | Manifest closure and unreferenced/missing artifact tests. |
| P18-BND-002 | Bundle sealing **MUST** reject digest mismatch, path escape, symlink escape, oversized artifact, unknown required media, missing classification, or missing required provenance. | Bundle owner | Adversarial filesystem and schema fixtures. |
| P18-BND-003 | Mutation of any covered byte after sealing **MUST** invalidate verification; verification **MUST NOT** repair or silently rehash the bundle. | Evidence Producer | Post-seal mutation tests. |
| P18-BND-004 | Regrade and report operations **MUST** be append-only derived artifacts and **MUST NOT** modify the sealed bundle or an older grade/report. | Grader and report owners | Before/after tree digest tests. |
| P18-BND-005 | Missing measurements **MUST** retain `unknown` plus a typed reason; measured zero, provider-reported, estimated, quota, and billed values **MUST** remain distinguishable. | Evidence contract owner | Serialization and report coverage fixtures. |
| P18-BND-006 | Raw credentials, unrestricted environment values, private raw reasoning, and unclassified prompt/tool/file content **MUST NOT** enter a sealed exportable bundle. | Privacy owner | Canary-secret and classified-content boundary tests. |

## Trajectory and causal-span hypotheses

Phase 18 preserves each Agent's native output first. It then tests two derived
representations:

- a validated ATIF trajectory candidate for common externally observable Agent
  activity; and
- a supplemental causal span graph for process, turn, LLM, tool, retry,
  compaction, grader, and artifact relationships.

The derived trajectory does not require private raw reasoning. Tool arguments,
results, prompts, files, and provider payloads cross only under explicit
capture/classification policy. Stable identities, parent relationships,
terminal facts, native references, timing source, and missing-data reason are
more important than copying an Agent's event names.

Opi can project Phase 17 run/turn/call evidence. Current pi coding-agent output
provides a different event/session vocabulary and lacks the complete Harness v2
target. The pi adapter derives only facts supported by current native output.
Harness v2's intent/effect/settlement distinctions guide the common failure
vocabulary, but no adapter invents durable state that the current product does
not expose.

Parallel calls are represented through parent/causal edges and per-source
ordering. A report computes critical path only when correlation and timing
coverage are sufficient; it does not force all events into a fabricated global
order.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-TRJ-001 | Every normalized trajectory or span fact **MUST** retain a reference to the native artifact and adapter rule that produced it. | Trajectory owner | Projection provenance and round-trip lookup tests. |
| P18-TRJ-002 | An adapter **MUST NOT** invent Harness v2 lanes, operation states, durability, correlation, usage, or timing absent from current native evidence. | Agent integration owners | Sparse pi fixture and unsupported-fact tests. |
| P18-TRJ-003 | Parallel activity **MUST** preserve causal/source ordering and **MUST NOT** be presented as a trustworthy global total order without evidence. | Trajectory owner | Parallel-call and ambiguous-order fixtures. |
| P18-TRJ-004 | Critical-path, time-to-first-token, compaction, retry, and failed-call consumption claims **MUST** expose measurement coverage and become unknown when required inputs are absent. | Report owner | Partial-telemetry report fixtures. |
| P18-TRJ-005 | ATIF, the span graph, and their canonical/supplemental/derived relationship **MUST** remain provisional until the complete conformance matrix passes. | Interface owner | Schema status and seam-evidence review. |

## Failure, cancellation, and classification

Failures are owned at the narrowest boundary:

| Boundary | Required distinguishable classes |
|---|---|
| Experiment | Invalid schema, unresolved identity, control mismatch, budget rejection, unsupported capability. |
| Trial durability | Not started, effect unknown, settlement failure, sealing failure, post-seal mutation. |
| Agent process | Configuration/auth/provider failure, Agent crash, invalid native output, timeout, cancellation, process cleanup unknown. |
| Adapter | Unsupported required schema, parse failure, missing terminal predicate, bounded-output violation. |
| Evidence | Missing/incomplete native evidence, redaction failure, artifact validation failure, unknown measurement. |
| Integrity | Revision not admitted, invalid/ambiguous/broken task, prompt/test mismatch, exclusion conflict. |
| Grader | Resolution failure, native grader non-zero exit, invalid output, timeout, cancellation, provenance mismatch. |
| Infrastructure | Container/image/tool acquisition, host resource failure, shared provider outage, orchestration failure outside the Agent. |
| Pair/report | Missing/duplicate pair, control mismatch, incomplete coverage, offline recomputation mismatch. |

Timeout ownership depends on the enforcing boundary: a valid task whose Agent
exceeds its pre-registered task limit is an Agent outcome; failure to provision
or supervise the promised environment is infrastructure. Cancellation is not
success. A user cancellation and an infrastructure abort retain distinct
sources.

Outer retries never run until success. Only a pre-registered infrastructure
policy may schedule replacement work, and it creates a new Opi/pi pair with
new trial identities. An Agent failure remains scored under the native grader
and is never reclassified as infrastructure merely to protect a result.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-FAL-001 | Every failure class above **MUST** remain distinguishable in the bundle and report through a stable owning-boundary code. | Boundary owners | Exhaustive mapping and fixture tests. |
| P18-FAL-002 | A boundary failure **MUST** stop all later authority transitions and **MUST NOT** be converted into success, zero, a native grader pass, or a silent exclusion. | Evaluation orchestrator | Per-boundary downstream call-count tests. |
| P18-FAL-003 | Invalid/broken tasks, infrastructure failures, and grader failures **MUST NOT** enter Agent success/failure scoring, while Agent crashes and Agent-owned timeouts on valid tasks **MUST** remain Agent outcomes. | Benchmark integrity owner | Classification/denominator matrix. |
| P18-FAL-004 | Cancellation, timeout, partial workspace mutation, effect unknown, and process-tree cleanup uncertainty **MUST** remain visible and **MUST NOT** imply safe replay. | Runner owner | Race and process-tree failure injection. |
| P18-FAL-005 | Any permitted infrastructure replacement **MUST** replace the complete Opi/pi pair with new identities and retain the original pair in coverage. | Pairing owner | Replacement-pair and history fixtures. |

## Offline recomputation and outcome-first reporting

Offline operations consume only sealed bundles, exact grader/runtime artifacts
already resolved by the bundle, and derived artifacts addressed by digest.
They do not call an Agent or provider.

Regrade re-invokes the benchmark-native grader over saved final output when the
required pinned grader environment is available. Recompute validates integrity,
pairing, coverage, metrics, and derived call/efficiency facts. Render produces
a deterministic normalized report. Volatile generation timestamps live in an
outer artifact envelope and do not change the normalized result.

The report separates:

- benchmark-native headline outcome and denominator;
- pair coverage, exclusions, integrity decisions, and every missing pair;
- Agent, adapter, grader, and infrastructure failures;
- wall and critical-path time plus time-to-first-token when supported;
- LLM/tool/compaction/retry counts and latency;
- input/output/cache/reasoning tokens;
- provider-reported, estimated, quota, and billed cost with coverage;
- compression and failed-call consumption; and
- native versus normalized artifact provenance.

Different dimensions never compensate for one another. Terminal-Bench 2.1,
Terminal-Bench 3.0, and DeepSWE v1.1 retain separate revision-native metrics
and aggregation identities. The Phase smoke report states `conformance-only`
and makes no population, significance, release, Z.ai-score reproduction, or
leaderboard claim.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-RPT-001 | Offline regrade/recompute/render **MUST NOT** execute an Agent, call a provider, or mutate a sealed bundle. | Report owner | Process-call spy and tree-digest tests. |
| P18-RPT-002 | The same sealed bundle, integrity record, grader identity, and reporter version **MUST** produce byte-stable normalized results. | Report owner | Repeat-run deterministic fixture. |
| P18-RPT-003 | Headline outcomes **MUST** come only from admitted benchmark-native grader artifacts; diagnostics or inferred attribution **MUST** remain separately labelled. | Evidence Producer | Report-source provenance tests. |
| P18-RPT-004 | Pair coverage, exclusions, invalid tasks, infrastructure/grader failures, missing telemetry, and unknown values **MUST** remain visible beside native results. | Report owner | Golden report and denominator fixtures. |
| P18-RPT-005 | Quality, cost, safety, efficiency, and authority **MUST NOT** be collapsed into one composite score or best-trial verdict. | Report owner | Schema and snapshot guard. |
| P18-RPT-006 | Phase 18 reports **MUST** label their native smoke as conformance evidence and **MUST NOT** claim official leaderboard verification or Opi superiority. | Phase lead | Report wording and artifact-classification review. |

## Privacy, authority, and supply-chain boundaries

Eval activation is explicit. The Companion receives only the credentials,
network, tools, files, and external commands authorized for the selected
profile. Agent or grader output cannot widen those inputs. Adapter capability
discovery informs validation but does not grant authority.

The deterministic default conformance profile uses no live credential.
Native grader assets, Node/npm packages, Python environments, container images,
and any new Rust dependency are reviewed as code and pinned by version plus
digest/lock material. External commands run with bounded argv and environment;
the Eval module does not shell-interpolate model or task content.

Sensitive capture is classified before export. Raw bundle staging stays in an
explicit private location. Exportable bundles include only reviewed content or
classified artifact references. A redaction/leakage failure blocks sealing and
report publication while retaining only safe diagnostic evidence.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-SEC-001 | Eval activation, credentials, network, mutating tools, external graders, and native-smoke profiles **MUST** be explicit and **MUST NOT** be enabled by ordinary Opi startup. | Human Authority and Evaluation product owner | Default-path and explicit-profile tests. |
| P18-SEC-002 | Agent, task, model, tool, grader, or adapter output **MUST NOT** widen the resolved authority, environment, network, tool, or artifact scope. | Evaluation orchestrator | Malicious-content source-to-sink matrix. |
| P18-SEC-003 | Every new dependency, npm/Python package, container image, native tool, and executable artifact **MUST** be pinned and reviewed with exact integrity/provenance. | Supply-chain owner | Manifest/lock/digest and Cargo.lock review. |
| P18-SEC-004 | External invocation **MUST** use structured argv/environment construction and **MUST NOT** interpolate untrusted content into a shell command. | Integration owners | Source review and metacharacter fixtures. |
| P18-SEC-005 | Redaction or leakage failure **MUST** block bundle sealing and report publication. | Privacy owner | Canary-secret and unsafe-summary tests. |
| P18-SEC-006 | Phase 18 **MUST NOT** claim that process boundaries, Docker, Harbor, or the Eval module provide stronger sandbox guarantees than their selected runtime actually supplies. | Safety owner | Documentation and diagnostic review. |

## Compatibility, migration, and Minimal Runtime

Phase 18 adds no compatibility shim to Opi or pi. It consumes exact current
process and artifact versions through provisional adapters:

- Opi's native NDJSON and Phase 17 evidence stay Opi-owned;
- pi's native JSON output and package/session behavior stay pi-owned;
- the Companion retains native bytes and creates a separate normalized
  projection;
- an unsupported required source version fails with remediation rather than
  best-effort parsing; and
- a new adapter version may import an older sealed bundle only through explicit
  versioned fixture/conformance evidence.

The new workspace package is opt-in and unpublished. Adding it to the workspace
does not register a provider, tool, package, command, extension, startup hook,
or default capture path in `opi`. Existing `docs/eval/` history and the
`opi-eval` skill retain their current meaning and are not silently migrated
into the Companion.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-MIG-001 | Existing Opi/pi sessions, native evidence, local Eval reports, configuration, credentials, and user artifacts **MUST NOT** be rewritten, moved, or silently upgraded by the Companion. | Integration owners | Byte-immutability and filesystem-boundary tests. |
| P18-MIG-002 | Unsupported required native output or evidence **MUST** fail closed with exact source identity and remediation; best-effort parsing **MUST NOT** claim a complete bundle. | Adapter owners | Old/new/unknown schema fixtures. |
| P18-MIG-003 | Native source artifacts **MUST** remain distinguishable from normalized and derived artifacts in every bundle. | Bundle owner | Artifact-role and provenance tests. |
| P18-MIG-004 | The Opi Minimal Runtime **MUST NOT** acquire Eval startup I/O, dependency activation, default capture, provider calls, background work, or changed authority. | Reference Product owner | Production-call-site, default CLI, and I/O acceptance. |
| P18-MIG-005 | The existing `opi-eval` skill and `docs/eval/` schema **MUST NOT** be silently repurposed as the Companion's stable API or history format. | Workflow owner | Documentation and schema ownership review. |
| P18-MIG-006 | Removed provisional interfaces **MUST NOT** be retained behind aliases, feature flags, or dual execution paths solely for Phase-local compatibility. | Workspace maintainers | Public API and source-structure review. |

## Platform scope

Manifest resolution, hashing, bundle validation, conformance fixtures, pairing,
offline recomputation, and report rendering are platform-neutral and run on
Linux, macOS, and Windows.

Real Opi and pi process adapter conformance uses the platforms on which the
required binaries/runtime are supported in repository CI. The required
two-Agent by three-benchmark-revision native-smoke artifact is pinned to Linux
x86_64 because Harbor/Pier, the official task images, Docker isolation, and
their resource assumptions own that environment. Phase 18 makes no
Windows/macOS native-verifier parity claim.

Tests use isolated temporary directories, deterministic local provider
fixtures, fake failure injectors, and saved public/non-sensitive native output.
They do not call paid providers, require live credentials, activate user
resources, or depend on an external leaderboard.

Three-platform pull-request evidence preserves the repository's existing
merge-ref integration semantics. Its receipt records the pull-request head,
the actual checked-out commit for every required job, workflow bytes, job
matrix, and artifact identities. Phase evidence may add an attestation job, but
it does not replace ordinary merge-ref checks with head-only validation.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-PLT-001 | Platform-neutral schemas, hashing, bundle verification, pairing, and offline reports **MUST** behave identically on Linux, macOS, and Windows. | Workspace maintainers | Three-platform workspace CI. |
| P18-PLT-002 | The real two-Agent by three-benchmark-revision native smoke **MUST** run on a pinned Linux x86_64 environment and **MUST** disclose that scope. | Native-smoke owner | CI/job identity, image digest, and report metadata. |
| P18-PLT-003 | Phase 18 **MUST NOT** claim native Terminal-Bench/DeepSWE execution parity on Windows or macOS. | Documentation owner | Platform-claim review. |
| P18-PLT-004 | Automated tests **MUST** use isolated filesystems and local deterministic provider fixtures and **MUST NOT** require paid providers, credentials, or live leaderboard access. | Test owner | Hermetic test source review and CI environment audit. |

## Acceptance scenarios and verification

| ID | Scenario | Observable acceptance |
|---|---|---|
| P18-A01 | Build the Companion with the Opi workspace present but no Opi linkage. | The package builds/tests, `cargo tree` contains no `opi-*` dependency, and no existing product has a reverse dependency. |
| P18-A02 | Run the same bounded workspace task through real Opi and pi processes using the deterministic local provider. | Both adapters pass the same process conformance while retaining distinct product/native identities and final workspaces. |
| P18-A03 | Opi runs with explicit trace capture. | Exactly one completed trace child contains native evidence and a finalized manifest; incomplete/missing/corrupt evidence fails the Opi integration. |
| P18-A04 | pi runs through the current coding-agent JSON path. | Current documented terminal/process facts are captured; unavailable Harness v2/telemetry facts remain unknown and no scaffold behavior is claimed. |
| P18-A05 | An Agent emits an unknown required schema, malformed stream, missing terminal event, or excessive output. | The selected adapter fails closed, no fallback Agent/parser runs, and the owning failure remains visible. |
| P18-A06 | Timeout or cancellation races with workspace mutation and process exit. | Actual partial artifacts and cleanup status are retained; cancellation/timeout is not success and replay safety is not inferred. |
| P18-A07 | The runner crashes after intent and before settlement. | The trial reopens as effect-unknown, retains its identity and artifacts, and any replacement uses a new paired trial group. |
| P18-A08 | Terminal-Bench 2.1 resolves one admitted official task package and grades Opi/pi outputs. | The complete pinned task contract and native verifier run for both harnesses; exact task, environment, verifier, output, and digest provenance is retained. |
| P18-A09 | Terminal-Bench 3.0 resolves one admitted official task package and grades Opi/pi outputs. | Each Agent runs in the official task environment and only declared artifacts reach the separate verifier container; the native metric remains authoritative. |
| P18-A10 | DeepSWE v1.1 resolves one admitted official task package and grades Opi/pi outputs. | The pinned v1.1 task collects the declared patch/commit and grades it in a pristine verifier environment with full native provenance. |
| P18-A11 | A task package contains only prompts/ids, omits an image/resource/verifier contract, or a selected native verifier fails. | Integration fails closed; no cached score, alternate revision, heuristic, or LLM fallback appears, and the owning package/verifier failure remains visible. |
| P18-A12 | One pre-registered task from each required revision has Opi and pi trials. | Every task/trial group has exactly one declared comparison edge under identical shared model controls; the report labels all results conformance-only. |
| P18-A13 | A pair is missing, duplicated, has mismatched model/resource controls, or one harness cannot express a required control. | The edge is incomplete or non-comparable, native facts remain visible, and coverage states the exact reason. |
| P18-A14 | A task is broken, ambiguous, prompt/test-misaligned, or infrastructure-invalid. | The integrity record excludes it from Agent scoring without hiding it from coverage or rewriting prior evidence. |
| P18-A15 | A sealed artifact byte, path, digest, media classification, or symlink target changes. | Bundle verification fails without repair or silent rehash. |
| P18-A16 | Opi and pi expose asymmetric call, usage, cost, retry, or compaction facts. | Native facts remain retained; the common report uses measured values or typed unknowns and never fabricates parity. |
| P18-A17 | Regrade, recompute, and render run repeatedly over the same sealed bundle. | No Agent/provider starts, the bundle is unchanged, and normalized outputs are byte-stable for the same tool identities. |
| P18-A18 | Canary secrets occur in environment, prompts, tool data, provider/verifier errors, and machine-local paths. | Exportable bundle and report contain no raw canary; leakage blocks sealing/publication. |
| P18-A19 | Run ordinary `opi` without Eval configuration before and after the Phase. | CLI behavior, default I/O, evidence capture, User Policy, provider routing, tools, sessions, and background activity are unchanged. |
| P18-A20 | Resolve an experiment schema with three harness subjects and a fourth benchmark descriptor, then inspect the Phase 18 seam-evidence matrix. | The generic shapes accept them without Opi/pi or three-revision hard-coding; unsupported adapters remain unexecuted/provisional, and the matrix covers both real Agents, all three real benchmark revisions, and at least two independently owned native-verifier contracts. |
| P18-A21 | Inspect the GLM-5.3 coverage roadmap. | All 16 comparison-table benchmark entries and the private Z.ai Code Bench are present with implementation/admission status, experiment-class distinction, and future authority gates. |
| P18-A22 | Run the required repository and native-smoke gates. | Local/focused/workspace gates pass, and the pinned Linux artifact proves both real Agents against all three required benchmark revisions and at least two native-verifier contracts without a paid provider. |

The implementation plan derives exact focused commands from the files and test
binaries it creates. At minimum it provides focused checks equivalent to:

```sh
cargo test -p opi-eval --test agent_integration_conformance
cargo test -p opi-eval --test benchmark_integration_conformance
cargo test -p opi-eval --test bundle_recompute
cargo test -p opi-eval --test pairing_and_integrity
cargo tree -p opi-eval --edges normal
cargo run -p opi-eval -- conformance phase18-linux-native
```

The exact native-smoke command, external lock files, image/tool digests, and CI
job are implementation-plan outputs because they are owned by the selected
pinned upstream revisions. Saved fixtures cover deterministic local behavior;
the actual native-smoke command remains separate Phase-exit evidence.

Phase exit additionally requires the repository gates below in order:

```sh
python scripts/opi-doc-check.py
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

If a test file is created or modified, its exact test binary or filter runs
before workspace gates. The implementation handoff records test impact as
`add`, `update`, `delete`, `retain`, or `none`. The Phase snapshot preserves the
native-smoke command, exact identities, artifacts, and results; this document
does not record those completion facts.

## Risk thresholds and rollback

Any of these observations blocks Phase exit:

- one dependency from the Eval package to an Opi crate or one reverse
  dependency/activation from the Opi product;
- one Opi/pi runtime linked in-process rather than invoked through the tested
  process boundary;
- fewer than two real Agent integrations, fewer than all three required
  benchmark-revision integrations, or fewer than two independently owned
  native-verifier contracts passing shared conformance;
- one mock/fixture-only result represented as real integration or native-grader
  evidence;
- one public/canonical package, trait, process, ATIF, span, or directory claim
  made before the complete conformance matrix;
- one selected Agent or grader failure falling back to a different
  implementation;
- one pair, telemetry gap, exclusion, integrity decision, invalid task, or
  infrastructure/grader failure hidden from coverage;
- one invalid/infrastructure/grader trial scored as Agent success/failure, or
  one valid Agent failure reclassified to protect the result;
- one Phase 18 native headline outcome overridden by a heuristic, custom
  grader, LLM, efficiency metric, or composite score;
- one real execution reusing a prior trial identity;
- one effect-unknown interval reported as not-started, safely replayable,
  settled, or successful;
- one sealed bundle mutation accepted, one partial bundle published as sealed,
  or one offline result that cannot be reproduced from saved artifacts;
- one unknown measurement converted to zero or one Agent-specific fact
  fabricated for the other Agent;
- one raw credential, private reasoning value, unclassified sensitive content,
  or unsafe path entering an exportable bundle/report;
- benchmark data, native graders, images, sandboxes, schedulers, exporters, or
  leaderboards entering the Eval module;
- a live/paid provider or credential becoming required by automated tests or
  Phase exit;
- one default Opi behavior, I/O, capture, authority, session, provider, or tool
  change caused by Eval; or
- an Opi AgentHarness, pi Harness v2 compatibility layer, named lane, session
  search, remote protocol, or other broad harness seam introduced to complete
  the Phase.

Rollback removes the complete optional Phase 18 activation and package
integration as one coherent change. It does not selectively leave competing
adapter, bundle, grader, or report paths active. Opi and pi runtime behavior,
configuration, sessions, User Policy, and Minimal Runtime remain unchanged.

Previously sealed bundles, integrity records, native artifacts, and reports
remain byte-identical. Rollback neither deletes nor down-converts them. The
Phase snapshot retains the schema and exact tool identities needed to
distinguish those artifacts even when the live prototype is removed. Rollback
does not silently relabel conformance evidence as a current or official score.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P18-RBK-001 | Any risk threshold above **MUST** block Phase exit until removed or this specification is explicitly revised. | Phase lead | Exit audit against every threshold. |
| P18-RBK-002 | Rollback **MUST** remove one coherent optional Eval path and **MUST NOT** leave dual Agent, grader, bundle, or report implementations active. | Release and Evaluation product owners | Revert review and source/call-graph scan. |
| P18-RBK-003 | Rollback **MUST NOT** modify Opi/pi runtime behavior, User Policy, credentials, sessions, native evidence, or Minimal Runtime defaults. | Reference Product owners | Before/after runtime and filesystem acceptance. |
| P18-RBK-004 | Rollback **MUST NOT** delete, rewrite, down-convert, or relabel sealed bundles, integrity records, native artifacts, or reports. | Evidence owner | Byte-immutability and artifact-classification tests. |
| P18-RBK-005 | If rollback or implementation requires a broader Agent Core or public protocol seam, delivery **MUST** stop for explicit shaping rather than retain a compatibility bridge. | Phase lead | Blocked handoff and route-revision review. |

## Delivery dependency order and implementation-plan handoff

This specification defines workstream dependencies, not ledger tasks or task
status:

```text
provisional package + canonical hashing + resolved experiment
    ├── bundle staging/sealing + trial intent/settlement
    └── benchmark integrity + pairing vocabulary

shared Agent process conformance
    ├── real Opi adapter + private Phase 17 evidence importer
    └── real pi adapter + current coding-agent JSON importer

shared benchmark task-package/native-verifier conformance
    ├── Terminal-Bench 2.1 adapter
    ├── Terminal-Bench 3.0 adapter
    └── DeepSWE v1.1 adapter

Agent conformance + bundle + integrity + benchmark conformance
    └── trajectory/span hypotheses + assembled paired smoke
            └── offline regrade/recompute/report
                    └── Linux native smoke
                            └── artifact-derived final seam-evidence matrix + local acceptance
                                    └── three-platform workspace/CI Phase exit
```

The package/bundle and integrity/pairing substrates may begin independently.
Opi and pi adapters share conformance but remain separate vertical slices.
The three benchmark-revision adapters likewise share conformance without
sharing revision-specific task or native-verifier parsing. The assembled smoke
waits for both Agent adapters, all three benchmark adapters, bundle sealing,
and integrity admission. Offline reporting consumes only sealed outputs from
that assembled path. The final seam-evidence matrix is derived only after the
Linux native artifact exists; the following three-platform receipt attests the
actual pull-request head and checked-out merge-ref identities without changing
the repository's ordinary integration-check semantics.

The `opi-implement` plan handoff registers:

- every `P18-*` requirement and `P18-A*` scenario;
- the exact provisional package, source, fixture, schema, test, CI, and
  documentation impact;
- one end-to-end Evaluation product owner;
- task-local definitions of done and success criteria;
- focused hermetic tests before native smoke and workspace gates;
- exact upstream benchmark, grader, package, image, and tool locks;
- an explicit no-Opi-dependency and no-reverse-dependency check;
- honest real-process/native-grader evidence with no mock substitution;
- failure ownership, effect-unknown, pairing, integrity, and bundle mutation
  negative paths in their owning tasks;
- a final seam-evidence matrix that freezes only conformance-proved fields;
- native-smoke platform/resource/cost assumptions and rollback order; and
- full repository and three-platform gates.

The planning workflow does not weaken the two-Agent/three-benchmark-revision
and two-native-verifier-contract requirements to fit local tooling. A
discovered need for live provider credentials, broad Harness v2 adoption,
Agent Core modification, grader vendoring, hosted
scheduling, or a premature public protocol is a shaping decision governed by
`P18-AUTH-005`, not an incidental implementation task.

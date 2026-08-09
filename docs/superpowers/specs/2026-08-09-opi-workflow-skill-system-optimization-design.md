# Opi Workflow Skill System Optimization Design

**Status:** Approved design

**Date:** 2026-08-09

**Scope:** `.claude/skills`, its English/Chinese workflow indexes, and the
root `AGENTS.md` / `CLAUDE.md` workflow pointers. Product code and the root
product README files are out of scope.

## Context

Opi is not a compatibility port of pi. It follows pi's design direction while
implementing the selected ideas in Rust. Opi also extends that foundation
through packages, adapters, and plugins that can be invoked independently or
combined into a broader ecosystem.

The current `.claude/skills` directory already has strong project-specific
execution machinery:

- a tracked implementation ledger and durable phase snapshots;
- task-owned paths, verification tiers, evidence footers, and recovery rules;
- independent phase audit, remediation, runtime evaluation, documentation, and
  release workflows.

Its main weakness is before ledger initialization. The workflow index describes
specification as a manual linear phase, while the actual product-shaping work is
uncertain, iterative, and sometimes spans many sessions. At the same time,
`opi-implement` currently composes several Superpowers skills whose state
machines overlap with the opi harness, while useful Matt Pocock skills for
research, wayfinding, test seams, and two-axis review are not explicitly
integrated.

This design keeps the opi-specific harness as the sole implementation state
machine. Matt skills provide decision and engineering semantics. Superpowers
skills remain only where they provide a small operational primitive that opi
does not already own.

## Design principles

1. **Pi is the inward reference.** `opi-realign` measures whether opi still
   follows pi's current design direction, allowing deliberate Rust-native
   divergence.
2. **Research expands outward.** `opi-research` investigates capabilities that
   pi lacks, implements poorly for opi's needs, or leaves to its surrounding
   ecosystem.
3. **Plugins are the default home for optional expansion.** Provider-specific,
   experimental, or non-core capabilities belong in packages/plugins unless a
   missing core extension seam must first be introduced.
4. **Facts do not decide products.** Research and realignment produce evidence;
   a human-led deliberation process turns that evidence into decisions.
5. **Shaping is not a mechanical phase.** The workflow may define entry and exit
   contracts, but it must permit repeated returns to research, realignment,
   prototyping, and decision work.
6. **One implementation state machine.** `.opi-impl-state.json` and
   `opi-implement` remain authoritative. Generic ticket, plan-execution, and
   subagent-development state machines must not run inside it.
7. **Adversarial planning is an admission gate.** `opi-implement plan` may try
   to falsify a design and its task graph, but it may not make product decisions
   or silently repair the source spec.
8. **Evidence before completion.** Mechanical verification remains separate
   from semantic review, and no phase passes on agent confidence alone.
9. **Skills are pointers, not caches.** Opi skills name the selected Matt or
   Superpowers subskill and state the local contract; they do not copy entire
   upstream skill bodies into the repository.

## Workflow architecture

The workflow has three lanes rather than one seven-stage pipeline.

```text
Evidence and shaping
  inward:  pi -> opi-realign ----\
                                    -> human deliberation -> reviewed spec
  outward: primary sources -> opi-research --/

Delivery
  reviewed spec -> opi-implement plan -> task loop -> phase snapshot

Assurance and release
  phase snapshot -> opi-audit -> opi-remediate -> opi-eval
                 -> opi-document -> opi-release
```

The evidence paths can run independently and repeatedly. Deliberation may send
work back to either path. Delivery begins only after a reviewed source exists.
Assurance consumes a fixed implementation point and never substitutes for
product shaping.

## Opi workflow router

Add a user-invoked `opi-workflow` skill. It is a router, not an orchestrator.
It identifies the current state, opens the actual selected skill, and stops at
human decision boundaries.

Routing rules:

| Situation | Route |
|---|---|
| Compare current opi with the latest pi design and implementation | `opi-realign` |
| Investigate a capability beyond or poorly served by pi | `opi-research` |
| Large, multi-session, foggy decision space | Matt `wayfinder` |
| Bounded design ambiguity in the current session | Matt `grill-with-docs` |
| Existing decisions need to be synthesized into a spec | Matt `to-spec` |
| A reviewed spec is ready for admission and task-graph construction | `opi-implement plan` |
| A ledger task is ready | `opi-implement` |
| A hard bug or performance regression needs diagnosis | Matt `diagnosing-bugs`, then the appropriate delivery route |
| A completed phase needs assurance | `opi-audit`, then `opi-remediate` |
| A release candidate needs runtime evidence | `opi-eval` |
| Documentation or release work is ready | `opi-document`, then `opi-release` |
| Test-binary count itself is the problem | `opi-slim-tests` |

The router must not automatically run the whole route. Direct invocation of
`wayfinder`, `opi-research`, `opi-realign`, or any later skill remains valid.

## Evidence lane

### Inward alignment: `opi-realign`

`opi-realign` owns design-lineage comparison with pi.

Required behavior:

- resolve a concrete, current pi revision before comparison;
- compare semantics, runtime behavior, extension direction, and design
  philosophy rather than file layout;
- use the spec's Full / Partial / Intentional Divergence / Missing / Out of
  Scope vocabulary;
- distinguish a missing selected capability from an intentionally unselected pi
  capability;
- evaluate whether Rust-native divergence preserves the underlying design
  intent;
- report evidence and recommendations without automatically changing the spec;
- batch independent measurements within the available concurrency limit rather
  than assuming dozens of simultaneous agents.

`opi-realign` does not invoke generic Matt `research`. Tracking pi is its own
bounded evidence discipline and must remain distinguishable from outward
capability discovery.

### Outward expansion: `opi-research`

Add `opi-research` as an opi-specific wrapper around Matt `research`.

Matt `research` remains responsible for delegating fact gathering to a
background agent, preferring primary sources, citing claims, and writing a
Markdown artifact. `opi-research` adds the project-specific question and output
contract.

Research reports go under `docs/research/YYYY-MM-DD-<topic>.md` and contain:

1. the capability or problem being investigated;
2. why pi is absent, insufficient, or unsuitable as the sole reference;
3. primary-source findings and competing approaches;
4. Rust feasibility and important platform constraints;
5. fit with opi's existing extension/package/process-adapter model;
6. the smallest missing core seam, if an ecosystem implementation cannot be
   expressed through existing seams;
7. candidate placement: core, extension seam, official plugin/package, or
   external example;
8. unresolved product and architecture decisions;
9. explicit non-findings and evidence limitations.

The placement section is analysis, not a product decision. A report may
recommend a placement, but only shaping can accept it.

## Human-led shaping

Shaping is a decision space with controlled artifacts, not an automatically
executed stage.

### Large or foggy work

Use Matt `wayfinder` when the destination cannot fit in one agent session or
the path contains unresolved decision dependencies. Its map contains decision
tickets, not implementation tasks. Research tickets may invoke `opi-research`;
pi-lineage questions may invoke `opi-realign`; grilling tickets use Matt
`grilling` plus `domain-modeling`.

The wayfinder map must not become a second implementation ledger. It ends when
the route to a reviewed spec is clear.

### Bounded work

Use Matt `grill-with-docs` when the decision tree fits in the current context.
It composes `grilling` with `domain-modeling`, ensuring resolved terminology and
rare load-bearing architecture decisions have one durable home.

### Spec synthesis

Matt `to-spec` is optional. Invoke it only when decisions are settled but a
coherent source document is still missing. If wayfinding already produced the
reviewed spec, skip it.

Opi supplemental specs continue to live under the repository's reviewed design
convention rather than being forced into an external issue tracker. The opi
wrapper must preserve the useful `to-spec` semantics—synthesis rather than a new
interview, explicit test seams, problem/solution/out-of-scope, and no fragile
file-level implementation detail—while following the repository's artifact
location.

## `opi-implement plan` admission review

`opi-implement plan` becomes the boundary between human-led shaping and
mechanical delivery.

### P.0 Source admission

Before drafting tasks, verify that:

- every source is reviewed and registered;
- problem, solution, out-of-scope, success, and exit criteria are explicit;
- evidence provenance is identified as pi alignment, outward research, or both;
- Rust-native divergence has a recorded rationale where relevant;
- new capabilities explain why they belong in core, an extension seam, or a
  plugin/package;
- changed domain terms agree with `docs/CONTEXT.md`;
- the public acceptance and test seams are explicit.

Failure stops without mutating the canonical ledger.

### P.1 Draft graph

Build the candidate graph in a temporary artifact. Do not replace
`.opi-impl-state.json` yet.

Each task must:

- be a vertical, independently demonstrable tracer bullet unless it is an
  explicitly justified expand-contract refactor step;
- fit in a fresh implementation context;
- have real blocking edges rather than presentation-order dependencies;
- map to at least one success/exit criterion;
- identify acceptance scenarios, production call paths, a pre-agreed public
  test seam, verification tier/addenda, owned paths, and forbidden scope;
- answer: "What can be demonstrated when this task is complete?"

These rules absorb the useful parts of Matt `to-tickets` without invoking its
tracker-writing state machine.

### P.2 Adversarial review

Review the source and draft graph in a fresh context and, when the environment
supports it, a different model family. A same-model fresh-context review is
reported as degraded independence rather than misrepresented as cross-model.

The review has two non-collapsible axes:

**Design readiness**

- preservation of pi's design direction;
- justified Rust-native divergence;
- plugin-first placement and minimal core seams;
- coherent domain language and deep module interfaces;
- explicit user-visible behavior and sufficiently high test seams;
- contradictions, unstated assumptions, and premature commitments.

**Execution readiness**

- complete criterion-to-task coverage;
- vertical slicing and demonstrability;
- correct blocking edges;
- no hidden horizontal infrastructure batches;
- plausible owned paths and production wiring;
- proportional verification tiers and forbidden-scope guards.

The reviewer reports findings. It does not edit the source or draft.

### P.3 Verdict and transition

The plan path has four results:

| Verdict | Effect |
|---|---|
| `READY` | Present the graph for user confirmation, then atomically write the canonical ledger |
| `RESEARCH_REQUIRED` | Stop and route to `opi-research` or `opi-realign`; do not write the ledger |
| `DESIGN_DECISION_REQUIRED` | Stop and route to `wayfinder` or `grill-with-docs`; do not write the ledger |
| `GRAPH_REVISION_REQUIRED` | Revise only the temporary graph and repeat the adversarial review |

Remove the current behavior that lets initializer or task-level grilling amend a
live normative spec in place. A fuzzy or incorrect source is returned to its
owning shaping artifact. Task execution may clarify implementation detail, but
it cannot silently change product meaning.

## Matt and Superpowers subskill selection

Matt skills are selected for semantics and human decision boundaries.
Superpowers skills are selected only for isolated operational enforcement.

| Opi location | Selected subskill | Decision |
|---|---|---|
| Workflow routing | Matt `ask-matt` phase-boundary principles; open the actual target skill | Use as a secondary routing reference, never as the load-bearing source |
| Large shaping | Matt `wayfinder` | Select |
| Bounded shaping | Matt `grill-with-docs` | Select |
| Domain decisions | Matt `domain-modeling` | Select only when the domain model changes |
| Interface/plugin/test seams | Matt `codebase-design` | Select as a design reference |
| Evidence beyond pi | Matt `research` through `opi-research` | Select |
| Spec synthesis | Matt `to-spec` | Optional; skip when a reviewed spec already exists |
| Task graph semantics | Matt `to-tickets` tracer-bullet rules | Incorporate rules; do not invoke the tracker workflow |
| Task implementation | Matt `tdd` | Replace `superpowers:test-driven-development` |
| Hard bugs/performance regressions | Matt `diagnosing-bugs` | Replace task-loop `superpowers:systematic-debugging` |
| Phase audit | Matt `code-review` | Invoke for Standards/Spec axes, then add opi-specific audit dimensions |
| Documentation | Matt `writing-for-agents` | Use for cache/pointer hierarchy and agent-readable prose |
| Final evidence gate | `superpowers:verification-before-completion` | Retain |
| Independent parallel units | `superpowers:dispatching-parallel-agents` | Retain conditionally, bounded by ownership and available slots |
| Generic Matt `implement` | None | Reject: duplicates TDD, review, and commit ownership |
| Superpowers `writing-plans` | None inside opi workflow | Reject: creates a second file-level plan and commit cadence |
| Superpowers `executing-plans` / `subagent-driven-development` | None | Reject: duplicate workspaces, ledgers, commits, and review loops |
| Superpowers `brainstorming` | None inside opi workflow | Reject: fixed single-spec state machine conflicts with optional wayfinding and repository commit policy |

Matt `tdd` is a better fit because opi already owns execution enforcement. It
adds the missing semantic constraints: behavior through public interfaces,
pre-agreed seams, one vertical red/green slice at a time, and no bulk imagined
tests. Cleanup/refactoring is performed after green under review, not used to
expand a red/green cycle.

Matt `diagnosing-bugs` is a better fit for this repository's provider, CLI,
streaming, and performance failures because it first constructs a tight
red-capable feedback loop and explicitly supports trace replay, differential
testing, minimization, nondeterminism, performance baselines, and secret
redaction.

Superpowers verification remains useful because no selected Matt skill provides
an equivalent fresh-command, evidence-before-claim gate.

## Per-skill optimization

### `opi-implement`

- add source admission and temporary adversarial graph review;
- replace generic task TDD/debugging drivers as described above;
- record the agreed test seam and demonstrable outcome in existing task planning
  evidence without introducing a second task store;
- block and return to shaping when the source meaning is wrong;
- remove host-specific target-directory examples as normative defaults;
- remove or make implementable the section-hash incremental-review claim;
- retain verification tiers, artifact truthfulness, owned paths, evidence
  footers, task commits, and separate ledger checkpoints;
- keep one task per invocation and the canonical ledger's recovery semantics.

### `opi-audit`

- compose Matt `code-review` against the ledger-derived fixed commit range;
- keep Standards and Spec results separate;
- add the repo's documented standards and Matt's smell baseline to Standards;
- retain opi-specific Security, Invariants, Integration, Residuals, and test
  quality dimensions;
- forbid recursive `code-review` or further agent spawning in reviewer prompts;
- use the shared severity vocabulary instead of redefining it.

### `opi-remediate`

- accept a normalized finding source from both audit and eval artifacts;
- preserve source identity and original severity before verification;
- use tier-scoped verification rather than defaulting every layer to the most
  expensive workspace test;
- treat a red baseline as an explicit decision, not something to normalize by
  silently continuing;
- use the shared finding/severity contract and the always-loaded root Git rules
  without conflicting rollback instructions.

### `opi-eval`

- remain explicitly user-invoked because it spends real provider credits;
- build in a persistent per-worktree release cache outside the repository;
- retain Cargo incremental compilation and never clean a cache during a task;
- report actual evaluator independence and degraded same-family operation;
- emit normalized regression findings that `opi-remediate` can consume;
- keep evaluators read-only and preserve raw trace provenance.

### `opi-document`

- replace phase-numbered Rust prose guards with one fast, source-derived
  documentation check that does not compile a Rust test binary;
- keep behavior and architecture assertions in their owning Rust suites;
- invoke Matt `writing-for-agents` for information hierarchy, environment
  pointers, completion criteria, and cache pruning;
- refer to translation skills by installed skill name rather than a
  machine-specific filesystem path;
- keep English/Chinese synchronization and spec-hash handling intact.

### `opi-release`

- compose `opi-document scope=version-bump` before staging the release change;
- reconcile release checksums with `docs/opi-spec.md` and upload
  `SHA256SUMS.txt` with binary artifacts;
- use the six-crate graph and compute publish order from metadata;
- remove broad staging, forbidden checkout rollback, false side-effect claims,
  and hard-coded orchestration tool names;
- provide Windows and Unix command paths without assuming one shell;
- retain explicit irreversible-boundary confirmations and resume state.

### `opi-realign`

- keep its inward-only definition prominent;
- resolve and report the exact pi revision;
- bound parallel work to available slots;
- separate evidence classification from optional prioritization;
- never treat every pi feature as automatically desirable.

### `opi-slim-tests`

- stop after verified changes unless the user separately asks to commit;
- classify current-contract, duplicate, superseded, historical-evidence,
  platform-only, and helper-binary tests from their bodies;
- delete superseded prose guards when a cheaper source-derived check replaces
  them; preserve only behavior or architecture evidence that still represents
  the current product.

## Cross-harness metadata

Every stateful, costly, or high-impact `opi-*` skill is user-invoked. Add
Codex `agents/openai.yaml` sidecars with implicit invocation disabled, matching
the current AI Hero Codex convention. Add or retain Claude
`disable-model-invocation: true` where implicit activation could mutate state,
spend credits, create commits, or publish artifacts.

Subskill names are explicit:

- unqualified names such as `tdd`, `wayfinder`, and `code-review` mean the Matt
  skill;
- `superpowers:<name>` means the Superpowers skill;
- before a load-bearing call, the opi skill opens the selected subskill rather
  than relying on the router's summary;
- missing required skills cause a clear setup failure, not silent fallback to a
  different workflow.

The repository does not vendor copies of either upstream skill package.

## Workflow indexes and root guidance

Rewrite `.claude/skills/README.md` and `README.zh.md` as synchronized maps rather
than duplicated manuals. They document:

- the inward/outward evidence distinction;
- optional human-led shaping;
- the delivery and assurance lanes;
- subskill choices and rejected overlapping state machines;
- durable artifacts and phase boundaries;
- explicit invocation and model-independence rules.

Remove machine-specific model recommendations and details that are already
discoverable from the individual skills.

Add only a concise pointer to root `AGENTS.md` and `CLAUDE.md`, which are always
loaded. The pointer states the design lineage, plugin-first expansion rule, and
which workflow map to consult. It must not restate the full lifecycle.

Keep the two root files in lockstep and correct the current dirty dependency
graph so `opi-coding-agent` continues to show its real `opi-protocol`
dependency.

## Failure and boundary behavior

- Missing evidence does not become an assumed fact; route to the appropriate
  evidence lane.
- An unresolved product choice does not become a task; return to shaping.
- An invalid task graph does not mutate the canonical ledger; revise the draft.
- A missing subskill stops at setup; do not silently substitute another skill.
- A same-model review is permitted only with explicit degraded-independence
  labeling.
- An external provider failure in eval does not authorize source edits.
- Any release-state mismatch stops before another irreversible action.
- Existing unrelated working-tree changes are preserved. Implementation edits
  must be reviewed against the pre-existing diff before application.

## Acceptance criteria

The optimization is complete when:

1. `opi-workflow` routes without becoming an end-to-end orchestrator.
2. `opi-realign` and `opi-research` have non-overlapping inward/outward
   contracts.
3. The workflow explicitly permits direct, iterative use of `wayfinder` and
   repeated returns to evidence gathering.
4. `opi-implement plan` drafts before mutation and has the four explicit
   admission verdicts.
5. A spec problem cannot be silently amended by task execution.
6. Matt `tdd`, `diagnosing-bugs`, and `code-review` replace the overlapping
   Superpowers semantic drivers.
7. Superpowers remains only for completion verification and bounded independent
   dispatch.
8. No generic plan/execution skill introduces a second ledger, worktree, or
   commit protocol.
9. Audit and eval findings can enter remediation without manual transcription.
10. Release, documentation, crate-count, checksum, Git-safety, and guard-count
    contradictions are resolved.
11. Every high-impact opi skill has consistent Claude/Codex explicit-invocation
    metadata.
12. English and Chinese workflow indexes agree.
13. Root `AGENTS.md` and `CLAUDE.md` remain synchronized and concise.
14. Runtime product source remains unchanged; the approved follow-up may remove
    obsolete test-only prose guards and add repository verification scripts.
15. No commit is created unless the user separately requests one.

## Verification strategy

This is primarily a skill/documentation change. Verification consists of:

- parse every skill frontmatter and Codex sidecar;
- check all referenced local paths and subskill names;
- search for obsolete skill selections, guard counts, crate counts, forbidden
  git commands, machine-specific model/target paths, and checksum conflicts;
- exercise read-only router examples for inward alignment, outward research,
  wayfinding, plan admission, audit, eval, and release;
- run the existing workflow JavaScript tests for any modified harness scripts;
- run the relevant ledger guard test if ledger semantics change;
- run documentation guard suites for touched product-facing documentation;
- run `git diff --check` and inspect the final diff against the pre-existing
  working-tree changes.

Because no Rust product code is in scope, the repository-wide Rust clippy gate
is not required solely for this optimization. Any accidental Rust code change
would expand the scope and require the normal code verification gates.

## Approved follow-up: verification economy

The post-review decision adds these constraints:

- `_shared/references/` keeps only contracts with multiple real consumers.
  Severity is co-located with the normalized finding schema. Git safety stays
  in the always-loaded root guidance instead of a second shared copy.
- `opi-implement` has one authoritative mechanical gate per task. D.3 adds
  only acceptance or platform checks that D.0/D.1 did not already run.
- Cargo targets are persistent per worktree/toolchain and live outside the
  repository. Incremental compilation stays enabled. Task-time `cargo clean`
  and end-of-session target deletion are forbidden; pruning is an explicit,
  inactive-cache maintenance operation. `scripts/opi-cargo-cache.py` owns
  stable resolution, active-process leases, status reporting, and dry-run-first
  age/size pruning of marker-owned caches.
- Task-level adversarial evaluation is reserved for security/safety,
  authentication/permission, public protocol/API, session durability,
  provider-wire/model behavior, and cross-crate semantic risk. Deterministic
  documentation, skill, test-only, mechanical, and behavior-preserving
  internal refactors skip it. Phase exit retains one independent adversarial
  review.
- `phaseN_*_docs.rs` and other narrative token guards are sediment, not product
  behavior. Current version/schema/link/mirror checks move to a fast Python
  script; runtime facts stay in behavior tests and architecture facts stay in
  topic-based contract tests.

## Sources

- Matt Pocock skill catalog and guidance: <https://www.aihero.dev/skills>
- Matt Pocock skills source: <https://github.com/mattpocock/skills>
- Superpowers source: <https://github.com/obra/superpowers>
- Opi technical specification: `docs/opi-spec.md`
- Opi domain vocabulary: `docs/CONTEXT.md`

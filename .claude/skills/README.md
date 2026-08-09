# Opi skills

The ten `.claude/skills/opi-*` skills implement one project workflow without
turning product discovery into a mechanical pipeline.

The design principle is:

> Follow pi's design ideas, implement them in Rust, and extend opi through
> plugin/package seams so extensions can run independently and enrich the opi
> ecosystem.

All `opi-*` skills require explicit invocation. Claude metadata uses
`disable-model-invocation: true`; Codex metadata uses
`policy.allow_implicit_invocation: false`. Use `opi-workflow` when the correct
entry point is unclear.

## Workflow map

| Concern | Entry | Output/next decision |
|---|---|---|
| Inward evidence | `opi-realign` | Exact-revision pi/opi delta ledger under `docs/realign/` |
| Outward evidence | `opi-research` | Primary-source capability study under `docs/research/` |
| Foggy shaping | Matt `wayfinder` directly | Decision map; repeated research/realign/grilling as needed |
| Bounded design challenge | Matt `grill-with-docs` directly | Explicit decisions and domain language |
| Settled design | Matt `to-spec` directly | Candidate implementation specification |
| Admission and delivery | `opi-implement plan`, then `opi-implement` | Reviewed task graph and canonical implementation ledger |
| Static assurance | `opi-audit` | Independent Standards/Spec findings |
| Runtime assurance | `opi-eval` | Runtime-fidelity findings and traces |
| Verified correction | `opi-remediate` | Source-preserving verification and optional fixes |
| Documentation | `opi-document` | Truthful EN/ZH docs and fast source-derived checks |
| Publication | `opi-release <version>` | GitHub assets and six crates.io releases |
| Test-link optimization | `opi-slim-tests` | Verified, uncommitted reduction in integration binaries |

### Inward and outward evidence are separate

`opi-realign` is inward. It pins an exact `earendil-works/pi` revision and asks
whether opi preserves pi's current semantics and design lineage using
Rust-native architecture. It does not propose unrelated features.

`opi-research` is outward. It investigates capabilities pi lacks or does not
serve well, prioritizes primary sources, evaluates Rust feasibility, and asks
whether the capability belongs in an existing plugin/package, a new plugin, or
the smallest evidenced core seam. It does not write a spec or authorize work.

Neither report is a requirement. Both are evidence for shaping.

### Shaping stays human-led

Turning evidence into a feature is intentionally not a fixed workflow. It is a
loop of clarification, experiments, trade-offs, rejection, and return to
evidence. Use Matt's tools directly according to uncertainty:

- `wayfinder` for large, foggy, multi-session design spaces;
- `grill-with-docs` for a bounded decision that needs adversarial questioning
  and domain-model updates;
- `to-spec` only after material decisions have settled;
- `research` or `opi-realign` again when a decision exposes an evidence gap.

`opi-workflow` routes to these skills but does not own another ledger or hide
the loop behind automatic transitions.

### `opi-implement plan` is an adversarial admission gate

The plan path does not design the product. It tests whether a candidate source
is ready to enter the canonical implementation state machine:

1. admit and pin the normative source;
2. derive a draft vertical-slice task graph without mutating the live ledger;
3. challenge design readiness and execution readiness separately;
4. return one deterministic verdict:
   `READY`, `RESEARCH_REQUIRED`, `DESIGN_DECISION_REQUIRED`, or
   `GRAPH_REVISION_REQUIRED`;
5. mutate `.opi-impl-state.json` only after `READY` and the user's graph gate.

A missing product decision routes back to shaping. Missing evidence routes to
`opi-research` or `opi-realign`. The plan reviewer never silently amends the
source or edits its own draft to manufacture a pass.

## Matt vs Superpowers

The local Matt package is the default source of reasoning- and artifact-level
subskills inside `opi-*`. Superpowers remains only for narrow operational
primitives that do not compete with opi's canonical ledger.

| Need | Choice | Rationale |
|---|---|---|
| Outward evidence | Matt `research` | Primary-source, repository-artifact contract |
| High-uncertainty shaping | Matt `wayfinder` | Decision-map workflow tolerates iteration and reversals |
| Bounded adversarial shaping | Matt `grill-with-docs` | Couples questioning with domain-language maintenance |
| Spec synthesis | Matt `to-spec` | Synthesizes settled context instead of restarting discovery |
| Implementation slices | Matt `tdd` | Public seam first; vertical red/green slices; no premature refactor phase |
| Hard diagnosis | Matt `diagnosing-bugs` | Establishes a red-capable feedback loop, then minimizes/differentiates |
| Audit lenses | Matt `code-review` | Keeps Standards and Spec axes separate |
| Documentation | Matt `writing-for-agents` | Favors cacheable facts, pointers, and no-op guidance |
| Completion proof | Superpowers `verification-before-completion` | Narrow evidence-before-claim discipline |
| Independent work | Superpowers `dispatching-parallel-agents` | Conditional concurrency primitive only |

Not composed inside `opi-implement`:

- Superpowers `brainstorming`, `writing-plans`, `executing-plans`, and
  `subagent-driven-development` would create a second planning/execution
  workflow beside the canonical ledger.
- Matt `to-tickets` and `implement` encode useful heuristics, but their state
  machine must not replace `.opi-impl-state.json`. Tracer-bullet decomposition
  is absorbed into plan admission instead.
- Direct shaping remains available outside `opi-implement`; exclusion from the
  harness is not a judgment that those skills are generally inferior.

This selection follows the progressive-disclosure and invocation guidance in
[AI Hero Skills](https://www.aihero.dev/skills), the locally pinned
[Matt skills package](https://github.com/mattpocock/skills), and the locally
available [Superpowers package](https://github.com/obra/superpowers). The full
rationale is recorded in
`docs/superpowers/specs/2026-08-09-opi-workflow-skill-system-optimization-design.md`.

## Assurance contract

`opi-audit` and `opi-eval` emit normalized findings using
`_shared/references/finding-contract.md`. Each finding preserves its source
kind/path/model, independence quality, axis, severity, evidence, reproduction,
confidence, and unverified status.

`opi-remediate` consumes either source without manual transcription. It
preserves provenance and severity, verifies the claim against code or runtime
artifacts, and records remediation verification separately from the original
finding. If a finding actually changes product intent, remediation stops and
routes back to shaping.

Use independent models/reviewers when practical, but report degraded
independence honestly. Never encode one preferred provider or model into the
project workflow.

## Durable artifact ownership

| Artifact | Owner |
|---|---|
| `docs/realign/*.md` | `opi-realign`; generated, non-normative inward evidence |
| `docs/research/*.md` | `opi-research`; generated, non-normative outward evidence |
| `docs/opi-spec.md` and registered supplemental specs | Human-led shaping; normative sources |
| `.opi-impl-state.json` | `opi-implement`; canonical tracked implementation ledger |
| `docs/snapshots/phase<N>/` | `opi-implement` archive plus audit/remediation evidence |
| `docs/eval/` | `opi-eval` reports/history |
| `.opi-release-state.json` | `opi-release`; resumable public/irreversible transition state |
| `_shared/references/finding-contract.md` | Cross-skill finding schema |
Git safety is defined once in the always-loaded `AGENTS.md` / `CLAUDE.md`, not
duplicated under `_shared`.

Only `opi-implement` writes the canonical implementation ledger. Research,
realign, audit, eval, remediation planning, documentation, and release must not
create competing task ledgers.

## Skill index

| Skill | Contract |
|---|---|
| `opi-workflow` | Thin router; no state machine and no implementation |
| `opi-realign` | Pinned-revision inward alignment; no outward proposals |
| `opi-research` | Primary-source outward exploration; no requirements or implementation |
| `opi-implement` | Source admission, adversarial graph review, TDD delivery, verification, and ledger checkpoints |
| `opi-audit` | Independent committed-range Standards/Spec audit; no fixes |
| `opi-eval` | Explicit, credentialed runtime regression evaluation in isolation |
| `opi-remediate` | Verify normalized audit/eval findings; execution remains user-gated |
| `opi-document` | Truthful documentation, EN/ZH synchronization, and a no-compile documentation check |
| `opi-release` | Seven gated local/public/irreversible release phases |
| `opi-slim-tests` | Preserve current behavior while deleting duplicate/superseded Rust test binaries; no automatic commit |

Read the selected skill's `SKILL.md` and only the references it routes to.
Invoke destructive, costly, credentialed, or publication skills explicitly.

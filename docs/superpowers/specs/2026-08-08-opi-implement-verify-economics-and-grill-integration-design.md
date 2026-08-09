# opi-implement: Verify-Economics + Grill-into-Ledger Integration — Design

- Date: 2026-08-08
- Status: Design (pending implementation)
- Scope: `opi-implement` skill, `scripts/opi-impl-smoke.{sh,ps1}`, `references/verification-tiers.md`, project context files (`CONTEXT.md`, `CLAUDE.md`, `AGENTS.md`)
- Origin: `/grill-me` session 2026-08-08 (stateless shaping; this doc is its `to-spec` output)

This spec is the captured shared understanding of a grilling session. It does not
implement anything; it is the input for `opi-implement` task decomposition.

## 1. Problem

Two separable pain categories, conflated in the request to "replace the superpowers
workflow." Grilling established that they are independent and must be treated as two
tracks.

### 1.1 Track A pain — verification economics (the #1 driver)

Per-task, a non-documentation task currently triggers workspace-wide compilation and
testing **redundantly**. Measured facts:

- **Phase A.3 smoke** (`scripts/opi-impl-smoke.sh:14-27`, `opi-impl-smoke.ps1:14-31`):
  five hardcoded, workspace-wide gates, not parameterizable:
  `cargo build --workspace`, `cargo fmt --check --all`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --all-targets`, toolchain check.
- **D.1 tier gates** (`references/verification-tiers.md`): the `library` tier runs
  `cargo test -p <crate>` **and then** `cargo build --workspace` (line 49), re-widening
  to a workspace build.
- **D.3 cross-cutting gates** (`verification-tiers.md:271-279`), run after tier gates
  for **all non-documentation tiers**: `fmt --check --all`, `clippy --workspace`,
  `rustdoc --workspace`, and **the smoke script again** (another
  `cargo test --workspace --all-targets`).

Net per non-doc task: workspace test runs **twice** (A.3 + D.3); workspace
build/clippy **two or more times**. The `documentation` tier is the only one that
genuinely skips compilation (`verification-tiers.md:21-35`).

The per-crate scoping in D.1 is **subsumed** by the D.3 cross-cutting gates on every
non-doc task: tiering is nominal for non-doc work.

### 1.2 Disk

- No `CARGO_TARGET_DIR` override anywhere in skill or scripts.
- No per-task `cargo clean`; `target/` (at repo root, on `D:`) only grows.
- `CARGO_INCREMENTAL=0` appears only as prose advice (`README.md:281`), not set in any
  script.
- Each worktree carries its own `target/`, multiplying accumulation.

### 1.3 Track B pain — missing upstream shaping

`opi` authors phase design specs ad-hoc, then feeds them to `opi-implement`, which
decomposes a spec into the task ledger. There is no disciplined clarification step at
the spec→ledger boundary: ambiguities surface late (during implementation) rather than
when the ledger is created. The Matt Pocock grilling discipline (already installed as
`mattpocock-skills:grilling` / bare `grilling`) supplies this, but it is not wired into
the flow.

### 1.4 Audit overlap (clarification)

Per-task verification passes are: D.0 (acceptance) → D.1 (tier) → D.2 (exec-verify,
risk-gated) → D.3 (cross-cutting). The heavier independent audits (`opi-audit`,
`opi-remediate`, `phase-exit.workflow.js`) run **only at phase boundaries**, not per
task. D.2 itself is read-only (no compilation; cost is agent tokens, not build time).

## 2. Goals / Non-goals

### Goals

- G1: Collapse the per-task workspace compile/test redundancy for the common
  single-crate case to roughly one workspace build + one scoped test.
- G2: Stop unbounded `target/` growth; relocate build output off the tight drive and
  reclaim per-task artifacts.
- G3: Tier-gate D.2 so low-risk tasks skip per-task exec-verify.
- G4: Fold grilling into ledger creation (initializer) and Phase B, with decisions
  landing in auditable, non-duplicated homes.
- G5: Keep the repo root clean; make key context traceable by humans, audits, **and**
  the implementing agent.

### Non-goals

- N1: Do **not** replace `opi-implement` or its task ledger / A–F stage structure. It
  stays as the execution and verification spine.
- N2: Do **not** add a `docs/adr/` directory (see §6.2).
- N3: Do **not** introduce a separate upstream Pocock "shaping pipeline" that runs
  before `opi-implement`. Clarification folds **into** ledger creation.
- N4: No Docker/VM/remote build backends; no changes to release/CI topology.
- N5: No fork of the installed Pocock skills; reuse them.

## 3. Design overview

Two tracks, executed in order, mutually independent:

- **Track A — verify economics** (§4): addresses G1–G3. Highest payoff, lowest risk.
- **Track B — grill folded into ledger creation** (§4 of the grilling → §5 here):
  addresses G4. Depends on nothing in Track A.
- **Constraints** (§6): G5, applied across both tracks.
- **Validation gate** (§7): the new verify economics must be proven not to leak defects
  before full cutover.

## 4. Track A — Verify economics

### 4.1 Collapse redundant workspace compiles

| Gate | Current | New |
|---|---|---|
| A.3 smoke | build+fmt+clippy+test, all `--workspace` | **build `--workspace` + `fmt --check --all` + `clippy --workspace`** (one compile; **drop the `test` gate** — it is the "is the tree buildable and lint-clean" early check) |
| D.1 `library` tier | `test -p <crate>` + `build --workspace` (line 49) | `test -p <crate>` **only**; drop the redundant `build --workspace` (A.3 already proves the workspace builds) |
| D.3 cross-cutting | `fmt` + `clippy --workspace` + `rustdoc --workspace` + full smoke, for all non-doc tiers | **tier- and diff-scoped** (see §4.2); full workspace smoke reserved for `workspace` / cross-crate tier only |

Net for the common single-crate task: from **3 workspace compiles + 2 workspace tests**
to roughly **1 workspace build + 1 scoped crate test**.

### 4.2 Tier → verify-budget matrix

D.3's cross-cutting gate becomes **tier-parametrized**. The smoke script must accept
arguments (tier and/or target crate / named test binary) so D.3 can invoke it scoped.
Target matrix (exact tier names reconcile against the current `verification-tiers.md`
at implementation time):

| Tier | D.1 | D.2 | D.3 (scoped) |
|---|---|---|---|
| `documentation` | `git diff --check` + doc guards | **skip** | doc guards only |
| `library` (single crate, isolated refactor) | `test -p <crate>` | **skip** | `clippy -p <crate>` + `test -p <crate>` |
| `cli-tool` / `cli-runtime` / `tui` | `test -p <crate>` | **keep** (runtime/CLI surface) | `clippy -p <crate>` + `test -p <crate>` + relevant binary smoke |
| `workspace` / cross-crate | full tier gates | **keep** | **full workspace smoke** (the only tier that runs it) |

The default rule is the static matrix; a diff-scope override (compute touched crates,
test only those) applies in the common case where a task touches a strict subset.

### 4.3 Disk — target relocation and cleanup

- `CARGO_TARGET_DIR=E:\opi-target\<session-or-worktree-id>`. **Per-session directories,
  not one shared dir** — a single shared target across concurrent agents/worktrees races
  and corrupts builds (cargo target lock contention). Each session/worktree gets its own
  dir under `E:\opi-target\`.
- Set `CARGO_INCREMENTAL=0` **in the smoke scripts and the skill's env** (currently only
  prose). Tradeoff: less incremental metadata / more recompilation, accepted for disk
  predictability on a large workspace.
- **Per-task reclaim**: after D.3, run `cargo clean -p <worked-crate>` (keeps dependency
  cache, reclaims the just-built debug artifacts for the crate under work).
- **Session-end cleanup**: remove the session's `E:\opi-target\<id>` directory.
- Verify link speed once after relocation (cross-drive `D:`→`E:` linking may be slower);
  disk exhaustion is the harder failure and takes priority.

### 4.4 D.2 tier-gating

D.2 (`exec.workflow.js`) is already risk-gated via `evaluator_required`. Add a **tier
gate on top**: skip D.2 for `documentation` and isolated single-crate `library`
refactors (D.1 tests + D.0 acceptance cover correctness there). Keep D.2 for tasks with
real risk surface: runtime/CLI/NDJSON/session/provider, cross-crate API, security/sandbox.

Do **not** move D.2 to phase-exit only: that conflates per-task exec-verify ("did this
commit do what its task said") with the independent phase-boundary audit ("did the whole
phase meet its spec") — different purposes.

## 5. Track B — Grill folded into ledger creation

### 5.1 Placement

- **Initializer (coarse, per spec)**: when `initializer.md` turns a spec into a task
  graph, run one grilling pass over the spec to settle cross-cutting decisions
  (vocabulary, scope boundaries, out-of-scope) **before** any task runs. Resolved
  decisions are written into each affected task entry's DoD / acceptance / out-of-scope
  at creation time.
- **Phase B (fine, per task)**: Phase B already has a user gate. Make it explicitly
  grill-capable: when a task's spec slice is fuzzy, invoke the installed `grilling`
  skill to sharpen it before planning.

This is **not** a separate upstream pipeline (N3). The clarification happens at the
spec→ledger boundary because that is where the spec meets reality.

### 5.2 Decision landing — one decision, one home

| Decision shape | Home |
|---|---|
| Terminology / ubiquitous language | `docs/CONTEXT.md` (glossary) |
| In-task decision | that task's ledger entry (DoD / acceptance / out-of-scope) |
| Cross-cutting architectural decision | phase design doc, via the §5.3 amendment procedure |

A single decision is recorded in **exactly one** home, never duplicated.

### 5.3 Spec-amend procedure (when grilling finds the spec wrong/incomplete)

Grilling may reveal that the source spec itself is wrong or incomplete. Procedure:

1. **Amend the live source spec in place** at the affected section, and add a dated
   marker at the edit point:
   `> Amendment (YYYY-MM-DD): <what changed and why>`.
2. **Re-derive only the affected task entries** (DoD / acceptance / out-of-scope). Do
   not re-derive the whole graph — that would discard `verified_at_commit` records of
   already-verified tasks.
3. If an affected task is already implemented/verified, the amendment triggers a
   **targeted re-verify** of that task by its tier (§4.2), not a full re-implementation.
4. The spec-hash re-sync is handled by the existing mechanism (live
   `.opi-impl-state.json` `spec_files_sha256`, CRLF-normalized; pinned by
   `tests/spec_ledger.rs`).

**Guardrail**: amend only the live spec (`docs/superpowers/specs/<active-phase>` and
`docs/opi-spec.md`). **Never** edit frozen copies under `docs/snapshots/phaseN/`.

### 5.4 Traceability and visibility

- The glossary (`docs/CONTEXT.md`) is **not** loaded by opi's runtime context loader
  (`crates/opi-coding-agent/src/context_files.rs:15` loads only `AGENTS.md` and
  `CLAUDE.md`). To make the glossary visible to the implementing agent, add a pointer
  line in **both** `CLAUDE.md` and `AGENTS.md`:
  `Domain glossary: see docs/CONTEXT.md`.
- Traceability is provided by: the pointer (discoverability), the §5.3 amendment
  markers + spec-hash re-sync + snapshot guardrail (decision history), and the
  one-decision-one-home rule (coherence).

## 6. Constraints

### 6.1 Root hygiene

`CONTEXT.md` moves from the repo root to `docs/CONTEXT.md`. The repo root keeps only
the runtime-loaded context files (`AGENTS.md`, `CLAUDE.md`) plus standard files
(`Cargo.toml`, `CHANGELOG.md`, `README*`, etc.). All path references to `CONTEXT.md`
(audit snapshot logic, doc-guards, any tests) are updated to the new location.

### 6.2 No `docs/adr/`

A `docs/adr/` directory would be a third home for architectural decisions, overlapping
the phase design docs and the §5.3 amendment procedure. The amendment procedure already
covers the only genuine gap (cross-cutting decisions that emerge during grilling), so an
ADR directory is redundant. Not introduced. (Note: this is also why visibility is solved
by the pointer, not by the file format — `docs/adr/*` would be just as invisible as
`docs/CONTEXT.md`, since the loader only walks ancestor directories for `AGENTS.md` /
`CLAUDE.md`.)

## 7. Validation gate (must pass before full cutover)

Cheaper verification that leaks defects is a regression. Before adopting Track A across
the board:

1. Pick a real, representative task (a `library`-tier single-crate change and one
   `cli-runtime`-tier change).
2. Run it through the **new** tier-scoped flow.
3. Compare defect-catch against the **old** full-workspace-smoke flow (or against the
   task's known-good state).
4. **Pass criterion**: the scoped flow catches every defect the full flow would have
   caught for that task's surface. If a defect is missed, the tier scoping is too
   aggressive for that tier and must widen.
5. Only after the gate passes does Track A become the default for that tier.

This gate is self-referential (Track A changes the implement skill that would verify
it); the comparison-against-old-flow step is what breaks the circularity.

## 8. Rollout

- Ledger state at design time: `current_phase = 16`, `tasks = []` — **no phase in
  flight**. Phase 16 is archived; Phases 17/18 have design docs but are not loaded as
  active phases.
- Cutover is therefore zero-disruption: apply Track A, pass §7, then Track B, all before
  the next phase starts.
- Optionally pilot on one small real change before declaring Track A default.

## 9. Concrete edit targets (file-level, for task decomposition)

Track A:

1. `scripts/opi-impl-smoke.sh` and `scripts/opi-impl-smoke.ps1`
   - Slim A.3 to build + fmt + clippy (drop the `test` gate).
   - Make the script parameterizable: accept tier and/or target crate / named test
     binary so D.3 can invoke it scoped.
2. `references/verification-tiers.md`
   - D.3 cross-cutting gate (`:271-279`) → tier-parametrized (§4.2 matrix).
   - Drop the `library`-tier `build --workspace` (`:49`).
   - Encode the D.2 skip rule (documentation + isolated single-crate library).
3. `skill.md`
   - Set `CARGO_TARGET_DIR=E:\opi-target\<session>` and `CARGO_INCREMENTAL=0` in the
     skill's env.
   - Add the per-task `cargo clean -p <worked-crate>` step after D.3.
   - Add session-end cleanup of the session target dir.
   - Wire D.2 `evaluator_required` to the tier gate.

Track B:

4. `references/initializer.md` and `skill.md`
   - Add the coarse grilling pass at spec→task-graph decomposition.
   - Make Phase B explicitly grill-capable on fuzzy task slices.
   - Document the §5.2 decision-landing rules.
5. `skill.md`
   - Document the §5.3 spec-amend procedure (amendment marker, affected-entry
     re-derivation, snapshot guardrail).

Constraints (either track):

6. Move `CONTEXT.md` → `docs/CONTEXT.md`; update all references.
7. Add the glossary pointer line to `CLAUDE.md` and `AGENTS.md`.

## 10. Out of scope / open

- Cross-drive link-speed measurement after `E:` relocation (verify once at rollout).
- Whether `CARGO_INCREMENTAL=0` remains the right tradeoff after the per-session target
  relocation is live (revisit after §7).
- Exact tier-name reconciliation against the current `verification-tiers.md` (resolved at
  implementation, not at design).
- This spec is EN-only and guard-neutral per the design-doc authoring convention; it does
  not by itself touch `opi-spec.md` §release-history or any released section.

## 11. Appendix — Matt Pocock skill coverage & engine decisions

Discussions during implementation raised how the opi flow relates to the Matt
Pocock skill library and the `superpowers` plugin. Recorded here so the question
does not recur.

### 11.1 Coverage map (which matt skills opi already incorporates)

| Matt skill | opi equivalent | Status |
|---|---|---|
| `grilling` / grill-me | `A.init.2b` + Phase B `B.1a` (§5) | incorporated |
| grill-with-docs (CONTEXT.md + decisions) | `A.init.2b` + `docs/CONTEXT.md` glossary | incorporated |
| `domain-modeling` (glossary) | `docs/CONTEXT.md` | incorporated |
| `tdd` (seams + anti-patterns) | `production_call_sites` + D.2 L-D2/L-D3 | content incorporated; driver stays `superpowers` (§11.2) |
| `to-spec` | design-doc authoring + grill→spec | incorporated |
| `to-tickets` | initializer spec→task-ledger decomposition | opi-native analog |
| `implement` | Phase C task loop | opi-native |
| `diagnosing-bugs` | `superpowers:systematic-debugging` (C.2) | covered via superpowers |
| `codebase-design` (seam vocabulary) | implicit in `production_call_sites` | partial |
| `wayfinder` (large-effort map) | phase design docs + roadmap | opi-native, not a wired skill |
| `triage` | `opi-audit` / `opi-remediate` (phase boundary) | partial |
| `prototype` | — | out of scope |
| `writing-for-agents` | used ad hoc for this design's skill edits | ad hoc |
| ask-matt / to-questionnaire / handoff / wait-what / writing-great-skills | — | not applicable |

### 11.2 TDD engine decision — keep `superpowers:test-driven-development`

C.1 stays `superpowers:test-driven-development`, not `mattpocock-skills:tdd`.
opi's verification layer already captures matt:tdd's high-value content — seam
discipline via `production_call_sites` + the Artifact Truthfulness Gate, and
anti-pattern coverage via D.2 L-D2 (tests-non-vacuous) + L-D3
(production-call-site-proven). Swapping would lose superpowers' stronger
verify-red enforcement and its in-loop refactor (matt exiles refactor to
code-review, which opi has no home for) — a regression — for marginal gain opi
already realises. A future full matt migration, if wanted, must be cohesive and
validated: `tdd` + `code-review` (refactor home) + `diagnosing-bugs` together.

### 11.3 `code-review` is the one real gap

opi has spec-compliance verification (D.2 evaluator, Artifact Truthfulness,
phase-exit audit) but no Matt-style `/code-review`: a pass that owns the refactor
step (Fowler smell baseline) and reviews the diff against the agreed seams. The
gap is latent while refactor lives inside C.1 (superpowers). If refactor is ever
moved out of C.1, `mattpocock-skills:code-review` (or equivalent) is its home.

### 11.4 Blend principle

`superpowers` stays the enforcement backbone (C.1 TDD, C.2 debugging) for its
factory-grade rigor; matt skills fold in where they add unique value opi lacks
(grilling at init/Phase B; CONTEXT.md domain language; a future code-review
refactor pass). This is a deliberate selective blend, not a replacement.

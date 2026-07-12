# opi skills

The `.claude/skills/opi-*` skills form the opi project's product lifecycle: from
spec, through implementation, independent audit, remediation, runtime regression,
documentation, to release. This README describes the end-to-end workflow and how
to use each skill.

There are seven opi-* skills: **opi-realign**, **opi-implement**, **opi-audit**,
**opi-remediate**, **opi-eval**, **opi-document**, and **opi-release**. They are
independent artifacts with strict ownership boundaries — each one states what it
does and, just as importantly, what it refuses to do.

> Scope note: this README covers the seven `opi-*` product skills only. The other
> skills in this directory (`caveman`, `grill-me`, `tdd`, `to-prd`, `prototype`,
> etc.) are general-purpose utilities unrelated to the opi lifecycle.

---

## The opi workflow

The lifecycle is a seven-phase spine with two side-branches. Each phase has an
entry condition and an exit gate; do not skip the gates.

| Phase | Skill | What happens |
|---|---|---|
| Phase 0 (optional) | `opi-realign` | strategic realignment vs a reference/upstream project |
| Phase 1 | manual | author `docs/opi-spec.md`; register the phase into `opi-implement` |
| Phase 2 | `opi-implement` | `--reinit` the ledger; evaluate its reasonableness |
| Phase 3 | `opi-implement` | per-task TDD loop; `compact` per task (operator convention: ultracode + GLM-5.2) |
| Phase 4 | `opi-audit` | multi-model, independent auditors |
| Phase 5 | `opi-remediate` | verify + fix; loop to pass |
| (eval gate) | `opi-eval` | runtime regression (real provider credits) |
| Phase 6 | `opi-document` | documentation + EN/ZH sync (guard-verified) |
| Phase 7 | `opi-release` | GitHub Releases + crates.io |

### Two cross-cutting patterns

- **Model independence.** The model that *evaluates* or *audits* an artifact
  should be **different** from the one that *built* it. Phase 2 (ledger
  evaluation), Phase 4 (audit), and Phase 5 (verification) all rely on this.
  Today model switching is manual — you change agent/model between phases.
  Automation is future work.
- **Context-bounded loops with `compact`.** Phases 3 and 5 are loops that exceed
  a single context window. The pattern is: do one unit of work, commit/record it,
  **`compact`** (or clear context), re-load only what matters, verify against the
  target (spec / audit reports), repeat until the exit gate passes.

### Phase 0 — Strategic realignment (optional, occasional)

**Skill:** `opi-realign`. Compare the opi implementation against a reference /
upstream project (e.g. `earendil-works/pi`) to detect architecture, feature,
design-philosophy, package-boundary, and roadmap drift *before* planning new
work. Use it when the spec or roadmap needs a reality check against upstream —
not every cycle.

- **Entry:** a target project path is available (or you provide one).
- **Exit:** a drift report with P0–P3 adjustment priorities; optional
  spec-adjustment addendum fed into Phase 1.

### Phase 1 — Requirements & spec authoring (manual, skill-independent)

No skill owns this. The user authors `docs/opi-spec.md` (and any PRDs) by hand,
then **registers** the new or changed phase work into `opi-implement`: the §15
roadmap tables for Phases 1–4, and the reviewed supplemental source registry in
`opi-implement/skill.md` for Phases 5–14. Iterate on the spec until it is stable.

- **Entry:** a product need.
- **Exit:** a spec section with success criteria, exit criteria, and a task
  roadmap; the phase work is registered into `opi-implement`.

### Phase 2 — Ledger initialization & evaluation loop

**Skill:** `opi-implement` (`--reinit` or first-time init). Parse the spec into
the `.opi-impl-state.json` task ledger (with inferred tier / commit type /
dependencies, composite-row decomposition, and a task-graph review gate). Then,
using a **different model** than the one that will implement, evaluate the
ledger's reasonableness: does task decomposition align with the spec? Are
boundaries covered cleanly? Is there redundancy, omission, or over-engineering?
Does every product success criterion own an acceptance scenario? Optimize the
ledger and re-init until the graph is stable.

- **Entry:** registered spec from Phase 1.
- **Exit gate:** task-graph review confirmed; every product success criterion is
  mapped to an owning task with an acceptance scenario; `spec_files` hashes are
  pinned.

### Phase 3 — Per-task implementation loop

**Skill:** `opi-implement`, run repeatedly (it auto-picks the lowest-ID
unblocked task). The operator convention for this project is to implement with
Claude Code **ultracode** + **GLM-5.2** — this is a workflow/model choice, not
something the skill encodes; `opi-implement` Phase C itself composes
`superpowers:test-driven-development` (red-green-refactor), with optional
parallel dispatch and `systematic-debugging` from attempt 3. Each task runs the
harness phases A→F: bootstrap, plan, implement via TDD, verify (tier gates +
Artifact Truthfulness Gate), commit with `Opi-*` footers, phase-exit check.
After each task commits, **`compact`**, then continue.

- **Entry:** finalized ledger from Phase 2.
- **Exit gate:** all tasks in the phase are `passing`; the phase-exit evaluator
  traces every success/exit criterion to `met` / `deferred-by-updated-design` /
  `not-met`; the phase is archived to `docs/snapshots/phase<N>/`.
- **Critical guardrails:** the harness never pushes commits, never publishes,
  never calls provider APIs, never edits `opi-spec.md`, never weakens tests to
  pass them, and never runs destructive git operations.

### Phase 4 — Independent audit (multi-model)

**Skill:** `opi-audit`. One or more **independent models** each audit the
finished phase: read the snapshot ledger + spec, audit across inferred
dimensions (Correctness, Security, Test quality, Spec compliance, Invariants,
Integration, Residuals), and write `docs/snapshots/phase<N>/audit.<model-id>.md`
with a Blocker/Major/Minor/Info finding set and a PASS / PASS-WITH-FINDINGS /
FAIL verdict.

- **Entry:** archived phase snapshot from Phase 3.
- **Independence rule:** an auditor must not read other audit reports or full
  evaluator transcripts for the phase before finishing its own.
- **Exit gate:** at least one audit report exists; ideally 2+ from different
  models so overlap validates real issues and divergence surfaces blind spots.

### Phase 5 — Remediation loop (context-bounded)

**Skill:** `opi-remediate`. Cross-reference all audit reports, normalize and
severity-unify findings, cluster by consensus (full / majority / unique), verify
each against the actual code (Confirmed / Partially confirmed / Cannot confirm /
Refuted), derive a dependency-layered `remediation-plan.md`, and — on explicit
opt-in — execute fixes layer by layer with per-layer verification gates.

Because the finding set can be large and context is limited, the **operator**
loops across context windows: run `opi-remediate`, **clear context**, re-load the
audit reports and current code, re-verify, repeat until the verification verdict
is pass. A single `opi-remediate` invocation is one forward pass through
dependency-ordered layers (each gated by `cargo fmt` / `clippy` / `test`),
ending with the workspace smoke script.

- **Entry:** audit report(s) from Phase 4.
- **Exit gate (workflow-level):** no open Blockers or Majors; every finding is
  either addressed by a fix or listed in the plan's Scope exclusions (Refuted /
  Deferred / Info / Duplicate).

### (eval gate) — Runtime regression

**Skill:** `opi-eval`. Before release, run the end-to-end regression eval:
compile opi, run structured cases against a real LLM provider, collect NDJSON
traces, dispatch a readonly evaluator, write `docs/eval/<version>-<date>-<model>.md`
and append `docs/eval/history.jsonl`. This catches runtime fidelity regressions
that a static audit cannot. It costs real API credits and never auto-fires.

- **Entry:** runtime changes merged; provider credentials configured.
- **Exit:** report written; regressions (if any) fed back to Phase 5.

### Phase 6 — Documentation & EN/ZH sync

**Skill:** `opi-document`. Refresh opi docs so they stay truthful to the shipped
code and the English/Chinese mirrors stay in sync, editing **inside** the eight
doc-guard suites rather than around them. Use it for a full phase doc refresh, a
targeted update after a change, or a version-bump doc-resync.

- **Entry:** Phases 3–5 (implement / audit / remediate) passing; or an ad-hoc
  doc-change / translation request.
- **Exit gate:** every guard suite (`productized_packages_docs`,
  `phase11_tooling_quality_docs`, `phase12_provider_correctness_docs`,
  `phase13_session_context_docs`, `observability_docs`, `runtime_contract_docs`,
  `transport`) reports `0 failed` for EN + ZH; no unintended phase-jargon
  remains; version-bearing lines moved in lockstep if a version bumped.
- **Critical guardrails:** never drops a guard-pinned token; never introduces a
  forbidden overclaim phrase outside a negation; re-syncs the phase4 spec-hash
  ledger if it touches `docs/opi-spec.md`; does not edit code, `Cargo.toml`, or
  version (that is `opi-release`'s job); does not weaken guard tests.

### Phase 7 — Release

**Skill:** `opi-release <version> [--fix] [--skip-cross]`. Run the seven-phase
publish pipeline — pre-flight, version bump, changelog, build, commit/tag/push +
draft GitHub Release, crates.io publish, publish draft + verify — to ship to
GitHub Releases and crates.io. Reversibility decreases as it progresses: Phases
1–4 are local and reversible, Phase 5 is partially reversible (commit/tag are
public on push), Phase 6 (crates.io) is irreversible. Explicit user confirmation
gates appear throughout.

- **Entry:** a release-ready workspace (Phases 3–5 pass, eval clean).
- **Exit:** published GitHub Release + crates.io versions; release report.

---

## Shared contracts

- **`.opi-impl-state.json`** (gitignored, repo root) — `opi-implement`'s live
  task ledger. `opi-remediate` and `opi-audit` only ever *read* it — and both
  read the **frozen per-phase snapshot** at `docs/snapshots/phase<N>/opi-impl-state.json`,
  not the live repo-root file. No other skill writes it.
- **`docs/snapshots/phase<N>/`** — frozen per-phase archive: a snapshot of
  `opi-impl-state.json`, `audit.<model-id>.md` reports, and `remediation-plan.md`.
- **`Opi-*` commit footers** (`Opi-Task`, `Opi-DoD-SHA256`, `Opi-Verification`,
  `Opi-Evaluator`, `Opi-Acceptance`) — make task completion reconstructable from
  git history without the ledger.
- **`.opi-release-state.json`** (repo root) — `opi-release`'s resume state,
  distinct from the implementation ledger.
- **`docs/eval/`** — `opi-eval` reports and `history.jsonl`.

---

## Per-skill reference

### opi-realign

Compare the current implementation against a target/reference project and
produce an architecture, feature, design-philosophy, package-boundary, and
roadmap realignment review.

- **When to invoke:** "realign", "audit drift", "compare a port/reimplementation",
  "check whether planned phases match an upstream project", "evaluate
  cross-language architecture against a target project path", or supply a target
  project path to compare against.
- **Inputs:** `target=<path>` (required; the skill asks if omitted). Optional:
  `current=<path>`, `current_label`, `target_label`, `scope=<text>`.
- **What it does:** builds evidence inventories for both projects; compares
  *semantics* (not file shapes) across architecture, runtime, data formats,
  provider/integration surfaces, extension model, tests, docs; classifies drift
  (Aligned / Intentional divergence / Partial / Missing / Overreach / Risk);
  recommends P0–P3 adjustments; for large audits writes a local report file and
  summarizes the highest-signal findings in chat.
- **What it does NOT do:** does not claim compatibility without evidence; does
  not treat target-project breadth as automatically desirable; does not copy
  target-language architecture when it conflicts with current-language norms;
  does not modify source/specs/roadmaps or commit unless you explicitly ask.
- **Artifacts:** reads guidance files, manifests, source topology, tests,
  roadmap artifacts in both projects; writes a report file (HTML or markdown)
  and, only if you ask for edits, spec files + a spec-adjustment addendum.
- **In the workflow:** Phase 0 (optional side-branch).

### opi-implement

Long-running-agent harness that drives implementation of `docs/opi-spec.md`
tasks and the reviewed supplemental Phase 5–14 specs, one task at a time, with
TDD, tiered verification, documentation guards, and JSON-ledger checkpointing.
This is a **harness**, not a coding assistant — it encodes opinions about state,
evidence, failure recovery, and escalation, and refuses to act if a rule would
be violated.

- **When to invoke:** "implement", "resume", "verify", "progress", or to check
  status, reinitialize the ledger, resume interrupted work, clear a blocker, or
  auto-select the next unblocked task. Not for merely reading or discussing specs.
- **Inputs / commands:**
  - `opi-implement` — auto-pick the next unblocked task.
  - `opi-implement <task-id>` — run a specific task (validates dependencies).
  - `opi-implement --status` — ledger summary (task table, phase, drift, blockers).
  - `opi-implement --reinit` — re-parse the spec and reconcile the ledger.
  - `opi-implement <task-id> --resume-from-manual` — verify a manual commit.
  - `opi-implement <task-id> --extend-cap <N>` — raise the iteration cap.
  - `opi-implement --clear-blocker <id> --because <text>` — unblock a task.
  - Requires `cargo` (Rust ≥ 1.97) and `git`.
- **What it does:** six-phase invocation per task — A Bootstrap, B Plan (print
  DoD + tier + acceptance scenarios + call sites + forbidden-scope guards, user
  gate), C Implement (TDD red-green-refactor, optional parallel dispatch,
  systematic debugging by attempt 3), D Verify (product acceptance D.0, Artifact
  Truthfulness Gate D.0a, tier gates D.1, risk evaluator D.2, cross-cutting gates
  D.3), E Commit + ledger update (Conventional commit with `Opi-*` footers),
  F Phase-Exit check. Infers task metadata on init/reinit; enforces a spec-hash
  alignment guard; runs tiered verification — **six tiers** (workspace /
  documentation / library / cli-tool / cli-runtime / tui) plus **conditional
  addenda** applied on top (provider-contract, multimodal, product acceptance).
- **What it does NOT do:** does not edit `opi-spec.md` (except a reviewed
  doc task that owns it), does not push commits/tags, does not publish or open
  PRs/releases, does not call provider APIs, does not delete or weaken tests,
  does not bypass clippy crate-wide or auto-accept TUI snapshots, does not run
  `git restore`/`clean`/`reset`/`--no-verify`/`--force`/`git add -A`, and does
  not satisfy a DoD with stubs or TODOs.
- **Artifacts:** reads `docs/opi-spec.md` §15 + reviewed Phase 5–14 sources;
  writes `.opi-impl-state.json` (gitignored), phase snapshots under
  `docs/snapshots/phase<N>/`, and task commits carrying `Opi-*` footers.
- **In the workflow:** Phases 2 (init) and 3 (implement loop).
- **Notes:** full-workspace smoke is expensive and has filled the host disk
  before — for library-tier tasks prefer the per-task library gates with
  `CARGO_INCREMENTAL=0`. On this Windows host use `python` not `python3`.

### opi-audit

Perform an independent, phase-level code audit of a specific opi implementation
phase: compare the design spec against the actual implementation and produce a
structured findings report with severity classifications.

- **When to invoke:** "audit", "code review", "review phase N", "compare spec
  and implementation", "check spec compliance", "find implementation gaps", or
  the Chinese triggers 审计 / 审查.
- **Inputs:** `phase=<N>` (required; the skill asks if omitted). Optional:
  `focus=<text>` to weight specific dimensions while still covering the basics.
- **What it does:** reads the **snapshot** ledger at
  `docs/snapshots/phase<N>/opi-impl-state.json` (not the live one) and the spec
  files it references; infers applicable audit dimensions and briefly confirms
  them (plus focus areas and any adds/drops) with the user; deep-reads affected
  source/test/doc files (recommends parallel subagents for large phases); audits
  each dimension; classifies findings Blocker / Major / Minor / Info; writes
  `docs/snapshots/phase<N>/audit.<model-id>.md` with an executive summary and
  PASS / PASS-WITH-FINDINGS / FAIL verdict.
- **What it does NOT do:** does not modify code/specs/tests/docs unless asked
  (it is an audit, not a fix-up); does not read other `audit.*.md` reports or
  full evaluator transcripts for the phase before finishing — the structural
  `evaluator_summary` field in `phase_exit` is the only exception (independence);
  does not reduce depth because other reports exist; does not treat every spec
  deviation as a defect.
- **Artifacts:** reads the snapshot ledger + spec files + `CLAUDE.md`/`AGENTS.md`
  + `docs/opi-spec.md`; writes `audit.<model-id>.md`.
- **In the workflow:** Phase 4.
- **Notes:** the `<model-id>` in the filename is self-determined from the
  auditing model's identity (e.g. `opus4.6`, `codex`, `glm5.2`, `gpt5.5`); the
  skill asks if uncertain. Handles both snapshot-schema v1 (`spec_path`) and v2
  (`spec_files`).

### opi-remediate

Cross-reference, verify against actual code, and remediate findings from
independent audit reports for a specific phase, producing a layered remediation
plan with optional user-gated execution.

- **When to invoke:** "remediate phase N", "verify audit findings", "fix audit
  issues", "confirm audit", or the Chinese triggers 修复审计 / 验证审计发现 /
  审计修复, or a request to act on `docs/snapshots/phase<N>/audit.*.md`.
- **Inputs:** `phase=<N>` (required). Optional: `scope=<text>`; `execute=<bool>`
  (default `false` — plan only; continue into execution when `true` or on opt-in
  after plan review).
- **What it does:** Phase A acquire (all `audit.*.md` + snapshot ledger + specs);
  Phase B cross-reference (normalize findings, severity-unify, cluster by
  consensus; single-report mode when only one audit exists); Phase C verify each
  finding against the full source file (Confirmed / Partially confirmed /
  Cannot confirm / Refuted); Phase D design decisions (auto-decide the obvious
  cases, escalate the rest with labeled options); Phase E derive a
  dependency-layered `remediation-plan.md`; Phase F (gated) execute fixes layer
  by layer with per-layer `cargo fmt`/`clippy`/`test` gates, then workspace smoke.
- **What it does NOT do:** does not write `.opi-impl-state.json`; does not
  refactor or improve code outside finding scope; does not add features; does
  not run `git reset --hard`/`checkout .`/`clean -fd`/`add -A`; does not advance
  to the next layer when a previous layer fails after two fix attempts; Phase F
  is never automatic.
- **Artifacts:** reads `audit.*.md` + snapshot ledger + specs +
  `cargo metadata`; writes `remediation-plan.md`; Phase F modifies only the
  source/test/doc files targeted by verified findings.
- **In the workflow:** Phase 5.
- **Notes:** when docs have a `.zh.md` counterpart, both EN and ZH must be
  updated in the same change.

### opi-eval

End-to-end regression eval for the opi runtime: compile opi, run structured test
cases against a real LLM provider, collect NDJSON runtime traces, and dispatch a
readonly evaluator subagent to detect fidelity degradation.

- **When to invoke:** user invocation only — the frontmatter sets
  `disable-model-invocation: true` and the skill consumes real API credits.
  Natural triggers: "eval", "regression", "runtime fidelity".
- **Inputs:** `model=<provider:model>` (optional; defaults to opi's default
  resolution; always recorded); `cases=<name,...>` or `all` (default). Requires
  real provider credentials.
- **What it does:** Step 1 clean-build `opi-coding-agent` in release; Step 2 run
  each case in an isolated temp workspace capturing `output.ndjson`; Step 3 parse
  signals (tool calls, compaction, retries, final answer, tokens, cost,
  diagnostics); Step 4 dispatch a **readonly** evaluator that scores each case on
  six dimensions (answer correctness, tool-call correctness, context integrity,
  chain efficiency, resource consumption, error handling); Step 5 write
  `docs/eval/<version>-<date>-<model-short>.md` and append `docs/eval/history.jsonl`
  with a version-delta section. Ships three built-in cases (`candy`,
  `tool_chain`, `context_retention`).
- **What it does NOT do:** does not modify opi source; does not execute anything
  in the evaluator subagent; does not write fixtures into the workspace root;
  does not suggest code changes (diagnosis only); does not fire without explicit
  invocation; does not abort the whole eval on one case crash.
- **Artifacts:** writes `docs/eval/<version>-<date>-<model-short>.md` and
  `docs/eval/history.jsonl`; reads `docs/eval/history.jsonl` and optional
  `pi-baseline.jsonl`.
- **In the workflow:** eval gate (side-branch, before release).
- **Notes:** the first run of any case establishes the resource baseline and
  cannot fail; only subsequent runs are scored against the 1.5×/3× thresholds.
  Add a new case by appending a `## Case N:` section to `test-cases.md`.

### opi-document

Refresh opi documentation so it stays truthful to the shipped code, with the
English and Simplified-Chinese mirrors kept in sync and the doc-guard suites
green. This is the dedicated Phase 6 skill (previously manual).

- **When to invoke:** "update the docs/README", "refresh the README", "sync
  EN/ZH", "translate the README", "fix doc drift", the Chinese triggers
  文档更新 / 文档同步 / 更新 README / 翻译, or at Phase 6 of the opi workflow
  (post-implementation, pre-release).
- **Inputs:** `scope=<full|targeted|version-bump>` (default targeted);
  `files=<...>`; `version=<X.Y.Z>` for a version bump.
- **What it does:** seven phases — discover the doc delta; audit docs for
  drift/noise/gaps against source; load the doc-guard constraints; decide
  guard-safe scope; edit EN docs; mirror to ZH surgically (composing
  `baoyu-translate` for net-new prose, preserving pinned ZH tokens verbatim);
  verify by running the eight guard suites plus a phase-jargon grep.
- **What it does NOT do:** no code/`Cargo.toml`/version changes; no commits or
  releases; no authoring of `opi-spec.md` normative content (only doc-sync
  edits, which re-sync the phase4 ledger); no weakening of guard tests; no
  free-regeneration of Chinese docs.
- **Artifacts:** reads the affected docs + `CHANGELOG.md` + crate `src/` + the
  guard-test files + baoyu-translate `EXTEND.md`; writes the docs (EN + ZH) and,
  on an `opi-spec.md` edit, the re-synced phase4 ledger hash.
- **In the workflow:** Phase 6.
- **Notes:** the doc-guard constraints live in
  `opi-document/references/doc-guards.md`.

### opi-release

Orchestrate the full release process for the opi Rust workspace — publish to
GitHub Releases and crates.io through seven phased safety gates, each requiring
user confirmation.

- **When to invoke:** "release <version>", "opi-release <version>", "ship
  version <version>", "publish opi <version>".
- **Inputs:** `<version>` (required semver); `--fix` (auto-fix fmt/clippy during
  pre-flight); `--skip-cross` (source-only release). Requires a clean tree on
  `main`, `cargo`/`git`/`gh`, and crates.io auth (`~/.cargo/credentials.toml` or
  `$CARGO_REGISTRY_TOKEN`).
- **What it does:** Phase 1 pre-flight (files, git state, CI, fmt/clippy/test/doc,
  audit, secret scan, metadata, package content, version semantics, `--version`
  command); Phase 2 bump workspace version + internal dep versions + dry-run
  publish; Phase 3 generate CHANGELOG + release notes from Conventional Commits;
  Phase 4 build (CI-driven recommended, or local `cross`, or `--skip-cross`) +
  package artifacts + checksums + self-check; Phase 5 commit/tag/push + draft
  GitHub Release (stages only `Cargo.toml`/`Cargo.lock`/`CHANGELOG.md`); Phase 6
  publish to crates.io in topological order; Phase 7 publish the draft + verify
  install. Resume support via `.opi-release-state.json`.
- **What it does NOT do:** does not run live provider/dogfood checks (pre-flight
  is deterministic); does not upload `SHA256SUMS.txt` to the release; does not
  auto-retry explicit cargo errors; does not auto-proceed to crates.io without
  approval; does not hardcode the publish order; does not use
  `git reset --hard` + force-push for rollback (uses `git revert` + tag delete);
  does not treat yank as deletion.
- **Artifacts:** reads/writes `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`,
  release notes, `release-artifacts/v$VERSION/`, `.opi-release-state.json`, the
  release commit + tag, the GitHub Release, and crates.io versions.
- **In the workflow:** Phase 7 (terminal).
- **Notes:** irreversibility boundary — commit/tag are public on push (Phase 5);
  crates.io publish is permanent (Phase 6). On this host `git push`/`cargo
  publish` can need retries (SSL drops over the proxy); verify a push with
  `git ls-remote`, not `gh api`.

---

## Gaps & future work

- **Model/agent switching** between phases (evaluation in Phase 2, auditing in
  Phase 4, verification in Phase 5) is currently manual. Automating multi-model
  orchestration is future work.

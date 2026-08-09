# Opi Workflow Skill System Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` to
> execute this plan task by task. This repository already has unrelated dirty
> files; preserve them and never use broad staging, reset, checkout, clean, or
> stash operations.

**Goal:** Rework the repository's `opi-*` skill system so pi alignment and
outward research remain distinct, human-led shaping feeds an adversarial plan
admission gate, Matt skills provide semantic subskills, and Superpowers remains
only for bounded operational enforcement.

**Architecture:** Add a thin `opi-workflow` router and an outward-facing
`opi-research` wrapper. Keep `opi-implement` as the only delivery state machine,
move design correction out of task execution, compose Matt review semantics into
assurance, and reduce always-loaded documentation to stable routing pointers.

**Tech stack:** Markdown skill contracts, YAML Codex sidecars, the existing
Claude Workflow JavaScript DSL, PowerShell verification, and Git read-only diff
inspection.

**Design source:**
`docs/superpowers/specs/2026-08-09-opi-workflow-skill-system-optimization-design.md`

## Global constraints

- Do not modify Rust product code, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`,
  root `README.md`, or root `README.zh.md`.
- Do not create commits, stage files, push, open a PR, or publish anything.
- Preserve all pre-existing working-tree changes. Before editing a dirty file,
  inspect its current diff and integrate rather than overwrite it.
- Use `apply_patch` for every repository file edit.
- Keep `.claude/skills/README.md` and `.claude/skills/README.zh.md` synchronized.
- Keep root `AGENTS.md` and `CLAUDE.md` synchronized.
- Matt skills are referenced by unqualified names. Superpowers skills are
  always qualified as `superpowers:<name>`.
- Do not vendor Matt or Superpowers skill contents. State the local composition
  contract and require the real subskill to be opened before a load-bearing
  call.
- `.opi-impl-state.json` remains schema v2 and is not modified by this plan.
- The current active and archived implementation ledger is not reinitialized.
- No new implementation tracker, worktree protocol, or commit protocol may be
  introduced.

## File map

### New files

- `.claude/skills/opi-workflow/SKILL.md` — user-invoked routing and phase
  boundaries only.
- `.claude/skills/opi-workflow/agents/openai.yaml` — Codex explicit-invocation
  metadata.
- `.claude/skills/opi-research/SKILL.md` — opi-specific outward research
  contract composing Matt `research`.
- `.claude/skills/opi-research/agents/openai.yaml` — Codex explicit-invocation
  metadata.
- `.claude/skills/_shared/references/finding-contract.md` — normalized audit and
  eval finding interchange consumed by remediation.
- `agents/openai.yaml` under each existing `opi-*` skill that lacks one.

### Existing files with focused changes

- `.claude/skills/opi-implement/skill.md` — source admission, subskill selection,
  and stop/return boundaries.
- `.claude/skills/opi-implement/references/initializer.md` — draft-before-write
  plan flow and four verdicts.
- `.claude/skills/opi-implement/references/verify-engine.md` — design and graph
  adversarial review contract.
- `.claude/skills/opi-implement/references/ledger-schema.md` — correct six-crate
  language and task evidence semantics without a schema bump.
- `.claude/skills/opi-implement/references/anti-patterns.md` — remove stale
  subskill and silent-spec-amend guidance.
- `.claude/skills/opi-implement/scripts/plan.workflow.js` — return findings and
  admission verdicts without auto-folding the draft.
- `.claude/skills/opi-audit/SKILL.md` and
  `.claude/skills/opi-audit/references/finding-template.md` — Matt two-axis review
  plus opi dimensions and normalized findings.
- `.claude/skills/opi-remediate/SKILL.md` and its references — accept audit and
  eval finding sources and use shared contracts.
- `.claude/skills/opi-eval/SKILL.md` and its references — isolated builds,
  truthful evaluator independence, and normalized regression findings.
- `.claude/skills/opi-document/SKILL.md` and its verification reference — a
  fast source-derived check and Matt `writing-for-agents` composition.
- `.claude/skills/opi-release/skill.md` — safe staging/rollback, six-crate
  topology, version-doc synchronization, and release checksums.
- `.claude/skills/opi-realign/SKILL.md` and its audit framework — inward-only
  scope, exact pi revision, and bounded concurrency.
- `.claude/skills/opi-slim-tests/skill.md` — verified stop without automatic
  commit.
- `.claude/skills/_shared/references/finding-contract.md` — the only shared
  cross-skill schema; severity is co-located and Git safety stays in root
  guidance.
- `.claude/skills/README.md` and `.claude/skills/README.zh.md` — synchronized
  workflow maps.
- `AGENTS.md` and `CLAUDE.md` — concise routing pointer and corrected crate
  graph.

---

### Task 1: Capture the dirty-tree baseline and protect existing work

**Files:** Read only; create no files.

- [ ] **Step 1: Record the complete current status**

Run:

```powershell
git status --short
```

Expected: the existing Phase 16 remediation and skill-maintenance files remain
visible, plus the approved design and this plan. Do not attempt to clean them.

- [ ] **Step 2: Inspect the pre-existing diff of every in-scope dirty file**

Run:

```powershell
git diff -- .claude/skills .gitignore AGENTS.md CLAUDE.md
```

Expected: identify the existing Git-safety, severity, eval independence,
verification-tier, release, and README edits that must be retained.

- [ ] **Step 3: Confirm protected out-of-scope files**

Run:

```powershell
git status --short -- Cargo.toml Cargo.lock CHANGELOG.md README.md README.zh.md crates
```

Expected: no changes caused by this plan. If any already exist, record them as
pre-existing and do not touch them.

---

### Task 2: Add the workflow router and outward research skill

**Files:**

- Create: `.claude/skills/opi-workflow/SKILL.md`
- Create: `.claude/skills/opi-workflow/agents/openai.yaml`
- Create: `.claude/skills/opi-research/SKILL.md`
- Create: `.claude/skills/opi-research/agents/openai.yaml`

- [ ] **Step 1: Demonstrate the missing entry points**

Run:

```powershell
Test-Path .claude/skills/opi-workflow/SKILL.md
Test-Path .claude/skills/opi-research/SKILL.md
```

Expected before editing:

```text
False
False
```

- [ ] **Step 2: Create the `opi-workflow` router contract**

Use `apply_patch` to create a user-invoked skill with this frontmatter shape:

```yaml
---
name: opi-workflow
description: Route opi work among inward pi realignment, outward capability research, human-led shaping, delivery, assurance, documentation, and release without creating a second state machine.
disable-model-invocation: true
---
```

The body must contain the exact routing distinctions from the design:

```text
pi delta                         -> opi-realign
external/non-pi capability       -> opi-research
large multi-session fog          -> wayfinder
bounded decision ambiguity       -> grill-with-docs
settled decisions lacking a spec -> to-spec
reviewed registered source       -> opi-implement plan
hard bug/performance regression  -> diagnosing-bugs
completed phase                  -> opi-audit -> opi-remediate
release candidate                -> opi-eval -> opi-document -> opi-release
```

State explicitly that the router opens the selected real skill, does not carry
load-bearing summaries, and stops at human decision/irreversible boundaries.

- [ ] **Step 3: Create the `opi-research` contract**

Use `apply_patch` to create a user-invoked skill with this frontmatter shape:

```yaml
---
name: opi-research
description: Research capabilities beyond or poorly served by pi, using Matt research against primary sources and evaluating Rust feasibility plus plugin-first placement for the opi ecosystem.
disable-model-invocation: true
---
```

Require it to invoke Matt `research` and write
`docs/research/YYYY-MM-DD-<topic>.md`. The report contract must include:

```text
question
relationship_to_pi
primary_source_findings
alternatives
rust_feasibility
existing_extension_fit
smallest_missing_core_seam
placement_candidates
unresolved_decisions
limitations_and_non_findings
```

The skill must not modify a spec, select the product direction, or treat its
placement recommendation as approved.

- [ ] **Step 4: Add Codex explicit-invocation sidecars**

Use this exact schema for both skills, changing only display text:

```yaml
interface:
  display_name: "Opi Workflow"
  short_description: "Route work through the opi lifecycle"
policy:
  allow_implicit_invocation: false
```

```yaml
interface:
  display_name: "Opi Research"
  short_description: "Research outward plugin capabilities"
policy:
  allow_implicit_invocation: false
```

- [ ] **Step 5: Verify the inward/outward boundary**

Run:

```powershell
rg -n "inward|pi|opi-realign|outward|plugin|Matt.*research|must not|does not" `
  .claude/skills/opi-workflow/SKILL.md `
  .claude/skills/opi-research/SKILL.md
```

Expected: both routes are present; `opi-research` composes Matt `research`; no
text makes research a child phase of realignment.

---

### Task 3: Turn `opi-implement plan` into an adversarial admission gate

**Files:**

- Modify: `.claude/skills/opi-implement/skill.md`
- Modify: `.claude/skills/opi-implement/references/initializer.md`
- Modify: `.claude/skills/opi-implement/references/verify-engine.md`
- Modify: `.claude/skills/opi-implement/references/ledger-schema.md`
- Modify: `.claude/skills/opi-implement/references/anti-patterns.md`
- Modify: `.claude/skills/opi-implement/scripts/plan.workflow.js`

- [ ] **Step 1: Capture the current conflicting behaviors**

Run:

```powershell
rg -n "Spec-amend|A\.init\.2b|grilling|superpowers:test-driven-development|superpowers:systematic-debugging|confirmed_folds|five crates|E:\\opi-target" `
  .claude/skills/opi-implement
```

Expected before editing: live-spec amendment, task-level semantic TDD/debugging
drivers, auto-fold behavior, and stale environment/crate wording are visible.

- [ ] **Step 2: Replace the plan sequence in `initializer.md`**

Use `apply_patch` to express this exact sequence:

```text
P.0 Source admission
  failure -> RESEARCH_REQUIRED or DESIGN_DECISION_REQUIRED; canonical ledger unchanged
P.1 Draft graph
  write/update only .opi-impl-state.draft.json
P.2 Fresh-context adversarial review
  axes: design_readiness and execution_readiness
  reviewer reports; reviewer does not mutate source or draft
P.3 Verdict
  READY | RESEARCH_REQUIRED | DESIGN_DECISION_REQUIRED | GRAPH_REVISION_REQUIRED
P.4 Human graph confirmation
P.5 Atomic canonical ledger write
```

Remove the initializer grilling pass that amends the live spec. When source
meaning is incomplete, return to `wayfinder`, `grill-with-docs`,
`opi-research`, or `opi-realign` as appropriate.

- [ ] **Step 3: Strengthen task graph readiness without a schema bump**

Update `initializer.md` and `ledger-schema.md` so existing fields carry the new
contract:

```text
acceptance_scenarios[].scenario  -> demonstrable outcome
acceptance_scenarios[].source    -> reviewed source criterion
production_call_sites            -> real production path
verification.behavioral_tests    -> agreed public test seam evidence
inference_notes                   -> seam/placement rationale when inferred
```

Do not add a new ledger field. Correct the `tasks[].crate` prose from five
crates to six workspace crates plus open packaging identifiers.

- [ ] **Step 4: Rewrite the plan workflow result contract**

Modify `plan.workflow.js` so the two top-level review axes are
`design-readiness` and `execution-readiness`. Sub-lenses may remain bounded
within those axes.

Replace `confirmed_folds` with a non-mutating result:

```javascript
return {
  verdict,
  design_findings,
  graph_findings,
  flagged_for_human,
  rejected,
  report,
}
```

The verdict enum must be exactly:

```javascript
['READY', 'RESEARCH_REQUIRED', 'DESIGN_DECISION_REQUIRED', 'GRAPH_REVISION_REQUIRED']
```

Do not auto-apply a reviewer suggestion. Preserve the existing dirty change
that normalizes string/object workflow arguments.

- [ ] **Step 5: Select Matt semantic subskills in `skill.md`**

Replace the execution composition with:

```text
Phase C feature/bug implementation -> Matt tdd
Hard bug or performance feedback loop -> Matt diagnosing-bugs
Pre-completion evidence gate -> superpowers:verification-before-completion
Disjoint task sub-units only -> superpowers:dispatching-parallel-agents
```

Require `tdd` to confirm the public seam before the first test and work one
vertical red/green slice at a time. Remove `superpowers:brainstorming` as a
failure handler; unresolved product meaning returns to shaping.

- [ ] **Step 6: Update anti-patterns and verify-engine reference**

Add explicit anti-patterns:

```text
silent source amendment
reviewer mutates the draft it reviews
auto-folding adversarial findings
second task/commit/worktree state machine
unconfirmed public test seam
horizontal task graph disguised as dependencies
```

Remove any auto-deep classifier that claims to use persisted section hashes
that the ledger does not store. Capability detection may select a bounded
multi-agent workflow or a single fresh reviewer, but both must return the same
schema and disclose degraded independence.

- [ ] **Step 7: Verify the JavaScript and contract changes**

Run:

```powershell
node --check .claude/skills/opi-implement/scripts/plan.workflow.js
rg -n "READY|RESEARCH_REQUIRED|DESIGN_DECISION_REQUIRED|GRAPH_REVISION_REQUIRED|design-readiness|execution-readiness" `
  .claude/skills/opi-implement
rg -n "confirmed_folds|superpowers:test-driven-development|superpowers:systematic-debugging|Spec-amend procedure|five crates|E:\\opi-target" `
  .claude/skills/opi-implement
```

Expected: syntax check exits 0; required verdicts and axes exist; the final
search has no active-contract hits. Historical discussion is not added.

---

### Task 4: Normalize audit, eval, and remediation findings

**Files:**

- Create: `.claude/skills/_shared/references/finding-contract.md`
- Modify: `.claude/skills/opi-audit/SKILL.md`
- Modify: `.claude/skills/opi-audit/references/finding-template.md`
- Modify: `.claude/skills/opi-remediate/SKILL.md`
- Modify: `.claude/skills/opi-remediate/references/cross-reference-matrix.md`
- Modify: `.claude/skills/opi-remediate/references/execution-protocol.md`
- Modify: `.claude/skills/opi-remediate/references/remediation-plan-template.md`
- Modify: `.claude/skills/opi-eval/SKILL.md`
- Modify: `.claude/skills/opi-eval/references/report-template.md`
- Reconcile: `.claude/skills/_shared/references/finding-contract.md`

- [ ] **Step 1: Show the missing interchange and manual handoff**

Run:

```powershell
Test-Path .claude/skills/_shared/references/finding-contract.md
rg -n "manual|hand.*remediate|audit\.\*\.md|different model family|cargo clean" `
  .claude/skills/opi-audit `
  .claude/skills/opi-remediate `
  .claude/skills/opi-eval
```

Expected before editing: no shared finding contract; remediation assumes audit
files; eval requires manual transcription and a clean build.

- [ ] **Step 2: Define the normalized finding contract**

Create a shared Markdown reference with these required fields:

```yaml
id: <source-stable identifier>
source_kind: audit | eval
source_path: <repo-relative path>
source_model: <reported identity>
independence: independent-family | fresh-context-same-family | unknown
axis: standards | spec | security | test-quality | invariants | integration | residuals | runtime-fidelity
severity: Blocker | Major | Minor | Info
title: <concise finding>
claim: <falsifiable problem statement>
evidence:
  - location: <file:line, trace event, or command>
    detail: <observed evidence>
criterion_source: <spec/rule citation or null>
reproduction: [<commands or eval case>]
confidence: high | medium | low
status: unverified
```

State that remediation preserves source fields, assigns its own verification
status separately, and never silently reranks the original severity.

- [ ] **Step 3: Compose Matt `code-review` into `opi-audit`**

At the ledger-derived fixed commit range, invoke Matt `code-review` for separate
Standards and Spec reports. Add this reviewer restriction verbatim:

```text
Do not invoke code-review, opi-audit, or spawn additional agents.
```

Then run opi-specific Security, Test quality, Invariants, Integration, and
Residuals dimensions. Keep axes separate in the report and emit normalized
finding blocks using the finding contract's severity scale.

- [ ] **Step 4: Let remediation acquire both source kinds**

Change remediation inputs to accept explicit `sources=<path,...>` with the
default still resolving phase `audit.*.md` files. Permit normalized finding
blocks from eval reports. Keep source identity through cross-reference,
verification, consensus grouping, and execution.

Replace forbidden rollback examples such as `git checkout -- <file>` with the
always-loaded root Git rules. A pre-existing red baseline must produce a user
decision or scoped exclusion; it must not be normalized as a passing gate.

- [ ] **Step 5: Make eval outputs directly consumable**

Remove `cargo clean`. Require a persistent per-worktree/toolchain
`CARGO_TARGET_DIR` outside the repo and a release build in that directory.
Record the actual evaluator relationship
as one of the shared independence values. Add a normalized finding section for
confirmed runtime regressions; do not suggest or execute fixes.

- [ ] **Step 6: Verify finding flow and root Git-rule consistency**

Run:

```powershell
rg -n "source_kind|independence|runtime-fidelity|criterion_source|status: unverified" `
  .claude/skills/_shared/references/finding-contract.md `
  .claude/skills/opi-audit `
  .claude/skills/opi-remediate `
  .claude/skills/opi-eval
rg -n "git checkout --|cargo clean|manual.*remediat" `
  .claude/skills/opi-remediate `
  .claude/skills/opi-eval
```

Expected: normalized fields are shared across all three skills; forbidden
rollback, clean-build, and manual-transcription instructions are absent.

---

### Task 5: Correct document, release, realign, and test-slimming contracts

**Files:**

- Modify: `.claude/skills/opi-document/SKILL.md`
- Modify: `.claude/skills/opi-document/references/documentation-checks.md`
- Modify: `.claude/skills/opi-release/skill.md`
- Modify: `.claude/skills/opi-realign/SKILL.md`
- Modify: `.claude/skills/opi-realign/references/audit-framework.md`
- Modify: `.claude/skills/opi-slim-tests/skill.md`
- Reconcile: root `AGENTS.md` / `CLAUDE.md` Git rules

- [ ] **Step 1: Capture active contradictions**

Run:

```powershell
rg -n "Eight|eight|SHA256SUMS|git checkout --|git add \$\(git diff|TaskCreate|AskUserQuestion|cargo clean|four crates|five crates|36|one measurer|commit" `
  .claude/skills/opi-document `
  .claude/skills/opi-release `
  .claude/skills/opi-realign `
  .claude/skills/opi-slim-tests
```

Expected before editing: guard-count, release, orchestration, concurrency, or
automatic-commit wording requiring classification and correction.

- [ ] **Step 2: Correct documentation composition**

Make all doc-guard references say ten suites and list the same ten names. Add
Matt `writing-for-agents` as a required reference for agent-facing docs:

```text
environment is the source of truth
root guidance points to deeper workflow docs
do not repeat easy repository lookups
state completion criteria and no-op conditions
```

Refer to translation by installed skill name, not a host path.

- [ ] **Step 3: Harden release workflow**

Apply these exact decisions:

```text
Phase 2/3 are locally reversible but have file side effects.
Version documentation is synchronized through opi-document scope=version-bump.
Only explicitly enumerated release-owned files are staged.
Rollback uses git revert and tag deletion after publication; no checkout/reset/force.
Publish order is computed from cargo metadata for all six publishable crates.
SHA256SUMS.txt is uploaded with release artifacts, matching docs/opi-spec.md.
Commands have PowerShell and POSIX forms where shell syntax differs.
Progress tracking and user confirmation are described semantically, not as TaskCreate/AskUserQuestion calls.
```

Correct the final release report so it covers all six workspace crates.

- [ ] **Step 4: Tighten inward-only realignment**

Require the exact pi revision in every report and state that `opi-realign` does
not perform outward capability research. Replace unbounded per-dimension fanout
with batches limited by the available concurrency slots. Keep priority
classification optional unless requested; do not claim all reports contain
P0–P3 by default.

- [ ] **Step 5: Remove automatic commit from test slimming**

End `opi-slim-tests` after targeted verification and a diff summary. Offer a
commit only if the user separately requests one. Remove shell concatenation as
a prescribed editing mechanism; describe semantic merge and byte-preservation
instead.

- [ ] **Step 6: Verify the corrected contracts**

Run:

```powershell
rg -n "ten|writing-for-agents|scope=version-bump|SHA256SUMS.txt|six|exact.*revision|available.*concurrency|separately requests" `
  .claude/skills/opi-document `
  .claude/skills/opi-release `
  .claude/skills/opi-realign `
  .claude/skills/opi-slim-tests
rg -n "TaskCreate|AskUserQuestion|git checkout --|git add \$\(git diff|do NOT upload|four crates|five crates|E:\\opi-target" `
  .claude/skills/opi-document `
  .claude/skills/opi-release `
  .claude/skills/opi-realign `
  .claude/skills/opi-slim-tests
```

Expected: required decisions exist; the forbidden-pattern search has no active
instruction hits.

---

### Task 6: Make every high-impact opi skill explicitly invoked across harnesses

**Files:**

- Create: `.claude/skills/opi-audit/agents/openai.yaml`
- Create: `.claude/skills/opi-document/agents/openai.yaml`
- Create: `.claude/skills/opi-eval/agents/openai.yaml`
- Create: `.claude/skills/opi-implement/agents/openai.yaml`
- Modify: `.claude/skills/opi-realign/agents/openai.yaml`
- Create: `.claude/skills/opi-release/agents/openai.yaml`
- Create: `.claude/skills/opi-remediate/agents/openai.yaml`
- Create: `.claude/skills/opi-slim-tests/agents/openai.yaml`
- Modify the corresponding `SKILL.md` / `skill.md` frontmatter where explicit
  invocation is missing.

- [ ] **Step 1: List missing sidecars and invocation flags**

Run:

```powershell
Get-ChildItem .claude/skills -Directory -Filter 'opi-*' | ForEach-Object {
  $skill = Get-ChildItem $_.FullName -File | Where-Object Name -in 'SKILL.md','skill.md'
  [pscustomobject]@{
    Skill = $_.Name
    SkillFile = $skill.Name
    Sidecar = Test-Path (Join-Path $_.FullName 'agents/openai.yaml')
    ClaudeExplicit = if ($skill) {
      [bool](Select-String -LiteralPath $skill.FullName -Pattern '^disable-model-invocation: true$')
    } else { $false }
  }
} | Format-Table -AutoSize
```

Expected before editing: among the pre-existing skills, only `opi-realign` has
a sidecar; Task 2 already added sidecars for `opi-workflow` and `opi-research`;
the other existing skills still lack sidecars and several lack the Claude
explicit-invocation flag.

- [ ] **Step 2: Add the Claude flags**

For every stateful/costly/high-impact opi skill, ensure frontmatter contains:

```yaml
disable-model-invocation: true
```

Do not change the skill's name or argument contract.

- [ ] **Step 3: Add Codex sidecars**

Every sidecar must use:

```yaml
interface:
  display_name: "<human-readable opi skill name>"
  short_description: "<one-sentence routing purpose>"
policy:
  allow_implicit_invocation: false
```

Update the existing `opi-realign` sidecar by adding the policy block without
discarding its current interface/default prompt.

- [ ] **Step 4: Verify metadata coverage**

Re-run the PowerShell inventory from Step 1.

Expected: every `opi-*` directory reports `Sidecar=True` and
`ClaudeExplicit=True`.

---

### Task 7: Rewrite workflow maps and update root agent guidance

**Files:**

- Modify: `.claude/skills/README.md`
- Modify: `.claude/skills/README.zh.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Capture stale workflow-index claims**

Run:

```powershell
rg -n "seven-stage|seven-phase|Phase 1.*manual|ultracode|GLM-5\.2|eight opi|P0.P3.*report|manual.*remediat" `
  .claude/skills/README.md `
  .claude/skills/README.zh.md
rg -n "opi-coding-agent.*opi-ai.*opi-agent.*opi-tui" AGENTS.md CLAUDE.md
```

Expected before editing: the indexes describe a linear seven-stage workflow,
contain machine/model-specific cache text, and the root graph omits the real
`opi-protocol` dependency.

- [ ] **Step 2: Replace the English README with a routing map**

Keep the document concise and organize it under:

```text
Design lineage
Evidence and shaping
Delivery
Assurance and release
Subskill policy
Durable artifacts
Skill index
```

The map must identify ten `opi-*` skills after adding `opi-workflow` and
`opi-research`: eight lifecycle skills plus the router and standalone
test-slimming utility. Do not prescribe a provider/model combination.

- [ ] **Step 3: Mirror the workflow map into Chinese**

Translate the same structure surgically. Preserve skill names, verdict enums,
file paths, schema versions, guard names, commands, and `Opi-*` footer names
verbatim. Confirm both indexes make `realign` inward and `research` outward.

- [ ] **Step 4: Add the concise root workflow pointer**

Add the same short section to `AGENTS.md` and `CLAUDE.md`:

```markdown
## Development workflow

Pi is the inward design reference; outward optional capabilities are researched
for plugin/package placement before core expansion. Consult
`.claude/skills/README.md` for routing among `opi-realign`, `opi-research`,
human-led shaping, `opi-implement`, assurance, documentation, and release.
Do not start ledger work until a reviewed source passes `opi-implement plan`.
```

Adjust wording only as needed to match each root file's existing prose.

- [ ] **Step 5: Correct the dependency graph in both root files**

Use this exact graph:

```text
opi-ai      (no internal deps)
opi-tui     (no internal deps)
opi-agent   -> opi-ai
opi-protocol (no internal deps)
opi-sandbox -> opi-protocol
opi-coding-agent -> opi-ai, opi-agent, opi-protocol, opi-tui
```

Preserve the existing explanatory descriptions after each dependency entry.

- [ ] **Step 6: Verify synchronized routing and root guidance**

Run:

```powershell
rg -n "opi-workflow|opi-research|inward|outward|wayfinder|opi-implement plan|writing-for-agents" `
  .claude/skills/README.md `
  .claude/skills/README.zh.md `
  AGENTS.md CLAUDE.md
rg -n "ultracode|GLM-5\.2|Phase 1.*manual|seven-stage|seven-phase" `
  .claude/skills/README.md `
  .claude/skills/README.zh.md
rg -n "opi-coding-agent -> opi-ai, opi-agent, opi-protocol, opi-tui" AGENTS.md CLAUDE.md
```

Expected: both languages and both root files carry the stable routing
contracts; stale cache text is absent; the dependency graph is correct.

---

### Task 8: Run focused and final verification

**Files:** Read all changed files; make only corrections required by failed
checks.

- [ ] **Step 1: Parse-check every modified workflow script**

Run:

```powershell
Get-ChildItem .claude/skills/opi-implement/scripts -Filter '*.workflow.js' | ForEach-Object {
  node --check $_.FullName
  if ($LASTEXITCODE -ne 0) { throw "JavaScript parse failed: $($_.FullName)" }
}
```

Expected: exit 0 for `plan.workflow.js`, `exec.workflow.js`, and
`phase-exit.workflow.js`.

- [ ] **Step 2: Run the existing ledger guard tests**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .claude/skills/opi-implement/scripts/ledger-guard.tests.ps1
```

Expected: all ledger guard cases pass. This plan does not change the canonical
ledger schema or active ledger.

- [ ] **Step 3: Verify skill and sidecar inventory**

Run:

```powershell
$errors = @()
Get-ChildItem .claude/skills -Directory -Filter 'opi-*' | ForEach-Object {
  $skill = Get-ChildItem $_.FullName -File | Where-Object Name -in 'SKILL.md','skill.md'
  $sidecar = Join-Path $_.FullName 'agents/openai.yaml'
  if (-not $skill) { $errors += "$($_.Name): missing skill file" }
  if (-not (Test-Path $sidecar)) { $errors += "$($_.Name): missing Codex sidecar" }
  if ($skill -and -not (Select-String -LiteralPath $skill.FullName -Pattern '^name: opi-[a-z0-9-]+$')) {
    $errors += "$($_.Name): invalid name"
  }
  if ($skill -and -not (Select-String -LiteralPath $skill.FullName -Pattern '^disable-model-invocation: true$')) {
    $errors += "$($_.Name): implicit Claude invocation still enabled"
  }
  if ((Test-Path $sidecar) -and -not (Select-String -LiteralPath $sidecar -Pattern '^  allow_implicit_invocation: false$')) {
    $errors += "$($_.Name): implicit Codex invocation still enabled"
  }
}
if ($errors.Count) { $errors; exit 1 }
"opi skill metadata: PASS"
```

Expected:

```text
opi skill metadata: PASS
```

- [ ] **Step 4: Run contradiction searches**

Run:

```powershell
rg -n "ultracode|GLM-5\.2|E:\\opi-target|TaskCreate|AskUserQuestion|do NOT upload.*SHA256SUMS|four crates|five crates|Eight guard|eight guard|manual.*remediat|superpowers:test-driven-development|superpowers:systematic-debugging|confirmed_folds" `
  .claude/skills AGENTS.md CLAUDE.md
```

Expected: no active workflow-contract hits. If a term appears only in a clearly
labelled rejected-pattern explanation, inspect it manually rather than deleting
the safety explanation.

- [ ] **Step 5: Verify EN/ZH and root lockstep manually**

Run:

```powershell
rg -n "opi-workflow|opi-research|opi-realign|wayfinder|READY|RESEARCH_REQUIRED|DESIGN_DECISION_REQUIRED|GRAPH_REVISION_REQUIRED|ten|SHA256SUMS" `
  .claude/skills/README.md `
  .claude/skills/README.zh.md `
  AGENTS.md CLAUDE.md
```

Expected: every stable identifier appears in each applicable counterpart. Read
the surrounding paragraphs to confirm semantic, not line-for-line, parity.

- [ ] **Step 6: Verify whitespace and protected scope**

Run:

```powershell
git diff --check
git status --short
git diff --stat
git diff -- Cargo.toml Cargo.lock CHANGELOG.md README.md README.zh.md crates
```

Expected: `git diff --check` exits 0; status contains only pre-existing files
plus this plan's approved skill/root-guidance files; protected-scope diff is
empty relative to the Task 1 baseline.

- [ ] **Step 7: Review the final diff against the design acceptance criteria**

Run:

```powershell
git diff -- .claude/skills AGENTS.md CLAUDE.md `
  docs/superpowers/specs/2026-08-09-opi-workflow-skill-system-optimization-design.md `
  docs/superpowers/plans/2026-08-09-opi-workflow-skill-system-optimization.md
```

Expected: every changed line traces to the approved design; no product behavior,
version, release history, or root product documentation has changed.

## Completion handoff

Report:

- files created and modified;
- Matt subskills selected and Superpowers subskills retained;
- verification commands and exact results;
- any checks skipped and why;
- pre-existing dirty files preserved;
- confirmation that no commit, staging, push, PR, or release occurred.

---

### Follow-up task: Reduce verification cost and remove prose-test sediment

**Scope:** shared skill references, verification routing/cache policy,
documentation checks, obsolete docs-only Rust integration tests, CI wiring,
and synchronized root workflow guidance. Runtime product code remains out of
scope.

- [ ] Merge severity rules into `_shared/references/finding-contract.md`,
  update consumers, and remove the duplicate severity and Git-safety references.
- [ ] Make `scripts/opi-impl-smoke.{sh,ps1}` the single mechanical gate:
  remove standalone workspace builds, remove D.1/D.3 repetition, and never
  clean task caches.
- [ ] Use a persistent external target cache per worktree/toolchain for
  implementation and eval; keep incremental compilation enabled. Implement
  stable resolution, leases, status, and dry-run-first pruning in
  `scripts/opi-cargo-cache.py`.
- [ ] Narrow `evaluator_required` to semantic high-risk changes and retain one
  adversarial phase-exit review.
- [ ] Add `scripts/opi-doc-check.py`, run it in CI, and route documentation-only
  work through it.
- [ ] Remove obsolete prose-only `*docs.rs` integration binaries and move any
  remaining current contract to source-derived checks or owning behavior tests.
- [ ] Update `opi-slim-tests`, both workflow indexes, and `AGENTS.md` /
  `CLAUDE.md` in lockstep.
- [ ] Verify with the Python doc check, script syntax checks, skill validators,
  Cargo metadata target-count comparison, `git diff --check`, and focused
  tests only. Do not run a redundant full-workspace Cargo gate for this
  documentation/test-inventory change.
